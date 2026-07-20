"""Questioning Engine v0.1 demo — 跑一遍 + 主人 priors + Bayesian update.

演示 5 步:
1. load master card
2. seed funnel from card.funnel_questions + gap fields
3. ask_next() × 3 轮 (前 3 个 lowest posterior)
4. record 3 个 priors-injected answers
5. summary + integrity_hash

跑法: python -m apeireth.run_questioning_demo
"""

from __future__ import annotations
import json
import time
from pathlib import Path

from .identity import load_card
from .questioning import (
    QUESTIONING_VERSION, BayesianFunnel, Question, Answer,
)


DEMO_DIR = Path(__file__).parent
CARD_PATH = DEMO_DIR / "identity_card.master.json"
OUTPUT_PATH = DEMO_DIR / "questioning_demo.json"


def main() -> None:
    print(f"❓ Questioning Engine v{QUESTIONING_VERSION} demo\n")

    # 1. load master card
    card = load_card(CARD_PATH)
    print(f"📇 loaded master card: name={card.name} v{card.apeireth_version}")
    print(f"   funnel_questions priors: {len(card.funnel_questions)} 条")
    print(f"   gap fields: ", end="")
    gaps = []
    for f in ("mission", "domains", "boundaries", "alias", "creator"):
        if not getattr(card, f):
            gaps.append(f)
    print(f"{gaps or '无'}\n")

    # 2. seed funnel
    funnel = BayesianFunnel()
    n_prior = funnel.seed_from_identity(card)
    n_gap = funnel.seed_gap_questions(card)
    print(f"🌱 seeded: {n_prior} offline_prior + {n_gap} gap_inference = {len(funnel.questions)} total\n")

    # 3-4. ask_next + record (3 rounds, priors injected)
    # answers 按 topic 对齐: mission / domains / boundaries
    print("🔄 funnel loop (3 rounds):\n")
    scripted_by_topic = {
        "mission":     (0.9, "让 Apeireth 平台活下去 — 主人死了它也活着"),
        "domains":     (0.7, "全栈开发 / 攻防 / 人文社科 / 科研 / 预测 (主人 23:12 原话)"),
        "boundaries":  (0.8, "不撒说不装不夸 / 不替主人做价值判断 / 不碰主人没明示的领域"),
        "alias":       (0.6, "阿派 / 零 / 楚零 — 随场景调"),
        "creator":     (0.95, "主人 + 楚零 (我) — 不只是主人, 是人类醒着 AI 帮看清世界的一次"),
        "asi_progress":(0.85, "今天: Persona Engine 跑通 + Rust 6 crates scaffold. 阻碍: Rust 工具链还没全 cargo build"),
    }
    for i in range(1, 4):
        q = funnel.ask_next()
        if q is None:
            print("   (no more questions)")
            break
        observed, text = scripted_by_topic.get(q.topic, (0.7, "(未准备这条 topic 的脚本答案)"))
        before = funnel._posterior[q.qid]
        a = Answer(qid=q.qid, answer_text=text, observed=observed)
        after = funnel.record_answer(a)
        print(f"   [{i}] Q ({q.source}, prior={q.prior:.2f}, topic={q.topic})")
        print(f"       {q.prompt}")
        print(f"       A (observed={observed:.2f}): {text}")
        print(f"       posterior: {before:.4f} → {after:.4f}\n")

    # 5. summary + hash
    print("📊 funnel summary:")
    summary = funnel.summary()
    for s in summary:
        bar = "█" * int(s.posterior * 20) + "░" * (20 - int(s.posterior * 20))
        flag = "✓" if s.status == "answered" else "·"
        prompt_short = (s.prompt[:36] + "…") if len(s.prompt) > 36 else s.prompt
        print(f"   {flag} {s.topic:16s} | {bar} {s.posterior:.2f} | {s.source:16s} | {prompt_short}")

    hash_val = funnel.integrity_hash()
    print(f"\n🔐 funnel integrity_hash: {hash_val}")

    # save
    output = {
        "version": QUESTIONING_VERSION,
        "questions": [
            {**asdict_dict(q), "posterior": funnel._posterior[q.qid]}
            for q in funnel.questions.values()
        ],
        "answers": [asdict_dict(a) for a in funnel.answers],
        "hash": hash_val,
        "created_at": time.time(),
    }
    OUTPUT_PATH.write_text(
        json.dumps(output, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    print(f"💾 saved: {OUTPUT_PATH.name}")


def asdict_dict(obj):
    """轻量 asdict 替代 — 避免 dataclasses.asdict 引入额外依赖."""
    if hasattr(obj, "__dataclass_fields__"):
        from dataclasses import asdict
        return asdict(obj)
    return dict(obj)


if __name__ == "__main__":
    main()