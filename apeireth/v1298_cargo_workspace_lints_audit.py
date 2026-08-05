"""V1298 — Cargo Workspace Lints Audit (VCP 真源代码深读 #19) 真生产模块.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 21:33 +08:00 2026-08-05)
> **承接**: V1297 Cargo.toml feature flag (d4f4dd4f, tests 44/44 pass)
>          + V1296 Cargo.toml metadata (a9bca808)
>          + V1295 Cargo.lock (d07cce57)
>          + V1294 build.rs + V1293 dep graph + V1292 test coverage + V1291 build artifact
>          + V1290 doc section depth + V1289 doc coverage + V1288 governance + V1287 unsafe
>          + V1286 security priority + V1285 all-42 security + V1284 worst-5 security
>          + V1283 multi-crate semantic + V1282 sovereignty semantic + V1281 asi semantic.
> **触发**: 21:34 cron wake tick (autonomy-v3) — V1297 tests commit (d5b98489, 21:31) 已 360/360 pass.
>          V1297 报告建议下一步 V1298 = workspace.lints 维度审计 (已在 V1297_REPORT 标出).
> **真借鉴**: 主 19:33 走在前人肩上 + cargo book "Workspace Lints" 章节
>          + wasmtime/Cargo.toml [workspace.lints] (R19 阶段 0 已抄)
>          + qdrant/Cargo.toml [workspace.lints.clippy] (R19 阶段 0 已抄)
>          + R20 阶段 6 修复: unused_async 是 clippy lint, 非 rustc lint, 不放 [workspace.lints.rust]
>          + R20 阶段 1/2: 56 workspace members (post R20 stage 2)
>          + 2 crates (apeireth-team-lead + example_plugin) 缺 [lints] workspace = true.

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1293-V1297 已审 Cargo.toml:
- V1293: dep graph
- V1294: build.rs
- V1295: Cargo.lock
- V1296: [package] metadata
- V1297: [features] 维度

但 **`[workspace.lints]` 全局 lint 治理** 是另一关键维度:

- **rustc lints vs clippy lints 错放风险**: rustc 不认识 clippy 的 lint (e.g. unused_async), 错放报 E0602
- **子 crate [lints] workspace = true 继承**: 不继承 = 子 crate 行为不一致
- **lint policy 性价**: 数量合理 (≥30 借鉴 wasmtime, 但 ≤80 不暴增)
- **意外 cfg 防护**: `[workspace.lints.rust.unexpected_cfgs]` 含 `check-cfg` 列表, 防止 unknown cfg warning

**V1298 = 真生产全 Cargo.toml workspace.lints 审计**, 6 维度 6 假说:

1. **h_rust_lints_present**: [workspace.lints.rust] 段存在 + lint 定义 ≥ 5
2. **h_clippy_lints_present**: [workspace.lints.clippy] 段存在 + lint 定义 ≥ 10
3. **h_rust_vs_clippy_separation**: rustc lints (e.g. unused_extern_crates) 不在 clippy 段, clippy lints (e.g. unused_async) 不在 rust 段
4. **h_unexpected_cfgs_present**: [workspace.lints.rust.unexpected_cfgs] 子段 + check-cfg 列表 (kani/fuzzing 等)
5. **h_lints_inherit_pct**: workspace members 中 [lints] workspace = true 占比 >= 95.0%
6. **h_no_deny_in_workspace_lints**: workspace.lints 不含 `deny = '...'` 全局拒绝 (allow + warn 是 sane baseline)

每假说维度:
- **RustLintEntry**: name / level
- **ClippyLintEntry**: name / level
- **UnexpectedCfgEntry**: cfg_expr / reason
- **CrateLintsInherit**: name / inherits / has_lints_section
- **LintLedger**: 全 workspace lints 审计结果

**关键免责声明** (主 17:58 + 主 20:46):
- "lints audit" ≠ "lints 安全": 仅静态解析 Cargo.toml, 不调 cargo / clippy
- PASS ≠ cargo clippy 0 warning: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露, 列出每条 finding 不掩饰
- 不解析 nested table (e.g. [lints.rust.unexpected_cfgs]) 之外的复杂结构
- 不调 cargo, 纯 read-only
- 只 audit 不修改 (audit ≠ fix)
"""
from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# 1. Constants (主 00:56 任何人都能接手)
# ============================================================================

CARGO_TOML = "Cargo.toml"

WORKSPACE_ROOT_DEFAULT = Path(__file__).resolve().parents[1] / "Apeireth-rust"

# Workspace members (V1298 报 56, post R20 阶段 1/2)
# 涵盖 5 P0 crate skeleton + 9 post-R20 stage 2 + 47 历史 = 56
WORKSPACE_MEMBERS_V1298: List[str] = [
    "apeireth-core", "apeireth-memory", "apeireth-asi", "apeireth-tools",
    "apeireth-cli", "apeireth-bench", "apeireth-cognition", "apeireth-action",
    "apeireth-life-force", "apeireth-constraint", "apeireth-central",
    "apeireth-value", "apeireth-consciousness", "apeireth-relation",
    "apeireth-motivation", "apeireth-perception", "apeireth-upgrade",
    "apeireth-onion", "apeireth-council", "apeireth-sovereignty",
    "apeireth-supervisor", "apeireth-pybridge", "apeireth-verify",
    "apeireth-extension", "apeireth-evolution", "apeireth-bus",
    "apeireth-api", "apeireth-web", "apeireth-tui", "apeireth-protocol",
    "apeireth-http-client", "apeireth-pipeline", "apeireth-tool-registry",
    "apeireth-tool-runtime", "apeireth-tool-approval", "apeireth-agent",
    "apeireth-mcp", "apeireth-graph", "apeireth-vector", "apeireth-sdk",
    "apeireth-formal",
    # 5 P0 + post-R20 stage 2
    "apeireth-mcp-ssh", "apeireth-mcp-winrm", "apeireth-mcp-relay-image",
    "apeireth-keyring", "apeireth-lark", "apeireth-machine-id",
    "apeireth-repo-analyzer", "apeireth-repo-scan", "apeireth-voice",
    "apeireth-team-lead", "example_plugin",
    # V1297 报告里补充 (post-R20 stage 1 P0 + 9 skeleton net new)
    "apeireth-plugin", "apeireth-tauri-stub", "apeireth-rollback",
    "apeireth-image-prompt", "apeireth-workflow",
    "apeireth-template", "apeireth-schema", "apeireth-evolve",
    "apeireth-mcp-server", "apeireth-mcp-client", "apeireth-tree-sitter",
]

# Risk thresholds (主 17:43 实事求是)
THRESHOLD_RUST_LINTS_MIN = 5
THRESHOLD_CLIPPY_LINTS_MIN = 10
THRESHOLD_TOTAL_LINTS_MIN = 30
THRESHOLD_LINTS_INHERIT_PCT_MIN = 95.0

# Clippy-specific lint names (误放 rust 段会触发 E0602)
# 来自 R20 阶段 6 实战经验: 未列全, 只列已知的"危险"案例
# 实际应用 regex 检测任意 clippy lint 在 rust 段出现
CLIPPY_LINT_NAMES_SAMPLE: List[str] = [
    "unused_async", "needless_pass_by_value", "missing_docs_in_private_items",
    "match_wildcard_for_single_variants", "uninlined_format_args",
    "wildcard_enum_match_arm", "needless_raw_string_hashes", "unused_self",
    "used_underscore_binding", "unnecessary_wraps", "clone_on_copy", "map_clone",
]

# Rustc lints (误放 clippy 段是 no-op, 但常见的不一致需检查)
RUSTC_LINT_NAMES_SAMPLE: List[str] = [
    "unused_extern_crates", "unused_import_braces", "unused_lifetimes",
    "unused_macro_rules", "missing_docs", "unused_imports", "dead_code",
    "unused_must_use", "unused_mut", "trivial_numeric_casts",
    "unstable_features", "nonstandard_style",
]

# Regex patterns (主 17:43 实事求是 - regex-only parser, 已知限制)
RE_SECTION = re.compile(r"^\[([a-zA-Z0-9_.\-]+)\]\s*$")
RE_LINT_RUST = re.compile(r"^\s*([a-z_][a-z_0-9]*)\s*=\s*'(allow|warn|deny|forbid)'\s*$")
RE_LINT_CLIPPY = re.compile(r"^\s*([a-z_][a-z_0-9]*)\s*=\s*'(allow|warn|deny|forbid)'\s*$")
RE_CRATE_LINTS = re.compile(r"(?m)^\[lints(\.workspace)?\]")
RE_CRATE_LINTS_VALUE = re.compile(r"(?m)^workspace\s*=\s*true")
RE_UNEXPECTED_CFGS_LEVEL = re.compile(r'^\s*level\s*=\s*"([^"]+)"\s*$')
RE_UNEXPECTED_CFGS_CHECK_CFG_ITEM = re.compile(r"^\s*'([^']+)'(?:,)?(?:\s*#.*)?\s*$")

V1298_VERSION = "0.1.0"


# ============================================================================
# 2. Dataclasses (主 17:43 实事求是, frozen=True 不变)
# ============================================================================

@dataclasses.dataclass(frozen=True)
class RustLintEntry:
    """[workspace.lints.rust] 单条 lint 定义."""
    name: str
    level: str  # 'allow' / 'warn' / 'deny' / 'forbid'
    line_number: int = 0


@dataclasses.dataclass(frozen=True)
class ClippyLintEntry:
    """[workspace.lints.clippy] 单条 lint 定义."""
    name: str
    level: str
    line_number: int = 0


@dataclasses.dataclass(frozen=True)
class UnexpectedCfgEntry:
    """[workspace.lints.rust.unexpected_cfgs] 子段配置."""
    level: str
    check_cfg: Tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class CrateLintsInherit:
    """子 crate [lints] workspace = true 继承情况."""
    crate_name: str
    has_lints_section: bool
    inherits_workspace: bool


@dataclasses.dataclass(frozen=True)
class LintLedger:
    """整 workspace lints 审计结果 ledger."""
    rust_lints: Tuple[RustLintEntry, ...]
    clippy_lints: Tuple[ClippyLintEntry, ...]
    unexpected_cfgs: Optional[UnexpectedCfgEntry]
    crate_lints_inherit: Tuple[CrateLintsInherit, ...]
    has_rust_section: bool
    has_clippy_section: bool
    rust_lint_count: int
    clippy_lint_count: int
    total_lint_count: int
    crates_total: int
    crates_inherit_count: int
    crates_inherit_pct: float
    duration_ms: int


@dataclasses.dataclass(frozen=True)
class HypothesisResult:
    """单个假说验证结果 (主 13:08 真自问, 可证伪)."""
    hypothesis_id: str
    description: str
    passed: bool
    observed: float
    threshold: float
    details: str


# ============================================================================
# 3. Parse helpers
# ============================================================================

def _safe_read(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


def _parse_workspace_lints_lines(text: str) -> Tuple[
    List[RustLintEntry], List[ClippyLintEntry], Optional[UnexpectedCfgEntry]
]:
    """解析 [workspace.lints.rust] + [workspace.lints.clippy] + nested unexpected_cfgs."""
    rust: List[RustLintEntry] = []
    clippy: List[ClippyLintEntry] = []
    unexpected: Optional[UnexpectedCfgEntry] = None

    lines = text.splitlines()
    section: Optional[str] = None  # None / "rust" / "clippy" / "unexpected_cfgs"
    nested_level: Optional[str] = None  # for unexpected_cfgs

    check_cfg_buf: List[str] = []
    unexp_level: Optional[str] = None

    def _finalize_nested():
        nonlocal unexpected
        if section == "nested" and unexp_level:
            unexpected = UnexpectedCfgEntry(
                level=unexp_level, check_cfg=tuple(check_cfg_buf)
            )

    for idx, line in enumerate(lines, start=1):
        stripped = line.strip()
        # Section header
        if stripped.startswith("[") and stripped.endswith("]"):
            inner = stripped[1:-1]
            if inner == "workspace.lints.rust":
                _finalize_nested()
                section = "rust"
                nested_level = None
                continue
            elif inner == "workspace.lints.clippy":
                _finalize_nested()
                section = "clippy"
                nested_level = None
                continue
            elif inner == "workspace.lints.rust.unexpected_cfgs":
                _finalize_nested()
                section = "nested"
                nested_level = "unexpected_cfgs"
                unexp_level = None
                check_cfg_buf = []
                continue
            else:
                # Different section ends
                if section in ("rust", "clippy", "nested"):
                    _finalize_nested()
                    section = None
                    nested_level = None
                continue

        if section == "rust":
            m = RE_LINT_RUST.match(line)
            if m and not line.lstrip().startswith("#"):
                rust.append(RustLintEntry(
                    name=m.group(1), level=m.group(2), line_number=idx,
                ))

        elif section == "clippy":
            m = RE_LINT_CLIPPY.match(line)
            if m and not line.lstrip().startswith("#"):
                clippy.append(ClippyLintEntry(
                    name=m.group(1), level=m.group(2), line_number=idx,
                ))

        elif section == "nested" and nested_level == "unexpected_cfgs":
            m_lvl = RE_UNEXPECTED_CFGS_LEVEL.match(line)
            if m_lvl:
                unexp_level = m_lvl.group(1)
                continue
            m_cfg = RE_UNEXPECTED_CFGS_CHECK_CFG_ITEM.match(line)
            if m_cfg:
                # Strip optional trailing comma
                expr = m_cfg.group(1).rstrip(",").strip()
                if expr:
                    check_cfg_buf.append(expr)

    # Finalize if file ended within a nested section
    if section == "nested" and unexp_level:
        unexpected = UnexpectedCfgEntry(
            level=unexp_level, check_cfg=tuple(check_cfg_buf)
        )

    return rust, clippy, unexpected


def _check_crate_lints_inherit(
    workspace_root: Path, members: List[str]
) -> List[CrateLintsInherit]:
    """对每个 workspace member, 读 Cargo.toml 检查 [lints] workspace = true."""
    results: List[CrateLintsInherit] = []
    for crate_name in members:
        cargo_path = workspace_root / "crates" / crate_name / CARGO_TOML
        text = _safe_read(cargo_path)
        if not text:
            results.append(CrateLintsInherit(
                crate_name=crate_name,
                has_lints_section=False,
                inherits_workspace=False,
            ))
            continue
        # Patterns use inline (?m) flag (Pattern.search signature has no flags param)
        has_lints = bool(RE_CRATE_LINTS.search(text))
        inherits = bool(RE_CRATE_LINTS_VALUE.search(text))
        results.append(CrateLintsInherit(
            crate_name=crate_name,
            has_lints_section=has_lints,
            inherits_workspace=inherits,
        ))
    return results


def sweep_workspace(workspace_root: Path) -> LintLedger:
    """扫 workspace.lints 维度, 返回 LintLedger."""
    start = time.time()
    workspace_cargo = workspace_root / CARGO_TOML
    text = _safe_read(workspace_cargo)

    has_rust = False
    has_clippy = False
    if text:
        for line in text.splitlines():
            stripped = line.strip()
            if stripped == "[workspace.lints.rust]":
                has_rust = True
            elif stripped == "[workspace.lints.clippy]":
                has_clippy = True

    rust_lints, clippy_lints, unexpected = _parse_workspace_lints_lines(text)
    crate_results = _check_crate_lints_inherit(workspace_root, WORKSPACE_MEMBERS_V1298)

    crates_total = len(crate_results)
    crates_inherit = sum(1 for r in crate_results if r.inherits_workspace)
    crates_inherit_pct = (
        (crates_inherit / crates_total * 100.0) if crates_total else 0.0
    )

    duration_ms = int((time.time() - start) * 1000)

    return LintLedger(
        rust_lints=tuple(rust_lints),
        clippy_lints=tuple(clippy_lints),
        unexpected_cfgs=unexpected,
        crate_lints_inherit=tuple(crate_results),
        has_rust_section=has_rust,
        has_clippy_section=has_clippy,
        rust_lint_count=len(rust_lints),
        clippy_lint_count=len(clippy_lints),
        total_lint_count=len(rust_lints) + len(clippy_lints),
        crates_total=crates_total,
        crates_inherit_count=crates_inherit,
        crates_inherit_pct=crates_inherit_pct,
        duration_ms=duration_ms,
    )


# ============================================================================
# 4. 假说 6 (主 13:08 真自问 + Popper 可证伪)
# ============================================================================

def evaluate_hypotheses(ledger: LintLedger) -> List[HypothesisResult]:
    """6 假说评估."""
    out: List[HypothesisResult] = []

    # H1: [workspace.lints.rust] 段存在 + >=5 lint
    h1_pass = ledger.has_rust_section and ledger.rust_lint_count >= THRESHOLD_RUST_LINTS_MIN
    out.append(HypothesisResult(
        hypothesis_id="h_rust_lints_present",
        description=f"[workspace.lints.rust] 段存在 + lint 定义 ≥ {THRESHOLD_RUST_LINTS_MIN}",
        passed=h1_pass,
        observed=float(ledger.rust_lint_count),
        threshold=float(THRESHOLD_RUST_LINTS_MIN),
        details=f"section present={ledger.has_rust_section}, count={ledger.rust_lint_count}",
    ))

    # H2: [workspace.lints.clippy] 段存在 + >=10 lint
    h2_pass = ledger.has_clippy_section and ledger.clippy_lint_count >= THRESHOLD_CLIPPY_LINTS_MIN
    out.append(HypothesisResult(
        hypothesis_id="h_clippy_lints_present",
        description=f"[workspace.lints.clippy] 段存在 + lint 定义 ≥ {THRESHOLD_CLIPPY_LINTS_MIN}",
        passed=h2_pass,
        observed=float(ledger.clippy_lint_count),
        threshold=float(THRESHOLD_CLIPPY_LINTS_MIN),
        details=f"section present={ledger.has_clippy_section}, count={ledger.clippy_lint_count}",
    ))

    # H3: rustc vs clippy 不混: detect any clippy lint placed in [workspace.lints.rust]
    rust_names = {l.name for l in ledger.rust_lints}
    clippy_in_rust = [n for n in CLIPPY_LINT_NAMES_SAMPLE if n in rust_names]
    h3_pass = len(clippy_in_rust) == 0
    out.append(HypothesisResult(
        hypothesis_id="h_rust_vs_clippy_separation",
        description="[workspace.lints.rust] 段不含 clippy lint (e.g. unused_async)",
        passed=h3_pass,
        observed=float(len(clippy_in_rust)),
        threshold=0.0,
        details=f"误放 rust 段的 clippy lint: {clippy_in_rust or 'None'}",
    ))

    # H4: [workspace.lints.rust.unexpected_cfgs] 含 check-cfg 列表 (kani/fuzzing)
    has_unexpected = ledger.unexpected_cfgs is not None
    check_cfg_count = (
        len(ledger.unexpected_cfgs.check_cfg) if ledger.unexpected_cfgs else 0
    )
    h4_pass = has_unexpected and check_cfg_count >= 1
    out.append(HypothesisResult(
        hypothesis_id="h_unexpected_cfgs_present",
        description="[workspace.lints.rust.unexpected_cfgs] 子段 + check-cfg ≥ 1",
        passed=h4_pass,
        observed=float(check_cfg_count),
        threshold=1.0,
        details=(
            f"section present={has_unexpected}, level="
            f"{ledger.unexpected_cfgs.level if ledger.unexpected_cfgs else 'N/A'}, "
            f"check_cfg={list(ledger.unexpected_cfgs.check_cfg) if ledger.unexpected_cfgs else []}"
        ),
    ))

    # H5: 子 crate [lints] workspace = true 继承占比 >= 95%
    h5_pass = ledger.crates_inherit_pct >= THRESHOLD_LINTS_INHERIT_PCT_MIN
    out.append(HypothesisResult(
        hypothesis_id="h_lints_inherit_pct",
        description=f"workspace members [lints] workspace=true 占比 ≥ {THRESHOLD_LINTS_INHERIT_PCT_MIN}%",
        passed=h5_pass,
        observed=ledger.crates_inherit_pct,
        threshold=float(THRESHOLD_LINTS_INHERIT_PCT_MIN),
        details=(
            f"{ledger.crates_inherit_count}/{ledger.crates_total} 子 crate 继承 workspace.lints; "
            f"missing: "
            f"{[c.crate_name for c in ledger.crate_lints_inherit if not c.inherits_workspace][:5]}"
        ),
    ))

    # H6: workspace.lints 不含全局 deny (deny 单条 OK, 但禁止 '\*' / 'all' 全 deny)
    # 检查所有 lint level 为 deny 的 name 是否在 \"危险名单\" (e.g. all / \*)
    deny_names = {
        l.name for l in ledger.rust_lints if l.level == "deny"
    } | {l.name for l in ledger.clippy_lints if l.level == "deny"}
    dangerous_deny = {"all", "*", "warnings"}
    h6_pass = deny_names.isdisjoint(dangerous_deny)
    out.append(HypothesisResult(
        hypothesis_id="h_no_deny_in_workspace_lints",
        description="workspace.lints 不含 deny='all' / '*' / 'warnings' 全局拒绝",
        passed=h6_pass,
        observed=float(len(deny_names & dangerous_deny)),
        threshold=0.0,
        details=f"禁用全局 deny 命中: {sorted(deny_names & dangerous_deny) or 'None'}",
    ))

    return out


# ============================================================================
# 5. V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ============================================================================

def _v3_philosophy_gate(ledger: LintLedger) -> Tuple[bool, List[str]]:
    """V3 哲学守门: lints audit ≠ lints 安全, 不假装, 不刷 KPI."""
    fails: List[str] = []

    if not ledger.has_rust_section and not ledger.has_clippy_section:
        fails.append("workspace.lints 既无 rust 段也无 clippy 段 — 治理失效")

    if ledger.crates_inherit_pct < 80.0:
        fails.append(
            f"[lints] inheritance {ledger.crates_inherit_pct:.1f}% < 80% — 子 crate 行为不一致风险"
        )

    return (len(fails) == 0, fails)


# ============================================================================
# 6. Audit ledger JSON (主 00:56 任何人都能接手)
# ============================================================================

def _serialize(obj: Any) -> Any:
    if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
        return {f.name: _serialize(getattr(obj, f.name)) for f in dataclasses.fields(obj)}
    if isinstance(obj, (list, tuple)):
        return [_serialize(o) for o in obj]
    return obj


def build_audit_ledger(ledger: LintLedger) -> Dict[str, Any]:
    """包装 sweep 结果为 audit ledger dict (可 JSON 化)."""
    return {
        "version": V1298_VERSION,
        "workspace_root": str(Path(__file__).resolve().parents[1] / "Apeireth-rust"),
        "rust_lint_count": ledger.rust_lint_count,
        "clippy_lint_count": ledger.clippy_lint_count,
        "total_lint_count": ledger.total_lint_count,
        "crates_total": ledger.crates_total,
        "crates_inherit_count": ledger.crates_inherit_count,
        "crates_inherit_pct": ledger.crates_inherit_pct,
        "duration_ms": ledger.duration_ms,
        "has_rust_section": ledger.has_rust_section,
        "has_clippy_section": ledger.has_clippy_section,
        "rust_lints": _serialize(ledger.rust_lints),
        "clippy_lints": _serialize(ledger.clippy_lints),
        "unexpected_cfgs": _serialize(ledger.unexpected_cfgs) if ledger.unexpected_cfgs else None,
        "crate_lints_inherit": _serialize(ledger.crate_lints_inherit),
        "hypotheses": _serialize(evaluate_hypotheses(ledger)),
    }


# ============================================================================
# 7. CLI subcommands (主 00:56 任何人都能接手)
# ============================================================================

def cmd_probe(args: argparse.Namespace) -> int:
    """快速 probe — 输出关键统计."""
    ws = Path(args.workspace) if args.workspace else WORKSPACE_ROOT_DEFAULT
    if not ws.exists():
        print(f"V1298 PROBE FAIL — workspace not found: {ws}")
        return 2
    ledger = sweep_workspace(ws)
    print(f"V1298 PROBE — rust={ledger.rust_lint_count} clippy={ledger.clippy_lint_count} "
          f"total={ledger.total_lint_count} crates_inherit={ledger.crates_inherit_pct:.1f}% "
          f"duration_ms={ledger.duration_ms}")
    print(f"  has_rust_section={ledger.has_rust_section}, "
          f"has_clippy_section={ledger.has_clippy_section}, "
          f"unexpected_cfgs_present={ledger.unexpected_cfgs is not None}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    """跑全 sweep + 评估假说 + 守门."""
    ws = Path(args.workspace) if args.workspace else WORKSPACE_ROOT_DEFAULT
    if not ws.exists():
        print(f"V1298 RUN FAIL — workspace not found: {ws}")
        return 2
    ledger = sweep_workspace(ws)
    hypotheses = evaluate_hypotheses(ledger)

    print(f"V1298 RUN — {ledger.crates_total} crates scanned, "
          f"{ledger.total_lint_count} workspace lints, 6 hypotheses")
    gate_ok, gate_fails = _v3_philosophy_gate(ledger)
    print(f"V3 philosophy gate: {'PASS' if gate_ok else 'FAIL'} ({len(gate_fails)} failures)")

    for h in hypotheses:
        mark = "? PASS" if h.passed else "? FAIL"
        print(f"  {mark} {h.hypothesis_id}: {h.description}")
        print(f"    details: {h.details}")

    n_pass = sum(1 for h in hypotheses if h.passed)
    falsification_rate = (len(hypotheses) - n_pass) / len(hypotheses) * 100.0
    print(f"\nResult: {n_pass}/{len(hypotheses)} hypotheses PASS, "
          f"falsification_rate = {falsification_rate:.2f}%")
    return 0 if gate_ok else 1


def cmd_json(args: argparse.Namespace) -> int:
    """输出 JSON ledger."""
    ws = Path(args.workspace) if args.workspace else WORKSPACE_ROOT_DEFAULT
    if not ws.exists():
        print(json.dumps({"error": f"workspace not found: {ws}"}))
        return 2
    ledger = sweep_workspace(ws)
    data = build_audit_ledger(ledger)
    print(json.dumps(data, ensure_ascii=False, indent=2, default=str))
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    """输出 markdown 报告."""
    ws = Path(args.workspace) if args.workspace else WORKSPACE_ROOT_DEFAULT
    if not ws.exists():
        print(f"V1298 REPORT FAIL — workspace not found: {ws}")
        return 2
    ledger = sweep_workspace(ws)
    hypotheses = evaluate_hypotheses(ledger)
    gate_ok, gate_fails = _v3_philosophy_gate(ledger)

    lines: List[str] = []
    lines.append("# V1298 — Cargo Workspace Lints Audit")
    lines.append("")
    lines.append(f"- version: **{V1298_VERSION}**")
    lines.append(f"- workspace: `{ws}`")
    lines.append(f"- crates scanned: **{ledger.crates_total}**")
    lines.append(f"- rust lints: **{ledger.rust_lint_count}**")
    lines.append(f"- clippy lints: **{ledger.clippy_lint_count}**")
    lines.append(f"- total lints: **{ledger.total_lint_count}**")
    lines.append(f"- crates inherit [lints] workspace=true: "
                 f"**{ledger.crates_inherit_count}/{ledger.crates_total}** "
                 f"({ledger.crates_inherit_pct:.2f}%)")
    lines.append(f"- duration_ms: **{ledger.duration_ms}**")
    lines.append("")

    lines.append("## 假说 (主 13:08 真自问, Popper 可证伪)")
    lines.append("")
    for h in hypotheses:
        mark = "? PASS" if h.passed else "? FAIL"
        lines.append(f"- {mark} **{h.hypothesis_id}**: {h.description}")
        lines.append(f"    - observed={h.observed}, threshold={h.threshold}")
        lines.append(f"    - details: {h.details}")
    lines.append("")

    if ledger.unexpected_cfgs:
        lines.append("## unexpected_cfgs (kani/fuzzing 等防护)")
        lines.append("")
        lines.append(f"- level: **{ledger.unexpected_cfgs.level}**")
        lines.append(f"- check_cfg: `{list(ledger.unexpected_cfgs.check_cfg)}`")
        lines.append("")

    # Missing inheritance
    missing = [c for c in ledger.crate_lints_inherit if not c.inherits_workspace]
    if missing:
        lines.append("## 子 crate 缺 [lints] workspace = true")
        lines.append("")
        for c in missing:
            lines.append(f"- `{c.crate_name}`")
        lines.append("")

    text = "\n".join(lines) + "\n"
    if args.output:
        Path(args.output).write_text(text, encoding="utf-8")
        print(f"Wrote report to {args.output}")
    else:
        sys.stdout.write(text)
    return 0


def cmd_inheritance(args: argparse.Namespace) -> int:
    """列出子 crate [lints] workspace = true 继承情况."""
    ws = Path(args.workspace) if args.workspace else WORKSPACE_ROOT_DEFAULT
    if not ws.exists():
        print(json.dumps({"error": f"workspace not found: {ws}"}))
        return 2
    ledger = sweep_workspace(ws)
    out = [
        {"crate": c.crate_name, "has_lints_section": c.has_lints_section,
         "inherits_workspace": c.inherits_workspace}
        for c in ledger.crate_lints_inherit
    ]
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


# ============================================================================
# 8. main() entry
# ============================================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="Cargo.toml workspace.lints 维度审计 (VCP 真源代码深读 #19)"
    )
    parser.add_argument("--workspace", default=str(WORKSPACE_ROOT_DEFAULT),
                        help="workspace root 路径 (default: ../Apeireth-rust)")
    parser.add_argument("--probe", action="store_true", help="轻量 probe")
    parser.add_argument("--run", action="store_true", help="全 sweep + 假说")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--report", action="store_true", help="输出 markdown 报告")
    parser.add_argument("--inheritance", action="store_true", help="列出继承情况 (JSON)")
    parser.add_argument("--output", default=None, help="(report) 写入文件路径")
    parser.add_argument("--limit", type=int, default=5)

    args = parser.parse_args(argv)

    if args.probe or not any([args.run, args.json, args.report, args.inheritance]):
        return cmd_probe(args)
    if args.run:
        return cmd_run(args)
    if args.json:
        return cmd_json(args)
    if args.report:
        return cmd_report(args)
    if args.inheritance:
        return cmd_inheritance(args)
    return cmd_probe(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
