"""Cron tick audit: real production vs shell."""
import os, subprocess, re

proj = r".openclaw\workspace\promethean"
os.chdir(proj)

# V modules
mods = sorted([f for f in os.listdir('apeireth') if f.startswith('v') and f.endswith('.py')])
v_count = len(mods)

# Test files + tests
test_files = sorted([f for f in os.listdir('tests') if f.startswith('test_v') and f.endswith('.py')])
total_tests = 0
for tf in test_files:
    with open(f'tests/{tf}', encoding='utf-8') as fp:
        content = fp.read()
    total_tests += len(re.findall(r'def test_', content))

# Commits
commits_raw = subprocess.run(['git', 'log', '--oneline'], capture_output=True).stdout.decode('utf-8', errors='replace').strip()
commits = len(commits_raw.split('\n'))

# Real vs shell
real = [(m, os.path.getsize(f'apeireth/{m}')) for m in mods if os.path.getsize(f'apeireth/{m}') > 4000]
shell = [(m, os.path.getsize(f'apeireth/{m}')) for m in mods if os.path.getsize(f'apeireth/{m}') <= 4000]

print(f"v-modules total: {v_count}")
print(f"real production (>4KB): {len(real)}")
print(f"shell/empty (<=4KB): {len(shell)}")
print(f"test files: {len(test_files)}")
print(f"tests: {total_tests}")
print(f"commits: {commits}")
print()
print("Latest 10 v-modules:")
for m in mods[-10:]:
    sz = os.path.getsize(f'apeireth/{m}')
    flag = "REAL" if sz > 4000 else "shell"
    print(f"  {m}: {sz} bytes [{flag}]")

# ASI north star
import sys
sys.path.insert(0, '.')
try:
    from apeireth.v21_north_star_measure import V21V01FormulaMeasure
    m = V21V01FormulaMeasure().measure()
    print(f"\nASI V0.1: {m.total:.4f}")
except Exception as e:
    print(f"\nASI V0.1 measure error: {e}")