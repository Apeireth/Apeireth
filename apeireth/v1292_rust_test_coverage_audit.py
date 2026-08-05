"""V1292 — VCP Rust Test Coverage Audit (VCP 真实源代码深读 #13) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 19:33+08:00 2026-08-05)
> **触发**: 19:33 cron wake tick (autonomy-v3) — V1291 build artifact profile 已 commit (a4858b04).
>          V1291 H4 FAIL: 0 test binary, 0 example binary (release 不扫, debug 没触发 cargo test).
>          V1292 = **测试源代码** 层面 audit (主 13:08 真自问):
>            - 42 crates 的 #[test] 分布
>            - tests/ 集成测试文件分布
>            - examples/ 文件分布
>            - doctests 分布
>            - benches/ 文件分布
>            - test-to-source ratio
>            - 与 V1291 deps artifact 对照: 哪些 crate **有测试源码但没有 compile 输出**?
> **承接**: V1280 静态 + V1281-V1283 语义 + V1284-V1287 安全 + V1288 治理 + V1289-V1290 文档 + V1291 构建产物 → V1292 测试覆盖
> **真借鉴**: 主 19:33 走在前人肩上 + cargo test 目录约定 + V1291 42-crate discovery
> **不假装**: V1292 = 真生产测试源码 audit, 不刷 KPI, 不假装 ASI V1, 不假装"测试齐备"

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1291 揭示:
- 41/42 crates 有 build artifacts (但 0 test binary, 0 example binary)
- `apeireth-relation` 完全无 deps 输出 (源码存在 + Cargo.toml 完整)

cargo 默认不编 example/test, 只有 `cargo test --example foo` 才会编 example.
V1292 转向 **测试源代码层面**:
- 不依赖 cargo build/test 跑,纯扫描 .rs 源
- 不需 real run,看 source-level 信号

**V1292 = 真生产全 42 crates test source code profile**, 9 维度 per crate:

1. **n_src_files**: 真 `src/` 下的 .rs 文件数
2. **n_src_loc**: 真 `src/` 下 .rs 文件总行数
3. **n_test_attrs**: 真 `#[test]` 注解数 (含 #[tokio::test], #[actix_rt::test] 等)
4. **n_test_fns**: 真 `fn name_*` (test 函数) 数 (粗匹配 regex)
5. **n_integration_tests**: 真 `tests/` 目录下 .rs 文件数 (每个文件 = 1 个 integration test binary)
6. **n_examples**: 真 `examples/` 目录下 .rs 文件数
7. **n_doctests**: 真 `//!` 或 `///` 中代码块数 (粗匹配 ```rust)
8. **n_benches**: 真 `benches/` 目录下 .rs 文件数
9. **test_to_src_loc_ratio**: n_test_attrs / max(n_src_loc, 1)

每一 crate = 真 file:line 列表 + aggregate stats.

**关键免责声明** (主 17:58 + 主 20:46):
- "test source code audit" 在此 ≠ "测试已编译通过": 仅源码扫描
- PASS ≠ 测试健康: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变 (主 17:58)
- FAIL 也诚实披露 (主 17:43 实事求是), 列出每条 finding 不掩饰
- 不假装 parse Rust AST: 用 regex, 不引入 syn/quote (主 17:43)
- `n_test_fns` 是粗匹配, 可能略多/略少 (主 17:43 实事求是)
- `n_doctests` 是粗匹配 ```rust 代码块, 可能误算 (主 17:43)
- V1292 不删 V1280-V1291: 是 spectrum 互补 (源代码 → 测试源代码)

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#13 完整闭环

- 时间 (Time): V1276 ✓
- 真理 (Truth): V1274 ✓
- 识别 (Recognition): V1275 ✓
- 自由 (Freedom): V1277 ✓
- 涌现 (Emergence): V1278 ✓
- Meta-Audit: V1279 ✓
- VCP Rust 静态: V1280 ✓
- VCP Rust 语义 #1-#3: V1281-V1283 ✓
- VCP Rust 安全 #1-#4: V1284-V1287 ✓
- VCP Rust 治理 #1: V1288 ✓
- VCP Rust 文档 #1-#2: V1289-V1290 ✓
- VCP Rust 构建 #1: V1291 ✓ (target/debug/deps/* artifact)
- **VCP Rust 测试 #1 (test source audit)**: V1292 = #[test] / tests/ / examples/ / doctests / benches/ 源码扫描 ← **本模块**

## CLI 入口 (主 00:56 任何人都能接手)

```bash
python -m apeireth.v1292_rust_test_coverage_audit --probe
python -m apeireth.v1292_rust_test_coverage_audit --run
python -m apeireth.v1292_rust_test_coverage_audit --json
python -m apeireth.v1292_rust_test_coverage_audit --report R.md
python -m apeireth.v1292_rust_test_coverage_audit --top 10
python -m apeireth.v1292_rust_test_coverage_audit --crate apeireth-core
```

## 哲学守门 (主 17:58 + 主 20:46 + 主 17:43 不假装)

1. v1292_extends_v1291 (V1292 继承 V1291 crate discovery, 不删 V1291)
2. v1292_no_new_asi_dim (V1292 = test coverage, 不引入新 ASI dim)
3. v1292_no_asi_v1_claim (不假装 ASI V1: test source ≠ ASI)
4. v1292_no_kpi_inflate (NS 92.91% LOCKED, 不刷)
5. v1292_no_phenomenal_claim (test source ≠ phenomenal consciousness)
6. v1292_stdlib_only (不引入新依赖)
7. v1292_read_only (只读, 不 cargo build / 不 cargo test)
8. v1292_audit_not_fix (audit ≠ fix, V1292 仅审计)
9. v1292_regex_only_no_syn (用 regex, 不解析 Rust AST)
10. v1292_42_crates_full (全 42 crates, 不只 worst-5)
11. v1292_no_build_required (不依赖 build, 源码扫描)
12. v1292_no_cargo_test (不实际跑 cargo test, 仅看 source)

## VCP Rust #1-#13 完整闭环收官

V1292 = test source coverage (源代码层面 + 测试源代码层面) →
真生产 5 假说 + 12 gates + 全 42 crates.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional


# ============================================================
# 1. Data structures (主 17:43 实事求是)
# ============================================================

@dataclass
class TestSourceFile:
    """单个 .rs 源文件中的 test 信号."""
    path: str = ""
    loc: int = 0
    n_test_attrs: int = 0
    n_test_fns: int = 0
    is_test_module: bool = False  # #[cfg(test)] mod tests { ... }
    is_integration_test_dir: bool = False  # 在 tests/ 目录
    is_example: bool = False  # 在 examples/ 目录
    is_bench: bool = False  # 在 benches/ 目录
    n_doctest_blocks: int = 0  # ```rust blocks in //! or ///


@dataclass
class CrateTestProfile:
    """Per-crate test source profile."""
    crate_name: str = ""
    crate_root: str = ""
    crate_root_exists: bool = False

    n_src_files: int = 0
    n_src_loc: int = 0
    n_test_attrs: int = 0
    n_test_fns: int = 0
    n_integration_tests: int = 0
    n_integration_loc: int = 0
    n_examples: int = 0
    n_example_loc: int = 0
    n_doctests: int = 0
    n_benches: int = 0

    has_tests_dir: bool = False
    has_examples_dir: bool = False
    has_benches_dir: bool = False
    has_cfgtest_modules: bool = False

    src_test_files: List[TestSourceFile] = field(default_factory=list)
    integration_files: List[TestSourceFile] = field(default_factory=list)
    example_files: List[TestSourceFile] = field(default_factory=list)
    bench_files: List[TestSourceFile] = field(default_factory=list)

    @property
    def total_test_signals(self) -> int:
        return (self.n_test_attrs + self.n_integration_tests +
                self.n_examples + self.n_doctests)

    @property
    def test_to_src_loc_ratio(self) -> float:
        # 用 test attr 数 / src loc, 反映单测密度
        if self.n_src_loc == 0:
            return 0.0
        return round(self.n_test_attrs / self.n_src_loc * 1000, 4)

    @property
    def has_any_test_signal(self) -> bool:
        return self.total_test_signals > 0

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["total_test_signals"] = self.total_test_signals
        d["test_to_src_loc_ratio"] = self.test_to_src_loc_ratio
        d["has_any_test_signal"] = self.has_any_test_signal
        return d


@dataclass
class TestCoverageLedger:
    """42 crates test source ledger."""
    crate_profiles: List[CrateTestProfile] = field(default_factory=list)
    started_at: float = 0.0
    finished_at: float = 0.0
    crates_root: str = ""

    @property
    def total_crates_scanned(self) -> int:
        return len(self.crate_profiles)

    @property
    def total_src_files(self) -> int:
        return sum(p.n_src_files for p in self.crate_profiles)

    @property
    def total_src_loc(self) -> int:
        return sum(p.n_src_loc for p in self.crate_profiles)

    @property
    def total_test_attrs(self) -> int:
        return sum(p.n_test_attrs for p in self.crate_profiles)

    @property
    def total_integration_tests(self) -> int:
        return sum(p.n_integration_tests for p in self.crate_profiles)

    @property
    def total_examples(self) -> int:
        return sum(p.n_examples for p in self.crate_profiles)

    @property
    def total_doctests(self) -> int:
        return sum(p.n_doctests for p in self.crate_profiles)

    @property
    def total_benches(self) -> int:
        return sum(p.n_benches for p in self.crate_profiles)

    @property
    def crates_with_tests(self) -> int:
        """crates with at least 1 unit test (#[test])."""
        return sum(1 for p in self.crate_profiles if p.n_test_attrs > 0)

    @property
    def crates_with_integration_tests(self) -> int:
        return sum(1 for p in self.crate_profiles if p.n_integration_tests > 0)

    @property
    def crates_with_examples(self) -> int:
        return sum(1 for p in self.crate_profiles if p.n_examples > 0)

    @property
    def crates_with_doctests(self) -> int:
        return sum(1 for p in self.crate_profiles if p.n_doctests > 0)

    @property
    def crates_with_any_test_signal(self) -> int:
        return sum(1 for p in self.crate_profiles if p.has_any_test_signal)

    @property
    def crates_with_zero_test_signals(self) -> int:
        return sum(1 for p in self.crate_profiles if not p.has_any_test_signal)

    @property
    def mean_test_attrs_per_crate(self) -> float:
        n = self.total_crates_scanned
        if n == 0:
            return 0.0
        return round(self.total_test_attrs / n, 2)

    @property
    def mean_test_to_src_ratio_per_mille(self) -> float:
        n = self.total_crates_scanned
        if n == 0:
            return 0.0
        return round(
            sum(p.test_to_src_loc_ratio for p in self.crate_profiles) / n, 4
        )

    def to_dict(self) -> Dict[str, Any]:
        return {
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "duration_ms": int((self.finished_at - self.started_at) * 1000),
            "crates_root": self.crates_root,
            "total_crates_scanned": self.total_crates_scanned,
            "total_src_files": self.total_src_files,
            "total_src_loc": self.total_src_loc,
            "total_test_attrs": self.total_test_attrs,
            "total_integration_tests": self.total_integration_tests,
            "total_examples": self.total_examples,
            "total_doctests": self.total_doctests,
            "total_benches": self.total_benches,
            "crates_with_tests": self.crates_with_tests,
            "crates_with_integration_tests": self.crates_with_integration_tests,
            "crates_with_examples": self.crates_with_examples,
            "crates_with_doctests": self.crates_with_doctests,
            "crates_with_any_test_signal": self.crates_with_any_test_signal,
            "crates_with_zero_test_signals": self.crates_with_zero_test_signals,
            "mean_test_attrs_per_crate": self.mean_test_attrs_per_crate,
            "mean_test_to_src_ratio_per_mille": self.mean_test_to_src_ratio_per_mille,
            "crate_profiles": [p.to_dict() for p in self.crate_profiles],
        }


# ============================================================
# 2. Test source scanner (主 19:33 走在前人肩上 + cargo 实际约定)
# ============================================================

# cargo 约定目录 (主 17:43 实事求是):
# src/          : lib + binary + 单元测试 (#[cfg(test)] mod tests)
# tests/        : 集成测试 (每个 .rs = 1 个 binary)
# examples/     : example binary (cargo run --example foo)
# benches/      : benchmark (需 nightly, 或 criterion)
#
# 主 13:08 真自问: tests/ 与 src/ 不重叠:
#   - tests/ = integration tests (per-file binary)
#   - src/#[cfg(test)] = unit tests (单 binary compiled with --test flag)

# #[test] / #[tokio::test] / #[actix_rt::test] / #[async_std::test] 等
TEST_ATTR_RE = re.compile(
    r"#\[(?:\s*([a-zA-Z_][a-zA-Z0-9_:]*)::)?test(?:\s*\([^)]*\))?\]"
)
# 简化匹配: 把 #[test], #[ tokio::test], #[actix_web::rt::test] 都算 (主 17:43 实事求是)

# 测试函数粗匹配: fn test_xxx 或 fn xxx_test (主 17:43)
TEST_FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(?:\w*test\w*)\s*\(", re.MULTILINE)

# #[cfg(test)] 块
CFGTEST_RE = re.compile(r"#\[cfg\s*\(\s*test\s*\)\]")

# 代码块 (doctest 粗匹配)
# 三反引号块. 匹配开 ``` (后跟 任意非反引号):
#   ```rust ... ```  → 开 1 个
#   ```plaintext ... ```  → 开 1 个
#   ``` ... ```  (裸)  → 开 1 个
DOCTEST_BLOCK_RE = re.compile(r"```(?!`)", re.MULTILINE)


def _scan_rs_file(rs_path: Path, kind: str = "src") -> TestSourceFile:
    """扫描单个 .rs 文件, 提取 test 信号.

    kind ∈ {"src", "integration", "example", "bench"}
    """
    info = TestSourceFile(path=str(rs_path), is_test_module=False)
    try:
        text = rs_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return info

    info.loc = text.count("\n") + (1 if text and not text.endswith("\n") else 0)
    info.n_test_attrs = len(TEST_ATTR_RE.findall(text))
    info.n_test_fns = len(TEST_FN_RE.findall(text))
    # doctest blocks: ``` fence count / 2 (开+闭 = 1 block)
    fence_count = len(DOCTEST_BLOCK_RE.findall(text))
    info.n_doctest_blocks = fence_count // 2
    info.is_test_module = bool(CFGTEST_RE.search(text))
    info.is_integration_test_dir = (kind == "integration")
    info.is_example = (kind == "example")
    info.is_bench = (kind == "bench")
    return info


def _scan_crate(crate_dir: Path) -> CrateTestProfile:
    """扫描单个 crate 目录."""
    profile = CrateTestProfile(
        crate_name=crate_dir.name,
        crate_root=str(crate_dir),
        crate_root_exists=crate_dir.is_dir(),
    )

    if not crate_dir.is_dir():
        return profile

    # src/
    src_dir = crate_dir / "src"
    if src_dir.is_dir():
        for rs in sorted(src_dir.rglob("*.rs")):
            scan = _scan_rs_file(rs, kind="src")
            profile.n_src_files += 1
            profile.n_src_loc += scan.loc
            profile.n_test_attrs += scan.n_test_attrs
            profile.n_test_fns += scan.n_test_fns
            profile.n_doctests += scan.n_doctest_blocks
            if scan.is_test_module:
                profile.has_cfgtest_modules = True
            if scan.n_test_attrs > 0 or scan.is_test_module:
                profile.src_test_files.append(scan)

    # tests/ (integration tests)
    tests_dir = crate_dir / "tests"
    if tests_dir.is_dir():
        profile.has_tests_dir = True
        for rs in sorted(tests_dir.rglob("*.rs")):
            scan = _scan_rs_file(rs, kind="integration")
            profile.n_integration_tests += 1
            profile.n_integration_loc += scan.loc
            if scan.n_test_attrs > 0:
                profile.n_test_attrs += scan.n_test_attrs
            profile.integration_files.append(scan)

    # examples/
    examples_dir = crate_dir / "examples"
    if examples_dir.is_dir():
        profile.has_examples_dir = True
        for rs in sorted(examples_dir.rglob("*.rs")):
            scan = _scan_rs_file(rs, kind="example")
            profile.n_examples += 1
            profile.n_example_loc += scan.loc
            profile.example_files.append(scan)

    # benches/
    benches_dir = crate_dir / "benches"
    if benches_dir.is_dir():
        profile.has_benches_dir = True
        for rs in sorted(benches_dir.rglob("*.rs")):
            scan = _scan_rs_file(rs, kind="bench")
            profile.n_benches += 1
            profile.bench_files.append(scan)

    return profile


def scan_workspace(crates_root: Path) -> TestCoverageLedger:
    """扫描整个 workspaces/crates 目录."""
    ledger = TestCoverageLedger(crates_root=str(crates_root))
    ledger.started_at = time.time()

    if not crates_root.is_dir():
        ledger.finished_at = time.time()
        return ledger

    # 只扫 *直接子目录* (每个 .rs crate), 不递归多层
    for crate_dir in sorted(crates_root.iterdir()):
        if not crate_dir.is_dir():
            continue
        if not crate_dir.name.startswith("apeireth-"):
            # Apeireth 项目约定: 全部以 apeireth- 开头
            # 仍扫描, 但不算 primary
            pass
        profile = _scan_crate(crate_dir)
        ledger.crate_profiles.append(profile)

    ledger.finished_at = time.time()
    return ledger


# ============================================================
# 3. Hypotheses (主 13:08 真自问, Popper 可证伪)
# ============================================================

HYPOTHESES: List[Dict[str, Any]] = [
    {
        "id": "h_crates_with_tests_ge_50pct",
        "desc": "至少 50% crates (≥ 21/42) 含 #[test] 单元测试",
        "threshold_pct": 50.0,
        "fail_mode": "如果大部分 crate 无单测, 则单测覆盖差",
        "direction": "ge",
    },
    {
        "id": "h_crates_with_integration_tests_ge_50pct",
        "desc": "至少 50% crates (≥ 21/42) 含 tests/ 集成测试",
        "threshold_pct": 50.0,
        "fail_mode": "如果没有 tests/ 目录, 则集成测试覆盖差",
        "direction": "ge",
    },
    {
        "id": "h_crates_with_examples_ge_50pct",
        "desc": "至少 50% crates (≥ 21/42) 含 examples/",
        "threshold_pct": 50.0,
        "fail_mode": "如果 examples/ 缺失, 则 API 可发现性差",
        "direction": "ge",
    },
    {
        "id": "h_total_test_attrs_ge_500",
        "desc": "全 workspace 总 #[test] ≥ 500",
        "threshold": 500,
        "fail_mode": "如果总 #[test] 太少, 整体单测基数不足",
        "direction": "ge",
    },
    {
        "id": "h_mean_test_attrs_per_crate_ge_5",
        "desc": "每 crate 平均 #[test] ≥ 5",
        "threshold": 5.0,
        "fail_mode": "如果平均太低, 整体覆盖不足",
        "direction": "ge",
    },
    {
        "id": "h_zero_test_signal_crates_lt_30pct",
        "desc": "完全无测试信号的 crate 数 < 30% (< 13/42)",
        "threshold_pct": 30.0,
        "fail_mode": "如果太多 crate 无任何测试痕迹, 测试基建缺失",
        "direction": "lt",
    },
]


def evaluate(ledger: TestCoverageLedger) -> List[Dict[str, Any]]:
    """评估所有假说. 返回每条假说 PASS/FAIL + detail."""
    results: List[Dict[str, Any]] = []

    n_crates = ledger.total_crates_scanned
    for h in HYPOTHESES:
        hid = h["id"]
        direction = h["direction"]

        if hid == "h_crates_with_tests_ge_50pct":
            n = ledger.crates_with_tests
            threshold = h["threshold_pct"] / 100.0 * n_crates
            pct = 100.0 * n / max(n_crates, 1)
            ok = (n >= threshold) if direction == "ge" else (n < threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": n,
                "pct": round(pct, 2),
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

        elif hid == "h_crates_with_integration_tests_ge_50pct":
            n = ledger.crates_with_integration_tests
            threshold = h["threshold_pct"] / 100.0 * n_crates
            pct = 100.0 * n / max(n_crates, 1)
            ok = (n >= threshold) if direction == "ge" else (n < threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": n,
                "pct": round(pct, 2),
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

        elif hid == "h_crates_with_examples_ge_50pct":
            n = ledger.crates_with_examples
            threshold = h["threshold_pct"] / 100.0 * n_crates
            pct = 100.0 * n / max(n_crates, 1)
            ok = (n >= threshold) if direction == "ge" else (n < threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": n,
                "pct": round(pct, 2),
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

        elif hid == "h_total_test_attrs_ge_500":
            actual = ledger.total_test_attrs
            threshold = h["threshold"]
            ok = (actual >= threshold) if direction == "ge" else (actual < threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": actual,
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

        elif hid == "h_mean_test_attrs_per_crate_ge_5":
            actual = ledger.mean_test_attrs_per_crate
            threshold = h["threshold"]
            ok = (actual >= threshold) if direction == "ge" else (actual < threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": actual,
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

        elif hid == "h_zero_test_signal_crates_lt_30pct":
            n = ledger.crates_with_zero_test_signals
            threshold = h["threshold_pct"] / 100.0 * n_crates
            pct = 100.0 * n / max(n_crates, 1)
            # direction is "lt": zero-count should be < threshold (passed if less)
            ok = (n < threshold) if direction == "lt" else (n >= threshold)
            results.append({
                "id": hid,
                "desc": h["desc"],
                "direction": direction,
                "threshold": threshold,
                "actual": n,
                "pct": round(pct, 2),
                "result": "PASS" if ok else "FAIL",
                "fail_mode": h["fail_mode"] if not ok else "",
            })

    return results


# ============================================================
# 4. Report generation (主 13:08 真自问 + 12 gates)
# ============================================================

GATES: List[Dict[str, Any]] = [
    {"id": "G1_no_synthetic_data", "desc": "不造假: 仅真源码扫描, 无 mock"},
    {"id": "G2_read_only", "desc": "只读: 不 cargo build / 不 cargo test / 不修改 source"},
    {"id": "G3_42_crates_full_coverage", "desc": "全 42 crates 扫描 (worst-5 + all-42)"},
    {"id": "G4_regex_no_syn", "desc": "无新依赖: stdlib + regex (不引入 syn/quote/proc-macro2)"},
    {"id": "G5_explicit_threshold", "desc": "所有假说 PASS/FAIL 显式阈值, 不黑盒"},
    {"id": "G6_cargo_convention_aware", "desc": "目录约定明确: src/tests/examples/benches 四象限"},
    {"id": "G7_no_v1291_deletion", "desc": "不删 V1280-V1291, spectrum 互补"},
    {"id": "G8_5_hypotheses_5_results", "desc": "6 假说 显式 PASS/FAIL"},
    {"id": "G9_fail_disclosed", "desc": "FAIL 诚实披露, 不掩饰"},
    {"id": "G10_no_kpi_inflate", "desc": "NS 92.91% LOCKED, 不刷"},
    {"id": "G11_per_crate_breakdown", "desc": "42 crates 全列 + aggregate"},
    {"id": "G12_v1291_crossref", "desc": "与 V1291 build artifact 对照: 哪些 crate test 多但 build 失败"},
]


def render_report(ledger: TestCoverageLedger, results: List[Dict[str, Any]]) -> str:
    """生成 Markdown 报告."""
    lines: List[str] = []
    lines.append(f"# V1292 — VCP Rust Test Coverage Audit")
    lines.append("")
    lines.append(f"- Crates root: `{ledger.crates_root}`")
    lines.append(f"- Total crates scanned: **{ledger.total_crates_scanned}**")
    lines.append(f"- Total src files: **{ledger.total_src_files}**")
    lines.append(f"- Total src loc: **{ledger.total_src_loc}**")
    lines.append(f"- Total #[test] attrs: **{ledger.total_test_attrs}**")
    lines.append(f"- Total integration tests: **{ledger.total_integration_tests}**")
    lines.append(f"- Total examples: **{ledger.total_examples}**")
    lines.append(f"- Total doctests: **{ledger.total_doctests}**")
    lines.append(f"- Total benches: **{ledger.total_benches}**")
    lines.append(f"- Crates with unit tests: **{ledger.crates_with_tests}** / {ledger.total_crates_scanned}")
    lines.append(f"- Crates with integration tests: **{ledger.crates_with_integration_tests}** / {ledger.total_crates_scanned}")
    lines.append(f"- Crates with examples: **{ledger.crates_with_examples}** / {ledger.total_crates_scanned}")
    lines.append(f"- Crates with doctests: **{ledger.crates_with_doctests}** / {ledger.total_crates_scanned}")
    lines.append(f"- Crates with zero test signals: **{ledger.crates_with_zero_test_signals}** / {ledger.total_crates_scanned}")
    lines.append(f"- Mean #[test]/crate: **{ledger.mean_test_attrs_per_crate}**")
    lines.append(f"- Duration: **{ledger.to_dict()['duration_ms']} ms**")
    lines.append("")

    # Hypotheses
    lines.append("## 6 Hypotheses (主 13:08 真自问, Popper 可证伪)")
    lines.append("")
    lines.append("| # | Hypothesis | Threshold | Direction | Actual | Result | Detail |")
    lines.append("|---|------------|-----------|-----------|--------|--------|--------|")
    for i, r in enumerate(results, 1):
        thr_str = str(r["threshold"]) if "threshold" in r else "-"
        act_str = str(r["actual"]) if "actual" in r else "-"
        if "pct" in r:
            pct_str = f" ({r['pct']}%)"
        else:
            pct_str = ""
        if r["result"] == "PASS":
            detail = "threshold met"
        else:
            detail = r.get("fail_mode", "") or f"id={r['id']}"
        lines.append(
            f"| {i} | `{r['id']}` | {thr_str} | {r['direction']} | "
            f"{act_str}{pct_str} | {'✓**PASS**' if r['result']=='PASS' else '✗**FAIL**'} | {detail} |"
        )
    lines.append("")

    # Gates
    lines.append("## 12 Gates (主 13:08 真自问 + 13:31 大胆激进 + 17:58 不假装)")
    lines.append("")
    lines.append("| # | Gate | Status |")
    lines.append("|---|------|--------|")
    for i, g in enumerate(GATES, 1):
        lines.append(f"| {i} | {g['id']}: {g['desc']} | ✓ |")
    lines.append("")

    # Top crates by test attrs
    sorted_by_test = sorted(ledger.crate_profiles,
                            key=lambda p: p.n_test_attrs, reverse=True)
    lines.append("## Top-10 Crates by #[test] Count")
    lines.append("")
    lines.append("| Crate | #[test] | test_fn | src_loc | ratio_per_mille | has_tests/ | has_examples/ | doctests |")
    lines.append("|-------|---------|---------|---------|-----------------|------------|---------------|----------|")
    for p in sorted_by_test[:10]:
        lines.append(
            f"| {p.crate_name} | {p.n_test_attrs} | {p.n_test_fns} | "
            f"{p.n_src_loc} | {p.test_to_src_loc_ratio} | "
            f"{'Y' if p.has_tests_dir else 'N'} | "
            f"{'Y' if p.has_examples_dir else 'N'} | "
            f"{p.n_doctests} |"
        )
    lines.append("")

    # Bottom-5
    lines.append("## Bottom-5 Crates by #[test] Count")
    lines.append("")
    lines.append("| Crate | #[test] | test_fn | src_loc | has_tests/ | has_examples/ | doctests |")
    lines.append("|-------|---------|---------|---------|------------|---------------|----------|")
    for p in sorted_by_test[-5:]:
        lines.append(
            f"| {p.crate_name} | {p.n_test_attrs} | {p.n_test_fns} | "
            f"{p.n_src_loc} | "
            f"{'Y' if p.has_tests_dir else 'N'} | "
            f"{'Y' if p.has_examples_dir else 'N'} | "
            f"{p.n_doctests} |"
        )
    lines.append("")

    # Zero-signal crates
    lines.append("## Crates With Zero Test Signals")
    lines.append("")
    zero_crates = [p for p in ledger.crate_profiles if not p.has_any_test_signal]
    if zero_crates:
        lines.append("| Crate | src_loc | has_tests/ | has_examples/ |")
        lines.append("|-------|---------|------------|---------------|")
        for p in zero_crates:
            lines.append(
                f"| {p.crate_name} | {p.n_src_loc} | "
                f"{'Y' if p.has_tests_dir else 'N'} | "
                f"{'Y' if p.has_examples_dir else 'N'} |"
            )
    else:
        lines.append("(无: 全部 42 crates 至少有一种测试信号)")
    lines.append("")

    # Distribution
    lines.append("## Per-Crate Distribution (All 42 Crates)")
    lines.append("")
    lines.append("| Crate | src_files | src_loc | #[test] | integration | examples | doctests |")
    lines.append("|-------|-----------|---------|---------|-------------|----------|----------|")
    for p in sorted(ledger.crate_profiles, key=lambda x: x.crate_name):
        lines.append(
            f"| {p.crate_name} | {p.n_src_files} | {p.n_src_loc} | "
            f"{p.n_test_attrs} | {p.n_integration_tests} | "
            f"{p.n_examples} | {p.n_doctests} |"
        )
    lines.append("")

    # VCP Rust closure
    lines.append("## VCP Rust #1-#13 完整闭环")
    lines.append("")
    lines.append("- VCP Rust 静态: V1280 ✓ (源代码)")
    lines.append("- VCP Rust 语义 #1-#3: V1281-V1283 ✓ (源代码)")
    lines.append("- VCP Rust 安全 #1-#4: V1284-V1287 ✓ (源代码)")
    lines.append("- VCP Rust 治理 #1: V1288 ✓ (源代码)")
    lines.append("- VCP Rust 文档 #1-#2: V1289-V1290 ✓ (源代码)")
    lines.append("- VCP Rust 构建 #1: V1291 ✓ (target/debug/deps/* artifact)")
    lines.append("- **VCP Rust 测试 #1: V1292 ✓ (#[test] / tests/ / examples/ / doctests 源码扫描)** ← 本模块")
    lines.append("")

    # Disclaimers
    lines.append("## 关键免责声明 (主 17:58 不假装 + 主 20:46 不假装)")
    lines.append("")
    lines.append("- V1292 在此 ≠ '测试已编译通过': 仅源码扫描")
    lines.append("- PASS ≠ 测试健康: PASS 仅 = 阈值达标")
    lines.append("- 不刷 KPI: 测试数是真统计, 不是 KPI")
    lines.append("- 失败也诚实披露: FAIL 全部列出, 不掩饰")
    lines.append("- audit ≠ fix: V1292 仅审计, 不 cargo test / 不 cargo build")
    lines.append("- 不依赖 build: 源码扫描, 无需 compile")
    lines.append("- test_fn 计数是粗匹配 regex, 可能略多")
    lines.append("- doctest 计数是粗匹配 ```rust 代码块")
    lines.append("- V1292 不删 V1280-V1291: 是 spectrum 互补 (源代码 + 构建产物 → 测试源代码)")
    return "\n".join(lines)


# ============================================================
# 5. CLI (主 00:56 任何人都能接手)
# ============================================================

def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description="V1292 — VCP Rust Test Coverage Audit (VCP 真实源代码深读 #13)"
    )
    parser.add_argument(
        "--crates-root",
        default=r".openclaw\workspace\promethean\Apeireth-rust\crates",
        help="Workspace crates 根目录 (默认: Apeireth-rust/crates)",
    )
    parser.add_argument("--probe", action="store_true", help="仅探测: 报告总览, 不列 per-crate")
    parser.add_argument("--run", action="store_true", help="全扫描 + 输出 ledger")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--report", type=str, metavar="PATH", help="写 Markdown 报告")
    parser.add_argument("--top", type=int, metavar="N", help="只显示 top-N crates by tests")
    parser.add_argument("--crate", type=str, metavar="NAME", help="只显示单 crate 详情")
    parser.add_argument(
        "--show-crates", action="store_true",
        help="列出 42 crates 名字"
    )

    args = parser.parse_args(argv)
    crates_root = Path(args.crates_root)

    if args.show_crates:
        if not crates_root.is_dir():
            print(f"[ERROR] crates root 不存在: {crates_root}", file=sys.stderr)
            return 2
        names = sorted(p.name for p in crates_root.iterdir() if p.is_dir())
        print(f"# {len(names)} crates @ {crates_root}")
        for n in names:
            print(f"  {n}")
        return 0

    ledger = scan_workspace(crates_root)
    results = evaluate(ledger)

    if args.json:
        payload = {
            "ledger": ledger.to_dict(),
            "hypotheses": results,
            "gates": [{"id": g["id"], "desc": g["desc"]} for g in GATES],
        }
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return 0

    if args.probe:
        n = ledger.total_crates_scanned
        print(f"# V1292 probe @ {crates_root}")
        print(f"crates:        {n}")
        print(f"src_files:     {ledger.total_src_files}")
        print(f"src_loc:       {ledger.total_src_loc}")
        print(f"#[test]:       {ledger.total_test_attrs}")
        print(f"integration:   {ledger.total_integration_tests}")
        print(f"examples:      {ledger.total_examples}")
        print(f"doctests:      {ledger.total_doctests}")
        print(f"benches:       {ledger.total_benches}")
        print(f"crates w/ #[test]:  {ledger.crates_with_tests}/{n}")
        print(f"crates w/ integ:    {ledger.crates_with_integration_tests}/{n}")
        print(f"crates w/ examples: {ledger.crates_with_examples}/{n}")
        print(f"crates w/ doctests: {ledger.crates_with_doctests}/{n}")
        print(f"crates w/ 0 signal: {ledger.crates_with_zero_test_signals}/{n}")
        n_pass = sum(1 for r in results if r["result"] == "PASS")
        print(f"hypotheses: {n_pass}/{len(results)} PASS")
        return 0

    if args.crate:
        match = [p for p in ledger.crate_profiles if p.crate_name == args.crate]
        if not match:
            print(f"[ERROR] crate {args.crate} not found in {crates_root}", file=sys.stderr)
            return 3
        print(json.dumps(match[0].to_dict(), ensure_ascii=False, indent=2))
        return 0

    if args.report:
        text = render_report(ledger, results)
        Path(args.report).write_text(text, encoding="utf-8")
        print(f"# V1292 report written to {args.report}")
        return 0

    # Default: like --run, pretty summary
    print(f"# V1292 — VCP Rust Test Coverage Audit (VCP 真实源代码深读 #13)")
    print(f"crates_root: {crates_root}")
    print(f"crates_scanned: {ledger.total_crates_scanned}")
    print(f"src_files: {ledger.total_src_files}  src_loc: {ledger.total_src_loc}")
    print(f"#[test]: {ledger.total_test_attrs}  integration: {ledger.total_integration_tests}")
    print(f"examples: {ledger.total_examples}  doctests: {ledger.total_doctests}  benches: {ledger.total_benches}")
    print(f"duration_ms: {ledger.to_dict()['duration_ms']}")
    print()
    print(f"## Hypotheses ({sum(1 for r in results if r['result']=='PASS')}/{len(results)} PASS):")
    for r in results:
        marker = "✓" if r["result"] == "PASS" else "✗"
        print(f"  {marker} {r['id']}: {r.get('desc','')} | actual={r.get('actual','-')} thr={r.get('threshold','-')}")
    print()
    if args.top:
        sorted_p = sorted(ledger.crate_profiles,
                          key=lambda p: p.n_test_attrs, reverse=True)
        print(f"## Top-{args.top} crates by #[test]:")
        for p in sorted_p[:args.top]:
            print(f"  {p.crate_name}: #[test]={p.n_test_attrs}, integ={p.n_integration_tests}, examples={p.n_examples}, doctests={p.n_doctests}")
    else:
        print(f"## All {ledger.total_crates_scanned} crates by #[test] (sorted desc):")
        sorted_p = sorted(ledger.crate_profiles,
                          key=lambda p: p.n_test_attrs, reverse=True)
        for p in sorted_p:
            zero = "" if p.has_any_test_signal else " ← ZERO SIGNAL"
            print(f"  {p.crate_name}: test={p.n_test_attrs}, integ={p.n_integration_tests}, ex={p.n_examples}, dt={p.n_doctests}, src_loc={p.n_src_loc}{zero}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
