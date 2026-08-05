#!/usr/bin/env python3
"""
Reproducible Dataset Script: Word List & Stats Builder
Parses a raw wordlist file, outputs words.txt, deletes old wordlists, 
and computes a character statistics map (words.stats.json) for the target language.

Usage:
  python build_wordlist_stats.py --country vn --source ../../Minidi/Data/vn_words/Viet74K.txt
"""

import os
import sys
import json
import argparse
import string
import unicodedata

def parse_args():
    parser = argparse.ArgumentParser(description="Clean word lists and build character frequency stats.")
    parser.add_argument(
        "--country", "-c",
        required=True,
        help="Country code (e.g., vn, en, zh) corresponding to configs/countries.json"
    )
    parser.add_argument(
        "--source", "-s",
        required=True,
        help="Path to the raw input word list text file (e.g., Viet74K.txt)"
    )
    parser.add_argument(
        "--config",
        default="configs/countries.json",
        help="Path to countries.json config file"
    )
    return parser.parse_args()

def resolve_output_dir(config_path, country_code):
    if not os.path.exists(config_path):
        # Fallback to current dir if run from root/scripts
        config_path = os.path.join(os.path.dirname(__file__), "..", config_path)
        if not os.path.exists(config_path):
            raise FileNotFoundError(f"Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = json.load(f)

    countries = config.get("countries", [])
    country_cfg = next((c for c in countries if c["code"] == country_code), None)
    if not country_cfg:
        raise ValueError(f"Unknown country code in config: {country_code}")

    output_parent = config.get("output_parent", "../../Minidi/Data")
    # Resolve relative paths
    if output_parent.startswith("../"):
        # Make path relative to where scripts live
        base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        output_parent = os.path.normpath(os.path.join(base_dir, output_parent))

    repo_dir = os.path.join(output_parent, country_cfg["repo"])
    docs_words_dir = os.path.join(repo_dir, "docs", "_data", "words")
    return docs_words_dir

def download_wordlist(country_code, target_path):
    lang_map = {
        "vn": "vi",
        "en": "en",
        "zh": "zh_cn",
        "fr": "fr",
        "es": "es",
        "de": "de",
        "ru": "ru",
        "hi": "hi",
        "pt": "pt_br",
        "ar": "ar",
        "bn": "bn"
    }
    lang = lang_map.get(country_code, country_code)
    url = f"https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/{lang}/{lang}_50k.txt"
    
    print(f"[INFO] Source file not found. Attempting to download standard frequency wordlist for '{country_code}' from: {url}")
    import urllib.request
    
    # Ensure directory exists
    os.makedirs(os.path.dirname(target_path), exist_ok=True)
    
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            with open(target_path, "wb") as f:
                f.write(response.read())
        print(f"[INFO] Download completed. Saved to: {target_path}")
    except Exception as e:
        raise RuntimeError(f"Failed to download wordlist for country code '{country_code}' from {url}: {e}")

def clean_and_compute_stats(source_path, target_words_dir, country_code):
    if not os.path.exists(source_path):
        download_wordlist(country_code, source_path)

    print(f"Reading source wordlist: {source_path}")
    with open(source_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    cleaned_words = []
    char_counts = {}

    for line in lines:
        raw_word = line.strip()
        if not raw_word:
            continue

        # If it is space separated (e.g. from HermitDave's list: "word frequency"), extract only the word
        parts = raw_word.split()
        if not parts:
            continue
        
        # Check if last element is a frequency digit, if so reconstruct word without it
        if parts[-1].isdigit() and len(parts) > 1:
            word = " ".join(parts[:-1])
        else:
            word = raw_word

        cleaned_words.append(word)

        # Compute character frequency statistics
        normalized_word = unicodedata.normalize("NFC", word.lower())
        for char in normalized_word:
            if char.isspace() or char in string.punctuation or char.isdigit():
                continue
            cat = unicodedata.category(char)
            if cat.startswith("L") or cat.startswith("M"):
                char_counts[char] = char_counts.get(char, 0) + 1

    # Ensure target directory exists
    os.makedirs(target_words_dir, exist_ok=True)
    target_words_path = os.path.join(target_words_dir, "words.txt")
    target_stats_path = os.path.join(target_words_dir, "words.stats.json")

    # Clean old .txt files in the target directory so there is ONLY one words.txt
    print(f"Cleaning existing .txt files in: {target_words_dir}")
    for item in os.listdir(target_words_dir):
        if item.endswith(".txt") and item != "words.txt":
            file_to_remove = os.path.join(target_words_dir, item)
            os.remove(file_to_remove)
            print(f"  Removed obsolete list: {item}")

    # Deduplicate and sort words alphabetically
    sorted_words = sorted(list(set(cleaned_words)), key=lambda s: unicodedata.normalize("NFC", s.lower()))

    # Write cleaned words list to words.txt
    print(f"Writing unified words list to: {target_words_path}")
    with open(target_words_path, "w", encoding="utf-8") as f:
        for word in sorted_words:
            f.write(f"{word}\n")

    # Sort stats alphabetically by character key for deterministic output
    sorted_stats = dict(sorted(char_counts.items()))

    # Write character statistics
    print(f"Writing character statistics to: {target_stats_path}")
    with open(target_stats_path, "w", encoding="utf-8") as f:
        json.dump(sorted_stats, f, indent=2, ensure_ascii=False)

    print("Success! Processed {} unique words.".format(len(sorted_words)))
    print("Character stats map contains {} unique letters.".format(len(sorted_stats)))

def main():
    args = parse_args()
    try:
        target_dir = resolve_output_dir(args.config, args.country)
        clean_and_compute_stats(args.source, target_dir, args.country)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
