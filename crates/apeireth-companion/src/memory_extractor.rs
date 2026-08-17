//! `apeireth-companion::memory_extractor` — 通用记忆提炼器 (自动捕获, 替代"加无数机制").
//!
//! 主人洞察 (2026-08-16): 「让它主动存审美偏好不能靠加一个机制又一个机制;
//! 要让它自动智能地捕获记忆, 或在反思期/做梦期自己提炼出关键信息。」
//!
//! 本模块 = **一个通用机制覆盖一切捕获**:
//! - 提炼维度: facts (事实) / preferences (偏好, 含审美/风格/语气) /
//!   commitments (约定承诺) / emotional (情绪信号)
//! - 触发: 对话后节流提炼 + 做梦期批量提炼 (serve 接线)
//! - 写入: facts/commitments → episodes (mem-ex-*); preferences → pref-* 偏好库
//! - 注入: preference_injection() 生成「主人偏好画像」注入对话 — 偏好自动跨场景应用
//!
//! 0 假装: lib 只定义 trait + 写入/注入 (无 LLM 依赖, 同做梦摘要的 trait 策略);
//! LLM 实现由调用方注入 (serve 的 MiniMax 版)。

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeMeta, EpisodeStore, Provenance, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// 一条记忆条目 (带重要性 — Generative Agents 式 LLM 打分, 1-10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// 重要性 1-10 (LLM 写入时打分; 缺失默认 5).
    #[serde(default = "default_importance")]
    pub importance: u8,
    pub content: String,
}

fn default_importance() -> u8 {
    5
}

impl MemoryItem {
    pub fn new(content: impl Into<String>, importance: u8) -> Self {
        Self { importance: importance.clamp(1, 10), content: content.into() }
    }
}

/// 完整记忆条目 (TP24/M5+N25): 来源链 + 时间元数据.
///
/// 字段:
/// - `id` / `timestamp` / `role` / `content` / `session_id`: 既有字段 (与 `CoreEpisode` 对齐)
/// - `valid_from_ms` / `valid_until_ms`: 时间有效性窗口 (epoch ms, None = 永久)
/// - `created_ms`: 创建时间 (epoch ms, 必填, 旧条目兜底 timestamp * 1000)
/// - `provenance`: 来源 (Dialog/Tool/Reflection/Observation/Manual, 默认 Manual)
///
/// 兼容旧条目 (V4 迁移前): 读取时按 [`apeireth_memory::normalize_meta`] 自动填默认.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    /// 事件时间戳 (epoch seconds, 与 Episode.timestamp 同义, 向后兼容保留).
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub session_id: String,
    /// 生效起点 (epoch ms). None = 永久生效.
    #[serde(default)]
    pub valid_from_ms: Option<i64>,
    /// 失效时间 (epoch ms). None = 永久有效 (per task 兼容默认).
    #[serde(default)]
    pub valid_until_ms: Option<i64>,
    /// 创建时间 (epoch ms, 必填, 旧条目兜底 timestamp * 1000).
    pub created_ms: i64,
    /// 来源 (默认 Manual).
    #[serde(default)]
    pub provenance: Provenance,
}

impl MemoryEntry {
    /// 从 `CoreEpisode` + `EpisodeMeta` 构造 (新代码路径)。
    pub fn from_episode(ep: &CoreEpisode, meta: &EpisodeMeta) -> Self {
        Self {
            id: ep.id.clone(),
            timestamp: ep.timestamp,
            role: ep.role.clone(),
            content: ep.content.clone(),
            session_id: ep.session_id.clone(),
            valid_from_ms: meta.valid_from_ms,
            valid_until_ms: meta.valid_until_ms,
            created_ms: meta.created_ms,
            provenance: meta.provenance,
        }
    }

    /// 提取 `EpisodeMeta` (给底层 store 写入用)。
    pub fn meta(&self) -> EpisodeMeta {
        EpisodeMeta {
            valid_from_ms: self.valid_from_ms,
            valid_until_ms: self.valid_until_ms,
            created_ms: self.created_ms,
            provenance: self.provenance,
        }
    }

    /// 提取 `CoreEpisode` (给底层 store 写入用, 5 字段子集)。
    pub fn core(&self) -> CoreEpisode {
        CoreEpisode {
            id: self.id.clone(),
            timestamp: self.timestamp,
            role: self.role.clone(),
            content: self.content.clone(),
            session_id: self.session_id.clone(),
        }
    }
}

/// 图谱事实三元组 (Zep 吸收, 2026-08-16): LLM 从对话抽取 (s, p, o).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphItem {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(default = "default_importance")]
    pub importance: u8,
}

/// 一次提炼的结果 (LLM 输出, JSON 对齐).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedMemory {
    /// 值得长期记住的事实 (带重要性).
    #[serde(default)]
    pub facts: Vec<MemoryItem>,
    /// 主人偏好 (审美/风格/语气/交互等) — 写入偏好库, 跨场景自动应用.
    #[serde(default)]
    pub preferences: Vec<MemoryItem>,
    /// 约定/承诺 (主人与 AI 之间的约定, 时间敏感事项).
    #[serde(default)]
    pub commitments: Vec<MemoryItem>,
    /// 情绪信号 (一句, 供后续关怀/节律参考).
    #[serde(default)]
    pub emotional: Option<String>,
    /// 图谱事实三元组 (Zep 双时态边; 写入时序知识图谱).
    #[serde(default)]
    pub graph: Vec<GraphItem>,
}

impl ExtractedMemory {
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.preferences.is_empty() && self.commitments.is_empty()
            && self.emotional.is_none() && self.graph.is_empty()
    }
}

/// importance 前缀 (写入格式: "【imp:N】内容"; 解析排序用).
pub const IMP_PREFIX: &str = "【imp:";

/// 从条目文本解析 importance (缺省 5).
pub fn parse_importance(content: &str) -> u8 {
    if let Some(rest) = content.strip_prefix(IMP_PREFIX) {
        if let Some(end) = rest.find('】') {
            if let Ok(n) = rest[..end].parse::<u8>() {
                return n.clamp(1, 10);
            }
        }
    }
    5
}

/// 提炼器 trait (LLM 实现由调用方注入; lib 无 LLM 依赖).
#[async_trait::async_trait]
pub trait MemoryExtractor: Send + Sync {
    /// 输入上下文 (最近对话/记忆文本), 返回结构化提炼.
    async fn extract(&self, context: &str) -> Result<ExtractedMemory, String>;

    /// 对账 (Mem0 式, 2026-08-16 吸收): 候选 vs 已有记忆 → 判定 ADD/UPDATE/DELETE.
    /// 默认实现 = 全 ADD (诚实降级: 未实现对账的调用方不假装).
    async fn reconcile(
        &self,
        candidates: &ExtractedMemory,
        _existing: &[String],
    ) -> Result<Vec<ReconcileAction>, String> {
        let mut out = Vec::new();
        for f in &candidates.facts {
            out.push(ReconcileAction { kind: ReconcileKind::Add, item: f.clone(), target_id: None });
        }
        for p in &candidates.preferences {
            out.push(ReconcileAction { kind: ReconcileKind::Add, item: p.clone(), target_id: None });
        }
        for c in &candidates.commitments {
            out.push(ReconcileAction { kind: ReconcileKind::Add, item: c.clone(), target_id: None });
        }
        Ok(out)
    }
}

/// 对账判定 (Mem0 式).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileKind {
    Add,
    Update,
    Delete,
}

/// 对账动作.
#[derive(Debug, Clone)]
pub struct ReconcileAction {
    pub kind: ReconcileKind,
    pub item: MemoryItem,
    /// Update/Delete 的目标条目 id (已有记忆).
    pub target_id: Option<String>,
}

/// 提炼服务: 写入 + 偏好库 + 注入 (纯机制, 无 LLM).
pub struct MemoryExtractionService {
    store: Arc<SqliteMemoryStore>,
}

impl MemoryExtractionService {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }

    /// 把提炼结果静默写入 (facts/commitments → mem-ex-*, preferences → pref-*).
    /// 写入格式带重要性前缀 "【imp:N】" (排序/反思阈值用).
    /// TP24: 5 种 items 都用 `Provenance::Dialog` (LLM 从对话提炼).
    pub fn apply(&self, ex: &ExtractedMemory) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        let prov = Provenance::Dialog;
        for f in &ex.facts {
            if !f.content.trim().is_empty() {
                self.put_with_provenance(
                    format!("mem-ex-{}", uuid::Uuid::new_v4()),
                    now,
                    &format!("{IMP_PREFIX}{}】{}", f.importance, f.content.trim()),
                    prov,
                )?;
            }
        }
        for c in &ex.commitments {
            if !c.content.trim().is_empty() {
                self.put_with_provenance(
                    format!("mem-ex-{}", uuid::Uuid::new_v4()),
                    now,
                    &format!("{IMP_PREFIX}{}】【约定】{}", c.importance, c.content.trim()),
                    prov,
                )?;
            }
        }
        for p in &ex.preferences {
            if !p.content.trim().is_empty() {
                self.put_with_provenance(
                    format!("pref-{}", uuid::Uuid::new_v4()),
                    now,
                    &format!("{IMP_PREFIX}{}】主人偏好: {}", p.importance, p.content.trim()),
                    prov,
                )?;
            }
        }
        if let Some(e) = &ex.emotional {
            if !e.trim().is_empty() {
                self.put_with_provenance(
                    format!("mem-ex-{}", uuid::Uuid::new_v4()),
                    now,
                    &format!("【情绪信号】{e}"),
                    prov,
                )?;
            }
        }
        Ok(())
    }

    /// 静默写入一条 (向后兼容路径: 不带元数据, 4 列均为 NULL → 老条目按 Manual + 永久有效).
    /// TP24 起, 推荐用 [`put_with_provenance`](Self::put_with_provenance) 或 [`put_with_meta`](Self::put_with_meta).
    fn put(&self, id: String, ts: i64, content: &str) -> Result<(), String> {
        let ep = CoreEpisode {
            id,
            timestamp: ts,
            role: "assistant".into(),
            content: content.to_string(),
            session_id: "me".into(),
        };
        self.store.put_episode(&ep).map_err(|e| e.to_string())
    }

    /// TP24: 写入一条, 显式指定来源 (默认元数据: created_ms=ts*1000, valid_from=created_ms, valid_until=None).
    pub fn put_with_provenance(
        &self,
        id: String,
        ts: i64,
        content: &str,
        provenance: Provenance,
    ) -> Result<(), String> {
        let created_ms = ts.saturating_mul(1000);
        let meta = EpisodeMeta {
            valid_from_ms: Some(created_ms),
            valid_until_ms: None,
            created_ms,
            provenance,
        };
        self.put_with_meta(id, ts, content, &meta)
    }

    /// TP24: 写入一条, 显式指定完整 `EpisodeMeta` (4 列全控).
    pub fn put_with_meta(
        &self,
        id: String,
        ts: i64,
        content: &str,
        meta: &EpisodeMeta,
    ) -> Result<(), String> {
        let ep = CoreEpisode {
            id,
            timestamp: ts,
            role: "assistant".into(),
            content: content.to_string(),
            session_id: "me".into(),
        };
        self.store.put_episode_full(&ep, meta).map_err(|e| e.to_string())
    }

    /// TP24: 按时间窗检索 (epoch_ms). 返回 `MemoryEntry` (含 provenance + 4 元数据列).
    /// 老条目 (4 列 NULL) 读取时按 `normalize_meta` 自动填默认 (Manual + timestamp*1000 + 永久).
    pub fn query_with_time_range(&self, from_ms: i64, until_ms: i64) -> Vec<MemoryEntry> {
        let eps = match self.store.query_with_time_range(from_ms, until_ms) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        eps.into_iter()
            .map(|ep| {
                let raw = self.store.read_episode_meta(&ep.id).ok().flatten();
                // 老条目 created_ms 兜底: raw.created_ms 为 0 (V4 前 NULL 列) → 当 None 处理
                let (vf, vu, cm, pr) = match raw {
                    Some(m) => {
                        let cm_opt = if m.created_ms <= 0 { None } else { Some(m.created_ms) };
                        apeireth_memory::normalize_meta(
                            m.valid_from_ms,
                            m.valid_until_ms,
                            cm_opt,
                            Some(m.provenance),
                            ep.timestamp,
                        )
                    }
                    None => apeireth_memory::normalize_meta(
                        None, None, None, None, ep.timestamp,
                    ),
                };
                MemoryEntry {
                    id: ep.id,
                    timestamp: ep.timestamp,
                    role: ep.role,
                    content: ep.content,
                    session_id: ep.session_id,
                    valid_from_ms: vf,
                    valid_until_ms: vu,
                    created_ms: cm,
                    provenance: pr,
                }
            })
            .collect()
    }

    /// 偏好库 → 「主人偏好画像」注入块 (按 importance 排序, 跨场景自动应用的核心).
    pub fn preference_injection(&self) -> String {
        let eps = self.store.recent_episodes("me", 300).unwrap_or_default();
        let mut prefs: Vec<(u8, String)> = eps
            .iter()
            .filter(|e| e.id.starts_with("pref-"))
            .filter(|e| !e.content.contains("【已废弃】"))
            .map(|e| (parse_importance(&e.content), e.content.clone()))
            .collect();
        prefs.sort_by(|a, b| b.0.cmp(&a.0)); // 重要性高的在前
        if prefs.is_empty() {
            return String::new();
        }
        let mut s = String::from("【主人偏好画像】(来自记忆提炼, 做审美/风格/交互类事情时优先沿用):\n");
        for (_, p) in prefs.iter().take(8) {
            s.push_str(&format!("  • {}\n", p.chars().take(120).collect::<String>()));
        }
        s
    }

    /// 有效记忆条目 (排除已废弃目标 — 对账 Delete/Update 的旧版本).
    pub fn active_episodes(&self, n: usize) -> Vec<CoreEpisode> {
        let eps = self.store.recent_episodes("me", n.max(50)).unwrap_or_default();
        // 收集废弃目标 id (tomb-* 条目内容含目标 id)
        let tombed: std::collections::HashSet<String> = eps
            .iter()
            .filter(|e| e.id.starts_with("tomb-"))
            .filter_map(|e| {
                e.content
                    .strip_prefix("【已废弃】")
                    .map(|s| s.trim().to_string())
            })
            .collect();
        let mut out: Vec<CoreEpisode> = eps
            .into_iter()
            .filter(|e| !e.id.starts_with("tomb-"))
            .filter(|e| !tombed.contains(&e.id))
            .collect();
        out.truncate(n);
        out
    }

    /// 对账应用: Add → 写入; Update → 废弃旧版 + 写入新版; Delete → 废弃旧版.
    pub fn apply_reconcile(&self, actions: &[ReconcileAction]) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        for a in actions {
            match a.kind {
                ReconcileKind::Add => {
                    let id = format!("mem-ex-{}", uuid::Uuid::new_v4());
                    self.put(id, now, &format!("{IMP_PREFIX}{}】{}", a.item.importance, a.item.content.trim()))?;
                }
                ReconcileKind::Update => {
                    if let Some(t) = &a.target_id {
                        self.put(format!("tomb-{}", uuid::Uuid::new_v4()), now, &format!("【已废弃】{t}"))?;
                    }
                    let id = format!("mem-ex-{}", uuid::Uuid::new_v4());
                    self.put(id, now, &format!("{IMP_PREFIX}{}】【更新】{}", a.item.importance, a.item.content.trim()))?;
                }
                ReconcileKind::Delete => {
                    if let Some(t) = &a.target_id {
                        self.put(format!("tomb-{}", uuid::Uuid::new_v4()), now, &format!("【已废弃】{t}"))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 图谱三元组写入 (Zep 双时态边 + A-MEM 自动链接; 调用方传 graph 服务).
    pub fn apply_graph(
        &self,
        graph: &[GraphItem],
        graph_svc: &crate::memory_graph::MemoryGraph,
    ) {
        for g in graph {
            if g.subject.trim().is_empty() || g.predicate.trim().is_empty() || g.object.trim().is_empty() {
                continue;
            }
            let id = graph_svc.add_fact(g.subject.trim(), g.predicate.trim(), g.object.trim(), g.importance);
            // A-MEM: 新事实写入后与既有记忆自动链接 (真实 id)
            graph_svc.link_on_write(&id, &format!("{} {} {}", g.subject, g.predicate, g.object));
        }
    }

    /// 提炼输入: 最近对话/记忆拼接 (供 LLM 提炼器).
    pub fn recent_context(&self, n: usize) -> String {
        let eps = self.store.recent_episodes("me", n.max(10)).unwrap_or_default();
        let mut parts: Vec<String> = Vec::new();
        for e in eps.iter().rev().take(n) {
            parts.push(format!("[{}] {}", e.role, e.content.chars().take(300).collect::<String>()));
        }
        parts.join("\n")
    }

    /// 提炼结果计数 (诊断).
    pub fn counts(&self) -> Value {
        let eps = self.store.recent_episodes("me", 500).unwrap_or_default();
        json!({
            "mem_ex": eps.iter().filter(|e| e.id.starts_with("mem-ex-")).count(),
            "prefs": eps.iter().filter(|e| e.id.starts_with("pref-")).count(),
        })
    }
}

/// 记忆分层排名 (模块 2, Generative Agents 式) —
/// score = importance×3 + access_count×0.3 + 组加成 + recency; 预算内注入.
/// 返回 (id, content) 供 access 追踪.
pub fn rank_memory_entries(
    eps: &[CoreEpisode],
    access: &std::collections::HashMap<String, (u64, i64)>,
    budget: usize,
) -> Vec<(String, String)> {
    let mut ranked: Vec<(&CoreEpisode, f64)> = eps.iter().map(|e| {
        let importance = parse_importance(&e.content) as f64;
        let (count, _) = access.get(&e.id).copied().unwrap_or((0, 0));
        // 组加成: 0=dream/pref (高价值常驻), 1=mem-ex/reflect, 2=其他
        let group_bonus = if e.id.starts_with("mem-dream-") || e.id.starts_with("pref-") {
            4.0
        } else if e.id.starts_with("mem-ex-") || e.id.starts_with("reflect-") {
            2.0
        } else {
            0.0
        };
        // recency: 最近 7 天内线性加成
        let age_days = (chrono::Utc::now().timestamp() - e.timestamp) as f64 / 86400.0;
        let recency = if age_days < 7.0 { (7.0 - age_days) / 7.0 } else { 0.0 };
        let score = importance * 3.0 + count as f64 * 0.3 + group_bonus + recency * 2.0;
        (e, score)
    }).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked
        .iter()
        .take(budget)
        .map(|(e, _)| (e.id.clone(), e.content.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn apply_writes_and_injects_preferences() {
        let s = MemoryExtractionService::new(store());
        let ex = ExtractedMemory {
            facts: vec![MemoryItem::new("主人周五考高数期中", 9)],
            preferences: vec![MemoryItem::new("唯美写意风格, 深蓝夜空配色", 8), MemoryItem::new("古风韵味", 6)],
            commitments: vec![MemoryItem::new("周六上午整理错题本", 7)],
            emotional: Some("今天有点累但心情平静".into()),
            graph: vec![],
        };
        s.apply(&ex).unwrap();
        // 偏好注入 (高 importance 在前)
        let inj = s.preference_injection();
        assert!(inj.contains("唯美写意"), "{inj}");
        assert!(inj.contains("古风"));
        // 计数
        let c = s.counts();
        assert_eq!(c["mem_ex"], json!(3)); // 2 facts/commitments + 1 emotional
        assert_eq!(c["prefs"], json!(2));
        // importance 前缀解析
        assert_eq!(parse_importance("【imp:9】主人周五考高数期中"), 9);
        assert_eq!(parse_importance("主人偏好: x"), 5);
        // 空提炼 → 偏好注入空
        let s2 = MemoryExtractionService::new(store());
        assert!(s2.preference_injection().is_empty());
    }

    #[test]
    fn reconcile_applies_add_update_delete() {
        let s = MemoryExtractionService::new(store());
        // 先有一条
        s.apply(&ExtractedMemory {
            facts: vec![MemoryItem::new("旧事实", 5)],
            preferences: vec![],
            commitments: vec![],
            emotional: None,
            graph: vec![],
        }).unwrap();
        let old_id = s.store.recent_episodes("me", 10).unwrap().iter().find(|e| e.id.starts_with("mem-ex-")).unwrap().id.clone();
        // 对账: Update 旧事实 + Add 新事实 + Delete 一条
        let actions = vec![
            ReconcileAction { kind: ReconcileKind::Update, item: MemoryItem::new("新事实 (取代旧)", 8), target_id: Some(old_id.clone()) },
            ReconcileAction { kind: ReconcileKind::Add, item: MemoryItem::new("全新事实", 6), target_id: None },
            ReconcileAction { kind: ReconcileKind::Delete, item: MemoryItem::new("", 1), target_id: Some("exp-nonexist".into()) },
        ];
        s.apply_reconcile(&actions).unwrap();
        // 废弃的旧事实被过滤
        let active = s.active_episodes(50);
        assert!(!active.iter().any(|e| e.id == old_id), "旧事实应被废弃过滤");
        assert!(active.iter().any(|e| e.content.contains("新事实")), "新版应存在");
        assert!(active.iter().any(|e| e.content.contains("全新事实")));
    }

    #[test]
    fn recent_context_has_roles() {
        let s = MemoryExtractionService::new(store());
        let ex = ExtractedMemory {
            facts: vec![MemoryItem::new("x", 5)],
            preferences: vec![],
            commitments: vec![],
            emotional: None,
            graph: vec![],
        };
        s.apply(&ex).unwrap();
        let ctx = s.recent_context(5);
        assert!(ctx.contains("assistant"));
    }

    // === TP24 (M5 + N25): 来源链 + 时间元数据 ===

    /// 验收 #1: 5 种 provenance 都能正确写入 + 读取.
    #[test]
    fn provenance_roundtrip_all_five_variants() {
        let s = MemoryExtractionService::new(store());
        let provenances = [
            Provenance::Dialog,
            Provenance::Tool,
            Provenance::Reflection,
            Provenance::Observation,
            Provenance::Manual,
        ];
        for prov in provenances {
            let id = format!("mem-ex-prov-{:?}", prov);
            let ts = 1_700_000_000_i64;
            s.put_with_provenance(id.clone(), ts, &format!("prov={:?}", prov), prov).unwrap();
            // 读取 raw 4 列元数据
            let meta = s.store.read_episode_meta(&id).unwrap().expect("just-written");
            assert_eq!(meta.provenance, prov, "provenance roundtrip failed for {:?}", prov);
            // created_ms = ts * 1000 (默认)
            assert_eq!(meta.created_ms, ts * 1000);
            // valid_from_ms = created_ms (默认), valid_until = None (永久)
            assert_eq!(meta.valid_from_ms, Some(ts * 1000));
            assert_eq!(meta.valid_until_ms, None);
        }
    }

    /// 验收 #2: 时间元数据边界 (valid_from/until 边界值).
    #[test]
    fn timing_metadata_boundaries() {
        let s = MemoryExtractionService::new(store());
        let id = "mem-ex-boundary".to_string();
        let ts = 1_700_000_000_i64;
        // 显式 meta: valid_from = 0 (epoch), valid_until = i64::MAX (几乎永久)
        let meta = EpisodeMeta {
            valid_from_ms: Some(0),
            valid_until_ms: Some(i64::MAX),
            created_ms: ts * 1000,
            provenance: Provenance::Dialog,
        };
        s.put_with_meta(id.clone(), ts, "边界条目", &meta).unwrap();
        let read = s.store.read_episode_meta(&id).unwrap().unwrap();
        assert_eq!(read.valid_from_ms, Some(0));
        assert_eq!(read.valid_until_ms, Some(i64::MAX));
        assert_eq!(read.created_ms, ts * 1000);
        assert_eq!(read.provenance, Provenance::Dialog);

        // 时间窗过滤: [0, i64::MAX] 应包含
        let entries = s.query_with_time_range(0, i64::MAX);
        assert!(entries.iter().any(|e| e.id == id), "边界条目应在时间窗内");

        // 时间窗过滤: [-1, 0] 边界 (from = 0 包含, until = 0 包含到 0)
        let entries = s.query_with_time_range(0, 0);
        assert!(entries.iter().any(|e| e.id == id), "valid_from=0 应被 >= 0 过滤命中");
    }

    /// 验收 #3: query_with_time_range 时间范围检索.
    #[test]
    fn query_with_time_range_filters_correctly() {
        let s = MemoryExtractionService::new(store());
        // 写入 3 条, ts 100/200/300 (秒) → created_ms 100_000/200_000/300_000
        // 第 3 条设 valid_until=Some(250_000) → 在 [50_000, 250_000] 仍有效, 之后失效
        for i in 1..=3 {
            let id = format!("mem-ex-time-{}", i);
            let ts = (100 * i) as i64; // 100, 200, 300
            if i == 3 {
                let meta = EpisodeMeta {
                    valid_from_ms: Some(ts * 1000),
                    valid_until_ms: Some(250_000),
                    created_ms: ts * 1000,
                    provenance: Provenance::Dialog,
                };
                s.put_with_meta(id, ts, &format!("t={}", ts), &meta).unwrap();
            } else {
                s.put_with_provenance(id, ts, &format!("t={}", ts), Provenance::Dialog).unwrap();
            }
        }

        // [50_000, 200_000]: t=100 (valid_until null, 永久 ✓) t=200 (created 200k ≥ 50k, vu null ✓) t=300 (valid_until 250k < 200k? 250k≥200k ✓; 但 created 300k≥50k ✓) → 3 都满足
        // 但 SQL 过滤是 created_ms >= from_ms AND (vu null OR vu >= until_ms), 没有 created_ms <= until_ms 上限
        // t=300 在 [50_000, 200_000]: created_ms=300_000 ≥ 50_000 ✓, vu=250_000 ≥ 200_000 ✓ → 命中 (永久判定 vs 失效判定都过)
        let r = s.query_with_time_range(50_000, 200_000);
        assert_eq!(r.len(), 3, "valid_until=250_000 ≥ until=200_000 应命中");
        let ids: Vec<_> = r.iter().map(|e| e.id.clone()).collect();
        assert!(ids.contains(&"mem-ex-time-3".to_string()));

        // [50_000, 200_000] 但 valid_until 改成 150_000 (t=300 失效) → 应只剩 t=100, t=200
        let st2 = store();
        let s2 = MemoryExtractionService::new(st2);
        s2.put_with_provenance("mem-ex-time-1".into(), 100, "t=100", Provenance::Dialog).unwrap();
        s2.put_with_provenance("mem-ex-time-2".into(), 200, "t=200", Provenance::Dialog).unwrap();
        let meta = EpisodeMeta {
            valid_from_ms: Some(300_000),
            valid_until_ms: Some(150_000), // 150_000 < 200_000 → 在 [50_000, 200_000] 内失效
            created_ms: 300_000,
            provenance: Provenance::Dialog,
        };
        s2.put_with_meta("mem-ex-time-3".into(), 300, "t=300", &meta).unwrap();
        let r = s2.query_with_time_range(50_000, 200_000);
        assert_eq!(r.len(), 2, "t=300 valid_until=150_000 < until=200_000 应被过滤");
        let ids: Vec<_> = r.iter().map(|e| e.id.clone()).collect();
        assert!(ids.contains(&"mem-ex-time-1".to_string()));
        assert!(ids.contains(&"mem-ex-time-2".to_string()));
        assert!(!ids.contains(&"mem-ex-time-3".to_string()));

        // [400_000, 500_000] → created_ms 全 < 400_000 → 空
        let r = s2.query_with_time_range(400_000, 500_000);
        assert!(r.is_empty(), "from_ms 大于所有 created_ms 应返回空");

        // 验证 MemoryEntry 字段都填齐 (provenance + 4 元数据列)
        for e in &s2.query_with_time_range(0, i64::MAX) {
            assert!(matches!(e.provenance, Provenance::Dialog | Provenance::Manual | Provenance::Tool | Provenance::Reflection | Provenance::Observation));
            assert!(e.created_ms > 0);
        }
    }

    /// 验收 #3 续: valid_until 失效过滤 (永久 vs 有上限).
    #[test]
    fn query_with_time_range_respects_valid_until() {
        let s = MemoryExtractionService::new(store());
        let ts = 1_700_000_000_i64;

        // 条目 A: 永久 (valid_until None)
        s.put_with_provenance("mem-ex-perm".into(), ts, "永久条目", Provenance::Manual).unwrap();
        // 条目 B: 在 [ts*1000+1000, ts*1000+5000] 有效
        let meta = EpisodeMeta {
            valid_from_ms: Some(ts * 1000 + 1000),
            valid_until_ms: Some(ts * 1000 + 5000),
            created_ms: ts * 1000,
            provenance: Provenance::Dialog,
        };
        s.put_with_meta("mem-ex-window".into(), ts, "窗口条目", &meta).unwrap();

        // 查询 [0, ts*1000+2000] → A 在 (永久), B 在 (valid_until >= 2_000)
        let r = s.query_with_time_range(0, ts * 1000 + 2000);
        let ids: Vec<_> = r.iter().map(|e| e.id.clone()).collect();
        assert!(ids.contains(&"mem-ex-perm".to_string()), "永久条目应始终命中");
        assert!(ids.contains(&"mem-ex-window".to_string()), "valid_until 在窗内应命中");

        // 查询 [0, ts*1000+6000] → B 已失效 (valid_until=5000 < until=6000)
        let r = s.query_with_time_range(0, ts * 1000 + 6000);
        let ids: Vec<_> = r.iter().map(|e| e.id.clone()).collect();
        assert!(!ids.contains(&"mem-ex-window".to_string()), "valid_until < until 应被过滤");
        assert!(ids.contains(&"mem-ex-perm".to_string()), "永久条目仍命中");
    }

    /// 验收 #4: 兼容测试 (旧条目无 4 列 → 自动填默认).
    #[test]
    fn backward_compat_old_entries_get_defaults() {
        let s = MemoryExtractionService::new(store());
        // 用 put() (旧 API, 不带元数据, 4 列 NULL)
        s.put("mem-ex-old".into(), 1_500_000, "老条目").unwrap();
        // 读取 4 列: 应该都是 NULL
        let raw = s.store.read_episode_meta("mem-ex-old").unwrap().unwrap();
        assert_eq!(raw.valid_from_ms, None, "老条目 valid_from_ms 应 NULL");
        assert_eq!(raw.valid_until_ms, None, "老条目 valid_until_ms 应 NULL");
        assert_eq!(raw.created_ms, 0, "老条目 created_ms 默认 0 (raw)");
        assert_eq!(raw.provenance, Provenance::Manual, "老条目 provenance 默认 Manual (raw)");

        // 通过 query_with_time_range 读取 (应用 normalize_meta)
        let r = s.query_with_time_range(0, i64::MAX);
        let old = r.iter().find(|e| e.id == "mem-ex-old").expect("应被读到");
        assert_eq!(old.provenance, Provenance::Manual, "兼容默认 Manual");
        assert_eq!(old.created_ms, 1_500_000 * 1000, "created_ms 兜底 timestamp*1000");
        assert_eq!(old.valid_from_ms, Some(1_500_000 * 1000), "valid_from 兜底 created_ms");
        assert_eq!(old.valid_until_ms, None, "valid_until 永久 (None 保留)");

        // 老条目应在时间窗内 (since created_ms = timestamp*1000)
        let r = s.query_with_time_range(1_500_000 * 1000, 1_500_000 * 1000);
        assert!(r.iter().any(|e| e.id == "mem-ex-old"), "老条目应按兜底 created_ms 命中时间窗");
    }

    /// 验收 #4 续: put() 旧路径写入条目, query_with_time_range 也能读到 (兼容).
    #[test]
    fn backward_compat_old_path_visible_in_new_query() {
        let s = MemoryExtractionService::new(store());
        s.put("mem-ex-old2".into(), 1_600_000, "老条目 2").unwrap();
        let r = s.query_with_time_range(0, i64::MAX);
        assert!(r.iter().any(|e| e.id == "mem-ex-old2"), "旧路径写入应能被新查询读到");
    }

    /// put_with_meta 显式控制 4 列.
    #[test]
    fn put_with_meta_full_control() {
        let s = MemoryExtractionService::new(store());
        let meta = EpisodeMeta {
            valid_from_ms: Some(100_000),
            valid_until_ms: Some(200_000),
            created_ms: 150_000,
            provenance: Provenance::Reflection,
        };
        s.put_with_meta("mem-ex-full".into(), 150, "全控条目", &meta).unwrap();
        let read = s.store.read_episode_meta("mem-ex-full").unwrap().unwrap();
        assert_eq!(read.valid_from_ms, Some(100_000));
        assert_eq!(read.valid_until_ms, Some(200_000));
        assert_eq!(read.created_ms, 150_000);
        assert_eq!(read.provenance, Provenance::Reflection);
    }

    /// 迁移 V4 已被应用 (sanity check: V4 加的 4 列存在).
    #[test]
    fn migration_v4_columns_exist() {
        let st = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = st.conn().unwrap();
        let mut stmt = conn
            .prepare("PRAGMA table_info(episodes)")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for col in ["valid_from_ms", "valid_until_ms", "created_ms", "provenance"] {
            assert!(cols.contains(&col.to_string()), "V4 列 {} 应存在, 实际: {:?}", col, cols);
        }
    }

    /// apply() 用 Provenance::Dialog 写入 (per 新设计).
    #[test]
    fn apply_uses_dialog_provenance() {
        let s = MemoryExtractionService::new(store());
        s.apply(&ExtractedMemory {
            facts: vec![MemoryItem::new("test", 5)],
            preferences: vec![],
            commitments: vec![],
            emotional: None,
            graph: vec![],
        }).unwrap();
        let r = s.query_with_time_range(0, i64::MAX);
        assert!(!r.is_empty());
        assert!(r.iter().all(|e| e.provenance == Provenance::Dialog), "apply 写入应统一为 Dialog 来源");
    }
}
