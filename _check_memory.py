import os, time
mtime = os.path.getmtime('memory/2026-08-01.md') if os.path.exists('memory/2026-08-01.md') else 0
print(f'memory file mtime: {mtime}')
print(f'now: {time.time()}')
print(f'age sec: {time.time() - mtime}')
print(f'age min: {(time.time() - mtime) / 60:.1f}')