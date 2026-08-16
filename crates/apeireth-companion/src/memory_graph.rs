//! `apeireth-companion::memory_graph` — 时序知识图谱 + 记忆链接 (Zep / A-MEM 吸收).
//!
//! 当初调研标注未落地 (docs/research/mempalace-vs-apeireth-memory.md: 「时序三元组借鉴
//! (Zep-killer 卖点)」, R179 P2), 2026-08-16 补齐.
//!
//! **Zep 双时态边**: 事实 = (subject, predicate, object, valid_at, invalid_at).
//! - 事实变化 → 旧边置 invalid (追加同 chain 新版, append-only 留痕), 新边新建
//! - 提取 = LLM 提炼器输出 graph 三元组 (对齐 Zep: LLM 抽取, 非规则)
//! - 幻觉校验合并进提炼对账 (Mem0 对账 LLM 判定矛盾 → update/delete)
//!
//! **A-MEM 带权链接 + CRAWL**: 条目写入时与既有条目计算文本重叠 → 生成链接 (link-*);
//! 注入时从选中条目沿链接展开 (CRAWL, 预算内).
//! 0 假装: v1 链接是规则级 (文本重叠), 非 LLM 语义链接; 权重 = 重叠率.

use std::sync::Arc;

use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 图谱事实 (双时态边).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphFact {
    pub id: String,
    /// 逻辑链标识 (同 s|p|o; 无效化 = 新 id + 同 chain).
    pub chain: String,
    /// 链内版本号.
    pub rev: u64,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    /// 生效时间 (unix 秒).
    pub valid_at: i64,
    /// 失效时间 (None = 当前有效).
    pub invalid_at: Option<i64>,
    pub importance: u8,
}

/// 记忆链接 (A-MEM 式, 规则权重).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    pub id: String,
    pub from: String,
    pub to: String,
    /// 权重 0..1 (v1 = 文本重叠率).
    pub weight: f64,
}

/// 图后端原语 (P1#5 审计 backlog 全清, 2026-08-16).
///
/// Kùzu 持久化后端的机制口: `MemoryGraph` 的时序/链接/爬取逻辑留在机制层,
/// 后端只做「存取原语」。当前默认实现 [`SqliteGraphBackend`] (episode 存储,
/// 跨重启持久); Kùzu 后端因本机无 cmake 工具链 + GitHub 直连被墙无法构建,
/// 如实标注: trait 口已备, 环境就绪后实现 `GraphBackend` 即可替换 (0 装 PASS)。
pub trait GraphBackend: Send + Sync {
    fn save_fact(&self, f: &GraphFact) -> Result<(), String>;
    fn load_facts(&self) -> Result<Vec<GraphFact>, String>;
    fn save_link(&self, l: &MemoryLink) -> Result<(), String>;
    fn load_links(&self) -> Result<Vec<MemoryLink>, String>;
    fn load_episodes(&self, session: &str, n: usize) -> Result<Vec<CoreEpisode>, String>;
}

/// SQLite 后端: factg-*/link-* 以 episode 形态持久化 (现有路径, 跨重启不丢).
pub struct SqliteGraphBackend {
    store: Arc<SqliteMemoryStore>,
}

impl SqliteGraphBackend {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }
}

impl GraphBackend for SqliteGraphBackend {
    fn save_fact(&self, f: &GraphFact) -> Result<(), String> {
        let content = serde_json::to_string(f).map_err(|e| format!("序列化失败: {e}"))?;
        let ep = CoreEpisode {
            id: f.id.clone(),
            timestamp: f.valid_at,
            role: "assistant".into(),
            content,
            session_id: "me".into(),
        };
        self.store
            .put_episode(&ep)
            .map_err(|e| format!("写入失败: {e}"))
    }

    fn load_facts(&self) -> Result<Vec<GraphFact>, String> {
        let eps = self
            .store
            .recent_episodes("me", 500)
            .map_err(|e| e.to_string())?;
        Ok(eps
            .iter()
            .filter(|e| e.id.starts_with("factg-"))
            .filter_map(|e| serde_json::from_str::<GraphFact>(&e.content).ok())
            .collect())
    }

    fn save_link(&self, l: &MemoryLink) -> Result<(), String> {
        let content = serde_json::to_string(l).map_err(|e| format!("序列化失败: {e}"))?;
        let ep = CoreEpisode {
            id: l.id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            role: "assistant".into(),
            content,
            session_id: "me".into(),
        };
        self.store
            .put_episode(&ep)
            .map_err(|e| format!("写入失败: {e}"))
    }

    fn load_links(&self) -> Result<Vec<MemoryLink>, String> {
        let eps = self
            .store
            .recent_episodes("me", 500)
            .map_err(|e| e.to_string())?;
        Ok(eps
            .iter()
            .filter(|e| e.id.starts_with("link-"))
            .filter_map(|e| serde_json::from_str::<MemoryLink>(&e.content).ok())
            .collect())
    }

    fn load_episodes(&self, session: &str, n: usize) -> Result<Vec<CoreEpisode>, String> {
        self.store
            .recent_episodes(session, n)
            .map_err(|e| e.to_string())
    }
}

/// 结构化图查询 (Kùzu 式查询语义的 SQLite 版; 过滤当前有效事实).
#[derive(Debug, Clone, Default)]
pub struct GraphQuery {
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object: Option<String>,
}

impl GraphQuery {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn subject(mut self, s: impl Into<String>) -> Self {
        self.subject = Some(s.into());
        self
    }
    pub fn predicate(mut self, p: impl Into<String>) -> Self {
        self.predicate = Some(p.into());
        self
    }
    pub fn object(mut self, o: impl Into<String>) -> Self {
        self.object = Some(o.into());
        self
    }
}

/// 时序图谱服务 (机制层; 后端可替换).
pub struct MemoryGraph {
    backend: Box<dyn GraphBackend>,
}

impl MemoryGraph {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self { backend: Box::new(SqliteGraphBackend::new(store)) }
    }

    /// 注入自定义后端 (Kùzu 等; trait 口).
    pub fn with_backend(backend: Box<dyn GraphBackend>) -> Self {
        Self { backend }
    }

    fn save_fact(&self, f: &GraphFact) {
        if let Err(e) = self.backend.save_fact(f) {
            eprintln!("[graph] {e}");
        }
    }

    /// 添加事实: 同 (s,p,o) 已有边 → 先无效化旧边 (双时态), 再建新边.
    /// rev 单调分配: 无效化版本 = 链内 max+1, 新边 = max+2 (确定性, 防同秒
    /// 排序抖动导致去重取到旧版 — 2026-08-16 全量测试随机挂根因).
    /// 返回新事实 id (供 A-MEM 链接).
    pub fn add_fact(&self, s: &str, p: &str, o: &str, importance: u8) -> String {
        let now = chrono::Utc::now().timestamp();
        let chain = format!("{s}|{p}|{o}");
        // 链内当前最大 rev (含已无效版本) + 1 = 下一个可用 rev
        let mut next_rev = self
            .all_chain_versions(&chain)
            .into_iter()
            .map(|v| v.rev)
            .max()
            .unwrap_or(0)
            + 1;
        if let Some(old) = self.valid_for(&chain) {
            let mut inv = old.clone();
            inv.id = format!("factg-{}", uuid::Uuid::new_v4());
            inv.rev = next_rev;
            inv.invalid_at = Some(now);
            self.save_fact(&inv);
            next_rev += 1; // 新边必须大于无效化版本
        }
        let id = format!("factg-{}", uuid::Uuid::new_v4());
        self.save_fact(&GraphFact {
            id: id.clone(),
            chain,
            rev: next_rev,
            subject: s.to_string(),
            predicate: p.to_string(),
            object: o.to_string(),
            valid_at: now,
            invalid_at: None,
            importance,
        });
        id
    }

    /// 链全部版本 (含无效).
    fn all_chain_versions(&self, chain: &str) -> Vec<GraphFact> {
        self.backend
            .load_facts()
            .unwrap_or_default()
            .into_iter()
            .filter(|f| f.chain == chain)
            .collect()
    }

    /// 当前有效事实 (按 chain 取最新且 invalid_at 为 None).
    pub fn active_facts(&self) -> Vec<GraphFact> {
        let mut by_chain: std::collections::HashMap<String, GraphFact> = std::collections::HashMap::new();
        for f in self.backend.load_facts().unwrap_or_default() {
            match by_chain.get(&f.chain) {
                Some(existing) if existing.rev >= f.rev => {}
                _ => {
                    by_chain.insert(f.chain.clone(), f);
                }
            }
        }
        by_chain
            .into_values()
            .filter(|f| f.invalid_at.is_none())
            .collect()
    }

    /// 结构化查询 (P1#5): 按 subject/predicate/object 过滤当前有效事实.
    pub fn query(&self, q: &GraphQuery) -> Vec<GraphFact> {
        self.active_facts()
            .into_iter()
            .filter(|f| q.subject.as_ref().is_none_or(|s| f.subject == *s))
            .filter(|f| q.predicate.as_ref().is_none_or(|p| f.predicate == *p))
            .filter(|f| q.object.as_ref().is_none_or(|o| f.object == *o))
            .collect()
    }

    fn valid_for(&self, chain: &str) -> Option<GraphFact> {
        self.active_facts().into_iter().find(|f| f.chain == chain)
    }

    /// 事实图注入块.
    pub fn graph_injection(&self) -> String {
        let facts = self.active_facts();
        if facts.is_empty() {
            return String::new();
        }
        let mut s = String::from("【事实图】(时序知识图谱, 双时态有效事实):\n");
        for f in facts.iter().take(10) {
            s.push_str(&format!("  • {} {} {} (有效自 {})\n", f.subject, f.predicate, f.object, f.valid_at));
        }
        s
    }

    /// 写入时自动链接 (A-MEM 规则版): 与既有条目文本重叠率 >= 0.3 → 链接.
    pub fn link_on_write(&self, new_id: &str, new_content: &str) {
        let eps = self.backend.load_episodes("me", 100).unwrap_or_default();
        for e in eps.iter() {
            if e.id == new_id || e.id.starts_with("link-") || e.id.starts_with("tomb-") {
                continue;
            }
            let w = text_overlap(new_content, &e.content);
            if w >= 0.3 {
                let link = MemoryLink {
                    id: format!("link-{}", uuid::Uuid::new_v4()),
                    from: new_id.to_string(),
                    to: e.id.clone(),
                    weight: w,
                };
                if let Err(err) = self.backend.save_link(&link) {
                    eprintln!("[graph] 链接写入失败: {err}");
                }
            }
        }
    }

    /// CRAWL (A-MEM): 从种子条目沿链接展开 (权重降序, 预算内).
    pub fn crawl(&self, seeds: &[String], budget: usize) -> Vec<String> {
        let eps = self.backend.load_episodes("me", 500).unwrap_or_default();
        let links = self.backend.load_links().unwrap_or_default();
        let content_of = |id: &str| -> Option<String> {
            eps.iter()
                .find(|e| e.id == id)
                .map(|e| e.content.clone())
        };
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: Vec<(String, f64)> = seeds
            .iter()
            .map(|s| (s.clone(), 1.0))
            .collect();
        while !queue.is_empty() && out.len() < budget {
            queue.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let (id, _) = queue.remove(0);
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(c) = content_of(&id) {
                out.push(c);
            }
            for l in links.iter().filter(|l| l.from == id) {
                if !seen.contains(&l.to) {
                    queue.push((l.to.clone(), l.weight));
                }
            }
        }
        out
    }
}

/// 文本重叠率 (字符集合 Jaccard).
fn text_overlap(a: &str, b: &str) -> f64 {
    let norm = |s: &str| {
        s.chars()
            .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
            .collect::<std::collections::HashSet<_>>()
    };
    let sa = norm(a);
    let sb = norm(b);
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    #[test]
    fn temporal_facts_invalidate_old() {
        let g = MemoryGraph::new(store());
        g.add_fact("主人", "备考", "高数期中", 8);
        // 同三元组新事实 → 旧边无效化
        g.add_fact("主人", "备考", "高数期中", 9);
        let active = g.active_facts();
        assert_eq!(active.len(), 1, "旧边应被无效化");
        assert_eq!(active[0].importance, 9, "新边生效");
        // 不同三元组共存
        g.add_fact("主人", "喜欢", "烟火", 7);
        assert_eq!(g.active_facts().len(), 2);
        let inj = g.graph_injection();
        assert!(inj.contains("备考"));
        assert!(inj.contains("喜欢"));
    }

    #[test]
    fn links_and_crawl() {
        let s = store();
        let g = MemoryGraph::new(Arc::clone(&s));
        // 写两条相似内容 (重叠率高 → 自动链接)
        let id1 = "mem-ex-a".to_string();
        let id2 = "mem-ex-b".to_string();
        s.put_episode(&CoreEpisode { id: id1.clone(), timestamp: 1, role: "assistant".into(), content: "主人喜欢水墨画风格".into(), session_id: "me".into() }).unwrap();
        s.put_episode(&CoreEpisode { id: id2.clone(), timestamp: 2, role: "assistant".into(), content: "主人偏好水墨画风格和深蓝夜空".into(), session_id: "me".into() }).unwrap();
        g.link_on_write(&id2, "主人偏好水墨画风格和深蓝夜空");
        // CRAWL 从 id2 沿链接找 id1
        let crawled = g.crawl(&[id2], 3);
        assert!(crawled.iter().any(|c| c.contains("喜欢水墨画")), "沿链接应展开到 id1: {crawled:?}");
    }

    #[test]
    fn text_overlap_basic() {
        assert!(text_overlap("abcde", "abcxy") > 0.3);
        assert!(text_overlap("今天天气很好", "主人喜欢深蓝夜空") < 0.3, "无共同字符");
    }

    #[test]
    fn structured_query_filters_active_facts() {
        let g = MemoryGraph::new(store());
        g.add_fact("主人", "备考", "高数期中", 8);
        g.add_fact("主人", "喜欢", "烟火", 7);
        g.add_fact("本座", "负责", "基地", 6);
        let by_subject = g.query(&GraphQuery::new().subject("主人"));
        assert_eq!(by_subject.len(), 2, "subject 过滤应命中 2 条");
        let by_pred = g.query(&GraphQuery::new().predicate("喜欢"));
        assert_eq!(by_pred.len(), 1);
        assert_eq!(by_pred[0].object, "烟火");
        let by_both = g.query(&GraphQuery::new().subject("主人").predicate("备考"));
        assert_eq!(by_both.len(), 1);
        let none = g.query(&GraphQuery::new().subject("不存在"));
        assert!(none.is_empty());
        // 全空查询 = 全部有效事实
        assert_eq!(g.query(&GraphQuery::new()).len(), 3);
    }

    /// 内存假后端: 验证 trait 注入路径 (Kùzu 后端的机制口).
    struct MemoryBackend {
        facts: std::sync::Mutex<Vec<GraphFact>>,
        links: std::sync::Mutex<Vec<MemoryLink>>,
    }

    impl GraphBackend for MemoryBackend {
        fn save_fact(&self, f: &GraphFact) -> Result<(), String> {
            self.facts.lock().unwrap().push(f.clone());
            Ok(())
        }
        fn load_facts(&self) -> Result<Vec<GraphFact>, String> {
            Ok(self.facts.lock().unwrap().clone())
        }
        fn save_link(&self, l: &MemoryLink) -> Result<(), String> {
            self.links.lock().unwrap().push(l.clone());
            Ok(())
        }
        fn load_links(&self) -> Result<Vec<MemoryLink>, String> {
            Ok(self.links.lock().unwrap().clone())
        }
        fn load_episodes(&self, _session: &str, _n: usize) -> Result<Vec<CoreEpisode>, String> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn custom_backend_injection_works() {
        let backend = MemoryBackend {
            facts: std::sync::Mutex::new(Vec::new()),
            links: std::sync::Mutex::new(Vec::new()),
        };
        let g = MemoryGraph::with_backend(Box::new(backend));
        g.add_fact("主人", "备考", "高数期中", 8);
        // 同三元组更新 → 双时态 (旧无效化 + 新边)
        g.add_fact("主人", "备考", "高数期中", 9);
        let active = g.active_facts();
        assert_eq!(active.len(), 1, "后端无关, 时序语义应一致");
        assert_eq!(active[0].importance, 9);
        assert_eq!(g.query(&GraphQuery::new().subject("主人")).len(), 1);
    }
}
