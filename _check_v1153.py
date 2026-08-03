"""Quick check: V1153 measure + dim breakdown."""
import json
import subprocess
import sys
from pathlib import Path

# Run --measure
r = subprocess.run(
    [sys.executable, "-m", "apeireth.v1153_asi_v06_formal_spec", "--measure"],
    cwd=r".openclaw\workspace\promethean",
    capture_output=True, text=True, timeout=30,
)
print("MEASURE STDOUT:", r.stdout.strip())
print("MEASURE STDERR:", r.stderr[:500])

# Run --json but capture to file (slow)
r2 = subprocess.run(
    [sys.executable, "-m", "apeireth.v1153_asi_v06_formal_spec", "--json"],
    cwd=r".openclaw\workspace\promethean",
    capture_output=True, text=True, timeout=60,
)
out_path = Path(r".openclaw\workspace\promethean\tmp_v1153_full.json")
out_path.write_text(r2.stdout, encoding="utf-8")
print(f"JSON saved: {out_path} ({len(r2.stdout)} bytes)")

# Parse dim breakdown
try:
    data = json.loads(r2.stdout)
    if "dim_results" in data:
        for d in sorted(data["dim_results"], key=lambda x: x["value"]):
            print(f"  {d['dim']:30s} value={d['value']:.4f} weight={d['weight']:.4f} status={d['status']} source={d.get('source','')}")
    if "asi_v06_score" in data:
        print(f"ASI V0.6 score: {data['asi_v06_score']}")
        print(f"North star: {data['north_star']}")
        print(f"Gap: {data['gap']}")
except Exception as e:
    print(f"Parse error: {e!r}")
