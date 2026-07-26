#!/usr/bin/env python3
"""
Pipeline Runner: Single Command Ingestion & Exporting
Automates:
  1. Optional wordlist consolidation & stats generation (if config exists)
  2. Cargo crawl for the target country code
  3. Cargo export to the target docs repository
  4. Dynamically generates config.js and compiles Vite frontend into docs/
  5. Summarizes execution results

Usage:
  python scripts/run_pipeline.py --country vn --limit 50 --release
"""

import os
import sys
import json
import argparse
import subprocess
import shutil
import re

COUNTRY_METADATA = {
    "vn": {
        "countryCode": "vn",
        "countryName": "Vietnam",
        "countryEmoji": "🇻🇳",
        "languages": ["en", "vi"],
        "defaultLanguage": "en",
        "title": "Vietnam Knowledge Graph",
        "subtitle": "Exploring {n} WikiData entities: history, geography, people & culture",
        "splashTitle": "🇻🇳 MiniDi",
        "splashMessage": "Loading Vietnam knowledge graph...",
        "mapCenter": [16.0, 107.5],
        "mapZoom": 6,
        "githubRepo": "minidivn/minidi-vn-data",
        "dataSource": "WikiData",
        "dataPath": "_data/index.json",
        "chatCountry": "Vietnam",
        "chatGreeting": "Ask me about Vietnam — history, people, places, events.",
        "slides": [
            {
                "label": "Hạ Long Bay", "lang": "en",
                "labelAlt": "Vịnh Hạ Long", "langAlt": "vi",
                "img": "https://upload.wikimedia.org/wikipedia/commons/thumb/3/38/Halong_Bay_in_2019.jpg/1280px-Halong_Bay_in_2019.jpg",
                "grad": "linear-gradient(135deg, #0f766e, #0d9488, #2dd4bf)"
            },
            {
                "label": "Huế Imperial City", "lang": "en",
                "labelAlt": "Cố đô Huế", "langAlt": "vi",
                "img": "https://upload.wikimedia.org/wikipedia/commons/thumb/1/17/Hue_Citadel.jpg/1280px-Hue_Citadel.jpg",
                "grad": "linear-gradient(135deg, #4a1d96, #8b5cf6, #c084fc)"
            }
        ],
        "eras": [
            {"id": "hong_bang", "emoji": "🏛️", "label": {"en": "Hồng Bàng Dynasty", "vi": "Thời kỳ Hồng Bàng"}, "years": [-2879, -258], "color": "#8b5cf6"},
            {"id": "chinese_domination", "emoji": "🌍", "label": {"en": "Chinese Domination", "vi": "Bắc thuộc"}, "years": [-111, 939], "color": "#ef4444"},
            {"id": "dynastic_vn", "emoji": "👑", "label": {"en": "Dynastic Vietnam", "vi": "Việt Nam thời quân chủ"}, "years": [939, 1858], "color": "#f59e0b"},
            {"id": "colonial", "emoji": "⚖️", "label": {"en": "French Colonial Period", "vi": "Pháp thuộc"}, "years": [1858, 1954], "color": "#3b82f6"},
            {"id": "vietnam_war", "emoji": "💥", "label": {"en": "Vietnam War", "vi": "Chiến tranh Việt Nam"}, "years": [1955, 1975], "color": "#dc2626"},
            {"id": "modern", "emoji": "📷", "label": {"en": "Modern Vietnam", "vi": "Việt Nam hiện đại"}, "years": [1975, 9999], "color": "#22c55e"}
        ],
        "translations": {
            "en": {
                "entities": "Entities", "events": "Events", "people": "People", "places": "Places", "all": "All",
                "search": "Search...", "noResults": "No matching entities", "loading": "Loading...", "results": "{n} results",
                "page": "Page {p} of {t}", "prev": "Prev", "next": "Next", "details": "Details", "relations": "Relations",
                "wiki": "Open in WikiData", "timeline": "Timeline", "tree": "Dynasty Tree", "builtWith": "Built with",
                "source": "source", "chatTitle": "MiniDi Assistant", "chatPlaceholder": "Ask about Vietnam...",
                "entitiesTotal": "{n} entities", "mapPlaces": "Places", "mapPeople": "People", "mapEvents": "Events"
            },
            "vi": {
                "entities": "Thực thể", "events": "Sự kiện", "people": "Nhân vật", "places": "Địa danh", "all": "Tất cả",
                "search": "Tìm kiếm...", "noResults": "Không tìm thấy", "loading": "Đang tải...", "results": "{n} kết quả",
                "page": "Trang {p} / {t}", "prev": "Trước", "next": "Sau", "details": "Chi tiết", "relations": "Quan hệ",
                "wiki": "Mở trong WikiData", "timeline": "Thời gian", "tree": "Đồng hồ", "builtWith": "Xây dựng với",
                "source": "Mã nguồn", "chatTitle": "Trợ lý MiniDi", "chatPlaceholder": "Hỏi về Việt Nam...",
                "entitiesTotal": "{n} thực thể", "mapPlaces": "Địa danh", "mapPeople": "Nhân vật", "mapEvents": "Sự kiện"
            }
        }
    },
    "fr": {
        "countryCode": "fr",
        "countryName": "France",
        "countryEmoji": "🇫🇷",
        "languages": ["en", "fr"],
        "defaultLanguage": "fr",
        "title": "France Knowledge Graph",
        "subtitle": "Exploring {n} WikiData entities: history, geography, people & culture",
        "splashTitle": "🇫🇷 MiniDi",
        "splashMessage": "Loading France knowledge graph...",
        "mapCenter": [46.2276, 2.2137],
        "mapZoom": 6,
        "githubRepo": "minidivn/minidi-fr-data",
        "dataSource": "WikiData",
        "dataPath": "_data/index.json",
        "chatCountry": "France",
        "chatGreeting": "Posez-moi des questions sur la France — histoire, personnages, lieux, événements.",
        "slides": [
            {
                "label": "Eiffel Tower", "lang": "en",
                "labelAlt": "Tour Eiffel", "langAlt": "fr",
                "img": "https://images.unsplash.com/photo-1502602898657-3e91760cbb34?w=1200&q=80",
                "grad": "linear-gradient(135deg, #0f766e, #0d9488, #2dd4bf)"
            },
            {
                "label": "Mont Saint-Michel", "lang": "en",
                "labelAlt": "Mont Saint-Michel", "langAlt": "fr",
                "img": "https://images.unsplash.com/photo-1563784462386-044fd95e9852?w=1200&q=80",
                "grad": "linear-gradient(135deg, #1e3a5f, #2563eb, #60a5fa)"
            }
        ],
        "eras": [
            {"id": "roman_gaul", "emoji": "🏛️", "label": {"en": "Roman Gaul", "fr": "Gaule romaine"}, "years": [-52, 486], "color": "#8b5cf6"},
            {"id": "middle_ages", "emoji": "👑", "label": {"en": "Middle Ages", "fr": "Moyen Âge"}, "years": [486, 1492], "color": "#f59e0b"},
            {"id": "ancien_regime", "emoji": "🏰", "label": {"en": "Ancien Régime", "fr": "Ancien Régime"}, "years": [1492, 1789], "color": "#3b82f6"},
            {"id": "revolution", "emoji": "⚖️", "label": {"en": "Revolution & Empire", "fr": "Révolution & Empire"}, "years": [1789, 1815], "color": "#ef4444"},
            {"id": "nineteenth_century", "emoji": "🏭", "label": {"en": "19th Century", "fr": "XIXe siècle"}, "years": [1815, 1914], "color": "#c084fc"},
            {"id": "modern", "emoji": "📷", "label": {"en": "Modern France", "fr": "France moderne"}, "years": [1914, 9999], "color": "#22c55e"}
        ],
        "translations": {
            "en": {
                "entities": "Entities", "events": "Events", "people": "People", "places": "Places", "all": "All",
                "search": "Search...", "noResults": "No matching entities", "loading": "Loading...", "results": "{n} results",
                "page": "Page {p} of {t}", "prev": "Prev", "next": "Next", "details": "Details", "relations": "Relations",
                "wiki": "Open in WikiData", "timeline": "Timeline", "tree": "Dynasty Tree", "builtWith": "Built with",
                "source": "source", "chatTitle": "MiniDi Assistant", "chatPlaceholder": "Ask about France...",
                "entitiesTotal": "{n} entities", "mapPlaces": "Places", "mapPeople": "People", "mapEvents": "Events"
            },
            "fr": {
                "entities": "Entités", "events": "Événements", "people": "Personnalités", "places": "Lieux", "all": "Tout",
                "search": "Rechercher...", "noResults": "Aucun résultat trouvé", "loading": "Chargement...", "results": "{n} résultats",
                "page": "Page {p} sur {t}", "prev": "Précédent", "next": "Suivant", "details": "Détails", "relations": "Relations",
                "wiki": "Ouvrir dans WikiData", "timeline": "Chronologie", "tree": "Généalogie", "builtWith": "Construit avec",
                "source": "Code source", "chatTitle": "Assistant MiniDi", "chatPlaceholder": "Poser une question sur la France...",
                "entitiesTotal": "{n} entités", "mapPlaces": "Lieux", "mapPeople": "Personnalités", "mapEvents": "Événements"
            }
        }
    }
}

def get_country_metadata(country_code, country_name, repo_name):
    country_code = country_code.lower()
    if country_code in COUNTRY_METADATA:
        meta = COUNTRY_METADATA[country_code].copy()
        meta["githubRepo"] = f"minidivn/{repo_name}"
        return meta
    
    # Generic fallback
    country_emoji_map = {
        "en": "🇺🇸", "zh": "🇨🇳", "es": "🇪🇸", "de": "🇩🇪", "ru": "🇷🇺",
        "hi": "🇮🇳", "pt": "🇧🇷", "ar": "🇪🇬", "bn": "🇧🇩"
    }
    emoji = country_emoji_map.get(country_code, "🌍")
    
    return {
        "countryCode": country_code,
        "countryName": country_name,
        "countryEmoji": emoji,
        "languages": ["en", country_code],
        "defaultLanguage": "en",
        "title": f"{country_name} Knowledge Graph",
        "subtitle": f"Exploring {{n}} WikiData entities: history, geography, people & culture",
        "splashTitle": f"{emoji} MiniDi",
        "splashMessage": f"Loading {country_name} knowledge graph...",
        "mapCenter": [20.0, 0.0],
        "mapZoom": 4,
        "githubRepo": f"minidivn/{repo_name}",
        "dataSource": "WikiData",
        "dataPath": "_data/index.json",
        "chatCountry": country_name,
        "chatGreeting": f"Ask me about {country_name} — history, people, places, events.",
        "slides": [
            {
                "label": f"{country_name} Overview", "lang": "en",
                "labelAlt": f"{country_name}", "langAlt": country_code,
                "img": "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?w=1200&q=80",
                "grad": "linear-gradient(135deg, #1e3a5f, #2563eb, #60a5fa)"
            }
        ],
        "eras": [
            {"id": "historical", "emoji": "🏛️", "label": {"en": "Historical Eras", country_code: "Époques historiques"}, "years": [-9999, 9999], "color": "#22c55e"}
        ],
        "translations": {
            "en": {
                "entities": "Entities", "events": "Events", "people": "People", "places": "Places", "all": "All",
                "search": "Search...", "noResults": "No matching entities", "loading": "Loading...", "results": "{n} results",
                "page": "Page {p} of {t}", "prev": "Prev", "next": "Next", "details": "Details", "relations": "Relations",
                "wiki": "Open in WikiData", "timeline": "Timeline", "tree": "Dynasty Tree", "builtWith": "Built with",
                "source": "source", "chatTitle": "MiniDi Assistant", "chatPlaceholder": f"Ask about {country_name}...",
                "entitiesTotal": "{n} entities", "mapPlaces": "Places", "mapPeople": "People", "mapEvents": "Events"
            },
            country_code: {
                "entities": "Entities", "events": "Events", "people": "People", "places": "Places", "all": "All",
                "search": "Search...", "noResults": "No matching entities", "loading": "Loading...", "results": "{n} results",
                "page": "Page {p} of {t}", "prev": "Prev", "next": "Next", "details": "Details", "relations": "Relations",
                "wiki": "Open in WikiData", "timeline": "Timeline", "tree": "Dynasty Tree", "builtWith": "Built with",
                "source": "source", "chatTitle": "MiniDi Assistant", "chatPlaceholder": f"Ask about {country_name}...",
                "entitiesTotal": "{n} entities", "mapPlaces": "Places", "mapPeople": "People", "mapEvents": "Events"
            }
        }
    }

def generate_config_js(metadata, total_nodes):
    import copy
    meta = copy.deepcopy(metadata)
    meta["subtitle"] = meta["subtitle"].replace("{n}", f"{total_nodes:,}")
    meta["splashMessage"] = meta["splashMessage"].replace("{n}", f"{total_nodes:,}")
    
    import json
    json_str = json.dumps(meta, indent=2, ensure_ascii=False)
    return f"export default {json_str};\n"

def setup_frontend(country, country_name, repo_name, output_parent, total_nodes):
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    output_parent_abs = os.path.normpath(os.path.join(base_dir, output_parent))
    
    src_repo_ghpage = os.path.join(output_parent_abs, "minidi-data-ghpage")
    dst_repo = os.path.join(output_parent_abs, repo_name)
    dst_repo_ghpage = os.path.join(dst_repo, "src", "ghpage")
    
    # 1. Copy ghpage project skeleton if missing or to keep it synchronized (excluding temporary files)
    if os.path.exists(src_repo_ghpage):
        if src_repo_ghpage.lower() != dst_repo_ghpage.lower():
            print(f"[INFO] Synchronizing frontend project skeleton to: {dst_repo_ghpage}")
            shutil.copytree(
                src_repo_ghpage, 
                dst_repo_ghpage, 
                dirs_exist_ok=True,
                ignore=shutil.ignore_patterns("node_modules", ".git", "dist", "config.js")
            )
    else:
        print(f"[WARN] Source frontend template not found at {src_repo_ghpage}")
        return

    # 2. Generate customized config.js
    metadata = get_country_metadata(country, country_name, repo_name)
    config_js_content = generate_config_js(metadata, total_nodes)
    
    config_path = os.path.join(dst_repo_ghpage, "src", "config.js")
    with open(config_path, "w", encoding="utf-8") as f:
        f.write(config_js_content)
    print(f"[INFO] Generated frontend configuration at: {config_path}")

    # 3. Customize index.html title
    index_html_path = os.path.join(dst_repo_ghpage, "index.html")
    if os.path.exists(index_html_path):
        with open(index_html_path, "r", encoding="utf-8") as f:
            html = f.read()
        
        html = re.sub(
            r"<title>.*?</title>",
            f"<title>MiniDi - {country_name} Knowledge Graph</title>",
            html
        )
        
        with open(index_html_path, "w", encoding="utf-8") as f:
            f.write(html)
        print(f"[INFO] Customized index.html title for {country_name}")

    # 4. Install and Build using pnpm
    print(f"[INFO] Building frontend for {country_name}...")
    run_cmd(["pnpm", "install"], cwd=dst_repo_ghpage)
    run_cmd(["pnpm", "build"], cwd=dst_repo_ghpage)
    
    # Copy build output to docs directory
    dist_dir = os.path.join(dst_repo_ghpage, "dist")
    docs_dir = os.path.join(dst_repo, "docs")
    if os.path.exists(dist_dir):
        print(f"[INFO] Copying build artifacts from {dist_dir} to {docs_dir}")
        shutil.copytree(dist_dir, docs_dir, dirs_exist_ok=True)

    # 5. Ensure .nojekyll exists in target docs directory to bypass Jekyll filtering of underscore folders
    nojekyll_path = os.path.join(dst_repo, "docs", ".nojekyll")
    with open(nojekyll_path, "w") as f:
        pass
    print(f"[INFO] Created .nojekyll at: {nojekyll_path}")
    print(f"[SUCCESS] Frontend built successfully and copied to {repo_name}/docs/")

def parse_args():
    parser = argparse.ArgumentParser(description="Run complete crawl and export pipeline for a country.")
    parser.add_argument(
        "--country", "-c",
        required=True,
        help="Country code to run (e.g., vn, en, zh)"
    )
    parser.add_argument(
        "--limit", "-l",
        type=int,
        default=0,
        help="Optional crawl node limit (defaults to config partition settings)"
    )
    parser.add_argument(
        "--release", "-r",
        action="store_true",
        help="Run cargo commands with --release optimization profile"
    )
    parser.add_argument(
        "--skip-stats",
        action="store_true",
        help="Skip generating word list and letter statistics"
    )
    parser.add_argument(
        "--skip-crawl",
        action="store_true",
        help="Skip crawling external sources (e.g. Wikidata)"
    )
    parser.add_argument(
        "--skip-export",
        action="store_true",
        help="Skip exporting database graph to JSON files"
    )
    parser.add_argument(
        "--skip-frontend",
        action="store_true",
        help="Skip copying/customizing and building Vite frontend"
    )
    return parser.parse_args()

def run_cmd(cmd_args, cwd=None):
    cmd_str = " ".join(cmd_args)
    print(f"\n[INFO] Running: {cmd_str}")
    import os
    env_vars = {**os.environ, "CI": "true"}
    result = subprocess.run(cmd_args, cwd=cwd, shell=True, env=env_vars)
    if result.returncode != 0:
        print(f"[ERROR] Command failed with exit code: {result.returncode}", file=sys.stderr)
        sys.exit(result.returncode)

def main():
    args = parse_args()
    country = args.country.lower()

    # Determine paths relative to root directory
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(base_dir)

    print(f"=== Starting Minidi Pipeline for Country: {country.upper()} ===")

    # 0. Load configs/countries.json to resolve metadata
    with open("configs/countries.json", "r", encoding="utf-8") as f:
        countries_cfg = json.load(f)
    
    country_info = None
    for c in countries_cfg["countries"]:
        if c["code"].lower() == country:
            country_info = c
            break
            
    if not country_info:
        print(f"[ERROR] Country '{country}' not found in configs/countries.json", file=sys.stderr)
        sys.exit(1)

    country_name = country_info["name"]
    repo_name = country_info["repo"]
    output_parent = countries_cfg.get("output_parent", "../../Minidi/Data")

    # 1. Check for configured wordlist dataset
    if not args.skip_stats:
        dataset_config = os.path.join("configs", "dataset", "wordlist_sources.json")
        if os.path.exists(dataset_config):
            with open(dataset_config, "r", encoding="utf-8") as f:
                sources = json.load(f)
            
            if country in sources:
                source_info = sources[country]
                print(f"[INFO] Found dictionary wordlist dataset for {country}: {source_info['description']}")
                # Run build_wordlist_stats.py script
                stats_script = os.path.join("scripts", "build_wordlist_stats.py")
                run_cmd([
                    sys.executable, stats_script,
                    "--country", country,
                    "--source", source_info["source"]
                ])
            else:
                print(f"[INFO] No wordlist dataset configured for country '{country}'. Skipping stats generation.")
        else:
            print("[WARN] Config file configs/dataset/wordlist_sources.json not found. Skipping stats.")
    else:
        print("[INFO] Skipping wordlist and statistics generation (--skip-stats)")

    # Determine release flag
    profile = ["--release"] if args.release else []

    # 2. Run Cargo Ingestion / Crawl
    if not args.skip_crawl:
        crawl_cmd = ["cargo", "run"] + profile + ["--", "crawl", "--country", country]
        if args.limit > 0:
            crawl_cmd += ["--limit", str(args.limit)]
        run_cmd(crawl_cmd)
    else:
        print("[INFO] Skipping data crawling (--skip-crawl)")

    # 3. Run Cargo Export
    if not args.skip_export:
        export_cmd = ["cargo", "run"] + profile + ["--", "export", "--country", country]
        run_cmd(export_cmd)
    else:
        print("[INFO] Skipping database exporting (--skip-export)")

    # 4. Check for generated statistics report and summarize
    report_file = os.path.join("stats", country, "report.json")
    total_nodes = 0
    total_edges = 0
    if os.path.exists(report_file):
        with open(report_file, "r", encoding="utf-8") as f:
            report = json.load(f)
        total_nodes = report.get("total_nodes", 0)
        total_edges = report.get("total_edges", 0)

    # 5. Build and Deploy Web Frontend using pnpm
    if not args.skip_frontend:
        setup_frontend(country, country_name, repo_name, output_parent, total_nodes)
    else:
        print("[INFO] Skipping frontend generation and compile build (--skip-frontend)")

    print(f"\n==================================================")
    print(f"[SUCCESS] Pipeline Completed Successfully for {country.upper()}!")
    print(f"  - Total nodes: {total_nodes}")
    print(f"  - Total edges: {total_edges}")
    print(f"  - Output Target Path: {repo_name}/docs/_data")
    if not args.skip_frontend:
        print(f"  - Search Frontend Web App: {repo_name}/docs/index.html")
    print(f"==================================================")

if __name__ == "__main__":
    main()
