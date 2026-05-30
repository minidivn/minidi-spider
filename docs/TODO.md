# MinidiSpider — TODO

## Phase 1: Core Engine (⚠️ In Progress)
- [ ] **P1** Define HyperGraph data model (`src/graph/`)
- [ ] **P1** Implement `sled`-backed graph storage
- [ ] **P1** WikiData SPARQL crawler (`src/crawler/`)
- [ ] **P2** Tantivy full-text indexer

## Phase 2: Vietnam Dataset
- [ ] **P1** SPARQL query: all Vietnamese cities + provinces + districts
- [ ] **P1** SPARQL query: Vietnamese historical events (battles, dynasties)
- [ ] **P1** SPARQL query: Famous Vietnamese people (scientists, writers, politicans)
- [ ] **P2** SPARQL query: Cultural heritage sites, festivals
- [ ] **P2** Fetch English + Vietnamese labels for all entities
- [ ] **P3** Fetch coordinate data → enable map view

## Phase 3: Export & Frontend
- [ ] **P1** Build `index.json` exporter (flat JSON for frontend consumption)
- [ ] **P1** Build HTML search page with vanilla JS
- [ ] **P1** Integrate Transformers.js for semantic search
- [ ] **P2** BM25 fallback search (Tantivy compiled to WASM or pre-ranked)
- [ ] **P2** Show entity relationships as interactive graph (D3.js / vis.js)
- [ ] **P3** Map view for geographic entities

## Phase 4: Deployment
- [ ] **P1** Export `index.json` + `index.html` to `docs/` for GitHub Pages
- [ ] **P1** GitHub Actions: weekly re-crawl + re-deploy
- [ ] **P2** Configure custom domain or project page
- [ ] **P3** Push to `midivn/minidi-spider`

## Phase 5: Polish
- [ ] **P2** Responsive design for mobile
- [ ] **P2** Pagination / infinite scroll for search results
- [ ] **P2** Vietnamese language tokenizer support for BM25
- [ ] **P3** Image thumbnails (WikiData Commons images)
- [ ] **P3** Offline PWA support
