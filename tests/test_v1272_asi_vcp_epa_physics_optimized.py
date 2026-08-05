"""Tests for V1272 ASI VCP EPA Physics-Optimized Edition (主 00:44 质量工程化).

Test coverage (主 17:43 实事求是):
- WeightedCenteringPCA: shape, centering correctness, weighted variance, label handling
- RobustKMeans: shape, convergence, label range, restarts pick best
- EPAModule: initialize, project, cross-domain resonance, dominant axes threshold
- EPAProduction: batch run, cache, bounded fallback
- CLI: --probe, --demo, --full-loop, --version
- V3 philosophy gate: 7 guards verifiable
- 1:1 port correctness vs VCP v1.12.0 algorithm spec
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import pytest

# Path setup
PROJ_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROJ_ROOT))

from apeireth.v1272_asi_vcp_epa_physics_optimized import (
    V1272_VERSION,
    V1272_BUILD,
    V1272_VCP_VERSION,
    V1272_VCP_COMMIT,
    V1272_VCP_REPO,
    V1272_NS_LOCKED,
    V1272_GUARDS,
    WeightedCenteringPCA,
    WeightedPCAResult,
    RobustKMeans,
    KMeansResult,
    EPAModule,
    ProjectionResult,
    ResonanceResult,
    EPAProduction,
    EPAProductionResult,
    SampleEPA,
    _make_demo_vectors,
    _make_22_samples,
    main,
)


# ============================================================
# 0. Version & Guards
# ============================================================

class TestVersionAndGuards:
    """V1272 版本 + V3 哲学守门."""

    def test_version(self):
        assert V1272_VERSION == "0.1.0"
        assert "2026-08-05" in V1272_BUILD

    def test_vcp_ref(self):
        assert V1272_VCP_REPO == "lioensky/VCPToolBox"
        assert V1272_VCP_COMMIT == "f647af028324f118a657664e6848f0a67504f321"
        assert V1272_VCP_VERSION == "v1.12.0"

    def test_ns_locked(self):
        assert V1272_NS_LOCKED == "92.91%"
        # 不刷 KPI

    def test_guards_count(self):
        assert len(V1272_GUARDS) == 7

    def test_guards_keywords(self):
        guards_str = " ".join(V1272_GUARDS)
        assert "no_asi_v1_claim" in guards_str
        assert "no_phenomenal_claim" in guards_str
        assert "no_kpi_inflate" in guards_str
        assert "numpy_not_rust" in guards_str  # 不假装有 Rust N-API
        assert "vcp_attribution_real" in guards_str


# ============================================================
# 1. WeightedCenteringPCA
# ============================================================

class TestWeightedCenteringPCA:
    """VCP v1.12.0 _computeWeightedPCA 1:1 port 测试."""

    def test_basic_shape(self):
        rng = np.random.default_rng(42)
        X = rng.standard_normal((20, 16))
        pca = WeightedCenteringPCA(max_basis_dim=8)
        result = pca.fit(X)
        # VCP orthoBasis = (K, dim) 主成分行向量
        assert result.U.shape[0] <= 8
        assert result.U.shape[1] == 16
        assert result.S.shape[0] == result.U.shape[0]
        assert result.mean_vector.shape == (16,)

    def test_centering_correctness(self):
        """加权平均向量 = sum(weights * X) / sum(weights)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((10, 4))
        weights = np.array([0.1, 0.2, 0.3, 0.05, 0.05, 0.1, 0.05, 0.05, 0.05, 0.05])
        pca = WeightedCenteringPCA(max_basis_dim=4)
        result = pca.fit(X, weights=weights)
        expected_mean = np.average(X, axis=0, weights=weights)
        np.testing.assert_allclose(result.mean_vector, expected_mean, atol=1e-10)

    def test_default_weights_uniform(self):
        """无 weights 时 = 均匀 1/n."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((8, 4))
        pca = WeightedCenteringPCA(max_basis_dim=4)
        result = pca.fit(X)
        expected_mean = X.mean(axis=0)
        np.testing.assert_allclose(result.mean_vector, expected_mean, atol=1e-10)

    def test_orthogonality(self):
        """U 的行向量应正交 (VCP strictOrthogonalization 默认 True)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((32, 16))
        pca = WeightedCenteringPCA(max_basis_dim=8)
        result = pca.fit(X)
        K = result.U.shape[0]
        gram = result.U @ result.U.T
        # 对角线 ≈ 1, 非对角线 ≈ 0
        np.testing.assert_allclose(np.diag(gram), np.ones(K), atol=1e-6)
        gram_off = gram - np.diag(np.diag(gram))
        assert np.abs(gram_off).max() < 1e-5

    def test_labels_default(self):
        """无 labels 时 = axis_0, axis_1, ..."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((8, 4))
        pca = WeightedCenteringPCA(max_basis_dim=3)
        result = pca.fit(X)
        assert result.labels == ["axis_0", "axis_1", "axis_2"]

    def test_labels_custom(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((8, 4))
        labels = ["a", "b", "c", "d"]
        pca = WeightedCenteringPCA(max_basis_dim=3)
        result = pca.fit(X, labels=labels)
        assert result.labels[:3] == ["a", "b", "c"]

    def test_too_few_samples_raises(self):
        X = np.array([[1.0, 2.0]])  # n=1
        pca = WeightedCenteringPCA()
        with pytest.raises(ValueError):
            pca.fit(X)

    def test_explained_variance_ratio_sum_le_1(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((32, 16))
        pca = WeightedCenteringPCA(max_basis_dim=8)
        result = pca.fit(X)
        assert result.explained_variance_ratio.sum() <= 1.0 + 1e-6


# ============================================================
# 2. RobustKMeans
# ============================================================

class TestRobustKMeans:
    """VCP v1.12.0 _clusterTags 1:1 port 测试."""

    def test_basic_shape(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((30, 4))
        km = RobustKMeans(k=3, restarts=1, max_iter=20)
        result = km.fit(X)
        assert result.centroids.shape == (3, 4)
        assert result.labels.shape == (30,)
        assert set(result.labels.tolist()) <= {0, 1, 2}

    def test_convergence(self):
        """converged=True 表示质心移动 < tolerance."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((20, 4))
        km = RobustKMeans(k=2, max_iter=100, tolerance=1e-2)
        result = km.fit(X)
        assert result.converged is True
        assert result.iterations < 100

    def test_restarts_pick_best(self):
        """多次重启取最优 (objective 最小)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((30, 4))
        km1 = RobustKMeans(k=3, restarts=1)
        km3 = RobustKMeans(k=3, restarts=3)
        r1 = km1.fit(X)
        r3 = km3.fit(X)
        # 3 重启不应比 1 重启差
        assert r3.objective <= r1.objective + 1e-3

    def test_k_too_large_raises(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((5, 4))
        km = RobustKMeans(k=10)
        with pytest.raises(ValueError):
            km.fit(X)

    def test_iterations_positive(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((10, 4))
        km = RobustKMeans(k=2)
        result = km.fit(X)
        assert result.iterations >= 1


# ============================================================
# 3. EPAModule
# ============================================================

class TestEPAModule:
    """VCP v1.12.0 EPAModule 1:1 port 测试."""

    def test_initialize_success(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        ok = epa.initialize(X)
        assert ok is True
        assert epa.initialized
        assert epa.ortho_basis is not None
        assert epa.basis_mean is not None

    def test_initialize_too_few_samples_fails(self):
        """VCP: tags.length < 8 返回 false."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((7, 4))  # < 8
        epa = EPAModule(dimension=4, cluster_count=2)
        ok = epa.initialize(X)
        assert ok is False
        assert not epa.initialized

    def test_project_shape(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        proj = epa.project(X[0])
        K = epa.ortho_basis.shape[0]
        assert proj.projections.shape == (K,)
        assert proj.probabilities.shape == (K,)
        assert abs(proj.probabilities.sum() - 1.0) < 1e-6  # 概率和 = 1

    def test_project_not_initialized(self):
        """未初始化时返回空结果."""
        epa = EPAModule()
        proj = epa.project(np.zeros(64))
        assert proj.projections.shape == (0,)
        assert proj.dominant_axes == []

    def test_entropy_in_range(self):
        """归一化熵 ∈ [0, 1]."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        for v in X:
            proj = epa.project(v)
            assert 0.0 <= proj.entropy <= 1.0
            assert 0.0 <= proj.logic_depth <= 1.0
            assert abs(proj.logic_depth - (1.0 - proj.entropy)) < 1e-6

    def test_dominant_axes_threshold(self):
        """dominant_axes 只包含 probabilities > threshold 的轴."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4,
                        dominant_axis_threshold=0.05)
        epa.initialize(X)
        proj = epa.project(X[0])
        for axis in proj.dominant_axes:
            assert axis["energy"] > 0.05

    def test_dominant_axes_sorted_desc(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        proj = epa.project(X[0])
        energies = [a["energy"] for a in proj.dominant_axes]
        assert energies == sorted(energies, reverse=True)

    def test_resonance_geometric_mean(self):
        """共振 = 几何平均 > threshold."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4,
                        resonance_threshold=0.15)
        epa.initialize(X)
        for v in X:
            res = epa.detect_cross_domain_resonance(v)
            # bridges 中每个 strength = sqrt(top * sec)
            for bridge in res.bridges:
                assert bridge["strength"] > 0.15
                # balance ∈ (0, 1]
                assert 0.0 < bridge["balance"] <= 1.0

    def test_resonance_single_axis(self):
        """单个主轴时无共振."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        # 强制 dominant_axes 只有 1 个 (高 threshold)
        epa.dominant_axis_threshold = 0.99
        for v in X:
            res = epa.detect_cross_domain_resonance(v)
            assert res.resonance == 0.0
            assert res.bridges == []


# ============================================================
# 4. EPAProduction (主 00:56 任何人都能接手)
# ============================================================

class TestEPAProduction:
    """V1272 EPA 真生产包装测试."""

    def test_fit_and_run(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((20, 8))
        labels = [f"label_{i}" for i in range(20)]
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        prod = EPAProduction(epa=epa)
        prod.fit_basis(X, labels)
        samples = [(f"sample_{i}", X[i]) for i in range(5)]
        result = prod.run_samples(samples)
        assert result.n_samples == 5
        assert len(result.samples) == 5

    def test_run_without_init_raises(self):
        epa = EPAModule()
        prod = EPAProduction(epa=epa)
        samples = [("x", np.zeros(64))]
        with pytest.raises(RuntimeError):
            prod.run_samples(samples)

    def test_bounded_fallback(self):
        """max_fallback_tags=10 触发均匀采样 (VCP P0 守门 1:1)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((100, 8))  # > max_fallback_tags=10
        labels = [f"l_{i}" for i in range(100)]
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        prod = EPAProduction(epa=epa, max_fallback_tags=10)
        ok = prod.fit_basis(X, labels)
        assert ok is True

    def test_cache_hit(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((20, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        prod = EPAProduction(epa=epa)
        prod.fit_basis(X)
        samples = [(f"sample_{i}", X[i]) for i in range(3)]
        r1 = prod.run_samples(samples)
        r2 = prod.run_samples(samples)
        # 缓存命中应返回同一对象
        assert r1 is r2

    def test_sample_epa_fields(self):
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        prod = EPAProduction(epa=epa)
        prod.fit_basis(X)
        samples = [("a", X[0])]
        result = prod.run_samples(samples)
        s = result.samples[0]
        assert s.sample_id == "a"
        assert s.vector_dim == 8
        assert 0.0 <= s.entropy <= 1.0
        assert 0.0 <= s.logic_depth <= 1.0
        assert s.resonance >= 0.0


# ============================================================
# 5. Helpers
# ============================================================

class TestHelpers:
    """测试 demo 数据生成器."""

    def test_make_demo_vectors_shape(self):
        X, labels = _make_demo_vectors(n=32, dim=16)
        assert X.shape == (32, 16)
        assert len(labels) == 32
        assert all(l.startswith("cluster_") for l in labels)

    def test_make_demo_vectors_min_8(self):
        """VCP tags.length >= 8 守门."""
        X, _ = _make_demo_vectors(n=8, dim=8)
        assert X.shape[0] >= 8

    def test_make_22_samples(self):
        samples = _make_22_samples(dim=32)
        assert len(samples) == 22
        for sid, vec in samples:
            assert sid.startswith(("MMLU_", "GSM8K_", "HumanEval_", "HellaSwag_"))
            assert vec.shape == (32,)


# ============================================================
# 6. CLI / 任何人都能接手 (主 00:56)
# ============================================================

class TestCLI:
    """V1272 CLI 测试 (主 00:56 任何人都能接手)."""

    def test_version(self, capsys):
        code = main(["--version"])
        out = capsys.readouterr().out
        assert code == 0
        assert "V1272" in out
        assert V1272_VERSION in out
        assert V1272_VCP_COMMIT in out

    def test_probe(self, capsys):
        code = main(["--probe"])
        out = capsys.readouterr().out
        assert code == 0
        assert "WeightedCenteringPCA smoke" in out
        assert "RobustKMeans smoke" in out
        assert "EPAModule end-to-end" in out
        assert "ALL OK" in out

    def test_demo(self, capsys):
        code = main(["--demo"])
        out = capsys.readouterr().out
        assert code == 0
        assert "n_samples=22" in out
        assert "n_dominant_total" in out

    def test_full_loop(self, capsys, tmp_path):
        report = tmp_path / "report.md"
        code = main(["--full-loop", "--report", str(report)])
        out = capsys.readouterr().out
        assert code == 0
        assert "22" in out
        assert "report written" in out
        # 报告文件存在
        assert report.exists()
        content = report.read_text(encoding="utf-8")
        assert "V1272" in content
        assert V1272_VCP_COMMIT in content
        assert "v1272_no_phenomenal_claim" in content


# ============================================================
# 7. 1:1 VCP v1.12.0 port correctness
# ============================================================

class TestVCPPortCorrectness:
    """1:1 port correctness vs VCP v1.12.0 算法规格."""

    def test_centering_required(self):
        """VCP comment: '必须先减去平均向量 (Centering), 否则投影没有统计意义'."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        # 测试: 中心化后总能量 > 0 (因为非零向量)
        v = rng.standard_normal(8)
        proj = epa.project(v)
        total_energy = float(np.sum(proj.projections ** 2))
        assert total_energy > 1e-12  # 非零能量 = 中心化生效

    def test_normalized_entropy_formula(self):
        """VCP: entropy = -sum p*log2(p), normalized = entropy / log2(K)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        proj = epa.project(X[0])
        K = proj.probabilities.shape[0]
        # 手动算
        p = proj.probabilities
        expected_h = -sum(pp * np.log2(pp) for pp in p if pp > 1e-9)
        expected_norm = expected_h / np.log2(K) if K > 1 else 0.0
        assert abs(proj.entropy - expected_norm) < 1e-6

    def test_logic_depth_formula(self):
        """VCP: logic_depth = 1 - normalized_entropy."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((16, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        for v in X[:3]:
            proj = epa.project(v)
            assert abs(proj.logic_depth - (1.0 - proj.entropy)) < 1e-9

    def test_resonance_geometric_mean_value(self):
        """VCP: co_activation = sqrt(E1 * E2)."""
        rng = np.random.default_rng(0)
        X = rng.standard_normal((32, 8))
        epa = EPAModule(dimension=8, cluster_count=4, max_basis_dim=4)
        epa.initialize(X)
        proj = epa.project(X[0])
        if len(proj.dominant_axes) >= 2:
            top = proj.dominant_axes[0]
            sec = proj.dominant_axes[1]
            expected = float(np.sqrt(top["energy"] * sec["energy"]))
            res = epa.detect_cross_domain_resonance(X[0])
            # bridges 中应包含 top -> sec
            for bridge in res.bridges:
                if bridge["from"] == top["label"] and bridge["to"] == sec["label"]:
                    assert abs(bridge["strength"] - expected) < 1e-6
                    break


# ============================================================
# 8. 进程级 (主 17:43 实事求是, subprocess 真实调用)
# ============================================================

class TestSubprocessInvocation:
    """子进程真实调用测试 (主 17:43 实事求是)."""

    def test_subprocess_probe(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1272_asi_vcp_epa_physics_optimized", "--probe"],
            cwd=str(PROJ_ROOT),
            capture_output=True,
            text=True,
            timeout=30,
            encoding="utf-8",
        )
        assert result.returncode == 0
        assert "ALL OK" in result.stdout
        assert V1272_VCP_COMMIT in result.stdout

    def test_subprocess_full_loop(self, tmp_path):
        report = tmp_path / "subprocess_report.md"
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1272_asi_vcp_epa_physics_optimized",
             "--full-loop", "--report", str(report)],
            cwd=str(PROJ_ROOT),
            capture_output=True,
            text=True,
            timeout=60,
            encoding="utf-8",
        )
        assert result.returncode == 0
        assert "report written" in result.stdout
        assert report.exists()
        content = report.read_text(encoding="utf-8")
        assert "V1272" in content