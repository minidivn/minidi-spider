use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Per-country partition definition loaded from external file.
#[derive(Debug, Clone, Deserialize)]
pub struct PartitionConfig {
    /// Partition name (e.g. "adm-north-provinces", "people-writers")
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Category grouping (admin, history, people, culture, nature, economy, sports)
    pub category: Option<String>,
    /// Path to .sparql file (relative to query_dir, or absolute)
    pub query_file: Option<String>,
    /// Inline SPARQL query (alternative to query_file)
    pub query_inline: Option<String>,
    /// SPARQL FILTER snippet injected into {FILTER} placeholder
    #[serde(default)]
    pub filter: String,
    /// Post-fetch filters (has_coords, min_population, etc.)
    pub filters: Option<HashMap<String, serde_json::Value>>,
    /// Expected node type for this partition
    pub node_type: String,
    /// Max entities to fetch
    #[serde(default = "default_partition_limit")]
    pub limit: usize,
}

fn default_partition_limit() -> usize {
    5000
}

/// Wikipedia API configuration for a country.
#[derive(Debug, Clone, Deserialize)]
pub struct WikipediaConfig {
    pub api: String,
    pub language: String,
}

/// Crawl behavior settings.
#[derive(Debug, Clone, Deserialize)]
pub struct CrawlSettings {
    #[serde(default = "default_max_edges")]
    pub max_edges_per_node: usize,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_ms: u64,
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
    #[serde(default = "default_depth")]
    pub link_traversal_depth: u32,
    #[serde(default = "default_max_articles")]
    pub max_wikipedia_articles: usize,
}

fn default_max_edges() -> usize {
    50
}
fn default_rate_limit() -> u64 {
    200
}
fn default_timeout() -> u64 {
    120
}
fn default_retries() -> u32 {
    3
}
fn default_depth() -> u32 {
    2
}
fn default_max_articles() -> usize {
    500_000
}

/// A single country configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CountryConfig {
    pub code: String,
    pub name: String,
    pub qid: String,
    pub language: String,
    pub language_name: String,
    pub repo: String,
    #[serde(default = "default_native_label")]
    pub native_label: bool,
    pub wikipedia: Option<WikipediaConfig>,
    pub sparql_endpoint: Option<String>,
    /// Reference to external partition file (e.g. "vn.json", "en.json")
    pub partition_file: Option<String>,
    /// Inline partitions (alternative to partition_file)
    pub partitions: Option<Vec<PartitionConfig>>,
    pub crawl_settings: Option<CrawlSettings>,
}

fn default_native_label() -> bool {
    true
}

/// Root config file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct CountriesConfig {
    pub comment: Option<String>,
    pub default_country: String,
    pub output_parent: String,
    /// Directory containing partition .json files (default: "configs/partitions")
    #[serde(default = "default_partition_dir")]
    pub partition_dir: String,
    /// Directory containing .sparql query files (default: "configs/queries")
    #[serde(default = "default_query_dir")]
    pub query_dir: String,
    pub default_crawl_settings: Option<CrawlSettings>,
    pub countries: Vec<CountryConfig>,
}

fn default_partition_dir() -> String {
    "configs/partitions".to_string()
}
fn default_query_dir() -> String {
    "configs/queries".to_string()
}

impl CountriesConfig {
    pub fn load() -> Result<Self> {
        Self::from_file("configs/countries.json")
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read countries config file")?;
        let config: CountriesConfig =
            serde_json::from_str(&content).context("Failed to parse countries config")?;
        Ok(config)
    }

    pub fn get(&self, code: &str) -> Option<&CountryConfig> {
        self.countries.iter().find(|c| c.code == code)
    }

    pub fn default(&self) -> Option<&CountryConfig> {
        self.get(&self.default_country)
    }

    pub fn output_path(&self, code: &str) -> Result<std::path::PathBuf> {
        let country = self.get(code).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown country code: {}. Check configs/countries.json",
                code
            )
        })?;
        let parent = Path::new(&self.output_parent);
        Ok(parent.join(&country.repo))
    }

    pub fn label_languages(&self, code: &str) -> Vec<String> {
        let country = self.get(code);
        match country {
            Some(c) if c.native_label => vec!["en".to_string(), c.language.clone()],
            _ => vec!["en".to_string()],
        }
    }

    pub fn all_codes(&self) -> Vec<&str> {
        self.countries.iter().map(|c| c.code.as_str()).collect()
    }

    /// Get partitions for a country, loading from external file if configured.
    pub fn get_partitions(&self, code: &str) -> Vec<PartitionConfig> {
        let country = match self.get(code) {
            Some(c) => c,
            None => return vec![],
        };

        // 1. Try external partition file
        if let Some(ref pf) = country.partition_file {
            let path = Path::new(&self.partition_dir).join(pf);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(parts) = serde_json::from_str::<Vec<PartitionConfig>>(&content) {
                    return parts;
                }
                tracing::warn!("Failed to parse partition file: {}", path.display());
            } else {
                tracing::warn!("Partition file not found: {}", path.display());
            }
        }

        // 2. Try inline partitions
        if let Some(ref parts) = country.partitions {
            return parts.clone();
        }

        // 3. Fallback: 3 generic partitions
        vec![
            PartitionConfig {
                name: "places".into(),
                description: None,
                category: None,
                query_file: Some("shared/places-filtered.sparql".into()),
                query_inline: None,
                filter: String::new(),
                filters: None,
                node_type: "Place".into(),
                limit: 5000,
            },
            PartitionConfig {
                name: "people".into(),
                description: None,
                category: None,
                query_file: Some("shared/people-filtered.sparql".into()),
                query_inline: None,
                filter: String::new(),
                filters: None,
                node_type: "Person".into(),
                limit: 5000,
            },
            PartitionConfig {
                name: "events".into(),
                description: None,
                category: None,
                query_file: Some("shared/events-filtered.sparql".into()),
                query_inline: None,
                filter: String::new(),
                filters: None,
                node_type: "Event".into(),
                limit: 3000,
            },
        ]
    }

    /// Resolve SPARQL query content, substitute all placeholders.
    pub fn resolve_partition_query(
        &self,
        partition: &PartitionConfig,
        country_qid: &str,
        lang: &str,
        limit: usize,
    ) -> Result<String> {
        let raw = self.load_query_raw(partition)?;

        // Substitute placeholders
        let mut q = raw
            .replace("{COUNTRY_QID}", country_qid)
            .replace("{LANG}", lang)
            .replace("{LIMIT}", &limit.to_string());

        // Handle {FILTER}: if partition has filter string, inject it; else comment it out
        if !partition.filter.is_empty() {
            // Extract OCCUPATION_QID from filter comments if present
            let filter_with_qid = if partition.filter.contains("OCCUPATION_QID=") {
                // This is a people-by-occupation query — extract QID
                if let Some(qid_start) = partition.filter.find("Q") {
                    let qid_str = &partition.filter[qid_start..];
                    let qid_end = qid_str
                        .find(|c: char| !c.is_alphanumeric())
                        .unwrap_or(qid_str.len());
                    let occupation_qid = &qid_str[..qid_end];
                    q = q.replace("{OCCUPATION_QID}", occupation_qid);
                }
                // Remove the comment line from filter, keep only actual SPARQL
                partition
                    .filter
                    .lines()
                    .filter(|l| !l.trim().starts_with('#'))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                partition.filter.clone()
            };

            q = q.replace("{FILTER}", &filter_with_qid);
        } else {
            // No filter: comment out the {FILTER} line
            q = q.replace("{FILTER}", "# (no filter)");
        }

        Ok(q)
    }

    /// Load the raw SPARQL query text from file or inline.
    fn load_query_raw(&self, partition: &PartitionConfig) -> Result<String> {
        if let Some(ref inline) = partition.query_inline {
            return Ok(inline.clone());
        }

        if let Some(ref qf) = partition.query_file {
            // Try multiple locations in order:
            let candidates = vec![
                Path::new(qf).to_path_buf(),         // absolute or cwd-relative
                Path::new(&self.query_dir).join(qf), // configs/queries/<qf>
                Path::new(&self.query_dir).join("shared").join(qf), // configs/queries/shared/<qf>
            ];

            for candidate in &candidates {
                if candidate.exists() {
                    return std::fs::read_to_string(candidate).context(format!(
                        "Failed to read query file: {}",
                        candidate.display()
                    ));
                }
            }

            anyhow::bail!(
                "Query file not found: '{}' (tried: {})",
                qf,
                candidates
                    .iter()
                    .map(|c| c.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        anyhow::bail!(
            "Partition '{}' has no query_file or query_inline",
            partition.name
        )
    }

    pub fn get_crawl_settings(&self, code: &str) -> CrawlSettings {
        let defaults = self
            .default_crawl_settings
            .clone()
            .unwrap_or(CrawlSettings {
                max_edges_per_node: 50,
                rate_limit_ms: 200,
                request_timeout_secs: 120,
                max_retries: 3,
                link_traversal_depth: 2,
                max_wikipedia_articles: 500_000,
            });
        let country = match self.get(code) {
            Some(c) => c,
            None => return defaults,
        };
        country.crawl_settings.clone().unwrap_or(defaults)
    }
}
