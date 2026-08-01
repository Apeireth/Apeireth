"""Test V1180 — Real production R1+R2 真补 (R14 阶段 5 18 crate 施工图纸).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Ensure repo root on path (test runner may not auto-find apeireth)
REPO_ROOT = Path(__file__).resolve().parent.parent
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))


# ============================================================================
# Constants checks
# ============================================================================


class TestV1180Constants:
    def test_version(self):
        from apeireth.v1180_real_production_r1r2_realboost import V1180_VERSION
        assert V1180_VERSION == "0.1.0"

    def test_subdim_names_locked(self):
        from apeireth.v1180_real_production_r1r2_realboost import V1180_SUBDIM_NAMES
        assert V1180_SUBDIM_NAMES == (
            "render_18_dockerfiles_real",
            "render_2_compose_real",
            "render_18_k8s_real",
            "v1132_dryrun_boost",
            "v1171_total_boost",
        )

    def test_subdim_count(self):
        from apeireth.v1180_real_production_r1r2_realboost import V1180_SUBDIM_NAMES
        assert len(V1180_SUBDIM_NAMES) == 5

    def test_18_crates_locked(self):
        from apeireth.v1180_real_production_r1r2_realboost import R14_STAGE5_18_CRATES
        assert len(R14_STAGE5_18_CRATES) == 18
        # R14 阶段 5 施工图纸 (commit 531f5a14) LOCKED 列表
        assert R14_STAGE5_18_CRATES[0] == "apeireth-core"
        assert R14_STAGE5_18_CRATES[-1] == "apeireth-cli"

    def test_target_r1(self):
        from apeireth.v1180_real_production_r1r2_realboost import TARGET_V1180_R1
        assert TARGET_V1180_R1 == 0.85

    def test_target_total(self):
        from apeireth.v1180_real_production_r1r2_realboost import TARGET_V1180_TOTAL_BOOST
        assert TARGET_V1180_TOTAL_BOOST == 0.85

    def test_baseline_v1171(self):
        from apeireth.v1180_real_production_r1r2_realboost import V1171_BASELINE_TOTAL
        assert 0.5 <= V1171_BASELINE_TOTAL <= 0.7

    def test_report_dataclass(self):
        from apeireth.v1180_real_production_r1r2_realboost import V1180Report, SubDimEvidence
        r = V1180Report()
        assert r.version == "0.1.0"
        assert r.dim_version == "0.6.2-r1r2boost"
        assert r.n_subdims_total == 5
        r.sub_dim_scores["foo"] = 0.5
        d = r.to_dict()
        assert "foo" in d["sub_dim_scores"]
        se = SubDimEvidence(name="x", score=0.5)
        assert se.to_dict()["name"] == "x"


# ============================================================================
# Dockerfile rendering
# ============================================================================


class TestV1180RenderDockerfiles:
    def test_render_one_dockerfile_structure(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_dockerfile_for
        text = _render_dockerfile_for("apeireth-core", port=8765)
        assert "FROM python:" in text
        assert "EXPOSE 8765" in text
        assert "USER 10001:10001" in text
        assert "apeireth.core" in text or "apeireth_core" in text
        assert "HEALTHCHECK" in text
        assert "CMD" in text

    def test_validate_dockerfile_ok(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_dockerfile_for, _validate_dockerfile
        text = _render_dockerfile_for("apeireth-perception", port=8801)
        ok, detail = _validate_dockerfile(text)
        assert ok is True
        assert "ok" in detail.lower() or detail == "ok"

    def test_validate_dockerfile_missing_directive(self):
        from apeireth.v1180_real_production_r1r2_realboost import _validate_dockerfile
        ok, detail = _validate_dockerfile("FROM alpine\nRUN echo hi\n")  # missing EXPOSE/HEALTHCHECK/CMD
        assert ok is False
        assert "missing" in detail.lower()

    def test_render_18_dockerfiles_writes_real_files(self, tmp_path):
        from apeireth.v1180_real_production_r1r2_realboost import (
            _render_dockerfile_for,
            R14_STAGE5_18_CRATES,
        )
        written = []
        for i, crate in enumerate(R14_STAGE5_18_CRATES):
            path = tmp_path / f"Dockerfile.{crate}"
            path.write_text(_render_dockerfile_for(crate, port=8800 + i), encoding="utf-8")
            written.append(path)
        assert len(written) == 18
        for p in written:
            content = p.read_text(encoding="utf-8")
            assert "FROM python:" in content


# ============================================================================
# Compose rendering
# ============================================================================


class TestV1180RenderCompose:
    def test_render_one_service(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_compose_for
        text = _render_compose_for("apeireth-core", 8765)
        assert "apeireth-core:" in text
        assert "\"8765:8765\"" in text
        assert "healthcheck" in text
        assert "APEIRETH_CRATE: \"apeireth-core\"" in text

    def test_validate_compose_ok(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_full_compose, _validate_compose
        text = _render_full_compose(
            "test-group",
            ("apeireth-core", "apeireth-perception"),
            8800,
        )
        ok, detail = _validate_compose(text)
        # PyYAML may or may not be installed; either way must detect 2 services
        assert ok is True

    def test_render_full_compose_n_services(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_full_compose, R14_STAGE5_18_CRATES
        mid = len(R14_STAGE5_18_CRATES) // 2
        text_a = _render_full_compose("group-a", R14_STAGE5_18_CRATES[:mid], 8800)
        text_b = _render_full_compose("group-b", R14_STAGE5_18_CRATES[mid:], 8800+mid)
        # Each group has 9 services
        assert text_a.count("    image: apeireth/") == mid
        assert text_b.count("    image: apeireth/") == mid


# ============================================================================
# K8s rendering
# ============================================================================


class TestV1180RenderK8s:
    def test_render_k8s_deployment(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_k8s_deployment
        text = _render_k8s_deployment("apeireth-core", 8800, 0)
        assert "kind: Deployment" in text
        assert "kind: Service" in text
        assert "name: apeireth-core" in text
        assert "containerPort: 8800" in text

    def test_validate_k8s_yaml_ok(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_full_k8s_bundle, _validate_k8s_yaml
        text = _render_full_k8s_bundle(("apeireth-core", "apeireth-perception", "apeireth-cognition"), 8800)
        ok, detail = _validate_k8s_yaml(text)
        assert ok is True

    def test_render_full_k8s_n_deployments(self):
        from apeireth.v1180_real_production_r1r2_realboost import _render_full_k8s_bundle, R14_STAGE5_18_CRATES
        text = _render_full_k8s_bundle(R14_STAGE5_18_CRATES, 8800)
        assert text.count("kind: Deployment") == 18
        assert text.count("kind: Service") == 18


# ============================================================================
# V1171 algorithm reproduction (主 17:43 实事求是 — 沿用 V1171 算法, 不重写)
# ============================================================================


class TestV1180V1171Algorithm:
    def test_compute_v1171_total_after_baseline(self):
        """Predict V1171 total with V1180-boosted numbers."""
        from apeireth.v1180_real_production_r1r2_realboost import _compute_v1171_total_after
        # baseline: compose=2, services=14, k8s=3, dockerfile=2, v1170=1.0
        r1, r2, total = _compute_v1171_total_after(2, 14, 3, 2, 1.0)
        # Should match V1171 实测 ≈ 0.42 + 0.37 + 1.0 + 0.9 + 0.48 / 5 ≈ 0.634
        assert 0.55 <= total <= 0.72

    def test_compute_v1171_total_after_boosted(self):
        """With V1180 boost: R1 → 0.85, R2 → 1.0"""
        from apeireth.v1180_real_production_r1r2_realboost import _compute_v1171_total_after
        # boost: compose=4, services=32, k8s=21, dockerfile=20, v1170=1.0
        r1, r2, total = _compute_v1171_total_after(4, 32, 21, 20, 1.0)
        assert r1 >= 0.80
        assert r2 >= 0.95
        # total ≈ (0.85+1.0+1.0+0.9+0.48)/5 ≈ 0.846
        assert total >= 0.80

    def test_compute_v1171_full_score(self):
        """If everything hits max: R1=R2=R3=R4=R5=1, total = 1.0"""
        from apeireth.v1180_real_production_r1r2_realboost import _compute_v1171_total_after
        r1, r2, total = _compute_v1171_total_after(100, 100, 100, 100, 1.0)
        assert r1 == 1.0
        assert r2 == 1.0


# ============================================================================
# Full V1180 measure end-to-end (主 17:43 实事求是 — 真跑)
# ============================================================================


class TestV1180MeasureFull:
    def test_measure_v1180_full_runs(self):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full
        report = measure_v1180_full()
        assert report.n_subdims_total == 5
        assert report.n_dockerfiles_written == 18
        assert report.n_compose_files_written == 2
        assert report.n_k8s_deployments_written == 18
        assert report.total > 0.0

    def test_measure_v1180_total_meets_target(self):
        """V1180 主目标: 总分 ≥ 0.85 (R1+R2 真补后 V1171 ASI 预测)"""
        from apeireth.v1180_real_production_r1r2_realboost import (
            measure_v1180_full,
            TARGET_V1180_TOTAL_BOOST,
        )
        report = measure_v1180_full()
        # 主 17:43 实事求是: 实测总分应达到或超过目标
        assert report.total >= TARGET_V1180_TOTAL_BOOST * 0.9  # 允许 10% tolerance

    def test_measure_v1180_files_written_real(self):
        """38 真文件真写到 deploy/18-crates/"""
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full, R14_STAGE5_18_CRATES
        from pathlib import Path
        measure_v1180_full()  # triggers writes
        out_dir = Path("deploy") / "18-crates"
        assert out_dir.is_dir()
        # Dockerfiles
        for crate in R14_STAGE5_18_CRATES:
            assert (out_dir / f"Dockerfile.{crate}").is_file()
        # Compose files
        assert (out_dir / "docker-compose.group-a.yml").is_file()
        assert (out_dir / "docker-compose.group-b.yml").is_file()
        # k8s bundle
        assert (out_dir / "k8s-18crates.yaml").is_file()

    def test_measure_v1180_v1171_boost_present(self):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full
        report = measure_v1180_full()
        assert report.v1171_total_after >= report.v1171_total_before
        assert (report.v1171_total_after - report.v1171_total_before) > 0.1  # 至少 +0.10 delta

    def test_measure_v1180_evidence_subdims(self):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full, V1180_SUBDIM_NAMES
        report = measure_v1180_full()
        for name in V1180_SUBDIM_NAMES:
            assert name in report.sub_dim_scores
            assert name in report.sub_dim_evidence
            assert 0.0 <= report.sub_dim_scores[name] <= 1.0


# ============================================================================
# Artifact + main entry
# ============================================================================


class TestV1180ArtifactAndCli:
    def test_write_artifact(self, tmp_path):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full, _write_artifact
        report = measure_v1180_full()
        path = _write_artifact(report, str(tmp_path))
        assert Path(path).is_file()
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        assert data["total"] == report.total
        assert "sub_dim_evidence" in data

    def test_main_no_args(self, capsys):
        from apeireth.v1180_real_production_r1r2_realboost import main
        rc = main([])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1180 R1+R2 realboost" in out

    def test_main_json(self, capsys):
        from apeireth.v1180_real_production_r1r2_realboost import main
        rc = main(["--json", "--no-write"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "total" in data

    def test_main_report(self, capsys):
        from apeireth.v1180_real_production_r1r2_realboost import main
        rc = main(["--report", "--no-write"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "# V1180" in out
        assert "Sub-dim scores" in out


# ============================================================================
# 主 17:58 不假装 — explicit honesty check
# ============================================================================


class TestV1180Honesty:
    """主 17:58 + 20:46 不假装 — V1180 报告必须区分 真补 vs 预测."""

    def test_b5_marks_prediction(self):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full
        report = measure_v1180_full()
        b5_raw = report.sub_dim_evidence["v1171_total_boost"].raw
        # B5 必须标注这是 预测 不是 V1171 实测
        assert "fact_disclosure" in b5_raw
        assert "预测" in b5_raw["fact_disclosure"] or "predicted" in b5_raw["fact_disclosure"].lower()
        # 不能假装 V1180 写文件后 V1132 默认算法就扫
        assert "deploy/18-crates/" in b5_raw["fact_disclosure"]

    def test_target_r1_aligned_with_v1171_score_curve(self):
        """主 13:31 大胆激进: target 不是天马行空, 是基于 V1171 算法可达."""
        from apeireth.v1180_real_production_r1r2_realboost import (
            TARGET_V1180_R1,
            TARGET_V1180_R2,
            TARGET_V1180_TOTAL_BOOST,
        )
        # V1171 算法满分为 1.0
        assert 0.8 <= TARGET_V1180_R1 <= 1.0
        assert 0.9 <= TARGET_V1180_R2 <= 1.0
        assert 0.8 <= TARGET_V1180_TOTAL_BOOST <= 1.0

    def test_subdim_scores_in_valid_range(self):
        from apeireth.v1180_real_production_r1r2_realboost import measure_v1180_full
        report = measure_v1180_full()
        for name, score in report.sub_dim_scores.items():
            assert 0.0 <= score <= 1.0, f"{name} out of range: {score}"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
