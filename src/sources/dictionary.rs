use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::graph::{HyperEdge, HyperNode, NodeType};
use super::{CrawlContext, CrawlResult, DataSource, SourceSchema};

/// Source that parses local word lists and bilingual translation dictionaries.
pub struct DictionaryDataSource {
    /// Directory containing the country data directories.
    /// Typically "../Minidi/Data" or similar.
    pub data_parent_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct TranslationEntry {
    translations: Vec<String>,
    pos: Option<String>,
    definitions: Option<HashMap<String, String>>,
    target_nodes: Option<Vec<String>>,
}

#[async_trait]
impl DataSource for DictionaryDataSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: "dictionary".into(),
            description: "Local Dictionary Source — parses raw wordlists and bilingual dictionary bridges".into(),
            version: "1.0.0".into(),
            endpoint: "local://_data/words,local://_data/dict".into(),
            entity_types: vec!["concept".into(), "other".into()],
            properties: vec!["bridges_to".into(), "translates_to".into()],
            metadata_fields: HashMap::from([
                ("pos".into(), "Part of speech tag".into()),
                ("translations".into(), "Bilingual translation list".into()),
                ("source_list".into(), "Source wordlist filename".into()),
            ]),
            attribution: "Local dictionary assets".into(),
        }
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<CrawlResult> {
        let country_code = ctx.language.as_deref().unwrap_or("vn");
        
        // Locate output directory from configs (e.g. data/minidi-vn-data/docs/_data)
        // Since we are reading from the country repository's docs/_data/ we construct it:
        // We look for minidi-<code_code>-data or similar. We can deduce directory name:
        // Typically output_parent/minidi-<code_code>-data. Let's find it.
        let repo_dir_name = if country_code == "vi" || country_code == "vn" {
            "minidi-vn-data"
        } else {
            // fallback guess
            &format!("minidi-{}-data", country_code)
        };
        
        let docs_data_dir = self.data_parent_dir
            .join(repo_dir_name)
            .join("docs")
            .join("_data");

        info!("[dictionary] Reading dictionary assets from: {}", docs_data_dir.display());

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // 1. Process words/ subdirectory (Raw Wordlists)
        let words_dir = docs_data_dir.join("words");
        if words_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&words_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
                        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
                        info!("  [dictionary] Loading wordlist: {}", filename);
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let mut word_count = 0;
                            for line in content.lines() {
                                let word = line.trim();
                                if word.is_empty() {
                                    continue;
                                }
                                // Limit wordlist ingestion count to prevent database inflation
                                word_count += 1;
                                if word_count > 500 { // Max 500 words parsed per list as concepts in spider limit
                                    break;
                                }
                                let id = format!("word:{}:{}", country_code, word.replace(' ', "_"));
                                let mut metadata = HashMap::new();
                                metadata.insert("source_list".into(), filename.clone());
                                metadata.insert("word_string".into(), word.to_string());
                                
                                nodes.push(HyperNode {
                                    id,
                                    label: word.to_string(),
                                    label_local: Some(word.to_string()),
                                    description: format!("Vocabulary entry in {}", filename),
                                    description_local: Some(format!("Từ vựng trong danh sách {}", filename)),
                                    aliases: Vec::new(),
                                    aliases_local: Vec::new(),
                                    node_type: NodeType::Other,
                                    wikidata_url: "".into(),
                                    metadata,
                                });
                            }
                        }
                    }
                }
            }
        } else {
            warn!("  [dictionary] Word directory not found: {}", words_dir.display());
        }

        // 2. Process dict/ subdirectory (Translation Pairs)
        let dict_dir = docs_data_dir.join("dict");
        if dict_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dict_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
                        info!("  [dictionary] Loading translation dict: {}", filename);
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(entries) = serde_json::from_str::<HashMap<String, TranslationEntry>>(&content) {
                                for (word, entry) in entries {
                                    let node_id = format!("dict:{}:{}", country_code, word.replace(' ', "_"));
                                    
                                    let mut metadata = HashMap::new();
                                    if let Some(ref pos) = entry.pos {
                                        metadata.insert("pos".into(), pos.clone());
                                    }
                                    metadata.insert("translations".into(), entry.translations.join(", "));

                                    let vi_desc = entry.definitions.as_ref()
                                        .and_then(|defs| defs.get("vi"))
                                        .cloned();
                                    let en_desc = entry.definitions.as_ref()
                                        .and_then(|defs| defs.get("en"))
                                        .cloned();

                                    nodes.push(HyperNode {
                                        id: node_id.clone(),
                                        label: word.clone(),
                                        label_local: Some(word.clone()),
                                        description: en_desc.unwrap_or_else(|| format!("Translation entry for: {}", word)),
                                        description_local: vi_desc,
                                        aliases: entry.translations.clone(),
                                        aliases_local: Vec::new(),
                                        node_type: NodeType::Concept,
                                        wikidata_url: "".into(),
                                        metadata,
                                    });

                                    // Create bridges (edges) to target nodes
                                    if let Some(targets) = entry.target_nodes {
                                        for target_id in targets {
                                            edges.push(HyperEdge {
                                                property_id: "bridges_to".into(),
                                                property_label: "bridges to".into(),
                                                source: node_id.clone(),
                                                target: target_id.clone(),
                                                target_label: target_id.split(':').last().unwrap_or(&target_id).replace('_', " "),
                                                qualifiers: HashMap::new(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            warn!("  [dictionary] Dictionary directory not found: {}", dict_dir.display());
        }

        let mut metadata = HashMap::new();
        metadata.insert("source".into(), "local_dictionary".into());
        metadata.insert("node_count".into(), nodes.len().to_string());
        metadata.insert("edge_count".into(), edges.len().to_string());

        Ok(CrawlResult {
            source: "dictionary".into(),
            nodes,
            edges,
            metadata,
        })
    }
}
