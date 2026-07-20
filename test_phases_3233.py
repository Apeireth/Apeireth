#!/usr/bin/env python3
"""Phase 32 + 33 真生产测试 — 主人 21:30 跨域."""
import sys, json
sys.path.insert(0, '.')

from apeireth.variety import ASHBY_VARIETY_VERSION, VarietyMeasure, RequisiteVarietyCalculator
from apeireth.active_inf import ACTIVE_INFERENCE_VERSION, Belief, Perception, ActiveInferenceAgent

print(f'Phase 32 RequisiteVariety: {ASHBY_VARIETY_VERSION}')
print(f'Phase 33 ActiveInference: {ACTIVE_INFERENCE_VERSION}')

# Requisite Variety
rvc = RequisiteVarietyCalculator()
env_v = rvc.shannon_entropy([10, 5, 3, 2, 1])
ctrl_v = rvc.shannon_entropy([1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
check = rvc.requisite_check(env_v, ctrl_v, context='中央 AI persona 多样性 vs 任务多样性')
print(f'  Ashby check: ctrl({ctrl_v.shannon_bits:.3f}) >= env({env_v.shannon_bits:.3f}) -> {check["requisite_satisfied"]}')

# Active Inference
ai = ActiveInferenceAgent()
b1 = ai.add_belief('主人 14:48 聚集全人类智慧', initial_precision=1.0)
ai.perceive('master 21:30 真生产跨域借鉴', prediction=0.0, actual=0.9)
ai.perceive('master 21:22 并行干提升效率', prediction=0.5, actual=0.8)
for _ in range(5):
    ai.update_belief(b1.belief_id, evidence=0.9, learning_rate=0.2)
print(f'  ActiveInf stats: {ai.stats()}')
print(f'  free_energy_history[-3:]: {[round(x, 3) for x in ai.free_energy_history[-3:]]}')
print('OK Phase 32+33 work')
