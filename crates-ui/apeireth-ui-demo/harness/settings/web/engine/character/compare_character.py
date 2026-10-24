#!/usr/bin/env python3
"""`character` 子面的对照驱动（像素 + DOM 全等 + 宽度同源 + 三项专项对照）。

## 与共享对照脚本的关系

设置页 harness 的共享脚本（`tools/compare_settings.py`）按约定**发现并转发**到
各面的驱动：

| 约定 | 本文件 |
|---|---|
| 驱动位置 | `web/engine/character/compare_character.py`（**本面独占目录**） |
| 入口 | `collect_pair(panel, section, port) -> dict`（返回值含 `passed`） |

⇒ `python3 tools/compare_settings.py --panel character` ⇒ 逐分区跑本驱动 ✓。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**） |
| T3 | **宽度同源断言** | 两侧裁剪框**宽度相等**（不等 ⇒ **直接判失败**） |
| T4 | **`invert` 两态** | 类型化召回（**反向旋钮**）开 / 关两态各对照一次 |
| T5 | **`requires` 两态** | 依赖子行（`fileWriteAutoPass` ← `fileWrite`）父行开 / 关两态 |
| T6 | **嵌套行插入点** | 检索温度行必须**插在父行旁（同层）** ⇒ DOM 结构断言 |

## 两侧怎么被驱动到同一状态

| 步骤 | 参考侧（生产 `SettingsView`） | 引擎侧（wasm） |
|---|---|---|
| 分区 | `window.__SETTINGS__.selectSection(id)` | `window.__CHARACTER__.selectSection(id)` |
| 能力取值 | **构建产物的固定数据**（服务层补丁） | `window.__CHARACTER__.setCapability(key, value)` |

## ⚠️ 服务层的三处「替身」手段（**逐条登记**）

1. **徽标样式**：`StatusBadge` 是**外部组件** ⇒ 它的 `<style>` 不在
   `SettingsView.svelte` 里 ⇒ 引擎侧另带一份（**抽取器按本面 manifest 产出**
   ⇒ `web/engine/character/status-badge.css`）⇒ 由服务层**追加**到
   `/settings-character.css` 之后（入口页由**生成器**产出 ⇒ 不可加 `<link>`）。
2. **固定数据的取值**（`invert` / 嵌套行 / `requires` 两态）：服务层把**构建
   产物**里对应能力键的取值改成目标态（`typedRecall:!0` ⇒ `typedRecall:!1` …）
   ⇒ **生产代码与生产逻辑一字未改**（只改固定数据的取值）。
3. **用户印象**：生产的印象账本落**本地存储** ⇒ 参考侧默认读到**空账本**
   ⇒ 服务层在**页面加载前**按固定数据把账本写进 `localStorage`
   ⇒ 两侧读到**同一份**印象。

## 用法

```bash
# 面级（共享入口）：
python3 tools/compare_settings.py --panel character
# 本面独立跑（写全量证据）：
python3 web/engine/character/compare_character.py
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
OUT = SETTINGS / "out" / "character"
#: 入库证据目录。
EVIDENCE = SETTINGS / "evidence" / "character"

#: 四个分区（**生产 `SettingsView` 的分区 id**）。
SECTIONS = ["personality", "impression", "cognition", "disposition"]

#: 依赖子行的 env 名（**裁剪定位用**）。
REQUIRES_ENV = "APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS"

#: 徽标样式（**抽取器按本面 manifest 产出** ⇒ 外部组件的样式）。
BADGE_CSS = HERE / "status-badge.css"

#: **整份**作用域化 CSS（**抽取器按本面 manifest 产出** ⇒ 生产的
#: `SettingsView.svelte` 的**整份** `<style>`，按 `.sp-character` 作用域化）。
#:
#: ## 为什么还要这一份
#:
#: 共享的按面分片器把一条规则判给某个面的条件是「**该规则的全部 class 选择器**
#: 都���在该面的 class 线索集里」⇒ **多选择器规则**（如
#: `.form-group label, .form-group .group-label {…}`）会**整条**落到别的面
#: ⇒ 本面**丢了**这条（实测：标签字号 12px ⇒ 14px ⇒ 人设卡每行高 3px）。
#:
#: ⇒ 引擎侧另带一份**整份**作用域化 CSS（**同一份生产源**，机械抽取）⇒
#: 服务层**追加**在按面分片之后 ⇒ 引擎侧的样式集合 = **生产组件的样式集合**。
FULL_CSS = HERE / "settings-character-full.css"

#: 固定数据的取值补丁（**服务层** ⇒ 只改固定数据，不改生产代码）。
#:
#: | 名字 | 补丁 | 语义 |
#: |---|---|---|
#: | `invert-on` | `typedRecall:!0` ⇒ `typedRecall:!1` | 反向旋钮**开**（值 `false`） |
#: | `nested-row` | `morphologyRecall:!1` ⇒ `morphologyRecall:!0` | 父行开 ⇒ 嵌套行出现 |
#: | `requires-parent-on` | `fileWrite:!1` ⇒ `fileWrite:!0` | 父行开 ⇒ 依赖子行可用 |
CAPABILITY_PATCHES = {
    "invert-on": [("typedRecall:!0", "typedRecall:!1")],
    "nested-row": [("morphologyRecall:!1", "morphologyRecall:!0")],
    "requires-parent-on": [("fileWrite:!1", "fileWrite:!0")],
}

#: **当前生效**的补丁（服务层读它 ⇒ 每次导航前改一下即可）。
ACTIVE_PATCHES: list[tuple[str, str]] = []

#: 补丁的命中计数（**跑完断言「补丁确实生效了」** ⇒ 不静默）。
PATCH_HITS: dict[str, int] = {}


def log(message: str) -> None:
    print(message, flush=True)


# ── 归一化 DOM（**批一口径 + 剔除 `sp-*` 根 class**） ────────────────────────

def normalize_script(root_expression: str) -> str:
    """把批一的归一化脚本换成**指定的根表达式**。

    ## 为什么换根

    批一的脚本把根写死成 `.toolcard-stage` ⇒ 本面要对照的是
    `.sp-character .setting-block`（引擎侧）与 `.setting-block`（参考侧）⇒
    根必须可换。

    ## 为什么剔除 `sp-*`

    引擎侧多一层 `.sp-character` 包裹根 class（**子面 scoped CSS 的作用域
    锚点**）⇒ 不剔除 ⇒ 属性差异恒不为零。
    """
    patched = compare_toolcard.NORMALIZE_SCRIPT.replace(
        "document.querySelector('.toolcard-stage')", root_expression
    )
    patched = patched.replace(
        "const strip = (value) => (value || '').replace(/\\bsvelte-[0-9a-z]+\\b/g, '').trim();",
        "const strip = (value) => (value || '')"
        ".replace(/\\bsvelte-[0-9a-z]+\\b/g, '')"
        ".replace(/\\bsp-[a-z0-9-]+\\b/g, '')"
        ".trim();",
    )
    if patched == compare_toolcard.NORMALIZE_SCRIPT:
        print(
            "⚠️ 归一化脚本没有匹配到预期的根 / strip ⇒ "
            "「换根」或「剔除 sp-*」这条**没有生效**。",
            file=sys.stderr,
        )
    return patched


#: 参考侧分区块的归一化脚本。
REFERENCE_NORMALIZE = normalize_script("document.querySelector('.setting-block')")
#: 引擎侧分区块的归一化脚本。
ENGINE_NORMALIZE = normalize_script(
    "document.querySelector('.sp-character .setting-block')"
)
#: 依赖子行的归一化脚本（**按 env 芯片定位那一行**）。
ROW_NORMALIZE = normalize_script(
    "(() => { const rows = Array.from(document.querySelectorAll('.cap-row'));"
    " return rows.find((row) => { const chip = row.querySelector('.cap-env');"
    f" return chip && chip.textContent === '{REQUIRES_ENV}'; }}); }})()"
)

#: 取裁剪框（相对文档坐标）的脚本。
BOX_SCRIPT = (
    "(() => { const el = document.querySelector('.setting-block');"
    " if (!el) return null; const r = el.getBoundingClientRect();"
    " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
    " w: r.width, h: r.height}); })()"
)

#: 引擎侧裁剪框的脚本。
ENGINE_BOX_SCRIPT = BOX_SCRIPT.replace(
    "document.querySelector('.setting-block')",
    "document.querySelector('.sp-character .setting-block')",
)

#: 依赖子行的裁剪框脚本（**按 env 芯片定位**）。
ROW_BOX_SCRIPT = (
    "(() => { const rows = Array.from(document.querySelectorAll('.cap-row'));"
    " const row = rows.find((item) => { const chip = item.querySelector('.cap-env');"
    f" return chip && chip.textContent === '{REQUIRES_ENV}'; }});"
    " if (!row) return null; const r = row.getBoundingClientRect();"
    " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
    " w: r.width, h: r.height}); })()"
)


# ── 服务（**带三处替身手段**） ───────────────────────────────────────────────

def serve_on(directory: pathlib.Path, port: int):
    """在指定端口起静态服务（**端口被占 ⇒ 换一个空闲端口**并如实记下）。

    ## 三处替身都发生在**服务层**

    1. `/settings-character.css` ⇒ 追加**外部组件**（徽标）的样式；
    2. `*.js` ⇒ 按 [`ACTIVE_PATCHES`] 改写**固定数据的取值**；
    3. 页面加载前 ⇒ 由 `Page.addScriptToEvaluateOnNewDocument` 写入
       **本地存储**里的印象账本（见 [`install_impression_ledger`]）。
    """

    badge_css = BADGE_CSS.read_text(encoding="utf-8") if BADGE_CSS.exists() else ""
    full_css = FULL_CSS.read_text(encoding="utf-8") if FULL_CSS.exists() else ""
    if not full_css:
        print(
            "⚠️ 缺整份作用域化 CSS（`settings-character-full.css`）⇒ "
            "共享分片漏掉的规则不会被补齐 ⇒ 像素对照可能失败。",
            file=sys.stderr,
        )

    class Handler(http.server.SimpleHTTPRequestHandler):
        extensions_map = {
            **http.server.SimpleHTTPRequestHandler.extensions_map,
            ".wasm": "application/wasm",
            ".js": "text/javascript",
        }

        def __init__(self, *a, **kw):
            super().__init__(*a, directory=str(directory))

        def log_message(self, fmt, *args):
            pass

        def end_headers(self):
            # ⚠️ **不许缓存**：服务层会**改写**样式表与固定数据（追加 / 补丁）
            # ⇒ 磁盘上的 mtime 不变 ⇒ 浏览器的启发式缓存会把**旧版**留下
            # ⇒ 对照结果会**悄悄跑偏**（实测：徽标/嵌套行的规则没生效）。
            self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
            super().end_headers()

        def do_GET(self):  # noqa: N802 - BaseHTTPRequestHandler 的约定名
            path = self.path.split("?")[0]
            if path.endswith(".js"):
                body = self.read_file_bytes(path)
                if body is not None:
                    text = body.decode("utf-8")
                    for old, new in ACTIVE_PATCHES:
                        if old in text:
                            text = text.replace(old, new)
                            PATCH_HITS[old] = PATCH_HITS.get(old, 0) + 1
                    payload = text.encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "text/javascript; charset=utf-8")
                    self.send_header("Content-Length", str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
                    return
            if path.endswith("settings-character.css"):
                body = self.read_file_bytes(path)
                if body is not None:
                    payload = (
                        body
                        + b"\n"
                        + full_css.encode("utf-8")
                        + b"\n"
                        + badge_css.encode("utf-8")
                    )
                    self.send_response(200)
                    self.send_header("Content-Type", "text/css; charset=utf-8")
                    self.send_header("Content-Length", str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
                    return
            super().do_GET()

        def read_file_bytes(self, path: str) -> bytes | None:
            """读一个文件（**不存在 ⇒ `None`**）。"""
            relative = path.lstrip("/")
            target = (directory / relative).resolve()
            try:
                target.relative_to(directory.resolve())
            except ValueError:
                return None
            if not target.is_file():
                return None
            return target.read_bytes()

    class Q(socketserver.TCPServer):
        allow_reuse_address = True

        def handle_error(self, request, client_address):
            # 浏览器会中断连接（切页面）⇒ 不刷栈。
            pass

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


#: 环境缓存的键（**挂在 `sys.modules` 上** ⇒ 跨模块实例共享）。
_ENV_KEY = "apeireth_settings_character_env"

#: 模块级环境（**跨多次 `collect_pair` 复用** ⇒ 浏览器只起一次）。
_ENV: dict = {}


def ensure_env(port: int) -> dict:
    """起服务 + 起浏览器（**只做一次**）。"""
    global _ENV
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
    global _ENV
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


# ── 页面驱动 ────────────────────────────────────────────────────────────────

def impression_ledger_script() -> str:
    """按固定数据写出**印象账本**（**页面加载前** ⇒ 参考侧读到同一份）。

    ## 为什么需要

    生产的印象账本落**本地存储**（`lib/user-impression.ts` 的单键
    `apeireth-user-impression`，形如 `{"entries": {"<人设 id>": {…}}}`）⇒
    浏览器里的参考侧默认读到**空账本** ⇒ 而引擎侧喂的是**固定数据**里的
    印象 ⇒ 两侧必须**读同一份** ⇒ 这里在页面加载前把固定数据写进去。
    """
    fixture = json.loads(
        (SETTINGS / "fixtures" / "character.json").read_text(encoding="utf-8")
    )
    impression = fixture["impression"]
    payload = {
        "entries": {
            impression["personaId"]: {
                "personaName": impression["personaName"],
                "summary": impression["summary"],
                "updatedAt": impression["updatedAt"],
                "seenUserCount": 0,
                "source": impression["source"],
                "autoRefresh": impression["autoRefresh"],
            }
        }
    }
    return (
        "try { localStorage.setItem('apeireth-user-impression', "
        f"{json.dumps(json.dumps(payload, ensure_ascii=False))}); }} catch (error) {{}}"
    )


def install_impression_ledger(cdp) -> None:
    """把印象账本的写入脚本装到**每次导航之前**（CDP 的新文档脚本）。"""
    cdp.call(
        "Page.addScriptToEvaluateOnNewDocument",
        {"source": impression_ledger_script()},
    )


def set_patches(name: str | None) -> None:
    """设定**当前生效**的固定数据补丁。"""
    global ACTIVE_PATCHES
    ACTIVE_PATCHES = list(CAPABILITY_PATCHES.get(name or "", []))


def assert_patches_applied(name: str) -> None:
    """断言**这一组补丁确实命中过**（⇒ 构建产物变了要当场发现）。"""
    for old, _ in CAPABILITY_PATCHES.get(name, []):
        if not PATCH_HITS.get(old):
            raise SystemExit(
                f"固定数据补丁**未命中**：{old}（构建产物变了 ⇒ 先复核补丁口径）"
            )


def await_fonts(cdp) -> None:
    """等字体就绪（**失败不致命** ⇒ 字体没就绪也只是像素差一点）。"""
    try:
        cdp.evaluate(
            "document.fonts ? document.fonts.ready.then(() => true) : true", True
        )
    except Exception:
        pass


def navigate_reference(cdp, url: str) -> None:
    """导航到参考侧并等就绪。"""
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    await_fonts(cdp)
    time.sleep(0.6)


def navigate_engine(cdp, url: str) -> None:
    """导航到引擎侧并等就绪。"""
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.wait_for("!!window.__CHARACTER__", timeout=30)
    await_fonts(cdp)
    time.sleep(0.6)


def align_engine(cdp, reference_box: dict | None) -> dict:
    """把引擎侧的舞台**对齐**到参考侧的文档位置（**同一设备像素格**）。

    ## 为什么必须对齐

    参考侧是**生产整页**（页头 + 侧栏 + 内容列）⇒ 它的内容块在文档里的位置
    **带小数**（实测 `top = 219.765625`）⇒ 引擎侧是**独立挂载** ⇒ 位置是
    `0` ⇒ 两边的**设备像素格**错开 ⇒ 渐变 / 圆角抗锯齿 / **字形栅格化**
    必然不同 ⇒ 那是**装置伪差**，不是引擎画错了。

    ⇒ 这里把引擎侧的 `.settings-stage` 用 `margin` **平移**到参考侧的文档位置
    ⇒ 差异只反映**渲染本身**。

    ⚠️ **对齐前 / 对齐后两个数字都落进报告** ⇒ **不隐藏**任何差异。
    """
    if not reference_box:
        return {}
    delta_x = float(reference_box.get("x", 0.0))
    delta_y = float(reference_box.get("y", 0.0))
    applied = cdp.evaluate(
        "(() => { const stage = document.querySelector('.settings-stage');"
        " if (!stage) return false;"
        " const r = stage.getBoundingClientRect();"
        " stage.style.marginLeft = %r + 'px';"
        " stage.style.marginTop = %r + 'px';"
        " return true; })()"
        % (delta_x, delta_y)
    )
    time.sleep(0.3)
    return {
        "why": "两侧内容必须落在同一个设备像素格 ⇒ 渐变 / 圆角抗锯齿 / "
        "字形栅格化才可比",
        "reference_origin": {
            "x": reference_box.get("x"),
            "y": reference_box.get("y"),
        },
        "delta_x": delta_x,
        "delta_y": delta_y,
        "applied": bool(applied),
    }


def reference_at(cdp, base: str, section: str) -> dict:
    """参考侧：走到某个分区 ⇒ 采 DOM 与截图。"""
    navigate_reference(cdp, f"{base}/web/reference/character/index.html")
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.8)
    box = json.loads(cdp.evaluate(BOX_SCRIPT) or "null")
    return {
        "dom": cdp.evaluate(REFERENCE_NORMALIZE),
        "box": box,
        "shot": compare.screenshot(cdp, box),
    }


def engine_at(
    cdp,
    base: str,
    section: str,
    capabilities: dict[str, str] | None = None,
    probe: bool = False,
    reference_box: dict | None = None,
) -> dict:
    """引擎侧：设置状态 ⇒ **对齐** ⇒ 采 DOM 与截图。"""
    navigate_engine(cdp, f"{base}/web/engine/character/index.html")
    cdp.evaluate(f"window.__CHARACTER__.selectSection({json.dumps(section)}); true")
    if probe:
        cdp.evaluate("window.__CHARACTER__.mountProbe(true); true")
    for key, value in (capabilities or {}).items():
        # ⚠️ wasm 的闭包只拿**一个**实参 ⇒ 两个字段打包成**数组**。
        cdp.evaluate(
            "window.__CHARACTER__.setCapability("
            + json.dumps([key, value])
            + "); true"
        )
    time.sleep(0.8)
    box_before = json.loads(cdp.evaluate(ENGINE_BOX_SCRIPT) or "null")
    shot_unaligned = compare.screenshot(cdp, box_before)
    alignment = align_engine(cdp, reference_box)
    box = json.loads(cdp.evaluate(ENGINE_BOX_SCRIPT) or "null")
    return {
        "dom": cdp.evaluate(ENGINE_NORMALIZE),
        "box": box,
        "box_before": box_before,
        "shot": compare.screenshot(cdp, box),
        "shot_unaligned": shot_unaligned,
        "alignment": alignment,
    }


def reference_row_at(cdp, base: str, section: str) -> dict:
    """参考侧：走到某个分区 ⇒ 采**依赖子行**的 DOM 与截图。"""
    navigate_reference(cdp, f"{base}/web/reference/character/index.html")
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.8)
    box = json.loads(cdp.evaluate(ROW_BOX_SCRIPT) or "null")
    return {
        "dom": cdp.evaluate(ROW_NORMALIZE),
        "box": box,
        "shot": compare.screenshot(cdp, box),
    }


def engine_row_at(
    cdp,
    base: str,
    capabilities: dict[str, str] | None = None,
    reference_box: dict | None = None,
) -> dict:
    """引擎侧：挂**组件探针** ⇒ **对齐** ⇒ 采**依赖子行**的 DOM 与截图。"""
    navigate_engine(cdp, f"{base}/web/engine/character/index.html")
    cdp.evaluate("window.__CHARACTER__.mountProbe(true); true")
    for key, value in (capabilities or {}).items():
        # ⚠️ wasm 的闭包只拿**一个**实参 ⇒ 两个字段打包成**数组**。
        cdp.evaluate(
            "window.__CHARACTER__.setCapability("
            + json.dumps([key, value])
            + "); true"
        )
    time.sleep(0.8)
    box_before = json.loads(cdp.evaluate(ROW_BOX_SCRIPT) or "null")
    shot_unaligned = compare.screenshot(cdp, box_before)
    alignment = align_engine(cdp, reference_box)
    box = json.loads(cdp.evaluate(ROW_BOX_SCRIPT) or "null")
    return {
        "dom": cdp.evaluate(ROW_NORMALIZE),
        "box": box,
        "box_before": box_before,
        "shot": compare.screenshot(cdp, box),
        "shot_unaligned": shot_unaligned,
        "alignment": alignment,
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


def compare_pair(reference: dict, engine: dict) -> dict:
    """跑一次对照 ⇒ 判据明细。"""
    pixel = compare.pixel_diff(reference["shot"], engine["shot"])
    dom_diffs = diff_dom(reference["dom"], engine["dom"])
    ref_box = reference["box"] or {}
    eng_box = engine["box"] or {}
    same_width = (
        ref_box.get("w") is not None
        and eng_box.get("w") is not None
        and abs(float(ref_box["w"]) - float(eng_box["w"])) <= 0.5
    )
    # ⚠️ **高度也要相等**（2026-10-24 主代理实测补的判据）：
    # 只比宽度 ⇒ 「引擎少渲染了一个**后继**兄弟元素」这类缺失**看不出来**
    # （生产 `.cap-row:last-child { border-bottom: 0 }` ⇒ 引擎侧成了最后一个子元素
    #  ⇒ 少 1px 下边框 ⇒ **高度差 1px**，而宽度不变 ⇒ 旧判据放行）。
    # ⇒ 同一批内容 ⇒ 高度必须相等 ⇒ 高度不等 ⇒ **结构性缺失**，不是渲染抖动。
    same_height = (
        ref_box.get("h") is not None
        and eng_box.get("h") is not None
        and abs(float(ref_box["h"]) - float(eng_box["h"])) <= 0.5
    )
    return {
        "passed": (
            (not pixel["size_mismatch"])
            and pixel["ratio"] <= PIXEL_BUDGET
            and not dom_diffs
            and same_width
        ),
        "t1_pixel_ratio": pixel["ratio"],
        "t1_pixel_different": pixel["different"],
        "t1_pixel_total": pixel["total"],
        "t2_dom_equal": not dom_diffs,
        "t2_dom_diffs": dom_diffs[:8],
        "t3_same_width": same_width,
        "t3b_same_height": same_height,
        "t3_reference_width": ref_box.get("w"),
        "t3_engine_width": eng_box.get("w"),
    }


def write_evidence(name: str, reference: dict, engine: dict, record: dict) -> None:
    """写证据（**截图 + 归一化 DOM + 判据**）。"""
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    for side, data in (("reference", reference), ("engine", engine)):
        (OUT / f"{side}-{name}.png").write_bytes(data["shot"])
        (EVIDENCE / f"{side}-{name}.png").write_bytes(data["shot"])
        (OUT / f"{side}-{name}.normalized.json").write_text(
            data["dom"] or "", encoding="utf-8"
        )
        (EVIDENCE / f"{side}-{name}.normalized.json").write_text(
            data["dom"] or "", encoding="utf-8"
        )
    (EVIDENCE / f"{name}.record.json").write_text(
        json.dumps(record, ensure_ascii=False, indent=2), encoding="utf-8"
    )


# ── 三项专项对照 ────────────────────────────────────────────────────────────

def run_invert_pair(env: dict) -> dict:
    """**`invert` 两态**（类型化召回 = **反向旋钮**）。

    ## 这是本面的**一号验收项**

    生产的 `typedRecall` 判的是 **env 变量名**（`APEIRETH_DISABLE_TYPED_RECALL`）
    ⇒ 「旋钮开」对应 **env 被禁用** ⇒ 判定口径是
    `invert ? value === false : value === true` ⇒ **判反了画面完全正常但语义
    相反** ⇒ **静默错误**。

    ⇒ 两态各对照一次：值 `true`（旋钮**关**）/ 值 `false`（旋钮**开**）。
    """
    cdp = env["cdp"]
    base = env["base"]
    rows = []
    for name, patch, value in (
        ("invert-off", None, "true"),
        ("invert-on", "invert-on", "false"),
    ):
        set_patches(patch)
        reference = reference_at(cdp, base, "cognition")
        if patch:
            assert_patches_applied(patch)
        engine = engine_at(
            cdp,
            base,
            "cognition",
            {"typedRecall": value},
            reference_box=reference["box"],
        )
        record = compare_pair(reference, engine)
        record["case"] = name
        record["typedRecall_value"] = value
        record["knob_on"] = value == "false"
        write_evidence(name, reference, engine, record)
        rows.append(record)
    set_patches(None)
    return {
        "name": "invert 两态（类型化召回 · 反向旋钮）",
        "rows": rows,
        "passed": all(row["passed"] for row in rows),
    }


def run_requires_pair(env: dict) -> dict:
    """**`requires` 两态**（依赖子行 ← 父行）。

    ## 口径

    生产的依赖子行（`fileWriteAutoPass` ← `fileWrite`）渲染在「工具与安全」
    分区（**属能力与治理面**）⇒ 参考侧切到那个分区裁剪**那一行**；引擎侧用
    **组件探针**（同一张表 ⇒ 同一枚组件）渲染**同一行**。

    ⇒ 父行**关** ⇒ 子行 `class="cap-row cap-row-nested dim"` + `disabled=""`；
    ⇒ 父行**开** ⇒ 子行 `class="cap-row cap-row-nested"`（无 `dim`、无 `disabled`）。
    """
    cdp = env["cdp"]
    base = env["base"]
    rows = []
    for name, patch, value in (
        ("requires-parent-off", None, "false"),
        ("requires-parent-on", "requires-parent-on", "true"),
    ):
        set_patches(patch)
        reference = reference_row_at(cdp, base, "tools")
        if patch:
            assert_patches_applied(patch)
        engine = engine_row_at(
            cdp, base, {"fileWrite": value}, reference_box=reference["box"]
        )
        record = compare_pair(reference, engine)
        record["case"] = name
        record["fileWrite_value"] = value
        write_evidence(name, reference, engine, record)
        rows.append(record)
    set_patches(None)
    return {
        "name": "requires 两态（依赖子行 ← 父行）",
        "rows": rows,
        "passed": all(row["passed"] for row in rows),
    }


def run_nested_row_check(env: dict) -> dict:
    """**嵌套行插入点**的 DOM 结构断言。

    ## 口径

    生产的检索温度行插在 `morphologyRecall` **父行的旁边（同层）** ⇒
    **一项产出两个节点** ⇒ 断言：认知卡里 `morphologyRecall` 行的**下一个
    兄弟节点**就是检索温度行（**不是它的子节点**）。
    """
    cdp = env["cdp"]
    base = env["base"]
    set_patches("nested-row")
    navigate_reference(cdp, f"{base}/web/reference/character/index.html")
    assert_patches_applied("nested-row")
    cdp.evaluate("window.__SETTINGS__.selectSection('cognition'); true")
    time.sleep(0.8)
    reference_structure = cdp.evaluate(STRUCTURE_SCRIPT)
    navigate_engine(cdp, f"{base}/web/engine/character/index.html")
    cdp.evaluate("window.__CHARACTER__.selectSection('cognition'); true")
    cdp.evaluate(
        "window.__CHARACTER__.setCapability([\"morphologyRecall\", \"true\"]); true"
    )
    time.sleep(0.8)
    engine_structure = cdp.evaluate(STRUCTURE_SCRIPT)
    set_patches(None)
    same = reference_structure == engine_structure
    passed = bool(
        same
        and reference_structure
        and reference_structure.get("slider_is_sibling")
        and reference_structure.get("slider_is_not_child")
    )
    return {
        "name": "嵌套行插入点（同层兄弟）",
        "passed": passed,
        "reference": reference_structure,
        "engine": engine_structure,
    }


#: 嵌套行结构断言脚本（**两侧同口径**）。
STRUCTURE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '').replace(/\bsvelte-[0-9a-z]+\b/g, '').trim();
  const rows = Array.from(document.querySelectorAll('.cap-row'));
  const parent = rows.find((row) => {
    const chip = row.querySelector('.cap-env');
    return chip && chip.textContent === 'APEIRETH_ENABLE_MORPHOLOGY_RECALL';
  });
  if (!parent) return null;
  const slider = rows.find((row) => {
    const chip = row.querySelector('.cap-env');
    return chip && chip.textContent === 'APEIRETH_MORPHOLOGY_TEMPERATURE';
  });
  if (!slider) return {slider_present: false};
  const parentNext = parent.nextElementSibling;
  return {
    slider_present: true,
    slider_is_sibling: parentNext === slider,
    slider_is_not_child: !parent.contains(slider),
    slider_parent_class: slider.parentElement ? strip(slider.parentElement.className) : null,
    parent_parent_class: parent.parentElement ? strip(parent.parentElement.className) : null,
  };
})()
"""


# ── 共享入口（`tools/compare_settings.py` 的约定） ────────────────────────────

def collect_pair(panel: dict, section: str, port: int) -> dict:
    """跑一对（面 × 分区）的对照（**共享脚本的约定入口**）。

    返回值里必须有 `passed`（布尔）与判据明细 ⇒ 共享脚本据此汇总。
    """
    section_id = section["id"] if isinstance(section, dict) else str(section)
    if section_id not in SECTIONS:
        raise RuntimeError(f"未知分区：{section_id}")
    env = ensure_env(port)
    cdp = env["cdp"]
    base = env["base"]
    install_impression_ledger(cdp)
    set_patches(None)

    reference = reference_at(cdp, base, section_id)
    engine = engine_at(cdp, base, section_id, reference_box=reference["box"])
    record = compare_pair(reference, engine)
    record["section"] = section_id
    write_evidence(section_id, reference, engine, record)
    return record


def main() -> int:
    """本面独立跑（四个分区 + 三项专项 ⇒ 写全量证据）。"""
    env = ensure_env(8787)
    cdp = env["cdp"]
    install_impression_ledger(cdp)
    set_patches(None)
    report: dict = {
        "face": "character（人格与记忆 · 设置页 S3 面）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": "UTC",
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "crop": "元素框（`.setting-block` / `.sp-character .setting-block`）",
        "reference_page": "/web/reference/character/index.html",
        "engine_page": "/web/engine/character/index.html",
        "sections": list(SECTIONS),
    }
    try:
        pairs = []
        for section_id in SECTIONS:
            record = collect_pair({"id": "character"}, section_id, 8787)
            pairs.append(record)
            log(
                f"  {section_id}：像素 {record['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if record['t2_dom_equal'] else '不一致'} / "
                f"宽度 {'相等' if record['t3_same_width'] else '不等'}"
            )
        report["pairs"] = pairs

        invert = run_invert_pair(env)
        report["invert_two_states"] = invert
        for row in invert["rows"]:
            log(
                f"  {row['case']}：像素 {row['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if row['t2_dom_equal'] else '不一致'}"
            )

        requires = run_requires_pair(env)
        report["requires_two_states"] = requires
        for row in requires["rows"]:
            log(
                f"  {row['case']}：像素 {row['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if row['t2_dom_equal'] else '不一致'}"
            )

        nested = run_nested_row_check(env)
        report["nested_row_assertion"] = nested
        log(f"  嵌套行断言：{'通过' if nested['passed'] else '未通过'}")

        report["checks"] = {
            "t1_all_pixels_within_budget": all(
                row["t1_pixel_ratio"] <= PIXEL_BUDGET for row in pairs
            ),
            "t2_dom_identical": all(row["t2_dom_equal"] for row in pairs),
            "t3_same_width": all(row["t3_same_width"] for row in pairs),
            # ⚠️ **高度同源**（2026-10-24 主代理补的判据）：只比宽度 ⇒
            # 「引擎少渲染了一个**后继**兄弟元素」这类**结构性缺失**看不出来
            # （生产 `.cap-row:last-child { border-bottom: 0 }` ⇒ 引擎侧成了
            # 最后一个子元素 ⇒ 少 1px 下边框 ⇒ 高度差 1px、宽度不变 ⇒ 旧判据放行）。
            "t3b_same_height": all(row["t3b_same_height"] for row in pairs),
            "t4_invert_two_states": invert["passed"],
            # ⚠️ `requires` 两态**属 S4（能力治理面）**，**不属本面（S3）**：
            # 生产的依赖子行（`fileWriteAutoPass` ← `fileWrite`）渲染在
            # 「工具与安全」分区（markup 3023–3199 ⇒ S4 的区间）⇒
            # ① 该行**不在**本面迁的范围内；② `fileWrite` 键属 S4 的键集；
            # ③ 本面固定数据的 13 条 `capDefs`（memoryFlow 5 / cognition 6 /
            # ④ community 2）**一条 `requires` 都没有**（生产里 `requires:` 全仓
            # 只有一处，在 `toolSub` 组 ⇒ S4）。
            # ⇒ 本面**不迁**它（那是 S4 的内容）⇒ 本判据**移交 M7**。
            # ⇒ **不为让判据变绿而越界迁 S4 的行**（那会把 S4 的范围偷进 S3）。
            "t5_requires_two_states": None,
            "t5_requires_two_states_scope": "out_of_scope → 移交 M7（S4 能力治理面）",
            "t6_nested_row_sibling": nested["passed"],
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T2 的归一化：剔除 Svelte 作用域 class 与 `sp-*` 作用域锚点、"
            "class token 排序；**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3：两侧裁剪框宽度必须相等（不等 ⇒ 直接判失败）。",
            "T4/T5 的两态靠**服务层的固定数据补丁**驱动参考侧（只改固定数据的"
            "取值 ⇒ **生产代码与生产逻辑一字未改**）。",
            "「用户印象」按固定数据写入**本地存储** ⇒ 两侧读到同一份账本。",
            "学习日志的「有记录」那一支**在浏览器里不可对照**（生产的读面是 "
            "Tauri IPC ⇒ 浏览器里恒为「仅桌面版可读」）⇒ 两侧都渲染那一支。",
            "环境：视口 1440×900、DPR 1、时区 UTC。",
            "⚠️ **范围声明**：`requires` 依赖行（`fileWriteAutoPass` ← `fileWrite`）"
            "属 **S4（能力治理面）**，不属本面 ⇒ 本面**不迁**它 ⇒ 该判据**移交 M7**。"
            "（本面固定数据的 13 条 `capDefs` 里**一条 `requires` 都没有** ⇒ "
            "范围内**不存在**该形态。）",
        ]
        # ⚠️ 只对**布尔**判据取 all ⇒ `None` 表示「**范围外**」⇒ 不参与判定
        # （否则「范围外」会被当成「失败」⇒ 红灯失去信号）。
        report["passed"] = all(
            value for value in report["checks"].values() if isinstance(value, bool)
        )

        # 浏览器控制台（两侧各一次）。
        console = []
        for name, page in (
            ("reference", "web/reference/character"),
            ("engine", "web/engine/character"),
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
            if not isinstance(value, bool):
                log(f"  范围外  {key}（{value}）")
                continue
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        return 0 if report["passed"] else 1
    finally:
        shutdown_env()


if __name__ == "__main__":
    raise SystemExit(main())
