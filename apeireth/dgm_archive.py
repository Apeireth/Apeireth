"""Phase 14 DGM Archive — Multi-Generation Harness Evolution.

DGM (arxiv 2505.22954) — Darwin Gödel Machine:
  - archive of generated coding agents (open-ended tree)
  - empirical validation each change
  - 持续演化不收敛

Apeireth Phase 14: archive of Harness generations
  - 每代 commit 写一份 snapshot
  - eval 验证 (commit or rollback)
  - 永远演化不收敛

不重复 HarnessEvolver — DGM archive 加 multi-generation tree branch + 跨代继承.
"""
from __future__ import annotations

import time
import uuid
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from .self_evolving import Harness, Patch


DGM_ARCHIVE_VERSION = "0.1.0"


@dataclass
class Generation:
    """One generation in the DGM archive tree (Sodoff-inspired)."""
    gen_id: str
    parent_gen_id: Optional[str]   # None = root
    harness: Harness
    patches: list[Patch] = field(default_factory=list)
    eval_score: float = 0.0
    eval_dimensions: dict = field(default_factory=dict)
    created_at: float = field(default_factory=time.time)
    child_gen_ids: list[str] = field(default_factory=list)
    status: str = "active"        # active | frozen | pruned
    # 借鉴 DGM (主 9:41 round-19 source-deep-read): 反收敛核心
    # selection_score = eval_score * 1/(1+children_count) — 鼓励探索低子代节点,
    # 避免 best_score 陷入局部最优. Round-19 推荐下周任务 (本周选 mem0 已落地).
    selection_score: float = 0.0
    # diagnose→fix 分离: proposed fixes (结构化 JSON) 由 reviewer LLM 输出, validator 才 apply
    proposed_fixes: list[dict] = field(default_factory=list)
    applied_fixes: list[dict] = field(default_factory=list)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class DGMArchive:
    """DGM-style archive of harness generations.

    Tree-structured: each gen can have multiple children (branching exploration).
    Owner 17:50 "永远演化 (no completion)" — archive 永远增长.

    借鉴 DGM (主 9:41 round-19 source-deep-read):
      - score_child_prop 反收敛核心: selection_score = eval_score * 1/(1+children_count)
      - diagnose→fix 分离: reviewer LLM 输出 structured proposed_fixes JSON, validator apply
    """
    archive_id: str = field(default_factory=lambda: uuid.uuid4().hex[:12])
    generations: dict[str, Generation] = field(default_factory=dict)
    root_gen_id: Optional[str] = None
    best_gen_id: Optional[str] = None
    db_path: Optional[str] = None

    def init_root(self, harness: Harness) -> str:
        """Initialize archive with root generation."""
        root = Generation(
            gen_id=uuid.uuid4().hex[:12],
            parent_gen_id=None,
            harness=harness,
            eval_score=0.0,
        )
        # root 的 selection_score = eval_score (无子代, 因子 = 1)
        root.selection_score = root.eval_score
        self.generations[root.gen_id] = root
        self.root_gen_id = root.gen_id
        self.best_gen_id = root.gen_id
        return root.gen_id

    def branch(self, parent_gen_id: str, new_harness: Harness, patches: list, eval_score: float, eval_dims: dict) -> str:
        """Add a new generation as branch from parent (DGM core operation).

        借鉴 DGM (主 9:41 round-19): 反收敛 — 用 selection_score 而非 raw eval_score
        """
        parent = self.generations[parent_gen_id]
        new_gen = Generation(
            gen_id=uuid.uuid4().hex[:12],
            parent_gen_id=parent_gen_id,
            harness=new_harness,
            patches=patches,
            eval_score=eval_score,
            eval_dimensions=eval_dims,
        )
        # DGM score_child_prop: selection_score = eval_score * 1/(1+parent.children_count)
        # parent.child_gen_ids 在 append 之前长度, 实际已经是 append 前 count
        # 但 selection_score 取决于父节点已有的 children_count, 而不是新建后
        parent_child_count_before = len(parent.child_gen_ids)
        new_gen.selection_score = eval_score * (1.0 / (1.0 + parent_child_count_before))
        self.generations[new_gen.gen_id] = new_gen
        parent.child_gen_ids.append(new_gen.gen_id)
        # best 仍然按 raw eval_score (selection_score 是探索指标, best 是质量指标)
        if eval_score > self.generations[self.best_gen_id].eval_score:
            self.best_gen_id = new_gen.gen_id
        return new_gen.gen_id

    def select_parent_for_branching(self) -> Optional[str]:
        """DGM 反收敛选择: 选 selection_score 最高的非 root 节点作为 parent.

        Round-19 推荐: 显式鼓励探索低子代节点, 避免陷入 best_score 局部最优.
        Root 永远存在, 但反收敛更倾向探索已生成分支.
        """
        if not self.generations:
            return None
        candidates = [(g.selection_score, gid) for gid, g in self.generations.items() if gid != self.root_gen_id]
        if not candidates:
            return self.root_gen_id  # 只有 root 时返回 root
        candidates.sort(reverse=True)
        return candidates[0][1]

    def propose_fix(self, gen_id: str, fix_proposal: dict) -> None:
        """diagnose→fix 分离: reviewer LLM 输出 structured fix_proposal, 存储到 proposed_fixes.

        Round-19 推荐: 不让 LLM 直接改代码, 而是通过 structured JSON proposal.
        fix_proposal 格式: {"type": "modify_harness", "target": "sct_weights",
                            "key": "调度者", "delta": {"motivational": +0.1},
                            "rationale": "...", "confidence": 0.8}
        validator 后续调 apply_fix() 才 commit.
        """
        if gen_id in self.generations:
            self.generations[gen_id].proposed_fixes.append(fix_proposal)

    def apply_fix(self, gen_id: str, fix_index: int, validator_signature: str = "") -> bool:
        """Apply 第 N 个 proposed fix 到 generation.

        validator_signature: 调用方需提供, 表明已 validate (签名 / sandbox / 单元测试通过).
        Round-19: 严格分离 diagnose + fix, 不让 LLM 跨过 validator.

        Returns: True if applied, False if rejected.
        """
        if gen_id not in self.generations:
            return False
        gen = self.generations[gen_id]
        if fix_index < 0 or fix_index >= len(gen.proposed_fixes):
            return False
        fix = gen.proposed_fixes[fix_index]
        if not validator_signature:
            # No validator signature — reject (主 22:08 V2 中央 AI 是调度者, 必须 validate)
            return False
        gen.applied_fixes.append({**fix, "validator": validator_signature})
        return True

    def get_lineage(self, gen_id: str) -> list[str]:
        """Get lineage (path from root to given gen)."""
        path = []
        cur = gen_id
        while cur:
            path.append(cur)
            cur = self.generations[cur].parent_gen_id
        return list(reversed(path))

    def get_best(self) -> Optional[Generation]:
        if self.best_gen_id:
            return self.generations[self.best_gen_id]
        return None

    def stats(self) -> dict:
        return {
            "archive_id": self.archive_id,
            "n_generations": len(self.generations),
            "best_score": self.generations[self.best_gen_id].eval_score if self.best_gen_id else 0,
            "depth": max((len(self.get_lineage(gid)) for gid in self.generations), default=0),
            "n_branches": sum(len(g.child_gen_ids) for g in self.generations.values()),
            "n_proposed_fixes": sum(len(g.proposed_fixes) for g in self.generations.values()),
            "n_applied_fixes": sum(len(g.applied_fixes) for g in self.generations.values()),
        }

    def save(self, path: str = None) -> None:
        path = path or self.db_path or "dgm_archive.json"
        data = {
            "archive_id": self.archive_id,
            "root_gen_id": self.root_gen_id,
            "best_gen_id": self.best_gen_id,
            "generations": {gid: g.to_dict() for gid, g in self.generations.items()},
        }
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(__import__('json').dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        self.db_path = path


def make_default_dgm_archive() -> DGMArchive:
    return DGMArchive()


__all__ = [
    "DGM_ARCHIVE_VERSION",
    "Generation",
    "DGMArchive",
    "make_default_dgm_archive",
]