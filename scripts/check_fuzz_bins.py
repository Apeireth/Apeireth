#!/usr/bin/env python3
"""fuzz 目标登记一致性检查器（**把三件靠人眼的事变成机器事实**）。

**为什么要有它（2026-10-26 实战）**：
我集成一个子代理的 `fuzz/Cargo.toml` hunk 时**没核 `path`** ⇒ `stealth_crawler` 的
`[[bin]]` 指向了 `fuzz_targets/spill_access.rs` ⇒ **CI 里那个叫
"stealth_crawler" 的 job 实际跑的是 spill_access** ⇒ 覆盖矩阵**看起来** 39 个
target、**实际上**少一个 ⇒ **静默的覆盖空洞**（最难发现的一类：
CI 全绿、报告好看、实际少测一个面）。

⇒ 三条检查，各对应一类静默空洞：

1. **`name` ↔ `path` 一致**：`[[bin]] name` 必须等于其 `path` 的文件名（去扩展名）。
   不一致 ⇒ **CI 跑的是另一个 target**（本次实测踩到）。
2. **无孤儿文件**：`fuzz/fuzz_targets/*.rs` 每个都必须在 `[[bin]]` 里有登记
   ⇒ 否则**写了但没跑**（写了 target 却忘了进矩阵 = 白写）。
3. **CI 矩阵覆盖**：`fuzz/Cargo.toml` 的每个 `[[bin]]` 都必须出现在
   `.github/workflows/fuzz.yml` 的 matrix 里 ⇒ 否则**登记了但 CI 不跑**
   （`git_write` 就是这样长期躺在仓里没人跑）。

**口径**：只读文件、只报事实；**退出码 1 = 有问题**（供 CI 用）。
**历史不改写**：本检查只看当前文件，不动任何历史记录。

用法:
    python3 scripts/check_fuzz_bins.py            # 报告（有问题则退出码 1）
    python3 scripts/check_fuzz_bins.py --quiet    # 只在有问题时输出
"""
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FUZZ = ROOT / "fuzz"
CARGO = FUZZ / "Cargo.toml"
TARGETS = FUZZ / "fuzz_targets"
WORKFLOW = ROOT / ".github" / "workflows" / "fuzz.yml"

BIN_RE = re.compile(r'\[\[bin\]\]\s*\nname\s*=\s*"([^"]+)"\s*\npath\s*=\s*"([^"]+)"')


def parse_bins(text: str) -> list[tuple[str, str]]:
    return BIN_RE.findall(text)


def parse_matrix(text: str) -> set[str]:
    """从 fuzz.yml 的 matrix 里取出全部 target 名。"""
    names: set[str] = set()
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("target:"):
            continue
        payload = stripped.split(":", 1)[1].strip()
        if payload.startswith("["):
            payload = payload[1:].rsplit("]", 1)[0]
        for item in payload.split(","):
            item = item.strip().strip('"').strip("'")
            if item:
                names.add(item)
    return names


def main() -> int:
    parser = argparse.ArgumentParser(description="fuzz 目标登记一致性检查")
    parser.add_argument("--quiet", action="store_true", help="只在有问题时输出")
    args = parser.parse_args()

    problems: list[str] = []

    cargo_text = CARGO.read_text(encoding="utf-8")
    bins = parse_bins(cargo_text)
    if not bins:
        print("[fuzz-bins] ⚠️ 一个 [[bin]] 都没解析到（Cargo.toml 格式变了？）")
        return 1

    # ① name ↔ path 一致 + 文件存在
    for name, path in bins:
        target = FUZZ / path
        if not target.exists():
            problems.append(f"① [[bin]] {name}: path 不存在 ⇒ {path}")
            continue
        expected = Path(path).stem
        if name != expected:
            problems.append(
                f"① [[bin]] {name}: path 指向 {path} ⇒ **CI 会跑成 {expected}**"
            )

    # ② 无孤儿文件
    registered_paths = {p for _, p in bins}
    for src in sorted(TARGETS.glob("*.rs")):
        rel = f"fuzz_targets/{src.name}"
        if rel not in registered_paths:
            problems.append(f"② 孤儿文件 {rel}: 写了但没有 [[bin]] 登记 ⇒ 不会跑")

    # ③ CI 矩阵覆盖
    matrix = parse_matrix(WORKFLOW.read_text(encoding="utf-8"))
    for name, _ in bins:
        if name not in matrix:
            problems.append(f"③ [[bin]] {name}: **CI 矩阵里没有** ⇒ 登记了但不跑")

    if not problems:
        if not args.quiet:
            print(
                f"[fuzz-bins] ✅ {len(bins)} 个 [[bin]] 全部一致（name↔path / 无孤儿 / CI 矩阵覆盖）"
            )
        return 0

    print(f"[fuzz-bins] ❌ {len(problems)} 处不一致：")
    for line in problems:
        print(f"   {line}")
    print("   → 修完再进 CI（这三类都是**静默**空洞：CI 全绿但覆盖面是假的）")
    return 1


if __name__ == "__main__":
    sys.exit(main())
