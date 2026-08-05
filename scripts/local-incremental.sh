#!/bin/sh
# =============================================================================
# Local PoC: Run incremental fill without GitHub Actions
# =============================================================================
# Tests the full pipeline locally: crawl → merge → commit → advance cursor
#
# Usage:
#   chmod +x examples/local-incremental.sh
#   ./examples/local-incremental.sh                    # cursor-based (next)
#   ./examples/local-incremental.sh adm-hanoi          # specific partition
#   ./examples/local-incremental.sh --list             # show partitions
#   ./examples/local-incremental.sh --reset            # reset cursor to start
# =============================================================================

set -e

# ── Config ──────────────────────────────────────────────────────────────────
# Point this to your local data repo (outside the spider project)
DATA_REPO="G:/i2c/PROJECTS/MiniPlatform/MiniDi/Data/minidi-vn-data"
SPIDER_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CURSOR="$DATA_REPO/_data/cursor.json"
SPIDER_DB="/tmp/minidi-spider-poc.db"
EXPORT_DIR="/tmp/minidi-spider-export"

# ── Parse args ──────────────────────────────────────────────────────────────
if [ "$1" = "--list" ]; then
  cd "$SPIDER_DIR"
  cargo run --release -- crawl --list-partitions
  exit 0
fi

if [ "$1" = "--reset" ]; then
  cp "$SPIDER_DIR/examples/cursor.seed.json" "$CURSOR"
  echo "✅ Cursor reset to start (adm-vn-country)"
  exit 0
fi

PARTITION="${1:-}"  # empty = read from cursor

# ── Ensure data repo exists ────────────────────────────────────────────────
mkdir -p "$DATA_REPO/_data" "$DATA_REPO/docs"

if [ ! -f "$CURSOR" ]; then
  cp "$SPIDER_DIR/examples/cursor.seed.json" "$CURSOR"
  echo "📦 Created initial cursor"
fi

# ── Resolve partition ───────────────────────────────────────────────────────
if [ -z "$PARTITION" ]; then
  PARTITION=$(python3 -c "import json;print(json.load(open('$CURSOR')).get('current_partition',''))")
fi
if [ -z "$PARTITION" ]; then
  echo "❌ No partition specified and cursor is empty"
  exit 1
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo "  MiniDi Spider — Local Incremental Fill"
echo "═══════════════════════════════════════════════════"
echo "  Partition: $PARTITION"
echo "  Data repo: $DATA_REPO"
echo "  Spider:    $SPIDER_DIR"
echo "───────────────────────────────────────────────────"

# ── Step 1: Build (if needed) ──────────────────────────────────────────────
if [ ! -f "$SPIDER_DIR/target/release/minidi-spider" ]; then
  echo ""
  echo "🔨 Building minidi-spider..."
  cd "$SPIDER_DIR"
  cargo build --release
fi

# ── Step 2: Crawl ──────────────────────────────────────────────────────────
echo ""
echo "🕷️  Crawling: $PARTITION"
cd "$SPIDER_DIR"
rm -f "$SPIDER_DB"
cargo run --release -- crawl --partition "$PARTITION" --db "$SPIDER_DB" --progress

# ── Step 3: Export ─────────────────────────────────────────────────────────
echo ""
echo "📦 Exporting..."
rm -rf "$EXPORT_DIR"
cargo run --release -- export --db "$SPIDER_DB" --output "$EXPORT_DIR"

# ── Step 4: Merge into data repo ───────────────────────────────────────────
echo ""
echo "🔗 Merging into data repo..."
cd "$DATA_REPO"
python3 << PYEOF
import json, os, shutil, glob

PARTITION = "$PARTITION"
EXPORT = "$EXPORT_DIR"
DATA = "."

# Load fresh export
fresh_file = os.path.join(EXPORT, "index.json")
with open(fresh_file) as f:
    fresh = json.load(f)

# Load existing index
data_file = os.path.join(DATA, "docs", "index.json")
existing = {"meta": {"entity_count": 0, "edge_count": 0}, "nodes": [], "edges": []}
if os.path.exists(data_file):
    with open(data_file) as f:
        existing = json.load(f)

# Deduplicate nodes
seen = {n["id"] for n in existing["nodes"]}
new_nodes = 0
for n in fresh["nodes"]:
    if n["id"] not in seen:
        existing["nodes"].append(n)
        seen.add(n["id"])
        new_nodes += 1

# Deduplicate edges
seen_e = {f'{e["s"]}|{e["r"]}|{e["t"]}' for e in existing["edges"]}
new_edges = 0
for e in fresh["edges"]:
    k = f'{e["s"]}|{e["r"]}|{e["t"]}'
    if k not in seen_e:
        existing["edges"].append(e)
        seen_e.add(k)
        new_edges += 1

# Update metadata
existing["meta"]["entity_count"] = len(existing["nodes"])
existing["meta"]["edge_count"] = len(existing["edges"])
existing["meta"]["last_partition"] = PARTITION

# Write merged index
with open(data_file, 'w') as f:
    json.dump(existing, f)
print(f"  Nodes: +{new_nodes} = {existing['meta']['entity_count']}")
print(f"  Edges: +{new_edges} = {existing['meta']['edge_count']}")

# Write lite index
lite = [{"id": n["id"], "l": n["l"], "lv": n.get("lv"), "t": n["t"], "u": n["u"]}
        for n in existing["nodes"]]
with open(os.path.join(DATA, "docs", "index.lite.json"), 'w') as f:
    json.dump(lite, f)

# Copy v1/ partitions
v1_src = os.path.join(EXPORT, "v1")
v1_dst = os.path.join(DATA, "docs", "v1")
if os.path.exists(v1_src):
    if os.path.exists(v1_dst):
        # Merge subdirectories
        for sub in ["entities", "timeline", "relations"]:
            sub_src = os.path.join(v1_src, sub)
            sub_dst = os.path.join(v1_dst, sub)
            if os.path.exists(sub_src):
                os.makedirs(sub_dst, exist_ok=True)
                for f in os.listdir(sub_src):
                    shutil.copy2(os.path.join(sub_src, f), os.path.join(sub_dst, f))
    else:
        shutil.copytree(v1_src, v1_dst)

# Copy other files
for f in ["embeddings.bin", "embeddings.json", "sources.json"]:
    src = os.path.join(EXPORT, f)
    if os.path.exists(src):
        shutil.copy2(src, os.path.join(DATA, "docs", f))

print(f"  Docs updated")
PYEOF

# ── Step 5: Advance cursor ─────────────────────────────────────────────────
echo ""
echo "🎯 Advancing cursor..."
cd "$SPIDER_DIR"
NEXT=$(cargo run --release -- crawl --list-partitions 2>/dev/null | \
  awk -v cur="$PARTITION" '{if(f){print $1;exit}}f{if($1==cur)f=1}' || echo "")

cd "$DATA_REPO"
python3 << PYEOF
import json
c = json.load(open("$CURSOR"))
if "$PARTITION" not in c["completed"]:
    c["completed"].append("$PARTITION")
c["current_partition"] = "$NEXT"
c["progress"] = f'{len(c["completed"])}/50'
import subprocess
c["updated_at"] = subprocess.getoutput("date -u +%Y-%m-%dT%H:%M:%SZ")
json.dump(c, open("$CURSOR", 'w'), indent=2)
print(f'  Progress: {c["progress"]}')
print(f'  Next:     {"$NEXT" or "COMPLETE 🎉"}')
PYEOF

# ── Step 6: Commit ─────────────────────────────────────────────────────────
echo ""
echo "📝 Committing..."
cd "$DATA_REPO"
git add docs/ _data/ 2>/dev/null || true
if git diff --cached --quiet 2>/dev/null; then
  echo "  No changes to commit"
else
  git commit -m "feat(data): add $PARTITION" --allow-empty
  echo "  ✅ Committed: feat(data): add $PARTITION"
fi

# ── Done ────────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════"
echo "  ✅ Done — $PARTITION processed"
echo "  Next: $NEXT"
echo "  Repo: $DATA_REPO"
echo "═══════════════════════════════════════════════════"
echo ""
echo "  To run the next partition:"
echo "    ./examples/local-incremental.sh"
echo ""
echo "  To run a specific partition:"
echo "    ./examples/local-incremental.sh hist-tran"
echo ""
echo "  To see all partitions:"
echo "    ./examples/local-incremental.sh --list"
echo ""
