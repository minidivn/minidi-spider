use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

use crate::graph::{HyperEdge, HyperGraph, HyperNode, NodeType};

/// Vietnamese historical eras with date boundaries.
const VIETNAM_ERAS: &[(&str, &str, &str, i64, i64)] = &[
    (
        "paleolithic",
        "Paleolithic",
        "~50,000 – 3000 BCE",
        -50000,
        -3000,
    ),
    ("hong-bang", "Hồng Bàng", "2879 – 258 BCE", -2879, -258),
    (
        "chinese-domination",
        "Chinese Domination",
        "111 BCE – 939 CE",
        -111,
        939,
    ),
    ("dynastic-vn", "Dynastic Vietnam", "939 – 1858", 939, 1858),
    ("colonial", "Colonial", "1858 – 1954", 1858, 1954),
    ("vietnam-war", "Vietnam War", "1955 – 1975", 1955, 1975),
    ("modern", "Modern", "1975 – present", 1975, i64::MAX),
];

/// Property IDs grouped by relation category for partitioned export.
const ADMIN_PROPERTIES: &[&str] = &["P150", "P131", "P17", "P36", "P47"];
const GEO_PROPERTIES: &[&str] = &["P625", "P2044", "P2046", "P402", "P421"];
const TEMPORAL_PROPERTIES: &[&str] = &["P580", "P582", "P585", "P569", "P570", "P571"];
const SOCIAL_PROPERTIES: &[&str] = &[
    "P106", "P69", "P1416", "P27", "P19", "P22", "P25", "P26", "P3373",
];

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
    pub entity_count: usize,
    pub edge_count: usize,
    pub search_fields: Vec<&'static str>,
    pub query_endpoint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ExportNode {
    pub id: String,
    pub l: String,                                    // label
    pub lv: Option<String>,                           // label_vi
    pub d: String,                                    // description
    pub dv: Option<String>,                           // description_vi
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
            lv: node.label_vi.clone(),
            d: node.description.clone(),
            dv: node.description_vi.clone(),
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
pub fn export_graph_to_json(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let nodes: Vec<ExportNode> = graph.nodes.values().map(ExportNode::from_node).collect();
    let edges: Vec<ExportEdge> = graph.edges.iter().map(ExportEdge::from_edge).collect();

    let index = ExportIndex {
        meta: ExportMeta {
            crawl_version: env!("CARGO_PKG_VERSION"),
            crawled_at: chrono::Utc::now().to_rfc3339(),
            entity_count: nodes.len(),
            edge_count: edges.len(),
            search_fields: &["label", "label_vi", "description", "aliases"],
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
        export_graph_to_json(graph, output_dir)?;
    }

    // Create a smaller "lite" version without full descriptions for mobile
    #[derive(Serialize)]
    struct LiteNode {
        id: String,
        l: String,
        lv: Option<String>,
        t: String,
        u: String,
    }

    let nodes: Vec<LiteNode> = graph
        .nodes
        .values()
        .map(|n| LiteNode {
            id: n.id.clone(),
            l: n.label.clone(),
            lv: n.label_vi.clone(),
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

    info!("Exported {} entity type partitions", by_type.len());
    Ok(())
}

/// Classify a year into a Vietnamese historical era slug.
fn classify_era(year: i64) -> &'static str {
    for (slug, _, _, start, end) in VIETNAM_ERAS {
        if year >= *start && year <= *end {
            return slug;
        }
    }
    "unknown"
}

/// Parse a year from a WikiData date string (ISO 8601 or similar).
fn parse_year(datestr: &str) -> Option<i64> {
    // Try ISO 8601: 1858-01-01T00:00:00Z or 1858
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(datestr, "%Y-%m-%d") {
        return Some(dt.year() as i64);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datestr, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.year() as i64);
    }
    // Bare year
    if let Ok(year) = datestr.parse::<i64>() {
        return Some(year);
    }
    // BCE dates: -2879-01-01 etc
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(datestr, "%-Y-%m-%d") {
        return Some(dt.year() as i64);
    }
    None
}

/// Get the best year estimate for an entity from its metadata.
fn entity_year(node: &HyperNode) -> Option<i64> {
    // Priority: point_in_time > start_time > birth_date > death_date
    for key in &["point_in_time", "start_time", "birth_date", "death_date"] {
        if let Some(val) = node.metadata.get(*key) {
            if let Some(year) = parse_year(val) {
                return Some(year);
            }
        }
    }
    None
}

/// Export entities partitioned by historical era into `v1/timeline/`.
pub fn export_partitioned_by_era(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base = output_dir.join("v1").join("timeline");

    let mut by_era: HashMap<&str, Vec<ExportNode>> = HashMap::new();
    let mut unknown: Vec<ExportNode> = Vec::new();

    for node in graph.nodes.values() {
        let en = ExportNode::from_node(node);
        match entity_year(node) {
            Some(year) => {
                let slug = classify_era(year);
                by_era.entry(slug).or_default().push(en);
            }
            None => unknown.push(en),
        }
    }

    // Write known eras
    for (slug, nodes) in &by_era {
        // Find the era display info
        let label = VIETNAM_ERAS
            .iter()
            .find(|e| e.0 == *slug)
            .map(|e| e.1)
            .unwrap_or(slug);
        let range = VIETNAM_ERAS
            .iter()
            .find(|e| e.0 == *slug)
            .map(|e| e.2)
            .unwrap_or("");

        let payload = serde_json::json!({
            "era": slug,
            "label": label,
            "date_range": range,
            "entity_count": nodes.len(),
            "entities": nodes,
        });
        write_json_partition(&base, &format!("era-{}.json", slug), &payload)?;
    }

    // Write unknown
    if !unknown.is_empty() {
        let payload = serde_json::json!({
            "era": "unknown",
            "label": "Unclassified",
            "date_range": "No temporal data",
            "entity_count": unknown.len(),
            "entities": unknown,
        });
        write_json_partition(&base, "era-unknown.json", &payload)?;
    }

    // Write all-eras index
    let era_index: Vec<serde_json::Value> = VIETNAM_ERAS
        .iter()
        .map(|(slug, label, range, _, _)| {
            let count = by_era.get(slug).map(|v| v.len()).unwrap_or(0);
            serde_json::json!({
                "slug": slug,
                "label": label,
                "date_range": range,
                "entity_count": count,
            })
        })
        .collect();
    write_json_partition(&base, "all.json", &era_index)?;

    info!(
        "Exported {} timeline era partitions ({} unknown)",
        by_era.len(),
        unknown.len()
    );
    Ok(())
}

/// Export edges partitioned by relation category into `v1/relations/`.
pub fn export_partitioned_relations(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let base = output_dir.join("v1").join("relations");

    let mut admin: Vec<ExportEdge> = Vec::new();
    let mut geo: Vec<ExportEdge> = Vec::new();
    let mut temporal: Vec<ExportEdge> = Vec::new();
    let mut social: Vec<ExportEdge> = Vec::new();
    let mut other: Vec<ExportEdge> = Vec::new();

    for edge in &graph.edges {
        let ee = ExportEdge::from_edge(edge);
        if ADMIN_PROPERTIES.contains(&edge.property_id.as_str()) {
            admin.push(ee);
        } else if GEO_PROPERTIES.contains(&edge.property_id.as_str()) {
            geo.push(ee);
        } else if TEMPORAL_PROPERTIES.contains(&edge.property_id.as_str()) {
            temporal.push(ee);
        } else if SOCIAL_PROPERTIES.contains(&edge.property_id.as_str()) {
            social.push(ee);
        } else {
            other.push(ee);
        }
    }

    let categories: Vec<(&str, &[ExportEdge])> = vec![
        ("administrative", &admin),
        ("geographic", &geo),
        ("temporal", &temporal),
        ("social", &social),
        ("other", &other),
    ];

    for (name, edges) in &categories {
        write_json_partition(&base, &format!("{}.json", name), edges)?;
    }

    // All edges combined
    let all: Vec<ExportEdge> = graph.edges.iter().map(ExportEdge::from_edge).collect();
    write_json_partition(&base, "all.json", &all)?;

    info!(
        "Exported {} relation partitions ({} total edges)",
        categories.len(),
        all.len()
    );
    Ok(())
}

/// Export all partitioned views. Creates `v1/schema.json`.
pub fn export_all_partitions(graph: &HyperGraph, output_dir: &Path) -> Result<()> {
    let v1_dir = output_dir.join("v1");
    fs::create_dir_all(&v1_dir)?;

    // Entity type partitions
    export_partitioned_by_type(graph, output_dir)?;

    // Timeline partitions
    export_partitioned_by_era(graph, output_dir)?;

    // Relation partitions
    export_partitioned_relations(graph, output_dir)?;

    // Schema descriptor
    let schema = serde_json::json!({
        "version": "1",
        "description": "Initial schema — flat nodes + edges with compact field names",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "entity_count": graph.node_count(),
        "edge_count": graph.edge_count(),
        "fields": {
            "nodes": {
                "id": "WikiData Q-id",
                "l": "English label",
                "lv": "Vietnamese label (optional)",
                "d": "English description",
                "dv": "Vietnamese description (optional)",
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
            "timeline": "Split by Vietnamese historical era",
            "relations": "Split by property category"
        },
        "eras": VIETNAM_ERAS.iter().map(|(slug, label, range, _, _)| {
            serde_json::json!({"slug": slug, "label": label, "date_range": range})
        }).collect::<Vec<_>>(),
    });

    let schema_path = v1_dir.join("schema.json");
    let schema_bytes = serde_json::to_vec_pretty(&schema)?;
    fs::write(&schema_path, &schema_bytes)?;
    info!(
        "Wrote schema descriptor: {} ({:.2} KB)",
        schema_path.display(),
        schema_bytes.len() as f64 / 1024.0
    );

    // Symlink-like latest version redirect (JSON file with target)
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

    export_graph_to_json(&sample_graph, output_dir)?;
    info!("Exported sample with {} nodes", count);
    Ok(())
}
