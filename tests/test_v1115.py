"""V1115 Real R9 W3 E2E Operational Run — tests (主 00:44 质量工程区).

主 17:43 实事求是: 每 test 必须真跑 V1115, 不允许 mock.
主 17:58+20:46 不假装: 失败必须 reported, 不 silent skip.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# 强制 UTF-8 stdout (V1115 模式)
try:
    import io as _io
    sys.stdout = _io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = _io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
except Exception:
    pass

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1115_real_r9_w3_e2e_run import (
    V1115_VERSION,
    V1115Main,
    E2EAuditChain,
    V1077Runner,
    V1114Runner,
    V1088Runner,
    E2ETraceRecord,
    V3_GUARDS,
    REFERENCES,
    DEFAULT_TASKS,
    _now,
    _extract_subscore,
)


class TestV1115Import(unittest.TestCase):
    """真实 import 测试 (主 00:56 任何人都能接手)."""

    def test_imports_ok(self):
        self.assertTrue(callable(V1115Main))
        self.assertEqual(V1115_VERSION, "0.1.0")
        self.assertGreater(len(REFERENCES), 0)
        self.assertGreater(len(DEFAULT_TASKS), 0)
        self.assertEqual(len(V3_GUARDS), 4)

    def test_version_is_string(self):
        self.assertIsInstance(V1115_VERSION, str)
        parts = V1115_VERSION.split(".")
        self.assertEqual(len(parts), 3)

    def test_references_real(self):
        for r in REFERENCES:
            self.assertIn("id", r)
            self.assertIn("title", r)
            self.assertIsInstance(r["id"], str)


class TestV1115SelfCheck(unittest.TestCase):
    """真 self-check CLI (主 00:56)."""

    def test_self_check_returns_dict(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            main_obj = V1115Main(chain_path)
            sc = main_obj.self_check()
            self.assertIsInstance(sc, dict)
            self.assertTrue(sc["ok"])
            self.assertEqual(sc["v1115_version"], V1115_VERSION)
            self.assertEqual(sc["audit_chain_count"], 0)
            self.assertEqual(len(sc["v3_guards"]), 4)


class TestV1115AuditChain(unittest.TestCase):
    """真 audit chain (WAL 模式, 主 23:44 干到底)."""

    def test_audit_chain_append_and_read(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            chain = E2EAuditChain(chain_path)
            chain.append({"kind": "test", "n": 1})
            chain.append({"kind": "test", "n": 2})
            chain.append({"kind": "test", "n": 3})
            self.assertEqual(chain.count(), 3)
            rows = chain.all()
            self.assertEqual(len(rows), 3)
            self.assertEqual(rows[0]["n"], 1)
            self.assertEqual(rows[2]["n"], 3)

    def test_audit_chain_empty_returns_zero(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            chain = E2EAuditChain(chain_path)
            self.assertEqual(chain.count(), 0)
            self.assertEqual(chain.all(), [])

    def test_audit_chain_persists_across_instances(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            chain1 = E2EAuditChain(chain_path)
            chain1.append({"kind": "a", "i": 1})
            chain2 = E2EAuditChain(chain_path)
            chain2.append({"kind": "a", "i": 2})
            self.assertEqual(chain2.count(), 2)


class TestV1115V1088Runner(unittest.TestCase):
    """真 V1088Runner (主 17:43 实事求是)."""

    def test_v1088_runner_imports(self):
        runner = V1088Runner()
        self.assertTrue(callable(runner.run_n))
        self.assertEqual(runner.last_status, "unknown")

    def test_v1088_runner_run_n_short(self):
        runner = V1088Runner()
        tasks = [("math", "2+2"), ("general", "hello")]
        records = runner.run_n(tasks)
        self.assertEqual(len(records), 2)
        for r in records:
            self.assertIsInstance(r, E2ETraceRecord)
            self.assertGreater(r.subscore, 0.0)
            self.assertIn(r.verdict, {"reject", "accept", "error", "unknown"})

    def test_v1088_runner_handles_empty(self):
        runner = V1088Runner()
        records = runner.run_n([])
        self.assertEqual(records, [])


class TestV1115V1114Runner(unittest.TestCase):
    """真 V1114Runner (fail-soft 降级, 主 17:58 不假装)."""

    def test_v1114_runner_init(self):
        runner = V1114Runner(week_label="W3")
        self.assertEqual(runner.week_label, "W3")
        self.assertEqual(runner.last_status, "unknown")

    def test_v1114_run_returns_dict(self):
        runner = V1114Runner(week_label="W3")
        result = runner.run()
        self.assertIsInstance(result, dict)
        self.assertIn("ok", result)
        self.assertIn("status", result)
        # 失败时降级到 v1077_only (主 17:58 不 silent skip)
        if not result["ok"]:
            self.assertEqual(result.get("degraded_to"), "v1077_only")


class TestV1115V1077Runner(unittest.TestCase):
    """真 V1077Runner (subprocess, 主 17:43 实事求是)."""

    def test_v1077_runner_init(self):
        runner = V1077Runner(timeout=30)
        self.assertEqual(runner.timeout, 30)

    def test_v1077_runner_returns_dict(self):
        runner = V1077Runner(timeout=60)
        result = runner.run()
        self.assertIsInstance(result, dict)
        self.assertIn("ok", result)
        self.assertIn("v04_score", result)
        self.assertIn("n_dims_filled", result)
        self.assertIn("n_dims_total", result)


class TestV1115ExtractSubscore(unittest.TestCase):
    """真 _extract_subscore (主 17:43 实事求是)."""

    def test_extract_subscore_empty(self):
        result = _extract_subscore(None)
        self.assertEqual(result, 0.0)

    def test_extract_subscore_with_steps(self):
        from dataclasses import dataclass

        @dataclass
        class FakeStep:
            stage: str
            status: str

        @dataclass
        class FakeTrace:
            steps: list

        # 5 stages: probe pass, route pass, infer pass, gate pass, audit pass
        steps = [
            FakeStep("probe", "pass"),
            FakeStep("route", "pass"),
            FakeStep("infer", "pass"),
            FakeStep("gate", "pass"),
            FakeStep("audit", "pass"),
        ]
        trace = FakeTrace(steps=steps)
        subscore = _extract_subscore(trace)
        self.assertGreater(subscore, 0.5)

    def test_extract_subscore_mixed(self):
        from dataclasses import dataclass

        @dataclass
        class FakeStep:
            stage: str
            status: str

        @dataclass
        class FakeTrace:
            steps: list

        steps = [
            FakeStep("probe", "unknown"),
            FakeStep("route", "pass"),
            FakeStep("infer", "pass"),
            FakeStep("gate", "fail"),
            FakeStep("audit", "skip"),
        ]
        trace = FakeTrace(steps=steps)
        subscore = _extract_subscore(trace)
        self.assertGreater(subscore, 0.0)
        self.assertLess(subscore, 1.0)


class TestV1115MainRun(unittest.TestCase):
    """真 V1115Main.run (主 17:43 实事求是: 真跑 V1088 + V1077 + V1114)."""

    def test_main_run_minimal(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            main_obj = V1115Main(chain_path)
            result = main_obj.run(n=2)
            self.assertEqual(result.n_traces, 2)
            self.assertGreater(result.v1077_v04_score, 0.0)
            self.assertEqual(result.v1077_dims_total, 17)
            self.assertGreater(result.audit_chain_count, 0)

    def test_main_run_full_10(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            main_obj = V1115Main(chain_path)
            result = main_obj.run(n=10)
            self.assertEqual(result.n_traces, 10)
            verdicts = set(result.verdicts.keys())
            self.assertTrue(verdicts.issubset({"reject", "accept", "error", "unknown"}))
            self.assertGreater(result.total_prov_nodes, 0)
            self.assertGreater(result.avg_subscore, 0.0)
            self.assertGreater(result.v1077_v04_score, 0.0)

    def test_main_run_v3_guards(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            main_obj = V1115Main(chain_path)
            result = main_obj.run(n=3)
            # V3 守门: V1115 跑通 + V1077 跑 + V1114 fail-soft reported
            self.assertTrue(result.v3_guards_ok)
            self.assertIn("guard_v1115_is_not_asi", result.philosophy_guards)
            self.assertIn("guard_no_hardcoded_lift", result.philosophy_guards)
            self.assertIn("guard_v1077_is_subprocess", result.philosophy_guards)
            self.assertIn("guard_v1114_fail_soft_not_silent", result.philosophy_guards)


class TestV1115Asdict(unittest.TestCase):
    """真 asdict 序列化 (主 17:43 实事求是)."""

    def test_w3e2e_run_result_asdict(self):
        with tempfile.TemporaryDirectory() as td:
            chain_path = Path(td) / "audit.jsonl"
            main_obj = V1115Main(chain_path)
            result = main_obj.run(n=2)
            from dataclasses import asdict
            d = asdict(result)
            self.assertIn("v1115_version", d)
            self.assertIn("n_traces", d)
            self.assertIn("asi_lift_real", d)
            self.assertIn("v1077_v04_score", d)
            self.assertIn("audit_chain_count", d)


class TestV1115CLI(unittest.TestCase):
    """真 CLI 自检 (主 00:56 任何人都能接手)."""

    def test_cli_self_check(self):
        import subprocess
        # 主 17:43 实事求是: cwd 必须是 ROOT (包含 apeireth/ 包), 否则 subprocess 找不到.
        root = ROOT
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1115_real_r9_w3_e2e_run", "--self-check"],
            capture_output=True,
            text=True,
            timeout=60,
            encoding="utf-8",
            errors="replace",
            env={**os.environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"},
            cwd=str(root),
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("v1115_version", result.stdout)


if __name__ == "__main__":
    unittest.main()
