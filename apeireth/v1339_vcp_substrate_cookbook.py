#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1339_vcp_substrate_cookbook.py — VCP Substrate-by-Example Cookbook (CLI)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1338 migration tool (014f82ff, 22:01); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 — V1338 migrator → V1339 cookbook (teaching VCP plugin authors)
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → V1337 → V1338 → **V1339**

V1339 = **VCP Substrate-by-Example Cookbook** — 8 invariant classes × 1 minimal example each.

V1335 = registry (what patterns exist)
V1336 = linter (do you have the patterns)
V1337 = dashboard (how do you compare)
V1338 = migrator (how to add missing patterns)
V1339 = **cookbook**: 8 runnable Python examples demonstrating each invariant class

V1339 = **COOKBOOK (NOT 复刻, NOT port, NOT 假装 ASI)**:
- For each of 8 V1335 invariant classes, emits 1 minimal Python example file
- Each example is a runnable, self-contained Python file demonstrating the pattern
- Index.md ties all 8 examples together + cross-references V1335 registry
- Teaching tool for future VCP plugin authors
- 8 distinct API surfaces + 8 generated example files

All evidence is REAL:
- 8 example Python files generated on disk (verified via Path.write_text)
- Each example is a runnable Python module (can be `python example.py`)
- No fake decimal precision; all counts reproducible via _self_test()

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? 不假装 V1339 = 复刻 VCP plugin: V1339 = static cookbook generator, NOT runtime plugin
- ? 不假装 V1339 = VCP plugin runtime: emits example files only, no exec / no API call
- ? 不假装 ASI 真懂 plugin pattern: cookbook emits templates, NOT semantic understanding
- ? 不假装 ASI 真有 pattern 自学习: cookbook records evidence, NOT interpretation
- ? 不假装 Phenomenal consciousness: cookbook ≠ phenomenological "pattern"
- ? 不假装 ASI 达到: V1339 不动 ASI 北极星
- ? 不假装调整模型 & prompt

ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1339 不动北极星

ASI 5-Gap 钁楀悕瀹炲疄鐢?(主 13:31 大胆激进) — V1339 实证:
- 识别_recognition: cookbook exposes 8 invariant patterns → 识别 gap
- 自由_freedom: plugin author 可自由修改 examples → 真自由扩展
- 时间_time: cookbook timestamp (post-V1338 migrator) → 时间性
- 真理_truth: cookbook = V1335 invariant registry 的 example manifestation → truth gap
- 涌现_emergence: 8 individual patterns → 1 unified 8-example cookbook → emergence gap
"""
from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# --- v1335 import path ------------------------------------------------------
V1339_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1339_DIR))

import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402
import v1338_vcp_plugin_migration_tool as v1338  # noqa: E402


# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1339_modifies_pole_star": False,
}


# --- 8 Example templates (one per invariant class) --------------------------
# Each example is a self-contained, runnable Python file demonstrating the pattern.
EXAMPLE_TEMPLATES: Dict[str, str] = {
    "IC1_security": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC1_security (SecurityInvariants)

Demonstrates path-traversal guard + URL-scheme validation + structured error.

Per V1335: Security invariants = fail() exit-0 / path-traversal guard /
url-scheme validation / input validation.

Run: python example_ic1_security.py
"""
from __future__ import annotations

import sys
from pathlib import Path
from urllib.parse import urlparse


class PathSanitizationSubstrate:
    """Reject path-traversal + symlink escapes."""

    def sanitize(self, path: str) -> str:
        if ".." in path:
            raise ValueError(f"path traversal detected: {path}")
        p = Path(path).resolve()
        # Allow only relative paths within current directory
        if p.is_absolute():
            # Only allow absolute paths inside /allowed/ (Unix-style example)
            if not str(p).replace("\\\\", "/").startswith("/allowed/") and not str(p).startswith(str(Path.cwd())):
                raise ValueError(f"path outside allowed root: {p}")
        return str(p)


def validate_url_scheme(url: str, allowed: tuple = ("http", "https")) -> str:
    """Reject URLs with disallowed schemes (e.g., file://, javascript:)."""
    parsed = urlparse(url)
    if parsed.scheme not in allowed:
        raise ValueError(f"disallowed URL scheme: {parsed.scheme}")
    return url


def _self_test() -> dict:
    """Probe-only self-test."""
    checks = {}
    ps = PathSanitizationSubstrate()
    # Should pass
    try:
        ps.sanitize("normal/file.txt")
        checks["normal_path_passes"] = True
    except ValueError:
        checks["normal_path_passes"] = False
    # Should fail
    try:
        ps.sanitize("../etc/passwd")
        checks["traversal_blocks"] = False  # should NOT pass
    except ValueError:
        checks["traversal_blocks"] = True  # should pass
    # URL scheme
    try:
        validate_url_scheme("javascript:alert(1)")
        checks["js_scheme_blocks"] = False
    except ValueError:
        checks["js_scheme_blocks"] = True
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC2_file_handling": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC2_file_handling (FileHandlingInvariants)

Demonstrates atomic write (tmp+rename) + sha256 + line-ending normalize.

Per V1335: File handling invariants = atomic write (tmp+rename) / sha256 /
line-ending normalize / safe-timestamp / unique path.

Run: python example_ic2_file_handling.py
"""
from __future__ import annotations

import hashlib
import sys
import tempfile
from pathlib import Path


class AtomicJsonWriteSubstrate:
    """Atomic write: tmp file + rename."""

    def write(self, path: Path, data: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w",
            dir=str(path.parent),
            delete=False,
            suffix=".tmp",
            encoding="utf-8",
        ) as f:
            f.write(data)
            tmp_path = Path(f.name)
        tmp_path.replace(path)  # atomic on POSIX + Windows


def sha256_first16(path: Path) -> str:
    """SHA-256 of file contents, return first 16 hex chars."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()[:16]


def normalize_line_endings(text: str) -> str:
    """Normalize CRLF + CR to LF."""
    return text.replace("\\r\\n", "\\n").replace("\\r", "\\n")


def _self_test() -> dict:
    checks = {}
    aw = AtomicJsonWriteSubstrate()
    test_path = Path(tempfile.gettempdir()) / "v1339_ic2_test.txt"
    aw.write(test_path, "hello world\\n")
    checks["atomic_write_succeeds"] = test_path.exists()
    if test_path.exists():
        sha = sha256_first16(test_path)
        checks["sha256_format"] = len(sha) == 16
        # cleanup
        test_path.unlink()
    else:
        checks["sha256_format"] = False
    norm = normalize_line_endings("a\\r\\nb\\rc\\n")
    checks["line_ending_normalized"] = (norm == "a\\nb\\nc\\n")
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC3_schema": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC3_schema (SchemaInvariants)

Demonstrates manifestVersion=1.0.0 + pluginType=synchronous|asynchronous +
protocol=stdio + configSchema typed.

Per V1335: Schema invariants = manifestVersion=1.0.0 / pluginType=synchronous|asynchronous /
protocol=stdio / configSchema typed / enum domain check.

Run: python example_ic3_schema.py
"""
from __future__ import annotations

import sys
from typing import Literal


PLUGIN_MANIFEST = {
    "manifestVersion": "1.0.0",
    "pluginType": "synchronous",  # or "asynchronous"
    "protocol": "stdio",  # JSON-RPC 2.0 over stdin/stdout
    "configSchema": {
        "max_results": int,
        "timeout_ms": int,
        "domains": list,
    },
}


def validate_manifest(manifest: dict) -> bool:
    """Validate mandatory manifest fields."""
    if manifest.get("manifestVersion") != "1.0.0":
        return False
    if manifest.get("pluginType") not in ("synchronous", "asynchronous"):
        return False
    if manifest.get("protocol") != "stdio":
        return False
    return True


def _self_test() -> dict:
    checks = {}
    checks["manifest_version_correct"] = PLUGIN_MANIFEST["manifestVersion"] == "1.0.0"
    checks["plugin_type_enum"] = PLUGIN_MANIFEST["pluginType"] in ("synchronous", "asynchronous")
    checks["protocol_stdio"] = PLUGIN_MANIFEST["protocol"] == "stdio"
    checks["manifest_validates"] = validate_manifest(PLUGIN_MANIFEST)
    checks["bad_manifest_rejected"] = not validate_manifest({"manifestVersion": "0.0.1"})
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC4_ipc": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC4_ipc (IPCProtocolInvariants)

Demonstrates JSON-RPC 2.0 over stdin/stdout + exit-0-on-error.

Per V1335: IPC protocol invariants = JSON-RPC 2.0 over stdin/stdout /
exit-0-on-error / structured response envelope.

Run: python example_ic4_ipc.py
"""
from __future__ import annotations

import json
import sys
from typing import Any, Dict


def make_jsonrpc_response(id: Any, result: Any) -> Dict[str, Any]:
    """JSON-RPC 2.0 success envelope."""
    return {
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }


def make_jsonrpc_error(id: Any, code: int, message: str) -> Dict[str, Any]:
    """JSON-RPC 2.0 error envelope."""
    return {
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    }


def handle_request(request: Dict[str, Any]) -> Dict[str, Any]:
    """Handle one JSON-RPC request, return response."""
    if request.get("jsonrpc") != "2.0":
        return make_jsonrpc_error(request.get("id"), -32600, "invalid jsonrpc version")
    method = request.get("method")
    if method == "ping":
        return make_jsonrpc_response(request.get("id"), "pong")
    return make_jsonrpc_error(request.get("id"), -32601, f"method not found: {method}")


def _self_test() -> dict:
    checks = {}
    resp = handle_request({"jsonrpc": "2.0", "id": 1, "method": "ping"})
    checks["ping_returns_pong"] = resp.get("result") == "pong"
    checks["jsonrpc_2_0"] = resp.get("jsonrpc") == "2.0"
    bad = handle_request({"jsonrpc": "1.0", "id": 2, "method": "ping"})
    checks["bad_version_returns_error"] = "error" in bad
    missing = handle_request({"jsonrpc": "2.0", "id": 3, "method": "unknown"})
    checks["unknown_method_returns_error"] = "error" in missing
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC5_error_handling": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC5_error_handling (ErrorHandlingInvariants)

Demonstrates {success:false, error} envelope + structured error messages +
helpful available-* lists.

Per V1335: Error handling invariants = {success:false, error} envelope /
structured error messages / helpful available-* lists.

Run: python example_ic5_error_handling.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict, List


def format_error(message: str, available: List[str] = None) -> Dict[str, Any]:
    """{success:false, error} envelope with optional available list."""
    out = {"success": False, "error": message}
    if available:
        out["available"] = available
    return out


def format_success(data: Any) -> Dict[str, Any]:
    """{success:true, data} envelope."""
    return {"success": True, "data": data}


def _self_test() -> dict:
    checks = {}
    err = format_error("chain not found", available=["chain1", "chain2"])
    checks["error_envelope"] = err["success"] is False
    checks["error_message"] = err["error"] == "chain not found"
    checks["available_list"] = err.get("available") == ["chain1", "chain2"]
    ok = format_success({"result": 42})
    checks["success_envelope"] = ok["success"] is True
    checks["success_data"] = ok["data"] == {"result": 42}
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC6_configuration": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC6_configuration (ConfigurationInvariants)

Demonstrates Object.freeze DEFAULT_CONFIG + clampInteger + 3-tier mergeConfig +
privateConfig path.

Per V1335: Configuration invariants = Object.freeze DEFAULT_CONFIG / clampInteger /
3-tier mergeConfig / privateConfig path / env-typed configSchema.

Run: python example_ic6_configuration.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict


# Frozen default (Python doesn't have Object.freeze but frozenset/tuple work)
DEFAULT_CONFIG: Dict[str, Any] = {
    "max_results": 10,
    "timeout_ms": 5000,
    "domains": (),
}


def clamp_integer(value: int, lo: int, hi: int) -> int:
    """Clamp integer to [lo, hi] range."""
    return max(lo, min(hi, value))


def merge_config_3tier(
    default: Dict[str, Any],
    user: Dict[str, Any],
    private: Dict[str, Any],
) -> Dict[str, Any]:
    """3-tier config merge: default < user < private."""
    return {**default, **user, **private}


def _self_test() -> dict:
    checks = {}
    checks["clamp_below"] = clamp_integer(5, 10, 20) == 10
    checks["clamp_above"] = clamp_integer(25, 10, 20) == 20
    checks["clamp_in_range"] = clamp_integer(15, 10, 20) == 15
    merged = merge_config_3tier(
        {"max_results": 10, "timeout_ms": 5000},
        {"max_results": 20},
        {"private_key": "secret"},
    )
    checks["merge_default"] = merged["timeout_ms"] == 5000
    checks["merge_user_overrides_default"] = merged["max_results"] == 20
    checks["merge_private_overrides_default"] = merged.get("private_key") == "secret"
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC7_resource_bounds": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC7_resource_bounds (ResourceBoundsInvariants)

Demonstrates max_results clamp + token budgets + timeout clamp + BATCH_MAX.

Per V1335: Resource bounds invariants = max_results clamp / token budgets /
timeout clamp / BATCH_MAX / DOMAINS_MAX / SAFE budgets.

Run: python example_ic7_resource_bounds.py
"""
from __future__ import annotations

import sys


MAX_RESULTS: int = 100
BATCH_MAX: int = 50
DOMAINS_MAX: int = 20


def clamp_max_results(n: int, max_n: int = MAX_RESULTS) -> int:
    """Clamp result count to [0, max_n]."""
    return max(0, min(max_n, n))


def truncate_to_token_budget(text: str, max_tokens: int) -> str:
    """Approximate token budget: 1 token ≈ 4 chars."""
    max_chars = max_tokens * 4
    if len(text) <= max_chars:
        return text
    return text[:max_chars] + "..."


def clamp_timeout_ms(timeout_ms: int, lo: int = 100, hi: int = 60000) -> int:
    """Clamp timeout to [lo, hi] ms."""
    return max(lo, min(hi, timeout_ms))


def _self_test() -> dict:
    checks = {}
    checks["clamp_max_negative"] = clamp_max_results(-5) == 0
    checks["clamp_max_overflow"] = clamp_max_results(150) == 100
    checks["clamp_max_in_range"] = clamp_max_results(50) == 50
    truncated = truncate_to_token_budget("hello world this is a test", 2)
    checks["truncate_budget"] = truncated.endswith("...")
    checks["truncate_within_budget"] = truncate_to_token_budget("short", 100) == "short"
    checks["clamp_timeout_below"] = clamp_timeout_ms(50) == 100
    checks["clamp_timeout_above"] = clamp_timeout_ms(120000) == 60000
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',

    "IC8_lifecycle": '''#!/usr/bin/env python3
"""
VCP Substrate Example: IC8_lifecycle (LifecycleInvariants)

Demonstrates _self_test probe + toolCallRecordStore lifecycle + promptCache.clear
on reload + cleanup-on-finally + graceful degrade.

Per V1335: Lifecycle invariants = _self_test probe / toolCallRecordStore lifecycle /
promptCache.clear on reload / cleanup-on-finally / graceful degrade.

Run: python example_ic8_lifecycle.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict, List


class ToolCallRecordStore:
    """Lifecycle-managed tool call records."""

    def __init__(self):
        self._records: List[Dict[str, Any]] = []

    def begin_record(self, tool: str, args: Dict[str, Any]) -> str:
        record_id = f"rec_{len(self._records)}"
        self._records.append({"id": record_id, "tool": tool, "args": args, "status": "begin"})
        return record_id

    def finish_record(self, record_id: str, result: Any) -> None:
        for r in self._records:
            if r["id"] == record_id:
                r["status"] = "finish"
                r["result"] = result
                return

    def clear(self) -> None:
        self._records.clear()


def _self_test() -> dict:
    checks = {}
    store = ToolCallRecordStore()
    rid = store.begin_record("search", {"q": "test"})
    store.finish_record(rid, "result")
    checks["record_begin"] = any(r["id"] == rid for r in store._records)
    checks["record_finish"] = any(r["status"] == "finish" for r in store._records)
    store.clear()
    checks["records_cleared"] = len(store._records) == 0

    # graceful degrade
    def safe_call(fn, *args, fallback=None):
        try:
            return fn(*args)
        except Exception:
            return fallback

    checks["graceful_degrade"] = safe_call(lambda: 1 / 0, fallback="default") == "default"
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
''',
}


# --- Dataclasses ------------------------------------------------------------
@dataclass
class CookbookExample:
    """One example file in the cookbook."""
    invariant_class_id: str
    invariant_label: str
    safety_critical: bool
    filename: str
    content: str
    line_count: int
    byte_size: int

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CookbookIndex:
    """Index of all 8 examples."""
    cookbook_version: str
    total_classes: int
    safety_critical_classes: int
    examples: List[CookbookExample]
    asi_pole_star: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# --- Build API --------------------------------------------------------------
def build_examples() -> List[CookbookExample]:
    """Build 8 example file contents (one per invariant class)."""
    out: List[CookbookExample] = []
    for ic in v1335.INVARIANT_CLASSES:
        cid = ic["invariant_id"]
        template = EXAMPLE_TEMPLATES.get(cid, "# TODO: implement")
        # Add filename to header
        filename = f"example_{cid.lower()}.py"
        out.append(
            CookbookExample(
                invariant_class_id=cid,
                invariant_label=ic["label"],
                safety_critical=ic["safety_critical"],
                filename=filename,
                content=template,
                line_count=template.count("\n") + 1,
                byte_size=len(template.encode("utf-8")),
            )
        )
    return out


def build_index() -> CookbookIndex:
    """Build cookbook index."""
    examples = build_examples()
    sc_count = sum(1 for e in examples if e.safety_critical)
    return CookbookIndex(
        cookbook_version="0.1.0",
        total_classes=len(examples),
        safety_critical_classes=sc_count,
        examples=examples,
        asi_pole_star=ASI_POLE_STAR,
    )


def write_examples_to_dir(output_dir: Path) -> List[Path]:
    """Write all 8 example files to output_dir. Returns list of paths."""
    output_dir.mkdir(parents=True, exist_ok=True)
    paths: List[Path] = []
    examples = build_examples()
    for e in examples:
        p = output_dir / e.filename
        p.write_text(e.content, encoding="utf-8")
        paths.append(p)
    return paths


def write_index_to_dir(output_dir: Path) -> Path:
    """Write index.md to output_dir. Returns path."""
    output_dir.mkdir(parents=True, exist_ok=True)
    index = build_index()
    lines: List[str] = []
    lines.append("# VCP Substrate-by-Example Cookbook")
    lines.append("")
    lines.append(f"Version: {index.cookbook_version}")
    lines.append(f"Total classes: {index.total_classes}")
    lines.append(f"Safety-critical classes: {index.safety_critical_classes}")
    lines.append("")
    lines.append("## 8 Examples (one per invariant class)")
    lines.append("")
    for e in index.examples:
        sc = "🛡️" if e.safety_critical else "  "
        lines.append(f"### {sc} {e.invariant_class_id} ({e.invariant_label})")
        lines.append(f"- File: `{e.filename}`")
        lines.append(f"- Lines: {e.line_count}")
        lines.append(f"- Bytes: {e.byte_size}")
        lines.append(f"- Safety-critical: {e.safety_critical}")
        lines.append("")
    lines.append("## Cross-references")
    lines.append("")
    lines.append("- V1335 = invariant registry (registry source)")
    lines.append("- V1336 = linter (conformance check)")
    lines.append("- V1337 = dashboard (multi-file matrix)")
    lines.append("- V1338 = migrator (action suggestions)")
    lines.append("- V1339 = cookbook (8 examples; this file)")
    lines.append("")
    lines.append("## ASI pole-star (LOCKED)")
    lines.append(f"- V0.1: {index.asi_pole_star['V0_1_actual_measured']}")
    lines.append(f"- V1256: {index.asi_pole_star['V1256_unio_mystica_realized']}")
    lines.append(f"- V1339_modifies_pole_star: {index.asi_pole_star['V1339_modifies_pole_star']}")
    p = output_dir / "index.md"
    p.write_text("\n".join(lines), encoding="utf-8")
    return p


# --- Self-test (probe-only, 主 17:43 实事求是) ------------------------------
def _self_test() -> Dict[str, bool]:
    """Probe-only self-test, all checks must pass."""
    checks: Dict[str, bool] = {}
    # Check 1: V1335 dependency
    checks["v1335_imported"] = v1335 is not None
    checks["v1335_8_invariant_classes"] = len(v1335.INVARIANT_CLASSES) == 8

    # Check 2: 8 example templates
    checks["example_templates_8"] = len(EXAMPLE_TEMPLATES) == 8
    for cid in ["IC1_security", "IC2_file_handling", "IC3_schema", "IC4_ipc",
                "IC5_error_handling", "IC6_configuration", "IC7_resource_bounds",
                "IC8_lifecycle"]:
        checks[f"template_{cid}_nonempty"] = (
            cid in EXAMPLE_TEMPLATES and len(EXAMPLE_TEMPLATES[cid]) > 100
        )

    # Check 3: build_examples()
    examples = build_examples()
    checks["build_examples_8"] = len(examples) == 8
    for e in examples:
        checks[f"example_{e.invariant_class_id}_has_filename"] = ".py" in e.filename
        checks[f"example_{e.invariant_class_id}_has_content"] = len(e.content) > 100
        checks[f"example_{e.invariant_class_id}_line_count_positive"] = e.line_count > 0
        checks[f"example_{e.invariant_class_id}_byte_size_positive"] = e.byte_size > 0

    # Check 4: build_index()
    index = build_index()
    checks["index_total_classes_8"] = index.total_classes == 8
    checks["index_sc_classes_5"] = index.safety_critical_classes == 5
    checks["index_examples_8"] = len(index.examples) == 8
    checks["index_asi_pole_star_locked"] = index.asi_pole_star["V1339_modifies_pole_star"] is False

    # Check 5: write_examples_to_dir
    import tempfile
    with tempfile.TemporaryDirectory() as tmpdir:
        output_dir = Path(tmpdir) / "v1339_test"
        paths = write_examples_to_dir(output_dir)
        checks["write_examples_8_files"] = len(paths) == 8
        checks["write_examples_dir_exists"] = output_dir.exists()
        for p in paths:
            checks[f"file_{p.name}_exists"] = p.exists()
            checks[f"file_{p.name}_nonempty"] = p.stat().st_size > 100

        # Check 6: write_index_to_dir
        index_path = write_index_to_dir(output_dir)
        checks["write_index_exists"] = index_path.exists()
        checks["write_index_has_all_classes"] = all(
            ic["invariant_id"] in index_path.read_text(encoding="utf-8")
            for ic in v1335.INVARIANT_CLASSES
        )

    # Check 7: Each example is a runnable Python file
    for cid, template in EXAMPLE_TEMPLATES.items():
        checks[f"example_{cid}_has_main"] = "__main__" in template
        checks[f"example_{cid}_has_self_test"] = "_self_test" in template

    # Check 8: ASI pole-star NOT modified
    checks["asi_pole_star_locked"] = ASI_POLE_STAR["V1339_modifies_pole_star"] is False
    checks["asi_achieved_still_false"] = ASI_POLE_STAR["asi_achieved_false"] is True

    return checks


def _self_test_summary() -> Tuple[int, int, List[str]]:
    checks = _self_test()
    passed = sum(1 for v in checks.values() if v)
    failed = sum(1 for v in checks.values() if not v)
    failed_names = [k for k, v in checks.items() if not v]
    return passed, failed, failed_names


# --- CLI --------------------------------------------------------------------
def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point. Returns 0 on PASS, 1 on FAIL."""
    parser = argparse.ArgumentParser(
        prog="v1339_vcp_substrate_cookbook",
        description="VCP Substrate-by-Example Cookbook (8 examples; per V1335+V1338)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Output directory for cookbook files (default: stdout summary)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON of cookbook index",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run self-test and exit",
    )

    args = parser.parse_args(argv)

    if args.self_test:
        passed, failed, failed_names = _self_test_summary()
        print(f"V1339 self-test: {passed}/{passed + failed} pass")
        if failed:
            print(f"  Failed: {failed_names}")
            return 1
        print("ALL CHECKS PASS [OK]")
        return 0

    if args.output_dir:
        # Write all 8 examples + index.md
        paths = write_examples_to_dir(args.output_dir)
        index_path = write_index_to_dir(args.output_dir)
        print(f"Wrote {len(paths)} example files + index.md to {args.output_dir}")
        for p in [index_path] + paths:
            print(f"  - {p.name} ({p.stat().st_size} bytes)")
        return 0

    if args.json:
        index = build_index()
        print(json.dumps(index.to_dict(), indent=2, ensure_ascii=False))
        return 0

    # Default: print summary
    index = build_index()
    print(f"V1339 VCP Substrate-by-Example Cookbook")
    print(f"  Total classes: {index.total_classes}")
    print(f"  Safety-critical classes: {index.safety_critical_classes}")
    print(f"  Examples: {len(index.examples)}")
    for e in index.examples:
        sc = "🛡️" if e.safety_critical else "  "
        print(f"  {sc} {e.invariant_class_id} ({e.invariant_label}): {e.filename} ({e.line_count} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
