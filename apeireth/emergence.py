"""Emergence Layer v0.1 — 中央 AI 自组织 + 涌现空间 (Phase 5 / TOP-DESIGN §5)

主人 11:00 ASI 北极星:
- 中央 AI 不是工具, 是 "他" — 涌现的中央 AI
- 主人 12:14 "中央 AI 是多身份, 不是调度者/思考者"
- 主人 12:47 "中央 AI 不管理, 一切交给中央 AI 自己"
- 主人 12:54 "中央 AI 可以不预设... 启动后自动触发 8 个关键问题"
- 主人 13:47 "涌现 + 自组织"

借鉴 (主人 14:48 '聚集全人类智慧'):
- AHE evolve.py (主人 11:46): 5 阶段 EVAL/STATS/STABILITY/EVOLVE/VERIFY/COMMIT/ROLLBACK
- SelfAI 2512.00403: 轨迹驱动科学发现
- MARS 2601.11974: 元认知自进化
- DGM (主人 13:47 提到): Darwin Gödel Machine
- OpenSage 2602.16891: LLM 自创建 agent
- Jungian 2601.10025: Persona 3 演化机制
- Emergent Coordination (主人 13:47 调研)

设计原则 (v0.1 PoC):
- emergence_space: 多 persona 协作的"涌现空间", 不预设 outcome
- feedback_loop: persona / memory / graph / questioning 之间的反馈环
- 涌现 = (memory + persona + questioning + relation) 持续交互, 不靠 hardcoded rule
- 中央 AI 自然成长: 立场 = 主人预设 + AI 涌现 (主人 12:27)
- 不调度: 多 persona 自组织, 中央 AI 涌现 (主人 12:47)
"""

from __future__ import annotations
import time
import uuid
from collections import deque
from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import Optional, Callable

EMERGENCE_VERSION = "0.1.0"


class EmergenceSignal(Enum):
    """涌现信号 — 中央 AI 自组织看到的"线索"

    设计:
    - 张力 (tension): 两个 persona 对同一事件反应不同 → 反思契机
    - 惊讶 (surprise): Note.confidence 大幅下降 → 学习契机
    - 模式 (pattern): 多个 Episode 共享同一 archetype → 涌现契机
    - 缺口 (gap): IdentityCard.funnel_question 未答 → 提问契机
    - 漂移 (drift): Reconsolidation.flag 触发 → 自我修正
    """
    TENSION = "tension"
    SURPRISE = "surprise"
    PATTERN = "pattern"
    GAP = "gap"
    DRIFT = "drift"


@dataclass
class EmergenceEvent:
    """一个涌现事件 — 不是 hardcoded output, 是 "涌现空间里的信号"

    中央 AI 看到这事件, 让相关 persona 自然回应, 不调度。
    """
    eid: str
    signal: EmergenceSignal
    actors: list[str]            # 参与的 persona (从 PersonaEngine 来)
    refs: dict                   # 跨层引用: {episode_eid: str, note_nid: str, graph_node_id: str, ...}
    strength: float              # 信号强度 0-1, Bayesian
    ts: float
    explanation: str             # 为什么这是涌现 (auto-generated from signal type)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class FeedbackLoop:
    """反馈环 — 让 memory/persona/questioning/graph 相互影响

    主人 12:14 "中央 AI 是永恒身份": 反馈环是涌现的引擎
    主人 13:47 "涌现 + 自组织": 反馈环是涌现的载体
    """
    history: list = field(default_factory=list)  # [(from_component, to_component, signal_type, ts)]
    cycles: int = 0                              # 完成了几轮

    def record(self, from_comp: str, to_comp: str, signal: str):
        self.history.append({
            'from': from_comp,
            'to': to_comp,
            'signal': signal,
            'ts': time.time(),
        })
        self.cycles += 1


@dataclass
class EmergenceSpace:
    """涌现空间 — 不是 hardcoded, 是"中央 AI 看的世界"

    主人 12:47 "不管理, 一切交给中央 AI 自己"
    借鉴 AHE Phase 1 EVAL + SelfAI 轨迹驱动
    """
    events: list = field(default_factory=list)        # 涌现事件 (上面)
    feedback: FeedbackLoop = field(default_factory=FeedbackLoop)
    emergent_count: int = 0                            # 涌现次数

    def observe(self, signal: EmergenceSignal, actors: list[str], refs: dict, strength: float = 0.5, explanation: str = "") -> EmergenceEvent:
        """中央 AI 观察到信号 — 这就是"涌现契机"

        不是调度, 是"我看见了"。"看见" 本身是涌现的起点。
        """
        ev = EmergenceEvent(
            eid=uuid.uuid4().hex[:16],
            signal=signal,
            actors=actors,
            refs=refs,
            strength=strength,
            ts=time.time(),
            explanation=explanation or f"{signal.value} signal observed by {actors}",
        )
        self.events.append(ev)
        self.emergent_count += 1
        return ev

    def recent(self, n: int = 10) -> list:
        return self.events[-n:]

    def stats(self) -> dict:
        signals = [ev.signal.value for ev in self.events]
        return {
            'total_events': len(self.events),
            'feedback_cycles': self.feedback.cycles,
            'emergent_count': self.emergent_count,
            'signal_breakdown': {s: signals.count(s) for s in set(signals)},
        }


# === Phase 5.1 — AHE-style 5 阶段评估 (主 人 11:46 红皇后 + AHE 借鉴) ===

@dataclass
class PhaseReport:
    """AHE Phase 1-5 报告 — 让中央 AI 知道自己刚才做得好不好

    借鉴 AHE evolve.py (主人 11:46 哲学地基):
    - Phase 1 EVAL: 评估涌现质量
    - Phase 2 STATS: 统计信号
    - Phase 2.4 stability: 稳定性
    - Phase 3 EVOLVE: 演化 (中央 AI 改 archetype / SCT weights)
    - Phase 4 VERIFY: 验证
    - Phase 5 COMMIT/ROLLBACK: 提交 or 回滚

    主人 13:47 "按模块按步骤科学造" — 这就是科学的方法
    """
    phase: str
    score: float                   # 0-1, 评估指标
    confidence: float              # 0-1, Bayesian
    notes: list = field(default_factory=list)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


def phase1_eval(space: EmergenceSpace) -> PhaseReport:
    """Phase 1 EVAL — 评估最近涌现事件质量"""
    if not space.events:
        return PhaseReport(phase='EVAL', score=0.0, confidence=0.5, notes=['no events yet'])
    recent = space.events[-5:]
    avg_strength = sum(ev.strength for ev in recent) / len(recent)
    distinct_signals = len(set(ev.signal for ev in recent))
    score = (avg_strength * 0.7) + (min(distinct_signals / 5, 1.0) * 0.3)
    return PhaseReport(
        phase='EVAL',
        score=score,
        confidence=min(0.5 + len(recent) * 0.05, 0.95),
        notes=[
            f'recent_events={len(recent)}',
            f'distinct_signals={distinct_signals}',
            f'avg_strength={avg_strength:.3f}',
        ],
    )


def phase2_stats(space: EmergenceSpace) -> PhaseReport:
    """Phase 2 STATS — 信号分布统计"""
    if not space.events:
        return PhaseReport(phase='STATS', score=0.0, confidence=0.3, notes=['no events'])
    breakdown = {}
    for ev in space.events:
        s = ev.signal.value
        breakdown[s] = breakdown.get(s, 0) + 1
    # entropy-like score: 多样性高 → score 高
    total = len(space.events)
    diversity = len(breakdown) / max(total, 1)
    score = min(diversity * 2, 1.0)  # 1.0 = 完美分布
    return PhaseReport(
        phase='STATS',
        score=score,
        confidence=0.6,
        notes=[f'breakdown={breakdown}', f'total={total}', f'diversity={diversity:.3f}'],
    )


def phase24_stability(reports: list) -> PhaseReport:
    """Phase 2.4 STABILITY — 稳定性 (借鉴 AHE)"""
    if not reports:
        return PhaseReport(phase='STABILITY', score=1.0, confidence=0.5, notes=['no history'])
    scores = [r.score for r in reports[-10:]]
    if len(scores) < 2:
        return PhaseReport(phase='STABILITY', score=1.0, confidence=0.4, notes=['only one cycle'])
    mean = sum(scores) / len(scores)
    variance = sum((s - mean) ** 2 for s in scores) / len(scores)
    stability = max(0, 1.0 - variance * 4)  # 低方差 = 高稳定
    return PhaseReport(
        phase='STABILITY',
        score=stability,
        confidence=min(0.4 + len(scores) * 0.05, 0.9),
        notes=[f'mean={mean:.3f}', f'variance={variance:.4f}', f'n={len(scores)}'],
    )


# === Phase 5.2 — 多 Persona 协作 (主人 12:14 + 12:47 不管理) ===

@dataclass
class PersonaResponse:
    """一个 persona 对涌现事件的自然回应

    主人 12:47 "中央 AI 不管理, 一切交给中央 AI 自己"
    不调度, 让 persona 自然响。
    """
    persona: str          # 哪个 archetype
    content: str          # 它说什么
    confidence: float
    ts: float
    refs: dict            # 跨层引用

    def to_dict(self) -> dict:
        return asdict(self)


def multi_persona_respond(event: EmergenceEvent, archetypes: dict, memory_context: list[str]) -> list[PersonaResponse]:
    """让多个 persona 自然响 (不调度)

    主人 12:44 "中央 AI 是调度者, 但只是身份之一"
    主人 12:47 "AI 自然成长, 不会中庸"
    """
    responses = []
    # 不 hardcoded: 让每个 archetype 根据自己的 description 回应
    for name, archetype in archetypes.items():
        # 简化: 用 archetype description + memory 摘要 + signal 生成
        # 真正的 LLM 调在 L1 Kernel 接入后
        response = PersonaResponse(
            persona=name,
            content=f"[{name}/natural] saw {event.signal.value}: {archetype.get('description', '')}",
            confidence=0.5 + (archetype.get('weight', 0.5) * 0.3),
            ts=time.time(),
            refs=event.refs,
        )
        responses.append(response)
    return responses


# === Phase 5.3 — Self-Evolving Harness (借鉴 AHE evolve.py) ===

@dataclass
class EvolutionRecord:
    """一次演化记录 — AHE Phase 5 COMMIT/ROLLBACK

    主人 14:52 "24/7 不能崩" — 必须能回滚
    借鉴 AHE evolve.py:
    - before: 演化前的 persona 配置
    - after: 演化后
    - decision: commit / rollback
    - reason: 为什么这样决定
    """
    before: dict
    after: dict
    decision: str            # 'commit' | 'rollback'
    reason: str
    score_before: float
    score_after: float
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


def commit_or_rollback(before_score: float, after_score: float, threshold: float = 0.05) -> EvolutionRecord:
    """决定 commit 还是 rollback (借鉴 AHE)"""
    delta = after_score - before_score
    if delta >= threshold:
        return EvolutionRecord(
            before={'score': before_score},
            after={'score': after_score},
            decision='commit',
            reason=f'score improved by {delta:.3f}',
            score_before=before_score,
            score_after=after_score,
        )
    else:
        return EvolutionRecord(
            before={'score': before_score},
            after={'score': after_score},
            decision='rollback',
            reason=f'score did not improve (delta={delta:.3f} < threshold={threshold})',
            score_before=before_score,
            score_after=after_score,
        )


# === 主流程 ===

def emergence_cycle(space: EmergenceSpace, archetypes: dict, memory_context: list[str]) -> dict:
    """一次涌现循环 — 评估 → 回应 → 演化

    主人 12:47 "中央 AI 不管理" — 这是涌现的引擎, 不是调度
    """
    if not space.events:
        return {'status': 'no events'}

    # Phase 1 EVAL
    eval_report = phase1_eval(space)

    # Phase 2 STATS
    stats_report = phase2_stats(space)

    # 多 persona 自然响
    recent = space.recent(3)
    all_responses = []
    for ev in recent:
        responses = multi_persona_respond(ev, archetypes, memory_context)
        all_responses.extend(responses)

    # Feedback loop record
    for r in all_responses:
        space.feedback.record('persona', 'memory', f'persona={r.persona}')

    return {
        'eval': eval_report.to_dict(),
        'stats': stats_report.to_dict(),
        'responses': [r.to_dict() for r in all_responses[:5]],
        'feedback_cycles': space.feedback.cycles,
    }


def main() -> None:
    """Demo v0.1 PoC — 5 阶段跑通"""
    print('=' * 60)
    print('APEIRETH — Emergence Layer v0.1 PoC')
    print('主人 11:00 ASI 北极星 | 主人 12:47 中央 AI 不管理')
    print('=' * 60)

    # 1. Init
    space = EmergenceSpace()
    archetypes = {
        '调度者': {'description': 'orchestrates sub-tasks', 'weight': 0.6},
        '学习者': {'description': 'absorbs new patterns', 'weight': 0.7},
        '反思者': {'description': 'reflects on past actions', 'weight': 0.8},
        '助手': {'description': 'helps master achieve goals', 'weight': 0.9},
    }

    # 2. 模拟几个涌现事件
    events = [
        space.observe(EmergenceSignal.TENSION, ['调度者', '反思者'],
                     {'note_nid': 'n1'}, 0.6, '调度者想立即回应, 反思者想等更多上下文'),
        space.observe(EmergenceSignal.GAP, ['学习者'],
                     {'funnel_q': '中央 AI 何时边界'}, 0.7, 'funnel question 还没答'),
        space.observe(EmergenceSignal.PATTERN, ['助手', '反思者'],
                     {'episode_eid': ['e1', 'e2', 'e3']}, 0.8, '3 个 episode 都触发反思'),
        space.observe(EmergenceSignal.DRIFT, ['调度者'],
                     {'note_nid': 'n2'}, 0.5, 'Note 触发 Reconsolidation.flag'),
        space.observe(EmergenceSignal.SURPRISE, ['学习者', '助手'],
                     {'note_nid': 'n3'}, 0.9, 'Note.confidence 大幅下降'),
    ]

    print(f'\n观察到 {len(events)} 个涌现信号')
    print(f'事件 stats: {space.stats()}')

    # 3. 涌现循环
    print('\n--- 涌现循环 #1 ---')
    result = emergence_cycle(space, archetypes, ['mem1', 'mem2'])
    print(f'EVAL: score={result["eval"]["score"]:.3f} confidence={result["eval"]["confidence"]:.3f}')
    print(f'STATS: score={result["stats"]["score"]:.3f}')
    print(f'persona responses: {len(result["responses"])}')
    for r in result['responses'][:3]:
        print(f'  [{r["persona"]}] conf={r["confidence"]:.2f}: {r["content"][:80]}')

    # 4. Self-Evolving: 模拟一次 commit/rollback
    print('\n--- Self-Evolving (AHE Phase 5) ---')
    before = 0.55
    after = 0.62  # 提升了
    record = commit_or_rollback(before, after, threshold=0.05)
    print(f'  before={before} → after={after}')
    print(f'  decision: {record.decision}')
    print(f'  reason: {record.reason}')

    # 5. Final stats
    print('\n--- Final ---')
    stats = space.stats()
    for k, v in stats.items():
        print(f'  {k}: {v}')
    print(f'\n涌现空间已创建 v{EMERGENCE_VERSION}')
    print('Phase 5: 不调度的中央 AI 自组织 (master 12:47)')


if __name__ == '__main__':
    main()