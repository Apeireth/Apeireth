"""Quick baseline check for V1202 candidate dims."""
import json
import sys

# Test what V1144 currently reports for candidate dims
from apeireth.v1144_asi_v05_17dim_real_measure_complete import (
    _measure_eternal_identity,
    _measure_capabilities,
    _measure_engineering,
    _measure_neurosymbolic,
    _measure_phi_proxy,
    _measure_cross_domain,
    _measure_vcp_4,
)

# Real V1072 measure
from apeireth.v1072_asi_central_ai_eternal_identity import v1072_bridge_measure

print("Current V1144 fallback vs V1072 real:")
print(f"  eternal_identity hardcoded  : {_measure_eternal_identity():.4f}")
print(f"  eternal_identity V1072 real : {v1072_bridge_measure():.4f}")
print(f"  capabilities hardcoded       : {_measure_capabilities():.4f}")
print(f"  engineering hardcoded        : {_measure_engineering():.4f}")
print(f"  neurosymbolic                : {_measure_neurosymbolic():.4f}")
print(f"  phi_proxy                    : {_measure_phi_proxy():.4f}")
print(f"  cross_domain                 : {_measure_cross_domain():.4f}")
print(f"  vcp_4                        : {_measure_vcp_4():.4f}")

# V1160 / V1165 / V1167 / V1169
print()
print("V0.6 real measured:")
from apeireth.v1160_asi_rubric_open_v06_real_measure import measure_rubric_open_v06
from apeireth.v1165_asi_self_organizing_core_v06_real_measure import measure_self_organizing_core_v06

print(f"  rubric_open (V1160)        : {measure_rubric_open_v06():.4f}")
print(f"  self_organizing_core (V1165): {measure_self_organizing_core_v06():.4f}")
