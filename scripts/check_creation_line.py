#!/usr/bin/env python3
"""**能力创造线进度页守卫** —— 让「做到哪了」不可能手写漂移。

# 为什么要有它（owner 2026-11-08 裁定「只拓新面」的必然要求）

本线 23 个新面**全部以「仓内 0 命中」为存在性判据** ⇒ ⇒ 「这族到底做没做」
**不是散文能回答的问题**，是**一次 grep 就能判定的问题** ⇒ ⇒ 于是：

* 进度页写「✅ 已落地」 ⇒ 该族的 0 命中探针**必须已经 >0**（否则 = 谎报）；
* 进度页写「❌ 未开工」 ⇒ 该族的探针**必须仍是基线值**（否则 = 已被他线实现，
  本线要么认领、要么改判，**不许装作没发生**）；
* 族行缺失 / 状态词不在词表 ⇒ **失败**（不许悄悄删行降难度）。

⇒ ⇒ **没有守卫的进度页 = 散文**（施工纪律 §三.3「无脚本不发布数字」的同一条纪律）。

# 用法

    python3 scripts/check_creation_line.py --check     # 门禁（默认）
    python3 scripts/check_creation_line.py --report    # 只打印实测值，不判成败

# 退出码

    0 = 页面与代码一致
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

#: 状态词表（进度页必须用这几个词之一，否则失败）。
STATUS_TODO = "未开工"
STATUS_WIP = "在途"
STATUS_DONE = "已落地"

#: 23 族的存在性探针。
#: (族 id, 一句话, 正则, 扫描范围, 基线命中数)
#: ⚠️ 基线 = 2026-11-08 实测值（落库当时的真实命中数）；「未开工」态必须仍等于基线。
FACTS: list[tuple[str, str, str, str, int]] = [
    ("C-A1", "只读 UI 控件树", r"ui_automation|UiAutomation|ControlTree|element_tree", "crates", 0),
    ("C-A2", "GUI 动作层", r"ClickAction|mouse_click|GuiAction|gui_action", "crates", 0),
    ("C-B1", "进程树快照与取消", r"process_tree_snapshot|snapshot_process_tree|ProcessTreeSnapshot", "crates", 0),
    ("C-B2", "带鉴权 HTTP 写面", r"Method::POST|Method::PUT", "crates", 0),
    ("C-B3", "沙箱代码执行面", r"CodeInterpreter|python_exec|exec_python|sandbox_exec_python", "crates", 0),
    ("C-B4", "浏览器自动化面", r"tool\.browser|BrowserAutomation|browser_automation|BrowserSession", "crates", 0),
    ("C-C1", "网关 WS 全双工 + SDK 流式", r"/v1/ws", "crates/adapters/gateway/src", 0),
    ("C-D1", "记忆对象级授权", r"MemoryGrant|object_grant|episode_grant", "crates", 0),
    ("C-D2", "检查点恢复", r"resume_from_checkpoint|ContextCheckpoint|restore_checkpoint", "crates", 0),
    ("C-D3", "跨设备漫游同步", r"cross_device|CrossDevice|sync_cursor", "crates", 0),
    ("C-E1", "技能包格式与安装", r"SkillManifest|skill_manifest|skill_package|SkillPackage", "crates", 0),
    ("C-E2", "包/插件签名验签", r"Ed25519Signature|ed25519_dalek|Ed25519Verifier", "crates", 0),
    ("C-F1", "Ed25519 真多签", r"MultiSigKeyring|MultiSignKeyset|verify_multisig_ed25519|MultiSigThreshold", "crates", 0),
    ("C-F2", "R4 反思期生效路径", r"reflection_period|ReflectionPeriod|cooling_off", "crates", 0),
    ("C-F3", "供应链验签", r"supply_chain|SupplyChain|artifact_signature", "crates", 0),
    ("C-G1", "OTA 灰度回滚", r"ota_update|OtaUpdate|gradual_rollout|canary_release", "crates", 0),
    ("C-G2", "会话回放评估器", r"ReplayScorer|replay_evaluator|ReplayEvaluation", "crates", 0),
    ("C-G3", "tracing 时间线", r"tracing_timeline|TimelineSpan|timeline_span", "crates", 0),
    ("C-G4", "会话层崩溃恢复", r"session_crash_recovery|SessionRecovery|session_recovery", "crates", 0),
    ("C-H1", "模型路由策略层", r"routing_policy|RoutingPolicy", "crates", 0),
    ("C-H2", "配额深度维", r"DepthQuota|depth_quota", "crates", 0),
    ("C-H3", "MCP 协议合规三件套", r"nextCursor|progressToken|Mcp-Session-Id", "crates/capabilities/tools/src/mcp_bridge", 0),
    ("C-H4", "互联网检索面", r"tool\.web_search|WebSearchTool|web_search_tool", "crates", 0),
]

ROW_RE = re.compile(r"^\|\s*(C-[A-Z]\d)\s*\|")


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


def parse_rows() -> dict[str, str]:
    """从进度页解析族行 ⇒ 状态词。"""
    if not PAGE.exists():
        return {}
    rows: dict[str, str] = {}
    for line in PAGE.read_text(encoding="utf-8").splitlines():
        if not ROW_RE.match(line):
            continue
        parts = [p.strip() for p in line.strip().strip("|").split("|")]
        if len(parts) < 4:
            continue
        fam = parts[0]
        if fam in rows:
            rows[fam] = "DUPLICATE"
        else:
            rows[fam] = parts[3]
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="判成败（默认）")
    ap.add_argument("--report", action="store_true", help="只打印实测值")
    args = ap.parse_args()

    rows = parse_rows()
    problems: list[str] = []

    print(f"[creation-line] 族数 = {len(FACTS)}（进度页解析到 {len(rows)} 行）")
    print(f"{'族':<7} {'实测命中':>8} {'基线':>5}  状态 / 判定")
    for fam, label, pattern, scope, baseline in FACTS:
        hits = count_matches(pattern, scope)
        state = rows.get(fam, "（页面缺行）")
        verdict = ""
        if fam not in rows:
            verdict = "❌ 族行缺失 ⇒ 补一行（不许删行降难度）"
            problems.append(f"{fam}: 进度页缺族行（{label}）")
        elif state == "DUPLICATE":
            verdict = "❌ 族行重复 ⇒ 页面只能有一行"
            problems.append(f"{fam}: 进度页族行重复")
        elif STATUS_DONE in state:
            if hits > baseline:
                verdict = "✅ 已落地（探针已长）"
            else:
                verdict = f"❌ 谎报：标已落地但探针仍 {hits}（基线 {baseline}）"
                problems.append(
                    f"{fam}: 标「{STATUS_DONE}」但探针命中 {hits}（基线 {baseline}）"
                    f" ⇒ 要么回填实现，要么改状态词（改探针须同时改设计稿与本脚本 FACTS）"
                )
        elif STATUS_TODO in state:
            if hits == baseline:
                verdict = "❌ 未开工（与基线一致）"
            else:
                verdict = f"❌ 已被他线实现（命中 {hits} ≠ 基线 {baseline}）⇒ 认领或改判"
                problems.append(
                    f"{fam}: 标「{STATUS_TODO}」但探针命中 {hits}（基线 {baseline}）"
                    f" ⇒ 该面已被他线实现 ⇒ 本线必须认领（改状态词）或改判（不在本线范围）"
                )
        elif STATUS_WIP in state:
            verdict = "🟡 在途（不判成败）"
        else:
            verdict = f"❌ 状态词不在词表（须含 {STATUS_TODO}/{STATUS_WIP}/{STATUS_DONE}）"
            problems.append(f"{fam}: 状态词非法 = {state!r}")
        print(f"{fam:<7} {hits:>8} {baseline:>5}  {state} ⇒ {verdict}")

    known = {fam for fam, *_ in FACTS}
    for fam in rows:
        if fam not in known:
            problems.append(f"{fam}: 进度页有本脚本不知道的族行 ⇒ 补进 FACTS 或删行")
            print(f"{fam:<7} {'-':>8} {'-':>5}  {rows[fam]} ⇒ ❌ 未知族")

    if args.report:
        print(f"\n[creation-line] 问题数 = {len(problems)}（--report 不判成败）")
        return 0

    if problems:
        print("\n[creation-line] ❌ FAIL")
        for p in problems:
            print(f"  - {p}")
        print("\n修法：改实现（让探针长出来）或改状态词（认领 / 改判）——**不许改基线**。")
        return 1

    print("\n[creation-line] ✅ PASS（页面状态与代码事实一致）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
