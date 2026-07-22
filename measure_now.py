#!/usr/bin/env python3
"""Quickly estimate ASI V0.2 with current state of components."""
import apeireth.v1002_asi_v02_measure as m

# Lower-bound estimate of where each dimension might stand today.
# Higher-weighted things done well, lower weights less so.
scores = {
    "phi_proxy": 0.85,
    "capabilities": 0.78,
    "cross_domain": 0.85,
    "engineering": 0.78,
    "vcp_4": 0.80,
    "v2_philosophy": 0.85,
    "rubric_open": 0.80,
    "real_production": 0.78,
    "cognitive_core": 0.85,
    "self_organizing_core": 0.55,
    "plugin_core": 0.45,
    "self_improving_core": 0.55,
    "neurosymbolic": 0.40,
    "world_model": 0.72,
    "reinforcement_learning": 0.30,
    "scientific_method": 0.85,
}

res = m.compute_asi_v02_total(scores)
print(f"V0.2 Total: {res.total:.4f}  Level: {res.level}")
for k, v in res.contributions.items():
    flag = "  " if v["contribution"] >= 0.04 else "L "
    print(f"  {flag} {k:32s} raw={v['raw_score']:.2f} weight={v['weight']:.2f} contrib={v['contribution']:.4f}")
