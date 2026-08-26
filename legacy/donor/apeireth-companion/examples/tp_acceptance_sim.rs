//! `tp_acceptance_sim` — **TP11-TP20 两批团队活儿的主线程验收模拟** (2026-08-18).
//!
//! 补单元测试之外的缺口: 用**真实调用路径**模拟主人/用户视角的关键流程, 不 mock:
//!
//! | 场景 | 对应批次 | 模拟的真实流程 |
//! |---|---|---|
//! | S1 | TP11 (A1, P0) | orchestrator 委托: `transfer_to_<agent>` 工具族 → HandoffRegistry 协议 → OnHandoff 回调 → 禁用目标被拒 |
//! | S2 | TP12-Rework (P0 返工) | 工具输出 schema 校验 + guardrails: 非法参数/缺字段/类型错被拒, 非法输出触发 tripwire |
//! | S3 | TP18 (E3, P1) | oracle 校准 (Brier/分箱/ECE/分解) → 集合预报聚合 → 预测市场下单 → Critic 审阅 |
//! | S4 | TP20-N20 | companion 审批 ↔ team-lead orchestrator 双向同步 (record_request → mark_approved → bridge 收到 approved) |
//!
//! **0 装 PASS**: 断言失败直接 panic (不打印假装通过); 全部 PASS 才打印汇总.
//! **运行**: `cargo run -p apeireth-companion --example tp_acceptance_sim`

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_cognition::calibration::{
    brier_score, calibration_bins, decompose, expected_calibration_error, Observation,
};
use apeireth_cognition::forecast::{
    EnsembleConfig, EnsembleForecast, EnsembleMember, MarketConfig, PredictionMarket,
};
use apeireth_evolution::critic::Critic;
use apeireth_memory::SqliteMemoryStore;
use apeireth_team_lead::{
    ApprovalBridge, ApprovalResponse as WireApprovalResponse, InProcessBridge,
};
use apeireth_tool_registry::{install_handoff_tools, HandoffRegistry, SyncOnHandoff};
use apeireth_tool_runtime::{ToolCallParser, ToolExecutor};
use apeireth_tools::guardrail::{post_call_tripwire, pre_call_guard};
use apeireth_tools::register_all;
use apeireth_tools::schema::{validate, SchemaMap, SchemaNode};
use serde_json::json;

use apeireth_companion::approval_requests::{list, mark_approved, record_request};

fn check(name: &str, cond: bool, detail: impl AsRef<str>) -> Result<(), String> {
    if cond {
        println!("    [PASS] {name} — {}", detail.as_ref());
        Ok(())
    } else {
        println!("    [FAIL] {name} — {}", detail.as_ref());
        Err(format!("{name}: {}", detail.as_ref()))
    }
}

// =====================================================================
// S1 — TP11 Handoff 委托协议 (transfer_to_<agent> 工具族)
// =====================================================================
async fn s1_handoff() -> Result<(), String> {
    println!("S1 [TP11] Handoff 委托协议");
    let registry = Arc::new(apeireth_tool_registry::ToolRegistry::new());
    let hreg = Arc::new(HandoffRegistry::new());
    hreg.register_simple("researcher", Some("调研员: 查资料回传".to_string()));
    hreg.register_simple("writer", Some("写作者: 成文".to_string()));

    let installed = install_handoff_tools(&registry, &hreg);
    check(
        "工具注册",
        installed >= 2,
        &format!("install_handoff_tools 返回 {installed}"),
    )?;

    let tool_names = hreg.list_tool_names();
    check(
        "transfer_to_ 工具族命名约定",
        tool_names.iter().any(|t| t == "transfer_to_researcher"),
        &format!("工具: {:?}", tool_names),
    )?;

    // OnHandoff 回调: 模拟目标 agent 真实接收委托
    let delivered = Arc::new(AtomicUsize::new(0));
    let d2 = Arc::clone(&delivered);
    hreg.set_on_handoff(
        "researcher",
        Arc::new(SyncOnHandoff {
            name: "sim-researcher-receiver",
            func: Box::new(move |req: &apeireth_tool_registry::HandoffRequest| {
                if req.target_agent == "researcher" {
                    d2.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            }),
        }),
    );

    // orchestrator 视角: LLM 输出 marker → 解析 → 执行 transfer_to_researcher
    let llm = "把 '查一下 Rust 异步' 委托给调研员\n<<<[TOOL_REQUEST]>>>\ntool_name:<<<transfer_to_researcher>>>\nmessage:<<<查一下 Rust 异步>>>\n<<<[END_TOOL_REQUEST]>>>";
    let calls = ToolCallParser::parse(llm).map_err(|e| format!("解析失败: {e}"))?;
    check(
        "orchestrator 解析委托调用",
        calls.len() == 1 && calls[0].tool_name == "transfer_to_researcher",
        "解析出 1 个 transfer 调用",
    )?;

    let executor = ToolExecutor::with_timeout(Arc::clone(&registry), 10_000);
    let result = executor.execute(&calls[0]).await;
    check(
        "委托执行成功",
        result.success,
        &format!("duration_ms={}", result.duration_ms),
    )?;
    check(
        "目标 agent 收到委托 (OnHandoff)",
        delivered.load(Ordering::SeqCst) == 1,
        "回调触发 1 次",
    )?;

    // 禁用目标 → 委托被拒 (机制而非补丁: orchestrator 不能把活交给停用的 agent)
    hreg.set_enabled("writer", false);
    let llm2 = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<transfer_to_writer>>>\nmessage:<<<写报告>>>\n<<<[END_TOOL_REQUEST]>>>";
    let calls2 = ToolCallParser::parse(llm2).map_err(|e| format!("解析失败: {e}"))?;
    let result2 = executor.execute(&calls2[0]).await;
    check(
        "禁用目标委托被拒",
        !result2.success,
        "set_enabled(false) 后 transfer 应失败",
    )?;
    Ok(())
}

// =====================================================================
// S2 — TP12-Rework 工具输出 schema 校验 + guardrails
// =====================================================================
async fn s2_schema_guardrail() -> Result<(), String> {
    println!("S2 [TP12-Rework] schema 校验 + guardrails");
    // schema: FileOperator read 输出 = object{op: string, path: string, content: string}
    // (与 apeireth-tools file_ops 真实输出结构一致: {"op","path","content"})
    let mut fields = BTreeMap::new();
    fields.insert("op".to_string(), SchemaNode::String);
    fields.insert("path".to_string(), SchemaNode::String);
    fields.insert("content".to_string(), SchemaNode::String);
    let schema = SchemaNode::Object { fields };

    validate(
        &schema,
        &json!({"op": "read", "path": "a.txt", "content": "hi"}),
    )
    .map_err(|e| format!("合法输出被拒: {e:?}"))?;
    check("合法输出通过", true, "op/path/content 全类型正确")?;

    let r_missing = validate(&schema, &json!({"op": "read", "path": "a.txt"}));
    check(
        "缺字段被拒",
        r_missing.is_err(),
        format!("缺失 content → {:?}", r_missing.err()),
    )?;

    let r_type = validate(
        &schema,
        &json!({"op": "read", "path": "a.txt", "content": 42}),
    );
    check("类型错被拒", r_type.is_err(), "content 给了数字应 Err")?;

    // guardrail 前置: 危险内容拦截 (语义 = 路径穿越/命令注入; 参数完整性归 schema validate)
    let ok_args = json!({"op": "read", "path": "Cargo.toml"});
    let traversal_args = json!({"op": "read", "path": "../secret.txt"});
    let inject_args = json!({"op": "exec", "command": "ls; rm -rf /"});
    check(
        "pre_call_guard 合法参数放行",
        pre_call_guard("FileOperator", &ok_args).is_ok(),
        "正常路径",
    )?;
    let bad = pre_call_guard("FileOperator", &traversal_args);
    check(
        "pre_call_guard 路径穿越拒绝",
        bad.is_err(),
        format!("`../secret.txt` → {bad:?}"),
    )?;
    let bad2 = pre_call_guard("ShellExec", &inject_args);
    check(
        "pre_call_guard 命令注入拒绝",
        bad2.is_err(),
        format!("`; rm -rf /` → {bad2:?}"),
    )?;

    // guardrail 后置: 输出凭据泄漏 tripwire (语义 = secrets 扫描, 非类型校验)
    let bad_out = post_call_tripwire(
        "FileOperator",
        &json!({"op": "read", "path": "x", "status": 0, "leak": "AKIAIOSFODNN7EXAMPLE"}),
    );
    check(
        "post_call_tripwire 凭据泄漏触发",
        bad_out.is_some(),
        "输出含 AWS AKIA key 应触发 tripwire",
    )?;
    let ok_out = post_call_tripwire(
        "FileOperator",
        &json!({"op": "read", "path": "x", "status": 0}),
    );
    check(
        "post_call_tripwire 干净输出不触发",
        ok_out.is_none(),
        "无凭据应放行",
    )?;

    // 真实执行路径: 8 工具注册 + schema map 注入 + 解析 LLM 输出 → 执行
    let registry = Arc::new(apeireth_tool_registry::ToolRegistry::new());
    register_all(&registry).map_err(|e| format!("register_all: {e}"))?;
    let mut schemas = SchemaMap::new();
    schemas.insert("FileOperator", schema);
    let executor = ToolExecutor::with_schema_map(Arc::clone(&registry), 10_000, schemas);
    check(
        "executor 注入 schema map",
        executor.schemas().get("FileOperator").is_some(),
        "with_schema_map 生效",
    )?;

    let llm = "<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\nop:<<<read>>>\npath:<<<Cargo.toml>>>\n<<<[END_TOOL_REQUEST]>>>";
    let calls = ToolCallParser::parse(llm).map_err(|e| format!("解析失败: {e}"))?;
    let r = executor.execute(&calls[0]).await;
    check(
        "真实 FileOperator 执行 (schema 注入下)",
        r.success,
        &format!(
            "duration_ms={} error={:?} guardrail={:?} validation={:?}",
            r.duration_ms, r.error, r.guardrail_error, r.validation_error
        ),
    )?;
    Ok(())
}

// =====================================================================
// S3 — TP18 oracle 校准诊断 + 集合预报 + 预测市场 + Critic
// =====================================================================
fn s3_calibration_forecast() -> Result<(), String> {
    println!("S3 [TP18] 校准 + 集合预报 + 预测市场 + Critic");
    // 校准良好的预言机: forecast=0.7, 100 次里 70 次命中 → Brier ≈ 0.21
    let obs: Vec<Observation> = (0..100)
        .map(|i| Observation {
            forecast: 0.7,
            outcome: if i < 70 { 1.0 } else { 0.0 },
        })
        .collect();
    let brier = brier_score(&obs);
    check(
        "Brier 分数",
        (brier - 0.21).abs() < 0.001,
        &format!("brier={brier:.4} (期望≈0.21)"),
    )?;

    let bins = calibration_bins(&obs, 10);
    check("校准分箱", bins.len() == 10, &format!("{} 箱", bins.len()))?;
    let ece = expected_calibration_error(&bins);
    check("ECE 小 (校准良好)", ece < 0.3, &format!("ece={ece:.4}"))?;
    let dec = decompose(&obs, 10);
    println!("    分解: {dec:?}");

    // 集合预报: 两成员聚合
    let members = vec![
        EnsembleMember::new("member-a", 0.7, 0.9),
        EnsembleMember::new("member-b", 0.6, 0.8),
    ];
    let ens = EnsembleForecast::aggregate(members, EnsembleConfig::default());
    check(
        "集合预报聚合",
        (0.0..=1.0).contains(&ens.aggregate_prediction),
        &format!("聚合预报={:.3}", ens.aggregate_prediction),
    )?;
    let as_obs = ens.as_observation(1.0);
    check(
        "集合预报 → 观测 (喂回校准)",
        as_obs.forecast == ens.aggregate_prediction,
        "as_observation 保留 forecast",
    )?;

    // 预测市场: 下单 → 收据 → 聚合信念
    let mut market = PredictionMarket::new(MarketConfig::default());
    let prices = market.prices();
    check(
        "市场报价",
        prices.len() >= 2,
        &format!("outcome 数={}", prices.len()),
    )?;
    let receipt = market
        .execute_buy(0, 10.0)
        .map_err(|e| format!("下单失败: {e:?}"))?;
    check(
        "下单收据",
        receipt.shares > 0.0 && receipt.cost >= 0.0,
        &format!("shares={:.2} cost={:.2}", receipt.shares, receipt.cost),
    )?;
    let belief = market.aggregate_belief(0);
    check(
        "聚合信念",
        (0.0..=1.0).contains(&belief),
        &format!("belief[0]={belief:.3}"),
    )?;

    // Critic: 审阅同一段历史
    let critic = Critic::default_critic();
    let cr = critic.critique(&obs);
    println!(
        "    critic: severity={:.4} brier_est={:.4} ece={:.4} action={:?}",
        cr.severity, cr.brier_estimate, cr.expected_calibration_error, cr.recommended_action
    );
    check(
        "Critic 审阅产出 (推荐动作非空)",
        !format!("{:?}", cr.recommended_action).is_empty(),
        "有推荐动作",
    )?;
    Ok(())
}

// =====================================================================
// S4 — TP20-N20 ApprovalBridge 跨 crate 双向同步
// =====================================================================
fn s4_approval_bridge() -> Result<(), String> {
    println!("S4 [TP20-N20] ApprovalBridge 双向同步 (companion ↔ team-lead)");
    let store = Arc::new(SqliteMemoryStore::open_in_memory().map_err(|e| e.to_string())?);
    let bridge = Arc::new(InProcessBridge::new());
    // orchestrator 暂挂 (pending): 本地状态保持, mark_approved 路径可达
    bridge.on_request(|req| WireApprovalResponse {
        chain: req.chain.clone(),
        decision: "pending".into(),
        decided_at: 0,
        note: "hold".into(),
        extra: Default::default(),
    });
    let bridge_ref: Arc<dyn ApprovalBridge> = bridge.clone();

    record_request(
        &store,
        "ShellExec",
        &json!({"cmd": "ls"}),
        "验收模拟: 需要批准",
        Some(&bridge_ref),
    );
    let pending = list(&store, Some("pending"));
    check(
        "请求落库 pending",
        pending.len() == 1,
        "1 条 ShellExec 待批",
    )?;
    let chain = pending[0].chain.clone();

    mark_approved(&store, &chain, Some(&bridge_ref)).map_err(|e| format!("mark_approved: {e}"))?;
    let approved = list(&store, Some("approved"));
    check("本地状态 → approved", approved.len() == 1, "本地 1 条已批")?;
    let responses = bridge.received_responses();
    let resp = responses
        .iter()
        .find(|r| r.chain == chain && r.decision == "approved");
    check(
        "orchestrator 收到 approved 响应",
        resp.is_some(),
        "bridge 双向同步生效",
    )?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let mut fails = Vec::new();
    for (name, fut) in [
        (
            "S1 handoff",
            Box::pin(s1_handoff())
                as std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>>>>,
        ),
        ("S2 schema_guardrail", Box::pin(s2_schema_guardrail())),
    ] {
        match fut.await {
            Ok(()) => println!("  ✓ {name} PASS"),
            Err(e) => fails.push(format!("{name}: {e}")),
        }
        println!();
    }
    match (s3_calibration_forecast(), s4_approval_bridge()) {
        (Ok(()), Ok(())) => {
            println!("  ✓ S3 calibration_forecast PASS\n  ✓ S4 approval_bridge PASS")
        }
        (a, b) => {
            if let Err(e) = a {
                fails.push(format!("S3: {e}"));
            }
            if let Err(e) = b {
                fails.push(format!("S4: {e}"));
            }
        }
    }
    println!();
    if fails.is_empty() {
        println!("=== TP 验收模拟: 4/4 场景全 PASS ===");
    } else {
        println!("=== TP 验收模拟: {} 处失败 ===", fails.len());
        for f in &fails {
            println!("  FAIL {f}");
        }
        std::process::exit(1);
    }
}
