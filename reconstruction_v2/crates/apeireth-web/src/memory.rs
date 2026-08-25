//! `apeireth-web` Memory UI 模块 (v2 reconstruction)
//!
//! 端到端接通 `apeireth-storage::memory_episode` 后端:
//! - `GET  /memory`         → 列最近 20 条 episode + 当前 IdentityCard
//! - `POST /memory/append`  → form (role + content) append 一条 episode
//!
//! **v2 适配 vs v1**:
//! - v1 依赖独立 `apeireth-memory` crate (SqliteMemoryStore + 真 SQLite); v2 没有该 crate,
//!   memory 子系统整合到 `apeireth-storage::memory_episode`. `Episode` 字段
//!   (id / timestamp / role / content / session_id) 跟 v1 完全对齐, 直接复用.
//! - v1 IdentityCard 字段 (continuity_id / birth_time / carriers / migration_history) 在 v2 core
//!   重构后改成 name / version / philosophy_anchors / created_at_datetime — 字段不兼容.
//!   web 这一层保留 v1 字段语义, 在本文件内本地定义 \`IdentityCard\` + 简单 in-memory store,
//!   不依赖 core. (后续可替换为 storage::memory_identity::UserIdentity, 字段不同需做映射.)
//!
//! **存储后端**:
//! - \`WebMemoryStore\` 内部包 \`InMemoryEpisodeStore\` (storage::memory_episode) + \`IdentityRegistry\` (HashMap)
//! - \`STORE: OnceLock<Arc<Mutex<WebMemoryStore>>>\` 全局单例, 启动期 lazy init.
//!
//! 不引入新依赖: 沿用 axum / serde / tracing (跟 main.rs 同款).
//! 时间格式: Hinnant `civil_from_days` 算法 (epoch → Y-M-D), 不引 chrono.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(feature = "ssr")]
use axum::{
    extract::Form,
    response::{Html, IntoResponse},
};
#[cfg(feature = "ssr")]
use tracing::{error, info, warn};

use apeireth_storage::memory_episode::{Episode, EpisodeQuery, EpisodeStore, InMemoryEpisodeStore};

#[cfg(feature = "ssr")]
use crate::templates::{html_escape, render_error_page};

/// 固定的 web session_id (MVP 简化: 浏览器 form 都写到同一个 session).
/// 后面 R19+ 可以用 cookie 区分浏览器 / 用户.
pub const WEB_SESSION_ID: &str = "web-session";

/// 默认 IdentityCard continuity_id (append 之前若 store 里没 card, 自动 seed 一个).
pub const DEFAULT_CONTINUITY_ID: &str = "apeireth-web-default";

/// 查询时最多展示的 episode 条数.
pub const RECENT_EPISODES_LIMIT: usize = 20;

// ============================================================
// IdentityCard (v1 兼容字段) — 本地定义, 不依赖 core
// ============================================================

/// 主体连续性 ID (跨载体唯一). v1 兼容字段语义, 不引 v2 core (v2 core 字段不一样).
#[derive(Debug, Clone)]
pub struct IdentityCard {
    /// 跨载体唯一 ID
    pub continuity_id: String,
    /// 诞生时间戳 (epoch seconds)
    pub birth_time: i64,
    /// 当前所在载体列表
    pub carriers: Vec<String>,
    /// 跨载体迁移历史 (载体名)
    pub migration_history: Vec<String>,
}

/// IdentityCard 注册表 trait (跟 v1 `IdentityCardStore` 语义对齐).
pub trait IdentityCardStore {
    fn create(&mut self, card: &IdentityCard) -> Result<(), String>;
    fn get(&self, continuity_id: &str) -> Option<IdentityCard>;
    fn list(&self) -> Vec<IdentityCard>;
    fn exists(&self, continuity_id: &str) -> bool;
}

/// In-memory IdentityCard 注册表 (HashMap by_id).
#[derive(Debug, Default)]
pub struct InMemoryIdentityCardStore {
    by_id: HashMap<String, IdentityCard>,
}

impl InMemoryIdentityCardStore {
    pub fn new() -> Self { Self::default() }
}

impl IdentityCardStore for InMemoryIdentityCardStore {
    fn create(&mut self, card: &IdentityCard) -> Result<(), String> {
        if card.continuity_id.trim().is_empty() {
            return Err("continuity_id 不能为空".into());
        }
        self.by_id.insert(card.continuity_id.clone(), card.clone());
        Ok(())
    }
    fn get(&self, continuity_id: &str) -> Option<IdentityCard> {
        self.by_id.get(continuity_id).cloned()
    }
    fn list(&self) -> Vec<IdentityCard> {
        self.by_id.values().cloned().collect()
    }
    fn exists(&self, continuity_id: &str) -> bool {
        self.by_id.contains_key(continuity_id)
    }
}

// ============================================================
// WebMemoryStore — Episode + IdentityCard 复合容器
// ============================================================

/// Web UI 用 memory 存储 (复合). 内部 episode + identity 两个子 store.
pub struct WebMemoryStore {
    pub episodes: InMemoryEpisodeStore,
    pub identities: InMemoryIdentityCardStore,
}

impl WebMemoryStore {
    pub fn new() -> Self {
        Self {
            episodes: InMemoryEpisodeStore::new(1024),
            identities: InMemoryIdentityCardStore::new(),
        }
    }

    /// Query episode (跟 v1 SqliteMemoryStore.query 同款).
    /// v2 EpisodeStore.query 返回 Vec<&Episode>; 这里 .cloned() 转 Vec<Episode>.
    pub fn query(&self, q: &EpisodeQuery) -> Vec<Episode> {
        self.episodes.query(q).into_iter().cloned().collect()
    }

    /// Append episode (兼容 v1 SqliteMemoryStore.put_episode 签名).
    /// v2 EpisodeStore 没有 put_episode, 用 append 代替 (append 返回 (), 这里包成 Ok(())).
    pub fn put_episode(&mut self, ep: &Episode) -> Result<(), String> {
        self.episodes.append(ep.clone());
        Ok(())
    }

    /// IdentityCard get / list / exists (wrapper).
    pub fn identity_get(&self, cid: &str) -> Option<IdentityCard> {
        self.identities.get(cid)
    }
    pub fn identity_list(&self) -> Vec<IdentityCard> {
        self.identities.list()
    }
    pub fn identity_exists(&self, cid: &str) -> bool {
        self.identities.exists(cid)
    }
    pub fn identity_create(&mut self, card: &IdentityCard) -> Result<(), String> {
        self.identities.create(card)
    }
}

impl Default for WebMemoryStore {
    fn default() -> Self { Self::new() }
}

/// 共享 store 句柄. main 启动期 lazy init 一次, 所有 handler 复用.
static STORE: OnceLock<Arc<Mutex<WebMemoryStore>>> = OnceLock::new();

/// 拿到共享 store (首次调用时初始化). 失败时返回错误字符串, 方便 handler 渲 error 页.
pub fn get_store() -> Result<Arc<Mutex<WebMemoryStore>>, String> {
    if let Some(s) = STORE.get() {
        return Ok(Arc::clone(s));
    }
    let path = resolve_db_path();
    info!(db_path = %path.display(), "opening WebMemoryStore for apeireth-web (in-memory v2)");
    let store = WebMemoryStore::new();
    let arc = Arc::new(Mutex::new(store));
    let _ = STORE.set(Arc::clone(&arc));
    Ok(arc)
}

/// 兼容 v1 的 `get_store` → 返回 `Arc<SqliteMemoryStore>` 风格的 alias.
/// v2 实际是 `Arc<Mutex<WebMemoryStore>>`. 调用方需要 `lock()` 才能用.
/// (v1 callers 都用 `.query()` / `.put_episode()` / `.get()`, 这里提供同名方法)

fn resolve_db_path() -> PathBuf {
    // v2 不依赖 SQLite, 这个 path 只是日志记录 + 兼容 v1 env var 读取
    if let Ok(p) = std::env::var("APEIRETH_MEMORY_DB") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("apeireth-memory.db")
}

fn seed_default_identity(store: &mut WebMemoryStore) -> Result<(), String> {
    if store.identity_exists(DEFAULT_CONTINUITY_ID) {
        return Ok(());
    }
    let now = chrono_now_unix();
    let card = IdentityCard {
        continuity_id: DEFAULT_CONTINUITY_ID.to_string(),
        birth_time: now,
        carriers: vec!["apeireth-web".to_string()],
        migration_history: vec![],
    };
    store.identity_create(&card)?;
    Ok(())
}

/// 拿当前 IdentityCard (取已 seed 的 default).
/// 返回 `None` 仅在 store 完全为空 (不应该发生, 因为 seed 过).
pub fn current_identity(store: &WebMemoryStore) -> Option<IdentityCard> {
    if let Some(card) = store.identity_get(DEFAULT_CONTINUITY_ID) {
        return Some(card);
    }
    let mut all = store.identity_list();
    if !all.is_empty() {
        return all.pop();
    }
    None
}

// ============================================================
// Handlers
// ============================================================

/// `GET /memory` — 渲染时间线 + IdentityCard.
#[cfg(feature = "ssr")]
pub async fn memory_page_handler() -> impl IntoResponse {
    let store_arc = match get_store() {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "memory store init failed");
            return Html(render_error_page(&e));
        },
    };
    let store = store_arc.lock().expect("memory mutex poisoned");

    // 拉最近 N 条
    let episodes: Vec<Episode> = store.query(
        &EpisodeQuery::new()
            .for_session(WEB_SESSION_ID)
            .limit(RECENT_EPISODES_LIMIT),
    );

    let identity = current_identity(&store);

    Html(render_memory_page(&episodes, identity.as_ref()))
}

/// POST form payload.
#[derive(Debug, serde::Deserialize)]
pub struct AppendForm {
    /// "system" / "user" / "assistant"
    pub role: String,
    pub content: String,
}

/// `POST /memory/append` — append 一条 episode, 然后 302 → `/memory`.
/// 用 `Result<Redirect, Html<String>>` 让 axum 统一处理成功 (302) 和错误 (200 错误页).
#[cfg(feature = "ssr")]
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

    let store_arc = get_store().map_err(|e| Html(render_error_page(&e)))?;
    let now = chrono_now_unix();
    let ep = Episode {
        id: format!("ep-web-{}", now),
        timestamp: now,
        role: role.clone(),
        content: content.clone(),
        session_id: WEB_SESSION_ID.to_string(),
        metadata: HashMap::new(),
    };

    {
        let mut store = store_arc.lock().expect("memory mutex poisoned");
        // 首次拿到 store 时 seed default identity
        if let Err(e) = seed_default_identity(&mut store) {
            warn!(error = %e, "seed default identity card failed (non-fatal)");
        }
        if let Err(e) = store.put_episode(&ep) {
            error!(error = %e, "put_episode failed");
            return Err(Html(render_error_page(&format!("append episode 失败: {e}"))));
        }
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
    identity: Option<&IdentityCard>,
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
            <p class="tagline">Episode 时间线 · append-only · 真接通 apeireth-storage::memory_episode (v2)</p>
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
pub fn chrono_now_unix() -> i64 {
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
        assert_eq!(epoch_to_iso8601_utc(946_684_800), "2000-01-01 00:00:00 UTC");
    }

    #[test]
    fn epoch_2024_01_01() {
        assert_eq!(
            epoch_to_iso8601_utc(1_704_067_200),
            "2024-01-01 00:00:00 UTC"
        );
    }

    #[test]
    fn epoch_2025_12_31_23_59_59() {
        assert_eq!(
            epoch_to_iso8601_utc(1_767_225_599),
            "2025-12-31 23:59:59 UTC"
        );
    }

    #[test]
    fn epoch_handles_seconds_within_day() {
        assert_eq!(
            epoch_to_iso8601_utc(1_704_078_855),
            "2024-01-01 03:14:15 UTC"
        );
    }

    #[test]
    fn identity_card_create_get() {
        let mut store = InMemoryIdentityCardStore::new();
        let card = IdentityCard {
            continuity_id: "cid-test".into(),
            birth_time: 100,
            carriers: vec!["web".into()],
            migration_history: vec![],
        };
        assert!(!store.exists("cid-test"));
        store.create(&card).unwrap();
        assert!(store.exists("cid-test"));
        let got = store.get("cid-test").unwrap();
        assert_eq!(got.continuity_id, "cid-test");
        assert_eq!(got.birth_time, 100);
    }

    #[test]
    fn identity_card_create_rejects_empty_id() {
        let mut store = InMemoryIdentityCardStore::new();
        let bad = IdentityCard {
            continuity_id: "  ".into(),
            birth_time: 0,
            carriers: vec![],
            migration_history: vec![],
        };
        assert!(store.create(&bad).is_err());
    }

    #[test]
    fn identity_card_list_returns_all() {
        let mut store = InMemoryIdentityCardStore::new();
        for i in 0..3 {
            let card = IdentityCard {
                continuity_id: format!("cid-{i}"),
                birth_time: i as i64,
                carriers: vec![],
                migration_history: vec![],
            };
            store.create(&card).unwrap();
        }
        assert_eq!(store.list().len(), 3);
    }

    #[test]
    fn web_memory_store_roundtrip_episode() {
        let mut store = WebMemoryStore::new();
        let ep = Episode {
            id: "e1".into(),
            timestamp: 100,
            role: "user".into(),
            content: "hello".into(),
            session_id: "s".into(),
            metadata: HashMap::new(),
        };
        store.put_episode(&ep).unwrap();
        let q = EpisodeQuery::new().for_session("s").limit(10);
        let got = store.query(&q);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "e1");
        assert_eq!(got[0].content, "hello");
    }

    #[test]
    fn render_memory_page_with_no_episodes() {
        let page = render_memory_page(&[], None);
        assert!(page.contains("暂无 episode"));
        assert!(page.contains("Append"));
    }

    #[test]
    fn render_memory_page_with_one_episode_and_identity() {
        let ep = Episode {
            id: "e1".into(),
            timestamp: 1_704_067_200,
            role: "user".into(),
            content: "你好".into(),
            session_id: "s".into(),
            metadata: HashMap::new(),
        };
        let card = IdentityCard {
            continuity_id: "cid".into(),
            birth_time: 1_704_067_200,
            carriers: vec!["web".into()],
            migration_history: vec![],
        };
        let page = render_memory_page(&[ep], Some(&card));
        assert!(page.contains("cid"));
        assert!(page.contains("你好"));
        assert!(page.contains("🪪"));
    }
}
