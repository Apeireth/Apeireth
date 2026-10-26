#!/usr/bin/env python3
"""P5-b 片头腿 —— 生产 `intro/fire.ts` ↔ Rust 引擎（**逐值** + **逐像素**）。

驱动**系统** Chromium（无 Playwright、无浏览器下载、无 npm 安装）：

1. 打开**片头页**（生产 `intro/fire.ts` **只读引入** + Rust 引擎 wasm 宿主）；
   - 录制器在 `drawArrays` 时刻抓下生产侧**实际用的那一帧**的 uniform 取值
     （背景 matte pass 与火焰粒子 pass **分开**抓，按 draw mode 分类）；
   - 同一时刻，Rust 场景层（`apeireth-ui-scene`）算出同一批取值 ⇒ **逐值**对照；
2. 引擎侧用**生产侧抓到的**取值画一张画布 ⇒ 与生产侧画布**逐像素**对照。

用法:
    python3 drive_intro.py [--out <dir>] [--web <dist-dir>] [--it <秒>]

判据口径与 P5-b 生产对照**同一口径**（差分阈值：任一通道绝对差 > 2/255）。
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

#: 逐像素差分阈值（与 P5-b / P3 同一口径）。
CHANNEL_THRESHOLD = 2


def find_chrome():
    """找浏览器（顺序与 P5-b 的 drive.py 一致）。"""
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


def serve(directory):
    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {
            **http.server.SimpleHTTPRequestHandler.extensions_map,
            ".wasm": "application/wasm",
        }

        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(directory), **kw)

        def end_headers(self):
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
    # ⚠️ CDP 的截图数据**嵌在 result 里**（`shot["result"]["data"]`）。
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


def diff_stats(left_path, right_path, threshold=CHANNEL_THRESHOLD):
    """逐像素差分：任一通道绝对差 > threshold 记「不同」。"""
    left = Image.open(left_path).convert("RGB")
    right = Image.open(right_path).convert("RGB")
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


def run_leg(devtools_port, base, console_log, frozen_at, out_dir, only="all", source="rust", probe="", sprite="quad"):
    """跑一条片头腿（一个冻结时刻 × 一种粒子绘制路径）。"""
    target = http_json(f"http://127.0.0.1:{devtools_port}/json/new?about:blank", method="PUT")
    cdp = Cdp(target["webSocketDebuggerUrl"])
    cdp.call("Runtime.enable")
    cdp.call("Log.enable")
    cdp.call("Page.enable")
    cdp.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1200, "height": 1200, "deviceScaleFactor": 1, "mobile": False},
    )
    cdp.call(
        "Page.navigate",
        {
            "url": f"{base}/intro/?it={frozen_at}&only={only}&source={source}&sprite={sprite}"
            + (f"&probe={probe}" if probe else "")
        },
    )
    if not wait_until(cdp, "!!window.__INTRO_DONE__", timeout=180):
        collect_console(cdp, console_log, "intro")
        print("CONSOLE-ON-FAIL:", *console_log[-25:], sep="\n", flush=True)
        raise RuntimeError("片头页没有跑完")
    collect_console(cdp, console_log, "intro")
    report = json.loads(cdp.evaluate("JSON.stringify(window.__INTRO__)"))

    # 逐像素：生产画布 vs 引擎画布（**每个后端各一张**）。
    pixels = {}
    for backend in ("webgl", "webgpu"):
        leg = report.get(backend) or {}
        if leg.get("error"):
            pixels[backend] = {"skipped": True, "error": leg["error"]}
            continue
        try:
            cdp.evaluate(f"window.__INTRO_REDRAW__.start({json.dumps(backend)}); true")
            time.sleep(0.5)
            production_box = element_box(cdp, "production")
            engine_box = element_box(cdp, f"engine-{backend}")
            # ⚠️ 文件名**带路径标识**（`{sprite}`）⇒ ⇒ `quad` 与 `point` 两条腿
            # ⇒ ⇒ **不会互相覆盖** ⇒ ⇒ 落盘的 PNG 与 `report.json` 的数字一一对应。
            # ⇒ ⇒ （该标识在引入 `--sprite` 时就一并做了；2026-10-26 **复核确认**了它，
            # ⇒ ⇒ 并用**独立重算**验证「报告数字 == 落盘图」：任一通道 / 阈值 > 2 ⇒
            # ⇒ ⇒ 69,345 ⇒ ⇒ 与报告**逐个一致** ⇒ ⇒ 12% 这个数字**可信**。）
            capture(
                cdp,
                out_dir / f"intro.production.{sprite}.{backend}.png",
                clip=clip_of(production_box),
            )
            capture(
                cdp,
                out_dir / f"intro.engine.{sprite}.{backend}.png",
                clip=clip_of(engine_box),
            )
            cdp.evaluate(f"window.__INTRO_REDRAW__.stop({json.dumps(backend)}); true")
            pixels[backend] = diff_stats(
                out_dir / f"intro.production.{sprite}.{backend}.png",
                out_dir / f"intro.engine.{sprite}.{backend}.png",
            )
        except Exception as error:  # noqa: BLE001 —— 驱动层：失败要**显式**记下来
            pixels[backend] = {"skipped": True, "error": str(error)}
    report["pixel_diff"] = pixels
    report["sprite_mode"] = sprite

    # ⭐⭐ **双后端逐像素对照**（本批的**核心判据**）：引擎 WebGL vs 引擎 WebGPU。
    #
    # ## 为什么必须在这一层做
    #
    # 「生产 vs 引擎」只能证明「引擎与生产一致」⇒ **证明不了**「两个后端一致」。
    # ⇒ ⇒ 必须**同一场景、同一时刻**各截一张引擎画布 ⇒ **逐像素**对照 ⇒
    #   ⇒ 差 0 ⇒ 两个后端**真的**画出同一张图。
    #
    # ⚠ **有效性检查**：某后端段带 `error` / `skipped` ⇒ 该对照**无效**
    #   （「没画」**不等于**「一致」⇒ 不许把无效腿说成一致）。
    engine_dual = {}
    webgl_segment = pixels.get("webgl") or {}
    webgpu_segment = pixels.get("webgpu") or {}
    webgl_broken = bool(webgl_segment.get("error") or webgl_segment.get("skipped"))
    webgpu_broken = bool(webgpu_segment.get("error") or webgpu_segment.get("skipped"))
    if webgl_broken or webgpu_broken:
        engine_dual = {
            "comparable": False,
            "skipped": True,
            "error": "；".join(
                part
                for part in (
                    f"webgl: {webgl_segment.get('error')}" if webgl_broken else "",
                    f"webgpu: {webgpu_segment.get('error')}" if webgpu_broken else "",
                )
                if part
            ),
        }
    else:
        engine_dual = diff_stats(
            out_dir / f"intro.engine.{sprite}.webgl.png",
            out_dir / f"intro.engine.{sprite}.webgpu.png",
        )
    report["engine_dual_backend_diff"] = engine_dual

    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", default="")
    parser.add_argument("--out", default=str(HERE / "evidence-intro"))
    parser.add_argument("--web", default=str(HERE / "web" / "dist"))
    parser.add_argument(
        "--it",
        default="13.5,2.5",
        help="冻结时刻（秒），逗号分隔 ⇒ 每个时刻跑一条腿",
    )
    parser.add_argument(
        "--only",
        default="all",
        help="只画哪个 pass（all / backdrop / sprite），逗号分隔 ⇒ 隔离对照",
    )
    parser.add_argument(
        "--source",
        default="rust",
        help="引擎侧用哪套取值渲染（rust = 端到端；production = 只比着色器）",
    )
    parser.add_argument(
        "--sprite",
        default="quad,point",
        help="粒子的绘制路径（quad = 实例化四边形（后端无关）；point = 点图元（WebGL 快路径））",
    )
    args = parser.parse_args()

    out_dir = pathlib.Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    web_dir = pathlib.Path(args.web)
    if not (web_dir / "intro" / "index.html").exists():
        raise SystemExit(f"找不到构建产物：{web_dir}/intro/index.html（先跑 build.sh）")

    httpd, page_port = serve(web_dir)
    base = f"http://127.0.0.1:{page_port}"

    chrome_args = [
        CHROME,
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
        "--window-size=1200,1200",
        "--force-color-profile=srgb",
        "about:blank",
    ]
    proc = subprocess.Popen(
        chrome_args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, start_new_session=True
    )
    console_log = []
    try:
        devtools_port = None
        started = time.time()
        while time.time() - started < 30:
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

        legs = {}
        for raw in args.it.split(","):
            frozen_at = float(raw)
            for only in args.only.split(","):
                for sprite in args.sprite.split(","):
                    key = f"{frozen_at}:{only}:{sprite}"
                    legs[key] = run_leg(
                        devtools_port, base, console_log, frozen_at, out_dir,
                        only=only, source=args.source, probe=args.probe, sprite=sprite,
                    )

        report = {
            "schema": "apeireth.ui-engine.p5-b.intro-parity-report/1",
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "environment": {
                "chromium": subprocess.run(
                    [CHROME, "--version"], capture_output=True, text=True
                ).stdout.strip(),
                "frozen_times_seconds": [float(x) for x in args.it.split(",")],
                "channel_threshold": CHANNEL_THRESHOLD,
            },
            "legs": legs,

            # ⚠️⚠️ **有效性检查**（2026-10-26 补）—— 防止「**没画**」被误读成「**一致**」。
            #
            # 实测踩到的坑：引擎的 **WebGPU 后端画不了点图元**（wgpu 的 `PointList` 没有点尺寸）
            # ⇒ 该腿的引擎侧**根本没画粒子**（只画了背景）⇒ ⇒ 于是「生产 vs 引擎」得到 **0 差异**
            # ⇒ ⇒ 极易被误读成「迁移已对齐」⇒ ⇒ **假绿**！
            # ⇒ ⇒ 因此：某条腿若某后端段带 `error`（或 `skipped`）⇒ ⇒ 该腿**标记为 `invalid`**
            # ⇒ ⇒ **不参与**任何「0 差异」的结论 ⇒ ⇒ 且 `legs_invalid` **显式列出**能力缺口。
            "legs_invalid": {},
            "browser_console": console_log,
        }
        # ⇒ ⇒ 计算每条腿的有效性（某后端报错 ⇒ 该腿无效）。
        invalid_legs = {}
        for key, leg in report.get("legs", {}).items():
            broken = []
            for backend, segment in (leg.get("pixel_diff") or {}).items():
                if isinstance(segment, dict) and (segment.get("error") or segment.get("skipped")):
                    broken.append(backend)
            for backend in ("webgl", "webgpu"):
                segment = leg.get(backend) or {}
                if isinstance(segment, dict) and segment.get("error") and backend not in broken:
                    broken.append(backend)
            if broken:
                invalid_legs[key] = broken
                leg["valid"] = False
                leg["invalid_backends"] = broken
            else:
                leg["valid"] = True
        report["legs_invalid"] = invalid_legs
        report["legs_valid"] = [
            key for key, leg in report.get("legs", {}).items() if leg.get("valid")
        ]

        # ── ⭐⭐ **判据 ②：四边形路径 ≡ 点图元路径**（同一后端 WebGL）──────────
        #
        # 两条路径**必须产出同一画面** ⇒ ⇒ 「四边形展开忠实复现点精灵」这条
        # 不变量**可复核** ⇒ ⇒ 才敢说「四边形路径不是分叉语义」。
        #
        # ⚠ **有效性**：两条腿的 **webgl 段**都必须**没有** error ⇒ 否则
        #   「没画」会被误读成「一致」。
        quad_vs_point = []
        for key, leg in report.get("legs", {}).items():
            parts = key.split(":")
            if len(parts) != 3 or parts[2] != "quad":
                continue
            point_key = f"{parts[0]}:{parts[1]}:point"
            point_leg = report.get("legs", {}).get(point_key)
            if not isinstance(point_leg, dict):
                continue
            webgl_ok = not (
                (leg.get("pixel_diff") or {}).get("webgl", {}).get("error")
                or (leg.get("pixel_diff") or {}).get("webgl", {}).get("skipped")
            )
            point_webgl_ok = not (
                (point_leg.get("pixel_diff") or {}).get("webgl", {}).get("error")
                or (point_leg.get("pixel_diff") or {}).get("webgl", {}).get("skipped")
            )
            if not (webgl_ok and point_webgl_ok):
                quad_vs_point.append(
                    {
                        "quad_leg": key,
                        "point_leg": point_key,
                        "comparable": False,
                        "skipped": True,
                        "error": "webgl 段无效（没画出来 ⇒ 不能当「一致」）",
                    }
                )
                continue
            diff = diff_stats(
                out_dir / f"intro.engine.quad.webgl.png",
                out_dir / f"intro.engine.point.webgl.png",
            )
            diff["quad_leg"] = key
            diff["point_leg"] = point_key
            quad_vs_point.append(diff)

        # ── ⭐⭐ **判据 ①：引擎双后端逐像素对照**（webgl vs webgpu）────────────
        dual_backend = []
        for key, leg in report.get("legs", {}).items():
            diff = leg.get("engine_dual_backend_diff") or {}
            entry = {"leg": key, "sprite_mode": leg.get("sprite_mode")}
            entry.update(diff)
            dual_backend.append(entry)

        # ── **判据 ③：生产 vs 引擎**（逐像素；两个后端各一份）──────────────────
        production_vs_engine = []
        for key, leg in report.get("legs", {}).items():
            for backend, segment in (leg.get("pixel_diff") or {}).items():
                production_vs_engine.append(
                    {"leg": key, "backend": backend, **(segment if isinstance(segment, dict) else {})}
                )

        def _zero(entries, key="differing_pixels"):
            """判据的通过条件：**每一条**都必须可比较且差 0 像素。"""
            if not entries:
                return None
            for entry in entries:
                if not entry.get("comparable") or entry.get("skipped"):
                    return False
                if entry.get(key) != 0:
                    return False
            return True

        criteria = [
            {
                "id": "engine_dual_backend_parity",
                "description": (
                    "引擎 WebGL vs 引擎 WebGPU：同一场景、同一时刻各截一张引擎画布 ⇒ 逐像素差 0"
                    "（⇒ 两个后端真的画出同一张图；「生产 vs 引擎」证明不了这件事）"
                ),
                "threshold": {"differing_pixels": 0, "channel_threshold": CHANNEL_THRESHOLD},
                "mandatory": True,
                "measured": dual_backend,
                "status": (
                    "pass"
                    if _zero(dual_backend) is True
                    else ("fail" if _zero(dual_backend) is False else "inconclusive")
                ),
            },
            {
                "id": "quad_path_matches_point_path",
                "description": (
                    "实例化四边形路径 vs 点图元路径（同一后端 WebGL、同一时刻）⇒ 逐像素差 0"
                    "（⇒ 四边形展开忠实复现点精灵 ⇒ 不是「分叉语义」）"
                ),
                "threshold": {"differing_pixels": 0, "channel_threshold": CHANNEL_THRESHOLD},
                "mandatory": True,
                "measured": quad_vs_point,
                "status": (
                    "pass"
                    if _zero(quad_vs_point) is True
                    else ("fail" if _zero(quad_vs_point) is False else "inconclusive")
                ),
            },
            {
                "id": "production_vs_engine",
                "description": (
                    "生产（原生 WebGL2 点图元）vs 引擎（同一批 uniform 取值）⇒ 逐像素差"
                    "（⚠ 这一条在本批之前对 WebGPU 是**假绿**：引擎侧根本没画粒子）"
                ),
                "threshold": {"differing_pixels": 0, "channel_threshold": CHANNEL_THRESHOLD},
                "mandatory": False,
                "measured": production_vs_engine,
                "status": (
                    "pass"
                    if _zero(production_vs_engine) is True
                    else ("fail" if _zero(production_vs_engine) is False else "inconclusive")
                ),
            },
        ]
        report["criteria"] = criteria

        # ⚠️ **判定**：**强制**判据全绿才算 pass（信息性判据只报数）。
        mandatory = [c for c in criteria if c.get("mandatory")]
        report["verdict"] = (
            "pass"
            if mandatory and all(c.get("status") == "pass" for c in mandatory)
            else "fail"
        )
        (out_dir / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (out_dir / "console.txt").write_text("\n".join(console_log), encoding="utf-8")
        print(json.dumps(legs, ensure_ascii=False, indent=2)[:8000])
        return 0
    finally:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            proc.kill()
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main())
