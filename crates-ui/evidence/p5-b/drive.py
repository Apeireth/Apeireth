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


def crop_stats(left_path, right_path, crop_box, threshold=CHANNEL_THRESHOLD):
    """差分（**左侧先裁剪**）：用于「同一场景、不同画布尺寸」的对照。

    场景着色器是 `gl_FragCoord` 的**纯函数**（几何全在设备像素里）⇒
    ⇒ 画布从 960×600 缩到 720×600 时，画面应当是**原图的左侧 720 列**
    （= **裁剪**，不是**缩放**）⇒ ⇒ 「裁剪 vs 引擎实拍」逐像素差 = 0
    ⇒ ⇒ 这一条同时证明「尺寸真的生效」与「画面没有被拉伸」。
    """
    left = load_rgb(left_path).crop(crop_box)
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


def squeeze_stats(source_path, right_path, size, threshold=CHANNEL_THRESHOLD):
    """**反事实**对照：把源图**缩放**到 `size` 再与右侧比。

    用途：证明「resize 腿的判据是**灵敏**的」—— 若引擎**没有**跟上尺寸
    （表面仍是旧尺寸 ⇒ 浏览器把旧画面**缩放**到新画布）⇒ 差分必然**很大**；
    ⇒ 与「裁剪 = 0 差异」并列 ⇒ ⇒ 0 差异不是因为「判据不灵敏」。
    """
    source = load_rgb(source_path).resize(size, Image.LANCZOS)
    right = load_rgb(right_path)
    if source.size != right.size:
        return {"comparable": False, "left_size": list(source.size), "right_size": list(right.size)}
    difference = ImageChops.difference(source, right)
    red, green, blue = difference.split()
    worst = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    histogram = worst.histogram()
    differing = sum(histogram[threshold + 1 :])
    total = source.size[0] * source.size[1]
    return {
        "comparable": True,
        "left_size": list(source.size),
        "right_size": list(right.size),
        "pixels_compared": total,
        "differing_pixels": differing,
        "differing_ratio": differing / total if total else 0.0,
        "max_channel_delta": worst.getextrema()[1],
        "channel_threshold": threshold,
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
        # ⚠️ **刻意等一拍**（2026-10-25 加）：给浏览器时间发生一次合成
        # ⇒ 如果引擎侧是「画完就停」，那次合成会把 drawing buffer 清空
        # ⇒ 之后的截图可能全白。
        # ⇒ 引擎页已改为**持续重绘**（画面恒定）⇒ 这一等**不改变结果**。
        # ⚠️ **诚实边界**：⚠️ **本机检不出**（实测：把重绘关掉，本机仍 0 差异
        # ⇒ 本机 Chrome 与 runner 的版本/时序不同）⇒ ⇒ 这一等只是
        # **降低**时序敏感出现的概率，**不是**回归保护；真正的保证是
        # **持续重绘**本身（drawing buffer 永远有内容 ⇒ 与时序无关）。
        time.sleep(2.0)
        for backend in ("webgl", "webgpu"):
            box = element_box(engine, backend)
            if not box:
                continue
            # ⭐ 截图前**开启该后端的持续重绘** ⇒ drawing buffer 永远有内容
            # ⇒ ⇒ 截图**对时序不敏感**（否则「画完就停」⇒ 合成清空 buffer ⇒ 截图全白
            # ⇒ ⇒ 被误判为「引擎画错」；2026-10-25 CI 实测 99.94% 差异）。
            engine.evaluate(f"window.__P5B_REDRAW__.start({json.dumps(backend)}); true")
            time.sleep(0.4)
            crops[backend] = capture(
                engine, out_dir / f"{backend}.canvas.png", clip=clip_of(box)
            )
            # 截完立即停 ⇒ 两个后端**不会**同时占着 context（runner 上资源紧）。
            engine.evaluate(f"window.__P5B_REDRAW__.stop({json.dumps(backend)}); true")
        capture(engine, out_dir / "engine.page.png")

        # ── 阶段 2′：⭐ resize **自侦测**腿（引擎必须**自己**跟上画布尺寸）────
        #
        # 探针**只**改画布的 `width/height`（**不调**任何引擎 resize 接口
        # ⇒ 引擎页里根本没有这种接口）⇒ ⇒ 引擎能跟上**只能**靠**引擎自己**
        # 侦测（`EngineOptions::canvas_size` ⇒ 每帧比对）。
        #
        # 尺寸取 **960×600 → 720×600**（**比例不同** ⇒ 构图变化可测）。
        #
        # ⚠ **两个后端各跑一遍**：WebGPU 后端改 `canvas.width/height` 会**作废
        #   交换链** ⇒ ⇒ 这一腿同时验「交换链作废后能**自动**恢复」。
        resize_leg = {"skipped": True, "backends": {}}
        for resize_backend in ("webgl", "webgpu"):
            try:
                box = element_box(engine, resize_backend)
                if not box:
                    continue
                # ① 持续重绘（drawing buffer 永远有内容 ⇒ 截图对时序不敏感）。
                engine.evaluate(
                    f"window.__P5B_REDRAW__.start({json.dumps(resize_backend)}); true"
                )
                time.sleep(0.5)
                before_state = engine.evaluate(
                    f"window.__P5B_PROBE__.surfaceState({json.dumps(resize_backend)})"
                )
                capture(
                    engine,
                    out_dir / f"resize.{resize_backend}.before.png",
                    clip=clip_of(box),
                )

                # ② 探针改画布尺寸（比例不同：960×600 → 720×600）。
                engine.evaluate(
                    f"window.__P5B_PROBE__.resizeCanvas({json.dumps(resize_backend)}, 720, 600); true"
                )
                # ③ 等引擎至少画一帧（自侦测发生在 `draw_frame` 里）。
                time.sleep(1.0)
                after_state = engine.evaluate(
                    f"window.__P5B_PROBE__.surfaceState({json.dumps(resize_backend)})"
                )
                box_after = element_box(engine, resize_backend)
                capture(
                    engine,
                    out_dir / f"resize.{resize_backend}.after.png",
                    clip=clip_of(box_after),
                )
                engine.evaluate(
                    f"window.__P5B_REDRAW__.stop({json.dumps(resize_backend)}); true"
                )
                resize_leg["backends"][resize_backend] = {
                    "skipped": False,
                    "backend": resize_backend,
                    "canvas_before": [960, 600],
                    "canvas_after": [720, 600],
                    "surface_before": before_state,
                    "surface_after": after_state,
                }
            except Exception as error:  # noqa: BLE001 —— 驱动层：失败要**显式**记下来
                resize_leg["backends"][resize_backend] = {
                    "skipped": True,
                    "error": str(error),
                }
                collect_console(engine, console_log, "engine")
        resize_leg["skipped"] = not resize_leg["backends"]

        # ── 阶段 3：逐像素对照 ───────────────────────────────────────────
        production_png = out_dir / "production.canvas.png"
        webgl_stats = diff_stats(production_png, out_dir / "webgl.canvas.png")
        webgpu_stats = diff_stats(production_png, out_dir / "webgpu.canvas.png")

        # ⭐ 引擎两个后端的互比（**必须同尺寸才可比** ⇒ 不然记为不可比）。
        engine_webgl_png = out_dir / "webgl.canvas.png"
        engine_webgpu_png = out_dir / "webgpu.canvas.png"
        if engine_webgl_png.exists() and engine_webgpu_png.exists():
            engine_backend_parity = diff_stats(engine_webgl_png, engine_webgpu_png)
        else:
            engine_backend_parity = {"comparable": False, "reason": "缺少某后端的画布"}

        diagnostics = {}
        if webgl_stats.get("comparable"):
            diagnostics["webgl_vs_vertically_flipped"] = flipped_diff_stats(
                production_png, out_dir / "webgl.canvas.png"
            )
        if webgpu_stats.get("comparable"):
            diagnostics["webgpu_vs_vertically_flipped"] = flipped_diff_stats(
                production_png, out_dir / "webgpu.canvas.png"
            )

        # ── 阶段 3′：resize 腿的逐像素判据 ──────────────────────────────
        #
        # 场景着色器是 `gl_FragCoord` 的**纯函数**（几何全在设备像素里）⇒
        # ⇒ 画布从 960×600 变成 720×600 时，画面应当是**原图左侧 720 列**
        # （= **裁剪**，不是**缩放**）⇒ ⇒ 判据：
        #   ① 「生产图裁剪 720 列」 vs 「引擎 resize 后的实拍」 ⇒ **0 差异**
        #      ⇒ 同时证明「尺寸真的生效」**与**「画面没有被拉伸」；
        #   ② 「resize 前的实拍裁剪 720 列」 vs 「resize 后的实拍」 ⇒ **0 差异**
        #      ⇒ 证明画面是**裁剪**而非**缩放**（构图真的随尺寸变了）；
        #   ③ **反事实**：把生产图**缩放**到 720×600 再比 ⇒ 差异**必须很大**
        #      ⇒ 证明判据①是**灵敏**的（0 差异不是因为「怎么比都一样」）。
        for resize_backend, leg in resize_leg.get("backends", {}).items():
            if leg.get("skipped"):
                continue
            after_png = out_dir / f"resize.{resize_backend}.after.png"
            before_png = out_dir / f"resize.{resize_backend}.before.png"
            leg["pixel_diff"] = {
                "production_crop_vs_engine_after": crop_stats(
                    production_png, after_png, (0, 0, 720, 600)
                ),
                "before_crop_vs_engine_after": crop_stats(
                    before_png, after_png, (0, 0, 720, 600)
                ),
                "counterfactual_squeezed_production_vs_engine_after": squeeze_stats(
                    production_png, after_png, (720, 600)
                ),
            }

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
            # ⭐⭐ **引擎双后端逐像素一致**（2026-10-26 新增）。
            #
            # ## 为什么要有这条
            #
            # 「同一份逻辑在 WebGL 与 WebGPU 上产出**同一张画面**」是**引擎自己的**
            # 硬要求 ⇒ ⇒ 否则「选后端」就等于「选画面」⇒ ⇒ 宿主没法信任任何一腿。
            #
            # ⭐ 实测依据：intro 场景曾出现「引擎 webgl vs 引擎 webgpu 差 **84.5%**」
            # ⇒ ⇒ 根因是点图元只在 GLES 可用（WebGPU 缺 `gl_PointSize`）
            # ⇒ ⇒ 引入实例化四边形后两条路径**互证一致**（仅差 15 像素）
            # ⇒ ⇒ ⇒ 这条判据就是为了**把那个能力缺口钉住**，不许复发。
            "engine_webgl_vs_engine_webgpu": engine_backend_parity,
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

        # ── resize **自侦测**腿的判据 ────────────────────────────────────
        #
        # ① 引擎**自己**把表面改到了新尺寸（宿主**没有**调任何 resize 接口）；
        # ② 表面**代号** +1（⇒ 表面确实被重新配置过 ⇒ 交换链重建）；
        # ③ 自动 resize 计数 +1（⇒ 是**自动**路径，不是宿主显式调用）；
        # ④ 画面 = 生产图**裁剪** 720 列（0 差异）⇒ 新尺寸**真的生效**，
        #    且画面**没有被拉伸**。
        #
        # ⚠ **逐后端判定**：某个后端不可验证（宿主没给适配器 / 探针失败）
        #   ⇒ 那一腿**不可验证**（非通过、非失败）⇒ 判定退到**实际跑过的那腿**。
        resize_leg_criteria = {}
        resize_ok = True
        resize_verified_any = False
        for resize_backend, leg in resize_leg.get("backends", {}).items():
            if leg.get("skipped"):
                resize_leg_criteria[resize_backend] = {
                    "verifiable": False,
                    "note": "该后端的 resize 腿未跑成 ⇒ **不可验证**（非通过、非失败）",
                    "error": leg.get("error"),
                }
                continue
            before_state = leg.get("surface_before", {})
            after_state = leg.get("surface_after", {})
            pixel = leg.get("pixel_diff", {})
            crop_diff = pixel.get("production_crop_vs_engine_after", {})
            counterfactual = pixel.get(
                "counterfactual_squeezed_production_vs_engine_after", {}
            )
            resize_surface_ok = (
                after_state.get("surface_width") == 720
                and after_state.get("surface_height") == 600
            )
            resize_generation_ok = (
                int(after_state.get("surface_generation", 0))
                > int(before_state.get("surface_generation", 0))
            )
            resize_auto_ok = int(after_state.get("auto_resize_count", 0)) > int(
                before_state.get("auto_resize_count", 0)
            )
            resize_pixels_ok = bool(
                crop_diff.get("comparable") and crop_diff.get("differing_pixels") == 0
            )
            # 反事实必须**显著不同**（否则判据不灵敏 ⇒ 0 差异没有意义）。
            counterfactual_sensitive = bool(
                counterfactual.get("comparable")
                and (counterfactual.get("differing_ratio") or 0.0) > 0.01
            )
            leg_ok = bool(
                resize_surface_ok
                and resize_generation_ok
                and resize_auto_ok
                and resize_pixels_ok
                and counterfactual_sensitive
            )
            resize_ok = resize_ok and leg_ok
            resize_verified_any = True
            resize_leg_criteria[resize_backend] = {
                "verifiable": True,
                "canvas_before": leg.get("canvas_before"),
                "canvas_after": leg.get("canvas_after"),
                "surface_before": before_state,
                "surface_after": after_state,
                "surface_size_followed": resize_surface_ok,
                "surface_generation_advanced": resize_generation_ok,
                "auto_resize_count_advanced": resize_auto_ok,
                "pixels_are_a_crop_not_a_stretch": resize_pixels_ok,
                "counterfactual_is_sensitive": counterfactual_sensitive,
                "pixel_diff": pixel,
                "ok": leg_ok,
            }
        if not resize_verified_any:
            # 一腿都没验成 ⇒ resize 腿**不可验证**（不判失败，也不算通过）。
            resize_ok = True
            resize_leg_criteria["note"] = (
                "resize 腿**没有任何后端**跑成 ⇒ **不可验证**（非通过、非失败）"
            )
        criteria["resize_leg"] = {
            "backends": resize_leg_criteria,
            "ok": resize_ok,
            "verified_any": resize_verified_any,
        }
        # ⭐⭐ **引擎双后端一致**也要过（**两腿都可验证时**才判）。
        #
        # ⚠ 若宿主只提供 WebGL（没有 WebGPU 适配器）⇒ ⇒ 双后端互比**不可验证**
        # ⇒ ⇒ 那时**不参与**判定（而不是假装通过）。
        backend_parity_comparable = bool(engine_backend_parity.get("comparable"))
        backend_parity_ok = (
            not backend_parity_comparable
            or (engine_backend_parity.get("differing_pixels") or 0) == 0
        )
        criteria["engine_backend_parity_comparable"] = backend_parity_comparable
        criteria["engine_backend_parity_ok"] = backend_parity_ok
        criteria["engine_backend_parity_note"] = (
            "引擎 WebGL 与 WebGPU 的画布**逐像素相同**（0 差异）⇒ "
            "⇒ 同一份逻辑在两个后端产出同一画面 ✔"
            if backend_parity_comparable and backend_parity_ok
            else (
                "两后端画布**不可比**（缺某后端产物）⇒ ⇒ 该判据**不可验证**"
                "（非通过、非失败）"
                if not backend_parity_comparable
                else "两后端画布**存在像素差异** ⇒ ⇒ 引擎的后端一致性**被破坏** ✗"
            )
        )

        criteria["verdict"] = (
            "pass"
            if (
                webgl_leg_ok
                and (webgpu_leg_ok or not webgpu_verifiable)
                and resize_ok
                and backend_parity_ok
            )
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
            "resize_leg": resize_leg,
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
                    "resize_leg": criteria.get("resize_leg"),
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
