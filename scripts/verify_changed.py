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
  python scripts/verify_changed.py --arch-strict                      # 架构守卫违规即失败（默认仅报告）

口径:
- 改动 .rs 映射到最近 Cargo.toml 所属 crate；Cargo.toml/Lock 改动 = 全 workspace。
- 受影响 = 改动 crate + 传递闭包（反向依赖）——依赖方的测试可能暴露接口破坏。
- test 优先 `cargo nextest run`（per-test 超时/并行/机器可读），无 nextest 回退 `cargo test`。
- arch_guard（架构契约守卫）先跑：默认 report-only，`--arch-strict` 升格拦截；详见 docs/architecture/。
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

# Windows 控制台默认 GBK ⇒ 破折号/箭头类字符会抛 UnicodeEncodeError ⇒ 强制 UTF-8
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # noqa: BLE001 —— 老解释器没有 reconfigure 时不阻断
        pass


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
    for pkg in meta["packages"]:
        manifest = Path(pkg["manifest_path"]).resolve()
        by_manifest[str(manifest).lower()] = pkg
    # 只认「根 workspace 的成员」：cargo metadata 会把三方依赖也放进 packages，
    # 把它们当 crate 传给 `cargo -p` 会在多版本依赖名上直接报
    # `specification 'xxx' is ambiguous` ⇒ 门禁假红（见缺陷账本 E-122）。
    workspace_members = set(meta.get("workspace_members", []))
    return by_manifest, workspace_members


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
    # 只取根 workspace 的成员（cargo metadata 会把三方依赖也放进 packages）。
    members = set(meta.get("workspace_members", []))
    if all_workspace or not seed_ids:
        return sorted({p["name"] for p in packages if p["id"] in members})
    reverse: dict[str, set[str]] = {}
    for node in nodes:
        for dep in node.get("dependencies", []):
            # cargo metadata 的 resolve.nodes[].dependencies 是**包 id 字符串**。
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
    return sorted(id_to_name[i] for i in seen if i in id_to_name and i in members)


def classify_files(files: list[str], by_manifest: dict[str, dict]) -> dict:
    """把改动文件分成三类（口径守卫见 scripts/check_fast_gate_classifier.py）。

    - 代码文件（`.rs`）⇒ 按最近的祖先 crate manifest 归属；
    - 清单文件（`Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` / `.cargo/config.toml`）
      ⇒ 根 manifest 与构建级输入按**全 workspace** 计；
    - 其余（`.md` / `.py` / `.ps1` / **非清单** `.toml` 等）⇒ **不是 Rust 变更**。

    ⚠️ 旧口径把所有 `.toml`（含 `docs/architecture/contract.toml`）当代码文件
    ⇒ 映射到根 manifest ⇒ 影响面按全 workspace 计 ⇒ 把**三方依赖名**一起塞进
    `cargo -p` ⇒ 撞上多版本依赖即报 `specification 'bit-set' is ambiguous` ⇒ 门禁假红
    （缺陷账本 docs/04-internal/ui-engine-defect-ledger.md E-122）。
    """
    code_files: list[str] = []
    manifest_files: list[str] = []
    foreign_files: list[str] = []  # 不属于根 workspace 的 .rs（独立 cargo workspace）
    seed_ids: list[str] = []
    all_workspace = False
    for path in files:
        name = path.rsplit("/", 1)[-1]
        if name == "Cargo.lock" or name == "rust-toolchain.toml" or path.endswith(".cargo/config.toml"):
            all_workspace = True
            manifest_files.append(path)
        elif name == "Cargo.toml":
            manifest_files.append(path)
            if "/" not in path:  # 根 manifest
                all_workspace = True
            else:
                pkg = crate_for_path(path, by_manifest)
                if pkg:
                    seed_ids.append(pkg["id"])
        elif path.endswith(".rs"):
            code_files.append(path)
            pkg = crate_for_path(path, by_manifest)
            if pkg:
                seed_ids.append(pkg["id"])
            else:
                foreign_files.append(path)
    return {
        "code_files": code_files,
        "manifest_files": manifest_files,
        "foreign_files": foreign_files,
        "seed_ids": seed_ids,
        "all_workspace": all_workspace,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="change-scoped verification (fast feedback gate)")
    parser.add_argument("--base", default=None, help="diff base（默认扫描工作树 git status）")
    parser.add_argument("--files", default=None, help="逗号分隔的改动文件（覆盖 git 检测；测试/手动用）")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--no-clippy", action="store_true")
    parser.add_argument("--no-test", action="store_true")
    parser.add_argument("--no-check", action="store_true")
    parser.add_argument("--no-arch", action="store_true", help="跳过架构守卫（arch_guard）")
    parser.add_argument("--arch-strict", action="store_true", help="arch_guard 违规即失败（默认仅报告）")
    parser.add_argument("--log", default=str(DEFAULT_LOG))
    args = parser.parse_args(argv)

    log_path = Path(args.log)
    if not log_path.is_absolute():
        log_path = ROOT / log_path
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(f"verify_changed run: {time.strftime('%Y-%m-%d %H:%M:%S')}\n", encoding="utf-8")

    files = changed_files(args.base, args.files)

    # 架构守卫（arch_guard）：默认 report-only；--arch-strict 升格拦截。
    # 放在 Rust 过滤之前——契约/文档批同样需要架构检查（contract.toml 是代码之外的架构真源）。
    if not args.no_arch:
        print("== arch_guard ... ", end="", flush=True)
        arch_mode = "--strict" if args.arch_strict else "--report"
        code = _run([sys.executable, "scripts/arch_guard.py", arch_mode], log_path)
        if code == 0:
            print("OK")
        else:
            print(f"WARN(exit={code})")
            if args.arch_strict:
                print("FAST GATE FAILED: arch_guard（详见日志）")
                return 1

    meta = load_metadata()
    by_manifest, _members = build_index(meta)
    classified = classify_files(files, by_manifest)
    seed_ids = classified["seed_ids"]
    all_workspace = classified["all_workspace"]
    code_files = classified["code_files"]
    manifest_files = classified["manifest_files"]
    foreign_files = classified["foreign_files"]

    if not code_files and not manifest_files:
        print(f"no Rust/manifest changes ({len(files)} files changed) — docs-only; run doc gates instead.")
        return 0

    pkgs = affected_packages(sorted(set(seed_ids)), meta, all_workspace)
    print(f"changed rust files: {len(code_files)}; manifest files: {len(manifest_files)}; affected crates: {len(pkgs)}")
    for pkg in pkgs:
        print(f"  - {pkg}")
    if foreign_files:
        # ⚠️ 独立 cargo workspace 的改动不在根 workspace 的影响面里 ⇒ 本仓 Rust 阶段不覆盖。
        # 「宁可明确说没验」⇒ 明写出来，不静默跳过。
        print("  ! 不属于根 workspace 的改动（独立 cargo workspace，本仓 Rust 阶段不覆盖，请另跑对应 workspace 的门禁）:")
        for path in foreign_files:
            print(f"    - {path}")
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
