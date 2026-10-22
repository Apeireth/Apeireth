#!/usr/bin/env python3
"""孤儿 API 审计：找「库级齐全、生产没接电」的公开符号。

判据（每个 `pub fn` / `pub struct` / `pub trait` 定义）：
- **code 引用数**：在非测试源码里出现的次数（去掉定义行本身）；
- **test 引用数**：在测试代码里出现的次数（`#[cfg(test)]` 之后 + `tests/` 目录 + `*_test.rs`）。

分类：
- `test-only`：code=0 且 test>0 —— 「只在测试里活着的 API」= 大概率没接电；
- `orphan`   ：code=0 且 test=0 —— 死 API（或被宏/反射使用，需人工判）。

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

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "unwired-baseline.txt"
SCAN_ROOTS = [
    "crates/adapters/gateway/src",
    "crates/adapters/cli/src",
    "crates/engine/runtime-assembly/src",
]

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
    """按首个顶层 `#[cfg(test)]` 切成 (生产代码, 测试代码)。"""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if CFG_TEST.match(line):
            return "\n".join(lines[:index]), "\n".join(lines[index:])
    return text, ""


def is_test_file(path: Path) -> bool:
    return "/tests/" in path.as_posix() or path.name.endswith("_tests.rs") or path.name == "tests.rs"


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

    # 单遍分词计数（O(N)）：标识符 → 出现次数。
    token = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
    code_counts: Counter[str] = Counter()
    for text in code_text.values():
        code_counts.update(token.findall(text))
    test_counts: Counter[str] = Counter()
    for text in test_text.values():
        test_counts.update(token.findall(text))

    rows: list[tuple[str, str, str, int, int]] = []  # (kind, symbol, file, code, test)
    for kind, name, rel in declarations:
        # 定义行本身计 1 次；减去它得到「被引用次数」。
        code_hits = code_counts.get(name, 0) - 1
        test_hits = test_counts.get(name, 0)
        if code_hits <= 0:
            rows.append(("test-only" if test_hits > 0 else "orphan", name, str(rel), code_hits, test_hits))

    rows.sort(key=lambda r: (r[0], r[1]))

    baseline: dict[str, str] = {}
    if BASELINE.exists():
        for line in BASELINE.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split(None, 1)
            if len(parts) == 2:
                baseline[parts[1]] = parts[0]

    current = {f"{kind} {name}": f"{rel}" for kind, name, rel, _, _ in rows}

    if args.update:
        lines = [
            "# 孤儿 API 基线（scripts/audit_unwired.py --update 生成；只减不增）",
            "# 每行: <test-only|orphan> <symbol>    # 理由（人工复核时补）",
            "",
        ]
        for key in sorted(current):
            lines.append(key)
        BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"基线已重写: {BASELINE.relative_to(ROOT)}（{len(current)} 条）")
        return 0

    if args.check:
        added = [key for key in current if key not in baseline]
        removed = [key for key in baseline if key not in current]
        if removed:
            print("基线里已消失（可缩表）: " + ", ".join(sorted(removed)[:10]))
        if added:
            print(f"新增孤儿 API {len(added)} 条（必须先接线或写进基线并给理由）:")
            for key in sorted(added):
                print(f"  {key}  ({current[key]})")
            return 1
        print(f"孤儿 API 基线一致（{len(current)} 条，无新增）")
        return 0

    print(f"孤儿 API 报告：test-only / orphan 共 {len(rows)} 条")
    for kind, name, rel, _, _ in rows:
        print(f"  [{kind:9}] {name:44} {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
