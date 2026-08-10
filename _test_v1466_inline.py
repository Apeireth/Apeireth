import subprocess
import sys
import json

cmd = [
    sys.executable, "-c",
    "import sys, json;"
    "sys.path.insert(0, '.');"
    "from apeireth.v1463_asi_lint_gate_subprocess_pipeline import _ADVERSARIAL_SPECS;"
    "out = [];"
    "for entry in _ADVERSARIAL_SPECS:"
    "  spec = entry['spec'];"
    "  out.append({"
    "    'label': entry['label'],"
    "    'expected': entry['expected'].value,"
    "    'policy': entry['policy'].value,"
    "    'spec': {"
    "      'image_alias': spec.image_alias,"
    "      'command': list(spec.command),"
    "      'timeout_s': spec.timeout_s,"
    "      'max_output_bytes': spec.max_output_bytes,"
    "      'env_extra': dict(spec.env_extra),"
    "      'workdir_basename': spec.workdir_basename,"
    "    }"
    "  });"
    "print(json.dumps(out))",
]
proc = subprocess.run(cmd, capture_output=True, timeout=10)
print("rc:", proc.returncode)
print("stdout len:", len(proc.stdout))
print("stdout first 200:", proc.stdout[:200].decode("utf-8", errors="replace"))
print("stderr:", proc.stderr[:500].decode("utf-8", errors="replace"))
