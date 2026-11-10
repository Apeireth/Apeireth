#!/usr/bin/env python3
"""**任务清单校验器**（非阻塞 · 2026-10-30）。

# 定位

owner 2026-10-30 裁定：任务清单（`.harness/tasks/*.yaml`）**可入仓、非强制、
**不接 required gate**。⇒ 本脚本是**提醒**，不是门禁。

# 它查什么

1. **必填字段**：`task_id` / `summary` / `allowed_paths`；
2. `task_id` 唯一；
3. `allowed_paths` 与 `forbidden_paths` **不得重叠**；
4. `forbidden_paths` **不得**命中契约 `[locked]` 的受锁源码
   ⇒ 真源：`docs/architecture/contract.toml`；
5. `required_checks` 里的 id **必须**存在于契约 `[ci_gates]`；
6. `base_sha` 若给了 ⇒ 必须是**能解析的** git 对象。

# 用法

    python3 scripts/check_task_manifest.py            # 校验全部清单
    python3 scripts/check_task_manifest.py --strict   # 有问题即 exit 1（默认也 exit 1，
                                                      # 但 --strict 额外要求至少有一份清单）
"""

from __future__ import annotations

import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TASKS_DIR = ROOT / ".harness" / "tasks"
CONTRACT = ROOT / "docs" / "architecture" / "contract.toml"
REQUIRED_FIELDS = ("task_id", "summary", "allowed_paths")


def parse_simple_yaml(path: Path) -> dict:
    """读本仓任务清单用的 YAML 子集（顶层标量 + 列表）。"""
    data: dict = {}
    key: str | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip()
        if not line.strip() or line.strip().startswith("#"):
            continue
        stripped = line.strip()
        if stripped.startswith("- "):
            if key:
                data.setdefault(key, []).append(stripped[2:].strip().strip('"').strip("'"))
            continue
        if ":" in stripped:
            name, _, value = stripped.partition(":")
            name = name.strip()
            value = value.strip().strip('"').strip("'")
            if value:
                data[name] = value
                key = name
            else:
                data.setdefault(name, [])
                key = name
    return data


def contract_facts() -> tuple[list[str], set[str]]:
    """(受锁源码, 门禁 id 集合)。"""
    try:
        with CONTRACT.open("rb") as handle:
            contract = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError):
        return [], set()
    locked = [
        value
        for value in contract.get("locked", {}).values()
        if isinstance(value, str) and value.endswith(".rs")
    ]
    gate_ids = {
        str(g.get("id"))
        for g in contract.get("ci_gates", {}).get("gate", [])
        if isinstance(g, dict)
    }
    return locked, gate_ids


def main() -> int:
    strict = "--strict" in sys.argv
    problems: list[str] = []

    if not TASKS_DIR.exists():
        print("（无 .harness/tasks/ ⇒ 没有任务清单 ⇒ 非阻塞提醒，不报错）")
        return 0

    manifests = sorted(TASKS_DIR.glob("*.yaml")) + sorted(TASKS_DIR.glob("*.yml"))
    locked, gate_ids = contract_facts()
    seen_ids: set[str] = set()

    for path in manifests:
        rel = path.relative_to(ROOT)
        try:
            data = parse_simple_yaml(path)
        except OSError as error:
            problems.append(f"{rel}: 读不了（{error}）")
            continue
        for field in REQUIRED_FIELDS:
            if not data.get(field):
                problems.append(f"{rel}: 缺必填字段 `{field}`")
        task_id = str(data.get("task_id", ""))
        if task_id:
            if task_id in seen_ids:
                problems.append(f"{rel}: `task_id` 与别的清单重复（{task_id}）")
            seen_ids.add(task_id)
        allowed = [str(x) for x in (data.get("allowed_paths") or [])]
        forbidden = [str(x) for x in (data.get("forbidden_paths") or [])]
        overlap = sorted(set(allowed) & set(forbidden))
        if overlap:
            problems.append(f"{rel}: `allowed_paths` 与 `forbidden_paths` 重叠：{overlap}")
        for pattern in forbidden:
            for locked_path in locked:
                if pattern.rstrip("*") in locked_path or locked_path.startswith(pattern.rstrip("*")):
                    problems.append(
                        f"{rel}: `forbidden_paths` 里的 `{pattern}` **命中受锁源码** `{locked_path}`"
                        f"（受锁源码本来就不许改 ⇒ 声明它为禁止项是多余的）"
                    )
        for check in data.get("required_checks") or []:
            if gate_ids and str(check) not in gate_ids:
                problems.append(
                    f"{rel}: `required_checks` 里的 `{check}` **不在契约 `[ci_gates]`**"
                    f" ⇒ 要么改 id，要么先把它登记进契约"
                )
        base_sha = data.get("base_sha")
        if base_sha:
            proc = subprocess.run(
                ["git", "cat-file", "-e", f"{base_sha}^{{commit}}"],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            if proc.returncode != 0:
                problems.append(f"{rel}: `base_sha` `{base_sha}` **不是可解析的 commit**")

    if problems:
        print(f"任务清单校验（{len(problems)} 处）", file=sys.stderr)
        for line in problems:
            print(f"  · {line}", file=sys.stderr)
        return 1
    print(f"OK: 任务清单校验通过（{len(manifests)} 份 · 非阻塞提醒）")
    if strict and not manifests:
        print("--strict 要求至少有一份任务清单", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
