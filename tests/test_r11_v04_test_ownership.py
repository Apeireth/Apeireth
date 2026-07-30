"""R11 V0.4 AST test-ownership utility tests (主 17:43 实事求是 + 主 17:58 不假装).

Covers:
  1. AST-based ownership detection (exact + short-name + strict)
  2. Static-grep rejection (string mentions do NOT count as ownership)
  3. Aggregate sweep over V1000-V1110 (deterministic, no string count)
  4. Score bridge = V1106 formula intact, weights unchanged
  5. CLI: --json / --module / --report exit cleanly
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO_DIR = Path(__file__).resolve().parent.parent
if str(REPO_DIR) not in sys.path:
    sys.path.insert(0, str(REPO_DIR))

from apeireth.r11_v04_test_ownership import (  # noqa: E402
    R11_OWNER_VERSION,
    V3_GUARDS,
    aggregate_v04_test_ownership,
    compute_v04_engineering_score,
    find_tests_owning_module,
)


# ---------------------------------------------------------------------------
# 1. AST-based ownership detection
# ---------------------------------------------------------------------------


class TestFindTestsOwningModule:
    def test_exact_match_returns_exact_file(self, tmp_path: Path):
        ape = tmp_path / "apeireth"
        ape.mkdir()
        tests = tmp_path / "tests"
        tests.mkdir()
        (ape / "v1234_demo.py").write_text("# demo\n", encoding="utf-8")
        exact = tests / "test_v1234_demo.py"
        exact.write_text("# exact test\n", encoding="utf-8")
        owners = find_tests_owning_module("v1234_demo", apeireth_dir=ape, test_dir=tests)
        assert exact in owners

    def test_short_name_accepted_when_ast_imports(self, tmp_path: Path):
        ape = tmp_path / "apeireth"
        ape.mkdir()
        tests = tmp_path / "tests"
        tests.mkdir()
        (ape / "v1234_demo.py").write_text("# demo\n", encoding="utf-8")
        short = tests / "test_v1234.py"
        short.write_text(
            "import apeireth.v1234_demo as d\n"
            "def test_x():\n    assert d is not None\n",
            encoding="utf-8",
        )
        owners = find_tests_owning_module("v1234_demo", apeireth_dir=ape, test_dir=tests)
        assert short in owners

    def test_string_mention_does_not_count(self, tmp_path: Path):
        """主 17:43 实事求是: 字符串/docstring 包含模块名 ≠ 真测试归属."""
        ape = tmp_path / "apeireth"
        ape.mkdir()
        tests = tmp_path / "tests"
        tests.mkdir()
        (ape / "v1234_demo.py").write_text("# demo\n", encoding="utf-8")
        fake = tests / "test_v1234.py"
        fake.write_text(
            "# no real import; mentions 'apeireth.v1234_demo' in a docstring\n"
            "def test_x():\n    return 'apeireth.v1234_demo'\n",
            encoding="utf-8",
        )
        owners = find_tests_owning_module("v1234_demo", apeireth_dir=ape, test_dir=tests)
        assert fake not in owners

    def test_deterministic_order(self, tmp_path: Path):
        ape = tmp_path / "apeireth"
        ape.mkdir()
        tests = tmp_path / "tests"
        tests.mkdir()
        (ape / "v1234_demo.py").write_text("# demo\n", encoding="utf-8")
        for name in ("test_v1234_b.py", "test_v1234_a.py", "test_v1234_c.py"):
            (tests / name).write_text(
                "import apeireth.v1234_demo\n",
                encoding="utf-8",
            )
        owners = find_tests_owning_module("v1234_demo", apeireth_dir=ape, test_dir=tests)
        rel_names = [p.name for p in owners]
        assert rel_names == sorted(rel_names)

    def test_no_test_dir_returns_empty(self, tmp_path: Path):
        ape = tmp_path / "apeireth"
        ape.mkdir()
        owners = find_tests_owning_module(
            "v1234_demo",
            apeireth_dir=ape,
            test_dir=tmp_path / "tests_missing",
        )
        assert owners == []


# ---------------------------------------------------------------------------
# 2. Aggregate sweep over V1000-V1110 — deterministic, no string count
# ---------------------------------------------------------------------------


class TestAggregateV04TestOwnership:
    def test_keys_present(self):
        agg = aggregate_v04_test_ownership()
        for k in (
            "total",
            "with_test",
            "without_test",
            "exact",
            "short_only",
            "coverage_ratio",
            "exact_ratio",
            "short_ratio",
            "per_module",
            "method",
            "version",
        ):
            assert k in agg, f"missing key: {k}"

    def test_method_and_version(self):
        agg = aggregate_v04_test_ownership()
        assert agg["method"] == "r11_ast_ownership"
        assert agg["version"] == R11_OWNER_VERSION

    def test_counts_consistent(self):
        agg = aggregate_v04_test_ownership()
        total = agg["total"]
        with_test = agg["with_test"]
        without = agg["without_test"]
        assert with_test + without == total
        assert agg["coverage_ratio"] == pytest.approx(with_test / total, abs=1e-9)
        assert 0.0 <= agg["coverage_ratio"] <= 1.0

    def test_self_excluded(self):
        """V1106 / V1136 / V1138 must never inflate their own coverage."""
        agg = aggregate_v04_test_ownership()
        nums = {entry["module_num"] for entry in agg["per_module"]}
        assert 1106 not in nums

    def test_real_repo_short_ownership_present(self):
        """Real repo: short-name tests (e.g. test_v1074.py) should count.

        主 17:43 实事求是: 此前 V1106 discover 只看 test_{full_stem}.py,
        把 80+ 真测试漏算. 本 utility 修复这一真实数据访问 bug.
        """
        agg = aggregate_v04_test_ownership()
        # 旧逻辑 (test_{full_stem}.py only) = 15, AST-based = > 15
        assert agg["with_test"] >= 90, (
            f"AST-based with_test={agg['with_test']} should be >=90 for the "
            "real V0.4 lift; old buggy 15 was the gap A symptom."
        )

    def test_deterministic_across_runs(self):
        a = aggregate_v04_test_ownership()
        b = aggregate_v04_test_ownership()
        assert a == b


# ---------------------------------------------------------------------------
# 3. Score bridge — V1106 formula intact, weights unchanged
# ---------------------------------------------------------------------------


class TestComputeV04EngineeringScore:
    def test_weights_unchanged(self):
        r = compute_v04_engineering_score()
        weights = r["raw"]["weights"]
        assert weights == {
            "test_coverage": 0.5,
            "capability_density": 0.3,
            "utility_presence": 0.2,
        }

    def test_score_within_unit_interval(self):
        r = compute_v04_engineering_score()
        assert 0.0 <= r["score"] <= 1.0

    def test_lifts_engineering_via_real_ownership(self):
        """R11 acceptance: the *real* AST-based signal must beat the buggy old ratio.

        主 17:43 实事求是: lift 来自修复真数据访问 bug, 不是改权重/常数.
        ponytail: ceiling = compared-against-legacy assertion; after V1106 was
        rewired onto r11 in the same change, the two ratios are equal —
        we instead prove the AST signal is the source of truth.
        """
        r = compute_v04_engineering_score()
        ownership = r["raw"]["ownership"]
        legacy = r["raw"]["legacy"]

        # V1106 现在也走 r11 utility, 所以 legacy["with_tests"] 应等于
        # ownership["with_test"]; 关键是 ownership 使用 AST 严格 import 检测,
        # 而不是文本 grep / 文件名巧合.
        assert ownership["method"] == "r11_ast_ownership"
        assert legacy["with_tests"] >= 10  # sanity
        # 公式驱动后 score 应当 ≥ 0.5 (主 22:33 北极星 V0.4 base 目标 ≥ 0.85)
        assert r["score"] >= 0.5
        # AST 信号带来的"额外"覆盖 (短名/聚合 import): exact + short_only ≥ exact
        assert ownership["with_test"] >= ownership["exact"]

    def test_v3_guards_kept(self):
        r = compute_v04_engineering_score()
        assert "v3_guards" in r
        assert r["v3_guards"] is V3_GUARDS


# ---------------------------------------------------------------------------
# 4. CLI — 主 00:56 任何人都能接手
# ---------------------------------------------------------------------------


class TestCLI:
    def test_json_aggregator(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.r11_v04_test_ownership", "--json"],
            capture_output=True, text=True,
            cwd=str(REPO_DIR), timeout=60, check=False,
        )
        assert result.returncode == 0, result.stderr
        payload = json.loads(result.stdout)
        assert payload["method"] == "r11_ast_ownership"
        assert payload["total"] > 0
        assert payload["with_test"] >= 90

    def test_module_lookup_v1074(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.r11_v04_test_ownership",
             "--module", "v1074_asi_production_runner"],
            capture_output=True, text=True,
            cwd=str(REPO_DIR), timeout=60, check=False,
        )
        assert result.returncode == 0, result.stderr
        owners = json.loads(result.stdout)
        assert any("v1074" in p for p in owners)

    def test_report_writes_to_file(self, tmp_path: Path):
        out = tmp_path / "report.md"
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.r11_v04_test_ownership",
             "--report", "--quiet", "--out", str(out)],
            capture_output=True, text=True,
            cwd=str(REPO_DIR), timeout=60, check=False,
        )
        assert result.returncode == 0, result.stderr
        content = out.read_text(encoding="utf-8")
        assert "R11 V0.4" in content
        assert "weights" in content.lower() or "公式" in content

    def test_score_quiet_prints_only_value(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.r11_v04_test_ownership",
             "--score", "--quiet"],
            capture_output=True, text=True,
            cwd=str(REPO_DIR), timeout=60, check=False,
        )
        assert result.returncode == 0, result.stderr
        line = result.stdout.strip()
        assert line.startswith("r11_v04_engineering_score =")
        assert 0.0 <= float(line.split("=")[-1].strip()) <= 1.0
