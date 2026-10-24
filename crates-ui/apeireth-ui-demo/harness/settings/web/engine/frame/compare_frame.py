#!/usr/bin/env python3
"""**scratch**：设置页 · `frame` 面的对照（像素 + DOM 全等 + 宽度同源）。

## 为什么是 scratch（诚实说明）

`harness/settings/tools/compare_settings.py` 是**共享文件**（切分线的骨架），
它的 `collect_pair()` 仍是 `NotImplementedError` ⇒ **本面不改共享文件** ⇒
对照驱动放在 `target/frame-scratch/`（scratch 区），**复用**批一的浏览器
驱动（`harness/tools/compare.py`：起服务 / 开 Chromium / CDP / 截图 / 像素差分）。

⇒ 本面跑完后，把「共享文件需要主线补什么」写进报告。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 侧栏**导航内容**裁剪框的差异像素占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | `.settings-subnav` 归一化后**完全相等**（文本节点逐字） |
| T3 | **宽度同源断言** | 两侧 `.settings-subnav` 元素框**宽度相等**（本面是**几何基准** ⇒ 不等即判失败） |
| T4 | **几何实测** | 侧栏 200px / 内容列 border-box 900px / 内容盒 828px 的**两侧实测值**落进证据 |

## 裁剪口径（**为什么裁「导航内容」而不是整个侧栏**）

参考侧是**整页**生产容器 ⇒ 它的内容列装着**别的面**的面板（`appearance`
分支，1121.69px）⇒ 网格行把侧栏**拉伸**到 1271.92px；引擎侧本面的内容列
**为空** ⇒ 侧栏不被拉伸 ⇒ **两侧整框高度必然不等**。

⇒ 逐像素对照针对**侧栏的导航内容**（最后一个 `.subnav-group` 的底边）⇒
两侧裁剪框**同尺寸**才有可比性；**整框宽度**仍按元素框断言（**宽度同源**）。

## 用法

```bash
python3 target/frame-scratch/compare_frame.py
```

产物：`crates-ui/apeireth-ui-demo/harness/settings/evidence/frame/**`。
"""

from __future__ import annotations

import json
import os
import pathlib
import signal
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
HARNESS = REPO / "crates-ui" / "apeireth-ui-demo" / "harness"
sys.path.insert(0, str(HARNESS / "tools"))

import compare  # noqa: E402
import compare_toolcard  # noqa: E402

# ⚠️ **路径必须与「本驱动放在哪」无关**（并行面 M3 原实现从自身位置推导 ⇒
# 一旦驱动被搬进仓库（`web/engine/frame/`）路径就失效 ⇒ 报「未找到构建产物」）。
# ⇒ 改为**向上查找 harness 根**：认 `web/shared/panels.js`（子面注册表）这个标记。
def _find_settings_root(start: pathlib.Path) -> pathlib.Path:
    for candidate in [start, *start.parents]:
        if (candidate / "web" / "shared" / "panels.js").is_file():
            return candidate
    raise SystemExit(
        "找不到设置页 harness 根（向上找不到 web/shared/panels.js）"
    )


SETTINGS = _find_settings_root(pathlib.Path(__file__).resolve().parent)
DIST = SETTINGS / "dist"
EVIDENCE = SETTINGS / "evidence" / "frame"
OUT = SETTINGS / "out" / "frame"

#: 视口与阈值（沿用批一口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 归一化脚本（批一口径 + 剔除 `sp-*` 根 class）。
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
  const root = document.querySelector('.settings-subnav');
  return JSON.stringify(root ? normalize(root) : null);
})()
"""

#: **原文** dump（含空白文本节点与注释锚点 ⇒ 用来核对空白折叠）。
RAW_SCRIPT = """
(() => {
  const root = document.querySelector('.settings-subnav');
  return root ? root.outerHTML : null;
})()
"""

#: 几何（**宽度同源断言**与**内容盒 828px** 的实测来源）。
GEOMETRY_SCRIPT = """
(() => {
  const pick = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return {
      selector: sel,
      x: r.left + window.scrollX, y: r.top + window.scrollY,
      w: r.width, h: r.height,
      padding_left: parseFloat(cs.paddingLeft),
      padding_right: parseFloat(cs.paddingRight),
      content_box_width: r.width - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight),
      max_width: cs.maxWidth,
      box_sizing: cs.boxSizing,
      grid_template_columns: cs.gridTemplateColumns,
    };
  };
  // 侧栏**导航内容**的包围盒（最后一个 `.subnav-group` 的底边）。
  const groups = Array.from(document.querySelectorAll('.settings-subnav .subnav-group'));
  const navContent = groups.length
    ? {
        top: Math.min(...groups.map((el) => el.getBoundingClientRect().top + window.scrollY)),
        bottom: Math.max(...groups.map((el) => el.getBoundingClientRect().bottom + window.scrollY)),
      }
    : null;
  return JSON.stringify({
    subnav: pick('.settings-subnav'),
    content: pick('.settings-content'),
    layout: pick('.settings-layout'),
    view: pick('.settings-view'),
    block: pick('.setting-block'),
    nav_content: navContent,
    ready: window.__SETTINGS_READY__ === true,
    error: window.__SETTINGS_ERROR__ ?? null,
  });
})()
"""


def log(message: str) -> None:
    print(message, flush=True)


def nav_crop_box(cdp) -> dict | None:
    """侧栏**导航内容**的裁剪框（宽度取侧栏元素框 ⇒ 宽度同源）。"""
    raw = cdp.evaluate(
        "(() => { const side = document.querySelector('.settings-subnav');"
        " if (!side) return null; const r = side.getBoundingClientRect();"
        " const groups = Array.from(side.querySelectorAll('.subnav-group'));"
        " if (!groups.length) return null;"
        " const top = Math.min(...groups.map((el) => el.getBoundingClientRect().top + window.scrollY));"
        " const bottom = Math.max(...groups.map((el) => el.getBoundingClientRect().bottom + window.scrollY));"
        " return JSON.stringify({x: r.left + window.scrollX, y: top, w: r.width,"
        " h: bottom - top, element_h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def navigate(cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.4)


def diff_dom(left: str | None, right: str | None) -> list[str]:
    """列出两侧 DOM 的差异点（复用批一的描述器）。"""
    return compare_toolcard.describe_dom_diff(
        json.loads(left or "null"), json.loads(right or "null"), "根"
    )


def main() -> int:
    if not DIST.exists():
        log(f"未找到构建产物：{DIST}（先跑设置页 harness 的 build.sh）")
        return 2
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)

    httpd, port = compare.serve(DIST)
    base = f"http://127.0.0.1:{port}"
    chrome, devtools_port = compare.launch_chrome()
    report: dict = {
        "panel": "frame",
        "title": "页面框架与导航",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": "UTC",
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/web/reference/frame/index.html",
        "engine_page": "/web/engine/frame/index.html",
        "crop": {
            "selector": ".settings-subnav",
            "mode": "元素框（宽度） + 导航内容（高度）",
            "why": "参考侧的内容列装着别的面的面板 ⇒ 网格行把侧栏拉伸 ⇒ 整框高度两侧不等；"
            "逐像素对照只针对导航内容，宽度仍按元素框断言。",
        },
    }
    try:
        cdp = compare.open_page(devtools_port)

        # ── 参考侧 ────────────────────────────────────────────────────────
        navigate(cdp, f"{base}/web/reference/frame/index.html")
        reference_box = nav_crop_box(cdp)
        reference_geometry = json.loads(cdp.evaluate(GEOMETRY_SCRIPT) or "{}")
        reference_dom = cdp.evaluate(NORMALIZE_SCRIPT)
        reference_raw = cdp.evaluate(RAW_SCRIPT)
        reference_shot = compare.screenshot(cdp, reference_box)

        # ── 引擎侧 ────────────────────────────────────────────────────────
        navigate(cdp, f"{base}/web/engine/frame/index.html")
        engine_box = nav_crop_box(cdp)
        engine_geometry = json.loads(cdp.evaluate(GEOMETRY_SCRIPT) or "{}")
        engine_dom = cdp.evaluate(NORMALIZE_SCRIPT)
        engine_raw = cdp.evaluate(RAW_SCRIPT)
        engine_shot = compare.screenshot(cdp, engine_box)

        # ── T3：宽度同源断言 ─────────────────────────────────────────────
        width_mismatch = None
        if reference_box is None or engine_box is None:
            width_mismatch = {"reference": reference_box, "engine": engine_box}
        elif abs(reference_box["w"] - engine_box["w"]) > 0.5:
            width_mismatch = {
                "reference_width": reference_box["w"],
                "engine_width": engine_box["w"],
            }
        log(
            f"T3 宽度同源：参考侧 {reference_box and reference_box['w']}px vs "
            f"引擎侧 {engine_box and engine_box['w']}px"
        )

        # ── T1：像素 ─────────────────────────────────────────────────────
        pixel = compare.pixel_diff(reference_shot, engine_shot)
        log(
            f"T1 像素：差异 {pixel['different']} / 总 {pixel['total']} = "
            f"{pixel['ratio'] * 100:.4f}%（尺寸不一致：{pixel.get('size_mismatch')}）"
        )

        # ── T2：DOM 全等 ─────────────────────────────────────────────────
        dom_diffs = diff_dom(reference_dom, engine_dom)
        log(f"T2 DOM 全等：{'一致' if not dom_diffs else f'{len(dom_diffs)} 处差异'}")
        for line in dom_diffs[:10]:
            log(f"     {line}")

        # ── 证据落盘 ─────────────────────────────────────────────────────
        (EVIDENCE / "reference-sidebar.png").write_bytes(reference_shot)
        (EVIDENCE / "engine-sidebar.png").write_bytes(engine_shot)
        (OUT / "reference-sidebar.png").write_bytes(reference_shot)
        (OUT / "engine-sidebar.png").write_bytes(engine_shot)
        (EVIDENCE / "reference-sidebar.raw.html").write_text(
            reference_raw or "", encoding="utf-8"
        )
        (EVIDENCE / "engine-sidebar.raw.html").write_text(engine_raw or "", encoding="utf-8")
        (EVIDENCE / "reference-sidebar.normalized.json").write_text(
            reference_dom or "null", encoding="utf-8"
        )
        (EVIDENCE / "engine-sidebar.normalized.json").write_text(
            engine_dom or "null", encoding="utf-8"
        )

        # ── 控制台（两侧） ───────────────────────────────────────────────
        console = []
        for name, page in (
            ("reference", "/web/reference/frame/index.html"),
            ("engine", "/web/engine/frame/index.html"),
        ):
            cdp.drain()
            navigate(cdp, f"{base}{page}")
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
                        {"kind": entry.get("level"), "text": str(entry.get("text", ""))}
                    )
            console.append({"side": name, "messages": messages})
        (EVIDENCE / "console.json").write_text(
            json.dumps(console, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        # ── 几何证据（**宽度同源**的实测数据） ───────────────────────────
        geometry = {
            "note": "本面是**几何基准**：其它内容面据此对齐（内容盒 828px）。",
            "reference": reference_geometry,
            "engine": engine_geometry,
            "width_same_source": {
                "sidebar_width_reference": reference_geometry.get("subnav", {}).get("w"),
                "sidebar_width_engine": engine_geometry.get("subnav", {}).get("w"),
                "content_border_box_width_reference": reference_geometry.get("content", {}).get("w"),
                "content_border_box_width_engine": engine_geometry.get("content", {}).get("w"),
                "content_box_width_reference": reference_geometry.get("content", {}).get(
                    "content_box_width"
                ),
                "content_box_width_engine": engine_geometry.get("content", {}).get(
                    "content_box_width"
                ),
                "setting_block_width_reference": (reference_geometry.get("block") or {}).get("w"),
                "setting_block_width_engine": (engine_geometry.get("block") or {}).get("w"),
            },
        }
        (EVIDENCE / "geometry.json").write_text(
            json.dumps(geometry, ensure_ascii=False, indent=2), encoding="utf-8"
        )

        checks = {
            "t1_pixels_within_budget": (not pixel.get("size_mismatch"))
            and pixel["ratio"] <= PIXEL_BUDGET,
            "t2_dom_identical": not dom_diffs,
            "t3_width_same_source": width_mismatch is None,
            "t4_engine_mounted": engine_geometry.get("ready") is True
            and engine_geometry.get("error") is None,
        }
        report["t1_pixels"] = pixel
        report["t2_dom"] = {"equal": not dom_diffs, "diffs": dom_diffs}
        report["t3_width_same_source"] = {
            "equal": width_mismatch is None,
            "mismatch": width_mismatch,
            "reference_box": reference_box,
            "engine_box": engine_box,
        }
        report["geometry"] = geometry
        report["console"] = console
        report["checks"] = checks
        report["passed"] = all(checks.values())

        (EVIDENCE / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        (OUT / "report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
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
        try:
            os.killpg(os.getpgid(chrome.pid), signal.SIGKILL)
        except Exception:
            chrome.kill()
        httpd.shutdown()


if __name__ == "__main__":
    raise SystemExit(main())
