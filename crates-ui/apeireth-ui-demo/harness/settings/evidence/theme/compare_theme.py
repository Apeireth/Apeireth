#!/usr/bin/env python3
"""主题子面（`theme`）的**面内对照驱动**：像素 + DOM 全等 + 宽度同源。

## 为什么本文件在「证据目录」里

共享的 `tools/compare_settings.py` 目前是**骨架**（`collect_pair` 直接
`NotImplementedError`，`main()` 恒返回 1）⇒ **本面自带一份可跑的驱动**，
口径与共享骨架一致（**逐字复用**批一的 `harness/tools/compare.py` 与
`compare_toolcard.py` 的浏览器驱动 / 截图 / 像素差分 / DOM 归一化）。

⇒ 主线把共享骨架接线时，**本文件可直接搬进 `tools/`**（只改面 id 与裁剪
选择器）。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（结构 / 属性 / 文本必须为零；文本节点**逐字**） |
| T3 | **宽度同源断言** | 两侧裁剪框**宽度必须相等**（不等 ⇒ **直接判失败**，不比较） |
| T4 | **内联样式逐字** | 两侧的 `style` 属性串**逐字相等**（T2 的子集 ⇒ 单独列出证据） |

## 两侧

- **参考侧**：`/web/reference/theme/index.html` —— 生产 `SettingsView`
  （**只读引入**）+ `initialSection="appearance"` ⇒ 裁剪 `.setting-block`。
- **引擎侧**：`/web/engine/theme/index.html` —— wasm 引擎独立挂载 ⇒ 裁剪
  `.sp-theme .setting-block`。

## 环境固定

视口 `1440×900`、`devicePixelRatio = 1`、时区 `UTC`、两侧同一份 CSS 与
同一份固定数据（`fixtures/theme.json`）。

## 用法

```bash
python3 evidence/theme/compare_theme.py
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
SETTINGS = HERE.parent.parent
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
OUT = SETTINGS / "out" / "theme"
EVIDENCE = HERE

#: 视口与阈值（沿用批一口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 本面的两侧裁剪选择器（参考侧 / 引擎侧）。
REFERENCE_SELECTOR = ".setting-block"
ENGINE_SELECTOR = ".sp-theme .setting-block"

#: 归一化 DOM 的脚本（**批一口径** + 「剔除 `sp-*` 根 class」这一条）。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '')
    .replace(/\bsvelte-[0-9a-z]+\b/g, '')
    .replace(/\bsp-[a-z0-9-]+\b/g, '')
    .trim();
  const normalize = (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      // 纯空白文本节点**丢弃**（不可见；组件编译器的模板空白与图标包内部的
      // 空文本节点都在此列）⇒ 两侧口径一致。
      if (!node.textContent.trim()) return null;
      // 其余文本节点**逐字**保留（不折叠、不裁剪）⇒ 前导空格等差异会被抓住。
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


def log(message: str) -> None:
    print(message, flush=True)


def normalize_script(selector: str) -> str:
    """按选择器取一份归一化脚本。"""
    return NORMALIZE_SCRIPT % json.dumps(selector)


def element_box(cdp: compare.Cdp, selector: str) -> dict | None:
    """取元素的外框（相对文档坐标）。"""
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def navigate(cdp: compare.Cdp, url: str) -> None:
    """导航到一侧并等就绪（两侧用**同一套** `window.__SETTINGS__` 契约）。"""
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.4)


def select_section(cdp: compare.Cdp, section: str) -> None:
    """切到目标分区（两侧同名同义）。"""
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.3)


def diff_dom(left: str | None, right: str | None) -> list[str]:
    """列出两侧 DOM 的差异点（逐条）。"""
    try:
        left_tree = json.loads(left or "null")
        right_tree = json.loads(right or "null")
    except json.JSONDecodeError:
        return ["DOM 不是合法 JSON"]
    if left_tree == right_tree:
        return []
    return describe_dom_diff(left_tree, right_tree, "根")


def describe_dom_diff(left, right, path: str) -> list[str]:
    """逐条列出 DOM 差异（结构 / 属性 / 文本）。"""
    if type(left) is not type(right):
        return [f"{path}: 节点类型不同（{left} / {right}）"]
    if isinstance(left, dict):
        if left.get("t") == "text":
            if left.get("v") != right.get("v"):
                return [
                    f"{path}: 文本不同 ⇒ 参考 {json.dumps(left.get('v'), ensure_ascii=False)}"
                    f" / 引擎 {json.dumps(right.get('v'), ensure_ascii=False)}"
                ]
            return []
        if left.get("tag") != right.get("tag"):
            return [f"{path}: 标签不同（{left.get('tag')} / {right.get('tag')}）"]
        diffs: list[str] = []
        left_attrs = left.get("attrs", {})
        right_attrs = right.get("attrs", {})
        for key in sorted(set(left_attrs) | set(right_attrs)):
            if left_attrs.get(key) != right_attrs.get(key):
                diffs.append(
                    f"{path}: 属性 `{key}` 不同 ⇒ 参考 {json.dumps(left_attrs.get(key), ensure_ascii=False)}"
                    f" / 引擎 {json.dumps(right_attrs.get(key), ensure_ascii=False)}"
                )
        left_children = left.get("children", [])
        right_children = right.get("children", [])
        if len(left_children) != len(right_children):
            diffs.append(
                f"{path}: 子节点数不同（{len(left_children)} / {len(right_children)}）"
            )
        for index in range(min(len(left_children), len(right_children))):
            diffs.extend(
                describe_dom_diff(
                    left_children[index], right_children[index], f"{path}/{index}"
                )
            )
        return diffs
    return []


def collect_inline_styles(cdp: compare.Cdp, selector: str) -> list[str]:
    """取裁剪框内全部元素的 `style` 属性串（**逐字**）。"""
    raw = cdp.evaluate(
        "(() => { const root = document.querySelector("
        + json.dumps(selector)
        + "); if (!root) return JSON.stringify([]);"
        " const out = [];"
        " for (const el of root.querySelectorAll('[style]')) {"
        "   out.push(el.getAttribute('style')); }"
        " return JSON.stringify(out); })()"
    )
    return json.loads(raw or "[]")


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
        "face": "theme（主题外观面板）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": "UTC",
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/web/reference/theme/index.html",
        "engine_page": "/web/engine/theme/index.html",
        "reference_selector": REFERENCE_SELECTOR,
        "engine_selector": ENGINE_SELECTOR,
        "section": "appearance",
    }
    try:
        cdp = compare.open_page(devtools_port)

        log("── 参考侧（生产 Svelte 组件） ──")
        navigate(cdp, f"{base}/web/reference/theme/index.html")
        select_section(cdp, "appearance")
        reference_box = element_box(cdp, REFERENCE_SELECTOR)
        reference_dom = cdp.evaluate(normalize_script(REFERENCE_SELECTOR))
        reference_styles = collect_inline_styles(cdp, REFERENCE_SELECTOR)
        reference_shot = compare.screenshot(cdp, reference_box)

        log("── 引擎侧（wasm 引擎） ──")
        navigate(cdp, f"{base}/web/engine/theme/index.html")
        select_section(cdp, "appearance")
        engine_box = element_box(cdp, ENGINE_SELECTOR)
        engine_dom = cdp.evaluate(normalize_script(ENGINE_SELECTOR))
        engine_styles = collect_inline_styles(cdp, ENGINE_SELECTOR)
        engine_box_before = engine_box
        engine_shot_unaligned = compare.screenshot(cdp, engine_box)

        # ── 装置对齐：两侧内容必须落在**同一个设备像素格** ──────────────────
        # 参考侧是**生产整页**（页头 + 侧栏 + 内容列）⇒ 它的内容块在文档里的
        # 位置**带小数**（本面实测 `top = 219.765625`）；引擎侧是**独立挂载**
        # ⇒ 位置是 `0`。
        # ⇒ 两边的**设备像素格**错开 0.765625px ⇒ 渐变条纹 / 圆角抗锯齿 /
        #   字形栅格化**必然不同** ⇒ 那是**装置伪差**，不是引擎画错了。
        # ⇒ 这里把引擎侧的舞台**对齐**到参考侧的文档位置（同一像素格）⇒
        #   差异只反映**渲染本身**。
        # ⚠️ 对齐前 / 对齐后两个数字**都**落进报告 ⇒ **不隐藏**任何差异。
        delta_y = (reference_box["y"] if reference_box else 0.0) - (
            engine_box["y"] if engine_box else 0.0
        )
        delta_x = (reference_box["x"] if reference_box else 0.0) - (
            engine_box["x"] if engine_box else 0.0
        )
        aligned = cdp.evaluate(
            "(() => { const stage = document.querySelector('.settings-stage');"
            " if (!stage) return false;"
            " stage.style.marginTop = %r + 'px';"
            " stage.style.marginLeft = %r + 'px';"
            " return true; })()"
            % (delta_y, delta_x)
        )
        time.sleep(0.3)
        engine_box = element_box(cdp, ENGINE_SELECTOR)
        engine_shot = compare.screenshot(cdp, engine_box)
        unaligned = compare.pixel_diff(reference_shot, engine_shot_unaligned)
        report["alignment"] = {
            "why": "两侧内容必须落在同一个设备像素格 ⇒ 渐变 / 圆角抗锯齿 / 字形栅格化才可比",
            "reference_origin": {
                "x": (reference_box or {}).get("x"),
                "y": (reference_box or {}).get("y"),
            },
            "engine_origin_before": {
                "x": (engine_box_before or {}).get("x"),
                "y": (engine_box_before or {}).get("y"),
            },
            "delta_x": delta_x,
            "delta_y": delta_y,
            "applied": aligned,
            "engine_origin_after": {
                "x": (engine_box or {}).get("x"),
                "y": (engine_box or {}).get("y"),
            },
            "pixel_ratio_unaligned": unaligned["ratio"],
            "pixel_different_unaligned": unaligned["different"],
        }
        log(
            "装置对齐：引擎侧舞台偏移 "
            f"(margin-left {delta_x}px / margin-top {delta_y}px)"
            f" ⇒ 对齐前像素差 {unaligned['ratio'] * 100:.4f}%"
        )

        # ── T3：宽度同源断言（不等 ⇒ 直接判失败） ─────────────────────────
        width_ok = (
            reference_box is not None
            and engine_box is not None
            and abs(reference_box["w"] - engine_box["w"]) <= 0.5
        )
        report["t3_width"] = {
            "reference": reference_box,
            "engine": engine_box,
            "equal": width_ok,
        }
        if not width_ok:
            log(
                f"T3 宽度不同源：参考侧 {reference_box} / 引擎侧 {engine_box}"
                " ⇒ 先修舞台宽度，不要把它算成像素差异。"
            )

        # ── T1：静态像素对照 ────────────────────────────────────────────────
        pixel = compare.pixel_diff(reference_shot, engine_shot)
        report["t1_pixels"] = pixel
        log(
            f"T1 像素：差异 {pixel['different']} / 总 {pixel['total']}"
            f" = {pixel['ratio'] * 100:.4f}%"
        )

        # ── T2：DOM 结构对照（归一化后全等） ────────────────────────────────
        dom_diffs = diff_dom(reference_dom, engine_dom)
        report["t2_dom"] = {"equal": not dom_diffs, "diffs": dom_diffs}
        log(f"T2 DOM 全等：{'一致' if not dom_diffs else f'{len(dom_diffs)} 处差异'}")
        for line in dom_diffs[:10]:
            log(f"     {line}")

        # ── T4：内联样式逐字 ────────────────────────────────────────────────
        styles_equal = reference_styles == engine_styles
        report["t4_inline_styles"] = {
            "equal": styles_equal,
            "reference": reference_styles,
            "engine": engine_styles,
        }
        log(
            f"T4 内联样式逐字：{'相等' if styles_equal else '不等'}"
            f"（{len(reference_styles)} / {len(engine_styles)} 条）"
        )
        if not styles_equal:
            for index, (left, right) in enumerate(
                zip(reference_styles, engine_styles)
            ):
                if left != right:
                    log(f"     #{index} 参考 {left!r}")
                    log(f"     #{index} 引擎 {right!r}")

        checks = {
            "t1_pixels_within_budget": (not pixel["size_mismatch"])
            and pixel["ratio"] <= PIXEL_BUDGET,
            "t2_dom_identical": not dom_diffs,
            "t3_same_width": width_ok,
            "t4_inline_styles_identical": styles_equal,
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T2 的归一化：剔除 Svelte 作用域 class 与引擎侧的 `sp-*` 作用域锚点、"
            "class token 排序；**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3 是**宽度同源**：参考侧靠生产自己的布局得到 828px，引擎侧靠"
            "对照专用样式表的 `.settings-stage` 钉成 828px ⇒ 不等即判失败。",
            "T4 是**逐字**比对 `style` 属性串（浏览器 CSSOM 的序列化口径）。",
            "**装置对齐**：参考侧是生产整页（内容块在文档里的位置带小数），"
            "引擎侧是独立挂载（位置 0）⇒ 两侧的设备像素格错开 ⇒ 渐变条纹 / "
            "圆角抗锯齿 / 字形栅格化必然不同 ⇒ 那是**装置伪差**。本驱动把"
            "引擎侧的舞台**对齐**到参考侧的文档位置后再比 ⇒ 差异只反映渲染本身。"
            "**对齐前 / 对齐后两个数字都在报告里**（`alignment` 段）⇒ 不隐藏。",
            "环境：视口 1440×900、DPR 1、时区 UTC。",
        ]
        report["checks"] = checks
        report["passed"] = all(checks.values())

        (OUT / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (EVIDENCE / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (EVIDENCE / "reference-appearance.png").write_bytes(reference_shot)
        (EVIDENCE / "engine-appearance.png").write_bytes(engine_shot)
        (OUT / "reference-appearance.png").write_bytes(reference_shot)
        (OUT / "engine-appearance.png").write_bytes(engine_shot)

        # 浏览器控制台日志（两侧：收集导航后的控制台消息与异常）。
        console = []
        for name, page in (
            ("reference", "/web/reference/theme/index.html"),
            ("engine", "/web/engine/theme/index.html"),
        ):
            cdp.drain()
            navigate(cdp, f"{base}{page}")
            time.sleep(0.4)
            messages = []
            for message in cdp.drain():
                method = message.get("method")
                if method == "Runtime.consoleAPICalled":
                    params = message.get("params", {})
                    texts = []
                    for arg in params.get("args", []):
                        texts.append(str(arg.get("value", arg.get("description", ""))))
                    messages.append({"kind": params.get("type"), "text": " ".join(texts)})
                elif method == "Runtime.exceptionThrown":
                    details = message.get("params", {}).get("exceptionDetails", {})
                    messages.append(
                        {
                            "kind": "exception",
                            "text": str(details.get("text", "")),
                            "detail": str(details.get("exception", {}).get("description", "")),
                        }
                    )
                elif method == "Log.entryAdded":
                    entry = message.get("params", {}).get("entry", {})
                    messages.append(
                        {"kind": entry.get("level"), "text": str(entry.get("text"))}
                    )
            console.append({"side": name, "messages": messages})
        (EVIDENCE / "console.json").write_text(
            json.dumps(console, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        log("")
        log("── 判据 ──")
        for key, value in checks.items():
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        cdp.close()
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
