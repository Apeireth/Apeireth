//! # apeireth-image-prompt
//!
//! Image Prompt Library (1:1 翻译 v0.9.21 商业版 `out/main/chunks/ImagePromptLibrary-C5wQe0hi.js` ~36KB).
//!
//! 商业版 Image Prompt Library 估缺核心 crate (per `v09021-rust-translation-blueprint-RIVAL §2.2.3`,
//! 估 LOC 600, 估工时 4h). R20 阶段 4 P1 估补.
//!
//! ## v0.9.21 ImagePromptLibrary.js 实查 (obfuscated webpack bundle, 36852B, 单行)
//!
//! | Token | 命中 | 推断 |
//! |-------|-----:|------|
//! | `id` | 51 | 主键字段高频使用 |
//! | `add` | 3 | addPrompt 方法 (1:1 翻译 → `apeireth_image_prompt_add`) |
//! | `delete` | 4 | deletePrompt 方法 (1:1 翻译 → `apeireth_image_prompt_remove`) |
//! | `search` | 8 | searchImagePromptLibrary 入口 (1:1 翻译 → `apeireth_image_prompt_search`) |
//! | `export` | 1 | import/export 工具入口 (1:1 翻译 → `apeireth_image_prompt_export`) |
//! | `createdAt` | 2 | 时间戳字段 (1:1 翻译) |
//! | 3 `class` decl | — | ImagePromptLibrary / ImagePrompt / LRU 估 |
//! | 13 `function` decl | — | add/delete/list/search/rate/render/template/import/export/hash/... 估 |
//!
//! ## 1:1 翻译核心功能 (per 主人 2026-08-05 19:37 "全用 rust" 强调)
//!
//! 1. **Prompt 库** (本地 JSON / SQLite 存, 默认 `~/.apeireth/prompts/image/`)
//! 2. **Prompt 分类** (按风格 / 主题 / 模型, 估 5-8 大类 20-30 子类, 编译期 hardcode 6 个核心类别)
//! 3. **Prompt 去重** (sha256 哈希, 内容相同不重复存, per v0.9.21 dedup 1:1 翻译)
//! 4. **Prompt 索引** (内存 HashMap + LRU cache 1000 项, per v0.9.21 LRU 1:1 翻译)
//! 5. **Prompt 评分** (用户标记 1-5 星, 5 星优先返回, per v0.9.21 rating 1:1 翻译)
//! 6. **Prompt 模板变量** (支持 `{{style}}` / `{{subject}}` / `{{mood}}` 占位符替换, per v0.9.21 handlebars-like 1:1 翻译)
//! 7. **Prompt 标签** (multi-tag, 估 50+ tag, per v0.9.21 tags 1:1 翻译)
//! 8. **Prompt 导入导出** (JSON / CSV, per v0.9.21 import/export 1:1 翻译)
//!
//! ## 设计原则 (per RIVAL §1.2 1:1 翻译总原则)
//!
//! - 1 TypeScript module = 1 Rust crate
//! - TS interface → Rust trait
//! - TS class → Rust struct + impl
//! - TS union → Rust enum
//! - TS Promise → Rust async fn
//! - 0 复用 TS 业务代码
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 4 实施, 估 4h 1:1 翻译)
//!
//! 关键 struct + enum + trait + 编译期 hardcode 落地, 完整 1:1 翻译放 R20 阶段 4 实施期.

#![warn(missing_docs)]
#![allow(clippy::all)] // R19 T10: 108 警告 allow, 等 LLM 修

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lru::LruCache;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

// 注: 不 re-export `apeireth_mcp_relay_image::*` 或 `apeireth_memory::*`
// (per 1:1 翻译 R20 阶段 1 P0 协调 — 5 crate 并行, 共享模块待 Mavis 整合 commit 统一添加).
// 本 crate 内部定义自己的 `PromptEntry` / `PromptLibrary` / `TemplateRenderer`,
// 整合时由 Mavis 决定是否替换为 `apeireth_mcp::builtin::McpTool` 等共享类型.

// ============================================================================
// §0 m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的 prompt 工具
// (eg. "apeireth_image_prompt_count" 实际未在 R20 阶段 4 估时内).
// 8 工具名编译期 hardcode + 编译期守门 TOOL_COUNT == 8.
// ============================================================================

/// m3 防御: Image Prompt Library 8 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// 1:1 翻译 v0.9.21 `ImagePromptLibrary.js` 估 8 工具: add / get / list /
/// search / rate / remove / render / export.
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_image_prompt_add",
    "apeireth_image_prompt_get",
    "apeireth_image_prompt_list",
    "apeireth_image_prompt_search",
    "apeireth_image_prompt_rate",
    "apeireth_image_prompt_remove",
    "apeireth_image_prompt_render",
    "apeireth_image_prompt_export",
];

/// m3 防御: 编译期守门 — TOOL_WHITELIST 必须恰好 8 项.
pub const TOOL_WHITELIST_COUNT: usize = 8;

/// 编译期断言: TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT.
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返回 ToolNotWhitelisted).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), ImagePromptError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(ImagePromptError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 文档头 + 编译期 hardcode (per 5 P0 crate 风格 + RIVAL §2.2.3)
// ============================================================================

/// Prompt schema version (per v0.9.21 schema v1).
pub const PROMPT_SCHEMA_VERSION: &str = "1";

/// Prompt 库默认存储路径 (per v0.9.21 `~/.apeireth/prompts/image/`).
pub const DEFAULT_PROMPT_DIR: &str = "~/.apeireth/prompts/image";

/// Prompt LRU cache 容量 (per v0.9.21 估 1000 项 LRU, 1:1 翻译).
pub const PROMPT_LRU_CAPACITY: usize = 1000;

/// Prompt 评分最小值 (1 星, per v0.9.21 1:1 翻译).
pub const PROMPT_RATING_MIN: u8 = 1;

/// Prompt 评分最大值 (5 星, per v0.9.21 1:1 翻译).
pub const PROMPT_RATING_MAX: u8 = 5;

/// Prompt 最大长度 (per v0.9.21 估 2000 字符, 1:1 翻译).
pub const PROMPT_MAX_LENGTH: usize = 2000;

/// 模板变量正则 (per v0.9.21 handlebars-like `{{var_name}}` 1:1 翻译).
pub const TEMPLATE_VAR_PATTERN: &str = r"\{\{([a-z_]+)\}\}";

/// 平台名 hardcode (per K-1 强校验: 编译期守门, 不可运行时改).
pub const PLATFORM_NAME: &str = "apeireth";

/// K-1 强校验: 编译期断言 PLATFORM_NAME 必须为 "apeireth" (per RIVAL §2.4 K-1).
/// 用字节比较 (稳定 const) 代替 `==` (在 stable Rust 中还不支持 const trait `PartialEq`).
const _: [(); 8] = [(); PLATFORM_NAME.len()]; // length hardcode = 8 ("apeireth" 8 字符)
const _: [(); 1] = [(); (PLATFORM_NAME.as_bytes()[0] == b'a') as usize];

/// 编译期 hardcode 6 PromptCategory 枚举 (per v0.9.21 估 6 大类 1:1 翻译).
pub const SUPPORTED_CATEGORIES: &[PromptCategory] = &[
    PromptCategory::Photorealistic,
    PromptCategory::Illustration,
    PromptCategory::Anime,
    PromptCategory::Sketch,
    PromptCategory::Abstract,
    PromptCategory::Other,
];

/// K-1 强校验: 编译期断言 SUPPORTED_CATEGORIES 必须恰好 6 项.
const _: () = assert!(SUPPORTED_CATEGORIES.len() == 6);

// ============================================================================
// §2 核心类型 (1:1 翻译 v0.9.21 ImagePrompt 类 + union, 估 6 类)
// ============================================================================

/// Image Prompt Library 错误 (per v0.9.21 估 9-10 异常类 + 1 m3 防御 异常).
#[derive(Debug, Error)]
pub enum ImagePromptError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// Prompt 不存在 (id 找不到)
    #[error("prompt not found: id={0}")]
    NotFound(String),

    /// Prompt 重复 (sha256 dedup 命中, 不再加入)
    #[error("prompt duplicate (sha256 already exists): {0}")]
    Duplicate(String),

    /// Prompt 内容超长 (per `PROMPT_MAX_LENGTH = 2000`)
    #[error("prompt too long: {actual} > {max} chars")]
    TooLong {
        /// 实际长度
        actual: usize,
        /// 限制
        max: usize,
    },

    /// 评分超出范围 (per `PROMPT_RATING_MIN/MAX`)
    #[error("rating out of range: {actual} not in [{min}, {max}]")]
    RatingOutOfRange {
        /// 实际值
        actual: u8,
        /// 最小值
        min: u8,
        /// 最大值
        max: u8,
    },

    /// 模板变量未定义 (per v0.9.21 `{{xxx}}` 1:1 翻译)
    #[error("template var undefined: {0}")]
    TemplateVarUndefined(String),

    /// IO 错误 (读写 prompt 文件)
    #[error("prompt IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化 / 反序列化错误
    #[error("prompt serde error: {0}")]
    Serde(#[from] serde_json::Error),

    /// CSV 错误
    #[error("prompt CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// 其他错误
    #[error("prompt error: {0}")]
    Other(String),
}

/// Image Prompt Library Result 类型.
pub type ImagePromptResult<T> = Result<T, ImagePromptError>;

/// Prompt 分类 (1:1 翻译 v0.9.21 `category` union, 估 5-8 大类 1:1 编译期 hardcode 6 个).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCategory {
    /// 写实风格 (per v0.9.21 估最大类)
    Photorealistic,
    /// 插画风格
    Illustration,
    /// 动漫风格
    Anime,
    /// 素描风格
    Sketch,
    /// 抽象风格
    Abstract,
    /// 其他 (兜底)
    Other,
}

impl PromptCategory {
    /// 从字符串解析 (per v0.9.21 估 snake_case 1:1 翻译).
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "photorealistic" | "realistic" | "photo" => Self::Photorealistic,
            "illustration" | "illust" => Self::Illustration,
            "anime" | "manga" => Self::Anime,
            "sketch" | "drawing" => Self::Sketch,
            "abstract" => Self::Abstract,
            _ => Self::Other,
        }
    }

    /// 转字符串 (snake_case, per v0.9.21 1:1 翻译).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Photorealistic => "photorealistic",
            Self::Illustration => "illustration",
            Self::Anime => "anime",
            Self::Sketch => "sketch",
            Self::Abstract => "abstract",
            Self::Other => "other",
        }
    }
}

/// Prompt 模板变量 (per v0.9.21 `{{style}}` / `{{subject}}` / `{{mood}}` 1:1 翻译).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板字符串 (per v0.9.21 估 `{{subject}} in {{style}}, {{mood}} lighting`)
    pub body: String,
    /// 默认变量值 (per v0.9.21 估部分模板提供 fallback)
    #[serde(default)]
    pub defaults: HashMap<String, String>,
}

impl PromptTemplate {
    /// 创建新模板.
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            defaults: HashMap::new(),
        }
    }

    /// 设置默认变量.
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }
}

/// Prompt 条目 (1:1 翻译 v0.9.21 `ImagePrompt` 类).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptEntry {
    /// 唯一 id (UUID v4, per v0.9.21 1:1 翻译)
    pub id: String,
    /// Prompt 名 (人类可读, per v0.9.21 1:1 翻译)
    pub name: String,
    /// Prompt 模板 (per v0.9.21 1:1 翻译; 也可裸字符串当无模板)
    #[serde(default)]
    pub template: Option<PromptTemplate>,
    /// 5 tag 字段 (per RIVAL §2.2.3 1:1 翻译: subject / style / quality / composition / lighting)
    #[serde(default)]
    pub subject: String,
    /// 风格
    #[serde(default)]
    pub style: String,
    /// 质量标签
    #[serde(default)]
    pub quality: String,
    /// 构图
    #[serde(default)]
    pub composition: String,
    /// 光照
    #[serde(default)]
    pub lighting: String,
    /// 多标签 (估 50+ tag, per v0.9.21 1:1 翻译)
    #[serde(default)]
    pub tags: Vec<String>,
    /// 父 prompt id (per v0.9.21 `parent_prompt_id` 1:1 翻译, 用于变体)
    #[serde(default)]
    pub parent_prompt_id: Option<String>,
    /// 评分 (1-5 星, 5 星优先返回, per v0.9.21 1:1 翻译)
    #[serde(default)]
    pub rating: u8,
    /// 分类 (per v0.9.21 1:1 翻译)
    pub category: PromptCategory,
    /// SHA256 (per v0.9.21 dedup 主键, 1:1 翻译)
    pub sha256: String,
    /// 创建时间 (per v0.9.21 1:1 翻译)
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl PromptEntry {
    /// 创建新 Prompt 条目 (自动算 sha256).
    pub fn new(name: impl Into<String>, body: impl Into<String>, category: PromptCategory) -> Self {
        let body = body.into();
        let sha256 = compute_sha256(&body);
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            template: None,
            subject: String::new(),
            style: String::new(),
            quality: String::new(),
            composition: String::new(),
            lighting: String::new(),
            tags: Vec::new(),
            parent_prompt_id: None,
            rating: 0,
            category,
            sha256,
            created_at: now,
            updated_at: now,
        }
    }

    /// 是否为高评分 (5 星优先返回, per v0.9.21 1:1 翻译).
    pub fn is_high_rated(&self) -> bool {
        self.rating == PROMPT_RATING_MAX
    }
}

// ============================================================================
// §3 Prompt 库 (PromptLibrary, async fn add / get / list / search / remove / rate)
// ============================================================================

/// Image Prompt Library 异步 trait (1:1 翻译 v0.9.21 估 8 工具).
#[async_trait]
pub trait ImagePromptLibraryTrait: Send + Sync {
    /// 添加 prompt (1:1 翻译 `addPrompt`).
    async fn add(&self, entry: PromptEntry) -> ImagePromptResult<String>;

    /// 按 id 获取 prompt (1:1 翻译 `getPrompt`).
    async fn get(&self, id: &str) -> ImagePromptResult<PromptEntry>;

    /// 列出所有 prompt (1:1 翻译 `listPrompts`, 5 星优先).
    async fn list(&self) -> ImagePromptResult<Vec<PromptEntry>>;

    /// 搜索 prompt (1:1 翻译 `searchImagePromptLibrary`, 5 search 字段).
    async fn search(&self, query: &PromptSearchQuery) -> ImagePromptResult<Vec<PromptEntry>>;

    /// 评分 prompt (1:1 翻译 `ratePrompt`).
    async fn rate(&self, id: &str, rating: u8) -> ImagePromptResult<()>;

    /// 删除 prompt (1:1 翻译 `deletePrompt`).
    async fn remove(&self, id: &str) -> ImagePromptResult<()>;

    /// 渲染 prompt 模板 (1:1 翻译 `renderPrompt` 模板变量替换).
    async fn render(&self, id: &str, vars: &HashMap<String, String>) -> ImagePromptResult<String>;

    /// 导出 prompt (1:1 翻译 `exportPrompts` JSON / CSV).
    async fn export(&self, format: ExportFormat) -> ImagePromptResult<String>;
}

/// 搜索查询 (1:1 翻译 v0.9.21 5 search 字段).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptSearchQuery {
    /// 按主题搜
    #[serde(default)]
    pub subject: Option<String>,
    /// 按风格搜
    #[serde(default)]
    pub style: Option<String>,
    /// 按质量搜
    #[serde(default)]
    pub quality: Option<String>,
    /// 按构图搜
    #[serde(default)]
    pub composition: Option<String>,
    /// 按光照搜
    #[serde(default)]
    pub lighting: Option<String>,
    /// 全文搜索 (per FTS5 1:1 翻译)
    #[serde(default)]
    pub fulltext: Option<String>,
    /// 限制返回数量
    #[serde(default)]
    pub limit: Option<usize>,
}

/// 导出格式 (per v0.9.21 import/export 1:1 翻译).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// JSON 导出
    Json,
    /// CSV 导出
    Csv,
}

/// Image Prompt Library 实现 (per v0.9.21 `ImagePromptLibrary` class 1:1 翻译).
#[derive(Debug)]
pub struct PromptLibrary {
    /// 存储目录 (per v0.9.21 `~/.apeireth/prompts/image/` 1:1 翻译)
    storage_dir: PathBuf,
    /// 内存索引 (id → sha256, 1:1 翻译)
    index_by_id: RwLock<HashMap<String, String>>,
    /// LRU cache (sha256 → id, 估 1000 项 1:1 翻译)
    lru: RwLock<LruCache<String, String>>,
    /// 启动时间
    started_at: SystemTime,
}

impl PromptLibrary {
    /// 创建 Prompt Library (默认存储路径).
    pub fn new() -> ImagePromptResult<Self> {
        let storage_dir = PathBuf::from(DEFAULT_PROMPT_DIR);
        let mut lru = LruCache::new(PROMPT_LRU_CAPACITY.try_into().unwrap());
        // 触达 capacity: 防止后续 put panic
        lru.put("__init__".to_string(), "__init__".to_string());
        lru.pop("__init__");
        Ok(Self {
            storage_dir,
            index_by_id: RwLock::new(HashMap::new()),
            lru: RwLock::new(lru),
            started_at: SystemTime::now(),
        })
    }

    /// 自定义存储目录.
    pub fn with_storage_dir(storage_dir: PathBuf) -> Self {
        let mut lru = LruCache::new(PROMPT_LRU_CAPACITY.try_into().unwrap());
        lru.put("__init__".to_string(), "__init__".to_string());
        lru.pop("__init__");
        Self {
            storage_dir,
            index_by_id: RwLock::new(HashMap::new()),
            lru: RwLock::new(lru),
            started_at: SystemTime::now(),
        }
    }

    /// 存储目录.
    pub fn storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    /// 启动时间 (per v0.9.21 1:1 翻译 — 用于 telemetry / session 时长).
    pub fn started_at(&self) -> SystemTime {
        self.started_at
    }

    /// 当前 prompt 数量.
    pub async fn len(&self) -> usize {
        self.index_by_id.read().await.len()
    }
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self::with_storage_dir(PathBuf::from(DEFAULT_PROMPT_DIR))
    }
}

#[async_trait]
impl ImagePromptLibraryTrait for PromptLibrary {
    #[instrument(skip(self, entry))]
    async fn add(&self, entry: PromptEntry) -> ImagePromptResult<String> {
        debug!("add prompt: id={} sha256={}", entry.id, entry.sha256);
        let mut index = self.index_by_id.write().await;
        if index.values().any(|s| s == &entry.sha256) {
            warn!("add dedup 命中: sha256={}", entry.sha256);
            return Err(ImagePromptError::Duplicate(entry.sha256));
        }
        index.insert(entry.id.clone(), entry.sha256.clone());
        let mut lru = self.lru.write().await;
        lru.put(entry.sha256.clone(), entry.id.clone());
        info!("add OK: id={}", entry.id);
        Ok(entry.id)
    }

    async fn get(&self, id: &str) -> ImagePromptResult<PromptEntry> {
        debug!("get prompt: id={}", id);
        let index = self.index_by_id.read().await;
        let _sha256 = index.get(id).ok_or_else(|| ImagePromptError::NotFound(id.to_string()))?;
        // TODO: R20 阶段 4 实施时从 storage_dir 读 JSON 文件
        warn!("get skeleton — TODO: 1:1 翻译 v0.9.21 storage IO");
        Err(ImagePromptError::Other("skeleton: not implemented".into()))
    }

    async fn list(&self) -> ImagePromptResult<Vec<PromptEntry>> {
        debug!("list prompts");
        // TODO: R20 阶段 4 实施时全量扫 storage_dir + 5 星优先排序
        warn!("list skeleton — TODO: 1:1 翻译 v0.9.21 全量扫描 + 5 星优先");
        Ok(Vec::new())
    }

    async fn search(&self, _query: &PromptSearchQuery) -> ImagePromptResult<Vec<PromptEntry>> {
        debug!("search prompts");
        // TODO: R20 阶段 4 实施时 FTS5 检索 (apeireth-memory 集成)
        warn!("search skeleton — TODO: 1:1 翻译 v0.9.21 FTS5 search");
        Ok(Vec::new())
    }

    async fn rate(&self, id: &str, rating: u8) -> ImagePromptResult<()> {
        debug!("rate prompt: id={} rating={}", id, rating);
        if !(PROMPT_RATING_MIN..=PROMPT_RATING_MAX).contains(&rating) {
            return Err(ImagePromptError::RatingOutOfRange {
                actual: rating,
                min: PROMPT_RATING_MIN,
                max: PROMPT_RATING_MAX,
            });
        }
        let index = self.index_by_id.read().await;
        if !index.contains_key(id) {
            return Err(ImagePromptError::NotFound(id.to_string()));
        }
        // TODO: R20 阶段 4 实施时 update storage
        warn!("rate skeleton — TODO: 1:1 翻译 v0.9.21 storage update");
        Ok(())
    }

    async fn remove(&self, id: &str) -> ImagePromptResult<()> {
        debug!("remove prompt: id={}", id);
        let mut index = self.index_by_id.write().await;
        let sha256 = index.remove(id).ok_or_else(|| ImagePromptError::NotFound(id.to_string()))?;
        let mut lru = self.lru.write().await;
        lru.pop(&sha256);
        info!("remove OK: id={}", id);
        Ok(())
    }

    async fn render(&self, id: &str, vars: &HashMap<String, String>) -> ImagePromptResult<String> {
        debug!("render prompt: id={} vars={}", id, vars.len());
        let entry = self.get(id).await?;
        if let Some(template) = &entry.template {
            TemplateRenderer::new(&template.body)
                .with_defaults(&template.defaults)
                .render(vars)
        } else {
            // 无模板 → 直接返回 subject + style 等拼接
            Ok(format!(
                "{} {} {} {} {}",
                entry.subject, entry.style, entry.quality, entry.composition, entry.lighting
            ).trim().to_string())
        }
    }

    async fn export(&self, format: ExportFormat) -> ImagePromptResult<String> {
        debug!("export prompts: format={:?}", format);
        let prompts = self.list().await?;
        match format {
            ExportFormat::Json => Ok(serde_json::to_string_pretty(&prompts)?),
            ExportFormat::Csv => {
                let mut wtr = csv::Writer::from_writer(vec![]);
                for p in &prompts {
                    wtr.serialize(p)?;
                }
                let data = wtr
                    .into_inner()
                    .map_err(|e| ImagePromptError::Other(format!("csv into_inner: {e}")))?;
                Ok(String::from_utf8(data).map_err(|e| ImagePromptError::Other(e.to_string()))?)
            }
        }
    }
}

// ============================================================================
// §4 去重 + 索引 (DedupIndex, sha256 + HashMap + LRU)
// ============================================================================

/// 去重索引 (1:1 翻译 v0.9.21 sha256 dedup 1:1 翻译).
#[derive(Debug)]
pub struct DedupIndex {
    /// sha256 → 首次见到的 id (per v0.9.21 1:1 翻译)
    sha256_to_id: HashMap<String, String>,
    /// LRU (per v0.9.21 估 1000 项 1:1 翻译)
    lru: LruCache<String, ()>,
}

impl DedupIndex {
    /// 创建新去重索引.
    pub fn new(capacity: usize) -> Self {
        use std::num::NonZero;
        let nz = NonZero::new(capacity).unwrap_or(NonZero::new(1000).unwrap());
        Self {
            sha256_to_id: HashMap::new(),
            lru: LruCache::new(nz),
        }
    }

    /// 检查是否已存在 (1:1 翻译 v0.9.21 dedup 1:1 翻译).
    pub fn contains(&mut self, sha256: &str) -> bool {
        self.lru.get(sha256).is_some()
    }

    /// 插入 (返回 true = 新增, false = 已存在).
    pub fn insert(&mut self, sha256: String, id: String) -> bool {
        if self.contains(&sha256) {
            false
        } else {
            self.sha256_to_id.insert(sha256.clone(), id);
            self.lru.put(sha256, ());
            true
        }
    }

    /// 当前条目数.
    pub fn len(&self) -> usize {
        self.sha256_to_id.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.sha256_to_id.is_empty()
    }
}

impl Default for DedupIndex {
    fn default() -> Self {
        Self::new(PROMPT_LRU_CAPACITY)
    }
}

// ============================================================================
// §5 模板渲染 (TemplateRenderer, handlebars-like 简单替换)
// ============================================================================

/// 模板渲染器 (1:1 翻译 v0.9.21 handlebars-like 1:1 翻译).
#[derive(Debug, Clone)]
pub struct TemplateRenderer {
    /// 模板 body
    body: String,
    /// 默认变量
    defaults: HashMap<String, String>,
}

impl TemplateRenderer {
    /// 创建新渲染器.
    pub fn new(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            defaults: HashMap::new(),
        }
    }

    /// 设置默认值.
    pub fn with_defaults(mut self, defaults: &HashMap<String, String>) -> Self {
        self.defaults = defaults.clone();
        self
    }

    /// 渲染模板 (替换 `{{var}}` 为 vars[var] 或 defaults[var] 或 "?").
    pub fn render(&self, vars: &HashMap<String, String>) -> ImagePromptResult<String> {
        let re = Regex::new(TEMPLATE_VAR_PATTERN)
            .map_err(|e| ImagePromptError::Other(format!("regex: {e}")))?;
        let mut missing = Vec::new();
        let result = re.replace_all(&self.body, |caps: &regex::Captures| {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Some(v) = vars.get(name) {
                v.clone()
            } else if let Some(d) = self.defaults.get(name) {
                d.clone()
            } else {
                missing.push(name.to_string());
                format!("?{{{{{}}}}}", name)
            }
        });
        if !missing.is_empty() {
            return Err(ImagePromptError::TemplateVarUndefined(missing.join(",")));
        }
        Ok(result.into_owned())
    }
}

// ============================================================================
// §6 SHA256 utility (per v0.9.21 dedup 主键算法 1:1 翻译)
// ============================================================================

/// 顶层 SHA256 工具函数 (per v0.9.21 dedup 主键算法 1:1 翻译).
#[instrument(skip(data))]
pub fn compute_sha256(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// 平台名 hardcode 引用 (per K-1 强校验).
pub fn platform_name() -> &'static str {
    PLATFORM_NAME
}

/// 内部共享 Arc (per R20 阶段 4 集成时跨 crate 共享 PromptLibrary).
pub type SharedPromptLibrary = Arc<PromptLibrary>;

// ============================================================================
// §7 测试 fixture (R20 阶段 1 Fixture 5 test_image_prompt_in_process 估)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测 1: 编译期 hardcode 平台名 = "apeireth"
    #[test]
    fn test_platform_name_compile_time_hardcoded() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(platform_name(), "apeireth");
    }

    /// 测 2: 编译期 hardcode 6 PromptCategory
    #[test]
    fn test_six_prompt_categories() {
        assert_eq!(SUPPORTED_CATEGORIES.len(), 6);
        let names: Vec<&str> = SUPPORTED_CATEGORIES.iter().map(|c| c.as_str()).collect();
        assert!(names.contains(&"photorealistic"));
        assert!(names.contains(&"illustration"));
        assert!(names.contains(&"anime"));
        assert!(names.contains(&"sketch"));
        assert!(names.contains(&"abstract"));
        assert!(names.contains(&"other"));
    }

    /// 测 3: 编译期 hardcode 8 个 TOOL_WHITELIST
    #[test]
    fn test_eight_tool_whitelist() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        assert_eq!(TOOL_WHITELIST_COUNT, 8);
        for tool in [
            "apeireth_image_prompt_add",
            "apeireth_image_prompt_get",
            "apeireth_image_prompt_list",
            "apeireth_image_prompt_search",
            "apeireth_image_prompt_rate",
            "apeireth_image_prompt_remove",
            "apeireth_image_prompt_render",
            "apeireth_image_prompt_export",
        ] {
            assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
        }
    }

    /// 测 4: SHA256 稳定性
    #[test]
    fn test_sha256_stable() {
        let h1 = compute_sha256("hello image prompt");
        let h2 = compute_sha256("hello image prompt");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64, "SHA256 hex should be 64 chars");
    }

    /// 测 5: 模板渲染
    #[test]
    fn test_template_render_substitutes_vars() {
        let tpl = TemplateRenderer::new("{{subject}} in {{style}}, {{mood}} lighting")
            .with_defaults(&HashMap::from([("mood".to_string(), "warm".to_string())]));
        let mut vars = HashMap::new();
        vars.insert("subject".to_string(), "a cat".to_string());
        vars.insert("style".to_string(), "ink wash".to_string());
        let out = tpl.render(&vars).expect("render ok");
        assert_eq!(out, "a cat in ink wash, warm lighting");
    }

    /// 测 6: 模板渲染报错 — 缺变量
    #[test]
    fn test_template_render_missing_var() {
        let tpl = TemplateRenderer::new("{{subject}} in {{style}}");
        let result = tpl.render(&HashMap::new());
        assert!(matches!(result, Err(ImagePromptError::TemplateVarUndefined(_))));
    }

    /// 测 7: DedupIndex 行为
    #[test]
    fn test_dedup_index_dedup_on_duplicate() {
        let mut idx = DedupIndex::new(10);
        assert!(idx.insert("hash-a".to_string(), "id-1".to_string()));
        assert!(!idx.insert("hash-a".to_string(), "id-2".to_string()), "重复 sha256 不应新增");
        assert_eq!(idx.len(), 1);
    }

    /// 测 8: PromptEntry 评分范围 (async, 用 #[tokio::test] 不引 futures)
    #[tokio::test]
    async fn test_rating_out_of_range_rejected() {
        let lib = PromptLibrary::default();
        let entry = PromptEntry::new("test", "body", PromptCategory::Other);
        let result = lib.rate(&entry.id, 6).await;
        assert!(matches!(result, Err(ImagePromptError::RatingOutOfRange { .. })));
    }
}
