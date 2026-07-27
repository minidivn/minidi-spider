pub mod export;

use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, TantivyDocument};
use tracing::info;

use crate::graph::HyperGraph;

/// Creates and populates a Tantivy full-text index from a HyperGraph.
pub struct FullTextIndex {
    index: Index,
    schema: Schema,
    reader: IndexReader,
}

impl FullTextIndex {
    /// Build a new Tantivy index in memory from the given graph.
    pub fn build(graph: &HyperGraph) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let label_field = schema_builder.add_text_field("label", TEXT | STORED);
        let label_local_field = schema_builder.add_text_field("label_local", TEXT | STORED);
        let description_field = schema_builder.add_text_field("description", TEXT | STORED);
        let aliases_field = schema_builder.add_text_field("aliases", TEXT);
        let aliases_local_field = schema_builder.add_text_field("aliases_local", TEXT);
        let node_type_field = schema_builder.add_text_field("node_type", STRING | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        // Register Vietnamese-friendly tokenizer
        // Use SimpleTokenizer with lowercase for broadest language support
        index
            .tokenizers()
            .register("default", TextAnalyzer::from(SimpleTokenizer::default()));

        let mut writer: IndexWriter = index.writer_with_num_threads(1, 50_000_000)?;

        info!("Indexing {} nodes into Tantivy...", graph.node_count());
        for node in graph.nodes.values() {
            // Combine aliases into one field
            let all_aliases = [node.aliases.as_slice(), node.aliases_local.as_slice()].concat();

            writer.add_document(doc!(
                id_field => node.id.clone(),
                label_field => node.label.clone(),
                label_local_field => node.label_local.as_deref().unwrap_or(""),
                description_field => node.description.clone(),
                aliases_field => all_aliases.join("; "),
                aliases_local_field => node.aliases_local.join("; "),
                node_type_field => format!("{:?}", node.node_type),
            ))?;
        }

        writer.commit()?;
        info!("Index commit complete");

        let reader = index.reader()?;
        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Open an existing index from disk.
    pub fn open(path: &str) -> Result<Self> {
        let schema = Schema::builder().build(); // placeholder
        let index = Index::open_in_dir(path)?;
        let reader = index.reader()?;
        Ok(Self {
            index,
            schema,
            reader,
        })
    }

    /// Search the index and return top-k results.
    pub fn search(&self, query_str: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let schema = self.index.schema();

        let id_field = schema.get_field("id")?;
        let label_field = schema.get_field("label")?;
        let description_field = schema.get_field("description")?;
        let node_type_field = schema.get_field("node_type")?;

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                schema.get_field("label")?,
                schema.get_field("label_local")?,
                schema.get_field("description")?,
                schema.get_field("aliases")?,
                schema.get_field("aliases_local")?,
            ],
        );

        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc::<TantivyDocument>(doc_address)?;
            results.push(SearchResult {
                id: doc
                    .get_first(id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                label: doc
                    .get_first(label_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: doc
                    .get_first(description_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                node_type: doc
                    .get_first(node_type_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                score,
            });
        }

        Ok(results)
    }

    /// Get total number of indexed documents.
    pub fn doc_count(&self) -> usize {
        self.reader.searcher().num_docs() as usize
    }
}

/// A search result from the full-text index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub label: String,
    pub description: String,
    pub node_type: String,
    pub score: f32,
}
