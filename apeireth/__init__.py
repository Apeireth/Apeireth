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
Phase 5.5 (今日): Linkage Layer v0.1 — Reconsolidation ↔ Funnel ↔ Persona 闭环
Phase 6 v0.1 (今日): Self-Organizing Team Engine — L5 自组织临时团 (主人 12:14)
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
from .linkage import (
    LINKAGE_VERSION,
    LinkageTurn, LinkageOrchestrator,
    path_a_reconsolidation_to_funnel,
    path_b_question_to_persona,
    path_c_feedback_loop,
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
from .self_org_team import (
    SELF_ORG_TEAM_VERSION,
    TaskEvent, TeamSpec, MemberContribution,
    SelfOrgTeam, SelfOrgOrchestrator,
    TEAM_TEMPLATES, match_team_spec,
)

__version__ = "0.12.0"
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
    # Phase 5.5: Linkage Layer (Phase 2↔4↔5 闭环)
    "LINKAGE_VERSION",
    "LinkageTurn",
    "LinkageOrchestrator",
    "path_a_reconsolidation_to_funnel",
    "path_b_question_to_persona",
    "path_c_feedback_loop",
    # Phase 6: Self-Organizing Team Engine (L5 自组织临时团 — 主人 12:14)
    "SELF_ORG_TEAM_VERSION",
    "TaskEvent",
    "TEAM_TEMPLATES",
    "TeamSpec",
    "match_team_spec",
    "MemberContribution",
    "SelfOrgTeam",
    "SelfOrgOrchestrator",
    # Phase 6.5: SqliteIdentityStore — 身份卡真持久化 (DEV-LOG 17:14 + 17:43 限制)
    "SQLITE_IDENTITY_VERSION",
    "SqliteIdentityStore",
    "migrate_from_identity_store",
] 

from .zvec_store import ZvecMemoryStore, ZvecConfig, _ZVEC_AVAILABLE  # Phase 2.6 zvec integration (TOP 1)
from .karpathy_principles import PRINCIPLES, KarpathyPrinciple, render_full, check_action  # 主人 13:51 + 17:29 Karpathy 升级版
from .proactive_loop import PROACTIVE_LOOP_VERSION, Goal, CuriositySignal, ProactiveLoop, make_default_proactive_loop  # Phase 11 主动性 (V2 唯一 gap)
from .mirror import MIRROR_VERSION, SelfState, SelfNarrative, Mirror, make_default_mirror  # Phase 10 意识 Layer 1 (FSA) 主人 17:58
from .meta_cognition import META_COGNITION_VERSION, FailurePattern, MetaReview, MetaMonitor  # Phase 10.x 意识 Layer 2 (HOT) — Rosenthal/Lau meta-cognition
from .self_model import SELF_MODEL_VERSION, SomaticMarkers, SelfObject, SelfModel, make_default_self_model  # Phase 10.x 意识 Layer 4 (SMM) — Metzinger/Damasio self-model + somatic markers
from .sqlite_identity_store import (  # Phase 6.5 — 身份卡真持久化 (DEV-LOG 17:14 + 17:43 限制)
    SqliteIdentityStore,
    migrate_from_identity_store,
    SQLITE_IDENTITY_VERSION,
)

__version__ = "0.13.0"
from .skill_library import SKILL_LIBRARY_VERSION, Skill, SkillLibrary, make_default_skill_library, install_seed_skills  # Phase 13 Voyager-inspired Skill Library
from .phi_proxy import PHI_PROXY_VERSION, compute_phi_proxy, compute_phi_proxy_via_mirror, AWARENESS_VALUES  # Phase 10.x IIT Phi-proxy (量化 consciousness)
from .dgm_archive import DGM_ARCHIVE_VERSION, Generation, DGMArchive, make_default_dgm_archive  # Phase 14 DGM-inspired Multi-Generation Archive (永远演化)
from .deliberation import DELIBERATION_VERSION, ThoughtStep, ThoughtBranch, DeliberationResult, DeliberationEngine, make_default_deliberation_engine  # Phase 19 思考层 (ASI self-thinking, LLM-agnostic, DeepSeek-R1 + ToT + Reflexion 借鉴)
from .asi_north_star import ASI_NORTH_STAR_VERSION, TARGET_ASI_APPROACH, CURRENT_PHASE, ASIApproachReport, compute_v6_approach, compute_v7_approach, compute_target_approach  # Phase 20 ASI 逼近指数 metric (主人 20:46 哲学修正: 不是距离, 是逼近)
from .llm_kernel import LLM_KERNEL_VERSION, LLMConfig, LLMResponse, call_llm_minimax, call_llm_template, make_call_llm  # Phase 21 真生产 LLM Kernel (MiniMax 默认, LLM-agnostic)
from .kickoff_enrichment import (  # Phase 1 v0.4 enrichment — recall_anchor + evidence_refs + completeness_score + version_migration (DEV-LOG 21:09)
    EnrichmentReport,
    enrich,
    derive_recall_anchor,
    suggest_evidence_refs,
    compute_completeness,
    check_version,
    migrate as enrich_migrate,
)
from .observation import OBSERVATION_VERSION, Observation, MetaObservation, MetaMetaObservation, ThreeTierObservation  # Phase 24 二阶控制论 3 阶观察循环 
from .ecology import NICHE_VERSION, Niche, NicheSpec, NicheConstructor  # Phase 25 生态位构造器 (Ecology Engineering 真生产借鉴)
from .self_ref import KLEIN_BOTTLE_VERSION, KleinBottleSelfRef, CentralAITopology  # Phase 30 Klein Bottle 自指拓扑工程化 (跨域调研借鉴) 
from .mind_eco import BATESON_MIND_VERSION, MindEntity, MindRelation, MindEcosystem  # Phase 31 Bateson 心灵生态学工程化 (跨域调研借鉴)
from .variety import ASHBY_VARIETY_VERSION, VarietyMeasure, RequisiteVarietyCalculator  # Phase 32 Ashby 必要多样性律 (跨域调研借鉴) 
from .active_inf import ACTIVE_INFERENCE_VERSION, Belief, Perception, ActiveInferenceAgent  # Phase 33 Friston Active Inference (跨域调研借鉴)
from .autopoiesis import AUTOPOIESIS_VERSION, AutopoieticComponent, AutopoieticSystem  # Phase 34 Maturana 自创生 (跨域调研借鉴) 
from .systems_theory import GST_VERSION, GST_PRINCIPLES, SystemPrinciple, SystemsTheoryLibrary  # Phase 35 Von Bertalanffy 系统论 (跨域调研借鉴)
from .physical_emergence import PHYSICAL_EMERGENCE_VERSION, Fluctuation, PhaseTransition, PhysicalEmergenceSystem  # Phase 36 Meyer-Ortmanns 物理涌现 (跨域调研借鉴) 
from .complexity import COMPLEXITY_VERSION, COMPLEXITY_LAWS, CrossDomainApplication, ComplexityHub  # Phase 37 Complexity Hub 跨域综合 (跨域调研借鉴)
from .game_theory import NASH_VERSION, Agent, NashEquilibrium, IncentiveEngine  # Phase 38 Nash 均衡机制设计 (跨域调研借鉴) 
from .metaphor import LAKOFF_VERSION, Metaphor, EmbodimentTrace, MetaphorEngine  # Phase 39 Lakoff 隐喻引擎 (跨域调研借鉴, 主人 22:05 不偏离哲学) 
from .small_world import SMALL_WORLD_VERSION, Node, Link, SmallWorldGraph  # Phase 40 Watts-Strogatz Small-World Network (跨域调研借鉴) 
from .philosophy import PHILOSOPHY_VERSION, PHILOSOPHY_LINES, PhilosophyCheck, check_philosophy, apeireth_philosophy_summary  # Apeireth 设计哲学守门 (主人 22:05 警告不偏离)
from .identity_card import IDENTITY_VERSION, MASTER_QUOTES_CENTRAL_AI_V2, VCP_4_PARADIGMS, IdentityCardV3  # Phase 41 V3 IdentityCard (主人 22:08 V2 哲学完整还原, VCP 4 范式引入)
from .phi_proxy_v2 import PHI_PROXY_V2_VERSION, IntegrationMeasure, PhiProxyV2  # Phase 45 Φ-proxy V2 (借鉴 IIT, 不假装实现 Phenomenal, 仅 engineering approximation, 主 17:58 终极目标不假装, V0.1 透明公式)
from .memory_3tier import MEMORY_3TIER_VERSION, MemoryAnchor, TopicSummary, Memory3Tier  # Phase 46 STM/MTM/LTM 三层 Memory (主 14:50 + MemoryOS-Rust 借鉴, 主 22:33 自主)
from .asi_coordinator import ASI_COORDINATOR_VERSION, PHASE_REGISTRY, CoordinationLink, ASICoordinator  # Phase 49 ASI Coordinator (主 22:46 真生产协同器, 19 模块真生产链接, 中央 AI = ASI 位置 V2)
from .human_wisdom_aggregator import HUMAN_WISDOM_VERSION, WisdomSource, WisdomAggregation, HumanWisdomAggregator  # Phase 50 Human Wisdom Aggregator (主 22:52 真哲学: 调研+工程+实践结合, 聚合人类智慧, 主 14:48 同根同源, 真生产 filter+评估+决策)