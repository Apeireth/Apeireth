"""Apeireth — ASI 地基平台
Phase 1 (Week 1-2): Identity Store v0.1 PoC
Phase 1.5 (今日): AnySearch 联网集成 (L2 Interaction Layer)
Phase 2 (Week 3-4): Memory Layer v0.1 PoC
Phase 2.5 (今日): SQLite + FTS5 真持久化 + 3-layer search
Phase 3 (今日): Relation Graph v0.1 PoC (L4 Identity Layer 子组件)
Phase 3.5 (今日): Relation Graph v0.2 SQLite 持久化 + 跨 session 存活
Phase 3.6 (今日): Memory ↔ Graph Linker 跨层自动绑定
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
from .memory_store import SqliteMemoryStore, migrate_from_memory_store
from .relation import (
    RelationGraph,
    Node as RNode,
    Edge as REdge,
    save_graph,
    load_graph,
)
from .relation_store import SqliteRelationStore, migrate_from_relation_graph
from .linker import (
    LINKER_VERSION,
    Linker,
    ensure_central_ai_node,
    link_episode,
    link_note,
    sync_all,
)

__version__ = "0.5.0"
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
    # Phase 2: Memory (v0.1 in-memory)
    "Episode",
    "Note",
    "MemoryStore",
    "save_store",
    "load_store",
    "forget_sweep",
    "reconsolidate",
    # Phase 2.5: Memory v0.2 (SQLite + FTS5)
    "SqliteMemoryStore",
    "migrate_from_memory_store",
    # Phase 3: Relation Graph (L4 Identity Layer)
    "RelationGraph",
    "RNode",
    "REdge",
    "save_graph",
    "load_graph",
    # Phase 3.5: Relation Graph v0.2 (SQLite persistence)
    "SqliteRelationStore",
    "migrate_from_relation_graph",
    # Phase 3.6: Memory ↔ Graph Linker
    "LINKER_VERSION",
    "Linker",
    "ensure_central_ai_node",
    "link_episode",
    "link_note",
    "sync_all",
] 
