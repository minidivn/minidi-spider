mod config;
mod graph;
mod index;
mod search;
mod sources;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use sources::{CrawlContext, SourceRegistry};

#[derive(Parser)]
#[command(
    name = "minidi-spider",
    version,
    about = "Generic crawler: WikiData → HyperGraph → Search for any country"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all available sources and their schemas
    Sources,

    /// List configured countries from countries.json
    Countries {
        /// Path to countries config
        #[arg(long, default_value = "configs/countries.json")]
        config: PathBuf,
    },

    /// Crawl data sources and store in the graph database
    Crawl {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,

        /// Country code (e.g. vn, en, zh). Reads from countries.json.
        /// Use --config to specify custom config path.
        #[arg(short, long)]
        country: Option<String>,

        /// Path to countries config
        #[arg(long, default_value = "configs/countries.json")]
        config: PathBuf,

        /// Which source(s) to crawl. Omit = all registered sources
        #[arg(short, long)]
        source: Vec<String>,

        /// Max entities per source (0 = no limit)
        #[arg(short, long, default_value = "0")]
        limit: usize,

        /// Show progress bars
        #[arg(short, long)]
        progress: bool,
    },

    /// Export the graph to JSON for the frontend.
    /// Output path can be a country repo dir like ../Minidi/Data/minidi-vn-data
    Export {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,

        /// Output directory (default: auto-resolve from countries.json + --country)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Country code (e.g. vn, en). Resolves output path from config if --output not set.
        #[arg(short, long)]
        country: Option<String>,

        /// Path to countries config
        #[arg(long, default_value = "configs/countries.json")]
        config: PathBuf,

        /// Only export first N nodes (for testing)
        #[arg(long)]
        sample: Option<usize>,

        /// Compute and export BOW embeddings
        #[arg(long)]
        embeddings: bool,

        /// Country name override for metadata (default: auto from config)
        #[arg(long)]
        country_name: Option<String>,
    },

    /// Search the graph via full-text index
    Search {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,

        /// Search query
        query: String,

        /// Number of results
        #[arg(short, long, default_value = "10")]
        top_k: usize,
    },

    /// Show graph statistics
    Stats {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,
    },

    /// Initialize the project (create required directories)
    Init,
}

fn build_registry() -> SourceRegistry {
    let mut reg = SourceRegistry::new();
    reg.register(Box::new(sources::wikidata::WikiDataSource));
    reg.register(Box::new(sources::wikipedia::WikipediaSource));
    reg.register(Box::new(sources::openstreetmap::OpenStreetMapSource));
    reg
}

fn resolve_country_config(
    config_path: &PathBuf,
    country_code: &Option<String>,
) -> Result<(config::CountriesConfig, config::CountryConfig)> {
    let path = if config_path.exists() {
        config_path.clone()
    } else {
        // Fallback: try root-level countries.json
        let fallback = PathBuf::from("countries.json");
        if fallback.exists() {
            tracing::warn!(
                "Using deprecated countries.json at root. Move to configs/countries.json"
            );
            fallback
        } else {
            anyhow::bail!("Config file not found: {}", config_path.display());
        }
    };

    let countries = config::CountriesConfig::from_file(&path)?;
    let code = country_code
        .clone()
        .unwrap_or_else(|| countries.default_country.clone());
    let country = countries
        .get(&code)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Unknown country: {}. Check config file", code))?;
    Ok((countries, country))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Sources => {
            cmd_sources()?;
        }
        Command::Countries { config } => {
            cmd_countries(&config)?;
        }
        Command::Crawl {
            db,
            country,
            config,
            source,
            limit,
            progress,
        } => {
            cmd_crawl(db, country, config, source, limit, progress).await?;
        }
        Command::Export {
            db,
            output,
            country,
            config,
            sample,
            embeddings,
            country_name,
        } => {
            cmd_export(
                db,
                output,
                country,
                config,
                sample,
                embeddings,
                country_name,
            )
            .await?;
        }
        Command::Search { db, query, top_k } => {
            cmd_search(db, &query, top_k).await?;
        }
        Command::Stats { db } => {
            cmd_stats(db).await?;
        }
        Command::Init => {
            cmd_init()?;
        }
    }

    Ok(())
}

fn cmd_sources() -> Result<()> {
    let registry = build_registry();
    println!("\n📦 Available Data Sources\n");
    for source in registry.all() {
        let s = source.schema();
        println!("  {:15} v{}", s.name, s.version);
        println!("  │  📝 {}", s.description);
        println!("  │  🔗 {}", s.endpoint);
        println!("  │  🏷️  Entities: {}", s.entity_types.join(", "));
        println!("  │  © {}", s.attribution);
        println!();
    }
    Ok(())
}

fn cmd_countries(config_path: &PathBuf) -> Result<()> {
    let countries = config::CountriesConfig::from_file(config_path)?;
    println!("\n🌍 Configured Countries\n");
    println!(
        "  {:<6} {:<22} {:<10} {:<12} {:<18} {:<10}",
        "Code", "Name", "QID", "Language", "Output Repo", "Partitions"
    );
    println!("  {}", "─".repeat(82));
    for c in &countries.countries {
        let parts = countries.get_partitions(&c.code);
        let part_list: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
        let part_str = part_list.join(", ");
        println!(
            "  {:<6} {:<22} {:<10} {:<12} {:<18} {:<10}",
            c.code, c.name, c.qid, c.language_name, c.repo, part_str
        );
    }
    println!(
        "\n  Default: {} ({})",
        countries.default_country,
        countries.output_path(&countries.default_country)?.display()
    );

    // Show crawl settings example
    let settings = countries.get_crawl_settings(&countries.default_country);
    println!("\n  Crawl Settings (default):");
    println!(
        "    Rate limit: {}ms | Timeout: {}s | Retries: {} | Depth: {} | Max articles: {}",
        settings.rate_limit_ms,
        settings.request_timeout_secs,
        settings.max_retries,
        settings.link_traversal_depth,
        settings.max_wikipedia_articles,
    );

    println!("\n  Use: cargo run -- crawl --country <code>");
    println!("       cargo run -- export --country <code>\n");
    Ok(())
}

async fn cmd_crawl(
    db_path: PathBuf,
    country_code: Option<String>,
    config_path: PathBuf,
    sources: Vec<String>,
    limit: usize,
    progress: bool,
) -> Result<()> {
    let (countries_config, country) = resolve_country_config(&config_path, &country_code)?;
    info!(
        "Crawling for country: {} ({}) QID={} language={}",
        country.name, country.code, country.qid, country.language
    );
    info!("Store at: {}", db_path.display());

    let store = graph::store::HyperGraphStore::open(&db_path)?;
    let registry = build_registry();

    // Use partitions from config if available
    let partitions = countries_config.get_partitions(&country.code);
    let effective_limit = if limit > 0 {
        limit
    } else {
        partitions.first().map(|p| p.limit).unwrap_or(5000)
    };

    let ctx = CrawlContext::new(
        effective_limit,
        progress,
        Some(country.qid.clone()),
        Some(country.language.clone()),
        None,
    )?;

    let targets: Vec<String> = if sources.is_empty() {
        registry
            .all()
            .iter()
            .map(|s| s.schema().name.clone())
            .collect()
    } else {
        sources.iter().map(|s| s.clone()).collect()
    };

    for source_name in &targets {
        let source = match registry.get(source_name) {
            Some(s) => s,
            None => {
                tracing::warn!("Unknown source: {}. Skipping.", source_name);
                continue;
            }
        };

        let schema = source.schema();
        info!("Crawling source: {} (v{})", schema.name, schema.version);

        match source.crawl(&ctx).await {
            Ok(result) => {
                info!(
                    "  → {} nodes, {} edges from source '{}'",
                    result.nodes.len(),
                    result.edges.len(),
                    result.source
                );

                store.put_nodes_batch(&result.nodes)?;
                store.set_edges(&result.edges)?;
                store.flush()?;
            }
            Err(e) => {
                tracing::warn!("  ✗ Source '{}' failed: {}", source_name, e);
            }
        }
    }

    // Also crawl each partition from config if no specific source or wikidata is requested
    if sources.is_empty() || sources.contains(&"wikidata".to_string()) {
        for part in &partitions {
            // Use partition-specific limit
            let part_limit = if limit > 0 { limit } else { part.limit };

            let query = match countries_config.resolve_partition_query(
                part,
                &country.qid,
                &country.language,
                part_limit,
            ) {
                Ok(q) => q,
                Err(e) => {
                    tracing::warn!("  ✗ Partition '{}' query error: {}", part.name, e);
                    continue;
                }
            };

            info!(
                "  Partition '{}' (type={}) — {} limit={}",
                part.name,
                part.node_type,
                &query.lines().next().unwrap_or(""),
                part_limit
            );

            // Delegate to sources::wikidata for execution
            if let Some(wd_source) = registry.get("wikidata") {
                let mut part_ctx = CrawlContext::new(
                    part_limit,
                    progress,
                    Some(country.qid.clone()),
                    Some(country.language.clone()),
                    Some(part.name.clone()),
                )?;
                part_ctx.custom_query = Some(query);
                part_ctx.custom_node_type = Some(part.node_type.clone());
                match wd_source.crawl(&part_ctx).await {
                    Ok(result) => {
                        info!(
                            "  → Partition '{}': {} nodes, {} edges",
                            part.name,
                            result.nodes.len(),
                            result.edges.len()
                        );
                        store.put_nodes_batch(&result.nodes)?;
                        store.set_edges(&result.edges)?;
                    }
                    Err(e) => {
                        tracing::warn!("  ✗ Partition '{}' failed: {}", part.name, e);
                    }
                }
            }
        }
    }

    store.flush()?;
    info!("Crawl complete for {}!", country.name);
    Ok(())
}

async fn cmd_export(
    db_path: PathBuf,
    output: Option<PathBuf>,
    country_code: Option<String>,
    config_path: PathBuf,
    sample: Option<usize>,
    embeddings: bool,
    country_name: Option<String>,
) -> Result<()> {
    let (countries_config, country) = resolve_country_config(&config_path, &country_code)?;

    let output_dir = match output {
        Some(p) => p,
        None => countries_config.output_path(&country.code)?,
    };

    // Data files go into _data/ subfolder
    let data_dir = output_dir.join("_data");

    let display_name = country_name.unwrap_or_else(|| country.name.clone());

    info!("Exporting for: {} ({})", display_name, country.code);
    info!("Output: {}", output_dir.display());

    let store = graph::store::HyperGraphStore::open(&db_path)?;
    let graph = store.load_full_graph()?;
    info!(
        "Loaded graph: {} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    std::fs::create_dir_all(&output_dir)?;

    if let Some(max_nodes) = sample {
        index::export::export_sample(&graph, &data_dir, max_nodes)?;
    } else {
        index::export::export_graph_to_json(&graph, &data_dir, &country)?;
        index::export::export_graph_to_compressed_json(&graph, &data_dir)?;
        index::export::export_all_partitions(&graph, &data_dir)?;
    }

    // Export sources list
    let registry = build_registry();
    let schemas_json = serde_json::to_string_pretty(&registry.schemas())?;
    std::fs::write(data_dir.join("sources.json"), &schemas_json)?;

    if embeddings {
        info!("Computing BOW embeddings...");
        let embedding_index = search::EmbeddingIndex::compute_bow(&graph);

        let emb_bytes =
            search::EmbeddingIndex::export(&embedding_index.vectors, embedding_index.dimension)?;
        std::fs::write(data_dir.join("embeddings.bin"), &emb_bytes)?;

        let emb_json = serde_json::to_string(&search::EmbeddingIndex {
            vectors: embedding_index.vectors,
            dimension: embedding_index.dimension,
        })?;
        std::fs::write(data_dir.join("embeddings.json"), &emb_json)?;

        info!("Embeddings exported");
    }

    info!(
        "Export complete for {}! Output: {}",
        country.name,
        output_dir.display()
    );
    Ok(())
}

async fn cmd_search(db_path: PathBuf, query: &str, top_k: usize) -> Result<()> {
    info!("Opening store at: {}", db_path.display());
    let store = graph::store::HyperGraphStore::open(&db_path)?;
    let graph = store.load_full_graph()?;

    info!("Building full-text index...");
    let ft_index = index::FullTextIndex::build(&graph)?;

    info!("Searching for: '{}'", query);
    let results = ft_index.search(query, top_k)?;

    if results.is_empty() {
        println!("No results found.");
    } else {
        println!("\nTop {} results for '{}':\n", results.len(), query);
        for (i, r) in results.iter().enumerate() {
            println!(
                "{:>3}. [{:.4}] {} — {}",
                i + 1,
                r.score,
                r.label,
                r.description
            );
            println!(
                "     ID: {} | Type: {} | https://www.wikidata.org/wiki/{}\n",
                r.id, r.node_type, r.id
            );
        }
    }

    Ok(())
}

async fn cmd_stats(db_path: PathBuf) -> Result<()> {
    let store = graph::store::HyperGraphStore::open(&db_path)?;

    println!("\n📊 MinidiSpider — Graph Statistics\n");
    println!("  Database:   {}", db_path.display());
    println!("  Nodes:      {}", store.node_count());
    println!("  Edges:      {}", store.get_all_edges()?.len());

    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for node_res in store.iter_nodes() {
        if let Ok(node) = node_res {
            *type_counts
                .entry(format!("{:?}", node.node_type))
                .or_insert(0) += 1;
        }
    }

    if !type_counts.is_empty() {
        println!("\n  By Type:");
        for (t, count) in &type_counts {
            println!("    {:>15}: {}", t, count);
        }
    }

    println!();
    Ok(())
}

fn cmd_init() -> Result<()> {
    let dirs = vec!["data", "docs"];
    for dir in &dirs {
        std::fs::create_dir_all(dir)?;
        println!("  Created: {}", dir);
    }
    println!("\n✅ MinidiSpider initialized. Run `cargo run -- sources` to see available sources.");
    println!("   Run `cargo run -- countries` to see configured countries.");
    Ok(())
}
