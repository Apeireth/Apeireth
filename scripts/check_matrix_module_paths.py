#!/usr/bin/env python3
"""**能力矩阵模块路径守卫**（2026-11-03 加，防「矩阵宣称了不存在的模块」）。

# 为什么需要（**当场抓到 1 处真失真**）
能力矩阵 `docs/03-reference/capabilities-matrix.md` 的每行第 3 列是**模块路径**
（形如 `apeireth-memory::wiki_corpus`）⇒ ⇒ 那是**对外的能力宣称**。
⇒ ⇒ 如果那个模块**在仓里不存在** ⇒ ⇒ **矩阵在对外说谎**。

**实测抓到**：`apeireth-orchestration::prompt_stabilizer` —— 该模块已随
**ADR-0007 / R7-4a**（owner 2026-10-20 裁定）归档到 `legacy/archived/`，
但矩阵仍把它当可用能力宣称（其余 62 个引用**都对得上**）。

# 判据（**精准，无假阳性**）
* 扫**能力矩阵**里的 `apeireth-<crate>::<...>::<module>` 引用 ⇒ 取**最后一段**作模块名；
* 该名字在 `crates/**` 下**必须存在**（同名 `.rs` 文件或同名目录）
  ⇒ ⇒ 归档模块（不在 `crates/` 下）⇒ **判定为失真**；
* ⚠️ **删除线例外**：引用写在 `~~…~~` 里 ⇒ **视为已如实标注**（放行）
  ⇒ ⇒ 修好的旧项**不会**反复报错，而**新**失真会被抓住。

# 为什么只查矩阵（而**不是**全文扫「归档模块是否被提及」）
实测：全文扫会把「**提到**某归档模块」与「**宣称**它可用」混为一谈
⇒ 196 处假阳性（`provider` 等通用名）⇒ ⇒ 「提及」**不可机器判**，
而**能力清单的表格行**是**结构化**的 ⇒ ⇒ **只查它**。

# 用法
```bash
python3 scripts/check_matrix_module_paths.py           # 有失真 ⇒ exit 1
```
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATRIX = ROOT / "docs" / "03-reference" / "capabilities-matrix.md"
CRATES = ROOT / "crates"

# `apeireth-foo::a::b::module`（至少两段）
REFERENCE = re.compile(r"`?apeireth-[a-z0-9\-]+::([a-z0-9_:]+)`?")
DELETED = re.compile(r"~~[^~]*~~")


def live_module_names() -> set[str]:
    names: set[str] = set()
    for path in CRATES.rglob("*.rs"):
        names.add(path.stem)
        names.add(path.parent.name)
    return names


def problems(text: str, live: set[str] | None = None) -> list[str]:
    """给定矩阵文本 ⇒ 返回失真点列表（**可测的纯函数**）。

    ⭐ 抽成函数是为了**能写自测**（门禁自己坏掉 ⇒ 等于把病放回去）。
    """
    if live is None:
        live = live_module_names()
    found: list[str] = []
    for number, line in enumerate(text.splitlines(), 1):
        # 删除线里的引用 ⇒ 已如实标注 ⇒ 放行
        scrubbed = DELETED.sub("", line)
        for match in REFERENCE.finditer(scrubbed):
            module = match.group(1).split("::")[-1]
            if not module:
                continue
            if module not in live:
                found.append(
                    f"{MATRIX.relative_to(ROOT)}:{number}: 模块 `{module}` 在仓里不存在"
                    f"（矩阵这一行宣称了 it ⇒ 对外说谎）".replace("it", "它")
                )
    return found


def main() -> int:
    if not MATRIX.is_file():
        print(f"找不到能力矩阵：{MATRIX}", file=sys.stderr)
        return 1

    live = live_module_names()
    text = MATRIX.read_text(encoding="utf-8")
    # ⚠️ 局部名**不能**叫 `problems`（会遮蔽同名的纯函数 ⇒ UnboundLocalError）
    found = problems(text, live)
    checked = len(REFERENCE.findall(DELETED.sub("", text)))

    if found:
        print(f"FAIL: 能力矩阵宣称了**不存在的模块**（{len(found)} 处 / 共核 {checked} 个引用）")
        for item in found:
            print(f"  · {item}")
        print(
            "\n⚠️ 修法：① 模块确实存在 ⇒ 改矩阵里的**路径**（应写 `crate::module`）；\n"
            "   ② 模块已归档 ⇒ 那一行改成「**⚠️ 已归档（不在生产）**」并把引用写进 `~~…~~`\n"
            "      （删除线 = 已如实标注 ⇒ 本守卫放行）；\n"
            "   ③ 能力压根没有 ⇒ **删掉那一行**（矩阵不该留不存在的能力）。"
        )
        return 1

    print(f"OK: 能力矩阵 {checked} 个模块引用全部对得上仓里的真实模块")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
