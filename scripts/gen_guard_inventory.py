#!/usr/bin/env python3
"""生成**护栏清单**（`scripts/guard-inventory.json`）—— 仓里每一个守卫 / 棘轮 / 检查的机器可读台账。

# 为什么要有这个脚本

仓里的守卫脚本是**散落的**：有的在 `scripts/` 根下，有的在 `scripts/audit/` 下，
有的只被 CI 调、有的压根没人调。
⇒ 「这仓到底有哪些门禁、哪些会挡住人、哪些只出一份报告」**没人能一次答全**。

⇒ 本脚本把这件事变成**一份 JSON**（`scripts/guard-inventory.json`）：
每个守卫一行，含 **id / path / kind / blocking / invocation / ci_workflows /
has_baseline / notes / flags**。

⇒ 这份 JSON 是后续 `scripts/harness.py` 统一入口与「单一真源契约」的底座
⇒ 所以它必须**可重算**（本脚本）与**可校验**（`scripts/check_guard_inventory.py`）。

# 发现规则（Discovery）

一个脚本被收进清单，当且仅当满足下面**任一**条：

1. **名字像守卫**：位于 `scripts/*.py`、`scripts/*.sh`、`scripts/audit/*.sh`，
   且名字（相对 `scripts/`、去掉扩展名）匹配下面**任一**模式：
   `check_*`、`audit_*`、`*_ratchet*`、`verify_*`、`arch_guard`、`docs_truth`、
   `docs_index`、`docs_links`、`wiring_report`、`d4_ledger`、`internal_golden`、
   `group_unwired_baseline`；
2. **CI 调过它**：`.github/workflows/*.yml` 的 `run:` 块里出现过该脚本
   （`.py` / `.sh`，任意深度）。

# 判定口径（诚实优先 —— 读不出来就说读不出来）

* `kind`
  * `ratchet` —— 有 `scripts/` 下的基线文件，且能重生成/更新基线；
  * `guard` —— 违规时**会非零退出**（能挡人）；
  * `generator` —— 写/下载/改动文件（产物，不是门禁）；
  * `report` —— 只打印报告，退出 0；
  * `unknown` —— 从源码**读不出来**（不猜）。
* `blocking` —— 静态读源码（Python 走 AST，shell 去注释），判断它**能不能**
  非零退出；**读不出来就是 `"unknown"`**（不假装知道）。
* `flags` —— 真的跑一遍 `--help` 抓出来的；
  没有 `--help`、且会真干活的脚本 ⇒ `flags: []` 且**不跑**（不冒险）。
* `notes` —— 只在**脚本自己的 docstring / 注释头**能确认时才写；
  否则空串（**不替人编**）。

# 用法

```sh
python3 scripts/gen_guard_inventory.py            # 写 scripts/guard-inventory.json
python3 scripts/gen_guard_inventory.py --check    # 只算不写；与现文件有差异则退出 1
```

# 幂等

输出**只依赖仓库现状**（不含时间戳、不含随机序）⇒ 连跑两次字节一致。
"""

from __future__ import annotations

import argparse
import ast
import fnmatch
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = ROOT / "scripts"
WORKFLOWS_DIR = ROOT / ".github" / "workflows"
OUT_PATH = SCRIPTS_DIR / "guard-inventory.json"

SCHEMA = "apeireth.guard-inventory.v1"
GENERATOR = "scripts/gen_guard_inventory.py"

# ── 发现：名字像守卫的模式（对「相对 scripts/、去扩展名」的名字做 fnmatch）──
NAME_PATTERNS = (
    "check_*",
    "audit_*",
    "*_ratchet*",
    "verify_*",
    "arch_guard",
    "docs_truth",
    "docs_index",
    "docs_links",
    "wiring_report",
    "d4_ledger",
    "internal_golden",
    "group_unwired_baseline",
)

# ── 分类：判定「这是个检查器」的额外词表（名字口径，不影响发现范围）──
CHECKER_NAMES = (
    "check_*",
    "audit_*",
    "verify_*",
    "*checklist*",
    "vet",
    "arch_guard",
    "arch_doc",
    "docs_truth",
    "docs_index",
    "docs_links",
    "wiring_report",
    "d4_ledger",
    "internal_golden",
    "group_unwired_baseline",
)

# 「检查模式」标志（出现即认为这个脚本是在做检查）
CHECK_FLAGS = (
    "--check",
    "--verify",
    "--strict",
    "--audit",
    "--lint",
    "--crosscheck",
    "--block-stale",
    "--wired-gate",
    "--check-env-registry",
)

# 「能重生成基线」标志
REGEN_FLAGS = (
    "--update",
    "--update-baseline",
    "--update-baseline-file",
    "--update-ledger",
    "--gen",
    "--generate",
    "--write",
    "--regen",
    "--refresh",
    "--sync",
    "--export",
)

SCAN_GLOBS = ("*.py", "*.sh")
SCAN_SUBDIRS = ("audit",)
CI_EXTENSIONS = (".py", ".sh")
HELP_TIMEOUT = 45

KEBAB_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

# 命令行「头词」白名单：只有这些头词开头的行才算「真的调用了这个脚本」
CMD_HEAD_RE = re.compile(
    r"^\s*(?:[A-Za-z_][A-Za-z0-9_]*=\S*\s+)*"
    r"(?:python3?|bash|sh|zsh|ksh|node|npx|deno|bun|cargo|pwsh|powershell|make|\./)"
    r"\b"
)
# `python -m py_compile a.py b.py` 是**编译检查**，不是调用那几个脚本
PY_MODULE_RE = re.compile(r"^\s*python3?\s+-m\s+(py_compile|compileall|pydoc)\b")


# --------------------------------------------------------------------------- #
# 发现
# --------------------------------------------------------------------------- #
def rel_stem(path: Path) -> str:
    """相对 scripts/ 的路径，去掉扩展名。"""
    rel = path.relative_to(SCRIPTS_DIR).as_posix()
    return rel[: -len(path.suffix)] if path.suffix else rel


def looks_like_guard(stem: str) -> bool:
    return any(fnmatch.fnmatchcase(stem, pat) for pat in NAME_PATTERNS)


def looks_like_checker(stem: str) -> bool:
    base = os.path.basename(stem)
    return any(fnmatch.fnmatchcase(base, pat) for pat in CHECKER_NAMES)


def iter_script_files() -> list[Path]:
    """scripts/ 下顶层 + 已知子目录的候选脚本。"""
    out: list[Path] = []
    for pattern in SCAN_GLOBS:
        for p in sorted(SCRIPTS_DIR.glob(pattern)):
            if p.is_file():
                out.append(p)
    for sub in SCAN_SUBDIRS:
        d = SCRIPTS_DIR / sub
        if not d.is_dir():
            continue
        for p in sorted(d.glob("*.sh")):
            if p.is_file():
                out.append(p)
    return out


def all_scripts_under_scripts() -> list[Path]:
    """scripts/ 下所有 .py / .sh（任意深度），供 CI 引用判定用。"""
    out: list[Path] = []
    for p in sorted(SCRIPTS_DIR.rglob("*")):
        if not p.is_file():
            continue
        if any(part in {"__pycache__", "node_modules"} for part in p.parts):
            continue
        if p.suffix in CI_EXTENSIONS:
            out.append(p)
    return out


# --------------------------------------------------------------------------- #
# CI 引用
# --------------------------------------------------------------------------- #
def _yaml_run_blocks(text: str) -> list[list[str]]:
    """抽出 workflow 里所有 `run:` 命令块（每个块是一组命令行）。"""
    lines = text.splitlines()
    blocks: list[list[str]] = []
    i = 0
    while i < len(lines):
        m = re.match(r"^(\s*)run:\s*(.*)$", lines[i])
        if not m:
            i += 1
            continue
        indent = len(m.group(1))
        inline = m.group(2).strip()
        if inline and inline not in {"|", ">", "|-", ">-", "|+", ">+"}:
            blocks.append([inline])
            i += 1
            continue
        body: list[str] = []
        j = i + 1
        while j < len(lines):
            nxt = lines[j]
            if not nxt.strip():
                body.append("")
                j += 1
                continue
            cur = len(nxt) - len(nxt.lstrip())
            if cur <= indent:
                break
            body.append(nxt.strip())
            j += 1
        blocks.append(body)
        i = j
    return blocks


def _join_continuations(lines: list[str]) -> list[str]:
    """把 `\\` 续行拼成一条命令行。"""
    out: list[str] = []
    buf = ""
    for raw in lines:
        s = raw.strip()
        if not s or s.startswith("#"):
            continue
        if s.endswith("\\"):
            buf += s[:-1].strip() + " "
            continue
        buf += s
        out.append(buf.strip())
        buf = ""
    if buf.strip():
        out.append(buf.strip())
    return out


def _segments(cmd: str) -> list[str]:
    """把一条命令行按管道 / && / || / ; 拆成候选命令段（剥掉尾部重定向）。"""
    parts = re.split(r"\|\||&&|\||;", cmd)
    out: list[str] = []
    for part in parts:
        seg = part.strip()
        seg = re.sub(r"\s*\d?>&\d+\s*$", "", seg)
        seg = re.sub(r"\s*\d?>>?\s*[^\s&]+\s*$", "", seg)
        seg = seg.strip()
        if seg:
            out.append(seg)
    return out


def _is_invocation_segment(seg: str, script_path: str) -> bool:
    """这一段是不是「真的调用了这个脚本」。"""
    if not seg or seg.startswith("#"):
        return False
    if PY_MODULE_RE.match(seg):
        return False
    if not CMD_HEAD_RE.match(seg):
        return False
    # 脚本路径必须是一个裸 token（不能是 `':(exclude)scripts/x.sh'` 这种 git pathspec）
    for m in re.finditer(r"[A-Za-z0-9_./\-]+\.(?:py|sh)\b", seg):
        tok = m.group(0)
        if tok != script_path and os.path.basename(tok) != os.path.basename(script_path):
            continue
        before = seg[: m.start()].rstrip()
        after = seg[m.end():].lstrip()
        if before.endswith(("'", '"')) or after.startswith(("'", '"')):
            continue  # 引号包起来的路径（多半是 pathspec / 说明文字）
        return True
    return False


def ci_command_index() -> dict[str, dict[str, list[str]]]:
    """{脚本相对路径: {workflow 相对路径: [命令, ...]}}"""
    index: dict[str, dict[str, list[str]]] = {}
    if not WORKFLOWS_DIR.is_dir():
        return index
    for wf in sorted(WORKFLOWS_DIR.glob("*.yml")):
        try:
            text = wf.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        wf_name = wf.relative_to(ROOT).as_posix()
        for block in _yaml_run_blocks(text):
            for cmd in _join_continuations(block):
                for seg in _segments(cmd):
                    for m in re.finditer(r"scripts/[A-Za-z0-9_./\-]+\.(?:py|sh)\b", seg):
                        rel = m.group(0)
                        if not _is_invocation_segment(seg, rel):
                            continue
                        per = index.setdefault(rel, {})
                        cmds = per.setdefault(wf_name, [])
                        if seg not in cmds:
                            cmds.append(seg)
    return index


def ci_workflow_files_for(rel_path: str, ci_index: dict[str, dict[str, list[str]]]) -> list[str]:
    """哪些 workflow 真的调用了它。"""
    per = ci_index.get(rel_path)
    if per:
        return sorted(per)
    # 兜底：workflow 里用裸 basename 调（路径写法不同）
    base = os.path.basename(rel_path)
    hits: list[str] = []
    for wf in sorted(WORKFLOWS_DIR.glob("*.yml")):
        try:
            text = wf.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        found = False
        for block in _yaml_run_blocks(text):
            for cmd in _join_continuations(block):
                for seg in _segments(cmd):
                    if base in seg and _is_invocation_segment(seg, base):
                        found = True
                        break
                if found:
                    break
            if found:
                break
        if found:
            hits.append(wf.relative_to(ROOT).as_posix())
    return sorted(set(hits))


def ci_commands_for(rel_path: str, ci_index: dict[str, dict[str, list[str]]]) -> list[str]:
    """该脚本在 CI 里的确切命令行（去重、保序）。"""
    out: list[str] = []
    for wf_name in sorted(ci_index.get(rel_path, {})):
        for cmd in ci_index[rel_path][wf_name]:
            if cmd not in out:
                out.append(cmd)
    return out


# --------------------------------------------------------------------------- #
# id
# --------------------------------------------------------------------------- #
def make_id(stem: str) -> str:
    base = os.path.basename(stem)
    base = re.sub(r"^\d+[-_]", "", base)  # `8-promise-audit` -> `promise-audit`
    base = base.lower()
    base = re.sub(r"[^a-z0-9]+", "-", base)
    base = re.sub(r"-{2,}", "-", base).strip("-")
    return base


# --------------------------------------------------------------------------- #
# 源码静态分析
# --------------------------------------------------------------------------- #
def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None


def module_docstring(src: str) -> str:
    """取模块 docstring（拿不到就退回首个三引号块）。"""
    try:
        tree = ast.parse(src)
        doc = ast.get_docstring(tree)
        if doc:
            return doc
    except (SyntaxError, ValueError):
        pass
    m = re.search(r'^\s*(?:#![^\n]*\n)?\s*"""(.*?)"""', src, re.S)
    if m:
        return m.group(1)
    m = re.search(r"^\s*(?:#![^\n]*\n)?\s*'''(.*?)'''", src, re.S)
    if m:
        return m.group(1)
    return ""


def strip_shell_comments(src: str) -> str:
    """去掉 shell 注释（整行注释 + 行尾注释），避免把说明文字当代码。"""
    out: list[str] = []
    for line in src.splitlines():
        s = line.strip()
        if s.startswith("#"):
            continue
        in_s = in_d = False
        cut = len(line)
        for idx, ch in enumerate(line):
            if ch == "'" and not in_d:
                in_s = not in_s
            elif ch == '"' and not in_s:
                in_d = not in_d
            elif ch == "#" and not in_s and not in_d:
                cut = idx
                break
        out.append(line[:cut])
    return "\n".join(out)


def py_code_strings(src: str) -> list[str]:
    """Python 源码里的**字符串常量**（排除 docstring）—— 只认代码里的路径字面量。"""
    try:
        tree = ast.parse(src)
    except (SyntaxError, ValueError):
        return []
    doc_nodes: set[int] = set()
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            body = getattr(node, "body", [])
            if (
                body
                and isinstance(body[0], ast.Expr)
                and isinstance(body[0].value, ast.Constant)
                and isinstance(body[0].value.value, str)
            ):
                doc_nodes.add(id(body[0].value))
    out: list[str] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            if id(node) not in doc_nodes:
                out.append(node.value)
    return out


def shell_header_comment(src: str, script_name: str) -> str:
    """shell 脚本首个有意义的注释行（跳过 shebang / 分隔线 / 自述行）。"""
    for i, line in enumerate(src.splitlines()):
        s = line.strip()
        if i == 0 and s.startswith("#!"):
            continue
        if not s.startswith("#"):
            continue
        body = s.lstrip("#").strip()
        if not body:
            continue
        if set(body) <= set("=-_*#"):
            continue
        if body.startswith("scripts/") or body.startswith(script_name):
            continue
        return body
    return ""


def summarize(text: str, limit: int = 80) -> str:
    """把一段说明压成 ≤limit 字的短句；压不动就返回空串。"""
    if not text:
        return ""
    para = ""
    for chunk in re.split(r"\n\s*\n", text):
        chunk = chunk.strip()
        if chunk:
            para = chunk
            break
    if not para:
        return ""
    para = para.replace("\n", " ")
    para = para.replace("**", "").replace("`", "")
    para = re.sub(r"\([^()]*scripts/[^()]*\)", "", para)
    para = re.sub(r"（[^（）]*scripts/[^（）]*）", "", para)
    para = re.sub(r"\s+", " ", para).strip(" -*·")
    if not para:
        return ""
    cut = -1
    for m in re.finditer(r"[。．！？!?]|(?<![A-Za-z0-9_-])\.(?=\s|$)", para):
        cut = m.start()
        break
    if cut >= 0 and cut >= 8:
        para = para[:cut]
    para = para.strip(" -*·,，、")
    if not para:
        return ""
    if len(para) > limit:
        para = para[: limit - 1].rstrip() + "…"
    return para


def detect_blocking_py(src: str) -> bool | str:
    """AST 判定：能不能非零退出。解析不了 => "unknown"。"""
    try:
        tree = ast.parse(src)
    except (SyntaxError, ValueError):
        return "unknown"
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            func = node.func
            name = None
            if isinstance(func, ast.Attribute):
                base = func.value
                if isinstance(base, ast.Name):
                    name = f"{base.id}.{func.attr}"
            elif isinstance(func, ast.Name):
                name = func.id
            if name in {"sys.exit", "os._exit", "SystemExit"}:
                if not node.args:
                    continue
                arg = node.args[0]
                if isinstance(arg, ast.Constant) and arg.value == 0:
                    continue
                return True
            if name in {
                "subprocess.run",
                "subprocess.call",
                "subprocess.check_call",
                "subprocess.check_output",
                "Popen",
            }:
                for kw in node.keywords:
                    if kw.arg == "check" and not (
                        isinstance(kw.value, ast.Constant) and kw.value.value is False
                    ):
                        return True
        if isinstance(node, ast.Raise) and node.exc is not None:
            exc = node.exc
            is_sys_exit = (
                isinstance(exc, ast.Call)
                and isinstance(exc.func, ast.Name)
                and exc.func.id == "SystemExit"
            )
            if not is_sys_exit:
                # 未捕获的普通异常 ⇒ Python 退出码 1
                return True
    return False


def detect_blocking_shell(src: str) -> bool | str:
    """静态判定 shell 脚本能不能非零退出（去注释后看 `exit`）。"""
    code = strip_shell_comments(src)
    for m in re.finditer(r"\bexit\s+([^;&|\n]*)", code):
        arg = m.group(1).strip()
        if not arg:
            continue
        if re.fullmatch(r"0", arg):
            continue
        return True
    return False


def detect_blocking(src: str, suffix: str) -> bool | str:
    if src is None:
        return "unknown"
    if suffix == ".py":
        return detect_blocking_py(src)
    return detect_blocking_shell(src)


BASELINE_PATTERNS = (
    re.compile(r"[A-Za-z0-9_.\-]*-baseline\.[A-Za-z0-9]+"),
    re.compile(r"[A-Za-z0-9_.\-]*-ledger\.json"),
    re.compile(r"[A-Za-z0-9_.\-]*\.toml"),
)


def sibling_module_paths(path: Path, src: str) -> list[Path]:
    """脚本 import 的兄弟模块（用于解析写在别处的基线路径常量）。"""
    out: list[Path] = []
    try:
        tree = ast.parse(src)
    except (SyntaxError, ValueError):
        return out
    mods: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module:
            mods.add(node.module.split(".")[0])
        elif isinstance(node, ast.Import):
            for alias in node.names:
                mods.add(alias.name.split(".")[0])
    for mod in sorted(mods):
        for cand in (path.parent / f"{mod}.py", SCRIPTS_DIR / f"{mod}.py"):
            if cand.is_file() and cand != path and cand not in out:
                out.append(cand)
                break
    return out


def detect_baseline(src: str, suffix: str, path: Path | None = None) -> bool:
    """是否读/写 `scripts/` 下的基线文件（*-baseline.* / *-ledger.json / *.toml）。"""
    if not src:
        return False
    if suffix == ".py":
        chunks = ["\n".join(py_code_strings(src))]
        if path is not None:
            for sib in sibling_module_paths(path, src):
                sib_src = read_text(sib)
                if sib_src:
                    chunks.append("\n".join(py_code_strings(sib_src)))
        haystack = "\n".join(chunks)
    else:
        haystack = strip_shell_comments(src)
    names: set[str] = set()
    for pat in BASELINE_PATTERNS:
        names |= set(pat.findall(haystack))
    for name in names:
        name = name.strip('"\'')
        if not name:
            continue
        if (SCRIPTS_DIR / name).is_file():
            return True
        if f"scripts/{name}" in haystack:
            return True
    return False


def detect_regen(src: str, flags: list[str]) -> bool:
    for f in flags:
        if f in REGEN_FLAGS:
            return True
    for f in REGEN_FLAGS:
        if re.search(re.escape(f) + r"\b", src):
            return True
    return False


def detect_check_mode(src: str, flags: list[str]) -> bool:
    for f in flags:
        if f in CHECK_FLAGS:
            return True
    for f in CHECK_FLAGS:
        if re.search(re.escape(f) + r"\b", src):
            return True
    return False


WRITE_PATTERNS = (
    r"\.write_text\(",
    r"\.write_bytes\(",
    r"open\([^)]*[\"']w[\"']",
    r"json\.dump\(",
    r"shutil\.copy",
    r"\.write\(",
    r"shutil\.rmtree",
    r"os\.remove",
    r"\.unlink\(",
    r"urlopen\(",
    r"requests\.(get|post|put)\(",
    r"\bcurl\s",
    r"\bwget\s",
)
SHELL_WRITE_PATTERNS = (
    r"\btee\s",
    r"\bcp\s",
    r"\binstall\s",
    r"\bmkdir\s",
    r"\btar\s",
    r"\bzip\s",
    r"\brm\s+-[rf]",
    r">\s*[\w./\-]+\.(?:json|txt|md|sig|spdx|cdx)",
)


def detect_manipulates(src: str, suffix: str) -> bool:
    """是否产出/改动文件（生成物，不是门禁）。"""
    if not src:
        return False
    pats = WRITE_PATTERNS if suffix == ".py" else (WRITE_PATTERNS + SHELL_WRITE_PATTERNS)
    return any(re.search(p, src) for p in pats)


def detect_report_only(src: str, flags: list[str]) -> bool:
    if "--report" in flags:
        return True
    return bool(re.search(r"report[-_ ]only|只出报告|只打印报告|不阻塞", src, re.I))


# --------------------------------------------------------------------------- #
# --help 探测（安全优先）
# --------------------------------------------------------------------------- #
def supports_help(is_py: bool, src: str) -> bool:
    """静态判断 `--help` 是否安全：脚本自己认这个参数（argparse / 显式分支）。"""
    if is_py:
        if re.search(r"^\s*import\s+argparse\b", src, re.M) or re.search(
            r"^\s*from\s+argparse\b", src, re.M
        ):
            return True
        return bool(re.search(r"[\"']--help[\"']", src))
    return bool(re.search(r"--help", src))


def run_help(path: Path, suffix: str) -> tuple[list[str], str]:
    """跑 `--help` 抓 flags。返回 (flags, probe_status)。"""
    if suffix == ".py":
        cmd = [sys.executable, str(path), "--help"]
    else:
        cmd = ["bash", str(path), "--help"]
    env = dict(os.environ)
    env["LC_ALL"] = "C.UTF-8"
    env["PYTHONIOENCODING"] = "utf-8"
    try:
        proc = subprocess.run(
            cmd, cwd=str(ROOT), capture_output=True, text=True, timeout=HELP_TIMEOUT, env=env
        )
    except subprocess.TimeoutExpired:
        return [], "help-timeout"
    except OSError:
        return [], "help-error"
    out = (proc.stdout or "") + "\n" + (proc.stderr or "")
    if proc.returncode != 0:
        return [], f"help-nonzero(exit={proc.returncode})"
    return parse_flags(out, suffix), "help"


OPTION_LINE_RE = re.compile(
    r"^\s{1,8}(-{1,2}[A-Za-z0-9][\w\-]*(?:,\s*-{1,2}[A-Za-z0-9][\w\-]*)*)(?=\s|$)"
)


def parse_flags(help_text: str, suffix: str) -> list[str]:
    """从 --help 输出里抓 flag 列表。"""
    lines = help_text.splitlines()
    if suffix == ".py":
        start = None
        for i, line in enumerate(lines):
            if line.strip().lower() in ("options:", "optional arguments:", "options"):
                start = i + 1
                break
        scope = lines[start:] if start is not None else lines
        found: list[str] = []
        for line in scope:
            m = OPTION_LINE_RE.match(line)
            if not m:
                continue
            for tok in re.findall(r"-{1,2}[A-Za-z0-9][\w\-]*", m.group(1)):
                if tok not in found:
                    found.append(tok)
        if found:
            return sorted(found)
        found = []
        for line in lines:
            m = re.match(r"^\s+(--[A-Za-z0-9][\w\-]*)", line)
            if m and m.group(1) not in found:
                found.append(m.group(1))
        return sorted(found)
    found = []
    for tok in re.findall(r"--[A-Za-z][\w\-]*", help_text):
        if tok not in found:
            found.append(tok)
    return sorted(found)


# --------------------------------------------------------------------------- #
# 单条记录
# --------------------------------------------------------------------------- #
def default_invocation(rel: str) -> str:
    if rel.endswith(".py"):
        return f"python3 {rel}"
    if rel.endswith(".sh"):
        return f"bash {rel}"
    return rel


def build_record(path: Path, ci_index: dict[str, dict[str, list[str]]]) -> dict:
    rel = path.relative_to(ROOT).as_posix()
    src = read_text(path)
    if src is None:
        return {
            "id": make_id(rel_stem(path)),
            "path": rel,
            "kind": "unknown",
            "blocking": "unknown",
            "invocation": [default_invocation(rel)],
            "ci_workflows": [],
            "has_baseline": False,
            "notes": "",
            "flags": [],
        }

    # ---- --help 探测（不安全就不跑）----
    flags: list[str] = []
    if supports_help(path.suffix == ".py", src):
        flags, _status = run_help(path, path.suffix)

    # ---- notes：只认脚本自己的 docstring / 注释头 ----
    if path.suffix == ".py":
        notes = summarize(module_docstring(src))
    else:
        notes = summarize(shell_header_comment(src, path.name))

    blocking = detect_blocking(src, path.suffix)
    has_baseline = detect_baseline(src, path.suffix, path)
    regen = detect_regen(src, flags)
    check_mode = looks_like_checker(rel_stem(path)) or detect_check_mode(src, flags)
    manipulates = detect_manipulates(src, path.suffix)

    if has_baseline and regen:
        kind = "ratchet"
    elif check_mode and blocking is True:
        kind = "guard"
    elif check_mode and blocking is False:
        kind = "report"
    elif manipulates:
        kind = "generator"
    elif blocking is True:
        kind = "guard"
    elif blocking is False and detect_report_only(src, flags):
        kind = "report"
    else:
        kind = "unknown"

    cmds = ci_commands_for(rel, ci_index)
    invocation = cmds if cmds else [default_invocation(rel)]
    ci_wfs = ci_workflow_files_for(rel, ci_index)

    return {
        "id": make_id(rel_stem(path)),
        "path": rel,
        "kind": kind,
        "blocking": blocking,
        "invocation": invocation,
        "ci_workflows": ci_wfs,
        "has_baseline": has_baseline,
        "notes": notes,
        "flags": flags,
    }


# --------------------------------------------------------------------------- #
# 主流程
# --------------------------------------------------------------------------- #
def discover(ci_index: dict[str, dict[str, list[str]]]) -> list[Path]:
    """按发现规则挑出所有守卫脚本。"""
    picked: dict[str, Path] = {}
    for p in iter_script_files():
        if looks_like_guard(rel_stem(p)):
            picked[p.as_posix()] = p
    for p in all_scripts_under_scripts():
        rel = p.relative_to(ROOT).as_posix()
        if rel in ci_index or rel in picked:
            picked[p.as_posix()] = p
    for rel in ci_index:
        p = ROOT / rel
        if p.is_file():
            picked[p.as_posix()] = p
    return [picked[k] for k in sorted(picked)]


def build_inventory() -> dict:
    ci_index = ci_command_index()
    records = []
    used_ids: dict[str, str] = {}
    for path in discover(ci_index):
        rec = build_record(path, ci_index)
        if rec["id"] in used_ids:
            parent = re.sub(r"[^a-z0-9]+", "-", path.parent.name.lower()).strip("-")
            rec["id"] = f"{rec['id']}-{parent}" if parent else f"{rec['id']}-dup"
        used_ids[rec["id"]] = rec["path"]
        records.append(rec)
    records.sort(key=lambda r: (r["id"], r["path"]))
    return {
        "schema": SCHEMA,
        "generated_from": GENERATOR,
        "guards": records,
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="生成 scripts/guard-inventory.json")
    ap.add_argument("--check", action="store_true", help="只算不写；与现文件有差异则退出 1")
    args = ap.parse_args(argv[1:])

    inv = build_inventory()
    text = json.dumps(inv, ensure_ascii=False, indent=2, sort_keys=False) + "\n"

    if args.check:
        current = OUT_PATH.read_text(encoding="utf-8") if OUT_PATH.is_file() else ""
        if current != text:
            print(f"[guard-inventory] OUT OF SYNC: {OUT_PATH.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print("[guard-inventory] in sync")
        return 0

    OUT_PATH.write_text(text, encoding="utf-8")

    counts: dict[str, int] = {}
    for rec in inv["guards"]:
        counts[rec["kind"]] = counts.get(rec["kind"], 0) + 1
    blocking_true = sum(1 for r in inv["guards"] if r["blocking"] is True)
    blocking_false = sum(1 for r in inv["guards"] if r["blocking"] is False)
    blocking_unknown = sum(1 for r in inv["guards"] if r["blocking"] == "unknown")
    summary = " ".join(f"{k}={counts[k]}" for k in sorted(counts))
    print(
        f"[guard-inventory] {len(inv['guards'])} guards -> {OUT_PATH.relative_to(ROOT)} "
        f"| {summary} | blocking=true:{blocking_true} false:{blocking_false} "
        f"unknown:{blocking_unknown}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
