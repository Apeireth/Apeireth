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

## 棘轮（**只减不增**）

`scripts/memory-reachability-baseline.txt` 钉住**可达模块数下限**
⇒ ⇒ **少一个（能力死了）⇒ ⇒ 闸门红**；**多一个（接上了）⇒ ⇒ 绿** ⇒ ⇒
⇒ ⇒ ⇒ **这是「只减不增」家族的一个（接线面）**，与 `unwired-baseline.txt`（孤儿面）互补。

## 用法

```sh
python3 scripts/check_memory_reachability.py            # 报告（带逐模块清单）
python3 scripts/check_memory_reachability.py --check     # 闸门（退出码 0/1）
python3 scripts/check_memory_reachability.py --update    # 重写基线（需明示）
```
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict, deque
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MEM_CRATE = ROOT / "crates" / "engine" / "memory"
SRC = MEM_CRATE / "src"
BASELINE = ROOT / "scripts" / "memory-reachability-baseline.txt"

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


def module_edges(module_names: set[str]) -> dict[str, set[str]]:
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
        edges[name] = deps
    return edges


def compute() -> tuple[set[str], set[str], set[str], dict[str, set[str]]]:
    names = set(top_modules())
    # ⚠️ `lib` / `mod` 是**清单**不是能力 ⇒ ⇒ 不计入分母（否则分母虚高 1）
    manifest_only = {"lib", "mod"}
    names -= manifest_only
    entries = production_entries(names)
    edges = module_edges(names)

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


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="闸门模式（退出码 0/1）")
    ap.add_argument("--update", action="store_true", help="重写基线")
    args = ap.parse_args()

    names, reach, entries, _edges = compute()
    total = len(names)
    ratio = len(reach) * 100 // max(total, 1)

    print(f"  记忆 crate 顶层模块：{total}")
    print(f"  生产入口文件：{len(entries)}")
    print(f"  ✅ 可达（生产链路上）：{len(reach)}")
    print(f"  ❌ 不可达：{total - len(reach)}")
    print(f"  ⇒ ⇒ **能力可达率 = {len(reach)}/{total} = {ratio}%**")

    base = read_baseline()
    if args.update:
        write_baseline(total, len(reach), names, reach)
        print(f"  ✅ 基线已重写：{BASELINE.relative_to(ROOT)}")
        return 0

    if args.check:
        if base is None:
            print(f"  ❌ 基线不存在 ⇒ ⇒ 需先 --update（{BASELINE.relative_to(ROOT)}）")
            return 1
        if len(reach) < base:
            print(
                f"  ❌ **可达模块数 {len(reach)} < 基线 {base}** ⇒ ⇒ "
                f"**有 {base - len(reach)} 个能力掉出生产链路** ⇒ ⇒ 闸门红"
            )
            missing = sorted(set(base_names(base)) - reach)
            if missing:
                print(f"      掉线的模块：{', '.join(missing[:12])}")
            return 1
        if len(reach) > base:
            print(f"  ✅ 可达模块数 {len(reach)} > 基线 {base} ⇒ ⇒ **接线面改善** ⇒ ⇒ 建议 --update 抬基线")
            return 0
        print(f"  ✅ 可达模块数 {len(reach)} == 基线 {base} ⇒ ⇒ 一致")
        return 0

    print("  ── 可达模块 ──")
    for name in sorted(reach):
        print(f"      {name}")
    print("  ── 不可达模块 ──")
    for name in sorted(names - reach):
        print(f"      {name}")
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
