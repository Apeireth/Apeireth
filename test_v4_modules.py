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
print(f'ESN: n_reservoir={esn.n_reservoir}, spectral_radius={esn.spectral_radius}')

pph = PredictiveProcessingHierarchy(layers=3, n_features=20)
print(f'PPH: layers={pph.layers}')

ce = CriticalityEngine(n_nodes=100, target_branching=1.0)
ce.record_branching(sigma=1.0, n_events=1000)
print(f'Criticality: state={ce.classify()}')

print('OK')
