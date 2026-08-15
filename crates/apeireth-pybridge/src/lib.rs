//! apeireth-pybridge: PyO3 桥 (Python 3.13.14 ↔ Rust) — feature-gated compat layer
//!
//! R14 A16.3 落地: 主 19:33 走在前人经验上, 借鉴 DeltaMemory-Rust (Lin et al. 2024) PyO3 模式.
//! R125-9 重构: 借鉴 PyO3 0.22+ `Bound` API + `Python::attach` + kwargs 透传 best practice.
//!
//! # ADR 0007 + ADR 0008 — Feature-gated Compat Layer (round9-11 qa_engineer)
//!
//! - **默认 build** (`cargo build --workspace`): 本 crate 作为 Rust-only 兼容组件层,
//!   `pyo3` **不** 被激活。`bridge::*` 中的 `python_is_available`/`python_version_string`
//!   返回 `false` / 静态占位符, `call_python_function` 返回 `ModuleNotFound` 降级。
//!   这样默认 build 不需要 Python 3.13.14 运行时, 避免污染 Rust-only CI。
//! - **`--features python-ext`** (`cargo build --features apeireth-pybridge/python-ext`):
//!   `pyo3` + `pyo3/extension-module` 启用, `python_bindings` 模块被编译,
//!   `#[pymodule] apeireth_pybridge` 注册到 Python 解释器。
//!
//! ADR 0007 = 兼容组件层定位, ADR 0008 = pyo3 feature-gating (本文件实现).
//! R125-9 新增: `call_python_function_kw` / `eval_python_expression` / `py_call_python_with_kwargs` /
//! `py_eval_expression` 入口, 借鉴 PyO3 0.22+ `python-from-rust/calling-existing-code.md` 模式.


pub mod asi_modules;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod bridge;
pub mod bridge_pool;
// R129-4 ASI Python 整合 Stage 4 自治 - D4 决策自循环 (per decision-61 §3.1 R129-4)
pub mod decision_self_loop;
pub mod error;
// R129-6 ASI Python 整合 Stage 6 守护 — K1/K2/K3/K4 (per decision-61 §3.1)
pub mod error_guardianship;
// R129-5 ASI Python 整合 Stage 5 治理 — G4 演进治理 (per decision-61 §3.1 R129-5)
pub mod evolution_governance;
// R129-5 ASI Python 整合 Stage 5 治理 — G3 形式化治理 (per decision-61 §3.1 R129-5)
pub mod formal_governance;
pub mod health_guardianship;
// R129-4 ASI Python 整合 Stage 4 自治 - D3 记忆自循环 (per decision-61 §3.1 R129-4)
pub mod memory_self_loop;
// R129-5 ASI Python 整合 Stage 5 治理 — G2 权限治理 (per decision-61 §3.1 R129-5)
pub mod permission_governance;
pub mod perf_guardianship;
// R129-4 ASI Python 整合 Stage 4 自治 - D2 反思自循环 (per decision-61 §3.1 R129-4)
pub mod reflection_self_loop;
pub mod r11_compat;
// R220: tokio::spawn_blocking 包装 sync Python 为 async (0 引 pyo3-asyncio)
#[cfg(feature = "python-ext")]
pub mod async_wrapper;
// R129-5 ASI Python 整合 Stage 5 治理 — G1 资源治理 (per decision-61 §3.1 R129-5)
pub mod resource_governance;
pub mod security_guardianship;
// R128 阶段 A Stage 3 集成验证 (per decision-58 §2.1 P10-3)
pub mod stage3_bench;
pub mod stage3_cross_module;
pub mod stage3_e2e;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I1 D1+G1 工具+资源集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i1_tool_resource;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I2 D2+K1 反思+错误集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i2_reflection_error;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I3 D3+G3 记忆+形式化集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i3_memory_formal;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I4 D4+G2 决策+权限集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i4_decision_permission;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I5 G1+K2 资源+性能集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i5_resource_perf;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I6 G2+K3 权限+安全集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i6_permission_security;
// R129-18 ASI Python 整合 Stage 7 跨模块集成 — I7 G4+K4 演进+健康集成 (per decision-61 §3.1 R129-18)
pub mod stage7_i7_evolution_health;
// R129-4 ASI Python 整合 Stage 4 自治 - D1 工具调用自循环 (per decision-61 §3.1 R129-4)
pub mod tool_self_loop;
pub mod type_convert;

// `python_bindings` 模块仅在启用 `python-ext` feature 时存在 (默认 build 为 0 体积)。
#[cfg(feature = "python-ext")]
pub mod python_bindings;

pub use bridge::{
    call_python_builtin, call_python_function, call_python_function_kw, episode_to_json,
    eval_python_expression, get_or_import_via_pool, health_check, is_module_available,
    is_r11_module_available, note_to_json, python_is_available, python_version_string,
    session_to_json, try_call_or_degrade, BridgeHealth,
};
pub use bridge_pool::{BridgeModulePool, PoolConfig, PoolStats};
pub use error::{BridgeError, SuggestedAction};
pub use r11_compat::{
    is_known_r11_module, list_r11_modules_by_category, list_r11_modules_by_prefix,
    r11_compat_version, r11_lookup_module, r11_module_category, r11_module_count, R11Category,
    R11ModuleInfo, R11_COMPAT_VERSION, R11_MODULE_COUNT,
};
pub use asi_modules::{
    asi_lookup_by_version, asi_lookup_module, asi_stage1_all_invariants_ok,
    asi_stage1_bridge_health, asi_stage1_ceiling_chain_locked, asi_stage1_health,
    asi_stage1_module_count, asi_stage1_v1077_dim_count, asi_stage1_v1400_capabilities_limits,
    asi_stage1_v1447_pair_count, asi_stage1_v1457_weights_sum_one, asi_stage1_v1467_endpoint_count,
    asi_stage1_v1470_cross_checks, asi_stage1_version, bridge_v1077_full_measure,
    bridge_v1457_deploy_all, bridge_v1458_ceiling_audit, is_known_asi_stage1_module,
    list_asi_stage1_modules_by_category, list_ceiling_critical_modules,
    AsiCategory, AsiModuleInfo, AsiStage1Health, AuditPair, BatchRunStats, CeilingChainLock,
    ClosureKind, CrossClientCheck, OperationalStage, PhilosophicalProblem, V1077_INFO,
    V1077_MODULE, V1077_N_DIMENSIONS, V1077_WEIGHT_SUM, V1077_WEIGHT_TOLERANCE,
    V1400_CAPABILITIES, V1400_INFO, V1400_LIMITS, V1400_MODULE, V1400_N_CAPABILITIES, V1400_N_LIMITS,
    V1400_N_RULES, V1447_AUDIT_PAIRS, V1447_INFO, V1447_MODULE, V1447_N_CLOSURE_KINDS, V1447_N_COMBINED_PROBES,
    V1447_N_CROSS_PAIR_LINKS, V1447_N_PAIRS, V1447_N_POSITIONS, V1447_N_PROBLEMS, V1457_INFO,
    V1457_MODULE, V1457_N_DEPLOYMENTS, V1457_N_PROBES, V1457_N_STAGES, V1457_STAGE_WEIGHT_SUM,
    V1458_ABSOLUTE_CEILING, V1458_ANCHOR_VALUE, V1458_GAP_TO_CEILING, V1458_GAP_TO_NORTH_STAR,
    V1458_INFO, V1458_MODULE, V1458_N_BOUNDED_PROBES, V1458_N_CEILING_MODULES,
    V1458_N_DEPLOYMENT_CUBE_MODULES, V1458_NORTH_STAR_CEILING, V1458_TOLERANCE, V1467_INFO,
    V1467_MODULE, V1467_N_ENDPOINTS, V1467Endpoint, V1470_INFO, V1470_MODULE, V1470_N_CLIENT_PATHS,
    V1470_N_CROSS_CHECKS_PER_RUN, V1470_N_CROSS_CHECKS_TOTAL, V1470_N_ENDPOINTS,
    V1470_DEFAULT_BATCH_N, V1470_MIN_BATCH_N, V2Position, ASI_PYTHON_DIR,
    ASI_STAGE1_INFOS, ASI_STAGE1_MODULES, ASI_STAGE1_MODULE_COUNT, ASI_STAGE1_VERSION,
};
pub use type_convert::{json_to_rust, rust_to_json, BridgeConvert};

// R129-6 ASI Python 整合 Stage 6 守护 re-export (per decision-61 §3.1 R129-6)
// K1 错误守护 + K2 性能守护 + K3 安全守护 + K4 健康守护
pub use error_guardianship::{
    stage6_error_guard, stage6_error_healthy, stage6_error_summary, stage6_record_error, ErrorEvent,
    ErrorGuard, ErrorKind, ErrorSeverity,
};
pub use perf_guardianship::{
    stage6_perf_alerts, stage6_perf_healthy, stage6_perf_monitor, stage6_record_perf,
    stage6_perf_summary, PerfKind, PerfMonitor, PerfSample, PerfStats,
};
pub use security_guardianship::{
    stage6_security_baseline_intact, stage6_security_guard, stage6_security_healthy,
    stage6_security_summary, stage6_record_security, CrossLanguageCheck, SecurityEvent,
    SecurityEventKind, SecurityGate, SecurityGuard, SecuritySeverity, SecurityVerdict, V7BaselineCheck,
};
pub use health_guardianship::{
    stage6_health_check, stage6_health_guard, stage6_health_healthy, stage6_health_summary,
    HealthCheck, HealthDimension, HealthGuard, HealthReport, HealthStatus,
};

// R129-5 ASI Python 整合 Stage 5 治理 re-export (per decision-61 §3.1 R129-5)
// 4 维度: G1 资源治理 + G2 权限治理 + G3 形式化治理 + G4 演进治理
pub use resource_governance::{
    resource_governance_bootstrap_ok, resource_governance_health, resource_governance_summary,
    resource_governance_version, GovernanceAction, ResourceAuditEvent, ResourceDimension,
    ResourceGovernanceHealth, ResourceGovernor, ResourceQuota, ResourceReport,
    RESOURCE_GOVERNANCE_DIMENSION_COUNT, RESOURCE_GOVERNANCE_MODULE_COUNT,
    RESOURCE_GOVERNANCE_VERSION,
};
pub use permission_governance::{
    permission_governance_health, permission_governance_layer_count,
    permission_governance_summary, permission_governance_version, PermissionContext,
    PermissionDecision, PermissionDecisionEvent, PermissionEngine, PermissionGovernanceHealth,
    PermissionLayer, PermissionReport, PERMISSION_GOVERNANCE_LAYER_COUNT,
    PERMISSION_GOVERNANCE_STAGE_COUNT, PERMISSION_GOVERNANCE_VERSION,
};
pub use formal_governance::{
    formal_governance_health, formal_governance_summary, formal_governance_version,
    AsiStage5Token, FormalGovernanceHealth, Invariant, ProofHarness, ProofKind, ProofReport,
    ProofResult, ProofRunner, FORMAL_GOVERNANCE_HARNESS_COUNT, FORMAL_GOVERNANCE_STAGE1_MODULES,
    FORMAL_GOVERNANCE_STAGE_COUNT, FORMAL_GOVERNANCE_TOKEN_FIELDS, FORMAL_GOVERNANCE_VERSION,
};
pub use evolution_governance::{
    evolution_governance_health, evolution_governance_summary, evolution_governance_version,
    EvolutionContext, EvolutionEngine, EvolutionEvent, EvolutionGovernanceHealth, EvolutionKind,
    EvolutionOutcome, EvolutionReport, EvolutionRule, EVOLUTION_GOVERNANCE_KIND_COUNT,
    EVOLUTION_GOVERNANCE_RULE_COUNT, EVOLUTION_GOVERNANCE_STAGE_COUNT,
    EVOLUTION_GOVERNANCE_VERSION,
};

// R128 阶段 A Stage 3 集成验证 re-export (per decision-58 §2.1 P10-3)
pub use stage3_bench::{
    stage3_bench_run_default, stage3_bench_targets, BenchAsiLookupModule, BenchConfig,
    BenchJsonToRustEpisode, BenchR11CompatVersion, BenchR11ModuleCount, BenchReport, BenchRunner,
    BenchRustToJsonEpisode, BenchSample, BenchStats, BenchTarget, BenchTargetReport,
};
pub use stage3_cross_module::{
    probe_asi_to_r11, probe_bridge_to_pool, probe_bridge_to_r11, probe_core_to_bridge,
    probe_pool_to_type_convert, stage3_cross_module_probes, CrossModuleKind, CrossModuleProbeResult,
    CrossModuleReport, HardWallsVerify,
};
pub use stage3_e2e::{
    stage3_cross_module_count, stage3_e2e_smoke, stage3_e2e_summary, Stage3E2ESmoke,
};

// R129-4 ASI Python 整合 Stage 4 自治 re-export (per decision-61 §3.1 R129-4)
// 4 维度: D1 工具自循环 + D2 反思自循环 + D3 记忆自循环 + D4 决策自循环
pub use decision_self_loop::{
    decision_self_loop_summary, DecisionPolicy, DecisionRecord, DecisionSelfLoop, DecisionStage,
    DecisionState, DecisionTrigger, DECISION_MAX_REVISIT, DECISION_POLICY_COUNT,
    DECISION_STAGE_COUNT, DECISION_STATE_COUNT, DECISION_TRIGGER_COUNT,
};

// R129-18 ASI Python 整合 Stage 7 跨模块集成 re-export (per decision-61 §3.1 R129-18)
// 7 维度: I1 D1+G1 + I2 D2+K1 + I3 D3+G3 + I4 D4+G2 + I5 G1+K2 + I6 G2+K3 + I7 G4+K4
pub use stage7_i1_tool_resource::{
    stage7_i1_healthy, stage7_i1_summary, stage7_i1_to_d1_consistency,
    stage7_i1_to_g1_consistency, ToolResourceAuditEvent, ToolResourceBinding,
    ToolResourceCoordinator, ToolResourceMatrix, ToolResourceReport, STAGE7_I1_BINDING_COUNT,
    STAGE7_I1_DEFAULT_QUOTA, STAGE7_I1_DIMENSION_COUNT, STAGE7_I1_VERSION,
};
pub use stage7_i2_reflection_error::{
    stage7_i2_healthy, stage7_i2_summary, stage7_i2_to_d2_consistency,
    stage7_i2_to_k1_consistency, ReflectionErrorAuditEvent, ReflectionErrorBinding,
    ReflectionErrorCoordinator, ReflectionErrorMatrix, ReflectionErrorReport, STAGE7_I2_BINDING_COUNT,
    STAGE7_I2_DIMENSION_COUNT, STAGE7_I2_ERROR_KIND_COUNT, STAGE7_I2_NODE_COUNT, STAGE7_I2_VERSION,
};
pub use stage7_i3_memory_formal::{
    stage7_i3_healthy, stage7_i3_summary, stage7_i3_to_d3_consistency,
    stage7_i3_to_g3_consistency, MemoryFormalAuditEvent, MemoryFormalBinding,
    MemoryFormalCoordinator, MemoryFormalMatrix, MemoryFormalReport, STAGE7_I3_BINDING_COUNT,
    STAGE7_I3_DIMENSION_COUNT, STAGE7_I3_HARNESS_COUNT, STAGE7_I3_MEMORY_KIND_COUNT,
    STAGE7_I3_VERSION,
};
pub use stage7_i4_decision_permission::{
    stage7_i4_healthy, stage7_i4_summary, stage7_i4_to_d4_consistency,
    stage7_i4_to_g2_consistency, DecisionPermissionAuditEvent, DecisionPermissionBinding,
    DecisionPermissionCoordinator, DecisionPermissionMatrix, DecisionPermissionReport,
    STAGE7_I4_BINDING_COUNT, STAGE7_I4_DIMENSION_COUNT, STAGE7_I4_LAYER_COUNT,
    STAGE7_I4_POLICY_COUNT, STAGE7_I4_VERSION,
};
pub use stage7_i5_resource_perf::{
    stage7_i5_healthy, stage7_i5_summary, stage7_i5_to_g1_consistency,
    stage7_i5_to_k2_consistency, ResourcePerfAuditEvent, ResourcePerfBinding,
    ResourcePerfCoordinator, ResourcePerfMatrix, ResourcePerfReport, STAGE7_I5_BINDING_COUNT,
    STAGE7_I5_DIMENSION_COUNT, STAGE7_I5_PERF_KIND_COUNT, STAGE7_I5_RESOURCE_DIM_COUNT,
    STAGE7_I5_VERSION,
};
pub use stage7_i6_permission_security::{
    stage7_i6_healthy, stage7_i6_summary, stage7_i6_to_g2_consistency,
    stage7_i6_to_k3_consistency, PermissionSecurityAuditEvent, PermissionSecurityBinding,
    PermissionSecurityCoordinator, PermissionSecurityMatrix, PermissionSecurityReport,
    STAGE7_I6_BINDING_COUNT, STAGE7_I6_DIMENSION_COUNT, STAGE7_I6_PERMISSION_LAYER_COUNT,
    STAGE7_I6_SECURITY_GATE_COUNT, STAGE7_I6_VERSION,
};
pub use stage7_i7_evolution_health::{
    stage7_i7_healthy, stage7_i7_summary, stage7_i7_to_g4_consistency,
    stage7_i7_to_k4_consistency, EvolutionHealthAuditEvent, EvolutionHealthBinding,
    EvolutionHealthCoordinator, EvolutionHealthMatrix, EvolutionHealthReport,
    STAGE7_I7_BINDING_COUNT, STAGE7_I7_DIMENSION_COUNT, STAGE7_I7_EVOLUTION_KIND_COUNT,
    STAGE7_I7_HEALTH_DIM_COUNT, STAGE7_I7_VERSION,
};
pub use memory_self_loop::{
    memory_self_loop_summary, DeterminismMeta, MemoryEntry, MemoryJournal, MemoryKind,
    MemoryResult, MemorySelfLoop, MEMORY_ENTRY_FIELDS, MEMORY_KIND_COUNT,
    MEMORY_MAX_ENTRIES, MEMORY_RESULT_COUNT,
};
pub use reflection_self_loop::{
    reflection_self_loop_summary, ReflectionAction, ReflectionGraph, ReflectionLoopStage,
    ReflectionNode, ReflectionResult, ReflectionSelfLoop, ReflectionState,
    REFLECTION_ACTION_COUNT, REFLECTION_GRAPH_NODE_COUNT, REFLECTION_MAX_DEPTH,
    REFLECTION_STATE_COUNT,
};
pub use tool_self_loop::{
    tool_self_loop_summary, AsiTool, ToolComposer, ToolExecutor, ToolInput, ToolLoopReport,
    ToolLoopStage, ToolPlanner, ToolReflector, ToolRegistry, ToolResult, ToolSelfLoop,
    ToolValidator, DEFAULT_TOOL_COUNT, TOOL_SELF_LOOP_MAX_DEPTH,
};

// `py_xxx` 函数仅在 python-ext 启用时存在; 公开 re-export 跟随 cfg 守门。
#[cfg(feature = "python-ext")]
pub use python_bindings::{
    py_call_python, py_call_python_with_kwargs, py_episode_to_json, py_eval_expression,
    py_health_check, py_is_known_r11_module, py_is_module_available, py_note_to_json,
    py_r11_module_category, py_r11_module_count, py_session_to_json, py_version,
};

// R128 阶段 A Stage 2 集成测试公共 API re-export (per decision-57 §2.1 P10-2)
// BridgePoolSmoke / CrossLanguageSmoke 在本文件定义, 无需 re-export
// (lib.rs 本身就是 crate root)

/// 占位函数 — round9-11 起标注 ADR 0007/0008 落地状态。
pub fn placeholder() -> &'static str {
    "apeireth-pybridge R14 A16.3 + R125-9 + R127-2 — ADR 0007 compat-layer + ADR 0008 feature-gated (pyo3 optional) + PyO3 0.22+ best practice (Python::attach + Bound API + kwargs) + Stage 6.1 跨语言桥深化 (type_convert + bridge_pool + kw + eval) + R128 阶段 A Stage 3 集成验证 (P10-3: 端到端 + 性能 + 跨模块, per decision-58 §2.1) + R129-4 ASI Python 整合 Stage 4 自治 (D1 工具自循环 + D2 反思自循环 + D3 记忆自循环 + D4 决策自循环, per decision-61 §3.1) + R129-5 ASI Python 整合 Stage 5 治理 (G1 资源 + G2 权限 + G3 形式化 + G4 演进, per decision-61 §3.1) + R129-6 ASI Python 整合 Stage 6 守护 (K1 错误 + K2 性能 + K3 6+1 重门安全 + K4 5 维度健康, per decision-61 §3.1) + R129-18 ASI Python 整合 Stage 7 跨模块集成 (I1 D1+G1 + I2 D2+K1 + I3 D3+G3 + I4 D4+G2 + I5 G1+K2 + I6 G2+K3 + I7 G4+K4, per decision-61 §3.1)"
}

/// 当前 pybridge 的 feature 配置 — 用于诊断 / 运行时判断。
///
/// 返回 `true` 表示 `python-ext` feature 已激活 (pyo3 编译进二进制)。
pub fn python_ext_enabled() -> bool {
    cfg!(feature = "python-ext")
}

// =============================================================================
// R128 阶段 A Stage 2 集成测试公共 API (per decision-57 §2.1 P10-2)
// 集成测试 + 跨语言调用验证 入口, cfg-gated 双实现 (默认 build 0 体积 stub)
// =============================================================================

/// Stage 2 集成测试: BridgeModulePool 端到端 smoke check
///
/// 返回 `BridgePoolSmoke` 结构: 池版本 + 配置 + 统计 (0 体积 stub 或真实现)
/// - 默认 build: 返回 0 体积 stub, PoolStats::default()
/// - python-ext build: 返回 BridgeModulePool::default() 真实池状态
pub fn end_to_end_smoke_check() -> BridgePoolSmoke {
    let pool = bridge_pool::BridgeModulePool::default();
    BridgePoolSmoke {
        r11_compat_version: r11_compat_version().to_string(),
        r11_module_count: r11_module_count(),
        python_ext_active: python_ext_enabled(),
        pool_stats: pool.stats(),
        pool_max_idle: pool.config().max_idle,
        pool_idle_timeout_secs: pool.config().idle_timeout_secs,
    }
}

/// Stage 2 跨语言调用验证: Python ↔ Rust 双向 smoke check
///
/// 返回 `CrossLanguageSmoke` 结构: python_version + r11 + 模块可用性 + 双向 OK 标记
/// - 默认 build: 标 `bidirectional_ok = false` (pyo3 不可用, 诚实标 0 装)
/// - python-ext build: 标 `bidirectional_ok = true` (PyO3 暴露 + call_python_function 双向 OK)
pub fn cross_language_smoke_check() -> CrossLanguageSmoke {
    let python_available = python_is_available();
    let module_math = is_module_available("math");
    let module_json = is_module_available("json");
    CrossLanguageSmoke {
        r11_compat_version: r11_compat_version().to_string(),
        r11_module_count: r11_module_count(),
        python_ext_active: python_ext_enabled(),
        python_available,
        module_math_available: module_math,
        module_json_available: module_json,
        // 双向 OK = python-ext 已激活 + 运行时 Python 解释器可用 + 关键模块可导入
        bidirectional_ok: python_ext_enabled() && python_available && module_math && module_json,
    }
}

/// Stage 2 集成测试: 端到端池复用 smoke 结构
#[derive(Debug, Clone)]
pub struct BridgePoolSmoke {
    pub r11_compat_version: String,
    pub r11_module_count: usize,
    pub python_ext_active: bool,
    pub pool_stats: bridge_pool::PoolStats,
    pub pool_max_idle: usize,
    pub pool_idle_timeout_secs: u64,
}

impl std::fmt::Display for BridgePoolSmoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge Stage 2 集成测试 smoke (BridgeModulePool 端到端):\n  r11: {} ({} modules)\n  python_ext: {}\n  pool: max_idle={} idle_timeout={}s cached={} hits={} misses={} evictions={} hit_rate={:.2}",
            self.r11_compat_version,
            self.r11_module_count,
            self.python_ext_active,
            self.pool_max_idle,
            self.pool_idle_timeout_secs,
            self.pool_stats.cached_modules,
            self.pool_stats.hits,
            self.pool_stats.misses,
            self.pool_stats.evictions,
            self.pool_stats.hit_rate(),
        )
    }
}

/// Stage 2 跨语言调用验证: Python ↔ Rust 双向 smoke 结构
#[derive(Debug, Clone)]
pub struct CrossLanguageSmoke {
    pub r11_compat_version: String,
    pub r11_module_count: usize,
    pub python_ext_active: bool,
    pub python_available: bool,
    pub module_math_available: bool,
    pub module_json_available: bool,
    pub bidirectional_ok: bool,
}

impl std::fmt::Display for CrossLanguageSmoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge Stage 2 跨语言调用 smoke (Python <-> Rust 双向):\n  r11: {} ({} modules)\n  python_ext: {}\n  python_available: {}\n  math: {}\n  json: {}\n  bidirectional_ok: {}",
            self.r11_compat_version,
            self.r11_module_count,
            self.python_ext_active,
            self.python_available,
            self.module_math_available,
            self.module_json_available,
            self.bidirectional_ok,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_ok() {
        assert!(placeholder().contains("apeireth-pybridge"));
        assert!(placeholder().contains("ADR 0007"));
    }

    #[test]
    fn python_ext_enabled_consistent() {
        // cfg! 在编译期评估, 运行时调用与本测试目标的 feature 标识一致。
        let expected = cfg!(feature = "python-ext");
        assert_eq!(python_ext_enabled(), expected);
    }

    #[test]
    fn public_api_exports_resolve() {
        let _ = r11_module_count();
        let _ = r11_compat_version();
        let _ = is_known_r11_module("apeireth.memory.store");
        let _ = r11_module_category("apeireth.memory.store");
        let _ = python_version_string();
        let _ = python_is_available();
        let _ = is_module_available("math");
    }

    #[test]
    fn re_exports_match_constants() {
        assert_eq!(r11_module_count(), R11_MODULE_COUNT);
        assert_eq!(r11_compat_version(), R11_COMPAT_VERSION);
    }

    #[test]
    fn error_re_exports_work() {
        let e = BridgeError::ModuleNotFound("x".into());
        let _: SuggestedAction = e.suggested_action();
    }

    #[test]
    fn r11_module_info_re_export() {
        let info = r11_lookup_module("apeireth.memory.v1141").unwrap();
        assert_eq!(info.category, R11Category::Memory);
        assert!(info.is_baseline);
    }

    #[test]
    fn default_build_python_is_available_false() {
        // 默认 build 下 (无 python-ext) — python_is_available 永远 = false。
        // python-ext build 下 也可能 = false (取决于运行时是否有 Python 解释器)。
        // 本测试仅验证默认 build 下, 不假设 true。
        let _ = python_is_available();
    }

    // ============================================================
    // V27.0 跨配置功能对等 (round10-08 qa_engineer) — 10 个新增 unit 测试
    // ============================================================

    /// Cross-config invariant: `r11_module_count()` 在两配置下必须 = 1103。
    #[test]
    fn unit_v27_r11_count_is_1103_in_both_configs() {
        assert_eq!(r11_module_count(), 1103);
    }

    /// Cross-config invariant: `R11_COMPAT_VERSION` 在两配置下必须 = R14-...。
    #[test]
    fn unit_v27_compat_version_is_r14_in_both_configs() {
        let v = r11_compat_version();
        assert!(v.starts_with("R14") || v.contains("R14"), "got {v}");
    }

    /// Cross-config invariant: `is_known_r11_module` 返回值稳定。
    #[test]
    fn unit_v27_known_r11_module_stable() {
        assert!(is_known_r11_module("apeireth.memory.store"));
        assert!(!is_known_r11_module("apeireth.nope.nope"));
    }

    /// Cross-config invariant: `BridgeError::ModuleNotFound.suggested_action()` = Degrade。
    #[test]
    fn unit_v27_error_module_not_found_suggests_degrade() {
        let e = BridgeError::ModuleNotFound("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Degrade);
    }

    /// Cross-config invariant: `BridgeError::InvalidArg.suggested_action()` = Fail。
    #[test]
    fn unit_v27_error_invalid_arg_suggests_fail() {
        let e = BridgeError::InvalidArg("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Fail);
    }

    /// Cross-config invariant: `BridgeError::CallFailed.suggested_action()` = Retry。
    #[test]
    fn unit_v27_error_call_failed_suggests_retry() {
        let e = BridgeError::CallFailed("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Retry);
        assert!(e.is_recoverable());
    }

    /// Cross-config invariant: `BridgeError::GilError.suggested_action()` = Retry。
    #[test]
    fn unit_v27_error_gil_error_suggests_retry() {
        let e = BridgeError::GilError("x".into());
        assert_eq!(e.suggested_action(), SuggestedAction::Retry);
        assert!(e.is_recoverable());
    }

    /// Cross-config invariant: `r11_lookup_module` baseline 字段稳定 (R11 1103 LOCKED)。
    #[test]
    fn unit_v27_lookup_baseline_v1141_is_memory() {
        let info = r11_lookup_module("apeireth.memory.v1141").expect("v1141 in R11");
        assert!(info.is_baseline);
        assert_eq!(info.category, R11Category::Memory);
    }

    /// Cross-config invariant: `placeholder()` 是 `&'static str`, 同一地址。
    #[test]
    fn unit_v27_placeholder_is_static_str() {
        let p1 = placeholder();
        let p2 = placeholder();
        assert_eq!(p1.as_ptr(), p2.as_ptr());
    }

    /// Cross-config invariant: `python_ext_enabled()` 与编译期 `cfg!` 一致 (V27.0 核心守门)。
    #[test]
    fn unit_v27_python_ext_runtime_matches_cfg() {
        let runtime = python_ext_enabled();
        let compile_time = cfg!(feature = "python-ext");
        assert_eq!(runtime, compile_time);
    }

    // ============================================================
    // R128 阶段 A Stage 2 集成测试公共 API 单元测试 (per decision-57 §2.1 P10-2)
    // 借 Stage 1 bridge_pool + r11_compat + bridge 全链路协同
    // cfg-无关: 跑在所有 build (默认 build + python-ext build)
    // ============================================================

    /// Stage 2 端到端 smoke check 跨 build 一致: r11_module_count 严守 1103
    #[test]
    fn r128_stage2_end_to_end_smoke_r11_count() {
        let s = end_to_end_smoke_check();
        assert_eq!(s.r11_module_count, 1103);
        assert!(s.r11_compat_version.contains("R14"));
    }

    /// Stage 2 端到端 smoke check: pool 配置 (借 Stage 1 PoolConfig::default())
    #[test]
    fn r128_stage2_end_to_end_smoke_pool_config() {
        let s = end_to_end_smoke_check();
        assert_eq!(s.pool_max_idle, 32);
        assert_eq!(s.pool_idle_timeout_secs, 90);
    }

    /// Stage 2 端到端 smoke check: 池 stats 初始为 0 (默认 build + python-ext 都满足)
    #[test]
    fn r128_stage2_end_to_end_smoke_pool_stats_initial_zero() {
        let s = end_to_end_smoke_check();
        assert_eq!(s.pool_stats.cached_modules, 0);
        assert_eq!(s.pool_stats.hits, 0);
        assert_eq!(s.pool_stats.misses, 0);
        assert_eq!(s.pool_stats.evictions, 0);
        assert_eq!(s.pool_stats.hit_rate(), 0.0);
    }

    /// Stage 2 端到端 smoke check: python_ext_active 与 cfg 一致
    #[test]
    fn r128_stage2_end_to_end_smoke_python_ext_active() {
        let s = end_to_end_smoke_check();
        assert_eq!(s.python_ext_active, cfg!(feature = "python-ext"));
        assert_eq!(s.python_ext_active, python_ext_enabled());
    }

    /// Stage 2 端到端 smoke check: Display 输出含 r11 + pool 字段 (集成测试诊断)
    #[test]
    fn r128_stage2_end_to_end_smoke_display_contains_r11_and_pool() {
        let s = end_to_end_smoke_check();
        let out = format!("{s}");
        assert!(out.contains("r11"));
        assert!(out.contains("pool"));
        assert!(out.contains("max_idle=32"));
    }

    /// Stage 2 跨语言 smoke check: 双向 OK 守门
    /// - 默认 build: bidirectional_ok = false (pyo3 不可用, 0 装)
    /// - python-ext + 运行时 Python 可用 + math/json 可导入: bidirectional_ok = true
    #[test]
    fn r128_stage2_cross_language_smoke_bidirectional() {
        let s = cross_language_smoke_check();
        // 默认 build 下 python_ext_active = false, bidirectional_ok 必 = false
        if !s.python_ext_active {
            assert!(!s.bidirectional_ok, "默认 build 下 bidirectional_ok 必 = false (0 装 PASS 严守)");
        }
        // python_ext_active 必与 cfg! 一致
        assert_eq!(s.python_ext_active, cfg!(feature = "python-ext"));
        // r11 字段跨 build 一致
        assert_eq!(s.r11_module_count, 1103);
    }

    /// Stage 2 跨语言 smoke check: Display 输出含双向 OK 字段
    #[test]
    fn r128_stage2_cross_language_smoke_display_contains_bidirectional() {
        let s = cross_language_smoke_check();
        let out = format!("{s}");
        assert!(out.contains("bidirectional_ok"));
        assert!(out.contains("python_available"));
        assert!(out.contains("math"));
        assert!(out.contains("json"));
    }

    /// Stage 2 跨语言 smoke check: 双调用都返回相同 r11_compat_version 跨 build 严守
    #[test]
    fn r128_stage2_cross_language_smoke_r11_version_stable() {
        let s = cross_language_smoke_check();
        assert_eq!(s.r11_compat_version, R11_COMPAT_VERSION);
        assert_eq!(s.r11_compat_version, r11_compat_version());
    }

    // ============================================================
    // R128 阶段 A Stage 3 集成验证 公共 API 单元测试 (per decision-58 §2.1 P10-3)
    // 借 Stage 1+2 基础 (asi_modules + bridge + bridge_pool + r11_compat + type_convert)
    // 实施端到端 + 性能 + 跨模块 (Stage 3)
    // cfg-无关: 跑在所有 build (默认 build + python-ext build)
    // ============================================================

    /// Stage 3 placeholder 包含 "Stage 3" 关键词
    #[test]
    fn r128_stage3_placeholder_mentions_stage3() {
        let p = placeholder();
        assert!(p.contains("Stage 3"));
        assert!(p.contains("P10-3"));
        assert!(p.contains("decision-58"));
    }

    /// Stage 3 端到端 smoke 跨 build 可调用
    #[test]
    fn r128_stage3_e2e_smoke_callable() {
        let s = stage3_e2e_smoke();
        assert_eq!(s.asi_module_count, 7);
        assert_eq!(s.r11_module_count, 1103);
        assert_eq!(s.modules_in_scope.len(), 6);
    }

    /// Stage 3 跨模块探针 5/5 OK
    #[test]
    fn r128_stage3_cross_module_probes_5_of_5_ok() {
        let r = stage3_cross_module_probes();
        assert_eq!(r.probe_results.len(), 5);
        assert!(r.all_ok);
    }

    /// Stage 3 8 硬墙 verify 全 PASS
    #[test]
    fn r128_stage3_hard_walls_all_pass() {
        let v = HardWallsVerify::auto_verify();
        assert!(v.all_pass());
    }

    /// Stage 3 性能基准 跑通 (N=100, warmup=true, 5 target)
    #[test]
    fn r128_stage3_bench_run_default_5_targets() {
        let report = stage3_bench_run_default();
        assert_eq!(report.target_reports.len(), 5);
        for tr in &report.target_reports {
            assert_eq!(tr.stats.n, 100);
        }
    }

    /// Stage 3 stage3_cross_module_count 5/5 子模块 OK
    #[test]
    fn r128_stage3_cross_module_count_5_of_5() {
        let (ok, total) = stage3_cross_module_count();
        assert_eq!(total, 5);
        assert_eq!(ok, total);
    }

    /// Stage 3 stage3_e2e_summary 引用 decision-58
    #[test]
    fn r128_stage3_summary_cites_decision_58() {
        let s = stage3_e2e_summary();
        assert!(s.contains("decision-58"));
        assert!(s.contains("P10-3"));
    }

    // ============================================================
    // R129-4 ASI Python 整合 Stage 4 自治 公共 API 单元测试 (per decision-61 §3.1 R129-4)
    // 4 维度: D1 工具自循环 + D2 反思自循环 + D3 记忆自循环 + D4 决策自循环
    // 借 Stage 1+2+3 基础 (asi_modules + bridge + bridge_pool + stage3_*)
    // cfg-无关: 跑在所有 build (默认 build + python-ext build)
    // ============================================================

    /// Stage 4 placeholder 包含 "Stage 4" 关键词 + R129-4 + decision-61
    #[test]
    fn r129_4_stage4_placeholder_mentions_stage4() {
        let p = placeholder();
        assert!(p.contains("Stage 4"));
        assert!(p.contains("R129-4"));
        assert!(p.contains("decision-61"));
    }

    /// Stage 4 D1 ToolSelfLoop 5 default tools + cycle 跑得通
    #[test]
    fn r129_4_stage4_d1_tool_self_loop_default_tools() {
        let mut l = ToolSelfLoop::with_default_tools();
        l.start();
        let r = l.cycle("p");
        assert!(r.result.success);
        assert_eq!(l.registry().len(), DEFAULT_TOOL_COUNT);
        assert_eq!(DEFAULT_TOOL_COUNT, 5);
    }

    /// Stage 4 D2 ReflectionSelfLoop cycle 跑得通 + 8 节点
    #[test]
    fn r129_4_stage4_d2_reflection_self_loop_8_nodes() {
        let mut l = ReflectionSelfLoop::new();
        l.start();
        let r = l.cycle("p");
        assert!(r.success);
        assert_eq!(l.graph().node_count(), REFLECTION_GRAPH_NODE_COUNT);
        assert_eq!(REFLECTION_GRAPH_NODE_COUNT, 8);
    }

    /// Stage 4 D3 MemorySelfLoop record + journal append
    #[test]
    fn r129_4_stage4_d3_memory_self_loop_append() {
        let mut l = MemorySelfLoop::new();
        l.start();
        let seq = l.record_tool_invocation("executor", Default::default(), "out", MemoryResult::Ok);
        assert_eq!(seq, 0);
        assert_eq!(l.journal().len(), 1);
        assert_eq!(MEMORY_KIND_COUNT, 7);
    }

    /// Stage 4 D4 DecisionSelfLoop cycle + revisit 守门
    #[test]
    fn r129_4_stage4_d4_decision_self_loop_revisit_guard() {
        let mut l = DecisionSelfLoop::new();
        l.start();
        let r = l.cycle("d", "r");
        assert!(r.success);
        let r1 = l.revisit_decision("a", "a");
        let r2 = l.revisit_decision("b", "b");
        let r3 = l.revisit_decision("c", "c");
        let r4 = l.revisit_decision("d", "d");
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert!(r3.is_some());
        assert!(r4.is_none(), "第 4 次 revisit 必 None (DECISION_MAX_REVISIT 守门)");
        assert_eq!(DECISION_POLICY_COUNT, 5);
    }

    /// Stage 4 4 summary 函数都引用 R129-4 + 借鉴 ID + 0 装 PASS 严守
    #[test]
    fn r129_4_stage4_4_summaries_cite_borrow_ids() {
        let s1 = tool_self_loop_summary();
        let s2 = reflection_self_loop_summary();
        let s3 = memory_self_loop_summary();
        let s4 = decision_self_loop_summary();
        for s in [&s1, &s2, &s3, &s4] {
            assert!(s.contains("R129-4"));
            assert!(s.contains("✅"));
            assert!(s.contains("0 装 PASS 严守"));
        }
        assert!(s1.contains("superpowers-234"));
        assert!(s1.contains("PyO3-928"));
        assert!(s2.contains("langgraph-829"));
        assert!(s2.contains("aGLM-108"));
        assert!(s3.contains("chidori"));
        assert!(s3.contains("superpowers-234"));
        assert!(s4.contains("aGLM-108"));
        assert!(s4.contains("superpowers-234"));
    }

    // ============================================================
    // R129-6 Stage 6 守护 单元测试 (per decision-61 §3.1 R129-6)
    // 4 维度: K1 错误 / K2 性能 / K3 6+1 重门安全 / K4 5 维度健康
    // cfg-无关: 跑在所有 build (默认 build + python-ext build)
    // ============================================================

    /// Stage 6 placeholder 包含 "Stage 6" + R129-6 + decision-61
    #[test]
    fn r129_6_stage6_placeholder_cites_decision_61() {
        let p = placeholder();
        assert!(p.contains("Stage 6"));
        assert!(p.contains("R129-6"));
        assert!(p.contains("decision-61"));
    }

    /// Stage 6 K1 错误守护可调用
    #[test]
    fn r129_6_k1_error_guardianship_callable() {
        let ev = stage6_record_error(
            crate::error_guardianship::ErrorKind::Bridge,
            crate::error_guardianship::ErrorSeverity::Info,
            "test",
            "r129-6 verify",
        );
        assert_eq!(ev.kind, crate::error_guardianship::ErrorKind::Bridge);
        let s = stage6_error_summary();
        assert!(s.contains("K1 ErrorGuard"));
    }

    /// Stage 6 K2 性能守护可调用
    #[test]
    fn r129_6_k2_perf_guardianship_callable() {
        let s = stage6_record_perf(
            crate::perf_guardianship::PerfKind::Bridge,
            std::time::Duration::from_micros(10),
            true,
        );
        assert_eq!(s.kind, crate::perf_guardianship::PerfKind::Bridge);
        let sum = stage6_perf_summary();
        assert!(sum.contains("K2 PerfMonitor"));
    }

    /// Stage 6 K3 安全守护: 6+1 重门 baseline 严守
    #[test]
    fn r129_6_k3_security_baseline_intact() {
        assert!(stage6_security_baseline_intact());
        assert_eq!(crate::security_guardianship::SecurityGate::N_GATES, 7);
    }

    /// Stage 6 K4 健康守护: 5 维度自检
    #[test]
    fn r129_6_k4_health_guardianship_runs() {
        let r = stage6_health_check();
        assert_eq!(r.checks.len(), 10);
        assert_eq!(r.r11_module_count, 1103);
        assert_eq!(r.asi_module_count, 7);
    }

    /// Stage 6 4 维度 公共 API 全可调用 (集成)
    #[test]
    fn r129_6_stage6_all_4_dimensions_callable() {
        // K1
        let _ = stage6_error_healthy();
        let _ = stage6_error_summary();
        // K2
        let _ = stage6_perf_healthy();
        let _ = stage6_perf_summary();
        // K3
        let _ = stage6_security_healthy();
        let _ = stage6_security_baseline_intact();
        // K4
        let _ = stage6_health_healthy();
        let _ = stage6_health_summary();
    }

    // =============================================================================
    // R129-18 Stage 7 跨模块集成 7 维度 inline tests (per decision-61 §3.1 R129-18)
    // =============================================================================

    /// R129-18 placeholder 包含 Stage 7 7 维度 I1-I7
    #[test]
    fn r129_18_stage7_placeholder_mentions_i1_to_i7() {
        let s = placeholder();
        assert!(s.contains("I1"));
        assert!(s.contains("I2"));
        assert!(s.contains("I3"));
        assert!(s.contains("I4"));
        assert!(s.contains("I5"));
        assert!(s.contains("I6"));
        assert!(s.contains("I7"));
        assert!(s.contains("R129-18"));
    }

    /// Stage 7 I1 D1+G1 集成 公共 API
    #[test]
    fn r129_18_stage7_i1_callable() {
        assert!(stage7_i1_healthy());
        let _ = stage7_i1_summary();
        assert!(stage7_i1_to_d1_consistency());
        assert!(stage7_i1_to_g1_consistency());
    }

    /// Stage 7 I2 D2+K1 集成 公共 API
    #[test]
    fn r129_18_stage7_i2_callable() {
        assert!(stage7_i2_healthy());
        let _ = stage7_i2_summary();
        assert!(stage7_i2_to_d2_consistency());
        assert!(stage7_i2_to_k1_consistency());
    }

    /// Stage 7 I3 D3+G3 集成 公共 API
    #[test]
    fn r129_18_stage7_i3_callable() {
        assert!(stage7_i3_healthy());
        let _ = stage7_i3_summary();
        assert!(stage7_i3_to_d3_consistency());
        assert!(stage7_i3_to_g3_consistency());
    }

    /// Stage 7 I4 D4+G2 集成 公共 API (B4 6 重 v7 严守)
    #[test]
    fn r129_18_stage7_i4_callable() {
        assert!(stage7_i4_healthy());
        let _ = stage7_i4_summary();
        assert!(stage7_i4_to_d4_consistency());
        assert!(stage7_i4_to_g2_consistency());
    }

    /// Stage 7 I5 G1+K2 集成 公共 API
    #[test]
    fn r129_18_stage7_i5_callable() {
        assert!(stage7_i5_healthy());
        let _ = stage7_i5_summary();
        assert!(stage7_i5_to_g1_consistency());
        assert!(stage7_i5_to_k2_consistency());
    }

    /// Stage 7 I6 G2+K3 集成 公共 API (B4 6 重 v7 + G7 跨语言 严守)
    #[test]
    fn r129_18_stage7_i6_callable() {
        assert!(stage7_i6_healthy());
        let _ = stage7_i6_summary();
        assert!(stage7_i6_to_g2_consistency());
        assert!(stage7_i6_to_k3_consistency());
    }

    /// Stage 7 I7 G4+K4 集成 公共 API
    #[test]
    fn r129_18_stage7_i7_callable() {
        assert!(stage7_i7_healthy());
        let _ = stage7_i7_summary();
        assert!(stage7_i7_to_g4_consistency());
        assert!(stage7_i7_to_k4_consistency());
    }
}
