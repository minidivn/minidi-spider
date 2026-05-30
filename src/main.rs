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
    about = "Generic crawler: Sources → HyperGraph → Search"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all available sources and their schemas
    Sources,

    /// Crawl data sources and store in the graph database
    Crawl {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,

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

    /// Export the graph to JSON for the frontend
    Export {
        /// Path to the sled database
        #[arg(short, long, default_value = "data/spider.db")]
        db: PathBuf,

        /// Output directory (usually docs/)
        #[arg(short, long, default_value = "docs")]
        output: PathBuf,

        /// Only export first N nodes (for testing)
        #[arg(long)]
        sample: Option<usize>,

        /// Compute and export BOW embeddings
        #[arg(long)]
        embeddings: bool,
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

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Sources => cmd_sources(),
        Command::Crawl {
            db,
            source,
            limit,
            progress,
        } => {
            cmd_crawl(db, source, limit, progress).await?;
        }
        Command::Export {
            db,
            output,
            sample,
            embeddings,
        } => {
            cmd_export(db, output, sample, embeddings).await?;
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

async fn cmd_crawl(
    db_path: PathBuf,
    sources: Vec<String>,
    limit: usize,
    progress: bool,
) -> Result<()> {
    info!("Opening store at: {}", db_path.display());
    let store = graph::store::HyperGraphStore::open(&db_path)?;

    let registry = build_registry();
    let ctx = CrawlContext::new(limit, progress)?;

    let targets: Vec<&str> = if sources.is_empty() {
        registry
            .all()
            .iter()
            .map(|s| s.schema().name.as_str())
            .collect()
    } else {
        sources.iter().map(|s| s.as_str()).collect()
    };

    for source_name in targets {
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

    store.flush()?;
    info!("Crawl complete!");
    Ok(())
}

async fn cmd_export(
    db_path: PathBuf,
    output_dir: PathBuf,
    sample: Option<usize>,
    embeddings: bool,
) -> Result<()> {
    info!("Opening store at: {}", db_path.display());
    let store = graph::store::HyperGraphStore::open(&db_path)?;

    let graph = store.load_full_graph()?;
    info!(
        "Loaded graph: {} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    std::fs::create_dir_all(&output_dir)?;

    if let Some(max_nodes) = sample {
        index::export::export_sample(&graph, &output_dir, max_nodes)?;
    } else {
        index::export::export_graph_to_json(&graph, &output_dir)?;
        index::export::export_graph_to_compressed_json(&graph, &output_dir)?;
        index::export::export_all_partitions(&graph, &output_dir)?;
    }

    // Export sources list
    let registry = build_registry();
    let schemas_json = serde_json::to_string_pretty(&registry.schemas())?;
    std::fs::write(output_dir.join("sources.json"), &schemas_json)?;

    if embeddings {
        info!("Computing BOW embeddings...");
        let embedding_index = search::EmbeddingIndex::compute_bow(&graph);

        let emb_bytes =
            search::EmbeddingIndex::export(&embedding_index.vectors, embedding_index.dimension)?;
        std::fs::write(output_dir.join("embeddings.bin"), &emb_bytes)?;

        let emb_json = serde_json::to_string(&search::EmbeddingIndex {
            vectors: embedding_index.vectors,
            dimension: embedding_index.dimension,
        })?;
        std::fs::write(output_dir.join("embeddings.json"), &emb_json)?;

        info!("Embeddings exported");
    }

    info!("Export complete!");
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
    Ok(())
}
