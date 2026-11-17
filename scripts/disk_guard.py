#!/usr/bin/env python3
"""**磁盘 / 内存守卫** —— 在**做事之前**保证资源，���止「干着干着就满了，一切停摆」。

# 为什么要有它（⭐ owner 直接要求 + 今天实测的事故链）
```
磁盘 85% → 跑 cargo nextest → 产出 60~100 G 测试二进制 → 磁盘 100%
  → 生成脚本写文件**不报错**（静默截断）→ 留下 0 字节 / 半截 JSON
  → 下游报 `JSONDecodeError` → 我误判成「脚本坏了」→ 修错方向
  → 且真正的产物损坏 ⇒ ⇒ 一切停摆
```
⇒ ⇒ ⭐ **根因不是「忘了清」⇒ ⇒ 而是「没有人在动手之前看一眼」**
⇒ ⇒ ⇒ 所以守卫的**位置**比清��本身更重要。

# 三层防护
1. ⭐ **门禁之前**（挂 `harness.py`）：每批门禁跑之前 ⇒ ⇒ 空间不足 ⇒ ⇒ **先回收**
   ⇒ ⇒ 仍不足 ⇒ ⇒ **明确报「资源不足 ⇒ 本批门禁结论不可信」**（**不硬跑**）
   ⇒ ⇒ ⇒ 避免在空间不足时跑测试（那会产出半截文件 ⇒ ⇒ 根因链的最后一环）；
2. ⭐ **写文件之前**（已挂 10 个生成脚本）：`--ensure-free 300`；
3. ⭐ **定时兜底**（cron）：只在 ≥85% 时回收（不制造无谓重编译）。

# 内存也要看（owner 说「内存监控」）
`cargo` 编译/链接是**内存大户** ⇒ ⇒ 内存吃满 ⇒ ⇒ OOM ⇒ ⇒ 同样会「停摆」
⇒ ⇒ ⇒ 所以守卫**同时**报内存 ⇒ ⇒ 不足时**提前**告知（而不是等 OOM）。

# 用法
```sh
python3 scripts/disk_guard.py                 # 打印状态（磁盘 + 内存）
python3 scripts/disk_guard.py --ensure 5000   # 确保磁盘可用 ≥5000 MB（不足则先回收）
python3 scripts/disk_guard.py --ensure-mem 4000  # 确保内存可用 ≥4000 MB
python3 scripts/disk_guard.py --audit-targets  # 连带审计构建产物槽位（游离目录 ⇒ 非 0 退出）
```

# ⭐ 阈值口径（2026-11-17 补：百分比阈值在大盘上等于没有）
原口径只有百分比（`DISK_WARN_PCT=80` / `DISK_HARD_PCT=90`）⇒ ⇒ 在 924 GB 的盘上，
90% 才转红 ⇒ ⇒ 也就是「还剩 90 GB 才报警」⇒ ⇒ **等于没有报警**。
⇒ ⇒ 现在**并列**一套**绝对可用量**阈值（可被环境变量覆盖）：

| 口径 | 环境变量 | 默认 | 含义 |
|---|---|---|---|
| 绝对告警 | `APEIRETH_DISK_WARN_FREE_MB` | 40000 MB | 可用 < 40 GB ⇒ 🟡 |
| 绝对硬线 | `APEIRETH_DISK_HARD_FREE_MB` | 10000 MB | 可用 < 10 GB ⇒ 🔴 **中止** |
| 百分比告警 | `APEIRETH_DISK_WARN_PCT` | 80 | 保留（大盘上意义有限） |
| 百分比硬线 | `APEIRETH_DISK_HARD_PCT` | 90 | 保留 |
| 内存告警 | `APEIRETH_MEM_WARN_MB` | 4000 MB | 保留 |

* `--ensure MB` 取 **max(调用方要的, 绝对硬线)** ⇒ ⇒ 调用方即使只写 `--ensure 2000`
  （harness 的旧口径）⇒ ⇒ 实际仍按 10 GB 硬线执行 ⇒ ⇒ **口径不再被调用方悄悄放松**；
* 绝对硬线设成 `0` ⇒ ⇒ **关闭**这条下限（给 CI 这类空间本来紧张的机器用）；
* ⭐ **CI 例外（2026-11-17 补）**：托管 runner 的可用空间由**平台**决定（常只剩 10~20 GB）
  ⇒ ⇒ 套用本机 10 GB 底线会让构建中途跌破时**每条门禁都变 UNVERIFIED**
  ⇒ ⇒ ⇒ **长期红的守卫等于没有守卫** ⇒ ⇒ CI 下**不套用**本机底线；
  豁免是**有界且显式**的：只在 `CI` 为真**且**用户没有显式给 `APEIRETH_DISK_HARD_FREE_MB` 时生效
  ⇒ ⇒ 显式给了值就以用户值为准（不许自我豁免）。
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CLEANER = ROOT / "scripts" / "clean_test_binaries.py"
SLOT_AUDITOR = ROOT / "scripts" / "target_slot.py"
REGISTRY_PATH = ROOT / "docs" / "architecture" / "target-slots.json"

# 阈值（可被环境变量覆盖 ⇒ ⇒ 不同机器可调）
DISK_WARN_PCT = int(os.environ.get("APEIRETH_DISK_WARN_PCT", "80"))
DISK_HARD_PCT = int(os.environ.get("APEIRETH_DISK_HARD_PCT", "90"))
MEM_WARN_MB = int(os.environ.get("APEIRETH_MEM_WARN_MB", "4000"))
# ⭐ **绝对可用量**阈值（大盘上百分比阈值失效 ⇒ ⇒ 补一套按 MB 判的）
DISK_WARN_FREE_MB = int(os.environ.get("APEIRETH_DISK_WARN_FREE_MB", "40000"))
DISK_HARD_FREE_MB = int(os.environ.get("APEIRETH_DISK_HARD_FREE_MB", "10000"))

# ⭐ **CI 口径**（2026-11-17 补）：托管 runner 的可用空间由**平台**决定
#   （Ubuntu/macOS runner 常只剩 10~20 GB）⇒ ⇒ 本机底线（10 GB）套到 CI
#   ⇒ ⇒ 构建中途一旦跌破 ⇒ ⇒ `harness.py` 把**每条门禁**记 UNVERIFIED
#   ⇒ ⇒ ⇒ **长期红的守卫等于没有守卫** ⇒ ⇒ CI 下**不套用**本机底线。
# ⚠️ 豁免是**有界且显式**的：只在 ① `CI` 为真 ② 用户**没有**显式给
#    `APEIRETH_DISK_HARD_FREE_MB` 时生效 ⇒ ⇒ 显式给了值就以用户值为准（不许自我豁免）。
_CI_MODE = os.environ.get("CI", "").strip().lower() not in ("", "0", "false", "no")
_HARD_FREE_EXPLICIT = "APEIRETH_DISK_HARD_FREE_MB" in os.environ


def effective_hard_free_mb() -> int:
    """当前生效的**绝对硬线**（MB）。

    * 本机（无 `CI`）⇒ 10 GB 默认；
    * CI 且用户**未**显式给值 ⇒ 0（**关闭**本机底线，避免托管 runner 长期红）；
    * CI 且用户显式给值 ⇒ 以用户值为准。
    """
    if _HARD_FREE_EXPLICIT:
        return DISK_HARD_FREE_MB
    if _CI_MODE:
        return 0
    return DISK_HARD_FREE_MB


sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
try:
    from target_dir import resolve_target_dir  # noqa: E402
except ImportError:  # pragma: no cover - 解析真源缺失 ⇒ 回收退化为「不回收」
    resolve_target_dir = None  # type: ignore[assignment]


def disk_stats() -> tuple[int, int, int, int]:
    """⇒ ⇒ (总 MB, 已用 MB, 可用 MB, 使用百分比)。

    ⭐ 跨平台（2026-11-10）：原实现用 `os.statvfs("/")` ⇒ **只在 POSIX 存在**
    ⇒ Windows 上直接 `AttributeError` ⇒ ⇒ 门禁守卫（harness 每条门禁前都跑它）
    **在 Windows 上必然崩** ⇒ ⇒ 全部门禁被记 `UNVERIFIED` ⇒ 裁决恒为 `BLOCKED`。

    ⇒ 改用 `shutil.disk_usage`（标准库，POSIX 与 Windows 同义：
    `total = f_blocks*f_frsize`、`free = f_bavail*f_frsize` ⇒ 与原 Linux 数字逐字一致）。
    """
    total_b, _used_b, free_b = shutil.disk_usage("/")
    total = total_b // (1024 * 1024)
    free = free_b // (1024 * 1024)
    used = total - free
    pct = int(used * 100 / total) if total else 0
    return total, used, free, pct


def _mem_stats_windows() -> tuple[int, int]:
    """Windows 可用/总物理内存（MB）⇒ `GlobalMemoryStatusEx`（不用第三方库）。"""
    try:
        import ctypes

        class _MemoryStatusEx(ctypes.Structure):
            _fields_ = [
                ("dwLength", ctypes.c_ulong),
                ("dwMemoryLoad", ctypes.c_ulong),
                ("ullTotalPhys", ctypes.c_ulonglong),
                ("ullAvailPhys", ctypes.c_ulonglong),
                ("ullTotalPageFile", ctypes.c_ulonglong),
                ("ullAvailPageFile", ctypes.c_ulonglong),
                ("ullTotalVirtual", ctypes.c_ulonglong),
                ("ullAvailVirtual", ctypes.c_ulonglong),
                ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
            ]

        stat = _MemoryStatusEx()
        stat.dwLength = ctypes.sizeof(stat)
        if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(stat)):
            return stat.ullAvailPhys // (1024 * 1024), stat.ullTotalPhys // (1024 * 1024)
    except Exception:  # noqa: BLE001 —— 取不到就当「未知」
        return 0, 0
    return 0, 0


def mem_stats() -> tuple[int, int]:
    """⇒ ⇒ (可用 MB, 总 MB)（Linux 读 /proc/meminfo；Windows 读 GlobalMemoryStatusEx ⇒ 不用第三方库）。"""
    if os.name == "nt":
        return _mem_stats_windows()
    info: dict[str, int] = {}
    try:
        for line in pathlib.Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            key, _, rest = line.partition(":")
            value = rest.strip().split()[0]
            info[key] = int(value) // 1024  # kB ⇒ ⇒ MB
    except (OSError, ValueError, IndexError):
        return 0, 0
    total = info.get("MemTotal", 0)
    avail = info.get("MemAvailable", info.get("MemFree", 0))
    return avail, total


def _reclaim_targets(include_slots: bool) -> list[pathlib.Path]:
    """回收目标：① **当前生效**的产物目录（自己这条线）②（可选）登记槽位。

    ⚠️ **为什么不再无参调用**：旧实现的 `reclaim()` 不传 `--target` ⇒ ⇒
    `clean_test_binaries.py` 只在 Linux 的 `/home/bai`、`/var/tmp`、`/tmp` 下找候选
    ⇒ ⇒ **在 Windows 上等于空转**（回收从来没真发生过）。
    ⇒ ⇒ 现在**显式**把目录喂给它。
    """
    targets: list[pathlib.Path] = []
    if resolve_target_dir is not None:
        try:
            targets.append(resolve_target_dir(ROOT))
        except Exception:  # noqa: BLE001 —— 解析失败 ⇒ 不猜目录
            pass
    if include_slots:
        try:
            registry = json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))
            for slot in (registry.get("slots") or {}).values():
                raw = slot.get("path")
                if raw:
                    targets.append(pathlib.Path(raw))
        except (OSError, ValueError, TypeError):
            pass
    seen: set[str] = set()
    unique: list[pathlib.Path] = []
    for target in targets:
        key = str(target).lower()
        if key not in seen:
            seen.add(key)
            unique.append(target)
    return unique


def reclaim(include_slots: bool = False) -> bool:
    """回收 cargo 测试二进制 ⇒ ⇒ 返回是否成功（尽力而为）。"""
    targets = _reclaim_targets(include_slots)
    if not targets:
        return False
    argv = [sys.executable, str(CLEANER)]
    for target in targets:
        argv.extend(["--target", str(target)])
    try:
        proc = subprocess.run(
            argv,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=600,
            check=False,
        )
        if proc.returncode != 0:
            tail = (proc.stderr or proc.stdout or "").strip().splitlines()
            print(
                "  ⇒ 回收未完全成功："
                + (tail[-1] if tail else f"exit={proc.returncode}")
            )
        return proc.returncode == 0
    except Exception:  # noqa: BLE001 —— 回收是尽力而为
        return False


def audit_targets() -> bool:
    """审计构建产物槽位（游离目录 ⇒ 红）⇒ ⇒ 返回是否通过。

    审计器 = `scripts/target_slot.py`（唯一真源）⇒ ⇒ 这里**只调它**，不复制判定逻辑。
    """
    if not SLOT_AUDITOR.is_file():
        print(f"  ⇒ 槽位审计器缺失：{SLOT_AUDITOR}")
        return True  # 审计器不在 ⇒ 不阻断（但明确提示）
    try:
        proc = subprocess.run(
            [sys.executable, str(SLOT_AUDITOR), "--audit"],
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=900,
            check=False,
        )
    except Exception as error:  # noqa: BLE001 —— 审计异常不阻断（宁可少一道保险）
        print(f"  ⇒ 槽位审计调不动：{error}")
        return True
    out = (proc.stdout or "").strip()
    err = (proc.stderr or "").strip()
    if out:
        print(out)
    if err:
        print(err, file=sys.stderr)
    return proc.returncode == 0


def report() -> tuple[int, int, int, int, int, int]:
    d_total, d_used, d_free, d_pct = disk_stats()
    m_avail, m_total = mem_stats()
    # ⭐ 磁盘灯：**百分比与绝对可用量取更严的一档**（大盘上百分比失效 ⇒ 绝对值说了算）
    hard_free_mb = effective_hard_free_mb()
    pct_red = d_pct >= DISK_HARD_PCT
    pct_yellow = d_pct >= DISK_WARN_PCT
    free_red = hard_free_mb > 0 and d_free < hard_free_mb
    free_yellow = DISK_WARN_FREE_MB > 0 and d_free < DISK_WARN_FREE_MB
    flag_disk = "🔴" if (pct_red or free_red) else ("🟡" if (pct_yellow or free_yellow) else "🟢")
    flag_mem = "🔴" if m_avail < 1000 else ("🟡" if m_avail < MEM_WARN_MB else "🟢")
    floor_note = "（CI：不套用本机底线）" if (_CI_MODE and not _HARD_FREE_EXPLICIT) else ""
    print(
        f"  {flag_disk} 磁盘：{d_used / 1024:.0f}G / {d_total / 1024:.0f}G"
        f"（{d_pct}%）· 可用 **{d_free / 1024:.0f}G**"
        f"（绝对硬线 {hard_free_mb} MB / 告警 {DISK_WARN_FREE_MB} MB）{floor_note}"
    )
    print(
        f"  {flag_mem} 内存：可用 **{m_avail / 1024:.1f}G** / {m_total / 1024:.0f}G"
    )
    return d_total, d_used, d_free, d_pct, m_avail, m_total


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ensure", type=int, default=0, metavar="MB", help="确保磁盘可用 ≥ MB")
    ap.add_argument("--ensure-mem", type=int, default=0, metavar="MB", help="确保内存可用 ≥ MB")
    ap.add_argument(
        "--audit-targets",
        action="store_true",
        help="连带审计构建产物槽位（游离目录 ⇒ 非 0 退出）",
    )
    ap.add_argument(
        "--reclaim-slots",
        action="store_true",
        help="回收时把登记槽位也纳入（默认只回收当前生效的产物目录）",
    )
    args = ap.parse_args()

    # ⭐ `--ensure` 的**绝对下限**：调用方要多少不算数 ⇒ ⇒ 至少要满足绝对硬线
    #   （harness 旧口径写的是 2000 MB ⇒ 在 924 GB 盘上等于没有门禁 ⇒ 这里补回来）
    #   ⚠️ 但 CI 下不套用本机底线（见 `effective_hard_free_mb` 的说明）。
    ensure_disk = args.ensure
    hard_free_mb = effective_hard_free_mb()
    if hard_free_mb > 0:
        ensure_disk = max(ensure_disk, hard_free_mb)

    if not ensure_disk and not args.ensure_mem and not args.audit_targets:
        report()
        return 0

    d_total, d_used, d_free, d_pct, m_avail, m_total = report()

    if ensure_disk and d_free < ensure_disk:
        print(f"  ⇒ 磁盘 {d_free} MB < 需要 {ensure_disk} MB ⇒ ⇒ 先回收测试二进制")
        reclaim(include_slots=args.reclaim_slots)
        d_total, d_used, d_free, d_pct, m_avail, m_total = report()
        if d_free < ensure_disk:
            print(
                f"  🔴 回收后仍只有 {d_free} MB（需要 {ensure_disk} MB）⇒ ⇒ **中止**"
                f" ⇒ ⇒ 宁可**不做**，也不在空间不足时干活（空间不足 ⇒ ⇒ 写文件静默截断）",
                file=sys.stderr,
            )
            return 1

    if args.ensure_mem and m_avail < args.ensure_mem:
        print(
            f"  🔴 内存可用 {m_avail} MB < 需要 {args.ensure_mem} MB ⇒ ⇒ **中止**"
            f"（cargo 编译是内存大户 ⇒ ⇒ 内存不足 ⇒ ⇒ OOM ⇒ ⇒ 一样会停摆）",
            file=sys.stderr,
        )
        return 1

    if args.audit_targets and not audit_targets():
        print(
            "  🔴 构建产物槽位审计不通过（游离目录 ⇒ ⇒ 产物正长在没登记的地方）",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
