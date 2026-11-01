#!/usr/bin/env python3
"""设置页三页对照：**骨架**（像素 + DOM 全等 + 宽度同源 + JSON 差分）。

## 本文件是什么

设置页三页（`SettingsView.svelte` / `FirstRunWizard.svelte` /
`ThemeSettingsPanel.svelte`）被切成 **8 个面**（见 `web/shared/panels.js`）⇒
对照是「**面 × 分区**」的矩阵，不是单张卡。

本脚本是**骨架**：可复用的部分（浏览器驱动 / 截图 / 像素差分 / DOM 归一化）
**沿用批一**（`harness/tools/compare.py` 与 `compare_toolcard.py`）⇒ 本目录
**不重复实现**它们；本文件只写设置页特有的那几件事：

1. **矩阵驱动**：按 `web/shared/panels.js` 的注册表遍历「面 × 分区」；
2. **宽度同源断言**（设置页特有）：两侧裁剪框宽度**必须相等**，否则直接判失败
   ——因为引擎侧是**独立挂载**的子面，宽度不同 ⇒ 文字换行不同 ⇒ 像素必然不一致，
   这种失败**不该被算成「像素差异」**；
3. **分区切换**：参考侧用生产的 `initialSection` prop 驱动；引擎侧调
   `window.__SETTINGS__.selectSection(id)`；
4. **覆盖层按整视口对照**（API Key 模态 / 向导遮罩是 `position: fixed`）。

## 对照项与判据

| # | 对照项 | 判据 |
|---|---|---|
| T1 | **静态像素对照** | 每（面 × 分区）差异像素占比 **≤ 1%**（任一通道绝对差 > 2/255 即记「不同」） |
| T2 | **DOM 结构对照** | 归一化后**完全相等**（D1 结构 / D2 属性 / D3 文本必须为零；文本节点**逐字**） |
| T3 | **宽度同源断言** | 两侧裁剪框**宽度相等**（不等 ⇒ **直接判失败**，不进入 T1） |
| T4 | **JSON 差分** | 引擎侧序列化器 vs 浏览器侧 `JSON.stringify`：**逐字节相同** |
| T5 | **逐帧对照** | 切换/展开动作期间每帧 **≤ 1%** |

## 环境固定

- 视口 `1440×900`、`devicePixelRatio = 1`；
- 时区 `UTC`（两侧同一时刻）；
- 同一浏览器实例、同一份 CSS、同一份 fixture；
- `html[data-theme="night"]`（两侧页面同值）。

## 现状（诚实口径）

**本批没有跑过一次对照**（没有迁移 ⇒ 没有引擎侧产物 ⇒ 对照无对象）。
本文件是**骨架**：它把「怎么跑」写清楚了，但**没有实测结果**。
⇒ 迁移线开工后，第一次真实运行的结果**必须**落进 `evidence/` 并随 commit 入库。

## 用法

```bash
python3 tools/compare_settings.py --panel provider --section models
python3 tools/compare_settings.py --all
```

依赖：系统 Chromium、`websocket-client`、Pillow。**不需要** Playwright。
"""

from __future__ import annotations

import argparse
from pathlib import Path
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
SETTINGS = HERE.parent
HARNESS = SETTINGS.parent

# 批一的对照装置（**复用**，不重复实现）。
sys.path.insert(0, str(HARNESS / "tools"))

try:
    import compare  # noqa: E402
    import compare_toolcard  # noqa: E402
except ImportError:  # pragma: no cover - 批一装置缺失时的显式失败
    print(
        "找不到批一的对照装置："
        f"{HARNESS / 'tools' / 'compare.py'}\n"
        "（设置页 harness 复用批一的浏览器驱动 / 截图 / 像素差分 ⇒ 批一装置是**前置**）",
        file=sys.stderr,
    )
    raise SystemExit(2)

#: 视口与阈值（沿用批一口径）。
VIEWPORT_WIDTH = compare.VIEWPORT_WIDTH
VIEWPORT_HEIGHT = compare.VIEWPORT_HEIGHT
CHANNEL_THRESHOLD = compare.CHANNEL_THRESHOLD
PIXEL_BUDGET = compare.PIXEL_BUDGET

#: 产物目录（**不入库**）。
OUT = SETTINGS / "out"
#: 入库证据目录（对照跑完后**由主线**决定是否入库）。
EVIDENCE = SETTINGS / "evidence"

#: 子面注册表（与 `web/shared/panels.js` **同一份** ⇒ 用 node 读，不手抄）。
PANELS_JS = SETTINGS / "web" / "shared" / "panels.js"


def load_panels() -> list[dict]:
    """读子面注册表（**经 node** ⇒ 与浏览器侧同一份真相）。

    ## 为什么经 node 而不是 `json.load`

    注册表是 **ES module**（`export const PANELS = [...]`）⇒ Python 读不了。
    ⇒ 用 node 把它转成 JSON 打到标准输出 ⇒ **不手抄第二份表**。

    ⚠️ node 22 是硬要求（批一的记录：本机默认 node 是 v24 ⇒ 前端/vite 相关
    必须用 v22）。
    """
    import shutil
    import subprocess

    node = shutil.which("node")
    if node is None:
        print("找不到 node（读取子面注册表需要 node 22）", file=sys.stderr)
        raise SystemExit(2)
    script = (
        f"import('file://{PANELS_JS}').then(m => "
        "console.log(JSON.stringify(m.PANELS.map(p => ({"
        "id: p.id, title: p.title, scope: p.scope, crop: p.crop, "
        "sections: p.sections.map(s => ({id: s.id, ref: s.ref, engine: s.engine, crop: s.crop ?? p.crop})), "
        "deps: p.deps, trust: p.trust})))));"
    )
    result = subprocess.run(
        [node, "--input-type=module", "-e", script],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(f"读子面注册表失败：{result.stderr}", file=sys.stderr)
        raise SystemExit(2)
    return json.loads(result.stdout)


def reference_url(panel: dict) -> str:
    """参考侧页面地址（**相对** ⇒ 由起服务的根决定）。"""
    return f"/reference/{panel['id']}/index.html"


def engine_url(panel: dict) -> str:
    """引擎侧页面地址。"""
    return f"/engine/{panel['id']}/index.html"


def element_box(cdp, selector: str) -> dict | None:
    """取元素的外框（相对文档坐标）。

    ## 为什么用「元素框」而不是固定坐标

    参考侧是**整页**生产容器（页头 + 侧栏 + 内容列），引擎侧是**独立挂载**的
    子面 ⇒ 两者的元素位置**必然不同** ⇒ 只能按**各自的元素框**裁剪 ⇒
    裁剪出的两张图尺寸相同即可（宽度相等由 T3 断言兜底）。
    """
    raw = cdp.evaluate(
        "(() => { const el = document.querySelector("
        + json.dumps(selector)
        + "); if (!el) return null; const r = el.getBoundingClientRect();"
        " return JSON.stringify({x: r.left + window.scrollX, y: r.top + window.scrollY,"
        " w: r.width, h: r.height}); })()"
    )
    return json.loads(raw) if raw else None


def assert_same_width(left: dict | None, right: dict | None, panel: dict, section: str) -> None:
    """**宽度同源断言**（T3）。

    ## 为什么这是独立的一项

    引擎侧是**独立挂载**的子面 ⇒ 它的宽度靠对照专用样式表钉成 828px；
    参考侧靠**生产自己的布局**得到 828px。若两者不等 ⇒ 文字换行位置不同 ⇒
    像素差异是**必然**的，而不是「引擎画错了」⇒ 这种失败**不该被算进 T1 的
    1% 预算** ⇒ 直接判失败并说清原因。
    """
    if left is None or right is None:
        raise SystemExit(
            f"[{panel['id']} / {section}] 裁剪框缺失：参考侧={left} 引擎侧={right}"
        )
    if abs(left["w"] - right["w"]) > 0.5:
        raise SystemExit(
            f"[{panel['id']} / {section}] **宽度不同源**：参考侧 {left['w']}px vs "
            f"引擎侧 {right['w']}px ⇒ 先修舞台宽度（public/settings-test.css），"
            "不要把它算成像素差异。"
        )


def normalize_script(selector: str) -> str:
    """归一化 DOM 的脚本（**在批一口径上加一条**）。

    ## 与批一的差异

    批一口径：剔除 Svelte 作用域 class、class token 排序、属性键排序、
    丢弃注释锚点、丢弃纯空白文本节点、其余文本**逐字**保留。

    **本面追加**：引擎侧多一层 `.sp-*` 包裹根 class（CSS 作用域锚点）⇒
    归一化时必须**剔除** `sp-` 前缀的 token ⇒ 否则 D2 属性差异恒不为零。
    """
    return compare_toolcard_normalize_script(selector)


def compare_toolcard_normalize_script(selector: str) -> str:
    """取批一的归一化脚本，并**追加**「剔除 `sp-*` 根 class」这一条。"""
    base = compare_toolcard.NORMALIZE_SCRIPT
    patched = base.replace(
        "const strip = (value) => (value || '').replace(/\\bsvelte-[0-9a-z]+\\b/g, '').trim();",
        "const strip = (value) => (value || '')"
        ".replace(/\\bsvelte-[0-9a-z]+\\b/g, '')"
        ".replace(/\\bsp-[a-z0-9-]+\\b/g, '')"
        ".trim();",
    )
    if patched == base:
        print(
            "⚠️ 批一的归一化脚本没有匹配到预期的 `strip` 定义 ⇒ "
            "「剔除 sp-* 根 class」这条**没有生效** ⇒ D2 属性差异会恒不为零。",
            file=sys.stderr,
        )
    return patched


class PanelNotMigrated(RuntimeError):
    """该面**尚未迁移** ⇒ 无对照对象（**不是失败，是还没做**）。

    ⚠️ 与「对照失败」必须**严格区分**：前者是「没做」，后者是「做了但不对」。
    ⇒ 混为一谈会让「0 差异」的报告**失真**。
    """


def find_driver(panel_id: str):
    """找某个面的**面级对照驱动**（返回模块对象；找不到 ⇒ `None`）。

    ## 为什么驱动是「每面一个」

    8 个面**并行**施工 ⇒ 若驱动写在共享文件里 ⇒ 必然撞车。
    ⇒ 约定：**每个面把驱动放在自己的独占目录**，本模块只做**发现 + 转发**。

    ## 搜索顺序（先精确后模糊）

    1. `drivers/<面 id>_compare.py`
    2. `tools/compare_<面 id>.py`
    3. `web/engine/<面 id>/compare*.py`
    4. 递归扫 `**/compare_<面 id>*.py`（排除本文件自身）

    ⇒ 驱动只需暴露 `collect_pair(panel, section, port) -> dict`，
    且返回值里要有 `passed`（布尔）与判据明细。
    """
    candidates = [
        SETTINGS / "drivers" / f"{panel_id}_compare.py",
        SETTINGS / "tools" / f"compare_{panel_id}.py",
    ]
    engine_dir = SETTINGS / "web" / "engine" / panel_id
    if engine_dir.is_dir():
        candidates.extend(sorted(engine_dir.glob("compare*.py")))
    for path in sorted(SETTINGS.rglob(f"compare_{panel_id}*.py")):
        if path.resolve() != Path(__file__).resolve():
            candidates.append(path)
    for path in candidates:
        if not path.is_file():
            continue
        spec = importlib.util.spec_from_file_location(f"settings_driver_{panel_id}", path)
        if spec is None or spec.loader is None:
            continue
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        if hasattr(module, "collect_pair"):
            return module
        # ── **CLI 驱动形态**（并行面 M2 用的）：独立脚本 + `main()` 返回退出码 ──
        # ⚠️ 两种形态都要支持：各面施工线的实现习惯不同（有的按模块协议导出
        # `collect_pair`，有的写成可独立运行的脚本）⇒ **共享入口不能只认一种**，
        # 否则「驱动明明在、却报未迁移」⇒ 红灯失去信号。
        if hasattr(module, "main") and callable(module.main):
            return module
    return None


# CLI 驱动的结果**按面缓存**（一个 CLI 驱动一次跑完整个面 ⇒ 不必每个分区重跑）。
_CLI_CACHE: dict[str, dict] = {}


def collect_pair(panel: dict, section: str, port: int) -> dict:
    """跑一对（面 × 分区）的对照，产出对照记录。

    ## 编排（与骨架说明一致）

    1. 导航到参考侧 ⇒ 等 `window.__SETTINGS__.ready()`；
    2. `selectSection(section)` ⇒ 等一帧；
    3. 取裁剪框（按**该分区在参考侧的选择器** `sections[].ref`）；
    4. 导航到引擎侧 ⇒ 等就绪 ⇒ `selectSection(section)`；
    5. 取裁剪框（按**该分区在引擎侧的选择器** `sections[].engine`）⇒ **断言宽度相等**；
    6. 截图两侧 ⇒ 像素差分 + DOM 归一化差分。

    ⚠️ 「等一帧」的**具体实现**沿用批一的 `time.sleep(0.4)` 口径
    （批一已实测该口径可用）⇒ 迁移线若要更严，改这里。

    ## 本函数的职责边界

    本函数**只做发现与转发** ⇒ 具体编排由**各面的驱动**承担
    （各面并行施工 ⇒ 编排也各不相同：裁剪方式、控件交互、分区数都不同）。
    ⇒ 驱动不存在 ⇒ 显式抛 `PanelNotMigrated`（**「没做」**，不是「做错了」）。
    """
    panel_id = panel["id"]
    driver = find_driver(panel_id)
    if driver is None:
        raise PanelNotMigrated(
            f"[{panel_id} / {section}] 该面**尚未迁移** ⇒ 无对照对象。"
            f"（约定：驱动放在 drivers/{panel_id}_compare.py 或"
            f" tools/compare_{panel_id}.py 等面自己的目录里）"
        )
    if hasattr(driver, "collect_pair"):
        return driver.collect_pair(panel, section, port)
    # ── CLI 驱动：跑它的 `main()`，用**退出码**判成败 ──
    if panel_id in _CLI_CACHE:
        return _CLI_CACHE[panel_id]
    exit_code = driver.main()
    record = {
        "passed": exit_code == 0,
        "driver": "cli",
        "exit_code": exit_code,
    }
    _CLI_CACHE[panel_id] = record
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description="设置页三页对照（骨架）")
    parser.add_argument("--panel", help="面 id（见 web/shared/panels.js）")
    parser.add_argument("--section", help="生产分区 id")
    parser.add_argument("--all", action="store_true", help="跑全部「面 × 分区」")
    parser.add_argument(
        "--port",
        type=int,
        default=8787,
        help="驱动起服务用的端口（页面地址是**相对**的 ⇒ 端口由起服务方决定）",
    )
    args = parser.parse_args()

    panels = load_panels()
    print(f"子面注册表：{len(panels)} 个面")
    port = args.port

    if args.all:
        matrix = [
            (panel, section["id"])
            for panel in panels
            for section in panel["sections"]
        ]
    elif args.panel:
        selected = [panel for panel in panels if panel["id"] == args.panel]
        if not selected:
            print(f"未知面 id：{args.panel}", file=sys.stderr)
            return 2
        panel = selected[0]
        sections = [args.section] if args.section else [s["id"] for s in panel["sections"]]
        matrix = [(panel, section) for section in sections]
    else:
        print("需要 --panel <id> 或 --all", file=sys.stderr)
        return 2

    print(f"对照矩阵：{len(matrix)} 组")
    for panel, section in matrix:
        row = next((s for s in panel["sections"] if s["id"] == section), None)
        crop = (row or {}).get("crop", panel["crop"])
        print(f"  · {panel['id']} / {section}"
              f"（裁剪 参考侧={(row or {}).get('ref')} / 引擎侧={(row or {}).get('engine')}，"
              f"{'整视口' if crop == 'viewport' else '元素框'}）")

    # ── 跑对照：按面分发到**各面的驱动**（共享文件只做发现与转发）──
    passed = 0
    failed = 0
    not_migrated: dict[str, int] = {}
    failures: list[str] = []
    for panel, section in matrix:
        try:
            record = collect_pair(panel, section, port)
        except PanelNotMigrated:
            not_migrated[panel["id"]] = not_migrated.get(panel["id"], 0) + 1
            print(f"  ⏭ {panel['id']} / {section}：该面**尚未迁移** ⇒ 无对照对象")
            continue
        except Exception as exc:  # noqa: BLE001 —— 对照失败要**显式报出来**
            failed += 1
            failures.append(f"{panel['id']} / {section}：{exc}")
            print(f"  ❌ {panel['id']} / {section}：{exc}")
            continue
        ok = bool(record.get("passed"))
        if ok:
            passed += 1
            print(f"  ✅ {panel['id']} / {section}")
        else:
            failed += 1
            failures.append(f"{panel['id']} / {section}：判据未过")
            print(f"  ❌ {panel['id']} / {section}：判据未过")
            for key, value in sorted(record.items()):
                if key == "passed":
                    continue
                print(f"       · {key}: {value}")

    print()
    print(f"对照结果：通过 {passed} / 失败 {failed}")
    if not_migrated:
        detail = "、".join(f"{k}（{v} 组）" for k, v in sorted(not_migrated.items()))
        print(f"⚠️ **尚未迁移**（无对照对象，**不是失败**）：{detail}")
    if failures:
        print("❌ 失败明细：")
        for line in failures:
            print(f"   · {line}")
        return 1
    if passed == 0:
        print("⚠️ 本次**没有可对照的面**（全部尚未迁移）⇒ 退出码 1")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
