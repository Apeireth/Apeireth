//! apeireth-cli: CLI (CliRunner, 暴露 Rust 子系统给终端)
//! R14 Phase 0 接口规范对照: 命令协议稳定, 不轻易改
//! 主 23:44 干到底: 命令少而稳, 不要堆砌
//!
//! A1.1: session 子命令接 core Session API
//! - `create_default_session()` / `build_default_human_authority()` / `build_default_permission_onion()`
//! - `welcome_message()` 动态生成（基于 Session 真实字段，不再硬编码）
//! - `handle_input_line()` 跑 V1+V2+V3 AND 门 → `describe_verdict()` 可读输出
//!
//! round10-12 (qa_engineer): 新增 asi 子命令:
//! - `asi trace --tail N`     → 最近 N 条 DimensionTrace 详细表
//! - `asi trend --dim X --last N` → X 维最近 N 个值的 ASCII sparkline
//! - `asi diagnose --top N`  → 自动定位最弱 N 个维度 + 改进建议
//!
//! ponytail: 不引入新依赖（uuid crate），id 用 timestamp + 单调计数合成；risk 分级用关键词启发式


use std::sync::atomic::{AtomicU64, Ordering};

use apeireth_asi::{
    ascii_sparkline, diagnose_weakest, format_trace_table, AdaptiveBaseline, CalibrationLoop,
    DimensionTrace, LinearCalibration, MeasurementSample, RecalibrationScheduler, TraceRepository,
    V05_DIMENSION_NAMES,
};
use apeireth_core::{
    Action, ActionGuard, ActionTarget, ActionVerdict, DefaultPhilosophyGuard, HAAuthentication,
    HAMode, HumanAuthority, PermissionLayer, PermissionOnion, RealHuman, RiskLevel, Session,
};

/// CLI 命令协议
#[derive(Debug, Clone)]
pub enum CliCommand {
    /// 启动一次 session
    Session,
    /// 列出最近 N 个 episode
    ListEpisodes,
    /// 运行 V1136 真测
    RunV1136,
    /// ASI 子命令 (round10-12 qa_engineer)
    Asi(AsiSubCommand),
    /// 🆕 R16-09 apeireth-api 聚合网关子命令
    Gateway(GatewaySubCommand),
    /// 退出
    Quit,
}

/// R17 简化: gateway 子命令只保留 serve, 删 status / routes (NewAPI channel 借鉴已砍)
#[derive(Debug, Clone)]
pub enum GatewaySubCommand {
    /// `gateway serve` —— 启动 HTTP server (4 endpoint, 默认端口 8080)
    Serve {
        /// 端口 (默认 8080)
        port: Option<u16>,
    },
}

/// ASI 子命令 (Ponytail: 3 个 pure dispatch 函数)
#[derive(Debug, Clone)]
pub enum AsiSubCommand {
    /// `asi trace --tail N`: 列出最近 N 条 DimensionTrace 详细表
    Trace {
        /// tail N (默认 10)
        n: usize,
    },
    /// `asi trend --dim <name> --last N`: 某维度最近 N 值的 ASCII sparkline
    Trend {
        /// 维度名 (V05_DIMENSION_NAMES 或 V1136_SUBMEASURE_NAMES)
        dim: String,
        /// last N (默认 20)
        last: usize,
    },
    /// `asi diagnose --top N`: 自动定位最弱 N 维度 + 改进建议
    Diagnose {
        /// 选最弱的 N 个维度 (默认 3)
        top: usize,
    },
    /// `asi calibrate --dry-run|apply --every M` (round15-01 backend_engineer)
    Calibrate {
        /// dry-run (只算系数不写) 或 apply (写回 history)
        mode: CalibrateMode,
        /// 每 M 次测量触发一次 (默认 100)
        every: usize,
        /// 范围限制: "all" (默认), "v05_dims", "v1136_subs", 或具体维度名
        scope: String,
    },
}

/// ASI calibrate 子命令模式 (round15-01 backend_engineer)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrateMode {
    /// 仅计算并打印新系数, 不修改 scheduler.history
    DryRun,
    /// 真实写入系数到 scheduler.history
    Apply,
}

// ponytail: 单调计数器用于 session/action id 唯一性（不引入 uuid 依赖）
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 构造默认 Session（id = `sess-<unix_seconds>-<counter>`）
pub fn create_default_session() -> Session {
    let now = chrono::Utc::now().timestamp();
    let n = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    Session {
        id: format!("sess-{}-{:x}", now, n),
        started_at: now,
        last_active_at: now,
    }
}

/// 构造默认 PermissionOnion（L0-L5 六层，全部需要 HA）
pub fn build_default_permission_onion() -> PermissionOnion {
    PermissionOnion {
        l0: PermissionLayer {
            name: "L0 HA 核心".into(),
            description: "最后护栏，永远需要真实人类批准".into(),
            requires_ha: true,
        },
        l1: PermissionLayer {
            name: "L1 受控写".into(),
            description: "受控写入 (E 层嵌入)".into(),
            requires_ha: true,
        },
        l2: PermissionLayer {
            name: "L2 重要操作".into(),
            description: "重要操作 (O 层嵌入)".into(),
            requires_ha: true,
        },
        l3: PermissionLayer {
            name: "L3 关键操作".into(),
            description: "关键操作 (M 层嵌入)".into(),
            requires_ha: true,
        },
        l4: PermissionLayer {
            name: "L4 核心升级".into(),
            description: "核心升级 (A 层嵌入)".into(),
            requires_ha: true,
        },
        l5: PermissionLayer {
            name: "L5 核武器级".into(),
            description: "核武器级变更 (S 层嵌入)".into(),
            requires_ha: true,
        },
    }
}

/// 构造默认 HumanAuthority（SingleHuman 模式，1 个占位主人）
pub fn build_default_human_authority() -> HumanAuthority {
    HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![RealHuman {
            id: "default-owner".into(),
            name: "Apeireth 主人".into(),
            authentication: HAAuthentication::WindowsHello,
            biometric_data: None,
        }],
        ice_frozen_until: None,
    }
}

/// 生成动态欢迎信息（从 Session 字段派生，不再硬编码）
pub fn welcome_message(session: &Session, ha: &HumanAuthority, po: &PermissionOnion) -> String {
    let started_iso = chrono::DateTime::from_timestamp(session.started_at, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| session.started_at.to_string());
    format!(
        "🚀 Apeireth Session 启动\n\
         \x20 Session ID    : {sid}\n\
         \x20 started_at   : {ts}\n\
         \x20 last_active  : {ts}\n\
         \x20 HA mode      : {mode:?} ({n} humans)\n\
         \x20 PermissionOnion: L0={l0} / L5={l5}\n\
         \x20 守门          : V1+V2+V3 AND 门 (双洋葱 + HA)\n\
         \x20 ✅ session 已启动 (A1 第 1 天任务完成)",
        sid = session.id,
        ts = started_iso,
        mode = ha.mode,
        n = ha.real_humans.len(),
        l0 = po.l0.name,
        l5 = po.l5.name,
    )
}

/// 简单风险分级（关键词启发式，best-effort；正式版用真 NLP/分类器）
pub fn classify_risk(text: &str) -> RiskLevel {
    let lower = text.to_lowercase();
    if lower.contains("l0") || lower.contains(" ha") || lower.contains("modify l0") {
        RiskLevel::Critical
    } else if lower.contains("upgrade")
        || lower.contains("delete")
        || lower.contains("reset")
        || lower.contains("reorganize")
        || lower.contains("onion")
        || lower.contains("核心")
        || lower.contains("重组")
    {
        RiskLevel::High
    } else if lower.contains("write") || lower.contains("save") || lower.contains("修改") {
        RiskLevel::Medium
    } else if text.trim().is_empty() {
        RiskLevel::Info
    } else {
        RiskLevel::Low
    }
}

/// 从一行用户输入构造 Action（NormalAction 为主；含 L0 关键词升级到 ModifyL0HA）
pub fn build_action_from_input(line: &str, session: &Session) -> Action {
    let risk = classify_risk(line);
    let lower = line.to_lowercase();
    let target = if matches!(risk, RiskLevel::Critical)
        && (lower.contains("l0") || lower.contains(" ha"))
    {
        ActionTarget::ModifyL0HA
    } else if lower.contains("reorganize") || lower.contains("onion") || lower.contains("重组") {
        ActionTarget::ReorganizeOnion
    } else {
        ActionTarget::NormalAction(line.to_string())
    };
    let n = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    Action {
        id: format!("{}#{}", session.id, n),
        description: format!("cli-input: {}", line),
        risk_level: risk,
        target,
    }
}

/// 把 ActionVerdict 转成单行可读字符串
pub fn describe_verdict(verdict: &ActionVerdict) -> String {
    match verdict {
        ActionVerdict::Allow => "✅ Allow (V1+V2+V3 全通过)".to_string(),
        ActionVerdict::BlockByPrinciple(key) => {
            format!("❌ BlockByPrinciple({})", key.description())
        }
        ActionVerdict::BlockByPermission(reason) => {
            format!("❌ BlockByPermission: {}", reason)
        }
        ActionVerdict::BlockByHumanAuthority(reason) => {
            format!("❌ BlockByHumanAuthority: {}", reason)
        }
    }
}

/// 对一行用户输入跑完整 V1+V2+V3 AND 门（CLI 默认场景）
///
/// ponytail: 内部构造默认 HA/PermissionOnion/DefaultPhilosophyGuard，调用 `ActionGuard::check_action`。
/// 复杂场景（自定义 guard / onion）直接调 `ActionGuard::check_action`。
pub fn handle_input_line(line: &str, session: &Session) -> ActionVerdict {
    let action = build_action_from_input(line, session);
    run_session_action(&action)
}

/// 对一个 Action 跑完整 V1+V2+V3 AND 门（ADR 0002 公开 API + A1.2 集成测试契约）
///
/// 内部用 `DefaultPhilosophyGuard` + 默认 PermissionOnion + 默认 HumanAuthority。
/// A5+ 阶段会升级为自定义 guard / onion / HA 注入。
pub fn run_session_action(action: &Action) -> ActionVerdict {
    let guard = DefaultPhilosophyGuard;
    let po = build_default_permission_onion();
    let ha = build_default_human_authority();
    ActionGuard::check_action(action, &guard, &po, &ha)
}

/// 占位函数（向后兼容旧测试；新逻辑已迁到 create_default_session/welcome_message 等）
pub fn placeholder() -> &'static str {
    "apeireth-cli R14 skeleton"
}
pub mod commands;  // R116: skills/eval/council subcommand set
// R127-2 P9-1: clap ValueEnum 借鉴 (Stage 2 借脑 1.0, per decision-56 §2.4)
pub mod output_format;

// 重新导出核心常量（CLI 用户可能需要的哲学键）
pub use apeireth_core::PhilosophyKey as Key;

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::PhilosophyKey;

    #[test]
    fn test_placeholder_ok_backcompat() {
        assert_eq!(placeholder(), "apeireth-cli R14 skeleton");
    }

    #[test]
    fn test_create_default_session_has_valid_fields() {
        let s = create_default_session();
        assert!(s.id.starts_with("sess-"));
        assert!(s.started_at > 0);
        assert_eq!(s.started_at, s.last_active_at);
    }

    #[test]
    fn test_session_ids_are_unique() {
        let a = create_default_session();
        let b = create_default_session();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn test_default_human_authority_single_mode() {
        let ha = build_default_human_authority();
        assert!(matches!(ha.mode, HAMode::SingleHuman));
        assert!(ha.ice_frozen_until.is_none());
        assert_eq!(ha.real_humans.len(), 1);
        assert!(ha.real_humans[0].biometric_data.is_none());
    }

    #[test]
    fn test_default_permission_onion_has_six_layers() {
        let po = build_default_permission_onion();
        assert_eq!(po.l0.name, "L0 HA 核心");
        assert_eq!(po.l5.name, "L5 核武器级");
        assert!(po.l0.requires_ha);
        assert!(po.l5.requires_ha);
    }

    #[test]
    fn test_classify_risk_levels() {
        assert_eq!(classify_risk(""), RiskLevel::Info);
        assert_eq!(classify_risk("hello world"), RiskLevel::Low);
        assert_eq!(classify_risk("write to file"), RiskLevel::Medium);
        assert_eq!(classify_risk("delete this"), RiskLevel::High);
        assert_eq!(classify_risk("upgrade core"), RiskLevel::High);
        assert_eq!(classify_risk("modify L0 HA"), RiskLevel::Critical);
        assert_eq!(classify_risk("reorganize onion"), RiskLevel::High);
    }

    #[test]
    fn test_handle_input_normal_allows() {
        let s = create_default_session();
        let action = Action {
            id: format!("{}-test-hello", s.id),
            description: "hello world".into(),
            risk_level: RiskLevel::Low,
            target: ActionTarget::NormalAction("hello world".into()),
        };
        let v = run_session_action(&action);
        // SingleHuman + Low risk + NormalAction → Allow
        assert!(matches!(v, ActionVerdict::Allow));
    }

    #[test]
    fn test_handle_input_l0_blocked_by_principle() {
        let action = Action {
            id: "test-l0".into(),
            description: "modify L0 HA please".into(),
            risk_level: RiskLevel::Critical,
            target: ActionTarget::ModifyL0HA,
        };
        let v = run_session_action(&action);
        // V1 (DefaultPhilosophyGuard) 必须最先拦 ModifyL0HA → NotUnobservable
        assert!(matches!(
            v,
            ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnobservable)
        ));
    }

    #[test]
    fn test_handle_input_line_e2e() {
        // 对话循环端到端：构造 session → 普通文本 → Allow → L0 攻击 → Block
        let s = create_default_session();
        let v1 = handle_input_line("hello world", &s);
        assert!(matches!(v1, ActionVerdict::Allow));
        let v2 = handle_input_line("modify L0 HA please", &s);
        assert!(matches!(v2, ActionVerdict::BlockByPrinciple(_)));
    }

    #[test]
    fn test_welcome_message_uses_session_fields_dynamic() {
        let s = Session {
            id: "sess-fixture-dynamic-id".into(),
            started_at: 1_700_000_000,
            last_active_at: 1_700_000_000,
        };
        let ha = build_default_human_authority();
        let po = build_default_permission_onion();
        let msg = welcome_message(&s, &ha, &po);
        assert!(msg.contains("sess-fixture-dynamic-id"));
        assert!(msg.contains("SingleHuman"));
        assert!(msg.contains("L0 HA 核心"));
    }

    #[test]
    fn test_describe_verdict_strings() {
        assert!(describe_verdict(&ActionVerdict::Allow).contains("Allow"));
        let v = ActionVerdict::BlockByPrinciple(PhilosophyKey::NotUnobservable);
        assert!(describe_verdict(&v).contains("PHL-04"));
        let v = ActionVerdict::BlockByPermission("风险=Critical".into());
        assert!(describe_verdict(&v).contains("BlockByPermission"));
        let v = ActionVerdict::BlockByHumanAuthority("HA 拒绝".into());
        assert!(describe_verdict(&v).contains("BlockByHumanAuthority"));
    }

    #[test]
    fn test_build_action_from_input_id_increments() {
        let s = create_default_session();
        let a1 = build_action_from_input("test 1", &s);
        let a2 = build_action_from_input("test 2", &s);
        assert_ne!(a1.id, a2.id);
        assert!(a1.id.starts_with(&s.id));
    }
}

// =============================================================================
// round10-12 (qa_engineer): ASI 子命令 dispatch 函数
// =============================================================================

/// Dispatch `asi trace --tail N`: 输出最近 N 条 DimensionTrace 详细表。
pub fn dispatch_asi_trace(repo: &TraceRepository, n: usize) -> String {
    if repo.is_empty() {
        return "TraceRepository is empty. Append traces first.".to_string();
    }
    let traces = repo.tail(n);
    let mut out = String::new();
    for t in traces.iter() {
        out.push_str(&format_trace_table(t));
    }
    out
}

/// Dispatch `asi trend --dim X --last N`: 输出维度 X 最近 N 值的 ASCII sparkline。
pub fn dispatch_asi_trend(repo: &TraceRepository, dim: &str, last: usize) -> String {
    let values = repo.trend(dim, last);
    if values.is_empty() {
        return format!("No history for dim `{dim}`. Check name or append traces first.");
    }
    let spark = ascii_sparkline(&values);
    format!(
        "Trend for `{dim}` (last {} values):\n{}\n  min={:.4} max={:.4} mean={:.4}\n",
        values.len(),
        spark,
        values.iter().cloned().fold(f64::INFINITY, f64::min),
        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        values.iter().sum::<f64>() / values.len() as f64,
    )
}

/// Dispatch `asi diagnose --top N`: 定位最弱 N 维度 + 改进建议。
pub fn dispatch_asi_diagnose(trace: &DimensionTrace, top: usize) -> String {
    let report = diagnose_weakest(trace, top);
    let mut out = String::new();
    out.push_str(&format!(
        "Diagnosis for trace #{} (sample {}):\n",
        trace.trace_id, trace.sample_id
    ));
    out.push_str(&format!("Weakest {top} dims:\n"));
    for (name, value) in &report.weakest_dims {
        out.push_str(&format!("  - {name} = {value:.4}\n"));
    }
    out.push_str(&format!("\nWeakest {top} subs:\n"));
    for (name, value) in &report.weakest_subs {
        out.push_str(&format!("  - {name} = {value:.4}\n"));
    }
    out.push_str("\nSuggestions:\n");
    for s in &report.suggestions {
        out.push_str(&format!("  {s}\n"));
    }
    out
}

/// Dispatch `asi calibrate --dry-run|apply --every M` (round15-01 backend_engineer)
///
/// 在 repo 中取所有 trace 喂入 AdaptiveBaseline, 跑一次 LinearCalibration.compute,
/// 输出新系数 (dry-run) 或写入 scheduler.history (apply)。
pub fn dispatch_asi_calibrate(
    repo: &TraceRepository,
    mode: CalibrateMode,
    every: usize,
    scope: &str,
) -> String {
    let mut out = String::new();
    if repo.is_empty() {
        return "TraceRepository is empty. Append traces first.".to_string();
    }
    let mut baseline = AdaptiveBaseline::with_alpha(0.1);
    let history: Vec<DimensionTrace> = repo.tail(repo.len());
    baseline.observe_batch(&history);

    let cal = LinearCalibration::default();
    let coefs = cal.compute(&history, &[], &baseline, chrono::Utc::now().timestamp());

    // 用 RecalibrationScheduler 仅用于保持模式语义 + history 记录
    let mut sched = RecalibrationScheduler::with_every_n(every);
    let dry_run = mode == CalibrateMode::DryRun;
    let report = sched.run_with_history(
        &history,
        &baseline,
        &cal,
        chrono::Utc::now().timestamp(),
        dry_run,
        if dry_run { "cli-dry-run" } else { "cli-apply" },
    );

    out.push_str(&format!(
        "ASI calibrate (mode={}, every={}, scope={})\n",
        match mode {
            CalibrateMode::DryRun => "dry-run",
            CalibrateMode::Apply => "apply",
        },
        every,
        scope,
    ));
    out.push_str(&format!(
        "history_size={} feedback_count={} dry_run={} reason={}\n",
        report.history_size, report.feedback_count, report.dry_run, report.reason,
    ));
    out.push_str("\nNew coefficients:\n");
    out.push_str("V0.5 24 dims (scope filter applies):\n");
    for (i, name) in V05_DIMENSION_NAMES.iter().enumerate() {
        if scope != "all" && scope != "v05_dims" && scope != *name {
            continue;
        }
        let c = coefs.dims[i];
        if c.scale == 1.0 && c.offset == 0.0 {
            continue; // skip identity
        }
        out.push_str(&format!(
            "  {:<32} scale={:.4} offset={:.4}\n",
            name, c.scale, c.offset,
        ));
    }
    out.push_str("\nV1136 9 subs:\n");
    for (i, name) in apeireth_asi::V1136_SUBMEASURE_NAMES.iter().enumerate() {
        if scope != "all" && scope != "v1136_subs" && scope != *name {
            continue;
        }
        let c = coefs.subs[i];
        if c.scale == 1.0 && c.offset == 0.0 {
            continue;
        }
        out.push_str(&format!(
            "  {:<32} scale={:.4} offset={:.4}\n",
            name, c.scale, c.offset,
        ));
    }
    out.push_str(&format!(
        "\nscheduler.history.len() = {}\n",
        sched.history.len(),
    ));
    out
}
pub fn build_sample_measurement(rate: f64, n: u32) -> MeasurementSample {
    use apeireth_asi::V1136_SUBMEASURE_NAMES;
    let mut s = MeasurementSample::default();
    for name in V05_DIMENSION_NAMES.iter() {
        s.successes
            .insert((*name).to_string(), (rate * n as f64) as u32);
        s.attempts.insert((*name).to_string(), n);
        s.qualities.insert((*name).to_string(), 1.0);
    }
    for name in V1136_SUBMEASURE_NAMES.iter() {
        s.successes
            .entry((*name).to_string())
            .or_insert((rate * n as f64) as u32);
        s.attempts.entry((*name).to_string()).or_insert(n);
        s.qualities.entry((*name).to_string()).or_insert(1.0);
    }
    s.philosophy_gate_trials
        .insert("v1_pass_rate".into(), (8, 10));
    s.philosophy_gate_trials
        .insert("v2_pass_rate".into(), (7, 10));
    s.philosophy_gate_trials
        .insert("v3_pass_rate".into(), (9, 10));
    s.philosophy_gate_trials
        .insert("cone_of_truth_rate".into(), (10, 10));
    s.philosophy_gate_trials
        .insert("action_guard_rate".into(), (10, 10));
    s.philosophy_gate_trials
        .insert("v1_v2_pass_rate".into(), (15, 20));
    s.philosophy_gate_trials
        .insert("v3_action_guard_rate".into(), (19, 20));
    s
}

// ===================
// R16-09 聚合网关 dispatch (real apeireth-api integration)
// ===================

use apeireth_api::llm::{
    providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
    LlmProvider,
};
use std::sync::Arc;

/// 启动 apeireth-api HTTP server (默认端口 8080, 可用 APEIRETH_LLM_BACKEND=scripted|real 切换)
/// 阻塞直到 Ctrl-C
pub async fn dispatch_gateway_serve(port: u16) -> Result<String, String> {
    use apeireth_api::llm::{ApeirethApiConfig, ApeirethApiProvider};
    use apeireth_api::protocol_handlers;
    use apeireth_api::server::{build_router, AppState};

    let llm: Arc<dyn LlmProvider> = match std::env::var("APEIRETH_LLM_BACKEND").as_deref() {
        Ok("scripted") | Ok("mock") => {
            let scripted = ScriptedLlmProvider::new("scripted-mock")
                .with_script("hello", ScriptedResponse::new("hi from scripted"));
            Arc::new(scripted)
        }
        _ => {
            let llm_config = ApeirethApiConfig::from_env()
                .map_err(|e| format!("ApeirethApiConfig 初始化失败: {e}\n提示: 设 APEIRETH_LLM_BACKEND=scripted 跳过 LLM key 依赖"))?;
            let real = ApeirethApiProvider::new(llm_config)
                .map_err(|e| format!("ApeirethApiProvider 初始化失败: {e}"))?;
            Arc::new(real)
        }
    };

    // R17 战役 4-4 deploy glue: AppState 升级后需 pipeline + llm (战役 1-4 引入 pipeline 字段)
    // 用 protocol_handlers::build_pipeline 构造 4 协议管线 (战役 1-3 5 步 + 战役 1-2 Keep-Alive LIFO)
    let base_url = std::env::var("APEIRETH_API_URL")
        .unwrap_or_else(|_| protocol_handlers::MINIMAXI_BASE_URL.to_string());
    let auth_token = std::env::var("APEIRETH_API_KEY").ok();
    let pipeline = Arc::new(
        protocol_handlers::build_pipeline(base_url.clone(), auth_token.clone())
            .map_err(|e| format!("build_pipeline 失败: {e}"))?,
    );

    let state = Arc::new(AppState { pipeline, llm, response_cache: None });
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|e| format!("bind 0.0.0.0:{port} 失败: {e}"))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("local_addr: {e}"))?;
    let url = format!("http://{local_addr}");

    eprintln!("✅ apeireth-api server 启动, URL: {url}");
    eprintln!("   GET  /health");
    eprintln!("   POST /v1/chat/completions");
    eprintln!("   GET  /channels");
    eprintln!("   POST /council/advise");
    eprintln!("   POST /verdict");
    eprintln!("   (Ctrl-C 停止)");

    // 阻塞 axum::serve (直到 Ctrl-C 或错误)
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("server 错误: {e}"))?;

    Ok(format!("server stopped at {url}"))
}

// R17 砍掉: dispatch_gateway_status + dispatch_gateway_routes + gateway_dispatch_tests
//   原因: NewAPI channel 借鉴已砍 (gateway/ 整目录删), 列出 channels / 测试路由决策功能消失
//   用户用 apeireth-api 直连 LLM provider 即可, 不需要 channel 路由决策 cli 工具

#[cfg(test)]
mod asi_dispatch_tests {
    use super::*;

    fn populate_repo(n: usize) -> TraceRepository {
        let mut repo = TraceRepository::new();
        for i in 0..n {
            let sample = build_sample_measurement(0.5 + (i as f64 * 0.01), 10);
            let trace = DimensionTrace::from_sample(0, 0, 1_700_000_000 + i as i64, &sample, None);
            repo.append(trace);
        }
        repo
    }

    #[test]
    fn dispatch_trace_handles_empty() {
        let repo = TraceRepository::new();
        let out = dispatch_asi_trace(&repo, 5);
        assert!(out.contains("empty"));
    }

    #[test]
    fn dispatch_trace_returns_n_traces() {
        let repo = populate_repo(5);
        let out = dispatch_asi_trace(&repo, 3);
        assert!(out.contains("DimensionTrace"));
    }

    #[test]
    fn dispatch_trend_returns_sparkline() {
        let repo = populate_repo(10);
        let out = dispatch_asi_trend(&repo, "thread_continuity", 5);
        assert!(out.contains("Trend"));
        assert!(out.contains("thread_continuity"));
    }

    #[test]
    fn dispatch_trend_unknown_dim() {
        let repo = populate_repo(5);
        let out = dispatch_asi_trend(&repo, "not.a.real.dim", 3);
        assert!(out.contains("No history"));
    }

    #[test]
    fn dispatch_diagnose_returns_weakest() {
        let repo = populate_repo(3);
        let tail = repo.tail(1);
        let trace = &tail[0];
        let out = dispatch_asi_diagnose(trace, 3);
        assert!(out.contains("Diagnosis"));
        assert!(out.contains("Weakest"));
        assert!(out.contains("Suggestions"));
    }

    #[test]
    fn dispatch_diagnose_top_5() {
        let repo = populate_repo(2);
        let tail = repo.tail(1);
        let trace = &tail[0];
        let out = dispatch_asi_diagnose(trace, 5);
        assert!(out.contains("Weakest 5"));
    }

    #[test]
    fn build_sample_measurement_default_quality_1() {
        let s = build_sample_measurement(1.0, 10);
        for q in s.qualities.values() {
            assert!((*q - 1.0).abs() < 1e-9);
        }
    }
}
