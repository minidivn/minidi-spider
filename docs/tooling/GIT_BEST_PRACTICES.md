# Git Best Practices

Conventions for branches, commits, pull requests, tags, and hooks in MinidiSpider.

---

## Branching model

```
main  ←  develop  ←  backfill/* | feat/* | fix/* | docs/* | chore/*
```

- **`main`** — stable and deployable. Only receives **merges via PR** from `develop`. Pushing directly to `main` is forbidden.
- **`develop`** — the default working branch. All feature/fix/docs work lands here.
- **Short-lived branches** (`feat/`, `fix/`, `docs/`, `chore/`, `backfill/`) — for larger pieces of work; merged into `develop`, then deleted.

## Commit messages

Format:

```
<type>(<scope>): <subject>

<body — what & why, wrapped at 72 chars>

<footer — BREAKING CHANGE / issue refs (optional)>
```

Allowed types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `perf`, `build`, `ci`, `style`, `revert`.

### Rules

| Rule | Example |
|---|---|
| Imperative mood subject | `Fix QID typo` ✓ / `Fixed QID typo` ✗ |
| Subject ≤ 50 chars | `feat(partitions): add England landmark seeds` |
| No trailing period on subject | `add England landmark seeds` ✓ / `...seeds.` ✗ |
| Scope optional, lowercase, kebab-case | `fix(sparql):`, `docs(tooling):` |
| Blank line before body (required if body exists) | — |
| Body wraps at 72 chars | — |
| Breaking change: `!` after scope/type | `feat(export)!: change partition layout` |

### Examples

```
feat(partitions): add England landmark seeds to default config
fix(sparql): percent-encode URLs built from query results
docs(tooling): document git hooks and commit convention
refactor(export): move compiled data into docs/_data
```

### History notes

Older commits in this repo use backslash artifacts (`\ Fix QID typos ... \`) and `Update: ...` prefixes. These are **deprecated** — all new commits follow the convention above. The `commit-msg` hook enforces it.

## Branch naming

- `feat/<topic>` — new feature
- `fix/<topic>` — bug fix
- `docs/<topic>` — documentation
- `chore/<topic>` — maintenance, tooling, configs
- `backfill/<topic>` — one-off data fills

Keep branch names short, lowercase, and dash-separated. Delete branches after merging.

## Pull request workflow (`develop` → `main`)

1. Commit on `develop` (hooks validate each message).
2. Push: `git push origin develop`
3. Open the PR:

   ```bash
   gh pr create --base main --head develop \
     --title "<summary>" \
     --body-file pr-body.md
   ```

   Use `.github/pull_request_template.md` for the body structure.

4. PR body must contain: **summary**, **changes**, **validation run**, **impact** (data repos, GH Pages, breaking changes).
5. Check the CI result (`deploy-pages.yml` runs after merge; `docs/**` changes are skipped until exported data exists).
6. Merge with a **merge commit** to preserve history:

   ```bash
   gh pr merge --merge
   ```

   Do **not** delete the `develop` branch — it is the working branch.

## Tagging releases

Calendar versioning per data snapshot: `vYYYY.MM.REVISION` (e.g. `v2026.08.1`).

```bash
git tag -a v2026.08.1 -m "Data snapshot v2026.08.1"
git push origin v2026.08.1
```

(`.github/workflows/release-tag.yml` automates this.)

## Hooks

Install the hooks once per clone:

```bash
python3 scripts/install-git-hooks.py     # Windows: py -3 scripts/install-git-hooks.py
```

What gets installed:

- `commit-msg` → runs `scripts/check_commit_msg.py` on every commit; rejects messages that violate this document.

To remove: `rm .git/hooks/commit-msg`

## Pitfalls

- `git add .` stages generated output (`target/`, `data/`, `minidi-*-data/`) — add specific paths instead. If it happens: `git rm -r --cached <path>` and re-add `.gitignore` entries.
- Data exports (JSON > ~4 MB per partition) belong in the `minidi-<cc>-data` repos, never here.
- Keep the working tree clean before switching branches; use `git stash` for WIP.
- Never rewrite published history on shared branches (`develop`, `main`).
