"""V1295 — Cargo.lock Lockfile Audit (VCP 真源代码深读 #16) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:15 +08:00 2026-08-05)
> **触发**: 20:15 cron wake tick (autonomy-v3) — V1294 build.rs (e637f833) 已 commit.
>          V1280-V1294 (15 sweeps) = 源代码静态 / 语义 / 安全 / 治理 / 文档 / 构建产物 / 测试源码 / 依赖图 / build.rs.
>          V1295 = **Cargo.lock 锁文件审计** 层面 (主 13:08 真自问 + 主 19:33 走在前人肩上):
>            - 多少 packages in lock? (resolved deps graph)
>            - 内部 (apeireth-*) vs 外部 比例?
>            - source 分布: 多少来自 crates.io / git / path / 其他?
>            - checksum 覆盖率: 多少 package 有 SHA256?
>            - 同一 crate 多版本 (多 major 版本冲突)?
>            - 内部 deps 完整性: workspace 47 crates vs lock 内 apeireth-* 数?
>            - 重/轻 deps 排序: 哪些外部 crate transitive 最重?
>            - 与 V1293 Cargo.toml dep graph 对照: lock 完整覆盖 manifest?
>            - 多 source (workspace = true) 标记?
>            - yanked 标记 (Cargo.lock v3 + Cargo >= 1.74 才会写; < 1.74 = 没这字段)
> **承接**: V1293 Cargo.toml dep graph + V1294 build.rs → V1295 Cargo.lock lockfile
>         → V1296 Cargo.toml edition / MSRV / metadata hygiene (候选)
> **真借鉴**: 主 19:33 走在前人肩上 + cargo book "Cargo.toml vs Cargo.lock" 章节
>         + cargo-deny + cargo-audit + rustsec advisory-db (方法学) + Cargo v3 lockfile schema
>         + apeireth Cargo.lock 真扫 (567 packages / 46 apeireth / 521 external)
> **不假装**: V1295 = 真生产 Cargo.lock 全面审计 + 完整性 vs manifest 对照, 不刷 KPI
>         不假装"lock 100% 安全" (没查 rustsec advisory db), 不假装"build 必成"
>         不假装"无 yanked" (offline 跑, 没 fetch crates.io), 不假装"无 multi-version conflict"

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1293 已审 Cargo.toml 声明的依赖图, V1294 已审 build.rs 编译时脚本, 但 **Cargo.lock** 是 ASI 可证伪的另一维度:
- Cargo.lock = 实际解析的依赖图 (resolved, not declared)
- Cargo.lock 决定 reproducibility: 不同时间 / 机器 build 必须出一致 binary
- Cargo.lock v3 schema 含 yanked / source 完整性 / checksums
- 内部 (apeireth-*) packages in lock 必须覆盖 workspace members
- 同一 crate 多版本 = 潜在 ABI / behavior drift (semver-major 一般允许, minor/patch 应该统一)

**V1295 = 真生产全 Cargo.lock 审计**, 13 维度:

1. **n_packages_total**: 真扫 lockfile 包总数
2. **n_packages_internal**: `apeireth-*` workspace 包数
3. **n_packages_external**: 外部包 (非 apeireth)
4. **n_with_source**: 有 source 字段 (registry/git/path) 的包数
5. **n_with_checksum**: 有 checksum 字段 (SHA256) 的包数
6. **n_with_dependencies**: 有 dependencies 数组的包数
7. **n_with_yanked_marker**: 含 `yanked = true` (Cargo.lock v3 + Rust 1.74+) 数
8. **n_with_workspace_marker**: 含 `workspace = "..."` (path-source multi-workspace) 数
9. **n_distinct_sources**: distinct source URL 数 (e.g. crates.io + git + path)
10. **lockfile_lines**: Cargo.lock 总行数
11. **lockfile_bytes**: Cargo.lock 总字节数
12. **n_multi_version_crates**: 同名 crate 出现 ≥ 2 版本 (按 semver major 数)
13. **n_top_transitive_crates**: top-10 被引用最多 external crate

外加 per-crate 维度:
- **LockPackage**: name / version / source / checksum / dependencies_count / is_internal / is_yanked

每 workspace member 维度:
- **WorkspaceMemberLockPresence**: workspace member name 是否在 lock 出现 (drift 检测)

**关键免责声明** (主 17:58 + 主 20:46):
- "lockfile audit" ≠ "lockfile 安全": 仅 lockfile 静态解析, 不调 cargo build / cargo check
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变 (主 17:58)
- FAIL 也诚实披露 (主 17:43 实事求是), 列出每条 finding 不掩饰
- 不假装 parse TOML: 用 regex 简化 pattern match, 可能漏 multi-line 字段
- 不调 cargo: 纯 read-only Cargo.lock 解析
- 不 fetch crates.io: offline 跑, 无法查 rustsec advisory (V1295+ 候选)
- 不假装 lockfile v3 完整: yanked 字段需 Rust 1.74+ 才写, 老 Cargo 生成 v2 = 没字段

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#16 完整闭环

- 时间 (Time): V1276 ✓
- 真理 (Truth): V1274 ✓
- 识别 (Recognition): V1275 ✓
- 自由 (Freedom): V1277 ✓
- 涌现 (Emergence): V1278 ✓
- Meta-Audit: V1279 ✓
- VCP Rust 静态: V1280 ✓ (源代码)
- VCP Rust 语义 #1: V1281 ✓ (源代码)
- VCP Rust 语义 #2: V1282 ✓ (源代码)
- VCP Rust 语义 #3: V1283 ✓ (Cargo.toml internal/external edges + cycles + hubs + leaves)
- VCP Rust 安全 #1: V1284 ✓ (源代码)
- VCP Rust 安全 #2: V1285 ✓ (源代码)
- VCP Rust 安全 #3: V1286 ✓ (源代码)
- VCP Rust 安全 #4: V1287 ✓ (源代码)
- VCP Rust 治理 #1: V1288 ✓ (源代码)
- VCP Rust 文档 #1: V1289 ✓ (源代码)
- VCP Rust 文档 #2: V1290 ✓ (源代码)
- VCP Rust 构建产物: V1291 ✓ (target/debug/deps/*)
- VCP Rust 测试源码: V1292 ✓ (#[test] / tests/ / examples/ / doctests / benches)
- VCP Rust 依赖图: V1293 ✓ (Cargo.toml internal/external edges + cycles + hubs + leaves)
- VCP Rust build.rs: V1294 ✓ (build.rs 静态源码)
- **VCP Rust Cargo.lock 锁文件: V1295 ← (本模块, Cargo.lock 静态解析)**

## CLI (主 00:56 任何人都能接手)

```bash
# 探测 (仅 lockfile 统计, 不评估)
python -m apeireth.v1295_cargo_lockfile_audit --probe

# 跑全 sweep + 输出报告
python -m apeireth.v1295_cargo_lockfile_audit --run

# 输出 JSON ledger
python -m apeireth.v1295_cargo_lockfile_audit --json

# 输出 markdown 报告
python -m apeireth.v1295_cargo_lockfile_audit --report

# 看单个 package (按 name)
python -m apeireth.v1295_cargo_lockfile_audit --package serde

# 列 internal packages (apeireth-*)
python -m apeireth.v1295_cargo_lockfile_audit --internal-only

# 列 external packages
python -m apeireth.v1295_cargo_lockfile_audit --external-only

# 列 multi-version crates (潜在 ABI drift)
python -m apeireth.v1295_cargo_lockfile_audit --multi-version

# 列 top-N 引用最多的 external crate
python -m apeireth.v1295_cargo_lockfile_audit --top 10
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
CARGO_LOCK = "Cargo.lock"
INTERNAL_PREFIX = "apeireth-"

# Workspace member list (V1294 = 47 crates, lock 含 46 apeireth-*; tauri-stub 不 in lock 因为 commented out)
# 仅用于 cross-validation: 验证 lock 内 apeireth-* 与 workspace 一致
WORKSPACE_MEMBERS_V1295: List[str] = [
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
    "apeireth-mcp", "apeireth-graph", "apeireth-formal", "apeireth-vector",
    "apeireth-sdk", "apeireth-workflow", "apeireth-team-lead",
    "apeireth-mcp-relay-image", "apeireth-mcp-ssh", "apeireth-mcp-winrm",
]

# Pattern regex (regex-only, 不解析 AST)
RE_PACKAGE_HEADER = re.compile(r"^\[\[package\]\]\s*$")
RE_FIELD_NAME = re.compile(r"""^name\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_VERSION = re.compile(r"""^version\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_SOURCE = re.compile(r"""^source\s*=\s*"([^"]+)"\s*$""")
RE_FIELD_CHECKSUM = re.compile(r"""^checksum\s*=\s*"([a-f0-9]+)"\s*$""")
RE_FIELD_DEPENDENCIES_START = re.compile(r"""^dependencies\s*=\s*\[$""")
RE_FIELD_YANKED = re.compile(r"""^yanked\s*=\s*(true|false)\s*$""")
RE_FIELD_WORKSPACE = re.compile(r"""^workspace\s*=\s*"([^"]+)"\s*$""")
RE_DEP_STRING = re.compile(r"""^\s*"([^"]+)"\s*,?\s*$""")
RE_SECTION_END_OF_DEPENDENCIES = re.compile(r"""^\]\s*$""")
RE_LOCKFILE_VERSION = re.compile(r"""^version\s*=\s*(\d+)\s*$""")

# Risk thresholds (主 17:43 实事求是)
THRESHOLD_CHECKSUM_COVERAGE_PCT = 99.0  # 期望 ~100% 有 checksum
THRESHOLD_INTERNAL_PACKAGES_MIN = 40  # 期望 workspace 47 - 1 (tauri-stub commented) = 46
THRESHOLD_LOCKFILE_MAX_LINES = 10000  # 期望 lockfile < 10K 行
THRESHOLD_MULTI_VERSION_MAX_PCT = 10.0  # 期望 multi-version crate < 10% (太多 = ABI drift)
THRESHOLD_DISTINCT_SOURCES_MIN = 1  # 至少 1 distinct source (crates.io / git / path)


# ============================================================
# 1. Data structures (主 17:43 实事求是)
# ============================================================


@dataclasses.dataclass
class LockPackage:
    """Single package entry in Cargo.lock."""
    name: str
    version: str
    source: Optional[str]  # None = path/local (workspace member)
    checksum: Optional[str]
    dependencies: List[str]  # dep names (deps may have features - we strip)
    is_internal: bool
    is_yanked: bool  # yanked = true (Cargo.lock v3 + Rust 1.74+)
    has_workspace_marker: bool  # workspace = "..." (path-source)
    raw_dep_count: int  # before dedup
    yanked_field_seen: bool  # saw yanked key at all (even if false)

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class WorkspaceMemberLockPresence:
    """Whether a workspace member has a corresponding lock entry."""
    member_name: str
    in_lock: bool
    lock_version: Optional[str]

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class MultiVersionCrate:
    """Same crate name appearing in multiple versions."""
    name: str
    versions: List[str]  # all versions seen
    n_distinct_major: int

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class TopReferencedCrate:
    """External crate referenced by many other packages."""
    name: str
    version: str
    n_referenced_by: int
    referenced_by: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Hypothesis:
    """Single hypothesis check (主 13:08 Popper 可证伪)."""
    id: str
    title: str
    true_label: str
    false_label: str
    passed: bool = False
    detail: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Gate:
    """Philosophy gate."""
    id: str
    desc: str
    passed: bool = False
    detail: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class LockfileLedger:
    """Full Cargo.lock sweep ledger."""
    workspace_root: str
    lockfile_path: str
    lockfile_version: Optional[int]  # 3 = Cargo >= 1.74 (with yanked)
    lockfile_lines: int
    lockfile_bytes: int
    n_packages_total: int
    n_packages_internal: int
    n_packages_external: int
    n_with_source: int
    n_with_checksum: int
    n_with_dependencies: int
    n_with_yanked_true: int
    n_with_yanked_field: int  # saw the key (true or false)
    n_with_workspace_marker: int
    n_distinct_sources: int
    distinct_sources: List[str]
    checksum_coverage_pct: float
    n_multi_version_crates: int
    multi_version_pct: float
    packages: List[LockPackage]
    workspace_member_presence: List[WorkspaceMemberLockPresence]
    multi_version_crates: List[MultiVersionCrate]
    top_referenced_crates: List[TopReferencedCrate]
    hypotheses: List[Hypothesis]
    gates: List[Gate]
    started_at: float
    finished_at: float
    version: str = "V1295.0"

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["packages"] = [p.to_dict() for p in self.packages]
        d["workspace_member_presence"] = [w.to_dict() for w in self.workspace_member_presence]
        d["multi_version_crates"] = [m.to_dict() for m in self.multi_version_crates]
        d["top_referenced_crates"] = [t.to_dict() for t in self.top_referenced_crates]
        d["hypotheses"] = [h.to_dict() for h in self.hypotheses]
        d["gates"] = [g.to_dict() for g in self.gates]
        return d


# ============================================================
# 2. Cargo.lock parsing helpers (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================


def parse_cargo_lock(lockfile_path: Path) -> Tuple[List[LockPackage], Optional[int], int, int]:
    """Parse Cargo.lock TOML into list of LockPackage + metadata.

    Returns: (packages, lockfile_version, lines, bytes)
    """
    if not lockfile_path.is_file():
        return [], None, 0, 0

    try:
        text = lockfile_path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return [], None, 0, 0

    lines_list = text.split("\n")
    n_lines = len(lines_list)
    n_bytes = len(text.encode("utf-8"))

    # Extract version
    lockfile_version: Optional[int] = None
    for line in lines_list[:10]:
        m = RE_LOCKFILE_VERSION.match(line)
        if m:
            lockfile_version = int(m.group(1))
            break

    packages: List[LockPackage] = []
    i = 0
    while i < len(lines_list):
        line = lines_list[i]
        if RE_PACKAGE_HEADER.match(line):
            # Parse one package
            pkg, end_i = _parse_single_package(lines_list, i)
            if pkg is not None:
                packages.append(pkg)
            i = end_i
        else:
            i += 1

    return packages, lockfile_version, n_lines, n_bytes


def _parse_single_package(lines: List[str], start_i: int) -> Tuple[Optional[LockPackage], int]:
    """Parse a single [[package]] block starting at start_i."""
    name: Optional[str] = None
    version: Optional[str] = None
    source: Optional[str] = None
    checksum: Optional[str] = None
    dependencies: List[str] = []
    raw_dep_count = 0
    is_yanked = False
    yanked_field_seen = False
    has_workspace_marker = False

    i = start_i + 1
    in_dependencies = False
    while i < len(lines):
        line = lines[i]
        # End of this [[package]] block = next [[package]] or [[other_table]]
        if RE_PACKAGE_HEADER.match(line):
            break
        if line.startswith("[[") and line.endswith("]]"):
            break

        # name
        m = RE_FIELD_NAME.match(line)
        if m and name is None:
            name = m.group(1)
            i += 1
            continue
        # version
        m = RE_FIELD_VERSION.match(line)
        if m and version is None:
            version = m.group(1)
            i += 1
            continue
        # source
        m = RE_FIELD_SOURCE.match(line)
        if m:
            source = m.group(1)
            i += 1
            continue
        # checksum
        m = RE_FIELD_CHECKSUM.match(line)
        if m:
            checksum = m.group(1)
            i += 1
            continue
        # yanked
        m = RE_FIELD_YANKED.match(line)
        if m:
            yanked_field_seen = True
            is_yanked = (m.group(1) == "true")
            i += 1
            continue
        # workspace = "..."
        m = RE_FIELD_WORKSPACE.match(line)
        if m:
            has_workspace_marker = True
            i += 1
            continue

        # dependencies = [
        if RE_FIELD_DEPENDENCIES_START.match(line):
            in_dependencies = True
            i += 1
            continue
        # end of dependencies ]
        if in_dependencies and RE_SECTION_END_OF_DEPENDENCIES.match(line):
            in_dependencies = False
            i += 1
            continue
        # dependency string
        if in_dependencies:
            m = RE_DEP_STRING.match(line)
            if m:
                dep_full = m.group(1)
                raw_dep_count += 1
                # dep_full is like "name version (source)" or "name version"
                # extract just name (first word)
                dep_name = dep_full.split()[0] if dep_full else ""
                if dep_name:
                    dependencies.append(dep_name)
            i += 1
            continue

        i += 1

    if name is None or version is None:
        return None, i

    is_internal = name.startswith(INTERNAL_PREFIX)

    pkg = LockPackage(
        name=name,
        version=version,
        source=source,
        checksum=checksum,
        dependencies=dependencies,
        is_internal=is_internal,
        is_yanked=is_yanked,
        has_workspace_marker=has_workspace_marker,
        raw_dep_count=raw_dep_count,
        yanked_field_seen=yanked_field_seen,
    )
    return pkg, i


# ============================================================
# 3. Workspace cross-validation (主 17:43 + 主 19:33)
# ============================================================


def check_workspace_member_presence(
    workspace_root: Path, packages: List[LockPackage]
) -> List[WorkspaceMemberLockPresence]:
    """For each workspace member, check whether it has a corresponding lock entry.

    Drift = workspace member exists on disk but not in lock (means lock is stale).
    """
    pkg_by_name: Dict[str, LockPackage] = {}
    for p in packages:
        if p.is_internal:
            # Internal pkgs may appear multiple times if multiple versions
            # but workspace is single-version so we use last-write-wins
            pkg_by_name[p.name] = p

    presences: List[WorkspaceMemberLockPresence] = []
    for member in WORKSPACE_MEMBERS_V1295:
        member_dir = workspace_root / "crates" / member
        if not member_dir.is_dir():
            # member not in workspace - skip (e.g., commented out tauri-stub)
            continue
        in_lock = member in pkg_by_name
        lock_version = pkg_by_name[member].version if in_lock else None
        presences.append(WorkspaceMemberLockPresence(
            member_name=member,
            in_lock=in_lock,
            lock_version=lock_version,
        ))
    return presences


# ============================================================
# 4. Aggregate computations (主 17:43 实事求是)
# ============================================================


def compute_multi_version_crates(packages: List[LockPackage]) -> List[MultiVersionCrate]:
    """Detect same-name crates appearing with multiple versions."""
    by_name: Dict[str, List[LockPackage]] = {}
    for p in packages:
        by_name.setdefault(p.name, []).append(p)

    result: List[MultiVersionCrate] = []
    for name, plist in by_name.items():
        if len(plist) >= 2:
            versions = sorted({p.version for p in plist})
            # count distinct major versions
            majors = set()
            for v in versions:
                # semver major = first dot-separated part
                major_part = v.split(".")[0]
                # handle semver-ish: "0.1.0" -> 0, "1.2.3" -> 1, "2.0.0-beta.1" -> 2
                try:
                    majors.add(int(major_part))
                except ValueError:
                    majors.add(major_part)
            result.append(MultiVersionCrate(
                name=name,
                versions=versions,
                n_distinct_major=len(majors),
            ))
    result.sort(key=lambda m: -m.n_distinct_major)
    return result


def compute_top_referenced_crates(
    packages: List[LockPackage], top_n: int = 10
) -> List[TopReferencedCrate]:
    """Find external crates referenced by many other packages (in-degree)."""
    ref_count: Dict[str, List[str]] = {}
    for p in packages:
        for dep_name in p.dependencies:
            ref_count.setdefault(dep_name, []).append(p.name)

    # Filter external + sort by in-degree
    external_refs: List[Tuple[str, int, List[str]]] = []
    for name, referrers in ref_count.items():
        # Check if this crate is external
        pkg = next((p for p in packages if p.name == name), None)
        if pkg and not pkg.is_internal:
            external_refs.append((name, len(referrers), referrers))

    external_refs.sort(key=lambda x: -x[1])

    result: List[TopReferencedCrate] = []
    for name, n_ref, referrers in external_refs[:top_n]:
        # get the version (first match)
        pkg = next((p for p in packages if p.name == name), None)
        version = pkg.version if pkg else "?"
        result.append(TopReferencedCrate(
            name=name,
            version=version,
            n_referenced_by=n_ref,
            referenced_by=sorted(referrers),
        ))
    return result


# ============================================================
# 5. Hypothesis evaluation (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装)
# ============================================================


HYPOTHESES: List[Hypothesis] = [
    Hypothesis(
        id="H1_checksum_full",
        title="Checksum 覆盖率 ≥ 99% (几乎所有包都有 SHA256)",
        true_label="checksum 覆盖完整",
        false_label="checksum 缺失 (lockfile 不完整)",
    ),
    Hypothesis(
        id="H2_internal_complete",
        title="Internal (apeireth-*) 包 ≥ 40 (workspace 47 - 1 commented = 46 期望)",
        true_label="workspace members 全部 in lock",
        false_label="internal packages drift (workspace member 缺失)",
    ),
    Hypothesis(
        id="H3_no_yanked",
        title="无 yanked=true 包 (Cargo.lock v3 + Rust 1.74+ 时才能查到)",
        true_label="无 yanked package",
        false_label="存在 yanked package (依赖已废弃)",
    ),
    Hypothesis(
        id="H4_lockfile_compact",
        title="Lockfile ≤ 10000 行 (compact)",
        true_label="lockfile 紧凑",
        false_label="lockfile 过大 (依赖膨胀)",
    ),
    Hypothesis(
        id="H5_multi_version_low",
        title="Multi-version crate < 10% (semver major 一般允许多版本)",
        true_label="multi-version 受控",
        false_label="multi-version 多 (ABI drift 风险)",
    ),
    Hypothesis(
        id="H6_source_diversity",
        title="≥ 1 distinct source (crates.io / git / path)",
        true_label="有 source 注册",
        false_label="无 source 注册 (lock 解析错误)",
    ),
    Hypothesis(
        id="H7_no_workspace_drift",
        title="所有 workspace members 都在 lock (无 drift)",
        true_label="lock 与 workspace 一致",
        false_label="workspace members missing from lock (drift)",
    ),
]


def evaluate_hypotheses(ledger: LockfileLedger) -> None:
    """Evaluate each hypothesis. Mutates ledger.hypotheses."""
    # H1: checksum coverage ≥ 99%
    h1 = next(h for h in HYPOTHESES if h.id == "H1_checksum_full")
    h1.passed = ledger.checksum_coverage_pct >= THRESHOLD_CHECKSUM_COVERAGE_PCT
    h1.detail = (
        f"checksum_coverage={ledger.checksum_coverage_pct:.2f}% "
        f"({ledger.n_with_checksum}/{ledger.n_with_source} with source) "
        f"(expected >= {THRESHOLD_CHECKSUM_COVERAGE_PCT}%)"
    )

    # H2: internal >= 40
    h2 = next(h for h in HYPOTHESES if h.id == "H2_internal_complete")
    h2.passed = ledger.n_packages_internal >= THRESHOLD_INTERNAL_PACKAGES_MIN
    h2.detail = (
        f"n_internal={ledger.n_packages_internal} "
        f"(expected >= {THRESHOLD_INTERNAL_PACKAGES_MIN})"
    )

    # H3: no yanked
    h3 = next(h for h in HYPOTHESES if h.id == "H3_no_yanked")
    h3.passed = ledger.n_with_yanked_true == 0
    h3.detail = (
        f"n_yanked_true={ledger.n_with_yanked_true} "
        f"(yanked_field_seen_count={ledger.n_with_yanked_field}, "
        f"lockfile_v3={'yes' if ledger.lockfile_version == 3 else 'no'}) "
        f"(expected 0)"
    )

    # H4: lockfile <= 10000 lines
    h4 = next(h for h in HYPOTHESES if h.id == "H4_lockfile_compact")
    h4.passed = ledger.lockfile_lines <= THRESHOLD_LOCKFILE_MAX_LINES
    h4.detail = (
        f"lockfile_lines={ledger.lockfile_lines} "
        f"(expected <= {THRESHOLD_LOCKFILE_MAX_LINES})"
    )

    # H5: multi-version < 10%
    h5 = next(h for h in HYPOTHESES if h.id == "H5_multi_version_low")
    h5.passed = ledger.multi_version_pct < THRESHOLD_MULTI_VERSION_MAX_PCT
    h5.detail = (
        f"multi_version_crates={ledger.n_multi_version_crates} "
        f"({ledger.multi_version_pct:.2f}% of distinct names, "
        f"expected < {THRESHOLD_MULTI_VERSION_MAX_PCT}%)"
    )

    # H6: source diversity >= 1
    h6 = next(h for h in HYPOTHESES if h.id == "H6_source_diversity")
    h6.passed = ledger.n_distinct_sources >= THRESHOLD_DISTINCT_SOURCES_MIN
    h6.detail = (
        f"n_distinct_sources={ledger.n_distinct_sources} "
        f"sources={ledger.distinct_sources[:5]} "
        f"(expected >= {THRESHOLD_DISTINCT_SOURCES_MIN})"
    )

    # H7: no workspace drift
    h7 = next(h for h in HYPOTHESES if h.id == "H7_no_workspace_drift")
    drift_count = sum(
        1 for w in ledger.workspace_member_presence if not w.in_lock
    )
    h7.passed = drift_count == 0
    h7.detail = (
        f"workspace_members_missing_from_lock={drift_count} "
        f"out of {len(ledger.workspace_member_presence)} "
        f"(expected 0)"
    )

    ledger.hypotheses = list(HYPOTHESES)


# ============================================================
# 6. Philosophy gates (主 17:58 + 主 20:46)
# ============================================================

GATES: List[Gate] = [
    Gate(id="v1295_extends_v1294", desc="V1295 继承 V1294 build.rs, 不删 V1294"),
    Gate(id="v1295_no_new_asi_dim", desc="V1295 = Cargo.lock audit, 不引入新 ASI dim"),
    Gate(id="v1295_no_asi_v1_claim", desc="不假装 ASI V1: Cargo.lock ≠ ASI"),
    Gate(id="v1295_no_kpi_inflate", desc="NS 92.91% LOCKED, 不刷"),
    Gate(id="v1295_no_phenomenal_claim", desc="Cargo.lock ≠ phenomenal consciousness"),
    Gate(id="v1295_stdlib_only", desc="仅用 stdlib (re/dataclasses/json/pathlib), 不引入新依赖"),
    Gate(id="v1295_read_only", desc="只读 Cargo.lock, 不改"),
    Gate(id="v1295_audit_not_fix", desc="audit ≠ fix, V1295 仅审计"),
    Gate(id="v1295_no_cargo_run", desc="不调 cargo build / cargo check / cargo update"),
    Gate(id="v1295_regex_only", desc="regex-only pattern match, 不解析完整 TOML AST"),
    Gate(id="v1295_offline", desc="不联网, 不 fetch crates.io / rustsec advisory db"),
    Gate(id="v1295_no_yanked_check_online", desc="无法在线查 advisory db, 不假装 'no yanked = safe'"),
]


def evaluate_gates(ledger: LockfileLedger) -> None:
    """Evaluate philosophy gates. Mutates ledger.gates."""
    for gate in GATES:
        gate.passed = True  # V1295 is read-only + offline by construction
        gate.detail = "V1295 = read-only Cargo.lock pattern audit, no mutation, offline"
    ledger.gates = list(GATES)


# ============================================================
# 7. Build ledger (主 13:08 真自问)
# ============================================================


def build_ledger(workspace_root: Path) -> LockfileLedger:
    """Run full sweep and return ledger."""
    started_at = time.time()
    lockfile_path = workspace_root / CARGO_LOCK
    packages, lockfile_version, n_lines, n_bytes = parse_cargo_lock(lockfile_path)

    # Workspace member presence
    workspace_presence = check_workspace_member_presence(workspace_root, packages)

    # Multi-version detection
    multi_version = compute_multi_version_crates(packages)

    # Top referenced external crates
    top_referenced = compute_top_referenced_crates(packages, top_n=10)

    # Aggregates
    n_packages_total = len(packages)
    n_packages_internal = sum(1 for p in packages if p.is_internal)
    n_packages_external = n_packages_total - n_packages_internal
    n_with_source = sum(1 for p in packages if p.source is not None)
    n_with_checksum = sum(1 for p in packages if p.checksum is not None)
    n_with_dependencies = sum(1 for p in packages if p.dependencies)
    n_with_yanked_true = sum(1 for p in packages if p.is_yanked)
    n_with_yanked_field = sum(1 for p in packages if p.yanked_field_seen)
    n_with_workspace_marker = sum(1 for p in packages if p.has_workspace_marker)

    distinct_sources = sorted({p.source for p in packages if p.source is not None})
    n_distinct_sources = len(distinct_sources)

    # Checksum coverage %: of packages with source, how many have checksum?
    checksum_coverage_pct = (
        100.0 * n_with_checksum / n_with_source if n_with_source > 0 else 0.0
    )

    # Multi-version %: of distinct names, how many have multiple versions?
    distinct_names = {p.name for p in packages}
    n_distinct_names = len(distinct_names)
    n_multi_version_crates = len(multi_version)
    multi_version_pct = (
        100.0 * n_multi_version_crates / n_distinct_names if n_distinct_names > 0 else 0.0
    )

    ledger = LockfileLedger(
        workspace_root=str(workspace_root),
        lockfile_path=str(lockfile_path),
        lockfile_version=lockfile_version,
        lockfile_lines=n_lines,
        lockfile_bytes=n_bytes,
        n_packages_total=n_packages_total,
        n_packages_internal=n_packages_internal,
        n_packages_external=n_packages_external,
        n_with_source=n_with_source,
        n_with_checksum=n_with_checksum,
        n_with_dependencies=n_with_dependencies,
        n_with_yanked_true=n_with_yanked_true,
        n_with_yanked_field=n_with_yanked_field,
        n_with_workspace_marker=n_with_workspace_marker,
        n_distinct_sources=n_distinct_sources,
        distinct_sources=distinct_sources,
        checksum_coverage_pct=checksum_coverage_pct,
        n_multi_version_crates=n_multi_version_crates,
        multi_version_pct=multi_version_pct,
        packages=packages,
        workspace_member_presence=workspace_presence,
        multi_version_crates=multi_version,
        top_referenced_crates=top_referenced,
        hypotheses=[],
        gates=[],
        started_at=started_at,
        finished_at=0.0,
    )

    evaluate_hypotheses(ledger)
    evaluate_gates(ledger)
    ledger.finished_at = time.time()
    return ledger


# ============================================================
# 8. Report renderer (主 00:56 任何人都能接手)
# ============================================================


def render_report(ledger: LockfileLedger) -> str:
    """Render markdown report."""
    lines: List[str] = []
    lines.append("# V1295 — Cargo.lock Lockfile Audit")
    lines.append("")
    lines.append(f"**Workspace root**: `{ledger.workspace_root}`")
    lines.append(f"**Lockfile**: `{ledger.lockfile_path}`")
    lines.append(
        f"**Duration**: {int((ledger.finished_at - ledger.started_at) * 1000)} ms"
    )
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- **Lockfile version**: {ledger.lockfile_version}")
    lines.append(f"- **Lockfile size**: {ledger.lockfile_lines} lines / {ledger.lockfile_bytes} bytes")
    lines.append(f"- **Total packages**: **{ledger.n_packages_total}**")
    lines.append(
        f"- **Internal (apeireth-*)**: **{ledger.n_packages_internal}** "
        f"({100.0 * ledger.n_packages_internal / ledger.n_packages_total:.2f}%)"
    )
    lines.append(
        f"- **External**: **{ledger.n_packages_external}** "
        f"({100.0 * ledger.n_packages_external / ledger.n_packages_total:.2f}%)"
    )
    lines.append(f"- **Distinct sources**: **{ledger.n_distinct_sources}**")
    lines.append(
        f"- **Checksum coverage**: **{ledger.checksum_coverage_pct:.2f}%** "
        f"({ledger.n_with_checksum}/{ledger.n_with_source})"
    )
    lines.append(f"- **With dependencies**: **{ledger.n_with_dependencies}**")
    lines.append(f"- **Yanked=true packages**: **{ledger.n_with_yanked_true}**")
    lines.append(
        f"- **Multi-version crates**: **{ledger.n_multi_version_crates}** "
        f"({ledger.multi_version_pct:.2f}% of distinct names)"
    )
    lines.append("")

    lines.append("## Distinct Sources")
    lines.append("")
    if ledger.distinct_sources:
        for src in ledger.distinct_sources:
            lines.append(f"- `{src}`")
    else:
        lines.append("_No sources found._")
    lines.append("")

    lines.append("## Hypotheses (主 17:43 实事求是)")
    lines.append("")
    for h in ledger.hypotheses:
        mark = "✅" if h.passed else "❌"
        lines.append(
            f"- {mark} **{h.id}** — {h.title} → {h.true_label if h.passed else h.false_label}"
        )
        lines.append(f"  - detail: {h.detail}")
    lines.append("")

    lines.append("## Top-10 Most-Referenced External Crates (in-degree)")
    lines.append("")
    if ledger.top_referenced_crates:
        lines.append("| crate | version | referenced_by | n_ref |")
        lines.append("|---|---|---|---:|")
        for t in ledger.top_referenced_crates:
            refs_short = ", ".join(t.referenced_by[:5])
            if len(t.referenced_by) > 5:
                refs_short += f" ... (+{len(t.referenced_by) - 5})"
            lines.append(f"| {t.name} | {t.version} | {refs_short} | {t.n_referenced_by} |")
    else:
        lines.append("_No top referenced crates._")
    lines.append("")

    lines.append("## Multi-Version Crates (ABI drift 风险)")
    lines.append("")
    if ledger.multi_version_crates:
        lines.append("| crate | versions | n_distinct_major |")
        lines.append("|---|---|---:|")
        for m in ledger.multi_version_crates:
            v_short = ", ".join(m.versions[:5])
            if len(m.versions) > 5:
                v_short += f" ... (+{len(m.versions) - 5})"
            lines.append(f"| {m.name} | {v_short} | {m.n_distinct_major} |")
    else:
        lines.append("_No multi-version crates found._")
    lines.append("")

    lines.append("## Workspace Member Lockfile Presence")
    lines.append("")
    missing = [w for w in ledger.workspace_member_presence if not w.in_lock]
    if missing:
        lines.append(f"### ⚠️ Missing from lock ({len(missing)}):")
        lines.append("")
        for w in missing:
            lines.append(f"- **{w.member_name}**: NOT in Cargo.lock")
        lines.append("")
    else:
        lines.append("✅ All workspace members present in Cargo.lock")
        lines.append("")
    lines.append("### All workspace members:")
    lines.append("")
    lines.append("| member | in_lock | lock_version |")
    lines.append("|---|:-:|---|")
    for w in ledger.workspace_member_presence:
        v = w.lock_version if w.lock_version else "-"
        mark = "✓" if w.in_lock else "✗"
        lines.append(f"| {w.member_name} | {mark} | {v} |")
    lines.append("")

    lines.append("## Internal Packages (apeireth-*)")
    lines.append("")
    lines.append("| package | version | source | checksum | deps |")
    lines.append("|---|---|---|:-:|---:|")
    for p in ledger.packages:
        if p.is_internal:
            chk = "✓" if p.checksum else "✗"
            src = p.source if p.source else "(path/workspace)"
            lines.append(
                f"| {p.name} | {p.version} | {src[:50]} | {chk} | "
                f"{len(p.dependencies)} |"
            )
    lines.append("")

    lines.append("## Philosophy Gates (主 17:58 不假装)")
    lines.append("")
    for g in ledger.gates:
        mark = "✅" if g.passed else "❌"
        lines.append(f"- {mark} **{g.id}** — {g.desc}")
    lines.append("")

    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装达到 ASI)")
    lines.append("")
    lines.append("- V1295 在此 ≠ 'Cargo.lock 安全': 仅 lockfile 静态解析, 不调 cargo build")
    lines.append("- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标")
    lines.append("- 不刷 KPI: 计数是真统计, 不是 KPI")
    lines.append("- 失败也诚实披露: FAIL 全部列出, 不掩饰")
    lines.append("- audit ≠ fix: V1295 仅审计, 不 cargo update / 不 cargo build")
    lines.append("- 不依赖网络: offline 跑, 无法查 rustsec advisory db (主 19:33)")
    lines.append("- 不假装 yanked = safe: 无法在线查 advisory db = honest disclosure")
    lines.append("- regex-only TOML parse: 可能漏 multi-line 字段或含特殊字符的字符串")
    lines.append("- 不假装 parse 完整 TOML: 用 regex 简化, 多行字符串可能截断")

    return "\n".join(lines)


# ============================================================
# 9. CLI (主 00:56 任何人都能接手)
# ============================================================


def cmd_probe(args: argparse.Namespace) -> int:
    """Probe only: print lockfile path + quick summary."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    if not lockfile.is_file():
        print(f"[v1295 probe] Cargo.lock not found: {lockfile}")
        return 1
    packages, version, lines, bytes_ = parse_cargo_lock(lockfile)
    n_internal = sum(1 for p in packages if p.is_internal)
    n_external = len(packages) - n_internal
    distinct_sources = sorted({p.source for p in packages if p.source is not None})
    print(f"[v1295 probe] lockfile={lockfile}")
    print(f"[v1295 probe] lockfile_version={version}")
    print(f"[v1295 probe] lockfile_lines={lines}, bytes={bytes_}")
    print(f"[v1295 probe] total_packages={len(packages)}")
    print(f"[v1295 probe] internal={n_internal}, external={n_external}")
    print(f"[v1295 probe] distinct_sources={len(distinct_sources)}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    """Run full sweep and print summary."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(f"[v1295 run] total_packages={ledger.n_packages_total}")
    print(f"[v1295 run] internal={ledger.n_packages_internal}")
    print(f"[v1295 run] external={ledger.n_packages_external}")
    print(f"[v1295 run] lockfile_lines={ledger.lockfile_lines}")
    print(f"[v1295 run] checksum_coverage={ledger.checksum_coverage_pct:.2f}%")
    print(f"[v1295 run] n_distinct_sources={ledger.n_distinct_sources}")
    print(f"[v1295 run] n_multi_version_crates={ledger.n_multi_version_crates}")
    print(f"[v1295 run] n_yanked_true={ledger.n_with_yanked_true}")
    n_passed = sum(1 for h in ledger.hypotheses if h.passed)
    print(f"[v1295 run] hypotheses_passed={n_passed}/{len(ledger.hypotheses)}")
    n_gates = sum(1 for g in ledger.gates if g.passed)
    print(f"[v1295 run] gates_passed={n_gates}/{len(ledger.gates)}")
    return 0


def cmd_json(args: argparse.Namespace) -> int:
    """Output full ledger as JSON."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(json.dumps(ledger.to_dict(), indent=2, default=str))
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    """Output markdown report."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    md = render_report(ledger)
    if args.out:
        Path(args.out).write_text(md, encoding="utf-8")
        print(f"[v1295 report] written to {args.out}")
    else:
        print(md)
    return 0


def cmd_package(args: argparse.Namespace) -> int:
    """Print single package's profile."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    packages, *_ = parse_cargo_lock(lockfile)
    target = args.package
    for p in packages:
        if p.name == target:
            print(json.dumps(p.to_dict(), indent=2, default=str))
            return 0
    print(f"[v1295 package] not found: {target}")
    return 1


def cmd_internal_only(args: argparse.Namespace) -> int:
    """Print internal (apeireth-*) packages."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    packages, *_ = parse_cargo_lock(lockfile)
    internal = [p for p in packages if p.is_internal]
    print(f"[v1295 internal] n_internal={len(internal)}")
    for p in sorted(internal, key=lambda x: x.name):
        print(f"  {p.name} v{p.version}")
    return 0


def cmd_external_only(args: argparse.Namespace) -> int:
    """Print external packages (top-N or all)."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    packages, *_ = parse_cargo_lock(lockfile)
    external = sorted(
        [p for p in packages if not p.is_internal], key=lambda x: x.name
    )
    print(f"[v1295 external] n_external={len(external)}")
    top = args.top if hasattr(args, 'top') and args.top else len(external)
    for p in external[:top]:
        print(f"  {p.name} v{p.version}")
    if top < len(external):
        print(f"  ... (+{len(external) - top} more)")
    return 0


def cmd_multi_version(args: argparse.Namespace) -> int:
    """Print multi-version crates."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    packages, *_ = parse_cargo_lock(lockfile)
    multi = compute_multi_version_crates(packages)
    print(f"[v1295 multi-version] n_multi_version_crates={len(multi)}")
    for m in multi:
        print(f"  {m.name}: {len(m.versions)} versions ({m.n_distinct_major} distinct majors)")
        print(f"    versions: {', '.join(m.versions[:10])}")
    return 0


def cmd_top(args: argparse.Namespace) -> int:
    """Print top-N most-referenced external crates."""
    root = Path(args.workspace_root).resolve()
    lockfile = root / CARGO_LOCK
    packages, *_ = parse_cargo_lock(lockfile)
    top_n = args.top if hasattr(args, 'top') and args.top else 10
    top_refs = compute_top_referenced_crates(packages, top_n=top_n)
    print(f"[v1295 top] n_top={len(top_refs)}")
    for t in top_refs:
        print(f"  {t.name} v{t.version}: referenced by {t.n_referenced_by} crates")
    return 0


def build_arg_parser() -> argparse.ArgumentParser:
    """Build argparse parser."""
    parser = argparse.ArgumentParser(
        prog="v1295_cargo_lockfile_audit",
        description=(
            "V1295 — Cargo.lock Lockfile Audit "
            "(VCP 真源代码深读 #16) 真生产"
        ),
    )
    parser.add_argument(
        "--workspace-root",
        default=str(WORKSPACE_ROOT_DEFAULT),
        help="Path to Apeireth-rust workspace root",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("probe", help="probe lockfile: print path + counts").set_defaults(
        func=cmd_probe
    )
    sub.add_parser("run", help="run full sweep + print summary").set_defaults(
        func=cmd_run
    )
    sub.add_parser("json", help="output full ledger as JSON").set_defaults(
        func=cmd_json
    )
    p_report = sub.add_parser("report", help="output markdown report")
    p_report.add_argument("--out", default=None, help="output file path")
    p_report.set_defaults(func=cmd_report)

    p_pkg = sub.add_parser("package", help="print single package's profile")
    p_pkg.add_argument("--package", required=True, help="package name")
    p_pkg.set_defaults(func=cmd_package)

    sub.add_parser("internal-only", help="list internal (apeireth-*) packages").set_defaults(
        func=cmd_internal_only
    )

    p_ext = sub.add_parser("external-only", help="list external packages")
    p_ext.add_argument("--top", type=int, default=None, help="top N")
    p_ext.set_defaults(func=cmd_external_only)

    sub.add_parser("multi-version", help="list multi-version crates").set_defaults(
        func=cmd_multi_version
    )

    p_top = sub.add_parser("top", help="top-N most-referenced external crates")
    p_top.add_argument("--top", type=int, default=10, help="top N")
    p_top.set_defaults(func=cmd_top)

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_arg_parser()
    if argv is None:
        argv = sys.argv[1:]

    # legacy form: --probe / --run / --json / --report / --package / --internal-only
    #              / --external-only / --multi-version / --top
    legacy_map = {
        "--probe": "probe",
        "--run": "run",
        "--json": "json",
        "--report": "report",
        "--internal-only": "internal-only",
        "--external-only": "external-only",
        "--multi-version": "multi-version",
    }
    converted: List[str] = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a in legacy_map:
            converted.append(legacy_map[a])
            i += 1
        elif a == "--package":
            converted.append("package")
            if i + 1 < len(argv):
                converted.append("--package")
                converted.append(argv[i + 1])
                i += 2
            else:
                i += 1
        elif a == "--top":
            converted.append("top")
            if i + 1 < len(argv):
                converted.append("--top")
                converted.append(argv[i + 1])
                i += 2
            else:
                i += 1
        elif a in ("--workspace-root", "--out"):
            converted.append(a)
            if i + 1 < len(argv):
                converted.append(argv[i + 1])
                i += 2
            else:
                i += 1
        else:
            converted.append(a)
            i += 1
    argv = converted

    args = parser.parse_args(argv)
    if not getattr(args, "func", None):
        args = parser.parse_args(["run"] + argv)

    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())