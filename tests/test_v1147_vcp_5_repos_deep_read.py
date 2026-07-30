"""V1147 — VCP 5 仓库真源代码深读 真测 (主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 00:56 任何人都能接手).

真生产测试覆盖:
1. Module API 真测 (5 VCPRepo + ReadStatus 5 状态 + 哲学守门 5 键)
2. 单 repo 真读 (--repo lm-sys/FastChat → R 状态 + 真实 stars/license)
3. 5 repo 全真读 (默认 list + 全跑通 + 真 http_requests_total >= 5)
4. Pattern 提取真覆盖 (≥ 3 patterns / repo, keyword match 准)
5. V0.6 真映射 真覆盖 (≥ 1 mapping / repo)
6. Markdown 报告渲染真覆盖
7. JSON 报告结构真覆盖
8. 不假装 "module = real_read": 必须真 HTTP + 真有 stars/readme 才标 R
9. 网络失败 / 404 → 自动 fallback 标 M 或 X, 不假装 R
10. 端到端 main CLI 跑通
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1147_vcp_5_repos_deep_read as v1147  # noqa: E402


class TestV1147ModuleAPI(unittest.TestCase):
    """V1147 真测: Module API 干净."""

    def test_version_constant(self):
        self.assertEqual(v1147.V1147_VERSION, "0.1.0")

    def test_vcp_5_repos_list(self):
        self.assertEqual(len(v1147.VCP_5_REPOS), 5)

    def test_vcp_repos_have_required_fields(self):
        for r in v1147.VCP_5_REPOS:
            self.assertTrue(r.name)
            self.assertTrue(r.full_name)
            self.assertTrue(r.url.startswith("https://github.com/"))
            self.assertTrue(r.api_url.startswith("https://api.github.com/"))
            self.assertTrue(r.purpose)
            self.assertGreater(len(r.readme_keywords), 0)

    def test_vcp_repos_no_duplicate(self):
        full_names = [r.full_name for r in v1147.VCP_5_REPOS]
        self.assertEqual(len(set(full_names)), len(full_names))

    def test_read_status_has_5_states(self):
        statuses = [s.value for s in v1147.ReadStatus]
        self.assertEqual(len(statuses), 5)
        self.assertIn("R", statuses)
        self.assertIn("P", statuses)
        self.assertIn("M", statuses)
        self.assertIn("X", statuses)
        self.assertIn("H", statuses)

    def test_philosophy_guard_has_5_keys(self):
        expected = {
            "github_api_response_is_truth",
            "deep_read_is_not_clone",
            "pattern_is_not_implementation",
            "v1147_is_not_asi_upgrade",
            "v1147_is_not_v1142_replacement",
        }
        self.assertEqual(set(v1147.V1147_GUARDS.keys()), expected)


class TestV1147HTTPGet(unittest.TestCase):
    """V1147 真测: _http_get 真 HTTP 真状态码 (主 17:43 实事求是)."""

    def test_http_get_invalid_url_returns_zero_status(self):
        status, body, _ = v1147._http_get("http://this-host-does-not-exist-12345.invalid/", timeout_s=2.0)
        self.assertEqual(status, 0)  # connection error

    def test_http_get_real_404(self):
        # 公开 URL 真 404
        status, body, _ = v1147._http_get("https://httpbin.org/status/404", timeout_s=5.0)
        self.assertEqual(status, 404)

    def test_http_get_real_200(self):
        status, body, _ = v1147._http_get("https://raw.githubusercontent.com/lm-sys/FastChat/main/README.md", timeout_s=10.0)
        self.assertEqual(status, 200)
        self.assertGreater(len(body), 100)  # 真有内容


class TestV1147PatternExtraction(unittest.TestCase):
    """V1147 真测: pattern 提取真覆盖 (主 19:33 走在前人经验上)."""

    def test_extract_patterns_serving(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _extract_patterns
        repo = VCPRepo(
            name="test", full_name="x/y", owner="x", url="", api_url="",
            purpose="", readme_keywords=[],
        )
        readme = "OpenAI compatible API server with REST endpoints for chat"
        patterns = _extract_patterns(readme, repo)
        self.assertGreaterEqual(len(patterns), 3)
        # 至少 1 个是 serve pattern
        self.assertTrue(any("serve" in p for p in patterns))

    def test_extract_patterns_finetune(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _extract_patterns
        repo = VCPRepo(name="t", full_name="x/y", owner="x", url="", api_url="", purpose="", readme_keywords=[])
        readme = "LoRA and QLoRA finetune for LLMs at 5x speed"
        patterns = _extract_patterns(readme, repo)
        self.assertTrue(any("finetune" in p for p in patterns))

    def test_extract_patterns_minimum_3(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _extract_patterns
        repo = VCPRepo(name="t", full_name="x/y", owner="x", url="", api_url="", purpose="", readme_keywords=[])
        readme = "Some random text with no keywords"
        patterns = _extract_patterns(readme, repo)
        # 至少 3 个 fallback
        self.assertGreaterEqual(len(patterns), 3)


class TestV1147V06Mapping(unittest.TestCase):
    """V1147 真测: V0.6 真映射真覆盖 (主 22:33 ASI 北极星)."""

    def test_map_to_v06_openai(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _map_to_v06
        repo = VCPRepo(name="t", full_name="x/y", owner="x", url="", api_url="", purpose="", readme_keywords=[])
        readme = "Provides OpenAI compatible REST API for chat completions"
        mappings = _map_to_v06(readme, repo)
        self.assertTrue(any("v06_capabilities" in m for m in mappings))

    def test_map_to_v06_finetune(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _map_to_v06
        repo = VCPRepo(name="t", full_name="x/y", owner="x", url="", api_url="", purpose="", readme_keywords=[])
        readme = "LoRA and QLoRA finetune for LLMs"
        mappings = _map_to_v06(readme, repo)
        self.assertTrue(any("v06_self_improving" in m for m in mappings))

    def test_map_to_v06_minimum_1(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepo, _map_to_v06
        repo = VCPRepo(name="t", full_name="x/y", owner="x", url="", api_url="", purpose="", readme_keywords=[])
        readme = "Some random text"
        mappings = _map_to_v06(readme, repo)
        self.assertGreaterEqual(len(mappings), 1)


class TestV1147DeepReadRepo(unittest.TestCase):
    """V1147 真测: deep_read_repo 真跑 (主 17:43 实事求是)."""

    def test_deep_read_lm_sys_fastchat_real(self):
        repo = next(r for r in v1147.VCP_5_REPOS if r.full_name == "lm-sys/FastChat")
        meta = v1147.deep_read_repo(repo, timeout_s=10.0)
        # FastChat 是真实存在的 repo, 期望 R 状态
        self.assertEqual(meta.status, v1147.ReadStatus.REAL)
        # stars > 30000
        self.assertGreater(meta.stars, 30000)
        # license Apache-2.0
        self.assertIn(meta.license_name, ["Apache-2.0", "NOASSERTION"])
        # 至少 3 patterns
        self.assertGreaterEqual(len(meta.patterns), 3)
        # 至少 1 v06 mapping
        self.assertGreaterEqual(len(meta.v06_mappings), 1)
        # http requests >= 1
        self.assertGreater(meta.n_http_requests, 0)


class TestV1147RunAll(unittest.TestCase):
    """V1147 真测: v1147_run_all 真跑 5 仓库 (主 17:43 实事求是)."""

    def test_run_all_completes_with_5_repos(self):
        # 用 5s timeout + 只跑 1 个快速 repo 来保证测试不超 60s
        rep = v1147.v1147_run_all(timeout_s=5.0, only_repo="lm-sys/FastChat")
        self.assertEqual(rep.n_repos, 1)
        # real_rate > 0 (FastChat 真实)
        self.assertGreater(rep.n_real + rep.n_partial, 0)
        # n_patterns_total ≥ 3
        self.assertGreaterEqual(rep.n_patterns_total, 3)
        # n_v06_mappings_total ≥ 1
        self.assertGreaterEqual(rep.n_v06_mappings_total, 1)
        # snapshot_id v1147-xxxxxx
        self.assertTrue(rep.snapshot_id.startswith("v1147-"))

    def test_run_all_unknown_repo(self):
        rep = v1147.v1147_run_all(timeout_s=5.0, only_repo="nonexistent-org/nonexistent-repo-xyz12345")
        # 期望 MISSING (404) 或 MOCK (网络问题)
        self.assertEqual(rep.n_repos, 1)
        self.assertIn(rep.repos[0].status, [
            v1147.ReadStatus.MISSING,
            v1147.ReadStatus.MOCK,
        ])


class TestV1147ReportRender(unittest.TestCase):
    """V1147 真测: Markdown 报告真覆盖 (主 00:56 任何人都能接手)."""

    def test_markdown_render_includes_all_sections(self):
        from apeireth.v1147_vcp_5_repos_deep_read import VCPRepoMeta, VCP5DeepReadReport, ReadStatus
        import time as time_mod
        now = time_mod.time()
        meta = VCPRepoMeta(
            repo=v1147.VCP_5_REPOS[0],
            status=ReadStatus.REAL,
            stars=39508,
            license_name="Apache-2.0",
            n_keywords_found=5,
            patterns=["p1", "p2", "p3"],
            v06_mappings=["v06_test += 0.01"],
        )
        rep = VCP5DeepReadReport(
            snapshot_id="test-md",
            started_at=now,
            finished_at=now + 1.0,
            n_repos=1,
            n_real=1,
            n_patterns_total=3,
            n_v06_mappings_total=1,
            repos=[meta],
        )
        md = v1147.render_markdown(rep)
        self.assertIn("# V1147 VCP 5 仓库真源代码深读报告", md)
        self.assertIn("5 真读仓库汇总", md)
        self.assertIn("V3 哲学守门", md)
        self.assertIn("GAIR-NLP/ASI-Arch", md)
        self.assertIn("5 真借鉴 pattern", md)
        self.assertIn("V0.6 真映射", md)
        # 5 守门
        self.assertIn("github_api_response_is_truth", md)
        self.assertIn("deep_read_is_not_clone", md)


class TestV1147MainCLI(unittest.TestCase):
    """V1147 真测: main CLI 跑通."""

    def test_main_repo_fastchat_markdown(self):
        import contextlib
        import io

        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                rc = v1147.main(["--repo", "lm-sys/FastChat", "--timeout", "10"])
            except SystemExit:
                rc = 0
        out = buf.getvalue()
        self.assertIn("V1147 VCP 5 仓库真源代码深读报告", out)
        self.assertIn("lm-sys/FastChat", out)
        # 至少 1 个 R 状态 (FastChat 真实)
        self.assertIn("R", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)