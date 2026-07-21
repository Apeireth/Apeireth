"""V251-V500 真生产 batch tests (主 23:36)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
import importlib
import re


def test_v251_v500_sample():
    """V251-V500 真生产 250 modules basic instantiation."""
    from pathlib import Path
    base = Path(r".openclaw\workspace\promethean\apeireth")
    # 测 251-500, 简单正则
    modules = sorted(p for p in base.glob("v*.py")
                    if re.match(r"v(2[5-9][0-9]|[3-4][0-9][0-9]|500)_", p.stem))
    sample = modules[:15] + modules[80:95] + modules[180:195]
    assert len(sample) > 0, f"V251-V500 modules not found (found {len(modules)})"
    for p in sample:
        mod_name = "apeireth." + p.stem
        try:
            mod = importlib.import_module(mod_name)
        except Exception as e:
            pytest.fail(f"failed to import {mod_name}: {e}")
        classes = [name for name in dir(mod)
                   if name.startswith("V") and "Module" in name]
        assert len(classes) > 0, f"no V Module class in {mod_name}"
        for class_name in classes:
            cls = getattr(mod, class_name)
            obj = cls()
            assert obj is not None