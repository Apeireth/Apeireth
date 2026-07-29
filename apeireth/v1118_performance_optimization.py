"""Compatibility entry point for the canonical V1118 optimizer implementation.

Kept because R9-PO-002 names both ``v1118_performance_optimization`` and
``v1118_perf_optimizer_v01``.  The implementation has a single source of truth.
"""

from apeireth.v1118_perf_optimizer_v01 import *  # noqa: F401,F403
from apeireth.v1118_perf_optimizer_v01 import _cli


if __name__ == "__main__":
    raise SystemExit(_cli())
