#!/usr/bin/env python3
"""Probe V1065 measure/score for debugging."""
import sys
sys.path.insert(0, '.')

from apeireth.v1065_asi_self_organizing_core import build_self_organizing_core
soc = build_self_organizing_core()
m = soc.measure()
for k, v in m.items():
    w = soc.bridge.weights[k]
    print(f"  {k:36s} val={v:.3f} w={w:.2f} contrib={v*w:.3f}")
score = soc.score()
v0_2 = score["self_organizing_core_v0_2"]
print(f"V0.2 total: {v0_2}")
