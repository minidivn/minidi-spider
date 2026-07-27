#!/usr/bin/env python3
import os
import sys
import json
import argparse
import subprocess

def parse_args():
    parser = argparse.ArgumentParser(description="Create and sync Wikidata partition crawls or dictionary tasks as GitHub Issues.")
    parser.add_argument("--country", "-c", help="Country code (e.g. fr, vn)")
    parser.add_argument("--repo-path", "-r", help="Direct path to repository")
    parser.add_argument("--config", default="configs/countries.json", help="Path to countries config")
    parser.add_argument("--limit", type=int, default=5, help="Max number of issues to create in a single run")
    return parser.parse_args()

def resolve_repo_path(args):
    if args.repo_path:
        return os.path.normpath(args.repo_path)
        
    if not args.country:
        print("[ERROR] Must specify either --country (-c) or --repo-path (-r)", file=sys.stderr)
        sys.exit(1)
        
    country_code = args.country.lower()
    
    if not os.path.exists(args.config):
        fallback = os.path.join(os.path.dirname(os.path.dirname(__file__)), args.config)
        if os.path.exists(fallback):
            args.config = fallback
            
    try:
        with open(args.config, "r", encoding="utf-8") as f:
            cfg = json.load(f)
            
        output_parent = cfg.get("output_parent", "../../Minidi/Data")
        base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        output_parent_abs = os.path.normpath(os.path.join(base_dir, output_parent))
        
        for c in cfg.get("countries", []):
            if c["code"].lower() == country_code:
                return os.path.join(output_parent_abs, c["repo"])
                
        print(f"[ERROR] Country code '{country_code}' not found in {args.config}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"[ERROR] Failed to read config: {e}", file=sys.stderr)
        sys.exit(1)

def run_command(args, cwd=None):
    try:
        res = subprocess.run(args, cwd=cwd, shell=True, capture_output=True, text=True)
        return res.stdout.strip(), res.returncode
    except Exception as e:
        return "", -1

def get_existing_issues(repo_path):
    # Retrieve all open issue titles with 'type:gap' or 'type:dictionary' label
    cmd = ["gh", "issue", "list", "--state", "open", "--json", "title,number"]
    stdout, code = run_command(cmd, cwd=repo_path)
    if code != 0 or not stdout:
        return []
    try:
        return json.loads(stdout)
    except Exception:
        return []

def main():
    args = parse_args()
    repo_path = resolve_repo_path(args)
    country = args.country.lower() if args.country else "fr"
    
    print(f"=== Syncing Jobs as GitHub Issues for Repo: {repo_path} ===")
    
    # 1. Fetch current open issues on GitHub
    existing_issues = get_existing_issues(repo_path)
    existing_titles = {issue["title"] for issue in existing_issues}
    print(f"[INFO] Found {len(existing_titles)} open issues on GitHub repository.")
    
    # 2. Identify missing / incomplete partitions
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    partitions_file = os.path.join(base_dir, "configs", "partitions", f"{country}.json")
    if not os.path.exists(partitions_file):
        # fallback to default.json
        partitions_file = os.path.join(base_dir, "configs", "partitions", "default.json")
        
    if not os.path.exists(partitions_file):
        print(f"[ERROR] Partition configuration not found for country '{country}'.")
        sys.exit(1)
        
    with open(partitions_file, "r", encoding="utf-8") as f:
        partitions_data = json.load(f)
        
    partitions = partitions_data if isinstance(partitions_data, list) else partitions_data.get("partitions", [])
    print(f"[INFO] Scanning {len(partitions)} configured partitions inside: {partitions_file}")
    
    issues_created = 0
    
    # Check partition crawldb completeness
    for idx, p in enumerate(partitions):
        pname = p.get("name", f"partition_{idx}")
        issue_title = f"[Knowledge Gap] Ingest partition: {pname} ({country.upper()})"
        
        # Check if already opened
        if issue_title in existing_titles:
            print(f"  - Ingest partition '{pname}': Already exists (Open)")
            continue
            
        # Limit issues created in one cycle to prevent API rate limits
        if issues_created >= args.limit:
            print(f"[INFO] Reached maximum issues generation limit ({args.limit}). Postponing remaining tasks.")
            break
            
        # Compose task payload body
        payload = {
            "type": "crawl_partition",
            "country": country,
            "partition_name": pname,
            "settings": p
        }
        
        body_content = f"""This issue was automatically created to queue a missing dataset partition crawl.

### Ingestion Task Payload
```json
{json.dumps(payload, indent=2, ensure_ascii=False)}
```

An autonomous agent can run this ingestion crawl locally or via GitHub Actions, push the output data files, and close this issue.
"""
        
        print(f"[INFO] Creating GitHub issue: '{issue_title}'")
        cmd = ["gh", "issue", "create", "--title", issue_title, "--body", body_content, "--label", "type:gap"]
        stdout, code = run_command(cmd, cwd=repo_path)
        
        if code == 0:
            issues_created += 1
            print(f"  [PASS] Created Issue successfully: {stdout}")
        else:
            print(f"  [FAIL] Failed to create issue for partition {pname}")
            
    print(f"\n[SUCCESS] Sync completed. Created {issues_created} new job issues.")

if __name__ == "__main__":
    main()
