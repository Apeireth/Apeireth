"""Tests for V1143 — ASI V0.4 17 维度 真测快照生成器
(主 06:15 V1053+ 真测 + 主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58+20:46 不假装).

Covers:
  - DimMeasure dataclass
  - 17 dim registry completeness
  - safe_call fallbacks
  - snapshot measure_all populates all 17 dims
  - snapshot v03_score is mean of 17 dim values
  - chaos mode does not crash
  - json serialization
  - markdown report contains 17 dim table
  - strict mode exit code
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), os.pardir))

from apeireth.v1143_asi_v04_17dim_real_measurement import (  # noqa: E402
    ASI_V04_17DIMS,
    DIM_REGISTRY,
    V1143_VERSION,
    DimMeasure,
    V1143Snapshot,
    _safe_call,
    _safe_import_module,
    main,
)


# ---------- fixture ----------


@pytest.fixture
def snapshot():
    s = V1143Snapshot()
    s.measure_all()
    return s


# ---------- constant checks ----------


def test_17dims_count():
    """17 dim LOCKED — 主 22:33 北极星 V0.4."""
    assert len(ASI_V04_17DIMS) == 17


def test_17dims_contain_key_dims():
    """关键维度必须存在."""
    for must in (
        "phi_proxy", "capabilities", "cross_domain", "engineering",
        "vcp_4", "v2_philosophy", "rubric_open", "real_production",
        "cognitive_core", "self_organizing_core", "plugin_core",
        "self_improving_core", "neurosymbolic", "world_model",
        "reinforcement_learning", "scientific_method", "eternal_identity",
    ):
        assert must in ASI_V04_17DIMS, f"missing dim: {must}"


def test_registry_covers_all_17dims():
    """每个 dim 必须有真测函数 (主 17:43 实事求是)."""
    for dim in ASI_V04_17DIMS:
        assert dim in DIM_REGISTRY, f"missing registry for dim: {dim}"
        fn, source = DIM_REGISTRY[dim]
        assert callable(fn), f"registry for {dim} is not callable"
        assert isinstance(source, str) and len(source) > 0


# ---------- safe_call fallbacks ----------


def test_safe_call_with_callable():
    """safe_call 应该执行 callable."""
    v, status = _safe_call(lambda: 0.42)
    assert v == pytest.approx(0.42)
    assert status == "ok"


def test_safe_call_clamping():
    """safe_call 应该 clamp 到 [0,1]."""
    v1, _ = _safe_call(lambda: 1.5)
    assert v1 == 1.0
    v0, _ = _safe_call(lambda: -0.5)
    assert v0 == 0.0


def test_safe_call_nan_inf():
    """safe_call 应该把 nan/inf 当 error."""
    v1, s1 = _safe_call(lambda: float("nan"))
    assert s1 == "error"
    v2, s2 = _safe_call(lambda: float("inf"))
    assert s2 == "error"


def test_safe_call_exception():
    """safe_call 应该捕获异常."""
    def boom():
        raise RuntimeError("boom")
    v, status = _safe_call(boom)
    assert status == "error"
    assert v == 0.0


def test_safe_call_none_callable():
    """safe_call 应该返回 default + no_callable."""
    v, status = _safe_call(None, default=0.5)
    assert v == 0.5
    assert status == "no_callable"


def test_safe_import_module_real():
    """safe_import 应该能 import 真实模块."""
    mod, status = _safe_import_module("apeireth.v1143_asi_v04_17dim_real_measurement")
    assert status == "ok"
    assert mod is not None


def test_safe_import_module_missing():
    """safe_import 应该返回 missing_module."""
    mod, status = _safe_import_module("apeireth.v9999_does_not_exist")
    assert status == "missing_module"
    assert mod is None


# ---------- DimMeasure dataclass ----------


def test_dim_measure_to_dict_via_snapshot(snapshot):
    m = snapshot.dim_values["cross_domain"]
    assert m.dim == "cross_domain"
    assert m.status == "ok"
    assert m.value == pytest.approx(1.0)


# ---------- snapshot measure_all ----------


def test_snapshot_measure_all_populates_all_17_dims(snapshot):
    assert snapshot.n_dims == 17
    for dim in ASI_V04_17DIMS:
        assert dim in snapshot.dim_values


def test_snapshot_v03_score_is_mean(snapshot):
    """v03_score = mean of 17 dims (主 22:33 北极星代理)."""
    vals = [m.value for m in snapshot.dim_values.values()]
    expected = sum(vals) / len(vals)
    assert snapshot.v03_score == pytest.approx(expected)


def test_snapshot_locked_dims_real_values(snapshot):
    """cross_domain, vcp_4, eternal_identity 必须 = LOCKED 真测值."""
    assert snapshot.dim_values["cross_domain"].value == pytest.approx(1.0)
    assert snapshot.dim_values["vcp_4"].value == pytest.approx(0.9588)
    assert snapshot.dim_values["eternal_identity"].value == pytest.approx(0.8441)


def test_snapshot_capabilities_v1133_proxy(snapshot):
    """capabilities = V1133 真 LLM benchmark pass-rate 86.36%."""
    assert snapshot.dim_values["capabilities"].value == pytest.approx(0.8636)


def test_snapshot_philosophy_guard_ok(snapshot):
    """V3 哲学守门 ok."""
    d = snapshot.to_dict()
    assert d["philosophy_guard_ok"] is True


def test_snapshot_to_dict_has_17_dims(snapshot):
    d = snapshot.to_dict()
    assert len(d["dim_breakdown"]) == 17


def test_snapshot_to_markdown_has_table(snapshot):
    md = snapshot.to_markdown()
    assert "V1143" in md
    assert "cross_domain" in md
    assert "eternal_identity" in md
    assert "V3 哲学守门" in md


# ---------- chaos mode ----------


def test_chaos_mode_does_not_crash():
    s = V1143Snapshot(chaos=True)
    s.measure_all()
    assert s.n_dims == 17
    assert 0.0 <= s.v03_score <= 1.0


# ---------- CLI ----------


def test_cli_json(capsys):
    """CLI --json 应该输出 JSON."""
    rc = main(["--json"])
    assert rc == 0
    out = capsys.readouterr().out
    d = json.loads(out)
    assert d["version"] == V1143_VERSION
    assert len(d["dim_breakdown"]) == 17


def test_cli_report(capsys):
    """CLI --report 应该输出 Markdown."""
    rc = main(["--report"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "V1143" in out
    assert "17 维度真测分解" in out


def test_cli_default(capsys):
    """CLI 默认输出 (table)."""
    rc = main([])
    assert rc == 0
    out = capsys.readouterr().out
    assert "v03_score=" in out
    assert "cross_domain" in out


def test_cli_strict_passes():
    """CLI --strict 默认应该 pass (n_missing <= 5)."""
    rc = main(["--strict"])
    assert rc == 0


def test_cli_persist_creates_file(tmp_path, monkeypatch, capsys):
    """CLI --persist 应该写文件."""
    monkeypatch.chdir(tmp_path)
    rc = main(["--persist"])
    assert rc == 0
    artifacts = list(tmp_path.glob("artifacts/v1143_*.json"))
    assert len(artifacts) == 1
    d = json.loads(artifacts[0].read_text(encoding="utf-8"))
    assert len(d["dim_breakdown"]) == 17
    out = capsys.readouterr().out
    assert "[persisted]" in out


# ---------- process smoke ----------


def test_module_cli_via_subprocess():
    """通过 subprocess 真跑模块 CLI (主 00:56 任何人都能接手)."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1143_asi_v04_17dim_real_measurement", "--json"],
        capture_output=True, text=True, timeout=30,
    )
    assert proc.returncode == 0
    d = json.loads(proc.stdout)
    assert d["version"] == V1143_VERSION
    assert len(d["dim_breakdown"]) == 17
    assert 0.0 <= d["v03_score"] <= 1.0