# MinidiSpider — Execution Plan

## 🎯 Goal
Crawl WikiData → HyperGraph → Feature-rich static site (GitHub Pages), focused on Vietnam.

## 📦 Deliverable
1. `minidi-spider` CLI binary
2. `docs/index.html` — self-contained feature-rich search page
3. `docs/index.json` — hypergraph dump (~5-50MB)
4. `docs/` — weight dir for Transformers.js model

---

## 🧭 Feature Roadmap

### Phase 1: Core Engine ✅
- [x] Define HyperGraph data model (HyperNode, HyperEdge, NodeType)
- [x] sled-backed persistent graph storage
- [x] WikiData SPARQL crawler with retry + progress
- [x] Full-text search with Tantivy
- [x] JSON export + partitioned exports (type, timeline, relations)

### Phase 2: Frontend — Search & Browse ✅
- [x] BM25 keyword search
- [x] Entity type filters (Place, Person, Event)
- [x] Era timeline filters (Hồng Bàng, Dynastic, Colonial, etc.)
- [x] Entity detail overlay with relations
- [x] Bilingual EN/VI toggle
- [x] Animated loading screen with progress bar
- [x] Vietnam image slideshow (Wikimedia Commons)

### Phase 3: Frontend — Advanced Features 🔨
- [ ] 🤖 **Chat Bot** — AI assistant that answers questions using the local index data
- [ ] 🔤 **Alphabet Index** — Browse entities by first letter (A-Z, including Vietnamese letters)
- [ ] 🗂️ **Group Index** — Browse by entity type, category, era, region
- [ ] 🗺️ **Map View** — OpenStreetMap widget (Leaflet.js) showing geolocated entities
- [ ] 🕰️ **Interactive Timeline** — Zoomable horizontal timeline with events by century/era
- [ ] 👤 **People Detail** — Entity detail with WikiData image, biography, key facts
- [ ] 🌳 **Dynasty Tree** — Visual tree of Vietnamese kings, emperors, dynasties
- [ ] 📸 **Thumbnails** — Load images from WikiData Commons for people & places
- [ ] 🎨 **Better Footer** — Emoji-rich footer with links, stats, attribution
- [ ] ⭐ **GitHub Icon** — Repo badge/link in header

### Phase 4: WikiData Live Lookup 🔄
- [ ] **Live Detail Fetch** — When viewing entity detail, pull fresh data from WikiData API
- [ ] **Image Service** — Fetch Commons images for entity thumbnails
- [ ] **SPARQL Proxy** — Allow custom SPARQL queries from the UI

### Phase 5: Data Organization 📁
- [ ] **Partitioned timeline data** — Pre-computed event-by-era JSON files
- [ ] **Region partitions** — GeoJSON boundaries for provinces
- [ ] **Dynasty tree data** — Pre-computed parent-child relationships from P40/P22

---

## 🧱 Frontend Architecture (Single HTML Page)

```
index.html
├── Loading Splash          ← Animated ring + progress bar
├── Header
│   ├── Logo + MiniDi
│   ├── Search input
│   ├── Lang toggle EN/VI
│   └── View switcher (🔍 🗺️ 🕰️ 🌳 🤖)
├── Views (tabbed)
│   ├── #search            ← Default: BM25 search + filters
│   │   ├── Filter bar (All, Places, People, Events)
│   │   ├── Era bar (Hong Bang → Modern)
│   │   ├── Alphabet index (A-Z #)
│   │   ├── Result cards
│   │   └── Pagination
│   ├── #map               ← Leaflet.js with entity markers
│   │   ├── Full-screen map
│   │   ├── Marker clusters by type
│   │   └── Click → entity detail
│   ├── #timeline          ← Zoomable horizontal timeline
│   │   ├── Century bands
│   │   ├── Event dots
│   │   ├── Zoom in/out
│   │   └── Click → event detail
│   ├── #tree              ← Dynasty/empire tree
│   │   ├── Collapsible tree nodes
│   │   ├── Portrait thumbnails
│   │   └── Click → dynasty detail
│   └── #chat              ← AI assistant panel
│       ├── Chat messages
│       ├── Quick suggestions
│       └── Input + send
├── Detail Overlay          ← Entity detail panel
│   ├── Title (EN + VI)
│   ├── WikiData image
│   ├── Description
│   ├── Metadata table
│   ├── Relations list
│   └── WikiData link
├── Alphabet Index Bar      ← A-Z sidebar
└── Footer                  ← Emoji, stats, links, attribution
```

## 📐 Data Flow

```
         SPARQL fetch           sled write          JSON serialize
  WDQS ───────────▶ Crawler ───────────▶ Store ──────────────▶ index.json
                                          │                       │
                                          ▼                       ▼
                                       Tantivy               index.html
                                       (local search)     (single-page app)
                                                                 │
                                                     ┌───────────┼───────────┐
                                                     ▼           ▼           ▼
                                                 Search     Map+OSM     Timeline
                                                 Chat       Dynasty     Detail
```

## 🚧 Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| 7.7 MB index.json slow to download | XHR progress bar, show MB count |
| Leaflet.js map tiles slow | Use free OpenStreetMap tile server, cache |
| WikiData API rate limiting | Cache responses, batch requests |
| Large DOM from 14k entities | Lazy rendering, 40 per page |
| Vietnamese text rendering | System fonts, Unicode support |

## 🧪 Testing
- Open index.html in browser → verify all views load
- Test search with English and Vietnamese queries
- Verify map markers appear for entities with coordinates
- Check timeline renders events by century
- Validate dynasty tree shows parent-child relations
