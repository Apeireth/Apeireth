import sys
import json
import subprocess
import tempfile
import os

# Mimic the loader exactly as in v1466
loader = (
    "import sys, json\n"
    "sys.path.insert(0, '.')\n"
    "from apeireth.v1463_asi_lint_gate_subprocess_pipeline import _ADVERSARIAL_SPECS\n"
    "out = []\n"
    "for entry in _ADVERSARIAL_SPECS:\n"
    "    spec = entry['spec']\n"
    "    out.append({\n"
    "        'label': entry['label'],\n"
    "        'expected': entry['expected'].value,\n"
    "        'policy': entry['policy'].value,\n"
    "        'spec': {\n"
    "            'image_alias': spec.image_alias,\n"
    "            'command': list(spec.command),\n"
    "            'timeout_s': spec.timeout_s,\n"
    "            'max_output_bytes': spec.max_output_bytes,\n"
    "            'env_extra': dict(spec.env_extra),\n"
    "            'workdir_basename': spec.workdir_basename,\n"
    "        },\n"
    "    })\n"
    "print(json.dumps(out))\n"
)

with tempfile.NamedTemporaryFile(
    mode="w", suffix=".py", prefix="v1466_loader_",
    delete=False, encoding="utf-8",
) as f:
    f.write(loader)
    loader_path = f.name

print("loader path:", loader_path)
print("loader exists:", os.path.exists(loader_path))
print("loader bytes:", os.path.getsize(loader_path))
print("---")
with open(loader_path, "r", encoding="utf-8") as f:
    print(f.read())
print("---")

cmd = [sys.executable, loader_path]
print("cmd:", cmd)
proc = subprocess.run(cmd, capture_output=True, timeout=15)
print("rc:", proc.returncode)
print("stdout bytes:", len(proc.stdout))
print("stdout:", proc.stdout.decode("utf-8", errors="replace")[:300])
print("stderr bytes:", len(proc.stderr))
print("stderr:", proc.stderr.decode("utf-8", errors="replace")[:500])

os.unlink(loader_path)
