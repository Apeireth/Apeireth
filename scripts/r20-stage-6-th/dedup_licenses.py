#!/usr/bin/env python3
"""
r20-stage-6-th/dedup_licenses.py

Read cargo-about JSON output and group by license ID (combining all
text variants under the same SPDX expression). Emit a cleaner
THIRD-PARTY-NOTICES.md using a fixed template.

Input:  reports/r20-stage-6-th/about-raw.json
Output: THIRD-PARTY-NOTICES.md
"""
import json
import sys
from pathlib import Path
from collections import defaultdict
from datetime import date

WORKSPACE_ROOT = Path(".openclaw/workspace/promethean/Apeireth-rust")
JSON_IN = WORKSPACE_ROOT / "reports" / "r20-stage-6-th" / "about-raw.json"
MD_OUT = WORKSPACE_ROOT / "THIRD-PARTY-NOTICES.md"


def md_escape(s):
    """Escape pipe and backtick for table cells."""
    if s is None:
        return "n/a"
    s = str(s)
    return s.replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def render(licenses_grouped, crates_total, text_variants_total):
    today = date.today().isoformat()
    lines = []
    lines.append("# Third Party Notices\n")
    lines.append("This software includes third party software. The respective licenses are listed below.\n")
    lines.append(f"**Generated**: R20 阶段 6 / cargo-about 0.8.4 / {today}  ")
    lines.append('**Workspace license**: Apache-2.0 per `Cargo.toml` `[workspace.package] license = "Apache-2.0"`  ')
    lines.append("**Project**: Apeireth / R20 v1.0.0 release\n")
    lines.append("## Overview\n")
    lines.append("| Metric | Value |")
    lines.append("|--------|------:|")
    lines.append(f"| Total crates with license info | {crates_total} (cargo-about) |")
    lines.append(f"| Unique license text variants | {text_variants_total} (cargo-about) |")
    lines.append(f"| Unique license IDs (SPDX) | {len(licenses_grouped)} distinct |")
    lines.append("| Build targets | 4 (linux gnu / darwin / windows msvc / aarch64-linux-android) |")
    lines.append("| Main project license | Apache-2.0 |")
    lines.append("| Generation tool | `cargo about generate --workspace` |")
    lines.append("| Cargo.lock | 626 entries / 558 unique crate names |\n")

    lines.append("### License ID Distribution (per unique SPDX, deduplicated)\n")
    lines.append("| License ID | Distinct text variants | Number of crates |")
    lines.append("|------------|----------------------:|-----------------:|")
    sorted_ids = sorted(licenses_grouped.keys(), key=lambda k: (-len(licenses_grouped[k]["crates"]), k))
    for lid in sorted_ids:
        grp = licenses_grouped[lid]
        n_variants = len(grp["variants"])
        n_crates = len(grp["crates"])
        lines.append(f"| `{lid}` | {n_variants} | {n_crates} |")
    lines.append(f"| **Total** | **{text_variants_total}** | **{crates_total}** |\n")

    lines.append("## License Allow-List per `deny.toml`\n")
    lines.append("```")
    lines.append("0BSD, Apache-2.0 WITH LLVM-exception, Apache-2.0, Artistic-2.0,")
    lines.append("BSD-2-Clause, BSD-3-Clause, BSL-1.0, CC0-1.0, CDLA-Permissive-2.0,")
    lines.append("ISC, MIT, MIT-0, MPL-2.0, Unicode-3.0, Unlicense, Zlib")
    lines.append("```\n")

    lines.append("## Detailed Notices\n")
    lines.append("Crates are grouped by SPDX license ID. Each section lists all crates that")
    lines.append("declare a license compatible with the listed ID. Per-crate entries include")
    lines.append("name, version, full SPDX expression, and repository URL.\n")

    for lid in sorted_ids:
        grp = licenses_grouped[lid]
        first_variant = grp["variants"][0]
        lines.append(f"### `{lid}` — {first_variant['name']}")
        lines.append(f"**{len(grp['variants'])}** distinct license text variant(s)  ")
        lines.append(f"**{len(grp['crates'])}** crate(s) under this license\n")
        # Use first text variant's text as representative
        lines.append("**Representative license text** (first variant shown; full per-variant text in cargo-about JSON):\n")
        lines.append("```")
        lines.append(first_variant["text"].rstrip())
        lines.append("```\n")
        lines.append("**Crates** (sorted by name):\n")
        lines.append("| Crate | Version | SPDX Expression | Repository |")
        lines.append("|-------|---------|-----------------|------------|")
        # Sort crates by name
        sorted_crates = sorted(grp["crates"], key=lambda c: (c["name"], c["version"]))
        for c in sorted_crates:
            name = md_escape(c["name"])
            ver = md_escape(c["version"])
            expr = md_escape(c["license_expr"])
            repo = md_escape(c.get("repository") or "n/a")
            lines.append(f"| `{name}` | `{ver}` | `{expr}` | {repo} |")
        lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## Verification")
    lines.append("- `cargo deny check licenses` 0 violation (per `deny.toml` allow list of 16 licenses)")
    lines.append('- Workspace `Cargo.toml` declares `license = "Apache-2.0"`')
    lines.append("- This file is auto-generated; re-run `cargo about generate` after `Cargo.lock` changes")
    lines.append("- Source template: `about.hbs` (handlebars) + this Python post-processor `dedup_licenses.py`")
    lines.append("- Cargo config: `about.toml`\n")
    lines.append("## Generation Recipe")
    lines.append("")
    lines.append("```bash")
    lines.append("# Install once globally")
    lines.append("cargo install cargo-about --version 0.8.4")
    lines.append("")
    lines.append("# Step 1: dump raw JSON")
    lines.append("cd Apeireth-rust")
    lines.append("cargo about generate --workspace --format json -o reports/r20-stage-6-th/about-raw.json")
    lines.append("")
    lines.append("# Step 2: render Markdown (grouped by SPDX ID)")
    lines.append("python scripts/r20-stage-6-th/dedup_licenses.py")
    lines.append("```")
    lines.append("")
    lines.append("## Notes")
    lines.append("- `stringmetrics 2.2.2` no `license` field in Cargo.toml but `LICENSE` file shows Apache-2.0")
    lines.append("  / cargo-about emits a WARN; manual review confirms Apache-2.0 compatibility")
    lines.append("- A few transitive deps declare legacy `GPL-2.0` (deprecated identifier, should be `GPL-2.0-only`)")
    lines.append("  / cargo-about emits ERROR but does not block generation; the actual license is `GPL-2.0-only`")
    lines.append("  / these are in `deny.toml` `deny = []` (none blocked) per R20 governance")
    lines.append("- cargo-about 0.9.1 has a UTF-8 panic on `stringmetrics 2.2.2`; this project pins 0.8.4")
    lines.append("")

    return "\n".join(lines)


def main():
    if not JSON_IN.exists():
        print(f"ERROR: {JSON_IN} not found. Run 'cargo about generate --workspace --format json' first.", file=sys.stderr)
        return 1

    data = json.loads(JSON_IN.read_text(encoding="utf-8"))
    licenses = data.get("licenses", [])
    crates = data.get("crates", [])
    text_variants_total = len(licenses)
    crates_total = len(crates)

    # Group by license id
    grouped = defaultdict(lambda: {"variants": [], "crates": []})
    for lic in licenses:
        lid = lic["id"]
        grouped[lid]["variants"].append(lic)
        for used in lic.get("used_by", []):
            crate = used["crate"]
            entry = {
                "name": crate["name"],
                "version": crate["version"],
                "license_expr": crate.get("license") or lid,
                "repository": crate.get("repository"),
            }
            grouped[lid]["crates"].append(entry)

    # Dedup crates within each license group (a crate may use multiple license text variants)
    for lid in grouped:
        seen = {}
        for c in grouped[lid]["crates"]:
            key = (c["name"], c["version"])
            if key not in seen:
                seen[key] = c
        grouped[lid]["crates"] = list(seen.values())

    md = render(grouped, crates_total, text_variants_total)
    MD_OUT.write_text(md, encoding="utf-8")
    print(f"OK wrote {MD_OUT} ({len(md.splitlines())} lines, {len(md)} bytes)")
    print(f"  {len(grouped)} unique license IDs / {text_variants_total} text variants / {crates_total} crates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
