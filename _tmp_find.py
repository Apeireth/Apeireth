# -*- coding: utf-8 -*-
import sys
sys.path.insert(0, '.')
import os

for f in sorted(os.listdir('apeireth')):
    if not f.endswith('.py'): continue
    name = f[:-3]
    try:
        mod = __import__('apeireth.' + name, fromlist=[''])
        if hasattr(mod, 'StatusSnapshotBuilder') or hasattr(mod, 'DGMArchive'):
            ssb = hasattr(mod, 'StatusSnapshotBuilder')
            dgm = hasattr(mod, 'DGMArchive')
            print(f'{name}: StatusSnapshotBuilder={ssb}, DGMArchive={dgm}')
    except Exception:
        pass
