use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;
use std::path::Path;
use tracing::{info, warn};

use crate::graph::{HyperEdge, HyperNode, NodeType};
use super::{CrawlContext, CrawlResult, DataSource, SourceSchema};

/// Source that executes an external script (Python, JS, Shell) and parses JSON output.
pub struct ScriptDataSource {
    pub name: String,
    pub description: String,
    pub command_str: String,
    pub entity_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScriptOutput {
    nodes: Vec<ScriptNode>,
    edges: Vec<ScriptEdge>,
}

#[derive(Debug, Deserialize)]
struct ScriptNode {
    id: String,
    label: String,
    label_local: Option<String>,
    description: String,
    description_local: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    aliases_local: Vec<String>,
    node_type: String,
    #[serde(default)]
    wikidata_url: String,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ScriptEdge {
    property_id: String,
    property_label: String,
    source: String,
    target: String,
    target_label: String,
}

#[async_trait]
impl DataSource for ScriptDataSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: self.name.clone(),
            description: self.description.clone(),
            version: "1.0.0".into(),
            endpoint: format!("script://{}", self.command_str),
            entity_types: self.entity_types.clone(),
            properties: vec!["custom_relation".into()],
            metadata_fields: HashMap::new(),
            attribution: "External Script Plugin".into(),
        }
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<CrawlResult> {
        info!("[script:{}] Running external crawl script: {}", self.name, self.command_str);

        // Parse command string into command and arguments
        let parts: Vec<&str> = self.command_str.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty script command configuration");
        }

        let cmd = parts[0];
        let args = &parts[1..];

        // Add context params as environment variables or args
        let mut child = Command::new(cmd);
        child.args(args);
        
        // Pass crawl configuration parameters via environment variables
        child.env("MINIDI_CRAWL_LIMIT", ctx.limit.to_string());
        if let Some(ref qid) = ctx.country_qid {
            child.env("MINIDI_COUNTRY_QID", qid);
        }
        if let Some(ref lang) = ctx.language {
            child.env("MINIDI_LANGUAGE", lang);
        }

        let output = tokio::task::block_in_place(|| {
            child.output()
        }).context("Failed to execute external script command")?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("External script failed with status {}. Stderr: {}", output.status, stderr_str);
        }

        let stdout_str = String::from_utf8(output.stdout).context("Script output is not valid UTF-8")?;
        let parsed: ScriptOutput = serde_json::from_str(&stdout_str).context("Failed to parse script output JSON")?;

        let nodes: Vec<HyperNode> = parsed.nodes.into_iter().map(|n| {
            HyperNode {
                id: n.id,
                label: n.label,
                label_local: n.label_local,
                description: n.description,
                description_local: n.description_local,
                aliases: n.aliases,
                aliases_local: n.aliases_local,
                node_type: NodeType::from_string(&n.node_type),
                wikidata_url: n.wikidata_url,
                metadata: n.metadata,
            }
        }).collect();

        let edges: Vec<HyperEdge> = parsed.edges.into_iter().map(|e| {
            HyperEdge {
                property_id: e.property_id,
                property_label: e.property_label,
                source: e.source,
                target: e.target,
                target_label: e.target_label,
                qualifiers: HashMap::new(),
            }
        }).collect();

        let mut metadata = HashMap::new();
        metadata.insert("source_command".into(), self.command_str.clone());
        metadata.insert("node_count".into(), nodes.len().to_string());
        metadata.insert("edge_count".into(), edges.len().to_string());

        Ok(CrawlResult {
            source: self.name.clone(),
            nodes,
            edges,
            metadata,
        })
    }
}
