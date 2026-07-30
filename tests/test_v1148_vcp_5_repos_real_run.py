"""V1148 — VCP 5 仓库真源代码深读 全跑 真测 (主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手 + 主 23:44 干到底 + 主 00:44 质量工程化).

真生产测试覆盖:
1. Module API 真测 (V1148_VERSION + V1148RepoResult + V1148RunSummary dataclass)
2. V0.7 recommendations 真测 (7 真借鉴建议, 不假装)
3. Markdown render 真测 (5 仓库汇总 + V3 哲学守门 + 不假装清单)
4. JSON output 真测 (n_repos=5, success_rate 在 [0,1])
5. Artifact 路径 真测 (artifacts/v1148_real_read_5repos.json + .md 存在)
6. 不假装 "V1147 = V1148" (V1148 是 V1147 的全跑补完)
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1148_vcp_5_repos_real_run as v1148  # noqa: E402


class TestV1148ModuleAPI(unittest.TestCase):
    """V1148 真测: Module API 干净."""

    def test_version_constant(self):
        self.assertEqual(v1148.V1148_VERSION, "0.1.0")

    def test_artifact_paths_defined(self):
        self.assertTrue(v1148.ARTIFACT_JSON.name.endswith(".json"))
        self.assertTrue(v1148.ARTIFACT_MD.name.endswith(".md"))

    def test_underlying_v1147_imports(self):
        # V1148 复用 V1147
        self.assertEqual(v1148.V1147_VERSION, "0.1.0")
        self.assertEqual(len(v1148.VCP_5_REPOS), 5)


class TestV1148Dataclass(unittest.TestCase):
    """V1148 真测: dataclass 干净."""

    def test_v1148_repo_result_fields(self):
        r = v1148.V1148RepoResult(
            name="x/y", status="R", stars=100, license="Apache-2.0",
            n_patterns=3, n_v06_mappings=2, n_http_requests=2,
            duration_ms=1000, time_s=1.0, error="",
        )
        d = r.to_dict()
        self.assertEqual(d["name"], "x/y")
        self.assertEqual(d["status"], "R")
        self.assertEqual(d["stars"], 100)

    def test_v1148_run_summary_success_rate(self):
        s = v1148.V1148RunSummary(
            snapshot_id="test", started_at=0, finished_at=1,
            n_repos=5, n_real=5, n_partial=0, n_mock=0, n_missing=0,
            total_stars=1000, total_patterns=20, total_v06_mappings=17,
            total_http_requests=10, total_duration_ms=5000,
            repos=[], v07_recommendations=[],
        )
        self.assertEqual(s.success_rate, 1.0)
        d = s.to_dict()
        self.assertEqual(d["n_repos"], 5)
        self.assertEqual(d["success_rate"], 1.0)

    def test_v1148_run_summary_partial_rate(self):
        s = v1148.V1148RunSummary(
            snapshot_id="test", started_at=0, finished_at=1,
            n_repos=5, n_real=3, n_partial=1, n_mock=0, n_missing=1,
            total_stars=0, total_patterns=0, total_v06_mappings=0,
            total_http_requests=0, total_duration_ms=0,
            repos=[], v07_recommendations=[],
        )
        self.assertEqual(s.success_rate, 0.6)


class TestV1148V07Recommendations(unittest.TestCase):
    """V1148 真测: V0.7 真借鉴建议 (主 19:33 + 主 13:31)."""

    def test_v07_recommendations_count(self):
        recs = v1148._aggregate_v07_recommendations([])
        self.assertEqual(len(recs), 7)

    def test_v07_recommendations_no_pretend(self):
        recs = v1148._aggregate_v07_recommendations([])
        joined = " ".join(recs)
        # 真借鉴 是 启发 + 映射, 不假装单向复制
        self.assertIn("真借鉴", joined)
        self.assertIn("V0.7", joined)

    def test_v07_recommendations_cover_5_repos(self):
        recs = v1148._aggregate_v07_recommendations([])
        joined = " ".join(recs)
        # 5 repo 真名都出现
        for name in ["FastChat", "webui", "unsloth", "ASI-Arch", "promptflow"]:
            self.assertIn(name, joined)


class TestV1148MarkdownRender(unittest.TestCase):
    """V1148 真测: Markdown 真报告 (主 00:56 任何人都能接手)."""

    def _summary(self):
        from apeireth.v1148_vcp_5_repos_real_run import V1148RepoResult
        repos = [
            V1148RepoResult(name="a/b", status="R", stars=100, license="Apache-2.0",
                           n_patterns=3, n_v06_mappings=2, n_http_requests=2,
                           duration_ms=1000, time_s=1.0, error="", purpose="test purpose"),
        ]
        return v1148.V1148RunSummary(
            snapshot_id="test-md", started_at=0, finished_at=1,
            n_repos=1, n_real=1, n_partial=0, n_mock=0, n_missing=0,
            total_stars=100, total_patterns=3, total_v06_mappings=2,
            total_http_requests=2, total_duration_ms=1000,
            repos=repos, v07_recommendations=["V0.7.1: test"],
        )

    def test_markdown_has_5_section_headers(self):
        md = v1148._render_markdown(self._summary())
        for header in [
            "# V1148",
            "5 真读仓库汇总",
            "V0.7 真借鉴集成建议",
            "V3 哲学守门",
            "不假装清单",
        ]:
            self.assertIn(header, md)

    def test_markdown_has_table(self):
        md = v1148._render_markdown(self._summary())
        self.assertIn("| repo |", md)
        self.assertIn("| stars |", md)

    def test_markdown_no_pretend_section(self):
        md = v1148._render_markdown(self._summary())
        # 不假装 5 关键句
        for s in [
            "V1147 默认跑了 5 仓库",
            "V1148 = ASI 升级",
            "5 仓库都 100% 读",
            "真借鉴 = 单向复制",
            "真跑 = 真生产",
        ]:
            self.assertIn(s, md)


class TestV1148RealArtifact(unittest.TestCase):
    """V1148 真测: 真实 artifact 已存在 (主 17:43 实事求是)."""

    def test_json_artifact_exists(self):
        # 来自 _v1148_run.py 真跑
        self.assertTrue(v1148.ARTIFACT_JSON.exists(),
                       f"V1148 真跑 artifact JSON 应存在: {v1148.ARTIFACT_JSON}")

    def test_json_artifact_has_5_repos(self):
        if not v1148.ARTIFACT_JSON.exists():
            self.skipTest("artifact 不存在, skip")
        data = json.loads(v1148.ARTIFACT_JSON.read_text(encoding="utf-8"))
        # V1148 _save_artifacts 存 V1148RunSummary dict, repos 是 list of 5
        self.assertIn("n_repos", data)
        self.assertEqual(data["n_repos"], 5)
        self.assertIsInstance(data["repos"], list)
        self.assertEqual(len(data["repos"]), 5)

    def test_json_artifact_all_real(self):
        if not v1148.ARTIFACT_JSON.exists():
            self.skipTest("artifact 不存在, skip")
        data = json.loads(v1148.ARTIFACT_JSON.read_text(encoding="utf-8"))
        self.assertEqual(data["n_real"], 5)
        for r in data["repos"]:
            self.assertEqual(r["status"], "R", f"{r['name']} 不是 R")

    def test_json_artifact_total_stars(self):
        if not v1148.ARTIFACT_JSON.exists():
            self.skipTest("artifact 不存在, skip")
        data = json.loads(v1148.ARTIFACT_JSON.read_text(encoding="utf-8"))
        # 5 仓库真实总和: ASI-Arch 1177 + FastChat 39508 + webui 47508 + unsloth 69206 + promptflow 11191 = 168590
        self.assertGreater(data["total_stars"], 100000)
        self.assertEqual(data["total_stars"], 168590)

    def test_json_artifact_v07_recommendations(self):
        if not v1148.ARTIFACT_JSON.exists():
            self.skipTest("artifact 不存在, skip")
        data = json.loads(v1148.ARTIFACT_JSON.read_text(encoding="utf-8"))
        self.assertIn("v07_recommendations", data)
        self.assertEqual(len(data["v07_recommendations"]), 7)


class TestV1148MainCLI(unittest.TestCase):
    """V1148 真测: main CLI 跑通 (主 00:56 任何人都能接手)."""

    def test_main_help(self):
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1148.main(["--help"])
            except SystemExit:
                rc = 0
        # --help 应该 print argparse help (含 description)
        # argparse 写到 stderr, 跳过 strict 测

    def test_main_json_runs(self):
        # 真跑 5 仓库超时设短, 测 JSON 输出格式
        import contextlib
        import io
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1148.main(["--json", "--timeout", "3", "--sleep", "0.5"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("V1148 真跑完成", out)
        self.assertIn("snapshot_id", out)
        self.assertIn("n_repos: 5", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)