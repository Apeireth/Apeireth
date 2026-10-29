#!/usr/bin/env python3
"""`capability` 子面的对照驱动（像素 + DOM 全等 + 宽高同源 + 四项专项对照）。

## 与共享对照脚本的关系

设置页 harness 的共享脚本（`tools/compare_settings.py`）按约定**发现并转发**到
各面的驱动：

| 约定 | 本文件 |
|---|---|
| 驱动位置 | `web/engine/capability/compare_capability.py`（**本面独占目录**） |
| 入口 | `collect_pair(panel, section, port) -> dict`（返回值含 `passed`） |
| 独立 CLI | `main() -> int`（跑三个分区 + 全部态用例 ⇒ 写全量证据） |

⇒ `python3 tools/compare_settings.py --panel capability` ⇒ 逐分区跑本驱动 ✓。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 差异像素 **= 0**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**） |
| T3 | **宽 + 高同源断言** | 两侧裁剪框**宽度与高度都相等**（不等 ⇒ 直接判失败） |
| T4 | **原生控件属性** | 两侧 `select` / `input` 的属性集**逐字相等**；且**都不带** `value` 属性 |
| T5 | **配置 JSON 预览** | 两侧 `pre.code-box` 的文本**逐字节相同**（并与 `json.dumps(indent=2)` 交叉校验） |
| T6 | **`requires` 两态** | `fileWriteAutoPass` ← `fileWrite`：父行**关 / 开**两态各对照一次 |
| T7 | **嵌套行插入点 + MCP 行** | DOM 结构断言（子行是父行的**下一个兄弟**、MCP 行是卡内**最后一个元素**） |

## 两侧怎么被驱动到同一状态

| 步骤 | 参考侧（生产 `SettingsView`） | 引擎侧（wasm） |
|---|---|---|
| 分区 | `window.__SETTINGS__.selectSection(id)` | `window.__CAPABILITY__.selectSection(id)`（**共享入口也已被包裹** ⇒ 两条路都行） |
| 能力开关 | **点生产自己的行**（`button.cap-row`）⇒ 走生产交互路径 | `window.__CAPABILITY__.toggleCapability(key)`（**同一条语义**：翻转 + judge/council ⇒ 深度落 custom） |
| 原生控件取值 | 生产自己维护（`bind:value` / `value={…}`） | `window.__CAPABILITY__.setText([key, value])` ⇒ 宿主回写 property |
| 允许服务器 | 输入 + 点「添加」（走生产的 mock 数据面） | `window.__CAPABILITY__.setServers([...])` |
| 配置 JSON 预览 | 生产 `JSON.stringify` | 宿主生成（`ui_kit::json` + `num`）⇒ **逐字节**对照 |

## ⚠️ 服务层的一处「替身」手段（**逐条登记**）

**整份作用域化 CSS**：共享的按面分片器把一条规则判给某个面的条件是「该规则的
**全部** class 选择器都在该面的 class 线索集里」⇒ **多选择器规则**（如
`.form-group label, .form-group .group-label {…}`）会**整条**落到别的面 ⇒
本面丢了那条（同一现象 M6 已实测：标签字号 12px ⇒ 14px）。⇒ 引擎侧另带一份
**整份**作用域化 CSS（**同一份生产源**，`extract-assets.mjs --manifest` 机械抽取
⇒ `settings-capability-full.css`）⇒ 服务层**追加**在按面分片之后（入口页由生成器
产出 ⇒ 不可加 `<link>`）。

## 用法

```bash
# 面级（共享入口）：
python3 tools/compare_settings.py --panel capability
# 本面独立跑（写全量证据）：
python3 web/engine/capability/compare_capability.py
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
    import compare
    import compare_toolcard  # noqa: E402
    import interaction  # noqa: E402
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
#: ⚠️ 本面的像素判据是 **0**（不是 1%）：施工方案的验收口径写「差异 0」。
PIXEL_BUDGET = 0.0

#: 产物目录（**不入库**）。
OUT = SETTINGS / "out" / "capability"
#: 入库证据目录。
EVIDENCE = SETTINGS / "evidence" / "capability"

#: 三个分区（**生产 `SettingsView` 的分区 id**）。
SECTIONS = ["governance", "tools", "developer"]

#: 依赖子行的 env 名（**裁剪定位用**）。
REQUIRES_ENV = "APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS"

#: MCP 静态行的 env 名（**T7 的锚点**）。
MCP_ENV = "APEIRETH_ENABLE_MCP"

#: **整份**作用域化 CSS（**抽取器按本面 manifest 产出**）。
FULL_CSS = HERE / "settings-capability-full.css"

#: 参考侧分区块的归一化脚本。
REFERENCE_NORMALIZE = compare_toolcard.NORMALIZE_SCRIPT.replace(
    "document.querySelector('.toolcard-stage')",
    "document.querySelector('.setting-block')",
).replace(
    "const strip = (value) => (value || '').replace(/\\bsvelte-[0-9a-z]+\\b/g, '').trim();",
    "const strip = (value) => (value || '')"
    ".replace(/\\bsvelte-[0-9a-z]+\\b/g, '')"
    ".replace(/\\bsp-[a-z0-9-]+\\b/g, '')"
    ".trim();",
)

#: 引擎侧分区块的归一化脚本。
ENGINE_NORMALIZE = compare_toolcard.NORMALIZE_SCRIPT.replace(
    "document.querySelector('.toolcard-stage')",
    "document.querySelector('.sp-capability .setting-block')",
).replace(
    "const strip = (value) => (value || '').replace(/\\bsvelte-[0-9a-z]+\\b/g, '').trim();",
    "const strip = (value) => (value || '')"
    ".replace(/\\bsvelte-[0-9a-z]+\\b/g, '')"
    ".replace(/\\bsp-[a-z0-9-]+\\b/g, '')"
    ".trim();",
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
    "document.querySelector('.sp-capability .setting-block')",
)

#: 原生控件的属性快照（**T4**）：`标签|id|aria-label|属性名=属性值…`（保序）。
CONTROLS_SCRIPT = r"""
(() => {
  const root = document.querySelector(%(root)s);
  if (!root) return JSON.stringify([]);
  const rows = [];
  for (const el of Array.from(root.querySelectorAll('select, input'))) {
    const attrs = Array.from(el.attributes)
      // ⚠️ `class` 剔除：两边的作用域 token 不同（`svelte-…` / 无）
      // ⇒ T2 已按 token 归一化，T4 只看**属性存在性与取值**。
      .filter((attr) => attr.name !== 'class' && !attr.name.startsWith('svelte-'))
      .map((attr) => `${attr.name}=${attr.value}`)
      .sort();
    rows.push({
      tag: el.tagName.toLowerCase(),
      id: el.getAttribute('id') || '',
      aria: el.getAttribute('aria-label') || '',
      value_attr: el.hasAttribute('value'),
      attrs,
    });
  }
  return JSON.stringify(rows);
})()
"""

#: 配置 JSON 预览的原文（**T5**）。
PREVIEW_SCRIPT = (
    "(() => { const root = document.querySelector(%s);"
    " if (!root) return null;"
    " const pre = root.querySelector('pre.code-box');"
    " return pre ? pre.textContent : null; })()"
)

#: 结构断言（**T7**）：依赖子行是父行的**下一个兄弟**；MCP 行是卡内**最后一个元素**。
STRUCTURE_SCRIPT = r"""
(() => {
  // ⚠️ class **按 token 归一化**（剔除 `svelte-…` / `sp-…`、排序、单空格连接）：
  // 生产用 `class:dim={…}` 追加 token（`"cap-row cap-row-nested  dim"` 有**两个**
  // 空格），引擎用 `join(" ")`（一个空格）⇒ 原始 className 串**必然不同**，
  // 但 token 集合相同 ⇒ 判据比 token 集合，不比原始串。
  const strip = (value) => (value || '')
    .replace(/\bsvelte-[0-9a-z]+\b/g, '')
    .replace(/\bsp-[a-z0-9-]+\b/g, '')
    .split(/\s+/).filter(Boolean).sort().join(' ');
  const rows = Array.from(document.querySelectorAll('.cap-row'));
  const byEnv = (env) => rows.find((row) => {
    const chip = row.querySelector('.cap-env');
    return chip && chip.textContent === env;
  });
  const parent = byEnv('APEIRETH_ENABLE_FILE_WRITE');
  const sub = byEnv('APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS');
  const mcp = byEnv('APEIRETH_ENABLE_MCP');
  if (!parent || !sub) return null;
  const card = parent.closest('.cap-card');
  const cardChildren = card
    ? Array.from(card.children).filter((node) => node.nodeType === Node.ELEMENT_NODE)
    : [];
  return {
    sub_is_next_sibling: parent.nextElementSibling === sub,
    sub_is_not_child: !parent.contains(sub),
    sub_class: strip(sub.className),
    sub_disabled: sub.hasAttribute('disabled'),
    parent_class: strip(parent.className),
    mcp_present: !!mcp,
    mcp_is_last_element: mcp ? cardChildren[cardChildren.length - 1] === mcp : false,
    mcp_is_static: mcp ? mcp.classList.contains('cap-row-static') : false,
    card_last_class: cardChildren.length
      ? strip(cardChildren[cardChildren.length - 1].className)
      : null,
  };
})()
"""


def log(message: str) -> None:
    print(message, flush=True)


# ── 服务（**带一处替身手段：整份作用域化 CSS**） ────────────────────────────

def serve_on(directory: pathlib.Path, port: int):
    """在指定端口起静态服务（**端口被占 ⇒ 换一个空闲端口**并如实记下）。

    ## 替身发生在**服务层**

    `/settings-capability.css` ⇒ 追加**整份作用域化 CSS**（同一份生产源，机械抽取）
    ⇒ 引擎侧的样式集合 = 生产组件的样式集合（补齐共享按面分片漏掉的多选择器规则）。
    """

    full_css = FULL_CSS.read_text(encoding="utf-8") if FULL_CSS.exists() else ""
    if not full_css:
        print(
            "⚠️ 缺整份作用域化 CSS（`settings-capability-full.css`）⇒ "
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
            # ⚠️ **不许缓存**：服务层会**改写**样式表 ⇒ 磁盘上的 mtime 不变
            # ⇒ 浏览器的启发式缓存会把**旧版**留下 ⇒ 对照结果会悄悄跑偏。
            self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
            super().end_headers()

        def do_GET(self):  # noqa: N802 - BaseHTTPRequestHandler 的约定名
            path = self.path.split("?")[0]
            if path.endswith("settings-capability.css"):
                body = self.read_file_bytes(path)
                if body is not None:
                    payload = body + b"\n" + full_css.encode("utf-8")
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
_ENV_KEY = "apeireth_settings_capability_env"

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

def await_fonts(cdp) -> None:
    """等字体就绪（**失败不致命** ⇒ 字体没就绪也只是像素差一点）。"""
    try:
        cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
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
    cdp.wait_for("!!window.__CAPABILITY__", timeout=30)
    await_fonts(cdp)
    time.sleep(0.6)


def align_engine(cdp, reference_box: dict | None) -> dict:
    """把引擎侧的舞台**对齐**到参考侧的文档位置（**同一设备像素格**）。

    ## 为什么必须对齐

    参考侧是**生产整页**（页头 + 侧栏 + 内容列）⇒ 它的内容块在文档里的位置
    **带小数**（实测 `top = 219.765625`）⇒ 引擎侧是**独立挂载** ⇒ 位置是 `0`
    ⇒ 两边的**设备像素格**错开 ⇒ 渐变 / 圆角抗锯齿 / **字形栅格化**必然不同
    ⇒ 那是**装置伪差**，不是引擎画错了。

    ⇒ 这里把引擎侧的 `.settings-stage` 用 `margin` **平移**到参考侧的文档位置
    ⇒ 差异只反映**渲染本身**。**对齐前 / 对齐后两个数字都落进报告**。
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


def _value(raw):
    """CDP 返回值 ⇒ 结构（`returnByValue` 已经把 JSON 解析成对象 ⇒ 两种都吃）。"""
    if raw is None or isinstance(raw, (dict, list)):
        return raw
    return json.loads(raw)


def reference_at(cdp, base: str, section: str, scripts=None) -> dict:
    """参考侧：走到某个分区（可选：跑**多步**生产交互）⇒ 采 DOM / 截图 / 控件 / 预览。

    ⚠️ **每一步都要等一帧**：生产的开关点击是**异步**的（`applyCapabilityNow`
    `await onSave`）⇒ 同一段脚本里连点两下 ⇒ 第二个按钮**还没渲染出来**（或仍是
    `disabled=""`）⇒ 点击**静默失效** ⇒ 参考侧状态与预期不符（实测踩过）。
    """
    steps = scripts or []
    if isinstance(steps, str):
        steps = [steps]
    navigate_reference(cdp, f"{base}/web/reference/capability/index.html")
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.6)
    for script in steps:
        cdp.evaluate(f"{script}; true")
        time.sleep(0.9)
    box = json.loads(cdp.evaluate(BOX_SCRIPT) or "null")
    return {
        "dom": cdp.evaluate(REFERENCE_NORMALIZE),
        "box": box,
        "shot": compare.screenshot(cdp, box),
        "controls": json.loads(cdp.evaluate(CONTROLS_SCRIPT % {"root": "'.setting-block'"}) or "[]"),
        "preview": cdp.evaluate(PREVIEW_SCRIPT % "'.setting-block'"),
        "structure": _value(cdp.evaluate(STRUCTURE_SCRIPT)),
    }


def engine_at(
    cdp,
    base: str,
    section: str,
    scripts: list[str] | None = None,
    reference_box: dict | None = None,
) -> dict:
    """引擎侧：设置状态 ⇒ **对齐** ⇒ 采 DOM / 截图 / 控件 / 预览。"""
    navigate_engine(cdp, f"{base}/web/engine/capability/index.html")
    # ⚠️ 走**本面控制 API**（`__SETTINGS__.selectSection` 也已被包裹 ⇒ 两条路都行，
    # 见 `shared_api_selection` 断言）。
    cdp.evaluate(f"window.__CAPABILITY__.selectSection({json.dumps(section)}); true")
    time.sleep(0.3)
    for script in scripts or []:
        cdp.evaluate(f"{script}; true")
        time.sleep(0.15)
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
        "controls": json.loads(
            cdp.evaluate(CONTROLS_SCRIPT % {"root": "'.sp-capability .setting-block'"}) or "[]"
        ),
        "preview": cdp.evaluate(PREVIEW_SCRIPT % "'.sp-capability .setting-block'"),
        "structure": _value(cdp.evaluate(STRUCTURE_SCRIPT)),
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
    # ⚠️ **高度也要相等**：只比宽度 ⇒ 「引擎少渲染了一个**后继**兄弟元素」这类
    # 结构性缺失看不出来（生产 `.cap-row:last-child { border-bottom: 0 }`
    # ⇒ 引擎侧成了最后一个子元素 ⇒ 少 1px 下边框 ⇒ **高度差 1px**，宽度不变）。
    same_height = (
        ref_box.get("h") is not None
        and eng_box.get("h") is not None
        and abs(float(ref_box["h"]) - float(eng_box["h"])) <= 0.5
    )
    # T4：原生控件的属性集**逐字相等**（T2 已含属性，这里单列一条可复核的快照）。
    controls_equal = reference["controls"] == engine["controls"]
    # T5：配置 JSON 预览**逐字节相同**。
    preview_equal = reference["preview"] == engine["preview"]
    return {
        "passed": (
            (not pixel["size_mismatch"])
            and pixel["ratio"] <= PIXEL_BUDGET
            and not dom_diffs
            and same_width
            and same_height
            and controls_equal
            and preview_equal
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
        "t3_reference_height": ref_box.get("h"),
        "t3_engine_height": eng_box.get("h"),
        "t4_controls_equal": controls_equal,
        "t4_reference_controls": reference["controls"],
        "t4_engine_controls": engine["controls"],
        "t5_preview_equal": preview_equal,
        "t5_reference_preview": reference["preview"],
        "t5_engine_preview": engine["preview"],
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


# ── 态用例（**参考侧走生产交互路径** ⇒ 不改生产代码、不改固定数据） ──────────

#: 生产交互脚本（参考侧）。
CLICK_ROW = (
    "(() => { const label = %s;"
    " const button = Array.from(document.querySelectorAll('button.cap-row'))"
    ".find((row) => row.getAttribute('aria-label') === label);"
    " if (!button) return false; button.click(); return true; })()"
)

#: 态用例：`(名字, 分区, 参考侧脚本列表, 引擎侧脚本列表, 覆盖的判据)`
#:
#: ⚠️ 参考侧是**多步**：每步之间要等一帧（生产的开关点击是异步的 ⇒ 连点会静默失效）。
#: ⚠️ 引擎侧脚本**逐条对应**参考侧的生产交互（同一条语义：翻转 / 输入 / 添加）。
STATE_CASES = [
    (
        "governance",
        "governance",
        [],
        [],
        ["T1", "T2", "T3", "T4", "T5"],
    ),
    (
        "tools",
        "tools",
        [],
        [],
        ["T1", "T2", "T3", "T4", "T5", "T6(父行关)", "T7"],
    ),
    (
        "developer",
        "developer",
        [],
        [],
        ["T1", "T2", "T3", "T4", "T5"],
    ),
    (
        # 议会开 ⇒ 顾问数量行 + 单顾问超时行出现（range / number 两个原生控件）。
        "governance-council-on",
        "governance",
        [CLICK_ROW % json.dumps("议会 (Council)")],
        ['window.__CAPABILITY__.toggleCapability("council")'],
        ["T1", "T2", "T3", "T4"],
    ),
    (
        # shell 开 ⇒ 沙箱嵌套行出现（图标 shield-check、无 sandbox-off）。
        "tools-shell-on",
        "tools",
        [CLICK_ROW % json.dumps("Shell 命令工具")],
        ['window.__CAPABILITY__.toggleCapability("shell")'],
        ["T1", "T2", "T3", "T7"],
    ),
    (
        # shell 开 + 沙箱关 ⇒ sandbox-off + shield-off + 告警框。
        "tools-shell-bare",
        "tools",
        [
            CLICK_ROW % json.dumps("Shell 命令工具"),
            CLICK_ROW % json.dumps("AppContainer 沙箱执行"),
        ],
        [
            'window.__CAPABILITY__.toggleCapability("shell")',
            'window.__CAPABILITY__.toggleCapability("shellSandbox")',
        ],
        ["T1", "T2", "T3"],
    ),
    (
        # fileWrite 开 ⇒ 依赖子行去掉 dim / disabled（T6 父行开）。
        "tools-filewrite-on",
        "tools",
        [CLICK_ROW % json.dumps("文件写入（apply_patch · shell 写命令）")],
        ['window.__CAPABILITY__.toggleCapability("fileWrite")'],
        ["T1", "T2", "T3", "T6(父行开)", "T7"],
    ),
    (
        # 思考模式开 ⇒ 文本旋钮组出现（两个 text 输入 + 组内提交语义）。
        "developer-reasoning-on",
        "developer",
        [CLICK_ROW % json.dumps("思考模式 (Reasoning)")],
        ['window.__CAPABILITY__.toggleCapability("reasoningEnabled")'],
        ["T1", "T2", "T3", "T4"],
    ),
    (
        # 允许服务器输入框**有值** ⇒ 「添加」按钮**不再是 disabled**（属性存在性判据）。
        # ⚠️ 不覆盖「添加成功后的行」：参考侧的 `addAllowedServer` 替身有缺陷
        # （`mock/real/desktop-bridge.js` 引用了**未 import** 的 `capability`
        # ⇒ ReferenceError）⇒ 那条路径在参考侧**必然失败**（见报告的共享文件清单）。
        "tools-server-typed",
        "tools",
        [
            "(() => { const input = document.querySelector('#allowed-server-input');"
            " if (!input) return false; input.value = 'relay.example.com:8443';"
            " input.dispatchEvent(new Event('input', {bubbles: true}));"
            " return input.value; })()"
        ],
        [
            'window.__CAPABILITY__.setText(["allowedServerInput", "relay.example.com:8443"])'
        ],
        ["T1", "T2", "T3", "T4"],
    ),
]


def run_state_case(env: dict, name: str, section: str, reference_scripts, engine_scripts) -> dict:
    """跑一个态用例 ⇒ 判据明细。"""
    cdp = env["cdp"]
    base = env["base"]
    reference = reference_at(cdp, base, section, reference_scripts)
    engine = engine_at(cdp, base, section, engine_scripts, reference_box=reference["box"])
    record = compare_pair(reference, engine)
    record["case"] = name
    record["section"] = section
    record["reference_scripts"] = reference_scripts
    record["engine_scripts"] = engine_scripts
    write_evidence(name, reference, engine, record)
    return record


# ── 专项断言 ────────────────────────────────────────────────────────────────

def run_requires_states(env: dict) -> dict:
    """**T6 `requires` 两态**（父行关 / 开）。

    ## 口径（生产 `capDisabled`）

    ```text
    capDisabled(def) = def.requires !== undefined && capabilities[def.requires] !== true
    ```

    ⇒ 父行**关** ⇒ 子行 `class="cap-row cap-row-nested dim"` + `disabled=""`；
    ⇒ 父行**开** ⇒ 子行 `class="cap-row cap-row-nested"`（无 `dim`、无 `disabled`）。

    两态各跑一次**整块**对照（像素 + DOM），并额外核对该行的结构断言。
    """
    cdp = env["cdp"]
    base = env["base"]
    rows = []
    cases = (
        ("requires-parent-off", [], []),
        (
            "requires-parent-on",
            [CLICK_ROW % json.dumps("文件写入（apply_patch · shell 写命令）")],
            ['window.__CAPABILITY__.toggleCapability("fileWrite")'],
        ),
    )
    for name, reference_scripts, engine_scripts in cases:
        reference = reference_at(cdp, base, "tools", reference_scripts)
        engine = engine_at(cdp, base, "tools", engine_scripts, reference_box=reference["box"])
        record = compare_pair(reference, engine)
        record["case"] = name
        # 该态的结构快照（**两态都要**：dim / disabled 的存在性就是判据本身）。
        record["reference_structure"] = reference["structure"]
        record["engine_structure"] = engine["structure"]
        write_evidence(name, reference, engine, record)
        rows.append(record)
    return {
        "name": "requires 两态（fileWriteAutoPass ← fileWrite）",
        "rows": rows,
        "passed": all(row["passed"] for row in rows)
        and all(
            row["reference_structure"] == row["engine_structure"]
            for row in rows
            if row["reference_structure"] and row["engine_structure"]
        ),
    }


def run_nested_row_assertion(env: dict) -> dict:
    """**T7 嵌套行插入点 + MCP 行**的 DOM 结构断言（两侧同口径）。"""
    cdp = env["cdp"]
    base = env["base"]
    reference_structure = reference_at(cdp, base, "tools")["structure"]
    engine_structure = engine_at(cdp, base, "tools")["structure"]
    passed = bool(
        reference_structure
        and engine_structure
        and reference_structure == engine_structure
        and reference_structure.get("sub_is_next_sibling")
        and reference_structure.get("sub_is_not_child")
        and reference_structure.get("mcp_present")
        and reference_structure.get("mcp_is_last_element")
        and reference_structure.get("mcp_is_static")
    )
    return {
        "name": "嵌套行插入点（同层兄弟） + MCP 行（卡内最后一个元素）",
        "passed": passed,
        "reference": reference_structure,
        "engine": engine_structure,
    }


def run_preview_byte_check(env: dict) -> dict:
    """**T5**：配置 JSON 预览逐字节相同（并与 Python 的 `json.dumps(indent=2)` 交叉校验）。"""
    cdp = env["cdp"]
    base = env["base"]
    navigate_reference(cdp, f"{base}/web/reference/capability/index.html")
    cdp.evaluate("window.__SETTINGS__.selectSection('developer'); true")
    time.sleep(0.8)
    reference = cdp.evaluate(PREVIEW_SCRIPT % "'.setting-block'")
    navigate_engine(cdp, f"{base}/web/engine/capability/index.html")
    cdp.evaluate("window.__CAPABILITY__.selectSection('developer'); true")
    time.sleep(0.8)
    engine = cdp.evaluate(PREVIEW_SCRIPT % "'.sp-capability .setting-block'")
    # 交叉校验：把同一份对象用 Python 的缩进 2 序列化 ⇒ 与浏览器侧逐字节比。
    expected = json.dumps(
        {
            "baseUrl": "http://127.0.0.1:8080",
            "model": "gpt-4o",
            "provider": {
                "protocol": "openai",
                "preset": "openai",
                "baseUrl": "https://api.openai.com/v1",
                "apiKey": "",
                "model": "gpt-4o",
                "anthropicVersion": "2023-06-01",
            },
            "hasApiKey": False,
        },
        ensure_ascii=False,
        indent=2,
    )
    return {
        "name": "配置 JSON 预览（键序 baseUrl→model→provider→hasApiKey，缩进 2）",
        "passed": bool(reference) and reference == engine == expected,
        "reference_bytes": len(reference.encode("utf-8")) if reference else None,
        "engine_bytes": len(engine.encode("utf-8")) if engine else None,
        "python_bytes": len(expected.encode("utf-8")),
        "reference_equals_python": reference == expected,
        "engine_equals_python": engine == expected,
        "reference_preview": reference,
        "engine_preview": engine,
    }


def run_shared_api_selection(env: dict) -> dict:
    """**共享入口可驱动本面**：`window.__SETTINGS__.selectSection` 切得动分区。

    ## 为什么这条要单列

    共享的 `web/engine/main.js` 建 `__SETTINGS__` 时只调
    `module.set_settings_section(id)`（**向导面的导出**）⇒ 本面零 wasm 导出
    ⇒ 若不包裹，共享入口（`tools/compare_settings.py --panel capability`）
    切分区会**无效** ⇒ 报的差异是**装置**问题，不是引擎问题。
    """
    cdp = env["cdp"]
    base = env["base"]
    navigate_engine(cdp, f"{base}/web/engine/capability/index.html")
    observed = []
    for section in SECTIONS:
        cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
        time.sleep(0.5)
        observed.append(
            {
                "requested": section,
                "rendered_title": cdp.evaluate(
                    "(() => { const el = document.querySelector('.block-title');"
                    " return el ? el.textContent : null; })()"
                ),
            }
        )
    titles = [row["rendered_title"] for row in observed]
    return {
        "name": "共享入口的 selectSection 能驱动本面（包裹共享 API ⇒ 幂等）",
        "passed": titles == ["决策与治理", "工具与安全", "开发者选项"],
        "sections": observed,
    }


# ── 共享入口（`tools/compare_settings.py` 的约定） ────────────────────────────

def collect_pair(panel: dict, section: str, port: int) -> dict:
    """跑一对（面 × 分区）的对照（**共享脚本的约定入口**）。

    返回值里必须有 `passed`（布尔）与判据明细 ⇒ 共享脚本据此汇总。
    """
    section_id = section["id"] if isinstance(section, dict) else str(section)
    if section_id not in SECTIONS:
        raise RuntimeError(f"未知分区：{section_id}（`security` 不属本面）")
    env = ensure_env(port)
    reference = reference_at(env["cdp"], env["base"], section_id)
    engine = engine_at(
        env["cdp"], env["base"], section_id, reference_box=reference["box"]
    )
    record = compare_pair(reference, engine)
    record["section"] = section_id
    write_evidence(section_id, reference, engine, record)
    return record


def run_interaction_checks(env: dict) -> list:
    """⭐ **交互对照**（真实键入 ⇒ 两侧逐字比对）。

    ## 为什么这个面需要它

    本面的两个文本输入（**推理过滤器** / **推理标签**）**不带** `prop("value")`
    ⇒ ⇒ 取值靠**宿主回读 DOM** ⇒ ⇒ 一旦「宿主回读 → 引擎状态」这条路断了
    ⇒ ⇒ 用户敲的字**引擎不知道**（**幽灵输入**）
    ⇒ ⇒ 而**静态像素对照测不到**（浏览器自己画了值 ⇒ 画面对）
    ⇒ ⇒ 必须有**真实键入**的对照。

    ⚠️ 键入用**真实键盘事件**（`Input.dispatchKeyEvent`）⇒ 走真实事件路径。
    """
    cdp = env["cdp"]
    base = env["base"]

    def navigate(cdp, side: str) -> None:
        if side == "reference":
            navigate_reference(cdp, f"{base}/web/reference/capability/index.html")
        else:
            navigate_engine(cdp, f"{base}/web/engine/capability/index.html")

    # 本面两个文本输入的 aria-label（生产与引擎**逐字相同** ⇒ 两侧通用选择器）。
    cases = [
        (".cap-input-wrap input[aria-label]", "7pt"),
        (".cap-input-wrap input[type=text]", "abc"),
    ]
    records = []
    for selector, text in cases:
        record = interaction.type_into_pair(
            cdp, navigate, ("reference", "engine"), selector, text
        )
        records.append(record)
        log(
            f"  交互 {selector}：键入值一致="
            f"{record['equal']['typed_value']} / 提交值一致="
            f"{record['equal']['committed_value']}"
        )
    return records


def main() -> int:
    """本面独立跑（三个分区 + 七个态用例 + 四项专项 ⇒ 写全量证据）。"""
    env = ensure_env(8787)
    cdp = env["cdp"]
    report: dict = {
        "face": "capability（能力与治理 · 设置页 S4 面）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "crop": "元素框（`.setting-block` / `.sp-capability .setting-block`）",
        "reference_page": "/web/reference/capability/index.html",
        "engine_page": "/web/engine/capability/index.html",
        "sections": list(SECTIONS),
    }
    try:
        cases = []
        for name, section, reference_scripts, engine_scripts, criteria in STATE_CASES:
            record = run_state_case(env, name, section, reference_scripts, engine_scripts)
            record["criteria"] = criteria
            cases.append(record)
            log(
                f"  {name}：像素 {record['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if record['t2_dom_equal'] else '不一致'} / "
                f"宽高 {'相等' if record['t3_same_width'] and record['t3b_same_height'] else '不等'} / "
                f"控件 {'一致' if record['t4_controls_equal'] else '不一致'} / "
                f"预览 {'逐字节相同' if record['t5_preview_equal'] else '不同'}"
            )
        report["cases"] = cases

        requires = run_requires_states(env)
        report["requires_two_states"] = requires
        for row in requires["rows"]:
            log(
                f"  {row['case']}：像素 {row['t1_pixel_ratio'] * 100:.4f}% / "
                f"DOM {'一致' if row['t2_dom_equal'] else '不一致'}"
            )

        nested = run_nested_row_assertion(env)
        report["nested_row_assertion"] = nested
        log(f"  嵌套行 / MCP 行断言：{'通过' if nested['passed'] else '未通过'}")

        preview = run_preview_byte_check(env)
        report["config_json_byte_check"] = preview
        log(
            f"  配置 JSON 预览：{'逐字节相同' if preview['passed'] else '不同'}"
            f"（参考 {preview['reference_bytes']} / 引擎 {preview['engine_bytes']} / "
            f"python {preview['python_bytes']} 字节）"
        )

        shared = run_shared_api_selection(env)
        report["shared_api_selection"] = shared
        log(f"  共享入口切分区：{'通过' if shared['passed'] else '未通过'}")

        # ⭐ **交互对照**（真实键入 ⇒ 两侧逐字比对）——
        # 本面两个文本输入靠**宿主回读 DOM** ⇒ 「读路径断了」这类缺陷
        # **静态像素测不到**（浏览器自己画了值 ⇒ 画面对）。
        interactions = run_interaction_checks(env)
        report["interactions"] = interactions
        log(
            "  交互对照："
            + ("通过" if interaction.compare_records(interactions)["passed"] else "未通过")
        )

        report["checks"] = {
            "t1_pixels_zero": all(row["t1_pixel_ratio"] <= PIXEL_BUDGET for row in cases),
            "t2_dom_identical": all(row["t2_dom_equal"] for row in cases),
            "t3_same_width_and_height": all(
                row["t3_same_width"] and row["t3b_same_height"] for row in cases
            ),
            "t4_native_controls_equal": all(row["t4_controls_equal"] for row in cases),
            "t5_config_json_bytes": preview["passed"],
            "t6_requires_two_states": requires["passed"],
            "t7_nested_row_and_mcp": nested["passed"],
            "t8_shared_api_selection": shared["passed"],
            # ⭐ 键入后取值 / 提交后取值 / 回显文本 ⇒ 两侧**逐字相同**。
            "t9_interaction_identical": interaction.compare_records(interactions)["passed"],
        }
        report["notes"] = [
            "T1 的判据是**差异 0**（任一通道绝对差 > 2/255 即记「不同」）。",
            "T2 的归一化：剔除 Svelte 作用域 class 与 `sp-*` 作用域锚点、"
            "class token 排序；**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3：两侧裁剪框**宽度与高度都**必须相等（不等 ⇒ 直接判失败）。",
            "T4：`select` / `input` 的属性集逐字相等；两侧都**不带** `value` 属性"
            "（生产用 property ⇒ 宿主挂载后回写）。",
            "T5：预览由**宿主**用 `ui_kit::json`（保插入序）+ 缩进 2 生成 ⇒ "
            "与浏览器侧 `JSON.stringify` 逐字节相同。",
            "T6/T7 的两态靠**生产自己的交互路径**驱动参考侧（点行 / 输入 / 添加）"
            "⇒ **生产代码与生产固定数据一字未改**。",
            "T9 是**真实键入**的对照（`Input.dispatchKeyEvent`）⇒ 断言**键入值 / "
            "提交值 / 回显文本**两侧**逐字相同** ⇒ ⇒ 覆盖「用户输入进不进引擎状态」"
            "这类**静态像素测不到**的缺陷（2026-10-25 补）。",
            "环境：视口 1440×900、DPR 1、时区 Asia/Shanghai"
            "（⚠️ **刻意用非 UTC**：生产时刻显示走浏览器 `Intl`（本地时区）⇒"
            "若装置也用 UTC ⇒ **两边一起错 ⇒ 掩盖时区缺陷**）。",
            "⚠️ **范围声明**：`security` 分区（markup 3006–3021）**不迁** —— 那一段"
            "整块委派给独立面（`GovernanceView`）⇒ 不属本面。",
            "⚠️ **未覆盖**：`configJsonCopied` 的「已复制」态在无头浏览器里到不了"
            "（剪贴板写入被拒 ⇒ 生产的 `configJsonCopied` 恒为 false）⇒ 该分支只有"
            "原生单测覆盖（见 `settings_capability.rs` 的 "
            "`the_copy_button_switches_between_two_labels`）。",
            "⚠️ **未覆盖**：`pending` 态（`liveApplyPendingKey != null` ⇒ 全行禁用 + "
            "行内 `pending` class）**不可对照**（参考侧没有把它拨起来的入口）⇒ "
            "只有原生单测覆盖。",
        ]
        report["passed"] = all(
            value for value in report["checks"].values() if isinstance(value, bool)
        )

        # 浏览器控制台（两侧各一次）。
        console = []
        for name, page in (
            ("reference", "web/reference/capability"),
            ("engine", "web/engine/capability"),
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
