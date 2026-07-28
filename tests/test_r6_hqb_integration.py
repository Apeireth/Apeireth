"""R6 HQB integration smoke contract; no target runner mutation."""
from pathlib import Path

from apeireth.hqb_integration import HQBIntegration, guard_hqb_integration

ROOT = Path(__file__).resolve().parents[1]


def _adapter(tmp_path):
    return HQBIntegration(str(tmp_path / "hqb.db"), tmp_path / "v1086")


def test_v1074_hqb_pass(tmp_path):
    a = _adapter(tmp_path)
    out = a.record_v1074(0.85)
    assert out["verdict"] == "accept" and out["row"]["task_id"] == "v1074"
    a.close()


def test_v1082_hqb_pass(tmp_path):
    a = _adapter(tmp_path)
    out = a.record_v1082(0.96)
    assert out["verdict"] == "accept" and out["row"]["task_id"] == "v1082"
    a.close()


def test_v1083_hqb_pass(tmp_path):
    a = _adapter(tmp_path)
    out = a.record_v1083(0.55)
    assert out["verdict"] == "review" and out["row"]["task_id"] == "v1083"
    a.close()


def test_no_mutation_to_v1074(tmp_path):
    path = ROOT / "apeireth/v1074_asi_production_runner.py"
    before = path.read_bytes()
    a = _adapter(tmp_path); a.record_v1074(0.8); a.close()
    assert path.read_bytes() == before


def test_no_mutation_to_v1082(tmp_path):
    path = ROOT / "apeireth/v1082_asi_codebase_audit.py"
    before = path.read_bytes()
    a = _adapter(tmp_path); a.record_v1082(0.8); a.close()
    assert path.read_bytes() == before


def test_no_mutation_to_v1083(tmp_path):
    path = ROOT / "apeireth/v1083_asi_decision_router.py"
    before = path.read_bytes()
    a = _adapter(tmp_path); a.record_v1083(0.8); a.close()
    assert path.read_bytes() == before


def test_philosophy_guard_passes():
    result = guard_hqb_integration()
    assert result["guard_passed"] and result["guard_status"] == "PASS"
    assert result["deviation_count"] == 0


def test_adapters_distinct():
    from apeireth.v1085_hqb_core import HonestDecisionModule
    from apeireth.v1086_hqb_persistence import HQBPersistence
    assert HQBIntegration is not HonestDecisionModule
    assert HQBIntegration is not HQBPersistence
    assert not hasattr(HQBIntegration, "evaluate")
