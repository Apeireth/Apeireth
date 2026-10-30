#!/usr/bin/env python3
"""校验 `scripts/guard-inventory.json` 与仓库现状**是否同步**（只读，不改任何东西）。

# 为什么要有它

`scripts/guard-inventory.json` 是**机器读的护栏清单** ⇒ 它一旦过期，
下游（`scripts/harness.py` 统一入口、单源契约）就会**拿着过期真相干活**
⇒ 比方说：某个守卫被删了 / 新增了守卫忘了登记 / `blocking` 写错了 ⇒ 门禁漏人。

⇒ 本脚本是那份 JSON 的**守门人**：清单与现实对不上 ⇒ **退出码 1**，并说清楚差在哪。

# 查什么（任一不过 ⇒ exit 1）

1. 清单文件存在、**是合法 JSON**、顶层键齐全（`schema` / `generated_from` / `guards`）；
2. 每条记录字段齐全（`id` / `path` / `kind` / `blocking` / `invocation` /
   `ci_workflows` / `has_baseline` / `notes` / `flags`）；
3. **id 唯一**且是 **kebab-case**；
4. **`blocking` 只允许** `true` / `false` / `"unknown"`；
5. **`kind` 只允许** `guard` / `report` / `ratchet` / `generator` / `unknown`；
6. **`path` 指向的文件还在**（不许指向已删文件）；
7. **发现规则跑一遍**：新守卫必须已登记（不许漏），登记了但已不存在的也要报出来。

# 用法

```sh
python3 scripts/check_guard_inventory.py          # 校验（不改任何文件）
python3 scripts/check_guard_inventory.py --deep   # 额外重算一遍全量记录比对字段
```

`--deep` 会重跑每个守卫的 `--help` 探测（只读、不写文件），用于抓
`kind` / `blocking` / `flags` 的漂移；默认**不跑**（快）。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import gen_guard_inventory as gi  # noqa: E402  （同目录模块：复用同一套发现规则）

INVENTORY_PATH = HERE / "guard-inventory.json"

REQUIRED_TOP_KEYS = ("schema", "generated_from", "guards")
REQUIRED_FIELDS = (
    "id",
    "path",
    "kind",
    "blocking",
    "invocation",
    "ci_workflows",
    "has_baseline",
    "notes",
    "flags",
)
ALLOWED_KINDS = ("guard", "report", "ratchet", "generator", "unknown")
KEBAB_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def check_inventory(problems: list[str]) -> dict | None:
    """读 + 结构校验。返回解析后的对象（失败返回 None）。"""
    if not INVENTORY_PATH.is_file():
        problems.append(f"清单文件不存在: {INVENTORY_PATH.relative_to(ROOT)}")
        return None
    try:
        data = json.loads(INVENTORY_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        problems.append(f"清单不是合法 JSON: {exc}")
        return None
    if not isinstance(data, dict):
        problems.append("清单顶层不是 JSON 对象")
        return None
    for key in REQUIRED_TOP_KEYS:
        if key not in data:
            problems.append(f"清单缺少顶层键: {key}")
    if data.get("schema") != gi.SCHEMA:
        problems.append(f"schema 不对: {data.get('schema')!r} (期望 {gi.SCHEMA!r})")
    if not isinstance(data.get("guards"), list):
        problems.append("guards 不是数组")
        return data
    return data


def check_records(data: dict, problems: list[str]) -> None:
    """逐条记录校验：字段 / id / blocking / kind / path 存在性。"""
    seen_ids: dict[str, str] = {}
    for idx, rec in enumerate(data.get("guards", [])):
        tag = f"guards[{idx}]"
        if not isinstance(rec, dict):
            problems.append(f"{tag}: 不是对象")
            continue
        for field in REQUIRED_FIELDS:
            if field not in rec:
                problems.append(f"{tag}: 缺少字段 {field}")
        gid = rec.get("id")
        if not isinstance(gid, str) or not gid:
            problems.append(f"{tag}: id 不是非空字符串")
        else:
            if not KEBAB_RE.match(gid):
                problems.append(f"{tag}: id 不是 kebab-case: {gid!r}")
            if gid in seen_ids:
                problems.append(f"{tag}: id 重复: {gid!r} (已在 {seen_ids[gid]})")
            else:
                seen_ids[gid] = rec.get("path", "?")
        blocking = rec.get("blocking")
        if not (blocking is True or blocking is False or blocking == "unknown"):
            problems.append(
                f"{tag}: blocking 只能是 true / false / \"unknown\"，实际 {blocking!r}"
            )
        kind = rec.get("kind")
        if kind not in ALLOWED_KINDS:
            problems.append(f"{tag}: kind 只能是 {list(ALLOWED_KINDS)}，实际 {kind!r}")
        path = rec.get("path")
        if not isinstance(path, str) or not path:
            problems.append(f"{tag}: path 不是非空字符串")
        elif not (ROOT / path).is_file():
            problems.append(f"{tag}: path 指向的文件已不存在: {path}")
        notes = rec.get("notes")
        if isinstance(notes, str) and len(notes) > 80:
            problems.append(f"{tag}: notes 超过 80 字 ({len(notes)})")
        for field in ("invocation", "ci_workflows", "flags"):
            val = rec.get(field)
            if not isinstance(val, list) or any(not isinstance(x, str) for x in val):
                problems.append(f"{tag}: {field} 必须是字符串数组")


def check_discovery(data: dict, problems: list[str]) -> None:
    """发现规则跑一遍 ⇒ 清单不能漏、也不能多。"""
    ci_index = gi.ci_command_index()
    discovered = {p.relative_to(ROOT).as_posix() for p in gi.discover(ci_index)}
    listed = {
        rec.get("path")
        for rec in data.get("guards", [])
        if isinstance(rec, dict) and isinstance(rec.get("path"), str)
    }
    for missing in sorted(discovered - listed):
        problems.append(f"清单漏了守卫（发现规则命中，但清单里没有）: {missing}")
    for extra in sorted(listed - discovered):
        problems.append(f"清单多了一条（发现规则没命中）: {extra}")


def check_deep(problems: list[str]) -> None:
    """重算全量记录，比对 kind / blocking / flags 是否漂移。"""
    fresh = {r["path"]: r for r in gi.build_inventory()["guards"]}
    try:
        data = json.loads(INVENTORY_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return
    for rec in data.get("guards", []):
        if not isinstance(rec, dict):
            continue
        path = rec.get("path")
        new = fresh.get(path)
        if new is None:
            continue
        for field in ("kind", "blocking", "has_baseline", "flags"):
            if rec.get(field) != new.get(field):
                problems.append(
                    f"{path}: {field} 漂移了（清单 {rec.get(field)!r} vs 现状 {new.get(field)!r}）"
                )


def main(argv: list[str]) -> int:
    deep = "--deep" in argv[1:]
    problems: list[str] = []

    data = check_inventory(problems)
    if data is not None:
        check_records(data, problems)
        check_discovery(data, problems)
        if deep:
            check_deep(problems)

    if problems:
        print(f"[guard-inventory] OUT OF SYNC: {len(problems)} problem(s)", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print(
            "[guard-inventory] 修法: python3 scripts/gen_guard_inventory.py && "
            "git add scripts/guard-inventory.json",
            file=sys.stderr,
        )
        return 1

    n = len(data.get("guards", [])) if isinstance(data, dict) else 0
    print(f"[guard-inventory] OK: {n} guards, 与仓库现状一致")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
