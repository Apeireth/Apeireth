"""Apeireth — ASI 地基平台
Phase 1 (Week 1-2): Identity Store v0.1 PoC
Phase 1.5 (今日): AnySearch 联网集成 (L2 Interaction Layer)
Phase 2 (Week 3-4): Memory Layer v0.1 PoC
Phase 2.5 (今日): SQLite + FTS5 真持久化 + 3-layer search
Phase 3 (今日): Relation Graph v0.1 PoC (L4 Identity Layer 子组件)
Phase 3.5 (今日): Relation Graph v0.2 SQLite 持久化 + 跨 session 存活
Phase 3.6 (今日): Memory ↔ Graph Linker 跨层自动绑定
Phase 4 (今日): Persona Engine v0.1 PoC (L4 Identity 多身份子组件)
Phase 5 (今日): Emergence Layer v0.1 PoC (L5 Effect — 不调度的涌现)
Phase 5.1 (今日): Questioning Engine v0.1 PoC (L2 子组件)
Phase 5.2 (今日): Identity Store v0.2 — JSON Schema + 版本迁移 + 多卡容器
Phase 5.3 (今日): Self-Evolving Harness v0.1 PoC (L5 元层 — AHE 借鉴)
作者: 楚零 | 命名: 主人 2026-07-20
"""

from .identity import IdentityCard, load_card, save_card
from .identity_store import (
    IDENTITY_STORE_VERSION, FIELD_SCHEMA, SchemaError,
    validate_card, migrate_card, migrate_v01_to_v02,
    StoreEntry, IdentityStore,
)
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
from .persona import (
    PERSONA_VERSION, ARCHETYPES,
    SCTProfile, Persona, PersonaEngine,
    seed_default_personas,
)
from .questioning import (
    QUESTIONING_VERSION, ALPHA, BETA,
    Question, Answer, FunnelState, BayesianFunnel,
)
from .emergence import (
    EMERGENCE_VERSION,
    EmergenceSignal, EmergenceEvent, EmergenceSpace,
    FeedbackLoop,
    PhaseReport, EvolutionRecord,
    phase1_eval, phase2_stats, phase24_stability,
    multi_persona_respond, commit_or_rollback,
    emergence_cycle,
)
from .self_evolving import (
    SELF_EVOLVING_VERSION,
    EvolutionPhase, Harness, Patch, PatchArchive,
    EvalReport, StatsReport, EvolveProposal, Phase5Record,
    phase1_eval as harness_phase1_eval,
    phase2_stats as harness_phase2_stats,
    phase24_stability as harness_phase24_stability,
    phase3_evolve, phase4_verify, phase5_commit_or_rollback,
    HarnessEvolver,
)

__version__ = "0.10.0"
__all__ = [
    # Phase 1: Identity
    "IdentityCard",
    "load_card",
    "save_card",
    # Phase 1.2: Identity Store v0.2 (schema + migration + multi-card)
    "IDENTITY_STORE_VERSION",
    "FIELD_SCHEMA",
    "SchemaError",
    "validate_card",
    "migrate_card",
    "migrate_v01_to_v02",
    "StoreEntry",
    "IdentityStore",
    # Phase 1: Kickoff
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
    # Phase 4: Persona Engine (L4 多身份)
    "PERSONA_VERSION",
    "ARCHETYPES",
    "SCTProfile",
    "Persona",
    "PersonaEngine",
    "seed_default_personas",
    # Phase 5: Questioning Engine (L2 Interaction 子组件)
    "QUESTIONING_VERSION",
    "ALPHA",
    "BETA",
    "Question",
    "Answer",
    "FunnelState",
    "BayesianFunnel",
] 
