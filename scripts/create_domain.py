#!/usr/bin/env python3
import os
import sys
import json
import shutil
import argparse
import subprocess
import re

def parse_args():
    parser = argparse.ArgumentParser(description="Create new localized domain/country knowledge repo and setup GitHub Pages.")
    parser.add_argument("--country", "-c", help="Country code (e.g. fr, vn, de)")
    parser.add_argument("--domain", "-d", help="Domain space name (e.g. science.math, science.physics)")
    parser.add_argument("--name", "-n", required=True, help="Friendly name (e.g. France, Mathematics)")
    parser.add_argument("--qid", "-q", help="WikiData QID (e.g. Q142 for France)")
    parser.add_argument("--lang", "-l", help="Native language code (e.g. fr)")
    parser.add_argument("--output-parent", default="../../Minidi/Data", help="Parent directory for data repositories")
    parser.add_argument("--dry-run", action="store_true", help="Prepare folder structure locally, do not run Git/GH actions")
    return parser.parse_args()

def run_command(args, cwd=None, input_data=None):
    cmd_str = " ".join(args)
    print(f"[CMD] {cmd_str}")
    res = subprocess.run(args, cwd=cwd, shell=True, capture_output=True, text=True, input=input_data)
    if res.returncode != 0:
        print(f"[ERROR] Command failed with code {res.returncode}:\n{res.stderr}")
        sys.exit(res.returncode)
    return res.stdout.strip()

def main():
    args = parse_args()
    
    if not args.country and not args.domain:
        print("[ERROR] Either --country (-c) or --domain (-d) must be specified.", file=sys.stderr)
        sys.exit(1)
        
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    output_parent_abs = os.path.normpath(os.path.join(base_dir, args.output_parent))
    
    # Determine repo name and namespace
    if args.country:
        country_code = args.country.lower()
        repo_name = f"minidi-data-country.{country_code}"
        qid = args.qid or ""
        lang = args.lang or country_code
    else:
        repo_name = args.domain.lower()
        qid = args.qid or ""
        lang = args.lang or "en"
        
    dst_repo = os.path.join(output_parent_abs, repo_name)
    print(f"[INFO] Initializing new repository: {dst_repo}")
    
    # 1. Create directory structure
    os.makedirs(dst_repo, exist_ok=True)
    minidi_dir = os.path.join(dst_repo, ".minidi")
    os.makedirs(minidi_dir, exist_ok=True)
    os.makedirs(os.path.join(dst_repo, "docs"), exist_ok=True)
    os.makedirs(os.path.join(dst_repo, ".github", "workflows"), exist_ok=True)
    
    # 2. Write .minidi/config.json
    config_data = {
        "repo": repo_name,
        "name": args.name,
        "qid": qid,
        "language": lang,
        "crawl_settings": {
            "limit": 100,
            "rate_limit_ms": 200,
            "max_edges_per_node": 50,
            "link_traversal_depth": 2
        }
    }
    with open(os.path.join(minidi_dir, "config.json"), "w", encoding="utf-8") as f:
        json.dump(config_data, f, indent=2, ensure_ascii=False)
    print(f"[INFO] Created metadata config at: .minidi/config.json")
    
    # 3. Write .minidi/state.json initial state
    state_data = {
        "last_crawl_utc": "",
        "latest_manifest_hash": "",
        "crawled_partitions": []
    }
    with open(os.path.join(minidi_dir, "state.json"), "w", encoding="utf-8") as f:
        json.dump(state_data, f, indent=2, ensure_ascii=False)
        
    # 4. Copy Vite/pnpm frontend skeleton from Vietnam template
    src_skeleton = os.path.join(output_parent_abs, "minidi-data-ghpage")
    dst_skeleton = os.path.join(dst_repo, "src", "ghpage")
    if os.path.exists(src_skeleton):
        print(f"[INFO] Copying frontend skeleton template from: {src_skeleton}")
        shutil.copytree(
            src_skeleton, 
            dst_skeleton, 
            dirs_exist_ok=True,
            ignore=shutil.ignore_patterns("node_modules", ".git", "dist", "config.js")
        )
    else:
        print(f"[WARN] Ingestion source skeleton template not found at {src_skeleton}. Dynamic copies will be deferred.")

    # 5. Write standard .nojekyll and basic .gitignore
    with open(os.path.join(dst_repo, "docs", ".nojekyll"), "w") as f:
        pass
        
    gitignore_content = """node_modules/
dist/
target/
.env
*.db
*.log
jobs/
"""
    with open(os.path.join(dst_repo, ".gitignore"), "w") as f:
        f.write(gitignore_content)
        
    # 6. Write GHA pipeline.yml workflow
    workflow_content = f"""name: Minidi Ingestion Pipeline

on:
  schedule:
    - cron: '0 2 * * *'  # Runs at 2:00 AM UTC daily
  workflow_dispatch:
    inputs:
      limit:
        description: 'Max crawl nodes limit'
        required: false
        default: '15'

permissions:
  contents: write
  issues: write

jobs:
  run-pipeline:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Data Repo
        uses: actions/checkout@v4
        with:
          path: data-repo

      - name: Checkout Spider Repo
        uses: actions/checkout@v4
        with:
          repository: minidivn/minidi-spider
          path: spider-repo

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Set up Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install pnpm
        uses: pnpm/action-setup@v3
        with:
          version: 10

      - name: Install Frontend Dependencies
        run: |
          pnpm install
        working-directory: data-repo/src/ghpage

      - name: Run Pipeline Ingestion
        run: |
          # Copy run configs and run execution pipeline
          cd spider-repo
          cp -r ../data-repo/.minidi/* configs/ 2>/dev/null || true
          python scripts/run_pipeline.py --country {args.country or 'fr'} --limit ${{{{ github.event.inputs.limit || '15' }}}}
          
          # Copy outputs back to data-repo
          cp -r ../Minidi/Data/{repo_name}/* ../data-repo/
        env:
          CI: "true"

      - name: Commit and Push Updates
        run: |
          cd data-repo
          git config --global user.name "github-actions[bot]"
          git config --global user.email "github-actions[bot]@users.noreply.github.com"
          git add .
          if ! git diff --cached --quiet; then
            git commit -m "Automated build: Update dataset and recompile web assets"
            git push origin main
          else
            echo "No data updates detected. Skipping push."
          fi
"""
    workflow_path = os.path.join(dst_repo, ".github", "workflows", "pipeline.yml")
    with open(workflow_path, "w", encoding="utf-8") as f:
        f.write(workflow_content)
    print(f"[INFO] Created GitHub Actions workflow at: {workflow_path}")
    
    if args.dry_run:
        print("[SUCCESS] Dry-run completed. Folders prepared locally. Skipping Git/GitHub creation.")
        return

    # 7. Git Init and Initial Push
    print("[INFO] Initializing Git repository locally...")
    run_command(["git", "init", "-b", "main"], cwd=dst_repo)
    run_command(["git", "add", "."], cwd=dst_repo)
    run_command(["git", "commit", "-m", "Initial commit: Set up repository structure and GHA pipeline"], cwd=dst_repo)

    # 8. GitHub repo create via gh CLI
    print("[INFO] Creating GitHub repository via gh CLI...")
    run_command(["gh", "repo", "create", f"minidivn/{repo_name}", "--public", f"--source={dst_repo}", "--push", "-y"])

    # 9. Enable GitHub Pages via gh API
    print("[INFO] Activating GitHub Pages on /docs directory...")
    # Add a small delay to ensure repo creation propagates
    import time
    time.sleep(3)
    try:
        run_command([
            "gh", "api", "-X", "POST",
            f"repos/minidivn/{repo_name}/pages",
            "--input", "-"
        ], input_data='{"source":{"branch":"main","path":"/docs"}}')
        print(f"[SUCCESS] Activated GitHub Pages successfully for {repo_name}!")
    except Exception as e:
        print(f"[WARN] Failed to configure Pages automatically. Please verify repository settings. Error: {e}")

    print(f"\n==================================================")
    print(f"[SUCCESS] Repository {repo_name} created and initialized!")
    print(f"  - Local Path: {dst_repo}")
    # Fetch URL
    repo_url = f"https://github.com/minidivn/{repo_name}"
    pages_url = f"https://minidivn.github.io/{repo_name}/"
    print(f"  - GitHub Repository: {repo_url}")
    print(f"  - GitHub Pages App: {pages_url}")
    print(f"==================================================")

if __name__ == "__main__":
    main()
