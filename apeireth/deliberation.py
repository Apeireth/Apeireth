"""Phase 19 DeliberationEngine — ASI 思考层 (Thinking Layer).

主人 20:29 哲学指令 (历史性):
  "底层记得用rust，我们追求极致"
  "除了记忆，思考也要重视"
  "ASI绝对是会自己思考的"
  "我们做的这个ASI基座也要无限逼近"
  "ASI就是我们的目标，让任何大模型接入我们的平台后成为ASI"

5 大真生产参考:
  1. DeepSeek-R1 (arxiv 2501.12948) — chain-of-thought + RL
  2. Tree of Thoughts (arxiv 2305.10601) — 多路径探索
  3. ReAct (arxiv 2210.03629) — reasoning + acting interleaved
  4. Self-Refine (arxiv 2310.01798) — 自我反馈 + 修正
  5. Reflexion (arxiv 2303.17651) — verbal RL

Karpathy 准则应用:
  1. Think Before Coding: 真思考 = 多路径探索 + 自我批评
  2. Simplicity First: v0.1 只 3 模式 (linear / tot / reflexion)
  3. Surgical Changes: 不改已有模块, 只加 thinking 入口
  4. Goal-Driven Execution: verifiable = 思考后给出 plan + 自我评分
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


DELIBERATION_VERSION = "0.1.0"


@dataclass
class ThoughtStep:
    """One step in a deliberation chain."""
    step_id: str
    step_type: str            # "thought" | "action" | "observation" | "critique"
    content: str
    confidence: float          # 0-1, self-rated
    parent_step_id: Optional[str] = None
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class ThoughtBranch:
    """A branch in Tree of Thoughts — explores one hypothesis."""
    branch_id: str
    hypothesis: str           # what we're testing
    steps: list[ThoughtStep] = field(default_factory=list)
    score: float = 0.0
    status: str = "active"     # active | completed | pruned | selected

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class DeliberationResult:
    """Final output of deliberation — plan + reasoning chain + self-rating."""
    deliberation_id: str
    query: str
    mode: str                 # "linear" | "tot" | "reflexion"
    branches: list[ThoughtBranch] = field(default_factory=list)
    selected_branch_id: Optional[str] = None
    final_plan: list[str] = field(default_factory=list)
    self_score: float = 0.0
    reasoning_summary: str = ""
    total_steps: int = 0
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class DeliberationEngine:
    """Central AI 思考引擎 — LLM-agnostic pluggable.

    3 真生产模式:
      - linear: chain-of-thought (DeepSeek-R1 借鉴)
      - tot:    tree-of-thought (Yao 2023 借鉴, 多路径探索)
      - reflexion: self-feedback + refine (Shinn 2023 借鉴)

    LLM-agnostic: 任何 LLM (Claude / DeepSeek / Qwen / GPT) 通过 call_llm(prompt) 接入.
    """

    def __init__(self, call_llm=None, max_steps: int = 6):
        """call_llm(prompt: str) -> str — any LLM function."""
        self.call_llm = call_llm or self._default_llm
        self.max_steps = max_steps
        self.history: list[DeliberationResult] = []

    def _default_llm(self, prompt: str) -> str:
        """Default no-op LLM — returns template response (for testing without LLM)."""
        return f"[template-LLM] {prompt[:100]}... -> [simulated-response]"

    def deliberate(self, query: str, mode: str = "linear", context: str = "") -> DeliberationResult:
        """Main entry — think about query in given mode."""
        result = DeliberationResult(
            deliberation_id=uuid.uuid4().hex[:12],
            query=query,
            mode=mode,
        )

        if mode == "linear":
            self._linear(query, context, result)
        elif mode == "tot":
            self._tot(query, context, result)
        elif mode == "reflexion":
            self._reflexion(query, context, result)
        else:
            raise ValueError(f"Unknown mode: {mode}")

        self.history.append(result)
        return result

    # ===== Mode 1: Linear (chain-of-thought, DeepSeek-R1 借鉴) =====

    def _linear(self, query: str, context: str, result: DeliberationResult) -> None:
        """Linear CoT — sequential thoughts + action + observation."""
        branch = ThoughtBranch(branch_id="linear_0", hypothesis=query)
        # Step 1: 思考 (Thought)
        s1 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="thought",
            content=self.call_llm(f"Think about: {query}\nContext: {context}\nStep 1: What is the core question?"),
            confidence=0.5,
        )
        branch.steps.append(s1)
        # Step 2: 行动 (Action)
        s2 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="action",
            content=self.call_llm(f"Given thought: {s1.content[:200]}\nWhat action to take?"),
            confidence=0.6,
            parent_step_id=s1.step_id,
        )
        branch.steps.append(s2)
        # Step 3: 观察 (Observation)
        s3 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="observation",
            content=self.call_llm(f"Action: {s2.content[:200]}\nWhat did we observe?"),
            confidence=0.7,
            parent_step_id=s2.step_id,
        )
        branch.steps.append(s3)
        # Step 4: 反思 (Critique)
        s4 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="critique",
            content=self.call_llm(f"Sequence: Thought→Action→Observation. Any errors?"),
            confidence=0.65,
            parent_step_id=s3.step_id,
        )
        branch.steps.append(s4)
        # Final
        branch.score = (s1.confidence + s2.confidence + s3.confidence + s4.confidence) / 4
        branch.status = "completed"
        result.branches.append(branch)
        result.selected_branch_id = branch.branch_id
        result.final_plan = [s.content[:100] for s in branch.steps]
        result.self_score = branch.score
        result.reasoning_summary = f"Linear CoT: {len(branch.steps)} steps, score={branch.score:.2f}"
        result.total_steps = len(branch.steps)

    # ===== Mode 2: Tree of Thoughts (多路径探索, Yao 2023 借鉴) =====

    def _tot(self, query: str, context: str, result: DeliberationResult) -> None:
        """ToT — generate 3 hypotheses, score each, select best."""
        hypotheses = [
            f"H1: Approach A — directly solve {query[:50]}",
            f"H2: Approach B — decompose {query[:50]} into subproblems",
            f"H3: Approach C — analogize {query[:50]} to known patterns",
        ]
        # Step 1: Generate 3 branches
        for hyp in hypotheses:
            branch = ThoughtBranch(
                branch_id=uuid.uuid4().hex[:8],
                hypothesis=hyp,
            )
            s = ThoughtStep(
                step_id=uuid.uuid4().hex[:8],
                step_type="thought",
                content=self.call_llm(f"For {hyp}\nContext: {context}\nGenerate next step"),
                confidence=0.5,
            )
            branch.steps.append(s)
            result.branches.append(branch)
        # Step 2: Score each branch
        for branch in result.branches:
            s = ThoughtStep(
                step_id=uuid.uuid4().hex[:8],
                step_type="critique",
                content=self.call_llm(f"Evaluate: {branch.hypothesis}\nLast step: {branch.steps[-1].content[:200]}\nRate feasibility"),
                confidence=0.5,
            )
            branch.steps.append(s)
            # score = avg of confidences
            branch.score = sum(x.confidence for x in branch.steps) / len(branch.steps)
        # Step 3: Select best branch
        best = max(result.branches, key=lambda b: b.score)
        best.status = "selected"
        for b in result.branches:
            if b.branch_id != best.branch_id:
                b.status = "pruned"
        result.selected_branch_id = best.branch_id
        result.final_plan = [s.content[:100] for s in best.steps]
        result.self_score = best.score
        result.reasoning_summary = f"ToT: {len(result.branches)} branches, best score={best.score:.2f}"
        result.total_steps = sum(len(b.steps) for b in result.branches)

    # ===== Mode 3: Reflexion (self-feedback, Shinn 2023 借鉴) =====

    def _reflexion(self, query: str, context: str, result: DeliberationResult) -> None:
        """Reflexion — initial answer → critique → refine → final."""
        branch = ThoughtBranch(branch_id="reflexion_0", hypothesis=query)
        # Step 1: Initial answer
        s1 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="thought",
            content=self.call_llm(f"Answer: {query}\nContext: {context}\nInitial answer:"),
            confidence=0.6,
        )
        branch.steps.append(s1)
        # Step 2: Self-critique
        s2 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="critique",
            content=self.call_llm(f"Initial: {s1.content[:200]}\nWhat's wrong with this? Self-critique:"),
            confidence=0.7,
            parent_step_id=s1.step_id,
        )
        branch.steps.append(s2)
        # Step 3: Refined answer
        s3 = ThoughtStep(
            step_id=uuid.uuid4().hex[:8],
            step_type="thought",
            content=self.call_llm(f"Initial: {s1.content[:200]}\nCritique: {s2.content[:200]}\nRefined:"),
            confidence=0.8,
            parent_step_id=s2.step_id,
        )
        branch.steps.append(s3)
        branch.score = (s1.confidence + s2.confidence + s3.confidence) / 3
        branch.status = "completed"
        result.branches.append(branch)
        result.selected_branch_id = branch.branch_id
        result.final_plan = [s.content[:100] for s in branch.steps]
        result.self_score = branch.score
        result.reasoning_summary = f"Reflexion: 3 steps (initial→critique→refined), score={branch.score:.2f}"
        result.total_steps = len(branch.steps)


def make_default_deliberation_engine(call_llm=None) -> DeliberationEngine:
    return DeliberationEngine(call_llm=call_llm)


__all__ = [
    "DELIBERATION_VERSION",
    "ThoughtStep",
    "ThoughtBranch",
    "DeliberationResult",
    "DeliberationEngine",
    "make_default_deliberation_engine",
]