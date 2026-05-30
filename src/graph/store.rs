use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

use super::{HyperEdge, HyperGraph, HyperNode};

const NODE_PREFIX: &str = "node_";
const EDGE_KEY: &str = "edges";
const META_KEY: &str = "graph_meta";

/// Persistent hypergraph store backed by Sled (embedded database).
pub struct HyperGraphStore {
    db: Db,
}

impl HyperGraphStore {
    /// Open (or create) a database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path).context("Failed to open sled database")?;
        Ok(Self { db })
    }

    /// Insert or update a node.
    pub fn put_node(&self, node: &HyperNode) -> Result<()> {
        let key = format!("{}{}", NODE_PREFIX, node.id);
        let value = bincode::serialize(node).context("Failed to serialize node")?;
        self.db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// Get a node by its Q-id.
    pub fn get_node(&self, id: &str) -> Result<Option<HyperNode>> {
        let key = format!("{}{}", NODE_PREFIX, id);
        match self.db.get(key.as_bytes())? {
            Some(ivec) => {
                let node: HyperNode =
                    bincode::deserialize(&ivec).context("Failed to deserialize node")?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Check if a node exists.
    pub fn has_node(&self, id: &str) -> Result<bool> {
        let key = format!("{}{}", NODE_PREFIX, id);
        Ok(self.db.contains_key(key.as_bytes())?)
    }

    /// Add an edge to the edge list.
    pub fn put_edge(&self, edge: &HyperEdge) -> Result<()> {
        let mut edges = self.get_all_edges()?;
        edges.push(edge.clone());
        let value = bincode::serialize(&edges).context("Failed to serialize edges")?;
        self.db.insert(EDGE_KEY.as_bytes(), value)?;
        Ok(())
    }

    /// Remove all edges and replace with given list.
    pub fn set_edges(&self, edges: &[HyperEdge]) -> Result<()> {
        let value = bincode::serialize(edges).context("Failed to serialize edges")?;
        self.db.insert(EDGE_KEY.as_bytes(), value)?;
        Ok(())
    }

    /// Retrieve all edges.
    pub fn get_all_edges(&self) -> Result<Vec<HyperEdge>> {
        match self.db.get(EDGE_KEY.as_bytes())? {
            Some(ivec) => {
                let edges: Vec<HyperEdge> =
                    bincode::deserialize(&ivec).context("Failed to deserialize edges")?;
                Ok(edges)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Iterate over all nodes in the store.
    pub fn iter_nodes(&self) -> impl Iterator<Item = Result<HyperNode>> + '_ {
        self.db
            .scan_prefix(NODE_PREFIX.as_bytes())
            .filter_map(|res| match res {
                Ok((_, value)) => match bincode::deserialize(&value) {
                    Ok(node) => Some(Ok(node)),
                    Err(e) => Some(Err(anyhow::anyhow!("Deserialize error: {}", e))),
                },
                Err(e) => Some(Err(anyhow::anyhow!("Sled error: {}", e))),
            })
    }

    /// Count all nodes.
    pub fn node_count(&self) -> usize {
        self.db.scan_prefix(NODE_PREFIX.as_bytes()).count()
    }

    /// Store crawl metadata (JSON blob).
    pub fn put_meta(&self, meta: &str) -> Result<()> {
        self.db.insert(META_KEY.as_bytes(), meta.as_bytes())?;
        Ok(())
    }

    pub fn get_meta(&self) -> Result<Option<String>> {
        match self.db.get(META_KEY.as_bytes())? {
            Some(ivec) => Ok(Some(
                String::from_utf8(ivec.to_vec()).context("Meta is not valid UTF-8")?,
            )),
            None => Ok(None),
        }
    }

    /// Load the entire graph into memory (for export / indexing).
    pub fn load_full_graph(&self) -> Result<HyperGraph> {
        let mut graph = HyperGraph::new();
        for node_res in self.iter_nodes() {
            let node = node_res?;
            graph.upsert_node(node);
        }
        let edges = self.get_all_edges()?;
        for edge in edges {
            graph.add_edge(edge);
        }
        Ok(graph)
    }

    /// Batch insert many nodes (fast path).
    pub fn put_nodes_batch(&self, nodes: &[HyperNode]) -> Result<()> {
        let mut batch = sled::Batch::default();
        for node in nodes {
            let key = format!("{}{}", NODE_PREFIX, node.id);
            let value = bincode::serialize(node).context("Failed to serialize node")?;
            batch.insert(key.as_bytes(), value);
        }
        self.db.apply_batch(batch)?;
        Ok(())
    }

    /// Flush to disk.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
