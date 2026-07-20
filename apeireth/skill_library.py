"""Phase 13 Skill Library — Voyager-inspired skill persistence + curriculum.

主人 12:14 "干什么就组一个什么的专家团"
主人 18:07 "不吝借用好东西" — Voyager 真生产 (arxiv 2305.16291)

3 真生产组件借鉴:
  1. automatic curriculum: task difficulty auto-adjustment (curriculum.py)
  2. skill library: persistent executable skills (skill_library.py)
  3. iterative prompting: error → improve → re-add (curator.py)

Apeireth Phase 13 适配:
  - 技能 = Python callable (真生产, 不只是 prompt)
  - 库 = SQLite-backed (主人 16:44 "local-first")
  - curriculum = CuriosityScore-driven (复用 ProactiveLoop)
"""
from __future__ import annotations

import json
import time
import uuid
import inspect
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional, Callable


SKILL_LIBRARY_VERSION = "0.1.0"


@dataclass
class Skill:
    """A persistent executable skill (Voyager-inspired).

    与 SelfOrgTeam 的区别:
      - Skill = 可执行 callable, 持久化, 可检索
      - Team = 临时团, 短期, 不持久化成员
    """
    skill_id: str
    name: str
    description: str
    code: str                       # Python source (simplified)
    func: Optional[object] = field(default=None, repr=False)   # 实际 callable
    tags: list[str] = field(default_factory=list)
    use_count: int = 0
    success_count: int = 0
    fail_count: int = 0
    created_at: float = field(default_factory=time.time)
    last_used_at: float = 0.0
    version: int = 1
    parent_skill_id: Optional[str] = None    # 借鉴关系

    def to_dict(self) -> dict:
        d = asdict(self)
        d.pop("func", None)
        return d

    @property
    def success_rate(self) -> float:
        if self.use_count == 0:
            return 0.0
        return self.success_count / self.use_count


@dataclass
class SkillLibrary:
    """Local-first persistent skill library (Voyager-inspired).

    SQLite-backed storage — apeireth/data/skills.db (主人 16:44 "local-first").
    """
    library_id: str = field(default_factory=lambda: uuid.uuid4().hex[:12])
    skills: dict[str, Skill] = field(default_factory=dict)   # skill_id -> Skill
    tags_index: dict[str, set] = field(default_factory=dict)   # tag -> set of skill_ids
    db_path: Optional[str] = None

    def add(self, skill: Skill) -> str:
        """Add a skill (curator-style)."""
        if skill.skill_id in self.skills:
            # v0.1 — duplicate, increment version
            skill.version += 1
            skill.skill_id = f"{skill.skill_id}_v{skill.version}"
        self.skills[skill.skill_id] = skill
        # Index tags
        for tag in skill.tags:
            self.tags_index.setdefault(tag, set()).add(skill.skill_id)
        return skill.skill_id

    def retrieve_by_tag(self, tag: str) -> list[Skill]:
        """Retrieve skills by tag (Voyager's similarity-based retrieval simplified)."""
        sids = self.tags_index.get(tag, set())
        return [self.skills[sid] for sid in sids if sid in self.skills]

    def retrieve_relevant(self, query: str, topk: int = 5) -> list[Skill]:
        """Retrieve relevant skills by keyword matching (simplified Voyager retrieval)."""
        query_words = set(query.lower().split())
        scored = []
        for s in self.skills.values():
            name_words = set(s.name.lower().split())
            desc_words = set(s.description.lower().split())
            tag_words = set(s.tags)
            score = len(query_words & (name_words | desc_words | tag_words))
            if score > 0:
                scored.append((score, s))
        scored.sort(key=lambda x: -x[0])
        return [s for _, s in scored[:topk]]

    def use(self, skill_id: str, success: bool) -> None:
        """Record skill use (Voyager's success counter)."""
        if skill_id in self.skills:
            s = self.skills[skill_id]
            s.use_count += 1
            if success:
                s.success_count += 1
            else:
                s.fail_count += 1
            s.last_used_at = time.time()

    def save(self, path: str = None) -> None:
        """Save library to disk (JSON for v0.1)."""
        path = path or self.db_path or "skill_library.json"
        data = {
            "library_id": self.library_id,
            "skills": {sid: s.to_dict() for sid, s in self.skills.items()},
            "tags_index": {t: list(sids) for t, sids in self.tags_index.items()},
        }
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
        self.db_path = path

    @classmethod
    def load(cls, path: str) -> "SkillLibrary":
        """Load library from disk."""
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        lib = cls(library_id=data["library_id"], db_path=path)
        for sid, sd in data["skills"].items():
            lib.skills[sid] = Skill(**sd)
        for t, sids in data["tags_index"].items():
            lib.tags_index[t] = set(sids)
        return lib

    def stats(self) -> dict:
        return {
            "library_id": self.library_id,
            "n_skills": len(self.skills),
            "n_tags": len(self.tags_index),
            "total_uses": sum(s.use_count for s in self.skills.values()),
            "avg_success_rate": sum(s.success_rate for s in self.skills.values()) / max(1, len(self.skills)),
        }


# === Convenience ===

def make_default_skill_library(path: str = "skill_library.json") -> SkillLibrary:
    return SkillLibrary(db_path=path)


# === Built-in seed skills (Apeireth 真生产 starter set) ===

def install_seed_skills(lib: SkillLibrary) -> None:
    """Install 5 seed skills that every Apeireth needs."""
    seeds = [
        Skill(
            skill_id="research_summarize",
            name="research_summarize",
            description="Summarize a long-form research output into key findings + citations.",
            code="def run(query): ...",  # placeholder
            tags=["research", "summary", "v1"],
        ),
        Skill(
            skill_id="reflect_summarize",
            name="reflect_summarize",
            description="Reflect on a recent action sequence and extract meta-lessons.",
            code="def run(trace): ...",
            tags=["reflection", "meta", "v1"],
        ),
        Skill(
            skill_id="plan_decompose",
            name="plan_decompose",
            description="Decompose a high-level goal into atomic executable tasks.",
            code="def run(goal): ...",
            tags=["planning", "decomposition", "v1"],
        ),
        Skill(
            skill_id="debug_artifact",
            name="debug_artifact",
            description="Diagnose failures in artifact output and propose fixes.",
            code="def run(artifact): ...",
            tags=["debug", "fix", "v1"],
        ),
        Skill(
            skill_id="memory_consolidate",
            name="memory_consolidate",
            description="Consolidate recent episodes into stable Notes (reconsolidation).",
            code="def run(episodes): ...",
            tags=["memory", "consolidation", "v1"],
        ),
    ]
    for s in seeds:
        lib.add(s)


__all__ = [
    "SKILL_LIBRARY_VERSION",
    "Skill",
    "SkillLibrary",
    "make_default_skill_library",
    "install_seed_skills",
]