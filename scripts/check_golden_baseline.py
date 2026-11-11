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
python3 scripts/check_golden_baseline.py                 # 比对（默认；不改基线）
python3 scripts/check_golden_baseline.py --check         # 同上（显式写出来）
python3 scripts/check_golden_baseline.py --update \
    --version-note anchored-injection-2026-11-08          # 重盖基线（必须写版本说明）
```

⚠️ **依赖真实语料**：小说文件在**仓外**（`~/.apeireth/novels/`，
下载器 `scripts/download_novels.sh` 带 SHA-256）⇒ 缺失时**如实跳过**
（不是「通过」⇒ 打印明说跳过）。

# ⭐ `--update` 的纪律（2026-11-08 加）

**重盖基线 = 承认「口径变了」** ⇒ ⇒ 所以：

* `--version-note` **必填** ⇒ 口径变更的原因会**写进版本串**（`版本+说明`）
  ⇒ ⇒ 「为什么数字变了」**永远查得到**；
* `--update` **只改基线文件**，**绝不碰容忍度**
  ⇒ ⇒ `TOLERANCE_RATIO` / `TOLERANCE_FLOOR` **只由人改**，脚本改不了；
* ⚠️ **口径变了 ⇒ 送达率/groundedness 会变** ⇒ ⇒ 重盖前**先看证据页**
  （本次：2026-11-08 注入口径对齐生产形态 ⇒ `docs/04-internal/01-handoffs/memory-gate-caliber-alignment-2026-11-08.md`）。

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

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

# ⭐ **控制台可能不是 UTF-8**（Windows：`cp1252` / `cp936`）⇒ ⇒ `print` 会抛
# `UnicodeEncodeError` ⇒ ⇒ 门禁 exit 1 ⇒ ⇒ **把本来通过的 CI job 判成红**
# ⇒ ⇒ 这里在**任何打印之前**把 stdout/stderr 重配成 UTF-8（守卫见 `scripts/utf8_stdout.py`）。
sys.path.insert(0, str(Path(__file__).resolve().parent))
from utf8_stdout import force_utf8_stdout  # noqa: E402

force_utf8_stdout()

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


def run_current_scores(k_spec: str = "10") -> dict[str, Any]:
    """跑当前真实分数（Rust 侧跑检索 ⇒ 这里只解析它输出的一行 JSON）。

    ⚠️ `k_spec` = `"production"` ⇒ 跑**生产臂**（k = 生产默认召回名额）。
    """
    command = [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "-p",
        "apeireth-memory",
        "--example",
        "golden_baseline_report",
        "--",
        "--k",
        k_spec,
    ]
    try:
        proc = subprocess.run(
            command,
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8", errors="replace",
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


def compare_corpora(
    baseline_corpora: dict[str, Any],
    current_corpora: dict[str, Any],
    prefix: str = "",
) -> list[str]:
    """逐语料比检索指标 ⇒ 返回**问题清单**（空 = 全绿）。"""
    problems: list[str] = []

    for name, item in baseline_corpora.items():
        if name not in current_corpora:
            problems.append(f"{prefix}语料「{name}」这轮**没跑到** ⇒ **不可比**（不许当没掉）")
            continue
        got = current_corpora[name]
        base_cases = item.get("cases")
        got_cases = got.get("cases")
        if base_cases != got_cases:
            problems.append(
                f"{prefix}语料「{name}」题数不同（基线 {base_cases} 题 ⇒ 当前 {got_cases} 题）"
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
                    f"{prefix}语料「{name}」的 {label} **掉了** {drop:.6}"
                    f"（基线 {base_value:.6} → 当前 {got_value:.6}；"
                    f"相对劣化 {relative:.2}%；允许上限 {allowed:.6}）"
                )

    for name in current_corpora:
        if name not in baseline_corpora:
            problems.append(
                f"{prefix}语料「{name}」**不在基线里** ⇒ **不可比**（不许当没掉）"
            )

    return problems


def compare(baseline: dict[str, Any], current: dict[str, Any]) -> list[str]:
    """比一遍（历史臂：基线文件里的 k）⇒ 返回**问题清单**（空 = 全绿）。"""
    problems: list[str] = []

    baseline_k = baseline.get("k")
    current_k = current.get("k")
    if baseline_k != current_k:
        problems.append(
            f"评测口径的 k 不一致（基线 {baseline_k} ⇒ 当前 {current_k}）⇒ **不可比**"
        )

    baseline_corpora = {item.get("name"): item for item in baseline.get("corpora", [])}
    current_corpora = {item.get("name"): item for item in current.get("corpora", [])}

    problems.extend(compare_corpora(baseline_corpora, current_corpora))
    return problems


def compare_production_arm(arm: dict[str, Any], current: dict[str, Any]) -> list[str]:
    """比**生产臂**（k = 生产默认召回名额）⇒ 既是回归线，也是**生产口径的棘轮**。

    ⚠️ **为什么生产臂也要有棘轮**（2026-11-10）：生产默认的名额/跨度是**代码里的常量**
    ⇒ 改它**不会**自动惊动任何门禁 ⇒ ⇒ 评测面（k=10）与生产面（k=20）会**各测各的**
    ⇒ ⇒ 两条线的数字**互不可比**。⇒ 这里把「生产臂的 k / 单条跨度」写进基线
    ⇒ ⇒ 改了生产默认而没重盖基线 ⇒ ⇒ **判红**。
    """
    problems: list[str] = []

    if not arm:
        return [
            f"基线里**没有生产臂**（`production_arm`）⇒ ⇒ 生产默认口径**无覆盖**"
            f"（实测 k={current.get('k')} · 跨度 {current.get('span_chars')} 字）"
            " ⇒ ⇒ 需 `--update --version-note …` 显式重盖基线（一次性）"
        ]

    base_k = arm.get("k")
    current_k = current.get("k")
    if base_k != current_k:
        problems.append(
            f"生产臂的 k 不一致（基线 {base_k} ⇒ 当前 {current_k}）⇒ **不可比**"
            " ⇒ ⇒ 生产默认召回名额改了 ⇒ ⇒ 需显式重盖基线"
            "（`--update --version-note …`）"
        )

    base_span = arm.get("span_chars")
    current_span = current.get("span_chars")
    if base_span is not None and base_span != current_span:
        problems.append(
            f"生产臂的单条跨度变了（基线 {base_span} 字 ⇒ 当前 {current_span} 字）"
            " ⇒ ⇒ 生产注入口径改了 ⇒ ⇒ 需显式重盖基线（`--update --version-note …`）"
        )

    base_corpora = {item.get("name"): item for item in arm.get("corpora", [])}
    current_corpora = {item.get("name"): item for item in current.get("corpora", [])}

    problems.extend(compare_corpora(base_corpora, current_corpora, "生产臂："))
    return problems


def _arm_corpora(baseline: dict[str, Any], current: dict[str, Any]) -> list[dict[str, Any]]:
    """按**基线里的语料清单**取当前实测值（保持语料顺序与字段口径一致）。"""
    by_name = {item.get("name"): item for item in current.get("corpora", [])}
    corpora: list[dict[str, Any]] = []
    for item in baseline.get("corpora", []):
        name = item.get("name")
        got = by_name.get(name)
        if got is None:
            continue
        corpora.append(
            {
                "name": name,
                "recall_at_k": got.get("recall_at_k"),
                "ndcg_at_k": got.get("ndcg_at_k"),
                "cases": got.get("cases"),
                "groundedness": got.get("groundedness"),
                "evidence_precision": got.get("evidence_precision"),
                "delivery_rate": got.get("delivery_rate"),
                "case_fingerprint": got.get("case_fingerprint"),
            }
        )
    return corpora


def write_baseline(
    baseline: dict[str, Any],
    current: dict[str, Any],
    production_current: dict[str, Any],
    note: str,
) -> str:
    """用当前实测分数**重盖基线**（版本串追加口径变更说明）⇒ 返回新版本串。

    ⚠️ **只改基线文件** ⇒ **绝不碰容忍度**（`TOLERANCE_*` 只由人改）。
    """
    version = str(baseline.get("version", "")).strip()
    if note:
        version = f"{version}+{note}" if version else note
    corpora = _arm_corpora(baseline, current)
    production_arm = {
        "k": production_current.get("k"),
        "span_chars": production_current.get("span_chars"),
        "corpora": _arm_corpora(baseline, production_current),
    }
    document = {
        "version": version,
        "k": current.get("k"),
        "corpora": corpora,
        "production_arm": production_arm,
    }
    BASELINE_PATH.write_text(
        json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return version


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="check_golden_baseline.py",
        description="内部卷检索基线闸门：准确度不许掉（相对劣化 > 1% ⇒ 拦下）。",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="只比对（默认行为）",
    )
    parser.add_argument(
        "--update",
        action="store_true",
        help="用当前实测分数**重盖基线**（必须给 --version-note）",
    )
    parser.add_argument(
        "--version-note",
        default="",
        help="口径变更说明（--update 时**必填** ⇒ 写进版本串）",
    )
    args = parser.parse_args()

    print("=" * 72)
    print("内部卷检索基线闸门：准确度不许掉（相对劣化 > 1% ⇒ 拦下）")
    print("=" * 72)

    if args.update and not args.version_note.strip():
        print(
            "❌ `--update` 必须给 `--version-note`"
            "（**口径变更的原因必须写进版本串** ⇒ 否则没人知道数字为什么变）",
            file=sys.stderr,
        )
        return 1

    baseline = load_baseline()
    print(
        f"· 基线：{BASELINE_PATH}（版本 {baseline.get('version')} · k={baseline.get('k')}）"
    )

    missing = missing_corpora(baseline)
    if missing:
        print(f"· 语料目录：{novels_dir()}")
        for item in missing:
            print(f"· 语料不在：{item}")
        # ⚠️⚠️ **「残缺」与「没有」必须分开判**（2026-11-08 实测事故）。
        #
        # 事故原样：`gates.sh` 以 root 跑 ⇒ HOME=/root ⇒ 语料兜底路径落成
        # `/root/.apeireth/novels`，而那一份**存在但是残缺/非 UTF-8 的副本**
        # ⇒ ⇒ 脚本按「缺语料」**跳过并返回 0** ⇒ ⇒ **人只看到 ✅**
        # ⇒ ⇒ 而基线是用**对的那份**盖的 ⇒ ⇒ 两边根本不可比。
        #
        # ⇒ 现在的口径：
        #   * 语料目录**根本不存在** ⇒ 环境里就是没有语料（CI 无语料）⇒ **跳过**（退出码 0，如实写明）；
        #   * 语料目录**存在但文件残缺** ⇒ 环境是**半吊子**（下载中断 / 路径指错 / 副本坏了）
        #     ⇒ ⇒ **判红**（退出码 1）：不能比就是不能比，**不许用「跳过」冒充通过**。
        if novels_dir().is_dir():
            print(
                "❌ **语料目录存在但语料残缺** ⇒ 这是**半吊子环境**"
                "（下载中断 / 路径指错 / 副本损坏），**不是「没有语料」**"
            )
            print("   ⇒ ⇒ 本轮**没有可比分数** ⇒ ⇒ **判红**（不许当通过）")
            print(f"   ⇒ 修法：清掉该目录后重跑 `bash scripts/download_novels.sh`，"
                  f"或把 APEIRETH_DATA_DIR 指向**语料完整**的那份")
            return 1
        print("跳过（缺语料）—— **这不是通过**（本轮没有可比分数）")
        print(f"GATE_SKIP: 语料目录不存在（{novels_dir()}）⇒ 本门禁未执行可比测量")
        return 0

    current = run_current_scores(str(baseline.get("k", 10)))
    production_current = run_current_scores("production")
    for item in current.get("corpora", []):
        print(
            f"· 当前：{item.get('name')} · {item.get('cases')} 题 · "
            f"recall@{current.get('k')}={float(item.get('recall_at_k', 0.0)):.4f} · "
            f"nDCG@{current.get('k')}={float(item.get('ndcg_at_k', 0.0)):.4f}"
        )
    for item in production_current.get("corpora", []):
        print(
            f"· 生产臂：{item.get('name')} · {item.get('cases')} 题 · "
            f"recall@{production_current.get('k')}"
            f"={float(item.get('recall_at_k', 0.0)):.4f} · "
            f"nDCG@{production_current.get('k')}"
            f"={float(item.get('ndcg_at_k', 0.0)):.4f}"
            f" · 跨度 {production_current.get('span_chars')} 字"
        )

    # ---------- `--update`：用当前实测分数**重盖基线** ----------
    if args.update:
        version = write_baseline(
            baseline, current, production_current, args.version_note.strip()
        )
        print("-" * 72)
        print(f"✅ **已重盖基线**：{BASELINE_PATH}")
        print(f"   · 新版本串：{version}")
        print(
            "   · ⚠️ **口径变更说明必须同时写进证据页**"
            "（数字变了 ⇒ 要说清「变了什么、为什么」）"
        )
        print(
            "   · ⚠️ **文档里的数字要同步**"
            "（`python3 scripts/check_doc_numbers.py` 守逐字一致）"
        )
        return 0

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

    # ---------- 题集**指纹**比对（**可比性一眼可见**）----------
    #
    # ⚠️ **为什么**要这个（2026-10-27 实测踩到）：
    # 「语料减半 ⇒ recall 涨」其实是「**题集变简单了**」⇒ 结论被自己的下一个实验推翻。
    # ⇒ ⇒ **换语料 / 换题 ⇒ 数字不可比** ⇒ ⇒ 把**题集指纹**写进基线
    # ⇒ ⇒ 指纹变了 ⇒ **当场提示不可比**（而不是让人误读成「机制变了」）。
    baseline_corpora = {item.get("name"): item for item in baseline.get("corpora", [])}
    for item in current.get("corpora", []):
        name = item.get("name")
        base_fp = baseline_corpora.get(name, {}).get("case_fingerprint")
        got_fp = item.get("case_fingerprint")
        if not base_fp or not got_fp:
            continue
        if base_fp != got_fp:
            print(f"  ⚠️ {name}：**题集指纹变了** ⇒ **数字不可比**（换题了）")
            print(f"     基线 {base_fp[:16]}… → 当前 {str(got_fp)[:16]}…")

    problems = compare(baseline, current)
    problems += compare_production_arm(
        baseline.get("production_arm", {}), production_current
    )
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
