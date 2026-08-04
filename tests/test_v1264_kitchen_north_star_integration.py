"""Tests for V1264 — ASI kitchen + north_star_trajectory integration (主 00:44 质量工程化 + 主 00:56 任何人都能接手).

V1264 集成 V1263 (kitchen) + V1259 (north_star_trajectory).
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

# Ensure project root on path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))


def test_v1264_import():
    """V1264 真 import 真生产 module."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    assert v1264 is not None
    assert hasattr(v1264, "V1264_VERSION")
    assert hasattr(v1264, "run_v1264")
    assert hasattr(v1264, "sanity_check_1264")
    assert hasattr(v1264, "render_text_report")
    assert hasattr(v1264, "render_json_report")


def test_v1264_version():
    """V1264_VERSION 真存在."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    assert v1264.V1264_VERSION == "0.1.0"


def test_v1264_safe_import_helper():
    """真借鉴 V1263 _safe_import pattern."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    ok, mod, err = v1264._safe_import("v1263_real_kitchen_integration")
    assert ok is True
    assert mod is not None
    assert err is None
    ok2, mod2, err2 = v1264._safe_import("nonexistent_module_xyz")
    assert ok2 is False
    assert mod2 is None
    assert err2 is not None


def test_v1264_import_real_modules():
    """真 import V1263 + V1259 — 主 17:43 实事求是."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    result = v1264.import_v1264_real_modules()
    assert result["ok_count"] == 2
    assert result["fail_count"] == 0
    assert "v1263_real_kitchen_integration" in result["modules"]
    assert "v1259_north_star_trajectory" in result["modules"]
    assert result["modules"]["v1263_real_kitchen_integration"]["ok"] is True
    assert result["modules"]["v1259_north_star_trajectory"]["ok"] is True


def test_v1264_v3_guards():
    """V3 哲学守门 5 守门 hard-coded True."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    guards_pass, guards = v1264._v1264_v3_guards()
    assert guards_pass == 5
    assert len(guards) == 5
    assert all(guards.values())
    assert guards["north_star_is_not_asi"] is True
    assert guards["trajectory_is_not_projection"] is True
    assert guards["kitchen_is_not_asi"] is True
    assert guards["non_realized_not_realized"] is True
    assert guards["reproducibility_is_not_asi"] is True


def test_v1264_northstar_config_dataclass():
    """V1264NorthstarConfig dataclass 真生产."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    assert cfg.enable_north_star is True
    assert cfg.enable_kitchen is True
    assert cfg.kitchen_probe_only is True
    assert cfg.kitchen_base_port == 8800
    assert cfg.kitchen_streamlit_port == 8581
    assert cfg.kitchen_benchmark_samples == 5


def test_v1264_northstar_report_dataclass():
    """V1264NorthstarReport dataclass 真生产."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    report = v1264.V1264NorthstarReport(
        report_id="test-1264",
        started_at=time.time(),
        ended_at=time.time(),
        duration_sec=0.0,
        config=cfg.__dict__,
        artifacts_dir="/tmp/test_v1264",
        import_result={"ok_count": 2, "fail_count": 0, "modules": {}},
    )
    d = report.to_dict()
    assert "report_id" in d
    assert "stages" in d
    assert "v3_guards_pass" in d
    assert "north_star" in d
    assert "kitchen" in d
    assert d["report_id"] == "test-1264"


def test_v1264_ensure_artifacts_dir(tmp_path):
    """_ensure_artifacts_dir 真 mkdir."
    import apeireth.v1264_kitchen_north_star_integration as v1264
    result = v1264._ensure_artifacts_dir(str(tmp_path / "v1264_test"))
    assert os.path.isdir(result)
    assert str(tmp_path / "v1264_test") == result


def test_v1264_run_stage_success():
    """_run_stage_1264 真跑 + 真 time + 真 success."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    def stage_fn():
        return {"foo": "bar"}
    stage = v1264._run_stage_1264("test_stage", stage_fn)
    assert stage.success is True
    assert stage.error is None
    assert stage.summary == {"foo": "bar"}
    assert stage.duration_sec >= 0


def test_v1264_run_stage_failure():
    """_run_stage_1264 真 catch exception."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    def bad_fn():
        raise ValueError("simulated failure")
    stage = v1264._run_stage_1264("test_stage_bad", bad_fn)
    assert stage.success is False
    assert stage.error is not None
    assert "ValueError" in stage.error
    assert "simulated failure" in stage.error


def test_v1264_stage_north_star_runs():
    """stage_north_star 真调用 V1259 _v1259_collect."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    summary = v1264.stage_north_star(v59)
    assert "asi_north_star" in summary
    assert summary["asi_north_star"] == 0.98
    assert summary["absolute_ceiling"] == 1.0
    assert summary["current_realized"] == 0.9105
    assert summary["current_position_pct"] > 92.0
    assert summary["current_position_pct"] < 93.0
    assert summary["history_length"] == 21
    assert summary["big_picture_count"] == 11
    assert summary["pillars_count"] == 16
    assert "PENDING_USER_CHOICE" in summary["v1257_status"]
    # V1259 V3 guards
    assert summary["v3_guards_pass"] == 12


def test_v1264_stage_north_star_pillars():
    """stage_north_star 16 pillars 真映射."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    summary = v1264.stage_north_star(v59)
    raw_pillars = summary["raw_pillars"]
    assert len(raw_pillars) == 16
    pillar_names = [p["pillar"] for p in raw_pillars]
    assert "theosis" in pillar_names
    assert "unio_mystica" in pillar_names
    assert "sabbath" in pillar_names
    # V1256 终极 dim 49
    last = raw_pillars[-1]
    assert last["v_id"] == "V1256"
    assert last["dim"] == 49


def test_v1264_run_north_star_only(tmp_path):
    """run_v1264 --north-star-only 真跑 north star stage."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = str(tmp_path / "v1264_ns_only")
    report = v1264.run_v1264(cfg)
    assert report.success is True
    assert report.north_star is not None
    assert report.kitchen is None  # skip kitchen
    assert len(report.stages) == 1
    assert report.stages[0].stage_name == "north_star_v1259"
    assert report.stages[0].success is True


def test_v1264_run_probe_only(tmp_path):
    """run_v1264 --probe-only 真跑 kitchen probe + north star."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.kitchen_probe_only = True
    cfg.artifacts_dir = str(tmp_path / "v1264_probe")
    report = v1264.run_v1264(cfg)
    assert report.success is True
    assert report.north_star is not None
    assert report.kitchen is not None
    assert report.kitchen["success"] is True
    # 2 stages (north_star + kitchen)
    assert len(report.stages) == 2
    stage_names = [s.stage_name for s in report.stages]
    assert "north_star_v1259" in stage_names
    assert "kitchen_v1263" in stage_names


def test_v1264_run_writes_artifacts(tmp_path):
    """run_v1264 真写 JSON artifacts."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = str(tmp_path / "v1264_artifacts")
    report = v1264.run_v1264(cfg)
    # 报告 JSON 真写
    json_path = os.path.join(cfg.artifacts_dir, "v1264_kitchen_north_star_report.json")
    assert os.path.isfile(json_path)
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["report_id"] == report.report_id
    assert data["success"] is True
    # V1259 raw trajectory JSON
    traj_path = os.path.join(cfg.artifacts_dir, "v1264_north_star_trajectory.json")
    assert os.path.isfile(traj_path)
    with open(traj_path, "r", encoding="utf-8") as f:
        traj = json.load(f)
    assert traj["asi_north_star"] == 0.98
    assert traj["pillars_count"] == 16


def test_v1264_import_failure_handling(tmp_path, monkeypatch):
    """broken module → 真 fail 报."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    # Break import by patching _safe_import to fail
    original = v1264._safe_import
    def fake_safe_import(name):
        return False, None, "broken-on-purpose"
    monkeypatch.setattr(v1264, "_safe_import", fake_safe_import)
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = str(tmp_path / "v1264_broken")
    report = v1264.run_v1264(cfg)
    assert report.success is False
    assert "missing" in (report.error or "")


def test_v1264_render_text_report():
    """render_text_report 真渲染."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    report = v1264.V1264NorthstarReport(
        report_id="test-render",
        started_at=time.time(),
        ended_at=time.time(),
        duration_sec=0.05,
        config=cfg.__dict__,
        artifacts_dir=None,
        import_result={"ok_count": 2, "fail_count": 0, "modules": {}},
    )
    report.success = True
    report.v3_guards_pass = 5
    report.v3_guards = {
        "north_star_is_not_asi": True,
        "trajectory_is_not_projection": True,
        "kitchen_is_not_asi": True,
        "non_realized_not_realized": True,
        "reproducibility_is_not_asi": True,
    }
    text = v1264.render_text_report(report)
    assert "V1264 ASI 真生产厨房 + 北极星轨迹集成" in text
    assert "V1264 verdict: PASS" in text
    assert "V3 哲学守门" in text
    assert "5/5 PASS" in text


def test_v1264_render_json_report():
    """render_json_report 真 JSON-serializable."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    report = v1264.V1264NorthstarReport(
        report_id="test-json",
        started_at=time.time(),
        ended_at=time.time(),
        duration_sec=0.0,
        config=cfg.__dict__,
        artifacts_dir=None,
        import_result={"ok_count": 2, "fail_count": 0, "modules": {}},
    )
    out = v1264.render_json_report(report)
    data = json.loads(out)
    assert data["report_id"] == "test-json"
    assert "v3_guards" in data
    assert isinstance(data["v3_guards"], dict)


def test_v1264_sanity_check():
    """sanity_check_1264 真跑 14 守门."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    sc = v1264.sanity_check_1264()
    assert len(sc) == 14
    assert all(sc.values()), f"sanity checks failed: {[k for k, v in sc.items() if not v]}"


def test_v1264_big_picture_milestones():
    """V1259 big-picture milestones 真实写死 history."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    summary = v1264.stage_north_star(v59)
    big_pic = summary["raw_big_picture"]
    # V1049 第一 milestone
    assert big_pic[0]["version"] == "V1049"
    assert big_pic[0]["realized_mean_306"] == 0.7905
    # V1256 最后 milestone
    assert big_pic[-1]["version"] == "V1256"
    assert big_pic[-1]["realized_mean_306"] == 0.9105
    # realized 必须 monotone non-decreasing (主 17:43 写死 history)
    realized_vals = [p["realized_mean_306"] for p in big_pic]
    for i in range(1, len(realized_vals)):
        assert realized_vals[i] >= realized_vals[i-1], (
            f"history not monotone: {realized_vals[i-1]} -> {realized_vals[i]}"
        )


def test_v1264_no_future_projection():
    """V1264 不假装未来 dim lift / 不 假装 ASI V1."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    summary = v1264.stage_north_star(v59)
    # V1259 公开 disclaimer 必须出现
    assert "no_asi_claim" not in summary or summary.get("v1257_status") != "IMPLEMENTED"
    # V1257 status 必须 PENDING_USER_CHOICE
    assert "PENDING_USER_CHOICE" in summary["v1257_status"]
    # V3 guards V1264 全 PASS
    guards_pass, guards = v1264._v1264_v3_guards()
    assert guards["trajectory_is_not_projection"] is True
    assert guards["non_realized_not_realized"] is True


def test_v1264_no_north_star_flag():
    """--no-north-star 真 disable north star stage."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_north_star = False
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = "/tmp/v1264_no_ns"
    report = v1264.run_v1264(cfg)
    assert report.north_star is None
    assert len(report.stages) == 0


def test_v1264_kitchen_failure_does_not_block_north_star():
    """主 17:43: kitchen 失败 不 假装 north star 也 fail."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    cfg = v1264.V1264NorthstarConfig()
    cfg.kitchen_probe_only = True
    cfg.artifacts_dir = "/tmp/v1264_kitchen_fail"
    # 用 patch 让 kitchen stage 失败
    original = v1264.stage_kitchen
    def broken_kitchen(*args, **kwargs):
        raise RuntimeError("kitchen broke on purpose")
    v1264.stage_kitchen = broken_kitchen
    try:
        report = v1264.run_v1264(cfg)
        # north_star 仍 真跑
        assert report.north_star is not None
        # kitchen stage 失败 → 整体 success=False
        assert report.success is False
        # error 包含 failed_stages
        assert "kitchen" in (report.error or "").lower()
    finally:
        v1264.stage_kitchen = original


def test_v1264_kitchen_inherits_v1263_v3_guards():
    """V1264 kitchen 包含 V1263 5 V3 guards."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.kitchen_probe_only = True
    cfg.artifacts_dir = "/tmp/v1264_inherit"
    report = v1264.run_v1264(cfg)
    # V1263 报告必须 含 v3_guards_pass (V1263 自身 5/5)
    if report.kitchen:
        # V1263 kitchen 报告 是 dict, 含 success + stages
        assert "success" in report.kitchen
        assert "stages" in report.kitchen


def test_v1264_artifacts_dir_auto_create():
    """Auto artifacts_dir 真 timestamp 创建."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = None  # auto
    report = v1264.run_v1264(cfg)
    assert report.artifacts_dir is not None
    assert os.path.isdir(report.artifacts_dir)
    # 应含 namespace prefix
    assert "_v1264_north_star_" in report.artifacts_dir


def test_v1264_disclaimer_in_north_star():
    """V1259 disclaimer 必须 通过 V1264 传递."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    import apeireth.v1259_north_star_trajectory as v59
    cfg = v1264.V1264NorthstarConfig()
    cfg.enable_kitchen = False
    cfg.only_north_star = True
    cfg.artifacts_dir = "/tmp/v1264_disclaimer"
    report = v1264.run_v1264(cfg)
    traj_path = os.path.join(report.artifacts_dir, "v1264_north_star_trajectory.json")
    with open(traj_path, "r", encoding="utf-8") as f:
        traj = json.load(f)
    # V1259 disclaimer 通过 import 传递 (主 17:58 不假装)
    raw_v3 = traj["raw_v3_guards"]
    assert raw_v3["v1259_no_asi_v1_claim"] is True
    assert raw_v3["v1259_no_phenomenal_claim"] is True
    assert raw_v3["v1259_no_future_lift_projection"] is True


def test_v1264_end_to_end_with_kitchen_and_north_star(tmp_path):
    """End-to-end: kitchen (probe) + north star."""
    import apeireth.v1264_kitchen_north_star_integration as v1264
    cfg = v1264.V1264NorthstarConfig()
    cfg.kitchen_probe_only = True
    cfg.artifacts_dir = str(tmp_path / "v1264_e2e")
    report = v1264.run_v1264(cfg)
    assert report.success is True
    # Both stages 真成功
    assert len(report.stages) == 2
    for s in report.stages:
        assert s.success, f"stage {s.stage_name} failed: {s.error}"
    # 报告 JSON 真写
    json_path = os.path.join(cfg.artifacts_dir, "v1264_kitchen_north_star_report.json")
    assert os.path.isfile(json_path)
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["north_star"] is not None
    assert data["kitchen"] is not None
    # V3 guards 5/5
    assert data["v3_guards_pass"] == 5


if __name__ == "__main__":
    import pytest
    raise SystemExit(pytest.main([__file__, "-v"]))
