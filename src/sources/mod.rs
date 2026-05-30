use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

use crate::graph::{HyperEdge, HyperNode};

pub mod openstreetmap;
pub mod wikidata;
pub mod wikipedia;

/// Schema descriptor for a data source.
/// Tells the frontend what fields to expect and how to render them.
#[derive(Debug, Clone, Serialize)]
pub struct SourceSchema {
    /// Short name: "wikidata", "wikipedia", "openstreetmap"
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Schema version for this source
    pub version: String,
    /// URL/endpoint this source crawls from
    pub endpoint: String,
    /// Entity types this source produces
    pub entity_types: Vec<String>,
    /// Property/relation types this source produces
    pub properties: Vec<String>,
    /// Available metadata fields on entities
    pub metadata_fields: HashMap<String, String>,
    /// Attribution / license info
    pub attribution: String,
}

/// Result from a single source crawl.
#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub source: String,
    pub nodes: Vec<HyperNode>,
    pub edges: Vec<HyperEdge>,
    pub metadata: HashMap<String, String>,
}

/// Context shared across all sources during a crawl.
pub struct CrawlContext {
    pub client: reqwest::Client,
    pub limit: usize,
    pub progress: bool,
    pub user_agent: String,
    /// Optional named partition to restrict crawling to (e.g. "adm-north-provinces").
    /// None = crawl everything the source provides.
    pub partition: Option<String>,
}

impl CrawlContext {
    pub fn new(limit: usize, progress: bool, partition: Option<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!(
                "MinidiSpider/0.1 (+https://github.com/minidivn/minidi-spider)"
            ))
            .timeout(std::time::Duration::from_secs(120))
            .gzip(true)
            .brotli(true)
            .build()?;
        Ok(Self {
            client,
            limit,
            progress,
            partition,
            user_agent: format!("MinidiSpider/0.1"),
        })
    }
}

/// Core trait all data sources must implement.
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Return the schema descriptor
    fn schema(&self) -> SourceSchema;

    /// Execute a crawl. Returns nodes + edges.
    /// `ctx` provides the HTTP client, rate limiting, and progress tracking.
    async fn crawl(&self, ctx: &CrawlContext) -> Result<CrawlResult>;
}

/// Registry of all available sources.
pub struct SourceRegistry {
    sources: Vec<Box<dyn DataSource>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Register a source implementation.
    pub fn register(&mut self, source: Box<dyn DataSource>) {
        self.sources.push(source);
    }

    /// Get all registered sources.
    pub fn all(&self) -> &[Box<dyn DataSource>] {
        &self.sources
    }

    /// Find a source by name.
    pub fn get(&self, name: &str) -> Option<&Box<dyn DataSource>> {
        self.sources.iter().find(|s| s.schema().name == name)
    }

    /// Run all registered sources and collect results.
    pub async fn run_all(&self, ctx: &CrawlContext) -> Vec<(SourceSchema, Result<CrawlResult>)> {
        let mut results = Vec::new();
        for source in &self.sources {
            let schema = source.schema();
            tracing::info!("Crawling source: {}", schema.name);
            let result = source.crawl(ctx).await;
            results.push((schema, result));
        }
        results
    }

    /// Collect all schemas for frontend metadata export.
    pub fn schemas(&self) -> Vec<SourceSchema> {
        self.sources.iter().map(|s| s.schema()).collect()
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
