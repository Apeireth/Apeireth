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
//!
//! **Intrinsic Residual 锚增益 (N6, 吸收 VCP rust-vexus-lite `compute_intrinsic_residuals`)**:
//! 节点「特异性」信号, 与 importance 正交 — VCP 原版是向量层残差 (邻居基解释不了的分量),
//! 本图无嵌入向量, 落地为文本层等价 (机制而非补丁, 确定性, 0 LLM):
//! - 事实节点: s/p/o 实体的逆频稀有度均值 (tf-idf 精神); 全图唯一实体 → 1.0 (最大特异)
//! - 内容节点 (CRAWL): 字符集残差 = 本节点不被邻居字符集解释的比例 (VCP residual norm 文本等价)
//! - 检索排序: combined = importance_weight×(importance/10) + residual_weight×specificity, 权重可配
//! - 增量维护: 实体计数 lazy init 一次 + add_fact O(1) 增量 (同链替换不改活跃计数), 不全图重扫
//! 0 假装: 无嵌入向量 = 纯文本近似; 无随机成分 = 无需种子注入, 同输入恒同分同序.

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
    /// N2 OneRing: continuity 锚点 (消灭 "me" 硬编码). `Box<dyn GraphBackend>` 上层
    /// (`link_on_write`, `crawl`) 不再硬编码 "me", 改走后端自报 continuity_id.
    /// 返 `String` 而非 `&str`: trait object (`Box<dyn GraphBackend>`) 无法
    /// 暴露与 `&self` 同生死的借用, 内存假后端用 `Mutex<String>` 持有, 复制即出.
    fn continuity_id(&self) -> String;
}

/// SQLite 后端: factg-*/link-* 以 episode 形态持久化 (现有路径, 跨重启不丢).
pub struct SqliteGraphBackend {
    store: Arc<SqliteMemoryStore>,
    /// N2 OneRing: continuity 锚点 (消灭 "me" 硬编码).
    continuity_id: String,
}

impl SqliteGraphBackend {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self {
            store,
            continuity_id: crate::continuity::current_continuity_id(),
        }
    }

    /// 显式注入 continuity 锚点.
    pub fn with_continuity(mut self, continuity: impl Into<String>) -> Self {
        let cid = crate::continuity::normalize_continuity(&continuity.into(), &self.continuity_id);
        self.continuity_id = cid;
        self
    }
}

impl GraphBackend for SqliteGraphBackend {
    fn continuity_id(&self) -> String {
        self.continuity_id.clone()
    }

    fn save_fact(&self, f: &GraphFact) -> Result<(), String> {
        let content = serde_json::to_string(f).map_err(|e| format!("序列化失败: {e}"))?;
        let ep = CoreEpisode {
            id: f.id.clone(),
            timestamp: f.valid_at,
            role: "assistant".into(),
            content,
            session_id: self.continuity_id.clone(),
        };
        self.store
            .put_episode(&ep)
            .map_err(|e| format!("写入失败: {e}"))
    }

    fn load_facts(&self) -> Result<Vec<GraphFact>, String> {
        let eps = self
            .store
            .recent_episodes(&self.continuity_id, 500)
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
            session_id: self.continuity_id.clone(),
        };
        self.store
            .put_episode(&ep)
            .map_err(|e| format!("写入失败: {e}"))
    }

    fn load_links(&self) -> Result<Vec<MemoryLink>, String> {
        let eps = self
            .store
            .recent_episodes(&self.continuity_id, 500)
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

/// 检索排序配置 (N6): importance 与特异性 (intrinsic residual) 的组合权重.
///
/// combined = importance_weight × (importance/10) + residual_weight × specificity;
/// 两项均 [0,1] 尺度, 权重默认各 1.0, 经 [`MemoryGraph::with_rank_config`] 可配.
#[derive(Debug, Clone, Copy)]
pub struct GraphRankConfig {
    pub importance_weight: f64,
    pub residual_weight: f64,
}

impl Default for GraphRankConfig {
    fn default() -> Self {
        Self {
            importance_weight: 1.0,
            residual_weight: 1.0,
        }
    }
}

/// 时序图谱服务 (机制层; 后端可替换).
pub struct MemoryGraph {
    backend: Box<dyn GraphBackend>,
    /// N6: 检索排序权重 (importance × 特异性).
    rank_config: GraphRankConfig,
    /// N6: 实体计数 (s/p/o 文本 → 活跃事实数). lazy init 一次, 之后 add_fact O(1)
    /// 增量维护; None = 未初始化 (首次评分时借调用方已加载的事实初始化, 不额外读后端).
    entity_counts: std::sync::Mutex<Option<std::collections::HashMap<String, u32>>>,
}

impl MemoryGraph {
    pub fn new(store: Arc<SqliteMemoryStore>) -> Self {
        Self {
            backend: Box::new(SqliteGraphBackend::new(store)),
            rank_config: GraphRankConfig::default(),
            entity_counts: std::sync::Mutex::new(None),
        }
    }

    /// 注入自定义后端 (Kùzu 等; trait 口).
    pub fn with_backend(backend: Box<dyn GraphBackend>) -> Self {
        Self {
            backend,
            rank_config: GraphRankConfig::default(),
            entity_counts: std::sync::Mutex::new(None),
        }
    }

    /// N6: 配置检索排序权重 (importance 与特异性的组合比例).
    pub fn with_rank_config(mut self, cfg: GraphRankConfig) -> Self {
        self.rank_config = cfg;
        self
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
        let existing = self.valid_for(&chain);
        if let Some(old) = existing.as_ref() {
            let mut inv = old.clone();
            inv.id = format!("factg-{}", uuid::Uuid::new_v4());
            inv.rev = next_rev;
            inv.invalid_at = Some(now);
            self.save_fact(&inv);
            next_rev += 1; // 新边必须大于无效化版本
        }
        let id = format!("factg-{}", uuid::Uuid::new_v4());
        let fact = GraphFact {
            id: id.clone(),
            chain,
            rev: next_rev,
            subject: s.to_string(),
            predicate: p.to_string(),
            object: o.to_string(),
            valid_at: now,
            invalid_at: None,
            importance,
        };
        // N6 增量维护: 仅新三元组 +1; 同链替换活跃实体集不变, 计数无需动.
        if existing.is_none() {
            self.observe_fact(&fact);
        }
        self.save_fact(&fact);
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
        let mut by_chain: std::collections::HashMap<String, GraphFact> =
            std::collections::HashMap::new();
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
    /// N6: 结果按组合分降序 (importance × 特异性), 同分按 chain→id 稳定确定性排序.
    pub fn query(&self, q: &GraphQuery) -> Vec<GraphFact> {
        let facts = self.active_facts();
        self.ensure_counts_from(&facts);
        let filtered: Vec<GraphFact> = facts
            .into_iter()
            .filter(|f| q.subject.as_ref().is_none_or(|s| f.subject == *s))
            .filter(|f| q.predicate.as_ref().is_none_or(|p| f.predicate == *p))
            .filter(|f| q.object.as_ref().is_none_or(|o| f.object == *o))
            .collect();
        self.rank(filtered)
    }

    /// N6 增量维护: 实体计数 lazy init 一次 (借调用方已加载的事实, 0 额外后端读).
    fn ensure_counts_from(&self, active: &[GraphFact]) {
        let mut g = self.entity_counts.lock().unwrap();
        if g.is_some() {
            return;
        }
        let mut m = std::collections::HashMap::new();
        for f in active {
            for e in [&f.subject, &f.predicate, &f.object] {
                *m.entry(e.clone()).or_insert(0) += 1;
            }
        }
        *g = Some(m);
    }

    /// N6 增量维护: 自足版初始化 — 未初始化时自行加载活跃事实建表一次.
    /// 保证 specificity()/combined_score() 独立调用也永远正确 (不静默给 1.0).
    fn ensure_counts(&self) {
        if self.entity_counts.lock().unwrap().is_some() {
            return;
        }
        let active = self.active_facts();
        self.ensure_counts_from(&active);
    }

    /// N6 增量维护: 新事实 (新三元组) 实体计数 +1; 计数未初始化则跳过 (init 会全量建).
    fn observe_fact(&self, f: &GraphFact) {
        let mut g = self.entity_counts.lock().unwrap();
        if let Some(m) = g.as_mut() {
            for e in [&f.subject, &f.predicate, &f.object] {
                *m.entry(e.clone()).or_insert(0) += 1;
            }
        }
    }

    /// N6 特异性 (intrinsic residual, 事实节点): s/p/o 实体逆频稀有度均值, [0,1].
    /// 实体出现在越多活跃事实里越不特异; 三实体均全图唯一 → 1.0.
    /// 确定性 (无随机, 同输入同分); 计数未含的新实体按 freq=1 计 (最大特异).
    pub fn specificity(&self, f: &GraphFact) -> f64 {
        self.ensure_counts();
        let g = self.entity_counts.lock().unwrap();
        let rarity = |e: &str| -> f64 {
            match g.as_ref().and_then(|m| m.get(e)) {
                Some(&n) if n > 0 => 1.0 / f64::from(n),
                _ => 1.0,
            }
        };
        (rarity(&f.subject) + rarity(&f.predicate) + rarity(&f.object)) / 3.0
    }

    /// N6 组合分: importance 归一到 [0,1] 与特异性按配置权重组合.
    pub fn combined_score(&self, f: &GraphFact) -> f64 {
        let imp = f64::from(f.importance).min(10.0) / 10.0;
        self.rank_config.importance_weight * imp
            + self.rank_config.residual_weight * self.specificity(f)
    }

    /// N6 排序: 组合分降序; 同分按 chain → id 字典序 (确定性, 同输入恒同序).
    fn rank(&self, mut facts: Vec<GraphFact>) -> Vec<GraphFact> {
        facts.sort_by(|a, b| {
            self.combined_score(b)
                .partial_cmp(&self.combined_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chain.cmp(&b.chain))
                .then_with(|| a.id.cmp(&b.id))
        });
        facts
    }

    fn valid_for(&self, chain: &str) -> Option<GraphFact> {
        self.active_facts().into_iter().find(|f| f.chain == chain)
    }

    /// 事实图注入块.
    /// N6: 注入前按组合分排序 — 高重要度且高特异性的事实优先进上下文.
    pub fn graph_injection(&self) -> String {
        let facts = self.active_facts();
        if facts.is_empty() {
            return String::new();
        }
        self.ensure_counts_from(&facts);
        let ranked = self.rank(facts);
        let mut s = String::from("【事实图】(时序知识图谱, 双时态有效事实):\n");
        for f in ranked.iter().take(10) {
            s.push_str(&format!(
                "  • {} {} {} (有效自 {})\n",
                f.subject, f.predicate, f.object, f.valid_at
            ));
        }
        s
    }

    /// M2 双级检索分诊 (社区层): 确定性路由 — Entity 路由返回命中实体 (调用方
    /// 续走实体链 CRAWL), Broad 路由返回社区摘要 brief. CRAWL 本体评分 0 改动.
    pub fn triage(&self, query: &str) -> crate::community::TriageResult {
        let facts = self.active_facts();
        crate::community::triage(query, &facts, 5, 8)
    }

    /// 写入时自动链接 (A-MEM 规则版): 与既有条目文本重叠率 >= 0.3 → 链接.
    pub fn link_on_write(&self, new_id: &str, new_content: &str) {
        let cid = self.backend.continuity_id();
        let eps = self.backend.load_episodes(&cid, 100).unwrap_or_default();
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
    /// N6 锚增益: 扩展优先级 = 链接权重 × (1 + residual_weight × 内容残差) —
    /// 邻居解释不了的独特内容 (高残差) 被锚增益抬升, 与既有链接权重正交相乘.
    pub fn crawl(&self, seeds: &[String], budget: usize) -> Vec<String> {
        let cid = self.backend.continuity_id();
        let eps = self.backend.load_episodes(&cid, 500).unwrap_or_default();
        let links = self.backend.load_links().unwrap_or_default();
        let content_of = |id: &str| -> Option<String> {
            eps.iter().find(|e| e.id == id).map(|e| e.content.clone())
        };
        // 内容残差: 本节点字符集中不被任一邻居 (双向链接) 解释的比例.
        let residual_of = |id: &str| -> f64 {
            let Some(content) = content_of(id) else {
                return 0.0;
            };
            let neighbor_texts: Vec<String> = links
                .iter()
                .filter(|l| l.from == id || l.to == id)
                .filter_map(|l| content_of(if l.from == id { &l.to } else { &l.from }))
                .collect();
            let refs: Vec<&str> = neighbor_texts.iter().map(|s| s.as_str()).collect();
            content_residual(&content, &refs)
        };
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: Vec<(String, f64)> = seeds.iter().map(|s| (s.clone(), 1.0)).collect();
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
                    let boosted =
                        l.weight * (1.0 + self.rank_config.residual_weight * residual_of(&l.to));
                    queue.push((l.to.clone(), boosted));
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

/// 内容残差 (N6 intrinsic residual, 内容节点): 本节点特异字符中不被邻居字符集
/// 解释的比例, [0,1]. VCP residual norm 的文本层等价 (向量层 = 邻居基解释不了
/// 的分量; 文本层 = 邻居字符集覆盖不了的内容).
/// 无邻居 → 1.0 (固有最大特异); 空内容 → 0.0. 纯函数, 确定性, 无种子需求.
fn content_residual(content: &str, neighbors: &[&str]) -> f64 {
    let norm = |s: &str| {
        s.chars()
            .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
            .collect::<std::collections::HashSet<_>>()
    };
    let mine = norm(content);
    if mine.is_empty() {
        return 0.0;
    }
    if neighbors.is_empty() {
        return 1.0;
    }
    let mut theirs: std::collections::HashSet<char> = std::collections::HashSet::new();
    for n in neighbors {
        theirs.extend(norm(n));
    }
    let unexplained = mine.difference(&theirs).count();
    unexplained as f64 / mine.len() as f64
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
        let g = MemoryGraph::with_backend(Box::new(SqliteGraphBackend::new(Arc::clone(&s))));
        // 写两条相似内容 (重叠率高 → 自动链接)
        let id1 = "mem-ex-a".to_string();
        let id2 = "mem-ex-b".to_string();
        let cid = crate::continuity::current_continuity_id();
        s.put_episode(&CoreEpisode {
            id: id1.clone(),
            timestamp: 1,
            role: "assistant".into(),
            content: "主人喜欢水墨画风格".into(),
            session_id: cid.clone(),
        })
        .unwrap();
        s.put_episode(&CoreEpisode {
            id: id2.clone(),
            timestamp: 2,
            role: "assistant".into(),
            content: "主人偏好水墨画风格和深蓝夜空".into(),
            session_id: cid,
        })
        .unwrap();
        g.link_on_write(&id2, "主人偏好水墨画风格和深蓝夜空");
        // CRAWL 从 id2 沿链接找 id1
        let crawled = g.crawl(&[id2], 3);
        assert!(
            crawled.iter().any(|c| c.contains("喜欢水墨画")),
            "沿链接应展开到 id1: {crawled:?}"
        );
    }

    #[test]
    fn text_overlap_basic() {
        assert!(text_overlap("abcde", "abcxy") > 0.3);
        assert!(
            text_overlap("今天天气很好", "主人喜欢深蓝夜空") < 0.3,
            "无共同字符"
        );
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
        eps: std::sync::Mutex<Vec<CoreEpisode>>,
        /// N2 OneRing: continuity 锚点 (测试 fixture 校验用).
        continuity_id: std::sync::Mutex<String>,
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
        fn load_episodes(&self, session: &str, n: usize) -> Result<Vec<CoreEpisode>, String> {
            Ok(self
                .eps
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.session_id == session)
                .take(n)
                .cloned()
                .collect())
        }
        fn continuity_id(&self) -> String {
            self.continuity_id.lock().unwrap().clone()
        }
    }

    #[test]
    fn custom_backend_injection_works() {
        let backend = MemoryBackend {
            facts: std::sync::Mutex::new(Vec::new()),
            links: std::sync::Mutex::new(Vec::new()),
            eps: std::sync::Mutex::new(Vec::new()),
            continuity_id: std::sync::Mutex::new(crate::continuity::current_continuity_id()),
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

    // ===== N6: Intrinsic Residual 锚增益 (特异性: 与 importance 正交的锚增益) =====

    #[test]
    fn n6_boundary_empty_graph_and_single_node() {
        // 空图: 无评分无恐慌 (查询/注入/crawl 全空)
        let g = MemoryGraph::new(store());
        assert!(g.query(&GraphQuery::new()).is_empty());
        assert_eq!(g.graph_injection(), "");
        assert!(g.crawl(&[], 5).is_empty());
        // 单节点: 三实体全唯一 → 特异性 1.0 (最大)
        g.add_fact("主人", "备考", "高数期中", 8);
        let facts = g.query(&GraphQuery::new());
        assert_eq!(facts.len(), 1);
        let s = g.specificity(&facts[0]);
        assert!((s - 1.0).abs() < 1e-9, "单节点应最大特异, got {s}");
    }

    #[test]
    fn n6_specificity_discriminates_shared_vs_unique() {
        let g = MemoryGraph::new(store());
        g.add_fact("主人", "喜欢", "烟火", 7);
        g.add_fact("主人", "喜欢", "深蓝夜空", 7);
        g.add_fact("本座", "负责", "基地", 6);
        let active = g.active_facts();
        let common = active.iter().find(|f| f.object == "烟火").unwrap();
        let unique = active.iter().find(|f| f.subject == "本座").unwrap();
        // 主人 freq2 → 1/2, 喜欢 freq2 → 1/2, 烟火 freq1 → 1.0
        let s_common = g.specificity(common);
        let expected = (0.5 + 0.5 + 1.0) / 3.0;
        assert!(
            (s_common - expected).abs() < 1e-9,
            "got {s_common}, want {expected}"
        );
        let s_unique = g.specificity(unique);
        assert!((s_unique - 1.0).abs() < 1e-9);
        assert!(s_unique > s_common, "全唯一实体节点应比共享实体节点更特异");
    }

    #[test]
    fn n6_content_residual_boundary() {
        assert_eq!(content_residual("", &[]), 0.0, "空内容 → 0");
        assert_eq!(content_residual("abc", &[]), 1.0, "无邻居 → 固有最大特异");
        assert!(
            (content_residual("abc", &["abc"]) - 0.0).abs() < 1e-9,
            "同内容被邻居完全解释"
        );
        assert!(
            (content_residual("abc", &["xyz"]) - 1.0).abs() < 1e-9,
            "不相交 → 完全未解释"
        );
        assert!(
            (content_residual("abcd", &["ab"]) - 0.5).abs() < 1e-9,
            "半解释 → 0.5"
        );
    }

    #[test]
    fn n6_combined_rank_weights_configurable() {
        let build = || {
            let g = MemoryGraph::new(store());
            // 4 条共享 subject/predicate → 主人/喜欢 freq4, 特异性低
            g.add_fact("主人", "喜欢", "烟火", 9);
            g.add_fact("主人", "喜欢", "深蓝", 9);
            g.add_fact("主人", "喜欢", "水墨", 9);
            g.add_fact("主人", "喜欢", "星空", 9);
            // 全实体唯一 → 特异性 1.0, 但 importance 低
            g.add_fact("孤本", "罕有", "残卷", 3);
            g
        };
        // importance 主导 → 高重要度在前
        let g_imp = build().with_rank_config(GraphRankConfig {
            importance_weight: 1.0,
            residual_weight: 0.0,
        });
        assert_eq!(g_imp.query(&GraphQuery::new())[0].importance, 9);
        // 特异性主导 → 全唯一实体事实 (孤本) 排首
        let g_res = build().with_rank_config(GraphRankConfig {
            importance_weight: 0.0,
            residual_weight: 1.0,
        });
        let top = g_res.query(&GraphQuery::new());
        assert_eq!(
            top[0].subject,
            "孤本",
            "特异性主导排序: {:?}",
            top.iter().map(|f| f.subject.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn n6_deterministic_scores_and_order() {
        let build = || {
            let g = MemoryGraph::new(store());
            g.add_fact("主人", "喜欢", "烟火", 7);
            g.add_fact("主人", "备考", "高数期中", 8);
            g.add_fact("本座", "负责", "基地", 6);
            g.add_fact("烟火", "照亮", "夜空", 5);
            g
        };
        let a = build();
        let b = build();
        let ra = a.query(&GraphQuery::new());
        let rb = b.query(&GraphQuery::new());
        assert_eq!(ra.len(), rb.len());
        for (fa, fb) in ra.iter().zip(rb.iter()) {
            assert_eq!(fa.chain, fb.chain, "同输入应同序");
            assert!(
                (a.combined_score(fa) - b.combined_score(fb)).abs() < 1e-12,
                "同输入应同分"
            );
        }
    }

    #[test]
    fn n6_incremental_counts_match_cold_start() {
        let s = store();
        let g = MemoryGraph::new(Arc::clone(&s));
        g.add_fact("主人", "喜欢", "烟火", 7);
        let _ = g.query(&GraphQuery::new()); // 触发计数 lazy init
                                             // init 后: 新三元组增量 +1; 同链替换不改活跃实体集
        g.add_fact("主人", "备考", "高数期中", 8);
        g.add_fact("主人", "备考", "高数期中", 9); // 替换 → 计数不变
        let g_cold = MemoryGraph::new(Arc::clone(&s)); // 冷启动实例 = 全量扫描基线
        let _ = g_cold.query(&GraphQuery::new());
        let active = g.active_facts();
        assert_eq!(active.len(), 2);
        for f in &active {
            let inc = g.specificity(f);
            let cold = g_cold.specificity(f);
            assert!(
                (inc - cold).abs() < 1e-12,
                "增量计数评分应等于冷启动全量评分: {inc} vs {cold}"
            );
        }
    }

    #[test]
    fn n6_crawl_anchor_boost_prefers_residual() {
        let cid = crate::continuity::current_continuity_id();
        let backend = MemoryBackend {
            facts: std::sync::Mutex::new(Vec::new()),
            links: std::sync::Mutex::new(Vec::new()),
            eps: std::sync::Mutex::new(vec![
                CoreEpisode {
                    id: "mem-seed".into(),
                    timestamp: 1,
                    role: "assistant".into(),
                    content: "aaaa bbbb".into(),
                    session_id: cid.clone(),
                },
                CoreEpisode {
                    id: "mem-dup".into(),
                    timestamp: 2,
                    role: "assistant".into(),
                    content: "aaaa bbbb".into(),
                    session_id: cid.clone(),
                },
                CoreEpisode {
                    id: "mem-uniq".into(),
                    timestamp: 3,
                    role: "assistant".into(),
                    content: "zzzz yyyy".into(),
                    session_id: cid.clone(),
                },
            ]),
            continuity_id: std::sync::Mutex::new(cid),
        };
        // seed → dup: 高权重但内容被种子完全解释 (残差 0)
        // seed → uniq: 低权重但内容完全独特 (残差 1.0 → 锚增益翻倍)
        backend.links.lock().unwrap().push(MemoryLink {
            id: "link-1".into(),
            from: "mem-seed".into(),
            to: "mem-dup".into(),
            weight: 0.5,
        });
        backend.links.lock().unwrap().push(MemoryLink {
            id: "link-2".into(),
            from: "mem-seed".into(),
            to: "mem-uniq".into(),
            weight: 0.31,
        });
        let g = MemoryGraph::with_backend(Box::new(backend));
        let out = g.crawl(&["mem-seed".to_string()], 2);
        assert_eq!(out.len(), 2, "种子 + 1 展开");
        assert!(out[0].contains("aaaa"), "种子优先");
        assert!(
            out[1].contains("zzzz"),
            "锚增益应抬升独特内容越过高权重复读: {:?}",
            out[1]
        );
    }
}
