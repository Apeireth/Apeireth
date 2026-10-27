#!/usr/bin/env python3
"""⭐⭐ **引擎边界守卫** —— 把「初心边界」从文档变成**会红的门禁**。

## 为什么需要它（2026-10-27 立项依据）

引擎的**初心**（`docs/adr/0015-pure-rust-ui-engine.md` §2.1）写的是硬边界：

> 引擎只做三件事：响应式调度 / DOM 绑定 / 过渡时序。
> **不写 CSS 引擎、不写滤镜层、不碰 shader、不碰排版、不碰主题。**

owner 2026-10-27 已拍板：**片头（GPU 场景）留在引擎** ⇒ ⇒ 那就需要把
「**哪些层被授权碰 GPU**」**写清楚并守住** ⇒ ⇒ 否则边界只存在于口头，
下次就有人（人或 AI）把 shader 塞进 `ui-core` / `ui-dom` / `ui-kit`。

⇒ ⇒ 本脚本把边界落成**可执行断言**，进 CI。

## 它守什么（两条）

### 守卫 A：**三件套 0 shader**（`ui-core` / `ui-dom` / `ui-kit`）

这三个 crate **只做**响应式调度 / DOM 绑定 / 组件 ⇒ ⇒ **不得**含着色器源码
（GLSL / WGSL 的任何痕迹）或 GPU 相关资产。

⇒ ⇒ 依据：`docs/adr/0015` §2.1「不碰 shader」。

### 守卫 B：**组件不得自算时区**（`ui-demo`）

实测缺陷（2026-10-27）：`approval_card.rs` 有一份自己的「固定 UTC」实现
（`format_clock_utc`）⇒ ⇒ **绕过了**进程级时间器
⇒ ⇒ 宿主虽然装了浏览器 `Intl`，那个消费方**不消费**
⇒ ⇒ 实测非 UTC 时区下生产 `09:10` / 引擎 `01:10`（**差 8 小时**）。

⇒ ⇒ 故：**组件代码不得**引用自算时区的实现
⇒ ⇒ 时间文本**一律**走 `apeireth_ui_kit::governance_tab::time_text()`。

## 豁免（**有意为之，不是漏洞**）

- `apeireth-ui-gpu` / `apeireth-ui-scene`：**被授权**含着色器与 GPU 代码
  （owner 2026-10-27 拍板「片头留在引擎」⇒ ⇒ 授权范围见 `docs/adr/0017-*`）；
- `apeireth-ui-demo`：`format_clock_utc` 的**定义处**与**测试处**允许存在
  （它是 native 的**可复现基准**，且有单测）⇒ ⇒ 只禁**其它组件**引用它。

## 用法与退出码

```bash
python3 crates-ui/apeireth-ui-demo/harness/scripts/check_ui_boundaries.py
```

`0` = 通过；`1` = 有越界；`2` = 用法/环境错误。
"""

from __future__ import annotations

import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
CRATES_UI = HERE.parent.parent.parent

#: **三件套**：不得含着色器 / GPU 资产。
CORE_CRATES = ("apeireth-ui-core", "apeireth-ui-dom", "apeireth-ui-kit")

#: 着色器源码的**特征**（任一命中即视为「含 shader」）。
SHADER_MARKERS = (
    "#version 300 es",
    "#version 100",
    "gl_Position",
    "gl_PointSize",
    "gl_FragCoord",
    "gl_PointCoord",
    "fragColor",
    "fn main(",
    "@fragment",
    "@vertex",
    "wgpu::",
    "naga::",
)

#: 结构性禁令：三件套里**不得**出现 GPU 资产文件名。
FORBIDDEN_FILENAMES = ("shaders.rs", "glsl.rs", "wgsl.rs")

#: 组件层**不得**引用的「自算时区」符号。
SELF_COMPUTED_TIME_SYMBOLS = ("format_clock_utc",)

#: 允许出现这些符号的位置（定义处 + 测试处）。
_ALLOWED_CONTEXT = ("#[cfg(test)]", "mod tests", "assert_eq!", "assert!", "///", "//")


def _rel(path: pathlib.Path) -> str:
    try:
        return str(path.relative_to(CRATES_UI))
    except ValueError:
        return str(path)


def _is_in_allowed_context(line: str) -> bool:
    stripped = line.strip()
    return any(marker in stripped for marker in _ALLOWED_CONTEXT)


def check_no_shader_in_core_crates() -> list[str]:
    """守卫 A：三件套不得含着色器 / GPU 资产。"""
    violations: list[str] = []
    for crate in CORE_CRATES:
        crate_dir = CRATES_UI / crate / "src"
        if not crate_dir.is_dir():
            continue
        for path in sorted(crate_dir.rglob("*.rs")):
            name = path.name.lower()
            if name in FORBIDDEN_FILENAMES:
                violations.append(
                    f"{_rel(path)}：三件套内不得存在 GPU 资产文件（{path.name}）"
                )
                continue
            text = path.read_text(encoding="utf-8")
            for number, line in enumerate(text.splitlines(), start=1):
                if _is_in_allowed_context(line):
                    continue
                for marker in SHADER_MARKERS:
                    if marker in line:
                        violations.append(
                            f"{_rel(path)}:{number}：三件套内出现着色器痕迹 "
                            f"`{marker}` ⇒ ⇒ 越出 ADR-0015 §2.1「不碰 shader」"
                        )
                        break
    return violations


def check_no_self_computed_time_in_components() -> list[str]:
    """守卫 B：组件不得引用「自算时区」实现。"""
    violations: list[str] = []
    demo_src = CRATES_UI / "apeireth-ui-demo" / "src"
    if not demo_src.is_dir():
        return violations
    for path in sorted(demo_src.rglob("*.rs")):
        if path.name == "approval_card.rs":
            # 定义处 + 其测试 ⇒ 允许（native 可复现基准）。
            continue
        text = path.read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), start=1):
            if _is_in_allowed_context(line):
                continue
            for symbol in SELF_COMPUTED_TIME_SYMBOLS:
                if symbol in line:
                    violations.append(
                        f"{_rel(path)}:{number}：组件引用了自算时区的 `{symbol}` "
                        f"⇒ ⇒ 会绕过进程级时间器 ⇒ ⇒ 应改用 "
                        f"`apeireth_ui_kit::governance_tab::format_clock_hm` / `time_text()`"
                    )
                    break
    return violations


def main() -> int:
    if not CRATES_UI.is_dir():
        print(f"找不到 crates-ui 根目录：{CRATES_UI}", file=sys.stderr)
        return 2

    violations = check_no_shader_in_core_crates()
    violations += check_no_self_computed_time_in_components()

    if violations:
        print("✗ 引擎边界守卫：发现越界", file=sys.stderr)
        for item in violations:
            print(f"  · {item}", file=sys.stderr)
        print(
            "  ⇒ ⇒ 若确属**有意**（例如 owner 拍板扩大授权范围），\n"
            "     请改 `docs/adr/0017-*` 写清授权范围并**同步**本脚本的豁免表。",
            file=sys.stderr,
        )
        return 1

    print(
        "✅ 引擎边界守卫通过："
        f"三件套（{'/'.join(CORE_CRATES)}）0 shader 资产；"
        "组件层不自算时区"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
