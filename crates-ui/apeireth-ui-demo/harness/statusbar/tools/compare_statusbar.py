#!/usr/bin/env python3
"""状态条面对照：像素 + DOM 全等 + 逐帧 + 点击回调证据。

## 对照项

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每个用例差异像素占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**比较，不折叠空白） |
| T3 | **逐帧对照** | 悬停过渡期间的每一帧差异占比 **≤ 1%** |
| T4 | **点击回调证据** | 点第 0 格 ⇒ 两侧回调计数都 **+1**（证明 Rust 侧的回调**真的**接上了） |

## 两侧

- **参考侧**：`reference/statusbar/index.html` —— 生产组件（只读引入）+ 同一批设计 CSS。
- **引擎侧**：`engine/statusbar/index.html` —— wasm 引擎渲染同样的状态条 + 同一批设计 CSS。

## 环境固定

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧同一时刻）；
- 同一浏览器实例、同一份 CSS、同一份 fixture。

## 用法

```bash
python3 tools/compare_statusbar.py            # 跑全套，产物落在 out/ 与 evidence/
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
HARNESS = HERE.parent
# 共享的 CDP / 像素对照工具（**只读复用**，不改）。
sys.path.insert(0, str(HARNESS.parent / "tools"))

import compare  # noqa: E402

DIST = HARNESS / "dist"
OUT = HARNESS / "out"
EVIDENCE = HARNESS / "evidence"

#: 视口与阈值（沿用试点口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 逐帧采集的帧数与窗口（悬停过渡 180ms ⇒ 窗口略大于它）。
FRAME_COUNT = 12
FRAME_WINDOW_MS = 600

#: 归一化 DOM 的脚本。
#:
#: 与批一口径**同源**（剔除 Svelte 作用域 class、class token 排序、注释锚点），
#: 但**文本节点逐字保留**（不折叠空白、不裁剪）⇒ 比试点更严。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '').replace(/\bsvelte-[0-9a-z]+\b/g, '').trim();
  const normalize = (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      // 纯空白文本节点**丢弃**（不可见）⇒ 两侧口径一致。
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
    // 属性键**排序** ⇒ 两侧的属性书写顺序不同也不算差异。
    const sorted = {};
    for (const key of Object.keys(attrs).sort()) {
      sorted[key] = attrs[key];
    }
    const children = Array.from(node.childNodes).map(normalize).filter(Boolean);
    return { t: 'el', tag: node.tagName.toLowerCase(), attrs: sorted, children };
  };
  const root = document.querySelector('.statusbar-stage');
  return JSON.stringify(root ? normalize(root) : null);
})()
"""

#: 事件计数（过渡起播 / 结束）。
EVENTS_SCRIPT = r"""
(() => {
  window.__STATUSBAR_EVENTS__ = { counts: {} };
  const names = ['transitionrun', 'transitionstart', 'transitionend',
                 'animationstart', 'animationend', 'animationcancel'];
  for (const name of names) {
    window.__STATUSBAR_EVENTS__.counts[name] = 0;
    document.addEventListener(name, () => {
      window.__STATUSBAR_EVENTS__.counts[name] += 1;
    }, true);
  }
  return true;
})()
"""

EVENTS_READ_SCRIPT = r"""
(() => JSON.stringify((window.__STATUSBAR_EVENTS__ || { counts: {} }).counts))()
"""


def log(message: str) -> None:
    print(message, flush=True)


def stage_box(cdp: compare.Cdp) -> dict | None:
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector('.statusbar-stage');"
        " if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def cell_center(cdp: compare.Cdp, index: int) -> dict | None:
    """第 `index` 格的中心坐标（视口坐标）。"""
    raw = cdp.evaluate(
        "(() => { const el = document.querySelectorAll('.sb-item')[%d];"
        " if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + r.width / 2, y: r.top + r.height / 2}); })()"
        % index
    )
    return json.loads(raw) if raw else None


def move_mouse(cdp: compare.Cdp, x: float, y: float) -> None:
    cdp.call("Input.dispatchMouseEvent", {"type": "mouseMoved", "x": x, "y": y, "buttons": 0})


def navigate(cdp: compare.Cdp, url: str, static: bool = True) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__STATUSBAR__ && window.__STATUSBAR__.ready()", timeout=60)
    if static:
        # 停掉过渡 / 动画（两侧同时）⇒ 静态抓取可复现。
        cdp.evaluate("document.documentElement.dataset.statusbarStatic = '1'; true")
    # 鼠标挪到**容器外**（避免悬停态混进静态抓取）。
    move_mouse(cdp, 1, 1)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.35)


def select_case(cdp: compare.Cdp, index: int) -> None:
    cdp.evaluate(f"window.__STATUSBAR__.selectCase({index}); true")
    time.sleep(0.35)


def collect_states(cdp: compare.Cdp, base: str, page: str, count: int) -> list[dict]:
    """逐用例收集 DOM 与截图。"""
    url = f"{base}/{page}/index.html"
    states: list[dict] = []
    for index in range(count):
        navigate(cdp, url)
        select_case(cdp, index)
        item: dict = {"index": index}
        item["dom"] = cdp.evaluate(NORMALIZE_SCRIPT)
        box = stage_box(cdp)
        item["shot"] = compare.screenshot(cdp, box)
        states.append(item)
    return states


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


def capture_hover_frames(cdp: compare.Cdp, base: str, page: str) -> tuple[list, dict]:
    """悬停过渡的逐帧采集（**实时**；不停掉过渡）。"""
    navigate(cdp, f"{base}/{page}/index.html", static=False)
    select_case(cdp, 0)
    cdp.evaluate(EVENTS_SCRIPT)
    # 鼠标先停在容器外 ⇒ 颜色处于「未悬停」稳态。
    move_mouse(cdp, 1, 1)
    time.sleep(0.4)
    center = cell_center(cdp, 0)
    if center is None:
        return [], {}
    box = stage_box(cdp)
    # 触发悬停 ⇒ 立刻连拍（过渡 180ms）。
    move_mouse(cdp, center["x"], center["y"])
    frames = compare.capture_frames(cdp, FRAME_WINDOW_MS, FRAME_COUNT)
    events = json.loads(cdp.evaluate(EVENTS_READ_SCRIPT) or "{}")
    return frames, {"events": events, "box": box}


def within_side_changes(frames: list) -> list[int]:
    """同一侧**相邻帧**的差异像素数（证明采集窗口真的覆盖了动画）。"""
    changes: list[int] = []
    for index in range(1, len(frames)):
        diff = compare.pixel_diff(frames[index - 1][1], frames[index][1])
        changes.append(diff["different"])
    return changes


def count_elements(tree) -> int:
    """归一化树里的**元素**个数（用来证明 DOM 对照不是「空树对空树」）。"""
    if not isinstance(tree, dict):
        return 0
    total = 1 if tree.get("t") == "el" else 0
    for child in tree.get("children", []):
        total += count_elements(child)
    return total


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
        "batch": "状态条面（StatusBar + StatusBadge）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": "UTC",
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/reference/statusbar/index.html",
        "engine_page": "/engine/statusbar/index.html",
    }
    try:
        cdp = compare.open_page(devtools_port)

        # ── 用例数（两侧一致） ──────────────────────────────────────────
        navigate(cdp, f"{base}/reference/statusbar/index.html")
        count = cdp.evaluate("window.__STATUSBAR__.caseCount()")
        report["case_count"] = count

        log("── 参考侧（生产 Svelte 组件） ──")
        reference = collect_states(cdp, base, "reference/statusbar", count)
        log("── 引擎侧（wasm 引擎） ──")
        engine = collect_states(cdp, base, "engine/statusbar", count)

        # ── T1：逐用例静态像素 ─────────────────────────────────────────
        pixel_rows = []
        dom_rows = []
        for index in range(count):
            ref = reference[index]
            eng = engine[index]
            diff = compare.pixel_diff(ref["shot"], eng["shot"])
            pixel_rows.append({"case": index, **diff})
            dom_diffs = diff_dom(ref["dom"], eng["dom"])
            dom_rows.append(
                {"case": index, "equal": not dom_diffs, "diffs": dom_diffs}
            )
            (OUT / f"reference-case-{index}.png").write_bytes(ref["shot"])
            (OUT / f"engine-case-{index}.png").write_bytes(eng["shot"])
        report["t1_pixels"] = pixel_rows
        report["t2_dom"] = dom_rows
        # 元素计数（两侧各自）⇒ 证明 DOM 对照**不是**「空树对空树」。
        element_counts = []
        for index in range(count):
            reference_tree = json.loads(reference[index]["dom"] or "null")
            engine_tree = json.loads(engine[index]["dom"] or "null")
            element_counts.append(
                {
                    "case": index,
                    "reference": count_elements(reference_tree),
                    "engine": count_elements(engine_tree),
                }
            )
        report["t2_dom_element_counts"] = element_counts
        for row in pixel_rows:
            log(
                f"T1 用例 {row['case']}：差异 {row['different']} / 总 {row['total']}"
                f" = {row['ratio'] * 100:.4f}%"
            )
        dom_failures = [row for row in dom_rows if not row["equal"]]
        for row in dom_failures[:10]:
            log(f"T2 用例 {row['case']}：{len(row['diffs'])} 处差异")
            for line in row["diffs"][:8]:
                log(f"     {line}")
        log(f"T2 DOM 全等：{'一致' if not dom_failures else f'{len(dom_failures)} 个用例不一致'}")

        # ── T3：逐帧（悬停过渡） ───────────────────────────────────────
        frame_rows = []
        reference_frames: list = []
        engine_frames: list = []
        for side, page in (("reference", "reference/statusbar"), ("engine", "engine/statusbar")):
            frames, meta = capture_hover_frames(cdp, base, page)
            if side == "reference":
                reference_frames = frames
            else:
                engine_frames = frames
            frame_rows.append({"side": side, "frames": len(frames), **meta})
        pair_count = min(len(reference_frames), len(engine_frames))
        frame_diffs = []
        for index in range(pair_count):
            diff = compare.pixel_diff(reference_frames[index][1], engine_frames[index][1])
            frame_diffs.append({"index": index, **diff})
        # 同一侧**相邻帧**的差异 ⇒ 证明「采集窗口真的覆盖了动画」
        # （否则「两侧逐帧相同」可能是**空转**：两侧都没动）。
        within_side = {
            "reference": within_side_changes(reference_frames),
            "engine": within_side_changes(engine_frames),
        }
        report["t3_frames"] = {
            "reference_frame_count": len(reference_frames),
            "engine_frame_count": len(engine_frames),
            "target_frame_count": FRAME_COUNT,
            "frames": frame_diffs,
            "events": frame_rows,
            "within_side_changed_pixels": within_side,
        }
        for row in frame_diffs:
            log(
                f"T3 帧 {row['index']:>2}：差异 {row['different']} / 总 {row['total']}"
                f" = {row['ratio'] * 100:.4f}%"
            )
        log(
            "T3 采集窗口内的**同侧**帧间变化（证明动画被采到）："
            f"参考 {within_side['reference']} / 引擎 {within_side['engine']}"
        )

        # ── T4：点击回调证据（两侧回调都真的被调用） ───────────────────
        click_rows = []
        for side, page in (
            ("reference", "reference/statusbar"),
            ("engine", "engine/statusbar"),
        ):
            navigate(cdp, f"{base}/{page}/index.html")
            before = cdp.evaluate("window.__STATUSBAR__.clickCount()")
            cdp.evaluate("window.__STATUSBAR__.clickCell(0); true")
            time.sleep(0.15)
            after = cdp.evaluate("window.__STATUSBAR__.clickCount()")
            click_rows.append(
                {"side": side, "before": before, "after": after, "delta": after - before}
            )
        report["t4_click"] = click_rows
        for row in click_rows:
            log(f"T4 {row['side']}：回调计数 {row['before']} ⇒ {row['after']}")

        # ── 汇总判据 ────────────────────────────────────────────────────
        checks = {
            "t1_all_pixels_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in pixel_rows
            ),
            "t2_dom_identical": not dom_failures,
            "t2_dom_not_trivial": all(
                row["reference"] >= 6 and row["reference"] == row["engine"]
                for row in element_counts
            ),
            "t3_enough_frames": pair_count == FRAME_COUNT,
            "t3_all_frames_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in frame_diffs
            ),
            "t3_transition_ran_on_both_sides": all(
                row["events"].get("transitionrun", 0) > 0 for row in frame_rows
            ),
            "t3_captured_the_transition": all(
                max(changes) > 0 for changes in within_side.values() if changes
            ),
            "t4_click_callback_fired_on_both_sides": all(
                row["delta"] == 1 for row in click_rows
            ),
        }
        report["notes"] = [
            "T1/T3 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T1 静态抓取时两侧**同时**停掉过渡 / 动画（`data-statusbar-static`）"
            "并把鼠标挪到容器外 ⇒ 画面只取决于渲染实现（可复现）。",
            "T2 的归一化：剔除 Svelte 作用域 class 与注释锚点、class token 排序、"
            "属性键排序；**文本节点逐字保留**（不折叠空白、不裁剪）⇒ 比试点口径更严。",
            "T3 是**实时**对照：悬停过渡（color 0.18s）在两侧各跑一次 ⇒ "
            "两帧的采集耗时不同 ⇒ 相位会有偏移（采集侧的偏移，不是渲染差异）。",
            "T4 是**行为**证据：点第 0 格 ⇒ 参考侧（Svelte 状态）与引擎侧"
            "（Rust 计数器）的回调计数都 +1 ⇒ 回调**真的**接上了。",
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
        # 证据：两张静态图（参考 vs 引擎）+ 逐用例图。
        (EVIDENCE / "reference-static.png").write_bytes(reference[0]["shot"])
        (EVIDENCE / "engine-static.png").write_bytes(engine[0]["shot"])
        for index in range(count):
            (EVIDENCE / f"reference-case-{index}.png").write_bytes(
                reference[index]["shot"]
            )
            (EVIDENCE / f"engine-case-{index}.png").write_bytes(engine[index]["shot"])
        # 浏览器控制台日志（两侧：收集导航后的控制台消息与异常）。
        console = []
        for name, page in (
            ("reference", "reference/statusbar"),
            ("engine", "engine/statusbar"),
        ):
            cdp.drain()
            navigate(cdp, f"{base}/{page}/index.html")
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
                        {"kind": entry.get("level"), "text": str(entry.get("text", ""))}
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
    sys.exit(main())
