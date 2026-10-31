//! 开关生效链审计续: 「开关开 → 装配真生效 → 自报如实」的装配级钉子。
//!
//! 逐开关走完启用链的后三环 (装配消费 → 运行时生效 → 自报读值):
//! - 点名三链 (council / organs / reflexion): 开关开 = 模块真注册 + 自述
//!   名册照实 true; 开关关 = 不注册 + 照实 false。
//! - 断点修复断言: 挂在别的槽上的开关 (记忆固化/检索深度自适应/图社区分诊/
//!   自学习) 按**实际注册条件**取值, 上游槽缺席时自报照实 false; 外部工具桥
//!   (mcp) 行按"桥已装配且有可用服务器"取值 —— 已启用但无服务器配置 =
//!   无外部工具。
//!
//! 自报读值统一经 `tool.self_status` 真调用取证 (与生产同一条投影路径)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::clock::SystemClock;
use apeireth_core::kernel::Clock;
use apeireth_core::stored_doc;
use apeireth_memory::partner::InMemoryPartnerStore;
use apeireth_memory::reflexion::FileReflexionStore;
use apeireth_memory::SqliteMemoryStore;
use apeireth_orchestration::self_tuning::TuningValues;
use apeireth_orchestration::Council;
use apeireth_plugin::experience::{
    AssociationEdge, AssociationStore, GraphFact, GraphLink, KnowledgeGraphStore, WikiEntry,
    WikiEntryStore,
};
use apeireth_plugin::CapabilityResult;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    JudgeConfig, ProductionBackends, ProductionModules, ProductionModulesConfig, SelfTuningWire,
};
use apeireth_tools_canonical::mcp_bridge::{
    doc_compat, server_list_path, McpServerConfig, McpServerSpec,
};

fn clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

/// 与生产同款的最小装配: 记忆槽/偏好槽默认关 (逐用例显式打开)。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn ids_of(modules: &ProductionModules) -> Vec<String> {
    modules.ids()
}

/// 真调 `tool.self_status` 取能力名册 (与生产同一条投影路径)。
async fn roster_of(modules: &ProductionModules) -> serde_json::Value {
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status must be registered");
    let call = ToolCall {
        id: "call_roster".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: serde_json::Value =
        serde_json::from_str(&result.render()).expect("structured self-report json");
    report["capabilities"].clone()
}

fn memory_backends(dir: &std::path::Path) -> ProductionBackends {
    let store = Arc::new(SqliteMemoryStore::open(dir.join("cognitive.sqlite3")).expect("store"));
    ProductionBackends {
        memory: Some(store.clone()),
        memory_governance: Some(store),
        ..ProductionBackends::default()
    }
}

/// 点名链 ①: council 开关开 = 模块真注册 + 自报照实 true; 关 = 双双缺席。
#[tokio::test]
async fn council_switch_registers_the_module_and_reports_itself_honestly() {
    let mut config = base_config();
    config.council = true;
    let backends = ProductionBackends {
        council: Some(Arc::new(Council::default_allow())),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
    assert!(
        ids_of(&modules).iter().any(|id| id == "cognitive.council"),
        "开关开必须注册 council 模块: {:?}",
        ids_of(&modules)
    );
    let roster = roster_of(&modules).await;
    assert_eq!(roster["council"], true);

    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(!ids_of(&modules).iter().any(|id| id == "cognitive.council"));
    let roster = roster_of(&modules).await;
    assert_eq!(roster["council"], false);
}

/// 点名链 ②: 器官链开关开 = AfterTurn 模块真注册 + 自报照实 true。
#[tokio::test]
async fn organ_switch_registers_the_after_turn_module_and_reports_itself_honestly() {
    let mut config = base_config();
    config.organs = true;
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(
        ids_of(&modules).iter().any(|id| id == "cognitive.organs"),
        "开关开必须注册器官链模块: {:?}",
        ids_of(&modules)
    );
    let roster = roster_of(&modules).await;
    assert_eq!(roster["organs"], true);

    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(!ids_of(&modules).iter().any(|id| id == "cognitive.organs"));
    let roster = roster_of(&modules).await;
    assert_eq!(roster["organs"], false);
}

/// 点名链 ③: 反思沉淀开关开 = 模块带教训存储真注册 + 自报照实 true。
#[tokio::test]
async fn reflexion_switch_registers_with_its_store_and_reports_itself_honestly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = base_config();
    config.reflexion = true;
    let backends = ProductionBackends {
        reflexion_store: Some(Arc::new(FileReflexionStore::new(
            dir.path().join("reflexion"),
        ))),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
    assert!(
        ids_of(&modules)
            .iter()
            .any(|id| id == "cognitive.reflexion"),
        "开关开必须注册反思模块: {:?}",
        ids_of(&modules)
    );
    let roster = roster_of(&modules).await;
    assert_eq!(roster["reflexion"], true);

    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(!ids_of(&modules)
        .iter()
        .any(|id| id == "cognitive.reflexion"));
    let roster = roster_of(&modules).await;
    assert_eq!(roster["reflexion"], false);
}

/// 其余同型开关 (评审/伙伴羁绊/吸收洞察/教育工具) 同一条链: 开关开 =
/// 注册/装配真生效 + 自报照实 true, 关 = 双双缺席。
#[tokio::test]
async fn remaining_switches_register_and_report_on_the_same_chain() {
    // 评审: AfterModelResponse 评审模块。
    let mut config = base_config();
    config.judge = JudgeConfig {
        enabled: true,
        ..JudgeConfig::default()
    };
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(ids_of(&modules).iter().any(|id| id == "cognitive.judge"));
    assert_eq!(roster_of(&modules).await["judge"], true);

    // 伙伴羁绊: 模块带羁绊存储。
    let mut config = base_config();
    config.partner_bond = true;
    let backends = ProductionBackends {
        partner_store: Some(Arc::new(InMemoryPartnerStore::new())),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
    assert!(ids_of(&modules)
        .iter()
        .any(|id| id == "cognitive.partner_bond"));
    assert_eq!(roster_of(&modules).await["partner_bond"], true);

    // 吸收洞察: AfterTurn 认知模块。
    let mut config = base_config();
    config.absorption_insight = true;
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(ids_of(&modules)
        .iter()
        .any(|id| id == "cognitive.absorption_insight"));
    assert_eq!(roster_of(&modules).await["absorption_insight"], true);

    // 教育工具: 工具能力注册 + 自报照实。
    let mut config = base_config();
    config.education = true;
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(modules
        .capabilities()
        .iter()
        .any(|capability| capability.id().as_str() == "tool.education"));
    assert_eq!(roster_of(&modules).await["education"], true);

    // 全关对照: 不注册、自报 false。
    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    let roster = roster_of(&modules).await;
    assert_eq!(roster["judge"], false);
    assert_eq!(roster["partner_bond"], false);
    assert_eq!(roster["absorption_insight"], false);
    assert_eq!(roster["education"], false);
}

/// 断点修复断言: 记忆固化挂在记忆写入模块 —— 写入模块缺席时开关静默失效,
/// 自报按实际注册条件取 false (旧自报照抄配置文本报 true)。
#[tokio::test]
async fn consolidation_reports_the_memory_writeback_registration_condition() {
    let dir = tempfile::tempdir().expect("temp dir");

    // 写入模块在场: 开关开 = 真生效, 自报 true。
    let mut config = base_config();
    config.memory_writeback = true;
    config.consolidation = true;
    let modules =
        ProductionModules::build(config, memory_backends(dir.path()), clock()).expect("build");
    assert!(ids_of(&modules)
        .iter()
        .any(|id| id == "cognitive.memory_writeback"));
    assert_eq!(roster_of(&modules).await["consolidation"], true);

    // 写入模块缺席: 配置开了也没处生效, 自报照实 false。
    let mut config = base_config();
    config.consolidation = true;
    let modules =
        ProductionModules::build(config, ProductionBackends::default(), clock()).expect("build");
    assert_eq!(roster_of(&modules).await["consolidation"], false);
}

/// 断点修复断言: 检索深度自适应 / 图社区分诊 / 自学习挂在记忆召回模块,
/// 图社区分诊另需图谱槽 —— 槽缺席时自报照实 false, 槽在场且开关开 = true。
#[tokio::test]
async fn recall_dependent_switches_report_the_memory_recall_registration_condition() {
    let dir = tempfile::tempdir().expect("temp dir");

    // 记忆召回缺席 (接线都在但无处生效): 照实 false。
    let wire = Arc::new(SelfTuningWire::new(
        dir.path().join("tuning-log.jsonl"),
        TuningValues::baseline(),
    ));
    let mut config = base_config();
    config.morphology_recall = true;
    config.community_triage = true;
    let mut backends = memory_backends(dir.path());
    backends.self_tuning = Some(wire);
    let modules = ProductionModules::build(config, backends, clock()).expect("build");
    let roster = roster_of(&modules).await;
    assert_eq!(roster["morphology_recall"], false);
    assert_eq!(roster["community_triage"], false);
    assert_eq!(roster["self_tuning"], false);

    // 记忆召回在场 + 图谱槽在场 (经验三件套同注入): 真生效, 照实 true。
    let wire = Arc::new(SelfTuningWire::new(
        dir.path().join("tuning-log.jsonl"),
        TuningValues::baseline(),
    ));
    let mut config = base_config();
    config.memory_recall = true;
    config.morphology_recall = true;
    config.community_triage = true;
    let mut backends = memory_backends(dir.path());
    backends.self_tuning = Some(wire);
    backends.wiki = Some(Arc::new(FakeWiki));
    backends.graph = Some(Arc::new(FakeGraph));
    backends.associations = Some(Arc::new(FakeAssociations));
    let modules = ProductionModules::build(config, backends, clock()).expect("build");
    let roster = roster_of(&modules).await;
    assert_eq!(roster["morphology_recall"], true);
    assert_eq!(roster["community_triage"], true);
    assert_eq!(roster["self_tuning"], true);
}

/// 断点修复断言: 图社区分诊需要图谱槽 —— 召回在场但图谱槽缺席时照实 false。
#[tokio::test]
async fn community_triage_reports_false_without_the_graph_slot() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = base_config();
    config.memory_recall = true;
    config.community_triage = true;
    let modules =
        ProductionModules::build(config, memory_backends(dir.path()), clock()).expect("build");
    assert_eq!(roster_of(&modules).await["community_triage"], false);
}

/// MCP 自报语义 (如实): 桥已装配**且**有可用服务器配置才算外部工具面生效 ——
/// 已启用但无服务器配置 = 无外部工具 (照实 false)。
#[tokio::test]
async fn mcp_row_reports_external_tools_only_when_the_bridge_serves() {
    let dir = tempfile::tempdir().expect("temp dir");

    // 开关开 + 有可用服务器: 桥装配 + 自报 true。
    let body = McpServerConfig::new(vec![McpServerSpec::stdio("demo", "demo-server")]);
    stored_doc::save_single(
        &server_list_path(dir.path()),
        &doc_compat(),
        body,
        stored_doc::DEFAULT_DOC_MODE,
    )
    .expect("save server list");
    let mut config = base_config();
    config.mcp = true;
    config.mcp_data_dir = Some(PathBuf::from(dir.path()));
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(modules.mcp_definition().is_some(), "桥必须装配");
    assert_eq!(roster_of(&modules).await["mcp"], true);

    // 开关开 + 无服务器配置: 桥照常装配 (面在), 但无外部工具 —— 照实 false。
    let empty = tempfile::tempdir().expect("temp dir");
    let mut config = base_config();
    config.mcp = true;
    config.mcp_data_dir = Some(PathBuf::from(empty.path()));
    let modules = ProductionModules::build(config, ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(modules.mcp_definition().is_some());
    assert_eq!(
        roster_of(&modules).await["mcp"],
        false,
        "已启用但无服务器配置 = 无外部工具"
    );

    // 开关关: 不装配、照实 false。
    let modules = ProductionModules::build(base_config(), ProductionBackends::default(), clock())
        .expect("assembly builds");
    assert!(modules.mcp_definition().is_none());
    assert_eq!(roster_of(&modules).await["mcp"], false);
}

// ---- 经验三件套的最小桩 (图谱槽在场的正例需要三件同注入) ----

struct FakeWiki;

impl WikiEntryStore for FakeWiki {
    fn put_wiki(&self, _entry: &WikiEntry) -> CapabilityResult<()> {
        Ok(())
    }
    fn list_wiki(
        &self,
        _session_id: &str,
        _topic: &str,
        _limit: u32,
    ) -> CapabilityResult<Vec<WikiEntry>> {
        Ok(Vec::new())
    }
    fn wiki_for_episode(&self, _episode_id: &str) -> CapabilityResult<Vec<WikiEntry>> {
        Ok(Vec::new())
    }
}

struct FakeGraph;

impl KnowledgeGraphStore for FakeGraph {
    fn put_fact(&self, _fact: &GraphFact) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn put_link(&self, _link: &GraphLink) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    fn facts_from(&self, _subject_id: &str, _limit: u32) -> CapabilityResult<Vec<GraphFact>> {
        Ok(Vec::new())
    }
    fn links_from(&self, _from_id: &str, _limit: u32) -> CapabilityResult<Vec<GraphLink>> {
        Ok(Vec::new())
    }
    fn forget_subject(&self, _subject_id: &str) -> CapabilityResult<()> {
        Ok(())
    }
}

struct FakeAssociations;

impl AssociationStore for FakeAssociations {
    fn record_cooccurrence(
        &self,
        _from: &str,
        _to: &str,
        _episode_id: &str,
    ) -> CapabilityResult<()> {
        Ok(())
    }
    fn top_associations(
        &self,
        _entity: &str,
        _limit: u32,
    ) -> CapabilityResult<Vec<AssociationEdge>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------
// 口径随行: capabilities-matrix §0.2 开关行与能力名册实况一致
// ---------------------------------------------------------------------

/// §0.2 默认能力表的四开关行 (care / task_board / voice_speak / backup_export)
/// 与名册实况同真值: 文档行写明 env 旋钮名与名册行名, 名册行按**实际注册
/// 条件**取值 (配置槽注入才算生效, 注册即闸) —— 口径行与实况互钉。
#[test]
fn matrix_switch_rows_match_the_live_roster() {
    // 文档面: §0.2 有四开关的 env 旋钮名与名册行名。
    const MATRIX: &str = include_str!("../../../../docs/03-reference/capabilities-matrix.md");
    let start = MATRIX.find("### 0.2").expect("§0.2 present");
    let end = MATRIX[start..]
        .find("\n---")
        .map(|offset| start + offset)
        .unwrap_or(MATRIX.len());
    let section = &MATRIX[start..end];
    for needle in [
        "APEIRETH_ENABLE_CARE",
        "APEIRETH_ENABLE_TASK_BOARD",
        "APEIRETH_ENABLE_VOICE_SPEAK",
        "APEIRETH_ENABLE_BACKUP_EXPORT",
        "care_schedule",
        "task_board",
        "voice_speak",
        "backup_export",
    ] {
        assert!(section.contains(needle), "§0.2 缺开关行口径: {needle}");
    }

    // 实况面: 名册有这四行; 配置槽缺席照实 false (不复述"配置说开")。
    let config = ProductionModulesConfig::default();
    let roster =
        apeireth_runtime_assembly::roster_from_config(&config, true, false, false, false, &[]);
    let enabled = |roster: &[apeireth_tools_canonical::CapabilitySwitch], name: &str| {
        roster
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("名册缺行 {name}"))
            .enabled
    };
    assert!(
        !enabled(&roster, "care_schedule"),
        "数据目录槽缺席 = 照实 false"
    );
    assert!(
        !enabled(&roster, "task_board"),
        "任务队列槽缺席 = 照实 false"
    );
    assert!(
        !enabled(&roster, "voice_speak"),
        "发声配置槽缺席 = 照实 false"
    );
    assert!(
        !enabled(&roster, "backup_export"),
        "备份数据源槽缺席 = 照实 false"
    );

    // 槽注入 = 注册条件达成, 名册照实 true (注册即闸的正向面)。
    let mut wired = ProductionModulesConfig::default();
    wired.care_schedule = Some(PathBuf::from("."));
    let roster =
        apeireth_runtime_assembly::roster_from_config(&wired, true, false, false, false, &[]);
    assert!(enabled(&roster, "care_schedule"), "槽注入必须报真值 true");
}

/// §0.2 生产接线收口批四缝 (persona_consolidation / consolidation_hash_gate /
/// voice_emotion_tune / exposure_audit) 与名册实况同真值: 文档行写明 env 旋钮名
/// 与名册行名, 名册行按**实际注册条件**取值 —— 口径行与实况互钉。
#[test]
fn matrix_production_wiring_rows_match_the_live_roster() {
    // 文档面: §0.2 有四缝的 env 旋钮名与名册行名。
    const MATRIX: &str = include_str!("../../../../docs/03-reference/capabilities-matrix.md");
    let start = MATRIX.find("### 0.2").expect("§0.2 present");
    let end = MATRIX[start..]
        .find("\n---")
        .map(|offset| start + offset)
        .unwrap_or(MATRIX.len());
    let section = &MATRIX[start..end];
    for needle in [
        "APEIRETH_ENABLE_PERSONA_CONSOLIDATION",
        "APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE",
        "APEIRETH_ENABLE_EXPOSURE_AUDIT",
        "persona_consolidation",
        "consolidation_hash_gate",
        "voice_emotion_tune",
        "exposure_audit",
    ] {
        assert!(section.contains(needle), "§0.2 缺接线行口径: {needle}");
    }

    let enabled = |roster: &[apeireth_tools_canonical::CapabilitySwitch], name: &str| {
        roster
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("名册缺行 {name}"))
            .enabled
    };

    // 实况面: 默认态四行都照实 false (显式未接线, 不复述"配置说开")。
    let config = ProductionModulesConfig::default();
    let roster =
        apeireth_runtime_assembly::roster_from_config(&config, true, false, false, false, &[]);
    assert!(
        !enabled(&roster, "persona_consolidation"),
        "默认关 = 显式未接线"
    );
    assert!(
        !enabled(&roster, "consolidation_hash_gate"),
        "默认关 = 显式未接线"
    );
    assert!(
        !enabled(&roster, "voice_emotion_tune"),
        "发声配置没带情绪读取面 = 显式未接线"
    );
    assert!(
        !enabled(&roster, "exposure_audit"),
        "体检配置槽缺席 = 照实 false"
    );

    // 开关 + 上游槽齐备 = 真生效 true (注册即闸的正向面)。
    let mut wired = ProductionModulesConfig::default();
    wired.consolidation = true;
    wired.persona_consolidation = true;
    wired.consolidation_hash_gate = true;
    wired.exposure_audit = Some(apeireth_tools_canonical::ExposureAuditConfig::new("."));
    let voice = apeireth_tools_canonical::VoiceSpeakConfig::new(
        PathBuf::from("."),
        apeireth_tools_canonical::voice::fixed_clock(12 * 60),
    )
    .with_emotion_reader(Arc::new(|| None));
    wired.voice_speak = Some(voice);
    let roster =
        apeireth_runtime_assembly::roster_from_config(&wired, true, false, false, false, &[]);
    assert!(
        enabled(&roster, "persona_consolidation"),
        "开关 + 整编触发点齐备必须报真值 true"
    );
    assert!(
        enabled(&roster, "consolidation_hash_gate"),
        "开关 + 整编触发点齐备必须报真值 true"
    );
    assert!(
        enabled(&roster, "voice_emotion_tune"),
        "发声配置带情绪读取面必须报真值 true"
    );
    assert!(
        enabled(&roster, "exposure_audit"),
        "体检配置槽注入必须报真值 true"
    );

    // 整编触发点缺席 = 随整编跑的两行照实 false (上游槽口径不许分叉)。
    let mut no_trigger = wired.clone();
    no_trigger.consolidation = false;
    let roster =
        apeireth_runtime_assembly::roster_from_config(&no_trigger, true, false, false, false, &[]);
    assert!(
        !enabled(&roster, "persona_consolidation"),
        "整编关 = 提取步显式不跑"
    );
    assert!(
        !enabled(&roster, "consolidation_hash_gate"),
        "整编关 = 散列门显式不跑"
    );
}

/// §0.2 V4-c 检索调权行 (`retrieval_tuning`) 与名册实况同真值: 文档行写明 env
/// 旋钮名与名册行名, 名册行按**接线事实 AND 上游槽 (记忆召回)**取值 ——
/// 口径行与实况互钉。
#[test]
fn matrix_retrieval_tuning_row_matches_the_live_roster() {
    // 文档面: §0.2 有检索调权的 env 旋钮名与名册行名。
    const MATRIX: &str = include_str!("../../../../docs/03-reference/capabilities-matrix.md");
    let start = MATRIX.find("### 0.2").expect("§0.2 present");
    let end = MATRIX[start..]
        .find("\n---")
        .map(|offset| start + offset)
        .unwrap_or(MATRIX.len());
    let section = &MATRIX[start..end];
    for needle in ["APEIRETH_ENABLE_RETRIEVAL_TUNING", "retrieval_tuning"] {
        assert!(section.contains(needle), "§0.2 缺检索调权行口径: {needle}");
    }

    let enabled = |roster: &[apeireth_tools_canonical::CapabilitySwitch], name: &str| {
        roster
            .iter()
            .find(|row| row.name == name)
            .unwrap_or_else(|| panic!("名册缺行 {name}"))
            .enabled
    };

    // 实况面: 接线事实缺席 (开关默认关) = 显式未接线, 照实 false。
    let config = ProductionModulesConfig::default();
    let roster =
        apeireth_runtime_assembly::roster_from_config(&config, true, false, false, false, &[]);
    assert!(!enabled(&roster, "retrieval_tuning"), "默认关 = 显式未接线");

    // 接线事实 + 上游槽 (记忆召回) 齐备 = 真生效 true (注册即闸的正向面)。
    let mut wired = ProductionModulesConfig::default();
    wired.memory_recall = true;
    let roster =
        apeireth_runtime_assembly::roster_from_config(&wired, true, false, true, false, &[]);
    assert!(
        enabled(&roster, "retrieval_tuning"),
        "接线 + 召回槽齐备必须报真值 true"
    );

    // 上游槽缺席 = 静默失效照实 false (与自学习行同一口径, 不许分叉)。
    let mut no_recall = wired.clone();
    no_recall.memory_recall = false;
    let roster =
        apeireth_runtime_assembly::roster_from_config(&no_recall, true, false, true, false, &[]);
    assert!(
        !enabled(&roster, "retrieval_tuning"),
        "记忆召回缺席 = 无处生效照实 false"
    );
}
