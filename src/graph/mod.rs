pub mod store;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of a node in the hypergraph.
/// Helps the frontend render appropriate UI (badge color, icon, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    Place,        // Cities, provinces, countries, landmarks
    Person,       // Historical figures, politicians, artists
    Event,        // Battles, treaties, festivals, founding dates
    Concept,      // Abstract: "Democracy", "Buddhism", "Lunar New Year"
    Organization, // Governments, companies, universities
    Artifact,     // Books, paintings, documents
    Other,
}

impl NodeType {
    pub fn from_string(s: &str) -> Self {
        match s {
            "place" | "Place" | "location" | "Location" => NodeType::Place,
            "person" | "Person" | "human" | "Human" => NodeType::Person,
            "event" | "Event" => NodeType::Event,
            "concept" | "Concept" => NodeType::Concept,
            "org" | "Organization" | "organization" => NodeType::Organization,
            "artifact" | "Artifact" => NodeType::Artifact,
            _ => NodeType::Other,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            NodeType::Place => "📍 Place",
            NodeType::Person => "👤 Person",
            NodeType::Event => "📅 Event",
            NodeType::Concept => "💡 Concept",
            NodeType::Organization => "🏛️ Organization",
            NodeType::Artifact => "📜 Artifact",
            NodeType::Other => "🔗 Other",
        }
    }
}

/// A single node (entity) in the hypergraph.
/// Maps to a WikiData Q-item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperNode {
    /// e.g. "Q881" (Vietnam)
    pub id: String,
    /// Primary label (English)
    pub label: String,
    /// Vietnamese label (if available)
    pub label_vi: Option<String>,
    /// Short description (English)
    pub description: String,
    /// Vietnamese description
    pub description_vi: Option<String>,
    /// Alternative names
    pub aliases: Vec<String>,
    /// Vietnamese aliases
    pub aliases_vi: Vec<String>,
    /// Entity type
    pub node_type: NodeType,
    /// Permalink
    pub wikidata_url: String,
    /// Extra metadata (coordinates, dates, etc.)
    pub metadata: HashMap<String, String>,
}

/// A binary edge connecting two nodes.
/// Maps to a WikiData P-property claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperEdge {
    /// Property ID e.g. "P150" (contains administrative division)
    pub property_id: String,
    /// Property label e.g. "contains administrative division"
    pub property_label: String,
    /// Source node Q-id
    pub source: String,
    /// Target node Q-id
    pub target: String,
    /// Target node label (denormalized for export convenience)
    pub target_label: String,
    /// Qualifiers (temporal, precision, etc.)
    pub qualifiers: HashMap<String, Vec<String>>,
}

/// A hyperedge: an n-ary relationship.
/// E.g., "Battle of Dien Bien Phu" involves {location, date, commander1, commander2, result}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperEdgeSet {
    /// Unique ID for this hyperedge set
    pub id: String,
    /// Label describing the relationship
    pub label: String,
    /// The central entity (e.g., the battle)
    pub subject: String,
    /// Roles → entity Q-ids
    pub roles: HashMap<String, Vec<String>>,
    /// Role labels (denormalized)
    pub role_labels: HashMap<String, Vec<String>>,
}

/// The complete hypergraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperGraph {
    pub nodes: HashMap<String, HyperNode>,
    pub edges: Vec<HyperEdge>,
    pub hyper_edges: Vec<HyperEdgeSet>,
}

impl HyperGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            hyper_edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: HyperNode) {
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    pub fn upsert_node(&mut self, node: HyperNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: HyperEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&HyperNode> {
        self.nodes.get(id)
    }

    /// Get all edges where this node is the source
    pub fn edges_from(&self, source: &str) -> Vec<&HyperEdge> {
        self.edges.iter().filter(|e| e.source == source).collect()
    }

    /// Get all edges where this node is the target
    pub fn edges_to(&self, target: &str) -> Vec<&HyperEdge> {
        self.edges.iter().filter(|e| e.target == target).collect()
    }

    /// Get all neighbors (both directions) for a node
    pub fn neighbors(&self, node_id: &str) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        for e in &self.edges {
            if e.source == node_id {
                ids.push(&e.target);
            } else if e.target == node_id {
                ids.push(&e.source);
            }
        }
        ids.sort();
        ids.dedup();
        ids
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for HyperGraph {
    fn default() -> Self {
        Self::new()
    }
}
