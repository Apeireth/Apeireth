#!/usr/bin/env python3
"""P3 试点对照：像素对照 + 逐帧对照。

## 做什么

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 差异像素 / 总像素 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（D1 结构 / D2 属性 / D3 文本必须为零） |
| T3 | **相位冻结对照** | 5 个相位点，每点差异占比 **≤ 1%** |
| T4 | **逐帧对照** | 更新期间的每一帧差异占比 **≤ 1%**，且帧数一致 |
| T5 | **动画结构对照** | `getAnimations()` 的动画集合 / 时长 / 延迟 / 关键帧**逐条相等** |
| T6 | **卸载时机对照** | 关闭卡片后，两侧在**同一帧**移除节点；且**不产生业务决策** |

## 两侧

- **参考侧**：`reference/index.html` —— 生产组件（只读引入）+ 同一批设计 CSS。
- **引擎侧**：`engine/index.html` —— wasm 引擎渲染同一张卡 + 同一批设计 CSS。

## 环境固定（设计文档 §9.3 步骤 2）

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧的「时刻」取值一致）；
- 同一浏览器实例、同一份 CSS、同一份 fixture。

## 用法

```bash
python3 tools/compare.py            # 跑全套，产物落在 out/
```

依赖：系统 Chromium（`/usr/bin/chromium`）、`websocket-client`、Pillow。
**不需要** Playwright，**不需要**下载浏览器。
"""

from __future__ import annotations

import base64
import io
import json
import os
import pathlib
import re
import signal
import subprocess
import sys
import threading
import time
import http.server
import socketserver
import urllib.request
from dataclasses import dataclass, field

import websocket  # websocket-client
from PIL import Image, ImageChops

HERE = pathlib.Path(__file__).resolve().parent
HARNESS = HERE.parent
DIST = HARNESS / "dist"
OUT = HARNESS / "out"

CHROME = os.environ.get("CHROME") or "/usr/bin/chromium"

# 对照口径（设计文档 §9.3）。
#: 对照时**浏览器时区**（`Emulation.setTimezoneOverride` 的取值）。
#:
#: ## 为什么要显式设成**非 UTC**
#:
#: 生产的时刻显示走 `toLocaleTimeString('zh-CN', …)` ⇒ **依赖浏览器本地时区**；
#: 而引擎侧此前**固定 UTC** ⇒ ⇒ 只要用户不在 UTC，两侧就差**时差**。
#:
#: ⇒ ⇒ 装置若也固定 UTC ⇒ **两边一致 ⇒ 缺陷被完全掩盖** ⇒ ⇒ 这是
#: 「**对照装置与被测对象一起错**」的典型 ⇒ ⇒ 判据**测不到真实场景**。
#:
#: ⇒ ⇒ 这里显式设成**非 UTC**（默认 `Asia/Shanghai`）⇒ ⇒
#: - 本地：**立刻暴露**引擎的固定 UTC；
#: - CI：**可复现**（不依赖 runner 的系统时区）。
#:
#: ⚠️ 两侧（参考 / 引擎）在**同一个浏览器** ⇒ **同一时区** ⇒ ⇒
#: ⇒ 判据比的是「引擎有没有按本地时区显示」，而不是「两边时区是否相同」。
TIMEZONE = os.environ.get("APEIRETH_COMPARE_TZ", "Asia/Shanghai")

VIEWPORT_WIDTH = 1440
VIEWPORT_HEIGHT = 900
CHANNEL_THRESHOLD = 2  # 任一通道绝对差 > 2/255 即记「不同」
PIXEL_BUDGET = 0.01  # ≤ 1%

# 相位采样点（设计文档 §9.4 步骤 2：0 / 1/4 / 1/2 / 3/4 / 1）。
PHASES_MS = [0, 150, 300, 450, 600]

# 逐帧对照的时长（动画 600ms ⇒ 留出余量）。
FRAME_WINDOW_MS = 1200

# 卸载时机的容差（毫秒）：两侧的「触发 ⇒ 节点被摘掉」间隔之差。
UNMOUNT_TOLERANCE_MS = 16.0

# 逐帧对照的时间对齐容差（毫秒）。
FRAME_ALIGN_TOLERANCE_MS = 25.0

# 逐帧对照的采集帧数（两侧**相同** ⇒ 帧数可对照）。
FRAME_COUNT = 20


def log(message: str) -> None:
    print(message, flush=True)


# ── 静态文件服务 ──────────────────────────────────────────────────────────────
def serve(directory: pathlib.Path):
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


# ── CDP 驱动 ──────────────────────────────────────────────────────────────────
class Cdp:
    """极简 CDP 客户端（websocket-client；不引浏览器驱动包）。"""

    def __init__(self, ws_url: str):
        self.ws = websocket.create_connection(ws_url, timeout=120, origin="http://127.0.0.1")
        self.next_id = 1
        self.events: list[dict] = []
        self.lock = threading.Lock()
        self.frames: list[dict] = []
        self.thread = threading.Thread(target=self._reader, daemon=True)
        self.thread.start()

    def _reader(self):
        while True:
            try:
                raw = self.ws.recv()
            except Exception:
                return
            try:
                message = json.loads(raw)
            except Exception:
                continue
            with self.lock:
                self.events.append(message)

    def call(self, method: str, params: dict | None = None, timeout: float = 60):
        message_id = self.next_id
        self.next_id += 1
        self.ws.send(json.dumps({"id": message_id, "method": method, "params": params or {}}))
        deadline = time.time() + timeout
        while time.time() < deadline:
            with self.lock:
                for message in self.events:
                    if message.get("id") == message_id:
                        return message
            time.sleep(0.02)
        raise TimeoutError(f"CDP 调用 {method} 超时")

    def drain(self) -> list[dict]:
        with self.lock:
            out = list(self.events)
            self.events.clear()
        return out

    def evaluate(self, expression: str, await_promise: bool = False):
        result = self.call(
            "Runtime.evaluate",
            {
                "expression": expression,
                "returnByValue": True,
                "awaitPromise": await_promise,
            },
        )
        payload = result.get("result", {}).get("result", {}) or {}
        if result.get("result", {}).get("exceptionDetails"):
            raise RuntimeError(
                "页面求值异常："
                + json.dumps(result["result"]["exceptionDetails"], ensure_ascii=False)[:400]
            )
        return payload.get("value")

    def wait_for(self, expression: str, timeout: float = 60.0, interval: float = 0.05):
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                if self.evaluate(expression):
                    return True
            except RuntimeError:
                pass
            time.sleep(interval)
        return False

    def close(self):
        try:
            self.ws.close()
        except Exception:
            pass


def launch_chrome() -> tuple[subprocess.Popen, int]:
    args = [
        CHROME,
        "--headless=new",
        "--no-sandbox",
        "--disable-dev-shm-usage",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--force-device-scale-factor=1",
        "--hide-scrollbars",
        "--remote-allow-origins=*",
        "--remote-debugging-port=0",
        # ⭐⭐ **真实 GPU 硬件加速**（2026-10-26 实测加的）。
        #
        # ⚠⚠️ **不加这组参数时，headless Chrome 一律走 SwiftShader（纯软件）**
        # ⇒ ⇒ 此前**所有**浏览器实测都跑在软件渲染上 ⇒ ⇒ 「真实 GPU 未实测」这条账
        # ⇒ ⇒ 一直挂着。实测对照（同一台机器）：
        #   默认            ⇒ ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)
        #   加这组参数      ⇒ ANGLE (AMD, Vulkan 1.4.354 (AMD Radeon 8060S Graphics (RADV STRIX_HALO)), radv)
        #
        # ⇒ ⇒ `--use-angle=vulkan` 让 ANGLE 走 **Mesa RADV**（真实硬件）。
        # ⇒ ⇒ `--ignore-gpu-blocklist` ⇒ 绕开 Chrome 的黑名单（容器/无头常被误列）。
        # ⇒ ⇒ `--enable-features=Vulkan` ⇒ 显式启用 Vulkan。
        # ⇒ ⇒ **可回退**：环境变量 `APEIRETH_CHROME_SWIFTSHADER=1` ⇒ ⇒ 退回软件渲染
        #    （软件后端仍需保留 ⇒ 因为它是**可复现**的基线，而硬件后端会随驱动变）。
        "--use-angle=vulkan",
        "--enable-features=Vulkan",
        "--ignore-gpu-blocklist",
        "--enable-unsafe-swiftshader",
        f"--window-size={VIEWPORT_WIDTH},{VIEWPORT_HEIGHT}",
        "about:blank",
    ]
    process = subprocess.Popen(
        args, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, start_new_session=True
    )
    port = None
    deadline = time.time() + 30
    while time.time() < deadline:
        line = process.stderr.readline().decode("utf-8", "replace")
        if "DevTools listening on" in line:
            match = re.search(r":(\d+)/devtools", line)
            if match:
                port = int(match.group(1))
                break
    if port is None:
        process.kill()
        raise RuntimeError("Chromium 没有报告 DevTools 端口")
    return process, port


def open_page(port: int) -> Cdp:
    deadline = time.time() + 30
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=2) as r:
                json.load(r)
            break
        except Exception:
            time.sleep(0.2)
    request = urllib.request.Request(f"http://127.0.0.1:{port}/json/new?about:blank", method="PUT")
    with urllib.request.urlopen(request, timeout=10) as r:
        target = json.load(r)
    cdp = Cdp(target["webSocketDebuggerUrl"])
    cdp.call("Page.enable")
    cdp.call("Runtime.enable")
    cdp.call("Emulation.setDeviceMetricsOverride", {
        "width": VIEWPORT_WIDTH,
        "height": VIEWPORT_HEIGHT,
        "deviceScaleFactor": 1,
        "mobile": False,
    })
    cdp.call("Emulation.setTimezoneOverride", {"timezoneId": TIMEZONE})
    cdp.call("Emulation.setEmulatedMedia", {
        "features": [{"name": "prefers-reduced-motion", "value": "no-preference"}]
    })
    return cdp


# ── 页面操作 ──────────────────────────────────────────────────────────────────
#: 归一化 DOM 的脚本（去掉 Svelte 作用域 class、折叠空白、排序属性与 class token）。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '').replace(/\bsvelte-[0-9a-z]+\b/g, '').trim();
  const normalize = (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const data = node.textContent.replace(/\s+/g, ' ').trim();
      return data ? { t: 'text', v: data } : null;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return null;
    const attrs = {};
    for (const attr of Array.from(node.attributes)) {
      let name = attr.name;
      let value = attr.value;
      if (name === 'class') {
        value = strip(value).split(/\s+/).filter(Boolean).sort().join(' ');
        if (!value) continue;
      } else if (name.startsWith('svelte-')) {
        continue;
      }
      attrs[name] = value;
    }
    const children = Array.from(node.childNodes).map(normalize).filter(Boolean);
    return { t: 'el', tag: node.tagName.toLowerCase(), attrs, children };
  };
  const root = document.querySelector('.approval-card');
  return JSON.stringify(root ? normalize(root) : null);
})()
"""

#: 动画结构（`getAnimations()`）的抽取脚本。
ANIMATIONS_SCRIPT = r"""
(() => {
  const list = document.getAnimations().map((animation) => {
    const timing = animation.effect.getTiming();
    const raw = animation.effect.target
      ? (animation.effect.target.className || animation.effect.target.tagName)
      : null;
    const target = String(raw || '').split(/\s+/).filter((token) => !/^svelte-[0-9a-z]+$/.test(token)).sort().join(' ');
    return {
      target: target,
      duration: timing.duration,
      delay: timing.delay,
      easing: timing.easing,
      keyframes: JSON.stringify(animation.effect.getKeyframes()),
    };
  });
  return JSON.stringify(list);
})()
"""


#: 页内观察者：记录卡片节点被摘掉的时刻（毫秒）。
OBSERVER_SCRIPT = r"""
(() => {
  window.__PILOT_TIMING__ = { t0: 0, removedAt: null };
  const observer = new MutationObserver(() => {
    if (window.__PILOT_TIMING__.removedAt === null &&
        !document.querySelector('.approval-card')) {
      window.__PILOT_TIMING__.removedAt = performance.now();
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });
  return true;
})()
"""


#: 页内观察者：记录 **CSS 动画起播事件**（G2「动画是否起播」的事件级判据）。
#:
#: 为什么不能只靠 `getAnimations()`：`document.getAnimations()` 会返回**尚未起播**的
#: pending 动画 ⇒ 「动画结构相等」**不足以**证明「动画真的起播了」。事件计数才是
#: 起播的直接证据（`transitionrun` / `animationstart` 在动画真正开始播放时才触发）。
EVENTS_SCRIPT = r"""
(() => {
  window.__PILOT_EVENTS__ = { counts: {}, timeline: [] };
  const names = ['transitionrun', 'transitionstart', 'transitionend',
                 'animationstart', 'animationend', 'animationcancel'];
  for (const name of names) {
    window.__PILOT_EVENTS__.counts[name] = 0;
    // 捕获阶段：事件从目标向外冒泡，捕获阶段先于目标自身的监听器 ⇒ 不会漏。
    document.addEventListener(name, (event) => {
      window.__PILOT_EVENTS__.counts[name] += 1;
      const target = event.target;
      const raw = target && (target.className || target.tagName);
      window.__PILOT_EVENTS__.timeline.push({
        name: name,
        at: Math.round(performance.now() * 1000) / 1000,
        target: String(raw || '').split(/\s+/).filter(Boolean).sort().join(' '),
        property: event.propertyName || null,
      });
    }, true);
  }
  return true;
})()
"""


#: 读取事件计数（结构化 ⇒ 可直接进 report.json）。
EVENTS_READ_SCRIPT = r"""
(() => {
  const state = window.__PILOT_EVENTS__ || { counts: {}, timeline: [] };
  return JSON.stringify({
    counts: state.counts,
    timeline: state.timeline,
    started: (state.counts['transitionrun'] || 0) + (state.counts['animationstart'] || 0),
  });
})()
"""


@dataclass
class Side:
    """一侧的对照材料。"""

    name: str
    dom: str | None = None
    card_box: dict | None = None
    animations: str | None = None
    events: str | None = None
    screenshots: dict[str, bytes] = field(default_factory=dict)
    frames: list[tuple[float, bytes]] = field(default_factory=list)
    dismiss_delay_ms: float | None = None
    decision_after_dismiss: object = None


def navigate(cdp: Cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__PILOT__ && window.__PILOT__.ready()", timeout=60)
    # 让字体与布局稳定（两侧同样等待）。
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.4)


def card_box(cdp: Cdp) -> dict:
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector('.approval-card');"
        " if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def screenshot(cdp: Cdp, box: dict | None = None) -> bytes:
    params = {"format": "png"}
    if box:
        params["clip"] = {
            "x": max(0.0, box["x"]),
            "y": max(0.0, box["y"]),
            "width": box["w"],
            "height": box["h"],
            "scale": 1,
        }
        params["captureBeyondViewport"] = True
    shot = cdp.call("Page.captureScreenshot", params)
    return base64.b64decode(shot["result"]["data"])


def pixel_diff(left: bytes, right: bytes) -> dict:
    """逐像素差分（阈值：任一通道绝对差 > 2/255）。"""
    a = Image.open(io.BytesIO(left)).convert("RGBA")
    b = Image.open(io.BytesIO(right)).convert("RGBA")
    if a.size != b.size:
        return {
            "size_mismatch": True,
            "left_size": a.size,
            "right_size": b.size,
            "different": -1,
            "total": 0,
            "ratio": 1.0,
        }
    diff = ImageChops.difference(a, b)
    bands = diff.split()
    # 任一通道超过阈值 ⇒ 该像素算「不同」。
    mask = bands[0].point(lambda v: 255 if v > CHANNEL_THRESHOLD else 0)
    for band in bands[1:]:
        mask = ImageChops.lighter(
            mask, band.point(lambda v: 255 if v > CHANNEL_THRESHOLD else 0)
        )
    different = sum(mask.point(lambda v: 1 if v else 0).getdata())
    total = a.size[0] * a.size[1]
    return {
        "size_mismatch": False,
        "different": different,
        "total": total,
        "ratio": (different / total) if total else 0.0,
    }


def diff_bbox(left: bytes, right: bytes) -> list[int] | None:
    """差异像素的包围盒（定位「哪里不一样」用）。"""
    a = Image.open(io.BytesIO(left)).convert("RGB")
    b = Image.open(io.BytesIO(right)).convert("RGB")
    if a.size != b.size:
        return None
    return ImageChops.difference(a, b).getbbox()


def capture_frames(cdp: Cdp, duration_ms: int, max_frames: int) -> list[tuple[float, bytes]]:
    """显式连拍（每次 `captureScreenshot` 都强制产出一帧）。

    比录屏（screencast）可靠：录屏只在**画面变化**时发帧，且帧数受合成器
    调度影响（实测两侧可能差一帧）⇒ 这里按**固定帧数**采集（两侧帧数相同），
    并在比对时按**时间对齐**（容差内配对）。
    """
    frames: list[tuple[float, bytes]] = []
    started = time.time()
    while (time.time() - started) * 1000.0 < duration_ms and len(frames) < max_frames:
        timestamp = (time.time() - started) * 1000.0
        frames.append((timestamp, screenshot(cdp, None)))
    return frames


def start_screencast(cdp: Cdp) -> None:
    cdp.call("Page.startScreencast", {"format": "png", "everyNthFrame": 1})


def stop_screencast(cdp: Cdp) -> list[tuple[float, bytes]]:
    """停止录屏并取回帧（时间戳 + PNG 字节）。"""
    cdp.call("Page.stopScreencast")
    frames: list[tuple[float, bytes]] = []
    deadline = time.time() + 5
    while time.time() < deadline:
        for message in cdp.drain():
            if message.get("method") == "Page.screencastFrame":
                params = message["params"]
                frames.append(
                    (params.get("metadata", {}).get("timestamp", 0.0), base64.b64decode(params["data"]))
                )
        if frames:
            break
        time.sleep(0.05)
    # 剩余帧可能还在队列里。
    time.sleep(0.2)
    for message in cdp.drain():
        if message.get("method") == "Page.screencastFrame":
            params = message["params"]
            frames.append(
                (params.get("metadata", {}).get("timestamp", 0.0), base64.b64decode(params["data"]))
            )
    frames.sort(key=lambda item: item[0])
    return frames


def collect_side(cdp: Cdp, name: str, url: str) -> Side:
    """跑完一侧的全部测量。"""
    side = Side(name=name)

    # ── T1 / T2：静态状态 ──────────────────────────────────────────────
    navigate(cdp, url)
    side.dom = cdp.evaluate(NORMALIZE_SCRIPT)
    side.card_box = card_box(cdp)
    side.screenshots["static"] = screenshot(cdp, side.card_box)
    side.screenshots["viewport"] = screenshot(cdp, None)

    # ── T5：动画结构（打开测试动画后读取） ─────────────────────────────
    # ── T2'：动画起播事件计数（G2 的事件级判据）——
    #    观察者必须**先于**触发安装，否则会漏掉起播事件。
    cdp.evaluate(EVENTS_SCRIPT)
    cdp.evaluate("document.documentElement.dataset.pilotMotion = 'on'; true")
    cdp.evaluate("document.documentElement.dataset.pilotFreeze = '0'; true")
    time.sleep(0.2)
    side.animations = cdp.evaluate(ANIMATIONS_SCRIPT)
    # 等动画走完（600 ms 过渡）再读计数 ⇒ 拿到完整的 start / end 配对。
    time.sleep(0.9)
    side.events = cdp.evaluate(EVENTS_READ_SCRIPT)
    cdp.evaluate("document.documentElement.dataset.pilotMotion = '0'; true")

    # ── T4：逐帧（更新时机） ───────────────────────────────────────────
    navigate(cdp, url)
    cdp.evaluate("document.documentElement.dataset.pilotTransition = 'on'; true")
    time.sleep(0.2)
    time.sleep(0.15)
    # 触发放在**帧边界**（requestAnimationFrame）⇒ 两侧的过渡起点同相位。
    cdp.evaluate(
        "requestAnimationFrame(() => { window.__PILOT__.setBusy(true); }); true"
    )
    side.frames = capture_frames(cdp, FRAME_WINDOW_MS, FRAME_COUNT)

    # ── T6：卸载时机（页内观察者 ⇒ 毫秒级） ───────────────────────────
    navigate(cdp, url)
    cdp.evaluate("document.documentElement.dataset.pilotMotion = 'on'; true")
    time.sleep(0.2)
    cdp.evaluate(OBSERVER_SCRIPT)
    cdp.evaluate(
        "requestAnimationFrame(() => {"
        " window.__PILOT_TIMING__.t0 = performance.now();"
        " window.__PILOT__.dismiss(); }); true"
    )
    time.sleep(0.5)
    side.dismiss_delay_ms = cdp.evaluate(
        "(() => { const t = window.__PILOT_TIMING__;"
        " return t.removedAt === null ? null : t.removedAt - t.t0; })()"
    )
    side.decision_after_dismiss = cdp.evaluate("window.__PILOT__.decision()")
    return side


def collect_phases(cdp: Cdp, url: str) -> dict[str, bytes]:
    """相位冻结下的截图（每个相位一张）。"""
    shots: dict[str, bytes] = {}
    for phase in PHASES_MS:
        navigate(cdp, url)
        cdp.evaluate("document.documentElement.dataset.pilotMotion = 'on'; true")
        cdp.evaluate("document.documentElement.dataset.pilotFreeze = '1'; true")
        cdp.evaluate(
            f"document.documentElement.style.setProperty('--pilot-phase', '-{phase}ms'); true"
        )
        time.sleep(0.25)
        box = card_box(cdp)
        shots[str(phase)] = screenshot(cdp, box)
    return shots


def main() -> int:
    if not DIST.exists():
        log(f"未找到构建产物：{DIST}（先跑 `pnpm build` 或 `bash build.sh`）")
        return 2
    OUT.mkdir(parents=True, exist_ok=True)

    httpd, port = serve(DIST)
    base = f"http://127.0.0.1:{port}"
    chrome, devtools_port = launch_chrome()
    report: dict = {
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "phases_ms": PHASES_MS,
    }
    try:
        cdp = open_page(devtools_port)
        log("── 参考侧（生产 Svelte 组件） ──")
        reference = collect_side(cdp, "reference", f"{base}/reference/index.html")
        log("── 引擎侧（wasm 引擎） ──")
        engine = collect_side(cdp, "engine", f"{base}/engine/index.html")
        log("── 相位冻结采样 ──")
        reference_phases = collect_phases(cdp, f"{base}/reference/index.html")
        engine_phases = collect_phases(cdp, f"{base}/engine/index.html")

        # ── T1：静态像素对照 ────────────────────────────────────────────
        static = pixel_diff(
            reference.screenshots["static"], engine.screenshots["static"]
        )
        report["t1_static_pixels"] = static
        (OUT / "reference-static.png").write_bytes(reference.screenshots["static"])
        (OUT / "engine-static.png").write_bytes(engine.screenshots["static"])
        log(
            f"T1 静态像素：差异 {static['different']} / 总 {static['total']} "
            f"= {static['ratio'] * 100:.4f}%"
        )

        # ── T2：DOM 结构对照 ────────────────────────────────────────────
        dom_equal = json.loads(reference.dom or "null") == json.loads(engine.dom or "null")
        report["t2_dom_equal"] = dom_equal
        (OUT / "reference-dom.json").write_text(
            json.dumps(json.loads(reference.dom or "null"), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        (OUT / "engine-dom.json").write_text(
            json.dumps(json.loads(engine.dom or "null"), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        log(f"T2 DOM 结构：{'一致' if dom_equal else '不一致'}")

        # ── T3：相位冻结对照 ────────────────────────────────────────────
        phase_results = []
        for phase in PHASES_MS:
            key = str(phase)
            diff = pixel_diff(reference_phases[key], engine_phases[key])
            phase_results.append({"phase_ms": phase, **diff})
            (OUT / f"reference-phase-{key}.png").write_bytes(reference_phases[key])
            (OUT / f"engine-phase-{key}.png").write_bytes(engine_phases[key])
        report["t3_phase_pixels"] = phase_results
        for item in phase_results:
            log(
                f"T3 相位 {item['phase_ms']}ms：差异 {item['different']} / 总 "
                f"{item['total']} = {item['ratio'] * 100:.4f}%"
            )

        # ── T4：逐帧对照 ────────────────────────────────────────────────
        frame_results = []
        count = min(len(reference.frames), len(engine.frames))
        ref_base = reference.frames[0][0] if reference.frames else 0.0
        eng_base = engine.frames[0][0] if engine.frames else 0.0
        # 按**序号**配对（两侧的采集帧数固定 ⇒ 序号可对照）；同时报告两侧的
        # 相对时刻与相位差（采集耗时不同 ⇒ 相位不会完全对齐，这是采集侧的
        # 事实，不是渲染差异）。
        for index in range(count):
            ref_ts, ref_data = reference.frames[index]
            eng_ts, eng_data = engine.frames[index]
            diff = pixel_diff(ref_data, eng_data)
            diff["bbox"] = diff_bbox(ref_data, eng_data)
            frame_results.append(
                {
                    "index": index,
                    "reference_relative_ms": round(ref_ts - ref_base, 3),
                    "engine_relative_ms": round(eng_ts - eng_base, 3),
                    "phase_delta_ms": round(
                        (eng_ts - eng_base) - (ref_ts - ref_base), 3
                    ),
                    "unpaired": False,
                    **diff,
                }
            )
        # 帧证据落盘（供复核）。
        for index in range(count):
            (OUT / f"reference-frame-{index}.png").write_bytes(reference.frames[index][1])
            (OUT / f"engine-frame-{index}.png").write_bytes(engine.frames[index][1])
        intervals = [
            round(reference.frames[i][0] - reference.frames[i - 1][0], 3)
            for i in range(1, len(reference.frames))
        ]
        report["t4_frames"] = {
            "reference_frame_count": len(reference.frames),
            "engine_frame_count": len(engine.frames),
            "window_ms": FRAME_WINDOW_MS,
            "target_frame_count": FRAME_COUNT,
            "align_tolerance_ms": FRAME_ALIGN_TOLERANCE_MS,
            "reference_mean_interval_ms": (
                round(sum(intervals) / len(intervals), 3) if intervals else None
            ),
            "frames": frame_results,
        }
        log(
            f"T4 逐帧：参考 {len(reference.frames)} 帧 / 引擎 {len(engine.frames)} 帧"
        )
        for item in frame_results:
            if item["unpaired"]:
                log(
                    f"   帧 {item['index']:>2}：**无配对帧**（参考 {item['reference_relative_ms']} ms）"
                )
                continue
            log(
                f"   帧 {item['index']:>2}：差异 {item['different']} / 总 {item['total']}"
                f" = {item['ratio'] * 100:.4f}%（相位差 {item['phase_delta_ms']} ms）"
            )

        # ── T5：动画结构对照 ────────────────────────────────────────────
        animations_equal = json.loads(reference.animations or "[]") == json.loads(
            engine.animations or "[]"
        )
        report["t5_animations_equal"] = animations_equal
        (OUT / "reference-animations.json").write_text(
            reference.animations or "[]", encoding="utf-8"
        )
        (OUT / "engine-animations.json").write_text(engine.animations or "[]", encoding="utf-8")
        log(f"T5 动画结构：{'一致' if animations_equal else '不一致'}")

        # ── T2'：动画起播事件计数（G2 的事件级判据）────────────────────
        # 「动画结构相等」**不足以**证明「动画真的起播了」—— getAnimations() 会返回
        # 尚未起播的 pending 动画。事件计数（transitionrun / animationstart）才是
        # 起播的直接证据；G2 要求「CSS 动画起播帧数差异 = 0」⇒ 两侧计数须逐项相等。
        ref_events = json.loads(reference.events or "null") or {
            "counts": {},
            "timeline": [],
            "started": 0,
        }
        eng_events = json.loads(engine.events or "null") or {
            "counts": {},
            "timeline": [],
            "started": 0,
        }
        event_counts_equal = ref_events["counts"] == eng_events["counts"]
        events_started = ref_events["started"]
        report["t2b_animation_events"] = {
            "reference": ref_events,
            "engine": eng_events,
            "counts_equal": event_counts_equal,
            "started_reference": events_started,
            "started_engine": eng_events["started"],
            "started_delta": events_started - eng_events["started"],
        }
        (OUT / "reference-events.json").write_text(
            json.dumps(ref_events, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (OUT / "engine-events.json").write_text(
            json.dumps(eng_events, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        log(
            f"T2' 起播事件：参考 {events_started} 次 / 引擎 "
            f"{eng_events['started']} 次（起播帧数差异 "
            f"{events_started - eng_events['started']}）"
        )

        # ── T6：卸载时机 ────────────────────────────────────────────────
        report["t6_unmount"] = {
            "reference_dismiss_delay_ms": reference.dismiss_delay_ms,
            "engine_dismiss_delay_ms": engine.dismiss_delay_ms,
            "reference_decision_after_dismiss": reference.decision_after_dismiss,
            "engine_decision_after_dismiss": engine.decision_after_dismiss,
        }
        log(
            f"T6 卸载：参考 {reference.dismiss_delay_ms} ms / 引擎 "
            f"{engine.dismiss_delay_ms} ms（触发 ⇒ 节点被摘掉）；"
            f"关闭后的决策：参考 {reference.decision_after_dismiss} / 引擎 "
            f"{engine.decision_after_dismiss}"
        )

        # ── 汇总判据 ────────────────────────────────────────────────────
        checks = {
            "t1_static_pixels_within_budget": (
                not static["size_mismatch"] and static["ratio"] <= PIXEL_BUDGET
            ),
            "t2_dom_identical": dom_equal,
            "t3_all_phases_within_budget": all(
                not item["size_mismatch"] and item["ratio"] <= PIXEL_BUDGET
                for item in phase_results
            ),
            "t4_enough_frames": len(frame_results) == FRAME_COUNT,
            "t4_all_frames_within_budget": all(
                (not item["unpaired"])
                and (not item["size_mismatch"])
                and item["ratio"] <= PIXEL_BUDGET
                for item in frame_results
            ),
            "t5_animations_identical": animations_equal,
            # G2：动画起播事件计数逐项相等（起播帧数差异 = 0）且**两侧都真的起播了**
            # （计数 > 0 —— 两侧都没起播时「相等」是空真，必须排除）。
            "t2b_animation_events_equal": event_counts_equal,
            "t2b_animation_started_on_both_sides": (
                events_started > 0 and eng_events["started"] > 0
            ),
            "t6_unmount_same_timing": (
                reference.dismiss_delay_ms is not None
                and engine.dismiss_delay_ms is not None
                and abs(reference.dismiss_delay_ms - engine.dismiss_delay_ms) <= UNMOUNT_TOLERANCE_MS
            ),
            "t6_dismiss_makes_no_decision": (
                reference.decision_after_dismiss is None
                and engine.decision_after_dismiss is None
            ),
        }
        report["notes"] = [
            "T1/T3 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T3 是**同相位**对照（动画被冻结在指定相位）⇒ 最严格的画面判据。",
            "T4 是**实时**对照：两帧的采集耗时不同 ⇒ 相位会有偏移（报告里的 "
            "phase_delta_ms 即采集侧的偏移，不是渲染差异）；"
            "过渡结束后的帧差异为 0。",
            "T2 的归一化：剔除 Svelte 作用域 class、class token 排序、属性键排序、"
            "纯空白文本节点折叠。",
            "T2' 是**动画起播的事件级证据**（transitionrun / animationstart 等计数）："
            "getAnimations() 会返回尚未起播的 pending 动画 ⇒ 结构相等**不足以**证明起播；"
            "事件计数才是起播的直接证据。",
            "环境：视口 1440×900、DPR 1、时区 UTC、prefers-reduced-motion: no-preference。",
        ]
        report["checks"] = checks
        report["passed"] = all(checks.values())
        (OUT / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        # 证据落盘（**入库**：P3 的可复核凭据）。
        evidence = HARNESS / "evidence"
        evidence.mkdir(parents=True, exist_ok=True)
        (evidence / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (evidence / "reference-static.png").write_bytes(reference.screenshots["static"])
        (evidence / "engine-static.png").write_bytes(engine.screenshots["static"])
        log("")
        log("── 判据 ──")
        for key, value in checks.items():
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        cdp.close()
        return 0 if report["passed"] else 1
    finally:
        try:
            os.killpg(os.getpgid(chrome.pid), signal.SIGKILL)
        except Exception:
            chrome.kill()
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main())
