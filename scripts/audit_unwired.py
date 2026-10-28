#!/usr/bin/env python3
"""孤儿 API 审计：找「库级齐全、生产没接电」的公开符号。

判据（每个 `pub fn` / `pub struct` / `pub trait` 定义）：
- **code 引用数**：在非测试源码里出现的次数（去掉定义行本身）；
- **test 引用数**：在测试代码里出现的次数（`#[cfg(test)]` 之后 + `tests/` 目录 + `*_test.rs`）。

分类（**注意可靠性差异**）：
- `test-only`：code=0 且 test>0 —— 「只在测试里活着的 API」；
- `orphan`   ：code=0 且 test=0。

⚠️ **可靠性分级（别把这个工具当法官）**：

| 判据 | 可靠度 | 说明 |
|---|---|---|
| **code=0**（生产代码零引用） | **高** | 核心判据。已剔除注释/字符串/`use` 导入，保留「作为值传递」（`get(f)`）形态 |
| `test-only` vs `orphan` 的区分 | **低** | `SCAN_ROOTS` 只含 `src/`，**不扫 `tests/`** ⇒ tests 目录里的调用看不见 ⇒ 会把 test-only 报成 orphan。两者都表示 code=0，这是要点；具体哪一类需人工看 |

**已知盲区**（都表现为假阳性，不会漏报 —— 方向安全）：
- 宏展开生成的调用；
- 按名字字符串动态查找（反射式注册）；
- `tests/` 目录与跨 crate 集成测试里的调用。

**使用纪律**：本工具是**发现器**，不是判据。报告出来的每一条都要人核
「它是真的没人调，还是我的判据看不见」。基线里每条必须带理由。

用法：
  python scripts/audit_unwired.py            # 报告（人类可读）
  python scripts/audit_unwired.py --check    # 基线闸（新增孤儿即 exit 1）
  python scripts/audit_unwired.py --update   # 重写基线（需人工复核每条理由）

基线文件：scripts/unwired-baseline.txt（每行 `<kind> <symbol>`；`#` 后是理由）。
纪律同 `arch-baseline.txt`：**只减不增**，新增必须先接线或写清理由。
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from unwired_baseline_common import (  # noqa: E402
    GROUP_DESC,
    SCAN_ROOTS,
    render,
    split_entry,
    usage_counts,
)

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "unwired-baseline.txt"
DECL = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?(?:unsafe\s+)?(fn|struct|trait|enum)\s+([A-Za-z_][A-Za-z0-9_]*)")
CFG_TEST = re.compile(r"^\s*#\[cfg\(test\)\]")


def source_files() -> list[Path]:
    files: list[Path] = []
    for root in SCAN_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        files.extend(sorted(base.rglob("*.rs")))
    return [f for f in files if "target" not in f.parts]


def split_code_test(text: str) -> tuple[str, str]:
    """剔除 `#[cfg(test)]` 修饰的 item，其余都算生产代码。

    **判据 v2 修的第二处缺陷**（2026-10-24）：旧实现「按**首个** `#[cfg(test)]`
    切成两半」是错的 —— 一个文件里 `#[cfg(test)] mod tests { … }` 可能出现在
    **中间**（例如 `production.rs:104`，那个测试模块只占 19 行，
    但旧实现把它之后的 **1709 行全当测试代码**）⇒ 该文件所有生产调用
    都被判为「测试内引用」⇒ **大量假阳性**（已接线的被判成没接电）。

    新实现：按**花括号配对**精确剔除 `#[cfg(test)]` 修饰的整个 item，
    其余原样保留为生产代码。
    """
    lines = text.splitlines()
    code: list[str] = []
    test: list[str] = []
    index = 0
    while index < len(lines):
        if not CFG_TEST.match(lines[index]):
            code.append(lines[index])
            index += 1
            continue
        # 命中 `#[cfg(test)]`：跳过它 + 后续属性行 + 整个 item（花括号配对）
        start = index
        index += 1
        while index < len(lines) and lines[index].lstrip().startswith("#["):
            index += 1
        depth = 0
        started = False
        while index < len(lines):
            line = lines[index]
            depth += line.count("{") - line.count("}")
            if "{" in line:
                started = True
            index += 1
            if started and depth <= 0:
                break
        test.extend(lines[start:index])
    return "\n".join(code), "\n".join(test)


def is_test_file(path: Path) -> bool:
    """**不是生产代码**的文件（测试 / 示例 / 基准）。

    # ⚠️ 为什么必须排除 `examples/` 与 `benches/`（2026-10-28 修正，H1）

    原实现只排 `/tests/` ⇒ ⇒ **示例与基准被当成"生产引用"**
    ⇒ 后果（已实测）：只要往 `examples/` 里写一句 `use some_module::...`，
    该模块就从「整块未接线」审计里**消失** ⇒ 基线"改善" ⇒ ⇒ **假改善**。
    ⇒ ⇒ 修法：把示例/基准一并排除 ⇒ 「接线」只认**生产代码**。

    ⚠️ 已知残余局限：`build.rs`、`#[cfg(test)]` 内联模块、`crates/*/tests.rs`
    之外的测试辅助文件仍可能被算作生产 ⇒ 记为**未解决的边界**（要彻底解决需
    从 CLI / runtime 装配根做可达性分析，属更大改动）。
    """
    posix = path.as_posix()
    if "/tests/" in posix or "/examples/" in posix or "/benches/" in posix:
        return True
    if path.name.endswith("_tests.rs") or path.name == "tests.rs":
        return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="基线闸：新增孤儿即失败")
    parser.add_argument("--update", action="store_true", help="重写基线")
    args = parser.parse_args()

    # 1) 建索引：符号 → 在 code / test 里的引用次数
    code_text = defaultdict(str)
    test_text = defaultdict(str)
    declarations: list[tuple[str, str, Path]] = []  # (kind, name, file)

    for path in source_files():
        rel = path.relative_to(ROOT)
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        if is_test_file(path):
            code_text[str(rel)] = ""
            test_text[str(rel)] = text
        else:
            code, tests = split_code_test(text)
            code_text[str(rel)] = code
            test_text[str(rel)] = tests
            for line in code.splitlines():
                match = DECL.match(line)
                if match and match.group(1) == "fn":
                    declarations.append((match.group(1), match.group(2), rel))

    # 判据 v3（2026-10-24 定稿）。
    #
    # 演进过程（写清楚为什么最后是这个判据，避免以后又改回去）：
    # - **v1（原始）**：裸 token 计数。缺陷：`pub use`/私有 `use`、注释、字符串里的
    #   提及全算「使用」⇒ 把「导出了但没人调」误判成「已接电」；
    # - **v2（本批中间态）**：只认 `name(` / `name::` / `::name` 三种形态。
    #   缺陷：**函数作为值传递**（`get(admin_config_get)`、`map(fn_name)`、
    #   `Arc::new(ctor)`）不算 —— 而路由注册、回调注册大量是这种形态 ⇒ 假阳性；
    # - **v3（现在）**：先**剔除注释 / 字符串 / 全部 `use` 导入行**，
    #   再做裸 token 计数。既排除「只提到名字」，又保留「作为值传递」。
    #
    # 仍有的已知盲区（写下来，不假装完备）：
    # - 宏展开生成的调用（`impl_trait!` 之类）看不到；
    # - 反射 / 动态注册按名字字符串查找的，会漏；
    # - 这些盲区都会表现为**假阳性**（报成未接线），不会漏报 —— 方向安全。
    # 判据 v3 的实现**已提取到共享模块**（`unwired_baseline_common.py`），
    # 与 `audit_unwired_modules.py` 用**同一份** —— 两处各写一遍必然漂移。
    code_counts = usage_counts(code_text.values())
    test_counts = usage_counts(test_text.values())

    rows: list[tuple[str, str, str, int, int]] = []  # (kind, symbol, file, code, test)
    for kind, name, rel in declarations:
        # 定义行本身计 1 次；减去它得到「被引用次数」。
        code_hits = code_counts.get(name, 0) - 1
        test_hits = test_counts.get(name, 0)
        if code_hits <= 0:
            rows.append(("test-only" if test_hits > 0 else "orphan", name, str(rel), code_hits, test_hits))

    rows.sort(key=lambda r: (r[0], r[1]))

    # ⚠️ key 必须与 `--update` 写出的格式**完全一致**（`<kind> <symbol>` 整行）。
    # 原实现这里是 `baseline[parts[1]] = parts[0]`（只用 symbol 作 key），
    # 而 update 写的是 `kind symbol` ⇒ 两边 key 永远不匹配 ⇒ `--check`
    # **永远报「新增 157 条」** ⇒ 这个闸从来没可能通过 ⇒ 也就从来没接进 CI。
    # 「检测未接线的工具自己没接线」，本批一并修。
    baseline: set[str] = set()
    if BASELINE.exists():
        for line in BASELINE.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            # 剥掉行尾的分组标记（`# G3` / `# G3 理由`）——
            # 基线的**条目身份**是 `kind symbol`，标记只是注释。
            entry = line.split("#", 1)[0].strip()
            if entry:
                baseline.add(entry)

    # **基线只记 `orphan`（真零引用）**。
    #
    # `test-only`（生产零引用 + 测试有引用）**不是问题** —— 那是「库的公开 API
    # 被集成测试覆盖」，属正常状态。把它也塞进基线会让数字失真、掩盖真孤儿
    # （2026-10-24 实测：混着记时 892 条，只记 orphan 是 164 条）。
    current = {f"{kind} {name}": f"{rel}" for kind, name, rel, _, _ in rows if kind == "orphan"}

    if args.update:
        # **保留人工资产**：头部说明与每行的**分组标记**都是人写的判断依据。
        # 工具把它们冲掉，等于让每次还债都从零重建分类 —— 而且很可能悄悄丢掉判断依据。
        #
        # 分组信息放**行内**（`# <GROUP>`）而不是靠行序 ⇒ 「谁属于哪组」是数据而非
        # 排版，两个工具（`--update` 与 `group_unwired_baseline.py`）才能输出**逐字一致**，
        # 从而互相验证。
        entries: dict[str, str] = {}
        old_group_of: dict[str, str] = {}
        if BASELINE.exists():
            for raw_line in BASELINE.read_text(encoding="utf-8").splitlines():
                stripped = raw_line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                _, symbol, group, reason = split_entry(stripped)
                if not symbol:
                    continue
                entries[symbol] = reason
                old_group_of[symbol] = group or "G5"

        # 组内顺序：旧文件里的原序在前，新出现的按字母序追加在后
        groups: dict[str, list[str]] = {group: [] for group in GROUP_DESC}
        for symbol, group in old_group_of.items():
            if f"orphan {symbol}" in current:
                groups.setdefault(group, []).append(symbol)
        placed = {symbol for items in groups.values() for symbol in items}
        for key in sorted(current):
            symbol = key.split(None, 1)[1].strip() if len(key.split(None, 1)) > 1 else key.strip()
            if symbol not in placed:
                groups.setdefault("G5", []).append(symbol)  # 新孤儿先进待判组

        BASELINE.write_text(render(entries, groups), encoding="utf-8")
        print(
            f"基线已重写: {BASELINE.relative_to(ROOT)}"
            f"（{len(current)} 条；保留分组标记与人工注释）"
        )
        return 0

    if args.check:
        added = [key for key in current if key not in baseline]
        removed = [key for key in baseline if key not in current]
        if added:
            print(f"新增孤儿 API {len(added)} 条（必须先接线或写进基线并给理由）:")
            for key in sorted(added):
                print(f"  {key}  ({current[key]})")
            return 1
        # **棘轮只减不增的「减」这一半也要机检**：基线里的条目若已经不再孤儿
        # （被接线了），必须从基线删掉 —— 否则基线会永久虚高（把已还的债还记在
        # 账上），而「只减不增」就退化成「不增也不减」。此前这里只打印提示、
        # 仍 exit 0，靠人自觉（我这几轮就手工补删过好几次）。
        if removed:
            print(
                f"基线里有 {len(removed)} 条**已不再孤儿**（应删掉；棘轮只减不增）:"
            )
            for key in sorted(removed):
                print(f"  {key}")
            print(
                "修法：从 scripts/unwired-baseline.txt 删掉这些行"
                "（或 `python3 scripts/audit_unwired.py --update` 重写基线），"
                "然后重跑 `scripts/group_unwired_baseline.py` 保持分组一致。"
            )
            return 1
        print(
            f"孤儿 API 基线一致（{len(current)} 条，无新增、无过期）"
        )
        return 0

    orphans = [r for r in rows if r[0] == "orphan"]
    test_only = [r for r in rows if r[0] == "test-only"]
    print(
        f"孤儿 API 报告：orphan（真零引用）{len(orphans)} 条；"
        f"test-only（被集成测试覆盖，正常）{len(test_only)} 条"
    )
    for kind, name, rel, _, _ in orphans:
        print(f"  [{kind:9}] {name:44} {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
