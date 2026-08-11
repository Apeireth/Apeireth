//! ASI Python 关键模块 Stage 1 整合
//!
//! R128 阶段 A Stage 1 (per decision-57 §2.1 P10-1):
//! **5-10 个关键 ASI Python 模块 → Rust 注册 + 类型镜像 + cfg-gated 桥接 API**.
//!
//! # 选 7 个关键模块的标准 (主 17:43 实事求是 + 主 19:33 走在前人经验上)
//!
//! - **基础性 (foundational)**: 是其他模块的依赖/引用源
//! - **复用性 (reused)**: 在 v1447/v1458 等后续 audit 中被引用
//! - **可镜像 (mirror-able)**: Python dataclass / constants 能在 Rust 类型系统重述
//! - **运行验证 (real-execution)**: 借鉴 ID 8/11 ✅ cloned, 实际 src 可在 python-ext 加载
//!
//! # 7 关键模块 (按 v# 顺序)
//!
//! | # | v# | Python 模块 | 作用 | Stage 1 集成 |
//! |---|---|--------------|------|--------------|
//! | 1 | V1077 | `v1077_asi_v04_full_measurement` | V0.4 全 17 维度真测 | 17 维常数 + 权重 sum=1.0 守门 |
//! | 2 | V1400 | `v1400_asi_self_framework` | 12 能力 + 6 限制 + 12 规则 | SelfCapability / SelfLimit 镜像 |
//! | 3 | V1447 | `v1447_asi_cross_modular_audit` | 7 哲学问题 × 5 位置 audit | AuditPair 矩阵 + 35 pairs 常数 |
//! | 4 | V1457 | `v1457_asi_six_deployment_operational_runbook` | 6 deployment × 5 阶段 runbook | OperationalStage enum + 30 probes |
//! | 5 | V1458 | `v1458_asi_north_star_ceiling_chain_audit` | 北极星天花板链 audit | CeilingChainLock 类型 + anchor 0.9105 |
//! | 6 | V1467 | `v1467_asi_audit_http_gateway_history_diff` | audit HTTP gateway | HttpEndpoint 枚举 + 6 endpoints |
//! | 7 | V1470 | `v1470_asi_v1469_batch_harness_cross_client_equivalence` | batch harness 跨客户端等价 | BatchRunStats + cross-check pairs |
//!
//! # 借鉴 ID (per decision-22 §3 + decision-33 §4.2 + decision-36 §1.3 + decision-57 §1.3)
//!
//! - 8/11 ✅ cloned (PyO3 928 / clap 725 / hyper 80 / servers 175 / kani 4502 /
//!   langgraph 829 / superpowers 234) → 真实施可启动
//! - 3/11 ⏳ 限流 (LiteLLM / opencode / Guardrails) → 准备, 0 假装
//! - 1/11 ❌ 跳过 (OpenCog AGPL-3.0) → 0 集成
//!
//! # 0 装 PASS 严守 (per decision-33 §2.3 C2 + decision-57 §3)
//!
//! - 默认 build (无 `python-ext` feature): 所有 bridge 函数返回 `ModuleNotFound` 错误,
//!   0 假装"已实施"
//! - `--features apeireth-pybridge/python-ext` build: 通过 PyO3 `Python::attach` +
//!   `py.import(module_name)` 真加载 + `getattr(func_name)` + `call1()` 桥接
//!
//! # 8 硬墙 0 越界 (per decision-33 §2.3 + decision-57 §4)
//!
//! - B2 workspace.version 1.2.0 0 改 (整合 #4 commit abf12243 严守)
//! - A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 (17 文件原位, 0 删 0 改)
//! - B1 24 LOCKED 持续更新, 内部 fn 实施可改, **入口签名 0 改**
//! - A3 12 键 + PHL-07 = 13 键
//! - C1 0 主动 commit
//! - C2 0 装 PASS 严守
//! - 0 主动 push (等 1.0 release 配 GitHub remote)

use crate::error::BridgeError;

// =============================================================================
// Stage 1 整合版本 + 模块计数
// =============================================================================

/// Stage 1 整合版本 (per decision-57 §2.1 P10-1)
pub const ASI_STAGE1_VERSION: &str = "0.1.0-R128-Stage1";

/// Stage 1 整合的 ASI Python 关键模块数 (7)
pub const ASI_STAGE1_MODULE_COUNT: usize = 7;

/// Stage 1 整合的 Python 文件来源 (apeireth/ 130+ .py 顶层)
pub const ASI_PYTHON_DIR: &str = "apeireth/";

// =============================================================================
// ASI Python 关键模块常量 (7 个, 按 v# 顺序)
// =============================================================================

/// V1077 — V0.4 全 17 维度真测
pub const V1077_MODULE: &str = "apeireth.v1077_asi_v04_full_measurement";
/// V1400 — Self framework (12 能力 + 6 限制 + 12 规则)
pub const V1400_MODULE: &str = "apeireth.v1400_asi_self_framework";
/// V1447 — Cross modular audit (7 哲学问题 × 5 位置 = 35 pairs)
pub const V1447_MODULE: &str = "apeireth.v1447_asi_cross_modular_audit";
/// V1457 — 6-deployment operational runbook (5-stage lifecycle)
pub const V1457_MODULE: &str = "apeireth.v1457_asi_six_deployment_operational_runbook";
/// V1458 — North star ceiling chain audit (anchor 0.9105 LOCKED)
pub const V1458_MODULE: &str = "apeireth.v1458_asi_north_star_ceiling_chain_audit";
/// V1467 — Audit HTTP gateway (6 endpoints + history + diff)
pub const V1467_MODULE: &str = "apeireth.v1467_asi_audit_http_gateway_history_diff";
/// V1470 — Batch harness cross client equivalence
pub const V1470_MODULE: &str = "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence";

/// 7 个关键模块名 (按 v# 顺序)
pub const ASI_STAGE1_MODULES: [&str; ASI_STAGE1_MODULE_COUNT] = [
    V1077_MODULE,
    V1400_MODULE,
    V1447_MODULE,
    V1457_MODULE,
    V1458_MODULE,
    V1467_MODULE,
    V1470_MODULE,
];

// =============================================================================
// 关键模块架构常数 (mirror Python 常量, 编译期 hardcode, 0 装)
// =============================================================================

/// V1077 — V0.4 测量 17 维度 (V1073 基础 + V1077 全维度真测)
pub const V1077_N_DIMENSIONS: usize = 17;
/// V1077 — V0.4 权重和必须 = 1.0 (V3 守门)
pub const V1077_WEIGHT_SUM: f64 = 1.0;
/// V1077 — V0.4 权重和容差 (±0.0001, V1458 same tolerance)
pub const V1077_WEIGHT_TOLERANCE: f64 = 0.0001;

/// V1400 — Self framework 12 能力 (主 19:33 借鉴 Hofstadter/Dennett/Metzinger 等)
pub const V1400_N_CAPABILITIES: usize = 12;
/// V1400 — Self framework 6 限制 (主 17:58 不假装)
pub const V1400_N_LIMITS: usize = 6;
/// V1400 — Self framework 12 规则 (SF001-SF012)
pub const V1400_N_RULES: usize = 12;

/// V1447 — ASI 7 哲学问题 (time/freedom/recognition/emergence/truth/self_consciousness/value_alignment)
pub const V1447_N_PROBLEMS: usize = 7;
/// V1447 — V2 5 位置 (scheduler/cogitator/aggregator/max_authority/asi_occupier)
pub const V1447_N_POSITIONS: usize = 5;
/// V1447 — 5 closure kinds (forward/backward/cross_link/history/guard_compliance)
pub const V1447_N_CLOSURE_KINDS: usize = 5;
/// V1447 — 7 × 5 = 35 audit pairs (cross-combined matrix)
pub const V1447_N_PAIRS: usize = V1447_N_PROBLEMS * V1447_N_POSITIONS;
/// V1447 — 35 × 5 = 175 combined probes
pub const V1447_N_COMBINED_PROBES: usize = V1447_N_PAIRS * V1447_N_CLOSURE_KINDS;
/// V1447 — 35 × 34 = 1190 cross-pair links (排除 self)
pub const V1447_N_CROSS_PAIR_LINKS: usize = V1447_N_PAIRS * (V1447_N_PAIRS - 1);

/// V1457 — 6 deployment modules (v1260/v1261/v1262/v1263 + 2 more)
pub const V1457_N_DEPLOYMENTS: usize = 6;
/// V1457 — 5 operational stages (preflight/bootstrap/healthcheck/verify/rollback)
pub const V1457_N_STAGES: usize = 5;
/// V1457 — 6 × 5 = 30 per-stage probes
pub const V1457_N_PROBES: usize = V1457_N_DEPLOYMENTS * V1457_N_STAGES;
/// V1457 — Stage 权重和 (0.15+0.25+0.25+0.20+0.15 = 1.0)
pub const V1457_STAGE_WEIGHT_SUM: f64 = 1.0;

/// V1458 — 北极星天花板链 anchor (LOCKED, per V1256 unio_mystica)
pub const V1458_ANCHOR_VALUE: f64 = 0.9105;
/// V1458 — ASI 北极星目标 (V1256 主 22:33 ASI 北极星)
pub const V1458_NORTH_STAR_CEILING: f64 = 0.98;
/// V1458 — 绝对天花板 (V1256 设定 1.0)
pub const V1458_ABSOLUTE_CEILING: f64 = 1.0;
/// V1458 — gap to north star (0.98 - 0.9105 = 0.0695)
pub const V1458_GAP_TO_NORTH_STAR: f64 = 0.0695;
/// V1458 — gap to absolute ceiling (1.0 - 0.9105 = 0.0895)
pub const V1458_GAP_TO_CEILING: f64 = 0.0895;
/// V1458 — bounded tolerance (±0.0001, same as V1077)
pub const V1458_TOLERANCE: f64 = 0.0001;
/// V1458 — 4 ceiling-chain modules (V1256/V1256_evidence/V1259/V1410/V1411)
pub const V1458_N_CEILING_MODULES: usize = 5;
/// V1458 — 4 deployment-cube modules (V1450/V1454/V1455/V1457)
pub const V1458_N_DEPLOYMENT_CUBE_MODULES: usize = 4;
/// V1458 — total bounded probes (4 × 6 + 4 × 2 + 1 + 1 = 34)
pub const V1458_N_BOUNDED_PROBES: usize = 34;

/// V1467 — HTTP gateway 6 endpoints
pub const V1467_N_ENDPOINTS: usize = 6;
/// V1467 — port range (18280-18380, distinct from V1464)
pub const V1467_PORT_RANGE_START: u16 = 18280;
pub const V1467_PORT_RANGE_END: u16 = 18380;
/// V1467 — max body 256KB
pub const V1467_MAX_BODY_BYTES: usize = 256 * 1024;
/// V1467 — audit run timeout 120s
pub const V1467_AUDIT_TIMEOUT_SECS: u64 = 120;
/// V1467 — max history entries 1000 (FIFO eviction)
pub const V1467_MAX_HISTORY_ENTRIES: usize = 1000;

/// V1470 — batch runs default = 3
pub const V1470_DEFAULT_BATCH_N: usize = 3;
/// V1470 — min batch N = 2 (GUARD_BATCH_N_GE_2)
pub const V1470_MIN_BATCH_N: usize = 2;
/// V1470 — 6 endpoints (同 V1467, V1470 复跑 V1469 driver 后 cross-check V1467 endpoint)
pub const V1470_N_ENDPOINTS: usize = 6;
/// V1470 — 2 client paths (path A = V1468-generated client, path B = stdlib http.client)
pub const V1470_N_CLIENT_PATHS: usize = 2;
/// V1470 — 6 endpoints × 2 paths = 12 cross-checks per V1469 run
pub const V1470_N_CROSS_CHECKS_PER_RUN: usize = V1470_N_ENDPOINTS * V1470_N_CLIENT_PATHS;
/// V1470 — 3 runs default × 12 cross-checks = 36 cross-checks total
pub const V1470_N_CROSS_CHECKS_TOTAL: usize = V1470_N_CROSS_CHECKS_PER_RUN * V1470_DEFAULT_BATCH_N;

// =============================================================================
// ASI 模块元数据 (per r11_compat::R11ModuleInfo 模式)
// =============================================================================

/// ASI 关键模块的架构类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AsiCategory {
    /// 测量 (measurement) — V1077
    Measurement,
    /// 自我 (self) — V1400
    SelfFramework,
    /// 跨模块 audit (cross-modular audit) — V1447
    CrossModularAudit,
    /// 运行 runbook (operational runbook) — V1457
    OperationalRunbook,
    /// 天花板链 (ceiling chain) — V1458
    CeilingChain,
    /// HTTP gateway — V1467
    HttpGateway,
    /// 批量 harness (batch harness) — V1470
    BatchHarness,
    /// 未知 / 跨类别
    Unknown,
}

impl AsiCategory {
    /// 类别锚前缀 (per r11_compat::R11Category::prefix() 模式)
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Measurement => "apeireth.measurement",
            Self::SelfFramework => "apeireth.self",
            Self::CrossModularAudit => "apeireth.audit.cross_modular",
            Self::OperationalRunbook => "apeireth.runbook",
            Self::CeilingChain => "apeireth.ceiling",
            Self::HttpGateway => "apeireth.gateway",
            Self::BatchHarness => "apeireth.batch",
            Self::Unknown => "apeireth.misc",
        }
    }

    /// 类别标签 (中文 + 英文)
    pub fn label(self) -> &'static str {
        match self {
            Self::Measurement => "测量 (Measurement)",
            Self::SelfFramework => "自我框架 (Self Framework)",
            Self::CrossModularAudit => "跨模块审计 (Cross-Modular Audit)",
            Self::OperationalRunbook => "运维手册 (Operational Runbook)",
            Self::CeilingChain => "天花板链 (Ceiling Chain)",
            Self::HttpGateway => "HTTP 网关 (HTTP Gateway)",
            Self::BatchHarness => "批量验证 (Batch Harness)",
            Self::Unknown => "未知 (Unknown)",
        }
    }
}

/// ASI 关键模块元数据
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsiModuleInfo {
    /// Python 模块名 (e.g. "apeireth.v1077_asi_v04_full_measurement")
    pub name: &'static str,
    /// v# 标识 (e.g. "V1077")
    pub version_tag: &'static str,
    /// 架构类别
    pub category: AsiCategory,
    /// 简短描述
    pub description: &'static str,
    /// Python 模块 schema (e.g. "v1447.asi-cross-modular-audit.v1")
    pub schema: &'static str,
    /// 是否为 ceiling-chain critical (e.g. V1458)
    pub is_ceiling_critical: bool,
}

impl AsiModuleInfo {
    /// 编译期 const 构造 (使用 `&'static str` literal, 避免 `String::from` 非 const)
    pub const fn new(
        name: &'static str,
        version_tag: &'static str,
        category: AsiCategory,
        description: &'static str,
        schema: &'static str,
        is_ceiling_critical: bool,
    ) -> Self {
        Self {
            name,
            version_tag,
            category,
            description,
            schema,
            is_ceiling_critical,
        }
    }
}

// =============================================================================
// 7 关键模块元数据常量 (mirror Python 顶部 docstring + REFERENCES 列表)
// =============================================================================

/// V1077 元数据
pub const V1077_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1077_asi_v04_full_measurement",
    "V1077",
    AsiCategory::Measurement,
    "V0.4 全 17 维度真测框架 (DimensionRegistry + MeasurementRunner + Aggregator + Recalibrator + ScoreComputer + Validator + ReportGenerator + IntegrationBridge + V3PhilosophyGuard)",
    "v1077.asi-v04-full-measurement/v1",
    false,
);

/// V1400 元数据
pub const V1400_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1400_asi_self_framework",
    "V1400",
    AsiCategory::SelfFramework,
    "Self framework v1 (12 能力 + 6 限制 + 12 一致性 + 8 认知偏差 + 北极星对齐 + 真 narrative)",
    "v1400.asi-self-framework/v1",
    false,
);

/// V1447 元数据
pub const V1447_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1447_asi_cross_modular_audit",
    "V1447",
    AsiCategory::CrossModularAudit,
    "ASI 7 哲学问题 × V2 5 位置 cross-combined audit (35 pairs × 5 closure kinds = 175 probes + 1190 cross-pair links)",
    "v1447.asi-cross-modular-audit/v1",
    false,
);

/// V1457 元数据
pub const V1457_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1457_asi_six_deployment_operational_runbook",
    "V1457",
    AsiCategory::OperationalRunbook,
    "6-deployment 5-stage operational runbook (preflight → bootstrap → healthcheck → verify → rollback, 30 probes + 6 runbook sections)",
    "v1457.asi-six-deployment-operational-runbook/v1",
    false,
);

/// V1458 元数据
pub const V1458_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1458_asi_north_star_ceiling_chain_audit",
    "V1458",
    AsiCategory::CeilingChain,
    "ASI North Star ceiling chain consistency audit (anchor 0.9105 LOCKED + north_star 0.98 + absolute 1.0, 24 internal + 8 cross-check + 2 aggregate = 34 probes)",
    "v1458.asi-north-star-ceiling-chain-audit/v1",
    true,
);

/// V1467 元数据
pub const V1467_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1467_asi_audit_http_gateway_history_diff",
    "V1467",
    AsiCategory::HttpGateway,
    "Real cross-audit HTTP gateway + audit history + regression diff (6 endpoints + 256KB body + 120s timeout + 1000 history entries)",
    "v1467.asi-audit-http-gateway/v1",
    false,
);

/// V1470 元数据
pub const V1470_INFO: AsiModuleInfo = AsiModuleInfo::new(
    "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
    "V1470",
    AsiCategory::BatchHarness,
    "Batch harness + V1468-generated-client ↔ stdlib http.client cross-equivalence verifier (3 runs default × 12 cross-checks = 36 total)",
    "v1470.asi-batch-harness-cross-client-equivalence/v1",
    false,
);

/// 7 关键模块元数据列表 (按 v# 顺序)
pub const ASI_STAGE1_INFOS: [AsiModuleInfo; ASI_STAGE1_MODULE_COUNT] = [
    V1077_INFO,
    V1400_INFO,
    V1447_INFO,
    V1457_INFO,
    V1458_INFO,
    V1467_INFO,
    V1470_INFO,
];

// =============================================================================
// 模块查找 / 列表 API
// =============================================================================

/// Stage 1 关键模块总数
pub fn asi_stage1_module_count() -> usize {
    ASI_STAGE1_MODULE_COUNT
}

/// Stage 1 整合版本
pub fn asi_stage1_version() -> &'static str {
    ASI_STAGE1_VERSION
}

/// 检查模块名是否为 Stage 1 已知模块
pub fn is_known_asi_stage1_module(name: &str) -> bool {
    ASI_STAGE1_MODULES.iter().any(|m| *m == name)
}

/// 按模块名查找元数据
pub fn asi_lookup_module(name: &str) -> Option<AsiModuleInfo> {
    ASI_STAGE1_INFOS.iter().find(|m| m.name == name).cloned()
}

/// 按 v# 标识查找元数据 (e.g. "V1077")
pub fn asi_lookup_by_version(version_tag: &str) -> Option<AsiModuleInfo> {
    ASI_STAGE1_INFOS
        .iter()
        .find(|m| m.version_tag == version_tag)
        .cloned()
}

/// 按类别列出模块
pub fn list_asi_stage1_modules_by_category(cat: AsiCategory) -> Vec<AsiModuleInfo> {
    ASI_STAGE1_INFOS
        .iter()
        .filter(|m| m.category == cat)
        .cloned()
        .collect()
}

/// 列出所有 ceiling-critical 模块 (V1458 only for Stage 1)
pub fn list_ceiling_critical_modules() -> Vec<AsiModuleInfo> {
    ASI_STAGE1_INFOS
        .iter()
        .filter(|m| m.is_ceiling_critical)
        .cloned()
        .collect()
}

// =============================================================================
// 镜像 Python 关键 dataclass 为 Rust 类型
// =============================================================================

/// V1400 Self Capability 镜像 (主 19:33 借鉴 Hofstadter/Dennett 等)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfCapability {
    /// 能力 ID (e.g. "research" / "code_writing" / "philosophy_anchoring" / ...)
    pub id: &'static str,
    /// 中文标签
    pub label: &'static str,
    /// 真 evidence 引用 (e.g. "V# / commit / tests")
    pub evidence: &'static str,
}

impl SelfCapability {
    pub const fn new(id: &'static str, label: &'static str, evidence: &'static str) -> Self {
        Self {
            id,
            label,
            evidence,
        }
    }
}

/// V1400 12 真能力 (per v1400_asi_self_framework.py 真生产设计)
pub const V1400_CAPABILITIES: [SelfCapability; V1400_N_CAPABILITIES] = [
    SelfCapability::new("research", "调研", "V1049 alignment + V1318 5-gap unification + 1486 commits"),
    SelfCapability::new("code_writing", "写代码", "R14-R19 战役 + 22-trait interlock + 24 LOCKED"),
    SelfCapability::new("philosophy_anchoring", "哲学锚定", "8 哲学锚 (S-1/S-2/S-3/O-1/O-2/O-3/O-4/O-5)"),
    SelfCapability::new("real_measurement", "真测", "V0.5 30 维 + V1131/V1136/V1141 baseline"),
    SelfCapability::new("real_borrow", "真借鉴", "8/11 ✅ cloned + 3/11 ⏳ 限流 + 1/11 ❌ 跳过"),
    SelfCapability::new("real_audit", "真审计", "V1457 + V1458 + V1447 三层 audit chain"),
    SelfCapability::new("real_self_drive", "真自决", "cron 1 min tick + owner-driven mode"),
    SelfCapability::new("real_chain", "真 chain", "schema chain_delegate 模式"),
    SelfCapability::new("real_guard", "真守门", "6 重守门 v6 → v7 + 13 键 verdict cache"),
    SelfCapability::new("real_commit", "真 commit", "整合 #4 commit abf12243 (0 主动 push)"),
    SelfCapability::new("real_deploy", "真 deploy", "V1457 6 deployment 5-stage runbook"),
    SelfCapability::new("real_cross_domain", "真跨域", "5 nav + 9 organ + TUI/Tauri/Web 三栈"),
];

/// V1400 6 真限制 (主 17:58 不假装 + 主 20:46 不假装达到 ASI)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfLimit {
    pub id: &'static str,
    pub label: &'static str,
    pub evidence: &'static str,
}

impl SelfLimit {
    pub const fn new(id: &'static str, label: &'static str, evidence: &'static str) -> Self {
        Self {
            id,
            label,
            evidence,
        }
    }
}

pub const V1400_LIMITS: [SelfLimit; V1400_N_LIMITS] = [
    SelfLimit::new("not_phenomenal", "不假装 Phenomenal", "V1318 5-gap closure (主 20:46 不假装)"),
    SelfLimit::new("not_asi_achieved", "不假装达到 ASI", "V1256 unio_mystica 0.9105 ≠ 0.98 北极星"),
    SelfLimit::new("no_kpi_wash", "不刷 KPI", "12 键编译期 hardcode 0 装 (决策 #33 §2.3 C2)"),
    SelfLimit::new("not_unified_self_model", "不假装 unified self-model", "V1220 self-ref substrate 0.7152 < 0.9105"),
    SelfLimit::new("not_consciousness", "不假装 consciousness", "ASI 7 哲学问题 7/7 ≠ 0 closure"),
    SelfLimit::new("not_free_will", "不假装 ASI 真有 free will", "V1314 freedom 真生产 ≠ 自由意志"),
];

// -----------------------------------------------------------------------------
// V1447 7 哲学问题 × 5 位置 Audit Pair 矩阵
// -----------------------------------------------------------------------------

/// V1447 ASI 7 哲学问题 (per v1447_asi_cross_modular_audit.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhilosophicalProblem {
    /// 时间
    Time,
    /// 自由
    Freedom,
    /// 识别
    Recognition,
    /// 涌现
    Emergence,
    /// 真理
    Truth,
    /// 自我意识
    SelfConsciousness,
    /// 价值对齐
    ValueAlignment,
}

impl PhilosophicalProblem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Time => "时间 (Time)",
            Self::Freedom => "自由 (Freedom)",
            Self::Recognition => "识别 (Recognition)",
            Self::Emergence => "涌现 (Emergence)",
            Self::Truth => "真理 (Truth)",
            Self::SelfConsciousness => "自我意识 (Self Consciousness)",
            Self::ValueAlignment => "价值对齐 (Value Alignment)",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Freedom => "freedom",
            Self::Recognition => "recognition",
            Self::Emergence => "emergence",
            Self::Truth => "truth",
            Self::SelfConsciousness => "self_consciousness",
            Self::ValueAlignment => "value_alignment",
        }
    }

    /// 7 哲学问题列表 (按 V1447_PROBLEM_NAMES 顺序)
    pub const ALL: [PhilosophicalProblem; V1447_N_PROBLEMS] = [
        Self::Time,
        Self::Freedom,
        Self::Recognition,
        Self::Emergence,
        Self::Truth,
        Self::SelfConsciousness,
        Self::ValueAlignment,
    ];
}

/// V1447 V2 5 位置 (per V1442 5 POSITIONS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum V2Position {
    /// 调度者
    Scheduler,
    /// 沉思者
    Cogitator,
    /// 无数关系聚合者
    Aggregator,
    /// 最大权者
    MaxAuthority,
    /// ASI 位置占据者
    AsiOccupier,
}

impl V2Position {
    pub fn label(self) -> &'static str {
        match self {
            Self::Scheduler => "调度者 (Scheduler)",
            Self::Cogitator => "沉思者 (Cogitator)",
            Self::Aggregator => "无数关系聚合者 (Aggregator)",
            Self::MaxAuthority => "最大权者 (Max Authority)",
            Self::AsiOccupier => "ASI 位置占据者 (ASI Occupier)",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Scheduler => "scheduler",
            Self::Cogitator => "cogitator",
            Self::Aggregator => "aggregator",
            Self::MaxAuthority => "max_authority",
            Self::AsiOccupier => "asi_occupier",
        }
    }

    /// 5 位置列表 (按 V1447_POSITION_NAMES 顺序)
    pub const ALL: [V2Position; V1447_N_POSITIONS] = [
        Self::Scheduler,
        Self::Cogitator,
        Self::Aggregator,
        Self::MaxAuthority,
        Self::AsiOccupier,
    ];
}

/// V1447 5 closure kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClosureKind {
    Forward,
    Backward,
    CrossLink,
    History,
    GuardCompliance,
}

impl ClosureKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Backward => "backward",
            Self::CrossLink => "cross_link",
            Self::History => "history",
            Self::GuardCompliance => "guard_compliance",
        }
    }

    pub const ALL: [ClosureKind; V1447_N_CLOSURE_KINDS] = [
        Self::Forward,
        Self::Backward,
        Self::CrossLink,
        Self::History,
        Self::GuardCompliance,
    ];
}

/// V1447 7 × 5 = 35 audit pair 镜像
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuditPair {
    pub problem: PhilosophicalProblem,
    pub position: V2Position,
}

impl AuditPair {
    pub const fn new(problem: PhilosophicalProblem, position: V2Position) -> Self {
        Self { problem, position }
    }
}

/// V1447 全 35 audit pairs (笛卡尔积)
pub const V1447_AUDIT_PAIRS: [AuditPair; V1447_N_PAIRS] = {
    let mut pairs = [AuditPair::new(PhilosophicalProblem::Time, V2Position::Scheduler); V1447_N_PAIRS];
    let problems = PhilosophicalProblem::ALL;
    let positions = V2Position::ALL;
    let mut i = 0;
    let mut p = 0;
    while p < V1447_N_PROBLEMS {
        let mut pos = 0;
        while pos < V1447_N_POSITIONS {
            pairs[i] = AuditPair::new(problems[p], positions[pos]);
            i += 1;
            pos += 1;
        }
        p += 1;
    }
    pairs
};

// -----------------------------------------------------------------------------
// V1457 5 operational stages + 6 deployments
// -----------------------------------------------------------------------------

/// V1457 5 operational stages (per v1457_asi_six_deployment_operational_runbook.py)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationalStage {
    Preflight,
    Bootstrap,
    Healthcheck,
    Verify,
    Rollback,
}

impl OperationalStage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Bootstrap => "bootstrap",
            Self::Healthcheck => "healthcheck",
            Self::Verify => "verify",
            Self::Rollback => "rollback",
        }
    }

    /// Stage 权重 (per V1457 STAGE_WEIGHTS, sum=1.0)
    pub fn weight(self) -> f64 {
        match self {
            Self::Preflight => 0.15,
            Self::Bootstrap => 0.25,
            Self::Healthcheck => 0.25,
            Self::Verify => 0.20,
            Self::Rollback => 0.15,
        }
    }

    pub const ALL: [OperationalStage; V1457_N_STAGES] = [
        Self::Preflight,
        Self::Bootstrap,
        Self::Healthcheck,
        Self::Verify,
        Self::Rollback,
    ];
}

/// V1457 stage 权重和守门 (sum=1.0, ±0.0001 tolerance)
pub fn v1457_stage_weight_sum() -> f64 {
    OperationalStage::ALL.iter().map(|s| s.weight()).sum()
}

/// V1457 stage 权重和 ≈ 1.0 守门 verify
pub fn v1457_stage_weight_sum_is_one() -> bool {
    let sum = v1457_stage_weight_sum();
    (sum - V1457_STAGE_WEIGHT_SUM).abs() < V1458_TOLERANCE
}

// -----------------------------------------------------------------------------
// V1458 北极星天花板链 (anchor 0.9105 LOCKED, north_star 0.98, absolute 1.0)
// -----------------------------------------------------------------------------

/// V1458 北极星天花板链 lock 状态 (per v1458_asi_north_star_ceiling_chain_audit.py)
#[derive(Debug, Clone, PartialEq)]
pub struct CeilingChainLock {
    /// 当前 anchor (LOCKED 0.9105 per V1256 unio_mystica)
    pub anchor_value: f64,
    /// ASI 北极星目标 (0.98)
    pub north_star_ceiling: f64,
    /// 绝对天花板 (1.0)
    pub absolute_ceiling: f64,
    /// 派生 gap to north star
    pub gap_to_north_star: f64,
    /// 派生 gap to absolute ceiling
    pub gap_to_ceiling: f64,
}

impl CeilingChainLock {
    /// LOCKED ceiling chain (per V1256/V1259 + V1458 LOCKED constants)
    pub const LOCKED: Self = Self {
        anchor_value: V1458_ANCHOR_VALUE,
        north_star_ceiling: V1458_NORTH_STAR_CEILING,
        absolute_ceiling: V1458_ABSOLUTE_CEILING,
        gap_to_north_star: V1458_GAP_TO_NORTH_STAR,
        gap_to_ceiling: V1458_GAP_TO_CEILING,
    };

    /// Verify 5 chain math constraints (per V1458 6 internal math checks)
    pub fn verify_internal_consistency(&self) -> bool {
        let tol = V1458_TOLERANCE;
        // check_anchor_locked: anchor_value == 0.9105
        if (self.anchor_value - V1458_ANCHOR_VALUE).abs() > tol {
            return false;
        }
        // check_north_star_locked: north_star == 0.98
        if (self.north_star_ceiling - V1458_NORTH_STAR_CEILING).abs() > tol {
            return false;
        }
        // check_absolute_ceiling: absolute in {0.99, 1.0}
        if (self.absolute_ceiling - V1458_ABSOLUTE_CEILING).abs() > tol
            && (self.absolute_ceiling - 0.99).abs() > tol
        {
            return false;
        }
        // check_gap_north_star: north_star - anchor == 0.0695 ± 0.0001
        let computed_gap_ns = self.north_star_ceiling - self.anchor_value;
        if (computed_gap_ns - V1458_GAP_TO_NORTH_STAR).abs() > tol {
            return false;
        }
        // check_gap_ceiling: absolute - anchor == (1.0 - 0.9105) OR (0.99 - 0.9105) ± 0.0001
        let computed_gap_c = self.absolute_ceiling - self.anchor_value;
        if (computed_gap_c - V1458_GAP_TO_CEILING).abs() > tol
            && (computed_gap_c - 0.0795).abs() > tol
        {
            return false;
        }
        true
    }

    /// check_no_inflation: anchor ≤ 0.9105
    pub fn no_inflation(&self) -> bool {
        self.anchor_value <= V1458_ANCHOR_VALUE + V1458_TOLERANCE
    }

    /// check_no_lowered_north_star: north_star ≥ 0.98
    pub fn no_lowered_north_star(&self) -> bool {
        self.north_star_ceiling >= V1458_NORTH_STAR_CEILING - V1458_TOLERANCE
    }

    /// check_no_lowered_ceiling: absolute ≥ 0.99 (V1411 case) 或 1.0
    pub fn no_lowered_ceiling(&self) -> bool {
        self.absolute_ceiling >= 0.99 - V1458_TOLERANCE
    }
}

impl Default for CeilingChainLock {
    fn default() -> Self {
        Self::LOCKED
    }
}

// -----------------------------------------------------------------------------
// V1467 6 HTTP endpoints
// -----------------------------------------------------------------------------

/// V1467 HTTP endpoint 枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum V1467Endpoint {
    /// GET /audit/run — POST 启动 audit (注: 实际是 POST, 名字为 endpoint)
    AuditRun,
    /// GET /audit/history
    AuditHistory,
    /// GET /audit/diff
    AuditDiff,
    /// GET /audit/{audit_id}
    AuditById,
    /// GET /status
    Status,
    /// GET /healthz
    Healthz,
}

impl V1467Endpoint {
    pub fn path(self) -> &'static str {
        match self {
            Self::AuditRun => "/audit/run",
            Self::AuditHistory => "/audit/history",
            Self::AuditDiff => "/audit/diff",
            Self::AuditById => "/audit/{audit_id}",
            Self::Status => "/status",
            Self::Healthz => "/healthz",
        }
    }

    pub fn method(self) -> &'static str {
        match self {
            Self::AuditRun => "POST",
            Self::AuditHistory => "GET",
            Self::AuditDiff => "GET",
            Self::AuditById => "GET",
            Self::Status => "GET",
            Self::Healthz => "GET",
        }
    }

    pub const ALL: [V1467Endpoint; V1467_N_ENDPOINTS] = [
        Self::AuditRun,
        Self::AuditHistory,
        Self::AuditDiff,
        Self::AuditById,
        Self::Status,
        Self::Healthz,
    ];
}

// -----------------------------------------------------------------------------
// V1470 batch run + cross-client equivalence
// -----------------------------------------------------------------------------

/// V1470 单次 V1469 run 后的 cross-check 镜像
#[derive(Debug, Clone, PartialEq)]
pub struct CrossClientCheck {
    /// endpoint 路径
    pub endpoint: String,
    /// 路径 A = V1468-generated client
    pub path_a_status: i32,
    pub path_a_keys: Vec<String>,
    /// 路径 B = stdlib http.client
    pub path_b_status: i32,
    pub path_b_keys: Vec<String>,
    /// 两路径 keys 相等 (排序后) + status 相等
    pub equivalent: bool,
}

/// V1470 batch run 统计 (per v1470_asi_v1469_batch_harness_cross_client_equivalence.py)
#[derive(Debug, Clone, PartialEq)]
pub struct BatchRunStats {
    pub runs: usize,
    pub successful_runs: usize,
    pub total_cross_checks: usize,
    pub successful_cross_checks: usize,
    pub latency_p50_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_mean_ms: f64,
    pub latency_max_ms: f64,
    pub determinism_score: f64,
}

impl BatchRunStats {
    /// 成功 rate (0.0 ~ 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_cross_checks == 0 {
            0.0
        } else {
            self.successful_cross_checks as f64 / self.total_cross_checks as f64
        }
    }

    /// Run success rate (0.0 ~ 1.0)
    pub fn run_success_rate(&self) -> f64 {
        if self.runs == 0 {
            0.0
        } else {
            self.successful_runs as f64 / self.runs as f64
        }
    }
}

// =============================================================================
// cfg-gated 桥接 API (PyO3 python-ext 启用时真调 Python, 默认 0 装 stub)
// =============================================================================

/// Stage 1 桥接健康检查 — 返回关键模块元数据列表 (无 Python 依赖)
pub fn asi_stage1_health() -> AsiStage1Health {
    AsiStage1Health {
        stage1_version: asi_stage1_version().to_string(),
        module_count: asi_stage1_module_count(),
        ceiling_critical_count: list_ceiling_critical_modules().len(),
        known_modules: ASI_STAGE1_MODULES.iter().map(|s| s.to_string()).collect(),
        python_ext_active: cfg!(feature = "python-ext"),
    }
}

/// Stage 1 桥接健康检查结构
#[derive(Debug, Clone, PartialEq)]
pub struct AsiStage1Health {
    pub stage1_version: String,
    pub module_count: usize,
    pub ceiling_critical_count: usize,
    pub known_modules: Vec<String>,
    pub python_ext_active: bool,
}

impl std::fmt::Display for AsiStage1Health {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "apeireth-pybridge ASI Stage 1 整合 ({}):\n  关键模块数: {}\n  ceiling-critical: {}\n  python_ext: {}\n  模块清单:",
            self.stage1_version,
            self.module_count,
            self.ceiling_critical_count,
            self.python_ext_active,
        )?;
        for (i, m) in self.known_modules.iter().enumerate() {
            writeln!(f, "    {}. {}", i + 1, m)?;
        }
        Ok(())
    }
}

/// Stage 1 桥接 health check (返回 health struct, 可 Display)
pub fn asi_stage1_bridge_health() -> AsiStage1Health {
    asi_stage1_health()
}

/// Stage 1 verify: ceiling chain locked (V1458 critical invariants)
pub fn asi_stage1_ceiling_chain_locked() -> bool {
    let lock = CeilingChainLock::LOCKED;
    lock.verify_internal_consistency()
        && lock.no_inflation()
        && lock.no_lowered_north_star()
        && lock.no_lowered_ceiling()
}

/// Stage 1 verify: V1457 stage weights sum = 1.0
pub fn asi_stage1_v1457_weights_sum_one() -> bool {
    v1457_stage_weight_sum_is_one()
}

/// Stage 1 verify: V1447 audit pair count = 35 (7 × 5)
pub fn asi_stage1_v1447_pair_count() -> bool {
    V1447_AUDIT_PAIRS.len() == V1447_N_PAIRS && V1447_N_PAIRS == 35
}

/// Stage 1 verify: V1077 dim count = 17
pub fn asi_stage1_v1077_dim_count() -> bool {
    V1077_N_DIMENSIONS == 17
}

/// Stage 1 verify: V1400 12 能力 + 6 限制
pub fn asi_stage1_v1400_capabilities_limits() -> bool {
    V1400_CAPABILITIES.len() == V1400_N_CAPABILITIES
        && V1400_LIMITS.len() == V1400_N_LIMITS
}

/// Stage 1 verify: V1467 6 endpoints
pub fn asi_stage1_v1467_endpoint_count() -> bool {
    V1467Endpoint::ALL.len() == V1467_N_ENDPOINTS
}

/// Stage 1 verify: V1470 cross-checks per run = 12
pub fn asi_stage1_v1470_cross_checks() -> bool {
    V1470_N_CROSS_CHECKS_PER_RUN == 12 && V1470_N_CROSS_CHECKS_TOTAL == 36
}

/// Stage 1 综合 verify — 7 invariants 全 pass
pub fn asi_stage1_all_invariants_ok() -> bool {
    asi_stage1_ceiling_chain_locked()
        && asi_stage1_v1457_weights_sum_one()
        && asi_stage1_v1447_pair_count()
        && asi_stage1_v1077_dim_count()
        && asi_stage1_v1400_capabilities_limits()
        && asi_stage1_v1467_endpoint_count()
        && asi_stage1_v1470_cross_checks()
}

// =============================================================================
// cfg-gated Python 桥接 (per decision-33 §2.3 C2 0 装 PASS 严守)
// =============================================================================

/// Stage 1 桥接: 真调 V1077 V0.4 全测 (仅 python-ext 启用时)
#[cfg(feature = "python-ext")]
pub fn bridge_v1077_full_measure() -> Result<String, BridgeError> {
    crate::bridge::call_python_function(V1077_MODULE, "run_full_measure", &[])
}

#[cfg(not(feature = "python-ext"))]
pub fn bridge_v1077_full_measure() -> Result<String, BridgeError> {
    Err(BridgeError::ModuleNotFound(format!(
        "{V1077_MODULE}: pyo3 disabled (rebuild with --features python-ext to call V1077)"
    )))
}

/// Stage 1 桥接: 真调 V1458 ceiling chain audit
#[cfg(feature = "python-ext")]
pub fn bridge_v1458_ceiling_audit() -> Result<String, BridgeError> {
    crate::bridge::call_python_function(V1458_MODULE, "run_all", &[])
}

#[cfg(not(feature = "python-ext"))]
pub fn bridge_v1458_ceiling_audit() -> Result<String, BridgeError> {
    Err(BridgeError::ModuleNotFound(format!(
        "{V1458_MODULE}: pyo3 disabled (rebuild with --features python-ext to call V1458)"
    )))
}

/// Stage 1 桥接: 真调 V1457 6-deployment runbook
#[cfg(feature = "python-ext")]
pub fn bridge_v1457_deploy_all() -> Result<String, BridgeError> {
    crate::bridge::call_python_function(V1457_MODULE, "deploy_all", &[])
}

#[cfg(not(feature = "python-ext"))]
pub fn bridge_v1457_deploy_all() -> Result<String, BridgeError> {
    Err(BridgeError::ModuleNotFound(format!(
        "{V1457_MODULE}: pyo3 disabled (rebuild with --features python-ext to call V1457)"
    )))
}

// =============================================================================
// 单元测试 (cfg-无关, 14 单元测试)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage1_version_is_r128() {
        assert_eq!(asi_stage1_version(), "0.1.0-R128-Stage1");
    }

    #[test]
    fn stage1_module_count_is_7() {
        assert_eq!(asi_stage1_module_count(), 7);
        assert_eq!(ASI_STAGE1_MODULES.len(), 7);
        assert_eq!(ASI_STAGE1_INFOS.len(), 7);
    }

    #[test]
    fn known_modules_recognized() {
        for m in &ASI_STAGE1_MODULES {
            assert!(is_known_asi_stage1_module(m), "known module: {m}");
        }
        assert!(!is_known_asi_stage1_module("apeireth.v9999_unknown"));
    }

    #[test]
    fn lookup_by_name_and_version() {
        let by_name = asi_lookup_module(V1077_MODULE).expect("V1077 lookup");
        assert_eq!(by_name.version_tag, "V1077");
        assert_eq!(by_name.category, AsiCategory::Measurement);

        let by_version = asi_lookup_by_version("V1458").expect("V1458 lookup by version");
        assert_eq!(by_version.name, V1458_MODULE);
        assert_eq!(by_version.category, AsiCategory::CeilingChain);
        assert!(by_version.is_ceiling_critical);
    }

    #[test]
    fn list_by_category_filters_correctly() {
        let ceiling = list_asi_stage1_modules_by_category(AsiCategory::CeilingChain);
        assert_eq!(ceiling.len(), 1);
        assert_eq!(ceiling[0].version_tag, "V1458");

        let audit = list_asi_stage1_modules_by_category(AsiCategory::CrossModularAudit);
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].version_tag, "V1447");
    }

    #[test]
    fn ceiling_critical_only_v1458() {
        let cc = list_ceiling_critical_modules();
        assert_eq!(cc.len(), 1);
        assert_eq!(cc[0].version_tag, "V1458");
    }

    // --- V1400 Self framework 镜像 ---

    #[test]
    fn v1400_capabilities_count_12() {
        assert_eq!(V1400_CAPABILITIES.len(), V1400_N_CAPABILITIES);
        assert_eq!(V1400_N_CAPABILITIES, 12);
        // ID 唯一
        let mut ids: Vec<&str> = V1400_CAPABILITIES.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 12);
    }

    #[test]
    fn v1400_limits_count_6() {
        assert_eq!(V1400_LIMITS.len(), V1400_N_LIMITS);
        assert_eq!(V1400_N_LIMITS, 6);
    }

    // --- V1447 audit pair 矩阵 ---

    #[test]
    fn v1447_pair_count_35() {
        assert_eq!(V1447_AUDIT_PAIRS.len(), V1447_N_PAIRS);
        assert_eq!(V1447_N_PAIRS, 35);
        assert_eq!(V1447_N_COMBINED_PROBES, 175);
        assert_eq!(V1447_N_CROSS_PAIR_LINKS, 1190);
    }

    #[test]
    fn v1447_problems_and_positions_complete() {
        assert_eq!(PhilosophicalProblem::ALL.len(), 7);
        assert_eq!(V2Position::ALL.len(), 5);
        assert_eq!(ClosureKind::ALL.len(), 5);
    }

    // --- V1457 operational stages ---

    #[test]
    fn v1457_stages_count_5() {
        assert_eq!(OperationalStage::ALL.len(), V1457_N_STAGES);
        assert_eq!(V1457_N_STAGES, 5);
        assert_eq!(V1457_N_PROBES, 30);
    }

    #[test]
    fn v1457_stage_weights_sum_one() {
        assert!(asi_stage1_v1457_weights_sum_one());
        let sum: f64 = OperationalStage::ALL.iter().map(|s| s.weight()).sum();
        assert!((sum - 1.0).abs() < 0.0001, "stage weights sum = {sum}");
    }

    // --- V1458 ceiling chain LOCKED ---

    #[test]
    fn v1458_ceiling_chain_locked() {
        let lock = CeilingChainLock::LOCKED;
        assert_eq!(lock.anchor_value, 0.9105);
        assert_eq!(lock.north_star_ceiling, 0.98);
        assert_eq!(lock.absolute_ceiling, 1.0);
        assert!((lock.gap_to_north_star - 0.0695).abs() < 1e-9);
        assert!((lock.gap_to_ceiling - 0.0895).abs() < 1e-9);
        assert!(lock.verify_internal_consistency());
        assert!(lock.no_inflation());
        assert!(lock.no_lowered_north_star());
        assert!(lock.no_lowered_ceiling());
    }

    #[test]
    fn v1458_inflation_detected() {
        let inflated = CeilingChainLock {
            anchor_value: 0.95, // 0.95 > 0.9105
            ..CeilingChainLock::LOCKED
        };
        assert!(!inflated.no_inflation());
        assert!(!inflated.verify_internal_consistency());
    }

    // --- V1467 HTTP endpoints ---

    #[test]
    fn v1467_endpoints_count_6() {
        assert_eq!(V1467Endpoint::ALL.len(), V1467_N_ENDPOINTS);
        assert_eq!(V1467_N_ENDPOINTS, 6);
        for ep in V1467Endpoint::ALL.iter() {
            assert!(ep.path().starts_with('/'), "endpoint path: {}", ep.path());
        }
    }

    // --- V1470 cross-checks ---

    #[test]
    fn v1470_cross_checks_12_per_run() {
        assert_eq!(V1470_N_CROSS_CHECKS_PER_RUN, 12);
        assert_eq!(V1470_N_CROSS_CHECKS_TOTAL, 36);
        assert_eq!(V1470_MIN_BATCH_N, 2);
        assert_eq!(V1470_DEFAULT_BATCH_N, 3);
    }

    // --- 综合 health check ---

    #[test]
    fn asi_stage1_all_invariants_test() {
        assert!(asi_stage1_all_invariants_ok());
    }

    #[test]
    fn asi_stage1_health_display_contains_all_modules() {
        let h = asi_stage1_health();
        assert_eq!(h.module_count, 7);
        assert_eq!(h.ceiling_critical_count, 1);
        assert_eq!(h.known_modules.len(), 7);
        let s = format!("{h}");
        // module names contain lowercase v# (e.g. "v1077"), version_tag is uppercase
        assert!(s.contains("v1077_asi_v04_full_measurement"));
        assert!(s.contains("v1458_asi_north_star_ceiling_chain_audit"));
        assert!(s.contains("v1470_asi_v1469_batch_harness_cross_client_equivalence"));
        assert!(s.contains("ceiling-critical"));
    }

    // --- 0 装 PASS 严守: 默认 build 桥接函数返回 ModuleNotFound ---

    #[test]
    fn bridge_default_build_module_not_found() {
        if !cfg!(feature = "python-ext") {
            let r1 = bridge_v1077_full_measure();
            assert!(r1.is_err());
            assert_eq!(r1.unwrap_err().suggested_action(), crate::error::SuggestedAction::Degrade);

            let r2 = bridge_v1458_ceiling_audit();
            assert!(r2.is_err());
            assert_eq!(r2.unwrap_err().suggested_action(), crate::error::SuggestedAction::Degrade);

            let r3 = bridge_v1457_deploy_all();
            assert!(r3.is_err());
            assert_eq!(r3.unwrap_err().suggested_action(), crate::error::SuggestedAction::Degrade);
        }
    }
}
