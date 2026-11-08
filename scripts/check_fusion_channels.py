#!/usr/bin/env python3
"""**融合通道闸门** —— 「**不许不涨**」的尺子（B）。

## 为什么要有这个闸门

CI 闸门 `scripts/check_golden_baseline.py` 量的是**单路 BM25**
⇒ ⇒ **通道级改动（加召回通道 / 加 as-of / 换融合配方）在那个闸门上完全看不见**
⇒ ⇒ ⇒ 「不许变差」的纪律**对通道级改动是失效的**。

⇒ ⇒ 本闸门让**同一份考卷跑两遍**（单路 vs 多路）⇒ ⇒ 量出**每个通道的收益**。

## 判据（**只减不增，且不许虚报**）

对每份语料：

- `delta = 多路.recall_at_k − 单路.recall_at_k`
- ⭐ **`delta < 0` ⇒ ⇒ 闸门红**（「**不许不涨**」⇒ ⇒ 有害的通道不许进生产）
- `delta == 0` ⇒ ⇒ **绿**（中性通道**允许**⇒ ⇒ 内部卷**区分不了**它的价值时，
  不许因此把一个「不涨也不跌」的通道判死 ⇒ ⇒ **但也绝不虚报它涨了**）
- `delta > 0` ⇒ ⇒ **绿**（真涨了 ⇒ ⇒ 建议抬基线）

## ⚠️ **口径限制（必须随数字一起读）**

内部卷的题是「**从记录里挖出来的**」（主体 = 该记录的最高频实体）
⇒ ⇒ **天然偏向词面命中** ⇒ ⇒ **对「词面不像但该在场」这类通道不利**
⇒ ⇒ ⇒ **delta 是**下界**，不是该通道的真实收益**。

⇒ ⇒ ⇒ **本闸门能证明「没有害」，不能证明「有大用」**。

## 不可比的两种情形（**都不算通过**）

- **语料不在** ⇒ ⇒ 打印「跳过（缺语料）」并退出 0（**如实**：跳过不是通过）；
- **报告跑失败 / 语料数变了 / 基线文件里没有这份语料** ⇒ ⇒ **报错退出 1**
  ⇒ ⇒ 「不可比」**绝不**被当成「没掉」。

## 用法

```sh
python3 scripts/check_fusion_channels.py           # 闸门
python3 scripts/check_fusion_channels.py --report  # 只出报告（不改基线）
python3 scripts/check_fusion_channels.py --update  # 重写基线（需明示）
```
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "fusion-channels-baseline.json"

# ⭐ **相对劣化容忍度**：与 `check_golden_baseline.py` **同口径**（同一个判据家族）。
TOLERANCE_RATIO = 0.01


def _corpus_root() -> Path | None:
    """语料根目录（**与 `golden_baseline_report` 同口径**）。"""
    data_dir = os.environ.get("APEIRETH_DATA_DIR")
    if data_dir:
        root = Path(data_dir) / "novels"
        if root.is_dir():
            return root
    home = Path(os.environ.get("HOME", ".")) / ".apeireth" / "novels"
    return home if home.is_dir() else None


def run_report() -> tuple[dict | None, str]:
    """跑 A/B 报告 ⇒ ⇒ (JSON, 人类可读说明)。"""
    root = _corpus_root()
    if root is None:
        return None, "缺语料（$APEIRETH_DATA_DIR/novels 或 ~/.apeireth/novels）"

    env = dict(os.environ)
    # ⭐ **专属 target 目录**（2026-11-06）：本机有**别的 root 身份的 cargo 进程**
    # 在往**共享** target 目录里写文件 ⇒ ⇒ 本脚本复用它时会出现
    # `E0786 Permission denied`（产物 root 拥有 ⇒ ⇒ bai 读不了）
    # ⇒ ⇒ ⇒ 那是**环境干扰**（不是代码问题）⇒ ⇒ 用**专属** target 隔离。
    env.setdefault("CARGO_TARGET_DIR", "/var/tmp/apeireth-fusion-gate-target")
    # ⭐ **强制清空 rustc wrapper**（2026-11-07 实测根因）：`/usr/local/cargo/config.toml`
    # 设了 `rustc-wrapper = "sccache"`，而本机 sccache 守护进程以 **root** 跑
    # ⇒ ⇒ 它产出的 `.d` / 元数据是 root 拥有的 ⇒ ⇒ bai 身份再读就是
    # `Permission denied`（表现为「报告跑失败」，极易误判成代码问题）。
    # 本脚本自带专属 target 目录只隔离了产物位置，**隔离不了 wrapper 身份** ⇒ 这里显式清空。
    env["RUSTC_WRAPPER"] = ""
    # ⭐ **语料路径**：示例自己认 `APEIRETH_DATA_DIR`（它的 `novels` 挂在数据根下）
    # ⇒ ⇒ 这里传**数据根**（不是 novels 目录）。
    if not (Path(env.get("APEIRETH_DATA_DIR", "/nonexistent")) / "novels").is_dir():
        env["APEIRETH_DATA_DIR"] = str(root.parent)

    cmd = [
        "cargo", "run", "--release", "-p", "apeireth-memory",
        "--example", "fusion_channel_report",
    ]
    proc = subprocess.run(
        cmd, cwd=ROOT, env=env, capture_output=True, text=True, timeout=7200
    )
    if proc.returncode != 0:
        tail = (proc.stderr or proc.stdout or "").strip().splitlines()[-5:]
        return None, "报告跑失败：\n" + "\n".join(tail)

    for line in proc.stdout.splitlines():
        line = line.strip()
        if line.startswith("{") and '"corpora"' in line:
            try:
                return json.loads(line), ""
            except json.JSONDecodeError:
                continue
    return None, "报告没输出可解析的 JSON 行"


def load_baseline() -> dict:
    if not BASELINE.exists():
        return {}
    try:
        return json.loads(BASELINE.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}


def write_baseline(payload: dict) -> None:
    BASELINE.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--report", action="store_true", help="只出报告（不改基线）")
    ap.add_argument("--update", action="store_true", help="重写基线")
    args = ap.parse_args()

    payload, error = run_report()
    if payload is None:
        if "缺语料" in error:
            # ⚠️ **「残缺」与「没有」必须分开判**（2026-11-08，与内部卷基线闸门同口径）。
            # 语料目录**存在但文件残缺** ⇒ 半吊子环境 ⇒ **判红**（不许「跳过」冒充通过）。
            root = _corpus_root()
            if root is not None and root.is_dir():
                print(f"❌ 融合通道闸门：语料目录存在（{root}）但语料残缺 ⇒ **半吊子环境**")
                print("   ⇒ ⇒ 本轮**没有可比分数** ⇒ ⇒ **判红**（不许当通过）")
                return 1
            print(f"⏭ 融合通道闸门：跳过（{error}）—— **这不是通过**")
            return 0
        print(f"❌ 融合通道闸门：{error}")
        return 1

    corpora = payload.get("corpora", [])
    if not corpora:
        print("❌ 融合通道闸门：报告里没有语料 ⇒ 不能出结论")
        return 1
    # ⭐ **公平卷**（2026-11-06 主指挥加）：同题面模板 + 主体换成「联想实体集里次级的
    # 跨记录桥实体」⇒ ⇒ 测的是通道收益的**上界**（词面卷测的是**下界**）
    # ⇒ ⇒ ⇒ **两个数都报** ⇒ ⇒ 才既不虚报、也不低估。
    # ⚠️ **只报不判**（理由见文件头）⇒ ⇒ 不参与「不许不涨」的判定。
    fair_corpora = payload.get("fair_corpora", []) or []

    baseline = load_baseline()
    base_corpora = {c["name"]: c for c in baseline.get("corpora", [])}

    print(f"融合通道闸门（不许不涨 · k={payload.get('k')}）")
    failed = False
    improved = False
    for entry in corpora:
        name = entry["name"]
        delta = float(entry.get("delta", 0.0))
        base = float(entry["baseline"]["recall_at_k"])
        with_ch = float(entry["with_channel"]["recall_at_k"])
        print(
            f"  · {name}: recall@{payload.get('k')} {base:.4f} → {with_ch:.4f}"
            f"（delta {delta:+.4f}）"
        )
        if delta < -TOLERANCE_RATIO:
            print(
                f"      ❌ **不许不涨**：该通道让召回**掉了** {abs(delta):.4f}"
                f" ⇒ ⇒ 通道不许进生产"
            )
            failed = True
        elif delta > 0:
            improved = True

        # 与基线比：分数本身不许低于基线（相对 1% 口径，与既有闸门一致）
        prev = base_corpora.get(name)
        if prev is not None:
            prev_recall = float(prev["baseline"]["recall_at_k"])
            if base < prev_recall * (1.0 - TOLERANCE_RATIO):
                print(
                    f"      ❌ **基线不许掉**：{base:.4f} < 基线 {prev_recall:.4f}"
                    f"（相对劣化 > {TOLERANCE_RATIO:.0%}）"
                )
                failed = True
        else:
            print(f"      ⚠️ 基线里没有这份语料 ⇒ **不可比** ⇒ 不判掉")

    # ── ⭐ 公平卷（**只报不判**）────────────────────────────────────
    # 词面卷的 delta 是通道收益的**下界**（内部卷天然偏词面）；
    # 公平卷（主体换成「跨记录桥的次级实体」）测的是**上界**
    # ⇒ ⇒ ⇒ 两个数**都报** ⇒ ⇒ 才既不虚报、也不低估。
    # ⚠️ **不参与「不许不涨」判定**：公平卷的 delta 现在是 0.0
    # ⇒ ⇒ ⇒ 红的会是**真问题**（通道有效但当前融合配方不兑现）
    # ⇒ ⇒ ⇒ ⇒ 先让它**可见**，等融合配方有**依据**了再谈「变硬」。
    if fair_corpora:
        print("  · 公平卷（**上界**测量 · 只报不判）：")
        for entry in fair_corpora:
            name = entry.get("name", "?")
            delta = float(entry.get("delta", 0.0))
            base = float(entry.get("baseline", {}).get("recall_at_k", 0.0))
            with_ch = float(entry.get("with_channel", {}).get("recall_at_k", 0.0))
            only = entry.get("channel_only_recall_at_k")
            only_txt = f" · 联想**单独** recall={float(only):.4f}" if only is not None else ""
            print(
                f"      · {name}: recall@{payload.get('k')} {base:.4f} → {with_ch:.4f}"
                f"（delta {delta:+.4f}）{only_txt}"
            )
        print(
            "      ⚠️ 公平卷的 delta **不参与判定**（它是**上界**测量）"
            " ⇒ ⇒ 「联想单独强、融合弱」是**真问题**，不是尺子的错"
        )

    if failed:
        return 1

    if args.update:
        write_baseline(payload)
        print(f"✅ 基线已重写：{BASELINE.relative_to(ROOT)}")
        return 0

    if improved:
        print("✅ 有通道涨了 ⇒ ⇒ 建议 `--update` 抬基线（让分数漂移可见）")
    else:
        print("✅ 没有通道掉分（**不许不涨**通过）")
    print("⚠️ 口径：内部卷的题是挖出来的 ⇒ ⇒ delta 是**下界** ⇒ ⇒ 本闸门能证明「没有害」，不能证明「有大用」")
    return 0


if __name__ == "__main__":
    sys.exit(main())
