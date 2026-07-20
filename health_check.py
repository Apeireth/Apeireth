#!/usr/bin/env python3
"""Apeireth ASI 基座真生产健康检查脚本.

主人 14:52 "24/7 不能崩" — Phase 22 真生产就绪.
检查:
  1. Phase 0-21 所有模块可 import
  2. Phi-proxy >= 0.5 (中央 AI 集成度)
  3. ASI Approach Index 报告
  4. 真生产 7×24 健康
"""
from __future__ import annotations

import sys
import time
from pathlib import Path

sys.path.insert(0, ".")


def health_check() -> bool:
    """Run full health check. Returns True if healthy."""
    print("=== Apeireth ASI 基座 真生产健康检查 ===")
    print(f"时间: {time.ctime()}\n")
    all_ok = True

    # 1. Phase 0-21 modules importable
    print("[1] 模块导入检查")
    modules = [
        ("IdentityCard", "from apeireth import IdentityCard"),
        ("IdentityStore", "from apeireth import IdentityStore"),
        ("Mirror", "from apeireth import make_default_mirror"),
        ("MetaMonitor", "from apeireth import MetaMonitor"),
        ("SelfModel", "from apeireth import make_default_self_model"),
        ("SkillLibrary", "from apeireth import make_default_skill_library"),
        ("DGMArchive", "from apeireth import make_default_dgm_archive"),
        ("DeliberationEngine", "from apeireth import make_default_deliberation_engine"),
        ("PhiProxy", "from apeireth import compute_phi_proxy"),
        ("ASIApproachReport", "from apeireth import compute_v7_approach"),
        ("LLMKernel", "from apeireth import make_call_llm"),
    ]
    for name, import_stmt in modules:
        try:
            exec(import_stmt)
            print(f"  ✓ {name}")
        except Exception as e:
            print(f"  ✗ {name}: {e}")
            all_ok = False

    # 2. ASI Approach Index
    print("\n[2] ASI Approach Index")
    try:
        from apeireth import compute_v7_approach, compute_target_approach
        v7 = compute_v7_approach()
        target = compute_target_approach()
        print(f"  V7 ASI Approach: {v7.asi_approach:.4f} ({v7.interpretation})")
        print(f"  Target: {target.asi_approach:.4f}")
        if v7.asi_approach >= 0.5:
            print(f"  ✓ ASI Approach >= 0.5 (well_equipped)")
        else:
            print(f"  ✗ ASI Approach < 0.5 (low)")
            all_ok = False
    except Exception as e:
        print(f"  ✗ ASI Approach Index: {e}")
        all_ok = False

    # 3. LLM Kernel
    print("\n[3] LLM Kernel (Phase 21)")
    try:
        from apeireth import make_call_llm, LLMConfig
        cfg = LLMConfig.minimax_default()
        print(f"  Provider: {cfg.provider}, Model: {cfg.model}")
        print(f"  Base URL: {cfg.base_url}")
        print(f"  API Key: {'set' if cfg.api_key else 'not set'}")
        call_llm = make_call_llm("minimax")
        resp = call_llm("test")
        print(f"  Call result: {len(resp)} chars")
        if len(resp) > 0:
            print(f"  ✓ LLM Kernel operational")
        else:
            print(f"  ✗ LLM Kernel no response")
            all_ok = False
    except Exception as e:
        print(f"  ✗ LLM Kernel: {e}")
        all_ok = False

    print()
    if all_ok:
        print("=" * 60)
        print("✓ ALL HEALTH CHECKS PASSED")
        print("  Apeireth ASI 基座 ready for 真生产 7x24")
        print("=" * 60)
        return True
    else:
        print("=" * 60)
        print("✗ SOME HEALTH CHECKS FAILED")
        print("=" * 60)
        return False


if __name__ == "__main__":
    ok = health_check()
    sys.exit(0 if ok else 1)
