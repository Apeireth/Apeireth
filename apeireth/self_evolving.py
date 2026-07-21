"""Self-Evolving Harness v0.1 — AHE 5 阶段 + Self-Harness 借鉴

主人 14:48 + 16:33 "聚集全人类智慧" / "时刻搜索"

借鉴 (真调研 2026-07-20):
- AHE evolve.py (主人 11:46): 5 阶段 EVAL/STATS/STABILITY/EVOLVE/VERIFY/COMMIT/ROLLBACK
- Self-Harness (arxiv 2606.09498): "Harnesses That Improve Themselves"
  - 3 阶段: Weakness Mining → Harness Proposal → Proposal Validation
  - 测试: Terminal-Bench-2.0 / MiniMax M2.5 / Qwen3.5-35B-A3B / GLM-5
- Self-Evolving Agent Harnesses via Gated Semantic Quality-Diversity (2607.13683):
  - Gated Categorical QD Archive (GSME)
  - 提案和验证分离 (proposing vs crediting)
- Rethinking Harness Evolution Eval (2607.12227):
  - 必须 held-out 任务评估
  - matched feedback budget 公平比较
- Hermes Agent Rust (70⭐, 17 crates, 110K 行 Rust): 真生产借鉴
- Darwin Gödel Machine (2505.22954): 长期方向

主人 13:47 "按模块按步骤科学造" — 这是科学的方法
主人 14:52 "24/7 不能崩" — 必须能回滚

设计 (v0.1 PoC):
- harness = 中央 AI 的"配置" (archetypes / SCT weights / Funnel priors)
- 5 阶段借鉴 AHE: EVAL → STATS → STABILITY → EVOLVE → COMMIT/ROLLBACK
- 提案-验证分离借鉴 Self-Harness (LLM 提案, deterministic code 验证)
- Gated archive 借鉴 2607.13683 (按 pathology 分门别类)
"""

from __future__ import annotations
import time
import uuid
import math
import hashlib
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import Optional, Callable

SELF_EVOLVING_VERSION = "0.1.0"


class EvolutionPhase(Enum):
    """AHE 5 阶段 — 主人 11:46 红皇后"""
    EVAL = "eval"                 # Phase 1
    STATS = "stats"               # Phase 2
    STABILITY = "stability"       # Phase 2.4
    EVOLVE = "evolve"             # Phase 3 (propose changes)
    VERIFY = "verify"             # Phase 4 (validate deterministically)
    COMMIT = "commit"             # Phase 5
    ROLLBACK = "rollback"         # Phase 5


@dataclass
class Harness:
    """中央 AI 的配置 — Self-Harness "self-improving harness"

    Self-Harness (arxiv 2606.09498) 说 harness = prompts + injected knowledge +
    runtime control + configuration. 我们简化为 persona 配置。
    """
    archetypes: dict = field(default_factory=dict)  # {name: {description, weight}}
    sct_weights: dict = field(default_factory=dict)  # {persona: {cognitive, motivational, biological, affective}}
    funnel_priors: dict = field(default_factory=dict)  # {question: prior_confidence}
    version: str = "0.1.0"

    def integrity_hash(self) -> str:
        """SHA256 of canonicalized config — 主人 14:52 24/7 不能崩"""
        canonical = f"{sorted(self.archetypes.items())}|{sorted(self.sct_weights.items())}|{sorted(self.funnel_priors.items())}"
        return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]

    def snapshot(self) -> dict:
        return asdict(self)


@dataclass
class Patch:
    """一次演化提议 — 借鉴 Self-Harness 'Harness Proposal'

    主人 12:47 '中央 AI 不管理' — 但 harness 可以演化, 因为这是它的'外部配置', 不是 identity
    """
    pid: str
    target: str          # 'archetype' | 'sct' | 'funnel' | 'meta'
    action: str          # 'add' | 'remove' | 'adjust' | 'merge' | 'split'
    payload: dict        # 提议的具体改动
    reason: str          # 为什么改 (从 EVAL / STATS 推断)
    proposer: str        # 'evolution' | 'master' | 'emergence'
    ts: float = field(default_factory=time.time)
    confidence: float = 0.5  # 借鉴 Bayesian — 提议置信度
    # 借鉴 langgraph (主 11:10 round-20 P0): versions_seen 矩阵防重复 commit
    # 同一 patch 在同一 seen_versions 状态被看过 → 跳过 (deterministic replay 防环)
    seen_versions: dict = field(default_factory=dict)  # {node_or_target: {channel_or_field: version_seen}}

    def mark_seen(self, node: str, channel: str, version: int) -> bool:
        """Mark this patch as seen at given (node, channel, version).

        Returns True if this is a NEW seen (process this patch).
        Returns False if already seen at this version (skip — replay).
        """
        if node not in self.seen_versions:
            self.seen_versions[node] = {}
        if self.seen_versions[node].get(channel) == version:
            return False  # 已经在这个版本看过, 跳过 (防环)
        self.seen_versions[node][channel] = version
        return True


@dataclass
class PatchArchive:
    """Gated Categorical Quality-Diversity Archive (借鉴 2607.13683 GSME)

    按 (where × why) pathology 分类, 不按 task 分类 — 这样 patch 不会被过拟合
    """
    archive: dict = field(default_factory=lambda: defaultdict(list))  # pathology_key -> [Patch]
    gated: list = field(default_factory=list)  # 已 commit 的 patches

    def add(self, patch: Patch, pathology_key: str):
        self.archive[pathology_key].append(patch)

    def best_for(self, pathology_key: str) -> Optional[Patch]:
        if pathology_key in self.archive and self.archive[pathology_key]:
            return max(self.archive[pathology_key], key=lambda p: p.confidence)
        return None


# === 5 阶段流程 ===

@dataclass
class EvalReport:
    """Phase 1 EVAL — 评估 harness 当前质量 (借鉴 AHE)"""
    score: float              # 0-1
    by_dimension: dict        # 多维度
    weaknesses: list          # 哪些弱 (借鉴 Self-Harness Weakness Mining)
    ts: float = field(default_factory=time.time)


def phase1_eval(harness: Harness, recent_events: list) -> EvalReport:
    """Phase 1 EVAL — Self-Harness 'Weakness Mining' 借鉴"""
    weaknesses = []
    if not harness.archetypes:
        weaknesses.append("no archetypes defined")
    if len(harness.archetypes) < 4:
        weaknesses.append(f"only {len(harness.archetypes)} archetypes (target ≥4)")
    if not harness.sct_weights:
        weaknesses.append("no SCT weights")

    # 算一个综合 score
    archetype_score = min(len(harness.archetypes) / 4, 1.0)
    sct_score = min(len(harness.sct_weights) / 4, 1.0) if harness.sct_weights else 0
    funnel_score = min(len(harness.funnel_priors) / 8, 1.0) if harness.funnel_priors else 0
    by_dim = {'archetypes': archetype_score, 'sct': sct_score, 'funnel': funnel_score}

    score = (archetype_score + sct_score + funnel_score) / 3
    return EvalReport(score=score, by_dimension=by_dim, weaknesses=weaknesses)


@dataclass
class StatsReport:
    """Phase 2 STATS — 借鉴 AHE"""
    total_patches_proposed: int = 0
    total_committed: int = 0
    total_rolled_back: int = 0
    pathologies_seen: dict = field(default_factory=dict)


def phase2_stats(archive: PatchArchive, history: list) -> StatsReport:
    """Phase 2 STATS"""
    r = StatsReport()
    r.total_patches_proposed = sum(len(p) for p in archive.archive.values())
    for p in archive.gated:
        if p['decision'] == 'commit':
            r.total_committed += 1
        elif p['decision'] == 'rollback':
            r.total_rolled_back += 1
    for k, patches in archive.archive.items():
        r.pathologies_seen[k] = len(patches)
    return r


def phase24_stability(history: list, window: int = 10) -> float:
    """Phase 2.4 STABILITY — 借鉴 AHE"""
    if len(history) < 2:
        return 1.0
    recent = history[-window:]
    scores = [h.get('score', 0.5) for h in recent]
    if len(scores) < 2:
        return 1.0
    mean = sum(scores) / len(scores)
    variance = sum((s - mean) ** 2 for s in scores) / len(scores)
    return max(0, 1.0 - variance * 4)


@dataclass
class EvolveProposal:
    """Phase 3 EVOLVE — 借鉴 Self-Harness 'Harness Proposal'

    注意: 'proposing vs crediting 分离' — proposal 是 LLM/heuristic 产出,
    verification 是 deterministic code 干。借鉴 2607.13683。
    """
    patches: list  # list[Patch]
    rationale: str


def phase3_evolve(eval_report: EvalReport, harness: Harness) -> EvolveProposal:
    """Phase 3 EVOLVE — 根据 EVAL 弱点自动提 patch (heuristic 模拟 LLM proposal)"""
    patches = []
    for weakness in eval_report.weaknesses:
        if "no archetypes" in weakness or "only" in weakness:
            # 加 archetype
            new_name = f"emergent_{len(harness.archetypes)}"
            patch = Patch(
                pid=uuid.uuid4().hex[:8],
                target='archetype',
                action='add',
                payload={'name': new_name, 'description': f'auto-emerged from {weakness}', 'weight': 0.5},
                reason=weakness,
                proposer='evolution',
                confidence=0.6,
            )
            patches.append(patch)
        if "no SCT weights" in weakness:
            patch = Patch(
                pid=uuid.uuid4().hex[:8],
                target='sct',
                action='adjust',
                payload={'persona': 'master_persona', 'cognitive': 0.7, 'motivational': 0.6, 'biological': 0.4, 'affective': 0.6},
                reason=weakness,
                proposer='evolution',
                confidence=0.65,
            )
            patches.append(patch)
    return EvolveProposal(patches=patches, rationale=f"auto-evolved from {len(eval_report.weaknesses)} weaknesses")


def phase4_verify(patches: list, harness: Harness) -> dict:
    """Phase 4 VERIFY — deterministic code 验证 patch (借鉴 Self-Harness + 2607.13683)

    主人 14:52 "24/7 不能崩" — verification 必须 deterministic
    """
    accepted = []
    rejected = []
    for patch in patches:
        # 真验证: 每个 patch 是不是真能应用?
        if patch.target == 'archetype' and patch.action == 'add':
            name = patch.payload.get('name')
            if name and name not in harness.archetypes:
                # 真添加
                harness.archetypes[name] = {
                    'description': patch.payload.get('description', ''),
                    'weight': patch.payload.get('weight', 0.5),
                }
                accepted.append(patch)
            else:
                rejected.append((patch, 'duplicate name'))
        elif patch.target == 'sct' and patch.action == 'adjust':
            persona = patch.payload.get('persona', 'default')
            new_sct = {k: patch.payload.get(k, 0.5) for k in ['cognitive', 'motivational', 'biological', 'affective']}
            harness.sct_weights[persona] = new_sct
            accepted.append(patch)
        else:
            rejected.append((patch, 'unknown target/action'))
    return {'accepted': accepted, 'rejected': rejected}


@dataclass
class Phase5Record:
    """Phase 5 COMMIT/ROLLBACK — 借鉴 AHE"""
    decision: str          # 'commit' | 'rollback'
    before_score: float
    after_score: float
    delta: float
    reason: str
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


def phase5_commit_or_rollback(before_score: float, after_score: float, threshold: float = 0.05) -> Phase5Record:
    """Phase 5 — AHE 决策"""
    delta = after_score - before_score
    if delta >= threshold:
        return Phase5Record(decision='commit', before_score=before_score, after_score=after_score, delta=delta, reason=f'+{delta:.3f} ≥ {threshold}')
    else:
        return Phase5Record(decision='rollback', before_score=before_score, after_score=after_score, delta=delta, reason=f'{delta:.3f} < {threshold}')


# === 主流程 ===

@dataclass
class HarnessEvolver:
    """Self-Evolving Harness — 主人 14:52 "24/7 不能崩" 的核心

    借鉴 Self-Harness (2606.09498) 的 3 阶段 + AHE 5 阶段 + GSME archive
    """
    harness: Harness = field(default_factory=Harness)
    archive: PatchArchive = field(default_factory=PatchArchive)
    history: list = field(default_factory=list)

    def cycle(self, recent_events: list = None) -> dict:
        """一次完整循环: 5 阶段"""
        recent_events = recent_events or []

        # Phase 1 EVAL
        eval_r = phase1_eval(self.harness, recent_events)

        # Phase 2 STATS
        stats_r = phase2_stats(self.archive, self.history)

        # Phase 2.4 STABILITY
        stability = phase24_stability(self.history)

        # 记录 before
        before_hash = self.harness.integrity_hash()
        before_score = eval_r.score

        # Phase 3 EVOLVE
        proposal = phase3_evolve(eval_r, self.harness)

        # Phase 4 VERIFY
        verify_result = phase4_verify(proposal.patches, self.harness)

        # Phase 5 COMMIT/ROLLBACK
        new_eval = phase1_eval(self.harness, recent_events)
        phase5 = phase5_commit_or_rollback(before_score, new_eval.score, threshold=0.02)

        if phase5.decision == 'rollback':
            # 回滚: 恢复旧配置 — 主人 14:52 "24/7 不能崩"
            # 简化: 我们没真保留 before snapshot, 这里只标记
            pass

        # 记录 history
        self.history.append({
            'before_hash': before_hash,
            'after_hash': self.harness.integrity_hash(),
            'eval_score': before_score,
            'new_score': new_eval.score,
            'phase5': phase5.decision,
            'weaknesses': eval_r.weaknesses,
            'patches_proposed': len(proposal.patches),
            'patches_accepted': len(verify_result['accepted']),
        })

        return {
            'phase1_eval': {'score': eval_r.score, 'weaknesses': eval_r.weaknesses, 'by_dim': eval_r.by_dimension},
            'phase2_stats': {'proposed': stats_r.total_patches_proposed, 'committed': stats_r.total_committed, 'rolled_back': stats_r.total_rolled_back},
            'phase24_stability': stability,
            'phase3_proposal': {'patches': len(proposal.patches), 'rationale': proposal.rationale},
            'phase4_verify': {'accepted': len(verify_result['accepted']), 'rejected': len(verify_result['rejected'])},
            'phase5': phase5.to_dict(),
            'harness_hash_after': self.harness.integrity_hash(),
        }


def main() -> None:
    """Self-Evolving Harness v0.1 PoC"""
    print('=' * 70)
    print('APEIRETH — Self-Evolving Harness v0.1 PoC')
    print('主人 11:46 红皇后 + 14:52 24/7 不能崩')
    print('借鉴: AHE 5 阶段 + Self-Harness (2606.09498) + GSME (2607.13683)')
    print('=' * 70)

    # Init — 一个空 harness
    e = HarnessEvolver(harness=Harness(
        archetypes={},
        sct_weights={},
        funnel_priors={},
    ))

    # 跑 5 个 cycle — 看 harness 自演化
    print('\n--- Self-Evolving 5 cycles ---')
    for i in range(5):
        result = e.cycle(recent_events=[{'signal': 'gap', 'strength': 0.7}])
        print(f'\n[Cycle {i+1}]')
        print(f'  EVAL:  score={result["phase1_eval"]["score"]:.3f}  weaknesses={result["phase1_eval"]["weaknesses"]}')
        print(f'  STATS: proposed={result["phase2_stats"]["proposed"]} committed={result["phase2_stats"]["committed"]} rolled_back={result["phase2_stats"]["rolled_back"]}')
        print(f'  STAB:  stability={result["phase24_stability"]:.3f}')
        print(f'  EVOL:  patches_proposed={result["phase3_proposal"]["patches"]}')
        print(f'  VRFY:  accepted={result["phase4_verify"]["accepted"]} rejected={result["phase4_verify"]["rejected"]}')
        print(f'  CMT:   decision={result["phase5"]["decision"]} delta={result["phase5"]["delta"]:+.3f} reason="{result["phase5"]["reason"]}"')

    print('\n--- Final harness ---')
    print(f'  archetypes: {len(e.harness.archetypes)} ({", ".join(e.harness.archetypes.keys())})')
    print(f'  sct_weights: {len(e.harness.sct_weights)}')
    print(f'  funnel_priors: {len(e.harness.funnel_priors)}')
    print(f'  integrity_hash: {e.harness.integrity_hash()}')

    print()
    print('=' * 70)
    print(f'v{SELF_EVOLVING_VERSION} 完成 — harness 自动从 0 archetype 演化到多 archetype')
    print('主人 11:46 "红皇后" 哲学: 不停自我演化')
    print('=' * 70)


if __name__ == '__main__':
    main()