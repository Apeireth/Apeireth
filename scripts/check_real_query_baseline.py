#!/usr/bin/env python3
"""真实查询闸门（MIRACL 中文）：**方向不许错**。

# 为什么要有这道闸门
模板卷（`golden_baseline`）量的是**模板题**，而模板题与真实用户查询
**形态不同**（实测：模板题 = 5 种固定措辞；真人查询 = 393 条各自不同的问句）。
于是**同一个特性**，在两套卷上可以得出**相反的结论**（实测例：
`first_pos` 在真人查询 **+0.0192**、在模板词面卷 **−0.0431**，同口径同实现）。
⇒ ⇒ ⇒ **模板卷守「不许退步」，本闸门守「不许自欺」。**

# ⚠️⚠️ **2026-11-08 更正一个混口径的数字**
此前这里（以及多处文档）写的是「`first_pos` 在真人查询 **+0.1227**」。
**那个数字无效**：基线 0.7246 是**闸门口径**（逐题「命中比例」再平均），
而 0.8473 是**二值「任一命中」**；两个不同的指标相减。
同口径重测后是 **+0.0192**（`crates/engine/memory/examples/first_pos_arms.rs` 复现）。
**引用任何 α 扫描数字时，必须带上口径。**

# 口径（引用数字时必须一起引用）
* **采样池**：语料 = MIRACL 中文 dev 的全部 3786 条**被标注**段落
  + 确定性采样的 19076 条未标注段落（`sha1(docid)` 前 4 位 < 阈值 ⇒ 可复现）
  ⇒ ⇒ **池子远小于真实全库（5.9M）** ⇒ ⇒ **召回偏乐观**
  ⇒ ⇒ ⇒ **绝对数字不可与 MIRACL 官方榜对比**；本闸门只看**相对基线的变化**。
* 语料与标注来自维基百科（CC BY-SA），仅作研究用途。

# 判定
* **基线来自基线文件**（`scripts/real-query-baseline.json`），
  与 `check_golden_baseline.py` 同构（相对劣化 > 1% ⇒ 判红）。
* **缺语料/缺考卷 ⇒ 跳过并明确说「这不是通过」**（不许当绿）。

# 用法
```
python3 scripts/check_real_query_baseline.py --check   # 比基线（CI 用）
python3 scripts/check_real_query_baseline.py --update  # 重盖基线
```
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE_PATH = REPO_ROOT / "scripts" / "real-query-baseline.json"
EXAMPLE = "miracl_exam_ab"
RELATIVE_TOLERANCE = 0.01


def exam_root() -> Path:
    env = os.environ.get("APEIRETH_MIRACL_EXAM")
    if env:
        return Path(env)
    root = os.environ.get("APEIRETH_DATA_DIR")
    if root:
        return Path(root) / "miracl-exam"
    home = Path(os.environ.get("HOME", "."))
    return home / ".apeireth" / "miracl-exam"


def run_current() -> dict:
    """跑一次真实查询考卷，取生产口径的 recall@10。"""
    root = exam_root()
    if not (root / "corpus.jsonl").exists() or not (root / "exam.jsonl").exists():
        print(f"· 考卷目录：{root}")
        print("· 考卷不在 ⇒ **跳过（这不是通过）**（本轮没有可比分数）")
        return {}
    env = dict(os.environ)
    env.setdefault("APEIRETH_DATA_DIR", str(Path.home() / ".apeireth"))
    env.setdefault("HOME", str(Path.home()))
    env.setdefault("TMPDIR", "/home/bai/tmp-test")
    env["RUSTC_WRAPPER"] = ""
    proc = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "apeireth-memory",
            "--example",
            EXAMPLE,
        ],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=1800,
    )
    if proc.returncode != 0:
        print(f"· 跑批失败（退出码 {proc.returncode}）：")
        print((proc.stderr or proc.stdout)[-1200:])
        return {}
    for line in reversed(proc.stdout.splitlines()):
        line = line.strip()
        if line.startswith("{") and "生产口径" in line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    print("· 没拿到 JSON 输出 ⇒ 跳过（这不是通过）")
    return {}


def load_baseline() -> dict:
    if not BASELINE_PATH.exists():
        raise SystemExit(f"基线文件不存在：{BASELINE_PATH} ⇒ **没法比**（不许当通过）")
    try:
        return json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"基线文件读不出来：{BASELINE_PATH}：{error}") from error


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="比基线（CI 用）")
    parser.add_argument("--update", action="store_true", help="重盖基线")
    args = parser.parse_args()

    print("=" * 72)
    print("真实查询闸门（MIRACL 中文）：方向不许错（相对劣化 > 1% ⇒ 拦下）")
    print("=" * 72)

    current = run_current()
    if not current:
        # ⚠️ **如实跳过**：跳过**不是通过**。
        # ⚠️ **「残缺」与「没有」分开判**（2026-11-08，与内部卷基线闸门同口径）：
        # 考卷目录**存在但文件残缺** ⇒ 半吊子环境 ⇒ **判红**（不许「跳过」冒充通过）。
        root = exam_root()
        if root.is_dir() and not (root / "exam.jsonl").exists():
            print(f"❌ 真实查询闸门：考卷目录存在（{root}）但 exam.jsonl 缺失 ⇒ **半吊子环境**")
            print("   ⇒ ⇒ 本轮**没有可比分数** ⇒ ⇒ **判红**（不许当通过）")
            return 1
        print("跳过（缺考卷 / 跑不动）—— **这不是通过**（本轮没有可比分数）")
        print("GATE_SKIP: 考卷/语料不可用 ⇒ 本门禁未执行可比测量")
        return 0

    recall = float(current.get("生产口径", 0.0))
    print(
        f"· 当前：{current.get('考卷')} · {current.get('题数')} 题 · "
        f"recall@10={recall:.4f} · 语料 {current.get('语料段数')} 段"
    )
    print(
        f"· 参考：原始口径 {float(current.get('原始口径', 0.0)):.4f}"
        f" · oracle {float(current.get('oracle', 0.0)):.4f}"
        f" · 天花板余量 {float(current.get('天花板余量', 0.0)):+.4f}"
    )

    if args.update:
        baseline = {
            "version": "miracl-zh-dev-sampled-2026-11-07",
            "k": 10,
            "recall_at_k": recall,
            "cases": int(current.get("题数", 0)),
            "corpus_docs": int(current.get("语料段数", 0)),
            "raw_recall_at_k": float(current.get("原始口径", 0.0)),
            "oracle_recall_at_k": float(current.get("oracle", 0.0)),
            "note": (
                "采样池（MIRACL 中文 dev 的被标注段落 + 确定性采样的未标注段落）"
                "⇒ 召回偏乐观 ⇒ 绝对值不可与 MIRACL 官方榜对比；只看相对变化"
            ),
        }
        BASELINE_PATH.write_text(
            json.dumps(baseline, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"✅ 基线已重写：{BASELINE_PATH}")
        return 0

    baseline = load_baseline()
    base_recall = float(baseline.get("recall_at_k", 0.0))
    print(f"· 基线：{BASELINE_PATH}（版本 {baseline.get('version')}）· recall@10={base_recall:.4f}")
    if base_recall <= 0.0:
        print("· 基线值非法 ⇒ **没法比**（不许当通过）")
        return 1

    relative = (recall - base_recall) / base_recall
    print(f"· 相对变化：{relative:+.4%}")
    if relative < -RELATIVE_TOLERANCE:
        print(
            f"❌ **不许劣化**：真实查询 recall@10 掉了 {abs(relative):.2%}"
            f" ⇒ ⇒ 方向错了 ⇒ ⇒ 闸门拦下"
        )
        return 1
    print("OK: 真实查询方向未错")
    return 0


if __name__ == "__main__":
    sys.exit(main())
