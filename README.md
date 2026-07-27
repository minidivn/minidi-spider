# 🕷️ MinidiSpider

**WikiData → HyperGraph → Semantic Search for any country.**  
Crawl entities (places, people, events) from WikiData, index them as a typed hypergraph, and export searchable JSON datasets for static frontends.

```
┌──────────────────────┐     ┌──────────────────┐     ┌────────────────────┐
│  WikiData Crawler     │────▶│  HyperGraph Engine│────▶│  Search Frontend   │
│  (SPARQL + REST API)  │     │  (sled + tantivy) │     │  (static HTML +    │
│  Generic per country  │     │                   │     │   transformers.js) │
└──────────────────────┘     └──────────────────┘     └────────────────────┘
```

---

## 🌍 Features

- **Country-agnostic**: Crawl any WikiData country by QID — configured via `countries.json`
- **3 generic datasets**: Places (geography), People (notable individuals), Events (history)
- **Multilingual labels**: English + native language labels per country
- **Graph storage**: sled-embedded database for incremental crawling
- **Full-text search**: Tantivy index over labels, descriptions, and aliases
- **Partitioned export**: JSON output split by entity type, century timeline, and relation category
- **GitHub Actions**: Automated weekly crawl → push to per-country data repos
- **Edge AI embeddings**: Pre-computed BOW vectors for client-side semantic search

---

## 🚀 Quick Start

```bash
# Initialize directories
cargo run -- init

# See configured countries
cargo run -- countries

# List available data sources
cargo run -- sources

# Crawl WikiData for Vietnam (default from countries.json)
cargo run -- crawl --country vn

# Crawl for a specific country (e.g. United States)
cargo run -- crawl --country en

# Export to the configured output path
cargo run -- export --country en

# Export to a custom directory
cargo run -- export --country en --output ../Minidi/Data/minidi-en-data

# Search locally
cargo run -- search "Washington"

# View graph statistics
cargo run -- stats
```

### Local output structure

By default, export writes to `../Minidi/Data/minidi-<country_code>-data/` as configured in `countries.json`:

```
../Minidi/Data/
├── minidi-vn-data/        # Vietnam
├── minidi-en-data/        # United States
├── minidi-zh-data/        # China
├── minidi-hi-data/        # India
└── ...                    # Other countries
```

---

## 📋 CLI Commands

| Command | Description |
|---------|-------------|
| `crawl --country <code>` | Fetch entities for a country from WikiData via SPARQL |
| `export --country <code>` | Export graph to JSON for the frontend |
| `search <query>` | Full-text search via Tantivy |
| `stats` | Show entity counts by type |
| `countries` | List configured countries from `countries.json` |
| `sources` | Show available data source schemas |
| `init` | Create required directories |

### Key flags

| Flag | Used with | Description |
|------|-----------|-------------|
| `--country <code>` | crawl, export | Country code (e.g. vn, en, zh) |
| `--config <path>` | crawl, export | Path to `countries.json` |
| `--output <path>` | export | Output directory (overrides config) |
| `--db <path>` | crawl, export | sled database path |
| `--limit <N>` | crawl | Max entities per dataset |
| `--progress` | crawl | Show progress bars |
| `--embeddings` | export | Compute BOW embedding vectors |
| `--sample <N>` | export | Export only first N nodes (testing) |

---

## 🌐 Configuration: `countries.json`

The project uses `countries.json` at the project root to define which countries to crawl:

```json
{
  "default_country": "vn",
  "output_parent": "../Minidi/Data",
  "countries": [
    {
      "code": "vn",
      "name": "Vietnam",
      "qid": "Q881",
      "language": "vi",
      "language_name": "Vietnamese",
      "repo": "minidi-vn-data",
      "native_label": true
    },
    {
      "code": "en",
      "name": "United States",
      "qid": "Q30",
      "language": "en",
      "language_name": "English",
      "repo": "minidi-en-data",
      "native_label": false
    }
  ]
}
```

| Field | Description |
|-------|-------------|
| `code` | Two-letter country code (directory/CLI identifier) |
| `name` | Human-readable country name |
| `qid` | WikiData QID for the country entity |
| `language` | Native language code for labels |
| `repo` | Output directory / GitHub repo name |
| `native_label` | Whether to fetch native-language labels |

**Top 10 included by default**: English (US), Chinese, Hindi, Spanish, Arabic, French, Portuguese, Russian, Bengali, German, plus Vietnam.

---

## 🗂️ Data Organization

Exported data follows this structure (per country):

```
minidi-en-data/                     # e.g. for United States
├── index.json                      # Full export — all nodes + edges
├── index.lite.json                 # Lightweight — node IDs + labels only
├── sources.json                    # Data source schemas
│
├── v1/
│   ├── entities/                   # Partitioned by entity type
│   │   ├── place.json
│   │   ├── person.json
│   │   ├── event.json
│   │   ├── concept.json
│   │   ├── organization.json
│   │   ├── artifact.json
│   │   └── _index.json
│   │
│   ├── timeline/                   # Partitioned by century
│   │   ├── 19th-century.json
│   │   ├── 1901-1950.json
│   │   ├── 1951-2000.json
│   │   └── ...
│   │
│   ├── relations/                  # Partitioned by category
│   │   ├── geographic.json
│   │   ├── temporal.json
│   │   ├── social.json
│   │   ├── relations.json
│   │   └── all.json
│   │
│   └── schema.json                 # Schema descriptor
│
├── _metadata/                      # Audit trail
│   ├── crawl-report-latest.json
│   ├── provenance.json
│   └── changelog.json
│
├── embeddings.bin                  # BOW vectors (binary)
└── version/
    └── latest.json                 # Version alias
```

### JSON field format

**Node** (compact field names for smaller payloads):

| Field | Description |
|-------|-------------|
| `id` | WikiData Q-id |
| `l` | English label |
| `ll` | Local/native label (optional) |
| `d` | English description |
| `dl` | Local description (optional) |
| `t` | Entity type: place / person / event / concept / org / artifact |
| `u` | WikiData URL |
| `m` | Metadata map (coordinates, dates, occupations) |

**Edge**:

| Field | Description |
|-------|-------------|
| `s` | Source node Q-id |
| `r` | Relation/property label |
| `t` | Target node Q-id |
| `tl` | Target node label |

---

## 🤖 GitHub Actions

### 1. `crawl-all-countries.yml` — Multi-country crawl & push

**Trigger**: Weekly schedule (Sunday 08:00 UTC) or manual dispatch.

This workflow:
1. Builds the spider tool
2. Crawls WikiData for each country in the top-10 list (parallel matrix)
3. Exports data to `minidi-<cc>-data` directories
4. Creates/pushes to per-country GitHub repos using `GH_PAT`
5. Creates the repo if it doesn't exist, enables GitHub Pages

Requires: `secrets.GH_PAT` (GitHub Personal Access Token with `repo` and `admin:org` scopes).

```yaml
# Manual: Run for a single country
gh workflow run "Crawl & Push All Countries" -f country=en

# Manual: Run for all countries
gh workflow run "Crawl & Push All Countries"
```

### 2. `crawl-weekly.yml` — Single default country

**Trigger**: Weekly schedule (Sunday 06:00 UTC) or manual dispatch.

Crawls the default country from `countries.json`, exports to `docs/`, and creates a PR.

### 3. `deploy-pages.yml` — GitHub Pages deployment

Validates and deploys `docs/` to GitHub Pages.

### 4. `release-tag.yml` — Tag data release

Creates calendar-versioned tags (`vYYYY.MM.REVISION`) for data snapshots.

---

## 🔧 Setting up for a new country

1. **Add to `countries.json`**:
   ```json
   {
     "code": "jp",
     "name": "Japan",
     "qid": "Q17",
     "language": "ja",
     "language_name": "Japanese",
     "repo": "minidi-jp-data",
     "native_label": true
   }
   ```

2. **Crawl locally**:
   ```bash
   cargo run -- crawl --country jp --progress
   cargo run -- export --country jp
   ```

3. **Automate with GitHub Actions**:
   The `crawl-all-countries.yml` workflow uses a matrix strategy.
   To add Japan to the automated run, add `"jp"` to the default country list in the workflow's `matrix.country` field.

---

## 🏗️ Architecture

```
src/
├── main.rs              # CLI entrypoint with country-aware commands
├── config.rs            # Countries config loader
├── graph/
│   ├── mod.rs           # HyperNode, HyperEdge, HyperGraph types
│   └── store.rs         # sled-backed persistent storage
├── sources/
│   ├── mod.rs           # DataSource trait, SourceRegistry, CrawlContext
│   ├── wikidata.rs      # Generic WikiData SPARQL crawler (templated queries)
│   ├── wikipedia.rs     # Wikipedia source (WIP)
│   └── openstreetmap.rs # OpenStreetMap source (WIP)
├── index/
│   ├── mod.rs           # Tantivy full-text search
│   └── export.rs        # JSON export for frontend
└── search/
    └── mod.rs           # BOW embedding pre-computation
```

### SPARQL Query Templates

The generic SPARQL queries use `{COUNTRY_QID}` and `{LANG}` placeholders:

| Dataset | Query | Fields |
|---------|-------|--------|
| **Places** | Entities with `P17` (country) = QID | Labels, description, type, coordinates |
| **People** | Persons with `P27` (citizenship) or `P19` (birthplace) in country | Labels, birth/death dates, occupation |
| **Events** | Events with `P276` (location), `P17`, or `P710` (participant) involving country | Labels, dates, type |

---

## 🧪 Development

```bash
# Build
cargo build

# Run all tests
cargo test

# Test crawl with a small sample
cargo run -- crawl --country en --limit 100 --progress
cargo run -- export --country en --sample 50

# View results
cargo run -- search "president"
```

### Adding a new data source

1. Create `src/sources/yoursource.rs`
2. Implement the `DataSource` trait (see `wikidata.rs` for reference)
3. Register in `build_registry()` in `main.rs`

---

## 📄 License

MIT
