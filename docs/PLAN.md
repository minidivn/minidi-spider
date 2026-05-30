# MinidiSpider — Execution Plan

## 🎯 Goal
Crawl WikiData → HyperGraph → Searchable static site (GitHub Pages), focused on Vietnam.

## 📦 Deliverable
1. `minidi-spider` CLI binary
2. `docs/index.html` — self-contained search page
3. `docs/index.json` — hypergraph dump (~5-50MB)
4. `docs/` — weight dir for Transformers.js model

## 🧱 Execution Phases

### Phase 0: Scaffold (Current)
```
src/
├── main.rs              # CLI entrypoint
├── graph/
│   ├── mod.rs           # HyperNode, HyperEdge, HyperGraph types
│   └── store.rs         # sled-backed persistence
├── crawler/
│   ├── mod.rs           # WikiData client
│   └── sparql.rs        # SPARQL query constants
├── index/
│   ├── mod.rs           # Tantivy full-text index
│   └── export.rs        # JSON export for frontend
└── search/
    ├── mod.rs           # Query pipeline
    └── embed.rs         # Embedding helpers (for pre-compute)
```

### Phase 1: Graph Engine
1. Define `HyperNode`, `HyperEdge`, `NodeType` enums → `src/graph/mod.rs`
2. Implement `HyperGraphStore` backed by `sled` → `src/graph/store.rs`
   - `insert_node()`, `insert_edge()`, `get_node()`, `query_neighbors()`
   - Batch import from serde JSON
3. Test with small fixture data (5 entities)

### Phase 2: WikiData Crawler
1. SPARQL endpoint: `https://query.wikidata.org/sparql`
   - User-Agent: `MinidiSpider/0.1 (midivn)`
   - Retry with exponential backoff
2. Queries to write:
   - `VIETNAM_CITIES`: SELECT all admin divisions of Vietnam (Q881)
   - `VIETNAM_HISTORY_EVENTS`: SELECT events with point in time, located in Vietnam
   - `VIETNAM_PEOPLE`: SELECT notable people born in Vietnam
   - `VIETNAM_HERITAGE`: SELECT UNESCO/heritage sites
3. For each entity, fetch:
   - Labels (en, vi)
   - Descriptions (en, vi)
   - Aliases
   - Claims → edges to other Q-items
4. Store via `HyperGraphStore`

### Phase 3: Search Index + Export
1. Build Tantivy index on labels + descriptions + aliases (English and Vietnamese)
2. Pre-compute embeddings using `all-MiniLM-L6-v2` (rust-bert or ONNX)
   - Alternative: skip for now, let the browser do it via Transformers.js
3. Export to `docs/index.json`:
   ```json
   {
     "meta": { "crawled_at": "2025-...", "entity_count": 5000, "edge_count": 15000 },
     "nodes": [ { "id":"Q881", "label":"Vietnam", "desc":"...", "type":"Place", ... } ],
     "edges": [ { "src":"Q881", "rel":"P150", "tgt":"Q1234", "label":"contains administrative division" } ]
   }
   ```
4. Also export `node_embeddings.bin` if pre-computed

### Phase 4: Frontend — Search HTML Page
- Single `index.html` with inline CSS + JS (no build step)
- Search modes:
  1️⃣ **Semantic** (default): Transformers.js → cosine sim
  2️⃣ **Keyword**: BM25 fallback (simple JS implementation)
- Results show: title, description, type badge, link to WikiData
- Click → expand detail panel showing:
  - All edges (relationships to other entities)
  - Click any related entity → jump to it
  - WikiData permalink

### Phase 5: Deployment
1. GitHub repo: `midivn/minidi-spider`
2. GitHub Pages: `docs/` → `https://midivn.github.io/minidi-spider/`
3. GitHub Actions: Cron weekly rebuild
   ```yaml
   on:
     schedule:
       - cron: '0 6 * * 0'  # Sunday 6 AM
   ```

## 📐 Data Flow

```
         SPARQL fetch           sled write          JSON serialize
  WDQS ───────────▶ Crawler ───────────▶ Store ──────────────▶ index.json
                                          │                       │
                                          ▼                       ▼
                                       Tantivy               HTML Page
                                       (local search)     (browser search)
```

## 🚧 Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| WDQS rate limiting | Exponential backoff + User-Agent header + cache in sled |
| Large dataset (Vietnam = 50k+ entities) | Paginated SPARQL + streaming writes |
| Transformers.js ~23MB download | Show progress bar, LRU cache in IndexedDB |
| Vietnamese tokenization | Use unicode segmentation crate + custom Tantivy analyzer |
| GitHub Pages 1GB limit | Compress index.json with .gz, serve via brotli |

## 🧪 Testing Strategy
- `cargo test` — unit tests for graph model, SPARQL query construction
- `cargo run -- crawl --limit 10` — dry-run with small batch
- `cargo run -- export` — generate fresh `docs/` files
- Manual: open `docs/index.html` in browser, verify search works
