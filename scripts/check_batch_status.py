#!/usr/bin/env python3
"""**批次进度页守卫**（`scripts/check_batch_status.py`）—— 让「做到哪了」**不可能手写漂移**。

# 漏洞（2026-10-28 审计认定 H7）

设计稿 §8 的批次计划**零守卫** ⇒ 进度页可以整天不更新、或写着与代码不符的结论
⇒ ⇒ 实测事故：进度漂了**一整天没人知道**（我因此误判已有模块不存在、误判已接生产的能力未接线）。

# 修法：**自动生成块**

进度页 `docs/status/spec-batch-status.md` 里必须有一段**机器生成**的「事实块」：

```
<!-- BEGIN:auto-facts -->
…本脚本生成，别手改…
<!-- END:auto-facts -->
```

* `--gen` 重算事实并**重写该块**；
* `--check` 重算并比对 ⇒ **不一致即失败**（页面必须有 `--gen` 后的内容）。

⇒ ⇒ 于是「页面上写着的关键事实」**永远来自代码**，手写的散文无法覆盖它。
⇒ ⇒ 手写的部分仍然自由（判断、理由、边界），但**事实值**被钉住。

# 事实清单（全部是**便宜的 grep/读文件**，不跑 cargo）

见 `FACTS`：每条都有 `key` / 说明 / 判定函数。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs" / "status" / "spec-batch-status.md"
CLAIMS = ROOT / "docs" / "status" / "claims.yaml"
LEDGER = ROOT / "scripts" / "d4-ledger.json"

BEGIN = "<!-- BEGIN:auto-facts -->"
END = "<!-- END:auto-facts -->"


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def fact_d4_ledger() -> dict[str, str]:
    """B0-c 候选池台账的规模（来自 `scripts/d4-ledger.json`）。"""
    if not LEDGER.exists():
        return {"d4_modules": "台账缺失", "d4_candidates": "台账缺失", "d4_measured": "台账缺失"}
    data = json.loads(LEDGER.read_text(encoding="utf-8"))
    return {
        "d4_modules": str(data.get("module_count", "?")),
        "d4_candidates": str(data.get("candidate_count", "?")),
        "d4_measured": str(data.get("measured_count", "?")),
        "d4_not_measurable": str(data.get("not_measurable_count", "?")),
    }


def fact_hash_chain() -> str:
    """D0 哈希链是否**真的在写入路径**上。"""
    text = read(ROOT / "crates/engine/memory/src/backend/sqlite.rs")
    return "在写入路径" if "put_chained_episode" in text else "**不在写入路径**"


def fact_d4_default_off() -> str:
    """D4「认知体操」是否**默认关**（`APEIRETH_ENABLE_ABSORPTION_INSIGHT`）。"""
    text = read(ROOT / "crates/adapters/cli/src/lib.rs")
    has_env = "APEIRETH_ENABLE_ABSORPTION_INSIGHT" in text
    # 默认关的判据：读环境变量后与 "1" 比较（而不是默认 true）
    default_off = bool(re.search(r'APEIRETH_ENABLE_ABSORPTION_INSIGHT[\s\S]{0,200}== "1"', text))
    if has_env and default_off:
        return "生产已接 · **默认关**"
    if has_env:
        return "生产已接 · **默认开（可疑）**"
    return "**未见开关**"


def fact_file(rel: str) -> str:
    return "存在" if (ROOT / rel).exists() else "**缺失**"


def fact_cli_dispatch(name: str) -> str:
    """`apeireth memory <name>` 是否在生产分派里。"""
    text = read(ROOT / "crates/adapters/cli/src/main.rs")
    return "已分派" if f'Some("{name}")' in text else "**未分派**"


def fact_contracts() -> str:
    """§5 三接口契约是否都在（trait/结构体）。"""
    text = (
        read(ROOT / "crates/engine/memory/src/memory_view.rs")
        + read(ROOT / "crates/engine/memory/src/injection_renderer.rs")
    )
    wanted = ["pub trait MemoryView", "pub struct MemoryAnnotation", "pub trait InjectionRenderer"]
    present = [item for item in wanted if item in text]
    return f"{len(present)}/3" if present else "**0/3**"


def fact_claims() -> str:
    text = read(CLAIMS)
    total = len(re.findall(r"^\s*- id:", text, re.M))
    defer = len(re.findall(r"^\s+verify_defer_reason:", text, re.M))
    skip = len(re.findall(r"^\s+verify_skip_reason:", text, re.M))
    return f"{total} 条（延后 {defer} · 豁免 {skip}）"


def fact_presence(rel: str) -> str:
    return "有" if (ROOT / rel).exists() else "**无**"


#: 事实清单：(key, 人话说明, 取值函数)
FACTS: list[tuple[str, str, callable]] = [
    ("d0_hash_chain", "D0 保真：哈希链", fact_hash_chain),
    ("d4_default", "D4 姿态（默认关？）", fact_d4_default_off),
    ("d4_modules", "B0-c 候选池模块数", lambda: fact_d4_ledger()["d4_modules"]),
    ("d4_candidates", "B0-c 可测候选数", lambda: fact_d4_ledger()["d4_candidates"]),
    ("d4_measured", "B0-c 已测行数", lambda: fact_d4_ledger()["d4_measured"]),
    ("d4_not_measurable", "B0-c 测法无效行数", lambda: fact_d4_ledger()["d4_not_measurable"]),
    ("b1_memory_verify", "B1 `memory verify` CLI", lambda: fact_file("crates/adapters/cli/src/memory_verify_command.rs")),
    ("b4_offline_archive", "B4 脱机归档模块", lambda: fact_file("crates/engine/memory/src/offline_archive.rs")),
    ("b4_handover", "B4 handover 模块", lambda: fact_file("crates/engine/memory/src/handover.rs")),
    ("b4_migrate_rollback", "B4 迁移回滚模块", lambda: fact_file("crates/engine/memory/src/migrate_rollback.rs")),
    ("cli_export", "`memory export` 分派", lambda: fact_cli_dispatch("export")),
    ("cli_handover", "`memory handover` 分派", lambda: fact_cli_dispatch("handover")),
    ("b2_contracts", "B2 三接口契约", fact_contracts),
    ("b3_shadow_run", "B3 影子双跑模块", lambda: fact_file("crates/engine/memory/src/shadow_run.rs")),
    ("b0a_adapter", "B0-a PerLTQA 适配器", lambda: fact_file("crates/engine/memory/src/perltqa_adapter.rs")),
    ("b0a_reproduce", "B0-a `reproduce.sh`", lambda: fact_presence("scripts/reproduce.sh")),
    ("claims", "状态声明", fact_claims),
]


def compute() -> dict[str, str]:
    out: dict[str, str] = {}
    for key, _desc, func in FACTS:
        try:
            out[key] = str(func())
        except Exception as error:  # noqa: BLE001 - 守卫本身不许因单个事实崩掉
            out[key] = f"<算出错：{type(error).__name__}>"
    return out


def render_block(values: dict[str, str]) -> str:
    lines = [
        BEGIN,
        "<!-- 本块由 `python3 scripts/check_batch_status.py --gen` 生成；**别手改**（改了 --check 会失败） -->",
        "",
        "| 事实 | 值 |",
        "|---|---|",
    ]
    for key, desc, _ in FACTS:
        lines.append(f"| `{key}` · {desc} | **{values.get(key, '?')}** |")
    lines.append("")
    lines.append(END)
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true", help="门禁：事实块与代码一致")
    group.add_argument("--gen", action="store_true", help="重算并重写事实块")
    args = parser.parse_args()

    if not PAGE.exists():
        print(f"FAIL: 进度页不存在（{PAGE.relative_to(ROOT)}）", file=sys.stderr)
        return 1
    text = PAGE.read_text(encoding="utf-8")
    values = compute()
    block = render_block(values)

    if args.gen:
        if BEGIN in text and END in text:
            new_text = re.sub(
                re.escape(BEGIN) + r".*?" + re.escape(END),
                lambda _match: block,
                text,
                flags=re.S,
            )
        else:
            # 没有块 ⇒ 插到第一个二级标题之前（保持它在页面前部）
            anchor = re.search(r"^## ", text, re.M)
            insert_at = anchor.start() if anchor else 0
            new_text = text[:insert_at] + block + "\n\n" + text[insert_at:]
        PAGE.write_text(new_text, encoding="utf-8")
        print(f"✓ 已重写事实块（{len(values)} 条事实）→ {PAGE.relative_to(ROOT)}")
        for key, desc, _ in FACTS:
            print(f"  · {desc}: **{values[key]}**")
        return 0

    # --check
    if BEGIN not in text or END not in text:
        print(
            "FAIL: 进度页缺自动事实块 ⇒ 跑 `python3 scripts/check_batch_status.py --gen`",
            file=sys.stderr,
        )
        return 1
    current_block = re.search(re.escape(BEGIN) + r".*?" + re.escape(END), text, flags=re.S)
    assert current_block is not None
    if current_block.group(0).strip() == block.strip():
        print(f"OK: 批次进度页事实块与代码一致（{len(values)} 条事实）")
        return 0

    # 逐条报差异（便于修，而不是让人 diff）
    on_page = current_block.group(0)
    def normalize(value: str) -> str:
        """比对前统一去掉强调标记与空白。

        ⚠️ 踩过的坑：只对**页面值**做 `strip("*")` 而**不给**代码值做
        ⇒ 含 `**` 的值（如 `生产已接 · **默认关**`）会被误报成不一致
        ⇒ ⇒ 守卫的**假阳性**同样有害（会训练人忽略告警）⇒ 两边同一口径。
        """
        return re.sub(r"[*\s]+", " ", value).strip()

    diffs: list[str] = []
    for key, desc, _ in FACTS:
        row = re.search(rf"\| `{re.escape(key)}`[^|]*\| ([^|]+)\|", on_page)
        page_value = row.group(1) if row else "<页面上没有这一行>"
        if normalize(page_value) != normalize(values[key]):
            diffs.append(
                f"{desc}（`{key}`）：页面写 `{page_value.strip()}` · 代码事实 `{values[key]}`"
            )
    print("FAIL: 批次进度页事实块**与代码不一致**（页面过期）", file=sys.stderr)
    for line in diffs:
        print(f"  · {line}", file=sys.stderr)
    print(
        "\n修法：跑 `python3 scripts/check_batch_status.py --gen`（**只重写事实块**，"
        "手写的判断与理由不动）。",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
