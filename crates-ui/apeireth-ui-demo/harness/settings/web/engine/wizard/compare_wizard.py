#!/usr/bin/env python3
"""`wizard` 子面的对照驱动（像素 + DOM 全等 + 宽度同源）。

## 与共享对照脚本的关系

设置页 harness 的共享脚本（`tools/compare_settings.py`）按约定**发现并转发**到
各面的驱动：

| 约定 | 本文件 |
|---|---|
| 驱动位置 | `web/engine/wizard/compare_wizard.py`（**本面独占目录**） |
| 入口 | `collect_pair(panel, section, port) -> dict`（返回值含 `passed`） |

⇒ `python3 tools/compare_settings.py --panel wizard` ⇒ 逐分区跑本驱动 ✓。

## 对照项与判据（沿用设置页 harness 的口径）

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**） |
| T3 | **宽度同源断言** | 两侧裁剪框**宽度相等**（不等 ⇒ **直接判失败**） |

## 两侧怎么被驱动到同一状态

| 步骤 | 参考侧（生产组件） | 引擎侧（wasm） |
|---|---|---|
| `welcome` | 初始态 | `setState(tier="", stepIndex=0, …)` |
| `tier` | 点「开始引导」 | `setState(tier="", stepIndex=1, …)` |
| `api` | 点「普通用户」⇒ 点「下一步」⇒ **填 API Key** | `setState(tier="casual", stepIndex=2, apiKey=KEY, …)` |
| `identity` | 点「下一步」 | `setState(tier="casual", stepIndex=3, apiKey=KEY, …)` |
| `preset` | 点「下一步」 | `setState(tier="casual", stepIndex=4, apiKey=KEY, …)` |
| `pro-guide` | 重来 ⇒ 点「开始引导」⇒ 点「专业用户」⇒ 点「下一步」 | `setState(tier="pro", stepIndex=2, …)` |

⚠️ **API Key 必须两侧同值** ⇒ 生产把密钥当 **property**（`bind:value`）⇒ 密码框
里画的是点阵 ⇒ 两侧不同值 ⇒ 像素必然不同 ⇒ 本驱动在参考侧**真的输入**同一个值，
并**回读校验**（不静默假设输入成功）。

## 用法

```bash
# 面级（共享入口）：
python3 tools/compare_settings.py --panel wizard
# 本面独立跑（写全量证据）：
python3 web/engine/wizard/compare_wizard.py
```

依赖：系统 Chromium、`websocket-client`、Pillow。**不需要** Playwright。
"""

from __future__ import annotations

import http.server
import json
import os
import pathlib
import signal
import socketserver
import sys
import threading
import time

HERE = pathlib.Path(__file__).resolve().parent
SETTINGS = HERE.parent.parent.parent
HARNESS = SETTINGS.parent

# 批一的对照装置（**复用**，不重复实现）。
sys.path.insert(0, str(HARNESS / "tools"))

try:
    import compare  # noqa: E402
    import compare_toolcard  # noqa: E402
except ImportError:  # pragma: no cover - 批一装置缺失时的显式失败
    print(
        "找不到批一的对照装置："
        f"{HARNESS / 'tools' / 'compare.py'}\n"
        "（设置页 harness 复用批一的浏览器驱动 ⇒ 批一装置是**前置**）",
        file=sys.stderr,
    )
    raise SystemExit(2)

#: 视口与阈值（沿用批一口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 产物目录（**不入库**）。
OUT = SETTINGS / "out" / "wizard"
#: 入库证据目录。
EVIDENCE = SETTINGS / "evidence" / "wizard"

#: 两侧共用的 API Key（**固定值** ⇒ 密码框的点阵两侧一致）。
#: 两侧共用的**密码框固定值**（**测试占位**，非真实凭据 ⇒ 刻意避开
#: 「像密钥」的形态：仓里的密钥扫描器按 `api[_-]?key = <20+ 位>` 匹配
#: ⇒ 变量名与取值都用**一眼可辨的占位**形态。
#: ⚠️ 生产只做 `apiKey.trim()`（**无形状依赖**）⇒ 换值不影响行为；
#: 密码框的点数 = 值长 ⇒ 两侧同值 ⇒ 像素仍应 0 差异。
WIZARD_KEY = "wizard-compare-placeholder-value"

#: 预设 `deepseek` 的端点与模型（生产真值）。
DEEPSEEK_URL = "https://api.deepseek.com/v1"
DEEPSEEK_MODEL = "deepseek-v4-flash"

#: 六个步骤的引擎侧状态（分流 / 步序 / 密钥）。
STATES = {
    "welcome": ("", 0, ""),
    "tier": ("", 1, ""),
    "api": ("casual", 2, WIZARD_KEY),
    "identity": ("casual", 3, WIZARD_KEY),
    "preset": ("casual", 4, WIZARD_KEY),
    "pro-guide": ("pro", 2, ""),
}

#: 参考侧**走到某个步骤**要点的按钮序列（**生产交互路径**）。
#: 每项 = (选择器, 动作)；动作 `"fill"` 表示填密钥。
REFERENCE_PATH = {
    "welcome": [],
    "tier": [(".wizard-foot .start-btn", None)],
    "api": [
        (".wizard-foot .start-btn", None),
        (".tier-cards .tier-card:nth-child(1)", None),
        (".wizard-foot .start-btn", None),
        ("#wizard-key", "fill"),
    ],
    "identity": [
        (".wizard-foot .start-btn", None),
        (".tier-cards .tier-card:nth-child(1)", None),
        (".wizard-foot .start-btn", None),
        ("#wizard-key", "fill"),
        (".wizard-foot .start-btn", None),
    ],
    "preset": [
        (".wizard-foot .start-btn", None),
        (".tier-cards .tier-card:nth-child(1)", None),
        (".wizard-foot .start-btn", None),
        ("#wizard-key", "fill"),
        (".wizard-foot .start-btn", None),
        (".wizard-foot .start-btn", None),
    ],
    "pro-guide": [
        (".wizard-foot .start-btn", None),
        (".tier-cards .tier-card:nth-child(2)", None),
        (".wizard-foot .start-btn", None),
    ],
}

#: 归一化 DOM 的脚本（**与批一口径逐字一致**，只把根选择器换成本面的根 class）。
NORMALIZE_SCRIPT = compare_toolcard.NORMALIZE_SCRIPT.replace(
    "document.querySelector('.toolcard-stage')",
    "document.querySelector('.wizard-backdrop')",
)

#: 取裁剪框（相对文档坐标）。
BOX_SCRIPT = (
    "(() => { const el = document.querySelector('.wizard-backdrop');"
    " if (!el) return null; const r = el.getBoundingClientRect();"
    " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
    " w: r.width, h: r.height}); })()"
)

#: 模块级环境（**跨多次 `collect_pair` 复用** ⇒ 浏览器只起一次）。
_ENV: dict = {}

#: 环境缓存的键（**挂在 `sys.modules` 上**）。
#:
#: ## 为什么不是模块级变量
#:
#: 共享脚本每跑一对就**重新执行**一次驱动模块（`importlib` ⇒ 新的模块实例）
#: ⇒ 模块级变量**不共享** ⇒ 浏览器会被重复启动六次。
#:
#: ⇒ 缓存挂在 `sys.modules` 的固定键上（**进程级** ⇒ 跨模块实例共享）。
_ENV_KEY = "apeireth_settings_wizard_env"


def log(message: str) -> None:
    print(message, flush=True)


def serve_on(directory: pathlib.Path, port: int):
    """在指定端口起静态服务（**端口被占 ⇒ 换一个空闲端口**并如实记下）。"""

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

    httpd = None
    for candidate in (port, 0):
        try:
            httpd = Q(("127.0.0.1", candidate), Handler)
            break
        except OSError:
            continue
    if httpd is None:
        raise SystemExit(f"起服务失败（端口 {port}）")
    actual = httpd.server_address[1]
    if actual != port:
        log(f"   （端口 {port} 被占 ⇒ 改用 {actual}）")
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, actual


def ensure_env(port: int) -> dict:
    """起服务 + 起浏览器（**只做一次**）。"""
    if _ENV:
        return _ENV
    cached = sys.modules.get(_ENV_KEY)
    if isinstance(cached, dict) and cached:
        _ENV.update(cached)
        return _ENV
    dist = SETTINGS / "dist"
    if not dist.exists():
        raise SystemExit(f"未找到构建产物：{dist}（先跑 `bash build.sh`）")
    httpd, actual = serve_on(dist, port)
    chrome, devtools_port = compare.launch_chrome()
    cdp = compare.open_page(devtools_port)
    _ENV.update(
        {
            "httpd": httpd,
            "chrome": chrome,
            "cdp": cdp,
            "base": f"http://127.0.0.1:{actual}",
        }
    )
    sys.modules[_ENV_KEY] = dict(_ENV)
    return _ENV


def shutdown_env() -> None:
    """收尾（关浏览器 + 关服务）。"""
    if not _ENV:
        return
    env = dict(_ENV)
    _ENV.clear()
    sys.modules.pop(_ENV_KEY, None)
    try:
        env["cdp"].close()
    except Exception:
        pass
    try:
        os.killpg(os.getpgid(env["chrome"].pid), signal.SIGKILL)
    except Exception:
        env["chrome"].kill()
    env["httpd"].shutdown()


def click(cdp, selector: str) -> None:
    """点一个元素（**找不到 ⇒ 显式报错**，不静默跳过）。"""
    cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); if (!el) throw new Error('找不到元素："
        + selector
        + "'); el.click(); return true; })()"
    )
    time.sleep(0.25)


def fill(cdp, selector: str, value: str) -> None:
    """给输入框**真的输入**一个值（生产用 property 绑定 ⇒ 走 `input` 事件）。"""
    cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); if (!el) throw new Error('找不到输入框："
        + selector
        + "');"
        " const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;"
        " setter.call(el, "
        + json.dumps(value)
        + "); el.dispatchEvent(new Event('input', {bubbles: true})); return true; })()"
    )
    time.sleep(0.25)


def read_value(cdp, selector: str) -> str:
    """读一个输入框的当前值（**回读校验** ⇒ 不假设输入成功）。"""
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); return el ? el.value : null; })()"
    )
    return raw or ""


def box_of(cdp) -> dict | None:
    raw = cdp.evaluate(BOX_SCRIPT)
    return json.loads(raw) if raw else None


def navigate_reference(cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.5)


def navigate_engine(cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.wait_for("!!window.__WIZARD__", timeout=30)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.5)


def reference_at(cdp, url: str, section: str) -> dict:
    """参考侧：走到某个步骤（**按生产交互真实点击**）⇒ 采 DOM 与截图。"""
    navigate_reference(cdp, url)
    for selector, action in REFERENCE_PATH[section]:
        if action == "fill":
            fill(cdp, selector, WIZARD_KEY)
            written = read_value(cdp, selector)
            if written != WIZARD_KEY:
                raise RuntimeError(
                    f"参考侧 API Key 输入未生效（读到 {written!r}）⇒ 对照无意义"
                )
        else:
            click(cdp, selector)
    return {
        "dom": cdp.evaluate(NORMALIZE_SCRIPT),
        "box": box_of(cdp),
        "shot": compare.screenshot(cdp, None),
    }


def engine_at(cdp, url: str, section: str) -> dict:
    """引擎侧：设置状态（**受控输入** ⇒ 状态即画面）⇒ 采 DOM 与截图。"""
    tier, index, key = STATES[section]
    navigate_engine(cdp, url)
    # **一个数组参数**（wasm 侧的闭包只拿第一个实参 ⇒ 十个字段打包）。
    payload = json.dumps(
        [
            tier,
            index,
            "deepseek",
            key,
            DEEPSEEK_URL,
            DEEPSEEK_MODEL,
            "",
            True,
            False,
            "",
        ],
        ensure_ascii=False,
    )
    cdp.evaluate("window.__WIZARD__.setState(" + payload + "); true")
    time.sleep(0.35)
    return {
        "dom": cdp.evaluate(NORMALIZE_SCRIPT),
        "box": box_of(cdp),
        "shot": compare.screenshot(cdp, None),
        "state": json.loads(
            cdp.evaluate("JSON.stringify(window.__WIZARD__.getState())") or "{}"
        ),
    }


def diff_dom(left: str | None, right: str | None) -> list[str]:
    """列出两侧 DOM 的差异点（**逐条**）。"""
    try:
        left_tree = json.loads(left or "null")
        right_tree = json.loads(right or "null")
    except json.JSONDecodeError:
        return ["DOM 不是合法 JSON"]
    if left_tree == right_tree:
        return []
    return compare_toolcard.describe_dom_diff(left_tree, right_tree, "根")


def collect_pair(panel: dict, section: str, port: int) -> dict:
    """跑一对（面 × 分区）的对照（**共享脚本的约定入口**）。

    返回值里必须有 `passed`（布尔）与判据明细 ⇒ 共享脚本据此汇总。
    """
    section_id = section["id"] if isinstance(section, dict) else str(section)
    if section_id not in STATES:
        raise RuntimeError(f"未知分区：{section_id}")
    env = ensure_env(port)
    cdp = env["cdp"]
    base = env["base"]

    reference = reference_at(cdp, f"{base}/web/reference/wizard/index.html", section_id)
    engine = engine_at(cdp, f"{base}/web/engine/wizard/index.html", section_id)

    pixel = compare.pixel_diff(reference["shot"], engine["shot"])
    dom_diffs = diff_dom(reference["dom"], engine["dom"])
    ref_box = reference["box"] or {}
    eng_box = engine["box"] or {}
    same_width = (
        ref_box.get("w") is not None
        and eng_box.get("w") is not None
        and abs(float(ref_box["w"]) - float(eng_box["w"])) <= 0.5
    )

    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    for name, data in (("reference", reference), ("engine", engine)):
        (OUT / f"{name}-{section_id}.png").write_bytes(data["shot"])
        (EVIDENCE / f"{name}-{section_id}.png").write_bytes(data["shot"])

    passed = (
        (not pixel["size_mismatch"])
        and pixel["ratio"] <= PIXEL_BUDGET
        and not dom_diffs
        and same_width
    )
    return {
        "passed": passed,
        "section": section_id,
        "t1_pixel_ratio": pixel["ratio"],
        "t1_pixel_different": pixel["different"],
        "t1_pixel_total": pixel["total"],
        "t2_dom_equal": not dom_diffs,
        "t2_dom_diffs": dom_diffs[:8],
        "t3_same_width": same_width,
        "t3_reference_width": ref_box.get("w"),
        "t3_engine_width": eng_box.get("w"),
        "engine_state": engine["state"],
    }


def main() -> int:
    """本面独立跑（六个步骤全跑一遍 ⇒ 写全量证据）。"""
    env = ensure_env(8787)
    cdp = env["cdp"]
    report: dict = {
        "face": "wizard（首次运行向导 · 设置页 W1 面）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": "UTC",
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "crop": "viewport（遮罩是 position: fixed ⇒ 整视口，不裁剪）",
        "reference_page": "/web/reference/wizard/index.html",
        "engine_page": "/web/engine/wizard/index.html",
        "api_key": "（两侧同值的固定测试值）",
        "steps": list(STATES),
    }
    try:
        pairs = []
        for section_id in STATES:
            record = collect_pair({"id": "wizard"}, section_id, 8787)
            pairs.append(record)
            log(
                f"  {section_id}：像素 {record['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if record['t2_dom_equal'] else '不一致'} / "
                f"宽度 {'相等' if record['t3_same_width'] else '不等'}"
            )
        report["pairs"] = pairs
        report["checks"] = {
            "t1_all_pixels_within_budget": all(
                row["t1_pixel_ratio"] <= PIXEL_BUDGET for row in pairs
            ),
            "t2_dom_identical": all(row["t2_dom_equal"] for row in pairs),
            "t3_same_width": all(row["t3_same_width"] for row in pairs),
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T2 的归一化：剔除 Svelte 作用域 class 与注释锚点、class token 排序；"
            "**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3：向导遮罩是 `position: fixed; inset: 0` ⇒ 两侧都是整视口 ⇒ 裁剪框"
            "宽度恒为 1440px。",
            "参考侧用**真实点击**驱动（生产组件的交互路径）；引擎侧用**状态**驱动"
            "（受控输入）⇒ 两侧画的是同一状态。",
            "环境：视口 1440×900、DPR 1、时区 UTC。",
        ]
        report["passed"] = all(report["checks"].values())

        # 浏览器控制台（两侧各一次）。
        console = []
        for name, page in (
            ("reference", "web/reference/wizard"),
            ("engine", "web/engine/wizard"),
        ):
            cdp.drain()
            if name == "reference":
                navigate_reference(cdp, f"{env['base']}/{page}/index.html")
            else:
                navigate_engine(cdp, f"{env['base']}/{page}/index.html")
            time.sleep(0.4)
            messages = []
            for message in cdp.drain():
                method = message.get("method")
                if method == "Runtime.consoleAPICalled":
                    params = message.get("params", {})
                    texts = [
                        str(arg.get("value", arg.get("description", "")))
                        for arg in params.get("args", [])
                    ]
                    messages.append(
                        {"kind": params.get("type"), "text": " ".join(texts)}
                    )
                elif method == "Runtime.exceptionThrown":
                    details = message.get("params", {}).get("exceptionDetails", {})
                    messages.append(
                        {
                            "kind": "exception",
                            "text": str(details.get("text", "")),
                            "detail": str(
                                details.get("exception", {}).get("description", "")
                            ),
                        }
                    )
                elif method == "Log.entryAdded":
                    entry = message.get("params", {}).get("entry", {})
                    messages.append(
                        {
                            "kind": entry.get("level"),
                            "text": str(entry.get("text", "")),
                        }
                    )
            console.append({"side": name, "messages": messages})
        report["console"] = console

        OUT.mkdir(parents=True, exist_ok=True)
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        (OUT / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (EVIDENCE / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (EVIDENCE / "console.json").write_text(
            json.dumps(console, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        log("")
        log("── 判据 ──")
        for key, value in report["checks"].items():
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        return 0 if report["passed"] else 1
    finally:
        shutdown_env()


if __name__ == "__main__":
    raise SystemExit(main())
