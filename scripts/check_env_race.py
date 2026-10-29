#!/usr/bin/env python3
"""**跨测试 env 竞态清单**：找出「改了进程级环境变量、却没与其它测试串行化」的测试。

**为什么要有它（2026-10-27 实战撞了两次）**：
Rust 的 `std::env::set_var` / `remove_var` 改的是**进程级**状态 ⇒
**同一个测试二进制里**任何**另一个测试**都可能同时读到它 ⇒ ⇒
**跨测试污染** ⇒ ⇒ 表现为「**单跑 50 次 0 失败、全量并行时偶发**」。

**今天实测到的两个真实案例**（都不是我引入的，但都让全量门禁偶发红）：
1. `crates/adapters/cli/tests/onboarding.rs`：`activate_escape_hatch_disables_the_whole_profile`
   在**进程级**设 `APEIRETH_DISABLE_ONBOARDING=1` ⇒ 而
   `tuning_state_evolution_reshapes_budgets_during_use` **既不持锁也不清 env**
   ⇒ 并行时看到别人的 env ⇒ 提前返回 ⇒ 断言炸。
2. `crates/engine/memory/src/retrieval_pipeline.rs`：**6 个读侧测试**没持
   `retrieval_tuning::EFFECTIVE_TEST_GUARD` ⇒ 而写侧测试在**另一个文件**里持锁并改全局
   ⇒ ⇒ 候选分数受全局状态影响 ⇒ 并列顺序翻转 ⇒ 断言炸。

**本脚本做什么**：**列出候选清单**（不是判红门禁）——
- 找出所有**含 `set_var` / `remove_var`** 的测试文件；
- 在同一文件里找出**没持任何锁**的测试函数；
- **跨文件**的耦合（如案例 2）用「全局可写状态」提示。

**为什么是「清单」而不是「门禁」**：精确判据需要跨文件的数据流分析 ⇒
**机械判据必然有假阳性** ⇒ ⇒ 与其做一个会误报的���禁（**消警器**），不如给出**可复核的候选清单**，
由人逐条判定 ⇒ ⇒ **诚实边界写在输出里**。

用法:
    python3 scripts/check_env_race.py            # 打印清单
    python3 scripts/check_env_race.py --json     # 机器可读
"""
import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ENV_WRITE_RE = re.compile(r"\b(?:std::)?env::(set_var|remove_var)\s*\(")
# 测试函数：`#[test]` / `#[tokio::test]` 之后的 `fn name(...)`
TEST_ATTR_RE = re.compile(r"#\[(?:tokio::)?test\]")
FN_RE = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-z0-9_]+)\s*\(")
LOCK_RE = re.compile(r"\b(?:[A-Z_]*LOCK[A-Z_]*|[a-z_]*_guard|[a-z_]*_lock)\b|\.lock\(\)")
# 全局可写状态（跨文件耦合的线索）
GLOBAL_STATE_RE = re.compile(
    r"static\s+([A-Z_]+)\s*:\s*(?:std::sync::)?Mutex|static\s+([A-Z_]+)\s*:\s*OnceLock"
)

SKIP_DIRS = {"target", "fuzz", "node_modules", ".git", "legacy", "reports", "artifacts"}


def rust_files() -> list[Path]:
    out = []
    for base in ("crates",):
        for path in (ROOT / base).rglob("*.rs"):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            out.append(path)
    return sorted(out)


def main() -> int:
    parser = argparse.ArgumentParser(description="跨测试 env 竞态清单")
    parser.add_argument("--json", action="store_true", help="机器可读输出")
    args = parser.parse_args()

    findings = []
    global_states = []

    for path in rust_files():
        text = path.read_text(encoding="utf-8", errors="replace")
        lines = text.splitlines()

        # 全局可写状态（Mutex/OnceLock 静态量）⇒ 跨文件耦合线索
        for match in GLOBAL_STATE_RE.finditer(text):
            name = match.group(1) or match.group(2)
            if name:
                global_states.append((str(path.relative_to(ROOT)), name))

        if not ENV_WRITE_RE.search(text):
            continue

        # 逐个测试函数切段
        blocks = []
        current = None
        for index, line in enumerate(lines):
            if TEST_ATTR_RE.search(line):
                current = {"start": index, "name": None, "lines": []}
            if current is not None:
                current["lines"].append((index, line))
                if current["name"] is None:
                    fn = FN_RE.match(line)
                    if fn:
                        current["name"] = fn.group(1)
                # 函数体结束（缩进回到 0 且不是空行）
                elif line.strip() and not line.startswith((" ", "\t")):
                    blocks.append(current)
                    current = None
        if current is not None:
            blocks.append(current)

        for block in blocks:
            body = "\n".join(line for _, line in block["lines"])
            if not ENV_WRITE_RE.search(body):
                continue
            has_lock = bool(LOCK_RE.search(body))
            findings.append({
                "file": str(path.relative_to(ROOT)),
                "line": block["start"] + 1,
                "test": block["name"] or "(?)",
                "has_lock": has_lock,
                "env_calls": len(ENV_WRITE_RE.findall(body)),
            })

    report = {
        "env_writing_tests": findings,
        "unguarded": [f for f in findings if not f["has_lock"]],
        "global_states": sorted({f"{p}:{n}" for p, n in global_states}),
        "note": "清单型（非门禁）⇒ 精确判据需跨文件数据流分析 ⇒ 机械判据必然有假阳性 ⇒ 逐条人工判定",
    }

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0

    print(f"[env-race] 含 env 写（set_var/remove_var）的测试函数: **{len(findings)}** 个")
    unguarded = report["unguarded"]
    print(f"[env-race] 其中**未见同函数内持锁**的: **{len(unguarded)}** 个（候选，需人工判定）")
    for item in unguarded:
        print(f"   · {item['file']}:{item['line']}  fn {item['test']}()  "
              f"（env 写 {item['env_calls']} 处，同函数内未见锁）")
    print()
    print(f"[env-race] 全局可写状态（跨文件耦合线索）: **{len(report['global_states'])}** 个")
    for item in report["global_states"][:20]:
        print(f"   · {item}")
    print()
    print("  ⚠️ **本清单不是门禁** ⇒ 机械判据必然有假阳性（同函数内持锁 ≠ 覆盖全部用例）")
    print("  ⇒ 用途：把「全量并行偶发」的候选**一次暴露**，由人逐条判定 ⇒ 不做自动消警器")
    return 0


if __name__ == "__main__":
    sys.exit(main())
