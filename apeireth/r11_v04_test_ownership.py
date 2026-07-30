"""R11 V0.4 真测闭合 — AST-based test ownership utility (R11 backend, 主 17:43 实事求是).

Why this exists (主 17:43 实事求是):
  V1106 / V1060 / V1077 三处都自己手写 ``test_{module_stem}.py`` 是否存在的检查;
  但仓库里 80+ 真测试用了 ``test_vXXXX.py`` 这种短名, 不带完整模块 stem.
  这些测试通过 ``import`` 或 ``import from apeireth.v1xxx_... import ...`` 显式引用
  目标模块, 是真生产测试覆盖. 旧逻辑把它们全部漏掉, 让 V0.4 engineering
  维度从 0.27 (15/110) 拉低到远低于 V0.4 目标. 这不是改权重/常数刷分, 而是修
  复 V0.4 真测链路上的真实数据访问 bug.

真借鉴 (主 19:33):
  - Python 3.13 ``ast`` stdlib (literal_eval 替代 = 严格语法树)
  - pytest collection: ``test_*.py`` filename pattern + module-level import
  - Bazel 2017 ``visibility()`` 启发: "ownership" 由显式 import 决定, 不由字符串匹配

R11 真生产契约:
  - ``find_tests_owning_module(module_path, tests_dir) -> List[Path]``
    返回所有 *严格* 通过 AST import 引用目标模块的 test 文件 (精确, 注释/字符串忽略).
  - ``aggregate_v04_test_ownership(...)`` 跨 V1000-V1110 (排除 self) 返回总数.
  - ``compute_v04_engineering_score()`` 严格 = 旧公式 + AST test ownership 修正.

不假装守门 (主 17:58 + 主 20:46):
  - 不假装 test exists = module covered: 必须 AST import 显式证据.
  - 不假装 has_import = 静态 grep: 字符串/docstring 包含不算, 必须是 ``import`` /
    ``from ... import`` 节点.
  - 不假装 lift = ASI 突破: 修正工程 = 修正工程, ASI = ASI (主 22:33).
"""
from __future__ import annotations

import ast
import re
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Tuple

R11_OWNER_VERSION = "0.1.0"

V3_GUARDS: Dict[str, str] = {
    "ownership_is_not_coverage": (
        "AST import 证据 ≠ 测试全过. Ownership 仅证明 test 文件真 import 该模块, "
        "不代表该模块的所有行为已被覆盖."
    ),
    "static_grep_is_not_ownership": (
        "字符串/docstring 包含模块名 ≠ 真测试归属. 必须 import / from import 节点."
    ),
    "test_count_is_not_asi": (
        "测试数量本身 ≠ ASI 突破. 工程 lift = 工程 lift, ASI = ASI (主 22:33)."
    ),
}


# ---------------------------------------------------------------------------
# Constants + path helpers (主 17:43 实事求是: 显式路径, 不靠 __file__ 假设)
# ---------------------------------------------------------------------------

DEFAULT_TEST_DIR_NAMES: Tuple[str, ...] = ("tests",)
DEFAULT_TEST_FILE_PREFIX = "test_"
DEFAULT_MODULE_PREFIX = "apeireth."

# 排除规则: V1106 / V1136 / V1138 等 capability/measurement 模块自身不应被
# "discover" 进入统计, 避免自我提升. 这是历史 V1101 设计的延续.
DEFAULT_SELF_EXCLUDE: Tuple[int, ...] = (1106,)

# 默认扫描 V1000..V1110 (V1101 拓到 1110); 上限放宽, 但 self 排除永远生效.
DEFAULT_MIN_NUM = 1000
DEFAULT_MAX_NUM = 1110


def _module_number(stem: str) -> Optional[int]:
    """提取 v* 文件名开头的整数编号, 如 'v1074_asi_production_runner' -> 1074."""
    m = re.match(r"v(\d+)", stem)
    if not m:
        return None
    try:
        return int(m.group(1))
    except ValueError:
        return None


def _resolve_apeireth_dir(module_path: Optional[Path] = None) -> Path:
    """Return the apeireth/ source directory."""
    if module_path is not None:
        return Path(module_path)
    return Path(__file__).resolve().parent


def _resolve_test_dir(apeireth_dir: Path,
                      test_dir_names: Sequence[str] = DEFAULT_TEST_DIR_NAMES) -> Path:
    """Return the tests/ directory, falling back to the first existing candidate."""
    parent = apeireth_dir.parent
    for name in test_dir_names:
        cand = parent / name
        if cand.is_dir():
            return cand
    # 默认仍返回 apeireth/../tests, 后续方法会因不存在而返回空集.
    return parent / test_dir_names[0]


# ---------------------------------------------------------------------------
# AST ownership detection
# ---------------------------------------------------------------------------


def _candidate_test_paths(test_dir: Path, num: int) -> List[Path]:
    """Return candidate test files for a given module number.

    真借鉴 (pytest 命名约定): 优先 ``test_{full_stem}.py``, 短名兜底 ``test_v{num}*.py``.
    """
    out: List[Path] = []
    if not test_dir.is_dir():
        return out
    # short-name glob (e.g. test_v1074.py, test_v1001_v1010.py) — enumerate first
    out.extend(sorted(test_dir.glob(f"{DEFAULT_TEST_FILE_PREFIX}v{num}*.py")))
    return out


def _test_owns_module(test_path: Path, module_stem: str) -> bool:
    """AST-based ownership check: does the test really import this module?

    Strict rules (主 17:43 实事求是: 不靠文本匹配):
      - ``import apeireth.{stem}`` or ``import {stem}``
      - ``from apeireth import *`` where * contains {stem}
      - ``from apeireth.{stem} import ...``

    Non-owners: docstrings/comments containing the stem; non-import mentions
    (e.g. ``"{stem}"`` placeholder inside a function body) are deliberately
    ignored because the goal is *test ownership* not *string co-occurrence*.
    """
    if not test_path.is_file():
        return False
    try:
        src = test_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    try:
        tree = ast.parse(src, filename=str(test_path))
    except SyntaxError:
        return False

    full_name = f"{DEFAULT_MODULE_PREFIX}{module_stem}"
    short_name = module_stem
    package = "apeireth"

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name in (full_name, short_name):
                    return True
        elif isinstance(node, ast.ImportFrom):
            module = node.module or ""
            # ``from apeireth import foo`` where foo is our module stem
            if module == package and any(a.name == short_name for a in node.names):
                return True
            # ``from apeireth.v1xxx_... import ...``
            if module == full_name:
                return True
            # ``from apeireth.v1xxx_... import a, b, c`` (module stem matches)
            # already covered above with module == full_name check.
    return False


def find_tests_owning_module(
    module_stem: str,
    apeireth_dir: Optional[Path] = None,
    test_dir: Optional[Path] = None,
) -> List[Path]:
    """Return every test file that *strictly* owns ``module_stem``.

    Exact match ``test_{module_stem}.py`` is always considered an owner.
    Short-name tests ``test_v{num}*.py`` are accepted only when AST proves
    they import this module.
    """
    apeireth_dir = _resolve_apeireth_dir(apeireth_dir)
    test_dir = test_dir or _resolve_test_dir(apeireth_dir)

    owners: List[Path] = []
    exact = test_dir / f"{DEFAULT_TEST_FILE_PREFIX}{module_stem}.py"
    if exact.is_file():
        owners.append(exact)

    num = _module_number(module_stem)
    if num is not None:
        for cand in _candidate_test_paths(test_dir, num):
            if cand == exact:
                continue
            if _test_owns_module(cand, module_stem):
                owners.append(cand)
    # Stable, deterministic order for tests/CLI reproducibility
    owners.sort()
    return owners


# ---------------------------------------------------------------------------
# Aggregator (V1000-V1110 sweep) — used by V1106 via dependency injection
# ---------------------------------------------------------------------------


@dataclass
class ModuleTestOwnership:
    """Per-module test ownership record."""

    module_stem: str
    module_num: int
    module_path: str
    has_exact_test: bool
    short_test_owners: List[str] = field(default_factory=list)
    total_owners: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def aggregate_v04_test_ownership(
    apeireth_dir: Optional[Path] = None,
    test_dir: Optional[Path] = None,
    min_num: int = DEFAULT_MIN_NUM,
    max_num: int = DEFAULT_MAX_NUM,
    self_exclude: Sequence[int] = DEFAULT_SELF_EXCLUDE,
) -> Dict[str, Any]:
    """Sweep V1000..V1110 (excluding self) and aggregate test ownership.

    Returns a JSON-serializable dict with:
        - total: int (modules considered)
        - with_test: int (any owner found)
        - without_test: int
        - exact: int
        - short_only: int (owned only via AST, not by exact filename)
        - per_module: List[ModuleTestOwnership]
        - method: str
    """
    apeireth_dir = _resolve_apeireth_dir(apeireth_dir)
    test_dir = test_dir or _resolve_test_dir(apeireth_dir)

    per_module: List[ModuleTestOwnership] = []
    total = with_test = exact = short_only = 0
    self_exclude_set = set(self_exclude)

    if not apeireth_dir.is_dir():
        return {
            "total": 0,
            "with_test": 0,
            "without_test": 0,
            "exact": 0,
            "short_only": 0,
            "per_module": [],
            "method": "r11_ast_ownership",
            "version": R11_OWNER_VERSION,
        }

    for fpath in sorted(apeireth_dir.glob("v*.py")):
        stem = fpath.stem
        num = _module_number(stem)
        if num is None or num < min_num or num > max_num:
            continue
        if num in self_exclude_set:
            continue

        total += 1
        exact_path = (test_dir / f"{DEFAULT_TEST_FILE_PREFIX}{stem}.py") if test_dir.is_dir() else None
        has_exact = bool(exact_path and exact_path.is_file())
        if has_exact:
            exact += 1

        short_owners: List[str] = []
        for cand in _candidate_test_paths(test_dir, num):
            if exact_path and cand == exact_path:
                continue
            if _test_owns_module(cand, stem):
                short_owners.append(cand.name)

        total_owners = (1 if has_exact else 0) + len(short_owners)
        if total_owners > 0:
            with_test += 1
            if not has_exact and short_owners:
                short_only += 1

        per_module.append(
            ModuleTestOwnership(
                module_stem=stem,
                module_num=num,
                module_path=str(fpath.resolve()),
                has_exact_test=has_exact,
                short_test_owners=short_owners,
                total_owners=total_owners,
            )
        )

    return {
        "total": total,
        "with_test": with_test,
        "without_test": total - with_test,
        "exact": exact,
        "short_only": short_only,
        "coverage_ratio": (with_test / total) if total else 0.0,
        "exact_ratio": (exact / total) if total else 0.0,
        "short_ratio": (short_only / total) if total else 0.0,
        "per_module": [m.to_dict() for m in per_module],
        "method": "r11_ast_ownership",
        "version": R11_OWNER_VERSION,
    }


# ---------------------------------------------------------------------------
# Score bridge — keeps V1106 formula intact, only the test_coverage signal is fixed
# ---------------------------------------------------------------------------


def compute_v04_engineering_score(
    apeireth_dir: Optional[Path] = None,
    test_dir: Optional[Path] = None,
    min_num: int = DEFAULT_MIN_NUM,
    max_num: int = DEFAULT_MAX_NUM,
    self_exclude: Sequence[int] = DEFAULT_SELF_EXCLUDE,
) -> Dict[str, Any]:
    """V1106-aligned score with AST test ownership replacement.

    Reuses V1106's 0.5 * test_coverage + 0.3 * capability_density + 0.2 * utility_presence
    formula but feeds it the *real* test coverage signal. Weights are NOT changed.
    """
    try:
        from apeireth.v1106_engineering_lift import (
            ENGINEERING_CAPABILITIES,
            ENGINEERING_CAPABILITIES_LIST,
            discover_modules_with_capabilities,
        )
    except Exception as exc:  # pragma: no cover - V1106 should always import
        return {
            "score": 0.0,
            "raw": {"error": f"v1106 import failed: {exc}"},
            "method": "r11_v04_engineering_score",
            "weights": {
                "test_coverage": 0.5,
                "capability_density": 0.3,
                "utility_presence": 0.2,
            },
            "v3_guards": V3_GUARDS,
        }

    ownership = aggregate_v04_test_ownership(
        apeireth_dir=apeireth_dir,
        test_dir=test_dir,
        min_num=min_num,
        max_num=max_num,
        self_exclude=self_exclude,
    )

    # Delegate capability counting to V1106's existing AST helper so numbers stay
    # in sync with the legacy "method=ast_grep_capabilities" path.
    legacy = discover_modules_with_capabilities(
        module_dir=str(apeireth_dir) if apeireth_dir else "",
        min_num=min_num,
        max_num=max_num,
    )

    total = ownership["total"]
    if total == 0:
        return {
            "score": 0.0,
            "raw": {
                "ownership": ownership,
                "legacy": legacy,
            },
            "method": "r11_v04_engineering_score",
            "weights": {
                "test_coverage": 0.5,
                "capability_density": 0.3,
                "utility_presence": 0.2,
            },
            "v3_guards": V3_GUARDS,
        }

    test_cov = ownership["coverage_ratio"]
    cap_dens = (legacy.get("with_capabilities", 0) / total) if total else 0.0
    utility_present = 1.0 if len(ENGINEERING_CAPABILITIES) >= 10 else 0.0
    score = max(0.0, min(1.0, 0.5 * test_cov + 0.3 * cap_dens + 0.2 * utility_present))

    return {
        "score": float(score),
        "raw": {
            "ownership": ownership,
            "legacy": legacy,
            "test_coverage_ratio": test_cov,
            "capability_density_ratio": cap_dens,
            "utility_presence": utility_present,
            "utility_size": len(ENGINEERING_CAPABILITIES),
            "weights": {
                "test_coverage": 0.5,
                "capability_density": 0.3,
                "utility_presence": 0.2,
            },
        },
        "method": "r11_v04_engineering_score",
        "version": R11_OWNER_VERSION,
        "v3_guards": V3_GUARDS,
    }


# ---------------------------------------------------------------------------
# CLI: 主 00:56 任何人都能接手. Run: python -m apeireth.r11_v04_test_ownership --report
# ---------------------------------------------------------------------------


def _cli(argv: Optional[List[str]] = None) -> int:
    import argparse
    import json as _json

    parser = argparse.ArgumentParser(
        prog="r11_v04_test_ownership",
        description=(
            "R11 V0.4 AST test-ownership utility (主 17:43 实事求是): "
            "严格 import-based test coverage signal, no string grep, no fake KPI."
        ),
    )
    parser.add_argument("--module", default=None,
                        help="If set, list test files owning this exact module stem.")
    parser.add_argument("--json", action="store_true",
                        help="Emit JSON summary of V1000-V1110 aggregation.")
    parser.add_argument("--report", action="store_true",
                        help="Emit Markdown report of V1000-V1110 aggregation.")
    parser.add_argument("--score", action="store_true",
                        help="Compute V0.4 engineering score (AST ownership based).")
    parser.add_argument("--quiet", action="store_true",
                        help="Suppress human-readable output for --score.")
    parser.add_argument("--out", default=None,
                        help="Write Markdown report to this file (default stdout).")
    args = parser.parse_args(argv)

    if args.module:
        owners = find_tests_owning_module(args.module)
        print(_json.dumps([str(p) for p in owners], ensure_ascii=False, indent=2))
        return 0

    if args.json:
        agg = aggregate_v04_test_ownership()
        # Keep per_module compact for CLI consumers
        summary = {k: v for k, v in agg.items() if k != "per_module"}
        summary["per_module_count"] = len(agg["per_module"])
        print(_json.dumps(summary, ensure_ascii=False, indent=2))
        return 0

    if args.score:
        r = compute_v04_engineering_score()
        if not args.quiet:
            print(_json.dumps(r, ensure_ascii=False, indent=2))
        else:
            print(f"r11_v04_engineering_score = {r['score']:.4f}")
        return 0

    if args.report:
        agg = aggregate_v04_test_ownership()
        score = compute_v04_engineering_score()["score"]
        lines = [
            "# R11 V0.4 AST test-ownership Report",
            "",
            f"**Version**: {R11_OWNER_VERSION}",
            f"**Method**: {agg['method']}",
            "",
            "## 真覆盖信号 (主 17:43 实事求是: 严格 AST import)",
            "",
            f"- 总模块: {agg['total']}",
            f"- 有真测试: {agg['with_test']}",
            f"- 无真测试: {agg['without_test']}",
            f"- 真测试覆盖率: {agg['coverage_ratio']:.4f}",
            f"- exact 命中: {agg['exact']}",
            f"- short-only (AST 证据): {agg['short_only']}",
            "",
            "## V0.4 engineering 真分 (公式不变, 仅 test_coverage 信号修复)",
            "",
            f"- score: **{score:.4f}**",
            "- 公式: 0.5 × test_coverage + 0.3 × capability_density + 0.2 × utility_presence",
            "- 不刷 KPI: 权重 / 常数 全部未变 (主 17:43 实事求是).",
            "",
            "## V3 哲学守门",
            "",
        ]
        for name, desc in V3_GUARDS.items():
            lines.append(f"- {name}: {desc}")
        lines.append("")
        text = "\n".join(lines)
        if args.out:
            Path(args.out).write_text(text, encoding="utf-8")
            print(f"[r11] report written to {args.out}")
        else:
            print(text)
        return 0

    parser.print_help()
    return 0


__all__ = [
    "R11_OWNER_VERSION",
    "V3_GUARDS",
    "ModuleTestOwnership",
    "find_tests_owning_module",
    "aggregate_v04_test_ownership",
    "compute_v04_engineering_score",
]


if __name__ == "__main__":
    import sys
    sys.exit(_cli())
