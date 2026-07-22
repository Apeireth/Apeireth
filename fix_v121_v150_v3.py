#!/usr/bin/env python3
"""V121-V150 test compatibility: add missing test-expected APIs (round 3).
主 17:43 实事求是 — fix broken, 不假装已 pass. 主 23:44 干到底.
"""
from pathlib import Path

ROOT = Path("apeireth")


def patch(path: Path, old: str, new: str) -> bool:
    if not path.exists():
        print(f"  [skip] missing: {path}")
        return False
    src = path.read_text(encoding="utf-8")
    if old not in src:
        print(f"  [skip] pattern not found in {path.name}")
        return False
    src2 = src.replace(old, new, 1)
    path.write_text(src2, encoding="utf-8")
    print(f"  [ok]   patched {path.name}")
    return True


print("=== V121-V150 backward-compat fixes (round 3) ===")

# V134: dispatch_round_robin (alias for dispatch)
patch(
    ROOT / "v134_load_balancer.py",
    "    def dispatch(self):\n"
    "        if not self.backends:\n"
    "            return None\n",
    "    def dispatch(self):\n"
    "        if not self.backends:\n"
    "            return None\n"
    "    def dispatch_round_robin(self):\n"
    "        \"\"\"Test compat: alias for dispatch().\"\"\"\n"
    "        return self.dispatch()\n",
)

# V136: should_retry() with default attempt=0
patch(
    ROOT / "v136_retry_strategy.py",
    "    def should_retry(self, attempt):\n        return attempt < self.max_attempts",
    "    def should_retry(self, attempt: int = 0) -> bool:\n        return attempt < self.max_attempts",
)

# V139: stats['n']
patch(
    ROOT / "v139_debounce.py",
    "    def stats(self):\n"
    "        return {\"delay_seconds\": self.delay_seconds, \"n_calls\": self.n,",
    "    def stats(self):\n"
    "        d = {\"delay_seconds\": self.delay_seconds, \"n_calls\": self.n,",
)
patch(
    ROOT / "v139_debounce.py",
    "                \"n_keys\": len(self.calls), \"version\": V139_VERSION,\n"
    "                \"philosophy\": \"V139 debounce",
    "                \"n_keys\": len(self.calls), \"version\": V139_VERSION,\n"
    "                \"n\": self.n, \"philosophy\": \"V139 debounce",
)

# V143: stats['logged']
patch(
    ROOT / "v143_async_logger.py",
    "    def stats(self):\n"
    "        return {\"buffer_size\": self.buffer_size,\n"
    "                \"current\": len(self.buffer), \"n_logs\": self.n,",
    "    def stats(self):\n"
    "        d = {\"buffer_size\": self.buffer_size,\n"
    "                \"current\": len(self.buffer), \"n_logs\": self.n,",
)
patch(
    ROOT / "v143_async_logger.py",
    "                \"version\": V143_VERSION,\n"
    "                \"philosophy\": \"V143 async logger",
    "                \"logged\": self.n, \"version\": V143_VERSION,\n"
    "                \"philosophy\": \"V143 async logger",
)

print("\n=== Done. Run pytest to verify. ===")