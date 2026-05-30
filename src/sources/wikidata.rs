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

// ── SPARQL Queries ──────────────────────────────────────────────

const VIETNAM_ADMIN_DIVISIONS: &str = r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?coord WHERE {
  VALUES ?country { wd:Q881 }
  ?country wdt:P150 ?item .
  OPTIONAL { ?item wdt:P31 ?type . }
  OPTIONAL { ?item wdt:P625 ?coord . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en,vi" . }
}
LIMIT 1000
"#;

const VIETNAM_HISTORY_EVENTS: &str = r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?pointInTime ?startTime ?endTime WHERE {
  VALUES ?country { wd:Q881 }
  { ?item wdt:P276 ?location . }
  UNION
  { ?item wdt:P17 ?country . }
  UNION
  { ?item wdt:P710 ?participant . FILTER(?participant = wd:Q881) }
  { ?item wdt:P585 ?pointInTime . }
  UNION
  { ?item wdt:P580 ?startTime . }
  OPTIONAL { ?item wdt:P31 ?type . }
  OPTIONAL { ?item wdt:P582 ?endTime . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en,vi" . }
}
LIMIT 3000
"#;

const VIETNAM_PEOPLE: &str = r#"
SELECT DISTINCT ?person ?personLabel ?personDescription ?birthDate ?deathDate ?occupation ?occupationLabel WHERE {
  VALUES ?country { wd:Q881 }
  { ?person wdt:P27 ?country . }
  UNION
  { ?person wdt:P19 ?birthPlace . ?birthPlace wdt:P17 ?country . }
  ?person wdt:P31 wd:Q5 .
  OPTIONAL { ?person wdt:P569 ?birthDate . }
  OPTIONAL { ?person wdt:P570 ?deathDate . }
  OPTIONAL { ?person wdt:P106 ?occupation . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en,vi" . }
}
LIMIT 5000
"#;

const VIETNAM_HERITAGE: &str = r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?coord WHERE {
  VALUES ?country { wd:Q881 }
  { ?item wdt:P31 wd:Q9259 . }
    UNION
  { ?item wdt:P31 wd:Q916333 . }
    UNION
  { ?item wdt:P31 wd:Q110602949 . }
    UNION
  { ?item wdt:P17 ?country . ?item wdt:P31 wd:Q570116 . }
  OPTIONAL { ?item wdt:P625 ?coord . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "en,vi" . }
}
LIMIT 2000
"#;

fn entity_claims_query(qid: &str) -> String {
    format!(
        r#"
SELECT ?property ?propertyLabel ?value ?valueLabel WHERE {{
  wd:{qid} ?prop ?valueNode .
  ?property wikibase:directClaim ?prop .
  OPTIONAL {{ ?valueNode rdfs:label ?valueLabel . FILTER(LANG(?valueLabel) = "en") }}
  OPTIONAL {{ ?property rdfs:label ?propertyLabel . FILTER(LANG(?propertyLabel) = "en") }}
  FILTER(STRSTARTS(STR(?property), "http://www.wikidata.org/entity/P"))
  FILTER(STRSTARTS(STR(?valueNode), "http://www.wikidata.org/entity/Q"))
}}
LIMIT 200
"#,
        qid
    )
}

// ── Source Implementation ───────────────────────────────────────

pub struct WikiDataSource;

#[async_trait]
impl DataSource for WikiDataSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: "wikidata".into(),
            description: "WikiData — open knowledge graph with structured data about the world"
                .into(),
            version: "1.0.0".into(),
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
                "P150 (contains admin division)",
                "P131 (located in)",
                "P17 (country)",
                "P31 (instance of)",
                "P580 (start time)",
                "P585 (point in time)",
                "P569 (date of birth)",
                "P106 (occupation)",
                "P625 (coordinate location)",
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
        info!("[wikidata] Starting Vietnam crawl...");

        let datasets: Vec<(&str, &str)> = vec![
            ("admin_divisions", VIETNAM_ADMIN_DIVISIONS),
            ("history_events", VIETNAM_HISTORY_EVENTS),
            ("people", VIETNAM_PEOPLE),
            ("heritage", VIETNAM_HERITAGE),
        ];

        let mut all_nodes: Vec<HyperNode> = Vec::new();
        let mut all_edges: Vec<HyperEdge> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        let pb = ctx
            .progress
            .then(|| ProgressBar::new_spinner().with_message("[wikidata] crawling..."));

        for (name, query) in &datasets {
            let limit = ctx.limit;
            let final_query = if limit > 0 {
                apply_limit(query, limit)
            } else {
                query.to_string()
            };

            match self.exec_sparql(&ctx.client, &final_query).await {
                Ok(json) => {
                    let bindings = parse_bindings(&json);
                    let nodes: Vec<HyperNode> = bindings
                        .iter()
                        .filter_map(|row| binding_to_node(row, name))
                        .collect();
                    for node in nodes {
                        if seen_ids.insert(node.id.clone()) {
                            all_nodes.push(node);
                        }
                    }
                    let count = if limit > 0 {
                        limit.min(bindings.len())
                    } else {
                        bindings.len()
                    };
                    info!("  [wikidata] {} → {} entities", name, count);
                }
                Err(e) => warn!("  [wikidata] {} crawl failed: {}", name, e),
            }

            if let Some(ref pb) = pb {
                pb.set_message(format!("[wikidata] processed {name}"));
            }
        }

        // Fetch edges for central entities
        let central = vec!["Q881", "Q1858", "Q1854"];
        for qid in &central {
            if let Ok(edges) = self.fetch_edges_for(&ctx.client, qid).await {
                info!("  [wikidata] {} → {} edges", qid, edges.len());
                all_edges.extend(edges);
            }
        }

        // Fetch multilingual labels
        let qids: Vec<String> = all_nodes.iter().map(|n| n.id.clone()).collect();
        match self.fetch_labels_batch(&ctx.client, &qids).await {
            Ok(labels) => {
                for node in &mut all_nodes {
                    if let Some((en, vi)) = labels.get(&node.id) {
                        if !en.is_empty() {
                            node.label = en.clone();
                        }
                        node.label_vi = vi.clone();
                    }
                }
            }
            Err(e) => warn!("  [wikidata] label fetch failed: {}", e),
        }

        if let Some(pb) = pb {
            pb.finish_with_message(format!(
                "[wikidata] done: {} entities, {} edges",
                all_nodes.len(),
                all_edges.len()
            ));
        }

        let mut metadata = HashMap::new();
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
        let resp = client
            .post(WDQS_URL)
            .query(&[("format", "json")])
            .body(query.to_string())
            .header("Accept", "application/sparql-results+json")
            .send()
            .await
            .context("SPARQL request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("SPARQL {}: {}", status, body);
        }

        Ok(resp.json().await.context("Failed to parse SPARQL JSON")?)
    }

    async fn fetch_labels_batch(
        &self,
        client: &reqwest::Client,
        qids: &[String],
    ) -> Result<HashMap<String, (String, Option<String>)>> {
        let mut result = HashMap::new();
        for chunk in qids.chunks(50) {
            let ids = chunk.join("|");
            let params = [
                ("action", "wbgetentities"),
                ("ids", &ids),
                ("props", "labels"),
                ("languages", "en|vi"),
                ("format", "json"),
            ];

            if let Ok(resp) = client.get(WIKI_API).query(&params).send().await {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(entities) = json["entities"].as_object() {
                        for (qid, entity) in entities {
                            let en = entity["labels"]["en"]["value"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            let vi = entity["labels"]["vi"]["value"]
                                .as_str()
                                .map(|s| s.to_string());
                            result.insert(qid.clone(), (en, vi));
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Ok(result)
    }

    async fn fetch_edges_for(&self, client: &reqwest::Client, qid: &str) -> Result<Vec<HyperEdge>> {
        let query = entity_claims_query(qid);
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

fn apply_limit(query: &str, limit: usize) -> String {
    // Replace LIMIT clause if present
    let lowered = query.to_lowercase();
    if let Some(pos) = lowered.rfind("limit") {
        // Find end of the numeric limit
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

fn binding_to_node(row: &HashMap<String, String>, dataset: &str) -> Option<HyperNode> {
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
    } else if dataset == "history_events" || row.contains_key("pointInTime") {
        NodeType::Event
    } else if dataset == "heritage" || row.contains_key("coord") {
        match row.get("typeLabel").map(|s| s.as_str()) {
            Some("world heritage site") | Some("national park") => NodeType::Place,
            _ => NodeType::Place,
        }
    } else {
        NodeType::Place
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
        id,
        label,
        label_vi: None,
        description,
        description_vi: None,
        aliases: Vec::new(),
        aliases_vi: Vec::new(),
        node_type,
        wikidata_url: format!("https://www.wikidata.org/wiki/{}", &id),
        metadata,
    })
}
