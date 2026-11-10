#!/usr/bin/env python3
"""**上下文包生成器**（2026-10-30 · 效率型 harness）。

# 为什么要有它

> 「AI 不应该每次从头阅读几十份文档。」
> —— Astra 的效率型意见 §1 / §3

本仓 `docs/` 有 **500+ 份活跃文档**、`docs/status/` 20 份 ⇒ 每次开工前
「通读相关文档」既是**时间黑洞**，也是**误读旧口径**的主要来源
（仓库里大量文档写的是**当时**的口径 ⇒ 读了反而容易拿旧数字做决策）。

⇒ ⇒ 本脚本把「与**当前任务**有关的真实上下文」压成**一个短包**：

```
.harness/context/
├── repository-state.json     仓库状态（commit / tree / 分支 / 落后 main 多少）
├── affected-crates.json      受影响 crate + 反向依赖
├── dependency-subgraph.json  受影响部分的依赖子图
├── active-contracts.json     相关契约条目（locked / layers / env / budgets）
├── recent-failures.json      相关失败模式 + 最近一次运行结果
└── task-brief.md             人/AI 读的那一份（**短**）
```

# 口径（**不制造第二真源**）

* **任务边界**来自可选的 **task manifest**（`.harness/tasks/<id>.yaml`，
  owner 2026-10-30 裁定：**可入仓、非强制、不接 required gate**）
  ⇒ manifest **不存在**时也能跑（只用 `--task` 给的简报）；
* 契约信息**只读** `docs/architecture/contract.toml`（唯一真源）；
* 受影响 crate 优先用 `scripts/harness_impact.py`（若存在）⇒ **不重复实现**影响分析；
* 失败模式读 `.harness/knowledge/failure-patterns.yaml`（若存在）；
* **本脚本不写任何真源文件**（只写 `.harness/context/`，且该目录在 `.gitignore` 里）。

# 用法

```sh
python3 scripts/harness_context.py --task "把注入契约接进生产路径" [--base HEAD~1]
python3 scripts/harness_context.py --task .harness/tasks/2026-10-30-injection.yaml
```

# 输出到 stdout 的摘要（人看）

```
上下文包已生成：.harness/context/
  任务     : …
  受影响   : apeireth-memory (+2 反向依赖)
  影响级别 : L3（运行时接线）
  契约命中 : locked×3 · layers×2 · env×5
  失败提示 : 3 条相关模式
```
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "docs" / "architecture" / "contract.toml"
CONTEXT_DIR = ROOT / ".harness" / "context"
KNOWLEDGE = ROOT / ".harness" / "knowledge" / "failure-patterns.yaml"
TASKS_DIR = ROOT / ".harness" / "tasks"
IMPACT_SCRIPT = ROOT / "scripts" / "harness_impact.py"


def run_git(args: list[str]) -> str:
    """跑 git 并返回 stdout（失败 ⇒ 空串，**不抛**）。"""
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    return proc.stdout.strip() if proc.returncode == 0 else ""


def repository_state(base: str | None) -> dict:
    """仓库状态快照。"""
    head = run_git(["rev-parse", "HEAD"])
    tree = run_git(["rev-parse", "HEAD^{tree}"])
    branch = run_git(["rev-parse", "--abbrev-ref", "HEAD"])
    dirty = bool(run_git(["status", "--porcelain"]))
    # 与 main 的领先/落后
    ahead = behind = None
    if run_git(["rev-parse", "--verify", "main"]):
        counts = run_git(["rev-list", "--left-right", "--count", "main...HEAD"])
        if counts:
            parts = counts.split()
            if len(parts) == 2:
                behind, ahead = int(parts[0]), int(parts[1])
    return {
        "schema": "apeireth.repository-state.v1",
        "commit_sha": head,
        "tree_sha": tree,
        "branch": branch,
        "dirty": dirty,
        "ahead_of_main": ahead,
        "behind_main": behind,
        "base": base,
    }


def impact(base: str | None) -> dict:
    """调用 `scripts/harness_impact.py` 拿影响分析（**没有就退回轻量估计**）。"""
    if IMPACT_SCRIPT.exists():
        cmd = [sys.executable, str(IMPACT_SCRIPT), "--json"]
        if base:
            cmd += ["--base", base]
        try:
            proc = subprocess.run(
                cmd, cwd=ROOT, capture_output=True, text=True, timeout=300
            )
            if proc.returncode == 0:
                data = json.loads(proc.stdout)
                if isinstance(data, dict):
                    return data
        except (OSError, subprocess.SubprocessError, json.JSONDecodeError):
            pass
    # 退回：只看改了哪些文件
    diff = run_git(["diff", "--name-only", base or "HEAD~1", "--"])
    files = [line for line in diff.splitlines() if line.strip()]
    crates: list[str] = []
    for path in files:
        parts = Path(path).parts
        if len(parts) >= 2 and parts[0] == "crates":
            crates.append(parts[1])
    return {
        "schema": "apeireth.harness-impact.v1",
        "changed_files": files,
        "changed_count": len(files),
        "touched_crates": sorted(set(crates)),
        "affected_crates": sorted(set(crates)),
        "reverse_dependents": [],
        "level": "L1",
        "level_reasons": ["harness_impact.py 不可用 ⇒ 退回轻量估计（只看直接 crate）"],
        "touches": {},
        "suggested_stages": ["fast"],
    }


def active_contracts(crates: list[str]) -> dict:
    """从契约里挑出**与受影响 crate 相关**的条目。"""
    try:
        import tomllib

        with CONTRACT.open("rb") as handle:
            contract = tomllib.load(handle)
    except (OSError, ImportError, ValueError):
        return {"schema": "apeireth.active-contracts.v1", "available": False}

    def mentions_crates(value: object) -> bool:
        text = json.dumps(value, ensure_ascii=False)
        return any(crate in text for crate in crates)

    result: dict = {
        "schema": "apeireth.active-contracts.v1",
        "available": True,
        "locked": list(contract.get("locked", {}).items()),
        "layers": [
            layer
            for layer in contract.get("layers", [])
            if mentions_crates(layer)
        ],
        "env": [item for item in contract.get("env", []) if mentions_crates(item)],
        "budgets": contract.get("budgets", {}),
        "composition_roots": [
            root
            for root in contract.get("composition_roots", [])
            if mentions_crates(root)
        ],
        "ci_gates": [
            {"id": g.get("id"), "stage": g.get("stage"), "scope": g.get("scope")}
            for g in contract.get("ci_gates", {}).get("gate", [])
        ],
    }
    return result


def recent_failures(crates: list[str]) -> dict:
    """相关失败模式（知识库）+ 最近一次 harness 运行结果。"""
    patterns: list[dict] = []
    if KNOWLEDGE.exists():
        text = KNOWLEDGE.read_text(encoding="utf-8")
        # 极简 YAML 读取：`id:` / `trigger:` / `failure:` / `prevention:` / `category:` 逐行
        current: dict = {}
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("- id:"):
                if current:
                    patterns.append(current)
                current = {"id": stripped.split(":", 1)[1].strip()}
            elif current and ":" in stripped and line.startswith("  ") and not stripped.startswith("#"):
                key, _, value = stripped.partition(":")
                key = key.strip()
                if key in ("trigger", "failure", "prevention", "category", "guard"):
                    current[key] = value.strip().strip('"').strip("'")
        if current:
            patterns.append(current)

    last_run = {}
    last_run_path = ROOT / "scripts" / "harness-last-run.json"
    if last_run_path.exists():
        try:
            data = json.loads(last_run_path.read_text(encoding="utf-8"))
            last_run = {
                "verdict": data.get("verdict"),
                "commit_sha": data.get("commit_sha"),
                "failed_gates": [
                    g.get("id")
                    for g in data.get("gates", [])
                    if isinstance(g, dict) and g.get("status") == "FAIL"
                ],
            }
        except (OSError, json.JSONDecodeError):
            last_run = {}

    return {
        "schema": "apeireth.recent-failures.v1",
        "patterns": patterns,
        "last_run": last_run,
        "total_patterns": len(patterns),
    }


def load_task_manifest(path: Path) -> dict:
    """读**可选**的 task manifest（极简 YAML：`key: value` 与 `key:` + `- item` 列表）。"""
    if not path.exists():
        return {}
    manifest: dict = {}
    current_key = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.rstrip()
        if not line.strip() or line.strip().startswith("#"):
            continue
        stripped = line.strip()
        if stripped.startswith("- "):
            if current_key:
                manifest.setdefault(current_key, []).append(stripped[2:].strip().strip('"'))
            continue
        if ":" in stripped:
            key, _, value = stripped.partition(":")
            key = key.strip()
            value = value.strip().strip('"')
            if value:
                manifest[key] = value
                current_key = key
            else:
                manifest.setdefault(key, [])
                current_key = key
    return manifest


def render_brief(
    task: str,
    manifest: dict,
    state: dict,
    impact_data: dict,
    contracts: dict,
    failures: dict,
) -> str:
    """人/AI 读的那一份（**短**）。"""
    crates = impact_data.get("affected_crates") or []
    lines = [
        "# 任务简报（自动生成 · 短）",
        "",
        f"**任务**：{task}",
        "",
        "## 1. 仓库状态",
        f"- commit：`{state.get('commit_sha', '?')[:8]}` · tree：`{state.get('tree_sha', '?')[:8]}`",
        f"- 分支：`{state.get('branch') or '?'}`"
        + (
            f" · 落后 main **{state.get('behind_main')}** 个 commit"
            f" · 领先 **{state.get('ahead_of_main')}**"
            if state.get("behind_main") is not None
            else ""
        ),
        f"- 工作区{'**有未提交改动**' if state.get('dirty') else '干净'}",
        "",
        "## 2. 任务边界",
    ]
    if manifest:
        allowed = manifest.get("allowed_paths") or []
        forbidden = manifest.get("forbidden_paths") or []
        lines += [
            f"- 任务 id：`{manifest.get('task_id', '?')}`",
            f"- **允许改**：{', '.join(f'`{p}`' for p in allowed) if allowed else '（未声明）'}",
            f"- **禁止碰**：{', '.join(f'`{p}`' for p in forbidden) if forbidden else '（未声明）'}",
        ]
    else:
        lines.append("- （无 task manifest ⇒ **未声明边界**；建议先声明再动手）")
    lines += [
        "",
        "## 3. 影响面",
        f"- 影响级别：**{impact_data.get('level', '?')}**"
        f"（建议跑：{', '.join(impact_data.get('suggested_stages') or ['fast'])}）",
        f"- 受影响 crate：{', '.join(f'`{c}`' for c in crates) if crates else '（无）'}",
    ]
    for reason in (impact_data.get("level_reasons") or [])[:4]:
        lines.append(f"  - {reason}")
    lines += [
        "",
        "## 4. 相关契约（**真源**：`docs/architecture/contract.toml`）",
    ]
    locked = contracts.get("locked") or []
    if locked:
        lines.append(f"- 受锁源码（0 触碰）：{', '.join(f'`{v}`' for _, v in locked)}")
    if contracts.get("layers"):
        lines.append(f"- 相关层：{len(contracts['layers'])} 条")
    if contracts.get("env"):
        lines.append(f"- 相关开关：{len(contracts['env'])} 条")
    lines += [
        "",
        "## 5. 已知失败模式（**别再踩**）",
    ]
    patterns = failures.get("patterns") or []
    if patterns:
        for pattern in patterns[:8]:
            lines.append(
                f"- `{pattern.get('id', '?')}`（{pattern.get('category', '?')}）"
                f" ⇒ {pattern.get('prevention') or pattern.get('failure') or ''}"
            )
    else:
        lines.append("- （知识库为空）")
    last = failures.get("last_run") or {}
    if last:
        lines.append(
            f"- 最近一次 harness 裁决：**{last.get('verdict')}**"
            + (
                f" · 失败门禁：{', '.join(last['failed_gates'])}"
                if last.get("failed_gates")
                else ""
            )
        )
    lines += [
        "",
        "## 6. 怎么验（**短反馈优先**）",
        "",
        "```sh",
        "python3 scripts/harness.py check --stage fast   # 秒级（每次改动都能跑）",
        "python3 scripts/harness.py verify                # 两阶段：fast 过了才跑 deep",
        "```",
        "",
        "## 7. 完成标准",
        "",
        "- [ ] `harness.py check --stage fast` 全绿",
        "- [ ] 与本任务相关的 `deep` 门禁全绿",
        "- [ ] 声明（claim）有**可跑**的 `verify_cmd`",
        "",
        "---",
        "",
        f"（自动生成于 commit `{state.get('commit_sha', '?')[:8]}`；"
        "真源：`docs/architecture/contract.toml` · `scripts/harness.py`）",
    ]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", required=True, help="任务简述（文本）或 task manifest 路径")
    parser.add_argument("--base", default=None, help="基线 SHA（默认 HEAD~1）")
    parser.add_argument("--out", default=None, help="输出目录（默认 .harness/context）")
    args = parser.parse_args()

    out_dir = Path(args.out) if args.out else CONTEXT_DIR
    out_dir.mkdir(parents=True, exist_ok=True)

    manifest: dict = {}
    task_text = args.task
    task_path = Path(args.task)
    if task_path.exists() and task_path.is_file():
        manifest = load_task_manifest(task_path)
        task_text = manifest.get("task") or manifest.get("summary") or task_path.name
    elif task_path.suffix in (".yaml", ".yml") and not task_path.exists():
        print(
            f"warning: task manifest 不存在（{task_path}）⇒ 按纯文本任务处理",
            file=sys.stderr,
        )

    state = repository_state(args.base)
    impact_data = impact(args.base)
    crates = impact_data.get("affected_crates") or []
    contracts = active_contracts(crates)
    failures = recent_failures(crates)

    files = {
        "repository-state.json": state,
        "affected-crates.json": impact_data,
        "dependency-subgraph.json": {
            "schema": "apeireth.dependency-subgraph.v1",
            "crates": crates,
            "reverse_dependents": impact_data.get("reverse_dependents") or [],
        },
        "active-contracts.json": contracts,
        "recent-failures.json": failures,
    }
    for name, payload in files.items():
        (out_dir / name).write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
    (out_dir / "task-brief.md").write_text(
        render_brief(task_text, manifest, state, impact_data, contracts, failures),
        encoding="utf-8",
    )

    print(f"上下文包已生成：{out_dir}")
    print(f"  任务     : {task_text}")
    print(
        f"  受影响   : {', '.join(crates) if crates else '(无)'}"
        + (
            f" (+{len(impact_data.get('reverse_dependents') or [])} 反向依赖)"
            if impact_data.get("reverse_dependents")
            else ""
        )
    )
    print(f"  影响级别 : {impact_data.get('level', '?')}")
    print(f"  契约命中 : locked×{len(contracts.get('locked') or [])}")
    print(f"  失败提示 : {failures.get('total_patterns', 0)} 条模式")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
