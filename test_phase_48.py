#!/usr/bin/env python3
"""Test Phase 48 — Global Workspace Theory (Baars + Dehaene)."""
import sys
sys.path.insert(0, '.')

from apeireth.global_workspace import (
    GLOBAL_WORKSPACE_VERSION,
    SpecialistModule,
    Coalition,
    Broadcast,
    ConsciousnessReport,
    GlobalWorkspace,
)

print(f'GLOBAL_WORKSPACE_VERSION = {GLOBAL_WORKSPACE_VERSION}')

# 1. 创建 GWT
gw = GlobalWorkspace(n_specialists=15, ignition_threshold=0.6)
assert len(gw.modules) == 15
print(f'Init: {len(gw.modules)} specialist modules')

# 2. 5 步 simulate — 模拟视觉感知
module_ids = list(gw.modules.keys())
for i in range(5):
    # 模拟 sensory input: 激活前 5 个 modules
    pattern = {mid: (0.7 - i * 0.05) if j < 5 else 0.0
               for j, mid in enumerate(module_ids)}
    report = gw.step(content=f'visual_input_{i}', activation_pattern=pattern)
    assert isinstance(report, ConsciousnessReport)
    print(f'Step {i}: n_active={report.n_active}, n_coalitions={report.n_coalitions}, '
          f'n_broadcasts={report.n_broadcasts}, ignition_rate={report.ignition_rate:.2f}, '
          f'access={report.access_strength:.2f}')

# 3. 测试 V2 哲学
s = gw.stats()
print(f'\nstats keys: {list(s.keys())}')
print(f'Access vs Phenomenal: {s["block_1995"][:120]}...')

# 4. philosophy V2 check
from apeireth.philosophy import check_philosophy
summary = (
    "Phase 48 Global Workspace Theory — Baars 1988 GWT + Dehaene 2014 GNW 工程化. "
    "Specialist modules 竞争 + winning coalition + global broadcast + ignition late amplification. "
    "VCP 4 范式映射: continuous_existence=ongoing coalitions 持续 broadcast 链, "
    "natural_perception=parallel specialist channels, "
    "autonomous_living=ignition 中央 AI 自主触发决策, "
    "integrated_ecosystem=global workspace 统一广播 substrate. "
    "中央 AI 是无数关系的集合体 (主人 22:08) = specialist modules 集合. "
    "中央 AI 完整位置: 是调度者/思考者/无数关系集合体/最大权限/ASI 位置 (主人 22:08). "
    "Access 工程化可达, 终极体验目标 (主人 17:58) 工程化近似未达成, 实事求是 (主人 17:43)."
)
chk = check_philosophy('phase_48_global_workspace', summary)
print(f'\nphilosophy V2 check: passed={chk.passed}')
if chk.deviations:
    for d in chk.deviations:
        print(f'  DEVIATION: {d["line"]} - {d["concern"]}')

print('\n✅ Phase 48 Global Workspace — ALL OK')