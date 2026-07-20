"""Questioning Engine v0.1 — 提问引擎 (Phase 5 / TOP-DESIGN §4.4)

依据文献 (主人 14:27 '聚集全人类智慧'):
- Pep (2602.15012) — offline priors + online Bayesian, 3-5x 更少交互达 80.8% 对齐
- Funnel Question (2510.12015) — 由宽到窄 funnel, 每答一题缩窄下一步搜索空间
- Mom Test — 问题不是为证明自己聪明, 是为学主人没说的事

主人原话:
- 12:27 "LLM 不断向主人提问就行"
- 12:54 "启动后, 会自动触发几个预设的关键问题"
- 13:04 "造地基不能有杂质" (Q6 关系契约) "没硬性红线" (Q7 边界)

设计原则 (PoC v0.1):
- offline priors 从 IdentityCard.funnel_questions 灌入 (Q8 答的)
- online Bayesian update (Pep 范式): posterior = α·prior + β·observed
- ask_next() 选当前 uncertainty 最高的 (最低 prior + 未答)
- 8 + N 个问题, 不强行问完 — 主人答烦了随时叫停
- 不依赖 LLM — answer 来自 priors 注入 / stdin / 主人 / 后续 L1 Kernel

与已存在组件的关系:
- 输入: IdentityCard.funnel_questions (Phase 1)
- 触发: Reconsolidation.flag (Phase 2) 可 push 问题到 funnel
- 输出: Answer.evidence_refs 接到 Episode / Note (Phase 2) nid
- 联动: PersonaEngine.coordinate() 用 question.topic 选 persona (Phase 4)
"""

from __future__ import annotations
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


QUESTIONING_VERSION = "0.1.0"

# Bayesian 混合权重 — Pep 范式
# posterior = ALPHA * prior + BETA * observed_confidence
# ALPHA 大 = 偏信 priors (适合用户初始)
# BETA 大 = 偏信用户答 (适合后期用户信任建立)
ALPHA = 0.4   # prior 权重
BETA = 0.6    # observed 权重


@dataclass
class Question:
    """一个问题 — 8 字段 + 状态.

    source 解释:
    - 'offline_prior'  : 从 IdentityCard.funnel_questions 加载 (主人预设)
    - 'reconsolidation': Reconsolidation 触发 (例如 Note.conflict)
    - 'gap_inference'  : IdentityCard 字段空 → 自动衍生 (mission/domains/boundaries)
    - 'manual'         : 主人或 cron 主动加
    """
    qid: str
    prompt: str
    topic: str = ""                # e.g. "asi_progress" / "ethics" / "memory"
    prior: float = 0.5             # 0-1, priors 置信度 (1=主人已说清楚)
    source: str = "manual"         # 来源分类
    when_to_ask: str = "anytime"   # "anytime" / "weekly" / "when_stuck" / "never"
    rationale: str = ""            # 为什么问 (留给 future self)
    created_at: float = field(default_factory=time.time)


@dataclass
class Answer:
    """一次回答 — Bayesian update 用."""
    qid: str
    answer_text: str
    observed: float = 0.7          # 主人对这次回答的确信度 (0-1, 默认 0.7)
    evidence_refs: list = field(default_factory=list)  # 关联 Episode / Note nid
    asked_at: float = field(default_factory=time.time)
    answered_at: float = 0.0


@dataclass
class FunnelState:
    """单个问题的 funnel 状态 — 给 summary 用."""
    qid: str
    prompt: str
    topic: str
    prior: float
    posterior: float
    source: str
    n_answers: int
    last_answered_at: float
    status: str                    # "pending" / "answered" / "skipped"


class BayesianFunnel:
    """Funnel question engine — Pep + Funnel Question 范式.

    核心流程:
    1. seed_from_identity(card) — 灌入 Q8 priors
    2. add_question(q) — 加问题 (manual / reconsolidation / gap)
    3. ask_next() — 选下一个 (最低 prior × 未答)
    4. record_answer(a) — Bayesian update
    5. summary() — 看 funnel 状态
    """

    def __init__(self):
        self.questions: dict[str, Question] = {}
        self.answers: list[Answer] = []
        self._posterior: dict[str, float] = {}   # qid → posterior

    # ── seed ──────────────────────────────────────────────────────

    def seed_from_identity(self, card) -> int:
        """从 IdentityCard.funnel_questions 灌入 offline priors.

        Returns: how many questions added.
        """
        added = 0
        for prompt in card.funnel_questions or []:
            q = Question(
                qid=f"q_{uuid.uuid4().hex[:8]}",
                prompt=prompt,
                topic=self._infer_topic(prompt),
                prior=0.3,                 # 主人说"以后不断问" → low prior = 高 uncertainty
                source="offline_prior",
                when_to_ask="anytime",
                rationale="master card Q8 — 主人预设 funnel trigger",
            )
            self.questions[q.qid] = q
            self._posterior[q.qid] = q.prior
            added += 1
        return added

    def seed_gap_questions(self, card) -> int:
        """从 IdentityCard 字段空 → 衍生 gap 问题.

        依据: Master 12:54 "启动后, 会自动触发几个预设的关键问题"
        当前 master 卡空字段: mission / domains / boundaries / alias / creator
        """
        gap_prompts = []
        if not card.mission:
            gap_prompts.append(("mission", "你最重要的一个长期使命是什么? (mission 字段空)"))
        if not card.domains:
            gap_prompts.append(("domains", "你想最深入哪些领域? (domains 空)"))
        if not card.boundaries:
            gap_prompts.append(("boundaries", "我有什么不能做的? (boundaries 空)"))
        if not card.alias:
            gap_prompts.append(("alias", "我有别的称呼吗? (alias 空)"))
        if not card.creator:
            gap_prompts.append(("creator", "谁创造了我? (creator 空)"))

        added = 0
        for topic, prompt in gap_prompts:
            q = Question(
                qid=f"q_{uuid.uuid4().hex[:8]}",
                prompt=prompt,
                topic=topic,
                prior=0.1,                 # 空字段 → 极高 uncertainty
                source="gap_inference",
                when_to_ask="anytime",
                rationale="IdentityCard 字段空 → 自动衍生 (Mom Test 不强求)",
            )
            self.questions[q.qid] = q
            self._posterior[q.qid] = q.prior
            added += 1
        return added

    # ── ask ───────────────────────────────────────────────────────

    def add_question(self, q: Question) -> None:
        self.questions[q.qid] = q
        self._posterior.setdefault(q.qid, q.prior)

    def ask_next(self) -> Optional[Question]:
        """选 posterior 最低 (uncertainty 最高) 的未答问题 — Funnel Question 范式.

        Returns None = 所有问题都答过了.
        """
        answered = {a.qid for a in self.answers}
        pending = [
            (qid, self._posterior.get(qid, q.prior))
            for qid, q in self.questions.items()
            if qid not in answered and q.when_to_ask != "never"
        ]
        if not pending:
            return None
        pending.sort(key=lambda x: x[1])  # 最低 posterior = 最高信息增益
        return self.questions[pending[0][0]]

    # ── answer ────────────────────────────────────────────────────

    def record_answer(self, a: Answer) -> float:
        """Bayesian update — posterior = α·prior + β·observed.

        Returns: 新 posterior.
        """
        if a.qid not in self.questions:
            raise KeyError(f"unknown qid: {a.qid}")
        prior = self._posterior.get(a.qid, 0.5)
        posterior = ALPHA * prior + BETA * a.observed
        self._posterior[a.qid] = round(posterior, 4)
        a.answered_at = time.time()
        self.answers.append(a)
        return self._posterior[a.qid]

    # ── summary ───────────────────────────────────────────────────

    def summary(self) -> list[FunnelState]:
        """所有问题的 funnel 状态 — 给主人 / persona 看."""
        answered = {a.qid: a for a in self.answers}
        out = []
        for qid, q in self.questions.items():
            n = sum(1 for a in self.answers if a.qid == qid)
            last = max((a.answered_at for a in self.answers if a.qid == qid), default=0.0)
            status = "answered" if qid in answered else ("skipped" if q.when_to_ask == "never" else "pending")
            out.append(FunnelState(
                qid=qid,
                prompt=q.prompt,
                topic=q.topic,
                prior=q.prior,
                posterior=self._posterior.get(qid, q.prior),
                source=q.source,
                n_answers=n,
                last_answered_at=last,
                status=status,
            ))
        return out

    def integrity_hash(self) -> str:
        """SHA256 前 16 — 防偷偷改 funnel (PersistBench 97% sycophancy 风险)."""
        import hashlib, json
        payload = {
            "questions": sorted(
                ({"qid": q.qid, "prompt": q.prompt, "prior": q.prior}
                 for q in self.questions.values()),
                key=lambda d: d["qid"],
            ),
            "posterior": dict(sorted(self._posterior.items())),
            "answers": sorted(
                ({"qid": a.qid, "observed": a.observed, "text": a.answer_text}
                 for a in self.answers),
                key=lambda d: d["qid"],
            ),
            "version": QUESTIONING_VERSION,
        }
        canon = json.dumps(payload, sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]

    # ── helpers ───────────────────────────────────────────────────

    @staticmethod
    def _infer_topic(prompt: str) -> str:
        """PoC 关键词启发式 — 真接 LLM 后改 Bayesian (Pep 范式)."""
        kw = {
            "asi": "asi_progress",
            "涌现": "emergence",
            "阻碍": "blocker",
            "边界": "ethics",
            "伦理": "ethics",
            "记忆": "memory",
            "关系": "relationship",
            "身份": "identity",
            "价值": "value",
            "领域": "domain",
            "使命": "mission",
        }
        for k, t in kw.items():
            if k in prompt.lower():
                return t
        return "general"


__all__ = [
    "QUESTIONING_VERSION", "ALPHA", "BETA",
    "Question", "Answer", "FunnelState", "BayesianFunnel",
]