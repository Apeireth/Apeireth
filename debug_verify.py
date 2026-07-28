import sys
sys.argv = ['v1101']
if 'apeireth.v1101_asi_v04_dim_lift' in sys.modules:
    del sys.modules['apeireth.v1101_asi_v04_dim_lift']
from apeireth.v1101_asi_v04_dim_lift import LiftExecutor, LiftVerifier
e = LiftExecutor(dry_run=True, backup=False)
lift = e.execute_all()
v = LiftVerifier()
import importlib
out = {}
for dim in ['cognitive_core', 'engineering', 'v2_philosophy']:
    print(f'TRY {dim}', flush=True)
    try:
        if 'apeireth.v1077_asi_v04_full_measurement' in sys.modules:
            del sys.modules['apeireth.v1077_asi_v04_full_measurement']
        v1077 = importlib.import_module('apeireth.v1077_asi_v04_full_measurement')
        print(f'{dim}: v1077 imported', flush=True)
        spec = v1077.DimensionSpec(
            name=dim,
            weight=v1077.V04_WEIGHTS.get(dim, 0.0),
            module_id='V1061' if dim == 'cognitive_core' else ('V1060' if dim == 'engineering' else 'V1003'),
            measurement_kind='compute_metrics' if dim == 'cognitive_core' else ('test_coverage' if dim == 'engineering' else 'philosophy_guard_pass'),
            description=f'verify {dim}',
        )
        print(f'{dim}: spec made', flush=True)
        measurer = v1077.MeasurementRunner(v1077.DimensionRegistry())
        print(f'{dim}: measurer ready', flush=True)
        if dim == 'cognitive_core':
            raw = measurer._measure_compute_metrics(spec)
        elif dim == 'engineering':
            raw = measurer._measure_test_coverage(spec)
        else:
            raw = measurer._measure_philosophy_guard(spec)
        sc = raw.get('score', '?')
        print(f'{dim}: score = {sc}', flush=True)
        out[dim] = {'score': float(raw.get('score', 0.0)), 'raw': raw.get('raw', {})}
    except Exception as ex:
        print(f'{dim}: error {ex}', flush=True)
        out[dim] = {'error': str(ex)}
print('FINAL:', out, flush=True)