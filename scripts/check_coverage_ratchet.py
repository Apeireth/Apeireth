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
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COBERTURA = ROOT / "cobertura.xml"
BASELINE = ROOT / "scripts" / "coverage-baseline.txt"
TOLERANCE_PP = 0.5  # 百分点噪声容差：覆盖率测量本身有抖动，不容差的门禁会假红

# 计入分母的路径前缀。
#
# `legacy/` 与 `research/` 在 Cargo workspace 里是 **exclude** 的（`exclude = ["legacy", "research"]`），
# 不属于 30 members，因此**没有任何 workspace 测试能覆盖它们** —— 它们在报告里恒为 0%。
# 若把它们算进分母：
#   ① 覆盖率数字被稀释（本机实测：136791 行里 8837 行是这两个目录 → 32.10% 被压到）；
#   ② **legacy 一有增删，分母就变，棘轮会假红** —— 而那不是任何人改代码造成的。
# 所以基线与检查**两侧同口径**：都按本表过滤，legacy 的变动不参与棘轮判定。
COUNTED_PREFIXES = ("crates/", "src/", "benches/")


def read_cobertura(path: Path) -> tuple[float, int, int]:
    """返回 (行覆盖率百分比, 命中行数, 计入分母的行数)。

    **按 `<line hits=...>` 逐行统计**（不直接用 `<coverage>` 根节点的 line-rate）：
    根节点的聚合值包含了我们要排除的 legacy/research，必须自己按同一口径重算，
    否则「基线」与「检查」两侧口径不一致。
    """
    tree = ET.parse(path)
    root = tree.getroot()
    covered = 0
    valid = 0
    for cls in root.iter("class"):
        filename = cls.get("filename", "")
        if not filename.startswith(COUNTED_PREFIXES):
            continue
        for line in cls.iter("line"):
            valid += 1
            if int(line.get("hits", "0")) > 0:
                covered += 1
    if valid == 0:
        return 0.0, 0, 0
    return covered * 100.0 / valid, covered, valid


def read_baseline() -> float | None:
    return read_baseline_full()[0]


def read_baseline_full() -> tuple[float | None, str | None, int | None]:
    """返回 (基线百分比, 基线记录的 commit sha, 基线记录的分母行数)。

    基线文件可含 `commit = <sha>` 与 `valid_lines = <N>` 两行（可选）。
    `valid_lines` 是**判别测量范围是否一致**的关键：百分比是「命中/分母」，
    分母变了说明「纳入统计的文件集合」变了 —— 可能是代码长了，
    也可能是**测量环境不同**（本地与 CI 的 tarpaulin 版本/编译产物不同，
    实测本地比 CI 少 3800 行：CI 额外纳入 18 个不在 `-p` 列表 crate 的文件）。
    """
    if not BASELINE.exists():
        return None, None
    value: float | None = None
    commit: str | None = None
    valid: int | None = None
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.fullmatch(r"line_coverage_pp\s*=\s*([0-9.]+)", line)
        if m:
            value = float(m.group(1))
            continue
        m = re.fullmatch(r"commit\s*=\s*([0-9a-fA-F]{7,40})", line)
        if m:
            commit = m.group(1)
            continue
        m = re.fullmatch(r"valid_lines\s*=\s*(\d+)", line)
        if m:
            valid = int(m.group(1))
    return value, commit, valid


def current_commit() -> str | None:
    """当前 checkout 的短 sha（取不到就返回 None，不影响判定）。"""
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--short=12", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return None
    if out.returncode != 0:
        return None
    return out.stdout.strip() or None


def write_baseline(value: float, commit: str | None = None, valid: int | None = None) -> None:
    lines = [
        "# 覆盖率棘轮基线（scripts/check_coverage_ratchet.py --update 生成）",
        "# 规则：只减不增；下降超过 0.5pp 才判红（测量噪声容差）。",
        f"line_coverage_pp = {value:.2f}",
    ]
    if valid is not None:
        lines.append(f"# 测量时的分母行数（用于判别测量范围是否一致）")
        lines.append(f"valid_lines = {valid}")
    if commit:
        lines.append("# 基线测于该 commit（用于识别基线过期）")
        lines.append(f"commit = {commit}")
    BASELINE.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    mode = argv[1] if len(argv) > 1 else "--report"
    if not COBERTURA.exists():
        print(f"[coverage-ratchet] 找不到 {COBERTURA.relative_to(ROOT)}，先跑 tarpaulin --out Xml")
        return 0 if mode != "--check" else 1

    pct, covered, valid = read_cobertura(COBERTURA)
    base, base_commit, base_valid = read_baseline_full()
    print(f"[coverage-ratchet] 当前行覆盖 {pct:.2f}%（{covered}/{valid} 行）")

    # ── 基线过期检测 ────────────────────────────────────────────────────────
    # 棘轮是**百分比**门禁, 而分母随代码量增长。若基线是在**旧 commit** 上量的,
    # 而这期间新增了大量代码, 百分比会自然下降 —— 那是「代码长进来了」,
    # 不是「测试被删了」。但这两者在数字上**无法区分**,
    # 所以必须显式记录基线测于哪个 commit, 并在 commit 漂移时**提示**基线可能过期
    # (提示不判红, 但必须让人看见 —— 否则每次 main 前进都会重复误判一次)。
    #
    # 实测踩坑 (2026-10-24): 基线取自 5dd1b840 (34.38%),
    # 到 247f55dd 期间合入 PR #81..#85, 新增 18 个源文件 (~3800 行) →
    # 分母变大 → 同一批代码测出 33.36% → 棘轮判红。
    # **棘轮报的是真信号** (新代码没被测到), 但「基线过期」这个成因必须被看见。
    head = current_commit()
    stale = bool(base_commit and head and not head.startswith(base_commit))

    # 测量范围是否一致: 分母行数与基线记录的不一致 ⇒ 纳入统计的文件集合变了。
    # 两种成因在数字上无法区分: (a) 期间新增了代码; (b) **测量环境不同**
    # (本地与 CI 的 tarpaulin 报告的文件集不同 —— 实测本地比 CI 少 3800 行:
    #  CI 额外纳入 18 个不在 `-p` 列表 crate 的文件)。
    # 因此这里只**提示**, 不改判据 —— 判据由 owner 看提示后决定是否刷新基线。
    scope_diff = bool(base_valid and valid and abs(valid - base_valid) / base_valid > 0.01)
    if scope_diff:
        print(
            f"[coverage-ratchet] ⚠️ 测量范围与基线不同: 分母 {valid} 行 vs 基线 {base_valid} 行"
            f"（差 {(valid - base_valid) * 100 / base_valid:+.2f}%）"
        )
        print(
            "[coverage-ratchet]   ↳ 成因二选一, 数字上无法区分: 期间新增/删除了代码, "
            "或**测量环境不同**（本地 vs CI 的 tarpaulin 报告文件集不同）"
        )
        print("[coverage-ratchet]   ↳ 本门禁以 **CI** 为准（门禁在 CI 跑）；请在 CI 上核对后再决定是否刷新基线")
    if stale:
        print(
            f"[coverage-ratchet] ⚠️ 基线测于 commit {base_commit}，当前 HEAD {head} —— "
            f"基线可能已过期（期间新增代码会自然拉低百分比，"
            f"这与「测试被删」在数字上无法区分）"
        )
        print("[coverage-ratchet] 建议：在当前 HEAD 上重测并跑 --update 刷新基线")

    if mode == "--update":
        if base is not None and pct < base - TOLERANCE_PP:
            print(f"[coverage-ratchet] 拒绝更新：{pct:.2f}% 低于基线 {base:.2f}%（棘轮只减不增）")
            return 1
        write_baseline(pct, head, valid)
        print(f"[coverage-ratchet] 基线已写入 {pct:.2f}%" + (f"（commit {head}）" if head else ""))
        return 0

    if base is None:
        print("[coverage-ratchet] baseline not recorded — ratchet INACTIVE（尚未记录基线，本门禁未生效）")
        print("[coverage-ratchet] 记录方法：确认本数字可信后跑 --update 并提交基线文件")
        return 0

    print(f"[coverage-ratchet] 基线 {base:.2f}%（容差 {TOLERANCE_PP}pp）")
    if pct < base - TOLERANCE_PP:
        print(f"[coverage-ratchet] ✗ 覆盖率下降 {base - pct:.2f}pp，超过容差，判红")
        if stale:
            print(
                "[coverage-ratchet]   ↳ 注意：基线已过期（见上），本次判红**可能**只是"
                "「期间新增了未覆盖的新代码」，不是测试被删。"
            )
            print("[coverage-ratchet]   ↳ 请在当前 HEAD 上重测后 --update 刷新基线，再看是否仍低。")
        return 1
    if pct > base + TOLERANCE_PP:
        print(f"[coverage-ratchet] 覆盖率上升 {pct - base:.2f}pp —— 建议跑 --update 抬高基线")
    print("[coverage-ratchet] ✓ 通过")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
