#!/usr/bin/env python3
"""解析**真实生效的 cargo 产物目录**（target dir）—— 消除脚本里硬编码的 `<repo>\\target`。

# 为什么需要它（真实的「守卫失效」风险）

本仓多个 Windows 侧脚本把 cargo 产物目录**硬编码**成 `<repo>\\target`
⇒ ⇒ 但 cargo 的实际输出目录**可以**被改：
  * 环境变量 `CARGO_TARGET_DIR`（⇒ 例如指向 `C:\\Apeireth\\cargo-target`）；
  * cargo 配置里的 `build.target-dir`；
⇒ ⇒ 目录一改 ⇒ ⇒ 这些脚本**静默失效**（找不到文件却不报错 ⇒ 门禁自称绿）
⇒ ⇒ ⇒ 属「**守卫失效**」风险（比红更坏：它骗人）。

# 唯一的解析口径（其它脚本**只调它**，不复制粘贴）

  ① 环境变量 `CARGO_TARGET_DIR` 已设置 ⇒ 直接用它；
  ② 否则跑 `cargo metadata --no-deps --format-version 1 --offline` ⇒ 取 `target_directory`
     （它**自动尊重**环境变量与 cargo 配置 ⇒ 比硬编码可靠）；
  ③ 解析失败 ⇒ 回退 `<repoRoot>\\target`，并**在输出里明确提示用了回退**（不许静默）。

# 命令行入口（给 PowerShell 脚本吃）

    $t = (& python "$PSScriptRoot\\target_dir.py")

⇒ ⇒ **stdout 只有一行**（解析出来的路径）⇒ ⇒ PowerShell 可直接吃；
⇒ ⇒ 诊断信息（尤其「用了回退」的提示）走 **stderr** ⇒ ⇒ 不污染那一行。

# 相对路径的约定

`CARGO_TARGET_DIR` 若是**相对**路径 ⇒ ⇒ 本模块按 **repo 根**解析（不是按调用者 CWD），
⇒ ⇒ 因为 ② 级里 `cargo metadata` 也是以 repo 根为 cwd 跑的 ⇒ ⇒ 两级口径一致、可复核。

# 纪律

* **只用标准库**（零第三方依赖）；
* Windows 控制台安全（stdout/stderr 强制 UTF-8 + `errors="replace"`，
  同 `scripts/clean_test_binaries.py` 的既有做法）；
* `cargo metadata` 调用**带 `--offline`**、**带超时**（120 秒）、**捕获全部失败**。
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

__all__ = [
    "CARGO_METADATA_TIMEOUT_SEC",
    "SOURCE_ENV",
    "SOURCE_METADATA",
    "SOURCE_FALLBACK",
    "resolve_target_dir",
    "resolve_target_dir_with_source",
    "resolve_target_dir_cli",
]

# ⚠️ **Windows 控制台不是 UTF-8** ⇒ ⇒ `print` 里的中文 / `⇒` 会抛
#   `UnicodeEncodeError` ⇒ ⇒ 门禁 exit 1 ⇒ ⇒ **把本来通过的检查判成红**
#   ⇒ ⇒ 这里把 stdout/stderr 固定成 UTF-8（写不出就替换，不抛）。
#   （与 `scripts/clean_test_binaries.py` 的既有做法一致。）
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # pragma: no cover - 老解释器 / 非标准流
        pass

# `cargo metadata` 的超时（秒）—— 防止 cargo 卡住把调用方脚本一起拖死。
CARGO_METADATA_TIMEOUT_SEC = 120

# 解析来源标记（供调用方判断「是不是回退」⇒ ⇒ 回退必须提示，不许静默）。
SOURCE_ENV = "CARGO_TARGET_DIR"
SOURCE_METADATA = "cargo metadata"
SOURCE_FALLBACK = "fallback"

# 环境变量名（cargo 自己的口径）。
TARGET_DIR_ENV_VAR = "CARGO_TARGET_DIR"


def _default_repo_root() -> Path:
    """本仓根目录（本文件在 `<repo>\\scripts\\` 下 ⇒ ⇒ 上一级就是 repo 根）。"""
    return Path(__file__).resolve().parent.parent


def _anchor(raw: str, repo_root: Path) -> Path:
    """相对路径按 **repo 根** 解析（绝对路径原样返回）。"""
    path = Path(raw).expanduser()
    if path.is_absolute():
        return path
    return repo_root / path


def _env_target_dir(repo_root: Path) -> Path | None:
    """① 环境变量 `CARGO_TARGET_DIR`（已设置 ⇒ 直接用它）。"""
    raw = os.environ.get(TARGET_DIR_ENV_VAR, "").strip()
    if not raw:
        return None
    return _anchor(raw, repo_root)


def _metadata_target_dir(repo_root: Path) -> Path | None:
    """② `cargo metadata` 的 `target_directory`（失败 ⇒ 返回 None ⇒ 走回退）。"""
    try:
        proc = subprocess.run(
            [
                "cargo",
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--offline",
            ],
            cwd=str(repo_root),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=CARGO_METADATA_TIMEOUT_SEC,
        )
    except subprocess.TimeoutExpired:
        print(
            f"[target-dir] cargo metadata 超时（>{CARGO_METADATA_TIMEOUT_SEC}s）",
            file=sys.stderr,
        )
        return None
    except (OSError, subprocess.SubprocessError) as error:
        print(f"[target-dir] cargo metadata 调不动：{error}", file=sys.stderr)
        return None

    if proc.returncode != 0:
        tail = (proc.stderr or "").strip().splitlines()
        detail = tail[-1] if tail else "(无 stderr)"
        print(
            f"[target-dir] cargo metadata 退出码 {proc.returncode}：{detail}",
            file=sys.stderr,
        )
        return None

    try:
        payload = json.loads(proc.stdout)
    except (TypeError, ValueError) as error:
        print(f"[target-dir] cargo metadata 输出不是合法 JSON：{error}", file=sys.stderr)
        return None

    raw = payload.get("target_directory") if isinstance(payload, dict) else None
    if not raw or not isinstance(raw, str):
        print("[target-dir] cargo metadata 没有 target_directory 字段", file=sys.stderr)
        return None
    return Path(raw)


def resolve_target_dir_with_source(
    repo_root: Path | None = None,
) -> tuple[Path, str]:
    """解析真实生效的产物目录 ⇒ ⇒ 返回 `(目录, 来源)`。

    来源取值：`SOURCE_ENV` / `SOURCE_METADATA` / `SOURCE_FALLBACK`
    ⇒ ⇒ 调用方可据此**明确提示**「用了回退」（不许静默）。
    """
    root = Path(repo_root) if repo_root is not None else _default_repo_root()

    env_dir = _env_target_dir(root)
    if env_dir is not None:
        return env_dir, SOURCE_ENV

    metadata_dir = _metadata_target_dir(root)
    if metadata_dir is not None:
        return metadata_dir, SOURCE_METADATA

    fallback = root / "target"
    print(
        f"[target-dir] ⚠️ 解析失败 ⇒ 回退到 {fallback}（<repo-root>\\target）",
        file=sys.stderr,
    )
    return fallback, SOURCE_FALLBACK


def resolve_target_dir(repo_root: Path | None = None) -> Path:
    """解析真实生效的 cargo 产物目录（① env ⇒ ② cargo metadata ⇒ ③ 回退 `<repo>\\target`）。"""
    target_dir, _source = resolve_target_dir_with_source(repo_root)
    return target_dir


def resolve_target_dir_cli(argv: list[str] | None = None) -> int:
    """命令行入口：**stdout 只打印一行**（解析出来的产物目录路径）。

    用法（PowerShell）::

        $t = (& python "$PSScriptRoot\\target_dir.py")

    可选参数：`--repo-root PATH`（解析**别的**项目的产物目录，例如
    `frontend/companion-desktop/src-tauri` 这个独立 cargo 项目）。
    """
    args = list(sys.argv[1:] if argv is None else argv)
    repo_root: Path | None = None
    if args:
        if args[0] == "--repo-root" and len(args) >= 2:
            repo_root = Path(args[1])
        else:
            print(
                "usage: target_dir.py [--repo-root PATH]",
                file=sys.stderr,
            )
            return 2
    print(resolve_target_dir(repo_root))
    return 0


if __name__ == "__main__":
    sys.exit(resolve_target_dir_cli())
