use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use tracing::info;

use crate::graph::HyperGraph;

/// Pre-computed embeddings for all nodes in the graph.
/// These are exported alongside the index so the browser can do cosine similarity
/// without downloading a model or computing embeddings on-device.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingIndex {
    /// Node ID → embedding vector
    pub vectors: HashMap<String, Vec<f32>>,
    pub dimension: usize,
}

impl EmbeddingIndex {
    /// Compute TF-IDF style bag-of-words embeddings for all nodes.
    /// This is a simple local approximation — for production, replace with
    /// actual all-MiniLM-L6-v2 embeddings using ONNX or rust-bert.
    pub fn compute_bow(graph: &HyperGraph) -> Self {
        info!(
            "Computing BOW embeddings for {} nodes...",
            graph.node_count()
        );

        // Build vocabulary from all labels + descriptions
        let mut vocab: Vec<String> = Vec::new();
        let mut word_to_idx: HashMap<String, usize> = HashMap::new();

        // Collect all words
        for node in graph.nodes.values() {
            let text = format!(
                "{} {} {} {}",
                node.label,
                node.label_vi.as_deref().unwrap_or(""),
                node.description,
                node.description_vi.as_deref().unwrap_or("")
            );
            for word in text.split_whitespace() {
                let cleaned = word
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                    .to_lowercase();
                if cleaned.len() > 2 {
                    if !word_to_idx.contains_key(&cleaned) {
                        word_to_idx.insert(cleaned.clone(), vocab.len());
                        vocab.push(cleaned.clone());
                    }
                }
            }
        }

        let dimension = vocab.len().min(256); // cap dimension for performance

        let mut vectors = HashMap::new();
        for node in graph.nodes.values() {
            let text = format!(
                "{} {} {} {}",
                node.label,
                node.label_vi.as_deref().unwrap_or(""),
                node.description,
                node.description_vi.as_deref().unwrap_or("")
            );

            let mut vec = vec![0.0_f32; dimension];
            let mut word_count = 0;
            for word in text.split_whitespace() {
                let cleaned = word
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                    .to_lowercase();
                if let Some(&idx) = word_to_idx.get(&cleaned) {
                    if idx < dimension {
                        vec[idx] += 1.0;
                        word_count += 1;
                    }
                }
            }

            // Normalize to unit vector
            if word_count > 0 {
                let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
                if mag > 0.0 {
                    for v in &mut vec {
                        *v /= mag;
                    }
                }
            }

            vectors.insert(node.id.clone(), vec);
        }

        info!(
            "BOW embeddings computed: {} vectors @ {} dim",
            vectors.len(),
            dimension
        );

        EmbeddingIndex { vectors, dimension }
    }

    /// Search by cosine similarity against a query embedding vector.
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Vec<EmbeddingResult> {
        let mut results: Vec<EmbeddingResult> = self
            .vectors
            .iter()
            .map(|(id, vec)| {
                let score = cosine_similarity(query_vec, vec);
                EmbeddingResult {
                    id: id.clone(),
                    score,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    /// Export embeddings as a flat JSON + binary file for the frontend.
    pub fn export(vectors: &HashMap<String, Vec<f32>>, dimension: usize) -> Result<Vec<u8>> {
        // Format: [dimension: u32] [count: u32] [id_len: u32] [id: bytes] [vec: f32 x dim] ...
        let mut buf = Vec::new();
        buf.extend_from_slice(&(dimension as u32).to_le_bytes());
        buf.extend_from_slice(&(vectors.len() as u32).to_le_bytes());

        for (id, vec) in vectors {
            buf.extend_from_slice(&(id.len() as u32).to_le_bytes());
            buf.extend_from_slice(id.as_bytes());
            for v in vec {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }

        Ok(buf)
    }
}

/// A search result from embedding similarity.
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub id: String,
    pub score: f32,
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot // vectors are already normalized
}
