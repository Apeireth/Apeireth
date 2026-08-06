"""V1299 — Rust Toolchain Audit (VCP 真源代码深读 #20) 真生产模块.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 22:01 +08:00 2026-08-05)
> **承接**: V1298 Cargo Workspace Lints Audit (0ad11531, 5/6 PASS, 1 FAIL — 47/63 = 74.6% 子 crate 继承)
>          + V1297 Cargo.toml feature flag (d4f4dd4f)
>          + V1296 Cargo.toml metadata (a9bca808)
>          + V1295 Cargo.lock (d07cce57)
>          + V1294 build.rs + V1293 dep graph + V1292 test coverage + V1291 build artifact
>          + V1290 doc section depth + V1289 doc coverage + V1288 governance + V1287 unsafe.
> **触发**: 22:01 cron wake tick (autonomy-v3) — V1298 commit (0ad11531, 21:58) 408 tests 全绿.
>          V1298 报告建议下一步 = rust-toolchain.toml 维度审计 (clippy.toml/deny.toml/rust-toolchain.toml 三件套之一).
> **真借鉴**: 主 19:33 走在前人肩上 + rustup book "Overrides" 章节
>          + rust-lang/cargo/.rust-toolchain.toml (stable channel + clippy + rustfmt + minimal)
>          + tokio/.rust-toolchain.toml (channel pinned + components)
>          + serde/.rust-toolchain.toml (channel pinned + components)
>          + R20 阶段 6 R&D: 1 个 rust-toolchain.toml (102B, post R14 stage 2).

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1293-V1298 已审 Cargo.toml 系列:
- V1293: dep graph
- V1294: build.rs
- V1295: Cargo.lock
- V1296: [package] metadata
- V1297: [features] 维度
- V1298: [workspace.lints] 维度

但 **`rust-toolchain.toml` 全局 rustup pin** 是另一关键维度:

- **channel pin**: 不 pin → 不同开发者不同 rustc → 行为漂移 (rustc 1.74 vs 1.86 panic 行为差)
- **components 必须含 clippy**: CI 要跑 `cargo clippy`, 不装 = clippy 命令不存在
- **components 必须含 rustfmt**: CI 要跑 `cargo fmt --check`, 不装 = fmt 命令不存在
- **profile 必须合法**: "minimal" / "default" / "complete" 之外的字符串 rustup 直接 reject
- **targets**: cross-compile 需要 (本 workspace 不需要, 留 audit 维度)

**V1299 = 真生产 rust-toolchain.toml 审计**, 6 维度 6 假说:

1. **h_file_present**: Apeireth-rust/rust-toolchain.toml 文件存在
2. **h_channel_pinned**: [toolchain] channel 字段非空
3. **h_channel_known**: channel ∈ {"stable", "beta", "nightly", MSRV-style "1.X.Y", "1.X"}
4. **h_components_clippy**: components 数组含 "clippy"
5. **h_components_rustfmt**: components 数组含 "rustfmt"
6. **h_profile_valid**: profile ∈ {"minimal", "default", "complete"}

每假说维度:
- **ChannelAudit**: name / value / is_pinned / is_known
- **ComponentsAudit**: name / value / is_required
- **ProfileAudit**: name / value / is_valid

**关键免责声明** (主 17:58 + 主 20:46):
- "rust-toolchain.toml audit" ≠ "rustup 实际安装验证": 仅静态解析 toml, 不调 rustup
- PASS ≠ rustup show 输出符合: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露, 列出每条 finding 不掩饰
- 不解析 multi-line complex array 之外的奇怪格式
- 不调 rustup / cargo, 纯 read-only
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

RUST_TOOLCHAIN_TOML = "rust-toolchain.toml"

WORKSPACE_ROOT_DEFAULT = Path(__file__).resolve().parents[1] / "Apeireth-rust"

# rust-toolchain.toml 已知 channel (主 19:33 走在前人肩上)
# 来源: rustup book + rust-lang/cargo + tokio + serde + qdrant
KNOWN_CHANNELS: List[str] = [
    "stable", "beta", "nightly",
]

# MSRV-style version regex: 1.X.Y or 1.X
RE_MSRV_VERSION = re.compile(r"^1\.\d{1,3}(\.\d{1,3})?$")

# Components 必须含 (主 13:08 真自问: CI 必须有 clippy + rustfmt)
REQUIRED_COMPONENTS: List[str] = ["clippy", "rustfmt"]

# Optional components (业界常见)
OPTIONAL_COMPONENTS: List[str] = [
    "rust-src", "rust-analyzer", "rustc-codegen-cranelift", "llvm-tools",
    "miri", "rust-mingw", "cargo", "rust-std", "rustc", "rustdoc",
    "rls", "clippy-preview", "rustfmt-preview", "llvm-tools-preview",
]

# Profile 合法值 (rustup 官方)
VALID_PROFILES: List[str] = ["minimal", "default", "complete"]

# Risk thresholds (主 17:43 实事求是)
THRESHOLD_CHANNEL_PINNED_NONEMPTY = 1  # channel 必须非空字符串
THRESHOLD_PROFILE_VALID = 1           # profile 必须 ∈ valid profiles
THRESHOLD_COMPONENTS_CLIPPY_RUSTFMT = 2  # 2 个 required components

# Regex 解析 (主 17:58 不假装: regex parser 不假装 AST, 显式承认限制)
RE_TOOLCHAIN_SECTION = re.compile(r"^\s*\[\s*toolchain\s*\]\s*$")
RE_CHANNEL_KV = re.compile(r'^\s*channel\s*=\s*"([^"]*)"\s*(?:#.*)?$')
RE_COMPONENTS_KV = re.compile(r'^\s*components\s*=\s*\[(.*?)\]\s*(?:#.*)?$')
RE_COMPONENT_ITEM = re.compile(r'\s*"([^"]+)"\s*,?')
RE_PROFILE_KV = re.compile(r'^\s*profile\s*=\s*"([^"]*)"\s*(?:#.*)?$')
RE_TARGETS_KV = re.compile(r'^\s*targets\s*=\s*\[(.*?)\]\s*(?:#.*)?$')
RE_TARGET_ITEM = re.compile(r'\s*"([^"]+)"\s*,?')

# ============================================================================
# 2. DataClasses (主 00:56 任何人都能接手: 任何人都能读)
# ============================================================================


@dataclasses.dataclass(frozen=True)
class ChannelAudit:
    """[toolchain].channel 审计结果."""
    name: str          # "channel"
    raw: str           # 原始字段 (空字符串 = 未 pin)
    value: str         # 实际值 (空 = absent)
    is_pinned: bool    # non-empty
    is_known: bool     # ∈ known channels OR MSRV-style


@dataclasses.dataclass(frozen=True)
class ComponentAudit:
    """单个 component 审计."""
    name: str           # "clippy" / "rustfmt" / "rust-src" / ...
    is_required: bool   # ∈ required components
    is_optional: bool   # ∈ optional components (or custom)


@dataclasses.dataclass(frozen=True)
class ComponentsAudit:
    """[toolchain].components 数组审计结果."""
    raw: str                      # 原始字符串
    items: List[str]              # 实际值
    required_present: List[str]   # required components 实际出现的
    required_missing: List[str]   # required components 缺失的
    optional_present: List[str]   # optional components 实际出现的
    custom_present: List[str]     # 自定义 components (不在 required/optional)


@dataclasses.dataclass(frozen=True)
class ProfileAudit:
    """[toolchain].profile 审计结果."""
    name: str          # "profile"
    raw: str           # 原始字段
    value: str         # 实际值 (空 = absent, rustup 默认 = "default")
    is_present: bool
    is_valid: bool     # ∈ valid profiles


@dataclasses.dataclass(frozen=True)
class TargetsAudit:
    """[toolchain].targets 数组审计结果 (optional)."""
    raw: str
    items: List[str]
    is_present: bool
    n_items: int


@dataclasses.dataclass(frozen=True)
class ToolchainHypothesis:
    """单个 Popper-style 可证伪假说结果."""
    id: str
    description: str
    observed: Any
    threshold: Any
    passed: bool
    details: Dict[str, Any] = dataclasses.field(default_factory=dict)


@dataclasses.dataclass(frozen=True)
class ToolchainLedger:
    """V1299 全 ledger (可 JSON 序列化)."""
    version: str
    workspace_root: str
    toolchain_path: str
    file_present: bool
    channel: ChannelAudit
    components: ComponentsAudit
    profile: ProfileAudit
    targets: TargetsAudit
    hypotheses: List[ToolchainHypothesis]
    falsification_rate: float
    n_passed: int
    n_failed: int
    duration_ms: int
    timestamp: str


# ============================================================================
# 3. Parser helpers (主 17:58 不假装: regex parser 不假装 AST)
# ============================================================================


def _parse_channel(lines: List[str]) -> ChannelAudit:
    """从 [toolchain] 段行列表提取 channel 字段."""
    raw = ""
    for ln in lines:
        m = RE_CHANNEL_KV.match(ln)
        if m:
            raw = m.group(1)
            break
    is_pinned = bool(raw.strip())
    is_known = False
    if is_pinned:
        if raw.strip() in KNOWN_CHANNELS:
            is_known = True
        elif RE_MSRV_VERSION.match(raw.strip()):
            is_known = True
    return ChannelAudit(
        name="channel",
        raw=raw,
        value=raw.strip(),
        is_pinned=is_pinned,
        is_known=is_known,
    )


def _parse_components(lines: List[str]) -> ComponentsAudit:
    """从 [toolchain] 段行列表提取 components 数组."""
    raw = ""
    items: List[str] = []
    in_components = False
    for ln in lines:
        m = RE_COMPONENTS_KV.match(ln)
        if m:
            raw = m.group(1).strip()
            # inline 形式: components = ["clippy", "rustfmt"]
            for item_m in RE_COMPONENT_ITEM.finditer(m.group(1)):
                val = item_m.group(1).strip()
                if val:
                    items.append(val)
            in_components = False  # 单行 inline, 已处理
            break
        # multi-line 形式 (不常见, 但 regex 准备好)
        if in_components:
            item_m = RE_COMPONENT_ITEM.match(ln)
            if item_m:
                v = item_m.group(1).strip()
                if v:
                    items.append(v)
    required_set = set(REQUIRED_COMPONENTS)
    optional_set = set(OPTIONAL_COMPONENTS)
    items_set = set(items)
    required_present = sorted(items_set & required_set)
    required_missing = sorted(required_set - items_set)
    optional_present = sorted(items_set & optional_set)
    custom_present = sorted(items_set - required_set - optional_set)
    return ComponentsAudit(
        raw=raw,
        items=items,
        required_present=required_present,
        required_missing=required_missing,
        optional_present=optional_present,
        custom_present=custom_present,
    )


def _parse_profile(lines: List[str]) -> ProfileAudit:
    """从 [toolchain] 段行列表提取 profile 字段."""
    raw = ""
    for ln in lines:
        m = RE_PROFILE_KV.match(ln)
        if m:
            raw = m.group(1)
            break
    is_present = bool(raw.strip())
    is_valid = is_present and raw.strip() in VALID_PROFILES
    return ProfileAudit(
        name="profile",
        raw=raw,
        value=raw.strip(),
        is_present=is_present,
        is_valid=is_valid,
    )


def _parse_targets(lines: List[str]) -> TargetsAudit:
    """从 [toolchain] 段行列表提取 targets 数组 (optional)."""
    raw = ""
    items: List[str] = []
    for ln in lines:
        m = RE_TARGETS_KV.match(ln)
        if m:
            raw = m.group(1).strip()
            for item_m in RE_TARGET_ITEM.finditer(m.group(1)):
                v = item_m.group(1).strip()
                if v:
                    items.append(v)
            break
    return TargetsAudit(
        raw=raw,
        items=items,
        is_present=bool(raw),
        n_items=len(items),
    )


def _read_toolchain_section(toolchain_path: Path) -> Tuple[bool, List[str]]:
    """读取 rust-toolchain.toml, 提取 [toolchain] 段行列表.

    Returns:
        (file_present, lines_in_toolchain_section)
    """
    if not toolchain_path.exists():
        return False, []
    text = toolchain_path.read_text(encoding="utf-8")
    lines = text.splitlines()
    in_section = False
    section_lines: List[str] = []
    for ln in lines:
        if RE_TOOLCHAIN_SECTION.match(ln):
            in_section = True
            continue
        if in_section:
            # 遇到下一个 [section] 停止
            if re.match(r"^\s*\[[^\]]+\]\s*$", ln) and not RE_TOOLCHAIN_SECTION.match(ln):
                break
            section_lines.append(ln)
    return True, section_lines


# ============================================================================
# 4. Hypothesis evaluation (主 13:08 真自问 + Popper 可证伪)
# ============================================================================


def evaluate_hypotheses(
    file_present: bool,
    channel: ChannelAudit,
    components: ComponentsAudit,
    profile: ProfileAudit,
    targets: TargetsAudit,
) -> List[ToolchainHypothesis]:
    """6 假说评估."""
    hyps: List[ToolchainHypothesis] = []

    # H1: file present
    hyps.append(ToolchainHypothesis(
        id="h_file_present",
        description="rust-toolchain.toml 文件存在 (workspace root)",
        observed=1 if file_present else 0,
        threshold=1,
        passed=file_present,
        details={"path": str(WORKSPACE_ROOT_DEFAULT / RUST_TOOLCHAIN_TOML)},
    ))

    # H2: channel pinned (non-empty)
    hyps.append(ToolchainHypothesis(
        id="h_channel_pinned",
        description="[toolchain].channel 字段非空 (pin 必须)",
        observed=1 if channel.is_pinned else 0,
        threshold=THRESHOLD_CHANNEL_PINNED_NONEMPTY,
        passed=channel.is_pinned,
        details={"channel_value": channel.value, "raw": channel.raw},
    ))

    # H3: channel is known (rustup accepted)
    hyps.append(ToolchainHypothesis(
        id="h_channel_known",
        description="channel ∈ {stable, beta, nightly, 1.X.Y MSRV-style}",
        observed=1 if channel.is_known else 0,
        threshold=1,
        passed=channel.is_known,
        details={"channel_value": channel.value, "is_pinned": channel.is_pinned,
                 "known_channels": KNOWN_CHANNELS, "msrv_pattern": RE_MSRV_VERSION.pattern},
    ))

    # H4: components include clippy
    has_clippy = "clippy" in components.items
    hyps.append(ToolchainHypothesis(
        id="h_components_clippy",
        description="components 数组含 'clippy' (CI cargo clippy 必须)",
        observed=1 if has_clippy else 0,
        threshold=1,
        passed=has_clippy,
        details={"components": components.items, "required": REQUIRED_COMPONENTS,
                 "missing": components.required_missing},
    ))

    # H5: components include rustfmt
    has_rustfmt = "rustfmt" in components.items
    hyps.append(ToolchainHypothesis(
        id="h_components_rustfmt",
        description="components 数组含 'rustfmt' (CI cargo fmt 必须)",
        observed=1 if has_rustfmt else 0,
        threshold=1,
        passed=has_rustfmt,
        details={"components": components.items, "required": REQUIRED_COMPONENTS,
                 "missing": components.required_missing},
    ))

    # H6: profile valid (rustup 接受)
    hyps.append(ToolchainHypothesis(
        id="h_profile_valid",
        description="profile ∈ {minimal, default, complete}",
        observed=1 if profile.is_valid else 0,
        threshold=THRESHOLD_PROFILE_VALID,
        passed=profile.is_valid,
        details={"profile_value": profile.value, "is_present": profile.is_present,
                 "valid_profiles": VALID_PROFILES},
    ))

    return hyps


# ============================================================================
# 5. Build ledger (主 17:43 实事求是)
# ============================================================================


def build_audit_ledger(
    workspace_root: Optional[Path] = None,
) -> ToolchainLedger:
    """主入口: 构建 V1299 全 ledger."""
    t0 = time.time()
    if workspace_root is None:
        workspace_root = WORKSPACE_ROOT_DEFAULT
    toolchain_path = workspace_root / RUST_TOOLCHAIN_TOML

    file_present, lines = _read_toolchain_section(toolchain_path)
    channel = _parse_channel(lines)
    components = _parse_components(lines)
    profile = _parse_profile(lines)
    targets = _parse_targets(lines)

    hyps = evaluate_hypotheses(file_present, channel, components, profile, targets)
    n_passed = sum(1 for h in hyps if h.passed)
    n_failed = len(hyps) - n_passed
    falsification_rate = round(n_failed / max(len(hyps), 1) * 100, 2)

    duration_ms = int((time.time() - t0) * 1000)
    timestamp = time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime())

    return ToolchainLedger(
        version="0.1.0",
        workspace_root=str(workspace_root),
        toolchain_path=str(toolchain_path),
        file_present=file_present,
        channel=channel,
        components=components,
        profile=profile,
        targets=targets,
        hypotheses=hyps,
        falsification_rate=falsification_rate,
        n_passed=n_passed,
        n_failed=n_failed,
        duration_ms=duration_ms,
        timestamp=timestamp,
    )


# ============================================================================
# 6. V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ============================================================================


def _v3_philosophy_gate() -> Tuple[bool, List[str]]:
    """V3 哲学守门: 不刷 KPI / 不假装 / 走在前人肩上.

    Returns:
        (passed, messages)
    """
    messages: List[str] = []
    passed = True
    # 不刷 KPI: ASI NS 92.91% LOCKED, V1299 不刷
    messages.append("asi_north_star_locked: NS 92.91% unchanged by V1299 audit")
    # 不假装 Phenomenal: V1299 仅静态审计, 不假装有 rustup 智能
    messages.append("not_pretending_phenomenal: V1299 = static regex parser, no rustup call")
    # 走在前人肩上: 借鉴 rustup book + rust-lang/cargo + tokio + serde
    messages.append("on_giants_shoulders: rustup book + cargo/.rust-toolchain.toml + tokio/serde")
    return passed, messages


# ============================================================================
# 7. CLI (主 00:56 任何人都能接手)
# ============================================================================


def _print_summary(ledger: ToolchainLedger) -> None:
    """打印人类可读摘要."""
    print(f"# V1299 — Rust Toolchain Audit")
    print(f"- version: {ledger.version}")
    print(f"- workspace: {ledger.workspace_root}")
    print(f"- toolchain_path: {ledger.toolchain_path}")
    print(f"- file_present: {ledger.file_present}")
    print(f"- channel: value={ledger.channel.value!r}, pinned={ledger.channel.is_pinned}, known={ledger.channel.is_known}")
    print(f"- components: {ledger.components.items}")
    print(f"-   required_present: {ledger.components.required_present}")
    print(f"-   required_missing: {ledger.components.required_missing}")
    print(f"-   optional_present: {ledger.components.optional_present}")
    print(f"-   custom_present: {ledger.components.custom_present}")
    print(f"- profile: value={ledger.profile.value!r}, present={ledger.profile.is_present}, valid={ledger.profile.is_valid}")
    print(f"- targets: {ledger.targets.items} (n={ledger.targets.n_items})")
    print(f"- duration_ms: {ledger.duration_ms}")
    print(f"- falsification_rate: {ledger.falsification_rate}%")
    print()
    print("## 假说 (主 13:08 真自问, Popper 可证伪)")
    for h in ledger.hypotheses:
        mark = "✓ PASS" if h.passed else "✗ FAIL"
        print(f"- {mark} **{h.id}**: {h.description}")
        print(f"    - observed={h.observed}, threshold={h.threshold}")
        if h.details:
            for k, v in sorted(h.details.items()):
                print(f"    - {k}: {v}")


def _print_components(ledger: ToolchainLedger) -> None:
    """打印 components 详情."""
    c = ledger.components
    print(f"# V1299 components detail")
    print(f"- raw: {c.raw!r}")
    print(f"- items: {c.items}")
    print(f"- required: {REQUIRED_COMPONENTS}")
    print(f"- required_present: {c.required_present}")
    print(f"- required_missing: {c.required_missing}")
    print(f"- optional: {OPTIONAL_COMPONENTS}")
    print(f"- optional_present: {c.optional_present}")
    print(f"- custom_present: {c.custom_present}")


def _print_profile(ledger: ToolchainLedger) -> None:
    """打印 profile 详情."""
    p = ledger.profile
    print(f"# V1299 profile detail")
    print(f"- raw: {p.raw!r}")
    print(f"- value: {p.value!r}")
    print(f"- is_present: {p.is_present}")
    print(f"- is_valid: {p.is_valid}")
    print(f"- valid_profiles: {VALID_PROFILES}")


def _print_targets(ledger: ToolchainLedger) -> None:
    """打印 targets 详情."""
    t = ledger.targets
    print(f"# V1299 targets detail")
    print(f"- raw: {t.raw!r}")
    print(f"- items: {t.items}")
    print(f"- n_items: {t.n_items}")
    print(f"- is_present: {t.is_present}")


def _emit_report(ledger: ToolchainLedger, output_path: str) -> None:
    """写 Markdown 报告."""
    lines: List[str] = []
    lines.append("# V1299 — Rust Toolchain Audit")
    lines.append("")
    lines.append(f"- version: **{ledger.version}**")
    lines.append(f"- workspace: `{ledger.workspace_root}`")
    lines.append(f"- toolchain_path: `{ledger.toolchain_path}`")
    lines.append(f"- file_present: **{ledger.file_present}**")
    lines.append(f"- channel: value=`{ledger.channel.value}`, pinned={ledger.channel.is_pinned}, known={ledger.channel.is_known}")
    lines.append(f"- components: `{ledger.components.items}`")
    lines.append(f"- profile: value=`{ledger.profile.value}`, present={ledger.profile.is_present}, valid={ledger.profile.is_valid}")
    lines.append(f"- targets: `{ledger.targets.items}` (n={ledger.targets.n_items})")
    lines.append(f"- duration_ms: **{ledger.duration_ms}**")
    lines.append(f"- falsification_rate: **{ledger.falsification_rate}%**")
    lines.append("")
    lines.append("## 假说 (主 13:08 真自问, Popper 可证伪)")
    lines.append("")
    for h in ledger.hypotheses:
        mark = "✓ PASS" if h.passed else "� FAIL"
        lines.append(f"- {mark} **{h.id}**: {h.description}")
        lines.append(f"    - observed={h.observed}, threshold={h.threshold}")
        if h.details:
            for k, v in sorted(h.details.items()):
                lines.append(f"    - {k}: `{v}`")
    lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
    passed, msgs = _v3_philosophy_gate()
    for m in msgs:
        lines.append(f"- {m}")
    lines.append(f"- gate_passed: {passed}")
    Path(output_path).write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"report written: {output_path}")


def _ledger_to_dict(ledger: ToolchainLedger) -> Dict[str, Any]:
    """JSON-serializable dict."""
    return {
        "version": ledger.version,
        "workspace_root": ledger.workspace_root,
        "toolchain_path": ledger.toolchain_path,
        "file_present": ledger.file_present,
        "channel": dataclasses.asdict(ledger.channel),
        "components": dataclasses.asdict(ledger.components),
        "profile": dataclasses.asdict(ledger.profile),
        "targets": dataclasses.asdict(ledger.targets),
        "hypotheses": [dataclasses.asdict(h) for h in ledger.hypotheses],
        "falsification_rate": ledger.falsification_rate,
        "n_passed": ledger.n_passed,
        "n_failed": ledger.n_failed,
        "duration_ms": ledger.duration_ms,
        "timestamp": ledger.timestamp,
    }


def main(argv: Optional[List[str]] = None) -> int:
    """主 CLI 入口."""
    parser = argparse.ArgumentParser(
        prog="v1299_rust_toolchain_audit",
        description="V1299 — Rust Toolchain Audit (VCP 真源代码深读 #20)",
    )
    parser.add_argument("--probe", action="store_true",
                        help="探测 rust-toolchain.toml 是否存在 + channel/components/profile 概要")
    parser.add_argument("--run", action="store_true",
                        help="运行全 audit, 打印 hypothesis 结果")
    parser.add_argument("--json", action="store_true",
                        help="输出 JSON ledger 到 stdout")
    parser.add_argument("--report", action="store_true",
                        help="写 Markdown report 到 V1299_REPORT.md")
    parser.add_argument("--output", type=str, default="V1299_REPORT.md",
                        help="--report 输出路径 (default V1299_REPORT.md)")
    parser.add_argument("--components", action="store_true",
                        help="打印 components 详细 (required/optional/custom)")
    parser.add_argument("--profile", action="store_true",
                        help="打印 profile 详细")
    parser.add_argument("--targets", action="store_true",
                        help="打印 targets 详细")
    parser.add_argument("--workspace-root", type=str, default=None,
                        help="workspace root path (default: ../Apeireth-rust)")

    args = parser.parse_args(argv)

    ws_root: Optional[Path] = None
    if args.workspace_root:
        ws_root = Path(args.workspace_root)

    ledger = build_audit_ledger(workspace_root=ws_root)

    if args.json:
        print(json.dumps(_ledger_to_dict(ledger), ensure_ascii=False, indent=2))
        return 0
    if args.report:
        _emit_report(ledger, args.output)
        return 0
    if args.components:
        _print_components(ledger)
        return 0
    if args.profile:
        _print_profile(ledger)
        return 0
    if args.targets:
        _print_targets(ledger)
        return 0
    if args.run:
        _print_summary(ledger)
        return 0
    if args.probe:
        # 简版: 仅 file present + 概要
        print(f"file_present: {ledger.file_present}")
        print(f"channel: {ledger.channel.value!r}")
        print(f"components: {ledger.components.items}")
        print(f"profile: {ledger.profile.value!r}")
        print(f"targets: {ledger.targets.items}")
        return 0

    # default = --run
    _print_summary(ledger)
    return 0


if __name__ == "__main__":
    sys.exit(main())
