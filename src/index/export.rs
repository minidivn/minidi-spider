use anyhow::{Context, Result};
use chrono::Datelike;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

use crate::config::CountryConfig;
use crate::graph::{HyperEdge, HyperGraph, HyperNode, NodeType};

/// Generic century-based temporal groupings (works for any country).
const CENTURIES: &[(i64, i64, &str)] = &[
    (-5000, -3001, "5th-millennium-bce"),
    (-3000, -2001, "3rd-millennium-bce"),
    (-2000, -1001, "2nd-millennium-bce"),
    (-1000, -501, "1st-millennium-bce"),
    (-500, -1, "5th-1st-century-bce"),
    (1, 500, "1st-5th-century"),
    (501, 1000, "6th-10th-century"),
    (1001, 1300, "11th-13th-century"),
    (1301, 1500, "14th-15th-century"),
    (1501, 1700, "16th-17th-century"),
    (1701, 1800, "18th-century"),
    (1801, 1900, "19th-century"),
    (1901, 1950, "1901-1950"),
    (1951, 2000, "1951-2000"),
    (2001, i64::MAX, "21st-century"),
];

/// Property IDs grouped by relation category for partitioned export.
const GEO_PROPERTIES: &[&str] = &["P17", "P131", "P625", "P2044", "P2046", "P402", "P421"];
const TEMPORAL_PROPERTIES: &[&str] = &["P580", "P582", "P585", "P569", "P570", "P571"];
const SOCIAL_PROPERTIES: &[&str] = &[
    "P106", "P69", "P1416", "P27", "P19", "P22", "P25", "P26", "P3373",
];
const RELATION_PROPERTIES: &[&str] = &["P150", "P47", "P36", "P40", "P1037"];

/// Flat export format for the frontend (compact JSON).
#[derive(Debug, Serialize)]
pub struct ExportIndex {
    pub meta: ExportMeta,
    pub nodes: Vec<ExportNode>,
    pub edges: Vec<ExportEdge>,
}

#[derive(Debug, Serialize)]
pub struct ExportMeta {
    pub crawl_version: &'static str,
    pub crawled_at: String,
    pub country_code: String,
    pub country_name: String,
    pub country_qid: String,
    pub entity_count: usize,
    pub edge_count: usize,
    pub search_fields: Vec<&'static str>,
    pub query_endpoint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ExportNode {
    pub id: String,
    pub l: String,                                    // label (English)
    pub ll: Option<String>,                           // label (local/native)
    pub d: String,                                    // description (English)
    pub dl: Option<String>,                           // description (local)
    pub t: String,                                    // node_type (lowercase)
    pub u: String,                                    // wikidata_url
    pub m: std::collections::HashMap<String, String>, // metadata
}

#[derive(Debug, Serialize)]
pub struct ExportEdge {
    pub s: String,  // source
    pub r: String,  // relation label
    pub t: String,  // target
    pub tl: String, // target label
}

impl ExportNode {
    fn from_node(node: &HyperNode) -> Self {
        Self {
            id: node.id.clone(),
            l: node.label.clone(),
            ll: node.label_local.clone(),
            d: node.description.clone(),
            dl: node.description_local.clone(),
            t: format!("{:?}", node.node_type).to_lowercase(),
            u: node.wikidata_url.clone(),
            m: node.metadata.clone(),
        }
    }
}

impl ExportEdge {
    fn from_edge(edge: &HyperEdge) -> Self {
        Self {
            s: edge.source.clone(),
            r: edge.property_label.clone(),
            t: edge.target.clone(),
            tl: edge.target_label.clone(),
        }
    }
}

/// Convert a HyperGraph into the export format and write to disk.
pub fn export_graph_to_json(
    graph: &HyperGraph,
    output_dir: &Path,
    country: &CountryConfig,
) -> Result<()> {
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let nodes: Vec<ExportNode> = graph.nodes.values().map(ExportNode::from_node).collect();
    let edges: Vec<ExportEdge> = graph.edges.iter().map(ExportEdge::from_edge).collect();

    let index = ExportIndex {
        meta: ExportMeta {
            crawl_version: env!("CARGO_PKG_VERSION"),
            crawled_at: chrono::Utc::now().to_rfc3339(),
            country_code: country.code.clone(),
            country_name: country.name.clone(),
            country_qid: country.qid.clone(),
            entity_count: nodes.len(),
            edge_count: edges.len(),
            search_fields: vec!["label", "label_local", "description", "aliases"],
            query_endpoint: "https://query.wikidata.org/sparql",
        },
        nodes,
        edges,
    };

    let json_path = output_dir.join("index.json");
    let json_bytes = serde_json::to_vec(&index).context("Failed to serialize export JSON")?;
    fs::write(&json_path, &json_bytes).context("Failed to write index.json")?;

    let size_mb = json_bytes.len() as f64 / 1_048_576.0;
    info!(
        "Exported {} nodes, {} edges → {} ({:.2} MB)",
        index.meta.entity_count,
        index.meta.edge_count,
        json_path.display(),
        size_mb
    );

    Ok(())
}

/// Export a compressed version of the index for faster downloads.
pub fn export_graph_to_compressed_json(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base_path = output_dir.join("index.json");
    if !base_path.exists() {
        // If no country context, skip — caller should call the full export first
        return Ok(());
    }

    #[derive(Serialize)]
    struct LiteNode {
        id: String,
        l: String,
        ll: Option<String>,
        t: String,
        u: String,
    }

    let nodes: Vec<LiteNode> = graph
        .nodes
        .values()
        .map(|n| LiteNode {
            id: n.id.clone(),
            l: n.label.clone(),
            ll: n.label_local.clone(),
            t: format!("{:?}", n.node_type).to_lowercase(),
            u: n.wikidata_url.clone(),
        })
        .collect();

    let lite_path = output_dir.join("index.lite.json");
    let lite_bytes = serde_json::to_vec(&nodes)?;
    fs::write(&lite_path, &lite_bytes)?;

    info!(
        "Exported lite index: {} nodes → {} ({:.2} MB)",
        nodes.len(),
        lite_path.display(),
        lite_bytes.len() as f64 / 1_048_576.0
    );

    Ok(())
}

/// Write a JSON file to the partition directory.
fn write_json_partition(dir: &Path, filename: &str, data: &impl Serialize) -> Result<()> {
    fs::create_dir_all(dir).context("Failed to create partition dir")?;
    let path = dir.join(filename);
    let bytes = serde_json::to_vec(data).context("Failed to serialize partition")?;
    fs::write(&path, &bytes).context("Failed to write partition")?;
    info!(
        "  Wrote {} ({:.2} KB)",
        path.display(),
        bytes.len() as f64 / 1024.0
    );
    Ok(())
}

/// Export entities partitioned by NodeType into `v1/entities/`.
pub fn export_partitioned_by_type(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base = output_dir.join("v1").join("entities");

    let mut by_type: HashMap<String, Vec<ExportNode>> = HashMap::new();
    for node in graph.nodes.values() {
        let key = format!("{:?}", node.node_type).to_lowercase();
        by_type
            .entry(key)
            .or_default()
            .push(ExportNode::from_node(node));
    }

    for (type_name, nodes) in &by_type {
        write_json_partition(&base, &format!("{}.json", type_name), nodes)?;
    }

    // Write a type index
    let type_index: Vec<serde_json::Value> = by_type
        .iter()
        .map(|(t, nodes)| {
            serde_json::json!({
                "type": t,
                "count": nodes.len(),
            })
        })
        .collect();
    write_json_partition(&base, "_index.json", &type_index)?;

    info!("Exported {} entity type partitions", by_type.len());
    Ok(())
}

/// Parse a year from a WikiData date string (ISO 8601 or similar).
fn parse_year(datestr: &str) -> Option<i64> {
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(datestr, "%Y-%m-%d") {
        return Some(dt.year() as i64);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datestr, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.year() as i64);
    }
    if let Ok(year) = datestr.parse::<i64>() {
        return Some(year);
    }
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(datestr, "%-Y-%m-%d") {
        return Some(dt.year() as i64);
    }
    None
}

/// Get the best year estimate for an entity from its metadata.
fn entity_year(node: &HyperNode) -> Option<i64> {
    for key in &["point_in_time", "start_time", "birth_date", "death_date"] {
        if let Some(val) = node.metadata.get(*key) {
            if let Some(year) = parse_year(val) {
                return Some(year);
            }
        }
    }
    None
}

/// Classify a year into a century bucket slug.
fn classify_century(year: i64) -> &'static str {
    for (start, end, slug) in CENTURIES {
        if year >= *start && year <= *end {
            return slug;
        }
    }
    "unknown"
}

/// Export entities partitioned by century into `v1/timeline/`.
pub fn export_partitioned_by_century(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base = output_dir.join("v1").join("timeline");

    let mut by_century: HashMap<&str, Vec<ExportNode>> = HashMap::new();
    let mut unknown: Vec<ExportNode> = Vec::new();

    for node in graph.nodes.values() {
        let en = ExportNode::from_node(node);
        match entity_year(node) {
            Some(year) => {
                let slug = classify_century(year);
                by_century.entry(slug).or_default().push(en);
            }
            None => unknown.push(en),
        }
    }

    for (slug, nodes) in &by_century {
        let label = slug.replace('-', " ").replace("bce", "BCE");
        let payload = serde_json::json!({
            "period": slug,
            "label": label,
            "entity_count": nodes.len(),
            "entities": nodes,
        });
        write_json_partition(&base, &format!("{}.json", slug), &payload)?;
    }

    if !unknown.is_empty() {
        let payload = serde_json::json!({
            "period": "unknown",
            "label": "Unclassified",
            "entity_count": unknown.len(),
            "entities": unknown,
        });
        write_json_partition(&base, "unknown.json", &payload)?;
    }

    // Period index
    let period_index: Vec<serde_json::Value> = CENTURIES
        .iter()
        .map(|(_, _, slug)| {
            let count = by_century.get(slug).map(|v| v.len()).unwrap_or(0);
            let label = slug.replace('-', " ").replace("bce", "BCE");
            serde_json::json!({
                "slug": slug,
                "label": label,
                "entity_count": count,
            })
        })
        .collect();
    write_json_partition(&base, "_index.json", &period_index)?;

    info!(
        "Exported {} timeline partitions ({} unknown)",
        by_century.len(),
        unknown.len()
    );
    Ok(())
}

/// Export edges partitioned by relation category into `v1/relations/`.
pub fn export_partitioned_relations(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base = output_dir.join("v1").join("relations");

    let mut geo: Vec<ExportEdge> = Vec::new();
    let mut temporal: Vec<ExportEdge> = Vec::new();
    let mut social: Vec<ExportEdge> = Vec::new();
    let mut relation: Vec<ExportEdge> = Vec::new();
    let mut other: Vec<ExportEdge> = Vec::new();

    for edge in &graph.edges {
        let ee = ExportEdge::from_edge(edge);
        if GEO_PROPERTIES.contains(&edge.property_id.as_str()) {
            geo.push(ee);
        } else if TEMPORAL_PROPERTIES.contains(&edge.property_id.as_str()) {
            temporal.push(ee);
        } else if SOCIAL_PROPERTIES.contains(&edge.property_id.as_str()) {
            social.push(ee);
        } else if RELATION_PROPERTIES.contains(&edge.property_id.as_str()) {
            relation.push(ee);
        } else {
            other.push(ee);
        }
    }

    let categories: Vec<(&str, &[ExportEdge])> = vec![
        ("geographic", &geo),
        ("temporal", &temporal),
        ("social", &social),
        ("relations", &relation),
        ("other", &other),
    ];

    for (name, edges) in &categories {
        write_json_partition(&base, &format!("{}.json", name), edges)?;
    }

    let all: Vec<ExportEdge> = graph.edges.iter().map(ExportEdge::from_edge).collect();
    write_json_partition(&base, "all.json", &all)?;

    info!(
        "Exported {} relation partitions ({} total edges)",
        categories.len(),
        all.len()
    );
    Ok(())
}

/// Export all partitioned views. Creates `v1/schema.json` and standardized HGJ files.
pub fn export_all_partitions(graph: &HyperGraph, output_dir: &Path, country: &CountryConfig) -> Result<()> {
    let v1_dir = output_dir.join("v1");
    if v1_dir.exists() {
        let _ = fs::remove_dir_all(&v1_dir);
    }
    fs::create_dir_all(&v1_dir)?;

    // Standardized Minidi-HGJ tree
    export_minidi_hgj(graph, output_dir, country)?;

    // Entity type partitions
    export_partitioned_by_type(graph, output_dir)?;

    // Timeline partitions (by century)
    export_partitioned_by_century(graph, output_dir)?;

    // Relation partitions
    export_partitioned_relations(graph, output_dir)?;

    // Schema descriptor
    let schema = serde_json::json!({
        "version": "1",
        "description": "Generic hypergraph schema — flat nodes + edges with compact field names",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "entity_count": graph.node_count(),
        "edge_count": graph.edge_count(),
        "fields": {
            "nodes": {
                "id": "WikiData Q-id",
                "l": "English label",
                "ll": "Local/native label (optional)",
                "d": "English description",
                "dl": "Local description (optional)",
                "t": "Entity type: place/person/event/concept/organization/artifact/other",
                "u": "WikiData URL",
                "m": "Metadata map (coordinates, dates, occupations)"
            },
            "edges": {
                "s": "Source node Q-id",
                "r": "Relation/property label",
                "t": "Target node Q-id",
                "tl": "Target node English label"
            }
        },
        "partitions": {
            "entities": "Split by NodeType",
            "timeline": "Split by century bucket",
            "relations": "Split by property category"
        },
    });

    let schema_path = v1_dir.join("schema.json");
    let schema_bytes = serde_json::to_vec_pretty(&schema)?;
    fs::write(&schema_path, &schema_bytes)?;
    info!(
        "Wrote schema descriptor: {} ({:.2} KB)",
        schema_path.display(),
        schema_bytes.len() as f64 / 1024.0
    );

    // Version alias
    let version_dir = output_dir.join("version");
    fs::create_dir_all(&version_dir)?;
    let latest_redirect = serde_json::json!({
        "latest": "v1",
        "versions": ["v1"],
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });
    write_json_partition(&version_dir, "latest.json", &latest_redirect)?;

    info!("All partitioned exports complete");
    Ok(())
}

/// Generate a small sample export (first N nodes) for quick testing.
pub fn export_sample(graph: &HyperGraph, output_dir: &Path, max_nodes: usize) -> Result<()> {
    let mut sample_graph = HyperGraph::new();
    let mut count = 0;

    for node in graph.nodes.values() {
        if count >= max_nodes {
            break;
        }
        sample_graph.add_node(node.clone());
        count += 1;
    }

    // Use a minimal country config for sample export
    let dummy_country = CountryConfig {
        code: "sample".into(),
        name: "Sample".into(),
        qid: "Q000".into(),
        language: "en".into(),
        language_name: "English".into(),
        repo: "minidi-sample-data".into(),
        native_label: false,
        wikipedia: None,
        sparql_endpoint: None,
        partition_file: None,
        partitions: None,
        crawl_settings: None,
    };

    export_graph_to_json(&sample_graph, output_dir, &dummy_country)?;
    info!("Exported sample with {} nodes", count);
    Ok(())
}

/// Export the graph to the standardized Minidi-HGJ format tree.
pub fn export_minidi_hgj(
    graph: &HyperGraph,
    output_dir: &Path,
    country: &CountryConfig,
) -> Result<()> {
    let entities_dir = output_dir.join("v1").join("entities");
    fs::create_dir_all(&entities_dir).context("Failed to create entities dir")?;

    let mut global_index = HashMap::new();
    let mut dir_catalogs: HashMap<String, HashMap<String, String>> = HashMap::new();

    // Redundant Index Buckets
    let mut domain_buckets: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    let mut timeline_buckets: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

    let uuid_namespace = uuid::Uuid::NAMESPACE_DNS;
    let now = chrono::Utc::now().to_rfc3339();

    for node in graph.nodes.values() {
        let node_uuid = uuid::Uuid::new_v5(&uuid_namespace, node.id.as_bytes()).to_string();
        let part_folder = node_uuid[0..2].to_string();
        let file_name = format!("{}.json", node_uuid);

        let part_dir = entities_dir.join(&part_folder);
        fs::create_dir_all(&part_dir)?;

        // Map Node ID to relative path in global index.json
        let rel_path = format!("v1/entities/{}/{}", part_folder, file_name);
        global_index.insert(node.id.clone(), rel_path);

        // Map Node ID to local filename in index_ids.json
        dir_catalogs
            .entry(part_folder.clone())
            .or_default()
            .insert(node.id.clone(), file_name.clone());

        // Resolve domain name
        let domain = node
            .metadata
            .get("domain")
            .cloned()
            .unwrap_or_else(|| {
                if node.id.starts_with("topic:math:") {
                    "math".to_string()
                } else if node.id.starts_with("topic:coding:") {
                    "coding".to_string()
                } else if node.id.starts_with("topic:science:") {
                    "science".to_string()
                } else {
                    "general".to_string()
                }
            });

        // 1. Build the meta object
        let meta = serde_json::json!({
            "format": "Minidi-HGJ",
            "version": "1.0.0",
            "license": "CC-BY-SA-4.0",
            "trust_tier": 5,
            "source_repo": format!("https://github.com/minidivn/{}", country.repo),
            "created_at": now,
            "updated_at": now,
        });

        // 2. Build labels and descriptions multilingual maps
        let mut labels = HashMap::new();
        labels.insert("en".to_string(), node.label.clone());
        if let Some(ref local) = node.label_local {
            labels.insert(country.language.clone(), local.clone());
        }

        let mut descriptions = HashMap::new();
        descriptions.insert("en".to_string(), node.description.clone());
        if let Some(ref local) = node.description_local {
            descriptions.insert(country.language.clone(), local.clone());
        }

        // Parse content
        let mut latex = Vec::new();
        if let Some(latex_prop) = node.metadata.get("latex") {
            latex.push(latex_prop.clone());
        }
        let content = serde_json::json!({
            "markdown": node.metadata.get("summary").cloned().unwrap_or_else(|| {
                node.description_local.clone().unwrap_or(node.description.clone())
            }),
            "latex": latex,
            "code_blocks": []
        });

        let topic = serde_json::json!({
            "id": node.id,
            "type": format!("{:?}", node.node_type).to_lowercase(),
            "domain": domain,
            "labels": labels,
            "descriptions": descriptions,
            "content": content,
            "properties": node.metadata,
        });

        // 3. Build local graph neighborhood (1-hop neighbors)
        let mut neighbor_nodes = HashMap::new();
        let mut hyperedges = Vec::new();

        // Direct outgoing edges
        let outgoing = graph.edges_from(&node.id);
        for edge in outgoing {
            if let Some(target_node) = graph.get_node(&edge.target) {
                let mut neighbor_labels = HashMap::new();
                neighbor_labels.insert("en".to_string(), target_node.label.clone());
                if let Some(ref local) = target_node.label_local {
                    neighbor_labels.insert(country.language.clone(), local.clone());
                }

                neighbor_nodes.insert(
                    edge.target.clone(),
                    serde_json::json!({
                        "type": format!("{:?}", target_node.node_type).to_lowercase(),
                        "labels": neighbor_labels,
                    }),
                );
            }

            // Convert binary edge to a HGJ hyperedge
            let h_edge = serde_json::json!({
                "id": format!("edge:{}:{}", edge.property_id, edge.target.split(':').last().unwrap_or(&edge.target)),
                "type": edge.property_label,
                "weight": 1.0,
                "members": [
                    { "node_id": node.id.clone(), "role": "subject" },
                    { "node_id": edge.target.clone(), "role": "object" }
                ]
            });
            hyperedges.push(h_edge);
        }

        // Direct incoming edges
        let incoming = graph.edges_to(&node.id);
        for edge in incoming {
            if let Some(source_node) = graph.get_node(&edge.source) {
                let mut neighbor_labels = HashMap::new();
                neighbor_labels.insert("en".to_string(), source_node.label.clone());
                if let Some(ref local) = source_node.label_local {
                    neighbor_labels.insert(country.language.clone(), local.clone());
                }

                neighbor_nodes.insert(
                    edge.source.clone(),
                    serde_json::json!({
                        "type": format!("{:?}", source_node.node_type).to_lowercase(),
                        "labels": neighbor_labels,
                    }),
                );
            }

            let h_edge = serde_json::json!({
                "id": format!("edge:{}:{}", edge.property_id, edge.source.split(':').last().unwrap_or(&edge.source)),
                "type": edge.property_label,
                "weight": 1.0,
                "members": [
                    { "node_id": edge.source.clone(), "role": "subject" },
                    { "node_id": node.id.clone(), "role": "object" }
                ]
            });
            hyperedges.push(h_edge);
        }

        let graph_section = serde_json::json!({
            "nodes": neighbor_nodes,
            "hyperedges": hyperedges
        });

        // 4. Build sources
        let mut sources = Vec::new();
        if !node.wikidata_url.is_empty() {
            sources.push(serde_json::json!({
                "id": format!("src:wikidata:{}", node.id),
                "type": "wikidata",
                "url": node.wikidata_url,
                "title": format!("WikiData Q-item: {}", node.id)
            }));
        }

        // Final Minidi-HGJ schema layout
        let hgj_payload = serde_json::json!({
            "meta": meta,
            "topic": topic,
            "graph": graph_section,
            "sources": sources,
            "attachments": []
        });

        // Write individual JSON file
        let path = part_dir.join(&file_name);
        let bytes = serde_json::to_vec_pretty(&hgj_payload)?;
        fs::write(&path, &bytes)?;

        // Populate Redundant Index Data for Domains
        let header = serde_json::json!({
            "id": node.id,
            "type": format!("{:?}", node.node_type).to_lowercase(),
            "labels": topic["labels"],
            "descriptions": topic["descriptions"]
        });
        domain_buckets.entry(domain).or_default().push(header.clone());

        // Populate Redundant Index Data for Timelines
        if let Some(year) = entity_year(node) {
            let era_slug = classify_century(year);
            timeline_buckets.entry(era_slug.to_string()).or_default().push(header);
        }
    }

    // Write all localized catalog index_ids.json files
    for (part_folder, catalog) in &dir_catalogs {
        let part_dir = entities_dir.join(part_folder);
        let path = part_dir.join("index_ids.json");
        let bytes = serde_json::to_vec_pretty(catalog)?;
        fs::write(&path, &bytes)?;
    }

    // Write global index.json
    let global_index_path = output_dir.join("v1").join("index.json");
    let bytes = serde_json::to_vec_pretty(&global_index)?;
    fs::write(&global_index_path, &bytes)?;

    // Write redundant Domain indices: v1/domains/<domain>.json
    let domains_dir = output_dir.join("v1").join("domains");
    fs::create_dir_all(&domains_dir)?;
    for (domain, nodes) in domain_buckets {
        let path = domains_dir.join(format!("{}.json", domain));
        let bytes = serde_json::to_vec_pretty(&nodes)?;
        fs::write(&path, &bytes)?;
    }

    // Write redundant Era indices: v1/timeline/<era>.json
    let timelines_dir = output_dir.join("v1").join("timeline");
    fs::create_dir_all(&timelines_dir)?;
    for (era, nodes) in timeline_buckets {
        let path = timelines_dir.join(format!("{}.json", era));
        let bytes = serde_json::to_vec_pretty(&nodes)?;
        fs::write(&path, &bytes)?;
    }

    info!(
        "[export] Standardized Minidi-HGJ export complete ({} nodes into {} partitions)",
        graph.nodes.len(),
        dir_catalogs.len()
    );
    Ok(())
}
