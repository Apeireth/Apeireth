import sys
sys.path.insert(0, '.')
from apeireth.v21_north_star_measure import V21NorthStarMeasure, V01MeasureResult
m = V21NorthStarMeasure()
r = m.measure_all()
print('V0.1 total:', r.total)
print('level:', r.level)
print('attrs:', [x for x in dir(r) if not x.startswith('_')])
print('values:')
for x in dir(r):
    if not x.startswith('_'):
        try:
            v = getattr(r, x)
            if not callable(v):
                print(f'  {x}: {v}')
        except:
            pass
