# AI Code Guideline — MinidiSpider

## Project Identity
**MinidiSpider** = WikiData → HyperGraph → Semantic Search.  
Crawl Vietnamese WikiData entities (cities, history, geography, culture) and index them as a typed hypergraph. Expose a static HTML frontend with on-device AI search.

## Architecture Principles

```
┌──────────────────┐     ┌─────────────────┐     ┌───────────────────┐
│  WikiData Crawler │────▶│ HyperGraph Engine│────▶│  Search Frontend  │
│  (SPARQL + REST)  │     │ (sled + tantivy) │     │ (static HTML +    │
│                   │     │                   │     │  transformers.js) │
└──────────────────┘     └─────────────────┘     └───────────────────┘
```

| Layer | Tech | Role |
|-------|------|------|
| Crawler | `reqwest` + SPARQL | Fetch Q-items, P-properties, labels from WikiData |
| Graph | `sled` + custom HyperGraph model | Store typed nodes/edges/hyperedges |
| Index | `tantivy` | Full-text search over labels & descriptions |
| Export | `serde_json` | Build a static `index.json` for the frontend |
| Frontend | Vanilla HTML + Transformers.js | On-device semantic search, JSON index lookup |

## Code Style
- Use `anyhow` for error propagation; `thiserror` for library errors.
- All SPARQL queries go in `src/sparql/` as constants.
- Graph operations are **immutable by default** — return new graph snapshots.
- Prefer `tracing` over `println` everywhere.
- CLI is the only entrypoint; no config files, all via `clap` flags.

## Data Model — HyperGraph
```rust
struct HyperNode {
    id: String,          // Q42
    label: String,       // "Douglas Adams"
    description: String,
    aliases: Vec<String>,
    node_type: NodeType, // Person, Place, Event, Concept
    wikidata_url: String,
}

struct HyperEdge {
    id: String,          // P31 (instance of)
    source: String,      // Q-id
    target: String,      // Q-id
    label: String,       // "instance of"
    qualifiers: HashMap<String, Vec<String>>, // temporal, precision, etc.
}

struct HyperGraph {
    nodes: HashMap<String, HyperNode>,
    edges: Vec<HyperEdge>,
    // HyperEdge = n-ary relationship (e.g., Battle involves {armies, location, date})
    hyper_edges: Vec<HyperEdgeSet>,
}
```

## Edge AI Strategy
- **Model**: `Xenova/all-MiniLM-L6-v2` via Transformers.js (~23MB quantized)
- **Pipeline**: User query → embed → cosine similarity against pre-computed node embeddings → top-k results
- **Fallback**: If model fails to load, use Tantivy BM25 as degraded mode
- **Deployment**: Everything ships as static files on GitHub Pages — no backend needed after export

## Constraints
- Zero runtime backend after crawl/export phase
- Vietnam-first content scope until user expands
- All search runs client-side in the browser
- GitHub Pages hosts the static output in `docs/` (the `index.html` + `index.json` + model weights)
