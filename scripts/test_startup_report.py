"""Check regression policy and missing-evidence handling with independent receipts."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("startup_report", Path(__file__).with_name("startup_report.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class StartupReportTests(unittest.TestCase):
    def report(self):
        return dict(environment={k: "same" for k in ("platform", "cpu", "memory", "toolchain", "filesystem")},
                    fixture="session-pair-v1", measurements={
                        f"{profile}/{mode}/total": dict(n=30, p50_ms=10, p95_ms=20)
                        for profile in MODULE.BUDGETS for mode in MODULE.MODES})

    def test_first_and_cold_regressions_and_noise_floor(self):
        before = self.report()
        after = copy.deepcopy(before)
        after["measurements"]["Dev/first_run/total"]["p50_ms"] = 13
        after["measurements"]["Release/cold_start/total"]["p95_ms"] = 27
        after["measurements"]["Dev/normal_start/total"]["p50_ms"] = 10.9
        result = MODULE.compare(before, after)
        self.assertEqual(result["state"], "REGRESSION")
        self.assertEqual({r["key"] for r in result["regressions"]},
                         {"Dev/first_run/total", "Release/cold_start/total"})

    def test_unavailable_is_incomplete_and_environment_drift_is_rejected(self):
        before = self.report()
        after = copy.deepcopy(before)
        del after["measurements"]["Dev/cold_start/total"]
        self.assertEqual(MODULE.compare(before, after)["state"], "INCOMPLETE")
        after["environment"]["cpu"] = "different"
        with self.assertRaisesRegex(ValueError, "environment changed"):
            MODULE.compare(before, after)

    def test_denied_cache_drop_never_becomes_a_zero_duration_measurement(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "cold.log"
            log.write_text("STARTUP_META|cold_start|Dev|fixture=session-pair-v1\n"
                           "STARTUP_UNAVAILABLE|cold_start|n=0|Permission denied\n")
            records, unavailable = MODULE.parse_logs([log])
            self.assertEqual(records, {})
            self.assertIn("Dev/cold_start", unavailable)
            log.write_text(log.read_text() + "STARTUP|cold_start|Dev|total|1|0|0|0|0|0\n")
            with self.assertRaisesRegex(ValueError, "cannot supply valid samples"):
                MODULE.parse_logs([log])


if __name__ == "__main__":
    unittest.main()
