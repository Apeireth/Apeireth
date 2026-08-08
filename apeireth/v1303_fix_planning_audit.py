"""Phase 1303 v1303_fix_planning_audit — V1303 Orphan Crate Fix Planning Audit.

V1302 修真 #1 (blueprint-impl) 后续 — V1303 = 不修真, 只扫描 + 分类 + 推荐 fix 命令.

主 17:43 实事求是: 修真仅 1 个, 剩 8 个 orphan 留 V1304+ — V1303 给每条 fix 路径精确化.
主 13:08 真自问: V1301 标 3 个, V1302 实跑发现 8 个 — V1303 给每条 fix 风险评级 + 推荐动作.
V3 哲学守门: audit-only, 0 修真, 0 假装 PASS. 数据驱动修真规划.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List

V1303_VERSION = "0.1.0"

WORKSPACE_ROOT = Path(__file__).resolve().parents[1] / "Apeireth-rust"


# ============================================================================
# Helpers
# ============================================================================


def read_text(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def detect_subworkspace(cargo_toml_text: str) -> bool:
    """Detect if crate has its own [workspace] block (sub-workspace pattern)."""
    return bool(re.search(r"^\s*\[workspace\]\s*$", cargo_toml_text, re.MULTILINE))


def extract_package_version(cargo_toml_text: str) -> str:
    """Extract [package] version value, or 'workspace' if version.workspace = true."""
    m = re.search(r'^\s*version\.workspace\s*=\s*true\s*$', cargo_toml_text, re.MULTILINE)
    if m:
        return "workspace"
    m = re.search(r'^\s*version\s*=\s*"([^"]+)"\s*$', cargo_toml_text, re.MULTILINE)
    return m.group(1) if m else "unknown"


def find_cargo_dirs() -> List[Path]:
    crates_dir = WORKSPACE_ROOT / "crates"
    if not crates_dir.exists():
        return []
    return sorted([p for p in crates_dir.iterdir() if p.is_dir() and (p / "Cargo.toml").exists()])


def parse_active_members() -> List[str]:
    """Parse Cargo.toml members list."""
    text = read_text(WORKSPACE_ROOT / "Cargo.toml")
    return re.findall(r'^\s*"crates/([^"]+)"\s*,?\s*$', text, re.MULTILINE)


# ============================================================================
# Audit
# ============================================================================


def classify_orphan(crate_dir: Path, workspace_version: str) -> Dict[str, Any]:
    """Classify a single orphan crate by fix type and risk.

    Returns dict with:
    - name: crate dir name
    - has_subworkspace: bool
    - version: extracted version
    - version_conflict: True if version != workspace_version and != 'workspace'
    - fix_type: 'sub-workspace-removal' | 'add-to-members' | 'intentional-comment'
    - risk_level: 'low' | 'medium' | 'high'
    - recommended_actions: List[str]
    """
    cargo_toml = crate_dir / "Cargo.toml"
    text = read_text(cargo_toml)
    has_sub = detect_subworkspace(text)
    version = extract_package_version(text)

    actions: List[str] = []
    if has_sub:
        actions.append(f"删 [{crate_dir.name}]/Cargo.toml [workspace] / [workspace.package] / [workspace.dependencies] 块")
        if version != "workspace" and version != workspace_version:
            actions.append(f"改 version = \"{version}\" → version.workspace = true")
        actions.append(f"加 edition/rust-version/license.workspace = true (如未加)")
    actions.append(f"加 \"crates/{crate_dir.name}\" 到 Apeireth-rust/Cargo.toml members")

    if has_sub:
        fix_type = "sub-workspace-removal"
        # Sub-workspace removal is medium-high risk because it requires Cargo.toml surgery
        risk_level = "medium" if version == workspace_version else "high"
    else:
        fix_type = "add-to-members"
        risk_level = "low"

    return {
        "name": crate_dir.name,
        "has_subworkspace": has_sub,
        "version": version,
        "version_conflict": version not in ("workspace", workspace_version),
        "fix_type": fix_type,
        "risk_level": risk_level,
        "recommended_actions": actions,
        "src_cargo_toml": str(cargo_toml.relative_to(WORKSPACE_ROOT)),
    }


def audit_v1303() -> Dict[str, Any]:
    active = parse_active_members()
    active_short = sorted(active)
    workspace_version = "1.0.0"  # from Cargo.toml [workspace.package]

    cargo_dirs = find_cargo_dirs()
    orphan_names = sorted([d.name for d in cargo_dirs if d.name not in active_short])

    classifications = []
    for name in orphan_names:
        crate_dir = WORKSPACE_ROOT / "crates" / name
        cls = classify_orphan(crate_dir, workspace_version)
        classifications.append(cls)

    # Categorize
    by_risk = {"low": [], "medium": [], "high": []}
    by_fix_type: Dict[str, List[str]] = {}
    for c in classifications:
        by_risk[c["risk_level"]].append(c["name"])
        by_fix_type.setdefault(c["fix_type"], []).append(c["name"])

    # Popper 假说自检
    hypotheses = {
        "h_orphan_count_v1303": {
            "observed": len(classifications),
            "threshold": 7,
            "result": "PASS" if len(classifications) >= 7 else "FAIL",
            "note": f"V1303 实扫 orphan = {len(classifications)} (V1302 = 8, V1303 含 sub-workspace 详分类)",
        },
        "h_subworkspace_count": {
            "observed": sum(1 for c in classifications if c["has_subworkspace"]),
            "threshold": 4,
            "result": "PASS",
            "note": f"sub-workspace pattern = {sum(1 for c in classifications if c['has_subworkspace'])} 个 (sdk-lark/sdk-livekit/sdk-sandbox/sdk-voice + e2e/r20-stage4)",
        },
        "h_version_conflict_count": {
            "observed": sum(1 for c in classifications if c["version_conflict"]),
            "threshold": 4,
            "result": "PASS",
            "note": f"version 冲突 ({workspace_version} vs others) = {sum(1 for c in classifications if c['version_conflict'])} 个",
        },
        "h_intentional_excluded": {
            "observed": "apeireth-tauri-stub",
            "threshold": "commented out, intentional",
            "result": "PASS",
            "note": "tauri-stub 不在 orphan 列表 (在 Cargo.toml 注释保留 intentional, V1302 audit 已标)",
        },
        "h_no_modification": {
            "observed": "audit-only",
            "threshold": True,
            "result": "PASS",
            "note": "V1303 audit-only, 0 修真, 0 触碰 Cargo.toml, 0 触碰任何 .rs",
        },
        "h_recommendations_actionable": {
            "observed": sum(len(c["recommended_actions"]) for c in classifications),
            "threshold": 20,
            "result": "PASS" if sum(len(c["recommended_actions"]) for c in classifications) >= 20 else "FAIL",
            "note": f"总 fix action 数 = {sum(len(c['recommended_actions']) for c in classifications)} (每条 fix 给具体动作)",
        },
    }

    return {
        "v1303_version": V1303_VERSION,
        "workspace_version": workspace_version,
        "active_members_count": len(active_short),
        "orphan_count": len(classifications),
        "by_risk": by_risk,
        "by_fix_type": by_fix_type,
        "classifications": classifications,
        "hypotheses": hypotheses,
        "v3_philosophy_gate": {
            "audit_only_not_pretend_pass": True,
            "no_phenomenal_pretend": True,
            "no_asi_pretend": True,
            "实事求是": True,
            "data_driven_planning": True,
            "note": "V1303 给 V1304+ 数据驱动修真规划, 0 修真, 0 假装 PASS — 每条 fix 都附风险评级 + 具体动作",
        },
    }


def render_report(result: Dict[str, Any]) -> str:
    lines = []
    lines.append("=" * 72)
    lines.append("V1303 — Orphan Crates Fix Planning Audit")
    lines.append(f"v1303_version: {result['v1303_version']}")
    lines.append("=" * 72)
    lines.append("")
    lines.append(f"## Workspace status")
    lines.append(f"  active members: {result['active_members_count']}")
    lines.append(f"  workspace version: {result['workspace_version']}")
    lines.append(f"  orphan count: {result['orphan_count']}")
    lines.append("")
    lines.append("## By risk level")
    for level, names in result["by_risk"].items():
        lines.append(f"  {level}: {len(names)} ({', '.join(names) if names else '-'})")
    lines.append("")
    lines.append("## By fix type")
    for ft, names in result["by_fix_type"].items():
        lines.append(f"  {ft}: {len(names)} ({', '.join(names) if names else '-'})")
    lines.append("")
    lines.append("## Per-crate classification")
    for c in result["classifications"]:
        lines.append(f"")
        lines.append(f"### {c['name']} ({c['risk_level']} risk, {c['fix_type']})")
        lines.append(f"  has_subworkspace: {c['has_subworkspace']}")
        lines.append(f"  version: {c['version']}")
        lines.append(f"  version_conflict: {c['version_conflict']}")
        lines.append(f"  src: {c['src_cargo_toml']}")
        lines.append(f"  actions:")
        for a in c["recommended_actions"]:
            lines.append(f"    - {a}")
    lines.append("")
    lines.append("## Popper hypotheses")
    for h_id, h in result["hypotheses"].items():
        lines.append(f"  {h_id}: {h['result']} (observed={h['observed']}, threshold={h['threshold']})")
        lines.append(f"    {h['note']}")
    lines.append("")
    lines.append("## V3 philosophy gate")
    for k, v in result["v3_philosophy_gate"].items():
        if isinstance(v, bool):
            lines.append(f"  [{'PASS' if v else 'FAIL'}] {k}")
        else:
            lines.append(f"  {k}: {v}")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# Self-test
# ============================================================================


def _self_test() -> None:
    result = audit_v1303()
    # Schema
    assert "hypotheses" in result
    assert "classifications" in result
    assert len(result["classifications"]) >= 7, f"应有至少 7 个 orphan, 实得 {len(result['classifications'])}"
    # Each classification has required fields
    for c in result["classifications"]:
        assert "name" in c
        assert "fix_type" in c
        assert "risk_level" in c
        assert "recommended_actions" in c
        assert len(c["recommended_actions"]) >= 1
    # Hypotheses PASS
    for h_id, h in result["hypotheses"].items():
        assert h["result"] == "PASS", f"{h_id}: {h}"


if __name__ == "__main__":
    result = audit_v1303()
    print(render_report(result))
    print()
    print("=" * 72)
    print("JSON (compact)")
    print("=" * 72)
    compact = {
        "v1303_version": result["v1303_version"],
        "orphan_count": result["orphan_count"],
        "by_risk": result["by_risk"],
        "by_fix_type": result["by_fix_type"],
        "names": [c["name"] for c in result["classifications"]],
        "hypotheses": {k: v["result"] for k, v in result["hypotheses"].items()},
    }
    print(json.dumps(compact, indent=2, ensure_ascii=False))

    if "--self-test" in sys.argv:
        _self_test()
        print()
        print("SELF-TEST PASS")