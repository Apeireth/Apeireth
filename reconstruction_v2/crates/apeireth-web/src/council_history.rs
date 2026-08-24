//! `council_history` — Council 辩论历史模块 (v2 reconstruction)
//!
//! **职责**:
//! - 列出 apeireth-memory 中所有 `role="council"` 的 Episode
//! - 手动保存当前辩论到 memory (POST /council/save)
//! - HTML 渲染历史列表
//!
//! **v2 适配 vs v1**:
//! - v1 依赖独立 `apeireth-memory::SqliteMemoryStore` (Arc<SqliteMemoryStore> 共享 state),
//!   v2 整合到 `apeireth-storage::memory_episode::InMemoryEpisodeStore`, 这里包成
//!   `Arc<Mutex<...>>` 共享 state (跟 memory.rs 同款).
//! - `Episode` 字段 (id/timestamp/role/content/session_id) 跟 v1 完全对齐.
//!
//! **存储约定** (跟 v1 保持一致):
//! - Episode `session_id` = `"council-history"` (单 session, 跨多次辩论)
//! - Episode `role` = `"council"` (跟 "user"/"assistant" 区分)
//! - Episode `content` = JSON 序列化的 `CouncilDebate`
//! - Episode `continuity_id` 不再需要 (v2 EpisodeQuery 也没这个字段)

use std::sync::{Arc, Mutex};

use crate::memory::WebMemoryStore;

#[cfg(feature = "ssr")]
use axum::{extract::State, response::Html, Json};
#[cfg(feature = "ssr")]
use serde::Deserialize;

use apeireth_storage::memory_episode::{Episode, EpisodeQuery, EpisodeStore, InMemoryEpisodeStore};

use crate::api::{AdvisorOpinion, CouncilAdviseResponse};
#[cfg(feature = "ssr")]
use crate::templates::html_escape;

/// Council 辩论历史记录 (序列化到 Episode.content)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CouncilDebate {
    /// 辩论唯一 ID
    pub id: String,
    /// 时间戳 (epoch seconds)
    pub timestamp: i64,
    /// 议题
    pub topic: String,
    /// 上下文 (可选)
    pub context: Option<String>,
    /// 使用的 LLM 协议 ("openai" / "anthropic")
    pub protocol: String,
    /// 7 advisor 意见
    pub advisors: Vec<AdvisorOpinion>,
    /// 最终 verdict
    pub verdict: String,
}

impl CouncilDebate {
    /// 从 `CouncilAdviseResponse` 构造 (辩论 ID 用 timestamp 字符串)
    pub fn from_response(
        resp: &CouncilAdviseResponse,
        context: Option<String>,
        debate_id: String,
        timestamp: i64,
    ) -> Self {
        Self {
            id: debate_id,
            timestamp,
            topic: resp.topic.clone(),
            context,
            protocol: resp.protocol.clone(),
            advisors: resp.advisors.clone(),
            verdict: resp.verdict.clone(),
        }
    }

    /// 反序列化 (Episode.content JSON → CouncilDebate)
    pub fn from_episode(ep: &Episode) -> Option<Self> {
        serde_json::from_str(&ep.content).ok()
    }

    /// 序列化为 JSON 字符串 (写到 Episode.content)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Council 历史共享 state (v2: Arc<Mutex<WebMemoryStore>>, 复用 memory.rs 的 web store)
#[derive(Clone)]
pub struct CouncilHistoryState {
    pub store: Arc<Mutex<WebMemoryStore>>,
}

impl CouncilHistoryState {
    /// 构造 (从已有 web store Arc)
    pub fn new(store: Arc<Mutex<WebMemoryStore>>) -> Self {
        Self { store }
    }
}

// ============================================================
// Handlers
// ============================================================

/// GET /council/history — 列出历史辩论
#[cfg(feature = "ssr")]
pub async fn get_history_handler(
    State(state): State<CouncilHistoryState>,
) -> Result<Html<String>, String> {
    // 查询所有 role="council" 的 Episode
    let q = EpisodeQuery::new()
        .for_session("council-history")
        .with_role("council")
        .limit(100);
    let episodes = {
        let web = state.store.lock().expect("council history mutex poisoned");
        web.episodes.query(&q)
    };

    // 按时间倒序 (新 → 旧)
    let mut debates: Vec<CouncilDebate> = episodes
        .iter()
        .filter_map(CouncilDebate::from_episode)
        .collect();
    debates.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(Html(render_council_history(&debates)))
}

/// POST /council/save — 手动保存辩论到 memory
#[cfg(feature = "ssr")]
#[derive(Debug, Deserialize)]
pub struct SaveRequest {
    pub topic: String,
    pub verdict: String,
    pub protocol: String,
    pub context: Option<String>,
    pub advisors: Vec<AdvisorOpinion>,
}

#[cfg(feature = "ssr")]
pub async fn post_save_handler(
    State(state): State<CouncilHistoryState>,
    Json(req): Json<SaveRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let timestamp = unix_now();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let debate_id = format!("council-{}-{}", timestamp, nanos);

    let debate = CouncilDebate {
        id: debate_id.clone(),
        timestamp,
        topic: req.topic.clone(),
        context: req.context,
        protocol: req.protocol.clone(),
        advisors: req.advisors,
        verdict: req.verdict.clone(),
    };

    let content = debate
        .to_json()
        .map_err(|e| format!("serialize debate: {e}"))?;

    let ep = Episode {
        id: debate_id.clone(),
        timestamp,
        role: "council".into(),
        content,
        session_id: "council-history".into(),
    };

    {
        let mut web = state.store.lock().expect("council history mutex poisoned");
        web.episodes.put_episode(&ep).map_err(|e| format!("save episode: {e}"))?;
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "debate_id": debate_id,
        "timestamp": timestamp,
    })))
}

// ============================================================
// HTML 渲染
// ============================================================

/// 渲染 Council 辩论历史列表
pub fn render_council_history(debates: &[CouncilDebate]) -> String {
    if debates.is_empty() {
        return render_empty_history();
    }

    let mut items_html = String::new();
    for d in debates {
        let verdict_class = match d.verdict.as_str() {
            "approved" => "verdict-approved",
            "rejected" => "verdict-rejected",
            _ => "verdict-review",
        };
        let verdict_label = match d.verdict.as_str() {
            "approved" => "✓ APPROVED",
            "rejected" => "✗ REJECTED",
            _ => "○ NEEDS REVIEW",
        };
        let protocol_label = match d.protocol.as_str() {
            "anthropic" => "🅰️ Anthropic",
            _ => "🅾️ OpenAI",
        };
        let ts_str = epoch_to_iso8601_utc(d.timestamp);

        let advisors_summary = d
            .advisors
            .iter()
            .map(|a| {
                let icon = match a.stance.as_str() {
                    "approve" => "✓",
                    "reject" => "✗",
                    _ => "○",
                };
                format!("{} {}", icon, html_escape(&a.domain))
            })
            .collect::<Vec<_>>()
            .join(" · ");

        items_html.push_str(&format!(
            r#"<li class="history-item">
                <div class="history-row">
                    <span class="history-ts">{ts}</span>
                    <span class="history-verdict {verdict_class}">{verdict_label}</span>
                    <span class="history-protocol">{protocol_label}</span>
                    <span class="history-advisors">{advisors_count} advisors</span>
                </div>
                <div class="history-topic">
                    <span class="history-topic-label">议题:</span>
                    <span class="history-topic-text">{topic}</span>
                </div>
                <div class="history-advisor-summary">{advisors_summary}</div>
                <details class="history-details">
                    <summary>查看完整 7 advisor 推理</summary>
                    <div class="history-advisors-full">
                        {advisors_full}
                    </div>
                </details>
            </li>"#,
            ts = html_escape(&ts_str),
            verdict_class = verdict_class,
            verdict_label = verdict_label,
            protocol_label = protocol_label,
            advisors_count = d.advisors.len(),
            topic = html_escape(&d.topic),
            advisors_summary = advisors_summary,
            advisors_full = render_history_advisors(&d.advisors)
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Council — 辩论历史</title>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Council 辩论历史</h1>
            <p class="apeireth-tagline">{count} 条已存辩论 · 来自 apeireth-storage::memory_episode (v2)</p>
        </header>

        <ul class="history-list">
            {items}
        </ul>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"#,
        count = debates.len(),
        items = items_html
    )
}

fn render_empty_history() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Council — 辩论历史</title>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Council 辩论历史</h1>
            <p class="apeireth-tagline">0 条辩论</p>
        </header>

        <div class="apeireth-info">
            <p>📭 暂无历史辩论. 回到首页跑一次 Council 7 advisor, 完成后会自动存到这里.</p>
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/">← 返回首页</a>
        </div>
    </main>
</body>
</html>"#
    )
}

fn render_history_advisors(advisors: &[AdvisorOpinion]) -> String {
    let mut out = String::new();
    for adv in advisors {
        let (stance_class, stance_label) = match adv.stance.as_str() {
            "approve" => ("stance-approve", "✓ 赞成"),
            "reject" => ("stance-reject", "✗ 反对"),
            _ => ("stance-neutral", "○ 中立"),
        };
        out.push_str(&format!(
            r#"<div class="council-card">
                <div class="council-card-header">
                    <span class="council-domain">{domain}</span>
                    <span class="council-stance {stance_class}">{stance_label}</span>
                </div>
                <div class="council-reasoning">
                    <p>{reasoning}</p>
                </div>
            </div>"#,
            domain = html_escape(&adv.domain),
            stance_class = stance_class,
            stance_label = stance_label,
            reasoning = html_escape(&adv.reasoning)
        ));
    }
    out
}

// ============================================================
// Timestamp helpers (stdlib-only, 不引 chrono / uuid)
// ============================================================

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Unix epoch seconds → `YYYY-MM-DD HH:MM:SS UTC` 字符串.
fn epoch_to_iso8601_utc(epoch: i64) -> String {
    let secs_per_day = 86_400_i64;
    let days = epoch.div_euclid(secs_per_day);
    let secs_in_day = epoch.rem_euclid(secs_per_day);
    let hour = secs_in_day / 3600;
    let min = (secs_in_day % 3600) / 60;
    let sec = secs_in_day % 60;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        y, m, d, hour, min, sec
    )
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y_tmp = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y_tmp + 1 } else { y_tmp } as i32;
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn council_debate_roundtrip_json() {
        let debate = CouncilDebate {
            id: "d1".into(),
            timestamp: 100,
            topic: "t".into(),
            context: Some("c".into()),
            protocol: "openai".into(),
            advisors: vec![AdvisorOpinion { domain: "safety".into(), stance: "approve".into(), reasoning: "ok".into() }],
            verdict: "approved".into(),
        };
        let json = debate.to_json().unwrap();
        let back: CouncilDebate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "d1");
        assert_eq!(back.topic, "t");
        assert_eq!(back.advisors.len(), 1);
    }

    #[test]
    fn council_debate_from_response() {
        let resp = CouncilAdviseResponse {
            topic: "topic-x".into(),
            status: "ok".into(),
            advisors: vec![],
            verdict: "approved".into(),
            protocol: "openai".into(),
            debate_id: None,
        };
        let d = CouncilDebate::from_response(&resp, Some("ctx".into()), "d2".into(), 200);
        assert_eq!(d.id, "d2");
        assert_eq!(d.timestamp, 200);
        assert_eq!(d.topic, "topic-x");
        assert_eq!(d.context.as_deref(), Some("ctx"));
    }

    #[test]
    fn council_debate_from_episode() {
        let ep = Episode {
            id: "d3".into(),
            timestamp: 300,
            role: "council".into(),
            content: r#"{"id":"d3","timestamp":300,"topic":"t","protocol":"openai","advisors":[],"verdict":"approved"}"#.into(),
            session_id: "council-history".into(),
        };
        let d = CouncilDebate::from_episode(&ep).unwrap();
        assert_eq!(d.id, "d3");
        assert_eq!(d.verdict, "approved");
    }

    #[test]
    fn render_history_with_debates() {
        let debates = vec![CouncilDebate {
            id: "d".into(),
            timestamp: 1_704_067_200,
            topic: "x".into(),
            context: None,
            protocol: "openai".into(),
            advisors: vec![AdvisorOpinion { domain: "safety".into(), stance: "approve".into(), reasoning: "ok".into() }],
            verdict: "approved".into(),
        }];
        let html = render_council_history(&debates);
        assert!(html.contains("Council 辩论历史"));
        assert!(html.contains("APPROVED"));
        assert!(html.contains("safety"));
        assert!(html.contains("1 条已存辩论"));
    }

    #[test]
    fn render_empty_history_page() {
        let html = render_council_history(&[]);
        assert!(html.contains("0 条辩论"));
        assert!(html.contains("📭"));
    }

    #[test]
    fn epoch_to_iso_roundtrip() {
        assert_eq!(epoch_to_iso8601_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(epoch_to_iso8601_utc(1_704_067_200), "2024-01-01 00:00:00 UTC");
    }
}
