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

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
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
    pub fn apply(&self, ex: &ExtractedMemory) -> Result<(), String> {
        let now = chrono::Utc::now().timestamp();
        for f in &ex.facts {
            if !f.content.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, &format!("{IMP_PREFIX}{}】{}", f.importance, f.content.trim()))?;
            }
        }
        for c in &ex.commitments {
            if !c.content.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, &format!("{IMP_PREFIX}{}】【约定】{}", c.importance, c.content.trim()))?;
            }
        }
        for p in &ex.preferences {
            if !p.content.trim().is_empty() {
                self.put(format!("pref-{}", uuid::Uuid::new_v4()), now, &format!("{IMP_PREFIX}{}】主人偏好: {}", p.importance, p.content.trim()))?;
            }
        }
        if let Some(e) = &ex.emotional {
            if !e.trim().is_empty() {
                self.put(format!("mem-ex-{}", uuid::Uuid::new_v4()), now, &format!("【情绪信号】{e}"))?;
            }
        }
        Ok(())
    }

    /// 静默写入一条 (append-only).
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
}
