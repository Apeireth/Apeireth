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

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class DGMArchive:
    """DGM-style archive of harness generations.

    Tree-structured: each gen can have multiple children (branching exploration).
    Owner 17:50 "永远演化 (no completion)" — archive 永远增长.
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
        self.generations[root.gen_id] = root
        self.root_gen_id = root.gen_id
        self.best_gen_id = root.gen_id
        return root.gen_id

    def branch(self, parent_gen_id: str, new_harness: Harness, patches: list, eval_score: float, eval_dims: dict) -> str:
        """Add a new generation as branch from parent (DGM core operation)."""
        parent = self.generations[parent_gen_id]
        new_gen = Generation(
            gen_id=uuid.uuid4().hex[:12],
            parent_gen_id=parent_gen_id,
            harness=new_harness,
            patches=patches,
            eval_score=eval_score,
            eval_dimensions=eval_dims,
        )
        self.generations[new_gen.gen_id] = new_gen
        parent.child_gen_ids.append(new_gen.gen_id)
        if eval_score > self.generations[self.best_gen_id].eval_score:
            self.best_gen_id = new_gen.gen_id
        return new_gen.gen_id

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