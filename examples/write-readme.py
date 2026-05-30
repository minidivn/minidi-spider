#!/usr/bin/env python3
"""Write README.md to the data repo."""

import os

DATA = "G:/i2c/PROJECTS/MiniPlatform/MiniDi/Data/minidi-vn-data"

content = """# minidi-vn-data

**Vietnam Knowledge Graph** — 14,292 WikiData entities exported as a searchable hypergraph.

## Quick Links

| Link | Description |
|------|-------------|
| GitHub Pages | https://minidivn.github.io/minidi-vn-data/ |
| GraphQL API | https://minidi-graphql.i2cvnco.workers.dev/ |
| Crawler tool | https://github.com/minidivn/minidi-spider |
| Data source | https://github.com/minidivn/minidi-vn-data |

## Data Structure

```
docs/
  index.json           Full export: 14,292 entities
  index.lite.json      Compact version (IDs + labels only)
  index.html           Bilingual (EN+VI) search frontend
  v1/entities/         Partitioned by entity type
  v1/timeline/         Partitioned by historical era
  v1/relations/        Partitioned by property category
```

## GraphQL API

Endpoint: https://minidi-graphql.i2cvnco.workers.dev/graphql

Example queries:

```
query Search {
  search(query: "Hanoi", first: 10) {
    nodes { id label description type }
    total
  }
}

query EntityDetail {
  node(id: "Q881") {
    label labelVi type wikidataUrl
    edges { relation target { label } }
  }
}

query Stats {
  graphStats { entityCount edgeCount typeBreakdown { type count } }
}
```

## Stats

- Total entities: 14,292
- Events: 8,049 | People: 6,025 | Places: 218
- Data source: WikiData SPARQL
- License: CC0 - WikiData contributors
- Updated: Weekly via GitHub Actions

## Build

```bash
# Crawl data
cargo run --release -- crawl

# Export to docs/
cargo run --release -- export --embeddings

# Deploy
git push
```
"""

with open(os.path.join(DATA, "README.md"), "w", encoding="utf-8") as f:
    f.write(content)

print(f"README.md written ({len(content)} bytes)")
