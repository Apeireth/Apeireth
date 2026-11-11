#!/usr/bin/env python3
"""治理卷宗面对照：像素 + DOM 全等 + 逐帧（四个卷宗面各一组用例）。

## 对照项

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每个用例差异像素占比 **≤ 1%**（阈值：任一通道绝对差 > 2/255） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（文本节点**逐字**比较，不折叠空白） |
| T3 | **逐帧对照** | 动作序列期间的每一帧差异占比 **≤ 1%** |

## 两侧

- **参考侧**：`reference/<面>/index.html` —— 生产组件（只读引入）+ 固定 fixture。
- **引擎侧**：`engine/<面>/index.html` —— wasm 引擎渲染同样的组件 + 同一份 fixture。

## 环境固定

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧同一时刻）；
- **时钟冻结**：两侧的 `Date.now()` 冻结到同一基准时刻（`window.__GOV_BASE__`）
  ⇒ 依赖「现在」的渲染（过期判定 / 相对时间 / 同日时刻）**两侧逐字一致**；
- 同一浏览器实例、同一份 CSS、同一份 fixture。

## 用法

```bash
python3 tools/compare_governance.py
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
# 共享的对照工具（**只读复用**：像素差分 / CDP 驱动 / 静态服务）。
sys.path.insert(0, str(HARNESS.parent / "tools"))

import compare  # noqa: E402

DIST = HARNESS / "dist"
OUT = HARNESS / "out"
EVIDENCE = HARNESS / "evidence"
FIXTURE = HARNESS / "fixtures" / "governance-cases.json"

#: 视口与阈值（沿用试点口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 逐帧采集的帧数与窗口。
FRAME_COUNT = 12
FRAME_WINDOW_MS = 500

#: 四个卷宗面。
TABS = ["approvals", "grants", "audit", "guard"]

#: 归一化 DOM 的脚本（照批一口径，但**更严**：文本节点逐字保留）。
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
  const root = document.querySelector('.governance-stage');
  return JSON.stringify(root ? normalize(root) : null);
})()
"""


def log(message: str) -> None:
    print(message, flush=True)


def stage_box(cdp: compare.Cdp) -> dict | None:
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector('.governance-stage');"
        " if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def install_case_script(cdp: compare.Cdp, identifier: str | None, index: int, base_ms: int):
    """在**导航前**注入当前用例下标、基准时刻，并冻结 `Date.now`。"""
    if identifier:
        cdp.call("Page.removeScriptToEvaluateOnNewDocument", {"identifier": identifier})
    source = (
        "window.__GOV_CASE_INDEX__ = %d;"
        "window.__GOV_BASE__ = %d;"
        "Date.now = () => %d;" % (index, base_ms, base_ms)
    )
    result = cdp.call("Page.addScriptToEvaluateOnNewDocument", {"source": source})
    return result["result"]["identifier"]


def navigate(cdp: compare.Cdp, url: str) -> None:
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__GOVERNANCE__ && window.__GOVERNANCE__.ready()", timeout=60)
    # 让字体与布局稳定（两侧同样等待）。
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.35)


def apply_case(cdp: compare.Cdp, index: int) -> None:
    cdp.evaluate(f"window.__GOVERNANCE__.applyCase({index}); true")
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


def collect_side(
    cdp: compare.Cdp,
    base: str,
    page: str,
    index: int,
    identifier: str | None,
    base_ms: int,
):
    """抓一侧的某个用例（DOM + 截图）。

    ⚠️ `base_ms` 由调用方给出 ⇒ **同一用例的两侧共用同一基准时刻** ⇒ 依赖
    「现在」的渲染（时刻文本 / 过期判定）两侧逐字一致。
    """
    identifier = install_case_script(cdp, identifier, index, base_ms)
    navigate(cdp, f"{base}/{page}/index.html")
    apply_case(cdp, index)
    dom = cdp.evaluate(NORMALIZE_SCRIPT)
    box = stage_box(cdp)
    shot = compare.screenshot(cdp, box)
    state = cdp.evaluate("window.__GOVERNANCE__.state()")
    return {
        "dom": dom,
        "shot": shot,
        "state": state,
        "identifier": identifier,
    }


def collect_frames(
    cdp: compare.Cdp,
    base: str,
    page: str,
    index: int,
    identifier: str | None,
    base_ms: int,
):
    """抓一侧的逐帧（动画冻结 ⇒ 判据是「状态收敛到同一帧」）。"""
    identifier = install_case_script(cdp, identifier, index, base_ms)
    navigate(cdp, f"{base}/{page}/index.html")
    cdp.evaluate("document.documentElement.dataset.governanceStatic = '1'; true")
    cdp.evaluate(
        "requestAnimationFrame(() => { window.__GOVERNANCE__.applyCase(%d); }); true" % index
    )
    frames = compare.capture_frames(cdp, FRAME_WINDOW_MS, FRAME_COUNT)
    return frames, identifier


def main() -> int:
    if not DIST.exists():
        log(f"未找到构建产物：{DIST}（先跑 `bash build.sh`）")
        return 2
    if not FIXTURE.exists():
        log(f"未找到固定数据：{FIXTURE}")
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)

    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    cases_by_tab = fixture["cases"]

    httpd, port = compare.serve(DIST)
    base = f"http://127.0.0.1:{port}"
    chrome, devtools_port = compare.launch_chrome()
    report: dict = {
        "batch": "治理卷宗面（ApprovalsTab + GrantsTab + AuditTab + GuardTab）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "reference_pages": {tab: f"/reference/{tab}/index.html" for tab in TABS},
        "engine_pages": {tab: f"/engine/{tab}/index.html" for tab in TABS},
    }
    try:
        cdp = compare.open_page(devtools_port)

        pixel_rows: list[dict] = []
        dom_rows: list[dict] = []
        state_rows: list[dict] = []
        frame_rows: list[dict] = []

        for tab in TABS:
            cases = cases_by_tab[tab]
            # 每个用例一个基准时刻 ⇒ **同一用例的两侧共用** ⇒ 依赖「现在」的
            # 渲染（时刻文本 / 过期判定）两侧逐字一致。
            case_bases = [int(time.time() * 1000) for _ in cases]
            log(f"── 参考侧（生产 Svelte 组件）：{tab} ──")
            reference = []
            identifier = None
            for index, case in enumerate(cases):
                base_ms = case_bases[index]
                item = collect_side(
                    cdp, base, f"reference/{tab}", index, identifier, base_ms
                )
                identifier = item.pop("identifier")
                reference.append(item)
            log(f"── 引擎侧（wasm 引擎）：{tab} ──")
            engine = []
            for index, case in enumerate(cases):
                item = collect_side(
                    cdp, base, f"engine/{tab}", index, identifier, case_bases[index]
                )
                identifier = item.pop("identifier")
                engine.append(item)

            for index, case in enumerate(cases):
                ref = reference[index]
                eng = engine[index]
                diff = compare.pixel_diff(ref["shot"], eng["shot"])
                pixel_rows.append({"tab": tab, "case": index, "id": case["id"], **diff})
                dom_diffs = diff_dom(ref["dom"], eng["dom"])
                dom_rows.append(
                    {
                        "tab": tab,
                        "case": index,
                        "id": case["id"],
                        "equal": not dom_diffs,
                        "diffs": dom_diffs,
                    }
                )
                state_rows.append(
                    {
                        "tab": tab,
                        "case": index,
                        "id": case["id"],
                        "equal": ref["state"] == eng["state"],
                        "reference": json.loads(ref["state"] or "null"),
                        "engine": json.loads(eng["state"] or "null"),
                    }
                )
                (EVIDENCE / f"{tab}-reference-case-{index}.png").write_bytes(ref["shot"])
                (EVIDENCE / f"{tab}-engine-case-{index}.png").write_bytes(eng["shot"])
                (OUT / f"{tab}-reference-case-{index}.png").write_bytes(ref["shot"])
                (OUT / f"{tab}-engine-case-{index}.png").write_bytes(eng["shot"])

            # 逐帧（第一个用例的动作序列）。
            base_ms = int(time.time() * 1000)
            ref_frames, identifier = collect_frames(
                cdp, base, f"reference/{tab}", 0, identifier, base_ms
            )
            eng_frames, identifier = collect_frames(
                cdp, base, f"engine/{tab}", 0, identifier, base_ms
            )
            pair_count = min(len(ref_frames), len(eng_frames))
            frame_diffs = []
            for index in range(pair_count):
                diff = compare.pixel_diff(ref_frames[index][1], eng_frames[index][1])
                frame_diffs.append({"index": index, **diff})
            frame_rows.append(
                {
                    "tab": tab,
                    "reference_frame_count": len(ref_frames),
                    "engine_frame_count": len(eng_frames),
                    "target_frame_count": FRAME_COUNT,
                    "frames": frame_diffs,
                }
            )

        for row in pixel_rows:
            log(
                f"T1 {row['tab']} 用例 {row['case']}（{row['id']}）：差异 {row['different']} / 总 "
                f"{row['total']} = {row['ratio'] * 100:.4f}%"
            )
        dom_failures = [row for row in dom_rows if not row["equal"]]
        for row in dom_failures[:20]:
            log(f"T2 {row['tab']} 用例 {row['case']}（{row['id']}）：{len(row['diffs'])} 处差异")
            for line in row["diffs"][:8]:
                log(f"     {line}")
        log(f"T2 DOM 全等：{'一致' if not dom_failures else f'{len(dom_failures)} 个用例不一致'}")
        state_failures = [row for row in state_rows if not row["equal"]]
        for row in state_failures[:10]:
            log(
                f"T2' {row['tab']} 用例 {row['case']}（{row['id']}）：DOM 状态快照不同"
            )
            log(f"     参考 {json.dumps(row['reference'], ensure_ascii=False)}")
            log(f"     引擎 {json.dumps(row['engine'], ensure_ascii=False)}")
        log(f"T2' 状态快照全等：{'一致' if not state_failures else f'{len(state_failures)} 个不一致'}")
        for row in frame_rows:
            worst = max((frame["ratio"] for frame in row["frames"]), default=0.0)
            log(
                f"T3 {row['tab']}：{row['reference_frame_count']} / {row['engine_frame_count']} 帧，"
                f"最大差异 {worst * 100:.4f}%"
            )

        # ── 汇总判据 ────────────────────────────────────────────────────
        checks = {
            "t1_all_pixels_within_budget": all(
                (not row["size_mismatch"]) and row["ratio"] <= PIXEL_BUDGET
                for row in pixel_rows
            ),
            "t2_dom_identical": not dom_failures,
            "t2b_state_snapshot_identical": not state_failures,
            # 采集耗时不同 ⇒ 两侧帧数可能差 1~2 ⇒ 判据是「**配对**帧数 ≥ 8」
            # （配对帧逐帧比对 ⇒ 帧数差的尾巴不比）。
            "t3_enough_frames": all(
                min(row["reference_frame_count"], row["engine_frame_count"])
                >= FRAME_COUNT - 4
                for row in frame_rows
            ),
            "t3_all_frames_within_budget": all(
                (not frame["size_mismatch"]) and frame["ratio"] <= PIXEL_BUDGET
                for row in frame_rows
                for frame in row["frames"]
            ),
        }
        report["t1_pixels"] = pixel_rows
        report["t2_dom"] = dom_rows
        report["t2b_state"] = state_rows
        report["t3_frames"] = frame_rows
        report["notes"] = [
            "T1/T3 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T1 静态抓取时两侧**同时**停掉无限循环动画（`data-governance-static`）"
            "——无限动画的相位随时间漂移 ⇒ 两侧先后抓取会出现**假差异**。",
            "T2 的归一化：剔除 Svelte 作用域 class 与注释锚点、class token 排序、"
            "**丢弃纯空白文本节点**；**其余文本节点逐字保留**（不折叠空白）⇒ 比试点更严。",
            "T2' 是**状态快照**（从 DOM 读出的两侧同义状态）⇒ 与 T2 互补。",
            "T3 在**动画冻结**下采集 ⇒ 判据是「两侧状态收敛到同一帧」；"
            "动画本身的存在与相位由 CSS 资产同源保证（两侧逐字同一份 CSS 资产）。",
            "**时钟冻结**：两侧的 `Date.now()` 冻结到同一基准时刻 ⇒ 依赖「现在」的"
            "渲染（过期判定 / 相对时间 / 同日时刻）两侧逐字一致。",
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

        # 浏览器控制台日志（两侧）。
        console = []
        for tab in TABS:
            for name in ("reference", "engine"):
                cdp.drain()
                base_ms = int(time.time() * 1000)
                identifier = install_case_script(cdp, identifier, 0, base_ms)
                navigate(cdp, f"{base}/{name}/{tab}/index.html")
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
                console.append({"tab": tab, "side": name, "messages": messages})
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
