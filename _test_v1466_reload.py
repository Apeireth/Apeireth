import sys
sys.path.insert(0, ".")
import importlib

# Force fresh import
if "apeireth.v1466_asi_real_cross_process_lint_gate_subprocess_runner" in sys.modules:
    del sys.modules["apeireth.v1466_asi_real_cross_process_lint_gate_subprocess_runner"]

from apeireth.v1466_asi_real_cross_process_lint_gate_subprocess_runner import (
    _load_v1463_adversarial_specs,
    _run_subprocess,
    _try_parse_json_bytes,
)

print("loading 3 specs...")
specs = _load_v1463_adversarial_specs(3)
print("specs count:", len(specs))
if specs:
    print("first label:", specs[0].get("label"))
else:
    # Debug: call _run_subprocess + _try_parse_json_bytes manually
    print("---debugging---")
    import tempfile, os

    loader = (
        "import sys, json\n"
        "sys.path.insert(0, '.')\n"
        "from apeireth.v1463_asi_lint_gate_subprocess_pipeline import _ADVERSARIAL_SPECS\n"
        "out = []\n"
        "for entry in _ADVERSARIAL_SPECS:\n"
        "    spec = entry['spec']\n"
        "    out.append({'label': entry['label']})\n"
        "print(json.dumps(out))\n"
    )
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".py", prefix="v1466_loader_",
        delete=False, encoding="utf-8",
    ) as f:
        f.write(loader)
        loader_path = f.name
    rc, stdout, stderr, elapsed, err = _run_subprocess(
        [sys.executable, loader_path], timeout_s=15.0
    )
    print("rc:", rc, "err:", err)
    print("stdout len:", len(stdout))
    parsed, perr = _try_parse_json_bytes(stdout)
    print("parsed:", parsed is not None, "perr:", perr)
    if parsed:
        print("parsed type:", type(parsed), "len:", len(parsed))
    os.unlink(loader_path)
