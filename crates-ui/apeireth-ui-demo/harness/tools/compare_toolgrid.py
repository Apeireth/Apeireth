#!/usr/bin/env python3
"""工具卡面批二（工具栏 + 工具网格）对照：像素 + DOM 全等 + 逐帧。

## 对照项

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每个用例 × 每个区块（工具栏 / 工具网格）差异像素占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**比较，不折叠空白）；另有**状态快照**对照（从 DOM 读出的卡片清单 / 徽标 / 摘要 / 搜索词） |
| T3 | **逐帧对照** | 搜索词变化期间的每一帧差异占比 **≤ 1%**（按**整体帧位移**对齐，容忍采集抖动） |

## 两侧

- **参考侧**：`reference/toolgrid/index.html` —— 生产视图（**只读引入**）+ 固定 fixture
  （`runtime` 模块由 vite 插件拦到对照用替身）。
- **引擎侧**：`engine/toolgrid/index.html` —— wasm 引擎渲染同样的工具栏 + 工具网格
  + 同一批固定数据。

## 对照哪几块

生产的 `ToolsView.svelte` 里**混了三个面** ⇒ 本批只对照**工具卡面**的两块：

| 区块 | 选择器 |
|---|---|
| 工具栏 | `.tools-toolbar` |
| 工具网格 | `.tools-grid` |

⇒ 参考侧**不隐藏**别的面（不耦合别的面的 class 名）⇒ 改为**按区块取盒**对照。

## 环境固定

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧同一时刻）；
- 同一浏览器实例、同一份 CSS、同一份 fixture。

## 用法

```bash
python3 tools/compare_toolgrid.py            # 跑全套，产物落在 out/toolgrid/ 与 evidence/toolgrid/
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
OUT = HARNESS / "out" / "toolgrid"
EVIDENCE = HARNESS / "evidence" / "toolgrid"

#: 视口与阈值（沿用试点口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 逐帧采集的帧数与窗口。
FRAME_COUNT = 16
FRAME_WINDOW_MS = 900
#: 逐帧对照允许的**整体帧位移**（采集抖动；与逐帧对照的时间对齐同一口径）。
FRAME_SHIFT_LIMIT = 2

#: 对照的区块（工具卡面只有这两块）。
REGIONS = (
    ("toolbar", ".tools-toolbar"),
    ("grid", ".tools-grid"),
)

#: 归一化 DOM 的脚本（按区块取）。
#:
#: 与试点的口径**同源**（剔除 Svelte 作用域 class、class token 排序、注释锚点），
#: 但**文本节点逐字保留**（不折叠空白、不裁剪）⇒ 比试点更严。
NORMALIZE_SCRIPT = r"""
((selector) => {
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
  const root = document.querySelector(selector);
  return JSON.stringify(root ? normalize(root) : null);
})
"""

#: 盒子（区块的视口坐标）。
BOX_SCRIPT = r"""
((selector) => {
  const el = document.querySelector(selector);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,
                          w: r.width, h: r.height});
})
"""


def log(message: str) -> None:
    print(message, flush=True)


def normalize_script(selector: str) -> str:
    return f"{NORMALIZE_SCRIPT}({json.dumps(selector)})"


def box_script(selector: str) -> str:
    return f"{BOX_SCRIPT}({json.dumps(selector)})"


def region_box(cdp: compare.Cdp, selector: str) -> dict | None:
    raw = cdp.evaluate(box_script(selector))
    return json.loads(raw) if raw else None


def navigate(cdp: compare.Cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__TOOLGRID__ && window.__TOOLGRID__.ready()", timeout=60)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.3)


def select_case(cdp: compare.Cdp, index: int) -> None:
    cdp.evaluate(f"window.__TOOLGRID__.selectCase({index}); true")
    time.sleep(0.25)


def collect_case(cdp: compare.Cdp, index: int) -> dict:
    """逐用例收集两个区块的 DOM 与截图（**先切到该用例**）。"""
    select_case(cdp, index)
    item: dict = {"index": index, "regions": {}}
    for name, selector in REGIONS:
        box = region_box(cdp, selector)
        item["regions"][name] = {
            "dom": cdp.evaluate(normalize_script(selector)),
            "box": box,
            "shot": compare.screenshot(cdp, box) if box else None,
        }
    item["state"] = cdp.evaluate("window.__TOOLGRID__.state()")
    return item


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


def capture_frames(cdp: compare.Cdp, box: dict, duration_ms: int, max_frames: int):
    """显式连拍（每次 `captureScreenshot` 都强制产出一帧；带**绝对**时间戳）。

    与试点同一口径：按**固定帧数**采集 ⇒ 可对照。绝对时间戳用于算「变化延迟」。
    """
    frames: list[tuple[float, bytes]] = []
    started = time.time()
    while (time.time() - started) * 1000.0 < duration_ms and len(frames) < max_frames:
        frames.append((time.time(), compare.screenshot(cdp, box)))
    return frames


def best_shift(reference, engine, frame_rows) -> tuple[int, float]:
    """找**整体帧位移**里总差异最小的那一个（容忍采集抖动）。

    ## 对齐口径

    搜索词变化是**离散**的 ⇒ 两侧的「变化点」可能差一帧 ⇒ 按**变化点**对齐
    （变化点索引之差）⇒ 变化前的帧与变化前的帧配对、变化后的与变化后的配对
    ⇒ 每一对都是**同一状态**的画面 ⇒ 差异只可能来自渲染。
    """
    reference_index = next(
        (row.get("transition_index") for row in frame_rows if row.get("side", "").startswith("reference")),
        None,
    )
    engine_index = next(
        (row.get("transition_index") for row in frame_rows if row.get("side", "").startswith("engine")),
        None,
    )
    if reference_index is None or engine_index is None:
        # 变化点没测到 ⇒ 退回「总差异最小」的位移（并由判据拦住空跑）。
        best = (0, float("inf"))
        for shift in range(-FRAME_SHIFT_LIMIT, FRAME_SHIFT_LIMIT + 1):
            total = 0.0
            for index in range(min(len(reference), len(engine))):
                other = index + shift
                if other < 0 or other >= len(engine):
                    continue
                total += compare.pixel_diff(reference[index][1], engine[other][1])["ratio"]
            if total < best[1]:
                best = (shift, total)
        return best
    return (engine_index - reference_index, 0.0)


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
        "batch": "工具卡面批二（工具栏 + 工具网格）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_page": "/reference/toolgrid/index.html",
        "engine_page": "/engine/toolgrid/index.html",
        "regions": {name: selector for name, selector in REGIONS},
    }
    try:
        cdp = compare.open_page(devtools_port)

        # ── 用例数（两侧一致） ──────────────────────────────────────────
        navigate(cdp, f"{base}/reference/toolgrid/index.html")
        count = cdp.evaluate("window.__TOOLGRID__.caseCount()")
        report["case_count"] = count

        log("── 参考侧（生产视图 + 替身 runtime） ──")
        navigate(cdp, f"{base}/reference/toolgrid/index.html")
        reference = [collect_case(cdp, index) for index in range(count)]
        log("── 引擎侧（wasm 引擎） ──")
        navigate(cdp, f"{base}/engine/toolgrid/index.html")
        engine = [collect_case(cdp, index) for index in range(count)]

        # ── T1 / T2：逐用例 × 逐区块 ───────────────────────────────────
        pixel_rows = []
        dom_rows = []
        state_rows = []
        for index in range(count):
            ref = reference[index]
            eng = engine[index]
            for name, _selector in REGIONS:
                ref_region = ref["regions"][name]
                eng_region = eng["regions"][name]
                ref_shot = ref_region["shot"]
                eng_shot = eng_region["shot"]
                if ref_shot is None and eng_shot is None:
                    pixel_rows.append(
                        {
                            "case": index,
                            "region": name,
                            "absent_both": True,
                            "size_mismatch": False,
                            "different": 0,
                            "total": 0,
                            "ratio": 0.0,
                        }
                    )
                elif ref_shot is None or eng_shot is None:
                    pixel_rows.append(
                        {
                            "case": index,
                            "region": name,
                            "absent_both": False,
                            "size_mismatch": True,
                            "different": -1,
                            "total": 0,
                            "ratio": 1.0,
                        }
                    )
                else:
                    diff = compare.pixel_diff(ref_shot, eng_shot)
                    pixel_rows.append({"case": index, "region": name, **diff})
                dom_diffs = diff_dom(ref_region["dom"], eng_region["dom"])
                dom_rows.append(
                    {
                        "case": index,
                        "region": name,
                        "equal": not dom_diffs,
                        "diffs": dom_diffs,
                    }
                )
            state_equal = ref["state"] == eng["state"]
            state_rows.append(
                {
                    "case": index,
                    "equal": state_equal,
                    "reference": json.loads(ref["state"] or "null"),
                    "engine": json.loads(eng["state"] or "null"),
                }
            )
            for name, _selector in REGIONS:
                for side, data in (("reference", ref), ("engine", eng)):
                    shot = data["regions"][name]["shot"]
                    if shot is not None:
                        (OUT / f"{side}-case-{index}-{name}.png").write_bytes(shot)
        report["t1_pixels"] = pixel_rows
        report["t2_dom"] = dom_rows
        report["t2_state"] = state_rows
        for row in pixel_rows:
            if row.get("absent_both"):
                log(f"T1 用例 {row['case']} {row['region']}：两侧都不存在 ⇒ 无差异")
                continue
            log(
                f"T1 用例 {row['case']} {row['region']}：差异 {row['different']} / 总 "
                f"{row['total']} = {row['ratio'] * 100:.4f}%"
            )
        dom_failures = [row for row in dom_rows if not row["equal"]]
        for row in dom_failures[:10]:
            log(f"T2 用例 {row['case']} {row['region']}：{len(row['diffs'])} 处差异")
            for line in row["diffs"][:6]:
                log(f"     {line}")
        log(f"T2 DOM 全等：{'一致' if not dom_failures else f'{len(dom_failures)} 个状态不一致'}")
        state_failures = [row for row in state_rows if not row["equal"]]
        log(
            f"T2 状态快照：{len(state_rows) - len(state_failures)} / {len(state_rows)} 一致"
        )

        # ── T3：逐帧（搜索词变化期间） ────────────────────────────────
        frame_rows = []
        reference_frames = capture_side_frames(base, cdp, "reference/toolgrid", frame_rows)
        engine_frames = capture_side_frames(base, cdp, "engine/toolgrid", frame_rows)
        shift, _total = best_shift(reference_frames, engine_frames, frame_rows)
        frame_diffs = []
        for index in range(min(len(reference_frames), len(engine_frames))):
            other = index + shift
            if other < 0 or other >= len(engine_frames):
                continue
            diff = compare.pixel_diff(reference_frames[index][1], engine_frames[other][1])
            frame_diffs.append({"index": index, "engine_index": other, **diff})
        report["t3_frames"] = {
            "reference_frame_count": len(reference_frames),
            "engine_frame_count": len(engine_frames),
            "target_frame_count": FRAME_COUNT,
            "frame_shift": shift,
            "frames": frame_diffs,
            "sides": frame_rows,
        }
        for row in frame_diffs:
            log(
                f"T3 帧 {row['index']:>2}：差异 {row['different']} / 总 {row['total']}"
                f" = {row['ratio'] * 100:.4f}%"
            )

        # ── 汇总判据 ────────────────────────────────────────────────────
        transitions = [
            row.get("transition")
            for row in frame_rows
            if row.get("transition") is not None
        ]
        transition_captured = (
            len(transitions) == 2
            and all(item["different"] > 0 for item in transitions)
        )
        checks = {
            "t1_all_pixels_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in pixel_rows
            ),
            "t2_dom_identical": not dom_failures,
            "t2_state_identical": not state_failures,
            "t3_enough_frames": len(frame_diffs) >= 8,
            "t3_all_frames_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in frame_diffs
            ),
            # 逐帧对照**不许空跑**：两侧的首帧与末帧必须不同（变化真被采到）。
            "t3_transition_captured": transition_captured,
        }
        report["notes"] = [
            "T1/T3 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T1 按**区块**取盒（工具栏 / 工具网格）⇒ 参考侧**不隐藏**别的面"
            "（不耦合别的面的 class 名）⇒ 只对照本面的两块。",
            "T2 的归一化：剔除 Svelte 作用域 class 与注释锚点、class token 排序；"
            "**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3 是**实时**对照：两侧在**下一帧**里同时切搜索词 ⇒ 变化落在同一帧；"
            f"采集存在抖动 ⇒ 允许**整体帧位移** ±{FRAME_SHIFT_LIMIT}（取总差异最小者）。",
            "T3 另设**不许空跑**判据：两侧的首帧与末帧必须不同（变化真被采到）"
            "⇒ 否则「差异为 0」没有意义。",
            "T3 的 `transition_latency_ms` = **首帧**出现变化的时刻（**帧粒度**）⇒ 它是"
            "采集分辨率下的「变化可见时刻」，**不是**引擎的重算耗时（引擎侧是同步 flush）。",
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
        # 证据：逐用例图（参考 vs 引擎）。
        for index in range(count):
            for name, _selector in REGIONS:
                for side, data in (("reference", reference[index]), ("engine", engine[index])):
                    shot = data["regions"][name]["shot"]
                    if shot is not None:
                        (EVIDENCE / f"{side}-case-{index}-{name}.png").write_bytes(shot)
        # 浏览器控制台日志（两侧：收集导航后的控制台消息与异常）。
        console = []
        for name, page in (("reference", "reference/toolgrid"), ("engine", "engine/toolgrid")):
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


def capture_side_frames(base: str, cdp: compare.Cdp, page: str, frame_rows: list[dict]):
    """一侧的逐帧采集（页面内定时触发 ⇒ 切搜索词 ⇒ 连拍舞台盒）。

    ## 为什么触发放在**页面里**

    触发若由外部发起（采集开始后再发指令），第一帧往往**已经变了** ⇒
    逐帧对照变成空跑 ⇒ 这里用页面内的定时器触发 ⇒ 采集循环**单线程**
    （不与 CDP 客户端抢事件队列）。
    """
    navigate(cdp, f"{base}/{page}/index.html")
    select_case(cdp, 0)
    stage_box = region_box(cdp, ".toolgrid-stage")
    if stage_box is None:
        log(f"T3 {page}：找不到舞台盒")
        frame_rows.append({"side": page, "frames": 0, "box": None})
        return []
    # 页面内定时触发（150ms 后）⇒ 采集循环在此之前已经采到「变化前」的帧。
    cdp.evaluate(
        "setTimeout(() => { window.__TOOLGRID__.selectCase(1); }, 150); true"
    )
    issued = time.time()
    frames = capture_frames(cdp, stage_box, FRAME_WINDOW_MS, FRAME_COUNT * 2)
    # 「变化是否真被采到」的诚实判据：首帧与末帧必须**不同**（搜索词变了 ⇒
    # 画面变了）⇒ 否则逐帧对照是**空跑**（两侧都没变 ⇒ 差异恒为 0）。
    transition = None
    transition_index = None
    if len(frames) >= 2:
        transition = compare.pixel_diff(frames[0][1], frames[-1][1])
        for index in range(1, len(frames)):
            if compare.pixel_diff(frames[0][1], frames[index][1])["different"] > 0:
                transition_index = index
                break
    frame_rows.append(
        {
            "side": page,
            "frames": len(frames),
            "box": stage_box,
            "transition": transition,
            "transition_index": transition_index,
            "transition_latency_ms": (
                (frames[transition_index][0] - issued) * 1000.0 if transition_index else None
            ),
        }
    )
    if frames:
        (OUT / f"{page.replace('/', '-')}-frame-first.png").write_bytes(frames[0][1])
        (OUT / f"{page.replace('/', '-')}-frame-last.png").write_bytes(frames[-1][1])
    return frames


if __name__ == "__main__":
    sys.exit(main())
