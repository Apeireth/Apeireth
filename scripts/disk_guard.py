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
```
"""

from __future__ import annotations

import argparse
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CLEANER = ROOT / "scripts" / "clean_test_binaries.py"

# 阈值（可被环境变量覆盖 ⇒ ⇒ 不同机器可调）
DISK_WARN_PCT = int(os.environ.get("APEIRETH_DISK_WARN_PCT", "80"))
DISK_HARD_PCT = int(os.environ.get("APEIRETH_DISK_HARD_PCT", "90"))
MEM_WARN_MB = int(os.environ.get("APEIRETH_MEM_WARN_MB", "4000"))


def disk_stats() -> tuple[int, int, int, int]:
    """⇒ ⇒ (总 MB, 已用 MB, 可用 MB, 使用百分比)。"""
    st = os.statvfs("/")
    total = st.f_blocks * st.f_frsize // (1024 * 1024)
    free = st.f_bavail * st.f_frsize // (1024 * 1024)
    used = total - free
    pct = int(used * 100 / total) if total else 0
    return total, used, free, pct


def mem_stats() -> tuple[int, int]:
    """⇒ ⇒ (可用 MB, 总 MB)（读 /proc/meminfo ⇒ ⇒ 不用第三方库）。"""
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


def reclaim() -> bool:
    """回收 cargo 测试二进制 ⇒ ⇒ 返回是否成功。"""
    try:
        proc = subprocess.run(
            [sys.executable, str(CLEANER)],
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            timeout=300,
            check=False,
        )
        return proc.returncode == 0
    except Exception:  # noqa: BLE001 —— 回收是尽力而为
        return False


def report() -> tuple[int, int, int, int, int, int]:
    d_total, d_used, d_free, d_pct = disk_stats()
    m_avail, m_total = mem_stats()
    flag_disk = "🔴" if d_pct >= DISK_HARD_PCT else ("🟡" if d_pct >= DISK_WARN_PCT else "🟢")
    flag_mem = "🔴" if m_avail < 1000 else ("🟡" if m_avail < MEM_WARN_MB else "🟢")
    print(
        f"  {flag_disk} 磁盘：{d_used / 1024:.0f}G / {d_total / 1024:.0f}G"
        f"（{d_pct}%）· 可用 **{d_free / 1024:.0f}G**"
    )
    print(
        f"  {flag_mem} 内存：可用 **{m_avail / 1024:.1f}G** / {m_total / 1024:.0f}G"
    )
    return d_total, d_used, d_free, d_pct, m_avail, m_total


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ensure", type=int, default=0, metavar="MB", help="确保磁盘可用 ≥ MB")
    ap.add_argument("--ensure-mem", type=int, default=0, metavar="MB", help="确保内存可用 ≥ MB")
    args = ap.parse_args()

    if not args.ensure and not args.ensure_mem:
        report()
        return 0

    d_total, d_used, d_free, d_pct, m_avail, m_total = report()

    if args.ensure and d_free < args.ensure:
        print(f"  ⇒ 磁盘 {d_free} MB < 需要 {args.ensure} MB ⇒ ⇒ 先回收测试二进制")
        reclaim()
        d_total, d_used, d_free, d_pct, m_avail, m_total = report()
        if d_free < args.ensure:
            print(
                f"  🔴 回收后仍只有 {d_free} MB（需要 {args.ensure} MB）⇒ ⇒ **中止**"
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

    return 0


if __name__ == "__main__":
    sys.exit(main())
