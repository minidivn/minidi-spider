# Wikipedia Data Mining: HyperGraph Extraction Strategy

## The Problem

**Wikipedia is the richest publicly available knowledge graph ever built by humans.** It contains:

- 6.9 million English articles
- 60+ million inter-article links (directional, with anchor text context)
- 300+ language editions
- Infoboxes: structured key-value data on ~40% of articles
- Categories: a hierarchical taxonomy ~2M nodes deep
- Coordinates: 1.5M+ geotagged articles
- Temporal data: dates embedded in prose and infoboxes

**WikiData is the structured layer on top of Wikipedia**, but its graph is sparse:

- Only ~5% of Wikipedia articles have rich WikiData statements
- Most edges are basic `instance of` (P31) or `subclass of` (P279)
- Historical, cultural, and domain-specific relationships are **missing**
- WikiData models what editors explicitly entered — not what the text actually *means*

**The paradox:**

```
Wikipedia has the data (rich text)       but no machine-readable graph
WikiData has the schema (machine-ready)  but sparse coverage
```

**The constraint:** A full Wikipedia XML dump is ~60 GB compressed (enwiki), ~350 GB uncompressed. We cannot mirror it. We need surgical extraction that produces a **compact, traversable hypergraph**.

**Budget:**

| Tier | Total per country | Partitions at 4 MB each | Articles that can be covered |
|------|------------------|------------------------|------------------------------|
| **Target** | 512 MB | ~128 files | ~500k-1M articles |
| **Maximum** | 1024 MB (1 GB) | ~256 files | ~1M-2M articles |
| **Minimum viable** | 128 MB | ~32 files | ~100k-200k articles |

Each individual file stays **under 4 MB** regardless of total budget. This ensures fast loading, easy parsing, and parallel processing.

---

## Strategy Overview

```
Wikipedia                         MinidiSpider
──────────                       ───────────
6.9M articles                    500k-2M articles per country
~60 GB compressed                128-512 MB hypergraph per country
Full-text search                  Entity-relationship graph
Semi-structured (wikitext)        Structured (typed nodes + weighted edges)
No query API for graphs           SPARQL-like traversal on < 1 GB binary
```

The strategy is **not** to download and index Wikipedia. It is to **travel Wikipedia's link structure, extract entities and relationships as we go, and collapse the result into a compact, weighted hypergraph** that preserves the essential structure while discarding raw text.

```
       ┌─────────────────────────────────────────────────────┐
       │  Strategy: Travel → Extract → Compact → Traverse    │
       │                                                     │
       │  1. Start from seed entities (country's top QIDs)   │
       │  2. BFS along Wikipedia links with relevance scoring│
       │  3. Extract entities, relationships, tables, dates  │
       │  4. Weight edges by frequency and context           │
       │  5. Collapse into binary hypergraph (.bin.hg)       │
       │  6. AI agent traverses the hypergraph, not Wikipedia│
       └─────────────────────────────────────────────────────┘
```

---

## 1. Crawl Strategy: Relevance-Guided BFS

### 1.1 Seed Selection

Seeds are the top 1000 WikiData entities for a country (from Phase 1 SPARQL crawl):

```
adm-hanoi       → "Hanoi"        → https://en.wikipedia.org/wiki/Hanoi
people-hochiminh→ "Ho Chi Minh"  → https://en.wikipedia.org/wiki/Ho_Chi_Minh
hist-vn-war     → "Vietnam War"  → https://en.wikipedia.org/wiki/Vietnam_War
```

Each seed has a **base relevance score** from its WikiData partition weight:

| Seed source | Base score | Rationale |
|------------|-----------|-----------|
| Admin (capital, major cities) | 1.0 | High connectivity, many backlinks |
| People (top figures) | 0.9 | High informational value |
| Events (major historical) | 0.8 | Time-anchored, connects people/places |
| Culture (heritage sites) | 0.7 | Niche but high-quality links |

### 1.2 Frontier with Relevance Scoring

The crawl frontier is a priority queue. Every discovered URL gets a **relevance score**:

```
relevance(url) = link_frequency * 0.4
               + category_overlap * 0.3
               + page_importance * 0.2
               + depth_penalty * 0.1
```

**Components:**

| Factor | Calculation | Purpose |
|--------|------------|---------|
| `link_frequency` | How many visited seed pages link to this URL | Captures real graph importance |
| `category_overlap` | `| categories(url) ∩ categories(seeds) | / | categories(seeds) |` | Keeps crawl topically focused |
| `page_importance` | Length of article (proxies PageRank) | Longer articles have more links |
| `depth_penalty` | `1.0 / (1 + depth)` | Prevents infinite drift |

**Concrete example:**

```
URL: /wiki/Ao_dai (Vietnamese traditional clothing)
link_frequency:  0.85  (43 of 50 seed pages link to it)
category_overlap: 0.72  (shares "Vietnamese culture" with seeds)
page_importance:  0.60  (medium-length article)
depth_penalty:    0.50  (depth 1 from seeds)

relevance = 0.85*0.4 + 0.72*0.3 + 0.60*0.2 + 0.50*0.1
          = 0.34   + 0.216  + 0.12   + 0.05
          = 0.726  ← added to priority queue
```

**Thresholds:**

| Relevance | Action |
|-----------|--------|
| ≥ 0.7 | **Must crawl** — high confidence |
| 0.4 – 0.7 | **Queue** — crawl if budget remains |
| < 0.4 | **Skip** — too far from topic |

### 1.3 BFS Parameters

| Parameter | Default | Rationale |
|-----------|---------|-----------|
| `max_articles` | 500,000 | 512 MB budget allows 500k+ articles (~1 KB each) |
| `max_articles_extreme` | 2,000,000 | At 1 GB budget, scales to 2M articles |
| `max_depth` | 2 | Beyond 2 hops, topic drift is severe |
| `min_relevance` | 0.3 | Below this, articles are tangentially related |
| `rate_limit_ms` | 200 | Wikipedia API allows 200 req/s but we're polite |

### 1.4 Dedup and Loop Prevention

- **Visited set**: HashSet of article titles (in memory + sled checkpoint)
- **Link fingerprint**: `(source_title, target_title)` tuple prevents re-queuing
- **Depth tracking**: Store depth per article, never revisit with higher depth
- **Hash checkpoint**: Every 1000 articles, flush visited set to sled

---

## 2. Content Extraction: What We Take, What We Leave

### 2.1 API Strategy: REST, Not Scraping

We use the **Wikipedia REST API** exclusively — no HTML parsing, no wikitext:

```
GET /api/rest_v1/page/summary/{title}     → summary + extract + thumbnail
GET /api/rest_v1/page/html/{title}        → full HTML (for infobox parsing)
GET /api/rest_v1/page/links/{title}       → all outgoing links
```

**Why not the XML dump?**

| Approach | Bandwidth | Storage | Complexity |
|----------|-----------|---------|------------|
| Full XML dump | 60 GB download | 350 GB disk | High (wikitext parsing) |
| REST API (summary only) | ~50 MB per 100k articles | ~100 MB | Low (JSON) |
| REST API (full HTML) | ~500 MB per 100k articles | ~2 GB | Medium (HTML parsing) |

**Decision:** Use **summary + links** for bulk crawl (99% of articles). Use **full HTML** only for articles with infoboxes (~40% of articles).

### 2.2 Page Data Model

```rust
pub struct WikipediaPage {
    pub title: String,            // Article title
    pub page_id: u64,             // Wikipedia page ID
    pub qid: Option<String>,      // WikiData QID (from page props)

    // Content (always extracted)
    pub summary: String,          // First 2 paragraphs (~1 KB)
    pub categories: Vec<String>,  // Article categories
    pub links: Vec<PageLink>,     // All internal links

    // Content (conditional, only if infobox exists)
    pub content_sections: Vec<Section>,  // Section headings + text
    pub infobox: Option<Infobox>,        // Structured key-value data
    pub coordinates: Option<GeoPoint>,   // Geo coordinates
}

pub struct PageLink {
    pub target_title: String,     // Linked article title
    pub anchor_text: String,      // Display text of the link
    pub section: SectionType,     // Which section the link appears in
    pub position: u32,            // Character offset in article
}

pub enum SectionType {
    Lead,        // Before first heading
    Body,        // Main body section
    History,     // History section
    Geography,   // Geography section
    Culture,     // Culture section
    Biography,   // Biography section
    References,  // References section
    Other,       // Uncategorized
}

pub struct Infobox {
    pub template_name: String,    // "Infobox settlement", "Infobox person"
    pub fields: HashMap<String, Vec<String>>,  // Key → values
    pub raw_wikitext: Option<String>,          // For complex parsing
}
```

### 2.3 What We Discard

To keep storage under 512 MB (target) or 1 GB (extreme), we **discard**:

| Data | Discard? | Why |
|------|----------|-----|
| Full article text | **Discarded** (kept only as summary) | Text is 90% of size, graph structure is what matters |
| Image/media data | **Discarded** | Images are huge, URLs suffice |
| Edit history | **Discarded** | Irrelevant to knowledge graph |
| Talk pages | **Discarded** | Meta-discussion |
| External links | **Discarded** (except anchors) | Not part of the Wikipedia graph |
| Inter-language links | **Stored** as weak edges | Cross-lingual entity resolution |

**The 10% rule:** We keep roughly 10% of each article's raw data (summary + links + infobox). The remaining 90% (prose, images, tables, references) is discarded because it doesn't contribute to the entity-relationship graph.

**Scaling math:**

```
Per article:
  summary:    ~1 KB (2 paragraphs)
  links:      ~0.5 KB (50 links × 10 bytes avg)
  categories: ~0.2 KB (20 categories × 10 bytes)
  infobox:    ~0.5 KB (20 fields × 25 bytes) — 40% of articles
  metadata:   ~0.3 KB (flags, section markers, timestamps)
  ─────────────────────────────────
  total:      ~1.0-1.5 KB per article (extracted)

100,000  articles →  100-150 MB  (minimum viable)
500,000  articles →  500-750 MB  (target budget: 512 MB)
1,000,000 articles →  ~1 GB      (extreme budget: 1 GB)
```

At 4 MB per partition file:
- 512 MB / 4 MB = **128 partition files** minimum
- To keep files well under 4 MB (for faster loading), target **200-300 partitions**
- Each partition = ~2,000-5,000 entities depending on entity complexity

---

## 3. Entity Extraction

### 3.1 From Wikipedia Links

Every internal Wikipedia link is a candidate entity:

```
"Hanoi is the capital of [[Vietnam]] and its second largest city."
                                 ↓
Link target: "Vietland"           → Entity: "Vietnam" (Q881)
                   ↓
Link target is a Wikipedia article → Entity node in hypergraph
```

**Extraction rules:**

| Source | Entity | Confidence |
|--------|--------|------------|
| Internal link to existing article | Wikipedia title | 1.0 (exact) |
| Link with QID in URL | WikiData QID | 1.0 (if resolvable) |
| Redirect target | Canonical title | 0.9 (resolved via API) |
| Category page | Category entity | 0.8 (taxonomic node) |

### 3.2 QID Resolution

Each Wikipedia article links to a WikiData QID via the `wgWikibaseItemId` page property:

```
GET /api/rest_v1/page/summary/Hanoi
  → pageprops: { wikibase_item: "Q1858" }
```

We cache this mapping (`title → QID`) in sled. For articles where the API is slow, we fall back to:

```
GET /wiki/Special:EntityData/{title}.json
  → { id: "Q1858", ... }
```

**Batch resolution:** After each crawl batch (1000 articles), batch-resolve all unmapped titles via:

```
GET /api.php?action=wbgetentities&sites=enwiki&titles=Hanoi|Saigon|Danang
```

### 3.3 Type Inference from Categories

Wikipedia categories are a noisy but useful type system:

```
Page: "Hanoi"
Categories: ["Cities in Vietnam", "Capitals in Asia", "Populated places ..."]
            ↓
            ├── "Cities in Vietnam" → P31 (instance of) → Q515 (city)
            ├── "Capitals in Asia"  → P31 (instance of) → Q5119 (capital)
            └── "Populated places..." → ignored (too generic)
```

**Type mapping rules:**

| Category pattern | Inferred type | NodeType |
|-----------------|--------------|----------|
| `*cities*`, `*towns*`, `*villages*`, `*capitals*` | Geopolitical entity | Place |
| `*people*`, `*births*`, `*deaths*`, `*century*` | Person group | Person |
| `*battles*`, `*wars*`, `*treaties*`, `*events*` | Historical occurrence | Event |
| `*companies*`, `*organizations*`, `*universities*` | Organization | Organization |
| `*culture*`, `*art*`, `*music*`, `*literature*` | Cultural concept | Concept |
| `*buildings*`, `*structures*`, `*monuments*` | Physical structure | Place |

**Confidence scoring:** A page's type is the **majority vote** across its 20+ categories. If 12 of 20 categories suggest "Person" and 5 suggest "Place", the entity is confidently a Person (0.6 confidence).

### 3.4 Abstract Concept Detection

Some entities are abstract concepts, not concrete things. We detect these via:

- **Infobox type**: `Infobox philosophy`, `Infobox religion`, `Infobox ideology`
- **Category absence**: No geographic coordinates, no dates, no P31 in WikiData
- **Link pattern**: Links primarily to other abstract concepts

When detected, these become `NodeType::Concept` instead of `NodeType::Other`.

---

## 4. Relationship Mining: The Core Innovation

This is where Wikipedia data mining creates value that **doesn't exist in WikiData**. Every link between articles is a potential relationship. The algorithm infers typed, weighted edges from four signal sources.

### 4.1 Co-occurrence Mining

The simplest and most reliable signal: **two entities linked from the same page → they are related**.

```
Article: "Vietnam War"
Links to: ["Ho Chi Minh", "United States", "Saigon", "Hanoi",
           "Viet Cong", "Richard Nixon", "My Lai Massacre"]
           │            │              │         │
           └───── All pairwise co-occurrence edges ─────┘

  Ho Chi Minh ───── 0.83 ──── United States     (same article, same section)
  Ho Chi Minh ───── 0.71 ──── Viet Cong         (same article)
  Saigon      ───── 0.65 ──── Ho Chi Minh       (same article)
  ...
```

**Edge weight formula:**

```
weight(A, B) = Σ(1 / distance(A, B)) for all articles where both appear
             × section_bonus(article) × position_boost
```

Where:
- `distance`: number of links between A and B in the article (lower = closer)
- `section_bonus`: 1.5 if both in same section, 1.0 otherwise
- `position_boost`: 1.2 if in lead/lead section, 1.0 otherwise

### 4.2 Anchor Text Analysis

The **anchor text** of a link often encodes the relationship type:

```
Link: "[[Ho Chi Minh|He]] was born in [[Kim Lien|Nghe An province]]"
                 ↓                              ↓
       Anchor: "He" (pronoun)         Anchor: "Nghe An province"
       Generic, low value              Geographic, high value

Link: "[[Hanoi]] is the capital of [[Vietnam]]"
                    ↓                         ↓
          Anchor: "Hanoi"               Anchor: "Vietnam"
          Identity (self-ref)           Identity

Link: "[[Nguyen Hue|Emperor Quang Trung]] unified the country"
                    ↓
          Anchor: "Emperor Quang Trung"
          Alias/alternative name → aliases list
```

**Relationship pattern detection:**

| Pattern | Example | Inferred relationship |
|---------|---------|----------------------|
| `X was born in Y` | "Ho Chi Minh was born in Kim Lien" | P19 (place of birth) |
| `X is the capital of Y` | "Hanoi is the capital of Vietnam" | P1376 (capital of) |
| `X is a Y` | "Saigon is a city in Vietnam" | P31 (instance of) |
| `X fought in Y` | "Vo Nguyen Giap fought in the Vietnam War" | P241 (military branch) |
| `X was founded in Y` | "Hanoi was founded in 1010" | P571 (inception) |
| `X led Y` | "Le Loi led the Lam Son uprising" | P488 (commanded by) |

**Regex-based pattern matcher:**

```rust
lazy_static! {
    static ref BIRTH_PATTERN: Regex = Regex::new(
        r"(?i)(was born|born in|native of)\s+(\[\[[^\]]+\]\])"
    ).unwrap();
    static ref CAPITAL_PATTERN: Regex = Regex::new(
        r"(?i)(capital of|administrative center of)\s+(\[\[[^\]]+\]\])"
    ).unwrap();
    static ref FOUNDING_PATTERN: Regex = Regex::new(
        r"(?i)(founded|established|created)\s+(in|on)\s+(\[\[[^\]]+\]\])"
    ).unwrap();
}
```

**Precision/Recall trade-off:**

| Pattern | Precision | Recall | Use case |
|---------|-----------|--------|----------|
| Regex-based | ~0.75 | ~0.40 | High-confidence typed edges |
| Co-occurrence | ~0.60 | ~0.85 | Weighted, untyped edges |
| Infobox | ~0.95 | ~0.30 | Exact property assertions |
| Combined | ~0.82 | ~0.70 | Best of all worlds |

### 4.3 Section Context

A link's section tells us what *kind* of relationship it represents:

```
Article: "Ho Chi Minh"
  ├── Lead section: "Hồ Chí Minh, born Nguyễn Sinh Cung..."
  │     Links: Kim Lien → birthplace (P19)
  │             Vietnam → nationality (P27)
  ├── Early life: "was born in Kim Lien, Nghe An province..."
  │     Links: Kim Lien, Nghe An → geographic (P19)
  ├── Political career: "joined the [[Communist Party of France]]..."
  │     Links: Communist Party → affiliation (P1416)
  ├── Death and legacy: "died in [[Hanoi]]..."
  │     Links: Hanoi → death place (P20)
  └── See also: links to related articles
```

**Section-to-relation mapping:**

| Section heading | Likely relationship | WikiData property |
|----------------|-------------------|-------------------|
| `Early life`, `Childhood`, `Birth` | Birth, origin | P19, P569 |
| `Career`, `Political career`, `Military career` | Occupation, affiliation | P106, P1416 |
| `Death`, `Death and legacy` | Death | P20, P570 |
| `Geography`, `Geography and climate` | Location | P625, P2044 |
| `History`, `History and events` | Temporal relation | P580, P582 |
| `Culture`, `Culture and arts` | Cultural affiliation | P172, P361 |
| `Economy`, `Economy and infrastructure` | Economic relation | P127, P137 |

### 4.4 Sequential Link Patterns

Links that appear in **sequence** within a sentence encode structured relationships:

```
"S tripped and fell → Ai, the first emperor of China → born in → Handan"
  │                      │                              │           │
  sequential chain        │                              │           │
                          entity (Qin Shi Huang)         │           │
                                                        property    value
                                                        (P19)       (Handan)
```

We extract triples from sequences longer than 2:

```rust
// "Hanoi is the capital of Vietnam"
// Links: [Hanoi] [capital of] [Vietnam]
// Pattern: ENTITY - RELATION - ENTITY
fn extract_sequence_triple(links: &[PageLink]) -> Option<Triple> {
    if links.len() < 3 { return None; }
    // Heuristic: middle links with short anchor text are relation words
    let relation = links.iter().find(|l| l.anchor_text.len() < 20);
    let entity1 = links.first();
    let entity2 = links.last();
    // ...
}
```

### 4.5 Edge Dedup and Merging

When the same relationship `(source, target)` is discovered through multiple signals, we **merge** them:

```
Discovery 1: Co-occurrence on "Vietnam War" page → weight 0.83, untyped
Discovery 2: Anchor text "was born in" → weight 1.0, typed P19
Discovery 3: Infobox "birth_place=Kim Lien" → weight 1.0, typed P19, exact
Discovery 4: Section "Early life" → weight 0.7, typed P19

Merged edge:
  source: "Ho Chi Minh" (Q360)
  target: "Kim Lien" (Q1072044)
  type: P19 (place of birth)  ← consensus from 3 of 4 signals
  weight: 0.88  ← weighted average: (0.83 + 1.0 + 1.0 + 0.7) / 4
  confidence: 0.92  ← signal agreement: 3 of 4 agree on type
  evidence: ["Vietnam War article", "Anchor text", "Infobox", "Section context"]
```

**Merge algorithm:**

```rust
fn merge_edges(existing: &mut WeightedEdge, new_signal: EdgeSignal) {
    // Update weight: running average
    existing.weight = (existing.weight * existing.signal_count + new_signal.weight)
                    / (existing.signal_count + 1);

    // Type consensus: pick the type with most signals
    if new_signal.relation_type != existing.relation_type {
        existing.type_votes[new_signal.relation_type] += 1;
        if existing.type_votes[new_signal.relation_type] > existing.type_votes[existing.relation_type] {
            existing.relation_type = new_signal.relation_type;
        }
    }

    // Store evidence
    existing.sources.push(new_signal.source_article);
    if let Some(context) = new_signal.context_snippet {
        existing.contexts.push(context);
    }
    existing.signal_count += 1;
}
```

---

## 5. Table and Infobox Extraction

### 5.1 Infobox as Structured Data

Infoboxes are the highest-value, lowest-noise source of structured data. ~40% of Wikipedia articles have them.

```
┌────────────────────────────────────────┐
│         Infobox settlement              │
├────────────────────────────────────────┤
│  Name:           Hanoi                  │
│  Native name:    Hà Nội                │
│  Settlement type: Provincial capital    │
│  Country:        Vietnam                │
│  Region:         Red River Delta        │
│  Area total:     3,358.6 km²           │
│  Population:     8,053,663             │
│  Population rank: 2nd in Vietnam       │
│  Density:       2,400/km²              │
│  Timezone:       UTC+7                  │
│  Area codes:     24                     │
│  Website:        https://hanoi.gov.vn   │
│  Coordinates:    21°02′N 105°51′E      │
└────────────────────────────────────────┘
```

Each key-value pair becomes a **metadata field** on the entity:

```json
"m": {
  "area_total": "3,358.6 km²",
  "population": "8053663",
  "timezone": "UTC+7",
  "coordinates": "21°02′N 105°51′E"
}
```

**Infobox template → entity type mapping:**

| Template | Entity type | Example fields extracted |
|----------|-------------|------------------------|
| `Infobox settlement` | Place (city) | population, area, coordinates, timezone |
| `Infobox person` | Person | birth_date, birth_place, death_date, occupation |
| `Infobox military conflict` | Event | date, location, result, combatants |
| `Infobox company` | Organization | founded, founders, headquarters, revenue |
| `Infobox university` | Organization | established, city, students, affiliations |
| `Infobox museum` | Place (POI) | established, location, type, collection_size |
| `Infobox river` | Place (natural) | length, source, mouth, basin_countries |

### 5.2 Table Parsing

Wikitext tables (`{| ... |}`) are challenging. We handle them with a **two-pass strategy**:

- **Pass 1**: Detect if table is a "data table" (rows = records, columns = fields) vs. "layout table" (purely presentational)
- **Pass 2**: Extract column headers as property names, rows as entity records

```
Table: "Major battles of the Vietnam War"
┌──────────────────────┬──────────┬─────────────┬────────────┐
│ Battle               │  Date    │  Location   │  Result    │
├──────────────────────┼──────────┼─────────────┼────────────┤
│ Battle of Dien Bien  │ Mar-May  │ Dien Bien   │ Viet Minh  │
│   Phu                │  1954    │   Phu       │   victory  │
├──────────────────────┼──────────┼─────────────┼────────────┤
│ Tet Offensive        │  Jan 1968│ South VN    │ N. Viet    │
│                      │          │             │   victory  │
└──────────────────────┴──────────┴─────────────┴────────────┘

Extracted hyperedge:
{
  subject: "Major battles of the Vietnam War" (table),
  roles: {
    "battle": ["Battle of Dien Bien Phu", "Tet Offensive"],
    "date": ["Mar-May 1954", "Jan 1968"],
    "location": ["Dien Bien Phu", "South Vietnam"],
    "result": ["Viet Minh victory", "N. Vietnamese victory"]
  }
}
```

**Heuristic for data vs. layout tables:**

```rust
fn is_data_table(table: &WikitextTable) -> bool {
    // Data tables have: uniform column count, >2 rows, headers
    let uniform_cols = table.rows.iter().all(|r| r.cells.len() == table.rows[0].cells.len());
    let has_header = table.rows.first().map_or(false, |r|
        r.cells.iter().all(|c| c.is_bold || c.style.contains("th"))
    );
    uniform_cols && table.rows.len() > 2 && has_header
}
```

---

## 6. SpaceTime Index Construction

### 6.1 Spatial Extraction

Each entity with coordinates gets a **spatial index entry**:

```rust
pub struct SpaceTimeNode {
    pub node_id: u32,          // Entity ID in string table
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: Option<f64>,
    pub time_start: Option<i64>,  // Unix timestamp or year
    pub time_end: Option<i64>,    // Null if single point in time
    pub spatial_precision: SpatialPrecision,
}

pub enum SpatialPrecision {
    Exact,      // From coordinates (building, city center)
    Approximate, // From region/area centroid
    Inferred,   // From text mention (geocoded)
}
```

**Coordinate sources (prioritized):**

1. **Infobox coordinates** (highest precision) — `{{coord|21.03|105.85}}`
2. **Geo microformat** — `<span class="geo">21.03; 105.85</span>`
3. **Category-based** — If article is in "Populated places in X", inherit region centroid
4. **Text-based** — Extract location names, geocode via WikiData (lowest precision)

### 6.2 Temporal Extraction

Time anchors connect entities to historical periods:

| Source | Example | Precision |
|--------|---------|-----------|
| Infobox: `date = 1954` | "Battle of Dien Bien Phu" → 1954 | Exact |
| Infobox: `birth_date = 1890-05-19` | "Ho Chi Minh" → lifetime 1890-1969 | Exact |
| Category: "1954 in Vietnam" | Any article in category → year 1954 | Year |
| Text: `"founded in 1010"` | "Hanoi" → 1010 | Approximate |
| Sequential link: `"the [[1968]] battle"` | Context entity → 1968 | Approximate |

**Temporal resolution hierarchy:**

```
┌──────────────────┐
│   Exact date     │  ISO 8601 (1890-05-19) → store as is
├──────────────────┤
│   Year + month   │  "March 1954" → 1954-03
├──────────────────┤
│   Decade         │  "the 1960s" → 1960-1969
├──────────────────┤
│   Century        │  "15th century" → 1401-1500
├──────────────────┤
│   Era            │  "Colonial period" → mapped via country eras
└──────────────────┘
```

### 6.3 R-Tree Spatial Index

The SpaceTime index uses a **2D R-tree** (for spatial queries) with time as a **third dimension**:

```rust
pub struct SpaceTimeIndex {
    pub rtree: RTree<SpaceTimeNode>,   // Spatial (lat/lon)
    pub temporal_index: BTreeMap<i64, Vec<u32>>,  // Time → node IDs
    pub spatiotemporal: Vec<SpatioTemporalCell>,   // Space+Time grids
}
```

**Query types:**

| Query | Use case |
|-------|----------|
| `entities_within(lat, lon, 50km)` | "What happened within 50 km of here?" |
| `entities_in_year(1954)` | "What happened in 1954?" |
| `entities_near(lat, lon, 10km, 1960, 1975)` | "What happened in this area during the Vietnam War?" |
| `path_between(Hanoi, Saigon, 1960)` | "How were Hanoi and Saigon connected in 1960?" |

---

## 7. Weighted Graph Model

### 7.1 Edge Types

```
Edge types form a taxonomy:

┌──────────────────────────────────────────────────────────────┐
│                        HyperEdge                              │
├────────────┬───────────┬───────────┬───────────┬─────────────┤
│  Typed     │ Inferred  │Structural │ Temporal  │  Spatial    │
│  (P-props) │(co-occur) │(infobox)  │(time adj.)│(geo adj.)   │
├────────────┼───────────┼───────────┼───────────┼─────────────┤
│ P19 (birth)│ 0.83      │ "capital" │ same-era  │ 50 km       │
│ P17 (cntry)│ 0.71      │ "battle"  │ precedes  │ 200 km      │
│ P106 (occ) │ 0.65      │ "member"  │ follows   │ 1000 km     │
│ P580 (strt)│ 0.58      │ "result"  │ overlaps  │ contains    │
└────────────┴───────────┴───────────┴───────────┴─────────────┘
```

### 7.2 Weight Normalization

Raw co-occurrence counts are normalized to [0, 1]:

```
normalized_weight(raw) = raw / (raw + median)

  raw = 100 co-occurrences → 100 / (100 + 30) = 0.77
  raw = 10  co-occurrences → 10  / (10 + 30)  = 0.25
  raw = 1   co-occurrence  → 1   / (1 + 30)   = 0.03
```

**Type-specific weight modifiers:**

| Edge type | Base weight | Multiplier | Rationale |
|-----------|------------|------------|-----------|
| Typed (from infobox) | 1.0 | × 1.0 | Highest confidence |
| Typed (from anchor) | 0.9 | × 0.8 | High confidence |
| Inferred (co-occur) | 0.6 | × 1.0 | Medium confidence |
| Inferred (section) | 0.5 | × 1.1 | Slightly better context |
| Temporal | 0.4 | × 0.8 | Approximate |
| Spatial | 0.3 | × 0.7 | Weak (proximity ≠ relation) |

### 7.3 Global Edge Weighting

To prevent popular entities from dominating, we apply **TF-IDF style normalization**:

```
global_weight(A, B) = local_weight(A, B) × log(N / df(B))
```

Where:
- `N` = total entities in graph
- `df(B)` = number of entities connected to B

This demotes edges to entities like "United States" (which is linked from almost every article) and promotes edges to niche but important entities.

---

## 8. Storage: Compact Binary Format

### 8.1 `graph.bin.hg` Specification

```
┌──────────────────────────────────────────────────────┐
│  MAGIC: 4 bytes ("MNHG")                             │
│  VERSION: u32 little-endian                          │
│  HEADER_SIZE: u32                                    │
├──────────────────────────────────────────────────────┤
│  HEADER (24 bytes)                                   │
│  ├── country_code: [u8; 4]  ("vn\0\0")              │
│  ├── node_count: u32                                 │
│  ├── edge_count: u32                                 │
│  ├── weighted_edge_count: u32                        │
│  └── spacetime_node_count: u32                       │
├──────────────────────────────────────────────────────┤
│  STRING TABLE                                        │
│  ├── string_count: u32                               │
│  ├── offsets: [u32; string_count]                    │
│  └── utf8_data: [u8; ...]                           │
├──────────────────────────────────────────────────────┤
│  NODE TABLE (fixed-size per node)                    │
│  ├── For each node: 24 bytes total                   │
│  │   ├── id_str_index: u32    (→ "Q1858")           │
│  │   ├── label_index: u32     (→ "Hanoi")           │
│  │   ├── label_local_index: u32 | 0xFFFFFFFF        │
│  │   ├── node_type: u8                              │
│  │   ├── metadata_count: u16                        │
│  │   └── metadata_start: u32 (offset into MD table) │
│  └── ... × node_count                                │
├──────────────────────────────────────────────────────┤
│  METADATA TABLE                                      │
│  ├── For each entry:                                 │
│  │   ├── key_index: u32                             │
│  │   ├── value_index: u32                           │
│  │   └── value_type: u8                             │
│  └── ...                                             │
├──────────────────────────────────────────────────────┤
│  EDGE TABLE (16 bytes per edge)                      │
│  ├── For each edge:                                  │
│  │   ├── source: u32                                │
│  │   ├── target: u32                                │
│  │   ├── property_index: u32  (→ "P19")            │
│  │   └── weight: f32                                │
│  └── ... × edge_count                                │
├──────────────────────────────────────────────────────┤
│  WEIGHTED EDGE TABLE (variable)                      │
│  ├── For each:                                       │
│  │   ├── source, target, type_index                 │
│  │   ├── weight: f32                                │
│  │   ├── confidence: f32                            │
│  │   ├── signal_count: u16                          │
│  │   ├── source_count: u16                          │
│  │   └── source_articles: [u32; source_count]       │
│  └── ... × weighted_edge_count                       │
├──────────────────────────────────────────────────────┤
│  SPACETIME INDEX                                     │
│  ├── R-tree nodes: (lat, lon) bounds                │
│  ├── For each spacetime node:                       │
│  │   ├── node_id: u32                               │
│  │   ├── lat: f64, lon: f64                         │
│  │   ├── time_start: i64                            │
│  │   └── time_end: i64                              │
│  └── ... × spacetime_node_count                      │
└──────────────────────────────────────────────────────┘
```

**Size estimates at scale:**

| Entity count | Edge count | Raw `.bin.hg` | zstd compressed | Equivalent JSON |
|-------------|-----------|---------------|-----------------|-----------------|
| 100,000 | 500,000 | ~21 MB | ~6 MB | ~150 MB |
| 500,000 (target) | 2,500,000 | ~105 MB | ~30 MB | ~750 MB |
| 1,000,000 (extreme) | 5,000,000 | ~210 MB | ~60 MB | ~1.5 GB |
| 2,000,000 (max) | 10,000,000 | ~420 MB | ~120 MB | ~3 GB |

**Breakdown for 500k entities target (512 MB budget):**

| Section | Size | % of total |
|---------|------|-----------|
| String table (300k unique strings × 15 avg) | ~4.5 MB | 4% |
| Node table (500k × 24 bytes) | 12.0 MB | 11% |
| Metadata table (1.5M entries × 13 bytes) | 19.5 MB | 19% |
| Edge table (2.5M × 16 bytes) | 40.0 MB | 38% |
| Weighted edge table (1M × ~24 avg) | 24.0 MB | 23% |
| SpaceTime index (50k × 48 + R-tree) | ~2.5 MB | 2% |
| Embedding vectors (500k × 384 dim × f16) | ~375 MB | — (stored separately) |
| **Hypergraph total** | **~105 MB** | |
| **+ embeddings** | **~480 MB** | **Fits in 512 MB target** |

Embeddings (384-dim f16 vectors) dominate the budget. They can be stored separately and loaded on demand.

### 8.2 Compression

The `.bin.hg` file is further compressed with **zstd** (level 3):

| Format | 500k entities | Ratio |
|--------|--------------|-------|
| Raw `.bin.hg` | ~105 MB | 1.0 |
| `.bin.hg` + zstd | ~30 MB | 3.5× |
| Equivalent JSON | ~750 MB | 0.14× |

---

## 9. Portals, Wormholes & Alien Namespaces

### 9.1 The Problem: Entities Live in Multiple Countries

A hypergraph per country is clean, but reality is messy:

- Vietnam's data references the **United States** (Vietnam War, normalized relations)
- US data references **Vietnam** (Vietnam War, Vietnamese Americans)
- **World War II** entities appear in every European country's data
- **Albert Einstein** (Q937) is both German (born) and American (naturalized) and Swiss (patents)
- **COVID-19** is a global entity that touches every country

If each repo duplicates these shared entities, we get:
- Data bloat (same entity stored N times)
- Update inconsistency (which repo has the latest version?)
- No cross-repo traversal (agent can't follow edges across country boundaries)

**Solution: Portals/Wormholes** — cross-repo references that allow entities to be "imported" from an alien namespace without duplication.

---

### 9.2 Conceptual Model

```
┌─────────────────────────────┐     ┌─────────────────────────────┐
│     minidi-vn-data          │     │     minidi-en-data          │
│                             │     │                             │
│  Q8740 (Vietnam War)        │     │  Q30 (United States)        │
│    ├── P17: Vietnam ◄───────┼─────┼──► P150: States...          │
│    ├── P710: United States ─┼─────┼──► P27: US citizens         │
│    │                        │     │                             │
│    │    ╔══════════════╗    │     │    ╔══════════════════╗     │
│    └────║  PORTAL to   ║────┼─────┼────║  PORTAL to       ║     │
│         ║ minidi-en.   ║    │     │    ║ minidi-vn.       ║     │
│         ║ hist-wwii    ║    │     │    ║ hist-vn-war      ║     │
│         ╚══════════════╝    │     │    ╚══════════════════╝     │
│                             │     │                             │
│  Entity namespace: vn       │     │  Entity namespace: en       │
│  Native language: vi        │     │  Native language: en        │
└─────────────────────────────┘     └─────────────────────────────┘
         │                                    │
         │           ╔══════════════╗          │
         └───────────║  WORMHOLE   ║──────────┘
                     ║  Q8740 ↔ Q30║
                     ║  bidirectional║
                     ╚══════════════╝
```

**Key concepts:**

| Concept | Analogy | Definition |
|---------|---------|------------|
| **Portal** | Stargate door | A manifest entry pointing to another repo's partition or entity |
| **Wormhole** | Einstein-Rosen bridge | A bidirectional edge connecting specific entities across repos |
| **Alien namespace** | Foreign address | A namespace prefix (`en:`, `de:`, `fr:`) marking entities that live in another repo |

---

### 9.3 Portal Types

#### Type 1: Entity Portal

Points to a single entity in another repo. Used when a specific foreign entity is frequently referenced.

```json
{
  "name": "portal-einstein",
  "type": "entity",
  "namespace": "de",
  "target_repo": "minidi-de-data",
  "target_entity": "Q937",
  "target_url": "https://raw.githubusercontent.com/midivn/minidi-de-data/main/v1/partitions/people-scientists.json",
  "target_entity_url": "https://raw.githubusercontent.com/midivn/minidi-de-data/main/index.json#Q937",
  "label": "Albert Einstein",
  "local_label": "Einstein (nhà vật lý)",
  "bidirectional": true,
  "context": "German-born physicist, Vietnamese physics education references",
  "weight": 0.85
}
```

#### Type 2: Partition Portal

Points to an entire partition file in another repo. Used when a whole category of foreign entities is relevant.

```json
{
  "name": "portal-wwii-to-usa",
  "type": "partition",
  "namespace": "en",
  "target_repo": "minidi-en-data",
  "target_partition": "hist-wwii",
  "target_url": "https://raw.githubusercontent.com/midivn/minidi-en-data/main/v1/partitions/hist-wwii.json",
  "description": "All US WWII entities — battles, generals, home front",
  "bidirectional": true,
  "reverse_name": "portal-vnwar-to-vietnam",
  "reverse_partition": "hist-vn-war",
  "context": "WWII influenced Vietnam's 1945 independence",
  "estimated_entities": 15000,
  "estimated_size_bytes": 3200000
}
```

#### Type 3: Query Portal

A dynamic portal that resolves based on a SPARQL-like query across repos. Entities are fetched on demand.

```json
{
  "name": "portal-global-events",
  "type": "query",
  "namespace": "global",
  "query": {
    "relation": "P710",
    "target": "Q30",
    "date_range": [1941, 1945]
  },
  "target_repos": ["minidi-en-data", "minidi-de-data", "minidi-fr-data", "minidi-ru-data"],
  "description": "All countries involved in WWII home-front entities",
  "dynamic": true,
  "ttl_seconds": 86400
}
```

---

### 9.4 Manifest Registry

Portals live in `portals.json` at the repo root — separate from the manifest so it can be resolved lazily:

```
minidi-vn-data/
├── manifest.json          # Native partitions
├── portals.json            ★ Cross-repo references
├── index.json
├── cursor.json
└── v1/
```

**`portals.json` structure:**

```json
{
  "portal_version": "1.0",
  "repo": "minidi-vn-data",
  "country": "vn",
  "updated_at": "2025-06-01T06:00:00Z",
  "default_resolver": "https://raw.githubusercontent.com/midivn",
  "portals": [
    {
      "name": "en-main",
      "type": "entity",
      "namespace": "en",
      "target_repo": "minidi-en-data",
      "target_url": "index.json",
      "description": "US core entities accessible from VN data",
      "bidirectional": true
    },
    {
      "name": "en-wwii",
      "type": "partition",
      "namespace": "en",
      "target_repo": "minidi-en-data",
      "target_partition": "hist-wwii",
      "target_url": "v1/partitions/hist-wwii.json",
      "context": "WWII → Vietnam independence 1945",
      "bidirectional": true,
      "reverse_name": "vn-war",
      "reverse_partition": "hist-vn-war"
    },
    {
      "name": "de-scientists",
      "type": "partition",
      "namespace": "de",
      "target_repo": "minidi-de-data",
      "target_partition": "people-scientists",
      "target_url": "v1/partitions/people-scientists.json",
      "context": "German scientists referenced in Vietnamese education"
    },
    {
      "name": "fr-colonial",
      "type": "partition",
      "namespace": "fr",
      "target_repo": "minidi-fr-data",
      "target_partition": "hist-colonial",
      "target_url": "v1/partitions/hist-colonial.json",
      "context": "French colonial period in Indochina"
    },
    {
      "name": "zh-dynasties",
      "type": "partition",
      "namespace": "zh",
      "target_repo": "minidi-zh-data",
      "target_partition": "hist-qing",
      "target_url": "v1/partitions/hist-qing.json",
      "context": "Chinese domination of Vietnam 111 BCE-939 CE"
    },
    {
      "name": "global-covid",
      "type": "query",
      "namespace": "global",
      "query": { "qid": "Q84263196" },
      "target_repos": ["minidi-en-data", "minidi-zh-data", "minidi-hi-data"],
      "dynamic": true
    }
  ],
  "namespaces": {
    "native": "vn",
    "alien": ["en", "de", "fr", "zh", "global"]
  }
}
```

---

### 9.5 Alien Entity Encoding

When a partition file references an alien entity, the entity carries a namespace marker:

```json
// v1/partitions/hist-vn-war.json (in minidi-vn-data)
[
  {
    "id": "Q8740",
    "l": "Chiến tranh Việt Nam",
    "t": "event",
    "u": "https://www.wikidata.org/wiki/Q8740",
    "m": {
      "start_time": "1955",
      "end_time": "1975"
    }
  },
  {
    "id": "Q30",
    "ns": "en",                    // ← ALIEN: lives in minidi-en-data
    "l": "Hoa Kỳ",                // Label is Vietnamese (localized)
    "t": "place",
    "portal": "en-main",           // ← Which portal to fetch full data from
    "u": "https://www.wikidata.org/wiki/Q30",
    "m": {
      "role": "participant"        // Context: role in THIS partition
    }
  },
  {
    "id": "Q8740",
    "ns": "en",                    // ← Same entity, EN's perspective
    "l": "Vietnam War",
    "t": "event",
    "portal": "en-wwii",           // Different portal context
    "m": {
      "casualties": "1.3M-3.8M",
      "result": "North Vietnamese victory"
    }
  }
]
```

The `ns` field (namespace) tells the consumer:
- If `ns` is absent → native entity, fully contained in this partition
- If `ns` is `"en"` → alien stub, fetch full data from `portals["en-main"]`
- The stub carries **localized labels** (Vietnamese name for US) + **context-specific metadata** (role in this partition)

---

### 9.6 Wormhole Resolution Protocol

When an AI agent traverses the hypergraph, wormholes resolve as follows:

```
1. Agent calls   neighbors("Q8740")         // Vietnam War (in vn data)
2. Engine checks local edges               // → P17: Vietnam, P710: United States, ...
3. Engine hits entity Q30 with ns="en"     // → Alien stub found
4. Engine looks up portal "en-main"        // → portals.json → en-main
5. Engine fetches target_url from en repo:
   https://raw.githubusercontent.com/midivn/minidi-en-data/main/index.json
6. Engine finds Q30 full entity data       // → Complete US entity
7. Engine returns merged result:
   { id: "Q30", label: "Hoa Kỳ", ...     // Vietnamese label from stub
     full_data: { l: "United States",     // English data from portal
                  population: "331M", ... }
   }
```

**Cache behavior:**

| Portal type | Cache policy | Freshness |
|------------|-------------|-----------|
| Entity | Cache fetched entity for session | Stale-while-revalidate |
| Partition | Cache entire partition file | TTL: 1 hour |
| Query | Cache results with TTL | TTL: configurable |

---

### 9.7 Traversal API with Portals

```rust
/// Extended traversal that can cross repo boundaries via portals.
pub trait HyperGraphTraversal {
    // ... standard methods ...

    /// Resolve an alien entity from its portal.
    /// Returns full entity data from the remote repo.
    fn resolve_alien(&self, entity_id: &str, namespace: &str) -> Result<EntityView>;

    /// Follow a wormhole to a different namespace/repo.
    /// Teleports the traversal context to the target repo.
    fn traverse_wormhole(&self, portal_name: &str) -> Result<WormholeView>;

    /// Get all portals from the current repo's portals.json.
    fn list_portals(&self) -> Vec<PortalDef>;

    /// Check if a portal is still valid (TTL not expired).
    fn validate_portal(&self, portal_name: &str) -> bool;
}
```

**Agent interaction with wormholes:**

```python
graph = HyperGraph.load("minidi-vn-data/graph.bin.hg")

# Step 1: Find the Vietnam War
war = graph.search("Vietnam War")[0]
# → Q8740

# Step 2: Get neighbors — some are local, some are alien
neighbors = graph.neighbors(war.id)
# → [Vietnam (native), United States (alien:en), China (alien:zh), ...]

# Step 3: Resolve alien entity through portal
us = graph.resolve_alien("Q30", "en")
# → Full United States entity from minidi-en-data

# Step 4: Follow wormhole to US WWII data
# One hop: VN war → wormhole → US WWII
wwii = graph.traverse_wormhole("en-wwii")
# → All US WWII entities available as if they were local

# Step 5: Cross-repo path traversal
path = graph.follow_path(
    start="Q8740",                    # Vietnam War (vn)
    via_wormhole="en-wwii",           # → US WWII
    relation="participant",
    depth=3
)
# → Vietnam War → United States (portal:en)
#              → WWII (US partition)
#              → Franklin D. Roosevelt (US)
#              → New Deal (US, economic context)
```

### 9.8 Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Portals in separate `portals.json`** | Keeps manifest clean. Portals can be resolved lazily (only fetched when traversal crosses boundaries). |
| **Alien stubs carry localized labels** | Ensures the consumer can display something meaningful before resolving. "Hoa Kỳ" is useful even without fetching full English data. |
| **Bidirectional by default** | If VN references EN's WWII, EN should also reference VN's Vietnam War. The `reverse_name` field maintains symmetry. |
| **Query portals are dynamic** | Some relationships cross too many repos to enumerate statically. Query portals resolve at traversal time. |
| **Namespace prefix convention** | `en:Q30`, `de:Q937`, `fr:Q142` — alien IDs are distinguishable at a glance. |

---

## 10. AI Agent Traversal API

The resulting hypergraph is designed for traversal by AI agents:

```rust
/// The traversal API that AI agents use to navigate the hypergraph.
pub trait HyperGraphTraversal {
    /// Start from one or more seed entity IDs.
    fn seed(&self, qids: &[&str]) -> Vec<EntityView>;

    /// Get all entities connected to this one (1-hop).
    fn neighbors(&self, qid: &str, options: &TraversalOptions) -> Vec<NeighborView>;

    /// Follow a path: A → B → C with edge constraints.
    fn follow_path(&self, start: &str, relation: &str, depth: u32) -> Vec<PathView>;

    /// SpaceTime query: what exists near (lat, lon, time)?
    fn spacetime_query(&self, query: &SpaceTimeQuery) -> Vec<EntityView>;

    /// Full-text search over node labels and descriptions.
    fn search(&self, query: &str, limit: usize) -> Vec<SearchResult>;
}

pub struct TraversalOptions {
    pub min_weight: f32,           // Filter edges by weight
    pub relation_types: Vec<String>, // Only follow specific relations
    pub max_results: usize,        // Limit results
    pub include_metadata: bool,    // Include entity metadata
}
```

**Example agent interaction:**

```python
# AI agent exploring the Vietnam War era
graph = HyperGraph.load("minidi-vn-data/graph.bin.hg")

# Step 1: Find the Vietnam War entity
war = graph.search("Vietnam War")[0]
# → EntityView { id: "Q8740", label: "Vietnam War", type: Event }

# Step 2: Traverse to connected entities
key_figures = graph.neighbors(war.id, TraversalOptions(
    relation_types=["participant", "commander", "location"],
    min_weight=0.7,
    max_results=20
))
# → [Ho Chi Minh (0.95), Vo Nguyen Giap (0.91), Hanoi (0.85), ...]

# Step 3: Deep dive on a specific person
giap = graph.follow_path(war.id, "commander", depth=2)
# → Vo Nguyen Giap → Battle of Dien Bien Phu (commanded)
#                  → Hanoi (associated location)
#                  → 1954 (temporal anchor)

# Step 4: SpaceTime query
events_1968 = graph.spacetime_query(SpaceTimeQuery(
    lat=16.0, lon=108.0, # Central Vietnam
    radius_km=200,
    year=1968
))
# → Tet Offensive, Battle of Hue, My Lai Massacre, ...
```

---

## 11. Comparison with Alternatives

| Approach | Storage | Graph quality | Freshness | Complexity | Cost |
|----------|---------|--------------|-----------|-----------|------|
| **MinidiSpider (this design)** | **~30 MB/country** | **Good** (50k entities) | **Medium** (weekly) | **Medium** | **Free** |
| Full Wikipedia XML dump | 60 GB | Best (6.9M entities) | Snapshot | High (parsing) | $60/mo compute |
| WikiData SPARQL only | ~5 MB | Poor (sparse edges) | Live | Low | Free |
| DBpedia (Wikipedia parsed) | ~40 GB | Good (1M entities) | Stale (months) | Medium | $40/mo hosting |
| ConceptNet5 | ~5 GB | Good, but generic | Stale (years) | Low | Free |
| Google Knowledge Graph | API only | Best | Live | Low | $5/1000 queries |

**MinidiSpider's niche:** Per-country, compact, traversable hypergraphs that fit in memory, on disk, and in AI context windows. Not a replacement for full Wikipedia — a focused extraction for AI-powered exploration.

---

## 12. Implementation Roadmap

### Sprint 1: Page Fetcher + Link Traveler
- [ ] Wikipedia REST API client (`src/sources/wikipedia/crawler.rs`)
- [ ] BFS frontier with relevance scoring (`src/sources/wikipedia/traveler.rs`)
- [ ] Rate limiting, retry, backoff
- [ ] sled page cache (dedup + checkpoint)

### Sprint 2: Content Extraction
- [ ] Infobox HTML parser (extract key-value pairs from HTML tables)
- [ ] Category → entity type inference
- [ ] QID resolution (batch API calls)
- [ ] Section detection from HTML headings

### Sprint 3: Relationship Mining
- [ ] Co-occurrence edge builder
- [ ] Anchor text pattern matcher
- [ ] Section context → relation type mapper
- [ ] Weighted edge merging and dedup

### Sprint 4: SpaceTime + Tables
- [ ] Coordinate parser (DMS → decimal degrees)
- [ ] Date extractor (natural language → ISO 8601)
- [ ] Wikitext table → hyperedge converter
- [ ] R-tree spatial index

### Sprint 5: Binary Format + Export
- [ ] `graph.bin.hg` writer (string table, node/edge tables)
- [ ] `graph.bin.hg` reader/traversal API
- [ ] zstd compression
- [ ] Integration with existing JSON export pipeline

### Sprint 6: AI Agent API
- [ ] `HyperGraphTraversal` trait implementation
- [ ] Python bindings (via PyO3 or FFI)
- [ ] Example notebooks
- [ ] Documentation

---

## References

1. Wikipedia REST API documentation: https://en.wikipedia.org/api/rest_v1/
2. WikiData Entity API: https://www.wikidata.org/w/api.php
3. "DBpedia: A Nucleus for a Web of Open Data" (Auer et al., 2007)
4. "YAGO: A Core of Semantic Knowledge" (Suchanek et al., 2007)
5. "ConceptNet 5.5: An Open Multilingual Graph of General Knowledge" (Speer et al., 2017)
6. R-tree spatial indexing: Guttman (1984)
