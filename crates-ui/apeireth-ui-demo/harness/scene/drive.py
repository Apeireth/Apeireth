#!/usr/bin/env python3
"""场景层语义对照 —— 真浏览器验证驱动（CDP only）。

对每个用例：

1. 打开**参考页**（生产版场景类，按路径只读引入）⇒ 用 CDP 在页面 JS 运行前
   注入的录制器抓下**生产版实际用的那一帧**的 16 个 uniform 取值 + 帧时长序列，
   并截下画布；
2. 打开**引擎页** ⇒ Rust 场景层语义 crate（wasm）**自己算出**同一组取值，
   喂给引擎画到画布上；
3. **逐值**比较两侧的 16 个取值（T1），再**逐像素**比较两张画布（T2）。

用法:
    python3 drive.py [--out <evidence-dir>] [--web <dist-dir>] [--cases <cases.json>]

证据目录会得到:
    report.json          — 结构化判据（逐用例的 T1 / T2 结果）
    case-<id>-reference.png / case-<id>-engine-<backend>.png
    console.txt          — 浏览器控制台 + 页面错误
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
import struct
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
CANVAS_WIDTH = 1440
CANVAS_HEIGHT = 900

#: 逐像素差分阈值（任一通道绝对差 > 2/255 记「不同」；与 P3/P5 口径一致）。
CHANNEL_THRESHOLD = 2

#: 判定「通过」的差异像素占比上限。
MAX_DIFF_RATIO = 0.001  # 0.1%

#: 逐值比较的容差（浮点按位相等优先；否则用这个上限）。
VALUE_TOLERANCE = 1e-6

#: 帧时长上限（秒）—— 与生产侧的帧时长钳位同值。
DT_CLAMP_SEC = 0.05

#: 生产侧 uniform 录制器（在页面 JS 运行前注入 ⇒ 钩住 WebGL2 的 uniform 调用、
#: rAF 时间戳与起始时钟）。
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
  };

  // 帧时长序列：rAF 时间戳（生产侧的 `now`）。
  window.__SCENE_RAF__ = [];
  const origRaf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = function (cb) {
    return origRaf(function (ts) {
      window.__SCENE_RAF__.push(ts);
      return cb(ts);
    });
  };

  // 起始时钟（生产侧 `startLoop` 里的 `performance.now()` ⇒ 第一帧的 lastT）。
  window.__SCENE_PERF__ = [];
  const origNow = performance.now.bind(performance);
  performance.now = function () {
    const value = origNow();
    window.__SCENE_PERF__.push(value);
    return value;
  };

  // 在**绘制时刻**快照 ⇒ 抓到的就是这一帧真正用的值。
  const origDraw = proto.drawArrays;
  proto.drawArrays = function () {
    const snapshot = {};
    for (const [loc, name] of locToName) {
      if (values.has(loc)) snapshot[name] = values.get(loc);
    }
    window.__SCENE_DRAWS__ = (window.__SCENE_DRAWS__ || 0) + 1;
    window.__SCENE_LAST_DRAW__ = { uniforms: snapshot, draws: window.__SCENE_DRAWS__ };
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


def to_f32(value):
    """f64 → f32（IEEE 单精度，round-to-nearest-even）—— 与 WebGL 的转换同口径。"""
    return struct.unpack("f", struct.pack("f", value))[0]


def compare_values(production, engine):
    """逐值比较 16 个取值（f64 口径 + f32 口径）。"""
    rows = []
    exact = 0
    within = 0
    f32_exact = 0
    for name in sorted(production):
        left = production[name]
        right = engine.get(name)
        if right is None:
            rows.append({"name": name, "status": "missing_on_engine"})
            continue
        deltas = [abs(a - b) for a, b in zip(left, right)]
        worst = max(deltas) if deltas else 0.0
        bitwise = all(a == b for a, b in zip(left, right))
        if bitwise:
            status = "bitwise_equal"
            exact += 1
            within += 1
        elif worst <= VALUE_TOLERANCE:
            status = "within_tolerance"
            within += 1
        else:
            status = "differs"
        left32 = [to_f32(v) for v in left]
        right32 = [to_f32(v) for v in right]
        f32_bitwise = all(a == b for a, b in zip(left32, right32))
        if f32_bitwise:
            f32_exact += 1
        rows.append(
            {
                "name": name,
                "status": status,
                "production": left,
                "engine": right,
                "max_abs_delta": worst,
                "production_f32": left32,
                "engine_f32": right32,
                "f32_bitwise_equal": f32_bitwise,
            }
        )
    return {
        "uniform_count": len(production),
        "bitwise_equal": exact,
        "within_tolerance": within,
        "f32_bitwise_equal": f32_exact,
        "tolerance": VALUE_TOLERANCE,
        "rows": rows,
    }


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------
def open_target(devtools_port):
    target = http_json(f"http://127.0.0.1:{devtools_port}/json/new?about:blank", method="PUT")
    cdp = Cdp(target["webSocketDebuggerUrl"])
    cdp.call("Runtime.enable")
    cdp.call("Log.enable")
    cdp.call("Page.enable")
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        {
            "width": 1600,
            "height": 1000,
            "deviceScaleFactor": 1,
            "mobile": False,
        },
    )
    cdp.target_id = target.get("id")
    return cdp


def close_target(devtools_port, cdp):
    try:
        cdp.ws.close()
    except Exception:
        pass
    target_id = getattr(cdp, "target_id", None)
    if target_id:
        try:
            http_json(f"http://127.0.0.1:{devtools_port}/json/close/{target_id}")
        except Exception:
            pass


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


def wait_until(cdp, expression, timeout=120, interval=0.2):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            if cdp.evaluate(expression):
                return True
        except Exception:
            pass
        time.sleep(interval)
    return False


def case_query(case):
    """用例 → 参考页 / 引擎页的查询串。"""
    params = {
        "path": case["path"],
        "mode": case["mode"],
        "camera": str(case.get("camera", 0)),
    }
    if case.get("hour") is not None:
        params["hour"] = repr(float(case["hour"]))
    if case.get("t0") is not None:
        params["t0"] = repr(float(case["t0"]))
    pad = case.get("pad")
    if pad is not None:
        params["pad"] = ",".join(repr(float(v)) for v in pad)
        params["padP"] = repr(float(pad[0]))
        params["padA"] = repr(float(pad[1]))
        params["padD"] = repr(float(pad[2]))
    if case.get("intensity") is not None:
        params["intensity"] = repr(float(case["intensity"]))
    return params


def query_string(params):
    from urllib.parse import urlencode

    return urlencode(params)


def frame_deltas(cdp):
    """由录制器抓到的 rAF 时间戳 + 起始时钟 ⇒ 帧时长序列（与生产侧同算式）。"""
    raw = cdp.evaluate(
        "JSON.stringify({raf: window.__SCENE_RAF__ || [], perf: window.__SCENE_PERF__ || []})"
    )
    data = json.loads(raw)
    raf = data.get("raf") or []
    perf = data.get("perf") or []
    if not raf:
        return [], None
    start = perf[0] if perf else raf[0]
    deltas = []
    previous = start
    for timestamp in raf:
        delta = min((timestamp - previous) / 1000.0, DT_CLAMP_SEC)
        deltas.append(delta)
        previous = timestamp
    return deltas, start


def camera_channels_stable(cdp, previous):
    """相机通道是否与上一次读数一致（插值走完 ⇒ 机位不再变化）。"""
    raw = cdp.evaluate(
        "JSON.stringify((window.__SCENE_LAST_DRAW__ || {}).uniforms || {})"
    )
    uniforms = json.loads(raw)
    channels = ["uScale", "uIncl", "uAzim", "uDiskAzim", "uDiskIncl"]
    current = {name: uniforms.get(name) for name in channels}
    if previous is None:
        return False, current
    return current == previous, current


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(HERE.parent / "evidence" / "scene"))
    parser.add_argument("--web", default=str(HERE / "web" / "dist"))
    parser.add_argument("--cases", default=str(HERE / "cases.json"))
    parser.add_argument("--chrome", default=CHROME)
    parser.add_argument("--only", default=None, help="只跑 id 含该子串的用例")
    args = parser.parse_args()

    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    web_dir = pathlib.Path(args.web)
    if not (web_dir / "reference" / "index.html").exists():
        raise SystemExit(
            f"找不到构建产物：{web_dir}/reference/index.html（先跑 build.sh）"
        )
    cases = json.loads(pathlib.Path(args.cases).read_text(encoding="utf-8"))
    if args.only:
        cases = [c for c in cases if args.only in c["id"]]

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
        "--window-size=1600,1000",
        "--force-color-profile=srgb",
        "about:blank",
    ]

    # ⚠️ 时区固定为 UTC（进程环境变量，不是浏览器开关）⇒ 抓取时刻不随时区漂移。
    chrome_env = dict(os.environ)
    chrome_env["TZ"] = "UTC"

    proc = subprocess.Popen(
        chrome_args,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        start_new_session=True,
        env=chrome_env,
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

        results = []
        for case in cases:
            case_id = case["id"]
            print(f"── 用例 {case_id} ──", flush=True)
            params = case_query(case)

            # ── 阶段 1：参考页（生产版场景）──────────────────────────────────
            reference = open_target(devtools_port)
            reference.call(
                "Page.addScriptToEvaluateOnNewDocument", {"source": RECORDER_SOURCE}
            )
            reference.call(
                "Page.navigate",
                {"url": f"{base}/reference/?{query_string(params)}"},
            )
            if not wait_until(reference, "!!window.__SCENE_ENV__", timeout=120):
                collect_console(reference, console_log, "reference")
                raise RuntimeError(f"{case_id}: 参考页没跑起来")
            env = json.loads(reference.evaluate("JSON.stringify(window.__SCENE_ENV__)"))

            deltas = []
            if case["path"] == "animated":
                # 等机位插值走完（相机通道稳定）⇒ 抓到的就是最终状态。
                previous = None
                stable = False
                deadline = time.time() + 180
                while time.time() < deadline:
                    stable, current = camera_channels_stable(reference, previous)
                    previous = current
                    if stable:
                        break
                    time.sleep(0.25)
                if not stable:
                    raise RuntimeError(f"{case_id}: 机位插值没走完")
                # 冻结渲染循环 ⇒ 画布内容与抓到的取值属于**同一帧**。
                reference.evaluate("window.__SCENE_SCENE__.setPaused(true)")
                time.sleep(0.4)
            else:
                time.sleep(0.3)

            draw = json.loads(reference.evaluate("JSON.stringify(window.__SCENE_LAST_DRAW__)"))
            production_uniforms = draw.get("uniforms", {})
            deltas, clock_start = frame_deltas(reference)
            collect_console(reference, console_log, "reference")

            box = element_box(reference, "scene")
            if not box:
                raise RuntimeError(f"{case_id}: 参考页拿不到画布的位置")
            capture(reference, out_dir / f"case-{case_id}-reference.png", clip=clip_of(box))
            close_target(devtools_port, reference)

            # ── 阶段 2：引擎页（Rust 场景语义 crate 自己算取值）─────────────
            engine = open_target(devtools_port)
            engine.call(
                "Page.addScriptToEvaluateOnNewDocument",
                {"source": "window.__SCENE_DTS__ = " + json.dumps(deltas) + ";"},
            )
            engine.call(
                "Page.navigate",
                {"url": f"{base}/engine/?{query_string(params)}"},
            )
            if not wait_until(engine, "!!window.__SCENE_DONE__", timeout=240):
                collect_console(engine, console_log, "engine")
                raise RuntimeError(f"{case_id}: 引擎页没跑完")
            reports = json.loads(engine.evaluate("JSON.stringify(window.__SCENE_REPORTS__ || {})"))
            time.sleep(1.5)
            collect_console(engine, console_log, "engine")

            crops = {}
            for backend in ("webgl", "webgpu"):
                box = element_box(engine, backend)
                if not box:
                    continue
                crops[backend] = capture(
                    engine, out_dir / f"case-{case_id}-engine-{backend}.png", clip=clip_of(box)
                )
            close_target(devtools_port, engine)

            # ── 阶段 3：逐值 + 逐像素对照 ───────────────────────────────────
            webgl_report = reports.get("webgl", {})
            if webgl_report.get("error"):
                raise RuntimeError(f"{case_id}: 引擎页 webgl 腿失败：{webgl_report['error']}")
            engine_uniforms = webgl_report.get("uniforms", {})

            value_cmp = compare_values(production_uniforms, engine_uniforms)

            reference_png = out_dir / f"case-{case_id}-reference.png"
            pixel_cmp = {}
            for backend in ("webgl", "webgpu"):
                engine_png = out_dir / f"case-{case_id}-engine-{backend}.png"
                if not engine_png.exists():
                    continue
                pixel_cmp[backend] = diff_stats(reference_png, engine_png)

            webgl_pixels = pixel_cmp.get("webgl", {})
            webgpu_pixels = pixel_cmp.get("webgpu", {})
            webgpu_verifiable = bool(webgpu_pixels.get("comparable")) and (
                reports.get("webgpu", {}).get("frames_drawn") or 0
            ) > 0

            def ratio_ok(stats):
                if not stats.get("comparable") or stats.get("differing_ratio") is None:
                    return False
                return stats["differing_ratio"] <= MAX_DIFF_RATIO

            t1_ok = value_cmp["bitwise_equal"] == value_cmp["uniform_count"] == 16
            t1_tolerance_ok = value_cmp["within_tolerance"] == value_cmp["uniform_count"] == 16
            t2_ok = ratio_ok(webgl_pixels) and (
                not webgpu_verifiable or ratio_ok(webgpu_pixels)
            )
            record = {
                "id": case_id,
                "case": case,
                "reference": {
                    "source": "frontend/companion-desktop/src/lib/scene/blackhole.ts (read-only import)",
                    "env": env,
                    "draws": draw.get("draws"),
                    "uniforms": production_uniforms,
                    "frame_deltas_count": len(deltas),
                    "frame_deltas": deltas,
                    "clock_start_ms": clock_start,
                },
                "engine": {
                    "backend_actual": webgl_report.get("backend_actual"),
                    "encoding": webgl_report.get("encoding"),
                    "surface_format": webgl_report.get("surface_format"),
                    "frames_drawn": webgl_report.get("frames_drawn"),
                    "scene_frames": webgl_report.get("scene_frames"),
                    "uniforms": engine_uniforms,
                    "uniforms_f32": webgl_report.get("uniforms_f32"),
                    "webgpu": {
                        "backend_actual": reports.get("webgpu", {}).get("backend_actual"),
                        "frames_drawn": reports.get("webgpu", {}).get("frames_drawn"),
                        "error": reports.get("webgpu", {}).get("error"),
                    },
                },
                "t1_values": value_cmp,
                "t1_ok": t1_ok,
                "t1_within_tolerance_ok": t1_tolerance_ok,
                "t2_pixels": pixel_cmp,
                "t2_ok": t2_ok,
                "canvas_png_bytes": crops,
            }
            results.append(record)
            print(
                f"   T1: {value_cmp['bitwise_equal']}/{value_cmp['uniform_count']} 逐位相等"
                f"（f32 {value_cmp['f32_bitwise_equal']}/{value_cmp['uniform_count']}）"
                f" | T2 webgl 差异像素 {webgl_pixels.get('differing_pixels')}"
                f"/{webgl_pixels.get('pixels_compared')}",
                flush=True,
            )

        chromium_version = subprocess.run(
            [args.chrome, "--version"], capture_output=True, text=True
        ).stdout.strip()

        total = len(results)
        t1_pass = sum(1 for r in results if r["t1_ok"])
        t1_tolerance_pass = sum(1 for r in results if r["t1_within_tolerance_ok"])
        t2_pass = sum(1 for r in results if r["t2_ok"])
        report = {
            "schema": "apeireth.ui-engine.scene-parity-report/1",
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "environment": {
                "chromium": chromium_version,
                "timezone": "UTC",
                "device_pixel_ratio": 1,
                "canvas_css_px": [CANVAS_WIDTH, CANVAS_HEIGHT],
                "driver": "cdp (websocket-client) — no playwright / no browser download",
                "value_tolerance": VALUE_TOLERANCE,
                "channel_threshold": CHANNEL_THRESHOLD,
                "max_diff_ratio": MAX_DIFF_RATIO,
            },
            "criteria": {
                "case_count": total,
                "t1_bitwise_equal_cases": t1_pass,
                "t1_within_tolerance_cases": t1_tolerance_pass,
                "t2_pixel_equal_cases": t2_pass,
                "verdict": "pass"
                if (t1_tolerance_pass == total and t2_pass == total and total > 0)
                else "fail",
            },
            "cases": results,
            "browser_console": console_log,
        }
        (out_dir / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (out_dir / "console.txt").write_text("\n".join(console_log), encoding="utf-8")
        print(json.dumps(report["criteria"], ensure_ascii=False, indent=2))
        return 0 if report["criteria"]["verdict"] == "pass" else 2
    finally:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            proc.kill()
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main())
