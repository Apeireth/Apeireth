"""R6 self_mod_safety contract smoke tests.

占位契约壳: Protocol + 5 方法 + 3 dataclass + 守门 + 命名空间 + 与 reproduction 不重复.
**不**验证真实自改引擎 (R7+ 范围).
"""
from __future__ import annotations

from dataclasses import is_dataclass

import pytest

from apeireth import self_mod_safety as sms
from apeireth.self_mod_safety import (
    Checkpoint, DryRunResult, MODULE_NAME, PHILOSOPHY_NOTES, PROTOCOL_VERSION,
    SafetyVerification, SelfModSafetyProtocol, guard_self_mod_safety,
)


def test_protocol_exists() -> None:
    assert hasattr(sms, "SelfModSafetyProtocol")
    assert isinstance(SelfModSafetyProtocol, type)


def test_protocol_methods() -> None:
    expected = {"snapshot", "checkpoint", "rollback", "verify", "dry_run"}
    assert expected.issubset(set(dir(SelfModSafetyProtocol)))
    for name in expected:
        assert getattr(SelfModSafetyProtocol, name, None) is not None


def test_dataclasses() -> None:
    assert is_dataclass(Checkpoint) and is_dataclass(SafetyVerification) and is_dataclass(DryRunResult)
    cp = Checkpoint(label="phase-1", checkpoint_id="cp_001", ts=1700000000.0)
    sv = SafetyVerification(mutation_id="m_001", verified=True, risk_score=0.42, rationale="ok")
    dr = DryRunResult(mutation_id="m_002", expected_impact={"files": 2}, side_effects=["log"])
    assert cp.scope == "module" and sv.verified and dr.side_effects == ["log"]
    # 契约不变量
    with pytest.raises(ValueError):
        Checkpoint(label="", checkpoint_id="cp", ts=0.0)
    with pytest.raises(ValueError):
        SafetyVerification(mutation_id="m", verified=True, risk_score=1.5)
    with pytest.raises(TypeError):
        SafetyVerification(mutation_id="m", verified="yes", risk_score=0.1)


def test_no_real_implementation_yet() -> None:
    for name in ("snapshot", "checkpoint", "rollback", "verify", "dry_run"):
        attr = getattr(sms, name, None)
        if attr is not None and not isinstance(attr, type):
            pytest.fail(f"{name} 不应是模块级可调用, 但实际有: {attr!r}")
    assert sms.PROTOCOL_VERSION == "0.1.0-contract"


def test_philosophy_guard_imports() -> None:
    guard = guard_self_mod_safety()
    assert guard["module"] == MODULE_NAME
    assert guard["guard_status"] in {"PASS", "WARN"}
    assert guard["deviation_count"] == 0, f"守门出现偏差: {guard}"
    for key in ("not_undo", "not_proof", "not_safe"):
        assert key in PHILOSOPHY_NOTES and PHILOSOPHY_NOTES[key]


def test_module_in_apeireth() -> None:
    import apeireth
    assert apeireth.self_mod_safety is sms
    assert sms.MODULE_NAME == "self_mod_safety"
    assert sms.PROTOCOL_VERSION == "0.1.0-contract"


def test_distinct_from_reproduction() -> None:
    """与 self_reproduction Protocol 不重复 (variant vs replica), 不依赖其模块."""
    from apeireth import self_reproduction as sr
    from apeireth.self_reproduction import SelfReproductionProtocol
    assert SelfModSafetyProtocol is not SelfReproductionProtocol
    sms_m, sr_m = {"snapshot", "checkpoint", "rollback", "verify", "dry_run"}, {
        "snapshot", "verify", "restore", "reproduce", "reproduction_id"}
    assert sms_m != sr_m and "dry_run" in sms_m and "reproduce" in sr_m
    src = open(sms.__file__, "r", encoding="utf-8").read()
    for banned in ("import self_reproduction", "from .self_reproduction", "from apeireth.self_reproduction",
                   "import formal_verify", "from .formal_verify", "from apeireth.formal_verify"):
        assert banned not in src, f"self_mod_safety 不应包含禁用导入: {banned}"
