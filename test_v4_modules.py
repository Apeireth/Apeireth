#!/usr/bin/env python3
"""Test V4 modules with correct signatures."""
import sys
sys.path.insert(0, '.')

from apeireth.reservoir_computing import RESERVOIR_VERSION, EchoStateNetwork
from apeireth.predictive_processing import PREDICTIVE_PROCESSING_VERSION, PredictiveProcessingHierarchy
from apeireth.criticality import CRITICALITY_VERSION, CriticalityEngine

print(f'RESERVOIR={RESERVOIR_VERSION}')
print(f'PPH={PREDICTIVE_PROCESSING_VERSION}')
print(f'CRITICALITY={CRITICALITY_VERSION}')

esn = EchoStateNetwork(n_reservoir=100, spectral_radius=0.9)
print(f'ESN: n_reservoir={esn.n_reservoir}, target_spectral_radius={esn.target_spectral_radius}')

pph = PredictiveProcessingHierarchy(n_layers=3)
print(f'PPH: n_layers={pph.n_layers}')

ce = CriticalityEngine(n_nodes=100, target_branching=1.0)
report = ce.run(n_triggers=50)
print(f'Criticality: state={report.state}, is_critical={report.is_critical}')

print('OK')
