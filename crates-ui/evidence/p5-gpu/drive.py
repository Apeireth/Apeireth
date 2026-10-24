#!/usr/bin/env python3
"""P5 GPU bridge — real-browser verification driver (CDP only).

Drives the **system** Chromium over the DevTools Protocol (no Playwright, no
browser download, no npm install).  Runs the engine on **both** backends,
captures one screenshot per canvas, analyses the pixels and writes a
structured `report.json`.

Usage:
    python3 drive.py [--out <evidence-dir>]

The evidence directory receives:
    webgl.canvas.png / webgpu.canvas.png   — canvas crops
    webgl.page.png    / webgpu.page.png    — full-page screenshots
    console.txt                             — browser console + page errors
    report.json                             — structured verdict
"""
import argparse
import base64
import http.server
import json
import os
import pathlib
import re
import signal
import socketserver
import struct
import subprocess
import sys
import threading
import time
import urllib.request
import zlib

import websocket  # websocket-client

HERE = pathlib.Path(__file__).resolve().parent
CHROME = "/usr/bin/chromium"


# ---------------------------------------------------------------------------
# PNG decoding (no third-party imaging library).
# ---------------------------------------------------------------------------
def read_png(path):
    data = pathlib.Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a png"
    pos = 8
    idat = b""
    width = height = depth = color = None
    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        kind = data[pos + 4 : pos + 8]
        chunk = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color, _, _, _ = struct.unpack(">IIBBBBB", chunk)
        elif kind == b"IDAT":
            idat += chunk
        pos += 12 + length
    assert depth == 8, f"unsupported bit depth {depth}"
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]
    stride = width * channels
    raw = zlib.decompress(idat)
    out = bytearray()
    prev = bytearray(stride)
    p = 0
    for _ in range(height):
        filt = raw[p]
        p += 1
        line = bytearray(raw[p : p + stride])
        p += stride
        if filt == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 255
        elif filt == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif filt == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 255
        elif filt == 4:
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                pp = a + b - c
                pa, pb, pc = abs(pp - a), abs(pp - b), abs(pp - c)
                pred = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pred) & 255
        out += line
        prev = line
    return width, height, channels, bytes(out)


def analyse(path):
    width, height, channels, pixels = read_png(path)
    rgb = bytearray()
    for i in range(0, len(pixels), channels):
        rgb += pixels[i : i + 3]
    total = width * height
    sums = [0, 0, 0]
    non_black = 0
    unique = set()
    for i in range(0, len(rgb), 3):
        r, g, b = rgb[i], rgb[i + 1], rgb[i + 2]
        sums[0] += r
        sums[1] += g
        sums[2] += b
        if r + g + b > 12:
            non_black += 1
        if len(unique) < 4096:
            unique.add((r, g, b))
    return {
        "width": width,
        "height": height,
        "mean_rgb": [round(s / total, 3) for s in sums],
        "non_black_ratio": round(non_black / total, 6),
        "distinct_colors_capped": len(unique),
        "pixels": rgb,
    }


def compare(pa, pb):
    n = min(len(pa), len(pb))
    max_delta = 0
    differing = 0
    for i in range(0, n, 3):
        d = max(abs(pa[i] - pb[i]), abs(pa[i + 1] - pb[i + 1]), abs(pa[i + 2] - pb[i + 2]))
        if d:
            differing += 1
            if d > max_delta:
                max_delta = d
    pixels_compared = n // 3
    return {
        "pixels_compared": pixels_compared,
        "differing_pixels": differing,
        "max_channel_delta": max_delta,
        "identical": differing == 0,
    }


# ---------------------------------------------------------------------------
# Static file server + CDP plumbing.
# ---------------------------------------------------------------------------
def serve(directory):
    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {
            **http.server.SimpleHTTPRequestHandler.extensions_map,
            ".wasm": "application/wasm",
        }

        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(directory), **kw)

        def log_message(self, fmt, *args):
            pass

    class Q(socketserver.TCPServer):
        allow_reuse_address = True

    httpd = Q(("127.0.0.1", 0), Handler)
    port = httpd.server_address[1]
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, port


def wait_for_devtools(port, timeout=30):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=2) as r:
                return json.load(r)
        except Exception:
            time.sleep(0.3)
    raise RuntimeError("devtools endpoint never came up")


def http_json(url, method="GET"):
    req = urllib.request.Request(url, method=method)
    with urllib.request.urlopen(req, timeout=10) as r:
        return json.loads(r.read().decode())


class Cdp:
    def __init__(self, ws_url):
        self.ws = websocket.create_connection(ws_url, timeout=180, origin="http://127.0.0.1")
        self.next_id = 1
        self.events = []
        self.lock = threading.Lock()
        threading.Thread(target=self._reader, daemon=True).start()

    def _reader(self):
        while True:
            try:
                raw = self.ws.recv()
            except Exception:
                return
            try:
                self.events.append(json.loads(raw))
            except Exception:
                continue
            with self.lock:
                pass

    def call(self, method, params=None, timeout=120):
        mid = self.next_id
        self.next_id += 1
        self.ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
        deadline = time.time() + timeout
        while time.time() < deadline:
            with self.lock:
                for message in self.events:
                    if message.get("id") == mid:
                        return message
            time.sleep(0.05)
        raise TimeoutError(f"CDP call {method} timed out")

    def drain(self):
        with self.lock:
            out = list(self.events)
            self.events.clear()
        return out


def element_box(cdp, element_id):
    raw = cdp.call(
        "Runtime.evaluate",
        {
            "expression": (
                "(() => { const el = document.getElementById('%s');"
                " if (!el) return null;"
                " const r = el.getBoundingClientRect();"
                " return JSON.stringify({x:r.left+window.scrollX, y:r.top+window.scrollY,"
                " w:r.width, h:r.height}); })()" % element_id
            ),
            "returnByValue": True,
        },
    )
    value = (raw.get("result", {}).get("result", {}) or {}).get("value")
    return json.loads(value) if value else None


def capture(cdp, path, clip=None):
    params = {"format": "png"}
    if clip:
        params["clip"] = clip
        params["captureBeyondViewport"] = True
    shot = cdp.call("Page.captureScreenshot", params)
    png = base64.b64decode(shot["result"]["data"])
    pathlib.Path(path).write_bytes(png)
    return len(png)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(HERE / "evidence"))
    parser.add_argument("--no-webgpu", action="store_true")
    args = parser.parse_args()
    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    httpd, page_port = serve(HERE)
    url = f"http://127.0.0.1:{page_port}/index.html"

    chrome_args = [
        CHROME,
        "--headless=new",
        "--no-sandbox",
        "--disable-dev-shm-usage",
        "--enable-unsafe-swiftshader",
        "--use-gl=angle",
        "--use-angle=swiftshader",
        "--enable-features=Vulkan",
        # 让这台没有真 GPU 的机器给出**软件** WebGPU adapter，
        # 这样 WebGPU 分支才真的能被跑到。
        "--enable-unsafe-webgpu",
        "--use-webgpu-adapter=swiftshader",
        "--disable-dbus",
        "--no-first-run",
        "--no-default-browser-check",
        "--remote-allow-origins=*",
        "--remote-debugging-port=0",
        "--window-size=1100,760",
        "about:blank",
    ]

    proc = subprocess.Popen(
        chrome_args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, start_new_session=True
    )
    devtools_port = None
    t0 = time.time()
    while time.time() - t0 < 30:
        line = proc.stderr.readline().decode("utf-8", "replace")
        if "DevTools listening on" in line:
            match = re.search(r":(\d+)/devtools", line)
            if match:
                devtools_port = int(match.group(1))
                break
    if devtools_port is None:
        proc.kill()
        raise RuntimeError("chromium did not report a DevTools port")

    console = []
    try:
        wait_for_devtools(devtools_port)
        target = http_json(f"http://127.0.0.1:{devtools_port}/json/new?about:blank", method="PUT")
        cdp = Cdp(target["webSocketDebuggerUrl"])
        cdp.call("Runtime.enable")
        cdp.call("Log.enable")
        cdp.call("Page.enable")
        cdp.call("Page.navigate", {"url": url})

        done = False
        reports = {}
        deadline = time.time() + 180
        while time.time() < deadline:
            for message in cdp.drain():
                if message.get("method") == "Runtime.consoleAPICalled":
                    params = message["params"]
                    text = " ".join(
                        str(a.get("value", a.get("description", ""))) for a in params.get("args", [])
                    )
                    console.append(f"[console.{params.get('type')}] {text}")
                elif message.get("method") == "Runtime.exceptionThrown":
                    details = message["params"].get("exceptionDetails", {})
                    desc = details.get("exception", {}).get("description") or details.get("text")
                    console.append(f"[pageerror] {desc}")
                elif message.get("method") == "Log.entryAdded":
                    entry = message["params"]["entry"]
                    console.append(f"[log.{entry.get('level')}] {entry.get('text')}")

            raw = cdp.call(
                "Runtime.evaluate",
                {
                    "expression": "window.__P5_REPORTS__ ? JSON.stringify(window.__P5_REPORTS__) : null",
                    "returnByValue": True,
                },
            )
            value = (raw.get("result", {}).get("result", {}) or {}).get("value")
            if value:
                reports = json.loads(value)
            flag = cdp.call(
                "Runtime.evaluate", {"expression": "!!window.__P5_DONE__", "returnByValue": True}
            )
            if (flag.get("result", {}).get("result", {}) or {}).get("value"):
                done = True
                break
            time.sleep(0.4)

        # 让合成器把最后一帧贴上画布。
        time.sleep(2.5)

        page_shot = capture(cdp, out_dir / "page.png")
        crops = {}
        for backend, element_id in (("webgl", "webgl"), ("webgpu", "webgpu")):
            box = element_box(cdp, element_id)
            if not box:
                continue
            size = capture(
                cdp,
                out_dir / f"{backend}.canvas.png",
                clip={
                    "x": max(0.0, box["x"]),
                    "y": max(0.0, box["y"]),
                    "width": box["w"],
                    "height": box["h"],
                    "scale": 1,
                },
            )
            crops[backend] = size

        # 逐像素分析。
        analysis = {}
        raw_pixels = {}
        for backend in ("webgl", "webgpu"):
            path = out_dir / f"{backend}.canvas.png"
            if path.exists():
                stats = analyse(path)
                raw_pixels[backend] = stats.pop("pixels")
                analysis[backend] = stats

        parity = None
        if "webgl" in raw_pixels and "webgpu" in raw_pixels:
            parity = compare(raw_pixels["webgl"], raw_pixels["webgpu"])

        console.append("=== reports ===")
        console.append(json.dumps(reports, ensure_ascii=False, indent=2))

        webgl_report = reports.get("webgl", {})
        webgpu_report = reports.get("webgpu", {})
        webgl_stats = analysis.get("webgl", {})
        webgpu_stats = analysis.get("webgpu", {})

        criteria = {
            "both_backends_created": bool(webgl_report) and bool(webgpu_report),
            "webgl_no_error": "error" not in webgl_report,
            "webgpu_no_error": "error" not in webgpu_report,
            "webgl_frames_drawn": webgl_report.get("frames_drawn", 0),
            "webgpu_frames_drawn": webgpu_report.get("frames_drawn", 0),
            "webgl_canvas_not_blank": webgl_stats.get("non_black_ratio", 0) > 0.01,
            "webgpu_canvas_not_blank": webgpu_stats.get("non_black_ratio", 0) > 0.01,
            "webgl_encoding": webgl_report.get("encoding"),
            "webgpu_encoding": webgpu_report.get("encoding"),
            "encoding_parity": webgl_report.get("encoding") == webgpu_report.get("encoding"),
            "backend_pixel_parity": parity,
        }
        criteria["verdict"] = (
            "pass"
            if (
                criteria["both_backends_created"]
                and criteria["webgl_no_error"]
                and criteria["webgpu_no_error"]
                and criteria["webgl_frames_drawn"] > 0
                and criteria["webgpu_frames_drawn"] > 0
                and criteria["webgl_canvas_not_blank"]
                and criteria["webgpu_canvas_not_blank"]
                and criteria["encoding_parity"]
                and parity is not None
                and parity["identical"]
            )
            else "fail"
        )

        chromium_version = subprocess.run(
            [CHROME, "--version"], capture_output=True, text=True
        ).stdout.strip()

        report = {
            "schema": "apeireth.ui-engine.p5-gpu.browser-report/1",
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "environment": {
                "chromium": chromium_version,
                "host_target": "x86_64-unknown-linux-gnu",
                "wasm_target": "wasm32-unknown-unknown",
                "driver": "cdp (websocket-client) — no playwright / no browser download",
            },
            "runs": {
                "webgl": {
                    **{k: v for k, v in webgl_report.items() if k != "error"},
                    "error": webgl_report.get("error"),
                    "canvas_png": "webgl.canvas.png",
                    "canvas_bytes": crops.get("webgl"),
                    "pixels": webgl_stats,
                },
                "webgpu": {
                    **{k: v for k, v in webgpu_report.items() if k != "error"},
                    "error": webgpu_report.get("error"),
                    "canvas_png": "webgpu.canvas.png",
                    "canvas_bytes": crops.get("webgpu"),
                    "pixels": webgpu_stats,
                },
            },
            "criteria": criteria,
            "page_screenshot_bytes": page_shot,
            "browser_console": console,
        }
        (out_dir / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (out_dir / "console.txt").write_text("\n".join(console), encoding="utf-8")

        print(json.dumps({"verdict": criteria["verdict"], "criteria": criteria}, indent=2))
        return 0 if criteria["verdict"] == "pass" else 2
    finally:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            proc.kill()
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main())
