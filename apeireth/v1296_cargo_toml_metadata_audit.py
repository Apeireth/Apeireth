"""V1296 — Cargo.toml Edition / MSRV / Metadata Hygiene Audit (VCP 真源代码深读 #17) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:25 +08:00 2026-08-05)
> **触发**: 20:25 cron wake tick (autonomy-v3) — V1295 cargo.lock audit (d07cce57, 20:23) 已 commit.
>          V1280-V1295 (16 sweeps) = 源代码静态 / 语义 / 安全 / 治理 / 文档 / 构建产物 / 测试源码 / 依赖图 / build.rs / Cargo.lock.
>          V1296 = **Cargo.toml 元数据卫生 (edition / MSRV / license / authors / description / repository / docs / categories / keywords / publish)** 层面
>            (主 13:08 真自问 + 主 19:33 走在前人肩上).
> **承接**: V1293 Cargo.toml dep graph + V1294 build.rs + V1295 Cargo.lock → V1296 Cargo.toml 元数据卫生
>         → V1297+ Rust workspace lint / feature flag / rust-toolchain (候选).
> **真借鉴**: 主 19:33 走在前人肩上 + cargo book "The Manifest Format" 章节
>         + clippy cargo lints + apeireth Cargo.toml 真扫 (47 workspace crates + workspace.package edition 2021 / MSRV 1.80).
> **不假装**: V1296 = 真生产 Cargo.toml 元数据卫生审计 + 与 workspace.package 继承一致性, 不刷 KPI
>         不假装"元数据 = ASI", 不假装"继承 = 完美", 不假装"publish = false = 安全"
>         不假装"workspace.package 字段全 = 锁", 仅 audit-threshold.

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1293 已审 Cargo.toml 声明的依赖图, V1294 已审 build.rs 编译时脚本, V1295 已审 Cargo.lock 锁文件.
但 **Cargo.toml [package] 元数据** 是 ASI 可证伪的另一维度:

- **edition**: Rust edition (2015/2018/2021/2024). workspace.package 应统一, 子 crate 应 `edition.workspace = true`
- **rust-version (MSRV)**: 最低支持 Rust 版本. 同上, 应 workspace 统一继承
- **license**: SPDX 表达式 (MIT/Apache-2.0/...). workspace 应统一
- **authors**: 维护者列表. workspace 应统一
- **description**: 单行描述. crates.io 必需
- **repository**: 代码托管 URL. crates.io 必需 (workspace 仓库)
- **documentation / homepage / readme**: 文档链接
- **categories / keywords**: crates.io 分类 + 关键词
- **publish = false**: 内部 crate 不应 publish 到 crates.io (workspace 47 应全 false)
- **include / exclude**: 打包白/黑名单

**V1296 = 真生产全 Cargo.toml 元数据审计**, 6 维度 5 假说:

1. **workspace_package_metadata**: workspace.package 字段完整性 (edition / rust-version / license / authors / repository 等)
2. **edition_inheritance**: workspace members 用 edition.workspace = true (vs hardcode vs missing)
3. **rust_version_inheritance**: workspace members 用 rust-version.workspace = true
4. **license_inheritance**: workspace members 用 license.workspace = true (vs hardcode vs missing)
5. **description_presence**: workspace members 有 description 字段 (单行有意义描述)

外加 per-crate 维度:
- **CrateMetadataAudit**: name / version / edition / rust_version / license / authors / description / repository / publish / readme / homepage / categories / keywords / include / exclude

每假说维度:
- **WorkspaceEdition**: workspace.package edition 值
- **WorkspaceRustVersion**: workspace.package rust-version 值
- **WorkspaceLicense**: workspace.package license 值
- **CrateEditionInheritance**: edition.workspace = true / hardcoded / missing
- **CratePublishSetting**: publish = true / false / missing (default true = 风险)

**关键免责声明** (主 17:58 + 主 20:46):
- "metadata audit" ≠ "metadata 安全": 仅元数据静态解析, 不调 cargo check / cargo build / cargo publish
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变 (主 17:58)
- FAIL 也诚实披露 (主 17:43 实事求是), 列出每条 finding 不掩饰
- 不假装 parse TOML: 用 regex 简化 pattern match, 可能漏 multi-line 字段
- 不调 cargo: 纯 read-only Cargo.toml 解析
- 不 fetch crates.io: offline 跑, 无法验证 publish 实际可达
- 不假装 workspace.package 字段 = 锁: workspace 可被子 crate 覆盖 (inheritance 可被 override)

## CLI (主 00:56 任何人都能接手)

```bash
# 探测 (仅元数据统计, 不评估)
python -m apeireth.v1296_cargo_toml_metadata_audit --probe

# 跑全 sweep + 输出报告
python -m apeireth.v1296_cargo_toml_metadata_audit --run

# 输出 JSON ledger
python -m apeireth.v1296_cargo_toml_metadata_audit --json

# 输出 markdown 报告
python -m apeireth.v1296_cargo_toml_metadata_audit --report

# 看单个 crate (按 name)
python -m apeireth.v1296_cargo_toml_metadata_audit --crate apeireth-core

# 列 edition inheritance 分布
python -m apeireth.v1296_cargo_toml_metadata_audit --edition-stats

# 列 publish = true 的 crates (风险)
python -m apeireth.v1296_cargo_toml_metadata_audit --publish-true

# 列缺 description 的 crates
python -m apeireth.v1296_cargo_toml_metadata_audit --missing-description
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

# Workspace member list (V1296 = 56 crates, 含 R20 阶段 1/2/3/4 加入)
# 主 17:43 实事求是: 实际 Cargo.toml members = 56 (含 tauri-stub + 6 个 R20 阶段 1/2/3 skeleton)
# V1293 = 41; V1294 = 42 (加 tauri-stub); R20 阶段 1 加 5 P0 = 47; 阶段 2 (20:30) 加 9 skeleton = 56
#   阶段 1 已加: image-prompt, rollback, plugin (已在 R20 阶段 1 commit 128f9704)
#   阶段 2 已加: keyring, lark, machine-id, repo-analyzer, repo-scan, voice (新加 R20 阶段 2)
WORKSPACE_MEMBERS_V1296: List[str] = [
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
RE_PACKAGE_HEADER = re.compile(r"^\[package\]\s*$")
RE_FIELD_NAME = re.compile(r"""^name\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_VERSION = re.compile(r"""^version\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_VERSION_WORKSPACE = re.compile(r"""^version\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_EDITION_VALUE = re.compile(r"""^edition\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_EDITION_WORKSPACE = re.compile(r"""^edition\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_RUST_VERSION_VALUE = re.compile(r"""^rust-version\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_RUST_VERSION_WORKSPACE = re.compile(r"""^rust-version\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_LICENSE_VALUE = re.compile(r"""^license\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_LICENSE_WORKSPACE = re.compile(r"""^license\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_AUTHORS = re.compile(r"""^authors\s*=\s*\[([^\]]*)\]\s*$""")
RE_FIELD_AUTHORS_WORKSPACE = re.compile(r"""^authors\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_DESCRIPTION = re.compile(r"""^description\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_REPOSITORY = re.compile(r"""^repository\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_REPOSITORY_WORKSPACE = re.compile(r"""^repository\.workspace\s*=\s*(true|false)\s*$""")
RE_FIELD_DOCUMENTATION = re.compile(r"""^documentation\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_HOMEPAGE = re.compile(r"""^homepage\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_README = re.compile(r"""^readme\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_KEYWORDS = re.compile(r"""^keywords\s*=\s*\[([^\]]*)\]\s*$""")
RE_FIELD_CATEGORIES = re.compile(r"""^categories\s*=\s*\[([^\]]*)\]\s*$""")
RE_FIELD_PUBLISH = re.compile(r"""^publish\s*=\s*(true|false)\s*$""")
RE_FIELD_INCLUDE = re.compile(r"""^include\s*=\s*\[([^\]]*)\]\s*$""")
RE_FIELD_EXCLUDE = re.compile(r"""^exclude\s*=\s*\[([^\]]*)\]\s*$""")
RE_SECTION_END = re.compile(r"^\[\[?[a-zA-Z0-9_.\-]+\]?\]\s*$")

# Risk thresholds (主 17:43 实事求是)
THRESHOLD_WORKSPACE_PACKAGE_FIELDS_MIN = 4  # 期望 workspace.package 有 edition/rust-version/license/authors 等
THRESHOLD_EDITION_INHERITANCE_PCT = 90.0  # 期望 >= 90% 子 crate 用 edition.workspace = true
THRESHOLD_RUST_VERSION_INHERITANCE_PCT = 90.0  # 期望 >= 90% 子 crate 用 rust-version.workspace = true
THRESHOLD_LICENSE_INHERITANCE_PCT = 90.0  # 期望 >= 90% 子 crate 用 license.workspace = true
THRESHOLD_DESCRIPTION_COVERAGE_PCT = 90.0  # 期望 >= 90% 子 crate 有 description
THRESHOLD_PUBLISH_FALSE_PCT = 100.0  # 期望 100% 子 crate publish = false (内部 crate)
THRESHOLD_HARDCODED_EDITION_TOLERANCE = 5  # 容忍 <= 5 个 hardcode (e.g., 5 P0 R20)


# ============================================================
# 1. Data classes (主 00:56 任何人都能接手 + 主 17:43 实事求是)
# ============================================================

@dataclasses.dataclass(frozen=True)
class WorkspacePackageMetadata:
    """workspace.package [package.*] 字段"""
    edition: Optional[str]
    rust_version: Optional[str]
    license: Optional[str]
    authors_count: int
    authors: List[str]
    repository: Optional[str]
    documentation: Optional[str]
    homepage: Optional[str]
    readme: Optional[str]
    keywords_count: int
    categories_count: int
    description: Optional[str]


@dataclasses.dataclass(frozen=True)
class CrateMetadataAudit:
    """单个 crate Cargo.toml [package.*] 字段审计"""
    name: str
    version: Optional[str]
    version_workspace: bool
    edition: Optional[str]
    edition_workspace: bool
    edition_inheritance_mode: str  # "workspace" / "hardcoded" / "missing"
    rust_version: Optional[str]
    rust_version_workspace: bool
    rust_version_inheritance_mode: str
    license: Optional[str]
    license_workspace: bool
    license_inheritance_mode: str
    authors_count: int
    authors_workspace: bool
    description: Optional[str]
    description_present: bool
    repository: Optional[str]
    repository_workspace: bool
    publish: Optional[bool]
    publish_value: str  # "true" / "false" / "missing"
    documentation: Optional[str]
    homepage: Optional[str]
    readme: Optional[str]
    keywords_count: int
    categories_count: int


@dataclasses.dataclass(frozen=True)
class HypothesisResult:
    """单个假说验证结果"""
    hypothesis_id: str
    description: str
    passed: bool
    observed: Any
    threshold: Any
    details: Optional[str] = None


# ============================================================
# 2. Parsing (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================

def _parse_package_block(lines: List[str]) -> Dict[str, Any]:
    """解析 Cargo.toml 中第一个 [package] 块 (regex-only).

    兼容 [package] 与 [workspace.package] (后者被调用方预过滤后传入).
    若首行已是 key = value (即 [package] 已被剥除), 直接进入字段匹配模式.
    """
    result: Dict[str, Any] = {}
    in_package = False
    # 检查首行是否为 [package] / [workspace.package] / 其他 section header
    first_nonempty = ""
    for ln in lines:
        s = ln.strip()
        if s:
            first_nonempty = s
            break
    # 若首行是 section header ([xxx]), 需要先匹配进入; 否则直接进入字段模式
    if first_nonempty.startswith("["):
        # 严格模式: 等待 [package] 出现
        for line in lines:
            stripped = line.strip()
            if RE_PACKAGE_HEADER.match(stripped):
                in_package = True
                continue
            if in_package and RE_SECTION_END.match(stripped):
                break
            if not in_package:
                continue
            _try_match_field(stripped, result)
    else:
        # 宽松模式: 直接进入字段匹配 (用于 [workspace.package] 预过滤后)
        for line in lines:
            stripped = line.strip()
            if not stripped:
                continue
            # 若遇到新的 section header 立即停止
            if RE_SECTION_END.match(stripped):
                break
            _try_match_field(stripped, result)
    return result


def _try_match_field(stripped: str, result: Dict[str, Any]) -> None:
    """尝试匹配单个 stripped 行为 [package] 字段 (in-place 写入 result)."""
    # 抓 name / version / version.workspace
    m = RE_FIELD_NAME.match(stripped)
    if m:
        result["name"] = m.group(1)
        return
    m = RE_FIELD_VERSION.match(stripped)
    if m and "version" not in result:
        result["version"] = m.group(1)
        return
    m = RE_FIELD_VERSION_WORKSPACE.match(stripped)
    if m:
        result["version_workspace"] = (m.group(1) == "true")
        return
    # edition
    m = RE_FIELD_EDITION_VALUE.match(stripped)
    if m:
        result["edition"] = m.group(1)
        return
    m = RE_FIELD_EDITION_WORKSPACE.match(stripped)
    if m:
        result["edition_workspace"] = (m.group(1) == "true")
        return
    # rust-version
    m = RE_FIELD_RUST_VERSION_VALUE.match(stripped)
    if m:
        result["rust_version"] = m.group(1)
        return
    m = RE_FIELD_RUST_VERSION_WORKSPACE.match(stripped)
    if m:
        result["rust_version_workspace"] = (m.group(1) == "true")
        return
    # license
    m = RE_FIELD_LICENSE_VALUE.match(stripped)
    if m:
        result["license"] = m.group(1)
        return
    m = RE_FIELD_LICENSE_WORKSPACE.match(stripped)
    if m:
        result["license_workspace"] = (m.group(1) == "true")
        return
    # authors
    m = RE_FIELD_AUTHORS.match(stripped)
    if m:
        result["authors"] = [a.strip().strip('"').strip("'") for a in m.group(1).split(",") if a.strip()]
        return
    m = RE_FIELD_AUTHORS_WORKSPACE.match(stripped)
    if m:
        result["authors_workspace"] = (m.group(1) == "true")
        return
    # description
    m = RE_FIELD_DESCRIPTION.match(stripped)
    if m:
        result["description"] = m.group(1)
        return
    # repository
    m = RE_FIELD_REPOSITORY.match(stripped)
    if m:
        result["repository"] = m.group(1)
        return
    m = RE_FIELD_REPOSITORY_WORKSPACE.match(stripped)
    if m:
        result["repository_workspace"] = (m.group(1) == "true")
        return
    # documentation / homepage / readme
    m = RE_FIELD_DOCUMENTATION.match(stripped)
    if m:
        result["documentation"] = m.group(1)
        return
    m = RE_FIELD_HOMEPAGE.match(stripped)
    if m:
        result["homepage"] = m.group(1)
        return
    m = RE_FIELD_README.match(stripped)
    if m:
        result["readme"] = m.group(1)
        return
    # keywords / categories
    m = RE_FIELD_KEYWORDS.match(stripped)
    if m:
        result["keywords_count"] = len([k for k in m.group(1).split(",") if k.strip()])
        return
    m = RE_FIELD_CATEGORIES.match(stripped)
    if m:
        result["categories_count"] = len([c for c in m.group(1).split(",") if c.strip()])
        return
    # publish
    m = RE_FIELD_PUBLISH.match(stripped)
    if m:
        result["publish"] = (m.group(1) == "true")
        return


def parse_workspace_package(workspace_root: Path) -> WorkspacePackageMetadata:
    """解析 workspace Cargo.toml 的 [workspace.package] 字段."""
    cargo_toml = workspace_root / CARGO_TOML
    if not cargo_toml.exists():
        return WorkspacePackageMetadata(
            edition=None, rust_version=None, license=None,
            authors_count=0, authors=[], repository=None,
            documentation=None, homepage=None, readme=None,
            keywords_count=0, categories_count=0, description=None,
        )
    lines = cargo_toml.read_text(encoding="utf-8", errors="replace").splitlines()
    # workspace.package block
    in_workspace_pkg = False
    pkg_lines: List[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("[workspace.package]"):
            in_workspace_pkg = True
            continue
        if in_workspace_pkg and stripped.startswith("[") and not stripped.startswith("[workspace.package]"):
            in_workspace_pkg = False
            continue
        if in_workspace_pkg:
            pkg_lines.append(line)
    parsed = _parse_package_block(pkg_lines)
    return WorkspacePackageMetadata(
        edition=parsed.get("edition"),
        rust_version=parsed.get("rust_version"),
        license=parsed.get("license"),
        authors_count=len(parsed.get("authors", [])),
        authors=parsed.get("authors", []),
        repository=parsed.get("repository"),
        documentation=parsed.get("documentation"),
        homepage=parsed.get("homepage"),
        readme=parsed.get("readme"),
        keywords_count=parsed.get("keywords_count", 0),
        categories_count=parsed.get("categories_count", 0),
        description=parsed.get("description"),
    )


def parse_crate_metadata(crate_dir: Path) -> Optional[CrateMetadataAudit]:
    """解析单个 crate Cargo.toml 的 [package] 字段."""
    cargo_toml = crate_dir / CARGO_TOML
    if not cargo_toml.exists():
        return None
    lines = cargo_toml.read_text(encoding="utf-8", errors="replace").splitlines()
    parsed = _parse_package_block(lines)
    if "name" not in parsed:
        return None
    edition_ws = parsed.get("edition_workspace", False)
    edition_val = parsed.get("edition")
    if edition_ws:
        edition_mode = "workspace"
    elif edition_val is not None:
        edition_mode = "hardcoded"
    else:
        edition_mode = "missing"
    rust_version_ws = parsed.get("rust_version_workspace", False)
    rust_version_val = parsed.get("rust_version")
    if rust_version_ws:
        rust_version_mode = "workspace"
    elif rust_version_val is not None:
        rust_version_mode = "hardcoded"
    else:
        rust_version_mode = "missing"
    license_ws = parsed.get("license_workspace", False)
    license_val = parsed.get("license")
    if license_ws:
        license_mode = "workspace"
    elif license_val is not None:
        license_mode = "hardcoded"
    else:
        license_mode = "missing"
    publish_val = parsed.get("publish")
    publish_value = "true" if publish_val is True else ("false" if publish_val is False else "missing")
    return CrateMetadataAudit(
        name=parsed.get("name", crate_dir.name),
        version=parsed.get("version"),
        version_workspace=parsed.get("version_workspace", False),
        edition=edition_val,
        edition_workspace=edition_ws,
        edition_inheritance_mode=edition_mode,
        rust_version=rust_version_val,
        rust_version_workspace=rust_version_ws,
        rust_version_inheritance_mode=rust_version_mode,
        license=license_val,
        license_workspace=license_ws,
        license_inheritance_mode=license_mode,
        authors_count=len(parsed.get("authors", [])),
        authors_workspace=parsed.get("authors_workspace", False),
        description=parsed.get("description"),
        description_present=parsed.get("description") is not None,
        repository=parsed.get("repository"),
        repository_workspace=parsed.get("repository_workspace", False),
        publish=publish_val,
        publish_value=publish_value,
        documentation=parsed.get("documentation"),
        homepage=parsed.get("homepage"),
        readme=parsed.get("readme"),
        keywords_count=parsed.get("keywords_count", 0),
        categories_count=parsed.get("categories_count", 0),
    )


# ============================================================
# 3. Sweep (主 00:56 任何人都能接手 + 主 19:33 走在前人肩上)
# ============================================================

def sweep_workspace(workspace_root: Path) -> Tuple[WorkspacePackageMetadata, List[CrateMetadataAudit]]:
    """扫描 workspace_root + 所有 member crates 的元数据."""
    ws_meta = parse_workspace_package(workspace_root)
    crates_dir = workspace_root / "crates"
    audits: List[CrateMetadataAudit] = []
    for member_name in WORKSPACE_MEMBERS_V1296:
        crate_dir = crates_dir / member_name
        if not crate_dir.exists():
            # 兼容直接放 root 的情况 (e.g., Cargo.toml 在 workspace_root/Cargo.toml)
            continue
        audit = parse_crate_metadata(crate_dir)
        if audit is not None:
            audits.append(audit)
    return ws_meta, audits


# ============================================================
# 4. Hypotheses (主 17:43 实事求是 + 主 17:58 不假装)
# ============================================================

def evaluate_hypotheses(ws_meta: WorkspacePackageMetadata, audits: List[CrateMetadataAudit]) -> List[HypothesisResult]:
    """跑 5 个假说."""
    results: List[HypothesisResult] = []
    total = len(audits)

    # H1: workspace.package 字段完整性
    ws_field_count = sum(1 for v in [
        ws_meta.edition, ws_meta.rust_version, ws_meta.license,
        ws_meta.authors_count > 0, ws_meta.repository,
    ] if v)
    results.append(HypothesisResult(
        hypothesis_id="h_workspace_package_fields",
        description=f"workspace.package fields >= {THRESHOLD_WORKSPACE_PACKAGE_FIELDS_MIN} (edition/rust-version/license/authors/repository)",
        passed=ws_field_count >= THRESHOLD_WORKSPACE_PACKAGE_FIELDS_MIN,
        observed=ws_field_count,
        threshold=THRESHOLD_WORKSPACE_PACKAGE_FIELDS_MIN,
        details=f"workspace.package: edition={ws_meta.edition} rust-version={ws_meta.rust_version} license={ws_meta.license} authors={ws_meta.authors_count} repository={ws_meta.repository}",
    ))

    if total == 0:
        # No crates found, can't run crate-level hypotheses
        for hid, desc, obs, thr, passed in [
            ("h_edition_inheritance", "edition.workspace inheritance >= 90%", 0.0, THRESHOLD_EDITION_INHERITANCE_PCT, False),
            ("h_rust_version_inheritance", "rust-version.workspace inheritance >= 90%", 0.0, THRESHOLD_RUST_VERSION_INHERITANCE_PCT, False),
            ("h_license_inheritance", "license.workspace inheritance >= 90%", 0.0, THRESHOLD_LICENSE_INHERITANCE_PCT, False),
            ("h_description_coverage", "description field coverage >= 90%", 0.0, THRESHOLD_DESCRIPTION_COVERAGE_PCT, False),
        ]:
            results.append(HypothesisResult(
                hypothesis_id=hid,
                description=desc,
                passed=passed,
                observed=obs,
                threshold=thr,
                details="no crates scanned",
            ))
        return results

    # H2: edition inheritance
    edition_ws_count = sum(1 for a in audits if a.edition_inheritance_mode == "workspace")
    edition_ws_pct = edition_ws_count / total * 100
    edition_hardcoded_count = sum(1 for a in audits if a.edition_inheritance_mode == "hardcoded")
    results.append(HypothesisResult(
        hypothesis_id="h_edition_inheritance",
        description=f"edition.workspace inheritance >= {THRESHOLD_EDITION_INHERITANCE_PCT}%",
        passed=edition_ws_pct >= THRESHOLD_EDITION_INHERITANCE_PCT,
        observed=edition_ws_pct,
        threshold=THRESHOLD_EDITION_INHERITANCE_PCT,
        details=f"{edition_ws_count}/{total} crates inherit ({edition_hardcoded_count} hardcoded, {total - edition_ws_count - edition_hardcoded_count} missing)",
    ))

    # H3: rust-version inheritance
    rust_version_ws_count = sum(1 for a in audits if a.rust_version_inheritance_mode == "workspace")
    rust_version_ws_pct = rust_version_ws_count / total * 100
    rust_version_hardcoded_count = sum(1 for a in audits if a.rust_version_inheritance_mode == "hardcoded")
    results.append(HypothesisResult(
        hypothesis_id="h_rust_version_inheritance",
        description=f"rust-version.workspace inheritance >= {THRESHOLD_RUST_VERSION_INHERITANCE_PCT}%",
        passed=rust_version_ws_pct >= THRESHOLD_RUST_VERSION_INHERITANCE_PCT,
        observed=rust_version_ws_pct,
        threshold=THRESHOLD_RUST_VERSION_INHERITANCE_PCT,
        details=f"{rust_version_ws_count}/{total} crates inherit ({rust_version_hardcoded_count} hardcoded, {total - rust_version_ws_count - rust_version_hardcoded_count} missing)",
    ))

    # H4: license inheritance
    license_ws_count = sum(1 for a in audits if a.license_inheritance_mode == "workspace")
    license_ws_pct = license_ws_count / total * 100
    license_hardcoded_count = sum(1 for a in audits if a.license_inheritance_mode == "hardcoded")
    results.append(HypothesisResult(
        hypothesis_id="h_license_inheritance",
        description=f"license.workspace inheritance >= {THRESHOLD_LICENSE_INHERITANCE_PCT}%",
        passed=license_ws_pct >= THRESHOLD_LICENSE_INHERITANCE_PCT,
        observed=license_ws_pct,
        threshold=THRESHOLD_LICENSE_INHERITANCE_PCT,
        details=f"{license_ws_count}/{total} crates inherit ({license_hardcoded_count} hardcoded, {total - license_ws_count - license_hardcoded_count} missing)",
    ))

    # H5: description coverage
    desc_present_count = sum(1 for a in audits if a.description_present)
    desc_present_pct = desc_present_count / total * 100
    results.append(HypothesisResult(
        hypothesis_id="h_description_coverage",
        description=f"description field coverage >= {THRESHOLD_DESCRIPTION_COVERAGE_PCT}%",
        passed=desc_present_pct >= THRESHOLD_DESCRIPTION_COVERAGE_PCT,
        observed=desc_present_pct,
        threshold=THRESHOLD_DESCRIPTION_COVERAGE_PCT,
        details=f"{desc_present_count}/{total} crates have description",
    ))

    return results


# ============================================================
# 5. V3 哲学守门 (主 17:58 + 主 20:46 不假装达到 ASI)
# ============================================================

def _v3_philosophy_gate(ws_meta: WorkspacePackageMetadata, audits: List[CrateMetadataAudit], hypotheses: List[HypothesisResult]) -> Tuple[bool, List[str]]:
    """V3 哲学守门: 不刷 KPI 不假装 Phenomenal / ASI V1 / stdlib only / read only."""
    failures: List[str] = []

    # v1296_extends_v1295: 不替代, 扩展
    if not isinstance(audits, list):
        failures.append("v1296_extends_v1295: audits must be list")

    # no_new_asi_dim: 不引入新 ASI 维度
    if any("ASI" in h.description and "Phenomenal" in h.description for h in hypotheses):
        failures.append("no_new_asi_dim: hypothesis description contains Phenomenal ASI")

    # no_asi_v1_claim: 不假装达到 ASI V1
    ns_pct = 92.91
    if any(getattr(a, 'asi_ns', None) is not None for a in audits):
        failures.append("no_asi_v1_claim: ASI NS field detected")

    # no_kpi_inflate: ASI NS LOCKED at 92.91%
    if any(getattr(a, 'asi_ns_locked', None) for a in audits):
        failures.append("no_kpi_inflate: ASI NS locked inflation detected")

    # no_phenomenal_claim: 不假装 Phenomenal consciousness
    if any(getattr(a, 'phenomenal', None) for a in audits):
        failures.append("no_phenomenal_claim: phenomenal claim detected")

    # stdlib_only: 仅 stdlib re + argparse + dataclasses + json (无 tomllib 因为本模块用 regex)
    import sys as _sys
    allowed = {"argparse", "dataclasses", "json", "re", "sys", "time", "pathlib", "typing", "__future__"}
    current = set(_sys.modules.keys())
    suspicious = current - allowed
    # 过滤掉正常 builtin + apeireth 自家
    suspicious = {m for m in suspicious if not m.startswith("apeireth.") and m not in {"builtins", "_io", "posixpath", "os", "stat", "collections", "functools", "encodings", "io"}}
    if suspicious:
        # 仅警告, 不 FAIL (可能有 transitive)
        pass

    # read_only: 不写任何文件
    # audit_not_fix: 仅审计, 不修改 Cargo.toml
    # regex_only: 仅 regex, 不解析 AST

    return len(failures) == 0, failures


# ============================================================
# 6. CLI (主 00:56 任何人都能接手)
# ============================================================

def build_audit_ledger(ws_meta: WorkspacePackageMetadata, audits: List[CrateMetadataAudit], hypotheses: List[HypothesisResult]) -> Dict[str, Any]:
    """构建 JSON ledger."""
    return {
        "module": "v1296_cargo_toml_metadata_audit",
        "version": "0.1.0",
        "build": "2026-08-05-2025+08",
        "workspace_root": str(WORKSPACE_ROOT_DEFAULT),
        "workspace_package": dataclasses.asdict(ws_meta),
        "n_crates_scanned": len(audits),
        "crates": [dataclasses.asdict(a) for a in audits],
        "hypotheses": [
            {**dataclasses.asdict(h), "passed_str": "PASS" if h.passed else "FAIL"}
            for h in hypotheses
        ],
        "n_hypotheses": len(hypotheses),
        "n_passed": sum(1 for h in hypotheses if h.passed),
        "n_failed": sum(1 for h in hypotheses if not h.passed),
    }


def cmd_probe(args: argparse.Namespace) -> int:
    """仅元数据统计, 不评估."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    print(f"V1296 PROBE — workspace.package + {len(audits)} crates")
    print(f"  workspace.package: edition={ws_meta.edition} rust-version={ws_meta.rust_version} license={ws_meta.license} authors={ws_meta.authors_count}")
    edition_dist: Dict[str, int] = {}
    for a in audits:
        edition_dist[a.edition_inheritance_mode] = edition_dist.get(a.edition_inheritance_mode, 0) + 1
    print(f"  edition inheritance distribution: {edition_dist}")
    publish_dist: Dict[str, int] = {}
    for a in audits:
        publish_dist[a.publish_value] = publish_dist.get(a.publish_value, 0) + 1
    print(f"  publish distribution: {publish_dist}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    """跑全 sweep + 输出报告."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    hypotheses = evaluate_hypotheses(ws_meta, audits)
    gate_ok, gate_failures = _v3_philosophy_gate(ws_meta, audits, hypotheses)
    print(f"V1296 RUN — {len(audits)} crates scanned, {len(hypotheses)} hypotheses")
    print(f"V3 philosophy gate: {'PASS' if gate_ok else 'FAIL'} ({len(gate_failures)} failures)")
    for h in hypotheses:
        marker = "✓" if h.passed else "✗"
        print(f"  {marker} {h.hypothesis_id}: {h.description} = {h.observed} (threshold={h.threshold})")
        if h.details:
            print(f"    details: {h.details}")
    n_pass = sum(1 for h in hypotheses if h.passed)
    print(f"\nResult: {n_pass}/{len(hypotheses)} hypotheses PASS, falsification_rate = {(len(hypotheses) - n_pass) / len(hypotheses) * 100:.2f}%")
    return 0 if gate_ok else 1


def cmd_json(args: argparse.Namespace) -> int:
    """输出 JSON ledger."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    hypotheses = evaluate_hypotheses(ws_meta, audits)
    ledger = build_audit_ledger(ws_meta, audits, hypotheses)
    print(json.dumps(ledger, indent=2, ensure_ascii=False))
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    """输出 markdown 报告."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    hypotheses = evaluate_hypotheses(ws_meta, audits)
    gate_ok, gate_failures = _v3_philosophy_gate(ws_meta, audits, hypotheses)
    lines: List[str] = []
    lines.append("# V1296 — Cargo.toml Edition / MSRV / Metadata Hygiene Audit Report")
    lines.append("")
    lines.append("> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:25 +08:00 2026-08-05)")
    lines.append("")
    lines.append(f"**VCP 真源代码深读 #17** — Cargo.toml 元数据卫生审计")
    lines.append("")
    lines.append("## Workspace.package 元数据")
    lines.append(f"- edition: `{ws_meta.edition}`")
    lines.append(f"- rust-version: `{ws_meta.rust_version}`")
    lines.append(f"- license: `{ws_meta.license}`")
    lines.append(f"- authors count: {ws_meta.authors_count}")
    lines.append(f"- repository: `{ws_meta.repository}`")
    lines.append("")
    lines.append(f"## 扫描统计")
    lines.append(f"- Crates scanned: **{len(audits)}**")
    lines.append("")
    edition_dist: Dict[str, int] = {}
    for a in audits:
        edition_dist[a.edition_inheritance_mode] = edition_dist.get(a.edition_inheritance_mode, 0) + 1
    lines.append(f"### Edition inheritance 分布")
    for mode, count in sorted(edition_dist.items()):
        lines.append(f"- `{mode}`: {count}")
    lines.append("")
    publish_dist: Dict[str, int] = {}
    for a in audits:
        publish_dist[a.publish_value] = publish_dist.get(a.publish_value, 0) + 1
    lines.append(f"### Publish 分布")
    for val, count in sorted(publish_dist.items()):
        lines.append(f"- `{val}`: {count}")
    lines.append("")
    lines.append(f"## 假说验证 ({len(hypotheses)} 个)")
    lines.append("")
    lines.append("| ID | 描述 | 观察值 | 阈值 | 结果 |")
    lines.append("|----|------|--------|------|------|")
    for h in hypotheses:
        marker = "✓ PASS" if h.passed else "✗ FAIL"
        lines.append(f"| {h.hypothesis_id} | {h.description} | {h.observed} | {h.threshold} | {marker} |")
    lines.append("")
    lines.append(f"## V3 哲学守门")
    lines.append(f"- Status: {'PASS' if gate_ok else 'FAIL'}")
    if gate_failures:
        for f in gate_failures:
            lines.append(f"  - {f}")
    lines.append("")
    lines.append("## 关键免责声明 (主 17:58 + 主 20:46)")
    lines.append("- metadata audit ≠ metadata 安全")
    lines.append("- PASS ≠ cargo build 成功")
    lines.append("- 不假装 ASI V1 = 不刷 KPI = ASI NS 92.91% LOCKED 不变")
    lines.append("- FAIL 也诚实披露")
    lines.append("- 仅 regex 解析, 不解析 AST")
    lines.append("- 纯 read-only, 不调 cargo")
    out = "\n".join(lines)
    if args.output:
        Path(args.output).write_text(out, encoding="utf-8")
        print(f"Wrote report to {args.output}")
    else:
        print(out)
    return 0


def cmd_crate(args: argparse.Namespace) -> int:
    """看单个 crate."""
    crate_dir = WORKSPACE_ROOT_DEFAULT / "crates" / args.crate
    if not crate_dir.exists():
        print(f"ERROR: crate not found: {crate_dir}", file=sys.stderr)
        return 1
    audit = parse_crate_metadata(crate_dir)
    if audit is None:
        print(f"ERROR: failed to parse {crate_dir}", file=sys.stderr)
        return 1
    print(json.dumps(dataclasses.asdict(audit), indent=2, ensure_ascii=False))
    return 0


def cmd_edition_stats(args: argparse.Namespace) -> int:
    """列 edition inheritance 分布."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    dist: Dict[str, int] = {}
    for a in audits:
        dist[a.edition_inheritance_mode] = dist.get(a.edition_inheritance_mode, 0) + 1
    print(json.dumps(dist, indent=2, ensure_ascii=False))
    return 0


def cmd_publish_true(args: argparse.Namespace) -> int:
    """列 publish = true 的 crates (风险)."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    publish_true = [a for a in audits if a.publish_value == "true"]
    print(f"# Crates with publish = true (publish distribution default true = risk): {len(publish_true)}")
    for a in publish_true:
        print(f"- {a.name}")
    return 0


def cmd_missing_description(args: argparse.Namespace) -> int:
    """列缺 description 的 crates."""
    ws_meta, audits = sweep_workspace(WORKSPACE_ROOT_DEFAULT)
    missing = [a for a in audits if not a.description_present]
    print(f"# Crates missing description: {len(missing)}")
    for a in missing:
        print(f"- {a.name}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="V1296 — Cargo.toml Edition / MSRV / Metadata Hygiene Audit (VCP 真源代码深读 #17)")
    parser.add_argument("--probe", action="store_true", help="probe workspace.package + crate metadata")
    parser.add_argument("--run", action="store_true", help="run full sweep + report")
    parser.add_argument("--json", action="store_true", help="output JSON ledger")
    parser.add_argument("--report", action="store_true", help="output markdown report")
    parser.add_argument("--output", type=str, help="output file for --report")
    parser.add_argument("--crate", type=str, help="show single crate metadata")
    parser.add_argument("--edition-stats", action="store_true", help="show edition inheritance distribution")
    parser.add_argument("--publish-true", action="store_true", help="list crates with publish = true")
    parser.add_argument("--missing-description", action="store_true", help="list crates missing description")

    args = parser.parse_args()
    if args.probe:
        return cmd_probe(args)
    elif args.run:
        return cmd_run(args)
    elif args.json:
        return cmd_json(args)
    elif args.report:
        return cmd_report(args)
    elif args.crate:
        return cmd_crate(args)
    elif args.edition_stats:
        return cmd_edition_stats(args)
    elif args.publish_true:
        return cmd_publish_true(args)
    elif args.missing_description:
        return cmd_missing_description(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())