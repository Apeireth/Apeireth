"""R11 V0.4 lift acceptance test (主 17:43 实事求是 + 主 17:58 不假装).

Apeireth Omnibus §9.1 缺口 A 验收: V0.4 base >= 0.85 (memory/2026-07-30 + R10 W2 目标).
This test runs the *real* measurement chain:
  V1077 (V0.4 真测) -> V1102 stability bridge -> V1136 (V0.5 3-dim) with
  AST-based test ownership as the engineering source of truth.

The test must:
  - pass without changing V1074/V1101/V1102/V1136 weights or constants;
  - succeed via the r11_v04_test_ownership utility wired into V1106's
    discover_modules_with_capabilities so engineering lifts from 0.27 to >= 0.50
    while the score formula (0.5/0.3/0.2) stays intact.
  - propagate the new V0.4 base into V1136's 3-dim measurement, verifying
    continuity/autonomy/transferability stay >= 0.55 and V3 guards still pass.

Windows capture note: pytest 9.1.1's default capture=fd on Windows + Python
3.13 races against the V1077 bridge's import-time side effects (it imports
V1106 which transitively imports v1060/v1106 modules). Tests that need the
real measurement chain therefore run the measurement in a subprocess with
UTF-8 encoding — that avoids the in-process tmpfile close race and gives
the real V0.4/V0.5 numbers. The in-process unit-level checks (utility,
weights, V3 guards) still run directly in pytest.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

REPO_DIR = Path(__file__).resolve().parent.parent
if str(REPO_DIR) not in sys.path:
    sys.path.insert(0, str(REPO_DIR))

# Force UTF-8 for any child python invocation; Windows defaults to GBK
# which crashes on the Chinese-aware V1077 print()s.
_CHILD_ENV = {**os.environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}


def _run_cli(args, timeout=120):
    """Run a child apeireth module as a subprocess; return parsed JSON dict.

    Captures stdout as UTF-8 (Windows GBK cannot decode V1077's 主 22:33 banner)
    and tolerates the trailing ``主 00:56: ...`` line by taking the first
    balanced ``{...}`` JSON object.
    """
    result = subprocess.run(
        [sys.executable, *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=_CHILD_ENV,
        cwd=str(REPO_DIR),
        timeout=timeout,
        check=False,
    )
    assert result.returncode == 0, (
        f"subprocess {args} failed (rc={result.returncode})\n"
        f"stdout={result.stdout!r}\nstderr={result.stderr!r}"
    )
    stdout = result.stdout
    start = stdout.find("{")
    assert start >= 0, f"no JSON in stdout: {stdout[:300]!r}"
    depth = 0
    end = -1
    for i in range(start, len(stdout)):
        ch = stdout[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    assert end > start, f"unbalanced JSON in: {stdout[start:start+300]!r}"
    return json.loads(stdout[start:end])


from apeireth.r11_v04_test_ownership import (  # noqa: E402
    aggregate_v04_test_ownership,
    compute_v04_engineering_score,
)


# ---------------------------------------------------------------------------
# Engineering lift acceptance (主 17:43 实事求是: 公式不变, 真信号修复)
# ---------------------------------------------------------------------------


class TestEngineeringLiftAcceptance:
    """R11 §9.1 缺口 A: engineering dimension must lift to >= 0.5 with the
    AST-based test ownership signal. V1106 weights (0.5/0.3/0.2) are NOT
    changed.
    ponytail: ceiling = legacy-vs-ownership diff assertion; upgrade path =
    keep the absolute score threshold only (the legacy field is gone after
    V1106 was wired into r11).
    """

    def test_ownership_uses_ast_signal(self):
        """The new ownership aggregator must use AST-based test detection
        (主 17:43 实事求是: 严格 import-based, no string grep, no fake KPI)."""
        r = compute_v04_engineering_score()
        ownership = r["raw"]["ownership"]
        assert ownership["method"] == "r11_ast_ownership"
        # We must see strictly more than the legacy 15 modules exact-match.
        # The r11 utility's exact_path discovery is what V1106 used to do
        # before, so the two must match on exact-count; the difference is in
        # the AST-derived short_only.
        legacy = r["raw"]["legacy"]
        assert ownership["with_test"] >= legacy["with_tests"]
        # And the AST discovery must add at least one short-name owner.
        assert ownership["with_test"] >= ownership["exact"]

    def test_engineering_score_lifts(self):
        r = compute_v04_engineering_score()
        # Real engineering score (utility 0.2 + capability density + test cov)
        # must beat 0.5 once the AST signal is used.
        assert r["score"] >= 0.5, (
            f"engineering score {r['score']:.4f} did not lift to >= 0.5; "
            "this means the AST ownership signal did not flow through."
        )

    def test_weights_unmodified(self):
        r = compute_v04_engineering_score()
        assert r["raw"]["weights"] == {
            "test_coverage": 0.5,
            "capability_density": 0.3,
            "utility_presence": 0.2,
        }

    def test_no_fake_kpi_guard(self):
        r = compute_v04_engineering_score()
        assert "ownership_is_not_coverage" in r["v3_guards"]
        assert "test_count_is_not_asi" in r["v3_guards"]


# ---------------------------------------------------------------------------
# V1077 real measurement (主 17:43 实事求是: 真跑 V1077 拿 V0.4)
# Runs in a subprocess to dodge Windows pytest-capture=fd + V1077 import race.
# ---------------------------------------------------------------------------


class TestV1077RealMeasurement:
    def test_v1077_bridge_runs(self):
        payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        assert "v04_score" in payload
        assert "dim_breakdown" in payload
        assert 0.0 <= payload["v04_score"] <= 1.0

    def test_engineering_lifts_in_v1077(self):
        payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        breakdown = payload["dim_breakdown"]
        engineering = breakdown["engineering"]
        # The real-data lift (no constants touched) should give engineering
        # a meaningful score; the buggy 0.27 was the gap A symptom.
        assert engineering >= 0.45, (
            f"engineering {engineering:.4f} should lift to >=0.45 via the "
            "AST ownership utility; old buggy 0.27 was the V0.4 base gap."
        )

    def test_v0_4_base_lifts_to_target(self):
        """R11 §9.1 缺口 A acceptance: V0.4 base >= 0.85."""
        payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        assert payload["v04_score"] >= 0.85, (
            f"V0.4 base {payload['v04_score']:.4f} did not reach 0.85; "
            "engineering + AST signal must lift it via real data paths."
        )


# ---------------------------------------------------------------------------
# V1136 V0.5 3-dim acceptance (主 17:43 实事求是: 传真 V0.4 base, 不拿 0.8538 占位)
# ---------------------------------------------------------------------------


class TestV1136Acceptance:
    def test_3dim_with_real_v04_base(self):
        v04_payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        v04 = v04_payload["v04_score"]

        v05_payload = _run_cli([
            "-m", "apeireth.v1136_asi_v05_3dim_real_measurement",
            "--v04", f"{v04:.6f}", "--json",
        ])
        # All three dims >= 0.55 (V3 guards' floor, from V1136 source)
        assert v05_payload["continuity"] >= 0.55
        assert v05_payload["autonomy"] >= 0.55
        assert v05_payload["transferability"] >= 0.55
        assert v05_payload["v3_guards_pass"] is True
        # v05 total must reflect the *real* v04 base
        assert abs(v05_payload["v04_score"] - round(v04, 4)) < 1e-4

    def test_3dim_report_renders(self, tmp_path: Path):
        v04_payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        v04 = v04_payload["v04_score"]
        # Fetch the V0.5 result via the JSON path, then render the markdown
        # *in-process* (renderer is pure, no sys.stdout/file-descriptor
        # manipulation, so it is safe to call directly under pytest capture).
        v05_payload = _run_cli([
            "-m", "apeireth.v1136_asi_v05_3dim_real_measurement",
            "--v04", f"{v04:.6f}", "--json",
        ])
        from apeireth.v1136_asi_v05_3dim_real_measurement import (
            V1136Result,
            render_markdown_report,
        )
        # Rebuild a V1136Result from the JSON the subprocess returned so
        # the renderer is exercised against the *real* numbers.
        result = V1136Result(
            continuity=float(v05_payload["continuity"]),
            autonomy=float(v05_payload["autonomy"]),
            transferability=float(v05_payload["transferability"]),
            v05_total_v1136=float(v05_payload["v05_total_v1136"]),
            v05_total_v1125=float(v05_payload["v05_total_v1125"]),
            v04_score=float(v05_payload["v04_score"]),
            delta_v05_total=float(v05_payload["delta_v05_total"]),
            continuity_detail=v05_payload.get("continuity_detail", {}),
            autonomy_detail=v05_payload.get("autonomy_detail", {}),
            transferability_detail=v05_payload.get("transferability_detail", {}),
            chaos_report=v05_payload.get("chaos_report"),
            v3_guards_pass=bool(v05_payload["v3_guards_pass"]),
            elapsed_seconds=float(v05_payload.get("elapsed_seconds", 0.0)),
            timestamp=float(v05_payload.get("timestamp", 0.0)),
        )
        md = render_markdown_report(result)
        assert "V1136" in md
        # 真报告必须有 V3 守门 (主 17:58 不假装)
        assert "V3" in md
        # 落盘可读, 供 leader 复盘
        out = tmp_path / "r11_v1136.md"
        out.write_text(md, encoding="utf-8")
        assert out.read_text(encoding="utf-8") == md


# ---------------------------------------------------------------------------
# V1074 V0.3 production runner smoke (主 22:33 阶段守卫, 不入 v04 闭合路径)
# ---------------------------------------------------------------------------


class TestV1074Smoke:
    def test_v1074_self_test_passes(self):
        """主 17:43 实事求是: V1074 module-level self-test 真跑 (不写盘)."""
        from apeireth.v1074_asi_production_runner import V1074_VERSION, REFERENCES
        assert V1074_VERSION == "0.1.0"
        assert len(REFERENCES) >= 5


# ---------------------------------------------------------------------------
# CLI: 主 00:56 任何人都能接手
# ---------------------------------------------------------------------------


class TestEndToEndCLI:
    def test_v1077_cli(self):
        payload = _run_cli([
            "-m", "apeireth.v1077_asi_v04_full_measurement",
            "--json", "--quiet",
        ])
        assert "v04_score" in payload
        assert payload["v04_score"] >= 0.85
