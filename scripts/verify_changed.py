#!/usr/bin/env python3
"""变更影响面验证（fast feedback gate）。

用途：AI 高速迭代下，每批不跑全量，只跑"改动 crate + 反向依赖"的 check/clippy/test；
全量留给 wave 收口/夜间。输出落盘再判绿（防管道截断假绿）。

用法:
  python scripts/verify_changed.py --dry-run                          # 看工作树改动会影响哪些 crate
  python scripts/verify_changed.py                                    # 跑 fast gate（check+clippy+test）
  python scripts/verify_changed.py --no-clippy --no-test              # 只 check
  python scripts/verify_changed.py --base origin/main                 # 按 base..HEAD 的 diff
  python scripts/verify_changed.py --files crates/engine/core/src/lib.rs  # 显式指定（测试/手动）
  python scripts/verify_changed.py --log logs/fast-gate.log           # 输出落盘（默认 logs/）

口径:
- 改动 .rs 映射到最近 Cargo.toml 所属 crate；Cargo.toml/Lock 改动 = 全 workspace。
- 受影响 = 改动 crate + 传递闭包（反向依赖）——依赖方的测试可能暴露接口破坏。
- test 优先 `cargo nextest run`（per-test 超时/并行/机器可读），无 nextest 回退 `cargo test`。
- 退出码 = 任一阶段失败即 1；日志全量保留在 --log 指向文件。
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LOG = ROOT / "logs" / "verify-changed.log"


def _run(cmd: list[str], log_path: Path, append: bool = True) -> int:
    mode = "a" if append else "w"
    with open(log_path, mode, encoding="utf-8", errors="replace") as fh:
        fh.write("\n$ " + " ".join(cmd) + "\n")
        fh.flush()
        proc = subprocess.run(cmd, cwd=str(ROOT), stdout=fh, stderr=subprocess.STDOUT, text=True)
    return proc.returncode


def _run_capture(cmd: list[str]) -> tuple[int, str]:
    proc = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True, encoding="utf-8", errors="replace")
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def changed_files(base: str | None, files: str | None) -> list[str]:
    if files:
        return [item.strip().replace("\\", "/") for item in files.split(",") if item.strip()]
    if base:
        code, out = _run_capture(["git", "diff", "--name-only", f"{base}...HEAD"])
    else:
        code, out = _run_capture(["git", "status", "--porcelain"])
        if code != 0:
            raise SystemExit(f"git status failed: {out}")
        parsed: list[str] = []
        for line in out.splitlines():
            if not line.strip():
                continue
            path = line[3:].strip()
            if " -> " in path:  # rename: 取新路径
                path = path.split(" -> ", 1)[1]
            path = path.strip().strip('"')
            parsed.append(path.replace("\\", "/"))
        return parsed
    if code != 0:
        raise SystemExit(f"git diff failed: {out}")
    return [line.strip() for line in out.splitlines() if line.strip()]


def load_metadata() -> dict:
    code, out = _run_capture(["cargo", "metadata", "--format-version", "1"])
    if code != 0:
        raise SystemExit(f"cargo metadata failed: {out[:2000]}")
    return json.loads(out)


def build_index(meta: dict):
    by_manifest: dict[str, dict] = {}
    roots: list[str] = []
    for pkg in meta["packages"]:
        manifest = Path(pkg["manifest_path"]).resolve()
        by_manifest[str(manifest).lower()] = pkg
        if manifest.parent == ROOT:
            roots.append(pkg["id"])
    return by_manifest, roots


def crate_for_path(path: str, by_manifest: dict[str, dict]) -> dict | None:
    current = (ROOT / path).resolve()
    for parent in [current, *current.parents]:
        manifest = parent / "Cargo.toml"
        pkg = by_manifest.get(str(manifest).lower())
        if pkg:
            return pkg
    return None


def affected_packages(seed_ids: list[str], meta: dict, all_workspace: bool) -> list[str]:
    packages = meta["packages"]
    nodes = meta.get("resolve", {}).get("nodes", [])
    if all_workspace or not seed_ids:
        return sorted({p["name"] for p in packages})
    reverse: dict[str, set[str]] = {p["id"]: set() for p in packages}
    for node in nodes:
        for dep in node.get("dependencies", []):
            reverse.setdefault(dep, set()).add(node["id"])
    id_to_name = {p["id"]: p["name"] for p in packages}
    seen: set[str] = set()
    stack = list(seed_ids)
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        stack.extend(reverse.get(current, set()))
    return sorted(id_to_name[i] for i in seen if i in id_to_name)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="change-scoped verification (fast feedback gate)")
    parser.add_argument("--base", default=None, help="diff base（默认扫描工作树 git status）")
    parser.add_argument("--files", default=None, help="逗号分隔的改动文件（覆盖 git 检测；测试/手动用）")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--no-clippy", action="store_true")
    parser.add_argument("--no-test", action="store_true")
    parser.add_argument("--no-check", action="store_true")
    parser.add_argument("--log", default=str(DEFAULT_LOG))
    args = parser.parse_args(argv)

    log_path = Path(args.log)
    if not log_path.is_absolute():
        log_path = ROOT / log_path
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(f"verify_changed run: {time.strftime('%Y-%m-%d %H:%M:%S')}\n", encoding="utf-8")

    files = changed_files(args.base, args.files)
    rust_files = [f for f in files if f.endswith((".rs", ".toml"))]
    if not rust_files:
        print(f"no Rust/Toml changes ({len(files)} files changed) — docs-only; run doc gates instead.")
        return 0

    meta = load_metadata()
    by_manifest, _roots = build_index(meta)
    seed_ids: list[str] = []
    all_workspace = False
    for path in rust_files:
        if path in ("Cargo.toml", "Cargo.lock"):
            all_workspace = True
            continue
        pkg = crate_for_path(path, by_manifest)
        if pkg:
            seed_ids.append(pkg["id"])

    pkgs = affected_packages(sorted(set(seed_ids)), meta, all_workspace)
    print(f"changed rust files: {len(rust_files)}; affected crates: {len(pkgs)}")
    for pkg in pkgs:
        print(f"  - {pkg}")
    if args.dry_run:
        print("dry-run: no command executed.")
        return 0

    pkg_args = [arg for pkg in pkgs for arg in ("-p", pkg)]
    stages: list[tuple[str, list[str]]] = []
    if not args.no_check:
        stages.append(("check", ["cargo", "check", "--locked", "--all-targets", *pkg_args]))
    if not args.no_clippy:
        stages.append(("clippy", ["cargo", "clippy", "--locked", "--all-targets", *pkg_args, "--", "-D", "warnings"]))
    if not args.no_test:
        code, _ = _run_capture(["cargo", "nextest", "--version"])
        if code == 0:
            stages.append(("test(nextest)", ["cargo", "nextest", "run", "--locked", *pkg_args]))
        else:
            stages.append(("test(cargo)", ["cargo", "test", "--locked", "--all-targets", *pkg_args]))

    failed: list[str] = []
    for name, cmd in stages:
        start = time.time()
        print(f"== {name} ... ", end="", flush=True)
        code = _run(cmd, log_path)
        elapsed = time.time() - start
        status = "OK" if code == 0 else f"FAIL(exit={code})"
        print(f"{status}  {elapsed:.1f}s")
        if code != 0:
            failed.append(name)
            break  # 前一段失败后不再继续（避免用后续失败掩盖首因）

    print()
    print(f"log: {log_path}")
    if failed:
        print(f"FAST GATE FAILED: {', '.join(failed)} — see log for details")
        return 1
    print("FAST GATE PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
