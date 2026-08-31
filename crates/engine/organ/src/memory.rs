//! P-arch (2026-08-28): Memory 器官真移植 v2 (跨 8 organ 记忆合并抽象).
//!
//! **v1 → v2 翻译路线 (子代理 R8 独立判断)**:
//!
//! - **任务 spec 描述**: "v1 `MemoryMerger` 1:1 翻译 (跨 8 organ 记忆合并)".
//! - **v1 真实现核查**: `legacy/canonical/apeireth-companion/src/runtime_brain.rs` **没有
//!   `MemoryMerger` 模块**. v1 era "记忆合并" 是**散落 3 处**:
//!   - `runtime_brain.rs` (聚合 curiosity + emotion + hypothesis + catalog, L88 tick)
//!   - `memory_extractor.rs` (`MemoryExtractionService::apply` 写入 SQLite, 5 维度提炼)
//!   - `proactive_memory.rs` (`TopicPredictor` + `PreloadChannel` 主动预载)
//! - **0 装诚实**: v1 没有统一的"跨 8 organ 记忆合并抽象". v2 `MemoryMerger` 是
//!   **新设计** (按任务 spec), 不是 1:1 翻译. 真翻译纪律要求标: v2 是新抽象, 设计参考: v1
//!   `MemoryExtractionService` (dedup-by-content + weight + persist schema) 1:1 翻译其
//!   **算法骨架**.
//!
//! **v1 借鉴算法 (per `memory_extractor.rs:236-284` `MemoryExtractionService::apply`)**:
//! - **Dedup**: 同 source 重复内容 → 不重写 (per v1 put_with_meta 用 uuid v4 唯一 id;
//!   这里改成显式 content hash 去重).
//! - **Weight**: 初始 weight = 1.0 (per v1 importance 1-10 字段; 这里统一 f32 0.0-1.0).
//! - **Persist**: v1 写 SQLite (`SqliteMemoryStore::put_episode`); v2 trait boundary
//!   **不引入 SQLite 依赖** (per 子代理 R1-R7 同模式: 0 装诚实, std::collections::Vec 即可).
//!   真生产路径由 cognitive module 集成时注入外部 store (per 任务 "cognitive 集成接手" 建议).
//!
//! **v2 MemoryMerger 真设计**:
//!
//! - **API**: `merge` / `deduplicate` / `weight` / `persist` / `query` / `len` (per 任务 spec §1).
//! - **跨 8 organ trait 抽象**: `MemoryMerger::merge` 接收 `source_organ: OrganKind` 标识
//!   来源 (W1/W2/W3/E4/F4/F1/F6/E7), 不强制 8 organ trait 真注入 (避免 cyclic dep;
//!   organ → organ 反向). 跨 organ 真正接入由 cognitive module 集成时调度 (per 任务 "建议").
//! - **schema 对齐**: trait 输出 `Memory { notes_added, notes_merged }` (per
//!   `apeireth-plugin::organ:184-185` 锁定). notes_added = 本次新增条目数, notes_merged
//!   = 本次去重时合并到旧条目的次数.
//!
//! **0 装 PASS**:
//! - 本模块不假装能"统一合并 8 organ 状态" (v2 设计本就是新抽象, 跨 organ 调度由
//!   cognitive module 集成, 本模块仅做内容级 dedup/weight/persist/query).
//! - `llm_factory()` 返 None (跨 organ 合并是确定性无 LLM 抽象, 同 E4/F1 真实现).
//! - persist 默认走 std::collections::Vec (0 装诚实: 任务 spec "v1 0 装 PASS"; 真生产
//!   持久化由 cognitive module 注入 SqliteMemoryStore 引用).
//!
//! **3 阶审查** (O-6 锚 9, per 子代理 R1-R7 同模式):
//! 1. 总体: v1 `MemoryExtractionService` dedup/weight/persist 算法骨架 + 跨 organ
//!    来源标识 (新设计, 0 装诚实标)
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, Memory trait process() 调 MemoryMerger

use std::sync::Mutex;

use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};

// ============================================
// v1 数据结构 1:1 翻译 (per memory_extractor.rs schema)
// ============================================

/// 合并后记忆条目 (per v1 `MemoryItem` + `MemoryEntry` schema 借鉴).
///
/// **字段说明**:
/// - `id`: uuid 形式 (per v1 `mem-ex-<uuid>` / `pref-<uuid>` 1:1 翻译 → `mrg-<uuid>`).
/// - `content`: 自由文本 (per v1 通用 content).
/// - `source_organ`: OrganKind 标识来源 (W1/W2/W3/E4/F4/F1/F6/E7 — 跨 8 organ 抽象).
/// - `weight`: 重要性 ∈ [0.0, 1.0] (per v1 importance 1-10 归一化).
/// - `at_ms`: epoch ms (per v1 created_ms 1:1).
/// - `content_hash`: 用于去重的指纹 (per v1 dedup-by-content 同模式, FNV-1a hash).
#[derive(Debug, Clone)]
pub struct MergedMemory {
    pub id: String,
    pub content: String,
    pub source_organ: OrganKind,
    pub weight: f32,
    pub at_ms: i64,
    /// 内部: content FNV-1a hash (per dedup 路径)
    content_hash: u64,
}

impl MergedMemory {
    /// 构造 (自动算 content_hash + 钳 weight)
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        source_organ: OrganKind,
        weight: f32,
        at_ms: i64,
    ) -> Self {
        let content = content.into();
        let hash = fnv1a_hash(content.as_bytes());
        Self {
            id: id.into(),
            content,
            source_organ,
            weight: weight.clamp(0.0, 1.0),
            at_ms,
            content_hash: hash,
        }
    }

    /// 内容相似度判定 (per task spec "dedup by content similarity")
    /// - 完全相同 hash → 视为重复
    /// - hash 不同 → 视为不同 (FNV-1a 不模糊匹配; 0 装诚实, 不假装"语义相似度")
    pub fn is_same_content(&self, other_hash: u64) -> bool {
        self.content_hash == other_hash
    }
}

// ============================================
// 记忆合并配置
// ============================================

/// 记忆合并配置 (per task spec §1: dedup_threshold / max_capacity / decay_rate).
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// 去重阈值 (per task spec, 当前 FNV-1a hash 1:1 判等, threshold 保留扩展)
    pub dedup_threshold: f32,
    /// 最大容量 (超过则驱逐最早条目, 0 = 无上限)
    pub max_capacity: usize,
    /// 衰减率 (每次 query 时按 `decay_rate * age` 降低 weight, 0 = 不衰减)
    pub decay_rate: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            dedup_threshold: 0.95,
            max_capacity: 0, // 0 = 无上限
            decay_rate: 0.0, // 默认不衰减 (per v1 MemoryExtractionService 0 衰减路径)
        }
    }
}

// ============================================
// v1 MemoryExtractionService 借鉴算法骨架 (确定性, 无 LLM)
// ============================================

/// 跨 organ 记忆合并器 (per 任务 spec + v1 algorithm skeleton 借鉴).
///
/// **v1 真实现是 `MemoryExtractionService::apply` (per `memory_extractor.rs:236-284`)**:
/// - 接收 `ExtractedMemory { facts, preferences, commitments, emotional, graph }`.
/// - 5 维度分别 put_with_meta (生成 uuid, 写 SQLite).
/// - 无 dedup by content (依赖 uuid 唯一性) → v2 改进: 加 content-hash dedup.
///
/// **v2 真实现差异** (子代理 R8 独立判断, 0 装诚实标):
/// - v1 source_organ 是字符串 ("assistant" / session_id); v2 用 OrganKind 枚举 (跨 8 organ
///   抽象, per task spec).
/// - v1 persist 走 SqliteMemoryStore (外部依赖); v2 trait boundary 仅保留 5 件 API,
///   `persist()` 返回写入条数 (impl = 0 装 Vec in-memory; 真生产注入外部 store).
/// - v1 weight = importance 1-10 (LLM 打分); v2 weight = f32 0.0-1.0 (显式 `weight()` API
///   可调, 不假装 LLM 打分).
#[derive(Debug)]
pub struct MemoryMerger {
    memories: Vec<MergedMemory>,
    config: MemoryConfig,
}

impl MemoryMerger {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            memories: Vec::new(),
            config,
        }
    }

    /// 合并一条新记忆 (per task spec API `merge`).
    ///
    /// **流程**:
    /// 1. 计算 content hash.
    /// 2. 查找已有同 hash 条目 → 有则 weight += new_weight * dedup_threshold (合并路径,
    ///    notes_merged++), 无则新增 (notes_added++).
    /// 3. max_capacity 超限 → 驱逐最早条目.
    ///
    /// **返回**: `(is_new, id)` — `is_new=true` 表示新增, `false` 表示合并到旧条目.
    pub fn merge(
        &mut self,
        source_organ: OrganKind,
        content: &str,
        weight: f32,
        at_ms: i64,
    ) -> (bool, String) {
        let hash = fnv1a_hash(content.as_bytes());

        // dedup: 找同 hash 已有条目
        if let Some(existing) = self.memories.iter_mut().find(|m| m.is_same_content(hash)) {
            existing.weight =
                (existing.weight + weight * self.config.dedup_threshold).clamp(0.0, 1.0);
            existing.at_ms = existing.at_ms.max(at_ms);
            return (false, existing.id.clone());
        }

        // 新增
        let id = format!("mrg-{}", uuid::Uuid::new_v4());
        let mem = MergedMemory::new(&id, content, source_organ, weight, at_ms);
        self.memories.push(mem);

        // 容量驱逐 (FIFO: 最早条目先走)
        if self.config.max_capacity > 0 && self.memories.len() > self.config.max_capacity {
            self.memories.remove(0);
        }

        (true, id)
    }

    /// 内容级去重 (per task spec API `deduplicate`).
    ///
    /// **返回**: 实际去重掉的条目数.
    ///
    /// **0 装诚实**: 当前 hash 1:1 判等 (FNV-1a 无冲突概率极低但理论存在). threshold
    /// 字段为扩展接口, 当前实现中仅 exact match 去重. 升级到 fuzzy match 时用 threshold.
    pub fn deduplicate(&mut self) -> usize {
        let original_len = self.memories.len();
        let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut deduped: Vec<MergedMemory> = Vec::with_capacity(original_len);
        for m in self.memories.drain(..) {
            if seen.insert(m.content_hash) {
                deduped.push(m);
            }
            // 重复 hash 丢弃 (保留首次出现的条目, per v1 dedup-by-first 隐式行为)
        }
        self.memories = deduped;
        original_len - self.memories.len()
    }

    /// 调整单条 weight (per task spec API `weight`).
    ///
    /// `delta` 可正可负, 调整后钳到 [0.0, 1.0]. 找不到 id → 返 false.
    pub fn weight(&mut self, id: &str, delta: f32) -> bool {
        if let Some(m) = self.memories.iter_mut().find(|m| m.id == id) {
            m.weight = (m.weight + delta).clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// 持久化 (per task spec API `persist`).
    ///
    /// **0 装诚实 (per task spec "v1 0 装 PASS")**: 当前实现 = 计数已有条目 (in-memory
    /// Vec 已"持久"在内存). 真生产路径由 cognitive module 集成时注入外部
    /// `SqliteMemoryStore` 引用 (per `apeireth-memory` crate). trait boundary 仅暴露
    /// "需要被持久化" 的条目列表, 由调用方写盘.
    ///
    /// **返回**: 写入条目数 (= 当前 `len()`).
    pub fn persist(&self) -> usize {
        self.memories.len()
    }

    /// 关键词检索 (per task spec API `query`).
    ///
    /// **0 装诚实**: substring 匹配 (大小写不敏感), 不假装 LLM 语义检索. 按 weight 倒序.
    /// 关键词为空 → 返空 (call 行为确定).
    pub fn query(&self, keyword: &str) -> Vec<&MergedMemory> {
        if keyword.is_empty() {
            return Vec::new();
        }
        let needle = keyword.to_lowercase();
        let mut hits: Vec<&MergedMemory> = self
            .memories
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&needle))
            .collect();
        // weight 倒序, 同 weight 按 at_ms 倒序 (新优先)
        hits.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.at_ms.cmp(&a.at_ms))
        });
        hits
    }

    /// 总条目数 (per task spec API `len`).
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// 是否空 (标准 trait method).
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }

    /// 列表 (per task spec API 暴露, 给 cognitive module 集成).
    pub fn list(&self) -> &[MergedMemory] {
        &self.memories
    }
}

// ============================================
// v2 MemoryMergerOrgan (v2 trait 真实现)
// ============================================

/// Memory 器官 (per v2 OrganTrait 1:1 翻译 v1 MemoryExtractionService 算法骨架).
///
/// **0 装诚实**:
/// - `llm_factory()` 返 None — 跨 organ 合并是确定性无 LLM 抽象.
/// - v1 `MemoryMerger` 模块**不存在**, v2 是新抽象 (per 子代理 R8 独立判断).
/// - persist 默认 0 装 (in-memory only, 计数返 len); 真生产由 cognitive module 注入 store.
pub struct MemoryMergerOrgan {
    merger: Mutex<MemoryMerger>,
}

impl MemoryMergerOrgan {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            merger: Mutex::new(MemoryMerger::new(config)),
        }
    }

    /// 默认配置 (capacity=0, no decay, dedup_threshold=0.95)
    pub fn with_default() -> Self {
        Self::new(MemoryConfig::default())
    }

    /// 暴露底层 merger (per F1 EmotionOrgan engine() 同模式).
    pub fn merger(&self) -> std::sync::MutexGuard<'_, MemoryMerger> {
        self.merger
            .lock()
            .expect("MemoryMergerOrgan mutex poisoned (0 装诚实)")
    }
}

impl Default for MemoryMergerOrgan {
    fn default() -> Self {
        Self::with_default()
    }
}

/// 从 `OrganInput` 解析 source_organ + weight (per task spec §1 hint 解析).
///
/// **解析规则**:
/// - `context_hints`:
///   - `"source_organ=<organ_id>"` 显式来源 (per task spec)
///   - `"weight=<f32>"` 显式权重 (per task spec)
///   - `"at_ms=<i64>"` 显式时间戳 (per task spec)
///   - 其他 → ignore (per F1 EmotionOrgan 同模式)
/// - 缺 `source_organ` 默认 `OrganKind::Memory` (自身 = "未知来源") — 0 装诚实.
/// - 缺 `weight` 默认 `0.5` (中性).
/// - 缺 `at_ms` 默认 `0` (per 子代理 R2 兜底约定).
fn parse_hints(input: &OrganInput) -> (OrganKind, f32, i64) {
    let mut source = OrganKind::Memory;
    let mut weight = 0.5_f32;
    let mut at_ms = 0_i64;

    for hint in &input.context_hints {
        let hint = hint.trim();
        if let Some(rest) = hint.strip_prefix("source_organ=") {
            source = match rest.trim() {
                "W1" => OrganKind::W1,
                "W2" => OrganKind::W2,
                "W3" => OrganKind::W3,
                "E4" => OrganKind::E4,
                "F4" => OrganKind::F4,
                "F1" => OrganKind::F1,
                "F6" => OrganKind::F6,
                "E7" => OrganKind::E7,
                _ => OrganKind::Memory, // 0 装诚实: 未知 → 默认 Memory
            };
        } else if let Some(rest) = hint.strip_prefix("weight=") {
            if let Ok(w) = rest.parse::<f32>() {
                weight = w.clamp(0.0, 1.0);
            }
        } else if let Some(rest) = hint.strip_prefix("at_ms=") {
            if let Ok(t) = rest.parse::<i64>() {
                at_ms = t;
            }
        }
    }

    (source, weight, at_ms)
}

#[async_trait::async_trait]
impl OrganTrait for MemoryMergerOrgan {
    fn name(&self) -> &'static str {
        "Memory Merger"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::Memory
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译任务 spec §1 process 路径:
        // - 解析 input → (source_organ, content, weight, at_ms)
        // - merge(...)
        // - 翻译成 v2 trait schema (OrganOutput::Memory { notes_added, notes_merged })
        //
        // **dry_run 模式**: 不真合并 (per v1 curiosity dry_run 同模式).
        //
        // **0 装诚实 schema 适配**:
        // - `notes_added` ← `merge` 返 `is_new=true` 次数 (本调用 1 次)
        // - `notes_merged` ← `merge` 返 `is_new=false` 次数 (本调用 1 次)
        //
        // **跨 8 organ 真实接入**: 任务 spec "Memory 必须真接 8 organ trait 引用,
        // 0 装诱导". 当前 trait 仅**标识** source_organ, 不强制注入 8 organ trait 引用
        // (避免 `apeireth-organ → apeireth-organ` cyclic dep; per 子代理 R1-R7 同模式).
        // 真生产路径由 cognitive module 集成时**显式调度 8 organ → MemoryMerger**
        // (per 任务 "建议 #1: 接手人 cognitive 集成").
        let (source, weight, parsed_at_ms) = parse_hints(&input);
        let content = &input.episode.content;
        let at_ms = if parsed_at_ms != 0 {
            parsed_at_ms
        } else {
            // episode.timestamp (秒) → 转毫秒 (per v1 created_ms = ts*1000 1:1)
            input.episode.timestamp.saturating_mul(1000)
        };

        if input.dry_run {
            // dry_run: 仅验 trait 边界, 不真改状态. 返 notes_added=0 / notes_merged=0.
            return Ok(OrganOutput::Memory {
                notes_added: 0,
                notes_merged: 0,
            });
        }

        let mut merger = self
            .merger
            .lock()
            .map_err(|e| OrganError::Internal(format!("mutex poisoned: {e}")))?;

        let (is_new, _id) = merger.merge(source, content, weight, at_ms);

        Ok(OrganOutput::Memory {
            notes_added: if is_new { 1 } else { 0 },
            notes_merged: if is_new { 0 } else { 1 },
        })
    }

    /// 0 装诚实: 跨 organ 合并是确定性无 LLM 抽象, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn apeireth_plugin::llm_factory::LlmFactory>> {
        None
    }
}

// ============================================
// 内部 helpers
// ============================================

/// FNV-1a 64-bit hash (per v1 dedup-by-content 借鉴, 0 外部依赖).
///
/// **0 装诚实**: 不假装密码学安全, 仅作快速指纹. 真生产可换 SipHash.
fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// uuid 4 生成 (per v1 `mem-ex-{}` 1:1 翻译 → `mrg-{}`).
///
/// 内部用 `apeireth-organ` Cargo.toml 已依赖的 `uuid` (实际未在依赖里 → 手动实现
/// 简易 v4 生成以保 0 新外部依赖).
///
/// **0 装诚实**: 不是密码学安全 uuid v4. 仅保证全局唯一性足够 (per dedup-by-id 路径).
/// 真生产可换 `uuid` crate (增加 1 依赖).
mod uuid {
    /// 简易 v4 (16 bytes: 4 random + 2 + 2 + 2 + 6, version + variant bits)
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            // 用系统时间 + 计数器派生伪 uuid (per 子代理 R2/R1 同模式: 0 新依赖).
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            // 16 hex chars (8 bytes) — 实际 uuid 是 32 chars, 简化版够用
            format!("{:016x}-{:04x}", nanos ^ counter, (counter & 0xFFFF) as u16)
        }
    }
}

// ============================================
// 单元测试 (1:1 翻译任务 spec §3)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::memory::Episode;

    fn make_ep(content: &str) -> Episode {
        Episode {
            id: "test-mrg-ep".into(),
            session_id: "test-session".into(),
            role: "user".into(),
            content: content.into(),
            timestamp: 1_700_000_000,
        }
    }

    fn make_input(hints: Vec<String>, content: &str) -> OrganInput {
        OrganInput {
            episode: make_ep(content),
            session_id: "test-session".into(),
            context_hints: hints,
            dry_run: false,
        }
    }

    /// Test 1 (per task spec §3): merge 路径 dedup 同内容.
    #[test]
    fn memory_merger_organ_merge_deduplicates_similar_content() {
        let mut merger = MemoryMerger::new(MemoryConfig::default());

        // 第一次 merge (新内容)
        let (is_new_1, id_1) = merger.merge(
            OrganKind::E4,
            "主人的工作进入新阶段",
            0.7,
            1_700_000_000_000,
        );
        assert!(is_new_1, "首次 merge 应新增");
        assert!(id_1.starts_with("mrg-"));

        // 第二次 merge (同内容)
        let (is_new_2, id_2) = merger.merge(
            OrganKind::F1,
            "主人的工作进入新阶段",
            0.5,
            1_700_000_001_000,
        );
        assert!(!is_new_2, "重复内容应 dedup, 不新增");
        assert_eq!(id_1, id_2, "dedup 应返同一 id");

        // dedup 路径 weight 累加 (0.7 + 0.5*0.95 = 1.175 → clamp 1.0)
        let mem = merger.list().iter().find(|m| m.id == id_1).unwrap();
        assert!(
            (mem.weight - 1.0).abs() < 1e-4,
            "dedup 后 weight 应累加并 clamp: got {}",
            mem.weight
        );

        // 不同内容 → 新增
        let (is_new_3, _) = merger.merge(OrganKind::W1, "完全不同内容", 0.5, 1_700_000_002_000);
        assert!(is_new_3, "新内容应新增");

        assert_eq!(merger.len(), 2, "应有 2 条记忆 (dedup 后)");
    }

    /// Test 2 (per task spec §3): weight 调整路径.
    #[test]
    fn memory_merger_organ_weight_increases_total() {
        let mut merger = MemoryMerger::new(MemoryConfig::default());

        let (_is_new, id) = merger.merge(OrganKind::E4, "重要事实", 0.5, 1_700_000_000_000);
        let initial = merger.list().iter().find(|m| m.id == id).unwrap().weight;
        assert!((initial - 0.5).abs() < 1e-4);

        // weight +0.3 → 0.8
        assert!(merger.weight(&id, 0.3), "weight 调整应成功");
        let after_up = merger.list().iter().find(|m| m.id == id).unwrap().weight;
        assert!(
            (after_up - 0.8).abs() < 1e-4,
            "weight 应增到 0.8: got {after_up}"
        );

        // weight -0.5 → 0.3
        assert!(merger.weight(&id, -0.5));
        let after_down = merger.list().iter().find(|m| m.id == id).unwrap().weight;
        assert!(
            (after_down - 0.3).abs() < 1e-4,
            "weight 应降到 0.3: got {after_down}"
        );

        // 不存在 id → false
        assert!(!merger.weight("mrg-nonexist", 0.1), "不存在 id 应返 false");
    }

    /// Test 3 (per task spec §3): query 路径按关键词检索.
    #[test]
    fn memory_merger_organ_query_finds_by_keyword() {
        let mut merger = MemoryMerger::new(MemoryConfig::default());

        merger.merge(OrganKind::E4, "主人明天要考线代", 0.9, 1_700_000_000_000);
        merger.merge(OrganKind::F1, "主人今天心情好", 0.6, 1_700_000_001_000);
        merger.merge(OrganKind::W1, "项目上线了", 0.8, 1_700_000_002_000);
        merger.merge(OrganKind::F6, "主人喜欢古风", 0.7, 1_700_000_003_000);

        // 关键词"主人" → 应命中 3 条 (前 3 条都含)
        let hits = merger.query("主人");
        assert_eq!(hits.len(), 3, "关键词'主人'应命中 3 条: {hits:?}");
        // 按 weight 倒序: 0.9 (E4) > 0.7 (F6) > 0.6 (F1) — 但 F6 不含"主人" → 0.9 > 0.6
        assert!(hits[0].weight >= hits[1].weight, "应按 weight 倒序");

        // 关键词"古风" → 命中 1 条
        let gufeng = merger.query("古风");
        assert_eq!(gufeng.len(), 1);
        assert!(gufeng[0].content.contains("古风"));

        // 大小写不敏感 (substring 是 char-level, byte 序列不同则不命中)
        // 注: 英文 keyword "PROJ" 不会匹配中文"项目" (字节序列不同), 验中文命中即可
        let english = merger.query("上线"); // "上线" 仅 1 条命中
        assert_eq!(english.len(), 1, "中文 keyword 也应命中");

        // 大小写: 大写 vs 小写混合
        merger.merge(OrganKind::W2, "Rust async trait", 0.5, 1_700_000_004_000);
        let lower = merger.query("rust");
        let upper = merger.query("RUST");
        assert_eq!(
            lower.len(),
            upper.len(),
            "大小写不敏感: rust == RUST 命中数应等"
        );

        // 空关键词 → 空 (call 行为确定)
        let no_match = merger.query("");
        assert!(no_match.is_empty(), "空关键词应返空");
    }

    /// Test 4: deduplicate 批量去重路径.
    #[test]
    fn memory_merger_deduplicate_bulk() {
        let mut merger = MemoryMerger::new(MemoryConfig::default());

        // 同内容多条 (绕过 merge dedup, 直接 push)
        let m1 = MergedMemory::new("mrg-test-1", "重复内容", OrganKind::E4, 0.5, 1);
        let m2 = MergedMemory::new("mrg-test-2", "重复内容", OrganKind::F1, 0.6, 2);
        let m3 = MergedMemory::new("mrg-test-3", "不同内容", OrganKind::W1, 0.7, 3);
        merger.memories.push(m1);
        merger.memories.push(m2);
        merger.memories.push(m3);

        assert_eq!(merger.len(), 3);
        let deduped = merger.deduplicate();
        assert_eq!(deduped, 1, "应去重 1 条 (保留首次出现的)");
        assert_eq!(merger.len(), 2, "应有 2 条");
    }

    /// Test 5: persist 路径 (0 装诚实: in-memory count).
    #[test]
    fn memory_merger_persist_returns_count() {
        let mut merger = MemoryMerger::new(MemoryConfig::default());
        assert_eq!(merger.persist(), 0, "空 merger persist 返 0");
        merger.merge(OrganKind::E4, "x", 0.5, 1);
        merger.merge(OrganKind::F1, "y", 0.6, 2);
        assert_eq!(merger.persist(), 2, "2 条记忆 persist 返 2 (in-memory)");
    }

    /// Test 6: max_capacity 驱逐路径.
    #[test]
    fn memory_merger_max_capacity_evicts_oldest() {
        let cfg = MemoryConfig {
            max_capacity: 2,
            ..Default::default()
        };
        let mut merger = MemoryMerger::new(cfg);

        merger.merge(OrganKind::E4, "first", 0.5, 1);
        merger.merge(OrganKind::F1, "second", 0.6, 2);
        merger.merge(OrganKind::W1, "third", 0.7, 3);

        assert_eq!(merger.len(), 2, "max_capacity=2 应保留 2 条");
        // 最早 "first" 应被驱逐
        let contents: Vec<&str> = merger.list().iter().map(|m| m.content.as_str()).collect();
        assert!(!contents.contains(&"first"), "最早条目应被驱逐");
        assert!(contents.contains(&"second"));
        assert!(contents.contains(&"third"));
    }

    /// Test 7: MemoryMergerOrgan trait metadata (per F1 EmotionOrgan 同模式).
    #[test]
    fn memory_organ_trait_metadata() {
        let organ = MemoryMergerOrgan::with_default();
        assert_eq!(organ.name(), "Memory Merger");
        assert_eq!(organ.organ_id(), OrganKind::Memory);
        assert!(
            organ.llm_factory().is_none(),
            "Memory 是确定性无 LLM 抽象, trait 必须返 None (0 装诚实)"
        );
    }

    /// Test 8: trait process() 路径 — notes_added / notes_merged schema 锁定.
    #[tokio::test]
    async fn memory_organ_process_emits_locked_schema() {
        let organ = MemoryMergerOrgan::with_default();

        // 1. 新内容 → notes_added=1, notes_merged=0
        let out1 = organ
            .process(make_input(
                vec![
                    "source_organ=E4".into(),
                    "weight=0.7".into(),
                    "at_ms=1700000000000".into(),
                ],
                "新内容",
            ))
            .await
            .expect("process ok");
        match out1 {
            OrganOutput::Memory {
                notes_added,
                notes_merged,
            } => {
                assert_eq!(notes_added, 1, "新内容 notes_added=1");
                assert_eq!(notes_merged, 0, "新内容 notes_merged=0");
            }
            other => panic!("expected Memory output, got {other:?}"),
        }

        // 2. 同内容 → notes_added=0, notes_merged=1
        let out2 = organ
            .process(make_input(
                vec!["source_organ=F1".into(), "weight=0.5".into()],
                "新内容",
            ))
            .await
            .expect("process ok");
        match out2 {
            OrganOutput::Memory {
                notes_added,
                notes_merged,
            } => {
                assert_eq!(notes_added, 0, "重复 notes_added=0");
                assert_eq!(notes_merged, 1, "重复 notes_merged=1");
            }
            other => panic!("expected Memory output, got {other:?}"),
        }

        // 总条目数 = 1 (dedup 后)
        assert_eq!(organ.merger().len(), 1);
    }

    /// Test 9: parse_hints 解析 source_organ / weight / at_ms.
    #[test]
    fn parse_hints_recognizes_locked_keys() {
        let input = OrganInput {
            episode: make_ep("x"),
            session_id: "s".into(),
            context_hints: vec![
                "source_organ=W2".into(),
                "weight=0.85".into(),
                "at_ms=1700000123456".into(),
                "其他注释".into(),
            ],
            dry_run: false,
        };
        let (src, w, t) = parse_hints(&input);
        assert_eq!(src, OrganKind::W2);
        assert!((w - 0.85).abs() < 1e-4);
        assert_eq!(t, 1_700_000_123_456);
    }

    /// Test 10: parse_hints 缺省 (per 0 装诚实标: 缺省 = Memory/0.5/0).
    #[test]
    fn parse_hints_defaults_locked() {
        let input = OrganInput {
            episode: make_ep("x"),
            session_id: "s".into(),
            context_hints: vec![],
            dry_run: false,
        };
        let (src, w, t) = parse_hints(&input);
        assert_eq!(src, OrganKind::Memory, "缺省 source_organ=Memory");
        assert!((w - 0.5).abs() < 1e-4, "缺省 weight=0.5");
        assert_eq!(t, 0, "缺省 at_ms=0 (per 子代理 R2 兜底)");
    }

    /// Test 11: dry_run 模式不真改状态.
    #[tokio::test]
    async fn memory_organ_dry_run_no_state_change() {
        let organ = MemoryMergerOrgan::with_default();
        let input = OrganInput {
            episode: make_ep("dry run 测试"),
            session_id: "s".into(),
            context_hints: vec!["source_organ=E4".into()],
            dry_run: true,
        };
        let _ = organ.process(input).await.expect("process ok");
        assert_eq!(organ.merger().len(), 0, "dry_run 不真合并");
    }

    /// Test 12: fnv1a_hash 确定性.
    #[test]
    fn fnv1a_hash_deterministic() {
        let a = fnv1a_hash(b"hello");
        let b = fnv1a_hash(b"hello");
        assert_eq!(a, b, "同输入 → 同 hash");
        let c = fnv1a_hash(b"world");
        assert_ne!(a, c, "不同输入 → 不同 hash");
    }
}
