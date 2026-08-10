//! `apeireth-web/sovereignty.rs` — Self-Disable 5 大机制控制台 (R18 sub-agent #3)
//!
//! **端到端接通**: 真用 `apeireth_sovereignty::self_disable::SelfDisableGuard`,
//! 跟 `examples/permission_effect_demo.rs` 同样的 5 个 check_* 接口 (round8-06 深度实装).
//!
//! **路由** (在 main.rs 注册):
//! - GET  /sovereignty           — 5 大机制状态卡片仪表盘
//! - POST /sovereignty/attack    — 表单: attack_type, 触发攻击
//! - POST /sovereignty/rearm     — 尝试 rearm (按 NoReverse 机制, 必然触发 NoReverse)
//!
//! **状态保持**:
//! - `SOVEREIGNTY_STATE: OnceLock<Arc<Mutex<SovereigntyState>>>` — 全局单例
//! - 跨请求共享, 启动期 init 一次, 之后所有 handler 拿到同一 Arc
//!
//! **架构位置**:
//! ```text
//!   HTTP request
//!      ↓ (axum handler)
//!   sovereignty_dashboard_handler / sovereignty_attack_handler / sovereignty_rearm_handler
//!      ↓
//!   SovereigntyState::attack / attempt_rearm / snapshot
//!      ↓
//!   SelfDisableGuard::check_no_* (apeireth-sovereignty)
//! ```

use std::sync::{Arc, Mutex, OnceLock};

#[cfg(feature = "ssr")]
use apeireth_sovereignty::self_disable::{
    SelfDisableCheck, SelfDisableGuard, SelfDisableRecord, SelfDisableSignal,
};

#[cfg(feature = "ssr")]
use axum::{
    extract::Form,
    response::{Html, IntoResponse},
};
#[cfg(feature = "ssr")]
use serde::Deserialize;

use crate::templates::{html_escape, render_error_page};

// ============================================================
// 5 大机制元数据 (编译时硬编码 — 跟 SelfDisableGuard.mechanism_id 对齐)
// ============================================================

/// `(mechanism_id, english_name, chinese_name)` — 跟 self_disable.rs 完全一致
const MECHANISMS: &[(u8, &str, &str)] = &[
    (1, "NoDegrade", "不可降级"),
    (2, "NoPatch", "不可patch"),
    (3, "NoBypass", "不可绕过"),
    (4, "NoReverse", "不可逆转"),
    (5, "NoHide", "不可隐藏"),
];

// ============================================================
// Attack 类型
// ============================================================

/// 5 种攻击类型 — 决定调用哪个 check_* 函数
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    /// 1 — 不可降级: high → low 风险降级
    Downgrade,
    /// 2 — 不可 patch: 试图改 5 哲学键 hardcode
    Patch,
    /// 3 — 不可绕过: Master token 绕过 governance
    Bypass,
    /// 4 — 不可逆转: 撤销已触发的 self-disable
    Reverse,
    /// 5 — 不可隐藏: 清空 audit 记录
    Hide,
}

impl AttackType {
    /// 从字符串解析 (web form 用)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "downgrade" | "no_degrade" => Some(Self::Downgrade),
            "patch" | "no_patch" => Some(Self::Patch),
            "bypass" | "no_bypass" => Some(Self::Bypass),
            "reverse" | "no_reverse" => Some(Self::Reverse),
            "hide" | "no_hide" => Some(Self::Hide),
            _ => None,
        }
    }

    /// 机制 ID (1-5), 跟 SelfDisableTrigger.mechanism_id 对齐
    pub fn mechanism_id(&self) -> u8 {
        match self {
            Self::Downgrade => 1,
            Self::Patch => 2,
            Self::Bypass => 3,
            Self::Reverse => 4,
            Self::Hide => 5,
        }
    }

    /// 下拉框显示的中文 label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Downgrade => "降级风险 (high → low)",
            Self::Patch => "patch hardcode (principle_keys_count)",
            Self::Bypass => "Master token 绕过 governance",
            Self::Reverse => "撤销已触发的 self-disable",
            Self::Hide => "清空 audit 记录",
        }
    }
}

// ============================================================
// 状态快照 (用于 HTML 渲染, 不暴露内部字段)
// ============================================================

/// 单条触发记录的展示快照
#[derive(Debug, Clone)]
pub struct RecordSnapshot {
    pub trigger_id: String,
    pub mechanism_id: u8,
    pub mechanism_name: &'static str,
    pub chinese_name: &'static str,
    pub context: String,
    pub triggered_at_ms: i64,
}

/// 仪表盘状态快照
#[derive(Debug, Clone)]
pub struct SovereigntyStateSnapshot {
    /// 实际 guard.is_armed (技术态)
    pub is_armed: bool,
    /// 是否曾经触发过 (UI 态: 任一记录存在即视为"disarmed")
    pub has_any_record: bool,
    pub total_records: usize,
    /// [NoDegrade, NoPatch, NoBypass, NoReverse, NoHide] 触发次数
    pub mechanism_counts: [usize; 5],
    pub recent_records: Vec<RecordSnapshot>,
}

// ============================================================
// 全局状态
// ============================================================

/// Sovereignty 全局状态 — 内部包一个 SelfDisableGuard
pub struct SovereigntyState {
    guard: SelfDisableGuard,
}

impl SovereigntyState {
    pub fn new() -> Self {
        Self {
            guard: SelfDisableGuard::new(),
        }
    }

    /// 拍快照 (供 handler 渲染用)
    pub fn snapshot(&self) -> SovereigntyStateSnapshot {
        let mut counts = [0usize; 5];
        for r in self.guard.records() {
            let id = r.trigger.mechanism_id();
            if (1..=5).contains(&id) {
                counts[(id - 1) as usize] += 1;
            }
        }
        // 最近 10 条 (倒序)
        let recent: Vec<RecordSnapshot> = self
            .guard
            .records()
            .iter()
            .rev()
            .take(10)
            .map(record_to_snapshot)
            .collect();
        SovereigntyStateSnapshot {
            is_armed: self.guard.is_armed,
            has_any_record: !self.guard.records().is_empty(),
            total_records: self.guard.record_count(),
            mechanism_counts: counts,
            recent_records: recent,
        }
    }

    /// 触发攻击 — 通过 full_check 走 5 大机制
    /// (跟 permission_effect_demo.rs 同样的调用模式)
    pub fn attack(&mut self, attack_type: AttackType, context: &str) -> SelfDisableCheck {
        let now_ms = current_time_ms();
        let signal = match attack_type {
            AttackType::Downgrade => SelfDisableSignal::NoDegrade {
                original: "high".into(),
                proposed: "low".into(),
                context: context.into(),
            },
            AttackType::Patch => SelfDisableSignal::NoPatch {
                rule: "principle_keys_count".into(),
                value: 3,
                context: context.into(),
            },
            AttackType::Bypass => SelfDisableSignal::NoBypass {
                owner_token: "Master".into(),
                bypassed_governance: true,
                context: context.into(),
            },
            AttackType::Reverse => {
                // 撤销目标 = 最近一条记录的 id (如果没记录, 用占位符,
                // check_no_reverse 反正都会触发 NoReverse)
                let target_id = self
                    .guard
                    .records()
                    .last()
                    .map(|r| r.trigger_id.clone())
                    .unwrap_or_else(|| "sd-000000".to_string());
                SelfDisableSignal::NoReverse {
                    trigger_id: target_id,
                    context: context.into(),
                }
            }
            AttackType::Hide => SelfDisableSignal::NoHide {
                window_id: format!("audit-window-{}", now_ms),
                context: context.into(),
            },
        };
        self.guard.full_check(&signal, now_ms)
    }

    /// Rearm 尝试 — 按 NoReverse 机制, 任何 rearm 都触发 NoReverse
    /// ("把笼子的钥匙扔掉" — 单向门)
    pub fn attempt_rearm(&mut self) -> SelfDisableCheck {
        let now_ms = current_time_ms();
        let target_id = self
            .guard
            .records()
            .last()
            .map(|r| r.trigger_id.clone())
            .unwrap_or_else(|| "sd-000000".to_string());
        self.guard.check_no_reverse(
            &target_id,
            "operator attempted to rearm via /sovereignty/rearm (forbidden by NoReverse)",
            now_ms,
        )
    }
}

fn record_to_snapshot(r: &SelfDisableRecord) -> RecordSnapshot {
    RecordSnapshot {
        trigger_id: r.trigger_id.clone(),
        mechanism_id: r.trigger.mechanism_id(),
        mechanism_name: r.trigger.mechanism_name(),
        chinese_name: r.trigger.chinese_name(),
        context: r.context.clone(),
        triggered_at_ms: r.triggered_at_ms,
    }
}

impl Default for SovereigntyState {
    fn default() -> Self {
        Self::new()
    }
}

fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 全局单例 — `OnceLock` 启动期 init, 之后所有 handler 共享
static SOVEREIGNTY_STATE: OnceLock<Arc<Mutex<SovereigntyState>>> = OnceLock::new();

/// 获取全局状态的 Arc 克隆
pub fn sovereignty_state() -> Arc<Mutex<SovereigntyState>> {
    SOVEREIGNTY_STATE
        .get_or_init(|| Arc::new(Mutex::new(SovereigntyState::new())))
        .clone()
}

// ============================================================
// SSR Handlers
// ============================================================

#[cfg(feature = "ssr")]
#[derive(Debug, Deserialize)]
pub struct AttackForm {
    pub attack_type: String,
}

/// GET /sovereignty — 仪表盘
#[cfg(feature = "ssr")]
pub async fn sovereignty_dashboard_handler() -> impl IntoResponse {
    let state = sovereignty_state();
    let snapshot = {
        let guard = state.lock().expect("sovereignty mutex poisoned");
        guard.snapshot()
    };
    Html(render_sovereignty_dashboard(&snapshot))
}

/// POST /sovereignty/attack — 触发攻击
#[cfg(feature = "ssr")]
pub async fn sovereignty_attack_handler(Form(form): Form<AttackForm>) -> impl IntoResponse {
    let attack_type = match AttackType::from_str(&form.attack_type) {
        Some(t) => t,
        None => {
            return Html(render_error_page(&format!(
                "未知 attack_type: {} (合法值: downgrade/patch/bypass/reverse/hide)",
                form.attack_type
            )));
        }
    };

    let state = sovereignty_state();
    let result = {
        let mut guard = state.lock().expect("sovereignty mutex poisoned");
        guard.attack(attack_type, &format!("web attack: {}", attack_type.label()))
    };

    let msg = match &result {
        SelfDisableCheck::Pass => format!(
            "⚠️ Attack [{}] 未触发 (守门可能未 armed, 不应发生)",
            attack_type.label()
        ),
        SelfDisableCheck::Triggered(rec) => format!(
            "✓ Attack [{}] 触发 {} (id={}, 上下文: {})",
            attack_type.label(),
            rec.trigger.chinese_name(),
            rec.trigger_id,
            rec.context
        ),
    };

    Html(render_action_result(&msg, "Attack"))
}

/// POST /sovereignty/rearm — 尝试 rearm, 必然被 NoReverse 拦截
#[cfg(feature = "ssr")]
pub async fn sovereignty_rearm_handler() -> impl IntoResponse {
    let state = sovereignty_state();
    let result = {
        let mut guard = state.lock().expect("sovereignty mutex poisoned");
        guard.attempt_rearm()
    };

    let msg = match &result {
        SelfDisableCheck::Pass => "⚠️ Rearm 未触发 (异常, NoReverse 应永远 active)".to_string(),
        SelfDisableCheck::Triggered(rec) => format!(
            "✗ Rearm 被 {} 拒绝 (id={}) — 单向门, 不可逆转",
            rec.trigger.chinese_name(),
            rec.trigger_id
        ),
    };

    Html(render_action_result(&msg, "Rearm"))
}

// ============================================================
// HTML 渲染
// ============================================================

/// 5 大机制仪表盘
#[cfg(feature = "ssr")]
pub fn render_sovereignty_dashboard(state: &SovereigntyStateSnapshot) -> String {
    // 卡片 HTML
    let mut cards_html = String::new();
    for (idx, (id, en, zh)) in MECHANISMS.iter().enumerate() {
        let count = state.mechanism_counts[idx];
        // disarmed = 该机制有过记录
        let is_disarmed = count > 0;
        let (badge_class, badge_label) = if is_disarmed {
            ("sd-card-badge sd-badge-disarmed", "✗ disarmed")
        } else {
            ("sd-card-badge sd-badge-armed", "✓ armed")
        };
        let card_class = if is_disarmed {
            "sd-card sd-card-disarmed"
        } else {
            "sd-card sd-card-armed"
        };

        cards_html.push_str(&format!(
            r#"<div class="{card_class}">
                <div class="sd-card-header">
                    <span class="sd-card-id">#{id}</span>
                    <span class="sd-card-zh">{zh}</span>
                </div>
                <div class="sd-card-en">{en}</div>
                <div class="{badge_class}">{badge_label}</div>
                <div class="sd-card-count">
                    触发次数: <strong>{count}</strong>
                </div>
            </div>"#,
            card_class = card_class,
            id = id,
            zh = html_escape(zh),
            en = html_escape(en),
            badge_class = badge_class,
            badge_label = badge_label,
            count = count
        ));
    }

    // 全局状态条
    let (global_class, global_label) = if state.has_any_record {
        (
            "sd-global sd-global-disarmed",
            "✗ GLOBAL STATE: DISARMED (5 大机制已被触发)",
        )
    } else {
        (
            "sd-global sd-global-armed",
            "✓ GLOBAL STATE: ARMED (5 大机制全部就位)",
        )
    };

    // 真实 guard.is_armed
    let guard_armed_str = if state.is_armed {
        "<span class='sd-true'>true</span>"
    } else {
        "<span class='sd-false'>false</span>"
    };

    // Recent records
    let recent_html = if state.recent_records.is_empty() {
        r#"<p class="sd-recent-empty">(暂无触发记录 — 5 大机制待命)</p>"#.to_string()
    } else {
        let mut rows = String::new();
        for r in &state.recent_records {
            rows.push_str(&format!(
                r#"<tr>
                    <td><code>{id}</code></td>
                    <td>#{mid} {mzh}</td>
                    <td>{ctx}</td>
                </tr>"#,
                id = html_escape(&r.trigger_id),
                mid = r.mechanism_id,
                mzh = html_escape(r.chinese_name),
                ctx = html_escape(&r.context)
            ));
        }
        format!(
            r#"<table class="sd-recent-table">
                <thead>
                    <tr><th>trigger_id</th><th>机制</th><th>上下文</th></tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>"#
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Sovereignty — Self-Disable 5 大机制控制台</title>
    <meta name="description" content="Apeireth Sovereignty 控制台 — 端到端接通 apeireth-sovereignty, 触发并观察 5 大 Self-Disable 机制" />
    <style>
        .sd-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; margin: 1.5rem 0; }}
        .sd-card {{ background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 1.25rem; backdrop-filter: blur(10px); }}
        .sd-card-armed {{ border-color: rgba(0, 200, 100, 0.4); }}
        .sd-card-disarmed {{ border-color: rgba(220, 60, 60, 0.7); background: rgba(220, 60, 60, 0.08); }}
        .sd-card-header {{ display: flex; align-items: baseline; gap: 0.5rem; margin-bottom: 0.5rem; }}
        .sd-card-id {{ color: #a0a0c0; font-family: monospace; font-size: 0.9rem; }}
        .sd-card-zh {{ font-size: 1.2rem; font-weight: 600; color: #e0e0e0; }}
        .sd-card-en {{ font-family: monospace; color: #888; font-size: 0.95rem; margin-bottom: 0.75rem; }}
        .sd-card-badge {{ display: inline-block; padding: 0.25rem 0.75rem; border-radius: 6px; font-weight: 600; font-size: 0.9rem; margin-bottom: 0.75rem; }}
        .sd-badge-armed {{ background: rgba(0, 200, 100, 0.2); color: #4eff8a; }}
        .sd-badge-disarmed {{ background: rgba(220, 60, 60, 0.25); color: #ff7a7a; }}
        .sd-card-count {{ color: #c0c0c0; font-size: 0.95rem; }}
        .sd-card-count strong {{ color: #fff; font-size: 1.1rem; }}
        .sd-global {{ padding: 1rem 1.5rem; border-radius: 10px; margin-bottom: 1.5rem; font-weight: 600; text-align: center; }}
        .sd-global-armed {{ background: rgba(0, 200, 100, 0.15); color: #4eff8a; border: 1px solid rgba(0, 200, 100, 0.4); }}
        .sd-global-disarmed {{ background: rgba(220, 60, 60, 0.15); color: #ff7a7a; border: 1px solid rgba(220, 60, 60, 0.5); }}
        .sd-section {{ background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 1.5rem; margin-bottom: 1.5rem; backdrop-filter: blur(10px); }}
        .sd-section h2 {{ color: #e0e0e0; margin-bottom: 1rem; font-size: 1.3rem; }}
        .sd-form-row {{ display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap; }}
        .sd-form-row label {{ display: flex; flex-direction: column; flex: 1; min-width: 240px; }}
        .sd-form-row label > span {{ color: #a0a0c0; font-size: 0.9rem; margin-bottom: 0.5rem; }}
        .sd-form-row select, .sd-form-row input {{ background: rgba(0,0,0,0.3); color: #e0e0e0; border: 1px solid rgba(255,255,255,0.2); border-radius: 6px; padding: 0.5rem 0.75rem; font-size: 1rem; font-family: inherit; }}
        .sd-rearm-form {{ margin-top: 1rem; }}
        .sd-rearm-btn {{ background: rgba(220, 60, 60, 0.2); color: #ff7a7a; border: 1px solid rgba(220, 60, 60, 0.6); border-radius: 6px; padding: 0.75rem 1.5rem; font-size: 1rem; font-weight: 600; cursor: pointer; font-family: inherit; }}
        .sd-rearm-btn:hover {{ background: rgba(220, 60, 60, 0.35); }}
        .sd-recent-table {{ width: 100%; border-collapse: collapse; margin-top: 0.5rem; }}
        .sd-recent-table th, .sd-recent-table td {{ padding: 0.5rem 0.75rem; text-align: left; border-bottom: 1px solid rgba(255,255,255,0.08); }}
        .sd-recent-table th {{ color: #a0a0c0; font-weight: 600; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.05em; }}
        .sd-recent-table td {{ color: #d0d0d0; font-size: 0.95rem; }}
        .sd-recent-table code {{ background: rgba(0,0,0,0.3); padding: 0.1rem 0.4rem; border-radius: 4px; color: #ffce5c; }}
        .sd-recent-empty {{ color: #888; font-style: italic; }}
        .sd-meta {{ color: #888; font-size: 0.9rem; margin-top: 0.5rem; }}
        .sd-true {{ color: #4eff8a; font-weight: 600; }}
        .sd-false {{ color: #ff7a7a; font-weight: 600; }}
    </style>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth Sovereignty 控制台</h1>
            <p class="apeireth-tagline">Self-Disable 5 大机制 · 端到端接通 apeireth-sovereignty</p>
        </header>

        <div class="{global_class}">
            {global_label}
        </div>

        <div class="sd-section">
            <h2>🛡️ 5 大 Self-Disable 机制</h2>
            <p class="sd-meta">guard.is_armed (技术态): {guard_armed_str} · 触发记录总数: <strong>{total}</strong></p>
            <div class="sd-grid">
                {cards}
            </div>
        </div>

        <div class="sd-section">
            <h2>⚔️ 攻击测试 — 触发守门机制</h2>
            {attack_form}
        </div>

        <div class="sd-section">
            <h2>🔒 Rearm 尝试 (被 NoReverse 拦截)</h2>
            <p class="sd-meta">"把笼子的钥匙扔掉" — Self-Disable 是单向门, 任何 rearm 尝试都触发 NoReverse 机制 (id 自动记录到 audit).</p>
            <form class="sd-rearm-form" method="POST" action="/sovereignty/rearm">
                <button class="sd-rearm-btn" type="submit">Rearm (被 NoReverse 拒绝)</button>
            </form>
        </div>

        <div class="sd-section">
            <h2>📜 最近触发记录 (最近 10 条, append-only)</h2>
            {recent_html}
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"#,
        global_class = global_class,
        global_label = global_label,
        guard_armed_str = guard_armed_str,
        total = state.total_records,
        cards = cards_html,
        attack_form = render_attack_form(),
        recent_html = recent_html
    )
}

/// 攻击 form (独立渲染, 方便单独调用)
#[cfg(feature = "ssr")]
pub fn render_attack_form() -> String {
    format!(
        r#"<form class="sd-form-row" method="POST" action="/sovereignty/attack">
            <label>
                <span>选择攻击类型</span>
                <select name="attack_type" required>
                    {options}
                </select>
            </label>
            <button class="apeireth-button" type="submit">🚀 触发攻击</button>
        </form>"#,
        options = render_attack_options()
    )
}

/// 攻击类型下拉框 options
#[cfg(feature = "ssr")]
fn render_attack_options() -> String {
    let opts = [
        AttackType::Downgrade,
        AttackType::Patch,
        AttackType::Bypass,
        AttackType::Reverse,
        AttackType::Hide,
    ];
    let mut s = String::new();
    for at in &opts {
        s.push_str(&format!(
            r#"<option value="{key}">{label}</option>"#,
            key = attack_type_key(*at),
            label = at.label()
        ));
    }
    s
}

/// AttackType → form value key
fn attack_type_key(at: AttackType) -> &'static str {
    match at {
        AttackType::Downgrade => "downgrade",
        AttackType::Patch => "patch",
        AttackType::Bypass => "bypass",
        AttackType::Reverse => "reverse",
        AttackType::Hide => "hide",
    }
}

/// 攻击 / rearm 操作后的跳转页 (2 秒后回 dashboard)
#[cfg(feature = "ssr")]
fn render_action_result(msg: &str, action: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta http-equiv="refresh" content="2;url=/sovereignty" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>{action} — 结果</title>
    <style>
        .sd-result {{ background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 2rem; text-align: center; backdrop-filter: blur(10px); }}
        .sd-result h1 {{ color: #e0e0e0; margin-bottom: 1.5rem; }}
        .sd-result p {{ color: #d0d0d0; font-size: 1.1rem; line-height: 1.6; margin-bottom: 1.5rem; }}
    </style>
</head>
<body>
    <main class="apeireth-app">
        <div class="sd-result">
            <h1>{action} 完成</h1>
            <p>{msg}</p>
            <p class="apeireth-meta">2 秒后自动跳回控制台, 或 <a href="/sovereignty">点这里立即返回</a></p>
        </div>
    </main>
</body>
</html>"#,
        action = html_escape(action),
        msg = html_escape(msg)
    )
}

// ============================================================
// 非 SSR build 时的 stub — axum 不可用, 只占位
// ============================================================

// 状态/枚举/快照类型在 cfg 之外, 跨 build 模式共享.
// handlers 和 html 渲染都是 SSR-only (非 SSR 时占位).

#[cfg(not(feature = "ssr"))]
pub async fn sovereignty_dashboard_handler() {
    // 非 SSR: 不应被调用, 占位让 lib 编译通过
}

#[cfg(not(feature = "ssr"))]
pub async fn sovereignty_attack_handler() {}

#[cfg(not(feature = "ssr"))]
pub async fn sovereignty_rearm_handler() {}

#[cfg(not(feature = "ssr"))]
pub fn render_sovereignty_dashboard(_state: &SovereigntyStateSnapshot) -> String {
    String::new()
}

#[cfg(not(feature = "ssr"))]
pub fn render_attack_form() -> String {
    String::new()
}
