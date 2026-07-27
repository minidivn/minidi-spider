# MinidiSpider — Architecture & Roadmap

## Two Crawler Systems

```
┌─────────────────────────────────────────────────────────────┐
│                    MinidiSpider Engine                       │
├──────────────────────┬──────────────────────────────────────┤
│   Crawler Tier 1     │       Crawler Tier 2                 │
│   (WikiData SPARQL)  │   (Wikipedia Content Mining)         │
│                      │                                      │
│   • Structured data  │   • Full article text                │
│   • Properties/edges │   • Link graph with weights          │
│   • Labels/descs     │   • Tables → structured records      │
│   • Fast, reliable   │   • Entity co-occurrence edges       │
│   • 3 generic queries│   • Temporal/spatial extraction      │
└──────────────────────┴──────────────────────────────────────┘
         │                         │
         ▼                         ▼
   ┌─────────────────────────────────────────────────┐
   │           HyperGraph Storage (sled)              │
   │  Nodes | Edges | WeightedEdges | HyperEdgeSets   │
   │  Embeddings | SpaceTimeIndex                     │
   └─────────────────────────────────────────────────┘
         │                         │
         ▼                         ▼
   ┌─────────────────────────────────────────────────┐
   │           Export Formats                        │
   │  index.json (full) | index.lite.json            │
   │  graph.bin.hg (compact binary)                  │
   │  v1/entities/ | v1/timeline/ | v1/relations/   │
   └─────────────────────────────────────────────────┘
```

---

## Phase 1: Config System Overhaul

### 1.1 `configs/countries.json` — Per-country partition config

Move to `/configs` folder. Each country gets:

```json
{
  "code": "vn",
  "name": "Vietnam",
  "qid": "Q881",
  "language": "vi",
  "language_name": "Vietnamese",
  "repo": "minidi-vn-data",
  "native_label": true,
  "wikipedia": {
    "api": "https://vi.wikipedia.org/w/api.php",
    "language": "vi"
  },
  "sparql_endpoint": "https://query.wikidata.org/sparql",
  "partitions": [
    {
      "name": "places",
      "query_file": "places.sparql",
      "node_type": "Place",
      "limit": 5000,
      "filters": {
        "has_coords": true,
        "min_population": 10000
      }
    },
    {
      "name": "people",
      "query_file": "people.sparql",
      "node_type": "Person",
      "limit": 5000
    },
    {
      "name": "events",
      "query_file": "events.sparql",
      "node_type": "Event",
      "limit": 3000
    }
  ],
  "crawl_settings": {
    "max_edges_per_node": 50,
    "rate_limit_ms": 200,
    "request_timeout_secs": 120,
    "max_retries": 3,
    "link_traversal_depth": 2
  }
}
```

### 1.2 New config fields

| Field | Type | Purpose |
|-------|------|---------|
| `partitions[]` | Array | Per-country partition definitions with custom SPARQL queries |
| `partitions[].query_file` | String | Path to `.sparql` file in `configs/queries/` |
| `partitions[].filters` | Object | Post-filtering criteria (has_coords, population, etc.) |
| `wikipedia` | Object | Wikipedia API endpoint and language for content mining |
| `sparql_endpoint` | String | Overridable WDQS endpoint |
| `crawl_settings` | Object | Rate limiting, retries, traversal depth |

### 1.3 SPARQL query files

Each partition can reference an external `.sparql` file:

```
configs/
├── countries.json
├── queries/
│   ├── places.sparql       # Generic "entities in country" 
│   ├── people.sparql       # Generic "people from country"
│   ├── events.sparql       # Generic "events in country"
│   ├── vn-heritage.sparql  # Vietnam-specific: heritage sites
│   ├── en-presidents.sparql # US-specific: presidents
│   └── zh-dynasties.sparql # China-specific: dynasties
```

SPARQL files use `{COUNTRY_QID}` and `{LANG}` placeholders:

```sparql
# configs/queries/places.sparql
SELECT DISTINCT ?item ?itemLabel ?itemDescription ?type ?typeLabel ?coord WHERE {
  VALUES ?country { wd:{COUNTRY_QID} }
  { ?item wdt:P17 ?country . }
  ?item wdt:P31 ?type .
  OPTIONAL { ?item wdt:P625 ?coord . }
  SERVICE wikibase:label { bd:serviceParam wikibase:language "{LANG},en" . }
}
LIMIT {LIMIT}
```

### 1.4 Config loader updates (`src/config.rs`)

```rust
pub struct PartitionConfig {
    pub name: String,
    pub query_file: Option<String>,
    pub query_inline: Option<String>,
    pub node_type: String,
    pub limit: usize,
    pub filters: Option<HashMap<String, serde_json::Value>>,
}

pub struct WikipediaConfig {
    pub api: String,
    pub language: String,
}

pub struct CrawlSettings {
    pub max_edges_per_node: usize,
    pub rate_limit_ms: u64,
    pub request_timeout_secs: u64,
    pub max_retries: u32,
    pub link_traversal_depth: u32,
}

pub struct CountryConfig {
    // ... existing fields ...
    pub partitions: Option<Vec<PartitionConfig>>,
    pub wikipedia: Option<WikipediaConfig>,
    pub sparql_endpoint: Option<String>,
    pub crawl_settings: Option<CrawlSettings>,
}
```

---

## Phase 2: Wikipedia Content Mining Crawler

### 2.1 Problem Statement

**Current state**: WikiData SPARQL gives structured labels and properties but **almost no graph data** — edges are sparse. Wikipedia has rich inter-article links but is too large to mirror entirely (~60 GB compressed).

**Goal**: Design a crawler that:
1. Starts from seed articles (country-related WikiData entities)
2. Reads Wikipedia article content and links
3. Analyzes text to extract entities and relationships
4. Creates weighted edges between entities
5. Parses infoboxes/tables as structured data
6. Extracts temporal and spatial anchors
7. Stores everything in a compact hypergraph format

### 2.2 Architecture

```
Seed QIDs ──► WikiData Lookup ──► Wikipedia URLs
                                        │
                                        ▼
┌──────────────────────────────────────────────┐
│         Wikipedia Link Crawler                │
│                                              │
│  ┌─────────┐   ┌──────────┐   ┌──────────┐  │
│  │Page      │──►│Content   │──►│Link      │  │
│  │Fetcher   │   │Parser    │   │Traveler  │  │
│  └─────────┘   └──────────┘   └──────────┘  │
│       │              │              │        │
│       ▼              ▼              ▼        │
│  HTML/JSON     Text chunks     New URLs      │
│  (REST API)    + metadata      (BFS queue)   │
└──────────────────────────────────────────────┘
       │              │              │
       ▼              ▼              ▼
┌──────────┐  ┌────────────┐  ┌────────────┐
│  Entity   │  │ Relationship│  │  SpaceTime  │
│  Extractor│  │  Analyzer   │  │  Extractor  │
└──────────┘  └────────────┘  └────────────┘
       │              │              │
       ▼              ▼              ▼
┌──────────────────────────────────────────────┐
│         HyperGraph Builder                   │
│                                              │
│  Nodes: Entity IDs (WikiData QID / WP title) │
│  Edges: Weighted (link freq, co-occurrence)  │
│  HyperEdges: N-ary (tables, infoboxes)       │
│  Temporal: Event→Date anchors                │
│  Spatial: Entity→Coordinate anchors          │
└──────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────┐
│         Export Formats                       │
│                                              │
│  graph.json    — Complete hypergraph (dev)   │
│  graph.bin.hg  — Compact binary (production) │
│  index.json    — Search-optimized flat       │
└──────────────────────────────────────────────┘
```

### 2.3 Component Design

#### 2.3.1 Page Fetcher (`src/sources/wikipedia/crawler.rs`)

```
Input:  Wikipedia URLs (from BFS queue)
Output: Raw page content (JSON from REST API)

Strategy:
- Use Wikipedia REST API (not HTML scraping)
  GET https://en.wikipedia.org/api/rest_v1/page/summary/{title}
  GET https://en.wikipedia.org/api/rest_v1/page/html/{title}
- Extract: title, extract (summary), full text (optional), 
           links, categories, coordinates, infobox data
- Respect robots.txt and rate limits (configurable)
- Store raw pages in sled cache (dedup)
```

```rust
pub struct WikipediaPage {
    pub title: String,
    pub page_id: u64,
    pub summary: String,
    pub content: String,  // first N paragraphs only
    pub links: Vec<PageLink>,  // internal links
    pub categories: Vec<String>,
    pub coordinates: Option<GeoPoint>,
    pub infobox: Option<Infobox>,
}

pub struct PageLink {
    pub title: String,     // linked article title
    pub anchor_text: String, // display text of the link
    pub section: String,   // which section the link appears in
}

pub struct Infobox {
    pub template: String,  // e.g. "Infobox settlement"
    pub fields: HashMap<String, Vec<String>>,
}
```

#### 2.3.2 Link Traveler (`src/sources/wikipedia/traveler.rs`)

```
Input:  Seed set of Wikipedia article titles
Output: Prioritized crawl frontier

Strategy:
- BFS traversal from seed articles
- Priority queue based on relevance scoring:
  * Link frequency (how many seed pages link to this)
  * Category proximity (shared categories with seeds)
  * Page importance (Wikipedia page rank proxy)
- Max depth: configurable (default 2 hops from seeds)
- Max articles: configurable (default 100,000)
- Dedup: article hash set in sled
- Edge weight: number of seed articles linking to this + frequency
```

```rust
pub struct CrawlFrontier {
    queue: BinaryHeap<PrioritizedUrl>,
    visited: HashSet<String>,
    max_articles: usize,
    max_depth: u32,
}

// Scoring:
// relevance = link_count * 0.5 + category_overlap * 0.3 + page_importance * 0.2
```

#### 2.3.3 Entity & Relationship Analyzer (`src/sources/wikipedia/analyzer.rs`)

```
Input:  Page content and links
Output: HyperGraph nodes, edges, hyperedges

Analysis steps:

1. ENTITY EXTRACTION:
   - Internal Wikipedia links → Entity nodes (linked article title)
   - Wikidata QID mapping (via `?action=raw&section=0` or API)
   - Category membership → Type hints (Place, Person, Event)

2. RELATIONSHIP MINING:
   - Co-occurrence: Two entities linked from same page → weighted edge
   - Anchor text analysis: "was born in" → "place of birth" typed edge
   - Section context: Links in "History" section → temporal relation
   - Sequential links: "A is a B" patterns → "instance of" typed edge

3. TABLE / INFOBOX PARSING:
   - Infobox fields → structured property assertions
   - Tables → entity sets (hyperedge)
   - Key-value pairs → metadata fields

4. SPACETIME EXTRACTION:
   - Coordinates from infobox/geo tags → spatial index
   - Dates from infobox (founded, born, died, established) → temporal index
   - Location mentions in text → geocoding (via Wikidata)
```

#### 2.3.4 Weighted Graph Model

Extend the current `HyperEdge` to support weighted edges:

```rust
pub struct WeightedEdge {
    pub source: String,
    pub target: String,
    pub relation_type: RelationType,
    pub weight: f32,
    pub sources: Vec<String>,    // which articles produced this edge
    pub contexts: Vec<String>,   // excerpt/text snippets
    pub qualifiers: HashMap<String, Vec<String>>,
}

pub enum RelationType {
    Typed(String),   // Known property (P31, P17, etc.)
    Inferred,        // From co-occurrence
    Structural,      // From infobox/table
    Temporal,        // Time-based adjacency
    Spatial,         // Geographic proximity
}
```

### 2.4 Compact Binary Format (`graph.bin.hg`)

```
┌─────────────────────────────────────────────┐
│  MAGIC: "MNHG" (4 bytes)                    │
│  VERSION: u32 (1)                           │
│  HEADER_SIZE: u32                           │
├─────────────────────────────────────────────┤
│  HEADER                                      │
│  ├── country_code: [u8; 4]                  │
│  ├── crawl_timestamp: i64                    │
│  ├── node_count: u32                         │
│  ├── edge_count: u32                         │
│  ├── weighted_edge_count: u32                │
│  ├── hyperedge_count: u32                    │
│  ├── embedding_count: u32                    │
│  ├── embedding_dim: u16                      │
│  └── spacetime_node_count: u32              │
├─────────────────────────────────────────────┤
│  STRING TABLE                                │
│  ├── count: u32                              │
│  ├── offsets: [u32; count]                   │
│  └── utf8_data: [u8; ...]                   │
├─────────────────────────────────────────────┤
│  NODES                                       │
│  ├── For each node:                          │
│  │   ├── id: u32 (string table index)       │
│  │   ├── label: u32                         │
│  │   ├── label_local: u32 | 0xFFFFFFFF      │
│  │   ├── node_type: u8                      │
│  │   ├── metadata_count: u16                │
│  │   └── metadata: [(key_idx, val_idx); N]  │
│  └── ...                                     │
├─────────────────────────────────────────────┤
│  EDGES                                       │
│  ├── For each edge:                          │
│  │   ├── source: u32                        │
│  │   ├── target: u32                        │
│  │   ├── property: u32                      │
│  │   └── weight: f32                        │
│  └── ...                                     │
├─────────────────────────────────────────────┤
│  WEIGHTED EDGES                              │
│  ├── For each:                               │
│  │   ├── source, target, type               │
│  │   ├── weight: f32                        │
│  │   ├── source_count: u16                  │
│  │   └── source_articles: [u32; N]          │
│  └── ...                                     │
├─────────────────────────────────────────────┤
│  SPACETIME INDEX (R-tree)                    │
│  ├── For each node with coords:             │
│  │   ├── node_id: u32                       │
│  │   ├── lat: f64, lon: f64                 │
│  │   ├── time_start: i64 | 0               │
│  │   └── time_end: i64 | 0                  │
│  └── ...                                     │
└─────────────────────────────────────────────┘
```

### 2.5 Storage Budget

Wikipedia is ~60 GB compressed, but we only need:

| Data | Est. size per 100k articles |
|------|---------------------------|
| Summaries (first 2 paragraphs) | ~50 MB |
| Link graph (edges + weights) | ~20 MB |
| Infobox key-values | ~10 MB |
| Category tree | ~2 MB |
| Coordinates (10k entities) | ~0.5 MB |
| Temporal anchors | ~1 MB |
| Binary hypergraph (`.bin.hg`) | ~30 MB total |

**Total per country: ~30-50 MB**. Feasible for GitHub repositories.

### 2.6 Implementation Plan

| Step | File | Description |
|------|------|-------------|
| 1 | `src/sources/wikipedia/mod.rs` | Restructure: sub-modules for crawler, analyzer, traveler |
| 2 | `src/sources/wikipedia/crawler.rs` | Page fetcher, REST API client, rate limiting |
| 3 | `src/sources/wikipedia/traveler.rs` | BFS frontier, priority queue, dedup |
| 4 | `src/sources/wikipedia/analyzer.rs` | Entity extraction, relationship mining, infobox parser |
| 5 | `src/graph/weighted_edge.rs` | WeightedEdge, RelationType, qualifiers |
| 6 | `src/export/binary.rs` | `.bin.hg` format writer/reader |
| 7 | `src/spacetime/mod.rs` | SpaceTime index (R-tree + temporal) |

---

## Roadmap

### Now (Phase 1 — Config Overhaul)
- [x] `configs/countries.json` with partitions, wikipedia, crawl_settings
- [x] Updated `src/config.rs` with new structs
- [x] SPARQL query files in `configs/queries/`
- [x] Per-country partition override capability
- [x] Updated README

### Next (Phase 2 — Wikipedia Mining)
- [ ] Wikipedia link crawler (BFS from seeds)
- [ ] Content parser (summary + link extraction)
- [ ] Relationship analyzer (co-occurrence, anchor text)
- [ ] Infobox/table parser
- [ ] Weighted edge model
- [ ] SpaceTime index (R-tree + temporal)
- [ ] `.bin.hg` binary format
- [ ] Integration with GitHub Actions

### Future
- [ ] LLM-based relationship extraction (optional, for accuracy)
- [ ] Cross-country entity merging (e.g. "Barack Obama" in en + hi + ar)
- [ ] Incremental crawl (only fetch changed articles)
- [ ] Entity resolution (same entity, different Wikipedia titles)
