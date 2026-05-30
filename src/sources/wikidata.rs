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

// ── Partition Definitions ───────────────────────────────────────

/// A crawlable partition that maps to a subset of WikiData queries.
pub struct PartitionDef {
    pub name: &'static str,
    pub description: &'static str,
    /// Base query category: "admin", "history", "people", "heritage"
    pub category: &'static str,
    /// Optional SPARQL FILTER snippet (appended to base query WHERE clause).
    /// Empty string = no filter.
    pub filter: &'static str,
}

/// All 50 partitions for gradual data fill.
/// Each partition crawls a focused subset and can be committed independently.
pub const ALL_PARTITIONS: &[PartitionDef] = &[
    // ── Tier 1: Geography / Admin Divisions (10) ──
    PartitionDef {
        name: "adm-vn-country",
        description: "Vietnam country entity + neighbors",
        category: "admin",
        filter: "?item = wd:Q881",
    },
    PartitionDef {
        name: "adm-north-provinces",
        description: "Northern provinces (Red River Delta, Northeast, Northwest)",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-central-provinces",
        description: "North Central Coast provinces",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-south-provinces",
        description: "Southern provinces (Southeast, Mekong Delta)",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-highlands",
        description: "Central Highlands (Tây Nguyên) provinces",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-hanoi",
        description: "Hanoi districts and communes",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-hcmc",
        description: "Ho Chi Minh City districts and communes",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-danang",
        description: "Da Nang city districts",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-haiphong",
        description: "Hai Phong city districts",
        category: "admin",
        filter: "",
    },
    PartitionDef {
        name: "adm-cantho",
        description: "Can Tho city districts",
        category: "admin",
        filter: "",
    },
    // ── Tier 2: History by Era (12) ──
    PartitionDef {
        name: "hist-paleolithic",
        description: "Paleolithic cultures (Sơn Vi, Hòa Bình, Đông Sơn)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) < -1000)",
    },
    PartitionDef {
        name: "hist-hongbang",
        description: "Hồng Bàng dynasty (2879–258 BCE)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= -2879 && YEAR(?pointInTime) < -258)",
    },
    PartitionDef {
        name: "hist-chinese-dom",
        description: "Chinese domination (111 BCE–939 CE)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= -111 && YEAR(?pointInTime) < 939)",
    },
    PartitionDef {
        name: "hist-ngo-dinh-le",
        description: "Ngô-Đinh-Lê dynasties (939–1009)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 939 && YEAR(?pointInTime) < 1010)",
    },
    PartitionDef {
        name: "hist-ly",
        description: "Lý dynasty (1009–1225)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1010 && YEAR(?pointInTime) < 1225)",
    },
    PartitionDef {
        name: "hist-tran",
        description: "Trần dynasty + Mongol invasions (1225–1400)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1225 && YEAR(?pointInTime) < 1400)",
    },
    PartitionDef {
        name: "hist-le-so",
        description: "Later Lê dynasty (1428–1789)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1428 && YEAR(?pointInTime) < 1789)",
    },
    PartitionDef {
        name: "hist-tay-son",
        description: "Tây Sơn dynasty (1778–1802)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1778 && YEAR(?pointInTime) < 1802)",
    },
    PartitionDef {
        name: "hist-nguyen",
        description: "Nguyễn dynasty (1802–1945)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1802 && YEAR(?pointInTime) < 1945)",
    },
    PartitionDef {
        name: "hist-colonial",
        description: "French colonial period (1858–1954)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1858 && YEAR(?pointInTime) < 1954)",
    },
    PartitionDef {
        name: "hist-vn-war",
        description: "Vietnam War / American War (1955–1975)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1955 && YEAR(?pointInTime) < 1975)",
    },
    PartitionDef {
        name: "hist-modern",
        description: "Modern Vietnam (1975–present)",
        category: "history",
        filter: "FILTER(YEAR(?pointInTime) >= 1975)",
    },
    // ── Tier 3: People by Occupation (14) ──
    PartitionDef {
        name: "people-rulers",
        description: "Kings, emperors, presidents, prime ministers",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-military",
        description: "Generals, strategists, war heroes",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-writers",
        description: "Poets, writers, journalists",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-scientists",
        description: "Scientists, inventors, doctors",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-artists",
        description: "Painters, musicians, filmmakers",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-revolution",
        description: "Revolutionaries, independence fighters",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-religion",
        description: "Buddhist monks, religious figures",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-educators",
        description: "Teachers, professors, scholars",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-politicians",
        description: "Politicians, diplomats, officials",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-sports",
        description: "Athletes, coaches, sports figures",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-business",
        description: "Business people, entrepreneurs",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-foreign",
        description: "Foreigners significant to Vietnam",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-royalty",
        description: "Royal family members, consorts",
        category: "people",
        filter: "",
    },
    PartitionDef {
        name: "people-contemporary",
        description: "21st century contemporary figures",
        category: "people",
        filter: "",
    },
    // ── Tier 4: Culture & Heritage (8) ──
    PartitionDef {
        name: "culture-heritage",
        description: "UNESCO World Heritage sites",
        category: "heritage",
        filter: "?item wdt:P31 wd:Q9259",
    },
    PartitionDef {
        name: "culture-festivals",
        description: "Festivals and celebrations",
        category: "heritage",
        filter: "?item wdt:P31 wd:Q132241",
    },
    PartitionDef {
        name: "culture-religion",
        description: "Temples, pagodas, churches, religious sites",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "culture-cuisine",
        description: "Vietnamese dishes, ingredients, food culture",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "culture-music",
        description: "Traditional music, instruments, performers",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "culture-architecture",
        description: "Architectural works, monuments, landmarks",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "culture-clothing",
        description: "Traditional dress (áo dài, etc.)",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "culture-oral",
        description: "Intangible cultural heritage",
        category: "heritage",
        filter: "",
    },
    // ── Tier 5: Nature & Geography (6) ──
    PartitionDef {
        name: "nature-rivers",
        description: "Rivers, waterways, deltas",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "nature-mountains",
        description: "Mountains, passes, peaks",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "nature-national-parks",
        description: "National parks, nature reserves",
        category: "heritage",
        filter: "?item wdt:P31 wd:Q916333",
    },
    PartitionDef {
        name: "nature-islands",
        description: "Islands, archipelagos",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "nature-beaches",
        description: "Beaches, bays, coastal features",
        category: "heritage",
        filter: "",
    },
    PartitionDef {
        name: "nature-caves",
        description: "Caves, grottoes, karst formations",
        category: "heritage",
        filter: "",
    },
];

pub fn find_partition(name: &str) -> Option<&'static PartitionDef> {
    ALL_PARTITIONS.iter().find(|p| p.name == name)
}

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
        let mut all_nodes: Vec<HyperNode> = Vec::new();
        let mut all_edges: Vec<HyperEdge> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Determine which dataset(s) to crawl
        let datasets: Vec<(&str, &str, Option<String>)> = if let Some(ref pname) = ctx.partition {
            // Single partition mode
            let part = find_partition(pname).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown partition: {}. Use --list-partitions to see all.",
                    pname
                )
            })?;
            info!("[wikidata] Partition: {} — {}", part.name, part.description);
            match self.build_partition_query(part) {
                Some((name, query)) => vec![(name, query, None)],
                None => return Err(anyhow::anyhow!("No query for partition: {}", pname)),
            }
        } else {
            // Full crawl: run all 4 datasets
            vec![
                ("admin_divisions", "full", None),
                ("history_events", "full", None),
                ("people", "full", None),
                ("heritage", "full", None),
            ]
            .into_iter()
            .map(|(n, _, _)| {
                let q = match n {
                    "admin_divisions" => VIETNAM_ADMIN_DIVISIONS.to_string(),
                    "history_events" => VIETNAM_HISTORY_EVENTS.to_string(),
                    "people" => VIETNAM_PEOPLE.to_string(),
                    "heritage" => VIETNAM_HERITAGE.to_string(),
                    _ => unreachable!(),
                };
                let final_q = if ctx.limit > 0 {
                    apply_limit(&q, ctx.limit)
                } else {
                    q
                };
                (n, Some(final_q))
            })
            .collect::<Vec<_>>()
        };

        let pb = ctx
            .progress
            .then(|| ProgressBar::new_spinner().with_message("[wikidata] crawling..."));

        for (name, query_opt, _) in &datasets {
            let q = match query_opt {
                Some(q) => q.clone(),
                None => continue,
            };

            match self.exec_sparql(&ctx.client, &q).await {
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
                    let count = if ctx.limit > 0 {
                        ctx.limit.min(bindings.len())
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
    /// Build a SPARQL query for a specific partition definition.
    /// Returns (query_name, query_string).
    fn build_partition_query(&self, part: &PartitionDef) -> Option<(&str, String)> {
        let base = match part.category {
            "admin" => VIETNAM_ADMIN_DIVISIONS,
            "history" => VIETNAM_HISTORY_EVENTS,
            "people" => VIETNAM_PEOPLE,
            "heritage" => VIETNAM_HERITAGE,
            _ => return None,
        };

        let filter = part.filter.trim();
        let query = if filter.is_empty() {
            base.to_string()
        } else {
            // Insert filter before the SERVICE wikibase:label line
            if let Some(pos) = base.rfind("SERVICE wikibase:label") {
                // Build the FILTER or triple pattern
                let filter_clause = if filter.starts_with("FILTER") || filter.starts_with("filter")
                {
                    format!("  {}\n", filter)
                } else if filter.contains(' ') && !filter.contains("FILTER") {
                    // It's a raw triple pattern
                    format!("  {}.\n", filter)
                } else {
                    format!("  FILTER({}).\n", filter)
                };

                let before = &base[..pos];
                let after = &base[pos..];
                format!("{}{}{}", before, filter_clause, after)
            } else {
                base.to_string()
            }
        };

        Some((part.category, query))
    }

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
