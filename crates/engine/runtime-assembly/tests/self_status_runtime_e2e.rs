//! 结构化自述面 (`tool.self_status`) 的装配级端到端证明 (自省通道)。
//!
//! 装配走真 [`ProductionModules`] 生产组装根; 自述内容对账到真实生效值与
//! 真存储计数: 身份 / 能力名册 / 记忆账本 / 调参 / 预算 / 工作区逐项核对,
//! 缺失来源显式 null + 原因, 凭据只回存在性布尔 (绝不回显凭据本体)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_memory::reflexion::{FailureKind, FileReflexionStore, ReflexionStore, RuleCritic};
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_orchestration::self_tuning::TuningValues;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_runtime_assembly::{
    ProductionBackends, ProductionModules, ProductionModulesConfig, SelfTuningWire,
};
use apeireth_tools_canonical::{
    CapabilitySwitch, MemoryLedgerStats, SelfStatusTool, StatusProbe, ToolOutcome,
    DATA_PROBE_NOT_WIRED, PRESET_BALANCED, PRODUCT_NAME, RUNTIME_ROLE_GATEWAY_SIDECAR,
};
use serde_json::Value;

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

/// 与生产同款的最小装配: 记忆槽关闭 (无后端注入), 工具槽默认开。
fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn build(
    config: ProductionModulesConfig,
    backends: ProductionBackends,
) -> (ProductionModules, Arc<dyn ToolCapability>) {
    let modules = ProductionModules::build(config, backends, clock()).expect("assembly builds");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status must be registered");
    (modules, tool)
}

async fn invoke(tool: &Arc<dyn ToolCapability>) -> Value {
    let call = ToolCall {
        id: "call_self_status".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-report json")
}

fn episode(id: &str, session: &str, content: &str) -> apeireth_core::Episode {
    apeireth_core::Episode {
        id: id.to_string(),
        timestamp: 1_700_000_000,
        role: "user".to_string(),
        content: content.to_string(),
        session_id: session.to_string(),
    }
}

#[tokio::test]
async fn self_status_registers_alongside_the_builtin_tool_roster() {
    let with_workspace = ProductionBackends {
        workspace_root: Some(PathBuf::from(".")),
        ..ProductionBackends::default()
    };
    let (modules, _) = build(base_config(), with_workspace);
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    for expected in [
        "tool.filesystem",
        "tool.search",
        "tool.repo",
        "tool.self_status",
    ] {
        assert!(
            ids.iter().any(|id| id == expected),
            "missing {expected}: {ids:?}"
        );
    }

    // 显式关 = 不注册 (装配选择权仍在组合根)。
    let mut off = base_config();
    off.self_status = false;
    let modules =
        ProductionModules::build(off, ProductionBackends::default(), clock()).expect("build");
    assert!(modules
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != "tool.self_status"));
}

#[tokio::test]
async fn identity_reports_product_workspace_version_and_runtime_role() {
    let (_, tool) = build(base_config(), ProductionBackends::default());
    let report = invoke(&tool).await;
    let identity = &report["identity"];
    assert_eq!(identity["product_name"], PRODUCT_NAME);
    assert_eq!(identity["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(identity["runtime_role"], RUNTIME_ROLE_GATEWAY_SIDECAR);
    // 声明面: 无参数只读 (参数 schema 声明零个属性)。
    let declaration = tool.declaration();
    assert_eq!(declaration.name, "self_status");
    let properties = declaration
        .parameters
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(properties.is_empty(), "{:?}", declaration.parameters);
}

#[tokio::test]
async fn capability_roster_matches_the_effective_assembly_switches() {
    let mut config = base_config();
    config.organs = true;
    config.education = true;
    config.filesystem = false;
    config.search = false;
    config.consolidation = true;

    let wire = Arc::new(SelfTuningWire::new(
        std::env::temp_dir().join(format!(
            "apeireth-self-status-tuning-{}.jsonl",
            std::process::id()
        )),
        TuningValues::baseline(),
    ));
    let backends = ProductionBackends {
        self_tuning: Some(wire),
        self_status_extras: vec![CapabilitySwitch::new("local_read_tools", true)],
        ..ProductionBackends::default()
    };

    let (_, tool) = build(config, backends);
    let report = invoke(&tool).await;
    let roster = report["capabilities"].as_object().expect("roster object");
    assert_eq!(roster["organs"], true);
    assert_eq!(roster["education"], true);
    // 按实际注册条件取值 (开关生效链审计续): consolidation 挂在记忆写入模块
    // (本用例写入模块未注册) —— 配置开了但注册没成, 名册照实 false。
    // 旧行为: 照抄 config 报 true (配置文本, 非生效值)。
    assert_eq!(roster["consolidation"], false);
    assert_eq!(roster["filesystem"], false);
    assert_eq!(roster["search"], false);
    assert_eq!(roster["shell"], false);
    assert_eq!(roster["fetch"], false);
    // 同口径: 自学习接线挂在记忆召回模块 (本用例召回模块未注册) ——
    // 接线存在但无处生效, 名册照实 false。旧行为: 报接线存在 = true。
    assert_eq!(roster["self_tuning"], false);
    assert_eq!(roster["typed_recall"], false);
    assert_eq!(roster["local_read_tools"], true);
    // 名册全表: 每个开关一行, 值为布尔。
    assert!(roster.len() >= 25, "roster too small: {roster:?}");
    for (name, value) in roster {
        assert!(value.is_boolean(), "{name} must be a boolean: {value}");
    }
}

#[tokio::test]
async fn memory_ledger_counts_match_the_real_stores_without_echoing_content() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cognitive_db = dir.path().join("cognitive.sqlite3");
    let lessons_root = dir.path().join("reflexion");

    // 真存储造数: 3 条 episode (2 个会话), 1 条保护; 1 条教训。
    {
        let store = SqliteMemoryStore::open(&cognitive_db).expect("store");
        store
            .put_episode(&episode("e1", "s1", "内容原文-甲"))
            .expect("episode");
        store
            .put_episode(&episode("e2", "s1", "内容原文-乙"))
            .expect("episode");
        store
            .put_episode(&episode("e3", "s2", "内容原文-丙"))
            .expect("episode");
        store.protect_episode("e1", 0).expect("protect");
    }
    {
        let reflexion = FileReflexionStore::new(&lessons_root);
        reflexion
            .record_failure(FailureKind::DecisionRejected, "task", "失败样本", 1)
            .expect("record failure");
        assert_eq!(
            reflexion
                .process_unreflected(&RuleCritic, 2)
                .expect("reflect"),
            1
        );
    }

    let ledger_probe: StatusProbe<MemoryLedgerStats> = {
        let cognitive_db = cognitive_db.clone();
        let lessons_root = lessons_root.clone();
        Arc::new(move || {
            let store = SqliteMemoryStore::open(&cognitive_db)
                .map_err(|error| format!("memory store open failed: {error}"))?;
            let counts = store
                .ledger_counts()
                .map_err(|error| format!("memory ledger query failed: {error}"))?;
            let reflexion = FileReflexionStore::new(lessons_root.clone());
            let lessons = reflexion
                .list_reflections()
                .map_err(|error| format!("lesson store read failed: {error}"))?;
            Ok(MemoryLedgerStats {
                sessions: Some(counts.sessions),
                memories: Some(counts.memories),
                protected: Some(counts.protected),
                lessons: Some(lessons.len() as u64),
                reason: None,
            })
        })
    };

    let backends = ProductionBackends {
        self_status_ledger: Some(ledger_probe),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(base_config(), backends);
    let report = invoke(&tool).await;
    let ledger = &report["memory_ledger"];
    assert_eq!(ledger["sessions"], 2);
    assert_eq!(ledger["memories"], 3);
    assert_eq!(ledger["protected"], 1);
    assert_eq!(ledger["lessons"], 1);
    assert_eq!(ledger["reason"], Value::Null);
    // 计数不回内容原文。
    let rendered = report.to_string();
    for leaked in ["内容原文-甲", "内容原文-乙", "内容原文-丙", "失败样本"] {
        assert!(
            !rendered.contains(leaked),
            "memory content leaked: {leaked}"
        );
    }
}

#[tokio::test]
async fn missing_data_probes_are_explicit_null_with_reasons() {
    // 取值断言的确定性: 显式装基线生效值, 不吃宿主 env 的偶然值。
    apeireth_orchestration::self_tuning::install_effective_values(TuningValues::baseline());
    let (_, tool) = build(base_config(), ProductionBackends::default());
    let report = invoke(&tool).await;

    let ledger = &report["memory_ledger"];
    for field in ["sessions", "memories", "protected", "lessons"] {
        assert_eq!(ledger[field], Value::Null, "{field} must be explicit null");
    }
    assert!(
        ledger["reason"]
            .as_str()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED),
        "{}",
        ledger["reason"]
    );

    let workspace = &report["workspace"];
    assert_eq!(workspace["credentials_present"], Value::Null);
    assert!(
        workspace["reason"]
            .as_str()
            .unwrap()
            .contains(DATA_PROBE_NOT_WIRED),
        "{}",
        workspace["reason"]
    );

    // 生效值部分照常真实在场, 不整帧失败。
    assert_eq!(report["identity"]["product_name"], PRODUCT_NAME);
    assert_eq!(report["tuning"]["preset"], PRESET_BALANCED);
    assert_eq!(report["budget"]["source"], "constant");
}

#[tokio::test]
async fn credential_presence_is_boolean_and_never_echoes_the_credential() {
    let secret = "sk-live-should-never-be-echoed";
    let credentials_probe: StatusProbe<bool> = {
        let secret = secret.to_string();
        Arc::new(move || Ok(secret.contains("sk-live")))
    };
    let backends = ProductionBackends {
        self_status_credentials: Some(credentials_probe),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(base_config(), backends);
    let report = invoke(&tool).await;
    assert_eq!(report["workspace"]["credentials_present"], true);
    assert!(
        !report.to_string().contains(secret),
        "credential value echoed"
    );

    // 探测失败 = 显式 null + 原因, 不猜。
    let failing: StatusProbe<bool> = Arc::new(|| Err("probe unavailable".to_string()));
    let backends = ProductionBackends {
        self_status_credentials: Some(failing),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(base_config(), backends);
    let report = invoke(&tool).await;
    assert_eq!(report["workspace"]["credentials_present"], Value::Null);
    assert!(
        report["workspace"]["reason"]
            .as_str()
            .unwrap()
            .contains("probe unavailable"),
        "{}",
        report["workspace"]["reason"]
    );
}

#[tokio::test]
async fn tuning_section_reports_sliders_preset_and_self_learning_switch() {
    // 自学习生效 = 接线 AND 记忆召回 (信号链挂记忆召回模块, 按实际注册条件
    // 取值); 取值为生效值 (未设旋钮 = 全基线)。
    let dir = tempfile::tempdir().expect("temp dir");
    let store =
        Arc::new(SqliteMemoryStore::open(dir.path().join("cognitive.sqlite3")).expect("store"));
    let wire = Arc::new(SelfTuningWire::new(
        std::env::temp_dir().join(format!(
            "apeireth-self-status-tuning-{}-2.jsonl",
            std::process::id()
        )),
        TuningValues::baseline(),
    ));
    let mut config = base_config();
    config.memory_recall = true;
    let backends = ProductionBackends {
        memory: Some(store.clone()),
        memory_governance: Some(store),
        self_tuning: Some(wire),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(config, backends);
    let report = invoke(&tool).await;
    let tuning = &report["tuning"];
    assert_eq!(tuning["self_learning"], true);
    assert_eq!(tuning["preset"], PRESET_BALANCED);
    assert_eq!(tuning["values"]["memory_fade"], 1.0);
    assert_eq!(tuning["values"]["curiosity_strength"], 1.0);
    assert_eq!(tuning["values"]["tone_saturation"], 1.0);
    assert_eq!(tuning["values"]["consolidation_cadence"], 1.0);
    let budget = &report["budget"];
    assert_eq!(budget["max_rounds_per_turn"], 8);
    assert_eq!(budget["max_tool_calls_per_round"], 16);
    assert_eq!(budget["source"], "constant");
    assert!(
        budget["note"].as_str().unwrap().contains("constants"),
        "{}",
        budget["note"]
    );

    // 召回缺席 = 信号链无处生效, 照实 false (旧行为: 报"接线在场" = true)。
    let wire = Arc::new(SelfTuningWire::new(
        std::env::temp_dir().join(format!(
            "apeireth-self-status-tuning-{}-3.jsonl",
            std::process::id()
        )),
        TuningValues::baseline(),
    ));
    let backends = ProductionBackends {
        self_tuning: Some(wire),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(base_config(), backends);
    let report = invoke(&tool).await;
    assert_eq!(report["tuning"]["self_learning"], false);
}

#[tokio::test]
async fn workspace_section_reports_root_path() {
    let root = PathBuf::from("workspace-root-probe");
    let backends = ProductionBackends {
        workspace_root: Some(root.clone()),
        ..ProductionBackends::default()
    };
    let (_, tool) = build(base_config(), backends);
    let report = invoke(&tool).await;
    assert_eq!(
        report["workspace"]["root"],
        root.to_string_lossy().to_string()
    );
}

/// 与输出归一合同的对账: 冻结形状 = `ToolResult` 渲染即结构化 JSON。
#[tokio::test]
async fn the_frame_is_the_normalized_contract_shape() {
    let (_, tool) = build(base_config(), ProductionBackends::default());
    let call = ToolCall {
        id: "call_contract".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result: ToolResult = tool.invoke(&call).await;
    let outcome = ToolOutcome::freeze(&result);
    assert!(outcome.ok);
    assert!(SelfStatusTool::output_schema()
        .validate(outcome.structured().expect("structured"))
        .is_ok());
    assert_eq!(
        outcome.output_text,
        result.render(),
        "输出归一合同: output_text 即渲染文本"
    );
}
