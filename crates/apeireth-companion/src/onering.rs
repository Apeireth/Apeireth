//! `apeireth-companion::onering` — OneRing 统一上下文账本 (backlog N2, VCP OneRing 对照吸收).
//!
//! **职责** (team-work-doc §8.4 方向①③):
//! 跨前端唯一 Agent 的统一时间线 — SSE / Web / Lark / Telegram / CLI 的每条
//! User/Assistant 发言都归入**同一 continuity 锚点**的账本, 逐条留痕
//! (来源对象 sender + 前端来源 frontend + 时间戳 ts).
//!
//! **对照 VCP OneRing** (research/source/vcptoolbox/Plugin/OneRing):
//! | VCP 机制 | 本模块 | 差异 |
//! |---|---|---|
//! | messages(agentName, role, senderName, frontendSource, content, timestamp) | onering_messages(continuity_id, role, sender, frontend, content, ts) | agentName → continuity 锚点 |
//! | pruneAgentMessages(maxRecords=100) | prune (默认 200, 可配) | — |
//! | 排序 timestamp | 排序 **seq 单调自增** (秒级时间戳竞争用单调序号解决, §1.3.7) | 更确定 |
//! | fuzzy diff / 时间线插入策略 | **不吸收** (0 假装: 本模块只做账本; 前端上下文回放由记忆注入管线负责) | 见 0 假装 |
//!
//! **存储**: 记忆库同文件的自有表 `onering_messages` (经 `store.conn()`, 同
//! continuity_link.rs 的建表模式) — 不污染 episodes 记忆管线 (提取/做梦/反思
//! 只读 episodes, 账本与记忆分流).
//!
//! **0 假装**:
//! - 不实现 VCP 的 fuzzy diff 历史比对 / RawClientTimeline / ServerInferredTimeline
//!   时间线插入 (那是"前端上下文回放"问题, Apeireth 由记忆注入管线覆盖).
//! - 不实现系统提示词占位符触发 (Apeireth 的触发点 = HTTP 端点本身, 无需占位符).
//! - prune 会删除旧行: 账本是"最近窗口留痕", 不是永久档案 (永久档案走 episodes 记忆).

use std::sync::Arc;

use apeireth_memory::SqliteMemoryStore;
use rusqlite::params;

/// 账本每锚点保留的最大条数缺省值 (VCP 缺省 100; 本库对话密度低, 放宽到 200).
pub const DEFAULT_MAX_RECORDS: usize = 200;

/// 合法发言角色 (VCP OneRing 同: user/assistant 两类留痕).
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";

/// 一条账本留痕.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// 单调自增序号 (同锚点内严格递增, 排序依据).
    pub seq: i64,
    /// 所属 continuity 锚点 (统一时间线的键).
    pub continuity_id: String,
    /// "user" | "assistant".
    pub role: String,
    /// 来源对象 (如 "master" / "apeireth"; 可为空串 = 未知).
    pub sender: String,
    /// 前端来源 (如 "web" / "openai-compat" / "cli" / "proactive").
    pub frontend: String,
    /// 发言内容.
    pub content: String,
    /// 时间戳 (epoch seconds, 留痕用; 排序用 seq).
    pub ts: i64,
}

/// 统一上下文账本 (Clone 廉价: Arc 共享).
#[derive(Clone)]
pub struct OneRingLedger {
    inner: Arc<LedgerInner>,
}

struct LedgerInner {
    store: Arc<SqliteMemoryStore>,
    continuity: String,
    max_records: usize,
}

impl OneRingLedger {
    /// 打开账本 (建表幂等). `continuity` 为锚点; 空锚点 → Err (0 装 PASS).
    pub fn new(store: Arc<SqliteMemoryStore>, continuity: impl Into<String>) -> Result<Self, String> {
        let continuity = continuity.into().trim().to_string();
        if continuity.is_empty() {
            return Err("continuity 锚点为空, 无法打开账本".into());
        }
        let this = Self {
            inner: Arc::new(LedgerInner {
                store,
                continuity,
                max_records: DEFAULT_MAX_RECORDS,
            }),
        };
        this.ensure_table()?;
        Ok(this)
    }

    /// 覆盖每锚点保留上限.
    pub fn with_max_records(mut self, n: usize) -> Self {
        let max = n.max(1);
        let inner = Arc::get_mut(&mut self.inner).expect("fresh ledger");
        inner.max_records = max;
        self
    }

    pub fn continuity(&self) -> &str {
        &self.inner.continuity
    }

    pub fn max_records(&self) -> usize {
        self.inner.max_records
    }

    /// 留痕一条发言到本账本锚点. 校验失败 (非法角色/空内容/空前端) → Err.
    pub fn record(
        &self,
        role: &str,
        sender: Option<&str>,
        frontend: &str,
        content: &str,
    ) -> Result<LedgerEntry, String> {
        self.record_as(&self.inner.continuity, role, sender, frontend, content)
    }

    /// 留痕到指定锚点 (多锚点场景: HTTP header 带不同 X-Apeireth-Continuity).
    pub fn record_as(
        &self,
        continuity: &str,
        role: &str,
        sender: Option<&str>,
        frontend: &str,
        content: &str,
    ) -> Result<LedgerEntry, String> {
        let continuity = continuity.trim();
        if continuity.is_empty() {
            return Err("continuity 锚点为空, 拒绝留痕".into());
        }
        if role != ROLE_USER && role != ROLE_ASSISTANT {
            return Err(format!("非法角色 `{role}` (账本只留 user/assistant)"));
        }
        let frontend = frontend.trim();
        if frontend.is_empty() {
            return Err("前端来源为空, 拒绝留痕 (OneRing 必须可溯源)".into());
        }
        let content = content.trim();
        if content.is_empty() {
            return Err("发言内容为空, 拒绝留痕".into());
        }
        let sender = sender.map(|s| s.trim().to_string()).unwrap_or_default();
        let ts = chrono::Utc::now().timestamp();

        let conn = self.inner.store.conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO onering_messages (continuity_id, role, sender, frontend, content, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![continuity, role, sender, frontend, content, ts],
        )
        .map_err(|e| e.to_string())?;
        let seq = conn.last_insert_rowid();
        // prune: 只保留最近 max_records 条 (按 seq, 确定性)
        conn.execute(
            "DELETE FROM onering_messages
              WHERE continuity_id = ?1
                AND seq NOT IN (
                    SELECT seq FROM onering_messages
                     WHERE continuity_id = ?1
                     ORDER BY seq DESC LIMIT ?2
                )",
            params![continuity, self.inner.max_records as i64],
        )
        .map_err(|e| e.to_string())?;
        Ok(LedgerEntry {
            seq,
            continuity_id: continuity.to_string(),
            role: role.to_string(),
            sender,
            frontend: frontend.to_string(),
            content: content.to_string(),
            ts,
        })
    }

    /// 最近 `limit` 条 (seq 升序, 时间线语义). limit=0 → 空.
    pub fn recent(&self, limit: usize) -> Result<Vec<LedgerEntry>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let conn = self.inner.store.conn().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, continuity_id, role, sender, frontend, content, ts
                   FROM onering_messages
                  WHERE continuity_id = ?1
                  ORDER BY seq DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![self.inner.continuity, limit as i64], |r| {
                Ok(LedgerEntry {
                    seq: r.get(0)?,
                    continuity_id: r.get(1)?,
                    role: r.get(2)?,
                    sender: r.get(3)?,
                    frontend: r.get(4)?,
                    content: r.get(5)?,
                    ts: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out: Vec<LedgerEntry> = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        out.reverse();
        Ok(out)
    }

    /// 本锚点账本条数.
    pub fn len(&self) -> Result<usize, String> {
        let conn = self.inner.store.conn().map_err(|e| e.to_string())?;
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM onering_messages WHERE continuity_id = ?1",
                params![self.inner.continuity],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(n.max(0) as usize)
    }

    pub fn is_empty(&self) -> Result<bool, String> {
        Ok(self.len()? == 0)
    }

    fn ensure_table(&self) -> Result<(), String> {
        let conn = self.inner.store.conn().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS onering_messages (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                continuity_id TEXT NOT NULL,
                role TEXT NOT NULL,
                sender TEXT NOT NULL DEFAULT '',
                frontend TEXT NOT NULL,
                content TEXT NOT NULL,
                ts INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_onering_continuity_seq
                ON onering_messages(continuity_id, seq);",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn records_and_replays_in_order() {
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        l.record("user", Some("master"), "web", "你好").unwrap();
        l.record("assistant", Some("apeireth"), "web", "主人好").unwrap();
        let evs = l.recent(10).unwrap();
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].role, "user");
        assert_eq!(evs[1].role, "assistant");
        assert!(evs[0].seq < evs[1].seq);
        assert_eq!(evs[0].frontend, "web");
        assert!(evs[0].ts > 0);
        assert_eq!(l.len().unwrap(), 2);
    }

    #[test]
    fn cross_frontend_same_timeline() {
        // OneRing 核心: SSE/Lark/Telegram/Web/CLI 归入同一锚点时间线
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        l.record("user", Some("master"), "web", "网页问的").unwrap();
        l.record("user", Some("master"), "openai-compat", "SSE 问的").unwrap();
        l.record("assistant", Some("apeireth"), "proactive", "主动问候").unwrap();
        l.record("user", Some("master"), "cli", "终端问的").unwrap();
        let evs = l.recent(10).unwrap();
        assert_eq!(evs.len(), 4, "四端发言应在同一时间线");
        let frontends: Vec<&str> = evs.iter().map(|e| e.frontend.as_str()).collect();
        assert_eq!(frontends, vec!["web", "openai-compat", "proactive", "cli"]);
    }

    #[test]
    fn multi_anchor_isolated() {
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        l.record("user", None, "web", "A 的话").unwrap();
        l.record_as("c-other", "user", None, "web", "B 的话").unwrap();
        assert_eq!(l.len().unwrap(), 1);
        let evs = l.recent(5).unwrap();
        assert_eq!(evs[0].content, "A 的话");
    }

    #[test]
    fn rejects_invalid_role() {
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        assert!(l.record("system", None, "web", "x").is_err());
        assert!(l.record("", None, "web", "x").is_err());
    }

    #[test]
    fn rejects_empty_content_or_frontend_or_anchor() {
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        assert!(l.record("user", None, "web", "   ").is_err());
        assert!(l.record("user", None, "  ", "内容").is_err());
        assert!(l.record_as(" ", "user", None, "web", "内容").is_err());
        assert!(OneRingLedger::new(store(), "  ").is_err());
    }

    #[test]
    fn prunes_to_max_records() {
        let l = OneRingLedger::new(store(), "c-main").unwrap().with_max_records(3);
        for i in 0..10 {
            l.record("user", None, "web", &format!("第{i}条")).unwrap();
        }
        assert_eq!(l.len().unwrap(), 3);
        let evs = l.recent(10).unwrap();
        assert_eq!(evs[0].content, "第7条", "prune 保留最近的");
        assert_eq!(evs[2].content, "第9条");
    }

    #[test]
    fn recent_limit_zero_is_empty() {
        let l = OneRingLedger::new(store(), "c-main").unwrap();
        l.record("user", None, "web", "x").unwrap();
        assert!(l.recent(0).unwrap().is_empty());
    }

    #[test]
    fn ledger_does_not_pollute_episodes() {
        use apeireth_memory::EpisodeStore;
        let st = store();
        let l = OneRingLedger::new(Arc::clone(&st), "c-main").unwrap();
        l.record("user", None, "web", "账本条目").unwrap();
        assert_eq!(st.count_by_session("c-main").unwrap(), 0, "账本与记忆管线分流");
    }
}
