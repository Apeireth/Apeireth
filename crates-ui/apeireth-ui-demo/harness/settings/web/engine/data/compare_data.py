#!/usr/bin/env python3
"""数据与用户子面（`data`）的**面内对照驱动**：像素 + DOM 全等 + 宽高同源。

## 为什么本文件曾「丢失」

⚠️ **2026-10-25 实测记录**：本驱动在 M5 批**没有入库**
（根 `.gitignore` 全局忽略 `*.py` ⇒ 该目录当时**没有** `!*.py` 放行）
⇒ 共享入口 `tools/compare_settings.py --panel data` 报「**尚未迁移**」
⇒ 而**上一批报告的「21/21 通过」里有 2 组来自本驱动** ⇒ 那个数字**依赖了未入库的文件**。

⇒ 本批**重建**本驱动（照 `theme` 面的形态，口径一致）⇒ 并补上 `.gitignore` 放行。

## 本面的四个用例

本面有 **2 个分区 × 2 个读数态**：

| 分区 | 态 | 说明 |
|---|---|---|
| `data` | 有读数 / 读不到 | 运维读数（侧车状态面 + 常驻探测面） |
| `user` | 有读数 / 读不到 | 用户资料（昵称 / 签名 / 头像） |

- **参考侧**：`/web/reference/data/index.html` —— 生产 `SettingsView`（**只读引入**）
  ⇒ 裁剪 `.setting-block`。
- **引擎侧**：`/web/engine/data/index.html` —— wasm 引擎独立挂载
  ⇒ 裁剪 `.sp-data .setting-block`。

## 两态怎么切

- **参考侧**：在**装载页面之前**注入 `globalThis.__DATA_READINGS__ = false`
  （CDP `Page.addScriptToEvaluateOnNewDocument`）⇒ 替身返回 `null` ⇒ 读不到。
- **引擎侧**：调 `window.__DATA__.setReadings(false)`。
⇒ **两侧读的是同一份 `fixtures/data.json`**，只是「能不能读到」不同 ⇒ 可比。

## 环境固定

视口 `1440×900`、`devicePixelRatio = 1`、时区 `UTC`、两侧同一份 CSS 与同一份固定数据。

## 用法

```bash
python3 web/engine/data/compare_data.py
```

依赖：系统 Chromium、`websocket-client`、Pillow。**不需要** Playwright。
"""

from __future__ import annotations

import io
import json
import os
import pathlib
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
SETTINGS = HERE.parent.parent.parent
HARNESS = SETTINGS.parent

# 批一的对照装置（**复用**，不重复实现）。
sys.path.insert(0, str(HARNESS / "tools"))

try:
    import compare  # noqa: E402
except ImportError:  # pragma: no cover - 批一装置缺失时的显式失败
    print(
        "找不到批一的对照装置："
        f"{HARNESS / 'tools' / 'compare.py'}\n"
        "（设置页 harness 复用批一的浏览器驱动 / 截图 / 像素差分 ⇒ 批一装置是**前置**）",
        file=sys.stderr,
    )
    raise SystemExit(2)

DIST = SETTINGS / "dist"
OUT = SETTINGS / "out" / "data"
EVIDENCE = SETTINGS / "evidence" / "data"

#: 视口与阈值（沿用批一口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 本面的两侧裁剪选择器（参考侧 / 引擎侧）。
REFERENCE_SELECTOR = ".setting-block"
ENGINE_SELECTOR = ".sp-data .setting-block"

#: 本面的分区（`data` / `user`）与读数态（`readings` / `no-readings`）。
SECTIONS = ("data", "user")
READING_STATES = ("readings", "no-readings")

#: 归一化 DOM 的脚本（**批一口径** + 「剔除 `sp-*` 根 class」这一条）。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '')
    .replace(/\bsvelte-[0-9a-z]+\b/g, '')
    .replace(/\bsp-[a-z0-9-]+\b/g, '')
    .trim();
  const normalize = (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      if (!node.textContent.trim()) return null;
      return { t: 'text', v: node.textContent };
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
  const root = document.querySelector(%s);
  return JSON.stringify(root ? normalize(root) : null);
})()
"""


#: 参考侧已注入的脚本 id（`addScriptToEvaluateOnNewDocument` **累积** ⇒ 切态前要清）。
REFERENCE_INJECTED: dict = {"ids": []}


def log(message: str) -> None:
    print(message, flush=True)


def normalize_script(selector: str) -> str:
    """按选择器取一份归一化脚本。"""
    return NORMALIZE_SCRIPT % json.dumps(selector)


def await_fonts(cdp) -> None:
    """等字体就绪（**失败不致命** ⇒ 字体没就绪也只是像素差一点）。

    ⭐ **必要性**（2026-10-26 实测定位）：
    本面此前**没有**等字体 ⇒ ⇒ 截图时字体**可能还没加载完**
    ⇒ ⇒ 生产侧与引擎侧的字体**加载时机不同** ⇒ ⇒ **文字宽度**不同
    ⇒ ⇒ 按钮宽度随之不同 ⇒ ⇒ 表现为「**散布全图**的 0.063% 排版级差异」
    （实测差异包围盒覆盖整幅画面 ⇒ ⇒ 不是单个元素的问题）。
    ⇒ ⇒ 其余各面（capability / character / provider）**都有**这一步 ⇒ ⇒ 本面是**漏的**。
    """
    try:
        cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    except Exception:
        pass


def element_box(cdp, selector: str) -> dict | None:
    """取元素的外框（相对文档坐标）。"""
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def navigate(cdp, url: str) -> None:
    """导航到一个页面（等它加载完 + **等字体就绪**）。"""
    cdp.call("Page.navigate", {"url": url})
    # ⭐ **轮询等就绪**（2026-10-26 实测修的脆弱点）：
    # 原先只 `sleep(1.2)` ⇒ ⇒ 慢机器 / 冷启动时**页面还没加载完**
    # ⇒ ⇒ 随后的 `window.__SETTINGS__.selectSection(...)` 抛
    #     「Cannot read properties of undefined」⇒ ⇒ 整轮失败。
    # ⇒ ⇒ 改为等 `__SETTINGS__` 与 `ready()`（**与其它面同口径**）。
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    # ⭐ 等字体 ⇒ ⇒ 否则**文字宽度**会因加载时机不同而不同（2026-10-26 实测）。
    await_fonts(cdp)


def select_section(cdp, section: str) -> None:
    """切到某个分区（两侧同名同义）。"""
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.6)


def set_readings(cdp, side: str, enabled: bool) -> None:
    """切「读数能不能读到」这一态（**两侧各自的机制**）。

    ## ⚠️ 参考侧的注入**必须先清掉旧的**

    `Page.addScriptToEvaluateOnNewDocument` 是**累积**的（不调用 remove 就一直在）
    ⇒ 切态时若只加不删 ⇒ **两个脚本并存** ⇒ 每次导航都会重跑全部
    ⇒ 哪个后执行哪个赢 ⇒ **态会飘**（实测：两个态的参考侧截图**完全相同**
    ⇒ 说明注入没切住）。
    ⇒ ⇒ 本实现**每次先 remove 全部已注脚本**，再按需注入**一条**。
    """
    if side == "reference":
        # 先清掉此前注过的（累积 ⇒ 不清会飘）。
        for identifier in list(REFERENCE_INJECTED.get("ids", [])):
            cdp.call("Page.removeScriptToEvaluateOnNewDocument", {"identifier": identifier})
        REFERENCE_INJECTED["ids"] = []
        if not enabled:
            # 只在「读不到」这一态注入 `false` ⇒ 其它态**不注入**
            # ⇒ 页面全局缺省就是「有读数」（与替身的默认值一致）。
            result = cdp.call(
                "Page.addScriptToEvaluateOnNewDocument",
                {"source": "globalThis.__DATA_READINGS__ = false;"},
            )
            identifier = (result or {}).get("result", {}).get("identifier")
            if identifier:
                REFERENCE_INJECTED["ids"].append(identifier)
    else:
        cdp.evaluate(f"window.__DATA__.setReadings({json.dumps(enabled)}); true")
    time.sleep(0.4)


def describe_dom_diff(left_tree, right_tree, prefix: str = "根") -> list[str]:
    """描述两棵归一化 DOM 的差异（**逐字**文本差异也要报出来）。"""
    diffs: list[str] = []

    def walk(left, right, path: str) -> None:
        if len(diffs) >= 40:
            return
        if left is None or right is None:
            if left is not right:
                diffs.append(f"{path}: 一侧为 None、另一侧有内容")
            return
        if left.get("t") != right.get("t"):
            diffs.append(f"{path}: 节点类型不同 ⇒ {left.get('t')} vs {right.get('t')}")
            return
        if left.get("t") == "text":
            if left.get("v") != right.get("v"):
                diffs.append(
                    f"{path}: 文本不同 ⇒ 参考 {left.get('v')!r} / 引擎 {right.get('v')!r}"
                )
            return
        if left.get("tag") != right.get("tag"):
            diffs.append(f"{path}: 标签不同 ⇒ {left.get('tag')} vs {right.get('tag')}")
            return
        left_attrs = left.get("attrs") or {}
        right_attrs = right.get("attrs") or {}
        for key in sorted(set(left_attrs) | set(right_attrs)):
            if left_attrs.get(key) != right_attrs.get(key):
                diffs.append(
                    f"{path}: 属性 `{key}` 不同 ⇒ "
                    f"参考 {left_attrs.get(key)!r} / 引擎 {right_attrs.get(key)!r}"
                )
        left_children = left.get("children") or []
        right_children = right.get("children") or []
        if len(left_children) != len(right_children):
            diffs.append(
                f"{path}: 子节点数不同（{len(left_children)} / {len(right_children)}）"
            )
        for index, (lc, rc) in enumerate(zip(left_children, right_children)):
            walk(lc, rc, f"{path}/{index}")

    walk(left_tree, right_tree, prefix)
    return diffs


def collect_pair(panel: str, section: str, port: int) -> dict:
    """**模块协议形态**（共享入口按约定发现并转发）。

    ## 口径

    共享入口自己开浏览器、自己切分区 ⇒ 本函数**只负责采集一个分区的两侧**
    （DOM + 框 + 截图）并**做判据** ⇒ 返回值里带 `passed` 与判据明细。

    ⚠️ **读数态**：共享入口的调用**不带态参数** ⇒ 本函数采「**有读数**」这一态
    ⇒ 「读不到」那一态由本驱动的 `main()` 单独覆盖（4 组用例的完整集合在
    `evidence/data/report.json`）。
    """
    # ⚠️ **自己起服务**（与其它面同口径）：共享入口**不起服务**，只把 `--port`
    # 参数传下来 ⇒ 驱动必须自己 serve `dist/` ⇒ 否则访问到的是**前面某个面
    # 留下 / 已关闭的服务** ⇒ 页面 404 ⇒ `window.__SETTINGS__` 为 `undefined`
    # ⇒ 报「页面求值异常」（2026-10-25 实测）。
    # ⇒ 用 `compare.serve`（端口 0 ⇒ 系统分配 ⇒ **不会撞端口**）。
    httpd, actual = compare.serve(DIST)
    base = f"http://127.0.0.1:{actual}"
    chrome, devtools_port = compare.launch_chrome()
    try:
        cdp = compare.open_page(devtools_port)
        shots = {}
        for side, page in (
            ("reference", f"{base}/web/reference/data/index.html"),
            ("engine", f"{base}/web/engine/data/index.html"),
        ):
            navigate(cdp, page)
            select_section(cdp, section)
            selector = REFERENCE_SELECTOR if side == "reference" else ENGINE_SELECTOR
            box = element_box(cdp, selector)
            shots[side] = {
                "dom": json.loads(cdp.evaluate(normalize_script(selector)) or "null"),
                "box": box,
                "shot": compare.screenshot(cdp, box),
            }
            if side == "reference":
                shots[side]["index"] = element_index(cdp, box or {})
        # ── 装置对齐：两侧内容必须落在**同一个设备像素格** ─────────────────────
        # 参考侧是**生产整页**（页头 + 侧栏 + 内容列）⇒ 它的内容块在文档里的位置
        # **带小数**（实测 `top = 219.765625`）；引擎侧是**独立挂载** ⇒ 位置是 `0`。
        # ⇒ 两边的**设备像素格**错开 0.765625px ⇒ 渐变 / 圆角抗锯齿 / 字形栅格化
        #   **必然不同** ⇒ 那是**装置伪差**，不是引擎画错了。
        # ⇒ 把引擎侧的舞台**对齐**到参考侧的文档位置 ⇒ 差异只反映**渲染本身**。
        # ⚠️ 对齐前 / 对齐后两个数字**都**落进报告 ⇒ **不隐藏**任何差异。
        ref_box = shots["reference"]["box"] or {}
        eng_box = shots["engine"]["box"] or {}
        delta_y = float(ref_box.get("y", 0.0)) - float(eng_box.get("y", 0.0))
        delta_x = float(ref_box.get("x", 0.0)) - float(eng_box.get("x", 0.0))
        aligned = cdp.evaluate(
            "(() => { const stage = document.querySelector('.settings-stage');"
            " if (!stage) return false;"
            " stage.style.marginTop = %r + 'px';"
            " stage.style.marginLeft = %r + 'px';"
            " return true; })()"
            % (delta_y, delta_x)
        )
        time.sleep(0.3)
        eng_box_after = compare.wait_element_box_stable(cdp, ENGINE_SELECTOR, get_box=lambda: element_box(cdp, ENGINE_SELECTOR))
        engine_shot = compare.wait_stable(cdp, eng_box_after)
        unaligned = compare.pixel_diff(shots["reference"]["shot"], shots["engine"]["shot"])
        shots["engine"]["shot"] = engine_shot
        shots["engine"]["box"] = eng_box_after
        shots["engine"]["index"] = element_index(cdp, eng_box_after or {})
        alignment = {
            "why": "两侧内容必须落在同一个设备像素格 ⇒ 抗锯齿 / 字形栅格化才可比",
            "reference_origin": {"x": ref_box.get("x"), "y": ref_box.get("y")},
            "engine_origin_before": {"x": eng_box.get("x"), "y": eng_box.get("y")},
            "delta_x": delta_x,
            "delta_y": delta_y,
            "applied": bool(aligned),
            "engine_origin_after": {
                "x": (eng_box_after or {}).get("x"),
                "y": (eng_box_after or {}).get("y"),
            },
            "t1_pixel_ratio_unaligned": unaligned["ratio"],
        }

        pixel = compare.pixel_diff(shots["reference"]["shot"], shots["engine"]["shot"])
        dom_diffs = describe_dom_diff(shots["reference"]["dom"], shots["engine"]["dom"])
        ref_box = shots["reference"]["box"] or {}
        eng_box = shots["engine"]["box"] or {}
        same_width = abs(float(ref_box.get("w", 0)) - float(eng_box.get("w", 0))) <= 0.5
        same_height = abs(float(ref_box.get("h", 0)) - float(eng_box.get("h", 0))) <= 0.5
        # ── 元素级归因（口径 A）：差异必须落到「两侧同元素的同一样式」 ──────
        points = diff_pixel_points(
            shots["reference"]["shot"], shots["engine"]["shot"], limit=8
        )
        attributions = [
            attribute_pixel_difference(
                point,
                shots["reference"].get("index") or [],
                shots["engine"].get("index") or [],
                shots["reference"]["box"] or {},
                shots["engine"]["box"] or {},
            )
            for point in points
        ]
        all_attributed = all(item["attributed"] for item in attributions)
        result = {
            "section": section,
            "alignment": alignment,
            # ⚠️ **严格读数保留为报告项**（不删、不隐藏）：像素差个数与占比。
            "t1_pixel_different": pixel["different"],
            "t1_pixel_total": pixel["total"],
            "t1_pixel_ratio": pixel["ratio"],
            # ── 判据（有界）：占比 ≤ 预算，且每个差异都能归因到同元素同样式 ──
            "t1_pixel_within_budget": t1_within_budget(pixel["ratio"]),
            "t1_pixel_budget": DATA_PIXEL_BUDGET,
            "t1b_all_diffs_attributed": all_attributed,
            "t1_attribution": attributions,
            "t2_dom_equal": not dom_diffs,
            "t2_dom_diffs": dom_diffs,
            "t3_same_width": same_width,
            "t3b_same_height": same_height,
        }
        result["passed"] = (
            result["t1_pixel_within_budget"]
            and result["t1b_all_diffs_attributed"]
            and result["t2_dom_equal"]
            and same_width
            and same_height
        )
        return result
    finally:
        import signal

        try:
            os.killpg(os.getpgid(chrome.pid), signal.SIGKILL)
        except Exception:
            chrome.kill()
        httpd.shutdown()


# ═══════════════════════════════════════════════════════════════════════════
# T1 有界判据（2026-10-30 owner 裁定：**E-54 口径 A**）
# ═══════════════════════════════════════════════════════════════════════════
#: T1 像素差预算（占比）。实测 data 面 554/885132 = **0.0626%**
#: ⇒ 0.1% 留约 **1.6× 余量**；与 intro 面 `check_particle_edge.py` v2、
#:    pilot/工具卡面（`compare.PIXEL_BUDGET`）**同范式**（仓库内既有做法）。
DATA_PIXEL_BUDGET = 0.001

#: 归因配对时每侧取**最深**的若干个命中元素（面积最小优先）。
ATTRIBUTION_CANDIDATES = 6


def t1_within_budget(ratio: float, budget: float = DATA_PIXEL_BUDGET) -> bool:
    """T1 纯判据：像素差占比 ≤ 预算 ⇒ 通过（**纯函数**，便于反事实自检）。"""
    return ratio <= budget


def _shift_png(data: bytes, dx: int, dy: int) -> bytes:
    """把 PNG 平移 (dx, dy) 像素（**仅供反事实自检**用）。"""
    from PIL import Image, ImageChops

    image = Image.open(io.BytesIO(data)).convert("RGB")
    buffer = io.BytesIO()
    ImageChops.offset(image, dx, dy).save(buffer, format="PNG")
    return buffer.getvalue()


def selfcheck_t1_criterion(sample_png: bytes | None = None) -> list[str]:
    """判据**可失败性**自检：合成反事实 ⇒ 若被判通过 ⇒ 判据坏了。

    ⚠️ 两层：
    ①**逻辑层** —— 0 差异必须 PASS、超预算必须 FAIL、恰好等于预算必须 PASS；
    ②**图像层**（真验证「像素统计 → 阈值 → 预算」整条链能失败）——
       同一张图 vs 自身 ⇒ 必须 PASS；同一张图 vs **自身平移 1px** ⇒ 必然远超预算 ⇒ 必须 FAIL。

    ⇒ 任一反事实不符 ⇒ 门禁自身失效 ⇒ 调用方须 `exit 2`（**判据坏了，不许报绿**）。
    """
    problems: list[str] = []
    if not t1_within_budget(0.0):
        problems.append("反事实①：0 差异被判失败 ⇒ 判据过严")
    if t1_within_budget(DATA_PIXEL_BUDGET * 10):
        problems.append("反事实②：超预算差异被判通过 ⇒ 判据失效")
    if not t1_within_budget(DATA_PIXEL_BUDGET):
        problems.append("反事实③：恰好等于预算被判失败 ⇒ 边界定义不一致")
    if sample_png is None:
        from PIL import Image

        # 自造**有结构**的图（纯色图平移 1px 后仍相同 ⇒ 反事实会失效）
        canvas = Image.new("RGB", (64, 64))
        pixels = canvas.load()
        for y in range(64):
            for x in range(64):
                pixels[x, y] = ((x * 7) % 256, (y * 11) % 256, ((x + y) * 13) % 256)
        buffer = io.BytesIO()
        canvas.save(buffer, format="PNG")
        sample_png = buffer.getvalue()
    if sample_png:
        same = compare.pixel_diff(sample_png, sample_png)
        if not t1_within_budget(same["ratio"]):
            problems.append("反事实④：同图自比被判失败 ⇒ 判据过严")
        shifted = _shift_png(sample_png, 1, 1)
        moved = compare.pixel_diff(sample_png, shifted)
        if t1_within_budget(moved["ratio"]):
            problems.append("反事实⑤：平移 1px 的自比被判通过 ⇒ 判据抓不到真差异")
    return problems


def selfcheck_attribution() -> list[str]:
    """**归因判据**的可失败性自检（合成两侧元素索引）。

    ⇒ 三条反事实：
    ①两侧**完全相同** ⇒ 必须归因成功（否则判据太严，永远红）；
    ②两侧**样式不同** ⇒ 必须归因失败（否则真缺陷会被预算吞掉）；
    ③两侧**几何不同** ⇒ 必须归因失败（同上）。

    ⇒ 任一不符 ⇒ 归因判据失效 ⇒ 调用方须 `exit 2`。
    """
    problems: list[str] = []
    base = {
        "tag": "div",
        "cls": "sample",
        "rect": {"x": 10.0, "y": 20.0, "w": 100.0, "h": 30.0},
        "style": {"color": "rgb(1, 2, 3)", "borderTopColor": "rgba(224, 91, 80, 0.35)"},
    }
    clip = {"x": 0.0, "y": 0.0, "w": 800.0, "h": 600.0}
    point = (12, 22)

    same = attribute_pixel_difference(point, [base], [dict(base)], clip, clip)
    if not same["attributed"]:
        problems.append("归因反事实①：两侧完全相同却归因失败 ⇒ 判据过严")

    other_style = dict(base)
    other_style["style"] = dict(base["style"])
    other_style["style"]["color"] = "rgb(9, 9, 9)"
    styled = attribute_pixel_difference(point, [base], [other_style], clip, clip)
    if styled["attributed"]:
        problems.append("归因反事实②：两侧样式不同却归因成功 ⇒ 真缺陷会被预算吞掉")

    other_rect = dict(base)
    other_rect["rect"] = {"x": 11.0, "y": 20.0, "w": 100.0, "h": 30.0}
    moved = attribute_pixel_difference(point, [base], [other_rect], clip, clip)
    if moved["attributed"]:
        problems.append("归因反事实③：两侧几何不同却归因成功 ⇒ 真缺陷会被预算吞掉")
    return problems


#: 页面内**元素索引**采集脚本：取裁剪框内所有可见元素的
#: (tag / class / 边界盒 / 关键计算样式) ⇒ 供**元素级归因**在本地比对。
ELEMENT_INDEX_SCRIPT = r"""(() => {
  const clip = %s;
  const keys = [
    'position', 'top', 'left', 'width', 'height',
    'borderTopWidth', 'borderTopColor', 'borderTopStyle',
    'borderBottomWidth', 'borderBottomColor', 'borderBottomStyle',
    'borderLeftWidth', 'borderLeftColor', 'borderLeftStyle',
    'borderRightWidth', 'borderRightColor', 'borderRightStyle',
    'backgroundColor', 'color', 'opacity', 'transform', 'filter',
    'boxShadow', 'mixBlendMode', 'isolation',
    'fontFamily', 'fontSize', 'fontWeight', 'lineHeight',
  ];
  const out = [];
  for (const el of document.querySelectorAll('*')) {
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) continue;
    const x = r.x + window.scrollX;
    const y = r.y + window.scrollY;
    if (x + r.width < clip.x || x > clip.x + clip.w) continue;
    if (y + r.height < clip.y || y > clip.y + clip.h) continue;
    const cs = getComputedStyle(el);
    const style = {};
    for (const key of keys) style[key] = cs[key];
    out.push({
      tag: el.tagName.toLowerCase(),
      cls: (el.getAttribute('class') || '').split(/\s+/).filter(Boolean).sort().join(' '),
      rect: { x: r.x, y: r.y, w: r.width, h: r.height },
      style: style,
    });
  }
  return JSON.stringify(out);
})()"""


def element_index(cdp, clip: dict) -> list[dict]:
    """采集裁剪框内的元素索引（页面坐标口径）。"""
    payload = json.dumps(
        {"x": float(clip.get("x", 0.0)), "y": float(clip.get("y", 0.0)),
         "w": float(clip.get("w", 0.0)), "h": float(clip.get("h", 0.0))}
    )
    raw = cdp.evaluate(ELEMENT_INDEX_SCRIPT % payload)
    if isinstance(raw, str):
        return json.loads(raw or "[]")
    return raw or []


def diff_pixel_points(left_png: bytes, right_png: bytes, limit: int = 8) -> list[tuple[int, int]]:
    """差异像素坐标（**截图内**坐标）：**全量收集后均匀取样** `limit` 个。

    ⚠️ 阈值口径与 `compare.pixel_diff` 一致（任一通道绝对差 > `CHANNEL_THRESHOLD`）。

    ⭐ **为什么是「均匀取样」而不是「取前 N 个」**：差异可能集中在少数几行
    （E-54 实测集中在 y≈913/989/1024/1069 的分数位置边框）
    ⇒ ⇒ 只取前 N 个会**整批落在顶部**⇒ ⇒ 归因覆盖不到真正的差异行
    ⇒ ⇒ 均匀取样才能代表**整个**差异分布。
    """
    from PIL import Image, ImageChops

    left = Image.open(io.BytesIO(left_png)).convert("RGB")
    right = Image.open(io.BytesIO(right_png)).convert("RGB")
    if left.size != right.size:
        return []
    difference = ImageChops.difference(left, right)
    mask = None
    for band in difference.split():
        part = band.point(lambda v: 255 if v > compare.CHANNEL_THRESHOLD else 0)
        mask = part if mask is None else ImageChops.lighter(mask, part)
    if mask is None:
        return []
    width, _height = mask.size
    all_points: list[tuple[int, int]] = []
    for index, value in enumerate(mask.tobytes()):
        if value:
            all_points.append((index % width, index // width))
            if len(all_points) >= 5000:
                break
    if not all_points:
        return []
    if len(all_points) <= limit:
        return all_points
    # 均匀取样（含首尾）⇒ 覆盖整个差异分布。
    step = (len(all_points) - 1) / (limit - 1)
    return [all_points[round(index * step)] for index in range(limit)]


def _covers(rect: dict, x: float, y: float) -> bool:
    """元素矩形是否覆盖点（**含边**）。"""
    return (
        rect["x"] <= x <= rect["x"] + rect["w"]
        and rect["y"] <= y <= rect["y"] + rect["h"]
    )


def attribute_pixel_difference(
    point: tuple[int, int],
    ref_index: list[dict],
    eng_index: list[dict],
    ref_clip: dict,
    eng_clip: dict,
) -> dict:
    """**元素级归因**：把一个差异像素落到两侧的具体元素上。

    ## 判定规则（口径 A 的关键）

    差异必须能归因到**两侧同元素的同一样式**（tag 相同 + 关键计算样式逐值相同
    + 边界盒逐值相同）⇒ ⇒ 该差异是**同一元素的合成舍入差**（浏览器侧）
    ⇒ ⇒ 才允许被 T1 预算容忍。

    ⇒ 若归因**不成立**（找不到同元素，或找到但样式/几何不同）⇒
    ⇒ **判失败**（哪怕差异占比很小）⇒ ⇒ 真缺陷不会被预算吞掉。
    """
    sx, sy = point
    ref_x = sx + float(ref_clip.get("x", 0.0))
    ref_y = sy + float(ref_clip.get("y", 0.0))
    eng_x = sx + float(eng_clip.get("x", 0.0))
    eng_y = sy + float(eng_clip.get("y", 0.0))
    ref_hits = [el for el in ref_index if _covers(el["rect"], ref_x, ref_y)]
    eng_hits = [el for el in eng_index if _covers(el["rect"], eng_x, eng_y)]
    # ⭐ **取最深命中元素**（面积最小）—— `html`/`body` 总能覆盖整页，
    #   若允许它们参与配对 ⇒ ⇒ 任何差异都会「归因成功」⇒ ⇒ 等于没归因。
    ref_hits.sort(key=lambda el: el["rect"]["w"] * el["rect"]["h"])
    eng_hits.sort(key=lambda el: el["rect"]["w"] * el["rect"]["h"])
    ref_hits = ref_hits[:ATTRIBUTION_CANDIDATES]
    eng_hits = eng_hits[:ATTRIBUTION_CANDIDATES]
    match = None
    for ref_el in ref_hits:
        for eng_el in eng_hits:
            if ref_el["tag"] != eng_el["tag"]:
                continue
            if ref_el["rect"] != eng_el["rect"]:
                continue
            if ref_el["style"] != eng_el["style"]:
                continue
            match = {
                "tag": ref_el["tag"],
                "class": ref_el["cls"],
                "rect": ref_el["rect"],
            }
            break
        if match:
            break
    return {
        "point": {"x": sx, "y": sy},
        "ref_page_point": {"x": ref_x, "y": ref_y},
        "eng_page_point": {"x": eng_x, "y": eng_y},
        "ref_candidates": len(ref_hits),
        "eng_candidates": len(eng_hits),
        "attributed": match is not None,
        "match": match,
        "ref_style_diff": [
            el for el in ref_hits
            if any(
                other["tag"] == el["tag"] and other["rect"] == el["rect"]
                and other["style"] != el["style"]
                for other in eng_hits
            )
        ][:2],
    }


def main() -> int:
    if not DIST.exists():
        log(f"未找到构建产物：{DIST}（先跑 `bash build.sh`）")
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)

    # ── ⭐ 判据**可失败性**自检（先于任何判定）：判据坏了 ⇒ exit 2 ──
    criterion_problems = selfcheck_t1_criterion() + selfcheck_attribution()
    if criterion_problems:
        log("❌ 判据自检未过（判据本身坏了 ⇒ 不许报绿）：")
        for problem in criterion_problems:
            log(f"   · {problem}")
        return 2
    log("✅ 判据自检通过（像素判据 5 条 + 归因判据 3 条反事实全部符合预期）")

    httpd, port = compare.serve(DIST)
    base = f"http://127.0.0.1:{port}"
    chrome, devtools_port = compare.launch_chrome()
    report: dict = {
        "face": "data（数据与用户）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/web/reference/data/index.html",
        "engine_page": "/web/engine/data/index.html",
        "reference_selector": REFERENCE_SELECTOR,
        "engine_selector": ENGINE_SELECTOR,
        "sections": list(SECTIONS),
        "reading_states": list(READING_STATES),
        "cases": [],
    }
    try:
        cdp = compare.open_page(devtools_port)
        for section in SECTIONS:
            for state in READING_STATES:
                enabled = state == "readings"
                log(f"── {section} / {state} ──")
                shots = {}
                for side, page in (
                    ("reference", f"{base}/web/reference/data/index.html"),
                    ("engine", f"{base}/web/engine/data/index.html"),
                ):
                    # ⚠️ **顺序**：引擎侧的 `setReadings` **必须在导航之后**
                    # （`window.__DATA__` 由页面装载时创建）⇒ 参考侧的注入
                    # 反而**必须在导航之前**（要赶在页面脚本执行前生效）。
                    if side == "reference":
                        set_readings(cdp, side, enabled)
                    navigate(cdp, page)
                    if side == "engine":
                        set_readings(cdp, side, enabled)
                    select_section(cdp, section)
                    selector = REFERENCE_SELECTOR if side == "reference" else ENGINE_SELECTOR
                    box = element_box(cdp, selector)
                    shots[side] = {
                        "dom": json.loads(cdp.evaluate(normalize_script(selector)) or "null"),
                        "box": box,
                        "shot": compare.screenshot(cdp, box),
                    }
                    # 参考侧的**元素索引**（元素级归因用）：此刻页面状态与截图一致。
                    if side == "reference":
                        shots[side]["index"] = element_index(cdp, box or {})
                    # 参考侧的注入**只对下一次装载生效** ⇒ 引擎侧不需要。
                    if side == "reference":
                        set_readings(cdp, side, True)

                # ── 装置对齐：两侧内容必须落在**同一个设备像素格** ─────────────
                # 参考侧是**生产整页**（页头 + 侧栏 + 内容列）⇒ 它的内容块在文档里
                # 的位置**带小数**（实测 `top = 219.765625`）；引擎侧是**独立挂载**
                # ⇒ 位置是 `0`。
                # ⇒ 两边的**设备像素格**错开 0.765625px ⇒ 渐变 / 圆角抗锯齿 /
                #   字形栅格化**必然不同** ⇒ 那是**装置伪差**，不是引擎画错了。
                # ⇒ 把引擎侧的舞台**对齐**到参考侧的文档位置 ⇒ 差异只反映**渲染本身**。
                # ⚠️ 对齐前 / 对齐后两个数字**都**落进报告 ⇒ **不隐藏**任何差异。
                ref_box = shots["reference"]["box"] or {}
                eng_box = shots["engine"]["box"] or {}
                delta_y = float(ref_box.get("y", 0.0)) - float(eng_box.get("y", 0.0))
                delta_x = float(ref_box.get("x", 0.0)) - float(eng_box.get("x", 0.0))
                aligned = cdp.evaluate(
                    "(() => { const stage = document.querySelector('.settings-stage');"
                    " if (!stage) return false;"
                    " stage.style.marginTop = %r + 'px';"
                    " stage.style.marginLeft = %r + 'px';"
                    " return true; })()"
                    % (delta_y, delta_x)
                )
                time.sleep(0.3)
                eng_box_after = compare.wait_element_box_stable(cdp, ENGINE_SELECTOR, get_box=lambda: element_box(cdp, ENGINE_SELECTOR))
                engine_shot = compare.wait_stable(cdp, eng_box_after)
                # 引擎侧元素索引**在对齐后**采（对齐改了位置 ⇒ 索引必须与截图同刻）。
                shots["engine"]["index"] = element_index(cdp, eng_box_after or {})
                unaligned = compare.pixel_diff(shots["reference"]["shot"], shots["engine"]["shot"])
                shots["engine"]["shot"] = engine_shot
                shots["engine"]["box"] = eng_box_after
                alignment = {
                    "why": "两侧内容必须落在同一个设备像素格 ⇒ 抗锯齿 / 字形栅格化才可比",
                    "reference_origin": {"x": ref_box.get("x"), "y": ref_box.get("y")},
                    "engine_origin_before": {"x": eng_box.get("x"), "y": eng_box.get("y")},
                    "delta_x": delta_x,
                    "delta_y": delta_y,
                    "applied": bool(aligned),
                    "engine_origin_after": {
                        "x": (eng_box_after or {}).get("x"),
                        "y": (eng_box_after or {}).get("y"),
                    },
                    "t1_pixel_ratio_unaligned": unaligned["ratio"],
                }

                # 诊断：逐个直接子元素量高度（找「谁多出 6.2px」）。
                heights = {}
                for side in ("reference", "engine"):
                    selector = REFERENCE_SELECTOR if side == "reference" else ENGINE_SELECTOR
                    heights[side] = json.loads(
                        cdp.evaluate(
                            "JSON.stringify(Array.from(document.querySelectorAll("
                            + json.dumps(selector + " > *")
                            + ")).map((el) => ({cls: el.className || el.tagName,"
                            " h: Math.round(el.getBoundingClientRect().height * 100) / 100})))"
                        )
                        or "[]"
                    )
                pixel = compare.pixel_diff(shots["reference"]["shot"], shots["engine"]["shot"])
                dom_diffs = describe_dom_diff(
                    shots["reference"]["dom"], shots["engine"]["dom"]
                )
                ref_box = shots["reference"]["box"] or {}
                eng_box = shots["engine"]["box"] or {}
                same_width = (
                    abs(float(ref_box.get("w", 0)) - float(eng_box.get("w", 0))) <= 0.5
                )
                same_height = (
                    abs(float(ref_box.get("h", 0)) - float(eng_box.get("h", 0))) <= 0.5
                )
                # ── 元素级归因（口径 A）：差异必须落到「两侧同元素的同一样式」 ──
                points = diff_pixel_points(
                    shots["reference"]["shot"], shots["engine"]["shot"], limit=8
                )
                attributions = [
                    attribute_pixel_difference(
                        point,
                        shots["reference"].get("index") or [],
                        shots["engine"].get("index") or [],
                        ref_box,
                        eng_box,
                    )
                    for point in points
                ]
                all_attributed = all(item["attributed"] for item in attributions)
                case = {
                    "case": f"{section}-{state}",
                    "section": section,
                    "reading_state": state,
                    "alignment": alignment,
                    # ⚠️ **严格读数保留为报告项**（不删、不隐藏）：像素差个数与占比。
                    "t1_pixel_different": pixel["different"],
                    "t1_pixel_total": pixel["total"],
                    "t1_pixel_ratio": pixel["ratio"],
                    # ── 判据（有界）：占比 ≤ 预算，且每个差异都能归因到同元素同样式 ──
                    "t1_pixel_within_budget": t1_within_budget(pixel["ratio"]),
                    "t1_pixel_budget": DATA_PIXEL_BUDGET,
                    "t1b_all_diffs_attributed": all_attributed,
                    "t1_attribution": attributions,
                    "t2_dom_equal": not dom_diffs,
                    "t2_dom_diffs": dom_diffs,
                    "t3_same_width": same_width,
                    "t3b_same_height": same_height,
                    "reference_box": ref_box,
                    "engine_box": eng_box,
                }
                case["passed"] = (
                    case["t1_pixel_within_budget"]
                    and case["t1b_all_diffs_attributed"]
                    and case["t2_dom_equal"]
                    and same_width
                    and same_height
                )
                report["cases"].append(case)
                log(
                    f"  像素 {case['t1_pixel_different']}/{case['t1_pixel_total']}"
                    f" / DOM {'一致' if case['t2_dom_equal'] else '不一致'}"
                    f" / 宽 {ref_box.get('w')} vs {eng_box.get('w')}"
                    f" / 高 {ref_box.get('h')} vs {eng_box.get('h')}"
                )
                for line in dom_diffs[:6]:
                    log(f"     {line}")
                (EVIDENCE / f"reference-{section}-{state}.png").write_bytes(
                    shots["reference"]["shot"]
                )
                (EVIDENCE / f"engine-{section}-{state}.png").write_bytes(
                    shots["engine"]["shot"]
                )

        report["checks"] = {
            # ⚠️ **有界判据**（2026-10-30 owner 裁定 E-54 口径 A）：
            #   像素差占比 ≤ 预算（0.1%）**且** 每个差异都能归因到「两侧同元素的同一样式」
            #   ⇒ ⇒ 真缺陷（样式/几何不同）即使占比极小也会红 ⇒ ⇒ 预算不会吞掉真缺陷。
            "t1_within_budget": all(
                case["t1_pixel_within_budget"] for case in report["cases"]
            ),
            "t1b_all_diffs_attributed": all(
                case["t1b_all_diffs_attributed"] for case in report["cases"]
            ),
            "t2_dom_identical": all(case["t2_dom_equal"] for case in report["cases"]),
            "t3_same_width": all(case["t3_same_width"] for case in report["cases"]),
            "t3b_same_height": all(case["t3b_same_height"] for case in report["cases"]),
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T1 判据（2026-10-30 owner 裁定 **E-54 口径 A**）："
            "**有界**（像素差占比 ≤ 0.1%）+ **元素级归因**"
            "（每个差异必须落到两侧同元素的同一样式：tag + 关键计算样式 + 边界盒逐值相同）"
            "+ DOM/宽高全等；**严格读数保留为报告项**（t1_pixel_different / t1_pixel_ratio）。",
            "T1 判据**可失败性**：每次运行先跑反事实自检（0 差异须 PASS、超预算须 FAIL、"
            "恰好等于预算须 PASS、同图自比须 PASS、平移 1px 自比须 FAIL）"
            "⇒ 自检不过 ⇒ **exit 2**（判据坏了，不许报绿）。",
            "T2 的归一化：剔除 Svelte 作用域 class 与 `sp-*` 作用域锚点、"
            "class token 排序；**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3：两侧裁剪框宽度必须相等；**高度也要相等**"
            "（只比宽度 ⇒ 「少渲染一个后继兄弟元素」这类结构性缺失看不出来）。",
            "两态：参考侧在**装载之前**注入 `__DATA_READINGS__`；"
            "引擎侧调 `window.__DATA__.setReadings()` ⇒ **两侧读同一份固定数据**。",
        ]
        report["passed"] = all(report["checks"].values())
        (EVIDENCE / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (OUT / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        log("")
        log("── 判据 ──")
        for key, value in report["checks"].items():
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        return 0 if report["passed"] else 1
    finally:
        import signal

        try:
            os.killpg(os.getpgid(chrome.pid), signal.SIGKILL)
        except Exception:
            chrome.kill()
        httpd.shutdown()


if __name__ == "__main__":
    raise SystemExit(main())
