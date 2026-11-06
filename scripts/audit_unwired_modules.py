#!/usr/bin/env python3
"""模块级「未接线」审计 —— 找**整块**能力没接电的模块。

## 为什么符号级不够

`scripts/audit_unwired.py` 查的是**单个符号**有没有生产调用者。这能抓到
「一个函数没人调」，但抓不到「**整个模块**都没接电」—— 后者的每个符号
都是孤儿，逐条看像是零散小事，加起来却是一整块能力缺失。

实测例子（2026-10-24 第四轮子代理核查时发现的）：

| 模块 | 状态 | 缺的能力 |
|---|---|---|
| `foundation/orchestration/job_board.rs` | 全模块仅集成测试构造 | 并行工具派发、长任务过程输出留存 |
| `engine/runtime` 的 `CallScheduler` | `runtime.rs:719` 恒 `None` | 并发安全白名单的生效路径 |
| `engine/memory/hallways.rs` | 整模块零调用 | 记忆「走廊」结构 |
| `engine/memory/cross_diary.rs` | 整模块零调用 | 跨日记上下文 |

这四条在符号级基线里**看不出来**（它们的 pub API 有的是被别的模块 re-export 的，
有的是 `pub use` 形态，符号级判据看不见「整块都没接」这件事）。

## 判据

一个**模块**（一个 `.rs` 文件，或一个目录下的全部 `.rs`）算「整块未接线」，
当且仅当：

1. 它声明了**至少 1 个** pub API（`pub fn` / `pub struct` / `pub trait` / `pub enum`，
   含 `pub(crate)`）—— 排除纯内部实现文件；
2. 这些 pub API **全部**在生产代码里零引用（复用 `audit_unwired.py` 的判据 v3）；
3. 它**没有**被 `pub use` 重导出到别的 crate（重导出 = 有外部消费者，不算孤立）。

## 用法

```
python3 scripts/audit_unwired_modules.py            # 列出整块未接线的模块
python3 scripts/audit_unwired_modules.py --json     # 机器可读
```

## 已知局限（**它是线索工具，不是判决**）

1. **`use` 引入后直接调用看不见** ——
   `use crate::foo::bar;` 之后调 `bar()`，限定路径 `foo::` 只出现在被剔掉的
   use 行里 ⇒ **会漏报**接电的模块
   （实测 `nervous_snapshot`：经 `self_status_source::nervous_snapshot::{...}`
   引入后调用，模块明明接电却被判孤立）。
2. **方法名撞名会让裸 token 计数偏松** ——
   `new` / `status` / `append` / `label` 全仓几十个同名 ⇒
   靠这些名字撑着的「引用」是假的（⇒ 对**方法**不用这个判据）。
3. **二进制入口不判**（`main.rs` 的 pub 类型是内部实现）。

⇒ 本工具的产出**必须人工过一眼**：它给的是「值得看一眼的线索」，
不是「没接电的结论」。

## 为什么不做成「基线棘轮」

模块级未接线**不是缺陷** —— 一半是「默认关的功能族」（生物机制、IM 通道），
一半是「还没立项的预留」。把它们做成棘轮会逼着人给每个模块编理由，
反而制造台账噪音。所以这个工具**只报告，不拦**；
要拦的话用 `scripts/unwired-baseline.txt` 的分组（G7 = owner 裁定不接）。
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from unwired_baseline_common import ROOT, SCAN_ROOTS, usage_counts  # noqa: E402

import audit_unwired as au  # noqa: E402

# 全仓出现次数 ≤ 这个值的标识符算「独特」（才可能是真信号）。
RARE_MAX = 40


def collect() -> tuple[dict[str, dict], dict[str, str], dict[str, int]]:
    """扫一遍源码。

    返回 ({文件: {符号名: 模块外引用次数}}, {文件: 生产代码文本}, {标识符: 全仓出现次数})。
    """
    code_text: dict[str, str] = {}
    declarations: list[tuple[str, str, str]] = []

    for root in SCAN_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        for path in sorted(base.rglob("*.rs")):
            if "target" in path.parts:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError):
                continue
            rel = str(path.relative_to(ROOT))
            if au.is_test_file(path):
                continue
            code, _ = au.split_code_test(text)
            code_text[rel] = code
            for line in code.splitlines():
                match = au.DECL.match(line)
                if match:
                    declarations.append((match.group(1), match.group(2), rel))

    # 生产代码里出现过的 token（**复用判据 v3 的同一份实现**）
    counts = usage_counts(code_text.values())

    per_file: dict[str, dict] = {}
    for _kind, name, rel in declarations:
        per_file.setdefault(rel, {})[name] = counts.get(name, 0)

    # **只按「类型级」判据**（struct / enum / trait），不看方法。
    #
    # 为什么：方法名**极容易撞名** —— `new` / `status` / `append` / `label` /
    # `update` / `build` 在全仓有几十个同名方法，裸 token 计数会把它们算成
    # 「模块外有人用」⇒ 模块级判定**永远不成立**（实测：`JobBoard` 模块 38 个
    # pub API 里有一半是通用方法名，于是整块未接线被判成「有引用」而漏报）。
    #
    # 类型名足够独特（`JobBoard` / `CrossDiaryIndex` / `Hallway`），
    # 撞名概率极低 ⇒ 判据可用。
    type_decls: dict[str, list[str]] = {}
    for kind, name, rel in declarations:
        if kind in ("struct", "enum", "trait"):
            type_decls.setdefault(rel, []).append(name)

    # **模块内部自调用不算「有人用」** ——
    # 否则 `cross_diary.rs` 里 `links_for_fact` 被同文件的 `render_cross_context`
    # 调用，整个模块看起来「有引用」，实际整块仍是孤立的。
    # 所以每个符号按「**除本文件外**」的引用数判定。
    outside: dict[str, dict] = {}
    for rel, symbols in per_file.items():
        type_names = set(type_decls.get(rel, []))
        outside[rel] = {}
        for name, count in symbols.items():
            own = usage_counts([code_text.get(rel, "")]).get(name, 0)
            # 方法级：只在**不是类型**时保留（供报告展示），判定不看它们。
            outside[rel][name] = count - own if name in type_names else -1
    return outside, code_text, counts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="机器可读输出")
    args = parser.parse_args()

    per_file, code_text, global_counts = collect()
    report = []
    for rel, symbols in sorted(per_file.items()):
        if not symbols:
            continue
        # **二进制入口不判**：`main.rs` 里的 pub 类型是内部实现
        # （CLI 的命令分派全在 `main.rs`），判它们会产出大量假阳性。
        if rel.endswith("/main.rs"):
            continue
        # **判定 = 独特标识符在模块外零引用 且 无私有 `use` 引入**。
        #
        # 「独特」= 该标识符在**全仓**出现次数 ≤ `RARE_MAX`。
        # 为什么必须这样：通用名（`new` / `status` / `len` / `parse` / `settle`）
        # 全仓成百上千个同名 ⇒ 裸计数会把它们算成「模块外有人用」
        # （实测 `new` 出现 5218 次、`settle` 在 job_board 之外也有同名），
        # 于是「整块未接线」的模块被判成「有引用」⇒ **漏报**。
        # 只有「全仓罕见」的标识符才可能是真信号。
        #
        # 两层缺一不可（实测踩过两个坑）：
        #
        # ① **只看类型级会有假阳性**：`nervous_snapshot` 的三个 pub 类型
        #    （`SignalSource` / `NervousMechanism` / `NervousSummary`）外部零引用
        #    —— 但它的**函数**在生产装配里被用（`tool_modules.rs`）。
        #    那三个类型只是实现细节（标了 pub 而已）⇒ 模块明明接电了。
        #    ⇒ 加第二层：**限定路径**（`模块名::`）在模块外出现 ⇒ 模块被整体用过。
        #
        # ② **只看函数级会有假阴性**：方法名极容易撞名
        #    （`new` / `status` / `append` / `label` 全仓几十个同名），
        #    裸 token 计数会把它们算成「模块外有人用」。
        #
        # 类型级（唯一性够）+ 限定路径（能看出「模块被整体用」）⇒ 两层合起来
        # 既不误报接电的模块，也不漏报整块孤立的模块。
        # `symbols` = {名字: 模块外引用次数}；`-1` 表示「方法级，不参与判定」。
        rare = {
            name: count
            for name, count in symbols.items()
            if count >= 0 and global_counts.get(name, 0) <= RARE_MAX
        }
        if not rare:
            continue  # 该模块没有独特标识符（纯通用名模块）⇒ 不判
        unwired = {name: count for name, count in rare.items() if count <= 0}
        if len(unwired) != len(rare):
            continue
        if _qualified_path_used(rel, code_text):
            continue
        # **`pub use`（重导出）只是转发，不是消费** —— `pub use job_board::{...}`
        # 把类型再导出一次，但没人真的用 JobBoard。
        # 反过来，**私有 `use` 才是消费信号**：
        # `use super::self_status_source::nervous_snapshot::{...}` 说明有人在用这个模块。
        private_uses, _public_uses = _use_sites(rel, code_text)
        if private_uses > 0:
            continue
        report.append({"file": rel, "symbols": sorted(unwired)})

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=1))
        return 0

    if not report:
        print("没有整块未接线的模块")
        return 0
    print(f"整块未接线的模块：{len(report)} 个\n")
    for item in report:
        print(f"  {item['file']}  ({len(item['symbols'])} 个 pub API 全零引用)")
        for name in item["symbols"]:
            print(f"      {name}")
    print(
        "\n注：这些不是缺陷 —— 一半是「默认关的功能族」，一半是「还没立项的预留」。\n"
        "    要拦的话用 `scripts/unwired-baseline.txt` 的分组（G7 = owner 裁定不接）。"
    )
    return 0


def _use_sites(rel: str, code_text: dict[str, str]) -> tuple[int, int]:
    """统计别的文件里**私有 use** 与 **pub use** 引入本模块的次数。

    `pub use` = 重导出（转发，**不算消费**）；私有 `use` = 真的在用。
    """
    import re

    module = rel.rsplit("/", 1)[-1].replace(".rs", "")
    # 一个 use 块：**连前面的 `pub` 一起捕获**（否则 `pub use` 会被当成私有 use）。
    # 可跨行（多行 use 声明）。
    use_block = re.compile(r"(?s)\b(?:pub(?:\s*\([^)]*\))?\s+)?use\b[^;]*;")
    module_ref = re.compile(rf"\b{re.escape(module)}\b")
    private = 0
    public = 0
    for other_rel, code in code_text.items():
        if other_rel == rel:
            continue
        for match in use_block.finditer(code):
            body = match.group(0)
            if not module_ref.search(body):
                continue
            # `pub use` / `pub(crate) use` ⇒ 重导出（转发，不算消费）
            if re.match(r"\s*pub(\s*\([^)]*\))?\s+use\b", body):
                public += 1
            else:
                private += 1
    return private, public


def _qualified_path_used(rel: str, code_text: dict[str, str]) -> bool:
    """该模块是否在别处以 `模块名::` 的形式被**真正使用**（说明模块被整体接电）。

    ⚠️ 必须先 `strip_noise`：否则 `pub use foo::{...}` 的重导出与
    doc 注释里的 `[`crate::foo::Bar`]` 都会被当成「有人用」⇒ 漏报整块未接线
    （实测：`job_board::` 只出现在 `lib.rs` 的 `pub use` 与 `job_ring.rs` 的
    文档注释里，两者都不是消费）。
    """
    import re

    from unwired_baseline_common import strip_noise

    module = rel.rsplit("/", 1)[-1].replace(".rs", "")
    pattern = re.compile(rf"\b{re.escape(module)}\s*::")
    for other_rel, code in code_text.items():
        if other_rel == rel:
            continue
        if pattern.search(strip_noise(code)):
            return True
    return False




if __name__ == "__main__":
    raise SystemExit(main())
