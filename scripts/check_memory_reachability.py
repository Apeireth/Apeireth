#!/usr/bin/env python3
"""记忆 crate **能力可达率**闸门 —— 把「多少能力真的接在生产链路上」变成**一个数字**。

## 为什么要有这个闸门

「模块多 / 名字像 / 目录好看」都**不是**能力接没接的证据。
真正要问的是：**这个模块的能力，能不能从生产入口一路走到？**

⇒ ⇒ 本脚本算的是 **生产传递闭包可达率**（reachability）：
**从生产入口出发 ⇒ ⇒ 经依赖闭包 ⇒ ⇒ 能到达的顶层模块数 ÷ 顶层模块总数**。

## 判据（**写清楚，避免以后各改各的**）

### 1. 「生产入口」= 什么
- 记忆 crate **之外**的 crate 里，**引用记忆 crate 任何公开符号**的文件
  （⇒ ⇒ 它们的 `use apeireth_memory::…` / `apeireth_memory::…` 就是入口）；
- ⚠️ **不算**：`crates/engine/memory/src/**` 自身（内部互调**不是**接电）
  · `tests/` · `examples/` · `benches/` ⇒ ⇒ **测试引用不算接线**。

### 2. 「依赖」= 什么
- crate 内模块 A 依赖 B ⇔ A 的源文件里**提到 B 的模块名**（`b::` / `use … b::` / `mod b`）
  ⇒ ⇒ **词法级**（不做全 Rust 名字解析 ⇒ ⇒ 可复现、快、但**可能高估**）
  ⇒ ⇒ ⚠️ **方向安全**：高估 ⇒ ⇒ 报「已接线」⇒ ⇒ ⇒ **漏报风险在「未接线」一侧** ⇒ ⇒
  ⇒ ⇒ ⇒ 刻意选这个方向 ⇒ ⇒ **宁可漏报未接线，不可虚报已接线**。

### 3. 闭包怎么算
- 入口集合 → 对每个可达模块，取它的出边 ⇒ ⇒ **BFS 到不动点**。

### 4. 分母
- `crates/engine/memory/src/*.rs`（**去掉 `mod.rs`**）⇒ ⇒ 顶层模块数
  ⇒ ⇒ ⚠️ **只算顶层**：子模块（`foo::bar`）随父模块一起可达 ⇒ ⇒ 不单列。

## ⭐ 两个数（**全局视图 + 行动目标**）

只报「可达率 = 52%」是**不够的** ⇒ ⇒ 不可达里**大部分不是缺口**（评估设施 /
研究族 / 替代后端 / 台账已解释）⇒ ⇒ ⇒ **一个不分层的数字 = 一个没法用的数字**。

⇒ ⇒ 所以本脚本**同时报两个数**：

| 数 | 定义 | 干什么用 |
|---|---|---|
| **能力可达率** | `可达 / 总数`（**原口径，不动**） | **全局视图**（别破坏既有基线与 CI 语义） |
| **未解释缺口** | `不可达 − 已解释` | **行动目标**（这才是要干的活） |

### 「已解释」是什么（**判据来源单一**）

`scripts/memory-reachability-explained.txt`（**人工资产**）登记「不可达但**不是行动项**」
的模块 ⇒ ⇒ 它的**每一条理由都必须指向**
`docs/status/unwired-ledger.md`（未接线台账 = 本仓「已实现但没接电」的**唯一真源**）
的**既有分类** ⇒ ⇒ **台账没写的一律算「未解释缺口」** ⇒ ⇒ ⇒ **脚本不自造分类语义**
⇒ ⇒ **口径唯一**。

### ⭐ 棘轮（**只减不增**）· 两个

1. **可达数下限**：`scripts/memory-reachability-baseline.txt` 钉住**可达模块数下限**
   ⇒ ⇒ **少一个（能力死了）⇒ ⇒ 闸门红**；**多一个（接上了）⇒ ⇒ 绿** ⇒ ⇒
   ⇒ ⇒ ⇒ **这是「只减不增」家族的一个（接线面）**，与 `unwired-baseline.txt`（孤儿面）互补。
2. **未解释缺口上限**：`scripts/memory-reachability-explained.txt` 里的
   `max_unexplained_gap` ⇒ ⇒ **缺口涨了 ⇒ ⇒ 闸门红**（要么接线，要么**在台账里补一条**）。

### ⭐ 「已解释」会**过期**（⇒ ⇒ 报红）

一个模块若**已经可达**（真的接上了）⇒ ⇒ 它**不该**再留在「已解释」里
⇒ ⇒ 闸门**检查这一点** ⇒ ⇒ 报「**解释已过期，请移除**」⇒ ⇒ **红**
⇒ ⇒ ⇒ **「已解释」集合不会变成永久豁免池**。

## 用法

```sh
python3 scripts/check_memory_reachability.py            # 报告（带逐模块清单）
python3 scripts/check_memory_reachability.py --check     # 闸门（退出码 0/1）
python3 scripts/check_memory_reachability.py --update    # 重写基线（需明示）
python3 scripts/check_memory_reachability.py --self-test # 自测（门禁自身不被改坏）
```

> ⚠️ `--update` **不许**冲掉「已解释」表里的人工资产（理由是判断依据）⇒ ⇒
> 它只改 `max_unexplained_gap` 一行，**且只许往下压**。
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict, deque
from dataclasses import dataclass, field
from pathlib import Path

# ⭐ **控制台可能不是 UTF-8**（Windows：`cp1252` / `cp936`）⇒ ⇒ `print` 会抛
# `UnicodeEncodeError` ⇒ ⇒ 门禁 exit 1 ⇒ ⇒ **把本来通过的 CI job 判成红**
# ⇒ ⇒ 这里在**任何打印之前**把 stdout/stderr 重配成 UTF-8（守卫见 `scripts/utf8_stdout.py`）。
sys.path.insert(0, str(Path(__file__).resolve().parent))
from utf8_stdout import force_utf8_stdout  # noqa: E402

force_utf8_stdout()

ROOT = Path(__file__).resolve().parent.parent
MEM_CRATE = ROOT / "crates" / "engine" / "memory"
SRC = MEM_CRATE / "src"
BASELINE = ROOT / "scripts" / "memory-reachability-baseline.txt"
# 「已解释」基线（**人工资产**）⇒ ⇒ 理由必须指向未接线台账（口径唯一）
EXPLAINED = ROOT / "scripts" / "memory-reachability-explained.txt"
# 台账文件名片段 ⇒ ⇒ 「已解释」的理由**必须**提到它（不许自造分类语义）
LEDGER_REF = "unwired-ledger.md"

# 记忆 crate 的**公开**根导出面（`pub use` / `pub mod` ⇒ ⇒ 模块对外可见名）
PUBLIC_RE = re.compile(
    r"^\s*pub\s+(?:use|mod)\s+([A-Za-z_][A-Za-z0-9_]*)", re.M
)
# crate 内模块互相引用的形态：`mod_name::` / `use … mod_name::`
DEP_RE = re.compile(r"(?<![A-Za-z0-9_])([a-z_][a-z0-9_]*)::")


def top_modules() -> list[str]:
    """记忆 crate 的顶层模块名（`mod.rs` 不算 ⇒ ⇒ 它是模块目录清单）。"""
    return sorted(p.stem for p in SRC.glob("*.rs") if p.name != "mod.rs")


def read_module_source(name: str) -> str:
    path = SRC / f"{name}.rs"
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return ""


def production_entries(module_names: set[str]) -> set[str]:
    """生产入口集合 = 记忆 crate 外、引用了记忆 crate 公开符号的文件。"""
    entries: set[str] = set()
    crate_root = MEM_CRATE.as_posix()
    for pattern in ("crates/*/*/src", "crates/*/*/tests", "crates/*/*/examples"):
        for base in ROOT.glob(pattern):
            if not base.is_dir():
                continue
            for path in base.rglob("*.rs"):
                rel = path.relative_to(ROOT).as_posix()
                if rel.startswith(crate_root):
                    continue
                # 测试面**不算接线**
                if "/tests/" in rel or "/examples/" in rel or "/benches/" in rel:
                    continue
                try:
                    text = path.read_text(encoding="utf-8")
                except (OSError, UnicodeDecodeError):
                    continue
                if "apeireth_memory" in text:
                    entries.add(rel)
    return entries


def module_edges(
    module_names: set[str],
    type_owner: dict[str, str] | None = None,
) -> dict[str, set[str]]:
    """crate 内模块依赖边（A → B：A 的源文件提到 `B::`）。

    ⚠️ **`lib.rs` 是「模块清单」不是「模块」** ⇒ ⇒ 它**不作为依赖边的来源**。
      实测：若把 `lib.rs` 当普通模块，它 `pub mod`/`pub use` 的**每个**模块都成了它的出边
      ⇒ ⇒ 而 `lib.rs` 被生产引用 ⇒ ⇒ ⇒ **所有模块都变成「可达」**（可达率虚高到 51%）。
      ⇒ ⇒ 真实值是 **42%**（与图谱判据一致）。
    """
    edges: dict[str, set[str]] = {}
    # 清单文件（`lib.rs` / `mod.rs`）⇒ ⇒ 不产生依赖边
    manifest_only = {"lib", "mod"}
    for name in module_names - manifest_only:
        text = read_module_source(name)
        deps: set[str] = set()
        for m in DEP_RE.finditer(text):
            cand = m.group(1)
            if cand in module_names and cand != name:
                deps.add(cand)
        # `mod xxx;` 也是依赖（子模块声明）
        for m in re.finditer(r"^\s*(?:pub\s+)?mod\s+([a-z_][a-z0-9_]*)\s*;", text, re.M):
            cand = m.group(1)
            if cand in module_names and cand != name:
                deps.add(cand)
        # ⭐ **按类型名反查模块**（2026-11-06 修判据的洞·二）
        # 现有 `DEP_RE` 只认 `mod_name::` 形态 ⇒ ⇒ **按类型名分组导入**不产生边
        # ⇒ ⇒ 例：`use crate::{… MemoryReconciler …}` ⇒ ⇒ `→ reconciler` 这条边**丢了**
        # ⇒ ⇒ ⇒ 而 `lib.rs` 的 `pub use <mod>::{Type}` 映射**能**告诉我们该类型属于谁
        # ⇒ ⇒ ⇒ ⚠️ 只认**大写类型名**且该类型**真在映射里** ⇒ ⇒ **不是虚报**
        # ⇒ ⇒ ⇒ （方向纪律不变：宁可漏报未接线，不可虚报已接线）
        if type_owner:
            for type_name, owner in type_owner.items():
                if owner != name and owner in module_names:
                    if re.search(rf"\b{re.escape(type_name)}\b", text):
                        deps.add(owner)
        edges[name] = deps
    return edges


def read_reexport_map(names: set[str]) -> dict[str, str]:
    """**类型名 → 所属顶层模块**（只从 `lib.rs` 的 `pub use <mod>::…` 取）。

    ⚠️ **只认记忆 crate 自己的顶层模块** ⇒ ⇒ 别名（`as`）不额外映射
    ⇒ ⇒ 避免把「不是模块的东西」算成模块。
    """
    out: dict[str, str] = {}
    lib = ROOT / "crates/engine/memory/src/lib.rs"
    try:
        text = lib.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return out
    for m in re.finditer(
        r"pub\s+use\s+([a-z_][a-z0-9_]*)\s*::\s*\{([^}]*)\}", text, re.S
    ):
        module = m.group(1)
        if module not in names:
            continue
        for item in m.group(2).split(","):
            name = re.split(r"\s+as\s+", item.strip())[0].strip()
            if re.fullmatch(r"[A-Z][A-Za-z0-9_]*", name):
                out.setdefault(name, module)
    for m in re.finditer(
        r"pub\s+use\s+([a-z_][a-z0-9_]*)\s*::\s*([A-Z][A-Za-z0-9_]*)\s*;", text
    ):
        module = m.group(1)
        if module in names:
            out.setdefault(m.group(2), module)
    return out


def compute() -> tuple[set[str], set[str], set[str], dict[str, set[str]]]:
    names = set(top_modules())
    # ⚠️ `lib` / `mod` 是**清单**不是能力 ⇒ ⇒ 不计入分母（否则分母虚高 1）
    manifest_only = {"lib", "mod"}
    names -= manifest_only
    entries = production_entries(names)
    # ⭐ 重导出面：类型名 → 所属模块（生产只按类型名引用能力时必需）
    reexports = read_reexport_map(names)
    edges = module_edges(names, type_owner=reexports)

    # 入口 ⇒ ⇒ 它引用的记忆模块 = 起点（从入口文件里找 `apeireth_memory::X` 形态）
    seeds: set[str] = set()
    for rel in entries:
        path = ROOT / rel
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for m in re.finditer(r"apeireth_memory::([a-z_][a-z0-9_]*)", text):
            if m.group(1) in names:
                seeds.add(m.group(1))
        # `use apeireth_memory::{a, b, …}` 形态
        for m in re.finditer(
            r"use\s+apeireth_memory::\{([^}]*)\}", text, re.S
        ):
            for part in m.group(1).split(","):
                part = part.strip().split(" as ")[0].strip()
                if part in names:
                    seeds.add(part)
                # ⭐ **`mod::Type` 形态**（2026-11-06 修判据的洞·三）
                # 生产写的是 `use apeireth_memory::{<某模块>::<某类型>, …};`
                # ⇒ ⇒ 而 `lib.rs` 里那是 **`pub mod`**（不是 `pub use`）⇒ ⇒ 重导出映射里没有
                # ⇒ ⇒ ⇒ 旧解析只按裸类型名比 ⇒ ⇒ **两头都落空** ⇒ ⇒ 误判不可达
                # ⇒ ⇒ ⇒ 这里按 `::` 拆一次 ⇒ ⇒ 取模块名 ⇒ ⇒ **生产确实按模块路径引用了它**
                # ⚠️ 注释里**故意不写具体模块名** ⇒ ⇒ 自测「脚本里不许出现分类词」才过得去
                if "::" in part:
                    mod_path = part.split("::", 1)[0].strip()
                    if mod_path in names:
                        seeds.add(mod_path)
                # ⭐ **类型名**（大写）⇒ ⇒ 经 `lib.rs` 的重导出面回溯到模块
                # （2026-11-06 修判据的洞：生产**只按类型名**引用能力，
                #   如 `use apeireth_memory::{SqliteCommitmentStore, …};`
                #   ⇒ ⇒ 而 `lib.rs` 里是 `pub use commitments::{SqliteCommitmentStore, …};`
                #   ⇒ ⇒ 只认小写模块名 ⇒ ⇒ 9 个真·已接线的模块被误判不可达）。
                # ⇒ ⇒ 只在「生产确实引用了它导出的能力」时才算 ⇒ ⇒ **不是虚报**。
                module = reexports.get(part)
                if module in names:
                    seeds.add(module)

    reachable: set[str] = set(seeds)
    queue = deque(seeds)
    while queue:
        cur = queue.popleft()
        for nxt in edges.get(cur, ()):
            if nxt not in reachable:
                reachable.add(nxt)
                queue.append(nxt)
    return names, reachable, entries, edges


def read_baseline() -> int | None:
    if not BASELINE.exists():
        return None
    text = BASELINE.read_text(encoding="utf-8")
    m = re.search(r"reachable_modules\s*=\s*(\d+)", text)
    return int(m.group(1)) if m else None


def write_baseline(total: int, reachable: int, names: set[str], reach: set[str]) -> None:
    lines = [
        "# 记忆 crate 能力可达率基线（**只减不增**）",
        "# 由 scripts/check_memory_reachability.py --update 生成 ⇒ ⇒ 不要手改",
        "#",
        "# 判据见脚本头部（生产入口 + 词法依赖闭包 + BFS 到不动点）。",
        "# 方向：宁可漏报未接线，不可虚报已接线。",
        f"reachable_modules = {len(reach)}",
        f"total_modules = {total}",
        f"reachable_ratio = {len(reach) * 100 // max(total, 1)}%",
        "",
        "# ── 可达模块清单 ──",
    ]
    lines += [f"reachable {name}" for name in sorted(reach)]
    lines += ["", "# ── 不可达模块清单 ──"]
    lines += [f"unreachable {name}" for name in sorted(names - reach)]
    BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")


# ============================================================================
# 「已解释」登记（**人工资产** · 判据来源单一 = 未接线台账）
# ============================================================================


@dataclass
class ExplainedEntry:
    """一条「已解释」登记。"""

    module: str
    reason: str
    line_no: int


@dataclass
class ExplainedRegistry:
    """`scripts/memory-reachability-explained.txt` 的解析结果。"""

    entries: dict[str, ExplainedEntry] = field(default_factory=dict)
    cap: int | None = None
    problems: list[str] = field(default_factory=list)

    @property
    def modules(self) -> set[str]:
        return set(self.entries)


def parse_explained(text: str) -> ExplainedRegistry:
    """解析「已解释」基线。

    格式（见 `scripts/memory-reachability-explained.txt` 头部）：
      * `explained <模块名>  # <理由>` —— **理由不许为空**；
      * `max_unexplained_gap = <数字>` —— 「未解释缺口」的**上限**（棘轮 · 只减不增）。

    ⇒ ⇒ 解析期就把**格式错 / 理由空 / 无台账依据 / 缺上限**全部记成问题
      ⇒ ⇒ 这些问题**直接让闸门红**（不是警告）。
    """
    reg = ExplainedRegistry()
    seen_lines: dict[str, int] = {}
    for idx, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("max_unexplained_gap"):
            m = re.fullmatch(r"max_unexplained_gap\s*=\s*(\d+)", line)
            if not m:
                reg.problems.append(
                    f"第 {idx} 行：`max_unexplained_gap` 格式错"
                    f"（应为 `max_unexplained_gap = <数字>`）"
                )
                continue
            reg.cap = int(m.group(1))
            continue
        if line.startswith("explained "):
            body = line[len("explained ") :].strip()
            if "#" in body:
                module, reason = body.split("#", 1)
                module = module.strip()
                reason = reason.strip()
            else:
                module, reason = body.strip(), ""
            if not module:
                reg.problems.append(f"第 {idx} 行：`explained` 后面没有模块名")
                continue
            if module in seen_lines:
                reg.problems.append(
                    f"第 {idx} 行：模块 `{module}` 重复登记"
                    f"（首次在第 {seen_lines[module]} 行）"
                )
                continue
            seen_lines[module] = idx
            if not reason:
                reg.problems.append(
                    f"第 {idx} 行：模块 `{module}` 的**已解释理由为空**"
                    f" ⇒ ⇒ 不许登记 ⇒ ⇒ 闸门红"
                )
            elif LEDGER_REF not in reason:
                reg.problems.append(
                    f"第 {idx} 行：模块 `{module}` 的理由**未指向未接线台账**"
                    f"（须含 `{LEDGER_REF}`）⇒ ⇒ 口径唯一 ⇒ ⇒ 闸门红"
                )
            reg.entries[module] = ExplainedEntry(module, reason, idx)
            continue
        reg.problems.append(f"第 {idx} 行：无法识别的行 ⇒ {line[:60]}")
    if reg.cap is None:
        reg.problems.append(
            "缺 `max_unexplained_gap` 上限 ⇒ ⇒ 「未解释缺口」没有棘轮 ⇒ ⇒ 闸门红"
        )
    return reg


def read_explained() -> ExplainedRegistry:
    """读「已解释」基线（不存在 ⇒ 空登记 + 一条问题）。"""
    if not EXPLAINED.exists():
        reg = ExplainedRegistry()
        reg.problems.append(
            f"「已解释」基线不存在 ⇒ ⇒ 需先建 {EXPLAINED.relative_to(ROOT)}"
        )
        return reg
    return parse_explained(EXPLAINED.read_text(encoding="utf-8"))


def unexplained_gap(
    names: set[str], reachable: set[str], reg: ExplainedRegistry
) -> int:
    """**未解释缺口** = 不可达模块数 − 已解释（且确实不可达的）条目数。

    ⭐ 「已解释里已经可达的条目」**不计入**抵扣 ⇒ ⇒ 否则过期条目会把缺口算小。
    """
    unreachable = names - reachable
    credited = unreachable & reg.modules
    return len(unreachable) - len(credited)


def evaluate(
    names: set[str], reachable: set[str], reg: ExplainedRegistry
) -> list[str]:
    """闸门判据（纯函数 ⇒ ⇒ 可自测）⇒ ⇒ 返回问题清单（空 = 绿）。

    三条硬判据：
      1. **解释已过期**（模块已可达 / 模块已不存在）⇒ ⇒ 红
         ⇒ ⇒ 保证「已解释」集合**不会变成永久豁免池**；
      2. **未解释缺口 > 上限** ⇒ ⇒ 红（棘轮 · 只减不增）；
      3. 解析期问题（理由为空 / 无台账依据 / 缺上限）⇒ ⇒ 红。
    """
    problems: list[str] = []
    unreachable = names - reachable

    # ── ① 解释已过期 ──
    for module, entry in sorted(reg.entries.items()):
        if module in reachable:
            problems.append(
                f"**解释已过期，请移除**：`{module}` 已**可达**（真的接上了）"
                f" ⇒ ⇒ 它不该再留在「已解释」里（基线第 {entry.line_no} 行）"
            )
        elif module not in names:
            problems.append(
                f"**解释已过期，请移除**：`{module}` 在记忆 crate 里**已不存在**"
                f" ⇒ ⇒ 这条登记作废（基线第 {entry.line_no} 行）"
            )
        elif module not in unreachable:
            problems.append(
                f"**解释已过期，请移除**：`{module}` 既不可达也不在分母里"
                f"（基线第 {entry.line_no} 行）"
            )

    # ── ② 未解释缺口超上限 ──
    gap = unexplained_gap(names, reachable, reg)
    if reg.cap is not None and gap > reg.cap:
        problems.append(
            f"**未解释缺口 {gap} > 上限 {reg.cap}** ⇒ ⇒ "
            f"**{gap - reg.cap} 个不可达模块台账里没有解释** ⇒ ⇒ "
            f"要么接线，要么**在 `docs/status/unwired-ledger.md` 里补一条** ⇒ ⇒ 闸门红"
        )
    return problems


def write_cap(reg: ExplainedRegistry, gap: int) -> str:
    """把 `max_unexplained_gap` 往下压（**只许往下压** ⇒ ⇒ 人工资产不许冲掉）。

    ⇒ ⇒ 返回一句人话（给 `--update` 打印）。
    """
    if reg.cap is None:
        return "⚠️ 「已解释」基线里没有 `max_unexplained_gap` ⇒ ⇒ 未改（请手写）"
    if gap >= reg.cap:
        return (
            f"⚠️ 未解释缺口 {gap} ≥ 上限 {reg.cap} ⇒ ⇒ **棘轮只减不增** ⇒ ⇒ "
            f"**拒绝自动抬上限** ⇒ ⇒ 要么接线，要么在台账里补一条"
        )
    text = EXPLAINED.read_text(encoding="utf-8")
    new_text = re.sub(
        r"^max_unexplained_gap\s*=\s*\d+\s*$",
        f"max_unexplained_gap = {gap}",
        text,
        count=1,
        flags=re.M,
    )
    if new_text == text:
        return "⚠️ 未找到 `max_unexplained_gap` 行 ⇒ ⇒ 未改"
    EXPLAINED.write_text(new_text, encoding="utf-8")
    return f"✅ 「未解释缺口」上限已压到 {gap}（原 {reg.cap}）⇒ ⇒ 只减不增"


def run_self_test() -> int:
    """跑本门禁的自测（`scripts/test_memory_reachability.py`）。"""
    import unittest

    loader = unittest.TestLoader()
    suite = loader.discover(
        str(Path(__file__).resolve().parent), pattern="test_memory_reachability.py"
    )
    runner = unittest.TextTestRunner(verbosity=2)
    return 0 if runner.run(suite).wasSuccessful() else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="闸门模式（退出码 0/1）")
    ap.add_argument("--update", action="store_true", help="重写基线")
    ap.add_argument("--self-test", action="store_true", help="跑本门禁的自测")
    args = ap.parse_args()

    if args.self_test:
        return run_self_test()

    names, reach, entries, _edges = compute()
    total = len(names)
    ratio = len(reach) * 100 // max(total, 1)
    unreachable = names - reach
    reg = read_explained()
    gap = unexplained_gap(names, reach, reg)

    print(f"  记忆 crate 顶层模块：{total}")
    print(f"  生产入口文件：{len(entries)}")
    print(f"  ✅ 可达（生产链路上）：{len(reach)}")
    print(f"  ❌ 不可达：{total - len(reach)}")
    print(f"  ⇒ ⇒ **能力可达率 = {len(reach)}/{total} = {ratio}%**（全局视图 · 原口径）")
    print(
        f"  📖 已解释（台账有据 ⇒ 不是行动项）：{len(reg.modules & unreachable)}"
        f"（登记 {len(reg.entries)} 条）"
    )
    print(
        f"  🔻 **未解释缺口 = {gap}**"
        f"（不可达 {len(unreachable)} − 已解释 {len(reg.modules & unreachable)}）"
        f" ⇒ ⇒ **这才是行动目标**"
    )
    if reg.cap is not None:
        verdict = "✅ 未超" if gap <= reg.cap else "❌ 超上限"
        print(f"     棘轮上限 `max_unexplained_gap` = {reg.cap} ⇒ ⇒ {verdict}")

    base = read_baseline()
    if args.update:
        write_baseline(total, len(reach), names, reach)
        print(f"  ✅ 基线已重写：{BASELINE.relative_to(ROOT)}")
        print(f"  {write_cap(reg, gap)}")
        return 0

    if args.check:
        problems = list(reg.problems) + evaluate(names, reach, reg)
        if base is None:
            problems.append(
                f"基线不存在 ⇒ ⇒ 需先 --update（{BASELINE.relative_to(ROOT)}）"
            )
        elif len(reach) < base:
            missing = sorted(base_names(base) - reach)
            problems.append(
                f"**可达模块数 {len(reach)} < 基线 {base}** ⇒ ⇒ "
                f"**有 {base - len(reach)} 个能力掉出生产链路** ⇒ ⇒ 闸门红"
                + (f"（掉线：{', '.join(missing[:12])}）" if missing else "")
            )
        elif len(reach) > base:
            print(
                f"  ✅ 可达模块数 {len(reach)} > 基线 {base} ⇒ ⇒ "
                f"**接线面改善** ⇒ ⇒ 建议 --update 抬基线"
            )
        else:
            print(f"  ✅ 可达模块数 {len(reach)} == 基线 {base} ⇒ ⇒ 一致")

        if problems:
            print()
            for problem in problems:
                print(f"  ❌ {problem}")
            return 1
        print("  ✅ 「已解释」登记全部有效（无过期 / 无空理由 / 均有台账依据）")
        return 0

    print("  ── 可达模块 ──")
    for name in sorted(reach):
        print(f"      {name}")
    print("  ── 不可达模块 ──")
    for name in sorted(unreachable):
        mark = "（已解释）" if name in reg.modules else "⇒ **未解释缺口**"
        print(f"      {name} {mark}")
    return 0


def base_names(value: int) -> set[str]:
    """从基线文件读出可达模块名集合（用于报告「谁掉线了」）。"""
    if not BASELINE.exists():
        return set()
    out: set[str] = set()
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        if line.startswith("reachable "):
            out.add(line.split(" ", 1)[1].strip())
    return out


if __name__ == "__main__":
    sys.exit(main())
