#!/usr/bin/env python3
"""**统一验证入口**（harness）—— 本仓**唯一**的验收入口。

# 它解决什么

门禁散落在 30+ 个脚本与 42 个 workflow 里，而「哪些门禁对本次改动是必需的」
「跳过到底算不算通过」从来没有机器可读的口径 ⇒ 于是「本地跑过了」被当成
「合并没问题」⇒ 漂移的温床。

本脚本把这件事收成**一条命令链**::

    python3 scripts/harness.py plan              # 只分类，不执行
    python3 scripts/harness.py check             # 执行门禁 + 落盘机器可读结果
    python3 scripts/harness.py attest            # 写出与 SHA 绑定的证据包
    python3 scripts/harness.py merge-decision    # 汇总成**一个**裁决（退出码即闸门）

本地快通道：`--only <id,...>` 只跑指定门禁；`--skip-gate <id,...>` 把指定门禁标成
**显式跳过**（记 `SKIPPED`，绝不算通过）—— 用于避开会触发 cargo 编译的门禁。

# 门禁登记表（唯一真源）

门禁清单**不写死在脚本里** —— 读 `docs/architecture/contract.toml` 的 `[ci_gates]`
段（`[[ci_gates.gate]]` 数组表：`id` / `cmd` / `scope` / `blocking` / `required` /
`timeout` / `env`）。

若契约还没有这一段 ⇒ 回退到内置的**小而安全**默认集（只读快检：
`docs_truth.py --verify --block-stale`、`docs_links.py --check`、`check_doc_class.py --check`
—— **不含** cargo build/nextest/clippy，那是 CI 的活），并在输出里明确提示
「契约尚无 ci_gates 段，已回退到内置默认集」。

契约里 `env = "cargo"` / `env = "ci"` 的门禁是 **CI 专属**（契约自己的注释：
「本地跳过 ⇒ 状态必须是 SKIPPED，不等于 PASS」）⇒ 本地**不执行**，记 `SKIPPED`
并写明原因（本地绝不跑 cargo）。

# 状态语义（**核心**）

每个门禁产出**恰好一个**状态：

| 状态        | 含义                                                        | 阻断合并 |
|-------------|-------------------------------------------------------------|----------|
| `PASS`      | 命令退出码 0                                                | 否       |
| `FAIL`      | 命令退出码非 0（记录命令 + 输出尾部 40 行）                  | **是**   |
| `SKIPPED`   | 本次 diff 不触发该门禁（scope 不相交 / CI 专属 / 未选中）     | required 时**是** |
| `UNVERIFIED`| 没能跑起来（缺命令 / 超时 / 环境无法执行 / 豁免无效）        | **是**   |
| `WAIVED`    | 命中显式豁免（`id` + `approver` + `expires_at` 齐全且未过期） | 否       |
| `BLOCKED`   | 门禁**执行前**的前置条件不满足（脏树 / 缺 base SHA 等）      | **是**   |

## SKIPPED 不等于 PASS

「这次没触发」**不是**「这次通过了」。`SKIPPED` 永远不得在任何地方被写成 pass；
`required = true` 的门禁若被 `SKIPPED` ⇒ 裁决 `BLOCKED`，原因写
「required gate not applicable/skipped without justification」。

## UNVERIFIED 不能进入合并

跑不起来 ≠ 跑过了。超时、命令缺失、环境不支持 ⇒ 一律记 `UNVERIFIED` ⇒ 阻断合并。
只认「确实跑过且退出码为 0」这一种证据。

# 裁决规则（`merge-decision`）

* `PASS`    ：每个 `required = true` 的门禁都是 `PASS`（或已被显式豁免 `WAIVED`），
  且没有任何门禁处于 `FAIL` / `UNVERIFIED` / `BLOCKED`。
* `FAIL`    ：出现任何一个 `FAIL`。
* `BLOCKED` ：出现 `UNVERIFIED` / `BLOCKED`，或 `required = true` 的门禁被 `SKIPPED`，
  或证据与当前 HEAD 不一致（stale evidence），或没有 git 上下文（no git context）。
* 退出码 0 **当且仅当** 裁决为 `PASS`。

# 证据（SHA 绑定）

* `check` ⇒ `scripts/harness-last-run.json`（schema `apeireth.harness-run.v1`）。
* `attest` ⇒ `artifacts/harness/<commit_sha>/`：`task.json` / `gate-results.json` /
  `evidence-index.json` / `summary.md` —— SHA 绑定 ⇒ 换 commit 即失效（stale evidence）。

# 只用标准库

Python 3.11+（`tomllib` 读契约）；更老的 Python 会提示并回退到内置默认集。
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - 老 Python 走回退集
    tomllib = None  # type: ignore[assignment]

# --------------------------------------------------------------------------- 常量

ROOT = Path(__file__).resolve().parent.parent
CONTRACT_PATH = ROOT / "docs" / "architecture" / "contract.toml"
LAST_RUN_PATH = ROOT / "scripts" / "harness-last-run.json"
ARTIFACT_ROOT = ROOT / "artifacts" / "harness"

RUN_SCHEMA = "apeireth.harness-run.v1"
TASK_SCHEMA = "apeireth.harness-task.v1"
EVIDENCE_SCHEMA = "apeireth.harness-evidence-index.v1"

DEFAULT_TIMEOUT = 900
TAIL_LINES = 40

S_PASS = "PASS"
S_FAIL = "FAIL"
S_SKIPPED = "SKIPPED"
S_UNVERIFIED = "UNVERIFIED"
S_WAIVED = "WAIVED"
S_BLOCKED = "BLOCKED"

VERDICT_PASS = "PASS"
VERDICT_FAIL = "FAIL"
VERDICT_BLOCKED = "BLOCKED"

# 契约尚未提供 [ci_gates] 时的内置回退集：**小而安全**，只含只读快检，
# 刻意**不含** cargo build/nextest/clippy（太慢，且那是 CI 的活）。
# required=false：回退集的门禁 scope 可能与本次 diff 不相交，
# 而「required 门禁被 SKIPPED ⇒ 裁决 BLOCKED」⇒ 回退集设为咨询性，
# 真正跑失败（FAIL）依然阻断合并。
FALLBACK_GATES: list[dict[str, Any]] = [
    {
        "id": "docs-truth",
        "cmd": ["python3", "scripts/docs_truth.py", "--verify", "--block-stale"],
        "scope": ["docs", "claims"],
        "blocking": True,
        "required": False,
        "timeout": 600,
        "env": "python",
    },
    {
        "id": "docs-links",
        "cmd": ["python3", "scripts/docs_links.py", "--check"],
        "scope": ["docs"],
        "blocking": True,
        "required": False,
        "timeout": 600,
        "env": "python",
    },
    {
        "id": "doc-class",
        "cmd": ["python3", "scripts/check_doc_class.py", "--check"],
        "scope": ["docs"],
        "blocking": True,
        "required": False,
        "timeout": 600,
        "env": "python",
    },
]

# 契约里 CI 专属的 env 取值（本地不执行）
CI_ONLY_ENVS = {"cargo", "ci"}


# --------------------------------------------------------------------------- 工具

def now_iso() -> str:
    """当前时间的 ISO-8601 字符串（带时区偏移）。"""
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def rel_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def _git(*args: str, timeout: int = 60) -> tuple[int, str]:
    """跑一条 git 命令；返回 (退出码, stdout)。git 不可用时返回 (1, "")。"""
    try:
        proc = subprocess.run(
            ["git", *args],
            cwd=str(ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError):
        return 1, ""
    return proc.returncode, proc.stdout.decode("utf-8", "replace").strip()


def parse_iso(value: str) -> dt.datetime | None:
    """宽松解析 ISO-8601（支持 'Z' 与纯日期）。失败返回 None。"""
    text = (value or "").strip()
    if not text:
        return None
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = dt.datetime.fromisoformat(text)
    except ValueError:
        try:
            parsed = dt.datetime.strptime(text, "%Y-%m-%d")
        except ValueError:
            return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=dt.timezone.utc)
    return parsed


# ------------------------------------------------------------------- diff 分类

def classify_file(path: str) -> list[str]:
    """按路径给单个文件打 diff 分类标签。"""
    rel = path.replace("\\", "/").strip()
    while rel.startswith("./"):
        rel = rel[2:]
    name = rel.rsplit("/", 1)[-1]
    tags: set[str] = set()

    # rust: crates/**/*.rs, Cargo.toml
    if (rel.startswith("crates/") and rel.endswith(".rs")) or name == "Cargo.toml":
        tags.add("rust")
    # docs: docs/**, *.md, AGENTS.md, CONTRIBUTING.md
    if rel.startswith("docs/") or name.endswith(".md") or name in ("AGENTS.md", "CONTRIBUTING.md"):
        tags.add("docs")
    # claims: docs/**/claims*.yaml|yml, docs/claims/**
    if rel.startswith("docs/claims/") or (
        rel.startswith("docs/") and name.startswith("claims") and name.endswith((".yaml", ".yml"))
    ):
        tags.add("claims")
    # ci: .github/**, .githooks/**, scripts/**
    if rel.startswith((".github/", ".githooks/", "scripts/")):
        tags.add("ci")
    # contract: docs/architecture/contract.toml, .harness/**
    if rel == "docs/architecture/contract.toml" or rel.startswith(".harness/"):
        tags.add("contract")
    # lock: Cargo.lock
    if rel == "Cargo.lock":
        tags.add("lock")
    # frontend: frontend/**, pnpm-lock.yaml
    if rel.startswith("frontend/") or name == "pnpm-lock.yaml":
        tags.add("frontend")

    return sorted(tags)


def changed_files(base: str | None) -> tuple[list[str], str]:
    """返回 (变更文件列表, 来源说明)。

    主路径：`git diff --name-only <base>..HEAD`；git 不可用 / 无该基点 ⇒
    回退 `git status --porcelain`（工作区未提交改动）。
    """
    ref = base or "HEAD~1"
    code, out = _git("diff", "--name-only", f"{ref}..HEAD")
    if code == 0:
        files = [line.strip() for line in out.splitlines() if line.strip()]
        return files, f"git diff --name-only {ref}..HEAD"
    code, out = _git("status", "--porcelain")
    if code == 0:
        files = []
        for line in out.splitlines():
            line = line.strip()
            if not line:
                continue
            path = line.split(" -> ")[-1].strip().strip('"')
            if path:
                files.append(path)
        return files, "git status --porcelain（回退：未提交改动）"
    return [], "无 git 上下文"


def diff_tags(files: list[str]) -> list[str]:
    """变更文件的标签并集（有序）。"""
    tags: set[str] = set()
    for path in files:
        tags.update(classify_file(path))
    return sorted(tags)


# ------------------------------------------------------------------- 门禁登记表

def normalize_gate(raw: Any) -> dict[str, Any] | None:
    """把契约里的一条 [[ci_gates.gate]] 归一化成内部结构。"""
    if not isinstance(raw, dict):
        return None
    gid = raw.get("id")
    cmd = raw.get("cmd")
    if not isinstance(gid, str) or not gid.strip():
        return None
    if not isinstance(cmd, list) or not cmd or not all(isinstance(x, str) for x in cmd):
        return None
    scope = raw.get("scope")
    if not isinstance(scope, list) or not scope:
        scope = ["*"]
    scope = [str(x) for x in scope]
    timeout = raw.get("timeout")
    if not isinstance(timeout, int) or timeout <= 0:
        timeout = DEFAULT_TIMEOUT
    return {
        "id": gid.strip(),
        "cmd": list(cmd),
        "scope": scope,
        "blocking": bool(raw.get("blocking", True)),
        "required": bool(raw.get("required", True)),
        "timeout": timeout,
        "env": str(raw.get("env", "python")),
        "require_clean_tree": bool(raw.get("require_clean_tree", False)),
        "note": str(raw.get("note", "")),
    }


def parse_exemptions(data: Any) -> list[dict[str, Any]]:
    """解析豁免记录（契约 [ci_gates].exemption(s) + 可选 .harness/exemptions.toml）。"""
    out: list[dict[str, Any]] = []

    def _push(item: Any) -> None:
        if not isinstance(item, dict):
            return
        out.append(
            {
                "id": str(item.get("id") or "").strip(),
                "gate": str(item.get("gate") or item.get("gate_id") or "").strip(),
                "approver": str(item.get("approver") or "").strip(),
                "expires_at": str(item.get("expires_at") or "").strip(),
                "reason": str(item.get("reason") or "").strip(),
            }
        )

    if isinstance(data, dict):
        section = data.get("ci_gates")
        if isinstance(section, dict):
            for key in ("exemption", "exemptions"):
                raw = section.get(key)
                if isinstance(raw, dict):
                    _push(raw)
                elif isinstance(raw, list):
                    for item in raw:
                        _push(item)

    harness_file = ROOT / ".harness" / "exemptions.toml"
    if tomllib is not None and harness_file.is_file():
        try:
            with harness_file.open("rb") as fh:
                hdata = tomllib.load(fh)
        except (OSError, ValueError):
            hdata = None
        if isinstance(hdata, dict):
            for key in ("exemption", "exemptions"):
                raw = hdata.get(key)
                if isinstance(raw, dict):
                    _push(raw)
                elif isinstance(raw, list):
                    for item in raw:
                        _push(item)

    return out


def load_registry() -> tuple[list[dict[str, Any]], list[str], list[dict[str, Any]]]:
    """读取门禁登记表。

    返回 (门禁列表, 提示信息列表, 豁免记录列表)。
    契约无 [ci_gates] 段 / 解析失败 ⇒ 回退到内置默认集，并给出明确提示。
    """
    notes: list[str] = []
    exemptions: list[dict[str, Any]] = []

    if tomllib is None:
        notes.append(
            "当前 Python 没有 tomllib（需 3.11+）⇒ 无法读取契约 ⇒ 已回退到内置默认门禁集。"
        )
        return list(FALLBACK_GATES), notes, exemptions

    if not CONTRACT_PATH.is_file():
        notes.append(
            f"契约缺失（{rel_path(CONTRACT_PATH)}）⇒ 已回退到内置默认门禁集。"
        )
        return list(FALLBACK_GATES), notes, exemptions

    try:
        with CONTRACT_PATH.open("rb") as fh:
            data = tomllib.load(fh)
    except (OSError, ValueError) as exc:
        notes.append(
            f"契约解析失败（{type(exc).__name__}: {exc}）⇒ 已回退到内置默认门禁集。"
        )
        return list(FALLBACK_GATES), notes, exemptions

    exemptions = parse_exemptions(data)
    section = data.get("ci_gates")
    raw_gates = section.get("gate") if isinstance(section, dict) else None
    gates = [g for g in (normalize_gate(item) for item in (raw_gates or [])) if g]

    if not gates:
        notes.append(
            "契约尚无 [ci_gates] 段（或该段为空）⇒ 已回退到内置默认门禁集"
            "（只读快检，不含 cargo build/nextest/clippy）。"
        )
        return list(FALLBACK_GATES), notes, exemptions

    notes.append(f"门禁登记表来源：{rel_path(CONTRACT_PATH)} [ci_gates]（{len(gates)} 条门禁）。")
    return gates, notes, exemptions


# ------------------------------------------------------------------- 豁免判定

def find_exemption(exemptions: list[dict[str, Any]], gate_id: str) -> dict[str, Any] | None:
    for item in exemptions:
        target = item.get("gate") or item.get("id")
        if target == gate_id:
            return item
    return None


def exemption_state(exemption: dict[str, Any] | None) -> tuple[str, str]:
    """判定豁免是否成立。

    返回 (state, detail)；state ∈ {"none", "valid", "expired", "incomplete"}。
    过期或字段缺失 ⇒ **不算豁免**（⇒ 退化为 UNVERIFIED）。
    """
    if not exemption:
        return "none", ""
    eid = (exemption.get("id") or "").strip()
    approver = (exemption.get("approver") or "").strip()
    expires = (exemption.get("expires_at") or "").strip()
    missing = [
        name
        for name, value in (("id", eid), ("approver", approver), ("expires_at", expires))
        if not value
    ]
    if missing:
        return "incomplete", f"豁免记录缺字段：{', '.join(missing)} ⇒ 不算豁免"
    when = parse_iso(expires)
    if when is None:
        return "incomplete", f"豁免 expires_at 无法解析：{expires} ⇒ 不算豁免"
    if when < dt.datetime.now(dt.timezone.utc):
        return "expired", f"豁免 {eid} 已于 {expires} 过期 ⇒ 不算豁免"
    return "valid", f"豁免 {eid}（批准人 {approver}，有效至 {expires}）"


# ------------------------------------------------------------------- 门禁执行

def decide_run(
    gate: dict[str, Any],
    tags: list[str],
    only: set[str] | None,
    skip: set[str] | None = None,
) -> tuple[bool, str]:
    """判断门禁是否应当执行，并给出原因。"""
    if only is not None and gate["id"] not in only:
        return False, "not selected by --only（本次未选中）"
    if skip and gate["id"] in skip:
        return False, "显式跳过（--skip-gate）"
    scope = gate.get("scope") or ["*"]
    if "*" not in scope:
        hit = sorted(set(scope) & set(tags))
        if not hit:
            return False, f"scope {scope} 与 diff 标签 {tags} 不相交"
    env = gate.get("env", "python")
    if env in CI_ONLY_ENVS:
        return False, f"CI 专属门禁（env={env}）：本地不执行，须由 GitHub Actions 跑"
    return True, "scope 命中" if scope != ["*"] else "scope=* 总是执行"


def tree_is_dirty() -> bool:
    code, out = _git("status", "--porcelain")
    if code != 0:
        return False
    return bool(out.strip())


def run_gate(gate: dict[str, Any], timeout: int) -> dict[str, Any]:
    """执行单个门禁；任何异常都被捕获 ⇒ 记 UNVERIFIED，不中断整轮。"""
    env = os.environ.copy()
    env["LC_ALL"] = "C.UTF-8"
    env["PYTHONIOENCODING"] = "utf-8"
    env["APEIRETH_HARNESS"] = "1"

    raw = b""
    exit_code: int | None = None
    status = S_UNVERIFIED
    reason: str | None = None
    started = time.monotonic()
    try:
        proc = subprocess.run(
            list(gate["cmd"]),
            cwd=str(ROOT),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            env=env,
        )
        raw = proc.stdout or b""
        exit_code = proc.returncode
        if exit_code == 0:
            status = S_PASS
            reason = None
        else:
            status = S_FAIL
            reason = f"退出码 {exit_code} ≠ 0"
    except subprocess.TimeoutExpired as exc:
        raw = exc.stdout or b""
        status = S_UNVERIFIED
        reason = f"timeout（超过 {timeout}s 未结束）"
    except FileNotFoundError as exc:
        raw = b""
        status = S_UNVERIFIED
        reason = f"命令不存在：{exc}"
    except Exception as exc:  # 任何门禁异常都不许中断整轮
        raw = b""
        status = S_UNVERIFIED
        reason = f"执行失败：{type(exc).__name__}: {exc}"
    duration = round(time.monotonic() - started, 3)

    text = raw.decode("utf-8", "replace")
    lines = text.splitlines()
    tail = "\n".join(lines[-TAIL_LINES:])
    full_sha = hashlib.sha256(raw).hexdigest()
    tail_sha = hashlib.sha256(tail.encode("utf-8")).hexdigest()

    return {
        "id": gate["id"],
        "status": status,
        "command": list(gate["cmd"]),
        "exit_code": exit_code,
        "duration_s": duration,
        "output_tail": tail,
        "required": bool(gate.get("required", True)),
        "reason": reason,
        # 附加信息（不改变上面的必备字段）
        "blocking": bool(gate.get("blocking", True)),
        "scope": list(gate.get("scope") or ["*"]),
        "timeout": timeout,
        "env": gate.get("env", "python"),
        "output_sha256": full_sha,
        "tail_sha256": tail_sha,
    }


def gate_record(
    gate: dict[str, Any],
    status: str,
    reason: str | None,
    exit_code: int | None = None,
    duration: float = 0.0,
    tail: str = "",
    full_sha: str = "",
    tail_sha: str = "",
) -> dict[str, Any]:
    """构造一条门禁结果记录（结构与执行时保持一致）。"""
    return {
        "id": gate["id"],
        "status": status,
        "command": list(gate["cmd"]),
        "exit_code": exit_code,
        "duration_s": duration,
        "output_tail": tail,
        "required": bool(gate.get("required", True)),
        "reason": reason,
        "blocking": bool(gate.get("blocking", True)),
        "scope": list(gate.get("scope") or ["*"]),
        "timeout": int(gate.get("timeout", DEFAULT_TIMEOUT)),
        "env": gate.get("env", "python"),
        "output_sha256": full_sha,
        "tail_sha256": tail_sha,
    }


# ------------------------------------------------------------------- 裁决

def evaluate_verdict(
    gates: list[dict[str, Any]],
    blocking_reasons: list[str] | None = None,
    info_reasons: list[str] | None = None,
) -> tuple[str, list[str]]:
    """按状态语义聚合出**一个**裁决。

    * 任何 FAIL ⇒ FAIL
    * 任何 UNVERIFIED/BLOCKED ⇒ BLOCKED
    * 任何 required 门禁被 SKIPPED ⇒ BLOCKED
    * 出现阻断性外部原因（no git context / stale evidence / missing base SHA）⇒ BLOCKED
    * 其余 ⇒ PASS（required 门禁须为 PASS 或已批准的 WAIVED）

    `blocking_reasons` 里的每一条都**独立**把裁决压到 BLOCKED；
    `info_reasons` 只作展示，不影响裁决。
    """
    reasons: list[str] = list(info_reasons or [])
    blocking: list[str] = list(blocking_reasons or [])
    fails = [g for g in gates if g.get("status") == S_FAIL]
    unver = [g for g in gates if g.get("status") in (S_UNVERIFIED, S_BLOCKED)]
    req_bad = [
        g
        for g in gates
        if g.get("required")
        and g.get("status") not in (S_PASS, S_WAIVED, S_FAIL, S_UNVERIFIED, S_BLOCKED)
    ]

    for gate in fails:
        reasons.append(f"门禁 {gate['id']} FAIL（退出码 {gate.get('exit_code')}）")
    for gate in unver:
        reasons.append(f"门禁 {gate['id']} {gate.get('status')}（{gate.get('reason') or '原因未记录'}）")
    for gate in req_bad:
        reasons.append(
            "required gate not applicable/skipped without justification: " + str(gate.get("id"))
        )
    reasons.extend(blocking)

    if fails:
        verdict = VERDICT_FAIL
    elif unver or req_bad or blocking:
        verdict = VERDICT_BLOCKED
    else:
        verdict = VERDICT_PASS
    return verdict, reasons


# ------------------------------------------------------------------- 上下文

def collect_context(base: str | None) -> dict[str, Any]:
    """收集 git 上下文 + 变更文件 + diff 标签。"""
    commit_sha = tree_sha = base_sha = None
    git_ok = False
    code, out = _git("rev-parse", "HEAD")
    if code == 0 and out:
        commit_sha = out
        git_ok = True
    code, out = _git("rev-parse", "HEAD^{tree}")
    if code == 0 and out:
        tree_sha = out
    if base:
        code, out = _git("rev-parse", "--verify", f"{base}^{{commit}}")
    else:
        code, out = _git("rev-parse", "--verify", "HEAD~1^{commit}")
    if code == 0 and out:
        base_sha = out

    files, source = changed_files(base)
    tags = diff_tags(files)
    return {
        "commit_sha": commit_sha,
        "tree_sha": tree_sha,
        "base_sha": base_sha,
        "git_ok": git_ok,
        "files": files,
        "diff_source": source,
        "tags": tags,
    }


def global_reasons(ctx: dict[str, Any]) -> list[str]:
    """全局层面的阻断原因（与单个门禁无关）。"""
    reasons: list[str] = []
    if not ctx.get("git_ok"):
        reasons.append("no git context（无法把证据绑定到 SHA）")
    elif not ctx.get("base_sha"):
        reasons.append("missing base SHA（HEAD~1 不存在且未给 --base）⇒ 变更范围不可信")
    return reasons


# ------------------------------------------------------------------- 打印

def print_table(headers: list[str], rows: list[list[str]]) -> None:
    if not rows:
        print("(无)")
        return
    widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            widths[i] = max(widths[i], len(cell))
    def fmt(cells: list[str]) -> str:
        return " | ".join(cell.ljust(widths[i]) for i, cell in enumerate(cells))
    print(fmt(headers))
    print("-+-".join("-" * w for w in widths))
    for row in rows:
        print(fmt(row))


def render_gate_table(gates: list[dict[str, Any]]) -> None:
    rows = []
    for gate in gates:
        exit_code = gate.get("exit_code")
        rows.append(
            [
                str(gate.get("id")),
                str(gate.get("status")),
                "yes" if gate.get("required") else "no",
                "-" if exit_code is None else str(exit_code),
                f"{float(gate.get('duration_s') or 0):.2f}",
                (gate.get("reason") or "").replace("\n", " ")[:70],
            ]
        )
    print_table(["GATE", "STATUS", "REQ", "EXIT", "SECS", "REASON"], rows)


def print_notes(notes: list[str], stream=sys.stdout) -> None:
    for note in notes:
        print(f"note: {note}", file=stream)


# --------------------------------------------------------------------------- plan

def cmd_plan(args: argparse.Namespace) -> int:
    ctx = collect_context(args.base)
    gates, notes, exemptions = load_registry()
    only = parse_only(args.only)
    skip = parse_only(getattr(args, "skip_gate", None))
    tags = ctx["tags"]
    files = ctx["files"]

    print("== 变更分类 ==")
    print(f"提交    : {ctx['commit_sha'] or '(无 git 上下文)'}")
    print(f"基线    : {ctx['base_sha'] or '(无)'}")
    print(f"来源    : {ctx['diff_source']}")
    print(f"变更文件: {len(files)} 个")
    print(f"diff 标签: {tags if tags else '(无)'}")
    if files:
        shown = files[:60]
        for path in shown:
            print(f"  - {path}  {classify_file(path)}")
        if len(files) > len(shown):
            print(f"  ...（另有 {len(files) - len(shown)} 个）")
    print_notes(notes)
    print()

    print("== 门禁计划（只分类，不执行）==")
    rows = []
    for gate in gates:
        run, why = decide_run(gate, tags, only, skip)
        rows.append(
            [
                gate["id"],
                "yes" if gate.get("required") else "no",
                ",".join(gate.get("scope") or ["*"]),
                ("yes" if run else "no") + " — " + why,
            ]
        )
    print_table(["GATE", "REQUIRED", "SCOPE", "WILL-RUN"], rows)

    skipped_required = [
        g["id"]
        for g in gates
        if g.get("required") and not decide_run(g, tags, only, skip)[0]
    ]
    print()
    print(
        f"提示：{len(skipped_required)} 个 required 门禁本次不适用 ⇒ "
        "按裁决规则「required gate not applicable/skipped without justification」"
        "⇒ merge-decision 裁决将为 BLOCKED（SKIPPED ≠ PASS）。"
        if skipped_required
        else "提示：本次所有 required 门禁都会执行。"
    )
    if skipped_required:
        print("      未适用的 required 门禁: " + ", ".join(skipped_required))
    return 0


def parse_only(raw: str | None) -> set[str] | None:
    if not raw:
        return None
    return {piece.strip() for piece in raw.split(",") if piece.strip()}


# --------------------------------------------------------------------------- check

def cmd_check(args: argparse.Namespace) -> int:
    ctx = collect_context(args.base)
    gates, notes, exemptions = load_registry()
    only = parse_only(args.only)
    skip = parse_only(getattr(args, "skip_gate", None))
    tags = ctx["tags"]

    known = {g["id"] for g in gates}
    if only is not None:
        unknown = sorted(only - known)
        if unknown:
            print(f"error: --only 里的未知门禁 id: {', '.join(unknown)}", file=sys.stderr)
            print(f"       登记表里的门禁: {', '.join(sorted(known))}", file=sys.stderr)
            return 2
    if skip:
        unknown = sorted(skip - known)
        if unknown:
            print(
                f"warning: --skip-gate 里的未知门禁 id（将被忽略）: {', '.join(unknown)}",
                file=sys.stderr,
            )

    started_at = now_iso()
    results: list[dict[str, Any]] = []
    dirty = tree_is_dirty()

    for gate in gates:
        run, why = decide_run(gate, tags, only, skip)
        if not run:
            results.append(gate_record(gate, S_SKIPPED, why))
            continue

        if gate.get("require_clean_tree") and dirty:
            results.append(
                gate_record(
                    gate,
                    S_BLOCKED,
                    "前置条件不满足：工作区不干净（require_clean_tree=true）",
                )
            )
            continue

        exemption = find_exemption(exemptions, gate["id"])
        state, detail = exemption_state(exemption)
        if state == "valid":
            results.append(gate_record(gate, S_WAIVED, detail))
            continue

        timeout = int(args.timeout) if args.timeout else int(gate.get("timeout", DEFAULT_TIMEOUT))
        record = run_gate(gate, timeout)
        if state in ("expired", "incomplete"):
            # 豁免不成立（过期 / 缺字段）⇒ 不是 WAIVED ⇒ 记 UNVERIFIED（阻断合并）
            record["status"] = S_UNVERIFIED
            record["reason"] = f"{detail}；门禁结果仅供参考：{record.get('reason') or '—'}"
        results.append(record)

    reasons = global_reasons(ctx)
    verdict, verdict_reasons = evaluate_verdict(results, reasons)
    finished_at = now_iso()

    run_doc = {
        "schema": RUN_SCHEMA,
        "commit_sha": ctx["commit_sha"],
        "tree_sha": ctx["tree_sha"],
        "base_sha": ctx["base_sha"],
        "started_at": started_at,
        "finished_at": finished_at,
        "diff_tags": tags,
        "gates": results,
        "verdict": verdict,
        # 附加信息
        "verdict_reasons": verdict_reasons,
        "changed_files": ctx["files"],
        "changed_file_count": len(ctx["files"]),
        "diff_source": ctx["diff_source"],
        "registry_source": notes,
        "restricted_to": sorted(only) if only is not None else None,
        "skipped_by_request": sorted(skip) if skip else None,
    }

    LAST_RUN_PATH.parent.mkdir(parents=True, exist_ok=True)
    LAST_RUN_PATH.write_text(
        json.dumps(run_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    table_stream = sys.stderr if args.json else sys.stdout
    print("== harness check ==", file=table_stream)
    print(f"提交     : {ctx['commit_sha'] or '(无 git 上下文)'}", file=table_stream)
    print(f"diff 标签: {tags if tags else '(无)'}", file=table_stream)
    print_notes(notes, table_stream)
    print(file=table_stream)
    render_gate_table(results)
    print(file=table_stream)
    counts: dict[str, int] = {}
    for record in results:
        counts[record["status"]] = counts.get(record["status"], 0) + 1
    summary = ", ".join(f"{k}={v}" for k, v in sorted(counts.items()))
    print(f"统计: {summary or '(无门禁)'}", file=table_stream)
    for reason in verdict_reasons:
        print(f"  - {reason}", file=table_stream)
    print(f"结果文件: {rel_path(LAST_RUN_PATH)}", file=table_stream)
    print(f"VERDICT: {verdict}", file=table_stream)

    if args.json:
        print(json.dumps(run_doc, ensure_ascii=False, indent=2))

    # 退出码语义：check 反映**已执行**门禁的结果；合并裁决看 merge-decision。
    #   0 = 无门禁失败；1 = 有门禁 FAIL；2 = 有门禁 UNVERIFIED/BLOCKED。
    #   --strict-exit ⇒ 退出码直接镜像裁决（0 当且仅当 PASS）。
    if args.strict_exit:
        return 0 if verdict == VERDICT_PASS else (1 if verdict == VERDICT_FAIL else 2)
    if any(r["status"] == S_FAIL for r in results):
        return 1
    if any(r["status"] in (S_UNVERIFIED, S_BLOCKED) for r in results):
        return 2
    return 0


# --------------------------------------------------------------------------- attest

def cmd_attest(args: argparse.Namespace) -> int:
    if not LAST_RUN_PATH.is_file():
        print(
            f"error: 找不到 {rel_path(LAST_RUN_PATH)} ⇒ 请先跑 `python3 scripts/harness.py check`。",
            file=sys.stderr,
        )
        return 2
    try:
        run_doc = json.loads(LAST_RUN_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        print(f"error: 读取 {rel_path(LAST_RUN_PATH)} 失败：{exc}", file=sys.stderr)
        return 2
    if not isinstance(run_doc, dict):
        print(f"error: {rel_path(LAST_RUN_PATH)} 内容不是合法 JSON 对象。", file=sys.stderr)
        return 2

    ctx = collect_context(None)
    head = ctx["commit_sha"]
    recorded = run_doc.get("commit_sha")
    if not head:
        print("error: no git context ⇒ 无法把证据绑定到 SHA。", file=sys.stderr)
        return 2
    if recorded != head:
        print(
            "error: stale evidence（证据与当前 HEAD 不一致）\n"
            f"       recorded: {recorded}\n"
            f"       HEAD    : {head}\n"
            "       ⇒ 请重跑 `python3 scripts/harness.py check` 后再 attest。",
            file=sys.stderr,
        )
        return 2

    task_id = args.task_id or "adhoc"
    agent = args.agent or os.environ.get("APEIRETH_AGENT") or "unknown"
    bundle_dir = ARTIFACT_ROOT / head
    bundle_dir.mkdir(parents=True, exist_ok=True)

    gates = run_doc.get("gates") or []
    evidence_gates = []
    for gate in gates:
        if not isinstance(gate, dict):
            continue
        evidence_gates.append(
            {
                "id": gate.get("id"),
                "status": gate.get("status"),
                "command": gate.get("command"),
                "exit_code": gate.get("exit_code"),
                "duration_s": gate.get("duration_s"),
                # sha256 覆盖**完整**采集输出（不是尾部）；尾部另给一个字段
                "sha256": gate.get("output_sha256"),
                "tail_sha256": gate.get("tail_sha256"),
                "required": gate.get("required"),
                "reason": gate.get("reason"),
            }
        )

    task_doc = {
        "schema": TASK_SCHEMA,
        "task_id": task_id,
        "agent": agent,
        "commit_sha": run_doc.get("commit_sha"),
        "tree_sha": run_doc.get("tree_sha"),
        "base_sha": run_doc.get("base_sha"),
        "started_at": run_doc.get("started_at"),
        "finished_at": run_doc.get("finished_at"),
        "created_at": now_iso(),
        "verdict": run_doc.get("verdict"),
    }

    evidence_doc = {
        "schema": EVIDENCE_SCHEMA,
        "task_id": task_id,
        "agent": agent,
        "commit_sha": run_doc.get("commit_sha"),
        "tree_sha": run_doc.get("tree_sha"),
        "base_sha": run_doc.get("base_sha"),
        "created_at": now_iso(),
        "gates": evidence_gates,
    }

    summary_md = render_summary(run_doc, task_doc)

    (bundle_dir / "task.json").write_text(
        json.dumps(task_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (bundle_dir / "gate-results.json").write_text(
        json.dumps(run_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (bundle_dir / "evidence-index.json").write_text(
        json.dumps(evidence_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (bundle_dir / "summary.md").write_text(summary_md, encoding="utf-8")

    print(f"证据包已写出: {rel_path(bundle_dir)}")
    for name in ("task.json", "gate-results.json", "evidence-index.json", "summary.md"):
        path = bundle_dir / name
        print(f"  - {name} ({path.stat().st_size} bytes)")
    print(f"任务: {task_id} | 代理: {agent} | 提交: {head}")
    print(f"VERDICT: {run_doc.get('verdict')}")
    return 0


def render_summary(run_doc: dict[str, Any], task_doc: dict[str, Any]) -> str:
    """人读摘要：每个门禁一个状态，SKIPPED 明确写成 SKIPPED（**不是通过**）。"""
    gates = run_doc.get("gates") or []
    lines: list[str] = []
    lines.append("# Harness 证据摘要")
    lines.append("")
    lines.append(f"- 任务：`{task_doc.get('task_id')}`")
    lines.append(f"- 代理：`{task_doc.get('agent')}`")
    lines.append(f"- 提交：`{run_doc.get('commit_sha')}`")
    lines.append(f"- 树：`{run_doc.get('tree_sha')}`")
    lines.append(f"- 基线：`{run_doc.get('base_sha')}`")
    lines.append(f"- 起：`{run_doc.get('started_at')}`　止：`{run_doc.get('finished_at')}`")
    lines.append(f"- diff 标签：`{', '.join(run_doc.get('diff_tags') or []) or '(无)'}`")
    lines.append(f"- 变更文件：{run_doc.get('changed_file_count', len(run_doc.get('changed_files') or []))} 个")
    lines.append(f"- **裁决：{run_doc.get('verdict')}**")
    lines.append("")
    lines.append("## 门禁结果")
    lines.append("")
    lines.append("| 门禁 | 状态 | 必需 | 退出码 | 耗时(s) | 说明 |")
    lines.append("|---|---|---|---|---|---|")
    for gate in gates:
        status = str(gate.get("status"))
        exit_code = gate.get("exit_code")
        note = str(gate.get("reason") or "").replace("\n", " ").replace("|", "\\|")
        if status == S_SKIPPED:
            note = (note + "　⚠ SKIPPED ≠ PASS（未跑过，不算通过）").strip()
        lines.append(
            "| `{id}` | **{status}** | {req} | {ec} | {sec} | {note} |".format(
                id=gate.get("id"),
                status=status,
                req="是" if gate.get("required") else "否",
                ec="-" if exit_code is None else str(exit_code),
                sec=f"{float(gate.get('duration_s') or 0):.2f}",
                note=note or "—",
            )
        )
    lines.append("")
    reasons = run_doc.get("verdict_reasons") or []
    if reasons:
        lines.append("## 裁决依据")
        lines.append("")
        for reason in reasons:
            lines.append(f"- {reason}")
        lines.append("")

    failed = [g for g in gates if g.get("status") == S_FAIL]
    if failed:
        lines.append("## 失败门禁输出尾部")
        lines.append("")
        for gate in failed:
            lines.append(f"### {gate.get('id')}（退出码 {gate.get('exit_code')}）")
            lines.append("")
            lines.append("```")
            lines.append(str(gate.get("output_tail") or "").rstrip())
            lines.append("```")
            lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("> 口径：**SKIPPED 不等于 PASS**；**UNVERIFIED 不能进入合并**；")
    lines.append("> 只有 `PASS`（或已批准的 `WAIVED`）才允许合并。")
    lines.append("")
    return "\n".join(lines)


# ------------------------------------------------------------------- merge-decision

def cmd_merge_decision(args: argparse.Namespace) -> int:
    if args.evidence_dir:
        evidence_path = Path(args.evidence_dir).expanduser()
        if not evidence_path.is_absolute():
            evidence_path = (Path.cwd() / evidence_path).resolve()
        gate_results = evidence_path / "gate-results.json"
        source = gate_results
    else:
        gate_results = LAST_RUN_PATH
        source = gate_results

    if not gate_results.is_file():
        print(
            f"error: 找不到证据文件 {gate_results} ⇒ 没有可裁决的证据（请先跑 check / attest）。",
            file=sys.stderr,
        )
        return 2
    try:
        run_doc = json.loads(gate_results.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        print(f"error: 读取 {gate_results} 失败：{exc}", file=sys.stderr)
        return 2
    if not isinstance(run_doc, dict):
        print(f"error: {gate_results} 内容不是合法 JSON 对象。", file=sys.stderr)
        return 2

    ctx = collect_context(None)
    head = ctx["commit_sha"]
    recorded = run_doc.get("commit_sha")
    gates = run_doc.get("gates") or []
    if not isinstance(gates, list):
        gates = []

    blocking: list[str] = []
    info: list[str] = []
    stale = False
    if not head:
        blocking.append("no git context（无法把证据绑定到 SHA）")
    elif recorded != head:
        stale = True
        blocking.append(f"stale evidence: recorded {recorded} != HEAD {head}")

    stored_reasons = run_doc.get("verdict_reasons")
    if isinstance(stored_reasons, list):
        for item in stored_reasons:
            text = str(item)
            if text not in blocking and text not in info:
                info.append(text)

    verdict, reasons = evaluate_verdict(gates, blocking, info)

    print("== merge-decision ==")
    print(f"证据文件: {source}")
    print(f"记录提交: {recorded}")
    print(f"当前 HEAD: {head}")
    print(f"门禁数  : {len(gates)}")
    print()
    render_gate_table([g for g in gates if isinstance(g, dict)])
    print()
    if reasons:
        print("裁决依据:")
        for reason in reasons:
            print(f"  - {reason}")
    else:
        print("裁决依据: 所有 required 门禁均为 PASS，无 FAIL/UNVERIFIED/BLOCKED。")
    print()
    if stale:
        print("提示: 证据不是当前 HEAD 的 ⇒ 视为 stale evidence ⇒ 不得据此合并。")
    print(f"VERDICT: {verdict}")
    return 0 if verdict == VERDICT_PASS else 1


# --------------------------------------------------------------------------- CLI

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="scripts/harness.py",
        description=(
            "统一验证入口：分类 → 执行门禁 → 落盘证据 → 合并裁决。"
            "（SKIPPED 不等于 PASS；UNVERIFIED 不能进入合并。）"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "子命令:\n"
            "  plan            分类当前 diff，打印哪些门禁应当运行（不执行）\n"
            "  check           执行门禁，写 scripts/harness-last-run.json\n"
            "  attest          写 SHA 绑定的证据包 artifacts/harness/<sha>/\n"
            "  merge-decision  汇总成一个裁决；退出码 0 当且仅当裁决为 PASS\n"
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_plan = sub.add_parser("plan", help="分类当前 diff 并打印门禁计划（不执行）")
    p_plan.add_argument("--base", default=None, help="基线 SHA（默认 HEAD~1）")
    p_plan.add_argument("--only", default=None, help="只考虑这些门禁 id（逗号分隔）")
    p_plan.add_argument(
        "--skip-gate", default=None, help="把这些门禁 id 标成显式跳过（逗号分隔）"
    )
    p_plan.set_defaults(func=cmd_plan)

    p_check = sub.add_parser("check", help="执行门禁并落盘机器可读结果")
    p_check.add_argument("--base", default=None, help="基线 SHA（默认 HEAD~1）")
    p_check.add_argument("--only", default=None, help="只跑这些门禁 id（逗号分隔）")
    p_check.add_argument(
        "--skip-gate",
        default=None,
        help=(
            "把这些门禁 id 标成显式跳过（SKIPPED，绝不算通过）"
            "—— 用于本地避开会触发 cargo 编译的门禁"
        ),
    )
    p_check.add_argument("--timeout", type=int, default=None, help="覆盖门禁超时秒数")
    p_check.add_argument("--json", action="store_true", help="把结果 JSON 打到 stdout")
    p_check.add_argument(
        "--strict-exit",
        action="store_true",
        help="退出码直接镜像裁决（0 当且仅当 PASS）；默认只反映已执行门禁的结果",
    )
    p_check.set_defaults(func=cmd_check)

    p_attest = sub.add_parser("attest", help="写 SHA 绑定的证据包")
    p_attest.add_argument("--task-id", default=None, help="任务 id（默认 adhoc）")
    p_attest.add_argument("--agent", default=None, help="代理名（默认取 $APEIRETH_AGENT，再默认 unknown）")
    p_attest.set_defaults(func=cmd_attest)

    p_merge = sub.add_parser("merge-decision", help="汇总成一个合并裁决")
    p_merge.add_argument(
        "--evidence-dir", default=None, help="证据目录（含 gate-results.json）；默认用 scripts/harness-last-run.json"
    )
    p_merge.set_defaults(func=cmd_merge_decision)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except KeyboardInterrupt:
        print("error: 被中断。", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
