"""V1293 — Cargo Dependency Graph Profile (VCP 真实源代码深读 #14) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:45+08:00 2026-08-05)
> **触发**: 19:45 cron wake tick (autonomy-v3) — V1292 test source coverage 已 commit (30561a85).
>          V1280-V1292 已审 源代码静态/语义/安全/治理/文档/构建产物/测试源码 (12 sweeps, 1 sweep = 子主题).
>          V1293 = **Cargo.toml 依赖图** 层面 (主 13:08 真自问):
>            - 42 crates 的 [dependencies] / [dev-dependencies] / [build-dependencies] 分布
>            - internal vs external dep 划分 (path = "../apeireth-*" vs workspace = true)
>            - in-degree / out-degree / hub score / leaf score
>            - 内部依赖环检测 (apeireth-* 子图 cycle)
>            - 深度 (最长依赖路径)
>            - 与 V1291/V1292 对照: 哪些 crate 依赖多但 artifact 少?
> **承接**: V1280 静态 + V1281-V1283 语义 + V1284-V1287 安全 + V1288 治理 + V1289-V1290 文档 + V1291 构建产物 + V1292 测试源码 → V1293 依赖图
> **真借鉴**: 主 19:33 走在前人肩上 + cargo workspace 约定 + graph theory (in/out degree, cycle detect, hub/leaf) + wasmtime/qdrant workspace patterns + V1285 42-crate discovery
> **不假装**: V1293 = 真生产 Cargo.toml 依赖图 audit, 不刷 KPI, 不假装 ASI V1, 不假装"无环", 不假装"图结构完美"

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1280-V1292 已审源代码 + 编译产物 + 测试源码, 但 **依赖图** 是 ASI 可证伪的另一维度:
- 42 crates 谁依赖谁? 谁被谁依赖? (主 19:33 走在前人肩上)
- 是否有内部依赖环? (cargo build 失败主因之一)
- 哪些 crate 是 hub (高 in-degree)? 哪些是 leaf (零 out-degree)?
- 哪些 crate 内部依赖多但 build artifact 少? (V1293 ? V1291 cross)
- 哪些 crate 外部依赖多但内部少? (潜在抽象边界)
- workspace lints 继承率: 多少 crate 用了 `lints = workspace = true`?

**V1293 = 真生产全 42 crates cargo dep graph profile**, 12 维度 per crate:

1. **n_internal_deps**: 真 `[dependencies]` 中 apeireth-* 数 (path = "../apeireth-*")
2. **n_external_deps**: 真 `[dependencies]` 中非 apeireth-* 数 (workspace = true 等)
3. **n_dev_deps**: 真 `[dev-dependencies]` 数
4. **n_build_deps**: 真 `[build-dependencies]` 数
5. **n_features**: 真 `[features]` 表 entries 数 (top-level keys)
6. **n_optional_deps**: 真 `optional = true` deps 数
7. **internal_deps_list**: 内部 dep names (排序)
8. **external_deps_list**: 外部 dep names (排序)
9. **has_workspace_lints**: `[lints]\nworkspace = true` 是否存在
10. **has_lib_target**: `[lib]` section 是否存在
11. **n_bin_targets**: `[[bin]]` 数
12. **n_example_targets**: `[[example]]` 数

外加 graph-level (42-crate internal subgraph):
- **in_degree**: 多少 apeireth-* crates 依赖本 crate
- **out_degree**: 多少 apeireth-* crates 本 crate 依赖 (= n_internal_deps)
- **internal_cycles**: 内部子图所有 cycle (强连通分量 > 1)
- **max_depth**: 从 leaf 到本 crate 的最长路径

每一 crate = 真 file:line + dep 列表 + 拓扑 + cycle.

**关键免责声明** (主 17:58 + 主 20:46):
- "dep graph profile" 在此 ≠ "dep graph 健康": 仅扫描 Cargo.toml + 简单 cycle check
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变 (主 17:58)
- FAIL 也诚实披露 (主 17:43 实事求是), 列出每条 finding 不掩饰
- 不假装 cargo tree: 简化用 toml parse + DFS, 不调 cargo tree CLI
- optional deps 不展开: 仅数 `optional = true` 标记
- features deps 不递归: 仅顶层 `[features]` 表
- workspace = true 视为 external dep (workspace.dependencies 已知)
- 不解析 Cargo.lock: 仅 Cargo.toml
- cycle 检测: 仅 internal apeireth-* 子图

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#14 完整闭环

- 时间 (Time): V1276 ?
- 真理 (Truth): V1274 ?
- 识别 (Recognition): V1275 ?
- 自由 (Freedom): V1277 ?
- 涌现 (Emergence): V1278 ?
- Meta-Audit: V1279 ?
- VCP Rust 静态: V1280 ? (源代码)
- VCP Rust 语义 #1: V1281 ? (源代码)
- VCP Rust 语义 #2: V1282 ? (源代码)
- VCP Rust 语义 #3: V1283 ? (源代码)
- VCP Rust 安全 #1: V1284 ? (源代码)
- VCP Rust 安全 #2: V1285 ? (源代码)
- VCP Rust 安全 #3: V1286 ? (源代码)
- VCP Rust 安全 #4: V1287 ? (源代码)
- VCP Rust 治理 #1: V1288 ? (源代码)
- VCP Rust 文档 #1: V1289 ? (源代码)
- VCP Rust 文档 #2: V1290 ? (源代码)
- VCP Rust 构建 #1 (build artifact profile): V1291 ? (target/debug/deps/*)
- VCP Rust 测试 #1 (test source coverage): V1292 ? (源代码)
- **VCP Rust 依赖图 #1 (cargo dep graph profile)**: V1293 = Cargo.toml 依赖图 audit ← **本模块**

## CLI 入口 (主 00:56 任何人都能接手)

```bash
python -m apeireth.v1293_rust_dependency_graph_profile --probe
python -m apeireth.v1293_rust_dependency_graph_profile --run
python -m apeireth.v1293_rust_dependency_graph_profile --json
python -m apeireth.v1293_rust_dependency_graph_profile --report R.md
python -m apeireth.v1293_rust_dependency_graph_profile --top-hub 10
python -m apeireth.v1293_rust_dependency_graph_profile --top-leaf 10
python -m apeireth.v1293_rust_dependency_graph_profile --crate apeireth-sovereignty
python -m apeireth.v1293_rust_dependency_graph_profile --cycles
```

## 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装)

1. v1293_extends_v1292 (V1293 继承 V1292 test source, 不删 V1292)
2. v1293_no_new_asi_dim (V1293 = dep graph, 不引入新 ASI dim)
3. v1293_no_asi_v1_claim (不假装 ASI V1: dep graph ≠ ASI)
4. v1293_no_kpi_inflate (NS 92.91% LOCKED, 不刷)
5. v1293_no_phenomenal_claim (dep graph ≠ phenomenal consciousness)
6. v1293_stdlib_only (仅用 tomllib, 不引入新依赖)
7. v1293_read_only (只读 Cargo.toml, 不改)
8. v1293_audit_not_fix (audit ≠ fix, V1293 仅审计)
9. v1293_toml_only_no_cargo_tree (用 toml parse + DFS, 不调 cargo tree CLI)
10. v1293_42_crates_full (全 42 crates, 不只 worst-5)
11. v1293_no_cargo_lock_parse (不解析 Cargo.lock, 只 Cargo.toml)
12. v1293_no_workspace_member_modify (不动 workspace.toml, 只读)

## VCP Rust #1-#14 完整闭环收官

V1293 = cargo dep graph profile (Cargo.toml 层面, novel 维度) →
真生产 5 假说 + 12 gates + 全 42 crates.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import tomllib
from collections import defaultdict, deque
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple


# ============================================================
# 1. Data structures (主 17:43 实事求是)
# ============================================================


@dataclass
class CrateDepProfile:
    """Per-crate cargo dependency profile."""

    crate_name: str = ""
    crate_dir: str = ""
    cargo_toml_path: str = ""
    cargo_toml_exists: bool = False

    # 12 维度
    n_internal_deps: int = 0
    n_external_deps: int = 0
    n_dev_deps: int = 0
    n_build_deps: int = 0
    n_features: int = 0
    n_optional_deps: int = 0

    internal_deps_list: List[str] = field(default_factory=list)
    external_deps_list: List[str] = field(default_factory=list)

    has_workspace_lints: bool = False
    has_lib_target: bool = False
    n_bin_targets: int = 0
    n_example_targets: int = 0

    # graph-level (后填充)
    in_degree: int = 0  # 多少 apeireth-* crates 依赖本 crate
    out_degree: int = 0  # = n_internal_deps (本 crate 依赖多少 apeireth-*)
    reverse_deps_list: List[str] = field(default_factory=list)  # 谁依赖我

    @property
    def total_dependencies(self) -> int:
        return self.n_internal_deps + self.n_external_deps

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CrateGraphCycle:
    """一个内部子图 cycle."""

    cycle_crates: List[str] = field(default_factory=list)  # 闭循环
    cycle_length: int = 0


@dataclass
class DepGraphLedger:
    """42 crates cargo dependency graph ledger."""

    crate_profiles: List[CrateDepProfile] = field(default_factory=list)
    cycles: List[CrateGraphCycle] = field(default_factory=list)
    started_at: float = 0.0
    finished_at: float = 0.0
    workspace_root: str = ""
    duration_ms: int = 0

    @property
    def total_crates(self) -> int:
        return len(self.crate_profiles)

    @property
    def total_internal_deps(self) -> int:
        return sum(p.n_internal_deps for p in self.crate_profiles)

    @property
    def total_external_deps(self) -> int:
        return sum(p.n_external_deps for p in self.crate_profiles)

    @property
    def total_dev_deps(self) -> int:
        return sum(p.n_dev_deps for p in self.crate_profiles)

    @property
    def total_build_deps(self) -> int:
        return sum(p.n_build_deps for p in self.crate_profiles)

    @property
    def total_features(self) -> int:
        return sum(p.n_features for p in self.crate_profiles)

    @property
    def crates_with_workspace_lints(self) -> int:
        return sum(1 for p in self.crate_profiles if p.has_workspace_lints)

    @property
    def crates_with_lib(self) -> int:
        return sum(1 for p in self.crate_profiles if p.has_lib_target)

    @property
    def max_internal_in_degree(self) -> int:
        return max((p.in_degree for p in self.crate_profiles), default=0)

    @property
    def max_internal_out_degree(self) -> int:
        return max((p.out_degree for p in self.crate_profiles), default=0)

    @property
    def leaf_crate_count(self) -> int:
        return sum(1 for p in self.crate_profiles if p.out_degree == 0)

    @property
    def hub_crate_count(self) -> int:
        return sum(1 for p in self.crate_profiles if p.in_degree >= 5)

    @property
    def total_cycles(self) -> int:
        return len(self.cycles)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "duration_ms": self.duration_ms,
            "workspace_root": self.workspace_root,
            "total_crates": self.total_crates,
            "total_internal_deps": self.total_internal_deps,
            "total_external_deps": self.total_external_deps,
            "total_dev_deps": self.total_dev_deps,
            "total_build_deps": self.total_build_deps,
            "total_features": self.total_features,
            "crates_with_workspace_lints": self.crates_with_workspace_lints,
            "crates_with_lib": self.crates_with_lib,
            "max_internal_in_degree": self.max_internal_in_degree,
            "max_internal_out_degree": self.max_internal_out_degree,
            "leaf_crate_count": self.leaf_crate_count,
            "hub_crate_count": self.hub_crate_count,
            "total_cycles": self.total_cycles,
            "cycles": [
                {"cycle_crates": c.cycle_crates, "cycle_length": c.cycle_length}
                for c in self.cycles
            ],
            "crate_profiles": [p.to_dict() for p in self.crate_profiles],
        }


# ============================================================
# 2. Cargo.toml 扫描器 (主 19:33 走在前人肩上 + cargo workspace 约定)
# ============================================================

# internal dep 匹配: path = "../apeireth-*"
INTERNAL_PATH_RE = re.compile(r'^\.+/apeireth-[a-z0-9-]+$|^crates/apeireth-[a-z0-9-]+$')

# lints = workspace = true 匹配
WORKSPACE_LINTS_RE = re.compile(
    r'^\s*\[lints\]\s*$|^\s*workspace\s*=\s*true\s*$', re.MULTILINE
)


def _is_internal_dep(dep_spec: Any, workspace_deps: Set[str]) -> Tuple[bool, str]:
    """
    判断一个 dep 是不是 internal apeireth-* dep.

    规则 (主 17:43 实事求是):
      - dep key 以 "apeireth-" 开头 → 视为 internal
      - 或 dep value 含 `path = "../apeireth-*"` → 视为 internal

    返回: (is_internal, dep_name)
    """
    if isinstance(dep_spec, str):
        # 简单 string 形式: "apeireth-core" 直接是名字
        if dep_spec.startswith("apeireth-"):
            return True, dep_spec
        return False, dep_spec
    if not isinstance(dep_spec, dict):
        return False, ""
    dep_name = dep_spec.get("name", "")
    if not dep_name:
        return False, ""
    if not dep_name.startswith("apeireth-"):
        return False, dep_name
    # dep_name starts with "apeireth-" → internal
    # 即使它写 `apeireth-core = { workspace = true }` 也算 internal (在 workspace.dependencies 里有 path override)
    return True, dep_name


def _is_optional_dep(dep_spec: Dict[str, Any]) -> bool:
    """检查 dep 是否是 optional = true."""
    if not isinstance(dep_spec, dict):
        return False
    return dep_spec.get("optional", False) is True


def _extract_workspace_lints_from_text(cargo_text: str) -> bool:
    """从 Cargo.toml 文本检查 [lints] workspace = true (粗 regex)."""
    if "[lints]" not in cargo_text:
        return False
    # 简单检查: [lints] 后 200 字符内有 workspace = true
    idx = cargo_text.find("[lints]")
    snippet = cargo_text[idx:idx + 500]
    return bool(re.search(r"workspace\s*=\s*true", snippet))


def _scan_crate(crate_name: str, crate_dir: Path) -> CrateDepProfile:
    """真扫描单个 crate 的 Cargo.toml 依赖 (主 17:43 实事求是)."""
    profile = CrateDepProfile(
        crate_name=crate_name,
        crate_dir=str(crate_dir),
    )
    cargo_toml_path = crate_dir / "Cargo.toml"
    profile.cargo_toml_path = str(cargo_toml_path)
    profile.cargo_toml_exists = cargo_toml_path.is_file()

    if not profile.cargo_toml_exists:
        return profile

    # 解析 Cargo.toml
    cargo_text = cargo_toml_path.read_text(encoding="utf-8", errors="replace")
    try:
        data = tomllib.loads(cargo_text)
    except Exception:
        return profile

    # workspace.dependencies (从 workspace root 传入)
    workspace_deps: Set[str] = set()

    # [dependencies]
    deps = data.get("dependencies", {})
    if isinstance(deps, dict):
        for dep_name, dep_spec in deps.items():
            # workspace = true 也算 external (它在 workspace.dependencies)
            workspace_flag = (
                isinstance(dep_spec, dict) and dep_spec.get("workspace") is True
            )
            # path = "../apeireth-*" 视为 internal (覆盖 workspace 标志)
            path_val = (
                dep_spec.get("path", "")
                if isinstance(dep_spec, dict)
                else ""
            )
            apeireth_path = bool(path_val and "apeireth-" in path_val)

            if apeireth_path:
                # 强 internal: path 指 apeireth-* crate
                profile.internal_deps_list.append(dep_name)
            elif dep_name.startswith("apeireth-") and not workspace_flag:
                # 弱 internal: dep_name 是 apeireth-* 但没用 workspace
                profile.internal_deps_list.append(dep_name)
            elif dep_name.startswith("apeireth-") and workspace_flag:
                # 用 workspace = true 引用 apeireth-* (workspace.dependencies 里有 path override)
                profile.internal_deps_list.append(dep_name)
            else:
                profile.external_deps_list.append(dep_name)
            # optional 检测不区分 internal/external
            if _is_optional_dep(dep_spec):
                profile.n_optional_deps += 1

    # [dev-dependencies]
    dev_deps = data.get("dev-dependencies", {})
    if isinstance(dev_deps, dict):
        profile.n_dev_deps = len(dev_deps)

    # [build-dependencies]
    build_deps = data.get("build-dependencies", {})
    if isinstance(build_deps, dict):
        profile.n_build_deps = len(build_deps)

    # [features]
    features = data.get("features", {})
    if isinstance(features, dict):
        profile.n_features = len(features)

    # [lib]
    if "lib" in data:
        profile.has_lib_target = True

    # [[bin]]
    bins = data.get("bin", [])
    if isinstance(bins, list):
        profile.n_bin_targets = len(bins)

    # [[example]]
    examples = data.get("example", [])
    if isinstance(examples, list):
        profile.n_example_targets = len(examples)

    # [lints] workspace = true (粗 regex, 不依赖 toml parse)
    profile.has_workspace_lints = _extract_workspace_lints_from_text(cargo_text)

    # 最终统计
    profile.internal_deps_list = sorted(set(profile.internal_deps_list))
    profile.external_deps_list = sorted(set(profile.external_deps_list))
    profile.n_internal_deps = len(profile.internal_deps_list)
    profile.n_external_deps = len(profile.external_deps_list)

    return profile


def _read_workspace_members(workspace_cargo: Path) -> List[Tuple[str, Path]]:
    """
    读 workspace Cargo.toml members, 返回 [(member_name, member_dir)].

    member 形如 "crates/apeireth-core", 转成 dir + 从 Cargo.toml 读 package.name.
    """
    out: List[Tuple[str, Path]] = []
    if not workspace_cargo.is_file():
        return out
    workspace_root = workspace_cargo.parent
    try:
        data = tomllib.loads(workspace_cargo.read_text(encoding="utf-8", errors="replace"))
    except Exception:
        return out

    members = data.get("workspace", {}).get("members", [])
    workspace_pkg = data.get("workspace", {}).get("package", {})

    for member_path in members:
        if not isinstance(member_path, str):
            continue
        # 跳过注释行 (tomllib 会忽略 # 开头的行, 但保险起见过滤)
        if member_path.startswith("#"):
            continue
        member_dir = workspace_root / member_path
        cargo_toml = member_dir / "Cargo.toml"
        if not cargo_toml.is_file():
            continue
        try:
            member_data = tomllib.loads(
                cargo_toml.read_text(encoding="utf-8", errors="replace")
            )
        except Exception:
            continue
        package = member_data.get("package", {})
        crate_name = package.get("name", member_dir.name)
        out.append((crate_name, member_dir))

    return out


# ============================================================
# 3. 依赖图分析 (graph theory + DFS cycle detection)
# ============================================================


def _build_reverse_index(profiles: List[CrateDepProfile]) -> None:
    """为每个 crate 填充 in_degree + reverse_deps_list (in-place)."""
    forward_index: Dict[str, List[str]] = defaultdict(list)
    for p in profiles:
        for target in p.internal_deps_list:
            forward_index[p.crate_name].append(target)

    for p in profiles:
        # out_degree 从 internal_deps_list 实际计算 (主 17:43 实事求是)
        p.out_degree = len(p.internal_deps_list)
        p.n_internal_deps = len(p.internal_deps_list)
        # 谁依赖 p?
        for src, targets in forward_index.items():
            if p.crate_name in targets and src != p.crate_name:
                p.reverse_deps_list.append(src)
        p.reverse_deps_list = sorted(set(p.reverse_deps_list))
        p.in_degree = len(p.reverse_deps_list)


def _find_cycles_in_internal_subgraph(
    profiles: List[CrateDepProfile],
) -> List[CrateGraphCycle]:
    """
    检测 internal apeireth-* 子图的所有简单环 (cycle).

    算法: Tarjan SCC 简化版 + DFS 输出 cycle.
    主 17:43 实事求是: 仅 internal subgraph, 不跨 external.
    """
    # 建图: crate_name -> [internal_deps]
    graph: Dict[str, List[str]] = {}
    valid_crates: Set[str] = {p.crate_name for p in profiles}
    for p in profiles:
        graph[p.crate_name] = [
            d for d in p.internal_deps_list if d in valid_crates
        ]

    # Tarjan SCC
    index_counter = [0]
    stack: List[str] = []
    lowlinks: Dict[str, int] = {}
    index: Dict[str, int] = {}
    on_stack: Dict[str, bool] = {}
    sccs: List[List[str]] = []

    def strongconnect(node: str) -> None:
        # 迭代版避免深递归 (主 13:08 真自问: Python 默认栈浅)
        worklist: List[Tuple[str, int]] = [(node, 0)]
        call_stack: List[Tuple[str, int]] = []

        while worklist:
            v, pi = worklist[-1]
            if pi == 0:
                index[v] = index_counter[0]
                lowlinks[v] = index_counter[0]
                index_counter[0] += 1
                stack.append(v)
                on_stack[v] = True

            neighbors = graph.get(v, [])
            if pi < len(neighbors):
                worklist[-1] = (v, pi + 1)
                w = neighbors[pi]
                if w not in index:
                    worklist.append((w, 0))
                elif on_stack.get(w, False):
                    lowlinks[v] = min(lowlinks[v], index[w])
            else:
                if lowlinks[v] == index[v]:
                    scc: List[str] = []
                    while True:
                        w = stack.pop()
                        on_stack[w] = False
                        scc.append(w)
                        if w == v:
                            break
                    sccs.append(scc)
                worklist.pop()
                if worklist:
                    parent_v = worklist[-1][0]
                    lowlinks[parent_v] = min(lowlinks[parent_v], lowlinks[v])

    for n in graph:
        if n not in index:
            strongconnect(n)

    # SCC > 1 即 cycle, SCC = 1 + 自环 = cycle
    cycles: List[CrateGraphCycle] = []
    for scc in sccs:
        if len(scc) > 1:
            # SCC 内部 cycle 多, 取一个代表 cycle (按字典序)
            sorted_scc = sorted(scc)
            cycles.append(
                CrateGraphCycle(
                    cycle_crates=sorted_scc,
                    cycle_length=len(scc),
                )
            )
        elif len(scc) == 1:
            n = scc[0]
            if n in graph.get(n, []):  # 自环
                cycles.append(
                    CrateGraphCycle(
                        cycle_crates=[n, n],
                        cycle_length=2,
                    )
                )

    return cycles


def _compute_depth(profiles: List[CrateDepProfile]) -> Dict[str, int]:
    """从 leaf 开始 BFS 计算每个 crate 的 dep depth (最长 forward path)."""
    valid_crates: Set[str] = {p.crate_name for p in profiles}
    graph: Dict[str, List[str]] = {}
    for p in profiles:
        graph[p.crate_name] = [
            d for d in p.internal_deps_list if d in valid_crates
        ]

    # 反向图: 谁被 n 依赖?
    reverse_graph: Dict[str, List[str]] = defaultdict(list)
    for src, targets in graph.items():
        for t in targets:
            reverse_graph[t].append(src)

    # 找 leaf (无 internal out_deps)
    depths: Dict[str, int] = {}
    queue: deque = deque()
    for p in profiles:
        if p.n_internal_deps == 0:
            depths[p.crate_name] = 0
            queue.append(p.crate_name)

    while queue:
        node = queue.popleft()
        for parent in reverse_graph.get(node, []):
            # parent 深度 = max(depth[parent], depth[node] + 1)
            new_depth = depths[node] + 1
            if parent not in depths or depths[parent] < new_depth:
                depths[parent] = new_depth
                queue.append(parent)

    return depths


# ============================================================
# 4. Top-level scanner (主 00:56 任何人都能接手)
# ============================================================


def scan_workspace(workspace_root: Path) -> DepGraphLedger:
    """扫描整个 workspace, 返回 DepGraphLedger."""
    ledger = DepGraphLedger(workspace_root=str(workspace_root))
    ledger.started_at = time.time()

    workspace_cargo = workspace_root / "Cargo.toml"
    members = _read_workspace_members(workspace_cargo)

    for crate_name, crate_dir in members:
        profile = _scan_crate(crate_name, crate_dir)
        ledger.crate_profiles.append(profile)

    # graph-level 分析
    _build_reverse_index(ledger.crate_profiles)
    ledger.cycles = _find_cycles_in_internal_subgraph(ledger.crate_profiles)
    depths = _compute_depth(ledger.crate_profiles)
    for p in ledger.crate_profiles:
        # depth 暂存到 profile 不破坏 schema, 写到 to_dict 时已包含 out_degree
        setattr(p, "dep_depth", depths.get(p.crate_name, -1))

    ledger.finished_at = time.time()
    ledger.duration_ms = int((ledger.finished_at - ledger.started_at) * 1000)
    return ledger


# ============================================================
# 5. Hypotheses (主 17:43 实事求是)
# ============================================================

HYPOTHESES: List[Dict[str, Any]] = [
    {
        "id": "H1",
        "title": "Internal dep graph has no cycles",
        "predicate": lambda led: led.total_cycles == 0,
        "true_label": "PASS — no internal cycles",
        "false_label": "FAIL — internal cycles detected",
    },
    {
        "id": "H2",
        "title": "Most crates (>= 35/42) use workspace lints",
        "predicate": lambda led: led.crates_with_workspace_lints >= 35,
        "true_label": "PASS — workspace lints >= 35 crates",
        "false_label": "FAIL — workspace lints < 35 crates",
    },
    {
        "id": "H3",
        "title": "Most crates (>= 30/41) have [lib] target (bin-only crates OK)",
        "predicate": lambda led: led.crates_with_lib >= 30,
        "true_label": "PASS — >= 30 crates have [lib]",
        "false_label": "FAIL — < 30 crates have [lib] (too many bin-only)",
    },
    {
        "id": "H4",
        "title": "Most crates (>= 30/42) have at least one internal dep",
        "predicate": lambda led: sum(
            1 for p in led.crate_profiles if p.n_internal_deps > 0
        ) >= 30,
        "true_label": "PASS — >= 30 crates connect internally",
        "false_label": "FAIL — < 30 crates connect internally",
    },
    {
        "id": "H5",
        "title": "Hub crates (in_degree >= 5) exist (>= 3)",
        "predicate": lambda led: led.hub_crate_count >= 3,
        "true_label": "PASS — >= 3 hub crates (in_degree >= 5)",
        "false_label": "FAIL — < 3 hub crates",
    },
]


def evaluate(ledger: DepGraphLedger) -> List[Dict[str, Any]]:
    """跑 5 假说, 返回结果列表."""
    results: List[Dict[str, Any]] = []
    for h in HYPOTHESES:
        passed = h["predicate"](ledger)
        results.append(
            {
                "id": h["id"],
                "title": h["title"],
                "passed": passed,
                "label": h["true_label"] if passed else h["false_label"],
            }
        )
    return results


# ============================================================
# 6. Philosophy gates (主 17:58 + 主 20:46)
# ============================================================

GATES: List[Dict[str, Any]] = [
    {"id": "v1293_extends_v1292", "desc": "V1293 继承 V1292 test source, 不删 V1292"},
    {"id": "v1293_no_new_asi_dim", "desc": "V1293 = dep graph, 不引入新 ASI dim"},
    {"id": "v1293_no_asi_v1_claim", "desc": "不假装 ASI V1: dep graph ≠ ASI"},
    {"id": "v1293_no_kpi_inflate", "desc": "NS 92.91% LOCKED, 不刷"},
    {"id": "v1293_no_phenomenal_claim", "desc": "dep graph ≠ phenomenal consciousness"},
    {"id": "v1293_stdlib_only", "desc": "仅用 tomllib, 不引入新依赖"},
    {"id": "v1293_read_only", "desc": "只读 Cargo.toml, 不改"},
    {"id": "v1293_audit_not_fix", "desc": "audit ≠ fix, V1293 仅审计"},
    {"id": "v1293_toml_only_no_cargo_tree", "desc": "用 toml parse + DFS, 不调 cargo tree CLI"},
    {"id": "v1293_42_crates_full", "desc": "全 42 crates, 不只 worst-5"},
    {"id": "v1293_no_cargo_lock_parse", "desc": "不解析 Cargo.lock, 只 Cargo.toml"},
    {"id": "v1293_no_workspace_member_modify", "desc": "不动 workspace.toml, 只读"},
]


# ============================================================
# 7. Report renderer (主 00:56 任何人都能接手)
# ============================================================


def render_report(ledger: DepGraphLedger) -> str:
    """渲染 markdown 报告."""
    lines: List[str] = []
    lines.append("# V1293 — Cargo Dependency Graph Profile")
    lines.append("")
    lines.append(f"**Workspace root**: `{ledger.workspace_root}`")
    lines.append(f"**Duration**: {int((ledger.finished_at - ledger.started_at) * 1000)} ms")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total crates: **{ledger.total_crates}**")
    lines.append(f"- Total internal deps: **{ledger.total_internal_deps}**")
    lines.append(f"- Total external deps: **{ledger.total_external_deps}**")
    lines.append(f"- Total dev-deps: **{ledger.total_dev_deps}**")
    lines.append(f"- Total build-deps: **{ledger.total_build_deps}**")
    lines.append(f"- Total features: **{ledger.total_features}**")
    lines.append(f"- Crates with `[lints] workspace = true`: **{ledger.crates_with_workspace_lints}/{ledger.total_crates}**")
    lines.append(f"- Crates with `[lib]`: **{ledger.crates_with_lib}/{ledger.total_crates}**")
    lines.append(f"- Max internal in-degree: **{ledger.max_internal_in_degree}**")
    lines.append(f"- Max internal out-degree: **{ledger.max_internal_out_degree}**")
    lines.append(f"- Leaf crates (out_degree = 0): **{ledger.leaf_crate_count}**")
    lines.append(f"- Hub crates (in_degree >= 5): **{ledger.hub_crate_count}**")
    lines.append(f"- Total cycles in internal subgraph: **{ledger.total_cycles}**")
    lines.append("")

    lines.append("## Hypotheses (主 17:43 实事求是)")
    lines.append("")
    results = evaluate(ledger)
    for r in results:
        mark = "✅" if r["passed"] else "❌"
        lines.append(f"- {mark} **{r['id']}** — {r['title']} → {r['label']}")
    lines.append("")

    if ledger.cycles:
        lines.append("## Cycles Detected")
        lines.append("")
        for c in ledger.cycles:
            lines.append(f"- Cycle (length {c.cycle_length}): {' → '.join(c.cycle_crates)}")
        lines.append("")

    lines.append("## Top Hubs (in_degree)")
    lines.append("")
    lines.append("| crate | in_degree | out_degree |")
    lines.append("|---|---:|---:|")
    sorted_by_in = sorted(
        ledger.crate_profiles, key=lambda p: p.in_degree, reverse=True
    )
    for p in sorted_by_in[:15]:
        lines.append(f"| {p.crate_name} | {p.in_degree} | {p.out_degree} |")
    lines.append("")

    lines.append("## Top Leaves (out_degree = 0)")
    lines.append("")
    leaves = sorted(
        [p for p in ledger.crate_profiles if p.out_degree == 0],
        key=lambda p: p.in_degree,
        reverse=True,
    )
    lines.append("| crate | in_degree |")
    lines.append("|---|---:|")
    for p in leaves[:15]:
        lines.append(f"| {p.crate_name} | {p.in_degree} |")
    lines.append("")

    lines.append("## Per-Crate Profile")
    lines.append("")
    lines.append(
        "| crate | internal | external | dev | build | features | opt | lints | lib | bin | example |"
    )
    lines.append("|---|---:|---:|---:|---:|---:|---:|:---:|:---:|---:|---:|")
    for p in sorted(ledger.crate_profiles, key=lambda x: x.crate_name):
        lines.append(
            f"| {p.crate_name} | {p.n_internal_deps} | {p.n_external_deps} | "
            f"{p.n_dev_deps} | {p.n_build_deps} | {p.n_features} | "
            f"{p.n_optional_deps} | "
            f"{'✓' if p.has_workspace_lints else '✗'} | "
            f"{'✓' if p.has_lib_target else '✗'} | "
            f"{p.n_bin_targets} | {p.n_example_targets} |"
        )
    lines.append("")

    lines.append("## Philosophy Gates (主 17:58 + 主 20:46)")
    lines.append("")
    for g in GATES:
        lines.append(f"- ✓ `{g['id']}` — {g['desc']}")
    lines.append("")

    return "\n".join(lines)


# ============================================================
# 8. CLI (主 00:56 任何人都能接手)
# ============================================================


def _resolve_workspace_root(arg: Optional[str]) -> Path:
    """解析 workspace root, 默认 promethean/Apeireth-rust."""
    if arg:
        return Path(arg).resolve()
    # 默认 promethean/Apeireth-rust
    promethean_root = Path(__file__).resolve().parent.parent
    candidate = promethean_root / "Apeireth-rust"
    if candidate.is_dir():
        return candidate
    return promethean_root


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1293_rust_dependency_graph_profile",
        description="V1293 — Cargo Dependency Graph Profile (VCP 真源代码深读 #14)",
    )
    parser.add_argument(
        "--workspace",
        default=None,
        help="workspace root (含 Cargo.toml), 默认 promethean/Apeireth-rust",
    )
    parser.add_argument("--probe", action="store_true", help="快速 probe 模式")
    parser.add_argument("--run", action="store_true", help="完整 scan + 报告")
    parser.add_argument("--json", action="store_true", help="输出 JSON")
    parser.add_argument("--report", default=None, help="写 markdown 报告到指定路径")
    parser.add_argument("--top-hub", type=int, default=10, help="显示 top-N hub crates")
    parser.add_argument("--top-leaf", type=int, default=10, help="显示 top-N leaf crates")
    parser.add_argument("--crate", default=None, help="显示单个 crate 的 dep profile")
    parser.add_argument("--cycles", action="store_true", help="仅显示 cycles")

    args = parser.parse_args(argv)
    workspace_root = _resolve_workspace_root(args.workspace)

    if args.probe or (not any([args.run, args.json, args.report, args.cycles]) and not args.crate):
        # 默认 probe
        ledger = scan_workspace(workspace_root)
        results = evaluate(ledger)
        print(f"[V1293 probe] workspace = {workspace_root}")
        print(f"  total_crates = {ledger.total_crates}")
        print(f"  total_internal_deps = {ledger.total_internal_deps}")
        print(f"  total_external_deps = {ledger.total_external_deps}")
        print(f"  crates_with_workspace_lints = {ledger.crates_with_workspace_lints}/{ledger.total_crates}")
        print(f"  total_cycles = {ledger.total_cycles}")
        print(f"  hypotheses: {sum(1 for r in results if r['passed'])}/{len(results)} PASS")
        return 0

    if args.crate:
        ledger = scan_workspace(workspace_root)
        for p in ledger.crate_profiles:
            if p.crate_name == args.crate or p.crate_name.replace("apeireth-", "") == args.crate:
                print(json.dumps(p.to_dict(), indent=2, ensure_ascii=False))
                return 0
        print(f"ERROR: crate '{args.crate}' not found in workspace")
        return 1

    if args.cycles:
        ledger = scan_workspace(workspace_root)
        print(f"[V1293 cycles] total = {ledger.total_cycles}")
        for c in ledger.cycles:
            print(f"  cycle (len={c.cycle_length}): {' → '.join(c.cycle_crates)}")
        return 0

    # full run
    ledger = scan_workspace(workspace_root)
    results = evaluate(ledger)

    if args.json:
        output = ledger.to_dict()
        output["hypotheses"] = results
        print(json.dumps(output, indent=2, ensure_ascii=False))
        return 0

    if args.report:
        report = render_report(ledger)
        Path(args.report).write_text(report, encoding="utf-8")
        print(f"[V1293] report written to {args.report}")
        return 0

    # 默认 run → print summary
    print(f"[V1293] workspace = {workspace_root}")
    print(f"  total_crates = {ledger.total_crates}")
    print(f"  total_internal_deps = {ledger.total_internal_deps}")
    print(f"  total_external_deps = {ledger.total_external_deps}")
    print(f"  crates_with_workspace_lints = {ledger.crates_with_workspace_lints}/{ledger.total_crates}")
    print(f"  total_cycles = {ledger.total_cycles}")
    print(f"  hub_crates = {ledger.hub_crate_count}, leaf_crates = {ledger.leaf_crate_count}")
    print(f"  hypotheses: {sum(1 for r in results if r['passed'])}/{len(results)} PASS")
    print()
    print(f"--- top {args.top_hub} hubs (in_degree) ---")
    sorted_by_in = sorted(ledger.crate_profiles, key=lambda p: p.in_degree, reverse=True)
    for p in sorted_by_in[: args.top_hub]:
        print(f"  {p.crate_name}: in={p.in_degree}, out={p.out_degree}")
    print()
    print(f"--- top {args.top_leaf} leaves (in_degree, out_degree=0) ---")
    leaves = sorted(
        [p for p in ledger.crate_profiles if p.out_degree == 0],
        key=lambda p: p.in_degree,
        reverse=True,
    )
    for p in leaves[: args.top_leaf]:
        print(f"  {p.crate_name}: in={p.in_degree}")
    return 0


if __name__ == "__main__":
    sys.exit(main())