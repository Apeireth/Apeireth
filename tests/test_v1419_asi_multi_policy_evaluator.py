"""V1419 — tests for ASI 总框架 multi-policy evaluator (post-V1418 next-step).

Module: apeireth.v1419_asi_multi_policy_evaluator
Schema: v1419.asi-multi-policy-evaluator/v1
Version: 0.1.0

40 tests across 11 sections.
"""

from __future__ import annotations

import dataclasses
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any, Dict, List

# Make sure apeireth is importable when running directly: `python -m tests.test_v1419...`
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1419_asi_multi_policy_evaluator import (  # noqa: E402
    DEFAULT_EVALUATOR_CONFIG,
    DEFAULT_HISTORY_PATH,
    DEFAULT_LAST_EVAL_PATH,
    DEFAULT_OUT_PATH,
    DEFAULT_THRESHOLD,
    MAX_THRESHOLD,
    MAX_WINDOW_SIZE,
    MIN_THRESHOLD,
    MIN_WINDOW_SIZE,
    MultiPolicyEvaluationReport,
    EvaluatorConfig,
    Severity,
    ShiftAlert,
    ShiftVerdict,
    V1419_BORROWED,
    V1419_GUARDS,
    V1419_MODULE,
    V1419_POLICIES,
    V1419_SCHEMA,
    V1419_SEVERITIES,
    V1419_V3_GUARDS,
    V1419_VERSION,
    WindowComparison,
    WindowDistribution,
    _atomic_write_json,
    _atomic_write_text,
    _safe_path,
    _safe_ratio,
    _window_label,
    _worst_severity,
    build_default_config,
    chain_delegate,
    compare_window_distributions,
    compute_window_distribution,
    detect_shift,
    evaluate,
    popper_self_test,
    render_evaluation_md,
    run_cli,
)


# ============================================================================
# Mock TickSnapshot for tests
# ============================================================================


class MockTickSnapshot:
    """Mimics apeireth.v1417_asi_dgm_tick_history.TickSnapshot."""

    def __init__(
        self,
        policy: str = "PROCEED",
        chain_ok: bool = True,
        alerts_count: int = 0,
        timestamp: str = "2026-08-10T00-00-00Z",
        tick_id: str = "t0",
        max_severity: str = "INFO",
        escalation_count: int = 0,
        n_modules: int = 5,
        n_snapshots_v1415: int = 0,
        note: str = "",
    ) -> None:
        self.policy = policy
        self.chain_ok = chain_ok
        self.alerts_count = alerts_count
        self.timestamp = timestamp
        self.tick_id = tick_id
        self.max_severity = max_severity
        self.escalation_count = escalation_count
        self.n_modules = n_modules
        self.n_snapshots_v1415 = n_snapshots_v1415
        self.note = note


# ============================================================================
# Section 1: TestConstants
# ============================================================================


class TestConstants(unittest.TestCase):
    """Verify V1419 constants are non-empty and correctly structured."""

    def test_001_version_is_string(self) -> None:
        self.assertIsInstance(V1419_VERSION, str)
        self.assertEqual(V1419_VERSION, "0.1.0")

    def test_002_schema_is_v1419(self) -> None:
        self.assertEqual(V1419_SCHEMA, "v1419.asi-multi-policy-evaluator/v1")

    def test_003_module_name(self) -> None:
        self.assertEqual(V1419_MODULE, "v1419_asi_multi_policy_evaluator")

    def test_004_guards_count_at_least_10(self) -> None:
        self.assertGreaterEqual(len(V1419_GUARDS), 10)
        for g in V1419_GUARDS:
            self.assertTrue(g.startswith("GUARD_"))

    def test_005_v3_guards_count_at_least_9(self) -> None:
        self.assertEqual(len(V1419_V3_GUARDS), 9)

    def test_006_borrowed_count_at_least_3(self) -> None:
        self.assertGreaterEqual(len(V1419_BORROWED), 3)
        for entry in V1419_BORROWED:
            self.assertEqual(len(entry), 2)
            self.assertIsInstance(entry[0], str)
            self.assertIsInstance(entry[1], str)

    def test_007_policies_tuple(self) -> None:
        self.assertEqual(V1419_POLICIES, ("PROCEED", "PAUSE", "LOCKDOWN"))

    def test_008_severities_tuple(self) -> None:
        self.assertEqual(V1419_SEVERITIES, ("INFO", "WARN", "CRITICAL"))

    def test_009_default_config_in_bounds(self) -> None:
        self.assertGreaterEqual(
            DEFAULT_EVALUATOR_CONFIG.window_size, MIN_WINDOW_SIZE
        )
        self.assertLessEqual(
            DEFAULT_EVALUATOR_CONFIG.window_size, MAX_WINDOW_SIZE
        )
        self.assertGreaterEqual(DEFAULT_EVALUATOR_CONFIG.threshold, MIN_THRESHOLD)
        self.assertLessEqual(DEFAULT_EVALUATOR_CONFIG.threshold, MAX_THRESHOLD)


# ============================================================================
# Section 2: TestDataclasses
# ============================================================================


class TestDataclasses(unittest.TestCase):
    """Verify dataclasses roundtrip via dataclasses.asdict."""

    def test_010_window_distribution_roundtrip(self) -> None:
        d = WindowDistribution(
            window_label="A:5",
            n_snapshots=5,
            proceed_count=4,
            pause_count=1,
            lockdown_count=0,
            proceed_ratio=0.8,
            pause_ratio=0.2,
            lockdown_ratio=0.0,
            chain_ok_count=4,
            chain_ok_rate=0.8,
            alerts_total=2,
            alerts_avg=0.4,
            first_timestamp="t1",
            last_timestamp="t5",
            note="test",
        )
        d2 = WindowDistribution(**dataclasses.asdict(d))
        self.assertEqual(d, d2)

    def test_011_window_comparison_roundtrip(self) -> None:
        c = WindowComparison(
            window_a_label="A:5",
            window_b_label="B:5",
            delta_proceed_ratio=-0.4,
            delta_pause_ratio=0.2,
            delta_lockdown_ratio=0.2,
            delta_chain_ok_rate=-0.2,
            delta_alerts_avg=1.0,
            shift_verdict="SHIFT",
            shift_magnitude=0.8,
            reason="test reason",
            note="test",
        )
        c2 = WindowComparison(**dataclasses.asdict(c))
        self.assertEqual(c, c2)

    def test_012_shift_alert_roundtrip(self) -> None:
        a = ShiftAlert(
            alert_type="PROCEED_RATIO_SHIFT",
            severity="WARN",
            magnitude=0.3,
            recommendation="Investigate",
            window_a_label="A:5",
            window_b_label="B:5",
            note="test note",
        )
        a2 = ShiftAlert(**dataclasses.asdict(a))
        self.assertEqual(a, a2)

    def test_013_multi_policy_report_roundtrip(self) -> None:
        d = WindowDistribution(
            window_label="A:5", n_snapshots=5, proceed_count=5, pause_count=0,
            lockdown_count=0, proceed_ratio=1.0, pause_ratio=0.0, lockdown_ratio=0.0,
            chain_ok_count=5, chain_ok_rate=1.0, alerts_total=0, alerts_avg=0.0,
            first_timestamp="t1", last_timestamp="t5", note="",
        )
        c = WindowComparison(
            window_a_label="A:5", window_b_label="B:5",
            delta_proceed_ratio=0.0, delta_pause_ratio=0.0,
            delta_lockdown_ratio=0.0, delta_chain_ok_rate=0.0,
            delta_alerts_avg=0.0, shift_verdict="STABLE", shift_magnitude=0.0,
            reason="STABLE", note="",
        )
        rep = MultiPolicyEvaluationReport(
            window_a=d, window_b=d, comparison=c, alerts=[],
            verdict="STABLE", n_alerts=0, worst_severity="INFO", note="",
        )
        rep2_dict = {
            "window_a": dataclasses.asdict(rep.window_a),
            "window_b": dataclasses.asdict(rep.window_b),
            "comparison": dataclasses.asdict(rep.comparison),
            "alerts": [],
            "verdict": rep.verdict,
            "n_alerts": rep.n_alerts,
            "worst_severity": rep.worst_severity,
            "note": rep.note,
        }
        # Re-build
        rep2 = MultiPolicyEvaluationReport(
            window_a=WindowDistribution(**rep2_dict["window_a"]),
            window_b=WindowDistribution(**rep2_dict["window_b"]),
            comparison=WindowComparison(**rep2_dict["comparison"]),
            alerts=[],
            verdict=rep2_dict["verdict"],
            n_alerts=rep2_dict["n_alerts"],
            worst_severity=rep2_dict["worst_severity"],
            note=rep2_dict["note"],
        )
        self.assertEqual(rep, rep2)


# ============================================================================
# Section 3: TestConfig
# ============================================================================


class TestConfig(unittest.TestCase):
    """Verify EvaluatorConfig + build_default_config."""

    def test_014_default_config_uses_constants(self) -> None:
        self.assertEqual(DEFAULT_EVALUATOR_CONFIG.window_size, 5)
        self.assertEqual(DEFAULT_EVALUATOR_CONFIG.threshold, 0.10)

    def test_015_build_default_config_applies_overrides(self) -> None:
        cfg = build_default_config({"window_size": 10, "threshold": 0.20})
        self.assertEqual(cfg.window_size, 10)
        self.assertEqual(cfg.threshold, 0.20)

    def test_016_build_default_config_rejects_unknown_keys(self) -> None:
        with self.assertRaises(ValueError):
            build_default_config({"bogus_key": 1})

    def test_017_config_rejects_window_size_zero(self) -> None:
        with self.assertRaises(ValueError):
            EvaluatorConfig(
                window_size=0,
                threshold=0.1,
                min_window_size=1,
                max_window_size=1024,
                history_path=Path("."),
            )

    def test_018_config_rejects_threshold_out_of_bounds(self) -> None:
        with self.assertRaises(ValueError):
            EvaluatorConfig(
                window_size=5,
                threshold=1.5,
                min_window_size=1,
                max_window_size=1024,
                history_path=Path("."),
            )


# ============================================================================
# Section 4: TestComputeWindowDistribution
# ============================================================================


class TestComputeWindowDistribution(unittest.TestCase):
    """Verify compute_window_distribution correctness."""

    def test_019_empty_window(self) -> None:
        dist = compute_window_distribution([], _window_label("A", 0))
        self.assertEqual(dist.n_snapshots, 0)
        self.assertEqual(dist.proceed_count, 0)
        self.assertEqual(dist.pause_count, 0)
        self.assertEqual(dist.lockdown_count, 0)
        self.assertEqual(dist.proceed_ratio, 0.0)
        self.assertEqual(dist.chain_ok_rate, 0.0)

    def test_020_counts_policies(self) -> None:
        snaps = [
            MockTickSnapshot(policy="PROCEED"),
            MockTickSnapshot(policy="PROCEED"),
            MockTickSnapshot(policy="PROCEED"),
            MockTickSnapshot(policy="PAUSE"),
            MockTickSnapshot(policy="LOCKDOWN"),
        ]
        dist = compute_window_distribution(snaps, "TEST")
        self.assertEqual(dist.proceed_count, 3)
        self.assertEqual(dist.pause_count, 1)
        self.assertEqual(dist.lockdown_count, 1)
        self.assertAlmostEqual(dist.proceed_ratio, 0.6)

    def test_021_counts_chain_ok(self) -> None:
        snaps = [
            MockTickSnapshot(chain_ok=True),
            MockTickSnapshot(chain_ok=True),
            MockTickSnapshot(chain_ok=False),
        ]
        dist = compute_window_distribution(snaps, "TEST")
        self.assertEqual(dist.chain_ok_count, 2)
        self.assertAlmostEqual(dist.chain_ok_rate, 2 / 3, places=5)

    def test_022_alerts_total_and_avg(self) -> None:
        snaps = [
            MockTickSnapshot(alerts_count=0),
            MockTickSnapshot(alerts_count=2),
            MockTickSnapshot(alerts_count=3),
            MockTickSnapshot(alerts_count=1),
        ]
        dist = compute_window_distribution(snaps, "TEST")
        self.assertEqual(dist.alerts_total, 6)
        self.assertAlmostEqual(dist.alerts_avg, 1.5)

    def test_023_first_last_timestamps(self) -> None:
        snaps = [
            MockTickSnapshot(timestamp="t1"),
            MockTickSnapshot(timestamp="t3"),
            MockTickSnapshot(timestamp="t5"),
        ]
        dist = compute_window_distribution(snaps, "TEST")
        self.assertEqual(dist.first_timestamp, "t1")
        self.assertEqual(dist.last_timestamp, "t5")


# ============================================================================
# Section 5: TestCompareWindowDistributions
# ============================================================================


class TestCompareWindowDistributions(unittest.TestCase):
    """Verify compare_window_distributions."""

    def test_024_equal_windows_is_stable(self) -> None:
        snaps = [MockTickSnapshot(policy="PROCEED", chain_ok=True)] * 5
        d_a = compute_window_distribution(snaps, "A:5")
        d_b = compute_window_distribution(snaps, "B:5")
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        self.assertEqual(cmp.shift_verdict, "STABLE")
        self.assertEqual(cmp.delta_proceed_ratio, 0.0)
        self.assertEqual(cmp.shift_magnitude, 0.0)

    def test_025_lockdown_emerging_is_shift(self) -> None:
        d_a = compute_window_distribution(
            [MockTickSnapshot(policy="LOCKDOWN", chain_ok=False, alerts_count=5)] * 5,
            "A:5",
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(policy="PROCEED", chain_ok=True, alerts_count=0)] * 5,
            "B:5",
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        self.assertEqual(cmp.shift_verdict, "SHIFT")
        self.assertEqual(cmp.delta_lockdown_ratio, 1.0)

    def test_026_chain_ok_drop_detected(self) -> None:
        d_a = compute_window_distribution(
            [MockTickSnapshot(chain_ok=False)] * 5, "A:5"
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(chain_ok=True)] * 5, "B:5"
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        self.assertEqual(cmp.delta_chain_ok_rate, -1.0)
        self.assertEqual(cmp.shift_verdict, "SHIFT")

    def test_027_threshold_validation(self) -> None:
        d_a = compute_window_distribution([MockTickSnapshot()], "A:1")
        d_b = compute_window_distribution([MockTickSnapshot()], "B:1")
        with self.assertRaises(ValueError):
            compare_window_distributions(d_a, d_b, threshold=1.5)

    def test_028_delta_alerts_avg(self) -> None:
        d_a = compute_window_distribution(
            [MockTickSnapshot(alerts_count=5)] * 4, "A:4"
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(alerts_count=0)] * 4, "B:4"
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        self.assertAlmostEqual(cmp.delta_alerts_avg, 5.0)


# ============================================================================
# Section 6: TestDetectShift
# ============================================================================


class TestDetectShift(unittest.TestCase):
    """Verify detect_shift alert emission."""

    def test_029_stable_no_alerts(self) -> None:
        snaps = [MockTickSnapshot(policy="PROCEED", chain_ok=True)] * 5
        d_a = compute_window_distribution(snaps, "A:5")
        d_b = compute_window_distribution(snaps, "B:5")
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        alerts = detect_shift(cmp)
        self.assertEqual(len(alerts), 0)

    def test_030_lockdown_alert_emitted(self) -> None:
        d_a = compute_window_distribution(
            [MockTickSnapshot(policy="LOCKDOWN", chain_ok=False)] * 5, "A:5"
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(policy="PROCEED", chain_ok=True)] * 5, "B:5"
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        alerts = detect_shift(cmp)
        self.assertGreaterEqual(len(alerts), 1)
        self.assertTrue(any(a.alert_type == "LOCKDOWN_RATIO_SHIFT" for a in alerts))

    def test_031_proceed_alert_emitted(self) -> None:
        d_a = compute_window_distribution(
            [MockTickSnapshot(policy="PROCEED")] * 5, "A:5"
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(policy="LOCKDOWN")] * 5, "B:5"
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        alerts = detect_shift(cmp)
        self.assertTrue(any(a.alert_type == "PROCEED_RATIO_SHIFT" for a in alerts))

    def test_032_critical_severity_on_large_lockdown(self) -> None:
        # 5/5 LOCKDOWN in window_a, 0/5 LOCKDOWN in window_b → delta=1.0 → 10x threshold
        d_a = compute_window_distribution(
            [MockTickSnapshot(policy="LOCKDOWN", chain_ok=False)] * 5, "A:5"
        )
        d_b = compute_window_distribution(
            [MockTickSnapshot(policy="PROCEED", chain_ok=True)] * 5, "B:5"
        )
        cmp = compare_window_distributions(d_a, d_b, threshold=0.10)
        alerts = detect_shift(cmp)
        ld_alerts = [a for a in alerts if a.alert_type == "LOCKDOWN_RATIO_SHIFT"]
        self.assertGreaterEqual(len(ld_alerts), 1)
        self.assertEqual(ld_alerts[0].severity, "CRITICAL")


# ============================================================================
# Section 7: TestEvaluate
# ============================================================================


class TestEvaluate(unittest.TestCase):
    """Verify evaluate end-to-end."""

    def test_033_empty_snapshots(self) -> None:
        rep = evaluate([])
        self.assertEqual(rep.verdict, "INSUFFICIENT_DATA")
        self.assertEqual(rep.n_alerts, 0)
        self.assertEqual(rep.worst_severity, "INFO")

    def test_034_window_split_correct(self) -> None:
        snaps = (
            [MockTickSnapshot(policy="PROCEED") for _ in range(5)]
            + [MockTickSnapshot(policy="LOCKDOWN") for _ in range(5)]
        )
        cfg = EvaluatorConfig(
            window_size=5,
            threshold=0.10,
            min_window_size=1,
            max_window_size=1024,
            history_path=Path("."),
        )
        rep = evaluate(snaps, cfg)
        self.assertEqual(rep.window_a.n_snapshots, 5)
        self.assertEqual(rep.window_b.n_snapshots, 5)
        self.assertEqual(rep.window_a.lockdown_count, 5)
        self.assertEqual(rep.window_b.lockdown_count, 0)

    def test_035_critical_shift_verdict(self) -> None:
        snaps = (
            [MockTickSnapshot(policy="PROCEED") for _ in range(5)]
            + [MockTickSnapshot(policy="LOCKDOWN") for _ in range(5)]
        )
        cfg = EvaluatorConfig(
            window_size=5,
            threshold=0.10,
            min_window_size=1,
            max_window_size=1024,
            history_path=Path("."),
        )
        rep = evaluate(snaps, cfg)
        # 5/5 LOCKDOWN in window_a → CRITICAL alert
        self.assertIn(rep.verdict, ("SHIFT", "CRITICAL_SHIFT"))
        self.assertEqual(rep.worst_severity, "CRITICAL")

    def test_036_stable_verdict(self) -> None:
        snaps = [MockTickSnapshot(policy="PROCEED", chain_ok=True)] * 10
        cfg = EvaluatorConfig(
            window_size=5,
            threshold=0.10,
            min_window_size=1,
            max_window_size=1024,
            history_path=Path("."),
        )
        rep = evaluate(snaps, cfg)
        self.assertEqual(rep.verdict, "STABLE")
        self.assertEqual(rep.n_alerts, 0)


# ============================================================================
# Section 8: TestRenderAndPopper
# ============================================================================


class TestRenderAndPopper(unittest.TestCase):
    """Verify render_evaluation_md + popper_self_test."""

    def test_037_render_has_5_sections(self) -> None:
        snaps = (
            [MockTickSnapshot(policy="PROCEED") for _ in range(5)]
            + [MockTickSnapshot(policy="LOCKDOWN") for _ in range(5)]
        )
        cfg = EvaluatorConfig(
            window_size=5, threshold=0.10, min_window_size=1,
            max_window_size=1024, history_path=Path("."),
        )
        rep = evaluate(snaps, cfg)
        md = render_evaluation_md(rep)
        self.assertIn("## 1.", md)
        self.assertIn("## 2.", md)
        self.assertIn("## 3.", md)
        self.assertIn("## 4.", md)
        self.assertIn("## 5.", md)
        self.assertIn("Honest disclosure", md)

    def test_038_render_alphabetical_alerts_sorted(self) -> None:
        # Force multiple alerts at different severities
        snaps = (
            [MockTickSnapshot(policy="PROCEED") for _ in range(5)]
            + [MockTickSnapshot(policy="LOCKDOWN", chain_ok=False) for _ in range(5)]
        )
        cfg = EvaluatorConfig(
            window_size=5, threshold=0.10, min_window_size=1,
            max_window_size=1024, history_path=Path("."),
        )
        rep = evaluate(snaps, cfg)
        md = render_evaluation_md(rep)
        # CRITICAL alerts should appear before WARN/INFO
        crit_idx = md.find("CRITICAL")
        warn_idx = md.find("[WARN]")
        info_idx = md.find("[INFO]")
        if crit_idx >= 0 and warn_idx >= 0:
            self.assertLess(crit_idx, warn_idx)

    def test_039_popper_15_of_15(self) -> None:
        all_ok, results = popper_self_test()
        self.assertTrue(all_ok, msg=f"popper failed: {results}")
        self.assertEqual(len(results), 15)
        for name, ok in results:
            self.assertTrue(ok, msg=f"popper failed: {name}")


# ============================================================================
# Section 9: TestChainDelegate
# ============================================================================


class TestChainDelegate(unittest.TestCase):
    """Verify V1419 chain_delegate probe."""

    def test_040_chain_delegate_returns_dict(self) -> None:
        d = chain_delegate()
        self.assertIsInstance(d, dict)
        self.assertEqual(d["schema"], V1419_SCHEMA)
        self.assertEqual(d["version"], V1419_VERSION)
        self.assertIn("all_ok", d)
        self.assertIn("n_modules", d)
        self.assertIn("n_modules_ok", d)
        self.assertIn("errors", d)


# ============================================================================
# Section 10: TestHelpers
# ============================================================================


class TestHelpers(unittest.TestCase):
    """Verify helper utilities."""

    def test_041_safe_path_rejects_dotdot(self) -> None:
        with self.assertRaises(ValueError):
            _safe_path(Path("../etc/passwd"))

    def test_042_safe_path_accepts_absolute(self) -> None:
        p = _safe_path(Path("C:/Users/test/file.json"))
        self.assertEqual(str(p), "C:\\Users\\test\\file.json")

    def test_043_safe_ratio_zero_denom(self) -> None:
        self.assertEqual(_safe_ratio(5, 0), 0.0)

    def test_044_safe_ratio_normal(self) -> None:
        self.assertAlmostEqual(_safe_ratio(3, 4), 0.75)

    def test_045_window_label(self) -> None:
        self.assertEqual(_window_label("A", 5), "A:last5")

    def test_046_worst_severity_empty(self) -> None:
        self.assertEqual(_worst_severity([]), "INFO")

    def test_047_worst_severity_critical(self) -> None:
        self.assertEqual(
            _worst_severity(["INFO", "WARN", "CRITICAL", "INFO"]), "CRITICAL"
        )

    def test_048_worst_severity_warn(self) -> None:
        self.assertEqual(_worst_severity(["INFO", "WARN", "INFO"]), "WARN")

    def test_049_atomic_write_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            p = Path(tmpdir) / "test.json"
            _atomic_write_json(p, {"k": "v", "n": 42})
            self.assertTrue(p.exists())
            self.assertFalse(p.with_suffix(p.suffix + ".tmp").exists())
            with open(p, "r", encoding="utf-8") as fh:
                d = json.load(fh)
            self.assertEqual(d, {"k": "v", "n": 42})

    def test_050_atomic_write_text(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            p = Path(tmpdir) / "test.md"
            _atomic_write_text(p, "# Hello\n\nWorld\n")
            self.assertTrue(p.exists())
            with open(p, "r", encoding="utf-8") as fh:
                content = fh.read()
            self.assertEqual(content, "# Hello\n\nWorld\n")


# ============================================================================
# Section 11: TestCLI (subprocess-driven)
# ============================================================================


def _run_cli_subprocess(args: List[str], cwd: Path) -> Dict[str, Any]:
    """Run `python -m apeireth.v1419_asi_multi_policy_evaluator <args>` as subprocess.

    Returns parsed JSON output. Raises RuntimeError on non-zero exit.
    """
    full = [sys.executable, "-m", "apeireth.v1419_asi_multi_policy_evaluator"] + args
    result = subprocess.run(
        full, capture_output=True, text=True, cwd=str(cwd),
        timeout=30, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"CLI failed (rc={result.returncode}): {result.stderr}\nstdout={result.stdout}"
        )
    return json.loads(result.stdout)


class TestCLI(unittest.TestCase):
    """Subprocess-driven CLI tests."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.cwd = ROOT

    def test_051_cli_version(self) -> None:
        full = [sys.executable, "-m", "apeireth.v1419_asi_multi_policy_evaluator", "version"]
        result = subprocess.run(
            full, capture_output=True, text=True, cwd=str(self.cwd),
            timeout=30, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("v1419_asi_multi_policy_evaluator", result.stdout)
        self.assertIn("0.1.0", result.stdout)

    def test_052_cli_meta_json(self) -> None:
        d = _run_cli_subprocess(["meta", "--json"], self.cwd)
        self.assertEqual(d["version"], V1419_VERSION)
        self.assertEqual(d["schema"], V1419_SCHEMA)
        self.assertGreaterEqual(len(d["guards"]), 10)
        self.assertEqual(len(d["v3_guards"]), 9)

    def test_053_cli_popper(self) -> None:
        full = [sys.executable, "-m", "apeireth.v1419_asi_multi_policy_evaluator", "popper"]
        result = subprocess.run(
            full, capture_output=True, text=True, cwd=str(self.cwd),
            timeout=30, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("popper: 15/15", result.stdout)

    def test_054_cli_demo(self) -> None:
        d = _run_cli_subprocess(["demo"], self.cwd)
        self.assertIn("verdict", d)
        self.assertIn("window_a", d)
        self.assertIn("window_b", d)
        self.assertIn("comparison", d)
        self.assertIn("alerts", d)
        self.assertIn(d["verdict"], ("STABLE", "SHIFT", "CRITICAL_SHIFT"))

    def test_055_cli_help(self) -> None:
        full = [sys.executable, "-m", "apeireth.v1419_asi_multi_policy_evaluator", "help"]
        result = subprocess.run(
            full, capture_output=True, text=True, cwd=str(self.cwd),
            timeout=30, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("multi-policy evaluator", result.stdout)
        self.assertIn("evaluate", result.stdout)


if __name__ == "__main__":
    unittest.main()