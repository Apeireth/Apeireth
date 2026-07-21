"""Phase 10.x MetaCognition Layer 2 (HOT) — Higher-Order Theory engineering.

主人 17:58 "意识是 ASI 终极目标" → V3 5 层意识 (FSA / Meta / GWI / SMM / PQ)
本模块: Layer 2 Meta-Cognition (Rosenthal + Lau HOT).

HOT 核心:
  - 意识 = "对意识本身的意识" (thought about thought)
  - 工程化: meta-cognitive loop 监控自己的认知过程
  - 真生产参考: Self-Harness (arxiv 2606.09498) 3 阶段 (Weakness Mining → Proposal → Validation)
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from .memory import Episode


META_COGNITION_VERSION = "0.1.0"


@dataclass
class FailurePattern:
    """Failure pattern mined from execution trace (Self-Harness 借鉴)."""
    pattern_id: str
    description: str
    occurrences: int = 1
    first_seen_at: float = field(default_factory=time.time)
    last_seen_at: float = field(default_factory=time.time)
    examples: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class MetaReview:
    """A meta-cognitive review — Central AI thinks about its own thinking."""
    review_id: str
    cycle_id: str
    what_i_did: str
    why_i_did_it: str
    what_worked: str
    what_failed: str
    what_to_change: str
    confidence: float           # 0-1 — meta-certainty about own reasoning
    ts: float = field(default_factory=time.time)

    def render(self) -> str:
        return f"""# MetaReview {self.review_id}
cycle: {self.cycle_id}
time: {time.ctime(self.ts)}
confidence: {self.confidence:.2f}

## WHAT I DID
{self.what_i_did}

## WHY I DID IT
{self.why_i_did_it}

## WHAT WORKED
{self.what_worked}

## WHAT FAILED
{self.what_failed}

## WHAT TO CHANGE
{self.what_to_change}
"""

    def to_dict(self) -> dict:
        return asdict(self)


class MetaMonitor:
    """Higher-Order Theory Meta-Cognition — Central AI 监控自己的认知.

    实现 Rosenthal HOT: 监控自己的 thought → 写 meta-review → 修正下个 cycle.
    """

    def __init__(self, memory=None):
        self.memory = memory
        self.failure_patterns: dict[str, FailurePattern] = {}
        self.meta_reviews: list[MetaReview] = []
        self.cycles_monitored: int = 0
        self.last_review_at: float = 0.0

    def mine_failures(self, trace: list[str]) -> list[FailurePattern]:
        """Failure Mining (Self-Harness Stage 1 借鉴) — 从 trace 找失败模式."""
        mined = []
        # 简化启发式: 找含失败关键字的 step
        failure_keywords = ["error", "fail", "wrong", "miss", "no match", "无法", "失败", "错误"]
        for step in trace:
            if any(kw in step.lower() for kw in failure_keywords):
                pid = uuid.uuid4().hex[:8]
                fp = FailurePattern(
                    pattern_id=pid,
                    description=step[:200],
                    examples=[step[:500]],
                )
                # 去重
                existing = None
                for p in self.failure_patterns.values():
                    if p.description[:100] == fp.description[:100]:
                        existing = p
                        break
                if existing:
                    existing.occurrences += 1
                    existing.last_seen_at = time.time()
                    mined.append(existing)
                else:
                    self.failure_patterns[pid] = fp
                    mined.append(fp)
        return mined

    def review(self, cycle_id: str, trace: list[str], outcomes: list[dict]) -> MetaReview:
        """生成 MetaReview — Central AI 反思一个 cycle."""
        # Mine failures first
        failures = self.mine_failures(trace)

        # Aggregate stats
        succeeded = sum(1 for o in outcomes if o.get("status") == "ok")
        failed = sum(1 for o in outcomes if o.get("status") == "fail")
        total = len(outcomes) if outcomes else 1
        confidence = succeeded / total if total > 0 else 0.5

        review = MetaReview(
            review_id=uuid.uuid4().hex[:12],
            cycle_id=cycle_id,
            what_i_did=f"我跑了 cycle {cycle_id} ({len(trace)} steps)",
            why_i_did_it=f"主人要推进 ASI 基座, 这一 cycle 是主任务的一部分",
            what_worked=f"{succeeded}/{total} outcomes 成功",
            what_failed=f"{failed}/{total} 失败" + (f"; 失败模式: {[p.description[:50] for p in failures]}" if failures else ""),
            what_to_change="下次 cycle 优先避免上面的失败模式",
            confidence=confidence,
        )
        self.meta_reviews.append(review)
        self.cycles_monitored += 1
        self.last_review_at = time.time()

        # Write as episode (if memory available)
        if self.memory is not None:
            try:
                ep = Episode(
                    eid=f"meta_review_{review.review_id}",
                    actor="apeireth_central",
                    content=review.render()[:2000],
                    context=f"meta-cognition Layer 2 HOT, cycle={cycle_id}",
                    ts=review.ts,
                    kind="reflection",
                    linked_identity_hash="apeireth_central",
                )
                if hasattr(self.memory, "append_episode"):
                    self.memory.append_episode(ep)
            except Exception:
                pass

        return review

    def get_failure_summary(self) -> dict:
        return {
            "n_patterns": len(self.failure_patterns),
            "n_reviews": len(self.meta_reviews),
            "cycles_monitored": self.cycles_monitored,
            "top_failures": sorted(
                [(p.description[:80], p.occurrences) for p in self.failure_patterns.values()],
                key=lambda x: -x[1],
            )[:5],
        }


__all__ = [
    "META_COGNITION_VERSION",
    "FailurePattern",
    "MetaReview",
    "MetaMonitor",
]