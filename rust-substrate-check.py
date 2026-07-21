import subprocess
import os
# Rust substrate check
print('=== Rust substrate check (主 14:32 + 14:47 高效 nb) ===')
os.chdir('rust-substrate')
r = subprocess.run(['cargo', 'check', '--workspace'], capture_output=True, text=True, timeout=120)
print(f'cargo check: {r.returncode}')
if r.returncode == 0:
    print('PASS: Rust substrate 6 crates workspace compile OK')
else:
    print(f'FAIL: {r.stderr[:500]}')