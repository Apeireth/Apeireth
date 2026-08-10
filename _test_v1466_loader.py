import sys
import json
import subprocess
import tempfile
import os

loader = '''import sys, json
sys.path.insert(0, ".")
from apeireth.v1463_asi_lint_gate_subprocess_pipeline import _ADVERSARIAL_SPECS
out = []
for entry in _ADVERSARIAL_SPECS:
    spec = entry["spec"]
    out.append({"label": entry["label"], "policy": entry["policy"].value, "expected": entry["expected"].value, "spec": {"image_alias": spec.image_alias, "command": list(spec.command)}})
print(json.dumps(out))
'''

with tempfile.NamedTemporaryFile(mode="w", suffix=".py", delete=False, encoding="utf-8") as f:
    f.write(loader)
    p = f.name

print("loader path:", p)
print("---")
print("contents:")
with open(p, "r", encoding="utf-8") as f:
    print(f.read())
print("---")
proc = subprocess.run([sys.executable, p], capture_output=True, timeout=15)
print("rc:", proc.returncode)
print("stdout:", proc.stdout.decode("utf-8", errors="replace")[:300])
print("stderr:", proc.stderr.decode("utf-8", errors="replace")[:500])
os.unlink(p)
