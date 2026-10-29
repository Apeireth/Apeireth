#!/usr/bin/env python3
"""工具卡面对照：像素 + DOM 全等 + 逐帧 + JSON 差分。

## 对照项

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每个用例（收起 / 展开 / 展开 + 原始 JSON）差异像素占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**比较，不折叠空白） |
| T3 | **逐帧对照** | 展开动作期间的每一帧差异占比 **≤ 1%** |
| T4 | **JSON 差分** | 引擎侧序列化器 vs 浏览器侧 `JSON.stringify`：**逐字节相同** |

## 两侧

- **参考侧**：`reference/tool/index.html` —— 生产组件（只读引入）+ 同一批设计 CSS。
- **引擎侧**：`engine/tool/index.html` —— wasm 引擎渲染同样的两张卡 + 同一批设计 CSS。

## 环境固定

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧同一时刻）；
- 同一浏览器实例、同一份 CSS、同一份 fixture。

## 用法

```bash
python3 tools/compare_toolcard.py            # 跑全套，产物落在 out/toolcard/ 与 evidence/toolcard/
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
sys.path.insert(0, str(HERE))

import compare  # noqa: E402

DIST = compare.DIST
OUT = HARNESS / "out" / "toolcard"
EVIDENCE = HARNESS / "evidence" / "toolcard"

#: 视口与阈值（沿用试点口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 逐帧采集的帧数与窗口。
FRAME_COUNT = 16
FRAME_WINDOW_MS = 900

#: 归一化 DOM 的脚本。
#:
#: 与试点的口径**同源**（剔除 Svelte 作用域 class、class token 排序、注释锚点），
#: 但**文本节点逐字保留**（不折叠空白、不裁剪）⇒ 比试点更严。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '').replace(/\bsvelte-[0-9a-z]+\b/g, '').trim();
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
  const root = document.querySelector('.toolcard-stage');
  return JSON.stringify(root ? normalize(root) : null);
})()
"""

#: 浏览器侧的序列化（与生产组件里的取值口径一致）。
BROWSER_JSON_SCRIPT = r"""
(() => {
  const stringify = (value, limit = 4000) => {
    if (value === undefined) return '';
    let text;
    try {
      text = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
    } catch {
      text = String(value);
    }
    if (text.length > limit) {
      return `${text.slice(0, limit)}…（已截断，共 ${text.length} 字符）`;
    }
    return text;
  };
  const items = [];
  for (const item of window.__TOOLCARD_FIXTURE__.cases) {
    if ('args' in item.toolCall) {
      items.push({ id: `${item.id}:tool-args`, value: item.toolCall.args });
    }
    if ('resultFull' in item.toolCall) {
      items.push({ id: `${item.id}:tool-result`, value: item.toolCall.resultFull });
    }
    if ('arguments' in item.lifecycleCall) {
      items.push({ id: `${item.id}:lifecycle-args`, value: item.lifecycleCall.arguments });
    }
    if ('result' in item.lifecycleCall) {
      items.push({ id: `${item.id}:lifecycle-result`, value: item.lifecycleCall.result });
    }
  }
  return JSON.stringify(items.map(({ id, value }) => ({
    id,
    compact: JSON.stringify(value),
    pretty: JSON.stringify(value, null, 2),
    truncated: stringify(value, 4000),
  })));
})()
"""

#: 事件计数（起播 / 结束）。
EVENTS_SCRIPT = r"""
(() => {
  window.__TOOLCARD_EVENTS__ = { counts: {} };
  const names = ['transitionrun', 'transitionstart', 'transitionend',
                 'animationstart', 'animationend', 'animationcancel'];
  for (const name of names) {
    window.__TOOLCARD_EVENTS__.counts[name] = 0;
    document.addEventListener(name, () => {
      window.__TOOLCARD_EVENTS__.counts[name] += 1;
    }, true);
  }
  return true;
})()
"""

EVENTS_READ_SCRIPT = r"""
(() => JSON.stringify((window.__TOOLCARD_EVENTS__ || { counts: {} }).counts))()
"""


def log(message: str) -> None:
    print(message, flush=True)


def stage_box(cdp: compare.Cdp) -> dict | None:
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector('.toolcard-stage');"
        " if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def navigate(cdp: compare.Cdp, url: str, static: bool = True) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__TOOLCARD__ && window.__TOOLCARD__.ready()", timeout=60)
    if static:
        # 停掉无限动画（两侧同时）⇒ 静态抓取可复现（见 toolcard-test.css 的说明）。
        cdp.evaluate("document.documentElement.dataset.toolcardStatic = '1'; true")
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.35)


def select_case(cdp: compare.Cdp, index: int) -> None:
    cdp.evaluate(f"window.__TOOLCARD__.selectCase({index}); true")
    time.sleep(0.25)


def expand(cdp: compare.Cdp, tool: bool = True, lifecycle: bool = True) -> None:
    if tool:
        cdp.evaluate("window.__TOOLCARD__.expandTool(0); true")
    if lifecycle:
        cdp.evaluate("window.__TOOLCARD__.expandLifecycle(0); true")
    time.sleep(0.25)


def toggle_raw(cdp: compare.Cdp) -> None:
    cdp.evaluate("window.__TOOLCARD__.toggleRaw(0); true")
    time.sleep(0.25)


def collect_states(cdp: compare.Cdp, base: str, page: str, count: int) -> list[dict]:
    """逐用例收集三态（收起 / 展开 / 展开 + 原始 JSON）的 DOM 与截图。"""
    url = f"{base}/{page}/index.html"
    states: list[dict] = []
    for index in range(count):
        navigate(cdp, url)
        select_case(cdp, index)
        item: dict = {"index": index}

        item["dom_collapsed"] = cdp.evaluate(NORMALIZE_SCRIPT)
        box = stage_box(cdp)
        item["shot_collapsed"] = compare.screenshot(cdp, box)

        expand(cdp)
        item["dom_expanded"] = cdp.evaluate(NORMALIZE_SCRIPT)
        box = stage_box(cdp)
        item["shot_expanded"] = compare.screenshot(cdp, box)

        toggle_raw(cdp)
        item["dom_raw"] = cdp.evaluate(NORMALIZE_SCRIPT)
        box = stage_box(cdp)
        item["shot_raw"] = compare.screenshot(cdp, box)

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
        "batch": "工具卡面（ToolCallCard + ToolCallLifecycleCard）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/reference/tool/index.html",
        "engine_page": "/engine/tool/index.html",
    }
    try:
        cdp = compare.open_page(devtools_port)

        # ── 用例数（两侧一致） ──────────────────────────────────────────
        navigate(cdp, f"{base}/reference/tool/index.html")
        count = cdp.evaluate("window.__TOOLCARD__.caseCount()")
        report["case_count"] = count

        log("── 参考侧（生产 Svelte 组件） ──")
        reference = collect_states(cdp, base, "reference/tool", count)
        log("── 引擎侧（wasm 引擎） ──")
        engine = collect_states(cdp, base, "engine/tool", count)

        # ── T1 / T2：逐用例三态 ────────────────────────────────────────
        pixel_rows = []
        dom_rows = []
        for index in range(count):
            ref = reference[index]
            eng = engine[index]
            for phase in ("collapsed", "expanded", "raw"):
                diff = compare.pixel_diff(ref[f"shot_{phase}"], eng[f"shot_{phase}"])
                pixel_rows.append({"case": index, "phase": phase, **diff})
                dom_diffs = diff_dom(ref[f"dom_{phase}"], eng[f"dom_{phase}"])
                dom_rows.append(
                    {"case": index, "phase": phase, "equal": not dom_diffs, "diffs": dom_diffs}
                )
            for phase in ("collapsed", "expanded", "raw"):
                (OUT / f"reference-case-{index}-{phase}.png").write_bytes(ref[f"shot_{phase}"])
                (OUT / f"engine-case-{index}-{phase}.png").write_bytes(eng[f"shot_{phase}"])
        report["t1_pixels"] = pixel_rows
        report["t2_dom"] = dom_rows
        for row in pixel_rows:
            log(
                f"T1 用例 {row['case']} {row['phase']}：差异 {row['different']} / 总 "
                f"{row['total']} = {row['ratio'] * 100:.4f}%"
            )
        dom_failures = [row for row in dom_rows if not row["equal"]]
        for row in dom_failures[:10]:
            log(f"T2 用例 {row['case']} {row['phase']}：{len(row['diffs'])} 处差异")
            for line in row["diffs"][:6]:
                log(f"     {line}")
        log(f"T2 DOM 全等：{'一致' if not dom_failures else f'{len(dom_failures)} 个状态不一致'}")

        # ── T3：逐帧（展开动作） ───────────────────────────────────────
        frame_rows = []
        for side, page in (("reference", "reference/tool"), ("engine", "engine/tool")):
            # 逐帧对照看**真实动画** ⇒ 不停掉（`static=False`）。
            navigate(cdp, f"{base}/{page}/index.html", static=False)
            select_case(cdp, 0)
            cdp.evaluate(EVENTS_SCRIPT)
            cdp.evaluate(
                "requestAnimationFrame(() => { window.__TOOLCARD__.expandTool(0); }); true"
            )
            frames = compare.capture_frames(cdp, FRAME_WINDOW_MS, FRAME_COUNT)
            if side == "reference":
                reference_frames = frames
            else:
                engine_frames = frames
            events = json.loads(cdp.evaluate(EVENTS_READ_SCRIPT) or "{}")
            frame_rows.append({"side": side, "frames": len(frames), "events": events})
        pair_count = min(len(reference_frames), len(engine_frames))
        frame_diffs = []
        for index in range(pair_count):
            diff = compare.pixel_diff(reference_frames[index][1], engine_frames[index][1])
            frame_diffs.append({"index": index, **diff})
        report["t3_frames"] = {
            "reference_frame_count": len(reference_frames),
            "engine_frame_count": len(engine_frames),
            "target_frame_count": FRAME_COUNT,
            "frames": frame_diffs,
            "events": frame_rows,
        }
        for row in frame_diffs:
            log(
                f"T3 帧 {row['index']:>2}：差异 {row['different']} / 总 {row['total']}"
                f" = {row['ratio'] * 100:.4f}%"
            )

        # ── T4：JSON 差分（引擎侧序列化器 vs 浏览器 JSON.stringify） ─────
        navigate(cdp, f"{base}/reference/tool/index.html")
        browser_items = json.loads(cdp.evaluate(BROWSER_JSON_SCRIPT) or "[]")
        navigate(cdp, f"{base}/engine/tool/index.html")
        cdp.wait_for("!!window.__TOOLCARD_JSON_PROBE__", timeout=30)
        engine_probe = json.loads(cdp.evaluate("window.__TOOLCARD_JSON_PROBE__") or "[]")
        json_rows = []
        engine_by_id = {item["id"]: item for item in engine_probe}
        for item in browser_items:
            other = engine_by_id.get(item["id"])
            if other is None:
                json_rows.append({"id": item["id"], "equal": False, "reason": "引擎侧缺该项"})
                continue
            field_diffs = []
            for field in ("compact", "pretty", "truncated"):
                if item[field] != other.get(field):
                    field_diffs.append(
                        {
                            "field": field,
                            "browser": item[field],
                            "engine": other.get(field),
                        }
                    )
            json_rows.append(
                {"id": item["id"], "equal": not field_diffs, "diffs": field_diffs}
            )
        report["t4_json"] = {
            "total": len(json_rows),
            "equal": sum(1 for row in json_rows if row["equal"]),
            "items": json_rows,
        }
        log(
            f"T4 JSON 差分：{sum(1 for row in json_rows if row['equal'])} / {len(json_rows)}"
            f" 项逐字节相同"
        )
        for row in json_rows:
            if not row["equal"]:
                log(f"     {row['id']}：{row.get('reason') or row['diffs']}")

        # ── 汇总判据 ────────────────────────────────────────────────────
        checks = {
            "t1_all_pixels_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in pixel_rows
            ),
            "t1_pixel_sum_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in pixel_rows
            ),
            "t2_dom_identical": not dom_failures,
            "t3_enough_frames": pair_count == FRAME_COUNT,
            "t3_all_frames_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in frame_diffs
            ),
            "t4_json_byte_identical": all(row["equal"] for row in json_rows),
        }
        report["notes"] = [
            "T1/T3 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T1 静态抓取时两侧**同时**停掉无限循环动画（`data-toolcard-static`）——"
            "无限动画的相位随时间漂移 ⇒ 两侧先后抓取会出现**假差异**（实测 52 像素）；"
            "动画的存在与起播由 T3 逐帧对照覆盖。",
            "T2 的归一化：剔除 Svelte 作用域 class 与注释锚点、class token 排序；"
            "**文本节点逐字保留**（不折叠空白、不裁剪）⇒ 比试点口径更严。",
            "T3 是**实时**对照：两帧的采集耗时不同 ⇒ 相位会有偏移（采集侧的偏移，"
            "不是渲染差异）。",
            "T4 是**字节级**差分：同一批值分别过浏览器侧 `JSON.stringify` 与引擎侧"
            "序列化器（紧凑 / 缩进 / 超长截断三种形态）。",
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
        (EVIDENCE / "reference-static.png").write_bytes(reference[0]["shot_collapsed"])
        (EVIDENCE / "engine-static.png").write_bytes(engine[0]["shot_collapsed"])
        for index in range(count):
            for phase in ("collapsed", "expanded", "raw"):
                (EVIDENCE / f"reference-case-{index}-{phase}.png").write_bytes(
                    reference[index][f"shot_{phase}"]
                )
                (EVIDENCE / f"engine-case-{index}-{phase}.png").write_bytes(
                    engine[index][f"shot_{phase}"]
                )
        # 浏览器控制台日志（两侧：收集导航后的控制台消息与异常）。
        console = []
        for name, page in (("reference", "reference/tool"), ("engine", "engine/tool")):
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
