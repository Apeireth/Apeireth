//! `apeireth-web` Memory UI 模块 (R18 sub-agent #1)
//!
//! 端到端真接通 apeireth-memory 后端:
//! - `GET  /memory`         → 列最近 20 条 episode + 当前 IdentityCard
//! - `POST /memory/append`  → form (role + content) append 一条 episode
//!
//! 共享 store: `OnceLock<Arc<SqliteMemoryStore>>`, 启动期 lazy init.
//!
//! DB 路径优先级:
//! 1. 环境变量 `APEIRETH_MEMORY_DB`
//! 2. fallback: `<workspace_root>/apeireth-memory.db`
//!    (`CARGO_MANIFEST_DIR/../apeireth-memory.db`)
//!
//! 不引入新依赖: 沿用 apeireth-api / axum / serde / tower-http (main.rs 同款).
//! 时间格式: Hinnant `civil_from_days` 算法 (epoch → Y-M-D), 不引 chrono.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use apeireth_core::Episode;
use apeireth_memory::{
    EpisodeQuery, EpisodeStore, IdentityCardStore, MemoryError, SqliteMemoryStore,
};
use axum::{
    extract::Form,
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::templates::{html_escape, render_error_page};

/// 固定的 web session_id (MVP 简化: 浏览器 form 都写到同一个 session).
/// 后面 R19+ 可以用 cookie 区分浏览器 / 用户.
pub const WEB_SESSION_ID: &str = "web-session";

/// 默认 IdentityCard continuity_id (append 之前若 store 里没 card, 自动 seed 一个).
pub const DEFAULT_CONTINUITY_ID: &str = "apeireth-web-default";

/// 查询时最多展示的 episode 条数.
const RECENT_EPISODES_LIMIT: usize = 20;

/// 共享 store 句柄. main 启动期 lazy init 一次, 所有 handler 复用.
static STORE: OnceLock<Arc<SqliteMemoryStore>> = OnceLock::new();

/// 拿到共享 store (首次调用时打开 DB). 失败时返回错误字符串, 方便 handler 渲 error 页.
pub fn get_store() -> Result<Arc<SqliteMemoryStore>, String> {
    if let Some(s) = STORE.get() {
        return Ok(Arc::clone(s));
    }
    let path = resolve_db_path();
    info!(db_path = %path.display(), "opening SqliteMemoryStore for apeireth-web");
    let store = SqliteMemoryStore::open(&path)
        .map_err(|e| format!("无法打开 memory store (path={}): {}", path.display(), e))?;

    // 第一次启动 seed 一个 IdentityCard (供 UI 显示主体信息), 不影响 episode 写入.
    if let Err(e) = seed_default_identity(&store) {
        warn!(error = %e, "seed default identity card failed (non-fatal)");
    }

    let arc = Arc::new(store);
    // STORE.set 可能 race: 别的线程已经 set 了, 那就丢掉我们的取他们的.
    let _ = STORE.set(Arc::clone(&arc));
    Ok(arc)
}

fn resolve_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("APEIRETH_MEMORY_DB") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    // fallback: <workspace_root>/apeireth-memory.db
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|p| p.join("apeireth-memory.db"))
        .unwrap_or_else(|| manifest_dir.join("apeireth-memory.db"))
}

fn seed_default_identity(store: &SqliteMemoryStore) -> Result<(), MemoryError> {
    use apeireth_core::IdentityCard;
    if store.exists(DEFAULT_CONTINUITY_ID)? {
        return Ok(());
    }
    let now = chrono_now_unix();
    let card = IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.to_string(),
        birth_time: now,
        carriers: vec!["apeireth-web".to_string()],
        migration_history: vec![],
    };
    store.create(&card)?;
    Ok(())
}

/// 拿当前 IdentityCard (取第一条 live 的, 或 fallback 到 seed 那个).
/// 返回 `None` 仅在 store 完全为空 (不应该发生, 因为 seed 过).
pub fn current_identity(store: &SqliteMemoryStore) -> Option<apeireth_core::IdentityCard> {
    // 优先: 显式查 default continuity_id
    if let Ok(Some(rec)) = store.get(DEFAULT_CONTINUITY_ID) {
        return Some(rec.into_core());
    }
    // fallback: 列表第一条 live card
    if let Ok(mut live) = store.list(false) {
        if let Some(rec) = live.pop() {
            return Some(rec.into_core());
        }
    }
    None
}

// ============================================================
// Handlers
// ============================================================

/// `GET /memory` — 渲染时间线 + IdentityCard.
pub async fn memory_page_handler() -> impl IntoResponse {
    let store = match get_store() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "memory store init failed");
            return Html(render_error_page(&e));
        }
    };

    // 拉最近 N 条 (按时间升序返回, 模板里我们 reverse 一下展示成"最新在上").
    let episodes: Vec<Episode> = match store.query(
        &EpisodeQuery::new()
            .for_session(WEB_SESSION_ID)
            .limit(RECENT_EPISODES_LIMIT),
    ) {
        Ok(v) => v,
        Err(e) => {
            return Html(render_error_page(&format!("读取 episode 失败: {e}")));
        }
    };

    let identity = current_identity(&store);

    Html(render_memory_page(&episodes, identity.as_ref()))
}

/// POST form payload.
#[derive(Debug, Deserialize)]
pub struct AppendForm {
    /// "system" / "user" / "assistant"
    pub role: String,
    pub content: String,
}

/// `POST /memory/append` — append 一条 episode, 然后 302 → `/memory`.
/// 用 `Result<Redirect, Html<String>>` 让 axum 统一处理成功 (302) 和错误 (200 错误页).
pub async fn memory_append_handler(
    Form(form): Form<AppendForm>,
) -> Result<axum::response::Redirect, Html<String>> {
    let role = form.role.trim().to_string();
    let content = form.content.trim().to_string();

    if role.is_empty() {
        return Err(Html(render_error_page("role 不能为空")));
    }
    if content.is_empty() {
        return Err(Html(render_error_page("content 不能为空")));
    }
    // 白名单 role, 防止脏数据
    if !matches!(role.as_str(), "system" | "user" | "assistant") {
        return Err(Html(render_error_page(
            "role 必须是 system / user / assistant",
        )));
    }

    let store = get_store().map_err(|e| Html(render_error_page(&e)))?;

    let now = chrono_now_unix();
    let ep = Episode {
        id: format!("ep-web-{}", now),
        timestamp: now,
        role: role.clone(),
        content: content.clone(),
        session_id: WEB_SESSION_ID.to_string(),
    };

    if let Err(e) = store.put_episode(&ep) {
        error!(error = %e, "put_episode failed");
        return Err(Html(render_error_page(&format!(
            "append episode 失败: {e}"
        ))));
    }

    info!(
        role = %role,
        id = %ep.id,
        "appended episode from web form"
    );

    // 302 重定向到 /memory (PRG 模式: 防止刷新重复提交)
    Ok(axum::response::Redirect::to("/memory"))
}

// ============================================================
// HTML 模板
// ============================================================

/// 渲染 Memory 页面.
pub fn render_memory_page(
    episodes: &[Episode],
    identity: Option<&apeireth_core::IdentityCard>,
) -> String {
    // 倒序: 最新在最上面
    let mut sorted: Vec<&Episode> = episodes.iter().collect();
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let mut episodes_html = String::new();
    if sorted.is_empty() {
        episodes_html.push_str(
            r#"<div class="memory-empty">
                <p>📭 暂无 episode. 用下面 form append 第一条吧.</p>
            </div>"#,
        );
    } else {
        for ep in &sorted {
            let role_label = match ep.role.as_str() {
                "user" => "👤 user",
                "assistant" => "🤖 assistant",
                "system" => "⚙️ system",
                other => other,
            };
            let ts = epoch_to_iso8601_utc(ep.timestamp);
            episodes_html.push_str(&format!(
                r#"<div class="memory-card">
                    <div class="memory-card-header">
                        <span class="memory-role memory-role-{role}">{role_label}</span>
                        <span class="memory-ts">{ts}</span>
                    </div>
                    <div class="memory-content">
                        <p>{content}</p>
                    </div>
                </div>"#,
                role = html_escape(&ep.role),
                role_label = html_escape(role_label),
                ts = html_escape(&ts),
                content = html_escape(&ep.content),
            ));
        }
    }

    let identity_panel = match identity {
        Some(card) => {
            let carriers = if card.carriers.is_empty() {
                "(无)".to_string()
            } else {
                card.carriers.join(", ")
            };
            let migrations = card.migration_history.len();
            format!(
                r#"<div class="memory-identity">
                <h2>🪪 当前 IdentityCard</h2>
                <dl class="memory-identity-fields">
                    <dt>continuity_id</dt>
                    <dd><code>{cid}</code></dd>
                    <dt>birth_time</dt>
                    <dd>{birth}</dd>
                    <dt>carriers</dt>
                    <dd>{carriers}</dd>
                    <dt>migration_history</dt>
                    <dd>{migrations} 条</dd>
                </dl>
            </div>"#,
                cid = html_escape(&card.continuity_id),
                birth = html_escape(&epoch_to_iso8601_utc(card.birth_time)),
                carriers = html_escape(&carriers),
                migrations = migrations,
            )
        }
        None => r#"<div class="memory-identity">
            <h2>🪪 当前 IdentityCard</h2>
            <p class="memory-empty">尚未 seed (store 应在启动时自动 seed default card).</p>
        </div>"#
            .to_string(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Memory — Episode 时间线</title>
    <meta name="description" content="apeireth-memory 真接通: append-only episode 流 + IdentityCard (R18 sub-agent #1)" />
    <style>
        .memory-app {{ max-width: 880px; margin: 0 auto; padding: 1.5rem; }}
        .memory-header h1 {{ margin: 0 0 .25rem 0; font-size: 1.6rem; }}
        .memory-header .tagline {{ color: #666; font-size: .95rem; margin: 0 0 1.5rem 0; }}
        .memory-identity {{
            border: 1px solid #d0d7de; border-radius: 8px; padding: 1rem 1.25rem;
            background: #f6f8fa; margin-bottom: 1.5rem;
        }}
        .memory-identity h2 {{ margin: 0 0 .75rem 0; font-size: 1.15rem; }}
        .memory-identity-fields {{ display: grid; grid-template-columns: 140px 1fr; gap: .35rem 1rem; margin: 0; }}
        .memory-identity-fields dt {{ color: #57606a; font-weight: 600; }}
        .memory-identity-fields dd {{ margin: 0; word-break: break-all; }}
        .memory-timeline h2 {{ font-size: 1.15rem; margin: 0 0 .75rem 0; }}
        .memory-card {{
            border: 1px solid #d0d7de; border-radius: 8px; padding: .75rem 1rem;
            margin-bottom: .75rem; background: #ffffff;
        }}
        .memory-card-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: .35rem; }}
        .memory-role {{ font-weight: 600; font-size: .9rem; }}
        .memory-role-user {{ color: #0969da; }}
        .memory-role-assistant {{ color: #1a7f37; }}
        .memory-role-system {{ color: #6e7781; }}
        .memory-ts {{ color: #6e7781; font-size: .8rem; font-family: ui-monospace, SFMono-Regular, monospace; }}
        .memory-content p {{ margin: 0; white-space: pre-wrap; word-break: break-word; }}
        .memory-empty {{ color: #6e7781; font-style: italic; }}
        .memory-form-wrap {{
            border: 1px solid #d0d7de; border-radius: 8px; padding: 1rem 1.25rem;
            background: #f6f8fa; margin-top: 1.5rem;
        }}
        .memory-form-wrap h2 {{ margin: 0 0 .75rem 0; font-size: 1.15rem; }}
        .memory-form label {{ display: block; margin-bottom: .75rem; }}
        .memory-form label > span {{ display: block; font-weight: 600; margin-bottom: .25rem; color: #57606a; font-size: .9rem; }}
        .memory-form select, .memory-form textarea {{
            width: 100%; padding: .5rem; border: 1px solid #d0d7de; border-radius: 6px;
            font: inherit; box-sizing: border-box;
        }}
        .memory-form textarea {{ min-height: 80px; resize: vertical; }}
        .memory-form button {{
            background: #1f883d; color: white; border: none; border-radius: 6px;
            padding: .5rem 1rem; font: inherit; font-weight: 600; cursor: pointer;
        }}
        .memory-form button:hover {{ background: #1a7f37; }}
        .memory-nav {{ margin-top: 1.5rem; }}
        .memory-nav a {{ color: #0969da; text-decoration: none; }}
        .memory-nav a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <main class="memory-app">
        <header class="memory-header">
            <h1>🧠 Apeireth Memory</h1>
            <p class="tagline">Episode 时间线 · append-only · 真接通 apeireth-memory (SqliteMemoryStore)</p>
        </header>

        {identity_panel}

        <section class="memory-timeline">
            <h2>📜 最近 {limit} 条 Episode (session: <code>{session}</code>)</h2>
            {episodes_html}
        </section>

        <div class="memory-form-wrap">
            <h2>✍️ Append 新 Episode</h2>
            <form class="memory-form" method="POST" action="/memory/append">
                <label>
                    <span>role</span>
                    <select name="role" required>
                        <option value="user" selected>user</option>
                        <option value="assistant">assistant</option>
                        <option value="system">system</option>
                    </select>
                </label>
                <label>
                    <span>content</span>
                    <textarea name="content" required placeholder="写一条 episode 进去..."></textarea>
                </label>
                <button type="submit">Append</button>
            </form>
        </div>

        <div class="memory-nav">
            <a href="/">← 返回 Council 首页</a>
        </div>
    </main>
</body>
</html>"#,
        identity_panel = identity_panel,
        limit = RECENT_EPISODES_LIMIT,
        session = html_escape(WEB_SESSION_ID),
        episodes_html = episodes_html,
    )
}

// ============================================================
// Helpers: epoch ↔ ISO-8601 (Hinnant civil_from_days)
// ============================================================

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

/// Unix epoch seconds (no chrono dep — 简单 wallclock-based 近似即可, Memory 只用来
/// 给 episode 一个单调递增 ID, 不要求绝对精确).
fn chrono_now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Howard Hinnant `civil_from_days` algorithm.
/// 输入: 自 1970-01-01 起的天数 (可负, 表示 1970 年之前).
/// 输出: (年, 月, 日) 公历日期.
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
    fn epoch_1970_01_01() {
        assert_eq!(epoch_to_iso8601_utc(0), "1970-01-01 00:00:00 UTC");
    }

    #[test]
    fn epoch_2000_01_01() {
        // 2000-01-01 00:00:00 UTC = 946684800
        assert_eq!(epoch_to_iso8601_utc(946_684_800), "2000-01-01 00:00:00 UTC");
    }

    #[test]
    fn epoch_2024_01_01() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(
            epoch_to_iso8601_utc(1_704_067_200),
            "2024-01-01 00:00:00 UTC"
        );
    }

    #[test]
    fn epoch_2025_12_31_23_59_59() {
        // 2025-12-31 23:59:59 UTC = 1767225599
        assert_eq!(
            epoch_to_iso8601_utc(1_767_225_599),
            "2025-12-31 23:59:59 UTC"
        );
    }

    #[test]
    fn epoch_handles_seconds_within_day() {
        // 2024-01-01 03:14:15 UTC = 1704067200 + 3*3600 + 14*60 + 15 = 1704078855
        assert_eq!(
            epoch_to_iso8601_utc(1_704_078_855),
            "2024-01-01 03:14:15 UTC"
        );
    }
}
