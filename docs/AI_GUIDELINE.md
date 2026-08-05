# 🤖 AI Guideline

Guidelines for AI coding agents (Zed, Cursor, Claude Code, Copilot, etc.) and human contributors working in this repository.

> **Short on time?** Read the [Golden Rules](#golden-rules) and [Before You Commit](#before-you-commit) sections. The rest is context.

---

## What this repository is

MinidiSpider is a Rust CLI that:

1. Crawls WikiData (SPARQL + REST) per country — places, people, events
2. Builds a typed hypergraph with sled-backed persistent storage and Tantivy full-text search
3. Exports small, partitionable JSON datasets (each partition < 4 MB) into **separate** `minidi-<cc>-data` repos

Exported data lives **outside this repo** (`../Minidi/Data/` by default). This repo holds the tool, the configs, the queries, and the docs.

## Repository map

| Path | What lives there |
|---|---|
| `src/` | Rust CLI — `main.rs` (commands + `build_registry()`), `config.rs`, `graph/` (hypergraph + sled store), `sources/` (crawler plugins: wikidata, wikipedia, openstreetmap, script, dictionary), `index/` (tantivy search + export), `search/` (BOW embeddings) |
| `configs/` | `countries.json` (country codes/QIDs/languages) + `partitions/*.json` (per-country crawl partitions, incl. `custom_seeds` QID lists) + `queries/*.sparql` (templated SPARQL with `{COUNTRY_QID}` / `{LANG}` placeholders) |
| `scripts/` | Python pipeline helpers — export/compile, repo verification, job sync, git hook installer + validator |
| `docs/` | `ARCHITECTURE.md`, `PLAN.md`, `AI_GUIDELINE.md`, `tooling/` (git best practices, PR template), `research/`, `progress/` |
| `.github/workflows/` | CI/CD — weekly crawl, multi-country crawl, GH Pages deploy, release tagging |
| `data/`, `target/`, `minidi-*-data/`, `jobs/` | Generated at runtime — gitignored, **never commit** |

## Golden rules

1. **Never commit generated output** — `target/`, `data/`, `jobs/`, `minidi-*-data/`, `*.db*`. They are gitignored for a reason.
2. **Keep partition files < 4 MB** — the export pipeline and `verify_repo.py` enforce this.
3. **Work on `develop`** — never push directly to `main`; all `main` changes go through a PR.
4. **Verify QIDs** — `custom_seeds` and partition configs contain explicit WikiData QIDs; a wrong QID silently crawls the wrong entity.
5. **Run validation** — `cargo build`/`cargo test` for Rust changes; `python3 -m py_compile` + a real run for script changes; small `--limit` crawl for SPARQL changes.
6. **Follow the commit convention** — see `docs/tooling/GIT_BEST_PRACTICES.md`; install the hooks so messages are validated automatically.
7. **Write detailed PRs** — use `.github/pull_request_template.md`; say what changed, why, and what you validated.

## Build & validation commands

```bash
cargo build                                        # compile the CLI
cargo test                                         # run Rust tests
cargo run -- stats                                 # graph stats
cargo run -- crawl --country en --limit 100 --progress   # small crawl test
cargo run -- export --country en --sample 50       # sample export
python3 scripts/verify_repo.py --country en        # verify an exported data repo
python3 scripts/install-git-hooks.py               # install commit-msg hook
python3 scripts/check_commit_msg.py <msg-file>     # validate a commit message
```

On Windows, use `py -3` if `python3` is not on PATH.

## Working in this repo

### Rust (`src/`)

- CLI commands and the source registry live in `main.rs` — register new sources in `build_registry()`.
- New data sources implement the `DataSource` trait in `src/sources/` (see `wikidata.rs` as the reference implementation).
- All storage goes through `graph/store.rs` (sled); keep raw sled access out of other modules.

### Configs (`configs/`)

- `countries.json`: `code`, `name`, `qid`, `language`, `repo` (output directory / data-repo name), `native_label`. `output_parent` points **outside** this repo.
- `partitions/*.json`: per-country crawl partitions. `custom_seeds` holds explicit QID lists — cross-check every QID on WikiData before editing.
- `queries/*.sparql`: templated with `{COUNTRY_QID}` / `{LANG}` placeholders. When URLs are assembled from query results, percent-encode them (SPARQL encoding regressions have happened before).

### Scripts (`scripts/`)

- Python 3, `#!/usr/bin/env python3`, argparse-based CLIs.
- `verify_repo.py` — structure, size, and semantic-quality checks on exported data repos.
- `install-git-hooks.py` + `hooks/commit-msg` + `check_commit_msg.py` — commit message validation.

### Workflows (`.github/workflows/`)

- `deploy-pages.yml` runs on push to `main` touching `docs/**` — validates `docs/_data/index.json` and deploys. If the exported data layout changes, update the validation too. The workflow skips deployment when no exported data is present.
- `crawl-weekly.yml` / `crawl-all-countries.yml` — scheduled crawls; require the `GH_PAT` secret (repo + admin:org scopes).

## Before you commit

1. `git status` — confirm only the files you intend to change.
2. Confirm no generated output (`target/`, `data/`, `minidi-*-data/`, …) is staged.
3. Commit message follows the convention: `type(scope): subject` — subject ≤ 50 chars, imperative mood, no trailing period, blank line before body.
4. Run the relevant validation from the section above.

## Anti-patterns to avoid

- ❌ `git add .` from the repo root — it WILL stage generated output. Add specific paths.
- ❌ Pushing straight to `main` (even via force-push).
- ❌ Committing large generated JSON exports to this repo — they belong in the `minidi-<cc>-data` repos.
- ❌ Editing `custom_seeds` QIDs without verifying them on WikiData.
- ❌ Commit messages like `\ Fix stuff \` or `Update: ...` — use the convention in `docs/tooling/GIT_BEST_PRACTICES.md` instead.
- ❌ Fixing data issues by editing `docs/_data/*` in this repo — that data is generated by the export pipeline.
