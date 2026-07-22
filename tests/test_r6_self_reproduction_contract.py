"""R6 self_reproduction contract smoke tests.

占位契约壳验证: Protocol 存在 + 5 方法签名 + dataclass 可构造 +
守门 PASS + 模块在命名空间. **不**验证真实繁殖 (R7+ 范围).
"""
from __future__ import annotations

from dataclasses import is_dataclass

import pytest

from apeireth import self_reproduction as sr
from apeireth.self_reproduction import (
    MODULE_NAME,
    PHILOSOPHY_NOTES,
    PROTOCOL_VERSION,
    ReproductionResult,
    ReproductionSpec,
    SelfReproductionProtocol,
    guard_self_reproduction,
)


def test_protocol_exists() -> None:
    """SelfReproductionProtocol 必须存在于 apeireth.self_reproduction."""
    assert hasattr(sr, "SelfReproductionProtocol")
    assert isinstance(SelfReproductionProtocol, type)


def test_protocol_methods() -> None:
    """Protocol 必须声明 5 个方法: snapshot/verify/restore/reproduce/reproduction_id."""
    expected = {"snapshot", "verify", "restore", "reproduce", "reproduction_id"}
    assert expected.issubset(set(dir(SelfReproductionProtocol)))
    for name in expected:
        attr = getattr(SelfReproductionProtocol, name, None)
        assert attr is not None, f"缺少方法: {name}"


def test_dataclasses() -> None:
    """ReproductionSpec/Result 必须是 dataclass 且可构造."""
    assert is_dataclass(ReproductionSpec)
    assert is_dataclass(ReproductionResult)

    spec = ReproductionSpec(seed=b"seed-x", target_path="/tmp/repro", expected_modules=3)
    assert spec.seed == b"seed-x"
    assert spec.target_path == "/tmp/repro"
    assert spec.expected_modules == 3

    result = ReproductionResult(success=True, reproduction_id="rid_abc")
    assert result.success is True
    assert result.reproduction_id == "rid_abc"
    assert result.diff_summary == ""
    assert result.manifest_delta == []

    # 契约不变量: target_path 非空 / expected_modules > 0
    with pytest.raises(ValueError):
        ReproductionSpec(seed=b"x", target_path="", expected_modules=1)
    with pytest.raises(ValueError):
        ReproductionSpec(seed=b"x", target_path="/tmp", expected_modules=0)


def test_no_real_implementation_yet() -> None:
    """契约壳不写真繁殖: 模块不应暴露 snapshot/verify/restore 的可调用实现.

    Protocol 成员仍是 placeholder (...), 普通类 import 后不会自动绑定这些名字.
    """
    for name in ("snapshot", "verify", "restore", "reproduce", "reproduction_id"):
        attr = getattr(sr, name, None)
        if attr is not None and not isinstance(attr, type):
            pytest.fail(f"{name} 不应是模块级可调用, 但实际有: {attr!r}")
    assert sr.PROTOCOL_VERSION.endswith("contract"), "协议版本必须标注 contract 阶段"


def test_philosophy_guard_imports() -> None:
    """philosophy_guard 守门应 PASS (引用守门, 不破坏)."""
    guard = guard_self_reproduction()
    assert guard["module"] == MODULE_NAME
    assert guard["guard_status"] in {"PASS", "WARN"}
    assert guard["deviation_count"] == 0, f"守门出现偏差: {guard}"
    # 三不哲学必须被引用
    for key in ("not_clone", "not_perfect", "not_uuid"):
        assert key in PHILOSOPHY_NOTES
        assert PHILOSOPHY_NOTES[key]


def test_module_in_apeireth() -> None:
    """self_reproduction 在 apeireth 命名空间 (占位成功)."""
    import apeireth

    assert hasattr(apeireth, "self_reproduction")
    assert apeireth.self_reproduction is sr
    assert sr.MODULE_NAME == "self_reproduction"
    assert sr.PROTOCOL_VERSION == "0.1.0-contract"
