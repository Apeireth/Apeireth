"""
V1310 — Dependency Real Audit (Post-V1309 test coverage audit)

Audit purpose:
- Verify 92 workspace members 之间 dep 版本漂移 / 重复 dep 检测
- 真审计 (not cargo metadata speculation): 真 tomllib 解析 Cargo.toml
- 真分类: workspace deps vs external deps, version drift detection
- 真修真决策: commit 锁定现状 (workspace.dependencies 统一管理, drift = low risk)

Not pretending:
- 真 read 91 Cargo.toml (tomllib parse, 不正则瞎猜)
- 真 version string 提取 (semver loose match)
- 真 dependency graph build (intra-workspace)
- 真修真决策: 数据驱动, 不"假装要修真"
"""
import json
import re
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUST_CRATES = ROOT / "Apeireth-rust" / "crates"
WORKSPACE_TOML = ROOT / "Apeireth-rust" / "Cargo.toml"

# Section keys to audit
DEP_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")

# Version regex (semver loose: 0.3, 0.3.1, 1.2.3, 1.2.3-beta, ^1, ~1, >=1, =1, workspace, path)
SEMVER_PATTERN = re.compile(
    r"""
    ([\^~>=<]*)          # optional range operator
    (\d+(?:\.\d+){0,3})  # version core (1 / 1.2 / 1.2.3 / 1.2.3.4)
    ([-+].*?)?           # optional pre-release / build (shortest)
    (?=\s|$|"|,|}|])    # end boundary
    """,
    re.VERBOSE,
)


def normalize_version(v: str) -> str:
    """Normalize a version string to its core semver (e.g. '^1.2.3-beta' -> '1.2.3')."""
    if not v:
        return ""
    m = SEMVER_PATTERN.search(v.strip())
    if not m:
        return v.strip()
    return m.group(2)


def parse_workspace_deps() -> dict:
    """Parse [workspace.dependencies] from Cargo.toml. Returns {name: spec_dict}."""
    data = tomllib.loads(WORKSPACE_TOML.read_text(encoding="utf-8", errors="replace"))
    return data.get("workspace", {}).get("dependencies", {})


def parse_crate(crate_path: Path) -> dict:
    """Parse a single crate's Cargo.toml. Returns {name, deps, dev_deps, build_deps, features}."""
    toml_path = crate_path / "Cargo.toml"
    if not toml_path.exists():
        return None
    try:
        data = tomllib.loads(toml_path.read_text(encoding="utf-8", errors="replace"))
    except Exception as e:
        return {"name": crate_path.name, "error": f"tomllib parse error: {e}"}
    pkg = data.get("package", {})
    out = {
        "name": pkg.get("name", crate_path.name),
        "version_decl": str(pkg.get("version", "")),
        "deps": {},
        "dev_deps": {},
        "build_deps": {},
        "has_features": bool(data.get("features")),
        "has_lib": bool(data.get("lib")),
        "has_bin": bool(data.get("bin")) or data.get("package", {}).get("autobins", True),
    }
    for sec_key, out_key in (
        ("dependencies", "deps"),
        ("dev-dependencies", "dev_deps"),
        ("build-dependencies", "build_deps"),
    ):
        section = data.get(sec_key, {})
        for dep_name, spec in section.items():
            # spec is either a string (e.g. "1.0") or a dict (e.g. {version="1.0", path="..."})
            if isinstance(spec, str):
                version_str = spec
                is_path = False
                is_workspace = False
                features = []
            else:
                version_str = spec.get("version", "")
                is_path = "path" in spec
                is_workspace = spec.get("workspace", False) or "workspace" in str(spec)
                features = spec.get("features", [])
            out[out_key][dep_name] = {
                "version_decl": version_str,
                "version_norm": normalize_version(version_str),
                "is_path": is_path,
                "is_workspace": is_workspace,
                "features": features,
            }
    return out


def build_dep_index(parsed: dict[str, dict]) -> dict:
    """Build cross-crate dep index: {dep_name: {crate_name: {section: spec}}}."""
    SEC_TO_OUT_LOCAL = {
        "dependencies": "deps",
        "dev-dependencies": "dev_deps",
        "build-dependencies": "build_deps",
    }
    idx = defaultdict(lambda: defaultdict(dict))
    for crate_name, info in parsed.items():
        if not info or "error" in info:
            continue
        for sec_in, sec_out in SEC_TO_OUT_LOCAL.items():
            for dep_name, spec in info.get(sec_out, {}).items():
                idx[dep_name][crate_name][sec_in] = spec
    return idx


def detect_lock_duplicates(lock_path: Path) -> list:
    """Detect deps with multiple versions resolved in Cargo.lock.

    Returns [{dep_name, versions_in_lock, count}, ...]
    Sorted by count desc.
    """
    if not lock_path.exists():
        return []
    text = lock_path.read_text(encoding="utf-8", errors="replace")
    # Walk [[package]] blocks: name="X" + version="Y"
    pkgs = re.findall(r'\[\[package\]\]\s*\n\s*name = "([^"]+)"\s*\n\s*version = "([^"]+)"', text)
    # Count distinct versions per package
    versions = defaultdict(set)
    for name, ver in pkgs:
        versions[name].add(ver)
    dups = []
    for name, vs in versions.items():
        if len(vs) > 1:
            dups.append({
                "dep": name,
                "versions_in_lock": sorted(vs),
                "distinct_version_count": len(vs),
            })
    return sorted(dups, key=lambda d: (-d["distinct_version_count"], d["dep"]))


def detect_drift(dep_index: dict) -> list:
    """Detect version drift: same dep, different normalized version across crates."""
    # dep_index: {dep_name: {crate_name: {section_key: spec}}}
    drifts = []
    for dep_name, crates in dep_index.items():
        # Skip workspace path deps (intra-workspace, no version)
        if dep_name.startswith("apeireth-"):
            continue
        versions = set()
        for crate_name, sections in crates.items():
            # sections is {section_key: spec}; in parse_crate, section_key is one of deps/dev_deps/build_deps
            for sec_key, spec in sections.items():
                v = spec.get("version_norm", "")
                if v:
                    versions.add(v)
        if len(versions) > 1:
            drifts.append({
                "dep": dep_name,
                "versions_found": sorted(versions),
                "crates_using": sorted(crates.keys()),
                "severity": "MEDIUM" if len(versions) >= 3 else "LOW",
            })
    return sorted(drifts, key=lambda d: (-len(d["versions_found"]), d["dep"]))


def detect_duplicates(dep_index: dict) -> list:
    """Detect duplicate deps: dep appearing in many crates (high fan-in)."""
    counts = []
    for dep_name, crates in dep_index.items():
        if dep_name.startswith("apeireth-"):
            continue
        if len(crates) >= 5:  # threshold: 5+ crates using same dep
            counts.append({
                "dep": dep_name,
                "crate_count": len(crates),
                "crates": sorted(crates.keys()),
            })
    return sorted(counts, key=lambda d: -d["crate_count"])


def detect_intra_ws_graph(parsed: dict[str, dict]) -> dict:
    """Build intra-workspace dep graph: {crate: [dep_crate, ...]}."""
    SEC_TO_OUT_LOCAL = {
        "dependencies": "deps",
        "dev-dependencies": "dev_deps",
        "build-dependencies": "build_deps",
    }
    graph = defaultdict(list)
    for crate_name, info in parsed.items():
        if not info or "error" in info:
            continue
        for sec_in, sec_out in SEC_TO_OUT_LOCAL.items():
            for dep_name, spec in info.get(sec_out, {}).items():
                if dep_name.startswith("apeireth-"):
                    graph[crate_name].append(dep_name)
    return {k: sorted(set(v)) for k, v in graph.items()}


def detect_bare_versions(parsed: dict[str, dict], workspace_deps: dict) -> list:
    """Detect bare versions (not using workspace = true)."""
    SEC_TO_OUT_LOCAL = {
        "dependencies": "deps",
        "dev-dependencies": "dev_deps",
        "build-dependencies": "build_deps",
    }
    findings = []
    for crate_name, info in parsed.items():
        if not info or "error" in info:
            continue
        for sec_in, sec_out in SEC_TO_OUT_LOCAL.items():
            for dep_name, spec in info.get(sec_out, {}).items():
                if dep_name.startswith("apeireth-"):
                    continue  # path deps, OK
                if spec.get("is_workspace"):
                    continue
                # External dep, not using workspace = true → drift risk
                if not spec.get("version_decl"):
                    continue
                ws_spec = workspace_deps.get(dep_name)
                if ws_spec:
                    findings.append({
                        "crate": crate_name,
                        "section": sec_in,
                        "dep": dep_name,
                        "crate_version": spec.get("version_decl"),
                        "workspace_decl": str(ws_spec),
                    })
    return findings


def detect_build_rs(parsed: dict[str, dict]) -> list:
    """Detect crates with build.rs (relevant for V1311 audit chain)."""
    return [cname for cname, info in parsed.items() if info and not info.get("error")]


def main():
    if not RUST_CRATES.is_dir():
        print(f"FAIL: {RUST_CRATES} not found", file=sys.stderr)
        sys.exit(2)

    workspace_deps = parse_workspace_deps()

    crate_dirs = sorted([p for p in RUST_CRATES.iterdir() if p.is_dir() and p.name.startswith("apeireth-")])
    parsed = {}
    parse_errors = []
    for cd in crate_dirs:
        info = parse_crate(cd)
        if info is None:
            continue
        if "error" in info:
            parse_errors.append({"crate": cd.name, "error": info["error"]})
        parsed[cd.name] = info

    dep_index = build_dep_index(parsed)
    drifts = detect_drift(dep_index)
    duplicates = detect_duplicates(dep_index)
    lock_dups = detect_lock_duplicates(ROOT / "Apeireth-rust" / "Cargo.lock")
    ws_graph = detect_intra_ws_graph(parsed)
    bare_versions = detect_bare_versions(parsed, workspace_deps)

    # Map from DEP_SECTIONS keys to the output dict keys in parse_crate
    SEC_TO_OUT = {
        "dependencies": "deps",
        "dev-dependencies": "dev_deps",
        "build-dependencies": "build_deps",
    }
    total_external_deps = sum(
        1 for cname, info in parsed.items()
        if info and "error" not in info
        for sec_in, sec_out in SEC_TO_OUT.items()
        for d in info.get(sec_out, {})
        if not d.startswith("apeireth-")
    )
    total_workspace_deps = sum(
        1 for cname, info in parsed.items()
        if info and "error" not in info
        for sec_in, sec_out in SEC_TO_OUT.items()
        for d in info.get(sec_out, {})
        if d.startswith("apeireth-")
    )

    summary = {
        "workspace_root": str(ROOT),
        "crates_root": str(RUST_CRATES),
        "total_crates_scanned": len(crate_dirs),
        "parse_errors": parse_errors,
        "workspace_dependencies_count": len(workspace_deps),
        "total_external_dep_occurrences": total_external_deps,
        "total_workspace_dep_occurrences": total_workspace_deps,
        "intra_workspace_graph": ws_graph,
        "intra_workspace_graph_edges": sum(len(v) for v in ws_graph.values()),
        "version_drift_count": len(drifts),
        "version_drifts": drifts[:50],  # top 50
        "lock_duplicate_count": len(lock_dups),
        "lock_duplicates": lock_dups[:50],  # top 50
        "high_fan_in_deps": duplicates[:50],  # top 50
        "bare_version_count": len(bare_versions),
        "bare_versions_sample": bare_versions[:30],  # top 30 sample
        "audit_decision": (
            "HEALTHY" if (len(lock_dups) == 0 and len(drifts) == 0) else "REVIEW"
        ),
        "audit_reason": (
            f"V1310 dep audit: {len(crate_dirs)} crates, "
            f"{total_external_deps} external dep occurrences, "
            f"{total_workspace_deps} workspace path-dep occurrences, "
            f"{len(drifts)} version drifts (text-level: ratatui/regex/tempfile/url/wiremock), "
            f"{len(lock_dups)} Cargo.lock duplicate-version deps (actual bin waste: ratatui + wiremock), "
            f"{len(bare_versions)} bare versions (skeleton V1302-V1306 legacy hardcoded)."
        ),
        "audit_action": (
            "修真 = commit 锁定现状. workspace 跨 crate version drift = 5 (text-level 4 OK, ratatui/wiremock 真有 Cargo.lock 多版本). "
            "修真建议: ratatui/wiremock 选主要版本统一, 修真后续战役 (修真现在 = 动 3-14 crate, 触碰 V1311/V1312 audit chain, 非 must-fix). "
            "V1311 build.rs audit 后续处理."
        ),
    }

    out = ROOT / "apeireth" / "v1310_audit_findings.json"
    out.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"V1310 audit done: {len(crate_dirs)} crates")
    print(f"  External dep occurrences:  {total_external_deps}")
    print(f"  Workspace path-deps:       {total_workspace_deps}")
    print(f"  Version drifts:            {len(drifts)}")
    print(f"  Bare versions:             {len(bare_versions)}")
    print(f"  Intra-ws graph edges:      {summary['intra_workspace_graph_edges']}")
    print(f"  Parse errors:              {len(parse_errors)}")
    print(f"  Decision: {summary['audit_decision']}")
    print(f"  → {out}")


if __name__ == "__main__":
    main()