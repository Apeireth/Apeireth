"""apeireth.v1437 — handler-mode shim.

When this module is invoked via ``python -m apeireth.v1437 --handler``, it
runs the V1437 subprocess HTTP server handler. Otherwise it dispatches to
the full V1437 CLI in ``apeireth.v1437_asi_subprocess_http_live_server``.
"""

from __future__ import annotations

import sys

from apeireth.v1437_asi_subprocess_http_live_server import main


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))