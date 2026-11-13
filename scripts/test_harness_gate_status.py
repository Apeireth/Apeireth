#!/usr/bin/env python3
"""`scripts/harness.py` 的**门禁状态判定**自测（防「门禁本身被改坏」）。

# 为什么守卫也要有自测

2026-11-08 实测事故：本地闸门 `check_golden_baseline.py` 在**语料路径指错**时
**跳过并退出 0** ⇒ 人只看到 ✅ ⇒ 「准确度不许掉」这道闸门**一直没在跑**。

⇒ ⇒ 根因有两层：
1. 闸门脚本把「跳过」和「通过」**用同一个退出码**表达；
2. harness 把「**退出码 0**」直接当成「**跑过且通过**」。

⇒ ⇒ 本批修了两端（闸门脚本区分「没有语料」与「语料残缺」；harness 识别
`GATE_SKIP:` 标记 ⇒ 记 **SKIPPED** 而不是 PASS）⇒ ⇒ 所以**必须钉住它真的有牙**：

1. 打印了 `GATE_SKIP:` 且退出 0 ⇒ ⇒ 状态**必须**是 `SKIPPED`（**不是 PASS**）；
2. 退出 0 且没有该标记 ⇒ ⇒ 状态**必须**是 `PASS`；
3. 退出非 0 ⇒ ⇒ 状态**必须**是 `FAIL`；
4. **环境跳过**（`skip_kind = "env"`）**不阻断**裁决（环境本来就没有那些前置条件）
   ⇒ ⇒ 但**必须出现在理由里**（否则「没跑」在报告里就消失了 ⇒ 又变成「只看 ✅」）；
5. **真·scope 跳过**（本次 diff 不触发该门禁）⇒ ⇒ 不阻断；
6. **适用却没跑**的 required 门禁（既不是 env 跳过、也没有 scope 不相交）⇒ ⇒ **必须 BLOCKED**。

# 用法

```sh
python3 scripts/test_harness_gate_status.py
```
"""

from __future__ import annotations

import argparse
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent


def _load_harness():
    spec = importlib.util.spec_from_file_location("harness_under_test", ROOT / "scripts" / "harness.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HARNESS = _load_harness()
REAL_RUN = subprocess.run


def _python_only_runner(directory: Path):
    def run(command, *args, **kwargs):
        # These tests must never run the real resource guard or target cleaner.
        if len(command) > 1 and Path(command[1]).name in {
            "disk_guard.py", "clean_test_binaries.py"
        }:
            return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
        if (
            command[0] != sys.executable
            or not Path(command[1]).resolve().is_relative_to(directory.resolve())
        ):
            raise AssertionError(f"only temporary Python fake gates may run: {command}")
        return REAL_RUN(command, *args, **kwargs)
    return run


def _run_isolated_gate(directory: Path, gate: dict) -> dict:
    with mock.patch.object(HARNESS.subprocess, "run", side_effect=_python_only_runner(directory)):
        return HARNESS.run_gate(gate, 60)


def _stub(directory: Path, name: str, body: str) -> list[str]:
    path = directory / f"{name}.py"
    path.write_text(body, encoding="utf-8")
    return [sys.executable, str(path)]


class GateStatusTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def _run(self, cmd: list[str]) -> dict:
        gate = {"id": "t", "cmd": cmd, "scope": ["rust"], "required": True}
        return _run_isolated_gate(self.dir, gate)

    def test_a_gate_that_skipped_is_not_recorded_as_pass(self):
        """① 打印了跳过标记 ⇒ 状态必须是 SKIPPED（**不是 PASS**）。"""
        cmd = _stub(self.dir, "skipper", 'print("GATE_SKIP: 语料目录不存在")\n')
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_SKIPPED, result["status"], "跳过的门禁不得记成 PASS")
        self.assertEqual("env", result["skip_kind"])
        self.assertIn("不是通过", result["reason"] or "")

    def test_a_gate_that_ran_and_passed_is_pass(self):
        """② 退出 0 且没有跳过标记 ⇒ PASS。"""
        cmd = _stub(self.dir, "passer", 'print("all good")\n')
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_PASS, result["status"])
        self.assertIsNone(result["skip_kind"])

    def test_a_gate_that_failed_is_fail(self):
        """③ 退出非 0 ⇒ FAIL。"""
        cmd = _stub(self.dir, "failing", "import sys\nsys.exit(3)\n")
        result = self._run(cmd)
        self.assertEqual(HARNESS.S_FAIL, result["status"])


class VerdictTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def _skipped_gate(self) -> dict:
        cmd = _stub(self.dir, "skipper", 'print("GATE_SKIP: 语料目录不存在")\n')
        return _run_isolated_gate(
            self.dir,
            {"id": "t-skip", "cmd": cmd, "scope": ["rust"], "required": True}
        )

    def test_environment_skip_does_not_block_but_is_visible(self):
        """④ 环境跳过 ⇒ 裁决仍 PASS，但**理由里必须出现「跳过」**。"""
        gate = self._skipped_gate()
        verdict, reasons = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_PASS, verdict, "环境缺前置条件不该阻断")
        self.assertTrue(
            any("跳过" in reason for reason in reasons),
            f"跳过必须在理由里出现（否则报告里只剩 ✅）：{reasons}",
        )

    def test_scope_skip_does_not_block(self):
        """⑤ 真·scope 跳过（本次 diff 不触发）⇒ 不阻断。"""
        gate = dict(self._skipped_gate())
        gate["scope_applies"] = False
        verdict, _ = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_PASS, verdict)

    def test_applicable_but_not_run_required_gate_blocks(self):
        """⑥ **适用却没跑**的 required 门禁 ⇒ 必须 BLOCKED（该查的没查）。"""
        gate = dict(self._skipped_gate())
        gate["skip_kind"] = None  # 不是环境跳过 ⇒ 就是「适用却没跑」
        gate["scope_applies"] = True
        verdict, reasons = HARNESS.evaluate_verdict([gate], [], [])
        self.assertEqual(HARNESS.VERDICT_BLOCKED, verdict, "适用却没跑必须阻断")
        self.assertTrue(any("skipped" in reason for reason in reasons), reasons)


class VerifyAggregationTests(unittest.TestCase):
    """Use real check/run_gate/attest with only external gate programs replaced."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = Path(temporary.name)
        self.context = {
            "commit_sha": "a" * 40, "tree_sha": "b" * 40, "base_sha": "c" * 40,
            "git_ok": True, "files": ["crates/fake/src/lib.rs", "docs/fake.md"],
            "diff_source": "test cumulative base", "tags": [
                "rust", "docs", "claims", "ci", "contract", "lock", "frontend"
            ],
        }
        self.dirty = False
        self.plan = {"available": True, "stages": ["fast", "deep"], "level": "L3"}
        self.gates = copy.deepcopy(HARNESS.load_registry()[0])
        self.controls = self.dir / "controls.json"
        self.calls = self.dir / "calls.txt"
        self.controls.write_text("{}", encoding="utf-8")
        fake_gate = self.dir / "fake_gate.py"
        fake_gate.write_text(
            "import json, pathlib, sys\n"
            "controls, gate, calls = sys.argv[1:]\n"
            "with pathlib.Path(calls).open('a', encoding='utf-8') as log:\n"
            "    log.write(gate + '\\n')\n"
            "print('evidence:' + gate)\n"
            "mode = json.loads(pathlib.Path(controls).read_text('utf-8')).get(gate)\n"
            "if mode == 'skip': print('GATE_SKIP: isolated fake corpus absent')\n"
            "if mode == 'fail': sys.exit(3)\n",
            encoding="utf-8",
        )
        for gate in self.gates:
            gate["cmd"] = [sys.executable, str(fake_gate), str(self.controls), gate["id"], str(self.calls)]
        self.stages = []
        self.phase_documents = []
        self.after_stage = None
        real_check = HARNESS.cmd_check

        def check(args):
            self.stages.append(args.stage)
            code = real_check(args)
            self.phase_documents.append(json.loads(HARNESS.LAST_RUN_PATH.read_text("utf-8")))
            if self.after_stage:
                self.after_stage(args.stage)
            return code

        for name, value in {
            "LAST_RUN_PATH": self.dir / "last-run.json",
            "ARTIFACT_ROOT": self.dir / "artifacts",
        }.items():
            patcher = mock.patch.object(HARNESS, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        for name, callback in {
            "collect_context": lambda _: copy.deepcopy(self.context),
            "tree_is_dirty": lambda: self.dirty,
            "impact_plan": lambda _: copy.deepcopy(self.plan),
            "load_registry": lambda: (copy.deepcopy(self.gates), ["isolated contract commands"], []),
            "cmd_check": check,
        }.items():
            patcher = mock.patch.object(HARNESS, name, side_effect=callback)
            patcher.start()
            self.addCleanup(patcher.stop)
        patcher = mock.patch.object(HARNESS.subprocess, "run", side_effect=_python_only_runner(self.dir))
        patcher.start()
        self.addCleanup(patcher.stop)

    def _run(self):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            code = HARNESS.cmd_verify(argparse.Namespace(base="frozen-base"))
        return code, json.loads(HARNESS.LAST_RUN_PATH.read_text("utf-8"))

    def _mode(self, gate, mode):
        self.controls.write_text(json.dumps({gate["id"]: mode}), encoding="utf-8")

    def _gate(self, stage):
        return next(g for g in self.gates if g.get("stage", "fast") == stage and g.get("env") == "python")

    def _small_registry(self):
        self.gates = [self._gate("fast"), self._gate("deep")]

    def _decision(self):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return HARNESS.cmd_merge_decision(argparse.Namespace(evidence_dir=None))

    def _context_summary(self, document):
        spec = importlib.util.spec_from_file_location("context_under_test", ROOT / "scripts/harness_context.py")
        self.assertIsNotNone(spec)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        scripts = self.dir / "scripts"
        scripts.mkdir(exist_ok=True)
        (scripts / "harness-last-run.json").write_text(json.dumps(document), encoding="utf-8")
        with mock.patch.object(module, "ROOT", self.dir), mock.patch.object(module, "KNOWLEDGE", self.dir / "absent.yaml"):
            return module.recent_failures([])["last_run"]

    def test_success_keeps_the_whole_contract_in_order_and_attests_both_phases(self):
        code, document = self._run()
        self.assertEqual(0, code)
        self.assertEqual([g["id"] for g in self.gates], [r["id"] for r in document["gates"]])
        self.assertEqual(len(self.gates), len({r["id"] for r in document["gates"]}))
        self.assertEqual(["fast", "deep"], self.stages)
        self.assertEqual(self.phase_documents, [p["run"] for p in document["phase_runs"]])
        original = {row["id"]: row for phase in self.phase_documents for row in phase["gates"]}
        self.assertEqual(original, {row["id"]: row for row in document["gates"]})
        for row in document["gates"]:
            if row["status"] == HARNESS.S_PASS:
                self.assertEqual(64, len(row["output_sha256"]))
                self.assertEqual(hashlib.sha256(row["output_tail"].encode()).hexdigest(), row["tail_sha256"])
                self.assertIn("evidence:" + row["id"], row["output_tail"])
                self.assertGreaterEqual(row["duration_s"], 0)
        self.assertEqual(HARNESS.VERDICT_PASS, document["verdict"])
        self.assertEqual(0, self._decision())
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(0, HARNESS.cmd_attest(argparse.Namespace(task_id="isolated", agent="test")))
        bundle = HARNESS.ARTIFACT_ROOT / self.context["commit_sha"]
        evidence = json.loads((bundle / "evidence-index.json").read_text("utf-8"))
        self.assertEqual([g["id"] for g in self.gates], [r["id"] for r in evidence["gates"]])
        self.assertEqual(document, json.loads((bundle / "gate-results.json").read_text("utf-8")))
        self.assertEqual({
            "verdict": HARNESS.VERDICT_PASS,
            "commit_sha": self.context["commit_sha"], "failed_gates": [],
        }, self._context_summary(document))

    def test_fast_failure_never_executes_or_certifies_deep(self):
        self._mode(self._gate("fast"), "fail")
        code, document = self._run()
        self.assertEqual(1, code)
        self.assertEqual(["fast"], self.stages)
        self.assertEqual(HARNESS.VERDICT_FAIL, document["verdict"])
        deep_ids = {g["id"] for g in self.gates if g.get("stage", "fast") == "deep"}
        self.assertTrue(deep_ids.isdisjoint(self.calls.read_text("utf-8").splitlines()))
        unrun = [r for r in document["gates"] if r["id"] in deep_ids]
        self.assertTrue(unrun)
        self.assertTrue(all(r["status"] == HARNESS.S_SKIPPED and r["exit_code"] is None for r in unrun))
        self.assertEqual(HARNESS.VERDICT_BLOCKED, HARNESS.evaluate_verdict(unrun)[0])
        self.assertEqual(1, self._decision())

    def test_deep_failure_retains_fast_evidence_and_recomputes_the_final_verdict(self):
        self._mode(self._gate("deep"), "fail")
        code, document = self._run()
        self.assertEqual(1, code)
        self.assertEqual(["fast", "deep"], self.stages)
        self.assertEqual(HARNESS.VERDICT_FAIL, document["verdict"])
        combined = {r["id"]: r for r in document["gates"]}
        for row in self.phase_documents[0]["gates"]:
            self.assertEqual(row, combined[row["id"]])
        self.assertEqual(1, self._decision())
        self.assertEqual([self._gate("deep")["id"]], self._context_summary(document)["failed_gates"])

    def test_changed_commit_tree_or_base_cannot_be_merged_between_phases(self):
        self._small_registry()
        for key in ("commit_sha", "tree_sha", "base_sha"):
            with self.subTest(binding=key):
                original = self.context[key]
                self.stages.clear()
                self.after_stage = lambda stage: self.context.update({key: "d" * 40}) if stage == "fast" else None
                code, document = self._run()
                self.assertEqual(2, code)
                self.assertEqual(["fast"], self.stages)
                self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
                self.assertTrue(document["verification_errors"])
                self.assertTrue(all(r["status"] == HARNESS.S_UNVERIFIED for r in document["gates"]))
                self.assertEqual(1, self._decision())
                self.context[key] = original

    def test_source_change_during_deep_preserves_fast_but_rejects_deep_certification(self):
        self._small_registry()
        self.after_stage = lambda stage: self.context.update(tree_sha="d" * 40) if stage == "deep" else None
        code, document = self._run()
        self.assertEqual(2, code)
        self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
        fast_ids = {r["id"] for r in self.phase_documents[0]["gates"]}
        for row in document["gates"]:
            if row["id"] in fast_ids:
                self.assertNotEqual(HARNESS.S_UNVERIFIED, row["status"])
            else:
                self.assertEqual(HARNESS.S_UNVERIFIED, row["status"])
                self.assertTrue(row["output_sha256"] or row["observed_status"] == HARNESS.S_DELEGATED)

    def test_working_tree_becoming_dirty_cannot_certify_the_same_commit(self):
        self._small_registry()
        self.after_stage = lambda stage: setattr(self, "dirty", True) if stage == "fast" else None
        code, document = self._run()
        self.assertEqual(2, code)
        self.assertEqual(["fast"], self.stages)
        self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
        self.assertTrue(any("dirty" in error for error in document["verification_errors"]))
        self.assertEqual(1, self._decision())

    def test_initial_dirty_state_keeps_the_existing_clean_tree_gate_rule(self):
        self._small_registry()
        self.dirty = True
        gate = self._gate("fast")
        gate["require_clean_tree"] = True
        code, document = self._run()
        self.assertEqual(2, code)
        row = next(r for r in document["gates"] if r["id"] == gate["id"])
        self.assertEqual(HARNESS.S_BLOCKED, row["status"])
        self.assertIn("require_clean_tree", row["reason"])

    def test_missing_deep_evidence_is_unverified_and_does_not_reuse_fast_results(self):
        self._small_registry()
        self.after_stage = lambda stage: HARNESS.LAST_RUN_PATH.unlink() if stage == "deep" else None
        code, document = self._run()
        self.assertEqual(2, code)
        self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
        combined = {r["id"]: r for r in document["gates"]}
        for row in self.phase_documents[0]["gates"]:
            self.assertEqual(row, combined[row["id"]])
        self.assertEqual(1, self._decision())

    def test_duplicate_or_missing_phase_rows_cannot_silently_reduce_gate_coverage(self):
        self._small_registry()
        for mode in ("duplicate", "missing"):
            with self.subTest(rows=mode):
                def alter_rows(stage):
                    if stage == "deep":
                        document = json.loads(HARNESS.LAST_RUN_PATH.read_text("utf-8"))
                        if mode == "duplicate":
                            document["gates"].append(copy.deepcopy(document["gates"][0]))
                        else:
                            document["gates"].pop()
                        HARNESS.LAST_RUN_PATH.write_text(json.dumps(document), encoding="utf-8")
                self.after_stage = alter_rows
                code, document = self._run()
                self.assertEqual(2, code)
                self.assertEqual(len(self.gates), len(document["gates"]))
                self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
                self.assertEqual(1, self._decision())

    def test_a_phase_document_with_another_source_is_rejected_even_if_head_is_stable(self):
        self._small_registry()
        def replace_binding(stage):
            if stage == "deep":
                document = json.loads(HARNESS.LAST_RUN_PATH.read_text("utf-8"))
                document["tree_sha"] = "d" * 40
                HARNESS.LAST_RUN_PATH.write_text(json.dumps(document), encoding="utf-8")
        self.after_stage = replace_binding
        code, document = self._run()
        self.assertEqual(2, code)
        self.assertEqual(HARNESS.VERDICT_BLOCKED, document["verdict"])
        self.assertEqual("d" * 40, document["phase_runs"][1]["run"]["tree_sha"])
        self.assertEqual(self.context["tree_sha"], document["tree_sha"])
        self.assertEqual(1, self._decision())

    def test_scope_environment_skip_and_delegation_keep_their_original_states(self):
        absent = self._gate("fast")
        absent["scope"] = ["absent-scope"]
        skipped = self._gate("deep")
        self._mode(skipped, "skip")
        code, document = self._run()
        self.assertEqual(0, code)
        rows = {r["id"]: r for r in document["gates"]}
        self.assertEqual(HARNESS.S_SKIPPED, rows[absent["id"]]["status"])
        self.assertFalse(rows[absent["id"]]["scope_applies"])
        self.assertEqual(HARNESS.S_SKIPPED, rows[skipped["id"]]["status"])
        self.assertEqual("env", rows[skipped["id"]]["skip_kind"])
        delegated = [g for g in self.gates if g.get("delegated_to") and g.get("env") in HARNESS.CI_ONLY_ENVS]
        self.assertTrue(delegated)
        self.assertTrue(all(rows[g["id"]]["status"] == HARNESS.S_DELEGATED for g in delegated))
        self.assertEqual(HARNESS.VERDICT_PASS, document["verdict"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
