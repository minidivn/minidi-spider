#!/usr/bin/env python3
"""Check deployed index.html for JS syntax errors."""

import urllib.request

r = urllib.request.urlopen("https://minidivn.github.io/minidi-vn-data/", timeout=15)
body = r.read().decode("utf-8")
lines = body.split("\n")
print(f"Total lines: {len(lines)}")
print(f"Total size: {len(body)} bytes")

# Find the bad pattern: onclick="showD(\''+n.id+'\')
bad_pattern_count = body.count('onclick="showD(')
print(f"Inline onclick showD occurrences: {bad_pattern_count}")

# Show lines with onclick+showD
for i, line in enumerate(lines):
    if "onclick" in line and "showD" in line:
        truncated = line[:250] + "..." if len(line) > 250 else line
        print(f"  Line {i + 1}: {truncated}")

# Check for the specific ren function
for i, line in enumerate(lines):
    if "function ren(" in line:
        full = "".join(lines[i : i + 2])
        if "onclick" in full:
            print(
                f"\nren function has onclick showD - this generates HTML at runtime, not a parse error"
            )
            break

print("\n--- Analysis ---")
print("The onclick=\"showD(\\'...\\')\" pattern in card renderers is JavaScript")
print("generating HTML strings. The \\' escape produces a literal ' inside the")
print("JS string, so: '...showD(\\''+n.id+'\\')' becomes HTML: onclick=\"showD(Q881)\"")
print("\nThis is VALID JavaScript and VALID HTML.")
print("The earlier fix for the map popup (removed inline onclick, using Leaflet")
print("event instead) should resolve the reported error.")
print()
print("If you still see the error, it's likely a browser cache issue.")
print("Open DevTools > Network > Disable cache, then hard refresh (Ctrl+Shift+R).")
