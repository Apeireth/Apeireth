#!/usr/bin/env python3
"""Fix leftover class-body self.N=0 lines in v121_vcp_eight_plugins.py."""
from pathlib import Path

p = Path("apeireth/v121_vcp_eight_plugins.py")
src = p.read_text(encoding="utf-8")

# Lines that should be inside __init__ only, not at class body.
# Before: __init__, then naked "self.nph = 0\n    self.nas = 0", then register().
broken = (
    "    def __init__(self):\n"
    "        self.n = 0\n"
    "        self.nph = 0\n"
    "        self.nas = 0\n"
    "\n"
    "    self.nph = 0\n"
    "    self.nas = 0\n"
    "    def register(self, name, types):"
)
fixed = (
    "    def __init__(self):\n"
    "        self.n = 0\n"
    "        self.nph = 0\n"
    "        self.nas = 0\n"
    "\n"
    "    def register(self, name, types):"
)
if broken in src:
    src = src.replace(broken, fixed)
    p.write_text(src, encoding="utf-8")
    print("Removed class-body self.*=0 lines.")
else:
    print("Pattern not found - file may already be fixed.")
