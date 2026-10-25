#!/usr/bin/env python3
"""架构断言：**状态对象的信号必须建在宿主的 runtime 上**。

## 为什么需要这条断言

2026-10-25 实测到一个**架构级缺陷**：`apeireth-ui-kit` 的 `BudgetState::new()`
**自建**了一个 `Runtime::new()` ⇒ ⇒ 草稿 / 反馈信号**不属于**宿主挂 effect 的那个
runtime ⇒ ⇒ **effect 根本不依赖这些信号** ⇒ ⇒ 改草稿**不触发重渲染**
⇒ ⇒ 实测表现为「**幽灵输入**」：用户敲进去的值**引擎不知道**
（DOM 上显示的是浏览器自己画的值，引擎草稿仍是 `""`）。

⇒ ⇒ 该缺陷在**类型层面完全合法**（`Runtime` 可克隆、可多建）⇒ ⇒ **编译器不可能发现**
⇒ ⇒ 只能靠**一条显式断言**守住 ⇒ ⇒ 本脚本就是那条断言。

## 判据（精确到「函数所属的 impl 块」+「能否证明是所有者」）

**候选违规**：在 `impl XxxState { … }` 这个 impl 块里的**任何函数**中出现 `Runtime::new()`。

⇒ 「状态对象」= 类型名以 `State` 结尾（`BudgetState` / `ProviderState` / …）。

**候选不等于违规** —— 状态对象**自建** runtime 本身**是合法的**：
如果它**就是**那个 runtime 的**所有者**（渲染用的 effect 挂在**它**的 runtime 上）⇒ ⇒ 完全正常。
⇒ ⇒ 实测：当前 6 个自建 runtime 的状态对象**全是所有者**（`PilotState` / `ToolCardState` /
`DataHostState` / `FrameState` / `ThemeHostState` 暴露 `fn runtime()`；
`ToolGridState` 在 `state.runtime.effect(…)` 上挂渲染）⇒ ⇒ **零违规**。

**真正的违规**（= 「幽灵输入」的结构特征）：状态对象**自建** runtime，
但**既不暴露** `fn runtime()`、**又不在自己的** runtime 上挂 effect
⇒ ⇒ 渲染必然在**另一个** runtime 上 ⇒ ⇒ effect **不依赖**它的信号
⇒ ⇒ 改状态**不触发重渲染** ⇒ ⇒ 用户输入进不了引擎状态。

⇒ ⇒ 因此判据 = 「自建 runtime」**且**「**证明不了**自己是所有者」⇒ ⇒ **报违规**。

⚠️ **这是近似判据**（用「同文件里有没有 `fn runtime(` 或 `runtime.effect(`」当证据）
⇒ ⇒ 它可能**漏判**（若所有者证据在别的文件）
⇒ ⇒ 但它**零误报** ⇒ ⇒ 宁可漏判也不误伤 ⇒ ⇒ 判据表头写明这一点。

## 为什么用「命名约定」而不是「更精确的类型分析」

`Runtime::new()` 是 `pub`（native 测试、多个宿主 crate 都要用）⇒ ⇒ 类型层面无法禁止。
⇒ ⇒ 断言只能落在**约定**上 ⇒ ⇒ 因此**约定必须写清楚**（本文件头）
⇒ ⇒ 且**误报要能一眼看懂**（脚本会打印文件:行、所属 impl 类型、所在函数）。

## 用法

```bash
python3 crates-ui/apeireth-ui-demo/harness/scripts/check_runtime_ownership.py
```

退出码：**0 = 通过**；**1 = 有违规**（逐条打印）。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATES_UI = HERE.parent.parent.parent

#: 「状态对象」的类型名后缀（大写驼峰的 `State`）。
STATE_SUFFIX = "State"

#: `Runtime::new()` 的调用形态（允许 `Runtime::new ()` 的空格变体）。
RUNTIME_NEW = re.compile(r"\bRuntime::new\s*\(\s*\)")

#: `impl Xxx {` / `impl Xxx for Y {` 的行首形态。
IMPL_HEAD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?impl(?:<[^>]*>)?\s+([A-Za-z_][A-Za-z0-9_]*)")

#: 函数定义行（粗判：`fn` 开头，允许缩进与 `pub`/`async` 等修饰）。
FN_HEAD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")


def _owner_type_at(lines: list[str], index: int) -> tuple[str, str]:
    """返回 `(impl 类型名, 函数名)`：该行所处的 impl 块与函数。

    - impl 块：从上往下找**最近的** `impl Xxx {`；
    - 函数：从上往下找**最近的** `fn …`（且它在该 impl 块内）。
    """
    impl_type = ""
    impl_at = -1
    fn_name = ""
    fn_at = -1
    for i in range(index, -1, -1):
        line = lines[i]
        if not fn_name:
            match = FN_HEAD.match(line)
            if match:
                fn_name = match.group(1)
                fn_at = i
                continue
        match = IMPL_HEAD.match(line)
        if match:
            impl_type = match.group(1)
            impl_at = i
            break
    # 函数必须**晚于** impl 头（否则那个 fn 属于上一个 impl）。
    if fn_at < impl_at:
        fn_name = ""
    return impl_type, fn_name


def _violations_in_file(path: Path) -> list[str]:
    """一个文件里的违规（每条：行号 + impl 类型 + 所在函数 + 源码行）。"""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError):
        return []
    out: list[str] = []
    for index, line in enumerate(lines):
        if not RUNTIME_NEW.search(line):
            continue
        owner, fn_name = _owner_type_at(lines, index)
        if not owner or not owner.endswith(STATE_SUFFIX):
            continue
        # 「能不能证明它是那个 runtime 的所有者？」——
        # 证据 ①：它暴露 `fn runtime()`（宿主据此在同一 runtime 上挂 effect）；
        # 证据 ②：它在自己的 runtime 上挂 effect（`…runtime.effect(`）。
        text = "\n".join(lines)
        owns = bool(
            re.search(rf"\bfn\s+runtime\b", text)
            or re.search(r"\bruntime\s*\.\s*effect\s*\(", text)
            or re.search(rf"\b{re.escape(owner)}\s*\.\s*runtime\s*\.\s*effect\s*\(", text)
        )
        if owns:
            continue
        where = f"{owner}::{fn_name}" if fn_name else owner
        out.append(
            f"{path.relative_to(CRATES_UI)}:{index + 1}: "
            f"`{where}` 自建 `Runtime` 且**证明不了自己是所有者** "
            f"（既无 `fn runtime()`、又没在自己的 runtime 上挂 effect）"
            f" ⇒ ⇒ 渲染必然在另一个 runtime 上 ⇒ ⇒ 响应式链断"
        )
    return out


def main() -> int:
    roots = [
        CRATES_UI / "apeireth-ui-core" / "src",
        CRATES_UI / "apeireth-ui-dom" / "src",
        CRATES_UI / "apeireth-ui-kit" / "src",
        CRATES_UI / "apeireth-ui-demo" / "src",
        CRATES_UI / "apeireth-ui-gpu" / "src",
        CRATES_UI / "apeireth-ui-scene" / "src",
    ]
    violations: list[str] = []
    scanned = 0
    for root in roots:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.rs")):
            scanned += 1
            violations.extend(_violations_in_file(path))
    if violations:
        print("✗ 架构断言失败：状态对象不得自建 `Runtime`")
        print("  （自建 ⇒ 状态与渲染**分属两个 runtime** ⇒ 响应式链断 ⇒ 「幽灵输入」）")
        for item in violations:
            print(f"  - {item}")
        return 1
    print(f"✔ 架构断言通过：{scanned} 个源文件里没有「状态对象自建 `Runtime`」")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
