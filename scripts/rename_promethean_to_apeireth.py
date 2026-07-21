"""rename promethean → apeireth in hardcoded paths (UTF-8 safe)."""
from pathlib import Path

files = [
    "apeireth/cron_self_update.py",
    "apeireth/deep_asi_research.py",
    "apeireth/deep_list_research.py",
    "apeireth/deep_research.py",
    "apeireth/deep_research_science.py",
    "apeireth/evolve_research.py",
    "apeireth/master_list_research.py",
    "apeireth/master_list_via_pat.py",
    "apeireth/memoryos_inspect.py",
    "apeireth/philosophy_biology_research.py",
    "apeireth/trending_research.py",
    "apeireth/v3_3_self_decision.py",
]

replacements = [
    (r".openclaw\workspace\promethean",
     r".openclaw\workspace\apeireth"),
    ('"promethean"', '"apeireth"'),
    ("# promethean/", "# apeireth/"),
    ("default: promethean/", "default: apeireth/"),
    ("promethean/DEV-LOG", "apeireth/DEV-LOG"),
]

for f in files:
    p = Path(f)
    if not p.exists():
        print(f"skip (missing): {f}")
        continue
    text = p.read_text(encoding="utf-8")
    original = text
    for old, new in replacements:
        text = text.replace(old, new)
    if text != original:
        p.write_text(text, encoding="utf-8")
        print(f"fixed: {f}")
    else:
        print(f"no change: {f}")

# Verify
print("\n=== verification ===")
import subprocess
for f in files:
    p = Path(f)
    if not p.exists():
        continue
    text = p.read_text(encoding="utf-8")
    hits = [line for line in text.splitlines() if "promethean" in line]
    if hits:
        print(f"{f}: {hits}")
print("done")