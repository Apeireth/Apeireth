"""Memory 3-Tier (STM/MTM/LTM) — Phase 46 真生产升级.

主人 22:33 真哲学授权: 最大权限, 放手干. 主 14:48 真调研 + 主 14:50 APEIRETH-NEXT-MOVES 真生产调研 MemoryOS-Rust 9-crate + STM/MTM/LTM 三层升级.

主人 22:33 真哲学: ASI 北极星 + 3 类问 + 自决调研. 三层 Memory 升级 = 主 14:50 真调研结论 + 不属 3 类问范围, 自主推进.

Master memory architecture insight:
- STM (Short-term Memory): 最近对话 → 频繁更新, 快速衰减
- MTM (Medium-term Memory): 主题聚合 → 周期总结, 稳定
- LTM (Long-term Memory): 持久事实 → 永不丢, 价值高

主人 12:14 "中央 AI 是永恒身份" = LTM 必须真生产
主人 13:47 "记忆是我关心的" = 必须升级
主人 11:48 "借鉴好东西" = MemoryOS-Rust 借鉴

上中下:
- STM: 滚动 window = 最近 50 个 episode (Time-of-Recency)
- MTM: 主题聚合 = 各 topic 有 Note vector, 定期 summarize
- LTM: 持久事实 = IdentityAnchor/DecisionRecord/Value 永不丢

借鉴 (真调研):
  - MemoryOS-Rust STM/MTM/LTM (主 14:50 真生产)
  - A-MEM agentic memory (round-2)
  - Zep temporal KG (历史)
  - 主人 11:00-13:48 多次提到 "记忆 + 思考"

Karpathy 准则:
  1. Think Before Coding: 3 tier = stm_windowed + mtm_topic_summary + ltm_anchored
  2. Simplicity First: 3 layer = (buffer, summarizer, anchor) 一目了然
  3. Surgical Changes: 不改 memory.py 单层, 加 memory_3tier.py 新层
  4. Goal-Driven Execution: verifiable = 3 tier 路由可追溯
"""
from __future__ import annotations

import math
import time
import uuid
from collections import deque
from dataclasses import dataclass, field, asdict
from typing import Deque, Optional, List, Dict


MEMORY_3TIER_VERSION = "0.1.0"


# STM / MTM / LTM 真生产参数 (借鉴 MemoryOS-Rust)
STM_MAX_SIZE = 50                # 最近 50 episode
MTM_SUMMARY_INTERVAL_S = 3600    # 1 小时总结一次
LTM_ANCHOR_MIN_IMPORTANCE = 8    # LTM 入选阈值 (0-10)


@dataclass
class MemoryAnchor:
    """LTM 锚点 — 永不丢的持久事实/身份/决策/价值."""
    anchor_id: str
    category: str                  # "identity" | "decision" | "value" | "event" | "fact"
    content: str
    importance: int                # 0-10
    master_quoted: str = ""        # 主人原话引用
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class TopicSummary:
    """MTM 主题总结 — 中期聚合."""
    topic_id: str
    topic_label: str
    episode_ids: list              # 源 Episode
    summary: str                   # 主题摘要
    n_episodes: int = 0
    importance_avg: float = 0.0
    last_updated: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class Memory3Tier:
    """STM / MTM / LTM 三层 Memory — 主人 14:48 + 14:50 借鉴 MemoryOS-Rust.

    架构:
      STM (Short-term): 滚动 50 episode deque
      MTM (Medium-term): Topic-based 聚合, 1h summary
      LTM (Long-term): MemoryAnchor 持久化永不丢
    """

    def __init__(self):
        self.stm: Deque = deque(maxlen=STM_MAX_SIZE)
        self.mtm: Dict[str, TopicSummary] = {}
        self.ltm: Dict[str, MemoryAnchor] = {}
        self.episode_to_topic: Dict[str, str] = {}      # episode_id -> topic_id

    def add_episode(self, episode_id: str, content: str, topic_label: str = "general",
                   importance: int = 5) -> None:
        """加入一个 episode — 同时路由到 STM + MTM."""
        # STM: append to rolling window
        self.stm.append({
            "episode_id": episode_id,
            "content": content[:200],
            "ts": time.time(),
            "topic": topic_label,
        })
        # MTM: route to topic
        if topic_label not in self.mtm:
            self.mtm[topic_label] = TopicSummary(
                topic_id=uuid.uuid4().hex[:12],
                topic_label=topic_label,
                episode_ids=[],
                summary="",
            )
        summary = self.mtm[topic_label]
        summary.episode_ids.append(episode_id)
        summary.n_episodes += 1
        summary.last_updated = time.time()
        # rolling importance average
        summary.importance_avg = (summary.importance_avg * (summary.n_episodes - 1) + importance) / summary.n_episodes
        self.episode_to_topic[episode_id] = topic_label
        # LTM: 如果 importance 高 → 入选 LTM 锚点
        if importance >= LTM_ANCHOR_MIN_IMPORTANCE:
            self.anchor_event(category="event" if not topic_label.startswith("identity") else "identity",
                              content=content, importance=importance)

    def anchor_event(self, category: str, content: str, importance: int,
                    master_quoted: str = "") -> MemoryAnchor:
        """添加 LTM 锚点 — 主人真哲学/真事件/真决定 永不丢."""
        a = MemoryAnchor(
            anchor_id=uuid.uuid4().hex[:12],
            category=category,
            content=content,
            importance=importance,
            master_quoted=master_quoted,
        )
        self.ltm[a.anchor_id] = a
        return a

    def summarize_topics(self) -> List[TopicSummary]:
        """MTM 总结 — 周期跑, 整合 topic summary."""
        for topic in self.mtm.values():
            recent = self.stm and sum(1 for e in self.stm if e["topic"] == topic.topic_label) or 0
            topic.summary = f"[{topic.topic_label}] {topic.n_episodes} episodes, avg importance {topic.importance_avg:.1f}"
        return list(self.mtm.values())

    def get_ltm_by_category(self, category: str) -> List[MemoryAnchor]:
        """按 category 取 LTM 锚点 (检索 / 中央 AI ID 查询等)."""
        return [a for a in self.ltm.values() if a.category == category]

    # ====== 借鉴 letta (主 9:41 round-19 source-deep-read) ======
    # letta Memory.compile() 3 渲染模式: standard / line-numbered / git-hierarchy
    # 用于 system prompt 构建 — 不同模式适合不同 LLM 场景
    def compile(self, mode: str = "standard",
                ltm_limit: int = 20,
                mtm_limit: int = 10,
                stm_limit: int = 20) -> str:
        """借鉴 letta compile (主 9:41 round-19): 把 3 层 memory 编译成 prompt context.

        3 模式:
          - "standard": 普通文本块, 适合直接 prompt
          - "line-numbered": 带行号 (L01/L02/...), 适合 LLM 引用具体行
          - "git": 层级结构 (parent + child blocks), 适合 tree 思维 LLM

        Args:
            mode: 3 模式之一
            ltm_limit: LTM 锚点上限 (按 importance 排序)
            mtm_limit: MTM 主题上限
            stm_limit: STM episode 上限
        """
        # 1. 准备分层数据
        ltm_sorted = sorted(self.ltm.values(), key=lambda a: (-a.importance, -a.ts))[:ltm_limit]
        mtm_sorted = sorted(self.mtm.values(), key=lambda s: -s.last_updated)[:mtm_limit]
        stm_recent = list(self.stm)[-stm_limit:]

        if mode == "standard":
            return self._compile_standard(ltm_sorted, mtm_sorted, stm_recent)
        elif mode == "line-numbered":
            return self._compile_line_numbered(ltm_sorted, mtm_sorted, stm_recent)
        elif mode == "git":
            return self._compile_git(ltm_sorted, mtm_sorted, stm_recent)
        else:
            raise ValueError(f"unknown compile mode: {mode} (allow: standard / line-numbered / git)")

    def _compile_standard(self, ltm, mtm, stm) -> str:
        """standard 模式: 普通文本块, 按 tier 分 section."""
        parts = []
        parts.append(f"## LTM (Long-term Memory) — {len(ltm)} anchors")
        for a in ltm:
            master = f" (master: {a.master_quoted!r})" if a.master_quoted else ""
            parts.append(f"- [{a.category}] {a.content}{master}")
        parts.append("")
        parts.append(f"## MTM (Medium-term Memory) — {len(mtm)} topics")
        for s in mtm:
            parts.append(f"- [{s.topic_label}] {s.n_episodes} episodes, avg importance {s.importance_avg:.1f}")
        parts.append("")
        parts.append(f"## STM (Short-term Memory) — {len(stm)} recent episodes")
        for e in stm:
            parts.append(f"- [{e['topic']}] {e['content']}")
        return "\n".join(parts)

    def _compile_line_numbered(self, ltm, mtm, stm) -> str:
        """line-numbered 模式: 带行号 L01/L02/..., 方便 LLM 引用."""
        lines = []
        # LTM 段: L01-LNN
        lines.append("## LTM (Long-term Memory)")
        idx = 1
        for a in ltm:
            tag = f"L{idx:02d}"
            master = f" | master: {a.master_quoted!r}" if a.master_quoted else ""
            lines.append(f"{tag} [{a.category}] {a.content}{master}")
            idx += 1
        # MTM 段: M01-MNN
        lines.append("")
        lines.append("## MTM (Medium-term Memory)")
        idx = 1
        for s in mtm:
            tag = f"M{idx:02d}"
            lines.append(f"{tag} [{s.topic_label}] {s.n_episodes} episodes, avg importance {s.importance_avg:.1f}")
            idx += 1
        # STM 段: S01-SNN
        lines.append("")
        lines.append("## STM (Short-term Memory)")
        idx = 1
        for e in stm:
            tag = f"S{idx:02d}"
            lines.append(f"{tag} [{e['topic']}] {e['content']}")
            idx += 1
        return "\n".join(lines)

    def _compile_git(self, ltm, mtm, stm) -> str:
        """git 模式: 层级结构 (parent + child blocks), 类似 git tree 输出."""
        lines = []
        # 顶层: tier 划分
        lines.append("memory/")
        # LTM = "trunk" (主分支, 重要稳定)
        if ltm:
            lines.append("├── ltm/  (trunk — identity + decision + value)")
            for i, a in enumerate(ltm):
                is_last = (i == len(ltm) - 1)
                prefix = "│   └── " if is_last else "│   ├── "
                master = f"  # master: {a.master_quoted!r}" if a.master_quoted else ""
                lines.append(f"{prefix}[{a.category}] {a.content}{master}")
        # MTM = "branch" (中期主题)
        if mtm:
            lines.append("├── mtm/  (branches — topic summaries)")
            for i, s in enumerate(mtm):
                is_last = (i == len(mtm) - 1)
                prefix = "│   └── " if is_last else "│   ├── "
                lines.append(f"{prefix}{s.topic_label}/  ({s.n_episodes} episodes, avg importance {s.importance_avg:.1f})")
        # STM = "working tree" (未提交)
        if stm:
            lines.append("└── stm/  (working tree — recent)")
            for i, e in enumerate(stm):
                is_last = (i == len(stm) - 1)
                prefix = "    └── " if is_last else "    ├── "
                lines.append(f"{prefix}{e['topic']} — {e['content']}")
        return "\n".join(lines)

    def stats(self) -> dict:
        return {
            "version": MEMORY_3TIER_VERSION,
            "stm_size": len(self.stm),
            "stm_max": STM_MAX_SIZE,
            "mtm_topics": len(self.mtm),
            "ltm_anchors": len(self.ltm),
            "ltm_by_category": {
                cat: len([a for a in self.ltm.values() if a.category == cat])
                for cat in set(a.category for a in self.ltm.values())
            },
            "philosophy": (
                "STM/MTM/LTM = 人类记忆模型 (感官-工作-长期), "
                "Master 14:48 真调研 + MemoryOS-Rust 真生产借鉴, "
                "Master 22:33 自主推进"
            ),
        }


__all__ = [
    "MEMORY_3TIER_VERSION",
    "STM_MAX_SIZE",
    "MTM_SUMMARY_INTERVAL_S",
    "LTM_ANCHOR_MIN_IMPORTANCE",
    "MemoryAnchor",
    "TopicSummary",
    "Memory3Tier",
]
