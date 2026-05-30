# Data Organization Strategy — MinidiSpider

## Repo Naming

**Primary suggestion: `minidi-vn-data`**

| Option | Pros | Cons |
|--------|------|------|
| `minidi-vn-data` | Short, project-prefixed, scope-clear | None significant |
| `minidi-data-wikidata-vietnam` | Maximum explicit | Too long (34 chars), repetitive |
| `vn-hypergraph` | Brandable, clean | Loses "minidi" lineage |
| `wikidata-vn` | Recognizable pattern | Generic, no project tie |

**Recommendation:** `minidi-vn-data` — GitHub handle `midivn/minidi-vn-data`.  
Pages URL: `https://midivn.github.io/minidi-vn-data/`

---

## Branch Topology

```
main ─────────────────────┬───────────────► (deploy to Pages)
   \                    /  \            /
    develop ──► PR ────    └─── fix ────

data/latest (moving tag, updated on each successful crawl)

backfill/* (one-off data import branches, squash-merged to develop)
```

| Branch | Purpose | Lifecycle |
|--------|---------|-----------|
| `main` | Stable published data. Git-tagged. Deploys to GH Pages | Permanent |
| `develop` | Latest crawl results. Schema-validated | Permanent |
| `data/latest` | Moving tag pointing at most recent successful crawl commit | Tag, moves |
| `backfill/<era>` | Backfill specific historical era (e.g., `backfill/french-colonial`) | Temporary, deleted after merge |
| `fix/*` | Data quality fixes (wrong labels, missing relations) | Temporary |

**Merge rules:**
- `develop` → `main`: Only via PR after validation (CI checks exist, schema valid)
- Direct push to `main`: Banned (branch protection)
- `backfill/*` → `develop`: Squash merge

---

## Tagging Strategy

**Calendar versioning** (semantic versioning for data is misleading):

```
v2025.06.1
│ │  │ └── Revision (incremented per release in same month)
│ │  └──── Month
│ └─────── Year
└───────── v prefix
```

| Tag | What it means | When created |
|-----|---------------|-------------|
| `v2025.06.1` | First June 2025 release | On merge to main after crawl |
| `v2025.07.1` | First July 2025 release | After next month's crawl |
| `v2025.07.2` | Second July release | Hotfix or re-crawl |
| `data/latest` | Movable tag | After every successful crawl |

**Tag lifecycle:**
1. Weekly crawl completes → commit to develop
2. PR merged to main → manual or auto-tag with `vYYYY.MM.REVISION`
3. `data/latest` moved to same commit

---

## Filesystem Namespace

All exported data lives under `docs/` (GH Pages root).

```
docs/
├── index.html                    # Frontend search app (Transformers.js)
├── index.json                    # Full export — all nodes + edges
├── index.lite.json               # Lightweight — node IDs + labels only
│
├── v1/                           # Schema version namespace
│   ├── entities/                 # Partitioned by entity type
│   │   ├── place.json            #   All Place-type nodes
│   │   ├── person.json           #   All Person-type nodes
│   │   ├── event.json            #   All Event-type nodes
│   │   ├── concept.json          #   All Concept-type nodes
│   │   ├── organization.json     #   All Organization-type nodes
│   │   └── artifact.json         #   All Artifact-type nodes
│   │
│   ├── regions/                  # Partitioned by geography
│   │   ├── north.json            #   Northern Vietnam entities
│   │   ├── central.json          #   Central Vietnam entities
│   │   ├── south.json            #   Southern Vietnam entities
│   │   └── unlocated.json        #   Entities without geo data
│   │
│   ├── timeline/                 # Partitioned by historical era
│   │   ├── era-paleolithic.json  #   ~50,000 – 3000 BCE
│   │   ├── era-hong-bang.json    #   2879 – 258 BCE (Hồng Bàng dynasty)
│   │   ├── era-chinese-dom.json  #   111 BCE – 939 CE (Chinese domination)
│   │   ├── era-dynastic-vn.json  #   939 – 1858 (Independent dynasties)
│   │   ├── era-colonial.json     #   1858 – 1954 (French colonial)
│   │   ├── era-vietnam-war.json  #   1955 – 1975 (Vietnam War era)
│   │   ├── era-modern.json       #   1975 – present
│   │   └── era-unknown.json      #   Entities without dates
│   │
│   ├── relations/                # Partitioned by relation category
│   │   ├── administrative.json   #   P150, P131 (admin hierarchy)
│   │   ├── geographic.json       #   P17, P36, P47 (geo relations)
│   │   ├── temporal.json         #   P580, P582, P585 (time relations)
│   │   ├── social.json           #   P106, P69, P1416 (human relations)
│   │   └── all.json              #   Every edge
│   │
│   └── schema.json               # Schema version, field descriptions
│
├── _metadata/                    # Audit trail (underscore prefix = not content data)
│   ├── crawl-report-latest.json  #   Timestamp, entity counts, query stats
│   ├── provenance.json           #   SPARQL queries used for each partition
│   ├── changelog.json            #   Per-release diff summary
│   └── schema.json               #   Field definitions, data types
│
├── embeddings.bin                # Pre-computed BOW vectors (binary)
├── embeddings.json               # Pre-computed BOW vectors (JSON)
│
└── version/                      # Version aliases
    └── latest -> ../../v1        # Symlink-like: JSON redirect
```

### Partitioning Rationale

| Partition | Why | Frontend use |
|-----------|-----|-------------|
| **entities/** | Filter by type for faster UI rendering | Show only Places on map view |
| **regions/** | Geographic proximity queries | Highlight entities in a region |
| **timeline/** | Temporal navigation | "Show events from the 15th century" |
| **relations/** | Edge-type specific queries | Graph visualization edge filtering |

### Era Boundaries for Vietnam

Eras defined in `v1/timeline/` follow Vietnamese historiography:

| Era | Date range | Key events |
|-----|-----------|------------|
| Paleolithic | ~50,000 – 3000 BCE | Sơn Vi culture, Hòa Bình culture |
| Hồng Bàng | 2879 – 258 BCE | Legendary Hùng kings, Văn Lang state |
| Chinese Domination | 111 BCE – 939 CE | Triệu, Han, Sui, Tang rule. Trưng Sisters rebellion (40 CE) |
| Dynastic Vietnam | 939 – 1858 | Ngô, Đinh, Lê, Lý, Trần, Lê Sơ, Nguyễn dynasties |
| Colonial | 1858 – 1954 | French conquest, Indochina, WWII |
| Vietnam War | 1955 – 1975 | Partition, War, Reunification |
| Modern | 1975 – present | Đổi Mới, economic growth, modern era |

---

## GitHub Actions Workflows

### 1. Weekly Crawl (`crawl-weekly.yml`)
- **Schedule:** Every Sunday 06:00 UTC
- **On demand:** `workflow_dispatch`
- **Steps:**
  1. Checkout code
  2. Restore Rust binary from cache or build
  3. Run `cargo run --release -- crawl --progress`
  4. Run `cargo run --release -- export --embeddings --output docs`
  5. Generate `_metadata/crawl-report-latest.json`
  6. If `docs/` changed → commit to `develop` + push
  7. Create/update PR: `develop → main`

### 2. Deploy Pages (`deploy-pages.yml`)
- **Trigger:** Push to `main` that changes `docs/**`
- **Steps:**
  1. Validate `docs/index.json` structure
  2. Deploy to GH Pages via `peaceiris/actions-gh-pages`

### 3. Release Tag (`release-tag.yml`)
- **Trigger:** Manual via `workflow_dispatch`
- **Steps:**
  1. Read version from `docs/_metadata/crawl-report-latest.json`
  2. Create tag `vYYYY.MM.REVISION`
  3. Move `data/latest` tag
  4. Update `docs/_metadata/changelog.json`

---

## Schema Versioning

`v1/` prefix allows future schema changes without breaking existing consumers:

- **v1** (current): Flat node + edge JSON, string-typed metadata
- **v2** (future): Could add typed properties, compressed binary formats, indexed lookup files

Each version directory is self-describing via `v1/schema.json`:
```json
{
  "version": "1",
  "description": "Initial schema — flat nodes + edges with compact field names",
  "fields": {
    "nodes.l": "English label",
    "nodes.lv": "Vietnamese label",
    "nodes.t": "Entity type (place/person/event/...)"
  },
  "partitions": ["entities", "regions", "timeline", "relations"],
  "created_at": "2025-06-01T00:00:00Z"
}
```

---

## Example: Full Release Cycle

```
Sunday 06:00 UTC
  │
  ▼
Crawl completes (5000 entities, 12000 edges)
  │
  ▼
Export generates docs/ files:
  ├── index.json (4.2 MB)
  ├── index.lite.json (1.1 MB)
  ├── v1/entities/place.json (1500 nodes)
  ├── v1/timeline/era-dynastic-vn.json (600 nodes)
  └── ...
  │
  ▼
Commit pushed to develop + PR created
  │
  ▼
Reviewer approves → merge to main
  │
  ▼
Tag v2025.06.1 created → deploy-pages.yml fires
  │
  ▼
Pages live at https://midivn.github.io/minidi-vn-data/
```
