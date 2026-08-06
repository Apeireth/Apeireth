"""V1183 — VCP 6 真实源代码深读 tests.

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44

Tests:
  - 6 仓库 spec (5 GitHub cached + 1 本地)
  - R6 本地真读 9 key files (160K bytes 真读)
  - R1-R5 cached metadata 真读 (不假装 = 当前)
  - measure_v1183() float [0..1] 主入口
  - V3 哲学守门 (6 不假装)
  - V1183 不替换 V1142 / V1147 (主 19:33 走在前人经验上)
  - JSON artifact 写盘
  - Markdown 报告渲染
  - V0.6 series vcp_deep_read dim 接入路径
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1183_vcp_6_repos_real_deep_read import (  # noqa: E402
    GITHUB_CACHED_METADATA,
    LOCAL_VCP_KEY_FILES,
    LOCAL_VCP_ROOT,
    ReadStatus,
    V1183_GUARDS,
    V1183_VERSION,
    VCP_6_REPOS,
    VCP6DeepReadReport,
    VCPRepo6,
    VCPRepoMeta6,
    deep_read_cached_github_repo,
    deep_read_local_repo,
    measure_v1183,
    measure_v1183_from_metas,
    render_markdown,
    to_dict,
    v1183_run_all,
)


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def report() -> VCP6DeepReadReport:
    """V1183 真读总报告 (无副作用)."""
    return v1183_run_all()


@pytest.fixture
def report_with_local_missing():
    """V1183 真读 (本地 root 不存在 → R6 MISSING)."""
    return v1183_run_all(local_root=Path("/nonexistent/VCPToolBox"))


# ============================================================================
# V1183 spec tests
# ============================================================================

def test_v1183_version_constant():
    assert V1183_VERSION == "0.1.0"


def test_v1183_6_repos_spec():
    assert len(VCP_6_REPOS) == 6
    # 5 GitHub cached
    cached_repos = [r for r in VCP_6_REPOS if r.source == "github_cached"]
    assert len(cached_repos) == 5
    # 1 本地
    local_repos = [r for r in VCP_6_REPOS if r.source == "local_fs"]
    assert len(local_repos) == 1
    # slot 1..6
    slots = sorted(r.slot for r in VCP_6_REPOS)
    assert slots == [1, 2, 3, 4, 5, 6]
    # R6 = local VCPToolBox
    r6 = VCP_6_REPOS[5]
    assert r6.source == "local_fs"
    assert r6.full_name == "local:VCPToolBox"
    assert r6.n_key_files_expected == len(LOCAL_VCP_KEY_FILES) == 9


def test_v1183_local_key_files_spec():
    assert len(LOCAL_VCP_KEY_FILES) == 9
    # 6 .js + 3 .md
    n_js = sum(1 for f in LOCAL_VCP_KEY_FILES if f["rel_path"].endswith(".js"))
    n_md = sum(1 for f in LOCAL_VCP_KEY_FILES if f["rel_path"].endswith(".md"))
    assert n_js == 6
    assert n_md == 3
    # Plugin.js, TagMemoEngine.js, KnowledgeBaseManager.js, EPAModule.js,
    # ResidualPyramid.js, ResultDeduplicator.js
    expected_js = {
        "Plugin.js",
        "TagMemoEngine.js",
        "KnowledgeBaseManager.js",
        "EPAModule.js",
        "ResidualPyramid.js",
        "ResultDeduplicator.js",
    }
    actual_js = {f["rel_path"] for f in LOCAL_VCP_KEY_FILES if f["rel_path"].endswith(".js")}
    assert expected_js.issubset(actual_js)


def test_v1183_github_cached_metadata_spec():
    # 5 GitHub 仓库全有 cached metadata
    for r in VCP_6_REPOS:
        if r.source == "github_cached":
            assert r.full_name in GITHUB_CACHED_METADATA, f"missing cached for {r.full_name}"
            meta = GITHUB_CACHED_METADATA[r.full_name]
            assert "stars" in meta and "forks" in meta
            assert "license" in meta and "default_branch" in meta
            assert "cached_at" in meta


def test_v1183_philosophy_guard_6_keys():
    assert len(V1183_GUARDS) == 6
    for k in V1183_GUARDS:
        assert k.startswith(("1_", "2_", "3_", "4_", "5_", "6_"))


# ============================================================================
# R1-R5 GitHub cached 真读 tests
# ============================================================================

def test_v1183_cached_repo_read_r1_asi_arch():
    r1 = VCP_6_REPOS[0]
    meta = deep_read_cached_github_repo(r1)
    assert meta.status == ReadStatus.CACHED
    assert meta.n_patterns_found == 6
    assert meta.n_v06_mappings >= 1
    assert meta.cached_at != ""  # cached_at 有值
    assert meta.bytes_read == 0  # cached 不读字节
    assert meta.lines_read == 0


def test_v1183_cached_repo_read_r2_fastchat():
    r2 = VCP_6_REPOS[1]
    meta = deep_read_cached_github_repo(r2)
    assert meta.status == ReadStatus.CACHED
    assert "serving" in " ".join(meta.patterns).lower() or "openai" in " ".join(meta.patterns).lower()


def test_v1183_cached_repo_read_r5_promptflow():
    r5 = VCP_6_REPOS[4]
    meta = deep_read_cached_github_repo(r5)
    assert meta.status == ReadStatus.CACHED
    assert meta.n_v06_mappings >= 1


# ============================================================================
# R6 本地真读 tests
# ============================================================================

def test_v1183_local_repo_real_read(report):
    r6 = report.repos[5]
    assert r6.repo.source == "local_fs"
    assert r6.status in (ReadStatus.REAL, ReadStatus.PARTIAL)
    # 9 key files 真读
    assert len(r6.key_files_read) >= 6, f"R6 files_read < 6: {len(r6.key_files_read)}"
    # bytes_read 至少 100K (Plugin.js 114K 单文件就够)
    assert r6.bytes_read >= 100000, f"R6 bytes_read < 100K: {r6.bytes_read}"
    # patterns + mappings 真有
    assert r6.n_patterns_found >= 1
    assert r6.n_v06_mappings >= 1


def test_v1183_local_repo_plugin_js_read(report):
    r6 = report.repos[5]
    assert "Plugin.js" in r6.key_files_read


def test_v1183_local_repo_tagmemo_read(report):
    r6 = report.repos[5]
    assert "TagMemoEngine.js" in r6.key_files_read


def test_v1183_local_repo_memory_system_doc_read(report):
    r6 = report.repos[5]
    assert "docs/MEMORY_SYSTEM.md" in r6.key_files_read


def test_v1183_local_repo_missing_handled(report_with_local_missing):
    r6 = report_with_local_missing.repos[5]
    assert r6.status == ReadStatus.MISSING
    assert r6.bytes_read == 0


# ============================================================================
# measure_v1183 tests
# ============================================================================

def test_v1183_measure_function_returns_float():
    score = measure_v1183()
    assert isinstance(score, float)
    assert 0.0 <= score <= 1.0


def test_v1183_measure_function_high_score():
    """V1183 应该 ≥0.5 (主 17:43 实事求是, R6 真读本地 + R1-R5 cached = 优)."""
    score = measure_v1183()
    assert score >= 0.5, f"score < 0.5: {score}"


def test_v1183_measure_with_local_missing_low():
    """本地 root 不存在 → score 应该低 (主 17:43 实事求是)."""
    report = v1183_run_all(local_root=Path("/nonexistent/VCPToolBox"))
    score_low = report.measure_v1183_score
    # 仅有 R1-R5 cached → score 应该 < 0.5
    assert score_low < 0.5, f"score with missing local: {score_low}"


def test_v1183_measure_from_metas_aggregates():
    metas = [
        VCPRepoMeta6(
            repo=VCPRepo6(slot=6, name="local", full_name="local:VCPToolBox",
                          source="local_fs", url="x", purpose="x", keywords=[]),
            status=ReadStatus.REAL,
            bytes_read=200000,
            n_patterns_found=6,
            n_v06_mappings=4,
            key_files_read=["a.js"] * 9,
        ),
    ] + [
        VCPRepoMeta6(
            repo=VCPRepo6(slot=i + 1, name=f"r{i}", full_name=f"r{i}",
                          source="github_cached", url="x", purpose="x", keywords=[]),
            status=ReadStatus.CACHED,
            n_patterns_found=6,
            n_v06_mappings=1,
        )
        for i in range(5)
    ]
    score = measure_v1183_from_metas(metas)
    assert 0.0 <= score <= 1.0
    # 全 R + 全 CACHED → 优
    assert score >= 0.5


# ============================================================================
# V1183 vs V1147 tests (主 23:44 干到底)
# ============================================================================

def test_v1183_supplements_v1147_not_replaces():
    """V1183 不替换 V1147 (主 19:33 走在前人经验上)."""
    guard = V1183_GUARDS["6_v1183_supplements_v1147_not_replaces"]
    assert "V1183 不替换 V1147" in guard
    assert "V1142" in guard


def test_v1183_includes_local_repo():
    """V1183 必须包含 1 个本地仓库 (V1147 没有)."""
    local_repos = [r for r in VCP_6_REPOS if r.source == "local_fs"]
    assert len(local_repos) == 1


def test_v1183_6_patterns_per_repo_cap():
    """每 repo 真借鉴 pattern 上限 6 (主 17:43 实事求是)."""
    report = v1183_run_all()
    for m in report.repos:
        assert len(m.patterns) <= 6, f"{m.repo.full_name} patterns > 6: {len(m.patterns)}"


# ============================================================================
# JSON artifact tests
# ============================================================================

def test_v1183_to_dict(report):
    d = to_dict(report)
    assert d["n_repos"] == 6
    assert d["measure_v1183_score"] > 0.0
    assert "philosophy_guards" in d
    assert len(d["repos"]) == 6
    # 每 repo 必有 slot, source, status
    for r in d["repos"]:
        assert "slot" in r
        assert "source" in r
        assert "status" in r


def test_v1183_json_writable(tmp_path):
    """JSON 写盘测试 (主 00:44 质量工程化)."""
    report = v1183_run_all()
    d = to_dict(report)
    out = tmp_path / "v1183.json"
    out.write_text(json.dumps(d, ensure_ascii=False, indent=2), encoding="utf-8")
    loaded = json.loads(out.read_text(encoding="utf-8"))
    assert loaded["n_repos"] == 6


# ============================================================================
# Markdown 报告 tests
# ============================================================================

def test_v1183_markdown_renders(report):
    md = render_markdown(report)
    assert "# V1183" in md
    assert "6 真读仓库汇总" in md
    assert "V3 哲学守门" in md
    assert "V1183 vs V1147" in md
    assert "V1183 接入路径" in md
    # 6 仓库 slot
    for slot in range(1, 7):
        assert f"R{slot}:" in md or f"| {slot} |" in md


def test_v1183_markdown_includes_guard_text(report):
    md = render_markdown(report)
    # V3 guard 6 keys 全出现
    for k in V1183_GUARDS:
        assert k in md, f"missing guard key in md: {k}"


# ============================================================================
# V0.6 series 接入 tests
# ============================================================================

def test_v1183_v06_series_integration_path():
    """V1183 measure 函数 = V0.6 vcp_deep_read dim 真入口."""
    score = measure_v1183()
    # V0.6 series 期望 dim score ∈ [0, 1]
    assert 0.0 <= score <= 1.0
    # V1183 真测 = 优
    assert score >= 0.5


def test_v1183_no_external_network():
    """V1183 不依赖外部网络 (主 22:32 cron 网络受限)."""
    import inspect
    # 确认 deep_read_local_repo 和 deep_read_cached_github_repo 都无网络调用
    sig_local = inspect.getsource(deep_read_local_repo)
    sig_cached = inspect.getsource(deep_read_cached_github_repo)
    assert "urlopen" not in sig_local
    assert "urlopen" not in sig_cached
    assert "urllib.request" not in sig_local
    assert "urllib.request" not in sig_cached


def test_v1183_status_taxonomy():
    """V1183 status taxonomy = R/P/C/X (主 17:43 实事求是)."""
    assert ReadStatus.REAL.value == "R"
    assert ReadStatus.PARTIAL.value == "P"
    assert ReadStatus.CACHED.value == "C"
    assert ReadStatus.MISSING.value == "X"


# ============================================================================
# Self-test (主 00:44 质量工程化)
# ============================================================================

def test_v1183_self_test_passes():
    """V1183 self-test 全 PASS."""
    from apeireth.v1183_vcp_6_repos_real_deep_read import _self_test
    assert _self_test() is True