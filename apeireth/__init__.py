"""Apeireth — ASI 地基平台
Phase 1 (Week 1-2): Identity Store v0.1 PoC
Phase 1.5 (今日): AnySearch 联网集成 (L2 Interaction Layer)
Phase 2 (Week 3-4): Memory Layer v0.1 PoC
作者: 楚零 | 命名: 主人 2026-07-20
"""

from .identity import IdentityCard, load_card, save_card
from .kickoff import KICKOFF_QUESTIONS, run_kickoff
from .research import AnySearch
from .github import GitHubResearch
from .memory import (
    Episode,
    Note,
    MemoryStore,
    save_store,
    load_store,
    forget_sweep,
    reconsolidate,
)

__version__ = "0.3.0"
__all__ = [
    # Phase 1: Identity
    "IdentityCard",
    "load_card",
    "save_card",
    "KICKOFF_QUESTIONS",
    "run_kickoff",
    # Phase 1.5: Interaction
    "AnySearch",
    "GitHubResearch",
    # Phase 2: Memory
    "Episode",
    "Note",
    "MemoryStore",
    "save_store",
    "load_store",
    "forget_sweep",
    "reconsolidate",
] 
