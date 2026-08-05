#!/usr/bin/env python3
import os
import sys
import json
import argparse

def parse_args():
    parser = argparse.ArgumentParser(description="Verify local data repository structure, size constraints, and quality metrics.")
    parser.add_argument("--country", "-c", help="Country code (e.g. fr, vn)")
    parser.add_argument("--repo-path", "-r", help="Direct path to repository")
    parser.add_argument("--config", default="configs/countries.json", help="Path to countries config")
    return parser.parse_args()

def resolve_repo_path(args):
    if args.repo_path:
        return os.path.normpath(args.repo_path)
        
    if not args.country:
        print("[ERROR] Must specify either --country (-c) or --repo-path (-r)", file=sys.stderr)
        sys.exit(1)
        
    country_code = args.country.lower()
    
    if not os.path.exists(args.config):
        # Check parent folder
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

def check_file(path, label):
    exists = os.path.exists(path)
    size_str = "N/A"
    if exists:
        size_bytes = os.path.getsize(path)
        size_str = f"{size_bytes / 1024:.2f} KB"
    print(f"  [{'PASS' if exists else 'FAIL'}] {label} ({size_str})")
    return exists

def main():
    args = parse_args()
    repo_path = resolve_repo_path(args)
    
    print(f"=== Starting Quality Checks for Repository: {repo_path} ===")
    
    # 1. Structure Verification
    print("\n[1] Verifying Directory Layout & Configuration Invariants:")
    docs_dir = os.path.join(repo_path, "docs")
    data_dir = os.path.join(docs_dir, "_data")
    v1_dir = os.path.join(data_dir, "v1")
    minidi_dir = os.path.join(repo_path, ".minidi")
    
    structure_ok = True
    structure_ok &= check_file(docs_dir, "docs/ publishing directory")
    structure_ok &= check_file(os.path.join(docs_dir, ".nojekyll"), "docs/.nojekyll Pages bypass file")
    structure_ok &= check_file(os.path.join(docs_dir, "index.html"), "docs/index.html web app entry")
    structure_ok &= check_file(os.path.join(minidi_dir, "config.json"), ".minidi/config.json metadata settings")
    structure_ok &= check_file(os.path.join(data_dir, "index.json"), "docs/_data/index.json global graph db")
    structure_ok &= check_file(os.path.join(v1_dir, "schema.json"), "docs/_data/v1/schema.json descriptor")
    structure_ok &= check_file(os.path.join(v1_dir, "entities", "_index.json"), "docs/_data/v1/entities/_index.json spine index")

    if not structure_ok:
        print("\n[WARN] Structure checks failed. Data might be corrupted or not yet compiled.", file=sys.stderr)
        
    # 2. Size Constraint Verification
    print("\n[2] Verifying Size & Memory Constraints:")
    skeleton_path = os.path.join(data_dir, "index.json")
    skeleton_ok = True
    if os.path.exists(skeleton_path):
        size_bytes = os.path.getsize(skeleton_path)
        size_mb = size_bytes / (1024 * 1024)
        limit_mb = 10.0
        status = "PASS" if size_mb <= limit_mb else "FAIL (Exceeds 10MB memory-mappable limit)"
        print(f"  - L2 index size: {size_mb:.2f} MB / {limit_mb:.2f} MB limit ({status})")
        if size_mb > limit_mb:
            skeleton_ok = False
    else:
        print("  - L2 index: Missing (cannot check size)")
        skeleton_ok = False
        
    # 3. Semantic Graph Quality Verification
    print("\n[3] Calculating Semantic Invariants & Node Density:")
    total_nodes = 0
    total_edges = 0
    avg_arity = 0.0
    low_opacity_ratio = 1.0
    
    if os.path.exists(skeleton_path):
        try:
            with open(skeleton_path, "r", encoding="utf-8") as f:
                graph = json.load(f)
                
            nodes = graph.get("nodes", [])
            edges = graph.get("edges", [])
            total_nodes = len(nodes)
            total_edges = len(edges)
            
            # Arity: count of metadata keys in node properties
            arity_sum = 0
            semantic_count = 0
            lexical_count = 0
            
            for node_data in nodes:
                metadata = node_data.get("m", {})
                arity_sum += len(metadata)
                
                ntype = node_data.get("t", "other")
                if ntype in ["place", "person", "event", "organization"]:
                    semantic_count += 1
                else:
                    lexical_count += 1
                    
            if total_nodes > 0:
                avg_arity = arity_sum / total_nodes
            if (semantic_count + lexical_count) > 0:
                low_opacity_ratio = semantic_count / (semantic_count + lexical_count)
                
            print(f"  - Total semantic nodes: {total_nodes}")
            print(f"  - Total edges: {total_edges}")
            
            arity_status = "PASS" if avg_arity >= 1.5 else "WARN (Low arity, metadata density should be improved)"
            print(f"  - Average Entity Metadata Arity: {avg_arity:.2f} (Target: >= 1.5) ({arity_status})")
            
            opacity_status = f"{low_opacity_ratio * 100:.1f}% semantic"
            print(f"  - Low-Opacity (Semantic Spine) Ratio: {opacity_status}")
            
        except Exception as e:
            print(f"  - [ERROR] Failed to parse graph file: {e}")
            
    # 4. Save and Report Metrics
    os.makedirs(minidi_dir, exist_ok=True)
    metrics_path = os.path.join(minidi_dir, "metrics.json")
    metrics_data = {
        "verified_at_utc": "", # Can be populated on push
        "structure_valid": structure_ok,
        "l2_size_ok": skeleton_ok,
        "total_nodes": total_nodes,
        "total_edges": total_edges,
        "average_metadata_arity": round(avg_arity, 3),
        "low_opacity_ratio": round(low_opacity_ratio, 3)
    }
    
    with open(metrics_path, "w", encoding="utf-8") as f:
        json.dump(metrics_data, f, indent=2)
    print(f"\n[INFO] Saved metrics report to: .minidi/metrics.json")
    print("\n=== Verification Scorecard Complete ===")

if __name__ == "__main__":
    main()
