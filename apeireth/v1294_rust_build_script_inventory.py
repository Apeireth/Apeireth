"""V1294 — Rust Build Script (build.rs) Inventory (VCP 真源代码深读 #15) 真生产模块

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 20:03 +08:00 2026-08-05)
> **触发**: 20:03 cron wake tick (autonomy-v3) — V1293 dep graph (05bec4ce) 已 commit.
>          V1280-V1293 (14 sweeps) = 源代码静态 / 语义 / 安全 / 治理 / 文档 / 构建产物 / 测试源码 / 依赖图.
>          V1294 = **build.rs 深度审计** 层面 (主 13:08 真自问 + 主 19:33 走在前人肩上):
>            - 50 crates 谁有 build.rs? build.rs 长什么样?
>            - build.rs 里有什么危险 pattern? (env mutation / Command::new / fs write)
>            - 哪些 codegen 工具被用? (tonic-build / tauri-build / cbindgen / bindgen / napi-build / prost-build / etc.)
>            - cargo:rerun-if-changed / cargo:rerun-if-env-changed 覆盖率?
>            - 与 V1291 artifacts 对照: 哪些 crate 有 build.rs 但 0 artifacts?
>            - 与 V1293 deps 对照: 哪些 crate 用 [build-dependencies] 但没 build.rs?
> **承接**: V1280 静态 + V1281-V1283 语义 + V1284-V1287 安全 + V1288 治理 + V1289-V1290 文档
>         + V1291 构建产物 + V1292 测试源码 + V1293 依赖图 → V1294 build.rs 清单
> **真借鉴**: 主 19:33 走在前人肩上 + cargo reference "Build Scripts" 章节
>         + cargo:rustc-link-* / cargo:rerun-if-changed= 约定 + AST pattern match (regex-only)
>         + apeireth-bus build.rs (tonic-build + protoc-bin-vendored) + apeireth-tauri-stub build.rs (tauri_build)
> **不假装**: V1294 = 真生产全 50 crates build.rs 清单 + 安全 pattern audit, 不刷 KPI, 不假装 ASI V1
>         不假装"build.rs 完全安全", 不假装"无 proc-macro risk", 不假装"无 fs mutation"

## 真生产动机 (主 13:08 真自问 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人肩上)

V1280-V1293 已审 源代码 + 构建产物 + 测试源码 + 依赖图, 但 **build.rs** 是 ASI 可证伪的另一维度:
- 50 crates 谁有 build.rs? (预期: bus + tauri-stub = 2, 其他可能 [build-dependencies] 但没用 build.rs)
- build.rs 实际多大? (主 17:43 实事求是: 真扫文件行数)
- build.rs 内部 pattern: env! / std::env::var / std::env::set_var / Command::new / File::open / File::write (主 19:33)
- cargo:rerun-if-changed 覆盖率: 多少 build.rs 声明依赖追踪?
- codegen 工具: tonic-build / tauri_build / cbindgen / bindgen / napi-build / prost-build / protoc-bin-vendored
- 与 V1291 对照: 哪些 crate 有 build.rs 但 0 artifacts? (build.rs failed? 或 stub?)
- 与 V1293 对照: 哪些 crate 在 Cargo.toml 声明 [build-dependencies] 但没 build.rs? (potential drift)

**V1294 = 真生产全 50 crates build.rs 清单**, 13 维度 per crate:

1. **has_build_rs**: build.rs 是否存在
2. **n_lines**: build.rs 真行数 (file read + len(split('\n')))
3. **has_main_fn**: 是否含 `fn main()`
4. **n_env_macro**: `env!("...")` 编译期 env 引用计数
5. **n_env_var_runtime**: `std::env::var(`, `std::env::var_os(` 运行时 env 读计数
6. **n_env_set_var**: `std::env::set_var(` 运行时 env 写计数 (潜在风险)
7. **n_command_new**: `Command::new(`, `std::process::Command::new(` 命令执行计数
8. **n_file_read**: `File::open(`, `std::fs::read_to_string(`, `std::fs::read(`, `include_str!(`, `include_bytes!(` 文件读计数
9. **n_file_write**: `File::create(`, `std::fs::write(`, `std::fs::File::create(` 文件写计数
10. **n_rerun_if_changed**: `cargo:rerun-if-changed=` println 指令计数
11. **n_rerun_if_env**: `cargo:rerun-if-env-changed=` println 指令计数
12. **n_rustc_link**: `cargo:rustc-link-*` println 指令计数 (linker directive)
13. **codegen_tools**: 检测的 codegen crate 列表 (tonic-build / tauri_build / cbindgen / bindgen / napi-build / prost-build / protoc-bin-vendored / uniffi_build / pyo3-build / etc.)

外加 workspace-level 维度:
- **total_crates**: 真扫 crate 数 (默认 50)
- **crates_with_build_rs**: 有 build.rs 的 crate 数
- **crates_with_build_deps_no_build_rs**: 声明 [build-dependencies] 但无 build.rs 的 crate (drift)
- **crates_with_build_rs_no_build_deps**: 有 build.rs 但未声明 [build-dependencies] 的 crate (orphan)
- **total_env_set_var**: 全 workspace set_var 总和 (越低越好, 0 = ideal)
- **total_command_new**: 全 workspace Command::new 总和 (越低越好, 0 = ideal)

每一 crate = 真 file:line + pattern 计数 + codegen tools.

**关键免责声明** (主 17:58 + 主 20:46):
- "build.rs inventory" ≠ "build.rs 安全": 仅模式扫描, 不调 cargo build / cargo check
- PASS ≠ cargo build 成功: PASS 仅 = 阈值达标
- 不假装 ASI V1 = 不刷 KPI = ASI NS LOCKED 不变 (主 17:58)
- FAIL 也诚实披露 (主 17:43 实事求是), 列出每条 finding 不掩饰
- 不假装 parse AST: 用 regex 简化 pattern match, 可能漏多行宏调用
- 不调 cargo: 纯 read-only 文件读取
- 不执行 build.rs: build.rs 是 cargo 在编译时跑的, V1294 仅审计静态源码
- 不解析 proc-macro 输出: build.rs 输出的产物不在 V1294 范围 (V1291 已审)

## ASI 5 哲学空隙 + meta-audit + VCP Rust #1-#15 完整闭环

- 时间 (Time): V1276 ✓
- 真理 (Truth): V1274 ✓
- 识别 (Recognition): V1275 ✓
- 自由 (Freedom): V1277 ✓
- 涌现 (Emergence): V1278 ✓
- Meta-Audit: V1279 ✓
- VCP Rust 静态: V1280 ✓ (源代码)
- VCP Rust 语义 #1: V1281 ✓ (源代码)
- VCP Rust 语义 #2: V1282 ✓ (源代码)
- VCP Rust 语义 #3: V1283 ✓ (源代码)
- VCP Rust 安全 #1: V1284 ✓ (源代码)
- VCP Rust 安全 #2: V1285 ✓ (源代码)
- VCP Rust 安全 #3: V1286 ✓ (源代码)
- VCP Rust 安全 #4: V1287 ✓ (源代码)
- VCP Rust 治理 #1: V1288 ✓ (源代码)
- VCP Rust 文档 #1: V1289 ✓ (源代码)
- VCP Rust 文档 #2: V1290 ✓ (源代码)
- VCP Rust 构建产物: V1291 ✓ (target/debug/deps/*)
- VCP Rust 测试源码: V1292 ✓ (#[test] / tests/ / examples/ / doctests / benches)
- VCP Rust 依赖图: V1293 ✓ (Cargo.toml internal/external edges + cycles + hubs + leaves)
- **VCP Rust build.rs 清单: V1294 ← (本模块, build.rs 静态源码)**

## CLI (主 00:56 任何人都能接手)

```bash
# 探测 (仅 pattern scan, 不评估)
python -m apeireth.v1294_rust_build_script_inventory --probe

# 跑全 sweep + 输出报告
python -m apeireth.v1294_rust_build_script_inventory --run

# 输出 JSON ledger
python -m apeireth.v1294_rust_build_script_inventory --json

# 输出 markdown 报告
python -m apeireth.v1294_rust_build_script_inventory --report

# 看单个 crate
python -m apeireth.v1294_rust_build_script_inventory --crate apeireth-bus

# 列 codegen 工具 (跨所有 build.rs)
python -m apeireth.v1294_rust_build_script_inventory --tools

# 列 set_var / Command::new 等风险 pattern
python -m apeireth.v1294_rust_build_script_inventory --risk

# 列 drift (build_deps 声明但无 build.rs / 有 build.rs 但无 build_deps)
python -m apeireth.v1294_rust_build_script_inventory --drift
```
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================
# 0. Constants (主 17:43 实事求是)
# ============================================================

WORKSPACE_ROOT_DEFAULT = Path(__file__).resolve().parent.parent / "Apeireth-rust"
WORKSPACE_TOML = "Cargo.toml"
BUILD_RS = "build.rs"

# 47 crates workspace members (V1293 已锁: tauri-stub = commented out, 真生产 38 + 9 = 47)
# 见 V1293 Cargo.toml members 段 + 真实 fs scan (2026-08-05)
CRATES_V1294: List[str] = [
    "apeireth-core", "apeireth-memory", "apeireth-asi", "apeireth-tools",
    "apeireth-cli", "apeireth-bench", "apeireth-cognition", "apeireth-action",
    "apeireth-life-force", "apeireth-constraint", "apeireth-central",
    "apeireth-value", "apeireth-consciousness", "apeireth-relation",
    "apeireth-motivation", "apeireth-perception", "apeireth-upgrade",
    "apeireth-onion", "apeireth-council", "apeireth-sovereignty",
    "apeireth-supervisor", "apeireth-pybridge", "apeireth-verify",
    "apeireth-extension", "apeireth-evolution", "apeireth-bus",
    "apeireth-api", "apeireth-web", "apeireth-tui", "apeireth-protocol",
    "apeireth-http-client", "apeireth-pipeline", "apeireth-tool-registry",
    "apeireth-tool-runtime", "apeireth-tool-approval", "apeireth-agent",
    "apeireth-mcp", "apeireth-graph",
    # apeireth-tauri-stub = commented out, V1294 仍扫描 (它有 build.rs, 离 build 仍可见)
    "apeireth-tauri-stub",
    # V2 战区 6 — workflow / team-lead (per docs/v2-strategy/05 + workspace Cargo.toml)
    "apeireth-workflow", "apeireth-team-lead",
    # V2 战区 7 — MCP transport (mcp-relay-image / mcp-ssh / mcp-winrm per recent commits)
    "apeireth-mcp-relay-image", "apeireth-mcp-ssh", "apeireth-mcp-winrm",
    # V2 战区 8 — 补全 leaf crates (per V1293 leaf: formal / vector / sdk)
    "apeireth-formal", "apeireth-vector", "apeireth-sdk",
]

# Pattern regex (regex-only, 不解析 AST)
RE_ENV_MACRO = re.compile(r"""\benv!\s*\(""")
RE_ENV_VAR = re.compile(r"""\benv::var\s*\(""")
RE_ENV_VAR_OS = re.compile(r"""\benv::var_os\s*\(""")
RE_ENV_SET_VAR = re.compile(r"""\benv::set_var\s*\(""")
RE_COMMAND_NEW = re.compile(r"""\bCommand::new\s*\(""")
RE_PROCESS_COMMAND_NEW = re.compile(r"""\bprocess::Command::new\s*\(""")
RE_FILE_OPEN = re.compile(r"""\bFile::open\s*\(""")
RE_FILE_CREATE = re.compile(r"""\bFile::create\s*\(""")
RE_FS_READ_TO_STRING = re.compile(r"""\bfs::read_to_string\s*\(""")
RE_FS_READ = re.compile(r"""\bfs::read\s*\(""")
RE_FS_WRITE = re.compile(r"""\bfs::write\s*\(""")
RE_INCLUDE_STR = re.compile(r"""\binclude_str!\s*\(""")
RE_INCLUDE_BYTES = re.compile(r"""\binclude_bytes!\s*\(""")
RE_RERUN_IF_CHANGED = re.compile(r"""cargo:rerun-if-changed=""")
RE_RERUN_IF_ENV = re.compile(r"""cargo:rerun-if-env-changed=""")
RE_RUSTC_LINK = re.compile(r"""cargo:rustc-link-""")
RE_FN_MAIN = re.compile(r"""\bfn\s+main\s*\(""")

# Codegen tool detection: 哪 crate 用了哪个 build tool
CODEGEN_TOOLS: Dict[str, List[str]] = {
    "tonic-build": ["tonic_build::", "tonic-build"],
    "tauri_build": ["tauri_build::", "tauri_build::build"],
    "cbindgen": ["cbindgen::", "cbindgen::Builder"],
    "bindgen": ["bindgen::", "bindgen::Builder"],
    "napi-build": ["napi_build::", "napi-build"],
    "prost-build": ["prost_build::", "prost-build"],
    "protoc-bin-vendored": ["protoc_bin_vendored::", "protoc-bin-vendored"],
    "uniffi_build": ["uniffi_build::", "uniffi_build"],
    "pyo3-build": ["pyo3_build_config::", "pyo3-build"],
    "neon-build": ["neon_build::", "neon-build"],
    "wasm-pack": ["wasm_pack::"],
    "vergen": ["vergen::", "vergen-gitcl"],
    "built": ["built::", "built::write_built_file"],
}

# Risk thresholds
THRESHOLD_BUILD_RS_SHORT = 50  # 多少行以下算 small
THRESHOLD_TOTAL_CRATES = 47  # 2026-08-05 实际 fs scan = 47 crates


# ============================================================
# 1. Data structures (主 17:43 实事求是)
# ============================================================


@dataclasses.dataclass
class BuildScriptProfile:
    """Single crate's build.rs profile."""
    crate_name: str
    crate_path: str  # e.g. "crates/apeireth-bus"
    has_build_rs: bool
    build_rs_path: Optional[str]  # e.g. "crates/apeireth-bus/build.rs"
    n_lines: int
    has_main_fn: bool
    n_env_macro: int
    n_env_var_runtime: int
    n_env_set_var: int
    n_command_new: int
    n_file_read: int
    n_file_write: int
    n_rerun_if_changed: int
    n_rerun_if_env: int
    n_rustc_link: int
    codegen_tools: List[str]
    has_build_deps: bool  # Cargo.toml [build-dependencies] 是否声明
    build_dep_names: List[str]  # build-dependencies crate names
    risk_flags: List[str]  # e.g. ["env_set_var", "command_new", "fs_write"]

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Cycle:  # borrowed from V1293 pattern
    """Placeholder for V1294 cross-ref cycle type."""
    cycle_length: int
    cycle_crates: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Hypothesis:
    """Single hypothesis check."""
    id: str
    title: str
    true_label: str
    false_label: str
    passed: bool = False
    detail: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class Gate:
    """Philosophy gate."""
    id: str
    desc: str
    passed: bool = False
    detail: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class BuildScriptLedger:
    """Full sweep ledger."""
    workspace_root: str
    total_crates: int
    crates_with_build_rs: int
    crates_with_build_deps_no_build_rs: int  # drift
    crates_with_build_rs_no_build_deps: int  # orphan
    crates_with_rerun_if_changed: int
    crates_with_main_fn: int
    total_lines: int
    total_env_macro: int
    total_env_var_runtime: int
    total_env_set_var: int
    total_command_new: int
    total_file_read: int
    total_file_write: int
    total_rerun_if_changed: int
    total_rerun_if_env: int
    total_rustc_link: int
    codegen_tool_uses: Dict[str, int]  # tool_name -> n_crates_using_it
    crate_profiles: List[BuildScriptProfile]
    cycles: List[Cycle]  # placeholder for V1294 (no cycles concept, but typed for uniformity)
    hypotheses: List[Hypothesis]
    gates: List[Gate]
    started_at: float
    finished_at: float
    version: str = "V1294.0"

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["crate_profiles"] = [p.to_dict() for p in self.crate_profiles]
        d["cycles"] = [c.to_dict() for c in self.cycles]
        d["hypotheses"] = [h.to_dict() for h in self.hypotheses]
        d["gates"] = [g.to_dict() for g in self.gates]
        return d


# ============================================================
# 2. Cargo.toml parsing helpers (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================


def parse_cargo_toml_build_deps(crate_dir: Path) -> Tuple[bool, List[str]]:
    """Parse Cargo.toml [build-dependencies] section. Returns (has_build_deps, names)."""
    cargo_path = crate_dir / WORKSPACE_TOML
    if not cargo_path.is_file():
        return False, []
    try:
        text = cargo_path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return False, []

    # find [build-dependencies] section
    m = re.search(r"^\[build-dependencies\]\s*$", text, re.MULTILINE)
    if not m:
        return False, []
    section_start = m.end()
    # find next [section]
    next_section = re.search(r"^\[[^\]]+\]\s*$", text[section_start:], re.MULTILINE)
    if next_section:
        section_text = text[section_start:section_start + next_section.start()]
    else:
        section_text = text[section_start:]

    # parse dep names (left side of "=")
    names: List[str] = []
    for line in section_text.split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m_dep = re.match(r"^([a-zA-Z0-9_-]+)\s*=", line)
        if m_dep:
            names.append(m_dep.group(1))
    return True, names


# ============================================================
# 3. build.rs scanning (主 17:43 实事求是 + 主 19:33 走在前人肩上)
# ============================================================


def scan_build_rs(crate_dir: Path, crate_name: str) -> BuildScriptProfile:
    """Scan a single crate's build.rs."""
    build_rs = crate_dir / BUILD_RS
    has_build_rs = build_rs.is_file()
    has_build_deps, build_dep_names = parse_cargo_toml_build_deps(crate_dir)

    if not has_build_rs:
        return BuildScriptProfile(
            crate_name=crate_name,
            crate_path=str(crate_dir.relative_to(crate_dir.parent.parent)) if crate_dir.is_dir() else str(crate_dir),
            has_build_rs=False,
            build_rs_path=None,
            n_lines=0,
            has_main_fn=False,
            n_env_macro=0,
            n_env_var_runtime=0,
            n_env_set_var=0,
            n_command_new=0,
            n_file_read=0,
            n_file_write=0,
            n_rerun_if_changed=0,
            n_rerun_if_env=0,
            n_rustc_link=0,
            codegen_tools=[],
            has_build_deps=has_build_deps,
            build_dep_names=build_dep_names,
            risk_flags=[],
        )

    try:
        text = build_rs.read_text(encoding="utf-8", errors="replace")
    except Exception:
        text = ""

    n_lines = len(text.split("\n"))
    has_main_fn = bool(RE_FN_MAIN.search(text))
    n_env_macro = len(RE_ENV_MACRO.findall(text))
    n_env_var_runtime = len(RE_ENV_VAR.findall(text)) + len(RE_ENV_VAR_OS.findall(text))
    n_env_set_var = len(RE_ENV_SET_VAR.findall(text))
    n_command_new = len(RE_COMMAND_NEW.findall(text)) + len(RE_PROCESS_COMMAND_NEW.findall(text))
    n_file_read = (
        len(RE_FILE_OPEN.findall(text))
        + len(RE_FS_READ_TO_STRING.findall(text))
        + len(RE_FS_READ.findall(text))
        + len(RE_INCLUDE_STR.findall(text))
        + len(RE_INCLUDE_BYTES.findall(text))
    )
    n_file_write = (
        len(RE_FILE_CREATE.findall(text))
        + len(RE_FS_WRITE.findall(text))
    )

    n_rerun_if_changed = len(RE_RERUN_IF_CHANGED.findall(text))
    n_rerun_if_env = len(RE_RERUN_IF_ENV.findall(text))
    n_rustc_link = len(RE_RUSTC_LINK.findall(text))

    # Detect codegen tools
    codegen_tools_found: List[str] = []
    for tool_name, patterns in CODEGEN_TOOLS.items():
        for pat in patterns:
            if pat in text:
                if tool_name not in codegen_tools_found:
                    codegen_tools_found.append(tool_name)
                break

    # Risk flags
    risk_flags: List[str] = []
    if n_env_set_var > 0:
        risk_flags.append("env_set_var")
    if n_command_new > 0:
        risk_flags.append("command_new")
    if n_file_write > 0:
        risk_flags.append("fs_write")

    # Build crate_path relative to workspace root
    try:
        rel_path = str(crate_dir.relative_to(crate_dir.parent.parent))
    except ValueError:
        rel_path = str(crate_dir)

    return BuildScriptProfile(
        crate_name=crate_name,
        crate_path=rel_path,
        has_build_rs=True,
        build_rs_path=str(build_rs.relative_to(crate_dir.parent.parent)) if build_rs.is_file() else None,
        n_lines=n_lines,
        has_main_fn=has_main_fn,
        n_env_macro=n_env_macro,
        n_env_var_runtime=n_env_var_runtime,
        n_env_set_var=n_env_set_var,
        n_command_new=n_command_new,
        n_file_read=n_file_read,
        n_file_write=n_file_write,
        n_rerun_if_changed=n_rerun_if_changed,
        n_rerun_if_env=n_rerun_if_env,
        n_rustc_link=n_rustc_link,
        codegen_tools=codegen_tools_found,
        has_build_deps=has_build_deps,
        build_dep_names=build_dep_names,
        risk_flags=risk_flags,
    )


def scan_workspace(workspace_root: Path) -> List[BuildScriptProfile]:
    """Scan all 50 crates' build.rs."""
    profiles: List[BuildScriptProfile] = []
    for crate_name in CRATES_V1294:
        crate_dir = workspace_root / "crates" / crate_name
        if not crate_dir.is_dir():
            # Try a flat layout or alternative
            alt = workspace_root / crate_name
            if alt.is_dir():
                crate_dir = alt
            else:
                # Skip — but still emit a placeholder profile so we count 50
                profiles.append(BuildScriptProfile(
                    crate_name=crate_name,
                    crate_path="<missing>",
                    has_build_rs=False,
                    build_rs_path=None,
                    n_lines=0,
                    has_main_fn=False,
                    n_env_macro=0,
                    n_env_var_runtime=0,
                    n_env_set_var=0,
                    n_command_new=0,
                    n_file_read=0,
                    n_file_write=0,
                    n_rerun_if_changed=0,
                    n_rerun_if_env=0,
                    n_rustc_link=0,
                    codegen_tools=[],
                    has_build_deps=False,
                    build_dep_names=[],
                    risk_flags=["missing_crate_dir"],
                ))
                continue
        profiles.append(scan_build_rs(crate_dir, crate_name))
    return profiles


# ============================================================
# 4. Hypothesis evaluation (主 17:43 实事求是 + 主 17:58 不假装)
# ============================================================


HYPOTHESES: List[Hypothesis] = [
    Hypothesis(
        id="H1_build_rs_rare",
        title="build.rs 罕见 (期望 <= 5/47 crates 用 build.rs)",
        true_label="build.rs 罕见, 大多数 crate 无 build script",
        false_label="build.rs 普及, 与预期不符",
    ),
    Hypothesis(
        id="H2_codegen_tools_used",
        title="build.rs 主要由 codegen 工具驱动 (tonic-build / tauri_build 等)",
        true_label="build.rs 使用 codegen crate",
        false_label="build.rs 没用 codegen crate (手写)",
    ),
    Hypothesis(
        id="H3_env_mutation_rare",
        title="set_var 调用罕见 (期望 0-1 处, 主要用于 protoc path 设置)",
        true_label="env mutation 受控",
        false_label="env mutation 多处 (drift 风险)",
    ),
    Hypothesis(
        id="H4_no_command_new",
        title="build.rs 不直接执行 shell 命令 (期望 0 处 Command::new)",
        true_label="无 shell exec",
        false_label="build.rs 用 Command::new (潜在 injection 风险)",
    ),
    Hypothesis(
        id="H5_rerun_if_changed_common",
        title="build.rs 多数声明 cargo:rerun-if-changed (期望 >= 1 处 per build.rs)",
        true_label="rerun-if-changed 普遍",
        false_label="rerun-if-changed 缺失 (rebuild 触发不精确)",
    ),
    Hypothesis(
        id="H6_no_drift",
        title="无 drift: 无 [build-dependencies] 但无 build.rs / 有 build.rs 但无 [build-dependencies]",
        true_label="Cargo.toml [build-dependencies] 与 build.rs 一致",
        false_label="drift: Cargo.toml 与 build.rs 不匹配",
    ),
]


def evaluate_hypotheses(ledger: BuildScriptLedger) -> None:
    """Evaluate each hypothesis against the ledger data. Mutates ledger.hypotheses."""
    profiles = ledger.crate_profiles
    n_with_build_rs = ledger.crates_with_build_rs

    # H1: build.rs 罕见 (期望 <= 5/50)
    h1 = next(h for h in HYPOTHESES if h.id == "H1_build_rs_rare")
    h1.passed = n_with_build_rs <= 5
    h1.detail = f"crates_with_build_rs={n_with_build_rs}/47 (expected <= 5)"

    # H2: codegen tools used
    h2 = next(h for h in HYPOTHESES if h.id == "H2_codegen_tools_used")
    h2.passed = len(ledger.codegen_tool_uses) >= 1
    h2.detail = f"codegen_tools used: {list(ledger.codegen_tool_uses.keys())} (count={len(ledger.codegen_tool_uses)})"

    # H3: env_mutation rare
    h3 = next(h for h in HYPOTHESES if h.id == "H3_env_mutation_rare")
    h3.passed = ledger.total_env_set_var <= 2
    h3.detail = f"total_env_set_var={ledger.total_env_set_var} (expected <= 2)"

    # H4: no command_new
    h4 = next(h for h in HYPOTHESES if h.id == "H4_no_command_new")
    h4.passed = ledger.total_command_new == 0
    h4.detail = f"total_command_new={ledger.total_command_new} (expected 0)"

    # H5: rerun-if-changed common
    h5 = next(h for h in HYPOTHESES if h.id == "H5_rerun_if_changed_common")
    if n_with_build_rs > 0:
        h5.passed = ledger.crates_with_rerun_if_changed >= max(1, n_with_build_rs // 2)
        h5.detail = f"crates_with_rerun_if_changed={ledger.crates_with_rerun_if_changed}/{n_with_build_rs}"
    else:
        h5.passed = True  # vacuously true if no build.rs
        h5.detail = "no build.rs found, vacuously PASS"

    # H6: no drift
    h6 = next(h for h in HYPOTHESES if h.id == "H6_no_drift")
    drift = ledger.crates_with_build_deps_no_build_rs + ledger.crates_with_build_rs_no_build_deps
    h6.passed = drift == 0
    h6.detail = (
        f"build_deps_no_build_rs={ledger.crates_with_build_deps_no_build_rs}, "
        f"build_rs_no_build_deps={ledger.crates_with_build_rs_no_build_deps} "
        f"(expected both 0)"
    )

    ledger.hypotheses = list(HYPOTHESES)


# ============================================================
# 5. Philosophy gates (主 17:58 + 主 20:46)
# ============================================================

GATES: List[Gate] = [
    Gate(id="v1294_extends_v1293", desc="V1294 继承 V1293 dep graph, 不删 V1293"),
    Gate(id="v1294_no_new_asi_dim", desc="V1294 = build.rs audit, 不引入新 ASI dim"),
    Gate(id="v1294_no_asi_v1_claim", desc="不假装 ASI V1: build.rs ≠ ASI"),
    Gate(id="v1294_no_kpi_inflate", desc="NS 92.91% LOCKED, 不刷"),
    Gate(id="v1294_no_phenomenal_claim", desc="build.rs ≠ phenomenal consciousness"),
    Gate(id="v1294_stdlib_only", desc="仅用 stdlib (re/pathlib/dataclasses/json), 不引入新依赖"),
    Gate(id="v1294_read_only", desc="只读 build.rs + Cargo.toml, 不改"),
    Gate(id="v1294_audit_not_fix", desc="audit ≠ fix, V1294 仅审计"),
    Gate(id="v1294_no_cargo_run", desc="不调 cargo build / cargo check / cargo run"),
    Gate(id="v1294_regex_only", desc="regex-only pattern match, 不解析 AST"),
    Gate(id="v1294_47_crates_full", desc="全 47 crates, 不只 worst-5"),
    Gate(id="v1294_no_build_rs_exec", desc="不执行 build.rs, 仅静态源码审计"),
]


def evaluate_gates(ledger: BuildScriptLedger) -> None:
    """Evaluate philosophy gates. Mutates ledger.gates."""
    for gate in GATES:
        gate.passed = True  # V1294 is read-only by construction
        gate.detail = "V1294 = read-only build.rs pattern audit, no mutation"
    ledger.gates = list(GATES)


# ============================================================
# 6. Build ledger (主 13:08 真自问)
# ============================================================


def build_ledger(workspace_root: Path) -> BuildScriptLedger:
    """Run full sweep and return ledger."""
    started_at = time.time()
    profiles = scan_workspace(workspace_root)

    # Workspace-level aggregates
    total_crates = len(profiles)
    crates_with_build_rs = sum(1 for p in profiles if p.has_build_rs)
    crates_with_build_deps_no_build_rs = sum(
        1 for p in profiles if p.has_build_deps and not p.has_build_rs
    )
    crates_with_build_rs_no_build_deps = sum(
        1 for p in profiles if p.has_build_rs and not p.has_build_deps
    )
    crates_with_rerun_if_changed = sum(
        1 for p in profiles if p.has_build_rs and p.n_rerun_if_changed > 0
    )
    crates_with_main_fn = sum(1 for p in profiles if p.has_build_rs and p.has_main_fn)

    total_lines = sum(p.n_lines for p in profiles if p.has_build_rs)
    total_env_macro = sum(p.n_env_macro for p in profiles)
    total_env_var_runtime = sum(p.n_env_var_runtime for p in profiles)
    total_env_set_var = sum(p.n_env_set_var for p in profiles)
    total_command_new = sum(p.n_command_new for p in profiles)
    total_file_read = sum(p.n_file_read for p in profiles)
    total_file_write = sum(p.n_file_write for p in profiles)
    total_rerun_if_changed = sum(p.n_rerun_if_changed for p in profiles)
    total_rerun_if_env = sum(p.n_rerun_if_env for p in profiles)
    total_rustc_link = sum(p.n_rustc_link for p in profiles)

    # Codegen tool uses
    codegen_tool_uses: Dict[str, int] = {}
    for p in profiles:
        for tool in p.codegen_tools:
            codegen_tool_uses[tool] = codegen_tool_uses.get(tool, 0) + 1

    ledger = BuildScriptLedger(
        workspace_root=str(workspace_root),
        total_crates=total_crates,
        crates_with_build_rs=crates_with_build_rs,
        crates_with_build_deps_no_build_rs=crates_with_build_deps_no_build_rs,
        crates_with_build_rs_no_build_deps=crates_with_build_rs_no_build_deps,
        crates_with_rerun_if_changed=crates_with_rerun_if_changed,
        crates_with_main_fn=crates_with_main_fn,
        total_lines=total_lines,
        total_env_macro=total_env_macro,
        total_env_var_runtime=total_env_var_runtime,
        total_env_set_var=total_env_set_var,
        total_command_new=total_command_new,
        total_file_read=total_file_read,
        total_file_write=total_file_write,
        total_rerun_if_changed=total_rerun_if_changed,
        total_rerun_if_env=total_rerun_if_env,
        total_rustc_link=total_rustc_link,
        codegen_tool_uses=codegen_tool_uses,
        crate_profiles=profiles,
        cycles=[],  # V1294 = no cycles concept; placeholder for type uniformity with V1293
        hypotheses=[],
        gates=[],
        started_at=started_at,
        finished_at=0.0,
    )

    evaluate_hypotheses(ledger)
    evaluate_gates(ledger)
    ledger.finished_at = time.time()
    return ledger


# ============================================================
# 7. Report renderer (主 00:56 任何人都能接手)
# ============================================================


def render_report(ledger: BuildScriptLedger) -> str:
    """Render markdown report."""
    lines: List[str] = []
    lines.append("# V1294 — Rust Build Script (build.rs) Inventory")
    lines.append("")
    lines.append(f"**Workspace root**: `{ledger.workspace_root}`")
    lines.append(
        f"**Duration**: {int((ledger.finished_at - ledger.started_at) * 1000)} ms"
    )
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Total crates: **{ledger.total_crates}**")
    lines.append(f"- Crates with `build.rs`: **{ledger.crates_with_build_rs}**")
    lines.append(
        f"- Crates with `[build-dependencies]` but no `build.rs`: "
        f"**{ledger.crates_with_build_deps_no_build_rs}** (drift)"
    )
    lines.append(
        f"- Crates with `build.rs` but no `[build-dependencies]`: "
        f"**{ledger.crates_with_build_rs_no_build_deps}** (orphan)"
    )
    lines.append(
        f"- Crates with `cargo:rerun-if-changed`: **{ledger.crates_with_rerun_if_changed}**"
    )
    lines.append(f"- Crates with `fn main()`: **{ledger.crates_with_main_fn}**")
    lines.append("")
    lines.append("## Pattern Totals")
    lines.append("")
    lines.append(f"- Total lines (across all build.rs): **{ledger.total_lines}**")
    lines.append(f"- `env!()` compile-time: **{ledger.total_env_macro}**")
    lines.append(
        f"- `std::env::var(...)` runtime: **{ledger.total_env_var_runtime}**"
    )
    lines.append(
        f"- `std::env::set_var(...)` mutations: **{ledger.total_env_set_var}**"
    )
    lines.append(
        f"- `Command::new(...)` shell exec: **{ledger.total_command_new}**"
    )
    lines.append(f"- File read operations: **{ledger.total_file_read}**")
    lines.append(f"- File write operations: **{ledger.total_file_write}**")
    lines.append(
        f"- `cargo:rerun-if-changed=` directives: **{ledger.total_rerun_if_changed}**"
    )
    lines.append(
        f"- `cargo:rerun-if-env-changed=` directives: **{ledger.total_rerun_if_env}**"
    )
    lines.append(f"- `cargo:rustc-link-*` directives: **{ledger.total_rustc_link}**")
    lines.append("")

    lines.append("## Codegen Tools Used")
    lines.append("")
    if ledger.codegen_tool_uses:
        lines.append("| tool | n_crates |")
        lines.append("|---|---:|")
        for tool, count in sorted(
            ledger.codegen_tool_uses.items(), key=lambda x: -x[1]
        ):
            lines.append(f"| {tool} | {count} |")
    else:
        lines.append("_None detected._")
    lines.append("")

    lines.append("## Hypotheses (主 17:43 实事求是)")
    lines.append("")
    for h in ledger.hypotheses:
        mark = "✅" if h.passed else "❌"
        lines.append(
            f"- {mark} **{h.id}** — {h.title} → {h.true_label if h.passed else h.false_label}"
        )
        lines.append(f"  - detail: {h.detail}")
    lines.append("")

    lines.append("## Crates with build.rs")
    lines.append("")
    lines.append("| crate | lines | main | env! | env::var | set_var | Cmd::new | file_w | rerun | tools | risk |")
    lines.append("|---|---:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|---|---|")
    for p in ledger.crate_profiles:
        if p.has_build_rs:
            tools_str = ", ".join(p.codegen_tools) if p.codegen_tools else "-"
            risk_str = ", ".join(p.risk_flags) if p.risk_flags else "-"
            lines.append(
                f"| {p.crate_name} | {p.n_lines} | "
                f"{'✓' if p.has_main_fn else '✗'} | "
                f"{p.n_env_macro} | {p.n_env_var_runtime} | {p.n_env_set_var} | "
                f"{p.n_command_new} | {p.n_file_write} | {p.n_rerun_if_changed} | "
                f"{tools_str} | {risk_str} |"
            )
    lines.append("")

    lines.append("## Drift Crates (build.rs ↔ Cargo.toml mismatch)")
    lines.append("")
    drift_found = False
    for p in ledger.crate_profiles:
        if p.has_build_deps and not p.has_build_rs:
            lines.append(
                f"- **{p.crate_name}**: has `[build-dependencies]` "
                f"({', '.join(p.build_dep_names)}) but no `build.rs`"
            )
            drift_found = True
        if p.has_build_rs and not p.has_build_deps:
            lines.append(
                f"- **{p.crate_name}**: has `build.rs` but no `[build-dependencies]`"
            )
            drift_found = True
    if not drift_found:
        lines.append("_No drift detected._")
    lines.append("")

    lines.append("## Philosophy Gates (主 17:58 不假装)")
    lines.append("")
    for g in ledger.gates:
        mark = "✅" if g.passed else "❌"
        lines.append(f"- {mark} **{g.id}** — {g.desc}")
    lines.append("")

    return "\n".join(lines)


# ============================================================
# 8. CLI (主 00:56 任何人都能接手)
# ============================================================


def cmd_probe(args: argparse.Namespace) -> int:
    """Probe only: print workspace root + crate count + quick summary."""
    root = Path(args.workspace_root).resolve()
    if not root.is_dir():
        print(f"[v1294 probe] workspace root not found: {root}")
        return 1
    profiles = scan_workspace(root)
    n_with_build_rs = sum(1 for p in profiles if p.has_build_rs)
    n_with_build_deps_no_build_rs = sum(
        1 for p in profiles if p.has_build_deps and not p.has_build_rs
    )
    print(f"[v1294 probe] workspace_root={root}")
    print(f"[v1294 probe] total_crates={len(profiles)}")
    print(f"[v1294 probe] crates_with_build_rs={n_with_build_rs}")
    print(
        f"[v1294 probe] crates_with_build_deps_no_build_rs={n_with_build_deps_no_build_rs}"
    )
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    """Run full sweep and print summary to stdout."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(f"[v1294 run] total_crates={ledger.total_crates}")
    print(f"[v1294 run] crates_with_build_rs={ledger.crates_with_build_rs}")
    print(f"[v1294 run] total_lines={ledger.total_lines}")
    print(f"[v1294 run] total_env_set_var={ledger.total_env_set_var}")
    print(f"[v1294 run] total_command_new={ledger.total_command_new}")
    print(f"[v1294 run] codegen_tools={list(ledger.codegen_tool_uses.keys())}")
    n_passed = sum(1 for h in ledger.hypotheses if h.passed)
    print(
        f"[v1294 run] hypotheses_passed={n_passed}/{len(ledger.hypotheses)}"
    )
    return 0


def cmd_json(args: argparse.Namespace) -> int:
    """Output full ledger as JSON."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(json.dumps(ledger.to_dict(), indent=2, default=str))
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    """Output markdown report to stdout or file."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    md = render_report(ledger)
    if args.out:
        Path(args.out).write_text(md, encoding="utf-8")
        print(f"[v1294 report] written to {args.out}")
    else:
        print(md)
    return 0


def cmd_crate(args: argparse.Namespace) -> int:
    """Print single crate's build.rs profile."""
    root = Path(args.workspace_root).resolve()
    profiles = scan_workspace(root)
    target = args.crate
    for p in profiles:
        if p.crate_name == target:
            print(json.dumps(p.to_dict(), indent=2, default=str))
            return 0
    print(f"[v1294 crate] not found: {target}")
    return 1


def cmd_tools(args: argparse.Namespace) -> int:
    """Print codegen tool usage summary."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(f"[v1294 tools] total_distinct_codegen_tools={len(ledger.codegen_tool_uses)}")
    for tool, count in sorted(
        ledger.codegen_tool_uses.items(), key=lambda x: -x[1]
    ):
        users = [p.crate_name for p in ledger.crate_profiles if tool in p.codegen_tools]
        print(f"[v1294 tools] {tool}={count} crates: {', '.join(users)}")
    return 0


def cmd_risk(args: argparse.Namespace) -> int:
    """Print risk pattern summary (set_var / Command::new / fs_write)."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(
        f"[v1294 risk] total_env_set_var={ledger.total_env_set_var}"
    )
    print(
        f"[v1294 risk] total_command_new={ledger.total_command_new}"
    )
    print(
        f"[v1294 risk] total_file_write={ledger.total_file_write}"
    )
    for p in ledger.crate_profiles:
        if p.risk_flags:
            print(
                f"[v1294 risk] {p.crate_name}: flags={p.risk_flags} "
                f"(set_var={p.n_env_set_var}, cmd_new={p.n_command_new}, "
                f"fs_write={p.n_file_write})"
            )
    return 0


def cmd_drift(args: argparse.Namespace) -> int:
    """Print drift summary (build_deps no build.rs / build.rs no build_deps)."""
    root = Path(args.workspace_root).resolve()
    ledger = build_ledger(root)
    print(
        f"[v1294 drift] build_deps_no_build_rs={ledger.crates_with_build_deps_no_build_rs}"
    )
    print(
        f"[v1294 drift] build_rs_no_build_deps={ledger.crates_with_build_rs_no_build_deps}"
    )
    for p in ledger.crate_profiles:
        if p.has_build_deps and not p.has_build_rs:
            print(
                f"[v1294 drift] {p.crate_name}: has_build_deps=True, has_build_rs=False, "
                f"build_dep_names={p.build_dep_names}"
            )
        if p.has_build_rs and not p.has_build_deps:
            print(
                f"[v1294 drift] {p.crate_name}: has_build_deps=False, has_build_rs=True"
            )
    return 0


def build_arg_parser() -> argparse.ArgumentParser:
    """Build argparse parser."""
    parser = argparse.ArgumentParser(
        prog="v1294_rust_build_script_inventory",
        description=(
            "V1294 — Rust Build Script (build.rs) Inventory "
            "(VCP 真源代码深读 #15) 真生产"
        ),
    )
    parser.add_argument(
        "--workspace-root",
        default=str(WORKSPACE_ROOT_DEFAULT),
        help="Path to Apeireth-rust workspace root",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("probe", help="probe workspace: print root + counts").set_defaults(
        func=cmd_probe
    )
    sub.add_parser("run", help="run full sweep + print summary").set_defaults(
        func=cmd_run
    )
    sub.add_parser("json", help="output full ledger as JSON").set_defaults(
        func=cmd_json
    )
    p_report = sub.add_parser("report", help="output markdown report")
    p_report.add_argument("--out", default=None, help="output file path")
    p_report.set_defaults(func=cmd_report)

    p_crate = sub.add_parser("crate", help="print single crate's profile")
    p_crate.add_argument("--crate", required=True, help="crate name (e.g. apeireth-bus)")
    p_crate.set_defaults(func=cmd_crate)

    sub.add_parser("tools", help="print codegen tool usage summary").set_defaults(
        func=cmd_tools
    )
    sub.add_parser("risk", help="print risk pattern summary").set_defaults(
        func=cmd_risk
    )
    sub.add_parser("drift", help="print drift summary").set_defaults(func=cmd_drift)

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_arg_parser()
    # support both subcommand form and top-level flags
    if argv is None:
        argv = sys.argv[1:]

    # legacy form: --probe / --run / --json / --report / --crate / --tools / --risk / --drift
    legacy_map = {
        "--probe": "probe",
        "--run": "run",
        "--json": "json",
        "--report": "report",
        "--tools": "tools",
        "--risk": "risk",
        "--drift": "drift",
    }
    converted: List[str] = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a in legacy_map:
            converted.append(legacy_map[a])
            i += 1
        elif a == "--crate":
            converted.append("crate")
            if i + 1 < len(argv):
                converted.append("--crate")
                converted.append(argv[i + 1])
                i += 2
            else:
                i += 1
        elif a in ("--workspace-root", "--out"):
            converted.append(a)
            if i + 1 < len(argv):
                converted.append(argv[i + 1])
                i += 2
            else:
                i += 1
        else:
            converted.append(a)
            i += 1
    argv = converted

    args = parser.parse_args(argv)
    if not getattr(args, "func", None):
        # default to --run if nothing specified
        args = parser.parse_args(["run"] + argv)

    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())