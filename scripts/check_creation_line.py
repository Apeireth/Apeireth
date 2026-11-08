#!/usr/bin/env python3
"""**能力创造线进度页守卫** —— 让「做到哪了」与「有没有重复」**同时**不可能漂移。

# 为什么有两个判据（2026-11-08 重复审计的产物）

重复审计实测：**23 族「按名 0 命中」，但真新面 = 0**（每族都有语义先例）⇒
⇒ **「符号 0 命中」只能证明「没造过同名物」，不能证明「没人做过这件事」**。

所以本守卫同时判两件事：

1. **落地判据（探针）**：族标「✅ 已落地」⇒ 探针必须已从基线长出来；标「❌ 未开工」⇒ 探针必须仍是基线。
   ⇒ 防谎报（页面与代码不一致）。
2. **重复判据（先例锚点）**：每族登记**不得重建的已有面**（file + 锚点字样）⇒
   守卫**核验这些锚点仍在**。先例被删 ⇒ **该族的重复判定作废** ⇒ 必须**重判**，不许沿用旧判定。
   ⇒ 防「重复面被忘掉，等于把已有能力再造一遍」。

# 用法

    python3 scripts/check_creation_line.py --check     # 门禁（默认）
    python3 scripts/check_creation_line.py --report    # 只打印实测值，不判成败

# 退出码

    0 = 页面状态与代码事实一致，且各族先例锚点仍在
    1 = 不一致（逐条打印差在哪 + 怎么修）
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs" / "status" / "capability-creation-line-progress.md"

STATUS_TODO = "未开工"
STATUS_WIP = "在途"
STATUS_DONE = "已落地"
STATUS_PENDING = "未排期"

#: 判定词表（进度页「重复判定」列必须落这几个词之一）。
VERDICTS = ("部分重复", "待立项", "真新面")

#: 族定义。
#: (族 id, 探针正则, 扫描范围, 基线命中数, 先例锚点[(文件, 锚点字样), ...])
#: ⚠️ 基线 = 2026-11-08 实测值；先例锚点 = 2026-11-08 重复审计认定的**不得重建**的既有面。
FACTS: list[tuple[str, str, str, int, list[tuple[str, str]]]] = [
    (
        "C-A1",
        r"native_control_tree|external_gui|external_control_tree",
        "crates",
        0,
        [
            ("crates/capabilities/tools/src/fetch/accessibility.rs", "AccessibilityTree"),
            ("crates/adapters/gateway/src/im_bridge.rs", "action"),
        ],
    ),
    (
        "C-B1",
        r"process_tree_snapshot|snapshot_process_tree",
        "crates",
        0,
        [
            ("crates/capabilities/tools/src/process/mod.rs", "ProcessTreeContainment"),
            ("crates/capabilities/tools/src/std_sub_supervisor.rs", "kill"),
        ],
    ),
    (
        "C-B2",
        r"post_with_headers|post_json|post_form|fn post\b",
        "crates/capabilities/tools/src/egress.rs",
        0,
        [
            ("crates/capabilities/tools/src/egress.rs", "get_with_headers"),
            ("crates/adapters/sdk/src/client.rs", "post"),
            ("crates/capabilities/tools/src/mcp_bridge/transport.rs", "fn post"),
        ],
    ),
    (
        "C-B3",
        r"script_exec",
        "crates",
        0,
        [("crates/capabilities/tools/src/shell.rs", "Shell")],
    ),
    (
        "C-B4",
        r"web_browser|BrowserSession",
        "crates",
        0,
        [
            ("crates/capabilities/tools/src/stealth_crawler.rs", "StealthCrawler"),
            ("crates/capabilities/tools/src/fetch.rs", "FetchTool"),
        ],
    ),
    (
        "C-D1",
        r"object_grant|MemoryGrant",
        "crates",
        0,
        [
            ("crates/engine/memory/src/scope.rs", "MemoryScope"),
            ("crates/foundation/governance/src/permission.rs", "PermissionPolicy"),
        ],
    ),
    (
        "C-D2",
        r"session_observability|SessionObservability",
        "crates",
        0,
        [
            ("crates/engine/memory/src/agent_trace.rs", "TraceSpan"),
            ("crates/foundation/orchestration/src/durable/replay.rs", "recover_attempt"),
            ("crates/engine/runtime/src/canonical/session_events.rs", "marker"),
        ],
    ),
    (
        "C-D3",
        r"cross_device|CrossDevice|sync_cursor",
        "crates",
        0,
        [("crates/engine/memory/src/research_roaming_memory.rs", "tombstone")],
    ),
    (
        "C-E1",
        r"skill_package|SkillPackage|verify_package_signature",
        "crates",
        0,
        [
            ("crates/capabilities/tools/src/skill/library.rs", "SkillLibrary"),
            ("crates/foundation/plugin/src/manifest.rs", "PluginManifest"),
        ],
    ),
    (
        "C-F1",
        r"Ed25519Verifier|verify_multisig_ed25519|MultiSigThreshold",
        "crates",
        0,
        [
            ("crates/foundation/core/src/onion.rs", "verify_multisig"),
            ("crates/foundation/governance/src/chain.rs", "PlaceholderMultiSignVerifier"),
        ],
    ),
    (
        "C-G2",
        r"replay_eval|ReplayEval",
        "crates",
        0,
        [
            ("crates/foundation/governance/src/eval.rs", "EvalScore"),
            ("crates/engine/memory/src/d4_eval.rs", "fn score"),
        ],
    ),
    (
        "C-H1",
        r"routing_policy|RoutingPolicy",
        "crates/foundation/core/src",
        0,
        [
            ("crates/foundation/core/src/cost_router.rs", "CostRouter"),
            ("crates/engine/runtime/src/canonical/provider.rs", "ProviderRouter"),
        ],
    ),
    (
        "C-H2",
        r"depth_quota|DepthQuota",
        "crates/engine/runtime/src",
        0,
        [
            ("crates/engine/runtime/src/canonical/quota.rs", "QuotaDimension"),
            ("crates/capabilities/tools/src/self_status.rs", "depth"),
        ],
    ),
    (
        "C-H4",
        r"tool\.web_search|WebSearchTool|web_search_tool",
        "crates",
        0,
        [
            ("crates/adapters/sdk/src/client.rs", "web_search"),
            ("crates/capabilities/tools/src/search.rs", "SearchTool"),
        ],
    ),
    (
        "C-H5",
        r"tool\.calendar|CalendarStore|CalendarEvent",
        "crates",
        0,
        [("crates/engine/organ/src/care/tool.rs", "CARE_SCHEDULE_CAPABILITY_ID")],
    ),
    (
        "C-L1",
        r"local_inference|LocalInferenceProvider",
        "crates",
        0,
        [("crates/engine/runtime/src/canonical/provider.rs", "ProviderRouter")],
    ),
]

#: 进度页族级看板的列序：族 | 能力 id/面名 | 重复判定 | 已有先例 | 波次 | 状态 | 治理档 | 还差什么
ROW_RE = re.compile(r"^\|\s*(C-[A-Z][0-9′']*)\s*\|")


def count_matches(pattern: str, scope: str) -> int:
    """数探针命中数（优先 rg；rg 不可用时退回纯 Python 扫描）。"""
    cmd = ["rg", "--no-heading", "-o", pattern, scope, "--glob", "*.rs"]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        return _count_python(pattern, scope)
    if proc.returncode not in (0, 1):  # 1 = 无命中（rg 语义）
        return _count_python(pattern, scope)
    return len([ln for ln in proc.stdout.splitlines() if ln.strip()])


def _count_python(pattern: str, scope: str) -> int:
    rx = re.compile(pattern)
    root = ROOT / scope
    total = 0
    for path in root.rglob("*.rs"):
        try:
            total += len(rx.findall(path.read_text(encoding="utf-8", errors="ignore")))
        except OSError:
            continue
    return total


def parse_rows() -> dict[str, tuple[str, str]]:
    """从进度页解析族行 ⇒ (状态词, 重复判定词)。"""
    if not PAGE.exists():
        return {}
    rows: dict[str, tuple[str, str]] = {}
    for line in PAGE.read_text(encoding="utf-8").splitlines():
        if not ROW_RE.match(line):
            continue
        parts = [p.strip() for p in line.strip().strip("|").split("|")]
        if len(parts) < 6:
            continue
        fam = parts[0]
        if fam in rows:
            rows[fam] = ("DUPLICATE", "")
        else:
            rows[fam] = (parts[5], parts[2])
    return rows


def prior_alive(rel: str, needle: str) -> bool:
    """先例锚点是否仍在（文件存在且含锚点字样）。"""
    path = ROOT / rel
    if not path.is_file():
        return False
    try:
        return needle in path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return False


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="判成败（默认）")
    ap.add_argument("--report", action="store_true", help="只打印实测值")
    args = ap.parse_args()

    rows = parse_rows()
    problems: list[str] = []

    print(f"[creation-line] 族数 = {len(FACTS)}（进度页解析到 {len(rows)} 行）")
    print(f"{'族':<8} {'探针命中':>8} {'基线':>5} {'先例':>6}  状态 / 判定")
    for fam, pattern, scope, baseline, priors in FACTS:
        hits = count_matches(pattern, scope)
        state, verdict_word = rows.get(fam, ("（页面缺行）", ""))
        alive = sum(1 for rel, needle in priors if prior_alive(rel, needle))
        prior_mark = f"{alive}/{len(priors)}"
        note = ""

        if fam not in rows:
            note = "❌ 族行缺失 ⇒ 补一行（不许删行降难度）"
            problems.append(f"{fam}: 进度页缺族行")
        elif state == "DUPLICATE":
            note = "❌ 族行重复 ⇒ 页面只能有一行"
            problems.append(f"{fam}: 进度页族行重复")
        else:
            if not any(v in verdict_word for v in VERDICTS):
                note = f"❌ 重复判定词不在词表（须含 {'/'.join(VERDICTS)}）"
                problems.append(f"{fam}: 重复判定词非法 = {verdict_word!r}")
            if STATUS_DONE in state:
                if hits > baseline:
                    note = "✅ 已落地（探针已长）"
                else:
                    note = f"❌ 谎报：标已落地但探针仍 {hits}（基线 {baseline}）"
                    problems.append(f"{fam}: 标「已落地」但探针命中 {hits}（基线 {baseline}）")
            elif STATUS_TODO in state:
                if hits == baseline:
                    note = "❌ 未开工（与基线一致）"
                else:
                    note = f"❌ 已被他线实现（命中 {hits} ≠ 基线 {baseline}）⇒ 认领或改判"
                    problems.append(f"{fam}: 标「未开工」但探针命中 {hits}（基线 {baseline}）")
            elif STATUS_PENDING in state:
                if hits == baseline:
                    note = "⛔ 未排期（待 owner 拍板；与基线一致）"
                else:
                    note = f"❌ 未排期却已被他线实现（命中 {hits} ≠ 基线 {baseline}）⇒ 认领或改判"
                    problems.append(f"{fam}: 标「未排期」但探针命中 {hits}（基线 {baseline}）")
            elif STATUS_WIP in state:
                note = "🟡 在途（不判成败）"
            else:
                note = f"❌ 状态词不在词表（须含 {STATUS_TODO}/{STATUS_WIP}/{STATUS_DONE}）"
                problems.append(f"{fam}: 状态词非法 = {state!r}")

            if alive != len(priors):
                missing = [rel for rel, needle in priors if not prior_alive(rel, needle)]
                note += f" ｜❌ 先例锚点消失：{', '.join(missing)}"
                problems.append(
                    f"{fam}: 先例锚点消失（{', '.join(missing)}）"
                    f" ⇒ 该族的重复判定**作废** ⇒ 必须**重判**（不许沿用旧判定）"
                )

        print(f"{fam:<8} {hits:>8} {baseline:>5} {prior_mark:>6}  {state} ⇒ {note}")

    known = {fam for fam, *_ in FACTS}
    for fam in rows:
        if fam not in known:
            problems.append(f"{fam}: 进度页有本脚本不知道的族行 ⇒ 补进 FACTS 或删行")
            print(f"{fam:<8} {'-':>8} {'-':>5} {'-':>6}  {rows[fam][0]} ⇒ ❌ 未知族")

    if args.report:
        print(f"\n[creation-line] 问题数 = {len(problems)}（--report 不判成败）")
        return 0

    if problems:
        print("\n[creation-line] ❌ FAIL")
        for p in problems:
            print(f"  - {p}")
        print("\n修法：改实现（让探针长出来）/ 改状态词（认领·改判）/ 重判重复性（先例变了）——**不许改基线**。")
        return 1

    print("\n[creation-line] ✅ PASS（页面状态与代码事实一致，且各族先例锚点仍在）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
