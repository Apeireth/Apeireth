#!/usr/bin/env python3
"""`budget` 子面的**面级对照驱动**（像素 + DOM 全等 + 宽度同源）。

## 与共享对照脚本的关系

设置页 harness 的共享脚本（`tools/compare_settings.py`）按约定**发现并转发**到
各面的驱动：

| 约定 | 本文件 |
|---|---|
| 驱动位置 | `web/engine/budget/compare_budget.py`（**本面独占目录**） |
| 入口 | `collect_pair(panel, section, port) -> dict`（返回值含 `passed`）+ `main()` |

⇒ `python3 tools/compare_settings.py --panel budget` ⇒ 逐分区跑本驱动 ✓。

## ⭐ 本面为什么**按区块**对照（而不是整块 `.setting-block`）

参考侧是**生产的 `SettingsView`** ⇒ 它的 `budget` 分区里**夹着**一块
`MetricsPanel`（`.metrics-card`）：

- `MetricsPanel` 是**外部组件**（205 行）⇒ **内部拉 `GET /v1/panel/metrics`**
  ⇒ **HTTP 契约** ⇒ 本面**不迁**（`SPLIT.md` §5.3）；
- 它的 `<style>` 是**自己的 scoped CSS** ⇒ **未抽取** ⇒ 引擎侧即使手写这段
  markup 也**无法逐字复现**（样式不同）。

⇒ 因此**整块 `.setting-block` 两侧结构必然不同**（参考侧多一块）⇒
本驱动**只对照本面自己拥有的区块**：

| 区域 | 参考侧元素 | 引擎侧元素 |
|---|---|---|
| `session` | `h3.block-title` + `p.block-desc` + 含 `.budget-remaining` 的 `.cap-card` | 同名（`.sp-budget .setting-block` 内） |
| `knobs` | 含 `.cap-slider-wrap` 的 `.cap-card` | 同 |
| `quota` | 含 `.quota-none` 的 `.cap-card` | 同 |
| `exhaust` | `.cap-default-card` | 同 |
| `runtime` | `h3.block-title` + `p.block-desc` + `button.quiet-button` | 同 |

⇒ 两侧都用**同一套选择器逻辑**（按 class 线索挑区块）⇒ **一一对应**。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每个区域的差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（结构 / 属性 / 文本必须为零；**文本节点逐字**） |
| T3 | **宽度同源断言** | 每个区域的裁剪框**宽度相等**（不等 ⇒ **直接判失败**） |
| T4 | **余量条取整口径** | 两侧 `.remaining-fill` 的 `style` 属性**逐字相同**（`width: N%;`） |

## ⭐ 装置对齐（**像素可比的前提**）

参考侧是**生产整页** ⇒ 它的内容块在文档里的位置**带小数**（本面实测
`top = 219.765625`）；引擎侧是**独立挂载** ⇒ 位置是 `0` ⇒ 两边的**设备像素格**
错开 ⇒ 渐变 / 圆角抗锯齿 / 字形栅格化**必然不同** ⇒ 那是**装置伪差**。

⇒ 本驱动**逐区域**把引擎侧的舞台（`.settings-stage`）对齐到参考侧的文档位置
⇒ 差异只反映**渲染本身**。
⚠️ **为什么逐区域**（而不是整块一次）：参考侧夹着 `MetricsPanel` ⇒ 它**之后**的
区块在两侧的纵向偏移**不同** ⇒ 整块一次对齐**只对齐第一块**。

## 用法

```bash
# 面级（共享入口）：
python3 tools/compare_settings.py --panel budget
# 本面独立跑（写全量证据）：
python3 web/engine/budget/compare_budget.py
```

依赖：系统 Chromium、`websocket-client`、Pillow。**不需要** Playwright。
"""

from __future__ import annotations

import json
import os
import pathlib
import signal
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
OUT = SETTINGS / "out" / "budget"
#: 入库证据目录。
EVIDENCE = SETTINGS / "evidence" / "budget"

#: 两侧的根选择器（参考侧 = 生产整页；引擎侧 = 独立挂载的子面）。
REFERENCE_SELECTOR = ".setting-block"
ENGINE_SELECTOR = ".sp-budget .setting-block"

#: **未迁**的外部组件（`MetricsPanel`）⇒ 两侧都**排除**它（见模块文档）。
EXTERNAL_CLASS = "metrics-card"

#: 区域名（**两侧一一对应**）。
REGIONS = ("session", "knobs", "quota", "exhaust", "runtime")

#: 归一化 DOM 的脚本（**批一口径** + 「剔除 `sp-*` 根 class」这一条）。
NORMALIZE_SCRIPT = r"""
(() => {
  const strip = (value) => (value || '')
    .replace(/\bsvelte-[0-9a-z]+\b/g, '')
    .replace(/\bsp-[a-z0-9-]+\b/g, '')
    .trim();
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
    const children = Array.from(node.childNodes).map(normalize).filter(Boolean);
    return { t: 'el', tag: node.tagName.toLowerCase(), attrs, children };
  };
  const root = document.querySelector(%s);
  if (!root) return null;
  // ⚠️ `MetricsPanel`（外部组件 + HTTP）**未迁** ⇒ 两侧都排除它。
  const children = Array.from(root.children).filter(
    (el) => !el.classList.contains('%s')
  );
  const pick = (pred) => children.filter(pred);
  const head = pick((el) => el.tagName === 'H3' || el.tagName === 'P');
  const parts = {
    head: head.map(normalize),
    session: pick((el) => el.querySelector('.budget-remaining')).map(normalize),
    knobs: pick((el) => el.querySelector('.cap-slider-wrap')).map(normalize),
    quota: pick((el) => el.querySelector('.quota-none')).map(normalize),
    exhaust: pick((el) => el.classList.contains('cap-default-card')).map(normalize),
    button: pick((el) => el.tagName === 'BUTTON').map(normalize),
  };
  return JSON.stringify(parts);
})()
"""

#: 量各区域**裁剪框**的脚本（**相对文档坐标**）。
REGION_BOX_SCRIPT = r"""
(() => {
  const root = document.querySelector(%s);
  if (!root) return null;
  const children = Array.from(root.children).filter(
    (el) => !el.classList.contains('%s')
  );
  const pick = (pred) => children.filter(pred);
  const head = pick((el) => el.tagName === 'H3' || el.tagName === 'P');
  const session = pick((el) => el.querySelector('.budget-remaining'));
  const knobs = pick((el) => el.querySelector('.cap-slider-wrap'));
  const quota = pick((el) => el.querySelector('.quota-none'));
  const exhaust = pick((el) => el.classList.contains('cap-default-card'));
  const button = pick((el) => el.tagName === 'BUTTON');
  const box = (els) => {
    let left = Infinity, top = Infinity, right = -Infinity, bottom = -Infinity;
    for (const el of els) {
      const r = el.getBoundingClientRect();
      left = Math.min(left, r.left);
      top = Math.min(top, r.top);
      right = Math.max(right, r.right);
      bottom = Math.max(bottom, r.bottom);
    }
    if (!isFinite(left) || !isFinite(top)) return null;
    return {
      x: left + window.scrollX,
      y: top + window.scrollY,
      w: right - left,
      h: bottom - top,
    };
  };
  const regions = {};
  if (session.length) regions.session = box(head.concat(session));
  if (knobs.length) regions.knobs = box(knobs);
  if (quota.length) regions.quota = box(quota);
  if (exhaust.length) regions.exhaust = box(exhaust);
  if (button.length) regions.runtime = box(head.concat(button));
  return JSON.stringify(regions);
})()
"""

#: 取余量条**内联宽度**的脚本（**逐字** ⇒ 验证 `Math.round(ratio * 100)` 口径）。
FILL_STYLE_SCRIPT = r"""
(() => {
  const root = document.querySelector(%s);
  if (!root) return null;
  const out = [];
  for (const el of root.querySelectorAll('.remaining-fill')) {
    out.push(el.getAttribute('style'));
  }
  return JSON.stringify(out);
})()
"""

#: 模块级环境（**跨多次 `collect_pair` 复用** ⇒ 浏览器只起一次）。
_ENV: dict = {}

#: 环境缓存的键（**挂在 `sys.modules` 上** ⇒ 进程级 ⇒ 跨模块实例共享）。
_ENV_KEY = "apeireth_settings_budget_env"


def log(message: str) -> None:
    print(message, flush=True)


def normalize_script(selector: str) -> str:
    """按选择器取一份归一化脚本。"""
    return NORMALIZE_SCRIPT % (json.dumps(selector), EXTERNAL_CLASS)


def region_box_script(selector: str) -> str:
    """按选择器取一份量框脚本。"""
    return REGION_BOX_SCRIPT % (json.dumps(selector), EXTERNAL_CLASS)


def fill_style_script(selector: str) -> str:
    """按选择器取一份余量条取值的脚本。"""
    return FILL_STYLE_SCRIPT % json.dumps(selector)


def ensure_env(port: int) -> dict:
    """起服务 + 起浏览器（**只做一次**）。"""
    if _ENV:
        return _ENV
    cached = sys.modules.get(_ENV_KEY)
    if isinstance(cached, dict) and cached:
        _ENV.update(cached)
        return _ENV
    dist = SETTINGS / "dist"
    if not dist.exists():
        raise SystemExit(f"未找到构建产物：{dist}（先跑 `bash build.sh`）")
    httpd, actual = compare.serve(dist)
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


def navigate(cdp: compare.Cdp, url: str) -> None:
    """导航到一侧并等就绪（两侧用**同一套** `window.__SETTINGS__` 契约）。"""
    cdp.call("Page.navigate", {"url": url})
    cdp.wait_for("!!window.__SETTINGS__ && window.__SETTINGS__.ready()", timeout=60)
    cdp.evaluate("document.fonts ? document.fonts.ready.then(() => true) : true", True)
    time.sleep(0.5)


def select_section(cdp: compare.Cdp, section: str) -> None:
    """切到目标分区（两侧同名同义）。"""
    cdp.evaluate(f"window.__SETTINGS__.selectSection({json.dumps(section)}); true")
    time.sleep(0.4)


def regions_of(cdp: compare.Cdp, selector: str) -> dict:
    """量各区域的裁剪框。"""
    raw = cdp.evaluate(region_box_script(selector))
    return json.loads(raw) if raw else {}


def set_stage_margin(cdp: compare.Cdp, margin_top: float, margin_left: float) -> bool:
    """把引擎侧的舞台**对齐**到指定的文档位置（**装置对齐**）。"""
    return bool(
        cdp.evaluate(
            "(() => { const stage = document.querySelector('.settings-stage');"
            " if (!stage) return false;"
            " stage.style.marginTop = %r + 'px';"
            " stage.style.marginLeft = %r + 'px';"
            " return true; })()" % (margin_top, margin_left)
        )
    )


def diff_dom(left: str | None, right: str | None) -> list[str]:
    """列出两侧 DOM 的差异点（**逐条**）。"""
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
            diffs.append(f"{path}: 子节点数不同（{len(left_children)} / {len(right_children)}）")
        for index in range(min(len(left_children), len(right_children))):
            diffs.extend(
                describe_dom_diff(left_children[index], right_children[index], f"{path}/{index}")
            )
        return diffs
    if isinstance(left, list):
        diffs = []
        if len(left) != len(right):
            diffs.append(f"{path}: 列表长度不同（{len(left)} / {len(right)}）")
        for index in range(min(len(left), len(right))):
            diffs.extend(describe_dom_diff(left[index], right[index], f"{path}/{index}"))
        return diffs
    return []


def run_knob_input_check(env: dict) -> dict:
    """⭐ **旋钮数字输入的对照**（**键入** ⇒ 断言取值与反馈**逐字相同**）。

    ## 为什么要有这个用例

    2026-10-25 实测到一个**架构级缺陷**：预算面的状态对象**自建了一个独立的
    `Runtime`** ⇒ ⇒ effect **不依赖**草稿信号 ⇒ ⇒ 改草稿**不重渲染**
    ⇒ ⇒ **「幽灵输入」**：用户敲进去的值**引擎不知道**
    （DOM 上显示的是浏览器自己画的值，引擎草稿仍是 `""`）。

    ⚠️ 而**本面此前的对照全是静态像素** ⇒ ⇒ **这类缺陷永远测不到**
    ⇒ ⇒ 这就是「**对照覆盖面要与引擎能力面匹配**」的缺口。

    ⇒ ⇒ 本用例用**真实键入**（两侧同路径）⇒ ⇒ 断言三样：
    ① 键入后 `input.value` 两侧**逐字相同**；
    ② 失焦（提交）后 `input.value` 两侧**逐字相同**（⇒ 引擎**接管**了取值）；
    ③ 反馈文案（`.budget-feedback`）两侧**逐字相同**（⇒ 引擎**算出了同样的归一化**）。

    ## 口径

    - 键入 `'7'`（原值 `8`）⇒ 越界（`max = 64`）⇒ 失焦后应被**钳制**；
    - 失焦用 `blur()` ⇒ 触发生产的「整组失焦提交」；
    - 反馈文案取 `.budget-feedback` 的文本（**生产与引擎同 class**）。
    """
    cdp = env["cdp"]
    base = env["base"]
    rows = []
    for side in ("reference", "engine"):
        navigate(cdp, f"{base}/web/{side}/budget/index.html")
        select_section(cdp, "budget")
        # 聚焦第一个数字步进器 ⇒ 键入一个字符 ⇒ 失焦（提交）。
        cdp.evaluate(
            "(() => { const el = document.querySelector('input[type=number]');"
            " if (el) el.focus(); return !!el; })()"
        )
        cdp.call(
            "Input.dispatchKeyEvent",
            {
                "type": "keyDown",
                "text": "7",
                "unmodifiedText": "7",
                "key": "7",
                "code": "Digit7",
                "windowsVirtualKeyCode": 55,
                "nativeVirtualKeyCode": 55,
            },
        )
        cdp.call(
            "Input.dispatchKeyEvent",
            {
                "type": "keyUp",
                "key": "7",
                "code": "Digit7",
                "windowsVirtualKeyCode": 55,
            },
        )
        time.sleep(0.4)
        typed = cdp.evaluate(
            "(() => { const el = document.querySelector('input[type=number]');"
            " return el ? el.value : '<无输入框>'; })()"
        )
        cdp.evaluate(
            "(() => { const el = document.querySelector('input[type=number]');"
            " if (el) el.blur(); return true; })()"
        )
        time.sleep(0.8)
        committed = cdp.evaluate(
            "(() => { const el = document.querySelector('input[type=number]');"
            " return el ? el.value : '<无输入框>'; })()"
        )
        feedback = cdp.evaluate(
            "(() => { const el = document.querySelector('.budget-feedback');"
            " return el ? (el.textContent || '').trim() : ''; })()"
        )
        log(
            f"  knob-input {side}：键入后={typed!r} / 失焦后={committed!r}"
            f" / 反馈={feedback!r}"
        )
        rows.append(
            {"side": side, "typed": typed, "committed": committed, "feedback": feedback}
        )

    reference, engine = rows[0], rows[1]
    record = {
        "case": "knob-input-typed",
        "driver": (
            "聚焦第一个数字步进器 ⇒ 键入 '7' ⇒ blur（提交）"
            "（两侧同路径）"
        ),
        "reference_typed_value": reference.get("typed"),
        "engine_typed_value": engine.get("typed"),
        "reference_committed_value": reference.get("committed"),
        "engine_committed_value": engine.get("committed"),
        "reference_feedback": reference.get("feedback"),
        "engine_feedback": engine.get("feedback"),
        "typed_value_equal": reference.get("typed") == engine.get("typed"),
        "committed_value_equal": reference.get("committed") == engine.get("committed"),
        "feedback_equal": reference.get("feedback") == engine.get("feedback"),
    }
    record["passed"] = bool(
        record["typed_value_equal"]
        and record["committed_value_equal"]
        and record["feedback_equal"]
    )
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "knob-input-typed.record.json").write_text(
        json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    log(
        f"  knob-input-typed：键入值一致={record['typed_value_equal']}"
        f" / 提交值一致={record['committed_value_equal']}"
        f" / 反馈一致={record['feedback_equal']}"
    )
    return record


def collect_pair(panel: dict, section: str, port: int) -> dict:
    """跑一对（面 × 分区）的对照（**共享脚本的约定入口**）。

    返回值里必须有 `passed`（布尔）与判据明细 ⇒ 共享脚本据此汇总。
    """
    section_id = section["id"] if isinstance(section, dict) else str(section)
    env = ensure_env(port)
    cdp = env["cdp"]
    base = env["base"]

    # ── 参考侧（生产组件，只读引入）──
    navigate(cdp, f"{base}/web/reference/budget/index.html")
    select_section(cdp, section_id)
    reference_dom = cdp.evaluate(normalize_script(REFERENCE_SELECTOR))
    reference_regions = regions_of(cdp, REFERENCE_SELECTOR)
    reference_fills = json.loads(
        cdp.evaluate(fill_style_script(REFERENCE_SELECTOR)) or "null"
    )

    # ── 引擎侧（wasm 引擎，独立挂载）──
    navigate(cdp, f"{base}/web/engine/budget/index.html")
    select_section(cdp, section_id)
    engine_dom = cdp.evaluate(normalize_script(ENGINE_SELECTOR))
    engine_fills = json.loads(cdp.evaluate(fill_style_script(ENGINE_SELECTOR)) or "null")

    dom_diffs = diff_dom(reference_dom, engine_dom)

    # ── 逐区域：像素 + 宽度同源（**逐区域对齐** ⇒ 参考侧的 `MetricsPanel`
    #    不会让后续区块的纵向偏移污染对照）──
    OUT.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    region_rows: dict = {}
    for name in REGIONS:
        reference_box = reference_regions.get(name)
        if reference_box is None:
            continue
        # 引擎侧：先把舞台归零，再量一次（⇒ 偏移量与「上一次对齐」无关）。
        set_stage_margin(cdp, 0.0, 0.0)
        engine_regions = regions_of(cdp, ENGINE_SELECTOR)
        engine_box = engine_regions.get(name)
        if engine_box is None:
            region_rows[name] = {"missing": "引擎侧没有这个区域"}
            continue
        delta_y = float(reference_box["y"]) - float(engine_box["y"])
        delta_x = float(reference_box["x"]) - float(engine_box["x"])
        set_stage_margin(cdp, delta_y, delta_x)
        aligned_regions = regions_of(cdp, ENGINE_SELECTOR)
        aligned_box = aligned_regions.get(name) or engine_box

        reference_shot = compare.screenshot(cdp, reference_box)
        engine_shot = compare.screenshot(cdp, aligned_box)
        pixel = compare.pixel_diff(reference_shot, engine_shot)
        same_width = (
            abs(float(reference_box["w"]) - float(aligned_box["w"])) <= 0.5
        )
        (OUT / f"reference-{section_id}-{name}.png").write_bytes(reference_shot)
        (OUT / f"engine-{section_id}-{name}.png").write_bytes(engine_shot)
        (EVIDENCE / f"reference-{section_id}-{name}.png").write_bytes(reference_shot)
        (EVIDENCE / f"engine-{section_id}-{name}.png").write_bytes(engine_shot)
        region_rows[name] = {
            "t1_pixel_ratio": pixel["ratio"],
            "t1_pixel_different": pixel["different"],
            "t1_pixel_total": pixel["total"],
            "t1_size_mismatch": pixel["size_mismatch"],
            "t3_same_width": same_width,
            "t3_reference_width": reference_box["w"],
            "t3_engine_width": aligned_box["w"],
            "alignment_delta": {"x": delta_x, "y": delta_y},
            "reference_box": reference_box,
            "engine_box": aligned_box,
        }

    # ── T4：余量条的内联宽度（**逐字** ⇒ 验证取整口径）──
    fills_equal = reference_fills == engine_fills

    passed = (
        not dom_diffs
        and fills_equal
        and bool(region_rows)
        and all(
            (not row.get("t1_size_mismatch", True))
            and row["t1_pixel_ratio"] <= PIXEL_BUDGET
            and row["t3_same_width"]
            for row in region_rows.values()
            if "missing" not in row
        )
        and not any("missing" in row for row in region_rows.values())
    )

    return {
        "passed": passed,
        "section": section_id,
        "t2_dom_equal": not dom_diffs,
        "t2_dom_diffs": dom_diffs[:8],
        "t4_fill_styles_equal": fills_equal,
        "t4_reference_fills": reference_fills,
        "t4_engine_fills": engine_fills,
        "regions": region_rows,
    }


def main() -> int:
    """本面独立跑（两个分区全跑一遍 ⇒ 写全量证据）。"""
    env = ensure_env(8787)
    cdp = env["cdp"]
    report: dict = {
        "face": "budget（预算与配额 · 设置页 S5 面）",
        "viewport": {"width": VIEWPORT_WIDTH, "height": VIEWPORT_HEIGHT},
        "device_pixel_ratio": 1,
        "timezone": compare.TIMEZONE,
        "channel_threshold": CHANNEL_THRESHOLD,
        "pixel_budget": PIXEL_BUDGET,
        "crop": "element（按**本面自己拥有的区块**裁剪；参考侧夹着的 `MetricsPanel` 是外部组件 ⇒ 未迁 ⇒ 不对照）",
        "reference_page": "/web/reference/budget/index.html",
        "engine_page": "/web/engine/budget/index.html",
        "reference_selector": REFERENCE_SELECTOR,
        "engine_selector": ENGINE_SELECTOR,
        "excluded_external_component": f".{EXTERNAL_CLASS}（MetricsPanel：外部组件 + HTTP ⇒ 未迁）",
        "sections": ["budget", "runtime"],
    }
    try:
        pairs = []
        for section_id in ("budget", "runtime"):
            record = collect_pair({"id": "budget"}, section_id, 8787)
            pairs.append(record)
            for name, row in record["regions"].items():
                if "missing" in row:
                    log(f"  {section_id} / {name}：❌ 区域缺失")
                    continue
                log(
                    f"  {section_id} / {name}：像素 {row['t1_pixel_ratio'] * 100:.4f}%"
                    f" / 宽度 {'相等' if row['t3_same_width'] else '不等'}"
                    f"（{row['t3_reference_width']} vs {row['t3_engine_width']}）"
                )
            log(
                f"  {section_id}：DOM {'一致' if record['t2_dom_equal'] else '不一致'}"
                f" / 余量条取整 {'一致' if record['t4_fill_styles_equal'] else '不一致'}"
            )
        report["pairs"] = pairs
        # ⭐ **键入对照**（2026-10-25 加）：静态像素**测不到**「用户输入进不进
        # 引擎状态」⇒ ⇒ 补一条**真实键入**的用例（见 `run_knob_input_check`）。
        knob_input = run_knob_input_check(env)
        report["knob_input"] = knob_input
        report["checks"] = {
            "t1_all_pixels_within_budget": all(
                row["t1_pixel_ratio"] <= PIXEL_BUDGET
                for pair in pairs
                for row in pair["regions"].values()
                if "missing" not in row
            ),
            "t2_dom_identical": all(pair["t2_dom_equal"] for pair in pairs),
            "t3_same_width": all(
                row["t3_same_width"]
                for pair in pairs
                for row in pair["regions"].values()
                if "missing" not in row
            ),
            "t4_fill_styles_identical": all(
                pair["t4_fill_styles_equal"] for pair in pairs
            ),
            # ⭐ 键入后取值 / 提交后取值 / 反馈文案 ⇒ 三者**逐字相同**。
            "t5_knob_input_identical": bool(knob_input.get("passed")),
        }
        report["notes"] = [
            "T1 的差异像素阈值：任一通道绝对差 > 2/255 即记「不同」。",
            "T2 的归一化：剔除 Svelte 作用域 class 与引擎侧的 `sp-*` 作用域锚点、"
            "class token 排序；**文本节点逐字保留**（不折叠空白、不裁剪）。",
            "T3 是**宽度同源**：参考侧靠生产自己的布局得到 828px，引擎侧靠"
            "对照专用样式表的 `.settings-stage` 钉成 828px ⇒ 不等即判失败。",
            "T4 是余量条内联宽度的**逐字**比对（`width: N%;`）⇒ 验证"
            "「`Math.round(ratio * 100)` 的**半数向 +∞**口径」两侧同源。",
            "**装置对齐**：参考侧是生产整页（内容块在文档里的位置带小数），"
            "引擎侧是独立挂载（位置 0）⇒ 逐区域把引擎侧舞台对齐到参考侧的文档位置"
            "（同一像素格）⇒ 差异只反映**渲染本身**。",
            "参考侧的 `MetricsPanel`（`.metrics-card`）是**外部组件 + HTTP**"
            "⇒ 本面**未迁** ⇒ 两侧都**排除**它（对照只覆盖本面自己的区块）。",
            "运行时分区的**诊断结果区**（`.diag-results`）用外部组件 `StatusBadge`"
            "（带自己的 scoped CSS）⇒ **未迁** ⇒ 参考侧的默认态亦无报告 ⇒ "
            "该分支**未覆盖**（诚实边界）。",
            "T5 是**真实键入**的对照（聚焦数字步进器 ⇒ 键入 '7' ⇒ blur 提交）"
            "⇒ 断言**键入值 / 提交值 / 反馈文案**三者两侧**逐字相同**"
            "⇒ ⇒ 覆盖「用户输入进不进引擎状态」这类**静态像素测不到**的缺陷。",
            f"环境：视口 1440×900、DPR 1、时区 {compare.TIMEZONE}"
            "（⚠️ **刻意用非 UTC**：生产时刻显示走浏览器 `Intl`（本地时区）⇒"
            "若装置也用 UTC ⇒ **两边一起错 ⇒ 掩盖时区缺陷**）。",
        ]
        report["passed"] = all(report["checks"].values())

        # 浏览器控制台（两侧各一次）。
        console = []
        for name, page, selector in (
            ("reference", "web/reference/budget", REFERENCE_SELECTOR),
            ("engine", "web/engine/budget", ENGINE_SELECTOR),
        ):
            cdp.drain()
            navigate(cdp, f"{env['base']}/{page}/index.html")
            select_section(cdp, "budget")
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
                    messages.append({"kind": entry.get("level"), "text": str(entry.get("text", ""))})
            console.append({"side": name, "selector": selector, "messages": messages})
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
            log(f"  {'PASS' if value else 'FAIL'}  {key}")
        log("")
        log(f"总判定：{'通过' if report['passed'] else '未通过'}")
        return 0 if report["passed"] else 1
    finally:
        shutdown_env()


if __name__ == "__main__":
    raise SystemExit(main())
