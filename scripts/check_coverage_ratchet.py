#!/usr/bin/env python3
"""覆盖率棘轮：**只减不增**（与 doc-caliber-baseline / known-missing 同一范式）。

## 为什么需要
`coverage.yml` 已经跑 tarpaulin 并上传 codecov，但**没有任何阈值** ——
覆盖率可以一路静默下滑，直到有人偶然看一眼报表。这个脚本把"下滑"变成红灯。

## 三种模式
- `--report`（默认）：只打印当前值与基线，**不判红**；
- `--check`：低于基线（扣掉噪声容差）则 exit 1；
- `--update`：把当前值写进基线（**只允许变大或首次记录**）。

## 基线未记录时的行为（重要）
仓里这份基线**首次记录前不生效**，脚本会明确打印
`baseline not recorded — ratchet INACTIVE`，而不是假装绿灯。
拒的做法：先塞一个凭空的数字当基线——那等于用假数据造一个假门禁。
"""

from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COBERTURA = ROOT / "cobertura.xml"
BASELINE = ROOT / "scripts" / "coverage-baseline.txt"
TOLERANCE_PP = 0.5  # 百分点噪声容差：覆盖率测量本身有抖动，不容差的门禁会假红


def read_cobertura(path: Path) -> tuple[float, int, int]:
    """返回 (line_rate 百分比, covered, valid)。"""
    tree = ET.parse(path)
    root = tree.getroot()
    rate = float(root.get("line-rate", "0"))
    covered = int(root.get("lines-covered", "0"))
    valid = int(root.get("lines-valid", "0"))
    return rate * 100.0, covered, valid


def read_baseline() -> float | None:
    if not BASELINE.exists():
        return None
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.fullmatch(r"line_coverage_pp\s*=\s*([0-9.]+)", line)
        if m:
            return float(m.group(1))
    return None


def write_baseline(value: float) -> None:
    BASELINE.write_text(
        "# 覆盖率棘轮基线（scripts/check_coverage_ratchet.py --update 生成）\n"
        "# 规则：只减不增；下降超过 0.5pp 才判红（测量噪声容差）。\n"
        f"line_coverage_pp = {value:.2f}\n",
        encoding="utf-8",
    )


def main(argv: list[str]) -> int:
    mode = argv[1] if len(argv) > 1 else "--report"
    if not COBERTURA.exists():
        print(f"[coverage-ratchet] 找不到 {COBERTURA.relative_to(ROOT)}，先跑 tarpaulin --out Xml")
        return 0 if mode != "--check" else 1

    pct, covered, valid = read_cobertura(COBERTURA)
    base = read_baseline()
    print(f"[coverage-ratchet] 当前行覆盖 {pct:.2f}%（{covered}/{valid} 行）")

    if mode == "--update":
        if base is not None and pct < base - TOLERANCE_PP:
            print(f"[coverage-ratchet] 拒绝更新：{pct:.2f}% 低于基线 {base:.2f}%（棘轮只减不增）")
            return 1
        write_baseline(pct)
        print(f"[coverage-ratchet] 基线已写入 {pct:.2f}%")
        return 0

    if base is None:
        print("[coverage-ratchet] baseline not recorded — ratchet INACTIVE（尚未记录基线，本门禁未生效）")
        print("[coverage-ratchet] 记录方法：确认本数字可信后跑 --update 并提交基线文件")
        return 0

    print(f"[coverage-ratchet] 基线 {base:.2f}%（容差 {TOLERANCE_PP}pp）")
    if pct < base - TOLERANCE_PP:
        print(f"[coverage-ratchet] ✗ 覆盖率下降 {base - pct:.2f}pp，超过容差，判红")
        return 1
    if pct > base + TOLERANCE_PP:
        print(f"[coverage-ratchet] 覆盖率上升 {pct - base:.2f}pp —— 建议跑 --update 抬高基线")
    print("[coverage-ratchet] ✓ 通过")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
