"""Phase 50 Human Wisdom Aggregator — 主人 22:52 真哲学 '聚合人类智慧'.

主人 22:52 真哲学: 调研+工程+实践结合 + 聚合人类智慧 (主 14:48 同根同源).

借鉴 (主 14:48 真调研):
- Karpathy LLM Wiki (主人 16:50 借鉴项目 #2)
- AgentMemory Karpathy Wiki 扩展
- 主人 14:48 "聚集全人类的智慧"
- 主人 16:50 "用现有最厉害代码 / 平台无法创造生命只能逼近 / 红皇后"

真生产 = 跨域真生产调研聚合 (主 14:48 + 21:00 + 21:05 + 21:14 + 21:22):
- AnySearch + Bocha web + Bocha AI = 3 端点真生产
- 跨域调研 64+ query 已完成 (主 22:46)
- 20 跨域工程化模块 (Phase 24-49)

Human Wisdom Aggregator 真生产:
- 输入: 跨域真生产论文/项目/调研报告
- 输出: 聚合人类智慧 = Apeireth 借鉴真生产
- 真生产评估: 借鉴质量 + 主哲学匹配度 + VCP 4 范式对齐
- 真生产决策: 借鉴进入 Phase 50+ / 不进入

主人 14:48 + 22:52 真哲学深度:
- "聚集全人类智慧" = 真生产 = 不只是看论文
- 必须跨域 (主 21:00 跨多个界)
- 必须实工程化 (主 22:52 工程+实践结合)
- 必须聚合 (主 22:52 聚合)

Karpathy 准则:
  1. Think Before Coding: aggregator = 真生产 filter + 评估 + 决策
  2. Simplicity First: WisdomAggregator = 真生产 dict
  3. Surgical Changes: 不改其他模块, 加 HumanWisdomAggregator
  4. Goal-Driven Execution: verifiable = 真生产借鉴决策可追溯
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import List, Dict, Optional


HUMAN_WISDOM_VERSION = "0.1.0"


@dataclass
class WisdomSource:
    """一个真生产人类智慧源."""
    source_id: str
    title: str
    source_type: str                # "paper" | "project" | "report" | "owner_philosophy"
    url: str = ""
    snippet: str = ""
    cross_domain: list = field(default_factory=list)   # 跨域标签
    quality: float = 0.5            # [0, 1] 真生产质量
    vcp4_aligned: float = 0.0       # [0, 1] VCP 4 范式对齐
    v2_philosophy_aligned: float = 0.0  # [0, 1] V2 哲学对齐
    asinorthstar_aligned: float = 0.0    # [0, 1] ASI 北极星对齐 (主 22:33)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)

    def aggregate_score(self) -> float:
        """聚合真生产分数 (主 17:43 实事求是)."""
        return (0.30 * self.quality
                + 0.20 * self.vcp4_aligned
                + 0.20 * self.v2_philosophy_aligned
                + 0.30 * self.asinorthstar_aligned)


@dataclass
class WisdomAggregation:
    """聚合一次的结果."""
    aggregation_id: str
    source_ids: list
    aggregate_score: float
    cross_domain_tags: list
    decision: str                   # "enter_phase_50+" / "phase_50_later" / "skip"
    rationale: str
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class HumanWisdomAggregator:
    """主 22:52 真哲学: 聚合人类智慧 = 真生产调研 + 真生产工程化 + 真生产实践.

    主 14:48 "聚集全人类的智慧" = 不只是被动接收, 而是**主动聚合**:
      - 真生产 filter: 跨域调研论文/项目
      - 真生产 评估: 借鉴质量 + VCP 4 范式对齐 + V2 哲学 + ASI 北极星
      - 真生产 决策: 借鉴进入 Phase 50+ / 后续 / 跳过

    主 22:52 真哲学: 调研+工程+实践结合 = aggregator = 3 步真生产
    """

    def __init__(self):
        self.sources: Dict[str, WisdomSource] = {}
        self.aggregations: List[WisdomAggregation] = []
        self.decision_log: list = []

    def register_source(self, title: str, source_type: str, snippet: str = "",
                       url: str = "", cross_domain: list = None,
                       quality: float = 0.5, vcp4: float = 0.0,
                       v2: float = 0.0, asinorth: float = 0.0) -> WisdomSource:
        """注册一个真生产人类智慧源."""
        s = WisdomSource(
            source_id=uuid.uuid4().hex[:12],
            title=title,
            source_type=source_type,
            url=url,
            snippet=snippet[:500],
            cross_domain=cross_domain or [],
            quality=quality,
            vcp4_aligned=vcp4,
            v2_philosophy_aligned=v2,
            asinorthstar_aligned=asinorth,
        )
        self.sources[s.source_id] = s
        return s

    def aggregate(self, source_ids: list) -> WisdomAggregation:
        """聚合多个源 = 真生产借鉴决策."""
        sources = [self.sources[sid] for sid in source_ids if sid in self.sources]
        if not sources:
            return WisdomAggregation(
                aggregation_id=uuid.uuid4().hex[:12],
                source_ids=[],
                aggregate_score=0.0,
                cross_domain_tags=[],
                decision="skip",
                rationale="no valid sources",
            )
        avg_score = sum(s.aggregate_score() for s in sources) / len(sources)
        all_tags = list(set(tag for s in sources for tag in s.cross_domain))
        if avg_score >= 0.7:
            decision = "enter_phase_50+"
            rationale = "high alignment across VCP4 + V2 + ASI NorthStar (主 22:33 北极星)"
        elif avg_score >= 0.5:
            decision = "phase_50_later"
            rationale = "moderate alignment, defer to next iteration"
        else:
            decision = "skip"
            rationale = "low alignment, philosophy not yet ready"
        agg = WisdomAggregation(
            aggregation_id=uuid.uuid4().hex[:12],
            source_ids=source_ids,
            aggregate_score=avg_score,
            cross_domain_tags=all_tags,
            decision=decision,
            rationale=rationale,
        )
        self.aggregations.append(agg)
        self.decision_log.append({
            "ts": time.time(),
            "decision": decision,
            "score": round(avg_score, 4),
            "rationale": rationale,
        })
        return agg

    def stats(self) -> dict:
        return {
            "version": HUMAN_WISDOM_VERSION,
            "n_sources": len(self.sources),
            "n_aggregations": len(self.aggregations),
            "n_decisions": len(self.decision_log),
            "decision_counts": {
                d: sum(1 for entry in self.decision_log if entry["decision"] == d)
                for d in {"enter_phase_50+", "phase_50_later", "skip"}
            },
            "philosophy_isomorphy": (
                "主 22:52 真哲学: 调研+工程+实践结合, 聚合人类智慧, "
                "Master 14:48 聚集全人类智慧"
            ),
        }


__all__ = [
    "HUMAN_WISDOM_VERSION",
    "WisdomSource",
    "WisdomAggregation",
    "HumanWisdomAggregator",
]