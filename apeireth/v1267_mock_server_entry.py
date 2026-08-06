"""Standalone entry point for V1267 Mock-LLM server subprocess.

Avoids -c escape issues on Windows by using a real script file.
The runner writes a port file under %TEMP%\\v1267_port_<pid>.txt and signals
READY on stderr.
"""
from __future__ import annotations

import argparse
import os
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description="V1267 Mock-LLM server entry")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--latency-jitter-ms", type=float, default=10.0)
    parser.add_argument("--fail-rate", type=float, default=0.0)
    args = parser.parse_args()

    # 确保 UTF-8
    os.environ["PYTHONIOENCODING"] = "utf-8"
    sys.path.insert(0, os.getcwd())

    from apeireth.v1267_asi_local_mock_llm_real_loop import (
        MockLLMServerSpec, serve_blocking,
    )

    spec = MockLLMServerSpec(
        host=args.host,
        port=args.port,
        latency_jitter_ms=args.latency_jitter_ms,
        fail_rate=args.fail_rate,
    )

    def _on_ready(port: int) -> None:
        # 写端口文件 (主 17:58 不假装)
        port_file = os.path.join(
            os.environ.get("TEMP", "/tmp"),
            f"v1267_port_{os.getpid()}.txt",
        )
        try:
            with open(port_file, "w", encoding="utf-8") as f:
                f.write(str(port))
        except OSError as exc:
            sys.stderr.write(f"PORTFILE_ERR: {exc}\n")
        # 真发 ready 信号
        sys.stderr.write("READY\n")
        sys.stderr.write(f"PORT={port}\n")
        sys.stderr.flush()

    serve_blocking(spec, on_ready=_on_ready)
    return 0


if __name__ == "__main__":
    sys.exit(main())
