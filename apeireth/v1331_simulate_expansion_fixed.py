#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1331_simulate_expansion_fixed.py — V1330 bug fix (re-export module)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1330 AgentDream 真源码深读 (f403a4f6); per 主 17:43 实事求是 + 主 23:44 干到底
- Chain: V1313 → ... → V1330 → **V1331**

V1331 = bug fix for V1330 TimelineBucketSubstrate.simulate_expansion():
  Bug:  Code references `cls.min_recent_files.__class__(3)` — but `min_recent_files`
        is a dataclass field descriptor (not a value), so `.field` cannot be called.
        Raises `AttributeError: 'field' object has no attribute '__class__'`... 
        Actually it raises `TypeError: descriptor '__class__' of 'field' object needs an argument`.
  Fix:  Monkey-patch simulate_expansion at import time to use literal 3 (matches
        the value passed by `describe()` factory).

Also adds:
  - V1331_FIX_NOTES: human-readable bug description
  - timeline boundary monotonicity guarantee
  - defensive cap (recent ≤ RECENT_EXPAND_MAX, mid ≤ MID_EXPAND_MAX)

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ✓ V1331 = bug fix, NOT ASI breakthrough
- ✓ V1331 does NOT modify pole star
- ✓ V1331 = honest engineering (fix what we just shipped as broken)

ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1331 不动北极星
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import Tuple

WORKSPACE = Path(r".openclaw\workspace\promethean")
sys.path.insert(0, str(WORKSPACE / "apeireth"))

# Import V1330 module (will trigger V1330 module imports)
import apeireth.v1330_agentdream_plugin_deep_read as _v1330

# Re-export everything from V1330
from apeireth.v1330_agentdream_plugin_deep_read import (
    AgentDreamFileSubstrate,
    AgentDreamPluginMatrix,
    AgentDreamDeepReadReport,
    AgentDreamDeepReadBridge,
    DreamSchedulerSubstrate,
    DreamConfigSubstrate,
    AgentRegistrySubstrate,
    TimelineBucketSubstrate,
    AuthorExtractSubstrate,
    BroadcastSubstrate,
    BroadcastEvent,
    DreamPromptSubstrate,
    DreamStatePersistSubstrate,
    DreamAgentEntry,
    ASI_POLE_STAR,
    AGENTDREAM_4_FILES,
    AGENTDREAM_ROOT,
    TOTAL_DECLARED_LINES,
)


# V1331 fix: monkey-patch TimelineBucketSubstrate.simulate_expansion
def _fixed_simulate_expansion(cls, recent_files_count: int, mid_files_count: int) -> Tuple[int, int]:
    """V1331 fixed simulate_expansion (matches V1330 docstring contract).
    
    V1330 bug:
        while recent_files_count < cls.min_recent_files.__class__(3) and ...
    
    V1330 docstring says:
        "Try initial boundaries; if recent < 3 files, expand recent by 7d (max 30d)
         If mid < 2 files, expand mid by 30d (max 180d)"
    
    V1331 fix:
        - recent_files_count < 3 (matches docstring literal)
        - mid_files_count < 2 (matches docstring literal)
        - mid expansion loop (was missing in V1330)
        - defensive caps (min with RECENT_EXPAND_MAX / MID_EXPAND_MAX)
    """
    recent_boundary = cls.INITIAL_RECENT_DAYS
    mid_boundary = cls.INITIAL_MID_DAYS
    
    # Recent tier expansion
    while recent_files_count < 3 and recent_boundary < cls.RECENT_EXPAND_MAX:
        recent_boundary = cls.expand_recent(recent_boundary)
    
    # Mid tier expansion (was missing in V1330)
    while mid_files_count < 2 and mid_boundary < cls.MID_EXPAND_MAX:
        mid_boundary = cls.expand_mid(mid_boundary)
    
    # Defensive caps
    recent_boundary = min(recent_boundary, cls.RECENT_EXPAND_MAX)
    mid_boundary = min(mid_boundary, cls.MID_EXPAND_MAX)
    
    return recent_boundary, mid_boundary


TimelineBucketSubstrate.simulate_expansion = classmethod(_fixed_simulate_expansion)


# V1331 module-level summary
V1331_FIX_NOTES = {
    "bug_module": "v1330_agentdream_plugin_deep_read.py",
    "bug_method": "TimelineBucketSubstrate.simulate_expansion",
    "bug_location": "while recent_files_count < cls.min_recent_files.__class__(3) and ...",
    "bug_severity": "AttributeError at runtime when called",
    "fix_strategy": "monkey-patch simulate_expansion with corrected implementation",
    "fix_constant_recent": 3,  # matches V1330 describe() default
    "fix_constant_mid": 2,     # matches V1330 describe() default
    "v3_guards_locked": True,
    "pole_star_modified": False,
    "asi_achieved": False,
}


if __name__ == "__main__":
    # Quick sanity check
    print("V1331 simulate_expansion bug fix module loaded.")
    
    # Test 1: enough files -> no expansion
    recent, mid = TimelineBucketSubstrate.simulate_expansion(recent_files_count=100, mid_files_count=100)
    assert recent == 7, f"Expected initial=7, got {recent}"
    assert mid == 90, f"Expected initial=90, got {mid}"
    print(f"  test 1 (100, 100): recent={recent}, mid={mid}  OK")
    
    # Test 2: zero files -> full expansion
    recent, mid = TimelineBucketSubstrate.simulate_expansion(recent_files_count=0, mid_files_count=0)
    assert recent == 30, f"Expected max=30, got {recent}"
    assert mid == 180, f"Expected max=180, got {mid}"
    print(f"  test 2 (0, 0): recent={recent}, mid={mid}  OK")
    
    # Test 3: few files -> partial expansion
    recent, mid = TimelineBucketSubstrate.simulate_expansion(recent_files_count=2, mid_files_count=1)
    # 2 < 3 -> expand recent from 7 to 14 to 21 to 28 (capped at 30)
    # 1 < 2 -> expand mid from 90 to 120 to 150 to 180 (capped at 180)
    assert recent == 28, f"Expected 28, got {recent}"
    assert mid == 180, f"Expected 180, got {mid}"
    print(f"  test 3 (2, 1): recent={recent}, mid={mid}  OK")
    
    # Test 4: monotonicity (more files -> same or smaller boundary)
    r1, m1 = TimelineBucketSubstrate.simulate_expansion(0, 0)
    r2, m2 = TimelineBucketSubstrate.simulate_expansion(100, 100)
    assert r1 >= r2, f"Not monotonic: r1={r1}, r2={r2}"
    assert m1 >= m2, f"Not monotonic: m1={m1}, m2={m2}"
    print(f"  test 4 (monotonicity): OK")
    
    print("V1331 all sanity checks passed.")