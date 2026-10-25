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
    """导航到一个页面（等它加载完）。"""
    cdp.call("Page.navigate", {"url": url})
    time.sleep(1.2)


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
        eng_box_after = element_box(cdp, ENGINE_SELECTOR)
        engine_shot = compare.screenshot(cdp, eng_box_after)
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

        pixel = compare.pixel_diff(shots["reference"]["shot"], shots["engine"]["shot"])
        dom_diffs = describe_dom_diff(shots["reference"]["dom"], shots["engine"]["dom"])
        ref_box = shots["reference"]["box"] or {}
        eng_box = shots["engine"]["box"] or {}
        same_width = abs(float(ref_box.get("w", 0)) - float(eng_box.get("w", 0))) <= 0.5
        same_height = abs(float(ref_box.get("h", 0)) - float(eng_box.get("h", 0))) <= 0.5
        result = {
            "section": section,
            "alignment": alignment,
            "t1_pixel_different": pixel["different"],
            "t1_pixel_total": pixel["total"],
            "t1_pixel_ratio": pixel["ratio"],
            "t2_dom_equal": not dom_diffs,
            "t2_dom_diffs": dom_diffs,
            "t3_same_width": same_width,
            "t3b_same_height": same_height,
        }
        result["passed"] = (
            result["t1_pixel_different"] == 0
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


def main() -> int:
    if not DIST.exists():
        log(f"未找到构建产物：{DIST}（先跑 `bash build.sh`）")
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)

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
                eng_box_after = element_box(cdp, ENGINE_SELECTOR)
                engine_shot = compare.screenshot(cdp, eng_box_after)
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
                case = {
                    "case": f"{section}-{state}",
                    "section": section,
                    "reading_state": state,
                    "alignment": alignment,
                    "t1_pixel_different": pixel["different"],
                    "t1_pixel_total": pixel["total"],
                    "t1_pixel_ratio": pixel["ratio"],
                    "t2_dom_equal": not dom_diffs,
                    "t2_dom_diffs": dom_diffs,
                    "t3_same_width": same_width,
                    "t3b_same_height": same_height,
                    "reference_box": ref_box,
                    "engine_box": eng_box,
                }
                case["passed"] = (
                    case["t1_pixel_different"] == 0
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
            "t1_all_pixels_zero": all(
                case["t1_pixel_different"] == 0 for case in report["cases"]
            ),
            "t2_dom_identical": all(case["t2_dom_equal"] for case in report["cases"]),
            "t3_same_width": all(case["t3_same_width"] for case in report["cases"]),
            "t3b_same_height": all(case["t3b_same_height"] for case in report["cases"]),
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
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
