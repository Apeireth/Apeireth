#!/usr/bin/env python3
"""内部卷检索基线闸门 —— 把「**准确度不许掉**」落成**机器门禁**。

# 为什么要有这个门禁

owner 裁定（2026-10-25）「**准确度一分不许掉**」⇒ 判据已经实现为
`apeireth_memory::golden_set::check_no_regression`（相对劣化 > 1% ⇒ 拦下）。

但**没有基线可比** ⇒ 改检索代码时分数涨跌**无人知晓** ⇒ 纪律落空。
⇒ 基线固化在 `scripts/golden-baseline.json` ⇒ 本脚本每次都拿**当前真实分数**
去跟它比 ⇒ 「掉了」在**门禁时就被发现**，而不是等 owner 发现。

# 流程

1. 读 `scripts/golden-baseline.json`（不存在 ⇒ **报错退出**，不许当通过）；
2. 检查语料在不在（`$APEIRETH_DATA_DIR/novels/` 或 `~/.apeireth/novels/`）
   ⇒ **缺语料 ⇒ 打印「跳过（缺语料）」并退出 0**（**如实**：跳过不是通过，
   打印里必须写明「不是通过」）；
3. **跑当前分数** —— ⚠️ **不在 Python 里重实现检索**（那是 Rust 的活，两套实现
   ⇒ 两个分数 ⇒ 基线失去意义）⇒ 调
   `cargo run -p apeireth-memory --example golden_baseline_report`
   ⇒ 它输出**一行 JSON**（当前分数）⇒ 本脚本解析它；
4. 与基线比（**相对劣化 > 1% ⇒ 掉了** ⇒ 打印**哪个语料、哪个指标、跌了多少**
   ⇒ 退出 1）；
5. 全绿 ⇒ 打印 `OK: 检索分数未低于基线` ⇒ 退出 0。

# 不可比的两种情形（都**不判掉**，但都**不算通过**）

* **题数变了** ⇒ 分数不是同一把尺子量出来的 ⇒ 不可比 ⇒ 报错退出；
* **基线里没有这份语料 / 跑出来的语料基线里没有** ⇒ 不可比 ⇒ 报错退出。

⇒ 「不可比」**绝不**被当成「没掉」—— 那正是纪律失效的方式。

# 退出码

* `0` = 通过（分数未低于基线）**或**如实跳过（缺语料，打印里写明）；
* `1` = 分数掉了 / 不可比 / 基线文件坏 / 跑分失败。

# 口径提醒

* 容忍度 = **相对 1%**（与 `check_no_regression` 同口径 ⇒ 同一个判据）。
* 基线**只固化两项检索指标**（`recall_at_k` / `ndcg_at_k`）⇒ 注入族指标
  （`evidence_precision` / `groundedness`）**不参与**本闸门
  （它们测的是「注入给模型什么」，与「检索准不准」不是一件事）。

# 用法

```bash
python3 scripts/check_golden_baseline.py
```

⚠️ **依赖真实语料**：小说文件在**仓外**（`~/.apeireth/novels/`，
下载器 `scripts/download_novels.sh` 带 SHA-256）⇒ 缺失时**如实跳过**
（不是「通过」⇒ 打印明说跳过）。

# 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）

四级口径：

* **IMPLEMENTED** ✔：能跑、能比、能拦（脚本级实现，仅此一级）；
* **PRODUCTION WIRED** ✘：**尚未接入 CI workflow**（需要 owner/总指挥挂到
  流水线里才成为「门禁」）⇒ 现状是「**能手工跑的闸**」，不是「CI 里的闸」；
* **DEFAULT ENABLED** ✘：没有开关位；
* **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过一遍
  （2026-10-26），**未在第二台机器复现** ⇒ 不宣称跨机器可复现。
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE_PATH = REPO_ROOT / "scripts" / "golden-baseline.json"

# 语料名 → 仓外文件名。
# ⚠️ **与 `crates/engine/memory/examples/golden_baseline_report.rs` 里的同一张表一致**
# （脚本要先判断语料在不在，才决定跑不跑 ⇒ 两边必须说同一件事）。
CORPUS_FILES = {
    "redchamber": "redchamber.txt",
    "watershed": "watershed.txt",
}

# 相对劣化容忍度（1%）—— 与 `golden_set::check_no_regression` **同口径**
# （同一个判据只应有一处定义；这里与 Rust 侧各写一份，是刻意并行的 ⇒ 两侧都有测试钉住）。
TOLERANCE_RATIO = 0.01

# 绝对下限（吸收浮点表示误差）—— 与 `check_no_regression` 同口径。
TOLERANCE_FLOOR = 1e-9


def novels_dir() -> Path:
    """语料目录：`$APEIRETH_DATA_DIR/novels/`，默认 `~/.apeireth/novels/`。"""
    root = os.environ.get("APEIRETH_DATA_DIR")
    base = Path(root) if root else Path.home() / ".apeireth"
    return base / "novels"


def load_baseline() -> dict[str, Any]:
    """读基线文件；坏文件 ⇒ 如实报错（不许当空基线蒙混过关）。"""
    if not BASELINE_PATH.exists():
        raise SystemExit(f"基线文件不存在：{BASELINE_PATH} ⇒ **没法比**（不许当通过）")
    try:
        data = json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"基线文件读不出来：{BASELINE_PATH}：{error}") from error
    if not isinstance(data, dict) or not isinstance(data.get("corpora"), list):
        raise SystemExit(f"基线文件格式不对（应为含 corpora 数组的对象）：{BASELINE_PATH}")
    return data


def missing_corpora(baseline: dict[str, Any]) -> list[str]:
    """返回**缺失**的语料文件（空列表 = 都在）。"""
    directory = novels_dir()
    missing: list[str] = []
    for corpus in baseline.get("corpora", []):
        name = corpus.get("name", "?")
        filename = CORPUS_FILES.get(name)
        if filename is None:
            missing.append(f"{name}（**基线里的语料名不认识** ⇒ 没有对应文件可查）")
            continue
        if not (directory / filename).exists():
            missing.append(f"{name}（{directory / filename}）")
    return missing


def run_current_scores() -> dict[str, Any]:
    """跑当前真实分数（Rust 侧跑检索 ⇒ 这里只解析它输出的一行 JSON）。"""
    command = [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "-p",
        "apeireth-memory",
        "--example",
        "golden_baseline_report",
    ]
    try:
        proc = subprocess.run(
            command,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        raise SystemExit(f"跑不了 cargo（{error}）⇒ **没法比**（不许当通过）") from error
    if proc.returncode != 0:
        print(proc.stderr.strip(), file=sys.stderr)
        raise SystemExit(
            f"跑分失败（退出码 {proc.returncode}）⇒ **没法比**（不许当通过）"
        )
    payload = parse_payload(proc.stdout)
    if payload is None:
        raise SystemExit(
            "跑分输出里没有可解析的 JSON ⇒ **没法比**（不许当通过）\n"
            f"stdout: {proc.stdout.strip()}"
        )
    return payload


def parse_payload(stdout: str) -> dict[str, Any] | None:
    """从 stdout 里挑出那**一行 JSON**（忽略 cargo 之类的前后噪声）。"""
    for line in reversed(stdout.splitlines()):
        text = line.strip()
        if not text.startswith("{"):
            continue
        try:
            data = json.loads(text)
        except json.JSONDecodeError:
            continue
        if isinstance(data, dict) and isinstance(data.get("corpora"), list):
            return data
    return None


def compare(baseline: dict[str, Any], current: dict[str, Any]) -> list[str]:
    """比一遍 ⇒ 返回**问题清单**（空 = 全绿）。"""
    problems: list[str] = []

    baseline_k = baseline.get("k")
    current_k = current.get("k")
    if baseline_k != current_k:
        problems.append(
            f"评测口径的 k 不一致（基线 {baseline_k} ⇒ 当前 {current_k}）⇒ **不可比**"
        )

    baseline_corpora = {item.get("name"): item for item in baseline.get("corpora", [])}
    current_corpora = {item.get("name"): item for item in current.get("corpora", [])}

    for name, item in baseline_corpora.items():
        if name not in current_corpora:
            problems.append(f"语料「{name}」这轮**没跑到** ⇒ **不可比**（不许当没掉）")
            continue
        got = current_corpora[name]
        base_cases = item.get("cases")
        got_cases = got.get("cases")
        if base_cases != got_cases:
            problems.append(
                f"语料「{name}」题数不同（基线 {base_cases} 题 ⇒ 当前 {got_cases} 题）"
                "⇒ **不可比** ⇒ 不判掉"
            )
            continue
        for label in ("recall_at_k", "ndcg_at_k"):
            base_value = float(item.get(label, 0.0))
            got_value = float(got.get(label, 0.0))
            allowed = base_value * TOLERANCE_RATIO + TOLERANCE_FLOOR
            drop = base_value - got_value
            if drop > allowed:
                relative = (drop / base_value * 100.0) if base_value > 0 else 0.0
                problems.append(
                    f"语料「{name}」的 {label} **掉了** {drop:.6}"
                    f"（基线 {base_value:.6} → 当前 {got_value:.6}；"
                    f"相对劣化 {relative:.2}%；允许上限 {allowed:.6}）"
                )

    for name in current_corpora:
        if name not in baseline_corpora:
            problems.append(f"语料「{name}」**不在基线里** ⇒ **不可比**（不许当没掉）")

    return problems


def main() -> int:
    print("=" * 72)
    print("内部卷检索基线闸门：准确度不许掉（相对劣化 > 1% ⇒ 拦下）")
    print("=" * 72)

    baseline = load_baseline()
    print(
        f"· 基线：{BASELINE_PATH}（版本 {baseline.get('version')} · k={baseline.get('k')}）"
    )

    missing = missing_corpora(baseline)
    if missing:
        # ⚠️ **如实跳过**：跳过**不是通过** ⇒ 打印里必须写明。
        print(f"· 语料目录：{novels_dir()}")
        for item in missing:
            print(f"· 语料不在：{item}")
        print("跳过（缺语料）—— **这不是通过**（本轮没有可比分数）")
        return 0

    current = run_current_scores()
    for item in current.get("corpora", []):
        print(
            f"· 当前：{item.get('name')} · {item.get('cases')} 题 · "
            f"recall@{current.get('k')}={float(item.get('recall_at_k', 0.0)):.4f} · "
            f"nDCG@{current.get('k')}={float(item.get('ndcg_at_k', 0.0)):.4f}"
        )

    # ---------- 观察项：**送达率**（**不参与闸门**，但**每次都要看见**）----------
    #
    # ⚠️ **为什么**不参与闸门：注入族指标（`groundedness` / `evidence_precision` /
    # `delivery_rate`）测的是「**注入给模型什么**」，与「**检索准不准**」不是一件事
    # ⇒ 混进同一个闸门 ⇒ 「为了过闸而调注入」会**扭曲**检索优化。
    #
    # ⚠️ **为什么**要打印：L2 的恶化会**静默**发生
    # ⇒ 实测（2026-10-27）：《紅樓夢》**送达率 0.31** ⇒ **69% 的正确答案
    # 「检索到了却没送到模型面前」** ⇒ 这个事实**必须**每次跑批都露面。
    print("-" * 72)
    print("观察项（**不参与闸门**）：证据送达率")
    baseline_corpora = {item.get("name"): item for item in baseline.get("corpora", [])}
    for item in current.get("corpora", []):
        name = item.get("name")
        current_rate = item.get("delivery_rate")
        base_rate = baseline_corpora.get(name, {}).get("delivery_rate")
        if current_rate is None:
            print(f"  · {name}：送达率 **没测到**")
            continue
        if base_rate is None:
            print(f"  · {name}：送达率 {float(current_rate):.4f}（基线里没有 ⇒ 只报当前值）")
            continue
        delta = float(current_rate) - float(base_rate)
        print(
            f"  · {name}：送达率 {float(current_rate):.4f}"
            f"（基线 {float(base_rate):.4f} · Δ {delta:+.4f}）"
            f" · groundedness {float(item.get('groundedness', 0.0)):.4f}"
        )

    problems = compare(baseline, current)
    print("-" * 72)
    if problems:
        print("检索分数**未达标**：", file=sys.stderr)
        for problem in problems:
            print(f"  ✗ {problem}", file=sys.stderr)
        return 1
    print("OK: 检索分数未低于基线")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
