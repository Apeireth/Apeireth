"""Test philosophy check on Phase 42-44 new modules."""
from apeireth.philosophy import check_philosophy

# Check Phase 42 — predictive_processing
r1 = check_philosophy('predictive_processing', 'Phase 42 Rao-Ballard + Friston hierarchical predictive coding, precision-weighted errors, minimize variational free energy, veridical perception (master 17:43 truth-first)')
print(f'Phase 42 predictive_processing: passed={r1.passed}')
if r1.deviations:
    for d in r1.deviations:
        print(f'  DEVIATION: {d["line"]} - {d["concern"]}')

# Check Phase 43 — reservoir_computing
r2 = check_philosophy('reservoir_computing', 'Phase 43 Jaeger ESN + Maass LSM, reservoir is fixed random RNN + linear readout, cheap training, edge-of-chaos spectral_radius')
print(f'Phase 43 reservoir_computing: passed={r2.passed}')
if r2.deviations:
    for d in r2.deviations:
        print(f'  DEVIATION: {d["line"]} - {d["concern"]}')

# Check Phase 44 — criticality
r3 = check_philosophy('criticality', 'Phase 44 Bak-Tang SOC + Beggs-Plenz neuronal avalanches, branching sigma ~1 critical state, power law avalanche size, master 17:50 emergence self-organization math language')
print(f'Phase 44 criticality: passed={r3.passed}')
if r3.deviations:
    for d in r3.deviations:
        print(f'  DEVIATION: {d["line"]} - {d["concern"]}')