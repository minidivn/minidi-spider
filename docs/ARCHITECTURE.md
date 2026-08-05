# MinidiSpider Architecture

## Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                      MinidiSpider Engine                          │
│                                                                  │
│  configs/countries.json  ──►  Crawler (SPARQL / Wikipedia)       │
│         │                       │                                │
│         ▼                       ▼                                │
│  configs/partitions/*.json ──►  Partition Executor                │
│  configs/queries/*.sparql       │                                │
│                                 ▼                                │
│                           HyperGraph Store (sled)                │
│                                 │                                │
│                                 ▼                                │
│                           Export Pipeline                        │
│                                 │                                │
│                    ┌────────────┼────────────┐                   │
│                    ▼            ▼            ▼                   │
│              index.json    v1/partitions/   manifest.json       │
│              (compact)     (< 4MB each)     (index of all)      │
│              cursor.json   (state tracker)  schema.json         │
└──────────────────────────────────────────────────────────────────┘
```

MinidiSpider converts massive public knowledge graphs (WikiData, Wikipedia) into **small, independent, processable JSON chunks** — each under 4 MB — so any AI pipeline, vector database, or frontend can consume them without memory pressure.

---

## Data Repo Structure

Each country gets its own GitHub repo (`minidi-<cc>-data`). The repo IS the database.

```
minidi-en-data/
│
├── index.json                  # Compact flat index (nodes + edges, ~10-50 MB)
├── index.lite.json             # Lightweight: IDs + labels only
│
├── manifest.json                   ★ KEY FILE: catalog of every partition
├── portals.json                    ★ KEY FILE: cross-repo references (wormholes)
├── cursor.json                     ★ KEY FILE: pipeline processing state
│
├── v1/
│   ├── schema.json             # Field definitions, types, version
│   │
│   ├── entities/               # Partitioned by entity type
│   │   ├── place.json          #   < 4 MB — all Place nodes
│   │   ├── person.json         #   < 4 MB
│   │   ├── event.json          #   < 4 MB
│   │   ├── concept.json
│   │   ├── organization.json
│   │   ├── artifact.json
│   │   └── _index.json         #   Summary: types + counts
│   │
│   ├── timeline/               # Partitioned by century
│   │   ├── 19th-century.json   #   < 4 MB
│   │   ├── 1901-1950.json
│   │   ├── 1951-2000.json
│   │   └── _index.json
│   │
│   ├── relations/              # Partitioned by category
│   │   ├── geographic.json
│   │   ├── temporal.json
│   │   ├── social.json
│   │   ├── relations.json
│   │   └── all.json
│   │
│   └── partitions/             ★ Crawl-time partitions (< 4 MB each)
│       ├── adm-north-provinces.json
│       ├── hist-colonial.json
│       ├── people-writers.json
│       ├── culture-festivals.json
│       ├── nature-rivers.json
│       └── ... (50-512 files, each < 4 MB)
│
├── _metadata/
│   ├── provenance.json         # Source queries, timestamps, versions
│   ├── crawl-report-latest.json
│   └── changelog.json
│
└── version/
    └── latest.json
```

### Why this structure?

| Problem | Solution |
|---------|----------|
| Raw WDQS dump is 150+ GB | SPARQL queries extract only what's needed per partition |
| Single JSON > 50 MB crashes browsers | Each partition file is **< 4 MB** |
| Hard to parallelize | Each partition is independent — process 50 files in 50 threads |
| No processing state | `cursor.json` tracks exactly which partitions are done |
| Schema unknown to consumers | `manifest.json` + `v1/schema.json` declare everything |

---

## Partition Strategy

### The Core Insight

A country's data can be broken along **5 axes**:

```
                          ┌──────────────┐
                          │   COUNTRY     │
                          └──────┬───────┘
              ┌──────────────────┼──────────────────┐
              ▼                  ▼                  ▼
        By TYPE             BY TIME            BY RELATION
   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
   │ Place        │   │ Pre-1000     │   │ Geographic   │
   │ Person       │   │ 1000-1500    │   │ Social       │
   │ Event        │   │ 1500-1800    │   │ Temporal     │
   │ Concept      │   │ 1800-1900    │   │ Hierarchical │
   │ Organization │   │ 1900-1950    │   │              │
   │ Artifact     │   │ 1950-2000    │   │              │
   └──────────────┘   │ 2000+        │   └──────────────┘
                      └──────────────┘

        By TOPIC               By OCCUPATION
   ┌──────────────┐   ┌──────────────┐
   │ Admin        │   │ Rulers       │
   │ History      │   │ Military     │
   │ Culture      │   │ Scientists   │
   │ Nature       │   │ Writers      │
   │ Sports       │   │ Artists      │
   │ Economy      │   │ Educators    │
   └──────────────┘   │ Politicians  │
                      │ ...          │
                      └──────────────┘
```

Every partition file is a **self-contained JSON array** of uniform entities.

### 4 MB Rule

Each partition file stays **under 4 MB**. This is a hard architectural constraint regardless of total budget. Here's why:

- 4 MB loads in < 200ms over 3G
- Fits in L1/L2 cache on most CPUs
- Any JSON parser can handle it without streaming
- A single HTTP fetch covers it
- Even at 512 MB total, each file is independently loadable

```
4000 entities × ~1000 bytes/entity = ~4 MB
↑                                    ↑
typical partition limit              hard cap per file
```

**Scaling with this constraint:**

| Budget | Max partitions | Min partitions | Typical partition count |
|--------|---------------|----------------|------------------------|
| 128 MB (minimum) | 32 | 16 | 20-25 |
| 512 MB (target) | 128 | 64 | 64-128 |
| 1024 MB (extreme) | 256 | 128 | 128-256 |

More partitions = finer granularity = more parallel processing = better AI context fit.

**512 MB / 4 MB = 128 files minimum** — scales to 256+ for the 1 GB extreme tier.

**Partition count vs. file size:**
```
Partitions │ File size   │ Use case
───────────┼─────────────┼──────────────────────
    32     │ ~4 MB       │ Minimum viable
    64     │ ~2 MB       │ Comfortable loading
   128     │ ~1 MB       │ Fast, parallel-friendly
   256     │ ~500 KB     │ Extreme granularity
```

---

## Schema

### `v1/schema.json`

```json
{
  "version": "1",
  "description": "Generic hypergraph schema — flat nodes + edges with compact field names",
  "created_at": "2025-06-01T00:00:00Z",
  "entity_count": 45231,
  "edge_count": 128940,
  "fields": {
    "nodes": {
      "id": "WikiData Q-id (e.g. Q30)",
      "l":  "English label",
      "ll": "Native language label (optional)",
      "d":  "English description",
      "dl": "Native description (optional)",
      "t":  "Entity type: place|person|event|concept|organization|artifact|other",
      "u":  "WikiData URL",
      "m":  "Metadata map (coordinates, dates, occupations)"
    },
    "edges": {
      "s":  "Source node Q-id",
      "r":  "Relation/property label",
      "t":  "Target node Q-id",
      "tl": "Target node label"
    }
  },
  "partitions": {
    "entities": "Split by NodeType",
    "timeline": "Split by century bucket",
    "relations": "Split by property category",
    "partitions": "Split by crawl partition (admin, history, people, culture, nature)"
  }
}
```

### Partition file format

Every partition file in `v1/partitions/*.json` is a plain array:

```json
[
  {
    "id": "Q30",
    "l": "United States of America",
    "ll": null,
    "d": "sovereign state in North America",
    "dl": null,
    "t": "place",
    "u": "https://www.wikidata.org/wiki/Q30",
    "m": {
      "coordinates": "Point(-100.0 40.0)",
      "population": "331900000"
    }
  }
]
```

This is deliberately simple — no nesting, no references. Every entity stands alone.

---

## Manifest

### `manifest.json`

The manifest is the **table of contents** for the entire data repo. Portal references live in a separate `portals.json` (see section below).

The manifest tells consumers:

- What partitions exist
- What type of data each contains
- File size and entity count
- Schema version
- How the partitions relate to each other

```json
{
  "manifest_version": "1.0",
  "repo": "minidi-en-data",
  "country": {
    "code": "en",
    "name": "United States",
    "qid": "Q30"
  },
  "crawl": {
    "timestamp": "2025-06-01T06:00:00Z",
    "version": "0.1.0",
    "workflow_run": "12345678"
  },
  "schema": {
    "version": "1",
    "path": "v1/schema.json"
  },
  "partitions": [
    {
      "name": "adm-states-northeast",
      "category": "admin",
      "path": "v1/partitions/adm-states-northeast.json",
      "size_bytes": 2847561,
      "entity_count": 3421,
      "node_type": "Place",
      "query_file": "shared/admin-divisions.sparql"
    },
    {
      "name": "hist-civil-war",
      "category": "history",
      "path": "v1/partitions/hist-civil-war.json",
      "size_bytes": 1248765,
      "entity_count": 1523,
      "node_type": "Event",
      "query_file": "shared/events-filtered.sparql",
      "filter": "YEAR(?startTime) >= 1861 && YEAR(?startTime) <= 1865"
    },
    {
      "name": "people-scientists",
      "category": "people",
      "path": "v1/partitions/people-scientists.json",
      "size_bytes": 1987332,
      "entity_count": 2187,
      "node_type": "Person",
      "query_file": "shared/people-by-occupation.sparql",
      "occupation_qid": "Q901"
    }
  ],
  "summary": {
    "total_partitions": 50,
    "total_entities": 45231,
    "total_edges": 128940,
    "total_size_bytes": 49876543,
    "max_partition_size_bytes": 3987654,
    "min_partition_size_bytes": 234567
  }
}
```

**Consumer workflow with manifest:**

```
1. Fetch manifest.json        → Know all partitions exist
2. Filter by node_type/       → Select only "Place" partitions
   category/name
3. For each matched partition → Fetch only that < 4 MB file
4. Process independently      → No cross-file dependencies
```

---

## Portals / Wormholes / Alien Namespaces

### Concept

A **portal** (or wormhole) is a cross-repo reference that lets entities from one country's data be "imported" into another's without duplication. This solves the problem of entities that span multiple countries (WWII, Einstein, COVID-19).

Portals live in `portals.json` — separate from `manifest.json` so they can be resolved lazily only when traversal crosses repo boundaries.

### Repo structure with portals

```
minidi-vn-data/
├── manifest.json          # Native partitions only
├── portals.json            ★ Cross-repo wormholes
│     ├── en-main          → minidi-en-data:index.json
│     ├── en-wwii          → minidi-en-data:v1/partitions/hist-wwii.json
│     ├── de-scientists    → minidi-de-data:v1/partitions/people-scientists.json
│     ├── fr-colonial      → minidi-fr-data:v1/partitions/hist-colonial.json
│     └── zh-dynasties     → minidi-zh-data:v1/partitions/hist-qing.json
├── index.json
├── cursor.json
└── v1/
```

### Alien entity encoding

Entities that belong to another repo carry an `ns` (namespace) field:

```json
// In minidi-vn-data/v1/partitions/hist-vn-war.json
[
  {
    "id": "Q8740",
    "l": "Chiến tranh Việt Nam",
    "t": "event",
    "u": "https://www.wikidata.org/wiki/Q8740"
    // no ns → native entity
  },
  {
    "id": "Q30",
    "ns": "en",              // ← ALIEN: lives in minidi-en-data
    "l": "Hoa Kỳ",
    "t": "place",
    "portal": "en-main",     // ← resolves via portals.json
    "u": "https://www.wikidata.org/wiki/Q30"
  }
]
```

### Traversal flow

```
1. Agent calls neighbors("Q8740")        // Vietnam War
2. Engine finds Q30 with ns="en"        // → alien stub
3. Engine looks up portal "en-main"      // → portals.json
4. Engine fetches en-main's target_url  // → minidi-en-data's index.json
5. Engine returns merged result
```

---

## Cursor

### `cursor.json`

The cursor is the **processing state machine**. It tracks:

- Which partitions have been fetched/processed
- Which are pending/in-progress/done
- Timestamps for each step
- Any errors encountered
- Checkpoint for resumable pipelines

```json
{
  "cursor_version": "1.0",
  "repo": "minidi-en-data",
  "country": "en",
  "created_at": "2025-06-01T06:00:00Z",
  "updated_at": "2025-06-01T06:45:00Z",
  "state": "partial",
  "pipeline": {
    "name": "embedding-generator",
    "version": "1.2.0"
  },
  "progress": {
    "total": 50,
    "completed": 32,
    "failed": 1,
    "pending": 17
  },
  "checkpoints": {
    "fetch": {
      "completed_at": "2025-06-01T06:10:00Z",
      "partitions_fetched": 50,
      "failed_fetches": 0
    },
    "embed": {
      "started_at": "2025-06-01T06:10:00Z",
      "progress_pct": 64.0
    }
  },
  "partitions": [
    {
      "name": "adm-states-northeast",
      "state": "done",
      "fetched_at": "2025-06-01T06:07:00Z",
      "processed_at": "2025-06-01T06:12:00Z",
      "size_bytes": 2847561,
      "entity_count": 3421,
      "hash": "sha256:a1b2c3d4...",
      "outputs": {
        "embeddings": "v2/embeddings/adm-states-northeast.bin",
        "vectors": 3421
      }
    },
    {
      "name": "hist-civil-war",
      "state": "done",
      "fetched_at": "2025-06-01T06:07:00Z",
      "processed_at": "2025-06-01T06:13:00Z",
      "size_bytes": 1248765,
      "entity_count": 1523,
      "hash": "sha256:e5f6g7h8..."
    },
    {
      "name": "people-scientists",
      "state": "failed",
      "fetched_at": "2025-06-01T06:07:00Z",
      "error": "Embedding model OOM on batch size 64",
      "retry_count": 2,
      "next_retry_at": "2025-06-01T07:00:00Z"
    },
    {
      "name": "nature-deserts",
      "state": "pending",
      "estimated_size": 450000
    }
  ]
}
```

### Cursor state machine

```
                  ┌──────────┐
                  │  pending │
                  └────┬─────┘
                       │ fetch / discover
                       ▼
                  ┌──────────┐
           ┌──────│ fetching │──────┐
           │      └────┬─────┘      │
           │           │ done       │ error
           ▼           ▼            ▼
     ┌─────────┐ ┌──────────┐ ┌─────────┐
     │ retry   │ │ fetched  │ │ failed  │
     └────┬────┘ └────┬─────┘ └────┬────┘
          │           │ process    │ manual fix
          │           ▼            │
          │      ┌──────────┐      │
          └──────│processing│──────┘
                 └────┬─────┘
                      │ done
                      ▼
                 ┌──────────┐
                 │   done   │
                 └──────────┘
```

### What cursor enables

| Capability | How |
|-----------|-----|
| **Resumable pipelines** | If pipeline crashes at partition 37, cursor survives. Resume from last `fetched` or `failed`. |
| **Incremental updates** | Cursor stores hashes. Only re-process partitions whose hash changed. |
| **Parallel fan-out** | 50 workers each claim 1 partition via cursor. No contention. |
| **Progress reporting** | `progress.completed / total` gives % at a glance. |
| **Error recovery** | Failed partitions get `retry_count` and `next_retry_at`. Exponential backoff. |
| **Data lineage** | Every output file is linked to its source partition hash. Audit trail. |

---

## Pipeline: How < 4 MB Partitions Boost Processing

### The Old Way: One Giant JSON

```
                    ┌──────────────────┐
                    │  index.json       │
                    │  150 MB           │────► OOM
                    │  45000 entities   │────► 30s parse
                    └──────────────────┘────► Single-threaded
                                              Cannot parallelize
                                              Git won't accept >100MB
```

### The New Way: Partitioned Pipeline

```
                    ┌──────────────────┐
                    │  manifest.json    │──► Plan: 50 partitions
                    └──────────────────┘
                            │
              ┌─────────────┼────────────── ─ ─ ─ ─ ┐
              ▼              ▼                         ▼
     ┌──────────────┐ ┌──────────────┐      ┌──────────────┐
     │ adm-north    │ │ people-writer│      │ nature-rivers│
     │ 3.2 MB       │ │ 2.8 MB       │ ...  │ 1.2 MB       │
     └──────────────┘ └──────────────┘      └──────────────┘
              │              │                         │
              ▼              ▼                         ▼
        ┌──────────┐   ┌──────────┐              ┌──────────┐
        │ Process  │   │ Process  │   8 threads   │ Process  │
        │ Thread 1 │   │ Thread 2 │   parallel    │ Thread 8 │
        └──────────┘   └──────────┘              └──────────┘
              │              │                         │
              ▼              ▼                         ▼
        ┌──────────┐   ┌──────────┐              ┌──────────┐
        │ Embed    │   │ Embed    │   ← 4 MB     │ Embed    │
        │ vectors  │   │ vectors  │   fits in     │ vectors  │
        │ .bin     │   │ .bin     │   L2 cache    │ .bin     │
        └──────────┘   └──────────┘              └──────────┘
              │              │                         │
              └──────────────┼─────────────────────────┘
                             ▼
                    ┌──────────────────┐
                    │  Merge / Index   │
                    │  Final output    │
                    └──────────────────┘
```

### Concrete benefits

| Metric | Monolith (150 MB) | Partitioned (< 4 MB each) |
|--------|-------------------|---------------------------|
| Parse time (single thread) | ~30 seconds | ~800 ms per file |
| Parse time (8 threads) | N/A | ~5 seconds total |
| Parse time (64 threads) | N/A | ~800 ms total (128 files) |
| Memory per parse | 500 MB+ | 8-12 MB |
| GPU embedding batch | Only 1 model load | Per-partition optimal batches |
| Retry on failure | Re-do entire 150 MB | Re-do 1 partition |
| Git push | Blocked by GH limit | Smooth (4 MB << 100 MB) |
| Browser XHR load | Timeout risk | < 200 ms |
| AI context window (8K) | Won't fit | Fits 20+ partitions |
| AI context window (128K) | Won't fit | Fits entire partition set |

**Worked example at 512 MB target:**

```
512 MB total / 128 partitions = 4 MB per partition
                                    
128 partitions × 800 ms = 102 seconds sequential
128 partitions / 16 threads = 6.4 seconds parallel
                                    
Total memory: 128 × 10 MB = 1.28 GB (peak)
With 16 workers: 16 × 10 MB = 160 MB (working set)
```