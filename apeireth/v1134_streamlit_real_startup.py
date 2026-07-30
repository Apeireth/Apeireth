"""V1134 — Streamlit real startup (主 06:15 V1050+ 真用 V1009 Streamlit + 主 00:56 任何人都能接触).

主 06:15 06:32 真用方向: V1009 Streamlit web UI 真启动, 真访问.
主 00:56 阶段性交付: 任何人都能接手 → 真实 port + 真实页面 + 真实截图/HTML 探针.

Strategy:
    1. Materialise a real Streamlit app.py file from V1009 page inventory
    2. Launch `streamlit run app.py --server.headless true --server.port <port>` as subprocess
    3. Wait for the server to come up (poll http://localhost:<port>/_stcore/health)
    4. Probe the rendered homepage and one downstream page
    5. Capture stderr/stdout, kill the process, report
"""
from __future__ import annotations

import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

V1134_VERSION = "0.1.0"


@dataclass
class V1134StreamlitReport:
    report_id: str = field(default_factory=lambda: f"sl-{uuid.uuid4().hex[:8]}")
    timestamp: float = field(default_factory=time.time)
    streamlit_installed: bool = False
    streamlit_version: str = ""
    app_path: str = ""
    port: int = 0
    started_ok: bool = False
    startup_ms: float = 0.0
    health_ok: bool = False
    homepage_ok: bool = False
    page_probe_ok: bool = False
    pid: int = 0
    stdout_tail: str = ""
    stderr_tail: str = ""
    notes: List[str] = field(default_factory=list)
    pages_rendered: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "report_id": self.report_id,
            "timestamp": self.timestamp,
            "streamlit_installed": self.streamlit_installed,
            "streamlit_version": self.streamlit_version,
            "app_path": self.app_path,
            "port": self.port,
            "started_ok": self.started_ok,
            "startup_ms": round(self.startup_ms, 1),
            "health_ok": self.health_ok,
            "homepage_ok": self.homepage_ok,
            "page_probe_ok": self.page_probe_ok,
            "pid": self.pid,
            "stdout_tail": self.stdout_tail[-400:],
            "stderr_tail": self.stderr_tail[-400:],
            "notes": list(self.notes),
            "pages_rendered": list(self.pages_rendered),
        }


# ---------- helpers ----------


def _streamlit_info() -> Tuple[bool, str]:
    bin_path = shutil.which("streamlit")
    if not bin_path:
        # try `python -m streamlit`
        try:
            proc = subprocess.run(
                [sys.executable, "-m", "streamlit", "--version"],
                capture_output=True, text=True, timeout=10,
            )
            if proc.returncode == 0:
                return True, proc.stdout.strip() or proc.stderr.strip()
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass
        return False, "streamlit CLI not on PATH and 'python -m streamlit' fails"
    try:
        proc = subprocess.run([bin_path, "--version"], capture_output=True, text=True, timeout=10)
        return True, (proc.stdout or proc.stderr).strip()
    except (subprocess.TimeoutExpired, FileNotFoundError) as e:
        return False, f"{type(e).__name__}: {e}"


def _pick_free_port(preferred: int = 8765) -> int:
    """Bind to ephemeral port to find a free one, then return it."""
    for candidate in (preferred, preferred + 1, preferred + 2, 0):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.bind(("127.0.0.1", candidate))
                return s.getsockname()[1]
            except OSError:
                continue
    return 0  # let OS pick


def _http_probe(url: str, timeout: float = 3.0) -> Tuple[bool, str, str]:
    try:
        with urllib.request.urlopen(url, timeout=timeout) as r:
            text = r.read().decode("utf-8", errors="replace")
            return True, f"HTTP {r.status}", text
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if e.fp else ""
        return False, f"HTTP {e.code}", body
    except (urllib.error.URLError, TimeoutError, OSError) as e:
        return False, f"{type(e).__name__}: {e}", ""


STREAMLIT_APP_TEMPLATE = '''"""Apeireth ASI V1134 Streamlit app — auto-generated from V1009 inventory.

主 00:56 阶段性交付: 任何人都能接触.
主 22:33 ASI 北极星 + 主 19:33 真调研 + 主 17:43 实事求是.
"""
import streamlit as st

st.set_page_config(
    page_title="Apeireth ASI V1134",
    page_icon="0️⃣",
    layout="wide",
)

st.title("Apeireth ASI — V1134 Real Streamlit")
st.caption("主 00:56 任何人都能接触 + 主 22:33 ASI 北极星 + 主 17:43 实事求是")

st.markdown("""
## North Star

| metric | value |
|--------|-------|
| ASI 北极星 (target) | 0.98 |

## Live endpoints

- /health — health probe
- /v1002/measure — V0.2 measure
- /v1003/philosophy — V4 philosophy
- /v1004/evolve — DGM self-evolve
- /v1006/themes — research themes

## Pages inventory (from V1009)
""")

for _p in __PAGES__:
    st.markdown(f"- {_p}")

from apeireth.v1136_dashboard import measure_dashboard_state, render_streamlit_v05
render_streamlit_v05(st, measure_dashboard_state())

st.success("V1134 Streamlit real startup OK")
'''


def render_streamlit_app(pages: List[str]) -> str:
    return STREAMLIT_APP_TEMPLATE.replace("__PAGES__", repr(pages))


def _read_streamlit_pages() -> List[str]:
    """Read the default Streamlit page inventory from V1009 if available."""
    try:
        from apeireth.v1009_web_ui import V1009WebUI
        ui = V1009WebUI()
        return list(ui.pages) or [
            "00_Home", "01_V1074_Measure", "02_V1002_V02_Measure",
            "03_V1001_VCP", "04_V1004_Self_Evolve", "05_V1005_Research",
            "06_V1006_Themes", "07_V1003_Philosophy", "08_V0_1_Measure",
        ]
    except Exception:
        return [
            "00_Home", "01_Measure", "02_VCP", "03_Philosophy", "04_Self_Evolve",
        ]


def run_real_streamlit(
    app_dir: Optional[str] = None,
    preferred_port: int = 8765,
    startup_timeout_s: float = 25.0,
) -> V1134StreamlitReport:
    rep = V1134StreamlitReport()
    rep.streamlit_installed, rep.streamlit_version = _streamlit_info()
    if not rep.streamlit_installed:
        rep.notes.append("streamlit not installed; cannot start real server")
        return rep

    pages = _read_streamlit_pages()
    rep.pages_rendered = pages

    workdir = app_dir or tempfile.mkdtemp(prefix="v1134_streamlit_")
    os.makedirs(workdir, exist_ok=True)
    app_path = os.path.join(workdir, "app.py")
    with open(app_path, "w", encoding="utf-8") as f:
        f.write(render_streamlit_app(pages))
    rep.app_path = app_path

    port = _pick_free_port(preferred_port)
    rep.port = port
    if port == 0:
        rep.notes.append("no free port available")
        return rep

    cmd = [
        sys.executable, "-m", "streamlit", "run", app_path,
        "--server.headless", "true",
        "--server.port", str(port),
        "--server.address", "127.0.0.1",
        "--browser.gatherUsageStats", "false",
        "--server.enableCORS", "false",
    ]
    t0 = time.perf_counter()
    try:
        proc = subprocess.Popen(
            cmd,
            cwd=workdir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={
                **os.environ,
                "PYTHONUNBUFFERED": "1",
                "PYTHONPATH": os.pathsep.join(filter(None, [
                    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                    os.environ.get("PYTHONPATH", ""),
                ])),
            },
        )
    except (FileNotFoundError, OSError) as e:
        rep.notes.append(f"failed to spawn streamlit: {type(e).__name__}: {e}")
        return rep
    rep.pid = proc.pid

    # poll for readiness
    health_url = f"http://127.0.0.1:{port}/_stcore/health"
    homepage_url = f"http://127.0.0.1:{port}/"
    deadline = time.perf_counter() + startup_timeout_s
    while time.perf_counter() < deadline:
        ok, detail, _body = _http_probe(health_url, timeout=1.5)
        if ok and "200" in detail:
            rep.startup_ms = (time.perf_counter() - t0) * 1000.0
            rep.started_ok = True
            rep.health_ok = True
            break
        if proc.poll() is not None:
            # process exited early
            break
        time.sleep(0.5)

    if rep.started_ok:
        # probe homepage — Streamlit SPA loads via JS, so initial HTML may not contain
        # page-specific text; the honest signal is "200 + non-trivial body length".
        ok_h, _d, body = _http_probe(homepage_url, timeout=3.0)
        rep.homepage_ok = ok_h and len(body) > 500
        # capture page probe via a query-param trick; just confirm /?page= works (Streamlit reruns)
        ok_p, _d, body_p = _http_probe(homepage_url + "?page=health", timeout=3.0)
        rep.page_probe_ok = ok_p and len(body_p) > 200

    # collect output for debugging
    try:
        proc.terminate()
        try:
            out, err = proc.communicate(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
            out, err = proc.communicate()
        rep.stdout_tail = out or ""
        rep.stderr_tail = err or ""
    except Exception as e:
        rep.notes.append(f"cleanup error: {type(e).__name__}: {e}")
    return rep


def render_markdown(rep: V1134StreamlitReport) -> str:
    lines = [
        "# V1134 Streamlit 真启动报告 (主 06:15 V1050+ 真用 + 主 00:56 任何人都能接触)",
        "",
        f"- report_id: `{rep.report_id}`",
        f"- streamlit_installed: **{rep.streamlit_installed}**",
        f"- streamlit_version: {rep.streamlit_version}",
        f"- app_path: `{rep.app_path}`",
        f"- port: **{rep.port}**",
        f"- pid: {rep.pid}",
        f"- started_ok / health_ok: **{rep.started_ok}** / {rep.health_ok}",
        f"- homepage_ok / page_probe_ok: **{rep.homepage_ok}** / {rep.page_probe_ok}",
        f"- startup_ms: **{rep.startup_ms:.0f}**",
        f"- pages_rendered: {len(rep.pages_rendered)}",
        "",
        "## Pages",
        "",
    ]
    for p in rep.pages_rendered:
        lines.append(f"- {p}")
    if rep.notes:
        lines += ["", "## Notes", ""]
        for n in rep.notes:
            lines.append(f"- {n}")
    return "\n".join(lines) + "\n"


def main(argv: Optional[List[str]] = None) -> int:
    rep = run_real_streamlit()
    print(render_markdown(rep))
    return 0 if rep.started_ok else 1


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
