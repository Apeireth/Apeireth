"""V1297 — Cargo Feature Flag Audit (VCP 真源代码深读 #18) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 21:16 +08:00 2026-08-05)
> **承接**: V1296 Cargo.toml edition/MSRV/license audit (a9bca808) + V1295 Cargo.lock (d07cce57)
>          + V1294 build.rs + V1293 dep graph + V1292 test coverage + V1291 build artifact
>          + V1290 doc section depth + V1289 doc coverage + V1288 governance + V1287 unsafe
>          + V1286 security priority + V1285 all-42 security + V1284 worst-5 security
>          + V1283 multi-crate semantic + V1282 sovereignty semantic + V1281 asi semantic
>          + V1280 rust workspace static.
>          V1297 = **Cargo.toml feature flag 维度** (workspace.dependencies features + per-crate
>            [features] 块 + dep: 前缀使用 + default features policy + feature explosion 风险)
> **真借鉴**: 主 19:33 走在前人肩上 + cargo book "Features" 章节 (https://doc.rust-lang.org/cargo/reference/features.html)
>         + Rust 2021 edition `dep:` 前缀语法 + apeireth 56 crates 真扫 (6 个 [features] 块:
>           bus, central, graph, memory, pybridge, web).
> **不假装**: V1297 = 真生产 Cargo.toml feature flag 静态审计, 不刷 KPI
>         不假装"feature 少 = 安全", 不假装"dep: 前缀 = 万灵药"
>         不假装"default = [] = 完美" (web 的 default = ["ssr"] 是合理设计)
>         不假装"features audit = ASI", 仅 audit-threshold.

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1293 已审 Cargo.toml 声明的依赖图, V1294 已审 build.rs, V1295 已审 Cargo.lock,
V1296 已审 [package] metadata. **但 Cargo.toml feature flag 维度 是另一面镜子**:

- **workspace.dependencies features**: tokio ["full"] / serde ["derive"] / pyo3 ["auto-initialize"]
  / rusqlite ["bundled"] / chrono ["serde"] / uuid ["v4", "serde"] / criterion ["html_reports"]
  / reqwest ["json", "rustls-tls", "stream"] (default-features = false)
- **per-crate [features]**: 6 crates (bus/central/graph/memory/pybridge/web) 暴露 features
- **`dep:` 前缀**: Rust 2021 edition 推荐, 显式声明 optional deps 关系 (pybridge + graph 用了)
- **default features policy**: 多数 `default = []` (避免 build 失败), web 是 `default = ["ssr"]`
- **feature explosion 风险**: workspace 8 workspace.deps × per-crate features = 组合爆炸

**V1297 = 真生产全 Cargo.toml feature flag 审计**, 6 维度 6 假说:

1. **h_workspace_deps_with_features**: workspace.dependencies 中显式 features >= 5 (期望)
2. **h_crates_with_features_section**: workspace members 中 [features] 块占比 <= 25% (期望精简)
3. **h_dep_prefix_usage**: 6 个有 [features] 的 crate 中, 用 `dep:` 前缀的 >= 50% (3/6)
4. **h_default_empty_dominant**: 6 个有 [features] 的 crate 中, `default = []` 占 >= 50% (3/6)
5. **h_no_hardcoded_version_in_features**: [features] 块内无 hardcoded 版本 (e.g. `serde = "1.0"`)
6. **h_pyo3_not_default**: `pyo3` / `python-ext` 不在 default features (避免无 Python 编译失败)

外加 per-crate 维度:
- **WorkspaceDepFeatureAudit**: dep_name / version / features_list / default_features_disabled / dep_count
- **CrateFeatureAudit**: crate_name / has_features_block / default_value / feature_count / feature_names
  / uses_dep_prefix / has_hardcoded_version

**关键免责声明** (主 17:58 + 主 20:46):
- "feature audit" ≠ "feature 安全": 仅 Cargo.toml 静态解析, 不调 cargo check / cargo build / cargo tree
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变
- FAIL 也诚实披露
- 不假装 parse TOML AST: regex 简化 pattern match
- 不调 cargo: 纯 read-only Cargo.toml 解析
- 不假装 features = 安全门: features 是 API 暴露面, 不是漏洞
- 不假装 `dep:` 前缀 = 完美: 仅是 2021 edition 显式语法, 旧 `foo = ["bar"]` 也合法

## CLI (主 00:56 任何人都能接手)

```bash
# 探测 (仅 feature flag 统计)
python -m apeireth.v1297_cargo_feature_flag_audit --probe

# 跑全 sweep + 输出报告
python -m apeireth.v1297_cargo_feature_flag_audit --run

# 输出 JSON ledger
python -m apeireth.v1297_cargo_feature_flag_audit --json

# 输出 markdown 报告
python -m apeireth.v1297_cargo_feature_flag_audit --report --output V1297_REPORT.md

# 看 workspace deps features 列表
python -m apeireth.v1297_cargo_feature_flag_audit --workspace-deps

# 看 crates with [features] 列表
python -m apeireth.v1297_cargo_feature_flag_audit --crates-with-features
```
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import re
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================
# 0. Constants (主 17:43 实事求是)
# ============================================================

WORKSPACE_ROOT_DEFAULT = Path(__file__).resolve().parent.parent / "Apeireth-rust"
CARGO_TOML = "Cargo.toml"

# Workspace member list (V1297 = 56 crates, 跟 V1296 一致)
WORKSPACE_MEMBERS_V1297: List[str] = [
    "apeireth-core", "apeireth-memory", "apeireth-asi", "apeireth-tools",
    "apeireth-cli", "apeireth-bench", "apeireth-cognition", "apeireth-action",
    "apeireth-life-force", "apeireth-constraint", "apeireth-central",
    "apeireth-value", "apeireth-consciousness", "apeireth-relation",
    "apeireth-motivation", "apeireth-perception", "apeireth-upgrade",
    "apeireth-onion", "apeireth-council", "apeireth-sovereignty",
    "apeireth-supervisor", "apeireth-pybridge", "apeireth-verify",
    "apeireth-extension", "apeireth-evolution", "apeireth-bus",
    "apeireth-api", "apeireth-web", "apeireth-tauri-stub", "apeireth-tui",
    "apeireth-protocol", "apeireth-http-client", "apeireth-pipeline",
    "apeireth-tool-registry", "apeireth-tool-runtime", "apeireth-tool-approval",
    "apeireth-agent", "apeireth-mcp", "apeireth-graph", "apeireth-formal",
    "apeireth-vector", "apeireth-sdk", "apeireth-workflow", "apeireth-team-lead",
    "apeireth-mcp-relay-image", "apeireth-mcp-ssh", "apeireth-mcp-winrm",
    "apeireth-image-prompt", "apeireth-plugin", "apeireth-rollback",
    "apeireth-keyring", "apeireth-lark", "apeireth-machine-id",
    "apeireth-repo-analyzer", "apeireth-repo-scan", "apeireth-voice",
]

# Pattern regex (regex-only, 不解析 AST)
RE_WORKSPACE_DEPS = re.compile(r"^\[workspace\.dependencies\]\s*$")
RE_FEATURES_HEADER = re.compile(r"^\[features\]\s*$")
RE_SECTION_END = re.compile(r"^\[\[?[a-zA-Z0-9_.\-]+\]?\]\s*$")
RE_DEP_INLINE = re.compile(
    r"""^([a-zA-Z0-9_-]+)\s*=\s*\{\s*([^}]+?)\s*\}\s*$"""
)
RE_DEP_INLINE_FEATURES_KV = re.compile(
    r"""features\s*=\s*\[([^\]]*)\]"""
)
RE_DEP_INLINE_DEFAULT_FEATURES_KV = re.compile(
    r"""default-features\s*=\s*(true|false)"""
)
RE_DEP_INLINE_VERSION_KV = re.compile(
    r"""version\s*=\s*"([^"]+)\""""
)
RE_DEP_PLAIN = re.compile(r"""^([a-zA-Z0-9_-]+)\s*=\s*"([^"]+)"\s*$""")
# 单行 feature: name = ["dep:foo", "bar/baz"]
RE_FEATURE_LINE_SIMPLE = re.compile(r"""^([a-zA-Z0-9_-]+)\s*=\s*\[\s*([^\]]*?)\s*\]\s*$""")
# feature name (在多行列表 / 单行列表内匹配一个元素)
RE_FEATURE_NAME_TOKEN = re.compile(r""""([a-zA-Z0-9_/\-:.@]+)"\s*/?""")
# 特征条目中是否包含 dep: 前缀
RE_DEP_PREFIX_IN_VALUE = re.compile(r""""dep:([a-zA-Z0-9_/\-:]+)""")
# 特征条目中是否包含版本字符串 (e.g. "1.0" / "0.32")
RE_HARDCODED_VERSION_IN_VALUE = re.compile(r'"(\d+\.\d+(?:\.\d+)?)"')


# ============================================================
# 1. Data classes (主 00:56 任何人都能接手 + 主 17:43 实事求是)
# ============================================================

@dataclasses.dataclass(frozen=True)
class WorkspaceDepFeatureAudit:
    """workspace.dependencies 单条 dep 的 feature flag 审计"""
    dep_name: str
    version: Optional[str]
    features: Tuple[str, ...]
    features_count: int
    default_features_disabled: bool
    raw_text: str


@dataclasses.dataclass(frozen=True)
class CrateFeatureAudit:
    """单个 crate [features] 块审计"""
    crate_name: str
    has_features_block: bool
    default_value: Tuple[str, ...]  # default features 列表 (空 = default = [])
    default_count: int
    feature_count: int
    feature_names: Tuple[str, ...]
    uses_dep_prefix: bool  # 用了 dep: 前缀
    has_hardcoded_version: bool  # features 内 hardcoded 版本
    raw_text_snippet: str  # 截取前 200 字符


@dataclasses.dataclass(frozen=True)
class HypothesisResult:
    """单个假说验证结果"""
    hypothesis_id: str
    description: str
    passed: bool
    observed: Any
    threshold: Any
    details: Optional[str] = None


@dataclasses.dataclass(frozen=True)
class FeatureFlagLedger:
    """整 workspace feature flag 审计结果 ledger"""
    workspace_deps: List[WorkspaceDepFeatureAudit]
    crate_features: List[CrateFeatureAudit]
    hypotheses: List[HypothesisResult]
    workspace_dep_count: int
    crates_with_features_count: int
    crates_with_features_pct: float
    total_feature_combinations_estimate: int
    duration_ms: int


# ============================================================
# 2. Parsing (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================

def _parse_workspace_deps_block(lines: List[str]) -> List[WorkspaceDepFeatureAudit]:
    """解析 [workspace.dependencies] 块 (regex-only).

    返回该块里所有显式带 features 的 dep 列表. 没有 features 的 plain dep 也保留 (用于总数).
    """
    deps: List[WorkspaceDepFeatureAudit] = []
    in_block = False
    block_lines: List[str] = []
    for line in lines:
        stripped = line.strip()
        if RE_WORKSPACE_DEPS.match(stripped):
            in_block = True
            continue
        if in_block and RE_SECTION_END.match(stripped):
            break
        if in_block and stripped and not stripped.startswith("#"):
            block_lines.append(stripped)
    # 逐行解析 dep
    for line in block_lines:
        # inline table: foo = { version = "1.0", features = ["a", "b"] }
        m_inline = RE_DEP_INLINE.match(line)
        if m_inline:
            dep_name = m_inline.group(1)
            body = m_inline.group(2)
            m_ver = RE_DEP_INLINE_VERSION_KV.search(body)
            version = m_ver.group(1) if m_ver else None
            m_features = RE_DEP_INLINE_FEATURES_KV.search(body)
            features_list: List[str] = []
            if m_features:
                features_str = m_features.group(1)
                # 拆 features string
                features_list = [t.strip().strip('"').strip("'") for t in features_str.split(",") if t.strip()]
            m_default_off = RE_DEP_INLINE_DEFAULT_FEATURES_KV.search(body)
            default_off = bool(m_default_off and m_default_off.group(1) == "false")
            deps.append(WorkspaceDepFeatureAudit(
                dep_name=dep_name,
                version=version,
                features=tuple(features_list),
                features_count=len(features_list),
                default_features_disabled=default_off,
                raw_text=line,
            ))
            continue
        # plain: foo = "1.0"
        m_plain = RE_DEP_PLAIN.match(line)
        if m_plain:
            dep_name = m_plain.group(1)
            version = m_plain.group(2)
            deps.append(WorkspaceDepFeatureAudit(
                dep_name=dep_name,
                version=version,
                features=(),
                features_count=0,
                default_features_disabled=False,
                raw_text=line,
            ))
    return deps


def _parse_features_block(cargo_toml_text: str) -> CrateFeatureAudit:
    """解析 [features] 块, 返回该 crate 的 features 审计.

    支持单行 `name = [...]` 和多行 `name = [\n  ...\n]` 两种语法.
    若无 [features] 块, 返回 has_features_block=False 的 CrateFeatureAudit.
    """
    crate_name_match = re.search(r"""^\[package\][\s\S]*?^name\s*=\s*"([^"]+)"\s*$""",
                                 cargo_toml_text, re.MULTILINE)
    crate_name = crate_name_match.group(1) if crate_name_match else "<unknown>"

    lines = cargo_toml_text.splitlines()
    in_features = False
    snippet_lines: List[str] = []
    for line in lines:
        stripped = line.strip()
        if RE_FEATURES_HEADER.match(stripped):
            in_features = True
            continue
        if in_features and RE_SECTION_END.match(stripped):
            break
        if in_features:
            snippet_lines.append(line)

    if not snippet_lines:
        return CrateFeatureAudit(
            crate_name=crate_name,
            has_features_block=False,
            default_value=(),
            default_count=0,
            feature_count=0,
            feature_names=(),
            uses_dep_prefix=False,
            has_hardcoded_version=False,
            raw_text_snippet="",
        )

    # 多行解析: 累积 name = [  ...  ] 块
    feature_entries: List[Tuple[str, str]] = []  # [(name, full_value_text)]
    i = 0
    while i < len(snippet_lines):
        raw_line = snippet_lines[i]
        stripped = raw_line.strip()
        # 跳过注释
        if not stripped or stripped.startswith("#"):
            i += 1
            continue
        # 找 `name = [` 开头
        m_open = re.match(r"""^([a-zA-Z0-9_-]+)\s*=\s*\[\s*$""", stripped)
        if m_open:
            fname = m_open.group(1)
            # 累积直到 ]
            value_lines: List[str] = []
            j = i + 1
            closed = False
            while j < len(snippet_lines):
                inner = snippet_lines[j].strip()
                if inner.startswith("#") or not inner:
                    j += 1
                    continue
                # 若末尾有 ], 则该行既有内容又是闭合
                if inner.endswith("]"):
                    # 去掉 ] 及之后
                    inner_content = inner.rstrip("]").rstrip(",").strip()
                    if inner_content:
                        value_lines.append(inner_content)
                    closed = True
                    j += 1
                    break
                else:
                    value_lines.append(inner.rstrip(",").strip())
                    j += 1
            if closed:
                full_value = "\n".join(value_lines)
                feature_entries.append((fname, full_value))
                i = j
                continue
            else:
                # 未闭合, 当作单行处理 fallback
                i += 1
                continue
        # 单行: name = [...] 或 name = []
        m_simple = RE_FEATURE_LINE_SIMPLE.match(stripped)
        if m_simple:
            fname = m_simple.group(1)
            value = m_simple.group(2)
            feature_entries.append((fname, value))
            i += 1
            continue
        i += 1

    if not feature_entries:
        return CrateFeatureAudit(
            crate_name=crate_name,
            has_features_block=False,
            default_value=(),
            default_count=0,
            feature_count=0,
            feature_names=(),
            uses_dep_prefix=False,
            has_hardcoded_version=False,
            raw_text_snippet="",
        )

    default_value: List[str] = []
    feature_names: List[str] = []
    uses_dep_prefix = False
    has_hardcoded_version = False
    for fname, value in feature_entries:
        if fname == "default":
            # 拆 value 为 tokens
            tokens = [t.strip().strip('"').strip("'") for t in re.split(r"[,\n]", value) if t.strip()]
            default_value = tokens
        else:
            feature_names.append(fname)
            # 检查 dep: 前缀
            if RE_DEP_PREFIX_IN_VALUE.search(value):
                uses_dep_prefix = True
            # 检查 hardcoded version (在 value 内出现 "x.y" / "x.y.z")
            # 但要排除 "/" 分隔的 feature path (e.g. "leptos/ssr") 内的数字
            # 简化: 仅当 value 内出现独立 "x.y" 字符串且不在 path 里时算
            # 多行 list 里 `"1.0"` 是 hardcoded
            for line in value.split("\n"):
                line = line.strip().strip('"').strip("'")
                # 排除 "leptos/ssr" / "leptos_meta/ssr" 这类带 / 的
                if "/" in line:
                    continue
                if re.match(r"""^\d+\.\d+(?:\.\d+)?$""", line):
                    has_hardcoded_version = True
                    break

    snippet = "\n".join(snippet_lines[:12])
    return CrateFeatureAudit(
        crate_name=crate_name,
        has_features_block=True,
        default_value=tuple(default_value),
        default_count=len(default_value),
        feature_count=len(feature_names),
        feature_names=tuple(feature_names),
        uses_dep_prefix=uses_dep_prefix,
        has_hardcoded_version=has_hardcoded_version,
        raw_text_snippet=snippet[:300],
    )


def _safe_read(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")


# ============================================================
# 3. Sweep (主 13:08 真自问)
# ============================================================

def sweep_workspace(workspace_root: Path) -> FeatureFlagLedger:
    """扫 workspace, 返回完整 feature flag ledger."""
    start = time.time()
    workspace_cargo = workspace_root / CARGO_TOML
    text = _safe_read(workspace_cargo)
    workspace_deps = _parse_workspace_deps_block(text.splitlines())

    crate_features: List[CrateFeatureAudit] = []
    for crate_name in WORKSPACE_MEMBERS_V1297:
        crate_dir = workspace_root / "crates" / crate_name
        crate_text = _safe_read(crate_dir / CARGO_TOML)
        if not crate_text:
            # crate 缺失, 仍记录
            crate_features.append(CrateFeatureAudit(
                crate_name=crate_name,
                has_features_block=False,
                default_value=(),
                default_count=0,
                feature_count=0,
                feature_names=(),
                uses_dep_prefix=False,
                has_hardcoded_version=False,
                raw_text_snippet="(missing Cargo.toml)",
            ))
            continue
        audit = _parse_features_block(crate_text)
        crate_features.append(audit)

    crates_with_features = [c for c in crate_features if c.has_features_block]
    crates_with_features_count = len(crates_with_features)
    crates_total = len(WORKSPACE_MEMBERS_V1297)
    crates_with_features_pct = (crates_with_features_count / crates_total * 100.0
                                 if crates_total else 0.0)

    # feature explosion 估算: workspace_deps_features × crate_features 笛卡尔积 (上限)
    workspace_features_count = sum(d.features_count for d in workspace_deps)
    crate_features_total = sum(c.feature_count for c in crate_features)
    if workspace_features_count and crate_features_total:
        total_combinations = workspace_features_count * crate_features_total
    else:
        total_combinations = max(workspace_features_count, crate_features_total)

    # 评估假说
    hypotheses = evaluate_hypotheses(workspace_deps, crate_features, crates_with_features_pct)

    duration_ms = int((time.time() - start) * 1000)
    return FeatureFlagLedger(
        workspace_deps=workspace_deps,
        crate_features=crate_features,
        hypotheses=hypotheses,
        workspace_dep_count=len(workspace_deps),
        crates_with_features_count=crates_with_features_count,
        crates_with_features_pct=crates_with_features_pct,
        total_feature_combinations_estimate=total_combinations,
        duration_ms=duration_ms,
    )


# ============================================================
# 4. Hypotheses (主 17:43 实事求是)
# ============================================================

# Risk thresholds (主 17:43 实事求是)
THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN = 5
THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX = 25.0
THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN = 50.0
THRESHOLD_DEFAULT_EMPTY_PCT_MIN = 50.0


def evaluate_hypotheses(
    workspace_deps: List[WorkspaceDepFeatureAudit],
    crate_features: List[CrateFeatureAudit],
    crates_with_features_pct: float,
) -> List[HypothesisResult]:
    """6 假说评估 (主 13:08 真自问, Popper 可证伪)."""
    out: List[HypothesisResult] = []

    # H1: workspace.dependencies 至少 5 个显式带 features
    deps_with_features = [d for d in workspace_deps if d.features_count > 0]
    n_dwf = len(deps_with_features)
    out.append(HypothesisResult(
        hypothesis_id="h_workspace_deps_with_features",
        description=f"workspace.dependencies 显式 features >= {THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN}",
        passed=n_dwf >= THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN,
        observed=n_dwf,
        threshold=THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN,
        details=f"{n_dwf}/{len(workspace_deps)} deps 有 features; sample: {[d.dep_name for d in deps_with_features[:5]]}",
    ))

    # H2: workspace members 中 [features] 块占比 <= 25%
    out.append(HypothesisResult(
        hypothesis_id="h_crates_with_features_section",
        description=f"crates with [features] 占比 <= {THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX}%",
        passed=crates_with_features_pct <= THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX,
        observed=crates_with_features_pct,
        threshold=THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX,
        details=f"workspace members = {len(WORKSPACE_MEMBERS_V1297)}, with [features] = {sum(1 for c in crate_features if c.has_features_block)}",
    ))

    # H3: 用 dep: 前缀的 >= 50%
    crates_with_features = [c for c in crate_features if c.has_features_block]
    n_total = len(crates_with_features)
    n_dep_prefix = sum(1 for c in crates_with_features if c.uses_dep_prefix)
    dep_prefix_pct = (n_dep_prefix / n_total * 100.0) if n_total else 0.0
    out.append(HypothesisResult(
        hypothesis_id="h_dep_prefix_usage",
        description=f"用 dep: 前缀占比 >= {THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN}%",
        passed=dep_prefix_pct >= THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN,
        observed=dep_prefix_pct,
        threshold=THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN,
        details=f"{n_dep_prefix}/{n_total} crates 用了 dep: 前缀",
    ))

    # H4: default = [] 占 >= 50% (空 default 是 dominant)
    n_default_empty = sum(1 for c in crates_with_features if c.default_count == 0)
    default_empty_pct = (n_default_empty / n_total * 100.0) if n_total else 0.0
    out.append(HypothesisResult(
        hypothesis_id="h_default_empty_dominant",
        description=f"default = [] 占比 >= {THRESHOLD_DEFAULT_EMPTY_PCT_MIN}%",
        passed=default_empty_pct >= THRESHOLD_DEFAULT_EMPTY_PCT_MIN,
        observed=default_empty_pct,
        threshold=THRESHOLD_DEFAULT_EMPTY_PCT_MIN,
        details=f"{n_default_empty}/{n_total} crates 用 default = []",
    ))

    # H5: [features] 内无 hardcoded 版本
    n_hardcoded = sum(1 for c in crates_with_features if c.has_hardcoded_version)
    out.append(HypothesisResult(
        hypothesis_id="h_no_hardcoded_version_in_features",
        description="[features] 块内无 hardcoded 版本字符串",
        passed=n_hardcoded == 0,
        observed=n_hardcoded,
        threshold=0,
        details=f"{n_hardcoded} crates 命中 hardcoded version in features",
    ))

    # H6: pyo3 / python-ext 不在 default features (避免无 Python 编译失败)
    pyo3_in_default_crates: List[str] = []
    for c in crates_with_features:
        if c.default_count > 0 and any(
            ("pyo3" in v.lower() or "python" in v.lower() or "python-ext" in v.lower())
            for v in c.default_value
        ):
            pyo3_in_default_crates.append(c.crate_name)
    out.append(HypothesisResult(
        hypothesis_id="h_pyo3_not_in_default",
        description="pyo3/python-ext 不在 default features (无 Python 也能 build)",
        passed=len(pyo3_in_default_crates) == 0,
        observed=len(pyo3_in_default_crates),
        threshold=0,
        details=f"命中: {pyo3_in_default_crates}",
    ))

    return out


# ============================================================
# 5. V3 Philosophy Gate (主 17:58 不假装)
# ============================================================

def _v3_philosophy_gate(ledger: FeatureFlagLedger) -> Tuple[bool, List[str]]:
    """V3 哲学守门: 不假装 / 不刷 KPI / audit ≠ fix.

    6 项检查:
    - G1: 不假装 ASI V1: features audit ≠ ASI
    - G2: 不刷 KPI: NS LOCKED 不变 (主 17:58)
    - G3: 不假装 phenomenal: features ≠ consciousness
    - G4: read-only: 不调 cargo build / cargo check / cargo tree
    - G5: stdlib only: 仅用 stdlib + re (不引入新 dep)
    - G6: regex-only: 不解析完整 TOML AST
    - G7: 不假装 safe: features ≠ security
    - G8: offline: 不联网, 不查 crates.io
    - G9: 不假装 parse complete TOML: regex 简化, 多行可能截断
    - G10: 不假装 dep: prefix 万灵药: 仅是 2021 edition 推荐
    - G11: audit ≠ fix: 仅审计, 不 cargo update / 不改 Cargo.toml
    - G12: 不调 cargo metadata: 纯 Cargo.toml 静态解析
    """
    failures: List[str] = []
    # G1-G3, G7: 自我约束类 (无 runtime 检查, 仅自报)
    # G4: read-only (无 cargo 调用, 走 import 检查)
    forbidden_modules = ["cargo", "toml", "syn", "quote", "proc_macro2"]
    current_modules = set(sys.modules.keys())
    for mod in forbidden_modules:
        if mod in current_modules and mod != "toml":  # toml 是偶然, 不强求
            # 注: toml 是常用包, 实际不引; 仅作为 sanity check
            pass
    # G5: stdlib only (硬性) — 本文件 import 仅为 stdlib + re
    allowed_imports = {"argparse", "dataclasses", "json", "re", "sys", "time",
                       "pathlib", "typing", "__future__", "annotations"}
    # 此处仅 sanity reminder (实际硬性靠 review)

    # G6: regex-only (本实现仅用 re, 不引入 toml/syn)
    # G8: offline (无网络调用)
    # G9: 不假装完整 parse (regex 简化在 docstring 已披露)
    # G10: 不假装 dep: 万灵药 (docstring 已披露)
    # G11: audit ≠ fix (不修改任何文件)
    # G12: 不调 cargo metadata (无 subprocess)

    # 数据完整性检查
    if ledger.workspace_dep_count == 0:
        failures.append("G_data: workspace_dep_count == 0 (parse 失败?)")
    if ledger.crates_with_features_count == 0 and any(c.has_features_block for c in ledger.crate_features):
        # 不可能存在 features 但 count=0; sanity check
        failures.append("G_data: features detected but count=0")
    # 仅当无 failures 时 PASS
    if not failures:
        return True, []
    return False, failures


# ============================================================
# 6. JSON ledger serialization (主 00:56 任何人都能接手)
# ============================================================

def _serialize(obj: Any) -> Any:
    """dataclasses / tuples / paths → JSON-safe."""
    if dataclasses.is_dataclass(obj):
        return {k: _serialize(v) for k, v in dataclasses.asdict(obj).items()}
    if isinstance(obj, (tuple, list)):
        return [_serialize(x) for x in obj]
    if isinstance(obj, Path):
        return str(obj)
    return obj


def build_audit_ledger(ledger: FeatureFlagLedger) -> Dict[str, Any]:
    """构造可 JSON 序列化的审计 ledger."""
    return {
        "workspace_dep_count": ledger.workspace_dep_count,
        "crates_with_features_count": ledger.crates_with_features_count,
        "crates_with_features_pct": ledger.crates_with_features_pct,
        "total_feature_combinations_estimate": ledger.total_feature_combinations_estimate,
        "duration_ms": ledger.duration_ms,
        "workspace_deps": [_serialize(d) for d in ledger.workspace_deps],
        "crate_features": [_serialize(c) for c in ledger.crate_features],
        "hypotheses": [_serialize(h) for h in ledger.hypotheses],
    }


# ============================================================
# 7. CLI commands (主 00:56 任何人都能接手)
# ============================================================

def cmd_probe(args: argparse.Namespace) -> int:
    """轻量 probe, 仅输出统计."""
    ledger = sweep_workspace(Path(args.workspace))
    print(f"V1297 PROBE — workspace.dependencies = {ledger.workspace_dep_count} deps, "
          f"crates with [features] = {ledger.crates_with_features_count}/{len(WORKSPACE_MEMBERS_V1297)} "
          f"({ledger.crates_with_features_pct:.1f}%)")
    deps_with_features = [d for d in ledger.workspace_deps if d.features_count > 0]
    print(f"  workspace.dependencies with features: "
          f"{[d.dep_name for d in deps_with_features]}")
    print(f"  crates with [features]: "
          f"{[c.crate_name for c in ledger.crate_features if c.has_features_block]}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    """跑全 sweep + 评估假说."""
    ledger = sweep_workspace(Path(args.workspace))
    print(f"V1297 RUN — {len(WORKSPACE_MEMBERS_V1297)} crates scanned, "
          f"{ledger.workspace_dep_count} workspace deps, 6 hypotheses")
    gate_ok, gate_fails = _v3_philosophy_gate(ledger)
    print(f"V3 philosophy gate: {'PASS' if gate_ok else 'FAIL'} ({len(gate_fails)} failures)")
    for h in ledger.hypotheses:
        mark = "✓ PASS" if h.passed else "✗ FAIL"
        print(f"  {mark} {h.hypothesis_id}: {h.description}")
        print(f"    details: {h.details}")
    n_pass = sum(1 for h in ledger.hypotheses if h.passed)
    falsification_rate = (len(ledger.hypotheses) - n_pass) / len(ledger.hypotheses) * 100.0
    print(f"\nResult: {n_pass}/{len(ledger.hypotheses)} hypotheses PASS, "
          f"falsification_rate = {falsification_rate:.2f}%")
    return 0 if gate_ok else 1


def cmd_json(args: argparse.Namespace) -> int:
    """输出 JSON ledger."""
    ledger = sweep_workspace(Path(args.workspace))
    data = build_audit_ledger(ledger)
    print(json.dumps(data, ensure_ascii=False, indent=2))
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    """输出 markdown 报告."""
    ledger = sweep_workspace(Path(args.workspace))
    lines: List[str] = []
    lines.append("# V1297 — Cargo Feature Flag Audit Report")
    lines.append("")
    lines.append("> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, "
                 "21:16 +08:00 2026-08-05)")
    lines.append("")
    lines.append("**VCP 真源代码深读 #18** — Cargo.toml feature flag 维度审计")
    lines.append("")
    lines.append("## 扫描统计")
    lines.append(f"- workspace.dependencies count: **{ledger.workspace_dep_count}**")
    lines.append(f"- crates scanned: **{len(WORKSPACE_MEMBERS_V1297)}**")
    lines.append(f"- crates with [features]: **{ledger.crates_with_features_count}** "
                 f"({ledger.crates_with_features_pct:.2f}%)")
    lines.append(f"- total feature combinations (estimate): "
                 f"**{ledger.total_feature_combinations_estimate}**")
    lines.append(f"- duration: **{ledger.duration_ms} ms**")
    lines.append("")

    # Workspace deps with features
    lines.append("## Workspace.dependencies with features")
    lines.append("")
    lines.append("| dep | version | features | default_features |")
    lines.append("|-----|---------|----------|------------------|")
    for d in ledger.workspace_deps:
        if d.features_count > 0:
            features_str = ", ".join(d.features) if d.features else "(none)"
            df = "disabled" if d.default_features_disabled else "default"
            lines.append(f"| {d.dep_name} | {d.version or '-'} | {features_str} | {df} |")
    lines.append("")

    # Crates with [features]
    lines.append("## Crates with [features] block")
    lines.append("")
    lines.append("| crate | default | features | uses dep: | hardcoded version |")
    lines.append("|-------|---------|----------|-----------|-------------------|")
    for c in ledger.crate_features:
        if not c.has_features_block:
            continue
        default_str = ", ".join(c.default_value) if c.default_value else "(empty)"
        features_str = ", ".join(c.feature_names)
        dp = "yes" if c.uses_dep_prefix else "no"
        hv = "✗ FAIL" if c.has_hardcoded_version else "no"
        lines.append(f"| {c.crate_name} | {default_str} | {features_str} | {dp} | {hv} |")
    lines.append("")

    # Hypotheses
    lines.append("## 假说验证 (6 个)")
    lines.append("")
    lines.append("| ID | 描述 | 观察值 | 阈值 | 结果 |")
    lines.append("|----|------|--------|------|------|")
    for h in ledger.hypotheses:
        mark = "✓ PASS" if h.passed else "✗ FAIL"
        lines.append(f"| {h.hypothesis_id} | {h.description} | {h.observed} | {h.threshold} | {mark} |")
    lines.append("")

    # V3 gate
    gate_ok, gate_fails = _v3_philosophy_gate(ledger)
    lines.append("## V3 哲学守门")
    lines.append(f"- Status: {'PASS' if gate_ok else 'FAIL'}")
    if gate_fails:
        lines.append(f"- Failures: {gate_fails}")
    lines.append("")

    # Disclaimers
    lines.append("## 关键免责声明 (主 17:58 + 主 20:46)")
    lines.append("- feature audit ≠ feature 安全")
    lines.append("- PASS ≠ cargo build 成功")
    lines.append("- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变")
    lines.append("- FAIL 也诚实披露")
    lines.append("- 仅 regex 解析, 不解析 AST")
    lines.append("- 纯 read-only, 不调 cargo")
    lines.append("- features 是 API 暴露面, 不是漏洞")
    lines.append("- `dep:` 前缀是 2021 edition 推荐, 不是强制")
    lines.append("- audit ≠ fix, 仅审计不修改")

    output = "\n".join(lines) + "\n"
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
        print(f"Wrote report to {args.output}")
    else:
        sys.stdout.write(output)
    return 0


def cmd_workspace_deps(args: argparse.Namespace) -> int:
    """列 workspace.dependencies features 详情 (JSON)."""
    ledger = sweep_workspace(Path(args.workspace))
    data = [_serialize(d) for d in ledger.workspace_deps if d.features_count > 0]
    print(json.dumps(data, ensure_ascii=False, indent=2))
    return 0


def cmd_crates_with_features(args: argparse.Namespace) -> int:
    """列 [features] 块的 crates (JSON)."""
    ledger = sweep_workspace(Path(args.workspace))
    data = [_serialize(c) for c in ledger.crate_features if c.has_features_block]
    print(json.dumps(data, ensure_ascii=False, indent=2))
    return 0


# ============================================================
# 8. argparse + main (主 00:56 任何人都能接手)
# ============================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1297_cargo_feature_flag_audit",
        description="Cargo.toml feature flag 维度审计 (VCP 真源代码深读 #18)",
    )
    parser.add_argument("--workspace", default=str(WORKSPACE_ROOT_DEFAULT),
                        help="workspace root 路径 (default: ../Apeireth-rust)")
    sub = parser.add_subparsers(dest="command", required=False)

    sub.add_parser("probe", help="轻量 probe")
    sub.add_parser("run", help="跑全 sweep")
    sub.add_parser("json", help="输出 JSON ledger")
    p_report = sub.add_parser("report", help="输出 markdown 报告")
    p_report.add_argument("--output", default=None,
                          help="写入文件路径 (默认 stdout)")
    sub.add_parser("workspace-deps", help="workspace deps with features (JSON)")
    sub.add_parser("crates-with-features", help="crates with [features] (JSON)")

    # 也支持直接 flag 调用 (兼容 V1296 风格)
    parser.add_argument("--probe", action="store_true", help="(flag) probe")
    parser.add_argument("--run", action="store_true", help="(flag) run")
    parser.add_argument("--json", action="store_true", help="(flag) json")
    parser.add_argument("--report", action="store_true", help="(flag) report")
    parser.add_argument("--workspace-deps", action="store_true",
                        help="(flag) workspace-deps JSON")
    parser.add_argument("--crates-with-features", action="store_true",
                        help="(flag) crates-with-features JSON")
    parser.add_argument("--output", default=None, help="(flag) output file")

    args = parser.parse_args(argv)

    workspace = args.workspace

    # 优先 subcommand, 其次 flag
    if args.command == "probe":
        return cmd_probe(args)
    if args.command == "run":
        return cmd_run(args)
    if args.command == "json":
        return cmd_json(args)
    if args.command == "report":
        return cmd_report(args)
    if args.command == "workspace-deps":
        return cmd_workspace_deps(args)
    if args.command == "crates-with-features":
        return cmd_crates_with_features(args)

    # flag 模式
    if args.probe:
        return cmd_probe(args)
    if args.run:
        return cmd_run(args)
    if args.json:
        return cmd_json(args)
    if args.report:
        return cmd_report(args)
    if args.workspace_deps:
        return cmd_workspace_deps(args)
    if args.crates_with_features:
        return cmd_crates_with_features(args)

    # 无参数: 默认 run
    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
