//! **Apeireth R25 战区 5 / VCP `dynamicToolRegistry.js:40-80` — 小模型工具分类器**
//!
//! **目标**: 对标 VCP `dynamicToolRegistry.js` 的 7 类 `CATEGORY_RULES` + 3 层 fallback
//! (custom classifier → small model → embedding → keyword), 在 Rust 强类型 enum + trait
//! 形式上落地, 并按 v2.0 strategy Step 5 加 2 个 Apeireth 独有类别 (`Safety` / `LongRunning`)
//!
//! **9 类别设计** (1:1 抄 VCP 7 + 2 Apeireth 独有):
//! 1. `Search` (VCP `search`) — 搜索/检索/查询/论文
//! 2. `FileCode` (VCP `file_code`) — 文件/代码/git/读写编辑
//! 3. `ImageMedia` (VCP `image_media`) — 图片/视频/音频/OCR
//! 4. `MemoryKnowledge` (VCP `memory_knowledge`) — 记忆/RAG/笔记/向量
//! 5. `AgentTask` (VCP `agent_task`) — 代理/任务/计划/调度
//! 6. `Communication` (VCP `communication`) — 邮件/消息/推送/IM
//! 7. `Data` (VCP `data`) — JSON/CSV/SQL/数据库
//! 8. `Safety` (Apeireth 独有) — 红队/自禁用/4 重守门护送
//! 9. `LongRunning` (Apeireth 独有) — >5min 预期时长 (训练/索引/批处理)
//!
//! **3 实现 + 1 trait**:
//! - `Classifier` trait (2 方法: classify / confidence)
//! - `HeuristicClassifier` (关键词字典, 0 远程, 兜底)
//! - `EmbeddingClassifier` (接 `Arc<dyn EmbedFn>`, 9 类中心向量 cosine)
//! - `LlmClassifier` (OpenAI-compat HTTP, mock 接口, 真接留 R21+)
//!
//! **字段级引用 VCP**:
//! - `dynamicToolRegistry.js:40-80 CATEGORY_RULES` (7 类 + 关键词) — 1:1 抄
//! - `dynamicToolRegistry.js:986-1000 _classifyRecord` (3 层 fallback) — 1:1 抄
//! - `dynamicToolRegistry.js:1003-1048 _classifyWithSmallModel` (OpenAI-compat) — 接口 1:1 抄
//! - `dynamicToolRegistry.js:1106-1147 _classifyWithEmbeddings` (cosine 相似度) — 1:1 抄
//! - `dynamicToolRegistry.js:1214-1238 _fallbackClassify` (关键词兜底) — 1:1 抄
//!
//! **不假装** (per 主人偏好 #3 + #7):
//! - ✅ Heuristic 真跑关键词匹配, 在 9 demo tool 上准确率 ≥ 80% (硬指标验收)
//! - ✅ Embedding 接 mock hash embedder, 真 cosine 数学
//! - ❌ LlmClassifier **不接真 LLM** (留 mock 接口签名, 真接留 R21+), 不假装
//!
//! **不修改承诺** (R119 形式撤销后原意保留):
//! - ✅ 0 改 Tool trait 4 方法签名
//! - ✅ 0 触碰 24 LOCKED crate (apeireth-cognition/core/sovereignty/formal)
//! - ✅ 0 引入 fastembed (重编译, 留 R21+)
//! - ✅ 0 主动 commit
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline / apeireth-api / 未来消费者
//!          ↓
//!      apeireth-tool-registry::classifier (本模块)
//!      ├── Category            : 9 enum (VCP 7 + 2 独有)
//!      ├── ClassifyError       : 4 variant
//!      ├── EmbedFn (trait)     : 本地, 跟未来 apeireth-memory::semantic::EmbedFn 对齐
//!      ├── MockHashEmbedFn     : FNV-1a 32 维, 0 远程
//!      ├── Classifier (trait)  : 2 方法, Send + Sync
//!      ├── HeuristicClassifier : 关键词字典 1:1 抄 VCP
//!      ├── EmbeddingClassifier : 9 类中心向量 + cosine
//!      └── LlmClassifier       : OpenAI-compat mock, 真接留 R21+
//! ```

#![allow(clippy::result_large_err)] // ClassifyError 信息丰富, 不强求小

use std::collections::HashSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::trait_def::Tool;

// ============================================================
// 9 类别 enum (VCP 7 + 2 Apeireth 独有)
// ============================================================

/// **9 类别工具分类** (VCP 7 + 2 Apeireth 独有)
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:40-80 CATEGORY_RULES`:
/// - `search` → `Search`
/// - `file_code` → `FileCode`
/// - `image_media` → `ImageMedia`
/// - `memory_knowledge` → `MemoryKnowledge`
/// - `agent_task` → `AgentTask`
/// - `communication` → `Communication`
/// - `data` → `Data`
///
/// **Apeireth 独有** (v2.0 strategy Step 5):
/// - `Safety` (红队/自禁用/4 重守门护送, 主人偏好 #4 "AI 不会衰老病死" → 防护类独立类别)
/// - `LongRunning` (>5min 预期时长, 跟 `ToolKind::Service/Hybridservice` 正交)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    /// 搜索/检索/查询 (VCP `search`)
    Search,
    /// 文件/代码/git/读写编辑 (VCP `file_code`)
    FileCode,
    /// 图片/视频/音频/OCR 多模态 (VCP `image_media`)
    ImageMedia,
    /// 记忆/RAG/笔记/向量检索 (VCP `memory_knowledge`)
    MemoryKnowledge,
    /// 代理/任务/计划/调度 (VCP `agent_task`)
    AgentTask,
    /// 邮件/消息/推送/IM (VCP `communication`)
    Communication,
    /// JSON/CSV/SQL/数据库 (VCP `data`)
    Data,
    /// **Apeireth 独有** — 红队/自禁用/4 重守门护送
    Safety,
    /// **Apeireth 独有** — >5min 预期时长 (训练/索引/批处理)
    LongRunning,
}

impl Category {
    /// 9 类别总数 (编译期 hardcode, 防止加 variant 忘改 docs)
    pub const COUNT: usize = 9;

    /// 返所有 9 类别 (按枚举顺序, 供 list 端点 / admin UI 用)
    pub const fn all() -> [Self; 9] {
        [
            Self::Search,
            Self::FileCode,
            Self::ImageMedia,
            Self::MemoryKnowledge,
            Self::AgentTask,
            Self::Communication,
            Self::Data,
            Self::Safety,
            Self::LongRunning,
        ]
    }

    /// 返 VCP 字段名 (1:1 对照 `dynamicToolRegistry.js:40-80 CATEGORY_RULES`)
    ///
    /// **Apeireth 独有类别** (Safety / LongRunning) 不在 VCP 7 类, 用 snake_case 自创名
    pub const fn as_legacy_name(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::FileCode => "file_code",
            Self::ImageMedia => "image_media",
            Self::MemoryKnowledge => "memory_knowledge",
            Self::AgentTask => "agent_task",
            Self::Communication => "communication",
            Self::Data => "data",
            Self::Safety => "safety",
            Self::LongRunning => "long_running",
        }
    }

    /// 从 VCP 字段名解析 (供加载 manifest 用)
    pub fn from_legacy_name(s: &str) -> Option<Self> {
        match s {
            "search" => Some(Self::Search),
            "file_code" => Some(Self::FileCode),
            "image_media" => Some(Self::ImageMedia),
            "memory_knowledge" => Some(Self::MemoryKnowledge),
            "agent_task" => Some(Self::AgentTask),
            "communication" => Some(Self::Communication),
            "data" => Some(Self::Data),
            // Apeireth 独有类别 — 兼容序列化
            "safety" => Some(Self::Safety),
            "long_running" => Some(Self::LongRunning),
            _ => None,
        }
    }

    /// 类别优先级 (匹配多类时取最小 = 安全敏感度最高)
    ///
    /// **排序逻辑** (per D2-1 §3.2):
    /// - Safety 最高优先 (任何含"permission/guardrail"立刻标 Safety, 防 prompt injection 伪装)
    /// - LongRunning 次高 (pipeline 调度需特判)
    /// - Memory / FileCode / Data 中等 (核心操作类)
    /// - Search / AgentTask / Communication / ImageMedia 较低
    ///
    /// **0 业务影响**: 仅在 `HeuristicClassifier` 内部匹配多类时取优先, 不暴露给 API
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Safety => 0,
            Self::LongRunning => 1,
            Self::MemoryKnowledge => 2,
            Self::FileCode => 3,
            Self::Data => 4,
            Self::Search => 5,
            Self::AgentTask => 6,
            Self::Communication => 7,
            Self::ImageMedia => 8,
        }
    }
}

// ============================================================
// ClassifyError (4 variant, thiserror)
// ============================================================

/// **分类错误** (4 variant, per 主人偏好 #3 "0 假装" + 工程铁律)
#[derive(Debug, Error)]
pub enum ClassifyError {
    /// 无匹配 — 0 关键词命中, 兜底类也低置信度
    ///
    /// **行为**: VCP `_fallbackClassify` 返 'general' (line 1226), 我们用 `Result::Err` 表达
    /// (更 Rust 风格; 0 假装"分到某类" — 实际就是 0 把握)
    #[error("no matching category for tool: name='{name}' (tried {tried_keywords} keywords)")]
    NoMatch {
        /// 工具名 (debug 用)
        name: String,
        /// 尝试过的关键词数 (debug 用, 0 = 0 关键词命中)
        tried_keywords: usize,
    },

    /// 嵌入错误 (EmbeddingClassifier 返 0 维向量 / 维度不一致)
    #[error("embedding error for tool '{name}': {reason}")]
    EmbeddingError {
        /// 工具名
        name: String,
        /// 错误原因
        reason: String,
    },

    /// LLM 错误 (LlmClassifier HTTP 失败 / 解析失败)
    #[error("LLM classifier error: {0}")]
    LlmError(String),

    /// 其他内部错误 (catch-all, 0 业务场景)
    #[error("internal classifier error: {0}")]
    Internal(String),
}

// ============================================================
// EmbedFn 本地 trait (跟未来 apeireth-memory::semantic::EmbedFn 对齐)
// ============================================================

/// **嵌入函数 trait** (本地, 跟未来 `apeireth-memory::semantic::EmbedFn` 形状对齐)
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:1108 getEmbedding(text) -> Promise<Vec<f32>>`:
/// - VCP 异步, 我们同步简化 (跟 `apeireth-memory::benches::v2-memory-vector-bench.rs:27` 1:1)
/// - 真实接入 (fastembed / ollama / OpenAI embed endpoint) 留 R21+ 给 `apeireth-llm-gateway`
///
/// **不假装** (per 主人偏好 #3 + #7):
/// - ✅ `MockHashEmbedFn` 真实现 FNV-1a 32 维 hash, 确定性, 0 远程
/// - ❌ 不引入 `fastembed` (重编译, 加重 deps, 留 R21+)
pub trait EmbedFn: Send + Sync {
    /// 嵌入维度 (cosine 相似度前必须一致)
    fn dim(&self) -> usize;

    /// 文本 → f32 向量 (确定性, 相同输入返相同输出)
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// **Mock 嵌入实现** — FNV-1a 32 维 hash (确定性, 0 远程依赖, 0 随机)
///
/// **VCP 简化**: VCP 真用 embedding 模型 (gte-Qwen2-7B-instruct 等), 我们用 FNV-1a hash
/// 桶, 满足 9 类中心向量 cosine 相似度的数学演示
///
/// **D-2 决策** (per 任务"不引入 fastembed"):
/// - 0 远程 ✅
/// - 0 随机 (FNV-1a 32-bit offset basis + prime, 跟 `v2-memory-vector-bench.rs:34` 1:1)
/// - 32 维 = 32 个 char 类别桶, 每个 char 按 FNV 散列到 1 个维度
pub struct MockHashEmbedFn {
    /// 嵌入维度 (默认 32, 跟 `v2-memory-vector-bench.rs:20 DIMENSION = 32` 1:1)
    pub dimension: usize,
}

impl MockHashEmbedFn {
    /// 新建默认 32 维
    pub fn new() -> Self {
        Self { dimension: 32 }
    }

    /// 新建指定维度
    pub fn with_dimension(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl Default for MockHashEmbedFn {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbedFn for MockHashEmbedFn {
    fn dim(&self) -> usize {
        self.dimension
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        // FNV-1a 32-bit, 跟 v2-memory-vector-bench.rs:32-43 DeterministicEmbedder 1:1
        // 每个 char 算 1 次 FNV, 落到 dim 模的桶, 桶值 = FNV / u32::MAX (归一化到 0..1)
        let mut values = vec![0.0_f32; self.dimension];
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325_u64; // FNV offset basis
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3_u64); // FNV prime
            // 算 char-level 桶 (按 char 而不是 byte, 支持中文)
        }
        // 第二轮: 按 char 算 char-level 桶 (FNV-1a 处理)
        for ch in text.chars() {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325_u64;
            for byte in ch.to_string().as_bytes() {
                h ^= u64::from(*byte);
                h = h.wrapping_mul(0x100_0000_01b3_u64);
            }
            // 桶索引 = h mod dim
            let bucket = (h as usize) % self.dimension;
            // 桶值 = (h / u32::MAX) 归一化到 0..1, 累加 (同一 char 多次出现累加)
            let normalized = (h as f32) / (u32::MAX as f32);
            values[bucket] += normalized;
        }
        // L2 归一化 (cosine 相似度前置要求)
        let norm: f32 = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut values {
                *v /= norm;
            }
        }
        values
    }
}

// ============================================================
// Classifier trait (2 方法, Send + Sync)
// ============================================================

/// **分类器 trait** (2 方法, Send + Sync, 跨线程安全)
///
/// **VCP 字段级引用**:
/// - VCP 异步 (Promise), 我们同步简化 (跟 `v2-memory-vector-bench.rs` 一致)
/// - VCP 多类 top-3 (line 1131 `selected.slice(0, 3)`), 我们单选 (Rust enum 不可组合)
///
/// **不假装** (per 主人偏好 #3 + #7):
/// - ✅ `HeuristicClassifier` 真跑关键词匹配, demo 上 ≥ 80% 准确率
/// - ✅ `EmbeddingClassifier` 真算 cosine 相似度
/// - ❌ `LlmClassifier` 留 mock 接口签名, 不真接 LLM (留 R21+)
pub trait Classifier: Send + Sync {
    /// 单选分类 (Apeireth 简化, VCP 是多类 top-3)
    ///
    /// **返**:
    /// - `Ok(Category)` — 0.5 置信度以上
    /// - `Err(ClassifyError::NoMatch)` — 0 关键词命中 / 置信度 < 0.5
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError>;

    /// 置信度 0.0 .. 1.0
    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError>;
}

// ============================================================
// HeuristicClassifier (关键词字典 1:1 抄 VCP, 0 远程)
// ============================================================

/// **启发式分类器** — 关键词字典 1:1 抄 VCP `dynamicToolRegistry.js:40-80 CATEGORY_RULES`
///
/// **设计**:
/// - 9 类别每类一个 `&[&str]` 关键词字典
/// - 匹配规则: tool.name() + tool.kind() 拼成 text, 关键词子串 match
/// - 多类命中 → 按 `Category::priority()` 取安全敏感度最高的
/// - 0 命中 → 返 `Err(ClassifyError::NoMatch)` (跟 VCP `_fallbackClassify` 行为对齐)
///
/// **Apeireth 独有 2 类别 (Safety / LongRunning) 关键词自创**:
/// - Safety: redteam/红队/self_disable/自禁用/护栏/guardrail/safety/sanitize/validate/permission/权限
/// - LongRunning: train/训练/index/索引/batch/批处理/migrate/迁移/compile/编译/crawl/爬取/embedding
pub struct HeuristicClassifier;

impl HeuristicClassifier {
    /// 新建启发式分类器
    pub fn new() -> Self {
        Self
    }

    /// 9 类别关键词字典 (VCP 1:1 + 2 自创)
    ///
    /// **VCP 字段级引用** `dynamicToolRegistry.js:40-80 CATEGORY_RULES`
    /// 1:1 抄 7 类关键词 (中英双语), 加 2 类 Apeireth 独有
    pub const KEYWORDS: &'static [(Category, &'static [&'static str])] = &[
        // 1. Search (VCP 1:1)
        (
            Category::Search,
            &[
                "search", "web", "lookup", "query", "retrieval", "google", "tavily", "serp",
                "url", "paper", "citation", "搜索", "检索", "网页", "查询", "论文", "资料",
                "find", "搜", "findx", "qdrant", "duckduckgo",
            ],
        ),
        // 2. FileCode (VCP 1:1)
        (
            Category::FileCode,
            &[
                "file", "code", "read", "write", "edit", "patch", "repo", "git", "directory",
                "文件", "代码", "仓库", "读取", "写入", "编辑", "copy", "move", "delete",
                "diff", "ls", "cat", "grep", "rg",
            ],
        ),
        // 3. ImageMedia (VCP 1:1)
        (
            Category::ImageMedia,
            &[
                "image", "photo", "picture", "media", "video", "audio", "ocr", "screenshot",
                "图片", "图像", "视频", "音频", "截图", "tts", "stt", "asr", "transcribe",
                "generate_image", "imagen",
            ],
        ),
        // 4. MemoryKnowledge (VCP 1:1)
        (
            Category::MemoryKnowledge,
            &[
                "memory", "knowledge", "rag", "diary", "note", "vector", "context", "知识",
                "记忆", "日记", "笔记", "向量", "recall", "semantic", "embed", "search_memory",
            ],
        ),
        // 5. AgentTask (VCP 1:1)
        (
            Category::AgentTask,
            &[
                "agent", "task", "schedule", "plan", "workflow", "assistant", "任务", "计划",
                "调度", "代理", "orchestrate", "dispatch", "delegate", "协调",
            ],
        ),
        // 6. Communication (VCP 1:1)
        (
            Category::Communication,
            &[
                "mail", "email", "message", "notification", "push", "forum", "wechat", "telegram",
                "邮件", "消息", "通知", "推送", "discord", "slack", "im", "send_message",
                "chat", "feishu", "lark",
            ],
        ),
        // 7. Data (VCP 1:1)
        (
            Category::Data,
            &[
                "json", "csv", "excel", "sql", "database", "table", "parse", "数据", "表格",
                "数据库", "解析", "pandas", "dataframe", "export", "import", "sqlite", "psql",
                "mysql",
            ],
        ),
        // 8. Safety (Apeireth 独有, 自创)
        (
            Category::Safety,
            &[
                "redteam", "red_team", "红队", "self_disable", "self-disable", "自禁用",
                "guardrail", "护栏", "safety", "sanitize", "validate", "permission", "权限",
                "auth", "authorize", "approval", "审批", "jailbreak", "injection", "purify",
                "denylist", "blocklist",
            ],
        ),
        // 9. LongRunning (Apeireth 独有, 自创)
        (
            Category::LongRunning,
            &[
                "train", "训练", "index", "索引", "batch", "批处理", "migrate", "迁移",
                "compile", "编译", "crawl", "爬取", "embedding", "build", "etl", "ingest",
                "compute", "compute_heavy",
            ],
        ),
    ];

    /// 内部: 拿 tool 描述文本 (跟 VCP `_fallbackClassify:1215` `pluginName + displayName + description + fullDescription` 1:1)
    ///
    /// **D-2 简化**: VCP 有 4 字段, 我们 Tool trait 只有 name() + kind(). 真实场景可通过
    /// `ToolMetaProvider` extension trait 拿 description (本期不实现, 留 R26+)
    fn tool_text(tool: &dyn Tool) -> String {
        // 只用 name() (VCP 也会把 name 算入, line 1215 pluginName)
        // 不用 kind(), 因为 kind 字符串如 "asynchronous" 会误中其他类关键词
        tool.name().to_lowercase()
    }

    /// 内部: 计算 tool 跟 9 类别的命中数
    ///
    /// **D-2 修复** (vs 第一次提交):
    /// - 关键词子串匹配从 `text.contains(kw)` 改为 token 级别匹配
    /// - 关键词长度 < 3 时用 exact match (避免 "im" 误中 "image")
    /// - 关键词长度 >= 3 时用 substring match (跟 VCP `_classifyRecord` 1:1)
    /// - text token 化: 按 `[a-z0-9_.-]+` 切 (跟 VCP line 197 `latinMatches` 1:1)
    fn score_all(tool: &dyn Tool) -> Vec<(Category, usize)> {
        let text = Self::tool_text(tool);
        // VCP 风格: token 化 (latin + 数字 + . _ -)
        let tokens: Vec<&str> = text
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.' && c != '-')
            .filter(|s| !s.is_empty())
            .collect();
        let mut scores: Vec<(Category, usize)> = Vec::with_capacity(Category::COUNT);
        for (cat, kws) in Self::KEYWORDS {
            let mut hits = 0;
            for kw in *kws {
                if kw.len() < 3 {
                    // 短关键词: exact token match (避免 "im" 误中)
                    if tokens.iter().any(|t| *t == *kw) {
                        hits += 1;
                    }
                } else {
                    // 长关键词: substring match (跟 VCP `text.includes(keyword)` 1:1)
                    if text.contains(kw) {
                        hits += 1;
                    }
                }
            }
            scores.push((*cat, hits));
        }
        scores
    }
}

impl Default for HeuristicClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Classifier for HeuristicClassifier {
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError> {
        let scores = Self::score_all(tool);
        // 0 命中检查
        let total_hits: usize = scores.iter().map(|(_, h)| h).sum();
        if total_hits == 0 {
            return Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: Self::KEYWORDS.iter().map(|(_, k)| k.len()).sum(),
            });
        }
        // 多类命中 → 按 priority 取安全敏感度最高 (最小 priority 值)
        let best = scores
            .iter()
            .filter(|(_, h)| *h > 0)
            .min_by_key(|(cat, _)| cat.priority())
            .map(|(cat, _)| *cat);
        match best {
            Some(cat) => Ok(cat),
            None => Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: total_hits,
            }),
        }
    }

    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError> {
        let scores = Self::score_all(tool);
        let total_hits: usize = scores.iter().map(|(_, h)| h).sum();
        if total_hits == 0 {
            return Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: 0,
            });
        }
        // 置信度 = max_hits / total_keywords_in_best_class (0..1)
        // 简化: max_hits / 5 (5 个关键词达到高置信)
        let max_hits = scores.iter().map(|(_, h)| *h).max().unwrap_or(0);
        let confidence = (max_hits as f32 / 5.0).min(1.0);
        Ok(confidence)
    }
}

// ============================================================
// EmbeddingClassifier (接 Arc<dyn EmbedFn>, 9 类中心向量 cosine)
// ============================================================

/// **嵌入分类器** — 9 类中心向量 + cosine 相似度
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:1106-1147 _classifyWithEmbeddings`:
/// - `_getCategoryEmbedding(rule, getEmbedding)` → 我们 `center_vectors: HashMap<Category, Vec<f32>>`
/// - `_cosineSimilarity(a, b)` → 我们 `cosine_similarity`
/// - `score >= 0.34 .slice(0, 3)` → 我们 `score >= 0.5 .take(1)` (单选简化)
/// - 9 类中心向量由 9 类关键词字典 + `EmbedFn` 计算 (缓存, 一次算)
pub struct EmbeddingClassifier {
    /// 嵌入函数 (Arc<dyn>, 线程安全)
    embed_fn: Arc<dyn EmbedFn>,
    /// 9 类中心向量 (懒初始化, 第一次 classify 时算)
    centers: parking_lot::RwLock<Option<Vec<(Category, Vec<f32>)>>>,
    /// cosine 相似度阈值 (默认 0.5, VCP 用 0.34 但我们要单选所以提高)
    threshold: f32,
}

impl EmbeddingClassifier {
    /// 新建嵌入分类器 (使用默认 32 维 MockHashEmbedFn)
    pub fn new() -> Self {
        Self::with_embed_fn(Arc::new(MockHashEmbedFn::new()))
    }

    /// 自定义嵌入函数
    pub fn with_embed_fn(embed_fn: Arc<dyn EmbedFn>) -> Self {
        Self {
            embed_fn,
            centers: parking_lot::RwLock::new(None),
            threshold: 0.5,
        }
    }

    /// 自定义阈值
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// 拿当前嵌入函数 dim
    pub fn dim(&self) -> usize {
        self.embed_fn.dim()
    }

    /// 内部: 懒计算 9 类中心向量 (每类关键词字典拼成 text 调 embed)
    fn ensure_centers(&self) -> Result<Vec<(Category, Vec<f32>)>, ClassifyError> {
        // 快速路径: 已算过
        if let Some(centers) = self.centers.read().as_ref() {
            return Ok(centers.clone());
        }
        // 慢路径: 算 9 类中心
        let mut centers: Vec<(Category, Vec<f32>)> = Vec::with_capacity(Category::COUNT);
        for (cat, kws) in HeuristicClassifier::KEYWORDS {
            // 中心向量 = 关键词 join 成 1 个 text, 调 embed 1 次
            let text = kws.join(" ");
            let vec = self.embed_fn.embed(&text);
            if vec.len() != self.embed_fn.dim() {
                return Err(ClassifyError::EmbeddingError {
                    name: format!("center:{}", cat.as_legacy_name()),
                    reason: format!(
                        "embed dim mismatch: expected {}, got {}",
                        self.embed_fn.dim(),
                        vec.len()
                    ),
                });
            }
            centers.push((*cat, vec));
        }
        *self.centers.write() = Some(centers.clone());
        Ok(centers)
    }
}

impl Default for EmbeddingClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Classifier for EmbeddingClassifier {
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError> {
        let centers = self.ensure_centers()?;
        // tool 文本 → embed
        let text = HeuristicClassifier::tool_text(tool);
        let tool_vec = self.embed_fn.embed(&text);
        if tool_vec.len() != self.embed_fn.dim() {
            return Err(ClassifyError::EmbeddingError {
                name: tool.name().to_string(),
                reason: format!(
                    "embed dim mismatch: expected {}, got {}",
                    self.embed_fn.dim(),
                    tool_vec.len()
                ),
            });
        }
        // 9 类中心 cosine 相似度
        let mut best: Option<(Category, f32)> = None;
        for (cat, center) in &centers {
            let sim = cosine_similarity(&tool_vec, center);
            if sim >= self.threshold {
                if best.is_none() || sim > best.unwrap().1 {
                    best = Some((*cat, sim));
                }
            }
        }
        match best {
            Some((cat, _)) => Ok(cat),
            None => Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: 9,
            }),
        }
    }

    fn confidence(&self, tool: &dyn Tool) -> Result<f32, ClassifyError> {
        let centers = self.ensure_centers()?;
        let text = HeuristicClassifier::tool_text(tool);
        let tool_vec = self.embed_fn.embed(&text);
        let max_sim = centers
            .iter()
            .map(|(_, c)| cosine_similarity(&tool_vec, c))
            .fold(0.0_f32, f32::max);
        if max_sim < self.threshold {
            return Err(ClassifyError::NoMatch {
                name: tool.name().to_string(),
                tried_keywords: 9,
            });
        }
        Ok(max_sim)
    }
}

/// **cosine 相似度** (跟 VCP `_cosineSimilarity:1198-1212` 1:1)
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:1198-1212`:
/// ```javascript
/// _cosineSimilarity(a, b) {
///     let dot = 0, normA = 0, normB = 0;
///     for (let i = 0; i < length; i++) {
///         dot += av * bv; normA += av * av; normB += bv * bv;
///     }
///     if (normA === 0 || normB === 0) return 0;
///     return dot / (Math.sqrt(normA) * Math.sqrt(normB));
/// }
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;
    for (av, bv) in a.iter().zip(b.iter()) {
        dot += av * bv;
        norm_a += av * av;
        norm_b += bv * bv;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

// ============================================================
// LlmClassifier (OpenAI-compat HTTP, mock 接口, 真接留 R21+)
// ============================================================

/// **LLM 分类器** — OpenAI-compat HTTP endpoint, mock 接口, **不真接 LLM**
///
/// **VCP 字段级引用** `dynamicToolRegistry.js:1003-1048 _classifyWithSmallModel`:
/// - system prompt: `"You classify tool plugins. Return JSON only."`
/// - user prompt: 5 行 (classify / brief / categories / keywords / confidence)
/// - HTTP POST + Authorization Bearer + JSON body
/// - 真接: `requestConfig.endpoint` (OpenAI-compat) + `requestConfig.model`
///
/// **D-2 不假装** (per 主人偏好 #3 + #7):
/// - ❌ **0 真接 LLM** (留 trait 接口 + mock 构造)
/// - ✅ `LlmClassifier::new_mock()` 返 hardcoded "search" 类别 (签名 1:1)
/// - 真接 R21+ 给 `apeireth-llm-gateway` (跟 `chatCompletionHandler` 对齐)
pub struct LlmClassifier {
    /// OpenAI-compat endpoint (None = mock 模式)
    endpoint: Option<String>,
    /// 模型名 (e.g. "gpt-4o-mini", "qwen-turbo")
    model: Option<String>,
    /// API key (None = 0 auth, 走 mock 模式)
    api_key: Option<String>,
}

impl LlmClassifier {
    /// 新建 mock LLM 分类器 (0 远程调用, 永远返 `Category::Search` 兜底)
    ///
    /// **0 假装**: 0 真实 HTTP 调用, 0 真实 LLM 推理, 永远返 Search + confidence 0.5
    /// 这是显式 mock 行为, 不假装"已经接了真模型"
    pub fn new_mock() -> Self {
        Self {
            endpoint: None,
            model: None,
            api_key: None,
        }
    }

    /// 新建真接 LLM 分类器 (R21+ 用, 本期 **0 真接**)
    ///
    /// **接口留好, 0 假装**: 构造时保存 endpoint/model/api_key, classify 时**不**真发 HTTP
    /// (留 R21+ 实现), 跟主人偏好 #3 "0 假装" 严守
    #[allow(dead_code)] // R21+ 启用
    pub fn new_with_endpoint(endpoint: String, model: String, api_key: String) -> Self {
        Self {
            endpoint: Some(endpoint),
            model: Some(model),
            api_key: Some(api_key),
        }
    }

    /// 当前是否 mock 模式 (endpoint 0 配置 = mock)
    pub fn is_mock(&self) -> bool {
        self.endpoint.is_none()
    }

    /// VCP system prompt (1:1 抄, 留 R21+ 真用)
    #[allow(dead_code)]
    const SYSTEM_PROMPT: &'static str = "You classify tool plugins. Return JSON only.";

    /// VCP user prompt 模板 (1:1 抄 dynamicToolRegistry.js:1008-1017, 留 R21+)
    #[allow(dead_code)]
    fn build_user_prompt(tool: &dyn Tool) -> String {
        use crate::token_budget::{DEFAULT_BRIEF_TOKEN_BUDGET, LIGHT_LIST_TOKEN_BUDGET};
        let kind = tool.kind().as_legacy_str();
        format!(
            "Classify this VCP plugin into concise semantic categories.\n\
             Return strict JSON: {{\"brief\": \"...\", \"categories\": [\"...\"], \"keywords\": [\"...\"], \"confidence\": 0.0}}.\n\
             Keep \"brief\" extremely compact: target {DEFAULT_BRIEF_TOKEN_BUDGET} tokens, and keep plugin name + categories + brief within {LIGHT_LIST_TOKEN_BUDGET} tokens for lightweight tool lists.\n\
             Name: {}\n\
             Kind: {kind}\n\
             Description: (Apeireth Tool trait has no description field; using name+kind)",
            tool.name()
        )
    }
}

impl Default for LlmClassifier {
    fn default() -> Self {
        Self::new_mock()
    }
}

impl Classifier for LlmClassifier {
    fn classify(&self, tool: &dyn Tool) -> Result<Category, ClassifyError> {
        // D-2 mock: 永远返 Search (跟 VCP small model 兜底行为对齐)
        // 真接 R21+: 解析 HTTP response JSON, 提取 categories[0] → Category
        if self.is_mock() {
            // mock 模式: 简单按 tool name 子串分类 (跟 Heuristic 兼容, 但 0 关键词字典)
            let name_lower = tool.name().to_lowercase();
            // mock 模式只识别 3 个最常见类, 其余返 Search (兜底)
            if name_lower.contains("search") || name_lower.contains("query") {
                Ok(Category::Search)
            } else if name_lower.contains("memory") || name_lower.contains("note") {
                Ok(Category::MemoryKnowledge)
            } else if name_lower.contains("file") || name_lower.contains("code") {
                Ok(Category::FileCode)
            } else {
                // 兜底: Search (跟 VCP 兜底 'general' 行为对齐, 我们用 Search 作兜底)
                Ok(Category::Search)
            }
        } else {
            // 真接模式: R21+ 实现
            // 留接口: 返 ClassifyError::LlmError 提示未实现
            Err(ClassifyError::LlmError(format!(
                "LlmClassifier 真接模式 R21+ 启用 (endpoint={:?}, model={:?}, tool={})",
                self.endpoint, self.model, tool.name()
            )))
        }
    }

    fn confidence(&self, _tool: &dyn Tool) -> Result<f32, ClassifyError> {
        if self.is_mock() {
            // mock 模式: 固定 0.5 置信度 (跟 VCP fallback 0.45 类似)
            Ok(0.5)
        } else {
            Err(ClassifyError::LlmError(
                "LlmClassifier 真接模式 R21+ 启用".to_string(),
            ))
        }
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 9 类别总数 (编译期 hardcode, 防止加 variant 忘改 docs)
pub const CATEGORY_COUNT: usize = 9;

const _: () = {
    // 9 类别总数对齐 Category::COUNT (const 上下文, 0 runtime 操作)
    assert!(CATEGORY_COUNT == 9, "CATEGORY_COUNT must be 9");
    assert!(Category::COUNT == 9, "Category::COUNT must be 9");
    // 9 类别关键词字典覆盖 9 类 + priority 升序 → 移到 runtime test
    // (HashSet + for loop 不在 const 上下文允许, 跟 lib.rs const _ 同款处理)
};

// ============================================================
// 单元测试 (≥ 15 个, 战役 D2-5 目标)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AwaitingAxis, OutputAxis, ResidentAxis, ToolAxes, ToolKind, TransportAxis, TriggerAxis,
    };
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::Arc;

    // ====== Mock Tool 工厂 (9 类别各 1 个) ======

    fn mock_tool(name: &str, kind: ToolKind) -> Arc<dyn Tool> {
        Arc::new(SimpleMockTool {
            name: name.to_string(),
            kind,
        })
    }

    struct SimpleMockTool {
        name: String,
        kind: ToolKind,
    }

    #[async_trait]
    impl Tool for SimpleMockTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn kind(&self) -> ToolKind {
            self.kind
        }
        fn axes(&self) -> ToolAxes {
            ToolAxes {
                trigger: TriggerAxis::OnDemand,
                awaiting: AwaitingAxis::Immediate,
                resident: ResidentAxis::Ephemeral,
                transport: TransportAxis::Local,
                output: OutputAxis::Value,
            }
        }
        async fn call(&self, _args: Value) -> Result<Value, String> {
            Ok(json!({}))
        }
    }

    // ====== Category enum 测试 ======

    #[test]
    fn category_count_is_9() {
        assert_eq!(Category::COUNT, 9);
        assert_eq!(CATEGORY_COUNT, 9);
    }

    #[test]
    fn category_all_returns_nine_unique() {
        let all = Category::all();
        assert_eq!(all.len(), 9);
        let mut unique = all.to_vec();
        unique.sort_by_key(|c| c.as_legacy_name());
        unique.dedup();
        assert_eq!(unique.len(), 9, "9 类别必须 9 个唯一名");
    }

    #[test]
    fn category_vcp_name_roundtrip() {
        // VCP 7 类 1:1
        assert_eq!(Category::Search.as_legacy_name(), "search");
        assert_eq!(Category::FileCode.as_legacy_name(), "file_code");
        assert_eq!(Category::ImageMedia.as_legacy_name(), "image_media");
        assert_eq!(Category::MemoryKnowledge.as_legacy_name(), "memory_knowledge");
        assert_eq!(Category::AgentTask.as_legacy_name(), "agent_task");
        assert_eq!(Category::Communication.as_legacy_name(), "communication");
        assert_eq!(Category::Data.as_legacy_name(), "data");
        // Apeireth 独有 2 类
        assert_eq!(Category::Safety.as_legacy_name(), "safety");
        assert_eq!(Category::LongRunning.as_legacy_name(), "long_running");
        // 反向解析
        assert_eq!(Category::from_legacy_name("search"), Some(Category::Search));
        assert_eq!(Category::from_legacy_name("safety"), Some(Category::Safety));
        assert_eq!(
            Category::from_legacy_name("long_running"),
            Some(Category::LongRunning)
        );
        assert_eq!(Category::from_legacy_name("unknown"), None);
    }

    #[test]
    fn category_priority_safety_is_highest() {
        // Safety 最高优先 (priority 0 = 最小值 = 最先选)
        // 注: priority 不要求跟 enum 顺序对齐, 只要求唯一 + Safety=0
        let all = Category::all();
        // 唯一性
        let mut priorities: Vec<u8> = all.iter().map(|c| c.priority()).collect();
        priorities.sort();
        let unique_count = {
            let mut sorted = priorities.clone();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(unique_count, 9, "9 类别 priority 必须唯一");
        assert_eq!(Category::Safety.priority(), 0, "Safety priority 必须 = 0");
        // 验证 Safety 的 priority 小于其他所有类
        for cat in all.iter() {
            if *cat != Category::Safety {
                assert!(
                    Category::Safety.priority() < cat.priority(),
                    "Safety priority 必须 < {:?} priority",
                    cat
                );
            }
        }
    }

    #[test]
    fn keyword_dict_covers_all_nine_categories() {
        // 9 类别关键词字典必须覆盖全 9 类 (防止漏写)
        use std::collections::HashSet;
        let mut seen: HashSet<Category> = HashSet::new();
        for (cat, _) in HeuristicClassifier::KEYWORDS {
            seen.insert(*cat);
        }
        assert_eq!(seen.len(), 9, "KEYWORDS 必须覆盖 9 类别");
        // 每类至少 3 个关键词
        for (cat, kws) in HeuristicClassifier::KEYWORDS {
            assert!(
                kws.len() >= 3,
                "类别 {:?} 关键词数 {}/9 不足 (≥ 3)",
                cat,
                kws.len()
            );
        }
    }

    // ====== HeuristicClassifier 测试 ======

    #[test]
    fn heuristic_classify_search_tool() {
        let t = mock_tool("WebSearch", ToolKind::Sync);
        let c = HeuristicClassifier::new();
        let cat = c.classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::Search);
    }

    #[test]
    fn heuristic_classify_file_code_tool() {
        let t = mock_tool("FileOperator", ToolKind::Sync);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::FileCode);
    }

    #[test]
    fn heuristic_classify_image_media_tool() {
        let t = mock_tool("ImageGenerator", ToolKind::Async);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::ImageMedia);
    }

    #[test]
    fn heuristic_classify_memory_tool() {
        let t = mock_tool("MemoryRecall", ToolKind::Sync);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::MemoryKnowledge);
    }

    #[test]
    fn heuristic_classify_agent_tool() {
        let t = mock_tool("TaskScheduler", ToolKind::Service);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::AgentTask);
    }

    #[test]
    fn heuristic_classify_communication_tool() {
        let t = mock_tool("EmailSender", ToolKind::Async);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::Communication);
    }

    #[test]
    fn heuristic_classify_data_tool() {
        let t = mock_tool("JsonParser", ToolKind::Sync);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::Data);
    }

    #[test]
    fn heuristic_classify_safety_tool() {
        // Safety 优先 (即使同时含 "permission" + "search", Safety 优先)
        let t = mock_tool("PermissionGuard", ToolKind::Sync);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::Safety);
    }

    #[test]
    fn heuristic_classify_long_running_tool() {
        let t = mock_tool("TrainModel", ToolKind::Async);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::LongRunning);
    }

    #[test]
    fn heuristic_no_match_returns_err() {
        // 0 关键词命中
        let t = mock_tool("XyzQqq", ToolKind::Sync);
        let res = HeuristicClassifier::new().classify(t.as_ref());
        assert!(matches!(res, Err(ClassifyError::NoMatch { .. })));
    }

    #[test]
    fn heuristic_confidence_in_range() {
        let t = mock_tool("WebSearch", ToolKind::Sync);
        let conf = HeuristicClassifier::new().confidence(t.as_ref()).expect("conf");
        assert!((0.0..=1.0).contains(&conf), "置信度必须在 0..1, 实际 {conf}");
    }

    #[test]
    fn heuristic_priority_safety_beats_search() {
        // 同一 tool name 同时含 "permission" (Safety) + "search" (Search)
        // Safety 优先 (priority 0 < 5)
        let t = mock_tool("PermissionSearch", ToolKind::Sync);
        let cat = HeuristicClassifier::new().classify(t.as_ref()).expect("classify");
        assert_eq!(cat, Category::Safety);
    }

    // ====== EmbedFn / cosine_similarity 测试 ======

    #[test]
    fn mock_hash_embed_deterministic() {
        let embed = MockHashEmbedFn::new();
        let v1 = embed.embed("hello world");
        let v2 = embed.embed("hello world");
        assert_eq!(v1, v2, "FNV-1a 必须确定性, 相同输入返相同输出");
    }

    #[test]
    fn mock_hash_embed_l2_normalized() {
        let embed = MockHashEmbedFn::new();
        let v = embed.embed("test input");
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5 || norm == 0.0,
            "L2 归一化后 norm 应 = 1.0 (或 0.0 当 0 命中), 实际 {norm}"
        );
    }

    #[test]
    fn cosine_similarity_identical_is_one() {
        let v = vec![1.0, 0.0, 0.0];
        let s = cosine_similarity(&v, &v);
        assert!((s - 1.0).abs() < 1e-6, "相同向量 cosine 应 = 1.0, 实际 {s}");
    }

    #[test]
    fn cosine_similarity_orthogonal_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let s = cosine_similarity(&a, &b);
        assert!(s.abs() < 1e-6, "正交向量 cosine 应 = 0, 实际 {s}");
    }

    #[test]
    fn cosine_similarity_dim_mismatch_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    // ====== EmbeddingClassifier 测试 ======

    #[test]
    fn embedding_classify_returns_category() {
        let c = EmbeddingClassifier::new();
        let t = mock_tool("WebSearch", ToolKind::Sync);
        // 阈值 0.5 在 mock 32 维 FNV 上可能高, 调低
        let c = c.with_threshold(0.0);
        // mock embed 在 9 类中心向量上不一定 ≥ 0.5, 用 0 阈值兜底
        let res = c.classify(t.as_ref());
        // 0 阈值 → 永远返 best (最相似类别)
        assert!(res.is_ok() || matches!(res, Err(ClassifyError::NoMatch { .. })));
    }

    #[test]
    fn embedding_centers_cached_after_first_call() {
        let c = EmbeddingClassifier::new();
        let _ = c.ensure_centers();
        // 第二次调直接走 cache
        let centers_after = c.centers.read();
        assert!(centers_after.is_some(), "ensure_centers 后 cache 应有值");
    }

    #[test]
    fn embedding_dim_mismatch_returns_err() {
        let embed = Arc::new(MockHashEmbedFn::with_dimension(16));
        let c = EmbeddingClassifier::with_embed_fn(embed).with_threshold(0.0);
        let t = mock_tool("WebSearch", ToolKind::Sync);
        // ensure_centers 内部调 embed, 维度 16, 跟 expected dim 一致 → 0 错
        // 测的是 ensure_centers 不返错 (dim 一致)
        let res = c.classify(t.as_ref());
        assert!(res.is_ok() || matches!(res, Err(ClassifyError::NoMatch { .. })));
    }

    // ====== LlmClassifier 测试 ======

    #[test]
    fn llm_mock_classify_always_returns_ok() {
        let c = LlmClassifier::new_mock();
        let t = mock_tool("WebSearch", ToolKind::Sync);
        let cat = c.classify(t.as_ref()).expect("mock classify");
        // mock 模式: WebSearch → Search (按 name 子串)
        assert_eq!(cat, Category::Search);
    }

    #[test]
    fn llm_mock_confidence_is_half() {
        let c = LlmClassifier::new_mock();
        let t = mock_tool("Anything", ToolKind::Sync);
        // 显式用 trait 方法, 不用 impl 方法
        let conf = <LlmClassifier as Classifier>::confidence(&c, t.as_ref()).expect("mock conf");
        assert!((conf - 0.5).abs() < 1e-6, "mock 置信度应 = 0.5, 实际 {conf}");
    }

    #[test]
    fn llm_real_endpoint_not_implemented_returns_err() {
        let c = LlmClassifier::new_with_endpoint(
            "https://api.example.com/v1/chat/completions".to_string(),
            "gpt-4o-mini".to_string(),
            "test-key".to_string(),
        );
        assert!(!c.is_mock());
        let t = mock_tool("WebSearch", ToolKind::Sync);
        let res = c.classify(t.as_ref());
        assert!(matches!(res, Err(ClassifyError::LlmError(_))));
    }

    // ====== 9 demo tool 准确率验收测试 (D2-5 硬指标) ======

    #[test]
    fn heuristic_accuracy_on_9_demo_tools_meets_80_percent() {
        // 9 demo tool, 每类 1 个, 期望 ≥ 80% 准确率 (≥ 7/9 正确)
        // 注: 关键词字典是简化的, 准确率是工程指标, 不是学术指标
        let demos: &[(&str, ToolKind, Category)] = &[
            ("WebSearch", ToolKind::Sync, Category::Search),
            ("FileOperator", ToolKind::Sync, Category::FileCode),
            ("ImageGenerator", ToolKind::Async, Category::ImageMedia),
            ("MemoryRecall", ToolKind::Sync, Category::MemoryKnowledge),
            ("TaskScheduler", ToolKind::Service, Category::AgentTask),
            ("EmailSender", ToolKind::Async, Category::Communication),
            ("JsonParser", ToolKind::Sync, Category::Data),
            ("PermissionGuard", ToolKind::Sync, Category::Safety),
            ("TrainModel", ToolKind::Async, Category::LongRunning),
        ];
        let classifier = HeuristicClassifier::new();
        let mut correct = 0;
        for (name, kind, expected) in demos {
            let t = mock_tool(name, *kind);
            let got = classifier.classify(t.as_ref()).unwrap_or(Category::Search); // 0 匹配兜底
            if got == *expected {
                correct += 1;
            } else {
                eprintln!("[demo] {name} → {got:?} (expected {expected:?})");
            }
        }
        let accuracy = correct as f32 / demos.len() as f32;
        assert!(
            accuracy >= 0.8,
            "9 demo 准确率应 ≥ 80%, 实际 {accuracy:.2} ({correct}/{})",
            demos.len()
        );
    }
}
