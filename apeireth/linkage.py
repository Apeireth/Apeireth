"""Linkage Layer v0.1 — 联动层 (Phase 5.5 / TOP-DESIGN §3.2 + §4.4)

把已完成的 Phase 2 / 4 / 5 模块串成闭环 — 中央 AI 多身份浮现的真实运转.

设计: 三条衔接路径
  A. Reconsolidation.flag  → Funnel.add_question
       主人说"不要提 X" → 触发 funnel 问题问"为什么" → 形成不提及的真实理由
  B. Funnel.ask_next()     → Persona.coordinate(q.topic)
       问问题前先让 2 个 persona 浮现 → 多视角 → 答案不单一
  C. Persona.adapt(fb)     → Funnel.record_answer
       主人对回答满意/不满意 → Bayesian update + persona 调权

主人原话 (12:14 / 12:54 / 13:04):
- "中央 AI 是永恒身份, 但不是调度者或思考者, 像人是一切社会关系的总和"
- "中央 AI 多身份浮现 (调度者 / 学习者 / 思考者 / 助手)"
- "AI 不会中庸, 因为他会成长"

不依赖 LLM — 这是 L4 Identity Layer 内部组件的连接器.
"""

from __future__ import annotations
import hashlib
import json
import time
import uuid
from dataclasses import dataclass, field
from typing import Optional

from .identity_store import IdentityStore, IdentityCard
from .memory import MemoryStore, Note
from .persona import PersonaEngine, Persona, seed_default_personas
from .questioning import BayesianFunnel, Question, Answer

LINKAGE_VERSION = "0.1.0"


# ────────────────────────────────────────────────────────────────────
# 三条衔接路径的 helper (纯函数, 易测试)
# ────────────────────────────────────────────────────────────────────


def path_a_reconsolidation_to_funnel(
    mem: MemoryStore,
    funnel: BayesianFunnel,
) -> list[str]:
    """Path A: 把所有 flag 的 Note 自动转成 funnel 问题.

    主人的话触发记忆 → 记忆标记 flag → funnel 知道"这件事还没真正搞明白"
    例子:
      master 说 "不要提我私人身份" → Note.flag=importance=0
      Path A → funnel.add_question(q_topic="边界", source="reconsolidation",
                                    prompt="为什么这条记忆被 flag 了? (boundary 未明)")
    """
    added_qids: list[str] = []
    # 去重: 已经由 Path A 处理过的 note (按 nid 标记) 跳过
    seen_marker = "q_rec_seen_"
    seen_nids = {
        q.rationale.split("|")[0].strip()
        for q in funnel.questions.values()
        if q.source == "reconsolidation" and "|" in q.rationale
    }
    for note in mem.notes:
        if note.importance == 0 and "reconsolidation" not in note.topic:
            if note.nid in seen_nids:
                continue
            topic = note.topic.split("_")[0] if "_" in note.topic else note.topic
            q = Question(
                qid=f"q_rec_{uuid.uuid4().hex[:6]}",
                prompt=f"为什么 note[{note.nid}] '{note.claim[:30]}...' 被 flag 了? (boundary 未明)",
                topic=topic,
                prior=0.1,
                source="reconsolidation",
                when_to_ask="anytime",
                rationale=f"{note.nid}|Path A — note importance=0, claim={note.claim[:50]}",
            )
            funnel.add_question(q)
            added_qids.append(q.qid)
    return added_qids


def path_b_question_to_persona(
    q: Question,
    personas: PersonaEngine,
) -> list[Persona]:
    """Path B: 拿 funnel 当前 question 去 persona engine 触发多身份.

    把 question.topic / prompt 当作 event, coordinate() 选 2 个 persona.
    这样答案天生不是单一视角, 而是 调度者+学习者 / 思考者+助手 / 等等.
    """
    event = f"{q.topic} :: {q.prompt}"
    return personas.coordinate(event, k=2)


def path_c_feedback_loop(
    answer: Answer,
    persona_pid: str,
    funnel: BayesianFunnel,
    personas: PersonaEngine,
    feedback_score: float,
) -> dict:
    """Path C: 主人对回答打分 → Bayesian update funnel + adapt persona.

    feedback_score ∈ [-1, +1]:
      +1: 非常满意 → funnel posterior↑ + persona activation↑
       0: 中立 → 只 funnel posterior
      -1: 不满意 → funnel posterior↓ (但不低于 prior) + persona activation↓
    """
    funnel.record_answer(answer)
    personas.adapt(persona_pid, feedback_score)
    return {
        "qid": answer.qid,
        "persona_pid": persona_pid,
        "feedback": feedback_score,
        "posterior_now": funnel._posterior.get(answer.qid, 0.0),
        "persona_activation_now": next(
            (p.activation for p in personas.personas if p.pid == persona_pid), None
        ),
    }


# ────────────────────────────────────────────────────────────────────
# 主类: LinkageOrchestrator
# ────────────────────────────────────────────────────────────────────


@dataclass
class LinkageTurn:
    """一次完整闭环的记录 — 主人看完能 review."""
    turn_id: str
    path: str                              # "A" / "B" / "C" / "A→B" / "A→B→C"
    question_qid: Optional[str]
    persona_pids: list[str]
    answer_qid: Optional[str]
    feedback: float
    note: str
    ts: float = field(default_factory=time.time)


class LinkageOrchestrator:
    """把 IdentityStore + MemoryStore + PersonaEngine + BayesianFunnel 串成闭环.

    典型用法:
        orch = LinkageOrchestrator(identity_store, memory_store, persona_engine, funnel)
        orch.run_path_a()           # 主人预设 → funnel 补问
        orch.run_full_loop(n=3)     # A → B → C 三次
    """

    def __init__(
        self,
        identity: IdentityStore,
        memory: MemoryStore,
        personas: PersonaEngine,
        funnel: BayesianFunnel,
    ):
        self.identity = identity
        self.memory = memory
        self.personas = personas
        self.funnel = funnel
        self.turns: list[LinkageTurn] = []

    # ── Path A: reconsolidation → funnel ────────────────────────────

    def run_path_a(self) -> list[str]:
        added = path_a_reconsolidation_to_funnel(self.memory, self.funnel)
        for qid in added:
            self.turns.append(LinkageTurn(
                turn_id="t_" + uuid.uuid4().hex[:6],
                path="A",
                question_qid=qid,
                persona_pids=[],
                answer_qid=None,
                feedback=0.0,
                note=f"Path A: flag note → funnel question {qid}",
            ))
        return added

    # ── Path B: funnel → persona ────────────────────────────────────

    def run_path_b(self) -> tuple[Optional[Question], list[Persona]]:
        q = self.funnel.ask_next()
        if q is None:
            return None, []
        activated = path_b_question_to_persona(q, self.personas)
        self.turns.append(LinkageTurn(
            turn_id="t_" + uuid.uuid4().hex[:6],
            path="B",
            question_qid=q.qid,
            persona_pids=[p.pid for p in activated],
            answer_qid=None,
            feedback=0.0,
            note=f"Path B: ask {q.qid} → activate {len(activated)} persona(s)",
        ))
        return q, activated

    # ── Path C: feedback → funnel + persona ─────────────────────────

    def run_path_c(
        self,
        qid: str,
        persona_pid: str,
        answer_text: str,
        observed: float,
        feedback_score: float,
    ) -> dict:
        answer = Answer(
            qid=qid,
            answer_text=answer_text,
            observed=observed,
            evidence_refs=[],
        )
        result = path_c_feedback_loop(
            answer, persona_pid, self.funnel, self.personas, feedback_score,
        )
        self.turns.append(LinkageTurn(
            turn_id="t_" + uuid.uuid4().hex[:6],
            path="C",
            question_qid=qid,
            persona_pids=[persona_pid],
            answer_qid=qid,
            feedback=feedback_score,
            note=f"Path C: feedback={feedback_score:+.2f}, posterior={result['posterior_now']:.3f}",
        ))
        return result

    # ── 完整闭环: A → B → C × n ────────────────────────────────────

    def run_full_loop(
        self,
        n: int = 3,
        scripted_answers: Optional[list[dict]] = None,
    ) -> list[LinkageTurn]:
        """n 轮完整闭环 — 用 scripted_answers 模拟主人回答 (无 LLM).

        scripted_answers[i] = {qid, text, observed, persona_pid, feedback}
        """
        # 1) 先跑 Path A — 把已 flag 的 note 转成 funnel 问题
        self.run_path_a()

        out: list[LinkageTurn] = []
        for i in range(n):
            # 2) Path B — 拿下一个问题, 激活 persona
            q, activated = self.run_path_b()
            if q is None or not activated:
                break

            # 3) Path C — 模拟主人回答 + 反馈
            if scripted_answers and i < len(scripted_answers):
                sa = scripted_answers[i]
                # 用脚本答案 (PoC 演示用)
                if sa.get("qid") and sa["qid"] != q.qid:
                    # 脚本指定了不同的 qid, 还是用当前的 q
                    pass
                answer_text = sa.get("text", f"[scripted #{i+1}] answer for {q.topic}")
                observed = sa.get("observed", 0.7)
                feedback = sa.get("feedback", 0.5)
                persona_pid = sa.get("persona_pid", activated[0].pid)
            else:
                # 兜底: 默认脚本
                answer_text = f"[auto #{i+1}] 关于 {q.topic} 的回答"
                observed = 0.6 + 0.1 * i
                feedback = 0.4
                persona_pid = activated[0].pid

            self.run_path_c(
                qid=q.qid,
                persona_pid=persona_pid,
                answer_text=answer_text,
                observed=observed,
                feedback_score=feedback,
            )

        # 返回本轮新增的 turn
        return [t for t in self.turns[-3*n:]] if self.turns else []

    # ── 持久化 / 自检 ──────────────────────────────────────────────

    def integrity_hash(self) -> str:
        """跨模块的 hash — 4 个子模块都算 + linkage turns.

        PersistBench (2602.01146) 警示 97% sycophancy 风险, 这是 L4 内部一致性的
        第五层防线 (identity / memory / graph / funnel / linkage).
        """
        payload = {
            "linkage_version": LINKAGE_VERSION,
            "identity_hash": self.identity.integrity_hash(),
            "memory_hash": self.memory.integrity_hash(),
            "funnel_hash": self.funnel.integrity_hash(),
            "persona_count": len(self.personas.personas),
            "persona_min_distance": self.personas.min_distance,
            "turns": [
                {"path": t.path, "qid": t.question_qid, "feedback": t.feedback}
                for t in self.turns
            ],
        }
        canon = json.dumps(payload, sort_keys=True, ensure_ascii=False)
        return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]

    def snapshot(self) -> dict:
        return {
            "version": LINKAGE_VERSION,
            "turn_count": len(self.turns),
            "turns": [
                {
                    "turn_id": t.turn_id, "path": t.path,
                    "question_qid": t.question_qid,
                    "persona_pids": t.persona_pids,
                    "feedback": t.feedback, "note": t.note, "ts": t.ts,
                }
                for t in self.turns
            ],
            "integrity_hash": self.integrity_hash(),
            "ts": time.time(),
        }


__all__ = [
    "LINKAGE_VERSION",
    "path_a_reconsolidation_to_funnel",
    "path_b_question_to_persona",
    "path_c_feedback_loop",
    "LinkageTurn", "LinkageOrchestrator",
]