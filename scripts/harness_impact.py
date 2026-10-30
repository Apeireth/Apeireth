#!/usr/bin/env python3
"""**变更影响面分级**（harness impact）—— L0..L4 + 建议验证深度。

# 它解决什么

仓库要落两段式验证（fast = 改动 crate + 反向依赖的 check/clippy/test；deep = 更宽的验证面）。
「这一批改动该跑哪一段」不能靠人凭感觉拍 —— 必须由**改动落在哪个边界上**推出来。
本脚本只做**分类**，不执行任何门禁。

> **重要边界（不要误读）**：本脚本给出的等级是**建议值**（suggestion），
> **两段式 runner 才是执行者、才是判定者**。
> 本脚本**不替代人工判断**，也**不声称**「L1 就一定安全」——
> 它只是把「为什么这次改动可能影响更广」变成**机器可读、可复现**的一条理由。
> 等级高 ⇒ 建议跑得更深；等级低 ⇒ 允许更快，但**允许**不等于**必须**。

# 等级语义

| 等级 | 语义 | 典型落点 |
|------|------|----------|
| **L0** | 内部实现 / 文档 / 纯格式 | 只有 `.md`/配置、只有测试文件、只有注释或空白改动 |
| **L1** | 模块行为 | 某个 `.rs` 模块的内部实现变了（无公共 API 变更） |
| **L2** | 公共 API | diff 里新增/删除了 `pub` 项 ⇒ 依赖方的编译面可能破 |
| **L3** | 运行时接线 / 装配 | 装配层（runtime/assembly/adapter/...）、crate 入口 `src/lib.rs`、或**高扇出 crate** |
| **L4** | 安全 / 权限 / 持久化 / 迁移边界 | guard/governance/权限策略、schema/迁移/SQL、`Cargo.lock` |

# 规则表（**确定性**：最高命中的规则决定等级，**每一条命中的规则都会写进 `level_reasons`**）

* **L4**
  * 安全/治理路径：`crates/engine/guard/**`、`crates/foundation/governance/**`，
    或路径里出现 `approval` / `permission` / `policy` / `auth` / `capability`（大小写不敏感）。
  * 持久化/迁移路径：路径含 `migration` / `migrate`、文件是 `*.sql`，
    或文件位于 `schema` / `persistence` / `store` 目录之下。
  * `Cargo.lock`（锁文件级别 ⇒ 影响面按全 workspace 计）。
* **L3**
  * 路径命中 `runtime` / `assembly` / `coordinator` / `wiring` / `adapter` / `bootstrap`，
    或命中某个 crate 的 `src/lib.rs`（且**不是**只加文档注释）。
    —— 关键词在**目录段**上一律判定；**文件名**只在它是代码文件（`.rs`/`.py`/…）时判定，
    避免 `docs/status/wiring.yaml` 这类台账文档被误判成 L3。
  * **高扇出 crate**：该 crate 被 ≥3 个其它 workspace crate 依赖（行为可远距扩散）。
    —— 该规则需要**行为面证据**：diff 里出现公共 API 变更、或改到 crate 入口 `src/lib.rs`、
    或 diff 里有实质代码改动。`--files` 模式**没有 diff 内容** ⇒ 此时只有 crate 入口能提供该证据。
* **L2**：某个 `.rs` 文件的 diff 出现 `^[+-]\\s*pub(\\s|\\()` —— 但 `pub(crate)` / `pub(super)` /
  `pub(in …)` 是**crate 内部**可见 ⇒ 记 **L1**，不算公共 API。
* **L1**：有 `.rs` 文件被改，且该改动**不是**纯公共 API、**也不是**纯注释/测试。
* **L0**：以上都没命中 —— 只有非 `.rs` 文件、只有测试文件、或只有注释/空白/格式改动。

# 依赖影响面

* 改动文件 → crate：向上找**最近的**含 `Cargo.toml` 的祖先目录，且该目录属于 workspace 包。
* 反向依赖：workspace 内「被依赖者 → 依赖者」边的**传递闭包**（改 core 会波及 cli、再波及 gateway）。
* `Cargo.toml` / `Cargo.lock` 变更 ⇒ 影响面按**全 workspace** 计（与 `scripts/verify_changed.py` 同口径）。
* `cargo metadata` 不可用（离线/无 cargo）⇒ **不崩**：影响面退化为「直接依赖估计（保守）」。

# 用法

```sh
python3 scripts/harness_impact.py                      # 默认 base=HEAD~1，含当前工作树改动
python3 scripts/harness_impact.py --base origin/main   # 指定 base
python3 scripts/harness_impact.py --files a.rs,b.rs     # 显式文件列表（测试/手动；覆盖 diff 检测）
python3 scripts/harness_impact.py --json                # 只输出 JSON（机器消费）
python3 scripts/harness_impact.py --quiet               # 不输出，只给退出码
```

退出码：正常 **0**；仅内部错误 **1**。

# 只用标准库

Python 3 标准库实现（`argparse` / `json` / `os` / `re` / `subprocess`），无第三方依赖。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = "apeireth.harness-impact.v1"
DEFAULT_BASE = "HEAD~1"

#: 每条规则最多列出多少个文件（超出折叠成一行计数），保证输出紧凑且确定
MAX_REASONS = 5

# ---------------------------------------------------------------- 关键词表

#: 安全 / 权限 / 治理边界：固定目录（仓库根相对）
SECURITY_DIRS = ("crates/engine/guard", "crates/foundation/governance")
#: 安全 / 权限关键词（大小写不敏感；按「路径段 / 文件名」做子串匹配）
SECURITY_WORDS = ("approval", "permission", "policy", "auth", "capability")

#: 持久化 / 迁移关键词
PERSIST_WORDS = ("migration", "migrate")
#: 持久化 / 迁移目录（目录名等于或以其开头）
PERSIST_DIRS = ("schema", "persistence", "store")

#: 运行时接线 / 装配关键词
RUNTIME_WORDS = ("runtime", "assembly", "coordinator", "wiring", "adapter", "bootstrap")

#: 代码文件后缀 —— 运行时关键词**只在目录段、以及代码文件的文件名**上判定；
#: 文档/配置（`.md` / `.yaml` / `.toml` …）不因文件名撞词就算「运行时接线」
#: （否则 `docs/status/wiring.yaml` 这种台账文档会被误判成 L3）。
CODE_SUFFIXES = (
    ".rs",
    ".py",
    ".sh",
    ".bash",
    ".zsh",
    ".ps1",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".go",
    ".rb",
    ".c",
    ".h",
    ".cc",
    ".cpp",
    ".hpp",
    ".java",
    ".kt",
)

#: CI / 脚本目录
CI_DIRS = (".github", ".githooks", "scripts")

#: 高扇出阈值：被 ≥ N 个其它 workspace crate 依赖
HUB_THRESHOLD = 3

#: 公共 API 行：`pub fn` / `pub struct` / `pub use` …；但 `pub(crate)` / `pub(super)` / `pub(in …)` 不算
PUB_RE = re.compile(r"^[+-]\s*pub(?:\s|\((?!crate\b|super\b|in\b))")

#: `author` / `authors` 里含有 "auth" 但**不是**权限边界 ⇒ 排除，避免误判
AUTHOR_RE = re.compile(r"\bauthors?\b")


# ---------------------------------------------------------------- 小工具


def _run(cmd: list[str], timeout: int = 180) -> tuple[int, str]:
    """跑一个命令并拿到 (returncode, stdout+stderr)。失败不抛异常。"""
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except (OSError, subprocess.SubprocessError):
        return 127, ""
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def _norm(path: str) -> str:
    """规范化成仓库根相对、正斜杠分隔的路径。"""
    p = str(path).strip().replace("\\", "/")
    while p.startswith("./"):
        p = p[2:]
    p = p.strip().strip('"')
    if p.startswith("/"):
        try:
            p = str(Path(p).resolve().relative_to(ROOT))
        except (ValueError, OSError):
            p = p.lstrip("/")
    return p


def _segments(path: str) -> list[str]:
    return [seg for seg in path.split("/") if seg]


def _dirs_of(path: str) -> list[str]:
    segs = _segments(path)
    return segs[:-1]


def _filename_of(path: str) -> str:
    segs = _segments(path)
    return segs[-1] if segs else ""


def _seg_has_word(seg: str, word: str) -> bool:
    """路径段是否命中关键词（大小写不敏感，子串匹配）。"""
    s = seg.lower()
    if word not in s:
        return False
    if word == "auth" and AUTHOR_RE.search(s):
        return False  # author / authors 不是权限边界
    return True


def _is_under(path: str, prefixes: tuple[str, ...]) -> str | None:
    """路径是否位于某个前缀目录之下，返回命中的前缀（否则 None）。"""
    for prefix in prefixes:
        if path == prefix or path.startswith(prefix + "/"):
            return prefix
    return None


def _is_comment_line(line: str) -> bool:
    """diff 行去掉 +/- 之后是否是空行或注释。"""
    body = line[1:].strip()
    if not body:
        return True
    return body.startswith("//") or body.startswith("/*") or body.startswith("*")


# ---------------------------------------------------------------- workspace 图


def _load_workspace() -> dict | None:
    """读 `cargo metadata --no-deps`，建 workspace 包索引 + 反向依赖表。失败返回 None。"""
    code, out = _run(["cargo", "metadata", "--no-deps", "--format-version", "1"])
    if code != 0:
        return None
    try:
        meta = json.loads(out)
    except (json.JSONDecodeError, TypeError, ValueError):
        return None
    packages = meta.get("packages") or []
    if not packages:
        return None

    by_name: dict[str, dict] = {}
    dir_to_name: dict[str, str] = {}
    for pkg in packages:
        name = pkg.get("name")
        if not name:
            continue
        by_name[name] = pkg
        manifest = pkg.get("manifest_path")
        if not manifest:
            continue
        parent = Path(manifest).parent
        for key in {str(parent), str(parent.resolve())}:
            dir_to_name[key] = name

    # 反向依赖：被依赖者 -> 依赖者
    reverse: dict[str, set[str]] = {}
    try:
        root_resolved = ROOT.resolve()
    except OSError:
        root_resolved = ROOT
    for pkg in packages:
        dependent = pkg.get("name")
        if not dependent:
            continue
        for dep in pkg.get("dependencies") or []:
            dep_name = dep.get("name")
            if not dep_name or dep_name == dependent:
                continue  # 自环无意义
            if dep_name not in by_name:
                continue  # 不是 workspace 包
            dep_path = dep.get("path")
            if dep_path:
                # 路径依赖必须落在本 workspace 之内
                try:
                    Path(dep_path).resolve().relative_to(root_resolved)
                except (ValueError, OSError):
                    continue
            reverse.setdefault(dep_name, set()).add(dependent)

    return {
        "packages": by_name,
        "dir_to_name": dir_to_name,
        "reverse": reverse,
        "all_names": sorted(by_name),
    }


def _workspace_by_scan() -> dict:
    """cargo metadata 不可用时的兜底：扫盘找 Cargo.toml，只拿 crate 名（无依赖图）。"""
    dir_to_name: dict[str, str] = {}
    skip_dirs = {"target", "node_modules", ".git", "artifacts", "logs", "reports", ".venv", "dist", "build"}
    name_re = re.compile(r"^\s*name\s*=\s*\"([^\"]+)\"", re.MULTILINE)
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in skip_dirs]
        if "Cargo.toml" not in filenames:
            continue
        try:
            text = (Path(dirpath) / "Cargo.toml").read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        match = name_re.search(text)
        if not match:
            continue
        directory = Path(dirpath)
        for key in {str(directory), str(directory.resolve())}:
            dir_to_name[key] = match.group(1)
    return {
        "packages": {},
        "dir_to_name": dir_to_name,
        "reverse": {},
        "all_names": sorted(set(dir_to_name.values())),
    }


def _crate_for_path(path: str, dir_to_name: dict[str, str]) -> str | None:
    """改动文件 → crate 名（最近的、属于 workspace 的祖先目录）。"""
    root = _crate_root_for_path(path, dir_to_name)
    if root is None:
        return None
    return dir_to_name[str(root)]


def _crate_root_for_path(path: str, dir_to_name: dict[str, str]) -> Path | None:
    """改动文件 → 其 crate 根目录（最近的、属于 workspace 的祖先目录）。"""
    try:
        current = (ROOT / path).resolve()
    except OSError:
        current = ROOT / path
    for parent in [current, *current.parents]:
        if str(parent) in dir_to_name:
            return parent
    return None


def _dependents_closure(seed: set[str], reverse: dict[str, set[str]]) -> set[str]:
    """反向依赖的传递闭包。"""
    seen: set[str] = set()
    stack = list(seed)
    while stack:
        current = stack.pop()
        for dependent in reverse.get(current, ()):  # type: ignore[arg-type]
            if dependent not in seen:
                seen.add(dependent)
                stack.append(dependent)
    return seen


# ---------------------------------------------------------------- 改动检测


def _resolve_base(base: str) -> tuple[str, bool]:
    """把 base 解析成 sha；解析不了就原样返回。"""
    code, out = _run(["git", "rev-parse", "--verify", f"{base}^{{commit}}"])
    if code == 0 and out.strip():
        return out.strip().splitlines()[0].strip(), True
    return base, False


def _parse_diff(out: str, target: dict[str, list[str]]) -> None:
    """解析 `git diff -U0` 输出 ⇒ {文件: [增删行]}。"""
    current: str | None = None
    for line in out.splitlines():
        if line.startswith("diff --git "):
            current = None
            continue
        if line.startswith("+++ "):
            target_path = line[4:].strip()
            if target_path == "/dev/null":
                current = None
                continue
            if target_path.startswith("b/"):
                target_path = target_path[2:]
            current = _norm(target_path)
            target.setdefault(current, [])
            continue
        if line.startswith("--- "):
            continue
        if current is None:
            continue
        if line.startswith("+") or line.startswith("-"):
            target[current].append(line)


def _detect_changes(base_sha: str) -> tuple[list[str], dict[str, list[str]], list[str]]:
    """检测改动文件 + 每个文件的 diff 行。返回 (files, diff_map, notes)。"""
    notes: list[str] = []
    files: list[str] = []
    diff_map: dict[str, list[str]] = {}

    code, out = _run(["git", "diff", "--name-only", f"{base_sha}...HEAD"])
    if code != 0:
        code, out = _run(["git", "diff", "--name-only", base_sha])
    if code != 0:
        notes.append("git diff 失败 ⇒ 改动集合退化为仅工作树状态")
    for line in out.splitlines():
        if line.strip():
            files.append(_norm(line))

    # 当前工作树（含已暂存 / 未暂存）——两级验证关心的是「眼前这批改动」
    code, out = _run(["git", "status", "--porcelain"])
    if code == 0:
        for line in out.splitlines():
            if not line.strip():
                continue
            path = line[3:].strip()
            if " -> " in path:  # 重命名：取新路径
                path = path.split(" -> ", 1)[1]
            path = path.strip().strip('"')
            if path:
                files.append(_norm(path))
        notes.append("改动集合 = base..HEAD 的提交差异 ∪ 当前工作树（含未暂存）改动")
    else:
        notes.append("git status 失败 ⇒ 未纳入工作树未提交改动")

    # diff 内容（提交差异 + 工作树差异）
    code, out = _run(["git", "diff", "-U0", "--no-color", f"{base_sha}...HEAD"])
    if code == 0:
        _parse_diff(out, diff_map)
    code, out = _run(["git", "diff", "-U0", "--no-color", "HEAD"])
    if code == 0:
        _parse_diff(out, diff_map)

    return sorted({f for f in files if f}), diff_map, notes


# ---------------------------------------------------------------- 路径判定


def _security_hit(path: str) -> str | None:
    """命中安全/权限/治理边界？返回命中依据。"""
    hit = _is_under(path, SECURITY_DIRS)
    if hit:
        return hit
    for seg in _segments(path):
        for word in SECURITY_WORDS:
            if _seg_has_word(seg, word):
                return f"{word}({seg})"
    return None


def _persist_hit(path: str) -> str | None:
    """命中持久化/迁移边界？返回命中依据。"""
    for seg in _segments(path):
        for word in PERSIST_WORDS:
            if _seg_has_word(seg, word):
                return f"{word}({seg})"
    filename = _filename_of(path)
    if filename.lower().endswith(".sql"):
        return "*.sql"
    for seg in _dirs_of(path):
        low = seg.lower()
        for directory in PERSIST_DIRS:
            if low == directory or low.startswith(directory):
                return f"{directory}/({seg})"
    return None


def _runtime_hit(path: str) -> str | None:
    """命中运行时接线/装配？返回命中依据。

    目录段一律参与判定；**文件名只在它是代码文件时**参与判定 ——
    否则 `docs/status/wiring.yaml` 这类台账文档会被误判成 L3。
    """
    segs = _segments(path)
    for seg in segs[:-1]:
        for word in RUNTIME_WORDS:
            if _seg_has_word(seg, word):
                return f"{word}({seg})"
    filename = segs[-1] if segs else ""
    if filename and Path(filename).suffix.lower() in CODE_SUFFIXES:
        stem = filename.split(".")[0]
        for word in RUNTIME_WORDS:
            if _seg_has_word(stem, word):
                return f"{word}({stem})"
    return None


def _is_test_file(path: str) -> bool:
    """是否是测试文件（`tests/` 目录下的 .rs，或 `*_test.rs` / `*.test.rs`）。"""
    segs = _segments(path)
    if not segs:
        return False
    filename = segs[-1]
    if filename.endswith("_test.rs") or filename.endswith(".test.rs"):
        return True
    return "tests" in _dirs_of(path)


def _is_lib_entry(path: str, crate_root: Path | None) -> bool:
    """是否命中某个 crate 的 `src/lib.rs`。"""
    if crate_root is None:
        return False
    try:
        rel = (ROOT / path).resolve().relative_to(crate_root)
    except (ValueError, OSError):
        return False
    return rel.as_posix() == "src/lib.rs"


# ---------------------------------------------------------------- 主分析


def _analyze_files(files: list[str], diff_map: dict[str, list[str]]) -> dict[str, dict]:
    """逐个 .rs 文件给出改动性质。"""
    per_file: dict[str, dict] = {}
    for path in files:
        if not path.endswith(".rs"):
            continue
        lines = diff_map.get(path)
        if lines is None:
            # 没有 diff 内容（--files 模式等）⇒ 性质未知：
            # 公共 API 记 false（未知），但不提供「行为面证据」。
            per_file[path] = {"kind": "unknown", "pub": False, "substantive": False}
            continue
        file_pub = any(PUB_RE.match(line) for line in lines)
        file_sub = any(
            (not _is_comment_line(line)) and (not PUB_RE.match(line)) for line in lines
        )
        if file_pub and not file_sub:
            kind = "public_api"
        elif file_sub and not file_pub:
            kind = "behavior"
        elif file_pub and file_sub:
            kind = "mixed"
        else:
            kind = "comment"
        per_file[path] = {"kind": kind, "pub": file_pub, "substantive": file_sub}
    return per_file


def analyze(base: str, files_arg: str | None) -> dict:
    """产出完整的影响面报告（dict）。"""
    notes: list[str] = []

    base_sha, base_resolved = _resolve_base(base)
    if not base_resolved:
        notes.append(f"base `{base}` 无法解析为 commit ⇒ 按原样使用")

    diff_map: dict[str, list[str]] = {}
    if files_arg is not None:
        files = sorted({_norm(f) for f in files_arg.split(",") if f.strip()})
        notes.append("改动集合来自 --files（显式指定）⇒ 无 diff 内容 ⇒ public_api 记为 false（未知）")
    else:
        files, diff_map, detect_notes = _detect_changes(base_sha)
        notes.extend(detect_notes)

    ws = _load_workspace()
    metadata_ok = ws is not None
    if not metadata_ok:
        ws = _workspace_by_scan()
        notes.append("cargo metadata 不可用 ⇒ 影响面按直接依赖估计（保守）")

    dir_to_name: dict[str, str] = ws["dir_to_name"]
    reverse: dict[str, set[str]] = ws["reverse"]

    # ---- crate 归属
    crate_of: dict[str, str | None] = {}
    crate_root_of: dict[str, Path | None] = {}
    for path in files:
        crate_of[path] = _crate_for_path(path, dir_to_name)
        crate_root_of[path] = _crate_root_for_path(path, dir_to_name)

    touched_crates = sorted({c for c in crate_of.values() if c})

    # ---- 依赖影响面（反向依赖的传递闭包）
    dependents = _dependents_closure(set(touched_crates), reverse) - set(touched_crates)

    manifest_change = any(_filename_of(f) in ("Cargo.toml", "Cargo.lock") for f in files)
    root_manifest_change = any(f in ("Cargo.toml", "Cargo.lock") for f in files)
    if root_manifest_change and metadata_ok:
        dependents |= set(ws["all_names"]) - set(touched_crates)
        notes.append("workspace 清单/锁文件变更 ⇒ 影响面按全 workspace 计")

    affected_crates = sorted(set(touched_crates) | dependents)

    # ---- 逐文件证据
    per_file = _analyze_files(files, diff_map)
    rust_files = [f for f in files if f.endswith(".rs")]
    has_pub = any(info["pub"] for info in per_file.values())

    touches: dict[str, bool] = {
        "docs_only": bool(files)
        and all(f.endswith(".md") or _is_under(f, ("docs",)) for f in files),
        "ci_or_scripts": any(_is_under(f, CI_DIRS) for f in files),
        "public_api": has_pub,
        "persisted_schema": any(_persist_hit(f) for f in files),
        "security_boundary": any(_security_hit(f) for f in files),
        "runtime_wiring": False,
        "dependency_manifest": manifest_change,
        "tests_only": bool(rust_files) and all(_is_test_file(f) for f in rust_files),
    }

    reasons: list[tuple[str, str]] = []

    # ---- L4：安全 / 权限 / 治理
    sec_hits = [(f, _security_hit(f)) for f in files if _security_hit(f)]
    for path, hit in sec_hits[:MAX_REASONS]:
        reasons.append(("L4", f"安全/权限边界变更：{path}（命中 {hit}）"))
    if len(sec_hits) > MAX_REASONS:
        reasons.append(("L4", f"另有 {len(sec_hits) - MAX_REASONS} 个文件命中安全/权限边界"))

    # ---- L4：持久化 / 迁移
    persist_hits = [(f, _persist_hit(f)) for f in files if _persist_hit(f)]
    for path, hit in persist_hits[:MAX_REASONS]:
        reasons.append(("L4", f"持久化/迁移边界变更：{path}（命中 {hit}）"))
    if len(persist_hits) > MAX_REASONS:
        reasons.append(("L4", f"另有 {len(persist_hits) - MAX_REASONS} 个文件命中持久化/迁移边界"))

    # ---- L4：依赖锁文件
    if "Cargo.lock" in files:
        reasons.append(("L4", "依赖锁文件变更：Cargo.lock ⇒ 影响面按全 workspace 计"))

    # ---- L3：crate 公共入口（不是只加文档注释）
    lib_entry_files: list[str] = []
    for path in rust_files:
        if not _is_lib_entry(path, crate_root_of.get(path)):
            continue
        if per_file.get(path, {}).get("kind") == "comment":
            continue  # 只加文档注释 ⇒ 不算运行时接线
        lib_entry_files.append(path)
        touches["runtime_wiring"] = True
        reasons.append(("L3", f"crate 公共入口变更：{path}（运行时接线面）"))

    # ---- L3：运行时接线 / 装配路径
    runtime_hits = [(f, _runtime_hit(f)) for f in files if _runtime_hit(f)]
    for path, hit in runtime_hits[:MAX_REASONS]:
        touches["runtime_wiring"] = True
        reasons.append(("L3", f"运行时接线/装配路径：{path}（命中 {hit}）"))
    if len(runtime_hits) > MAX_REASONS:
        reasons.append(("L3", f"另有 {len(runtime_hits) - MAX_REASONS} 个文件命中运行时接线/装配路径"))

    # ---- L3：高扇出 crate（被 ≥3 个 workspace crate 依赖）
    if metadata_ok:
        for crate in touched_crates:
            dep_count = len(reverse.get(crate, set()))
            if dep_count < HUB_THRESHOLD:
                continue
            crate_files = [f for f in files if crate_of.get(f) == crate]
            crate_api = any(per_file.get(f, {}).get("pub") for f in crate_files)
            crate_sub = any(per_file.get(f, {}).get("substantive") for f in crate_files)
            crate_lib = any(f in lib_entry_files for f in crate_files)
            if not (crate_api or crate_sub or crate_lib):
                continue  # 没有行为面证据 ⇒ 不做扇出推断（--files 模式下尤其重要）
            reasons.append(
                (
                    "L3",
                    f"高扇出 crate：{crate} 被 {dep_count} 个 workspace crate 依赖 ⇒ 行为面可远距扩散",
                )
            )

    # ---- L2：公共 API
    pub_files = [f for f in rust_files if per_file.get(f, {}).get("pub")]
    for path in pub_files[:MAX_REASONS]:
        reasons.append(("L2", f"公共 API 变更：{path} 新增/删除 pub 项"))
    if len(pub_files) > MAX_REASONS:
        reasons.append(("L2", f"另有 {len(pub_files) - MAX_REASONS} 个文件新增/删除 pub 项"))

    # ---- L1：模块行为（非公共 API、非纯注释、非测试）
    behavioural_rs = [
        f
        for f in rust_files
        if not _is_test_file(f)
        and per_file.get(f, {}).get("kind") in ("behavior", "mixed", "unknown")
    ]
    if behavioural_rs:
        head = behavioural_rs[0]
        extra = f"（共 {len(behavioural_rs)} 个 .rs 文件）" if len(behavioural_rs) > 1 else ""
        reasons.append(("L1", f"模块行为变更：{head}{extra}（crate 内实现变化，无公共 API 变更）"))

    # ---- 定级：命中最高者；一条都没命中 ⇒ L0
    level_order = {"L0": 0, "L1": 1, "L2": 2, "L3": 3, "L4": 4}
    if reasons:
        level = max((lvl for lvl, _ in reasons), key=lambda lvl: level_order[lvl])
        level_reasons = [reason for _lvl, reason in reasons]
    else:
        level = "L0"
        if not files:
            level_reasons = ["无改动文件"]
        elif touches["docs_only"]:
            level_reasons = ["仅文档变更（.md / docs/）"]
        elif touches["tests_only"]:
            level_reasons = ["仅测试文件变更"]
        elif not rust_files:
            level_reasons = ["仅非 Rust 文件变更（文档/配置/脚本）"]
        else:
            level_reasons = ["仅内部实现/注释/格式变更（无行为面证据）"]

    # ---- 建议阶段
    stages = {
        "L0": ["fast"],
        "L1": ["fast"],
        "L2": ["fast", "deep"],
        "L3": ["fast", "deep"],
        "L4": ["fast", "deep", "security"],
    }[level]

    notes.append(
        "等级是建议值：两段式 runner（fast/deep/security）据此决定跑什么，"
        "最终判定由 runner 与人工负责 —— 本脚本不替代判断"
    )
    if level == "L4":
        notes.append("L4 的 `security` 阶段是预留名称，未来实现；现在仍如实上报")

    return {
        "schema": SCHEMA,
        "base": base_sha,
        "changed_files": files,
        "changed_count": len(files),
        "touched_crates": touched_crates,
        "affected_crates": affected_crates,
        "reverse_dependents": sorted(dependents),
        "touches": touches,
        "level": level,
        "level_reasons": level_reasons,
        "suggested_stages": stages,
        "notes": "；".join(notes),
    }


# ---------------------------------------------------------------- 输出


def _json_text(payload: dict) -> str:
    text = json.dumps(payload, ensure_ascii=False, indent=2)
    try:
        text.encode(sys.stdout.encoding or "utf-8")
    except (UnicodeEncodeError, LookupError):
        text = json.dumps(payload, ensure_ascii=True, indent=2)
    return text


def _print_human(result: dict) -> None:
    touches = result["touches"]
    print(f"影响面等级: {result['level']}   建议阶段: {', '.join(result['suggested_stages'])}")
    print(f"base: {result['base']}")
    print(f"改动文件 ({result['changed_count']}):")
    for path in result["changed_files"]:
        print(f"  - {path}")
    print("touch: " + "  ".join(f"{k}={str(v).lower()}" for k, v in touches.items()))
    print(f"touched crates: {', '.join(result['touched_crates']) or '-'}")
    print(f"affected crates: {', '.join(result['affected_crates']) or '-'}")
    print(f"reverse dependents: {', '.join(result['reverse_dependents']) or '-'}")
    print("理由:")
    for reason in result["level_reasons"]:
        print(f"  - {reason}")
    print(f"备注: {result['notes']}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="harness_impact.py",
        description="变更影响面分级（L0..L4）+ 建议验证深度（fast/deep/security）",
        epilog="等级是建议值：两段式 runner 决定实际跑什么；本脚本不替代判断。",
    )
    parser.add_argument("--base", default=DEFAULT_BASE, help=f"diff base（默认 {DEFAULT_BASE}）")
    parser.add_argument(
        "--files", default=None, help="逗号分隔的改动文件（覆盖 diff 检测；测试/手动用）"
    )
    parser.add_argument(
        "--json", dest="json_output", action="store_true", help="只输出 JSON（机器消费）"
    )
    parser.add_argument("--quiet", action="store_true", help="不输出，只给退出码")
    args = parser.parse_args(argv)

    try:
        sys.stdout.reconfigure(errors="backslashreplace")  # type: ignore[attr-defined]
    except (AttributeError, OSError, ValueError):
        pass

    try:
        result = analyze(args.base, args.files)
    except Exception as exc:  # noqa: BLE001 —— 内部错误 ⇒ 退出码 1
        if not args.quiet:
            print(f"harness_impact: internal error: {exc}", file=sys.stderr)
        return 1

    if args.quiet:
        return 0
    if args.json_output:
        print(_json_text(result))
    else:
        _print_human(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
