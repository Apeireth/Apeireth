#!/usr/bin/env python3
"""**构建产物槽位（target slot）真源解析 + 游离目录审计**。

# 为什么要有它（实测的漂移）
本机 `C:\\Apeireth` 下同时存在**多个** cargo 产物根（`cargo-target` · `cargo-target-ui-desk` ·
`cargo-target-c4-ui` · `cargo-target-c13-model` · `cargo-target-check` …）以及**工作树级** `target/`
⇒ ⇒ 谁在哪个目录里长产物**没人说得清** ⇒ ⇒ 磁盘涨到看不清 ⇒ ⇒ 且仓内多个守卫脚本
（`check_stale_binaries.py` · `target-hygiene.ps1` · `target-sweep.ps1` · `measure-session-startup.ps1` …
）把路径**硬编码**成 `<repo>\\target` ⇒ ⇒ 产物目录一改 ⇒ ⇒ **守卫静默失效**。

⇒ ⇒ 本脚本给三件事各一个**机器可判定**的答案：
1. **真源**：当前真正生效的产物目录是哪个（`cargo metadata` ⇒ 尊重 `CARGO_TARGET_DIR` 与 cargo 配置）；
2. **审计**：`--audit` ⇒ 哪些目录**登记过**、哪些是**游离**（未登记 ⇒ 新漂移 ⇒ 红）；
3. **回收**：可再生产物的回收口径（只清测试二进制 ⇒ 保留 `*.so`）。

# 登记表（唯一真源）
`docs/architecture/target-slots.json` —— 目录、归属、用途、阈值、既有游离目录（legacy）与
证据保留目录（reserved）都在那里；**本脚本不改它，只读它**。

# 用法
```powershell
python scripts/target_slot.py --resolve        # 打印当前生效的产物目录（单行，无杂音）
python scripts/target_slot.py --audit          # 审计：游离目录 ⇒ 非 0 退出（守卫）
python scripts/target_slot.py --audit --json   # 机器可读输出
python scripts/target_slot.py --list           # 列出登记槽位
```

# 口径（为什么这样判）
* **游离（stray）** = 名字像产物根（`cargo-target*` / `target`）且**不在登记表**里
  ⇒ ⇒ **新增即红**（这是「先登记再长」的机械落点）；
* **既有游离（legacy）** = 登记表已登记的**历史遗留**目录（归属未知的在途产物）
  ⇒ ⇒ 只监控体量上限 ⇒ ⇒ 越涨越红 ⇒ ⇒ **只许缩不许涨**（棘轮）；
* **他仓产物（foreign）** = 工作树里属于**别的仓库**（如私有桌面前端仓）的产物目录
  ⇒ ⇒ **只报告不判红**（不替别人的线做决定）；
* **本机不适用** = 本机没有登记表里的任何槽位根（如 Linux CI）⇒ ⇒ **跳过并退出 0**
  （⇒ ⇒ CI 上不会因为「本机路径不存在」而红）。
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys

for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # pragma: no cover - 老解释器 / 非标准流
        pass

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY_PATH = ROOT / "docs" / "architecture" / "target-slots.json"

# ⭐ **解析真源只有一个**：`scripts/target_dir.py`
#   （仓内 6 个脚本已经改用它 ⇒ ⇒ 再复制一份解析逻辑 = 必然漂移）
#   ⇒ ⇒ 本模块**只调它**，不自己跑 cargo metadata。
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
try:
    from target_dir import (  # noqa: E402
        SOURCE_ENV,
        SOURCE_FALLBACK,
        SOURCE_METADATA,
        resolve_target_dir_with_source,
    )
except ImportError as _err:  # pragma: no cover - 解析真源缺失 ⇒ 明确报错（不许静默降级）
    raise SystemExit(
        "[target-slot] 解析真源 scripts/target_dir.py 缺失或不可导入：" + str(_err)
    )

# 产物根的目录名形状：`cargo-target*`（历史命名）或 `target`（cargo 默认）
TARGET_DIR_NAMES = ("cargo-target", "target")


# ────────────────────────────── 基础工具 ──────────────────────────────


def _norm(path: pathlib.Path | str) -> str:
    """规范化路径用于比较（Windows 大小写不敏感；`\\` 与 `/` 等价）。"""
    text = str(path).replace("/", "\\").rstrip("\\")
    return text.lower()


def _is_under(child: str, parent: str) -> bool:
    """`child` 是否在 `parent` 之内（含相等）。"""
    if child == parent:
        return True
    return child.startswith(parent + "\\")


def _looks_like_target_dir(name: str) -> bool:
    lowered = name.lower()
    return any(lowered == n or lowered.startswith(n + "-") or lowered.startswith(n + "_")
               for n in TARGET_DIR_NAMES)


def dir_size_bytes(path: pathlib.Path) -> int:
    """目录总字节数（`os.scandir` 走一遍 ⇒ 不引第三方依赖）。

    ⚠️ 大目录（数万文件）会花几百毫秒到数秒 ⇒ ⇒ 审计属**低频**命令，不进每条门禁。
    """
    total = 0
    stack = [path]
    seen: set[str] = set()
    while stack:
        current = stack.pop()
        try:
            key = _norm(current)
        except Exception:  # pragma: no cover - 理论上不会
            continue
        if key in seen:
            continue
        seen.add(key)
        try:
            with os.scandir(current) as entries:
                for entry in entries:
                    try:
                        if entry.is_dir(follow_symlinks=False):
                            stack.append(pathlib.Path(entry.path))
                        else:
                            total += entry.stat(follow_symlinks=False).st_size
                    except OSError:
                        continue
        except OSError:
            continue
    return total


def _mb(size: int) -> float:
    return size / (1024 * 1024)


def _gb(size: int) -> float:
    return size / (1024 ** 3)


# ────────────────────────────── 生效目录解析 ──────────────────────────────
#
# ⭐ **唯一解析真源 = `scripts/target_dir.py`**（见文件头 import 段）：
#   `CARGO_TARGET_DIR` → `cargo metadata` → `<repo>\target` 回退（回退必须显式提示）。
#   ⇒ ⇒ 本模块**不复制**那段逻辑（复制 = 必漂移）；只做「槽位归属」判定。


def resolve_target_dir(repo_root: pathlib.Path | None = None) -> tuple[pathlib.Path, str]:
    """⇒ ⇒ (生效产物目录, 来源说明)；委托给 `scripts/target_dir.py`。"""
    root = pathlib.Path(repo_root or ROOT)
    path, source = resolve_target_dir_with_source(root)
    return path, source


# ────────────────────────────── 登记表 ──────────────────────────────


def load_registry(path: pathlib.Path | None = None) -> dict:
    reg_path = pathlib.Path(path or REGISTRY_PATH)
    if not reg_path.is_file():
        raise SystemExit(f"登记表缺失：{reg_path}")
    with open(reg_path, encoding="utf-8") as fh:
        return json.load(fh)


def _slot_roots(registry: dict) -> list[str]:
    roots = []
    for slot in (registry.get("slots") or {}).values():
        path = slot.get("path")
        if path:
            roots.append(_norm(path))
    return roots


def _legacy_entries(registry: dict) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for item in registry.get("legacy_strays") or []:
        path = item.get("path")
        if path:
            out[_norm(path)] = item
    return out


def _reserved_entries(registry: dict) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for item in registry.get("reserved_evidence_paths") or []:
        path = item.get("path")
        if path:
            out[_norm(path)] = item
    return out


# ────────────────────────────── 候选目录发现 ──────────────────────────────


def _git_remote(path: pathlib.Path) -> str:
    """工作树/仓库的 `remote.origin.url`（空串 = 读不到）。"""
    try:
        proc = subprocess.run(
            ["git", "-C", str(path), "config", "--get", "remote.origin.url"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return ""
    if proc.returncode != 0:
        return ""
    return (proc.stdout or "").strip().lower()


def _our_repo_markers() -> set[str]:
    """本仓的远端标记（`origin` / `upstream`）⇒ 用来判「是不是别的仓」。"""
    markers = set()
    for remote in ("origin", "upstream"):
        try:
            proc = subprocess.run(
                ["git", "-C", str(ROOT), "config", "--get", f"remote.{remote}.url"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=30,
                check=False,
            )
        except (OSError, subprocess.SubprocessError):
            continue
        if proc.returncode == 0:
            url = (proc.stdout or "").strip().lower()
            if url:
                markers.add(url)
                markers.add(url.removesuffix(".git"))
    return markers


def find_candidates(registry: dict, max_depth: int = 3) -> list[pathlib.Path]:
    """在 `scan_roots` 下（深度受限）找出**形状像产物根**的目录。"""
    found: list[pathlib.Path] = []
    seen: set[str] = set()
    for raw_root in registry.get("scan_roots") or []:
        root = pathlib.Path(raw_root)
        if not root.is_dir():
            continue
        stack: list[tuple[pathlib.Path, int]] = [(root, 0)]
        while stack:
            current, depth = stack.pop()
            try:
                children = list(os.scandir(current))
            except OSError:
                continue
            for entry in children:
                try:
                    if not entry.is_dir(follow_symlinks=False):
                        continue
                except OSError:
                    continue
                child = pathlib.Path(entry.path)
                key = _norm(child)
                if key in seen:
                    continue
                seen.add(key)
                if _looks_like_target_dir(entry.name):
                    found.append(child)
                    continue  # 产物根内部不再下钻（deps/debug 之类不是候选）
                if depth + 1 < max_depth:
                    stack.append((child, depth + 1))
    return sorted(set(found), key=lambda p: _norm(p))


def _worktree_owner(path: pathlib.Path, our_markers: set[str]) -> str:
    """产物目录的归属判定：`ours` / `foreign` / `unknown`。"""
    parent = path.parent
    if not (parent / ".git").exists():
        return "unknown"
    remote = _git_remote(parent)
    if not remote:
        return "unknown"
    if remote in our_markers or remote.removesuffix(".git") in our_markers:
        return "ours"
    return "foreign"


# ────────────────────────────── 审计 ──────────────────────────────


def audit(registry: dict, repo_root: pathlib.Path | None = None) -> dict:
    slots = {name: slot for name, slot in (registry.get("slots") or {}).items()}
    slot_roots = _slot_roots(registry)
    legacy = _legacy_entries(registry)
    reserved = _reserved_entries(registry)
    thresholds = registry.get("thresholds") or {}

    candidates = find_candidates(registry)
    our_markers = _our_repo_markers()

    registered, legacy_hits, reserved_hits, strays, foreign = [], [], [], [], []

    for candidate in candidates:
        key = _norm(candidate)
        size = dir_size_bytes(candidate)
        owner = _worktree_owner(candidate, our_markers)
        if any(_is_under(key, root) for root in slot_roots):
            registered.append({"path": str(candidate), "size_gb": round(_gb(size), 2), "owner": owner})
            continue
        if key in reserved:
            reserved_hits.append({
                "path": str(candidate),
                "size_gb": round(_gb(size), 2),
                "reason": reserved[key].get("reason", ""),
            })
            continue
        if key in legacy:
            entry = legacy[key]
            ceiling = entry.get("max_gb")
            # `active` = 归属他线且**仍在写入**的目录 ⇒ 只报告不判红：
            #   棘轮（只许缩不许涨）适用于**已停用**的历史遗留；在途他线产物仍在长，
            #   按上限判红 ⇒ 守卫会长期红 ⇒ 长期红的守卫等于没有守卫。
            #   迁移动作（迁进登记槽位后移除本条）写在 reason 里。
            active = bool(entry.get("active"))
            legacy_hits.append({
                "path": str(candidate),
                "size_gb": round(_gb(size), 2),
                "max_gb": ceiling,
                "active": active,
                "reason": entry.get("reason", ""),
                "over_ceiling": bool(
                    (not active)
                    and ceiling is not None
                    and _gb(size) > float(ceiling)
                ),
            })
            continue
        if owner == "foreign":
            foreign.append({"path": str(candidate), "size_gb": round(_gb(size), 2)})
            continue
        strays.append({"path": str(candidate), "size_gb": round(_gb(size), 2), "owner": owner})

    # 生效目录必须在登记槽位之内（否则 ⇒ 产物正长在没登记的地方）
    effective, source = resolve_target_dir(repo_root)
    effective_key = _norm(effective)
    effective_in_slot = next(
        (name for name, slot in slots.items()
         if _is_under(effective_key, _norm(slot.get("path", "")))),
        None,
    )
    effective_is_fallback = source == SOURCE_FALLBACK

    stray_total_gb = round(sum(item["size_gb"] for item in strays), 2)
    legacy_over = [item for item in legacy_hits if item["over_ceiling"]]
    max_single = float(thresholds.get("max_single_stray_gb", 5))
    max_total = float(thresholds.get("max_stray_total_gb", 20))
    single_over = [item for item in strays if item["size_gb"] > max_single]
    total_over = stray_total_gb > max_total

    violations: list[str] = []
    if strays:
        violations.append(
            f"发现 {len(strays)} 个未登记的产物目录（新建即红 ⇒ 先登记再长）："
            + "；".join(f"{item['path']}（{item['size_gb']}G）" for item in strays)
        )
    if total_over:
        violations.append(f"游离目录总量 {stray_total_gb}G 超阈值 {max_total}G")
    if single_over:
        violations.append(
            "单个游离目录超阈值 "
            + f"{max_single}G："
            + "；".join(f"{item['path']}（{item['size_gb']}G）" for item in single_over)
        )
    if legacy_over:
        violations.append(
            "既有游离目录超出登记上限（棘轮：只许缩不许涨）："
            + "；".join(f"{item['path']}（{item['size_gb']}G > {item['max_gb']}G）" for item in legacy_over)
        )
    if effective_is_fallback:
        violations.append(f"生效产物目录解析失败并回退到 {effective} ⇒ 守卫可能失效")
    elif effective_in_slot is None:
        violations.append(f"生效产物目录 {effective} 不在任何登记槽位内 ⇒ 产物正长在没登记的地方")

    return {
        "registry": str(REGISTRY_PATH),
        "effective_target_dir": str(effective),
        "effective_source": source,
        "effective_slot": effective_in_slot,
        "registered": registered,
        "legacy": legacy_hits,
        "reserved": reserved_hits,
        "foreign": foreign,
        "strays": strays,
        "stray_total_gb": stray_total_gb,
        "thresholds": {"max_stray_total_gb": max_total, "max_single_stray_gb": max_single},
        "violations": violations,
        "ok": not violations,
    }


def _print_report(report: dict) -> None:
    print("== 构建产物槽位审计 ==")
    print(f"  生效产物目录：{report['effective_target_dir']}（来源：{report['effective_source']}）")
    if report["effective_slot"]:
        print(f"  归属槽位：{report['effective_slot']}")
    for item in report["registered"]:
        print(f"  ✅ 已登记：{item['path']}（{item['size_gb']}G）")
    for item in report["legacy"]:
        if item.get("active"):
            mark = "🔵 在途他线产物"
            limit = "只报告不判红"
        elif item["over_ceiling"]:
            mark = "🔴 超上限"
            limit = f"上限 {item['max_gb']}G"
        else:
            mark = "🟡 历史遗留"
            limit = f"上限 {item['max_gb']}G"
        print(f"  {mark}：{item['path']}（{item['size_gb']}G / {limit}）{item['reason']}")
    for item in report["reserved"]:
        print(f"  🔒 证据保留：{item['path']}（{item['size_gb']}G）{item['reason']}")
    for item in report["foreign"]:
        print(f"  ℹ️ 他仓产物：{item['path']}（{item['size_gb']}G）⇒ 只报告不判红")
    for item in report["strays"]:
        print(f"  ❌ 游离（未登记）：{item['path']}（{item['size_gb']}G）")
    print(
        f"  ⇒ ⇒ 游离总量 {report['stray_total_gb']}G"
        f"（阈值 总量 {report['thresholds']['max_stray_total_gb']}G /"
        f" 单个 {report['thresholds']['max_single_stray_gb']}G）"
    )
    if report["ok"]:
        print("  ✅ 审计通过")
    else:
        print("  🔴 审计不通过：")
        for violation in report["violations"]:
            print(f"     · {violation}")
        print("  ⇒ ⇒ 修法：① 登记进 docs/architecture/target-slots.json（路径+归属+用途）")
        print("        ② 或把产物迁进已登记槽位（scripts/cargo.ps1 会自动做）")
        print("        ③ 回收口径：python scripts/clean_test_binaries.py --target <目录>")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="构建产物槽位真源解析 + 游离目录审计")
    ap.add_argument("--audit", action="store_true", help="审计登记情况（游离 ⇒ 非 0 退出）")
    ap.add_argument("--resolve", action="store_true", help="打印当前生效的产物目录")
    ap.add_argument("--list", action="store_true", help="列出登记槽位")
    ap.add_argument("--json", action="store_true", help="以 JSON 输出审计结果")
    ap.add_argument("--registry", default=None, help="登记表路径（默认 docs/architecture/target-slots.json）")
    args = ap.parse_args(argv)

    registry = load_registry(pathlib.Path(args.registry) if args.registry else None)

    if args.resolve:
        effective, source = resolve_target_dir()
        print(effective)
        return 0

    if args.list:
        for name, slot in (registry.get("slots") or {}).items():
            print(f"{name}\t{slot.get('path')}\t{slot.get('owner')}\t{slot.get('purpose', '')}")
        return 0

    if not args.audit:
        ap.print_help()
        return 0

    # 本机不适用（没有槽位根，例如 Linux CI）⇒ ⇒ 跳过 ⇒ 退出 0
    slot_roots = [pathlib.Path(slot["path"]) for slot in (registry.get("slots") or {}).values() if slot.get("path")]
    if not any(root.is_dir() for root in slot_roots):
        print("== 构建产物槽位审计 ==\n  ⏭ 跳过：本机没有登记的槽位根（登记表是本机布局 ⇒ 与本机无关）")
        return 0

    report = audit(registry)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        _print_report(report)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
