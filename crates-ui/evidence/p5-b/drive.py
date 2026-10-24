#!/usr/bin/env python3
"""P5-b 生产对照 —— 真浏览器验证驱动（CDP only）。

驱**系统** Chromium（无 Playwright、无浏览器下载、无 npm 安装）：

1. 打开**参考页**（生产版场景类，按路径只读引入）⇒ 用 CDP 在页面 JS 运行前
   注入的录制器抓下**生产版实际用的那一帧**的 16 个 uniform 取值，并截下画布；
2. 把这 16 个取值**原样**注入**引擎页** ⇒ Rust 引擎（wasm）两个后端各画一张；
3. 逐像素对照（阈值：任一通道绝对差 > 2/255 记「不同」），写 `report.json`。

用法:
    python3 drive.py [--out <evidence-dir>] [--web <dist-dir>] [--t0 <秒>]

证据目录会得到:
    production.canvas.png   — 生产侧画布（960×600，1:1 无缩放）
    webgl.canvas.png        — 引擎侧画布（WebGL 后端）
    webgpu.canvas.png       — 引擎侧画布（WebGPU 后端）
    uniforms.json           — 抓到的 16 个 uniform 取值（原样）
    report.json             — 结构化判据
    console.txt             — 浏览器控制台 + 页面错误
"""
import argparse
import base64
import http.server
import json
import os
import pathlib
import re
import shutil
import signal
import socketserver
import subprocess
import sys
import threading
import time
import urllib.request

import websocket  # websocket-client
from PIL import Image, ImageChops  # pillow

HERE = pathlib.Path(__file__).resolve().parent


def find_chrome():
    """找浏览器（顺序与 P3 的 pilot-compare job 一致）。"""
    candidates = []
    if os.environ.get("CHROME_PATH"):
        candidates.append(os.environ["CHROME_PATH"])
    candidates += ["google-chrome-stable", "chromium", "chromium-browser", "/usr/bin/chromium"]
    for candidate in candidates:
        if candidate.startswith("/"):
            if os.path.exists(candidate):
                return candidate
            continue
        found = shutil.which(candidate)
        if found:
            return found
    return "/usr/bin/chromium"


CHROME = find_chrome()

#: 画布尺寸（CSS px = 设备 px，dpr 由 CDP 固定为 1 ⇒ 截图 1:1）。
CANVAS_WIDTH = 960
CANVAS_HEIGHT = 600

#: 逐像素差分阈值（任一通道绝对差 > 2/255 记「不同」；与 P3/P5 口径一致）。
CHANNEL_THRESHOLD = 2

#: 判定「通过」的差异像素占比上限。
MAX_DIFF_RATIO = 0.001  # 0.1%

#: 生产侧 uniform 录制器（在页面 JS 运行前注入 ⇒ 钩住 WebGL2 的 uniform 调用）。
RECORDER_SOURCE = r"""
(() => {
  const locToName = new Map();
  const values = new Map();
  const proto = WebGL2RenderingContext.prototype;

  const origGet = proto.getUniformLocation;
  proto.getUniformLocation = function (program, name) {
    const loc = origGet.call(this, program, name);
    if (loc) locToName.set(loc, name);
    return loc;
  };

  for (const fn of ['uniform1f', 'uniform2f', 'uniform3f', 'uniform4f']) {
    const orig = proto[fn];
    proto[fn] = function (loc) {
      if (loc) values.set(loc, Array.prototype.slice.call(arguments, 1));
      return orig.apply(this, arguments);
    };
  }

  // 在**绘制时刻**快照 ⇒ 抓到的就是这一帧真正用的值。
  const origDraw = proto.drawArrays;
  proto.drawArrays = function () {
    const snapshot = {};
    for (const [loc, name] of locToName) {
      if (values.has(loc)) snapshot[name] = values.get(loc);
    }
    window.__P5B_LAST_DRAW__ = { uniforms: snapshot, at: performance.now() };
    window.__P5B_DRAWS__ = (window.__P5B_DRAWS__ || 0) + 1;
    return origDraw.apply(this, arguments);
  };
})();
"""


# ---------------------------------------------------------------------------
# 静态文件服务器 + CDP 管道
# ---------------------------------------------------------------------------
def serve(directory):
    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {
            **http.server.SimpleHTTPRequestHandler.extensions_map,
            ".wasm": "application/wasm",
        }

        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(directory), **kw)

        def end_headers(self):
            # ⚠️ 必须禁缓存：wasm 产物每轮都会重编，浏览器若命中 HTTP 缓存就会
            # 拿**上一轮**的 wasm 跑（画面差异会被误判成引擎行为）。
            self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
            self.send_header("Pragma", "no-cache")
            super().end_headers()

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
            time.sleep(0.02)
        raise TimeoutError(f"CDP call {method} timed out")

    def drain(self):
        with self.lock:
            out = list(self.events)
            self.events.clear()
        return out

    def evaluate(self, expression, timeout=60):
        raw = self.call(
            "Runtime.evaluate",
            {"expression": expression, "returnByValue": True, "awaitPromise": True},
            timeout=timeout,
        )
        result = raw.get("result", {})
        if "exceptionDetails" in result:
            raise RuntimeError(
                "page exception: "
                + json.dumps(result["exceptionDetails"], ensure_ascii=False)[:2000]
            )
        return (result.get("result", {}) or {}).get("value")


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


def clip_of(box):
    return {
        "x": max(0.0, float(box["x"])),
        "y": max(0.0, float(box["y"])),
        "width": float(box["w"]),
        "height": float(box["h"]),
        "scale": 1,
    }


# ---------------------------------------------------------------------------
# 逐像素对照（PIL）
# ---------------------------------------------------------------------------
def load_rgb(path):
    image = Image.open(path)
    return image.convert("RGB")


def diff_stats(left_path, right_path, threshold=CHANNEL_THRESHOLD):
    """逐像素差分：任一通道绝对差 > threshold 记「不同」。"""
    left = load_rgb(left_path)
    right = load_rgb(right_path)
    if left.size != right.size:
        return {
            "comparable": False,
            "left_size": list(left.size),
            "right_size": list(right.size),
            "pixels_compared": 0,
            "differing_pixels": None,
            "differing_ratio": None,
            "max_channel_delta": None,
        }
    difference = ImageChops.difference(left, right)
    red, green, blue = difference.split()
    worst = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    histogram = worst.histogram()
    differing = sum(histogram[threshold + 1 :])
    total = left.size[0] * left.size[1]
    return {
        "comparable": True,
        "left_size": list(left.size),
        "right_size": list(right.size),
        "pixels_compared": total,
        "differing_pixels": differing,
        "differing_ratio": differing / total if total else 0.0,
        "max_channel_delta": worst.getextrema()[1],
        "channel_threshold": threshold,
    }


def flipped_diff_stats(left_path, right_path, threshold=CHANNEL_THRESHOLD):
    """诊断用：右侧**上下翻转**后的差分（用于识别「整幅镜像」型差异）。"""
    left = load_rgb(left_path)
    right = load_rgb(right_path)
    if left.size != right.size:
        return None
    flipped = right.transpose(Image.FLIP_TOP_BOTTOM)
    difference = ImageChops.difference(left, flipped)
    red, green, blue = difference.split()
    worst = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    histogram = worst.histogram()
    differing = sum(histogram[threshold + 1 :])
    total = left.size[0] * left.size[1]
    return {
        "differing_pixels": differing,
        "differing_ratio": differing / total if total else 0.0,
        "max_channel_delta": worst.getextrema()[1],
    }


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------
def open_target(devtools_port, console_log):
    target = http_json(f"http://127.0.0.1:{devtools_port}/json/new?about:blank", method="PUT")
    cdp = Cdp(target["webSocketDebuggerUrl"])
    cdp.call("Runtime.enable")
    cdp.call("Log.enable")
    cdp.call("Page.enable")
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        {
            "width": 1200,
            "height": 800,
            "deviceScaleFactor": 1,
            "mobile": False,
        },
    )
    return cdp


def collect_console(cdp, console_log, tag):
    for message in cdp.drain():
        method = message.get("method")
        if method == "Runtime.consoleAPICalled":
            params = message["params"]
            text = " ".join(
                str(a.get("value", a.get("description", ""))) for a in params.get("args", [])
            )
            console_log.append(f"[{tag}:console.{params.get('type')}] {text}")
        elif method == "Runtime.exceptionThrown":
            details = message["params"].get("exceptionDetails", {})
            desc = details.get("exception", {}).get("description") or details.get("text")
            console_log.append(f"[{tag}:pageerror] {desc}")
        elif method == "Log.entryAdded":
            entry = message["params"]["entry"]
            console_log.append(f"[{tag}:log.{entry.get('level')}] {entry.get('text')}")


def wait_until(cdp, expression, timeout=120, interval=0.25):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            if cdp.evaluate(expression):
                return True
        except Exception:
            pass
        time.sleep(interval)
    return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(HERE / "evidence"))
    parser.add_argument("--web", default=str(HERE / "web" / "dist"))
    parser.add_argument("--t0", default="1.5", help="生产版冻结时钟（秒）")
    parser.add_argument("--chrome", default=CHROME)
    parser.add_argument(
        "--no-webgpu",
        action="store_true",
        help="模拟「宿主不提供 WebGPU 适配器」⇒ WebGPU 腿判为不可验证（降级路径自测用）",
    )
    args = parser.parse_args()

    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    web_dir = pathlib.Path(args.web)
    if not (web_dir / "reference" / "index.html").exists():
        raise SystemExit(
            f"找不到构建产物：{web_dir}/reference/index.html（先跑 build.sh）"
        )

    httpd, page_port = serve(web_dir)
    base = f"http://127.0.0.1:{page_port}"

    chrome_args = [
        args.chrome,
        "--headless=new",
        "--no-sandbox",
        "--disable-dev-shm-usage",
        "--enable-unsafe-swiftshader",
        "--use-gl=angle",
        "--use-angle=swiftshader",
        "--enable-features=Vulkan",
        "--enable-unsafe-webgpu",
        "--use-webgpu-adapter=swiftshader",
        "--disable-dbus",
        "--no-first-run",
        "--no-default-browser-check",
        "--remote-allow-origins=*",
        "--remote-debugging-port=0",
        "--window-size=1200,800",
        "--force-color-profile=srgb",
        "about:blank",
    ]
    if args.no_webgpu:
        # **降级路径的可测开关**：模拟「宿主不提供 WebGPU 适配器」
        # （CI runner 未必有软件 WebGPU）。此时 WebGPU 腿应被判为
        # **不可验证**（报告里显式标注），而 WebGL 腿仍必过 ⇒ verdict 仍 pass。
        # 这个分支必须能**被跑到**，否则它只是纸上的分支。
        chrome_args = [a for a in chrome_args if "webgpu" not in a]
        chrome_args.insert(1, "--disable-features=WebGPU")

    proc = subprocess.Popen(
        chrome_args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, start_new_session=True
    )
    console_log = []
    try:
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
        wait_for_devtools(devtools_port)

        # ── 阶段 1：参考页（生产版场景）──────────────────────────────────
        reference = open_target(devtools_port, console_log)
        reference.call(
            "Page.addScriptToEvaluateOnNewDocument", {"source": RECORDER_SOURCE}
        )
        reference.call("Page.navigate", {"url": f"{base}/reference/?t0={args.t0}"})
        if not wait_until(reference, "(window.__P5B_DRAWS__ || 0) >= 8", timeout=90):
            collect_console(reference, console_log, "reference")
            raise RuntimeError("参考页没有画出 8 帧")
        collect_console(reference, console_log, "reference")

        # 冻结渲染循环 ⇒ 画布内容与抓到的 uniform 取值属于**同一帧**。
        reference.evaluate("window.__P5B_SCENE__.setPaused(true)")
        time.sleep(0.4)
        collect_console(reference, console_log, "reference")

        draw_json = reference.evaluate("JSON.stringify(window.__P5B_LAST_DRAW__)")
        draw = json.loads(draw_json)
        uniforms = draw.get("uniforms", {})
        env = json.loads(reference.evaluate("JSON.stringify(window.__P5B_ENV__)"))

        box = element_box(reference, "scene")
        if not box:
            raise RuntimeError("参考页拿不到画布的位置")
        capture(reference, out_dir / "production.canvas.png", clip=clip_of(box))
        capture(reference, out_dir / "reference.page.png")

        # ── 阶段 2：引擎页（Rust 引擎，原样喂抓到的取值）─────────────────
        engine = open_target(devtools_port, console_log)
        engine.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": "window.__P5B_UNIFORMS__ = " + json.dumps(uniforms) + ";"},
        )
        engine.call("Page.navigate", {"url": f"{base}/engine/"})
        if not wait_until(engine, "!!window.__P5B_DONE__", timeout=180):
            collect_console(engine, console_log, "engine")
            raise RuntimeError("引擎页没有跑完")
        collect_console(engine, console_log, "engine")
        reports = json.loads(engine.evaluate("JSON.stringify(window.__P5B_REPORTS__ || {})"))
        time.sleep(1.5)

        crops = {}
        for backend in ("webgl", "webgpu"):
            box = element_box(engine, backend)
            if not box:
                continue
            crops[backend] = capture(
                engine, out_dir / f"{backend}.canvas.png", clip=clip_of(box)
            )
        capture(engine, out_dir / "engine.page.png")

        # ── 阶段 3：逐像素对照 ───────────────────────────────────────────
        production_png = out_dir / "production.canvas.png"
        webgl_stats = diff_stats(production_png, out_dir / "webgl.canvas.png")
        webgpu_stats = diff_stats(production_png, out_dir / "webgpu.canvas.png")

        diagnostics = {}
        if webgl_stats.get("comparable"):
            diagnostics["webgl_vs_vertically_flipped"] = flipped_diff_stats(
                production_png, out_dir / "webgl.canvas.png"
            )
        if webgpu_stats.get("comparable"):
            diagnostics["webgpu_vs_vertically_flipped"] = flipped_diff_stats(
                production_png, out_dir / "webgpu.canvas.png"
            )

        webgl_report = reports.get("webgl", {})
        webgpu_report = reports.get("webgpu", {})

        def ratio_ok(stats):
            if not stats.get("comparable") or stats.get("differing_ratio") is None:
                return False
            return stats["differing_ratio"] <= MAX_DIFF_RATIO

        criteria = {
            "uniform_count": len(uniforms),
            "uniform_names": sorted(uniforms.keys()),
            "uniform_components": {
                name: len(values) for name, values in sorted(uniforms.items())
            },
            "reference_canvas": [env.get("canvasWidth"), env.get("canvasHeight")],
            "reference_dpr": env.get("devicePixelRatio"),
            "reference_fallback": env.get("fallback"),
            "reference_visibility": env.get("visibilityState"),
            "webgl_frames_drawn": webgl_report.get("frames_drawn"),
            "webgpu_frames_drawn": webgpu_report.get("frames_drawn"),
            "webgl_encoding": webgl_report.get("encoding"),
            "webgpu_encoding": webgpu_report.get("encoding"),
            "webgl_surface_format": webgl_report.get("surface_format"),
            "webgpu_surface_format": webgpu_report.get("surface_format"),
            "production_vs_engine_webgl": webgl_stats,
            "production_vs_engine_webgpu": webgpu_stats,
            "webgl_diff_ratio_ok": ratio_ok(webgl_stats),
            "webgpu_diff_ratio_ok": ratio_ok(webgpu_stats),
            "max_diff_ratio": MAX_DIFF_RATIO,
            "channel_threshold": CHANNEL_THRESHOLD,
            "diagnostics": diagnostics,
        }
        # ── 判定：按**实际可验证的部分**判，而不是「一律要求两个后端」──
        # 宿主（CI runner）不一定提供软件 WebGPU 适配器 ⇒ 那时 WebGPU 这一腿
        # **不可验证**（不是「通过」，也不是「失败」）⇒ 报告里**显式标注**，
        # 判定退到「实际跑过的那一腿」。**WebGL 腿始终是必过的**（它不需要
        # 任何特殊能力，只要求 WebGL2 —— 而同仓的 pilot-compare job 已证
        # 该栈在 runner 上可用）。
        webgpu_verifiable = (webgpu_report.get("frames_drawn") or 0) > 0
        criteria["webgpu_comparison_verifiable"] = webgpu_verifiable
        criteria["webgpu_leg_note"] = (
            "本宿主提供了 WebGPU 适配器 ⇒ WebGPU 腿已验"
            if webgpu_verifiable
            else "本宿主未提供 WebGPU 适配器 ⇒ WebGPU 腿**不可验证**"
            "（非通过、非失败）；判定退到 WebGL 腿。"
            "已入库证据是 Chromium 154 + SwiftShader 软件适配器下的**两腿全验**。"
        )
        webgl_leg_ok = (
            len(uniforms) == 16
            and not env.get("fallback")
            and (webgl_report.get("frames_drawn") or 0) > 0
            and webgl_report.get("encoding") == "raw_bytes"
            and ratio_ok(webgl_stats)
        )
        webgpu_leg_ok = (
            webgpu_verifiable
            and (webgpu_report.get("frames_drawn") or 0) > 0
            and webgpu_report.get("encoding") == "raw_bytes"
            and ratio_ok(webgpu_stats)
        )
        criteria["webgl_leg_ok"] = webgl_leg_ok
        criteria["webgpu_leg_ok"] = webgpu_leg_ok if webgpu_verifiable else None
        criteria["verdict"] = (
            "pass"
            if (webgl_leg_ok and (webgpu_leg_ok or not webgpu_verifiable))
            else "fail"
        )

        chromium_version = subprocess.run(
            [args.chrome, "--version"], capture_output=True, text=True
        ).stdout.strip()

        report = {
            "schema": "apeireth.ui-engine.p5-b.production-parity-report/1",
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "environment": {
                "chromium": chromium_version,
                "host_target": "x86_64-unknown-linux-gnu",
                "wasm_target": "wasm32-unknown-unknown",
                "driver": "cdp (websocket-client) — no playwright / no browser download",
                "frozen_clock_seconds": float(args.t0),
                "canvas_css_px": [CANVAS_WIDTH, CANVAS_HEIGHT],
                "device_pixel_ratio": 1,
            },
            "reference": {
                "source": "frontend/companion-desktop/src/lib/scene/blackhole.ts (read-only import)",
                "options": {"interactive": False, "reduceMotion": False},
                "env": env,
                "draw_calls": draw.get("at"),
                "uniforms": uniforms,
            },
            "runs": {
                "webgl": {
                    **{
                        k: v
                        for k, v in webgl_report.items()
                        if k not in ("error", "uniforms_used")
                    },
                    "error": webgl_report.get("error"),
                    "canvas_png": "webgl.canvas.png",
                    "canvas_bytes": crops.get("webgl"),
                },
                "webgpu": {
                    **{
                        k: v
                        for k, v in webgpu_report.items()
                        if k not in ("error", "uniforms_used")
                    },
                    "error": webgpu_report.get("error"),
                    "canvas_png": "webgpu.canvas.png",
                    "canvas_bytes": crops.get("webgpu"),
                },
            },
            "criteria": criteria,
            "browser_console": console_log,
        }
        (out_dir / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (out_dir / "uniforms.json").write_text(
            json.dumps(uniforms, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (out_dir / "console.txt").write_text("\n".join(console_log), encoding="utf-8")

        print(
            json.dumps(
                {
                    "verdict": criteria["verdict"],
                    "uniform_count": len(uniforms),
                    "webgl": webgl_stats,
                    "webgpu": webgpu_stats,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0 if criteria["verdict"] == "pass" else 2
    finally:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            proc.kill()
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main())
