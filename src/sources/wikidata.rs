use anyhow::{Context, Result};
use async_trait::async_trait;
use indicatif::ProgressBar;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

use crate::graph::{HyperEdge, HyperNode, NodeType};

use super::{CrawlContext, CrawlResult, DataSource, SourceSchema};

const WDQS_URL: &str = "https://query.wikidata.org/sparql";
const WIKI_API: &str = "https://www.wikidata.org/w/api.php";

// ── Generic SPARQL Templates ────────────────────────────────────
// {COUNTRY_QID} is substituted at crawl time.
// {LANG} is the label language (e.g. "en", "vi", "zh").

/// Places / geography: entities located in the country.
const QUERY_PLACES: &str = r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?coord WHERE {
  VALUES ?country { wd:{COUNTRY_QID} }
  { ?item wdt:P17 ?country . }
  ?item wdt:P31 ?type .
  OPTIONAL { ?item wdt:P625 ?coord . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "{LANG},en" . }
}
LIMIT 5000
"#;

/// People: persons with citizenship or birth place in the country.
const QUERY_PEOPLE: &str = r#"
SELECT DISTINCT ?person ?personLabel ?personDescription ?birthDate ?deathDate ?occupation ?occupationLabel WHERE {
  VALUES ?country { wd:{COUNTRY_QID} }
  { ?person wdt:P27 ?country . }
  UNION
  { ?person wdt:P19 ?birthPlace . ?birthPlace wdt:P17 ?country . }
  ?person wdt:P31 wd:Q5 .
  OPTIONAL { ?person wdt:P569 ?birthDate . }
  OPTIONAL { ?person wdt:P570 ?deathDate . }
  OPTIONAL { ?person wdt:P106 ?occupation . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "{LANG},en" . }
}
LIMIT 5000
"#;

/// Events: historical events that happened in or involved the country.
const QUERY_EVENTS: &str = r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?pointInTime ?startTime ?endTime WHERE {
  VALUES ?country { wd:{COUNTRY_QID} }
  { ?item wdt:P276 ?location . ?location wdt:P17 ?country . }
  UNION
  { ?item wdt:P17 ?country . }
  UNION
  { ?item wdt:P710 ?participant . FILTER(?participant = wd:{COUNTRY_QID}) }
  { ?item wdt:P585 ?pointInTime . }
  UNION
  { ?item wdt:P580 ?startTime . }
  OPTIONAL { ?item wdt:P31 ?type . }
  OPTIONAL { ?item wdt:P582 ?endTime . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "{LANG},en" . }
}
LIMIT 3000
"#;

/// Fetch all direct claims (edges) for a given entity QID.
fn entity_claims_query(qid: &str, lang: &str) -> String {
    format!(
        r#"
        SELECT ?property ?propertyLabel ?value ?valueLabel WHERE {{
          wd:{qid} ?prop ?valueNode .
          ?property wikibase:directClaim ?prop .
          OPTIONAL {{ ?valueNode rdfs:label ?valueLabel . FILTER(LANG(?valueLabel) = "{lang}") }}
          OPTIONAL {{ ?property rdfs:label ?propertyLabel . FILTER(LANG(?propertyLabel) = "{lang}") }}
          FILTER(STRSTARTS(STR(?property), "http://www.wikidata.org/entity/P"))
          FILTER(STRSTARTS(STR(?valueNode), "http://www.wikidata.org/entity/Q"))
        }}
        LIMIT 200
        "#
    )
}

/// Dataset definitions for the 3 generic crawl categories.
struct DatasetDef {
    name: &'static str,
    query_template: &'static str,
    node_type: NodeType,
}

const DATASETS: &[DatasetDef] = &[
    DatasetDef {
        name: "places",
        query_template: QUERY_PLACES,
        node_type: NodeType::Place,
    },
    DatasetDef {
        name: "people",
        query_template: QUERY_PEOPLE,
        node_type: NodeType::Person,
    },
    DatasetDef {
        name: "events",
        query_template: QUERY_EVENTS,
        node_type: NodeType::Event,
    },
];

// ── Source Implementation ───────────────────────────────────────

pub struct WikiDataSource;

#[async_trait]
impl DataSource for WikiDataSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: "wikidata".into(),
            description: "WikiData — open knowledge graph with structured data about the world"
                .into(),
            version: "1.1.0".into(),
            endpoint: WDQS_URL.into(),
            entity_types: vec![
                "place".into(),
                "person".into(),
                "event".into(),
                "concept".into(),
                "organization".into(),
                "artifact".into(),
            ],
            properties: vec![
                "P17 (country)".into(),
                "P31 (instance of)".into(),
                "P580 (start time)".into(),
                "P585 (point in time)".into(),
                "P569 (date of birth)".into(),
                "P106 (occupation)".into(),
                "P625 (coordinate location)".into(),
                "P27 (country of citizenship)".into(),
                "P276 (location)".into(),
            ],
            metadata_fields: HashMap::from([
                ("coordinates".into(), "Geo: latitude/longitude".into()),
                ("point_in_time".into(), "ISO date of event".into()),
                ("start_time".into(), "ISO start date".into()),
                ("end_time".into(), "ISO end date".into()),
                ("birth_date".into(), "ISO birth date".into()),
                ("death_date".into(), "ISO death date".into()),
                ("occupation".into(), "Occupation label".into()),
            ]),
            attribution: "CC0 — WikiData contributors".into(),
        }
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<CrawlResult> {
        let country_qid = ctx.country_qid.as_deref().unwrap_or("Q881"); // fallback: Vietnam
        let lang = ctx.language.as_deref().unwrap_or("en");

        info!(
            "[wikidata] Crawling for country QID: {}, language: {}",
            country_qid, lang
        );

        let mut all_nodes: Vec<HyperNode> = Vec::new();
        let mut all_edges: Vec<HyperEdge> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        let pb = ctx
            .progress
            .then(|| ProgressBar::new_spinner().with_message("[wikidata] crawling..."));

        // If a custom query is provided (from partition config), run it directly
        if let Some(ref custom_query) = ctx.custom_query {
            info!("  [wikidata] Running custom partition query");
            let final_query = if ctx.limit > 0 {
                apply_limit(custom_query, ctx.limit)
            } else {
                custom_query.clone()
            };

            match self.exec_sparql(&ctx.client, &final_query).await {
                Ok(json) => {
                    let bindings = parse_bindings(&json);
                    let custom_type = ctx.custom_node_type.as_deref().unwrap_or("Other");
                    let node_type = node_type_from_string(custom_type);

                    let nodes: Vec<HyperNode> = bindings
                        .iter()
                        .filter_map(|row| binding_to_node(row, "custom", node_type.clone()))
                        .collect();

                    for node in nodes {
                        if seen_ids.insert(node.id.clone()) {
                            all_nodes.push(node);
                        }
                    }
                    info!("  [wikidata] custom query → {} entities", bindings.len());
                }
                Err(e) => warn!("  [wikidata] custom query failed: {}", e),
            }

            if let Some(pb) = pb {
                pb.finish_with_message(format!(
                    "[wikidata] done: {} entities, {} edges",
                    all_nodes.len(),
                    all_edges.len()
                ));
            }

            let mut metadata = HashMap::new();
            metadata.insert("country_qid".into(), country_qid.to_string());
            metadata.insert("entity_count".into(), all_nodes.len().to_string());
            metadata.insert("edge_count".into(), all_edges.len().to_string());

            return Ok(CrawlResult {
                source: "wikidata".into(),
                nodes: all_nodes,
                edges: all_edges,
                metadata,
            });
        }

        for dataset in DATASETS {
            let query = dataset
                .query_template
                .replace("{COUNTRY_QID}", country_qid)
                .replace("{LANG}", lang);

            let final_query = if ctx.limit > 0 {
                apply_limit(&query, ctx.limit)
            } else {
                query
            };

            match self.exec_sparql(&ctx.client, &final_query).await {
                Ok(json) => {
                    let bindings = parse_bindings(&json);
                    let nodes: Vec<HyperNode> = bindings
                        .iter()
                        .filter_map(|row| {
                            binding_to_node(row, dataset.name, dataset.node_type.clone())
                        })
                        .collect();

                    let new_count = nodes.len();
                    for node in nodes {
                        if seen_ids.insert(node.id.clone()) {
                            all_nodes.push(node);
                        }
                    }
                    info!(
                        "  [wikidata] {} → {} entities ({} new)",
                        dataset.name,
                        bindings.len(),
                        new_count
                    );
                }
                Err(e) => warn!("  [wikidata] {} crawl failed: {}", dataset.name, e),
            }

            if let Some(ref pb) = pb {
                pb.set_message(format!("[wikidata] processed {}", dataset.name));
            }
        }

        // Fetch edges for the country entity itself
        if let Ok(edges) = self.fetch_edges_for(&ctx.client, country_qid, lang).await {
            info!("  [wikidata] {} → {} edges", country_qid, edges.len());
            all_edges.extend(edges);
        }

        // Also fetch edges for some of the most central nodes (top N by discovery order)
        let central_qids: Vec<&str> = all_nodes.iter().take(10).map(|n| n.id.as_str()).collect();
        for qid in &central_qids {
            if let Ok(edges) = self.fetch_edges_for(&ctx.client, qid, lang).await {
                info!("  [wikidata] {} → {} edges", qid, edges.len());
                all_edges.extend(edges);
            }
        }

        // Fetch multilingual labels if native language is configured
        if let Some(ref lang) = ctx.language {
            if lang != "en" {
                let qids: Vec<String> = all_nodes.iter().map(|n| n.id.clone()).collect();
                match self.fetch_labels_batch(&ctx.client, &qids, lang).await {
                    Ok(labels) => {
                        for node in &mut all_nodes {
                            if let Some(native_label) = labels.get(&node.id) {
                                node.label_local = Some(native_label.clone());
                            }
                        }
                    }
                    Err(e) => warn!("  [wikidata] native label fetch failed: {}", e),
                }
            }
        }

        if let Some(ref lang) = ctx.language {
            info!("  [wikidata] Fetching Wikipedia summaries (LOD word threshold = 150)...");
            if let Err(e) = self.fetch_wikipedia_summaries_batch(&ctx.client, &mut all_nodes, lang, 150).await {
                warn!("  [wikidata] Wikipedia summaries LOD fetch failed: {}", e);
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message(format!(
                "[wikidata] done: {} entities, {} edges",
                all_nodes.len(),
                all_edges.len()
            ));
        }

        let mut metadata = HashMap::new();
        metadata.insert("country_qid".into(), country_qid.to_string());
        metadata.insert("entity_count".into(), all_nodes.len().to_string());
        metadata.insert("edge_count".into(), all_edges.len().to_string());

        Ok(CrawlResult {
            source: "wikidata".into(),
            nodes: all_nodes,
            edges: all_edges,
            metadata,
        })
    }
}

impl WikiDataSource {
    async fn exec_sparql(&self, client: &reqwest::Client, query: &str) -> Result<Value> {
        let mut retries = 0;
        let max_retries = 5;
        let mut backoff = Duration::from_secs(2);

        loop {
            let resp_result = client
                .post(WDQS_URL)
                .query(&[("format", "json")])
                .header("Accept", "application/sparql-results+json")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", "MinidiSpider/1.1 (https://github.com/minidivn/minidi-spider; contact@minidi.vn)")
                .body(format!("query={}", url_encode(query)))
                .send()
                .await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp.json().await.context("Failed to parse SPARQL JSON")?);
                    }

                    if (status.is_server_error() || status.as_u16() == 429) && retries < max_retries {
                        retries += 1;
                        warn!(
                            "  [wikidata] SPARQL query failed with status {} ({}/{}). Retrying in {}s...",
                            status, retries, max_retries, backoff.as_secs()
                        );
                        tokio::time::sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }

                    let body = resp.text().await.unwrap_or_default();
                    anyhow::bail!("SPARQL {}: {}", status, body);
                }
                Err(e) if retries < max_retries => {
                    retries += 1;
                    warn!(
                        "  [wikidata] Network error sending SPARQL query ({}/{}): {}. Retrying in {}s...",
                        retries, max_retries, e, backoff.as_secs()
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                Err(e) => return Err(e).context("SPARQL request failed after retries"),
            }
        }
    }

    async fn fetch_labels_batch(
        &self,
        client: &reqwest::Client,
        qids: &[String],
        lang: &str,
    ) -> Result<HashMap<String, String>> {
        let mut result = HashMap::new();
        for chunk in qids.chunks(50) {
            let ids = chunk.join("|");
            let params = [
                ("action", "wbgetentities"),
                ("ids", &ids),
                ("props", "labels"),
                ("languages", &format!("en|{}", lang)),
                ("format", "json"),
            ];

            if let Ok(resp) = client.get(WIKI_API).query(&params).send().await {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(entities) = json["entities"].as_object() {
                        for (qid, entity) in entities {
                            if let Some(label) = entity["labels"][lang]["value"].as_str() {
                                result.insert(qid.clone(), label.to_string());
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Ok(result)
    }

    async fn fetch_edges_for(
        &self,
        client: &reqwest::Client,
        qid: &str,
        lang: &str,
    ) -> Result<Vec<HyperEdge>> {
        let query = entity_claims_query(qid, lang);
        let json = self.exec_sparql(client, &query).await?;
        let bindings = parse_bindings(&json);

        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for row in &bindings {
            let pid = row.get("property").and_then(|v| extract_qid(v));
            let target = row.get("value").and_then(|v| extract_qid(v));
            let plabel = row.get("propertyLabel").cloned().unwrap_or_default();
            let tlabel = row.get("valueLabel").cloned().unwrap_or_default();

            if let (Some(pid), Some(tid)) = (pid, target) {
                let key = format!("{}-{}-{}", qid, pid, tid);
                if seen.insert(key) {
                    edges.push(HyperEdge {
                        property_id: pid,
                        property_label: plabel,
                        source: qid.to_string(),
                        target: tid,
                        target_label: tlabel,
                        qualifiers: HashMap::new(),
                    });
                }
            }
        }
        Ok(edges)
    }

    async fn fetch_wikipedia_summaries_batch(
        &self,
        client: &reqwest::Client,
        nodes: &mut [HyperNode],
        lang: &str,
        word_threshold: usize,
    ) -> Result<()> {
        let limit_nodes = std::cmp::min(nodes.len(), 30);
        for i in 0..limit_nodes {
            let qid = &nodes[i].id;
            let params = [
                ("action", "wbgetentities"),
                ("ids", qid),
                ("props", "sitelinks"),
                ("format", "json"),
            ];

            let mut wiki_title = None;
            let mut wiki_lang = "en".to_string();

            if let Ok(resp) = client.get(WIKI_API).query(&params).send().await {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(entity) = json["entities"][qid].as_object() {
                        let local_wiki = format!("{}wiki", lang);
                        if let Some(title) = entity["sitelinks"][&local_wiki]["title"].as_str() {
                            wiki_title = Some(title.to_string());
                            wiki_lang = lang.to_string();
                        } else if let Some(title) = entity["sitelinks"]["enwiki"]["title"].as_str() {
                            wiki_title = Some(title.to_string());
                            wiki_lang = "en".to_string();
                        }
                    }
                }
            }

            if let Some(title) = wiki_title {
                let wp_api = format!("https://{}.wikipedia.org/w/api.php", wiki_lang);
                let wp_params = [
                    ("action", "query"),
                    ("prop", "extracts"),
                    ("exintro", "1"),
                    ("explaintext", "1"),
                    ("titles", &title),
                    ("format", "json"),
                    ("redirects", "1"),
                ];

                if let Ok(resp) = client.get(&wp_api).query(&wp_params).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(pages) = json["query"]["pages"].as_object() {
                            for (_, page) in pages {
                                if let Some(extract) = page["extract"].as_str() {
                                    if !extract.is_empty() {
                                        let truncated = truncate_to_word_count(extract, word_threshold);
                                        nodes[i].metadata.insert("summary".to_string(), truncated);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn parse_bindings(result: &Value) -> Vec<HashMap<String, String>> {
    result["results"]["bindings"]
        .as_array()
        .map(|b| {
            b.iter()
                .filter_map(|binding| {
                    binding.as_object().map(|obj| {
                        let mut row = HashMap::new();
                        for (key, val) in obj {
                            if let Some(value) = val["value"].as_str() {
                                row.insert(key.clone(), value.to_string());
                            }
                        }
                        row
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_qid(uri: &str) -> Option<String> {
    uri.rsplit('/')
        .next()
        .filter(|s| s.starts_with('Q'))
        .map(|s| s.to_string())
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for byte in s.bytes() {
        match byte {
            b' ' => out.push('+'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    out
}

fn apply_limit(query: &str, limit: usize) -> String {
    let lowered = query.to_lowercase();
    if let Some(pos) = lowered.rfind("limit") {
        let rest = &query[pos..];
        let after_limit = rest.trim_start_matches(|c: char| c.is_alphabetic() || c.is_whitespace());
        let num_end = after_limit
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after_limit.len());
        let new = format!(
            "{} LIMIT {}{}",
            &query[..pos].trim_end(),
            limit,
            &after_limit[num_end..]
        );
        new
    } else {
        format!("{} LIMIT {}", query.trim(), limit)
    }
}

/// Parse node type from a config string.
fn node_type_from_string(s: &str) -> NodeType {
    match s.to_lowercase().as_str() {
        "place" | "location" | "geography" => NodeType::Place,
        "person" | "people" | "human" => NodeType::Person,
        "event" | "history" => NodeType::Event,
        "concept" | "idea" => NodeType::Concept,
        "organization" | "org" => NodeType::Organization,
        "artifact" | "object" | "work" => NodeType::Artifact,
        _ => NodeType::Other,
    }
}

fn binding_to_node(
    row: &HashMap<String, String>,
    dataset: &str,
    default_type: NodeType,
) -> Option<HyperNode> {
    let id = row
        .get("item")
        .or(row.get("person"))
        .and_then(|v| extract_qid(v))?;
    let label = row
        .get("itemLabel")
        .or(row.get("personLabel"))
        .or(row.get("valueLabel"))
        .cloned()
        .unwrap_or_default();
    if label.is_empty() {
        return None;
    }

    let description = row
        .get("itemDescription")
        .or(row.get("personDescription"))
        .cloned()
        .unwrap_or_default();

    let node_type = if dataset == "people" || row.contains_key("birthDate") {
        NodeType::Person
    } else if dataset == "events" || row.contains_key("pointInTime") {
        NodeType::Event
    } else {
        default_type
    };

    let mut metadata = HashMap::new();
    for (k, v) in &[
        ("coord", "coordinates"),
        ("pointInTime", "point_in_time"),
        ("startTime", "start_time"),
        ("endTime", "end_time"),
        ("birthDate", "birth_date"),
        ("deathDate", "death_date"),
        ("occupationLabel", "occupation"),
    ] {
        if let Some(val) = row.get(*k) {
            metadata.insert(v.to_string(), val.clone());
        }
    }

    Some(HyperNode {
        id: id.clone(),
        label,
        label_local: None,
        description,
        description_local: None,
        aliases: Vec::new(),
        aliases_local: Vec::new(),
        node_type,
        wikidata_url: format!("https://www.wikidata.org/wiki/{id}"),
        metadata,
    })
}

fn truncate_to_word_count(text: &str, limit: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= limit {
        return text.to_string();
    }
    let truncated_text = words[..limit].join(" ");
    if let Some(last_period) = truncated_text.rfind('.') {
        if last_period > truncated_text.len() * 3 / 4 {
            return truncated_text[..=last_period].to_string();
        }
    }
    format!("{}...", truncated_text)
}
