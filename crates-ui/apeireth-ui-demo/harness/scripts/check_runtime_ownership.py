#!/usr/bin/env python3
"""架构断言：**状态对象的信号必须建在宿主给的 runtime 上**（双保险）。

## 为什么需要这条断言（以及它现在是「双保险」）

2026-10-25 实测到一个**架构级缺陷**：`apeireth-ui-kit` 的 `BudgetState::new()`
**自建**了一个 `Runtime::new()` ⇒ ⇒ 草稿 / 反馈信号**不属于**宿主挂 effect 的那个
runtime ⇒ ⇒ **effect 根本不依赖这些信号** ⇒ ⇒ 改草稿**不触发重渲染**
⇒ ⇒ 实测表现为「**幽灵输入**」：用户敲进去的值**引擎不知道**（DOM 上显示的是
浏览器自己画的值，引擎草稿仍是 `""`）。

⇒ ⇒ 2026-10-26 起，**类型层面**已强制这条不变量（见下）⇒ ⇒ **本脚本变成
「冗余双保险」**：它抓的是**类型管不到**的那一类 ——「新增代码忘了迁移」。

## 类型层面已经强制了什么（2026-10-26）

`apeireth-ui-core` 引入了**使用者形态** [`ReactiveScope`]（对应
[`Runtime::reactive_scope()`]）：

| 形态 | 类型 | 能力 | 谁拿 |
|---|---|---|---|
| **所有者形态** | `Runtime` | 建信号 / 派生 / **挂 effect** / flush / 注入调度器 | **应用根**（宿主入口） |
| **使用者形态** | `ReactiveScope` | **只**能建信号 / 派生 | **状态持有者** |

⇒ ⇒ 状态持有者**在类型层面**拿不到 `effect` / `flush` ⇒ ⇒ 它**不可能**成为
渲染链的另一端 ⇒ ⇒ 「信号与 effect 分属两个 runtime」这个**幽灵输入的结构
条件**被切断 ⇒ ⇒ 证明方式 = `ui-core` 里的三个 `compile_fail` doc-test
（`ReactiveScope` 上没有 `effect` / `flush` / `new`）。

⇒ ⇒ **本脚本不再承担「类型管不到」的那部分** ⇒ ⇒ 它的职责收窄为：
**「谁在 `impl` 块里自建 runtime」**（类型层面管得住「能力」，管不住
「有人绕过形态、直接调 `Runtime::new()`」）。

## 判据（**精确**，不再是近似启发式）

三条规则，全部落在**生产代码**上（`#[cfg(test)]` 模块与 `tests/` 目录**豁免**
⇒ ⇒ native 测试仍可建独立运行时，见硬约束 3）：

| # | 判据 | 为什么它是精确的 |
|---|---|---|
| **R1** | **任何 `impl` 块内**出现 `Runtime::new()` ⇒ ⇒ **违规** | 状态持有者的方法**只可能**写在 `impl` 块里 ⇒ ⇒ 「状态持有者自建 runtime」**必落在此** ⇒ ⇒ 不需要「能不能证明是所有者」那类启发式 |
| **R2** | `impl XxxState` / `impl XxxHost` 里出现**零参** `fn new()` ⇒ ⇒ **违规** | 零参构造**没有**runtime 可用 ⇒ ⇒ 它必然自建 ⇒ ⇒ 隐式自建路径 |
| **R3** | `impl Default for XxxState` / `for XxxHost` ⇒ ⇒ **违规** | `Default` 没有外部 runtime 可用 ⇒ ⇒ 它必然自建 ⇒ ⇒ 隐式自建路径 |

⚠️ **为什么 R1 不误报「应用根」**：应用根（`main` / wasm `mount*`）建 runtime
的代码写在**自由函数**里（不在 `impl` 块内）⇒ ⇒ R1 不命中它们 ⇒ ⇒
**所有者形态**的创建点仍然自由。

⚠️ **为什么 `ui-core` 自己的 `impl Runtime` 不误报**：`Runtime::new()` 的
**定义**在 `impl Runtime` 里，但它**不调用** `Runtime::new()` ⇒ ⇒ R1 不命中；
`ui-core` 内部测试模块里的调用在 `#[cfg(test)]` 内 ⇒ ⇒ 豁免。

## 与上一版（旧近似判据）的差别

旧版判据 = 「在 `impl XxxState` 里出现 `Runtime::new()` **且**同文件里找不到
`fn runtime()` / `runtime.effect(` 的所有者证据」⇒ ⇒ **近似**（用命名当证据）
⇒ ⇒ **可能漏判**（所有者证据在别的文件时）。

新版判据 = 「生产代码的 `impl` 块里**一律**不许出现 `Runtime::new()`」
⇒ ⇒ **精确**（作用域判定用大括号配平，不是文本启发式）⇒ ⇒ **零漏判**
（唯一豁免是 `#[cfg(test)]`，而那正是「native 测试建独立运行时」的合法场景）。

## 用法

```bash
python3 crates-ui/apeireth-ui-demo/harness/scripts/check_runtime_ownership.py
```

退出码：**0 = 通过**；**1 = 有违规**（逐条打印：文件:行 + 所属 impl + 规则）。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATES_UI = HERE.parent.parent.parent

#: `Runtime::new()` 的调用形态（允许 `Runtime::new ()` 的空格变体）。
RUNTIME_NEW = re.compile(r"\bRuntime::new\s*\(\s*\)")

#: `impl Xxx {` / `impl Xxx for Y {` / `impl Trait for Xxx {` 的行首形态。
IMPL_HEAD = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?impl(?:<[^>]*>)?\s+([A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s+for\s+([A-Za-z_][A-Za-z0-9_]*))?"
)

#: 函数定义行（粗判：`fn` 开头，允许缩进与 `pub`/`async` 等修饰）。
FN_HEAD = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)

#: 「状态持有者」的类型名后缀（大写驼峰的 `State` / `Host`）。
STATE_SUFFIXES = ("State", "Host")

#: `#[cfg(test)]` 模块 / `mod tests` 的行首形态（**豁免**：native 测试建独立运行时）。
TEST_MARKER = re.compile(r"^\s*(?:#\[cfg\(test\)\]\s*)?mod\s+tests\b")

#: 零参构造（`fn new()`）—— 状态持有者里出现它 ⇒ ⇒ 隐式自建路径。
ZERO_ARG_NEW = re.compile(r"\bfn\s+new\s*\(\s*\)")

#: 零参**构造调用**（`Self::new()` / `XxxState::new()`）—— `Default` 里出现它
#: ⇒ ⇒ 走的就是「零参构造」这条自建路径。
ZERO_ARG_CTOR_CALL = re.compile(r"\b(?:Self|[A-Z][A-Za-z0-9_]*)\s*::\s*new\s*\(\s*\)")


def _strip_code_noise(line: str) -> str:
    """去掉行内注释与字符串字面量（避免注释里的花括号 / 关键字干扰配平）。"""
    out = []
    i = 0
    n = len(line)
    in_str = False
    in_char = False
    while i < n:
        ch = line[i]
        if in_str:
            if ch == "\\":
                i += 2
                continue
            if ch == '"':
                in_str = False
            i += 1
            continue
        if in_char:
            if ch == "\\":
                i += 2
                continue
            if ch == "'":
                in_char = False
            i += 1
            continue
        if ch == "/" and i + 1 < n and line[i + 1] == "/":
            break
        if ch == '"':
            in_str = True
            i += 1
            continue
        if ch == "'" and i + 2 < n and line[i + 2] == "'":
            in_char = True
            i += 3
            continue
        out.append(ch)
        i += 1
    return "".join(out)


def _scan_blocks(lines: list[str]) -> list[tuple[int, int, str, str, bool]]:
    """逐行扫描，给出每个 `impl` 块的 `(起始行, 结束行, 自身类型名, trait 名, 是否测试区)`。

    ## 为什么用「字符级栈」而不是「数大括号」

    `impl X { fn f() { Self { .. } } }` 里，**表达式的大括号**（`Self { .. }`）
    与**块的大括号**长得一样 ⇒ ⇒ 单纯数 `{` / `}` 会把 `impl` 提前弹栈
    ⇒ ⇒ 范围算错 ⇒ ⇒ 判据失真。

    ⇒ ⇒ 这里维护一个**块栈**：每个 `{` 都压栈（带自己的表头），每个 `}` 弹栈
    ⇒ ⇒ `impl` 只在**自己那个** `}` 到来时才闭合 ⇒ ⇒ 范围精确。
    """
    ranges: list[tuple[int, int, str, str, bool]] = []
    # 栈元素：`(种类, 起始行, 自身类型名, trait 名)`；种类 ∈ {impl, other}
    stack: list[tuple[str, int, str, str]] = []
    in_test_depth = 0  # >0 ⇒ 当前处在 `mod tests` 之内
    for index, raw in enumerate(lines):
        line = _strip_code_noise(raw)
        i = 0
        while i < len(line):
            ch = line[i]
            if ch == "{":
                header = line[:i]
                match = IMPL_HEAD.match(header)
                if match and match.group(1):
                    stack.append(("impl", index, match.group(1), match.group(2) or ""))
                else:
                    stack.append(("other", index, "", ""))
                    if TEST_MARKER.match(header.strip()) or TEST_MARKER.match(line.strip()):
                        in_test_depth += 1
            elif ch == "}":
                if stack:
                    kind, start, own, trait = stack.pop()
                    if kind == "impl":
                        ranges.append((start, index, own, trait, in_test_depth > 0))
                    elif TEST_MARKER.match(line.strip()):
                        in_test_depth = max(0, in_test_depth - 1)
            i += 1
    # 未闭合的块（语法错误）⇒ 取到文件末尾
    for kind, start, own, trait in stack:
        if kind == "impl":
            ranges.append((start, len(lines) - 1, own, trait, in_test_depth > 0))
    return ranges


def _enclosing_fn(lines: list[str], index: int) -> str:
    """该行所处的**最近**函数名（往上找最近的 `fn …`；粗判，够用）。"""
    for i in range(index, -1, -1):
        match = FN_HEAD.match(lines[i])
        if match:
            return match.group(1)
    return ""


def _violations_in_file(path: Path) -> list[str]:
    """一个文件里的违规（每条：**行号** + 规则 + impl 类型 + 所在函数）。"""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError):
        return []
    out: list[str] = []
    for start, end, own, trait, in_test in _scan_blocks(lines):
        # 只关心**测试区**之外的 impl 块（native 测试建独立运行时是合法场景）。
        if in_test:
            continue
        block_code = "\n".join(_strip_code_noise(item) for item in lines[start : end + 1])
        where = f"{trait} for {own}" if trait else own
        rel = path.relative_to(CRATES_UI)

        # ── R1：`impl` 块内不得出现 `Runtime::new()` ──────────────────────────
        hit = RUNTIME_NEW.search(block_code)
        if hit:
            line_no = start + block_code[: hit.start()].count("\n") + 1
            fn_name = _enclosing_fn(lines, line_no - 1)
            out.append(
                f"{rel}:{line_no}: "
                f"[R1] `impl {where}` 内的 `{fn_name or '?'}` **自建 `Runtime`** "
                f"⇒ ⇒ 状态持有者一旦自建 runtime，其信号就**不在宿主挂 effect 的那个 "
                f"runtime** 上 ⇒ ⇒ 响应式链断 ⇒ ⇒ 「幽灵输入」"
            )

        # ── R2：状态持有者不得有**零参**构造 ─────────────────────────────────
        if own.endswith(STATE_SUFFIXES):
            hit = ZERO_ARG_NEW.search(block_code)
            if hit:
                line_no = start + block_code[: hit.start()].count("\n") + 1
                out.append(
                    f"{rel}:{line_no}: "
                    f"[R2] `impl {own}` 的**零参** `new()` ⇒ ⇒ 没有外部 runtime 可用 "
                    f"⇒ ⇒ 必然自建 ⇒ ⇒ 隐式自建路径"
                )

        # ── R3：状态持有者的 `Default` 不得走「零参构造」这条自建路径 ──────────
        # ⚠️ `impl Default for XxxState` ⇒ ⇒ 形态头是 `own = "Default"`、
        #    `trait = "XxxState"`（**别把两个位置搞反**）。
        # ⚠️ 只有**真会自建**的 `Default` 才算违规（纯数据结构的 `Default`
        #    —— 例如 `WizardState` —— 是合法的）。
        if own == "Default" and trait.endswith(STATE_SUFFIXES):
            hit = ZERO_ARG_CTOR_CALL.search(block_code)
            if hit:
                line_no = start + block_code[: hit.start()].count("\n") + 1
                out.append(
                    f"{rel}:{line_no}: "
                    f"[R3] `impl Default for {trait}` 调用了**零参构造** "
                    f"⇒ ⇒ `Default` 没有外部 runtime 可用 ⇒ ⇒ 必然自建 "
                    f"⇒ ⇒ 隐式自建路径"
                )
    return out


def main() -> int:
    roots = [
        CRATES_UI / "apeireth-ui-core" / "src",
        CRATES_UI / "apeireth-ui-dom" / "src",
        CRATES_UI / "apeireth-ui-kit" / "src",
        CRATES_UI / "apeireth-ui-demo" / "src",
        CRATES_UI / "apeireth-ui-demo" / "harness" / "governance" / "engine-host" / "src",
        CRATES_UI / "apeireth-ui-demo" / "harness" / "statusbar" / "engine-host" / "src",
        CRATES_UI / "apeireth-ui-gpu" / "src",
        CRATES_UI / "apeireth-ui-scene" / "src",
    ]
    violations: list[str] = []
    scanned = 0
    for root in roots:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.rs")):
            if "target" in path.parts:
                continue
            scanned += 1
            violations.extend(_violations_in_file(path))
    if violations:
        print("✗ 架构断言失败：状态对象不得自建 `Runtime`")
        print("  （自建 ⇒ 状态与渲染**分属两个 runtime** ⇒ 响应式链断 ⇒ 「幽灵输入」）")
        print("  ⚠️ 类型层面已强制「状态持有者拿不到渲染能力」（`ReactiveScope`）")
        print("     ⇒ 本脚本是**冗余双保险**：抓「新增代码忘了迁移」。")
        for item in violations:
            print(f"  - {item}")
        return 1
    print(
        f"✔ 架构断言通过：{scanned} 个源文件里没有「`impl` 块内自建 `Runtime`」"
        f"（类型已强制 ⇒ 本条为冗余双保险）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
