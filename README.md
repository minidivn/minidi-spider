# 🕷️ MinidiSpider

**WikiData → HyperGraph → Semantic Search.**  
Crawl Vietnamese WikiData entities (cities, history, geography, culture) and index them as a typed hypergraph. Expose a static HTML frontend with on-device AI search.

```
┌──────────────────┐     ┌─────────────────┐     ┌───────────────────┐
│  WikiData Crawler │────▶│ HyperGraph Engine│────▶│  Search Frontend  │
│  (SPARQL + REST)  │     │ (sled + tantivy) │     │ (static HTML +    │
│                   │     │                   │     │  transformers.js) │
└──────────────────┘     └─────────────────┘     └───────────────────┘
```

## Quick Start

```bash
# Initialize directories
cargo run -- init

# Crawl Vietnam entities from WikiData
cargo run -- crawl

# Export to frontend JSON
cargo run -- export --embeddings

# Search locally
cargo run -- search "Hanoi"

# View graph stats
cargo run -- stats
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `crawl` | Fetch Vietnam entities from WikiData via SPARQL |
| `export` | Export graph to `docs/` as JSON for the frontend |
| `search <query>` | Full-text search via Tantivy |
| `stats` | Show entity counts by type |
| `init` | Create required directories |

## Vietnam Data Scope

| Dataset | Source query | Expected entities |
|---------|-------------|-------------------|
| Administrative divisions | SPARQL `VIETNAM_ADMIN_DIVISIONS` | ~700 provinces, districts |
| Historical events | SPARQL `VIETNAM_HISTORY_EVENTS` | ~2000 battles, dynasties |
| Notable people | SPARQL `VIETNAM_PEOPLE` | ~5000 historical figures |
| Heritage sites | SPARQL `VIETNAM_HERITAGE` | ~500 cultural sites |

## Data Organization

See [docs/DATA_ORGANIZATION.md](docs/DATA_ORGANIZATION.md) for the full strategy:

- **Branching**: `main` (stable) ← `develop` (crawl results) ← `backfill/*` (one-off)
- **Tagging**: Calendar versioning `vYYYY.MM.REVISION`
- **Namespacing**: `v1/entities/`, `v1/timeline/`, `v1/regions/`, `v1/relations/`
- **Eras**: Paleolithic → Hồng Bàng → Chinese Domination → Dynastic → Colonial → Vietnam War → Modern

## GitHub Actions

| Workflow | Trigger | Action |
|----------|---------|--------|
| `crawl-weekly.yml` | Every Sunday 06:00 UTC | Crawl → Export → PR to main |
| `deploy-pages.yml` | Push to main (`docs/**`) | Validate → Deploy to GH Pages |
| `release-tag.yml` | Manual | Tag release → Update changelog |

## Frontend

The [docs/index.html](docs/index.html) is a self-contained search page that:

1. Loads `index.json` (the hypergraph export)
2. Uses **Transformers.js** (`all-MiniLM-L6-v2`) for semantic search on-device
3. Falls back to BM25 keyword search when AI model not available
4. Shows entity details with relationships to other entities
5. Links directly to WikiData source pages

## Architecture

    src/
    ├── main.rs              # CLI entrypoint
    ├── graph/
    │   ├── mod.rs           # HyperNode, HyperEdge, HyperGraph types
    │   └── store.rs         # sled-backed persistent storage
    ├── crawler/
    │   ├── mod.rs           # WikiData SPARQL client
    │   └── sparql.rs        # SPARQL query constants
    ├── index/
    │   ├── mod.rs           # Tantivy full-text search
    │   └── export.rs        # JSON export for frontend
    └── search/
        └── mod.rs           # BOW embedding pre-computation

## License

MIT
