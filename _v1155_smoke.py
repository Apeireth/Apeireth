import sys, time
sys.path.insert(0, '.')
from apeireth.v1155_asi_v06_trend_baseline import snapshot_v06, run_v1155_acceptance

t0 = time.time()
snap = snapshot_v06()
t1 = time.time()
print(f'snapshot took {t1-t0:.2f}s')
print(f'score={snap.score:.4f} gap={snap.gap:+.4f}')
print(f'n_dims={snap.n_dims} R={snap.n_real} H={snap.n_hardcoded} P={snap.n_partial} M={snap.n_missing}')

t0 = time.time()
acc = run_v1155_acceptance()
t1 = time.time()
print(f'acceptance took {t1-t0:.2f}s')
print(f'  pass {acc["n_pass"]}/{acc["n_tests"]}')
for t in acc["tests"]:
    mark = "OK" if t["passed"] else "FAIL"
    print(f'  {mark} {t["name"]}')