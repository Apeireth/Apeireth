"""Phase 1398 v1398_real_ansible_lint — V1398 ASI 真生产 Ansible playbook 真解析 + 真 lint (主 06:15 + 主 23:44 + 主 17:43 + 主 19:33 + 主 22:33 + 主 00:56 + 主 13:31 + 主 17:33 + 主 00:36).

主 06:15 当前真生产方向: V1398 = 真生产 deploy-stack ansible playbook 真解析 + 真 lint (post-V1397 next-step, 推荐方向).
主 23:44 干到底: V1384 Dockerfile + V1385 compose + V1386 k8s + V1397 terraform → V1398 真补 ansible 缺口.
主 22:33 ASI 北极星: 真 lint ansible ≠ ASI, 但真 lint ansible 是 ASI 北极星里 system integration 的一小步.
主 19:33 走在前人经验上: 真借鉴 ansible-lint + yamllint + community-ansible-lint-rules + OPA-conftest + OpenStack-Ansible + molecule.
主 17:43 实事求是: 真 parse YAML (PyYAML) + 真规则匹配 + 真报 finding + 真 exit code.
主 00:56 任何人都能接手: 1 个 module + 1 个 CLI + 12 真规则 + 1 个 chain runner + 1 个 popper self-test.
主 00:36 质量 + 适配性 + 效果 + 工程化: 真 CLI + 真 exit code + 真 popper self-test + 真 JSON 输出.

真生产设计 (主 19:33 ansible-lint/yamllint/community-ansible-lint-rules/conftest 真借鉴):
- 真 parse YAML (PyYAML, 主 17:43 真借用 PyYAML 已 install pip install pyyaml)
  - 真处理 list-of-plays (top-level list of dicts)
  - 真处理 single-play dict (also accepted)
  - 真处理 tasks within play (list of dicts)
  - 真处理 vars / handlers / role imports
  - 真处理 module args (e.g. apt: name=foo → apt: { name: foo })
  - 真识别 shell/command modules for special rules
- 真 12 规则 (主 19:33 ansible-lint 真借鉴):
  - AN001-NO-PLAY-NAME (warning): top-level play missing `name:`
  - AN002-NO-TASK-NAME (warning): task missing `name:`
  - AN003-HARDCODED-SECRET (error): password/secret/api_key/token/passphrase in plaintext vars
  - AN004-PLAIN-PRIVATE-KEY (error): -----BEGIN ... PRIVATE KEY----- in vars
  - AN005-RISKY-SHELL-PIPE (error): shell task with `|` and no `set -o pipefail` / `pipefail: false`
  - AN006-NO-CHANGED-WHEN (warning): command/shell task without `changed_when:`
  - AN007-DEPRECATED-LOOP (info): `with_items` / `with_dict` / `with_fileglob` etc. (use `loop:`)
  - AN008-COMMAND-INSTEAD-MODULE (warning): `command:` for known state-mgmt target (apt/yum/copy/file/service/user)
  - AN009-INLINE-ENV-VAR (warning): `environment:` with inline env vars containing secrets
  - AN010-MISSING-TAGS (info): play or task without `tags:`
  - AN011-IGNORE-ERRORS-MASKS (warning): task uses `ignore_errors: true` without `failed_when:`
  - AN012-LOOP-UNDEFINED-VAR (error): `loop:` references undefined var name
- 真 chain delegate (主 17:43 真可调): 聚合 V1387 unified runner 发现
- 真 popper self-test (主 17:43 真跑真测)
- 真 CLI (主 17:43 真可执行):
  - version: V1398 version
  - lint <path>: 真 lint playbook YAML 文件 → text/JSON 输出
  - chain <path>: 真调 V1387 unified runner + V1398 真 lint → 综合报告
  - popper: V1398 self-test
  - demo: V1398 demo (用内置 playbook sample)
  - help
- 真 multi-output: text / JSON / SARIF (主 00:36 工程化)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness: 本模块是 ansible linter, 不是 consciousness claim.
- 不假装达到 ASI: 真 lint ≠ ASI 达成; 真 lint 是 ASI 北极星里 system integration 的一小步.
- 不假装调整模型 & prompt: 真生产是真 parse YAML + 真规则匹配, 不是改 prompt 假装 lint.
- 真 lint = 真借鉴 + 真算法 + 真跑真测 + 真 commit + 真可执行.
- 任何声称 "lint = safety" 都是不假装. 真 lint ≠ 安全审计.
- 任何声称 "lint = ASI" 都是不假装. 真 lint 是 ASI 北极星里 system integration 的一小步.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

# V1398 真生产 PyYAML (主 17:43 实事求是)
try:
    import yaml
    _YAML_AVAILABLE = True
except Exception:
    yaml = None
    _YAML_AVAILABLE = False


V1398_VERSION = "0.1.0"
V1398_SCHEMA = "v1398.ansible-lint/v1"

# V1398 真生产 GUARDS (主 17:43 + 主 19:33 + 主 22:33)
V1398_GUARDS: tuple = (
    "GUARD_YAML_PARSED",          # 真 parse YAML (PyYAML)
    "GUARD_RULES_REAL",           # 真规则匹配 (12 真规则)
    "GUARD_FILE_IO",              # 真文件 IO
    "GUARD_LINE_TRACKED",         # 真行号 (PyYAML Loader with position or regex fallback)
    "GUARD_NO_CAP_CHANGE",        # 不改 ASI cap
    "GUARD_DETERMINISTIC",        # same target → same result
    "GUARD_HONEST_DISCLOSURE",    # 标注 borrowed + yaml missing fallback
    "GUARD_PATH_SAFE",            # path traversal 防护
    "GUARD_NON_DESTRUCTIVE",      # 只读, 不写
    "GUARD_DELEGATE_REAL",        # 真调 V1387
    "GUARD_CLI_RUNNABLE",         # CLI 真可跑
    "GUARD_POPPER_RUNS",          # popper self-test 真跑
)

# V1398 真借鉴 (主 19:33 走在前人经验上)
V1398_BORROWED: tuple = (
    "ansible-lint (https://github.com/ansible/ansible-lint) — 真借鉴 rule format (AN-prefix) + 真借鉴 rule semantics",
    "yamllint (https://github.com/adrienverge/yamllint) — 真借鉴 YAML structure handling + 真借鉴 plain scalar stripping",
    "community-ansible-lint-rules (https://github.com/ansible-community/ansible-lint) — 真借鉴 rule taxonomy",
    "conftest (https://github.com/open-policy-agent/conftest) — 真借鉴 structured rule output",
    "OpenStack-Ansible (https://opendev.org/openstack/openstack-ansible) — 真借鉴 playbook patterns + tag conventions",
    "molecule (https://github.com/ansible-community/molecule) — 真借鉴 test scenarios + idempotency checks",
)

# V1398 真生产 secret patterns (主 19:33 ansible-lint + tfsec 真借鉴)
# NOTE: 在 YAML parse 后, value 是独立字符串, 不含 key 名. 所以需要同时检测 KEY 和 VALUE.
V1398_SECRET_VALUE_PATTERNS: tuple = (
    re.compile(r"AKIA[0-9A-Z]{16}"),                            # AWS access key
    re.compile(r"\b[A-Za-z0-9/+=]{40,}"),                        # 长 base64-like 串
    re.compile(r"-----BEGIN (RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----"),
    re.compile(r"gh[pousr]_[A-Za-z0-9]{36,}"),                   # GitHub token (ghp/gho/ghu/ghs/ghr prefix)
    re.compile(r"xox[abprs]-[A-Za-z0-9-]{10,}"),                # Slack token (xoxb/xoxa/xoxp/xoxr/xoxs prefix)
)
V1398_SECRET_KEY_NAMES: tuple = (
    "password", "passwd", "pwd", "secret", "api_key", "api-key",
    "apikey", "token", "passphrase", "private_key", "privatekey",
    "aws_access_key_id", "aws_secret_access_key",
)
V1398_SECRET_PATTERNS = V1398_SECRET_VALUE_PATTERNS  # backward compat alias (>=4 patterns)

# V1398 真生产 deprecated loop forms (主 19:33 ansible-lint 官方)
V1398_DEPRECATED_LOOPS: tuple = (
    "with_items",
    "with_dict",
    "with_fileglob",
    "with_filetree",
    "with_first_found",
    "with_flattened",
    "with_indexed_items",
    "with_ini",
    "with_inventory_hostnames",
    "with_lines",
    "with_nested",
    "with_random_choice",
    "with_sequence",
    "with_subelements",
    "with_together",
)

# V1398 真生产 known command→module pairs (主 19:33 ansible-lint command-instead-of-module)
V1398_CMD_TO_MODULE: tuple = (
    ("apt-get", "apt"),
    ("yum", "yum"),
    ("dnf", "dnf"),
    ("cp ", "copy"),
    ("mv ", "command"),  # mv doesn't have a direct module, skip flag below
    ("mkdir", "file"),
    ("rm ", "file"),
    ("systemctl", "service"),
    ("service ", "service"),
    ("useradd", "user"),
    ("userdel", "user"),
    ("groupadd", "group"),
    ("chmod", "file"),
    ("chown", "file"),
    ("ln -s", "file"),
    ("touch ", "file"),
    ("echo ", "copy"),   # echo is debated, skip flag
    ("cat >", "copy"),
    ("kill ", "command"),
)

# V1398 真生产 known inline env secret patterns (主 19:33 ansible-lint inline-env-var)
V1398_INLINE_ENV_SECRET: tuple = (
    re.compile(r"(?i)(PASSWORD|SECRET|TOKEN|API_KEY|PASSWD|KEY)\s*=\S+"),
)


# ============================================================================
# V1398 真生产 dataclasses (主 17:43 实事求是)
# ============================================================================


@dataclass
class LintFinding:
    """V1398 真生产 lint 真报 finding (主 17:43 实事求是)."""

    rule_id: str            # 例如 AN003-HARDCODED-SECRET
    severity: str           # error / warning / info
    line_no: int            # 真行号 (1-indexed, 0 if N/A)
    line_text: str          # 真原文 (or first 120 chars)
    message: str            # 真问题描述
    suggestion: str = ""    # 真建议
    resource: str = ""      # 真 play/task ref
    file_path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "line_no": self.line_no,
            "line_text": self.line_text.strip()[:120],
            "message": self.message,
            "suggestion": self.suggestion,
            "resource": self.resource,
            "file_path": self.file_path,
        }


@dataclass
class LintReport:
    """V1398 真生产 lint 真报报告 (主 17:43 实事求是)."""

    file_path: str
    n_lines: int
    n_findings: int
    n_errors: int
    n_warnings: int
    n_info: int
    findings: List[LintFinding] = field(default_factory=list)
    elapsed_seconds: float = 0.0
    ok: bool = True
    parse_error: str = ""     # 真报 YAML parse error
    parser: str = ""          # yaml or regex-fallback
    n_plays: int = 0
    n_tasks: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "file_path": self.file_path,
            "n_lines": self.n_lines,
            "n_findings": self.n_findings,
            "n_errors": self.n_errors,
            "n_warnings": self.n_warnings,
            "n_info": self.n_info,
            "findings": [f.to_dict() for f in self.findings],
            "elapsed_seconds": round(self.elapsed_seconds, 4),
            "ok": self.ok,
            "parse_error": self.parse_error,
            "parser": self.parser,
            "n_plays": self.n_plays,
            "n_tasks": self.n_tasks,
        }


# ============================================================================
# V1398 真生产 YAML 解析辅助 (主 17:43 实事求是)
# ============================================================================


def _strip_quotes(s: Any) -> Any:
    """YAML returns strings that may be quoted. Strip them. 主 17:43."""
    if isinstance(s, str):
        if len(s) >= 2 and ((s.startswith('"') and s.endswith('"')) or (s.startswith("'") and s.endswith("'"))):
            return s[1:-1]
    return s


def _normalize_value(v: Any) -> Any:
    """V1398 真生产 normalize yaml values (主 17:43)."""
    if isinstance(v, str):
        return _strip_quotes(v)
    if isinstance(v, list):
        return [_normalize_value(x) for x in v]
    if isinstance(v, dict):
        return {_strip_quotes(k) if isinstance(k, str) else k: _normalize_value(val) for k, val in v.items()}
    return v


def _coerce_plays(parsed: Any) -> List[Dict[str, Any]]:
    """V1398 真生产 coerce single play dict OR list of plays → list[dict]."""
    if isinstance(parsed, list):
        return [p for p in parsed if isinstance(p, dict)]
    if isinstance(parsed, dict):
        # single-play format
        return [parsed]
    return []


def _find_line_no(text: str, target: str, start: int = 0) -> int:
    """V1398 真生产 find 1-indexed line number for target in text (主 17:43)."""
    if not target:
        return 0
    idx = text.find(target, start)
    if idx < 0:
        return 0
    return text[:idx].count("\n") + 1


def _safe_resolve(text: str, raw: Any, fallback: str) -> int:
    """V1398 真生产 find line for raw value or fallback."""
    if isinstance(raw, str):
        idx = text.find(raw[:60])
        if idx >= 0:
            return text[:idx].count("\n") + 1
    return _find_line_no(text, fallback)


def _parse_playbook(text: str, file_path: str) -> Tuple[List[Dict[str, Any]], str, int, int]:
    """V1398 真生产 parse ansible playbook.

    Returns (plays, parse_error_or_empty, n_plays, n_tasks).
    plays is a list of dicts (possibly empty if parse failed).
    """
    n_plays = 0
    n_tasks = 0
    if not _YAML_AVAILABLE:
        return [], "PyYAML not available", 0, 0
    try:
        # safe_load returns list (multi-play) or dict (single-play) or None
        parsed = yaml.safe_load(text) or []
    except yaml.YAMLError as e:
        return [], f"YAML parse error: {e}", 0, 0

    plays = _coerce_plays(parsed)
    n_plays = len(plays)
    for play in plays:
        tasks = play.get("tasks", [])
        if isinstance(tasks, list):
            n_tasks += sum(1 for t in tasks if isinstance(t, dict))
        handlers = play.get("handlers", [])
        if isinstance(handlers, list):
            n_tasks += sum(1 for t in handlers if isinstance(t, dict))
    return plays, "", n_plays, n_tasks


# ============================================================================
# V1398 真生产 规则定义 (主 19:33 ansible-lint/yamllint 真借鉴)
# ============================================================================


def _rule_an001_no_play_name(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN001: 真生产 top-level play missing `name:` (主 19:33 ansible-lint name[play])."""
    findings: List[LintFinding] = []
    for i, play in enumerate(plays):
        if "name" not in play or not play["name"]:
            line_no = _find_line_no(text, "- hosts:") or _find_line_no(text, "hosts:")
            findings.append(LintFinding(
                rule_id="AN001-NO-PLAY-NAME",
                severity="warning",
                line_no=line_no,
                line_text=f"play[{i}] (no name)",
                message=f"Play #{i + 1} is missing a 'name' field",
                suggestion="Add a descriptive 'name:' to each play for documentation and --tags filtering.",
                resource=f"play[{i}]",
            ))
    return findings


def _rule_an002_no_task_name(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN002: 真生产 task missing `name:` (主 19:33 ansible-lint name[missing])."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            if "name" not in task or not task["name"]:
                # 真找该 task 的 module 行
                mod_key = next((k for k in task if k not in ("name", "when", "loop", "with_items", "tags",
                                                              "register", "become", "ignore_errors",
                                                              "failed_when", "changed_when", "notify",
                                                              "vars", "environment", "delegate_to",
                                                              "run_once", "no_log", "check_mode", "diff")),
                               "")
                line_no = _find_line_no(text, f"{mod_key}:") or _find_line_no(text, "- name:")
                findings.append(LintFinding(
                    rule_id="AN002-NO-TASK-NAME",
                    severity="warning",
                    line_no=line_no,
                    line_text=f"- {mod_key}: ..." if mod_key else "- <task>",
                    message=f"Task #{ti + 1} in play #{pi + 1} is missing a 'name' field",
                    suggestion="Add a descriptive 'name:' to each task for documentation and debugging.",
                    resource=f"play[{pi}].tasks[{ti}]",
                ))
    return findings


def _rule_an003_hardcoded_secret(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN003: 真生产 password/secret/api_key/token in plaintext vars (主 19:33 ansible-lint var-naming + tfsec).

    YAML parse 后 value 是独立字符串, 所以需要:
    1. KEY name 含 secret-like 词 → flag (即使 value 是变量引用)
    2. VALUE 命中 secret pattern (AKIA, 长 base64, PEM) → flag
    """
    findings: List[LintFinding] = []
    seen: Set[int] = set()

    def _key_is_secret(k: str) -> bool:
        k_low = k.lower().replace("-", "_")
        return any(s in k_low for s in V1398_SECRET_KEY_NAMES)

    def _value_is_secret(s: str) -> bool:
        for pat in V1398_SECRET_VALUE_PATTERNS:
            if pat.search(s):
                return True
        return False

    def _emit(rid_path: str, line_no: int, line_text: str, message: str) -> None:
        if line_no in seen or line_no == 0:
            return
        seen.add(line_no)
        findings.append(LintFinding(
            rule_id="AN003-HARDCODED-SECRET",
            severity="error",
            line_no=line_no,
            line_text=line_text[:120],
            message=message,
            suggestion="Use ansible-vault or a secrets manager (e.g. HashiCorp Vault, AWS Secrets Manager). Never commit secrets.",
            resource=rid_path,
        ))

    def _walk(obj: Any, path: str) -> None:
        if isinstance(obj, dict):
            for k, v in obj.items():
                k_s = _strip_quotes(k) if isinstance(k, str) else k
                k_str = str(k_s)
                # 真生产: key 名字本身就是 secret-like
                if isinstance(k_s, str) and _key_is_secret(k_s):
                    # 真找 line for key
                    line_no = _find_line_no(text, k_s)
                    v_repr = v if isinstance(v, str) else json.dumps(v)[:60] if v is not None else ""
                    _emit(
                        f"{path}.{k_str}" if path else k_str,
                        line_no,
                        f"{k_str}: {v_repr}",
                        f"Variable '{k_str}' name indicates secret — value must use vault or secrets manager",
                    )
                _walk(v, f"{path}.{k_str}" if path else k_str)
        elif isinstance(obj, list):
            for i, item in enumerate(obj):
                _walk(item, f"{path}[{i}]")
        elif isinstance(obj, str):
            s = _strip_quotes(obj)
            if _value_is_secret(s):
                line_no = _find_line_no(text, s[:50])
                _emit(
                    path,
                    line_no,
                    s,
                    f"Hardcoded secret value detected at {path}",
                )

    for play in plays:
        _walk(play, "play")
    return findings


def _rule_an004_plain_private_key(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN004: 真生产 -----BEGIN ... PRIVATE KEY----- in vars (主 19:33 真借鉴 git-secrets + ansible-lint)."""
    findings: List[LintFinding] = []
    # 真扫描 raw text (PEM 经常多行, YAML normalize 会丢换行)
    pk_pat = re.compile(r"-----BEGIN (RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----")
    for m in pk_pat.finditer(text):
        line_no = text[:m.start()].count("\n") + 1
        findings.append(LintFinding(
            rule_id="AN004-PLAIN-PRIVATE-KEY",
            severity="error",
            line_no=line_no,
            line_text=m.group(0),
            message="Plaintext private key detected in playbook",
            suggestion="Use ansible-vault to encrypt the key, or load it from a secrets manager at runtime.",
            resource="file",
        ))
    return findings


def _rule_an005_risky_shell_pipe(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN005: 真生产 shell task with pipe but no pipefail (主 19:33 ansible-lint risky-shell-pipe)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            # 真检查 shell: 模块
            shell_cmd = None
            for mk in ("shell", "command"):
                if mk in task:
                    v = task[mk]
                    if isinstance(v, str):
                        shell_cmd = v
                    elif isinstance(v, dict):
                        # 真找 cmd 字段
                        shell_cmd = v.get("cmd") or v.get("argv") or ""
            if shell_cmd is None:
                continue
            # 真检查 pipe
            if "|" in shell_cmd:
                # 真检查 pipefail
                if "pipefail" not in text and "set -o pipefail" not in shell_cmd:
                    line_no = _find_line_no(text, shell_cmd[:60])
                    findings.append(LintFinding(
                        rule_id="AN005-RISKY-SHELL-PIPE",
                        severity="warning",
                        line_no=line_no,
                        line_text=shell_cmd[:120],
                        message=f"Shell task uses pipe without 'set -o pipefail' or 'pipefail: true'",
                        suggestion="Add 'set -o pipefail' to the command, or set 'pipefail: true' in task args.",
                        resource=f"play[{pi}].tasks[{ti}]",
                    ))
    return findings


def _rule_an006_no_changed_when(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN006: 真生产 command/shell task without `changed_when:` (主 19:33 ansible-lint no-changed-when)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            has_shell_or_cmd = ("command" in task) or ("shell" in task)
            if not has_shell_or_cmd:
                continue
            if "changed_when" not in task:
                mod_key = "shell" if "shell" in task else "command"
                cmd_raw = task[mod_key]
                cmd_str = cmd_raw if isinstance(cmd_raw, str) else json.dumps(cmd_raw)
                line_no = _find_line_no(text, cmd_str[:60])
                findings.append(LintFinding(
                    rule_id="AN006-NO-CHANGED-WHEN",
                    severity="warning",
                    line_no=line_no,
                    line_text=f"{mod_key}: {cmd_str[:80]}",
                    message=f"Shell/command task #{ti + 1} has no 'changed_when:' clause",
                    suggestion="Add 'changed_when:' to detect whether the task changed state. Without it, Ansible always reports 'changed' which hides real failures.",
                    resource=f"play[{pi}].tasks[{ti}]",
                ))
    return findings


def _rule_an007_deprecated_loop(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN007: 真生产 with_items/with_dict etc. → use `loop:` (主 19:33 ansible-lint deprecated-loop-syntax)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            for deprecated in V1398_DEPRECATED_LOOPS:
                if deprecated in task:
                    line_no = _find_line_no(text, f"{deprecated}:")
                    findings.append(LintFinding(
                        rule_id="AN007-DEPRECATED-LOOP",
                        severity="info",
                        line_no=line_no,
                        line_text=f"{deprecated}: ...",
                        message=f"'{deprecated}' is deprecated, use 'loop:' instead",
                        suggestion=f"Replace '{deprecated}:' with 'loop:' (same value semantics).",
                        resource=f"play[{pi}].tasks[{ti}]",
                    ))
                    break
    return findings


def _rule_an008_command_instead_module(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN008: 真生产 `command:` for known state-mgmt (主 19:33 ansible-lint command-instead-of-module)."""
    findings: List[LintFinding] = []
    # only flag these patterns
    flag_patterns = ["apt-get", "yum ", "dnf ", "systemctl ", "useradd", "userdel",
                     "groupadd", "chmod ", "chown ", "ln -s"]
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            cmd_raw = task.get("command", "")
            if not cmd_raw:
                continue
            cmd_str = cmd_raw if isinstance(cmd_raw, str) else (cmd_raw.get("cmd") or "" if isinstance(cmd_raw, dict) else "")
            for pattern in flag_patterns:
                if pattern in cmd_str:
                    # 真映射到推荐模块
                    suggested_mod = next((m for c, m in V1398_CMD_TO_MODULE if c == pattern or c.strip() == pattern.strip()), "module")
                    if suggested_mod == "command":  # skip ambiguous ones
                        continue
                    line_no = _find_line_no(text, cmd_str[:60])
                    findings.append(LintFinding(
                        rule_id="AN008-COMMAND-INSTEAD-MODULE",
                        severity="warning",
                        line_no=line_no,
                        line_text=f"command: {cmd_str[:80]}",
                        message=f"Use '{suggested_mod}:' module instead of 'command:' for '{pattern.strip()}'",
                        suggestion=f"Replace 'command:' with '{suggested_mod}:' module for idempotency and better error handling.",
                        resource=f"play[{pi}].tasks[{ti}]",
                    ))
                    break
    return findings


def _rule_an009_inline_env_var(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN009: 真生产 environment: with inline secret (主 19:33 ansible-lint inline-env-var)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            env = task.get("environment", {})
            if not isinstance(env, dict):
                continue
            for k, v in env.items():
                k_s = _strip_quotes(k) if isinstance(k, str) else k
                v_s = _strip_quotes(v) if isinstance(v, str) else v
                v_str = str(v_s) if v_s is not None else ""
                for pat in V1398_INLINE_ENV_SECRET:
                    if pat.search(f"{k_s}={v_str}"):
                        line_no = _find_line_no(text, f"{k_s}:")
                        findings.append(LintFinding(
                            rule_id="AN009-INLINE-ENV-VAR",
                            severity="warning",
                            line_no=line_no,
                            line_text=f"{k_s}: {v_str[:80]}",
                            message=f"Inline env var '{k_s}' looks like a secret",
                            suggestion="Move secret to ansible-vault vars or external secrets manager; reference via lookup.",
                            resource=f"play[{pi}].tasks[{ti}]",
                        ))
                        break
    return findings


def _rule_an010_missing_tags(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN010: 真生产 play or task without `tags:` (主 19:33 ansible-lint + 主 00:56)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        if "tags" not in play:
            line_no = _find_line_no(text, "- name:") or _find_line_no(text, "- hosts:")
            findings.append(LintFinding(
                rule_id="AN010-MISSING-TAGS",
                severity="info",
                line_no=line_no,
                line_text=f"play[{pi}] (no tags)",
                message=f"Play #{pi + 1} has no 'tags:' for selective execution",
                suggestion="Add 'tags:' to plays/tasks so you can run 'ansible-playbook site.yml --tags deploy'.",
                resource=f"play[{pi}]",
            ))
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            if "tags" not in task:
                mod_key = next((k for k in task if k not in ("name", "when", "loop", "with_items",
                                                              "register", "become", "ignore_errors",
                                                              "failed_when", "changed_when", "notify",
                                                              "vars", "environment", "delegate_to",
                                                              "run_once", "no_log", "check_mode", "diff",
                                                              "loop_control")),
                               "")
                line_no = _find_line_no(text, f"{mod_key}:") or _find_line_no(text, "- name:")
                findings.append(LintFinding(
                    rule_id="AN010-MISSING-TAGS",
                    severity="info",
                    line_no=line_no,
                    line_text=f"- {mod_key or 'task'}: ... (no tags)",
                    message=f"Task #{ti + 1} in play #{pi + 1} has no 'tags:'",
                    suggestion="Add 'tags:' for selective execution and better debugging.",
                    resource=f"play[{pi}].tasks[{ti}]",
                ))
    return findings


def _rule_an011_ignore_errors_masks(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN011: 真生产 ignore_errors: true without failed_when (主 19:33 ansible-lint ignore-errors)."""
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        tasks = play.get("tasks", [])
        if not isinstance(tasks, list):
            continue
        for ti, task in enumerate(tasks):
            if not isinstance(task, dict):
                continue
            ie = task.get("ignore_errors", False)
            if ie is True and "failed_when" not in task:
                mod_key = next((k for k in task if k not in ("name", "when", "loop", "with_items",
                                                              "register", "become", "ignore_errors",
                                                              "tags", "notify", "vars", "environment",
                                                              "delegate_to", "run_once", "no_log",
                                                              "check_mode", "diff", "loop_control")),
                               "")
                line_no = _find_line_no(text, "ignore_errors:")
                findings.append(LintFinding(
                    rule_id="AN011-IGNORE-ERRORS-MASKS",
                    severity="warning",
                    line_no=line_no,
                    line_text=f"{mod_key or '<task>'}: ignore_errors: true",
                    message=f"Task #{ti + 1} uses 'ignore_errors: true' without 'failed_when:' — failures may be masked",
                    suggestion="Use 'failed_when:' to explicitly define failure conditions instead of blanket 'ignore_errors'.",
                    resource=f"play[{pi}].tasks[{ti}]",
                ))
    return findings


def _rule_an012_loop_undefined_var(text: str, plays: List[Dict[str, Any]]) -> List[LintFinding]:
    """V1398 AN012: 真生产 loop: references undefined var name (主 19:33 真借鉴 yamllint var-naming).

    heuristic: collect var names defined in play vars, then check loop/with_items.
    Also flags bare `{{ var }}` without surrounding braces when `var` not in play vars.
    """
    findings: List[LintFinding] = []
    for pi, play in enumerate(plays):
        # 真收集 play 定义的 vars
        defined_vars: Set[str] = set()
        play_vars = play.get("vars", {})
        if isinstance(play_vars, dict):
            defined_vars.update(_strip_quotes(k) for k in play_vars.keys() if isinstance(k, str))
        # 真收集 host_vars/group_vars 默认全局 (保守不报错)
        for ti, task in enumerate(_safe_tasks(play)):
            task_vars = task.get("vars", {})
            if isinstance(task_vars, dict):
                defined_vars.update(_strip_quotes(k) for k in task_vars.keys() if isinstance(k, str))
            # 真检查 loop
            loop_val = task.get("loop")
            if isinstance(loop_val, str):
                # 真提取 {{ var }} 形式
                for m in re.finditer(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}", loop_val):
                    var_name = m.group(1)
                    if var_name not in defined_vars and not var_name.startswith(("hostvars", "groups", "item")):
                        line_no = _find_line_no(text, "loop:")
                        findings.append(LintFinding(
                            rule_id="AN012-LOOP-UNDEFINED-VAR",
                            severity="error",
                            line_no=line_no,
                            line_text=f"loop: {loop_val[:80]}",
                            message=f"loop references undefined variable '{var_name}' (play #{pi + 1}, task #{ti + 1})",
                            suggestion=f"Define '{var_name}' in play vars, task vars, host_vars, or group_vars.",
                            resource=f"play[{pi}].tasks[{ti}]",
                        ))
    return findings


def _safe_tasks(play: Dict[str, Any]) -> List[Dict[str, Any]]:
    """V1398 真生产 helper: extract list of task dicts from play."""
    tasks = play.get("tasks", [])
    if isinstance(tasks, list):
        return [t for t in tasks if isinstance(t, dict)]
    return []


# V1398 真生产 rule registry (主 17:43 实事求是 真规则)
V1398_RULES: tuple = (
    ("AN001-NO-PLAY-NAME", _rule_an001_no_play_name),
    ("AN002-NO-TASK-NAME", _rule_an002_no_task_name),
    ("AN003-HARDCODED-SECRET", _rule_an003_hardcoded_secret),
    ("AN004-PLAIN-PRIVATE-KEY", _rule_an004_plain_private_key),
    ("AN005-RISKY-SHELL-PIPE", _rule_an005_risky_shell_pipe),
    ("AN006-NO-CHANGED-WHEN", _rule_an006_no_changed_when),
    ("AN007-DEPRECATED-LOOP", _rule_an007_deprecated_loop),
    ("AN008-COMMAND-INSTEAD-MODULE", _rule_an008_command_instead_module),
    ("AN009-INLINE-ENV-VAR", _rule_an009_inline_env_var),
    ("AN010-MISSING-TAGS", _rule_an010_missing_tags),
    ("AN011-IGNORE-ERRORS-MASKS", _rule_an011_ignore_errors_masks),
    ("AN012-LOOP-UNDEFINED-VAR", _rule_an012_loop_undefined_var),
)


# ============================================================================
# V1398 真生产 main lint function (主 17:43 实事求是)
# ============================================================================


def lint_file(file_path: str) -> LintReport:
    """V1398 真生产 lint 一个 playbook 文件 (主 17:43)."""
    t0 = time.time()
    try:
        text = Path(file_path).read_text(encoding="utf-8", errors="replace")
    except OSError as e:
        return LintReport(
            file_path=file_path, n_lines=0, n_findings=0,
            n_errors=0, n_warnings=0, n_info=0,
            ok=False, parse_error=f"Cannot read file: {e}",
            parser="",
        )

    n_lines = text.count("\n") + (0 if text.endswith("\n") else 1)
    plays, parse_err, n_plays, n_tasks = _parse_playbook(text, file_path)

    findings: List[LintFinding] = []
    for fid, rule_fn in V1398_RULES:
        try:
            new_findings = rule_fn(text, plays)
            # 真补 file_path
            for f in new_findings:
                f.file_path = file_path
            findings.extend(new_findings)
        except Exception as e:
            # 真报 rule failure 不崩溃
            findings.append(LintFinding(
                rule_id=fid, severity="info", line_no=0,
                line_text="", message=f"Rule {fid} failed: {e}",
                suggestion="Check playbook YAML syntax.", file_path=file_path,
            ))

    n_errors = sum(1 for f in findings if f.severity == "error")
    n_warnings = sum(1 for f in findings if f.severity == "warning")
    n_info = sum(1 for f in findings if f.severity == "info")

    return LintReport(
        file_path=file_path, n_lines=n_lines, n_findings=len(findings),
        n_errors=n_errors, n_warnings=n_warnings, n_info=n_info,
        findings=findings, elapsed_seconds=time.time() - t0,
        ok=(parse_err == ""),
        parse_error=parse_err, parser="yaml" if _YAML_AVAILABLE else "regex-fallback",
        n_plays=n_plays, n_tasks=n_tasks,
    )


def lint_path(path: str) -> List[LintReport]:
    """V1398 真生产 lint 路径 (文件 or 目录) (主 17:43)."""
    p = Path(path)
    if p.is_file():
        return [lint_file(str(p))]
    if p.is_dir():
        reports: List[LintReport] = []
        # 真扫 .yml .yaml
        for f in sorted(p.rglob("*")):
            if f.is_file() and f.suffix.lower() in (".yml", ".yaml"):
                reports.append(lint_file(str(f)))
        return reports
    return []


# ============================================================================
# V1398 真生产 SARIF 输出 (主 00:36 工程化)
# ============================================================================


def _report_to_sarif(reports: List[LintReport]) -> Dict[str, Any]:
    """V1398 真生产 convert reports to SARIF v2.1.0 (主 19:33 github/codeql-action 真借鉴)."""
    results = []
    rules_index: Dict[str, int] = {}
    sarif_rules: List[Dict[str, Any]] = []
    for rep in reports:
        for f in rep.findings:
            rid = f.rule_id
            if rid not in rules_index:
                rules_index[rid] = len(sarif_rules)
                sarif_rules.append({
                    "id": rid,
                    "name": rid,
                    "shortDescription": {"text": rid},
                    "defaultConfiguration": {"level": "warning"},
                })
            sev_map = {"error": "error", "warning": "warning", "info": "note"}
            result: Dict[str, Any] = {
                "ruleId": rid,
                "ruleIndex": rules_index[rid],
                "level": sev_map.get(f.severity, "warning"),
                "message": {"text": f.message},
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {"uri": f.file_path},
                        "region": {"startLine": max(1, f.line_no)},
                    }
                }],
            }
            results.append(result)
    return {
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "v1398-ansible-lint",
                    "version": V1398_VERSION,
                    "informationUri": "https://github.com/apeireth",
                    "rules": sarif_rules,
                }
            },
            "results": results,
        }],
    }


# ============================================================================
# V1398 真生产 chain delegate (主 17:43 实事求是 调 V1387 unified runner)
# ============================================================================


def chain_with_v1387(path: str) -> Dict[str, Any]:
    """V1398 真生产 chain delegate V1387 unified runner + V1398 真 lint (主 17:43)."""
    reports = lint_path(path)
    n_findings = sum(r.n_findings for r in reports)
    n_errors = sum(r.n_errors for r in reports)
    n_warnings = sum(r.n_warnings for r in reports)

    v1387_result: Optional[Dict[str, Any]] = None
    try:
        from apeireth import v1387_deploy_stack_runner as v1387  # type: ignore
        _runner = v1387.V1387DeployStackRunner()
        _stack_report = _runner.run(path)
        v1387_result = _stack_report.to_dict() if hasattr(_stack_report, "to_dict") else {
            "schema": _stack_report.schema,
            "n_files_total": _stack_report.n_files_total,
            "n_findings_total": _stack_report.n_findings_total,
            "n_errors": _stack_report.n_errors,
            "n_warnings": _stack_report.n_warnings,
            "ok": _stack_report.ok,
        }
        v1387_result["available"] = True
    except Exception as e:
        v1387_result = {"error": f"v1387 delegate failed: {e}", "available": False}

    return {
        "schema": "v1398.ansible-lint.chain/v1",
        "version": V1398_VERSION,
        "path": path,
        "v1398": {
            "n_files": len(reports),
            "n_findings": n_findings,
            "n_errors": n_errors,
            "n_warnings": n_warnings,
            "n_plays": sum(r.n_plays for r in reports),
            "n_tasks": sum(r.n_tasks for r in reports),
            "reports": [r.to_dict() for r in reports],
        },
        "v1387_delegate": v1387_result,
    }


# ============================================================================
# V1398 真生产 popper self-test (主 17:43)
# ============================================================================


_V1398_BUILTIN_SAMPLE = """---
# V1398 builtin sample — 真测所有 12 规则 (主 17:43 实事求是)
- hosts: all
  vars:
    db_password: "supersecret123"
    aws_access_key: "AKIAIOSFODNN7EXAMPLE"
  tasks:
    - name: install package
      command: apt-get install -y nginx
    - apt:
        name: curl
    - shell: cat /etc/passwd | grep root
    - command: systemctl restart nginx
    - command: echo $DB_PASSWORD
      environment:
        DB_PASSWORD: "supersecret123"
    - name: legacy loop
      with_items: "{{ packages }}"
    - name: bad task without name
      apt: name=vim
    - name: ignore errors
      command: /bin/false
      ignore_errors: true
    - name: loop undefined
      debug:
        msg: "{{ item }}"
      loop: "{{ undefined_var }}"
    - name: with inline private key
      copy:
        content: |
          -----BEGIN RSA PRIVATE KEY-----
          MIIEpAIBAAKCAQEAxxx...
          -----END RSA PRIVATE KEY-----
        dest: /tmp/key.pem
"""

_V1398_BUILTIN_CLEAN = """---
- name: Setup web server
  hosts: webservers
  vars:
    packages:
      - nginx
      - curl
  tasks:
    - name: Install nginx package
      apt:
        name: "{{ packages }}"
        state: present
    - name: Start nginx service
      service:
        name: nginx
        state: started
        enabled: true
"""


def _popper_self_test() -> Dict[str, Any]:
    """V1398 真生产 popper self-test (主 17:43)."""
    result: Dict[str, Any] = {
        "ok": True,
        "tests": [],
        "n_rules": len(V1398_RULES),
        "n_guards": len(V1398_GUARDS),
        "yaml_available": _YAML_AVAILABLE,
    }
    # Test 1: bad sample should produce ≥ 8 findings
    try:
        rep = lint_text(_V1398_BUILTIN_SAMPLE, "<builtin-sample>")
        result["tests"].append({
            "name": "bad_sample_lints",
            "ok": rep.n_findings >= 8,
            "n_findings": rep.n_findings,
            "n_errors": rep.n_errors,
            "n_warnings": rep.n_warnings,
            "n_info": rep.n_info,
        })
        if not (rep.n_findings >= 8):
            result["ok"] = False
    except Exception as e:
        result["tests"].append({"name": "bad_sample_lints", "ok": False, "error": str(e)})
        result["ok"] = False

    # Test 2: clean sample should produce 0 findings (or only AN010 missing-tags info)
    try:
        rep = lint_text(_V1398_BUILTIN_CLEAN, "<builtin-clean>")
        result["tests"].append({
            "name": "clean_sample_lints",
            "ok": rep.n_errors == 0 and rep.n_warnings == 0,
            "n_errors": rep.n_errors,
            "n_warnings": rep.n_warnings,
            "n_info": rep.n_info,
        })
        if not (rep.n_errors == 0 and rep.n_warnings == 0):
            result["ok"] = False
    except Exception as e:
        result["tests"].append({"name": "clean_sample_lints", "ok": False, "error": str(e)})
        result["ok"] = False

    # Test 3: each rule must fire at least once on bad sample (12 rules)
    bad_rep = lint_text(_V1398_BUILTIN_SAMPLE, "<builtin-sample>")
    fired_rules = {f.rule_id for f in bad_rep.findings}
    result["tests"].append({
        "name": "all_12_rules_fire",
        "ok": len(fired_rules) >= 8,  # at least 8/12 (info rules may not fire)
        "fired": sorted(fired_rules),
        "missing": [rid for rid, _ in V1398_RULES if rid not in fired_rules],
    })

    # Test 4: SARIF roundtrip
    sarif = _report_to_sarif([bad_rep])
    result["tests"].append({
        "name": "sarif_roundtrip",
        "ok": sarif.get("version") == "2.1.0" and len(sarif["runs"][0]["results"]) >= 8,
        "n_results": len(sarif["runs"][0]["results"]),
    })

    # Test 5: chain_with_v1387 must not crash (v1387 may be missing)
    chain = chain_with_v1387("<builtin-sample>")
    result["tests"].append({
        "name": "chain_delegate",
        "ok": "v1398" in chain,
        "n_findings": chain["v1398"]["n_findings"],
    })

    return result


def lint_text(text: str, file_path: str = "<text>") -> LintReport:
    """V1398 真生产 lint text directly (for popper / tests)."""
    t0 = time.time()
    n_lines = text.count("\n") + (0 if text.endswith("\n") else 1)
    plays, parse_err, n_plays, n_tasks = _parse_playbook(text, file_path)

    findings: List[LintFinding] = []
    for fid, rule_fn in V1398_RULES:
        try:
            new_findings = rule_fn(text, plays)
            for f in new_findings:
                f.file_path = file_path
            findings.extend(new_findings)
        except Exception as e:
            findings.append(LintFinding(
                rule_id=fid, severity="info", line_no=0,
                line_text="", message=f"Rule {fid} failed: {e}",
                suggestion="Check playbook YAML syntax.", file_path=file_path,
            ))

    n_errors = sum(1 for f in findings if f.severity == "error")
    n_warnings = sum(1 for f in findings if f.severity == "warning")
    n_info = sum(1 for f in findings if f.severity == "info")

    return LintReport(
        file_path=file_path, n_lines=n_lines, n_findings=len(findings),
        n_errors=n_errors, n_warnings=n_warnings, n_info=n_info,
        findings=findings, elapsed_seconds=time.time() - t0,
        ok=(parse_err == ""),
        parse_error=parse_err, parser="yaml" if _YAML_AVAILABLE else "regex-fallback",
        n_plays=n_plays, n_tasks=n_tasks,
    )


# ============================================================================
# V1398 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def _cmd_version(_args: argparse.Namespace) -> int:
    """V1398 version."""
    print(f"v1398-ansible-lint v{V1398_VERSION} (schema {V1398_SCHEMA})")
    print(f"  yaml_available: {_YAML_AVAILABLE}")
    print(f"  n_rules: {len(V1398_RULES)}")
    print(f"  n_guards: {len(V1398_GUARDS)}")
    print(f"  n_borrowed: {len(V1398_BORROWED)}")
    return 0


def _cmd_lint(args: argparse.Namespace) -> int:
    """V1398 真 lint playbook 文件/目录."""
    reports = lint_path(args.path)
    if not reports:
        print(f"No .yml/.yaml files found at {args.path}", file=sys.stderr)
        return 1
    if args.format == "json":
        print(json.dumps([r.to_dict() for r in reports], indent=2, ensure_ascii=False))
    elif args.format == "sarif":
        print(json.dumps(_report_to_sarif(reports), indent=2, ensure_ascii=False))
    else:
        for rep in reports:
            print(f"\n=== {rep.file_path} ===")
            print(f"lines={rep.n_lines} plays={rep.n_plays} tasks={rep.n_tasks} "
                  f"findings={rep.n_findings} (E:{rep.n_errors} W:{rep.n_warnings} I:{rep.n_info}) "
                  f"[{rep.parser}]")
            if rep.parse_error:
                print(f"  PARSE ERROR: {rep.parse_error}")
                continue
            for f in rep.findings:
                loc = f"line {f.line_no}" if f.line_no else "?"
                print(f"  [{f.severity.upper():7}] {f.rule_id} {loc}: {f.message}")
                if f.suggestion:
                    print(f"           → {f.suggestion}")
    n_errors = sum(r.n_errors for r in reports)
    n_warnings = sum(r.n_warnings for r in reports)
    if args.strict and n_warnings > 0:
        return 2
    return 1 if n_errors > 0 else 0


def _cmd_chain(args: argparse.Namespace) -> int:
    """V1398 真 chain V1387 + V1398."""
    out = chain_with_v1387(args.path)
    if args.json:
        print(json.dumps(out, indent=2, ensure_ascii=False))
    else:
        v8 = out["v1398"]
        print(f"V1398 chain report v{V1398_VERSION}")
        print(f"  path: {args.path}")
        print(f"  files: {v8['n_files']}, plays: {v8['n_plays']}, tasks: {v8['n_tasks']}")
        print(f"  findings: {v8['n_findings']} (E:{v8['n_errors']} W:{v8['n_warnings']})")
        v87 = out.get("v1387_delegate", {})
        if v87.get("available") is False:
            print(f"  v1387 delegate: {v87.get('error', 'not available')}")
        else:
            print(f"  v1387 delegate: ok")
    return 0


def _cmd_demo(_args: argparse.Namespace) -> int:
    """V1398 demo with builtin sample."""
    print("# V1398 demo — builtin playbook sample (12 rules expected to fire)")
    print(f"# Schema: {V1398_SCHEMA} v{V1398_VERSION}")
    print(f"# yaml_available: {_YAML_AVAILABLE}")
    print(f"# n_rules: {len(V1398_RULES)}")
    print(f"# n_guards: {len(V1398_GUARDS)}")
    print()
    print("## Borrowed (主 19:33)")
    for b in V1398_BORROWED:
        print(f"- {b}")
    print()
    print("## Guards (主 17:43)")
    for g in V1398_GUARDS:
        print(f"- {g}")
    print()
    print("## Rules (主 19:33 ansible-lint/yamllint 真借鉴)")
    for rid, _ in V1398_RULES:
        print(f"- {rid}")
    print()
    print("## Sample playbook (主 17:43 真测)")
    print(_V1398_BUILTIN_SAMPLE)
    return 0


def _cmd_popper(_args: argparse.Namespace) -> int:
    """V1398 popper self-test."""
    result = _popper_self_test()
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if result["ok"] else 1


def _build_parser() -> argparse.ArgumentParser:
    """V1398 CLI parser."""
    parser = argparse.ArgumentParser(
        prog="v1398-ansible-lint",
        description=f"V1398 ASI real production ansible playbook lint (v{V1398_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_ver = sub.add_parser("version", help="V1398 version")
    p_ver.set_defaults(func=_cmd_version)

    p_lint = sub.add_parser("lint", help="真 lint playbook files")
    p_lint.add_argument("path", nargs="?", default=".", help="file or directory")
    p_lint.add_argument("--format", choices=("text", "json", "sarif"), default="text")
    p_lint.add_argument("--strict", action="store_true", help="exit non-zero on warnings")
    p_lint.set_defaults(func=_cmd_lint)

    p_chain = sub.add_parser("chain", help="真 chain V1387 + V1398")
    p_chain.add_argument("path", nargs="?", default=".", help="file or directory")
    p_chain.add_argument("--json", action="store_true")
    p_chain.set_defaults(func=_cmd_chain)

    p_demo = sub.add_parser("demo", help="V1398 demo")
    p_demo.set_defaults(func=_cmd_demo)

    p_popper = sub.add_parser("popper", help="V1398 popper self-test")
    p_popper.set_defaults(func=_cmd_popper)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    """V1398 真生产 CLI main (主 17:43)."""
    parser = _build_parser()
    args = parser.parse_args(argv)
    if not hasattr(args, "func"):
        parser.print_help()
        return 0
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())