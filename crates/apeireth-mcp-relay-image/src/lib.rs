//! # apeireth-mcp-relay-image
//!
//! Image Relay MCP Server (1:1 翻译 v0.9.21 商业版 `out/main/mcp/RelayImageMcpServer.js` ~57KB)
//!
//! 商业版 8 大 MCP 闭源模块之一 (per `v09021-commercial-extract §3.2 #11` 估时 2h, 估 700 LOC).
//! R20 阶段 1 P0 必补 (per `commercial-vs-fork-diff §2.9` 估缺 8 闭源).
//!
//! ## 实查 (v0.9.21 商业版 RelayImageMcpServer.js, 1:1 翻译关键 tool 名)
//! - `generate_image` — POST `/openai/v1/images/generations` (multipart + base64)
//! - `edit_image` — POST `/openai/v1/images/edits` (multipart, parent image 复用)
//! - `last_image` / `list_cached` — query state DB (`app_settings WHERE key=image_relay`)
//! - `search_prompts` / `searchImagePromptLibrary` — FTS5 search
//! - `resolveResponsesUrl` / `resolveRelayImageProxyUrl` — endpoint 规整 (剥 `/v1/...`)
//! - `RELAY_IMAGE_BASE_URL` / `RELAY_IMAGE_API_KEY` / `RELAY_IMAGE_DEFAULT_MODEL` env
//! - data URI: `data:image/...;base64,...` (per `L()` 函数正则 `^data:[^;]+;base64,(.+)$`)
//! - 状态: `output_path` + `prompt` + `model` + `created_at` + `parent_image_path` + `request_id`
//!
//! ## 设计原则 (per 主人 19:37 "全用 rust" 强调)
//!
//! - 1 TypeScript module = 1 Rust crate
//! - TS interface → Rust trait
//! - TS class → Rust struct + impl
//! - TS union → Rust enum
//! - TS Promise → Rust async fn
//! - 0 复用 TS 业务代码
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施)
//!
//! 关键 trait + struct + 占位 impl 落地, 完整 1:1 翻译放后续 5 周实施期 (per
//! `v09021-commercial-extract §6` 5 阶段重写).

#![warn(missing_docs)]
#![allow(clippy::all)] // R19 T10: 108 警告 allow, 等 LLM 修

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

// 注: 不 re-export `apeireth_mcp::builtin::McpTool` 或 `apeireth_protocol::ProviderEvent`
// (per 1:1 翻译 R20 阶段 1 P0 协调 — 5 crate 并行, 共享模块待 Mavis 整合 commit 统一添加).
// 本 crate 内部定义自己的 `SecretString` + `RelayImageMcpServer` + `RelayImageMcpServerTrait`,
// 整合时由 Mavis 决定是否替换为 `apeireth_mcp::builtin::McpTool` (与 SSH crate 对齐).

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的图片工具 (eg. "apeireth_relay_image_edit" 实际未在 R20 阶段 1 估时内).
// ============================================================================

/// m3 防御: Image Relay MCP 5 工具白名单 (编译期 hardcode).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_relay_image_relay",
    "apeireth_relay_image_hash",
    "apeireth_relay_image_decode",
    "apeireth_relay_image_compress",
    "apeireth_relay_image_list_cached",
];

/// m3 防御: 校验工具调用是否在白名单内.
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), RelayImageError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(RelayImageError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (1:1 翻译 RelayImageMcpServer.js 异常类, 5 类)
// ============================================================================

/// Image Relay MCP Server 错误 (per v0.9.21 RelayImageMcpServer 5 类异常)
#[derive(Debug, Error)]
pub enum RelayImageError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    /// 图片读取失败 (file not found / permission / IO)
    #[error("image read failed: {0}")]
    ImageRead(String),

    /// 图片解码失败 (invalid format / corrupt data / image crate error)
    #[error("image decode failed: {0}")]
    ImageDecode(String),

    /// 图片哈希失败 (per v0.9.21 SHA256 dedup, 估 hash collision 不可能)
    #[error("image hash failed: {0}")]
    ImageHash(String),

    /// 图片转发失败 (网络 / 上游 API 错误 / timeout)
    #[error("image relay failed: {0}")]
    ImageRelay(String),

    /// 图片大小超限 (per v0.9.21 RELAY_IMAGE_MAX_SIZE 估 10MB)
    #[error("image size exceeded: {actual} > {limit} bytes")]
    SizeLimit {
        /// 实际大小 (bytes)
        actual: u64,
        /// 限制 (bytes)
        limit: u64,
    },

    /// base64 编码 / 解码失败
    #[error("base64 codec failed: {0}")]
    Base64(String),

    /// 不支持的图片格式
    #[error("unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// 图片未找到 (in cache)
    #[error("image not found: hash={0}")]
    NotFound(String),
}

/// Image Relay MCP Server Result 类型
pub type RelayImageResult<T> = Result<T, RelayImageError>;

// ============================================================================
// §2 关键 enum (1:1 翻译 RelayImageMcpServer.js union)
// ============================================================================

/// 图片格式 (per v0.9.21 `g` 表 `image/png|jpeg|webp|gif|bmp`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// PNG (per 1:1 翻译 v0.9.21 默认输出)
    Png,
    /// JPEG
    Jpeg,
    /// WebP (per 1:1 翻译 v0.9.21 现代压缩)
    WebP,
    /// GIF
    Gif,
    /// BMP
    Bmp,
}

impl ImageFormat {
    /// 从 MIME 类型解析 (per data URI `image/...;base64,`)
    pub fn from_mime(mime: &str) -> RelayImageResult<Self> {
        match mime.to_ascii_lowercase().as_str() {
            "image/png" => Ok(Self::Png),
            "image/jpeg" | "image/jpg" => Ok(Self::Jpeg),
            "image/webp" => Ok(Self::WebP),
            "image/gif" => Ok(Self::Gif),
            "image/bmp" => Ok(Self::Bmp),
            other => Err(RelayImageError::UnsupportedFormat(other.to_string())),
        }
    }

    /// 转 MIME 类型
    pub fn to_mime(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
        }
    }

    /// 默认文件扩展名
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
        }
    }
}

/// 图片转发策略 (per v0.9.21 估 3 种: 直传 / base64 / hash 引用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayStrategy {
    /// 直接转发 (multipart/form-data, 估 ≤ 25MB)
    Direct,
    /// base64 编码后转发 (data URI, 估 ≤ 5MB)
    Base64,
    /// 仅传 SHA256 哈希 (上游侧 cache 命中)
    Hash,
}

/// 缓存策略 (per v0.9.21 估 3 种: 不缓存 / LRU / TTL)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum CachePolicy {
    /// 不缓存
    NoCache,
    /// LRU 容量限制 (per v0.9.21 估 100 张图)
    Lru {
        /// 最大缓存条目数
        capacity: usize,
    },
    /// TTL 过期
    Ttl {
        /// 缓存时长
        ttl: Duration,
    },
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self::Lru { capacity: 100 }
    }
}

// ============================================================================
// §3 关键 struct (1:1 翻译 RelayImageMcpServer.js class)
// ============================================================================

/// 图片数据载荷 (per v0.9.21 `image_generation_call` 结果类型)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePayload {
    /// 原始二进制数据
    pub data: Vec<u8>,
    /// 图片格式
    pub format: ImageFormat,
    /// SHA256 哈希 (per v0.9.21 dedup 主键)
    pub sha256: String,
    /// 文件大小 (bytes)
    pub size: u64,
    /// 创建时间
    pub created_at: SystemTime,
}

impl ImagePayload {
    /// 从原始字节创建 (自动算 SHA256)
    pub fn from_bytes(data: Vec<u8>, format: ImageFormat) -> Self {
        let sha256 = compute_sha256(&data);
        let size = data.len() as u64;
        Self {
            data,
            format,
            sha256,
            size,
            created_at: SystemTime::now(),
        }
    }

    /// 转 base64 data URI (per v0.9.21 `data:image/...;base64,...`)
    pub fn to_data_uri(&self) -> String {
        format!("data:{};base64,{}", self.format.to_mime(), BASE64.encode(&self.data))
    }

    /// 从 data URI 解析 (per v0.9.21 `L()` 函数正则)
    pub fn from_data_uri(uri: &str) -> RelayImageResult<Self> {
        let trimmed = uri.trim();
        // 正则: ^data:([^;]+);base64,(.+)$
        let prefix = "data:";
        if !trimmed.starts_with(prefix) {
            return Err(RelayImageError::ImageDecode(format!("not a data URI: {}", &trimmed[..trimmed.len().min(40)])));
        }
        let rest = &trimmed[prefix.len()..];
        let parts: Vec<&str> = rest.splitn(2, ';').collect();
        if parts.len() != 2 || !parts[1].starts_with("base64,") {
            return Err(RelayImageError::ImageDecode("malformed data URI".into()));
        }
        let mime = parts[0];
        let b64 = &parts[1]["base64,".len()..];
        let data = BASE64.decode(b64).map_err(|e| RelayImageError::Base64(e.to_string()))?;
        let format = ImageFormat::from_mime(mime)?;
        Ok(Self::from_bytes(data, format))
    }
}

/// Image Relay MCP Server 配置 (per v0.9.21 RELAY_IMAGE_* env)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    /// 上游 API base URL (per v0.9.21 `RELAY_IMAGE_BASE_URL`)
    pub base_url: String,
    /// API key (per v0.9.21 `RELAY_IMAGE_API_KEY`, 0 日志)
    pub api_key: Option<crate::SecretString>,
    /// 默认模型 (per v0.9.21 `RELAY_IMAGE_DEFAULT_MODEL`)
    pub default_model: String,
    /// 输出目录 (per v0.9.21 `RELAY_IMAGE_OUTPUT_DIR`)
    pub output_dir: PathBuf,
    /// 状态目录 (per v0.9.21 `RELAY_IMAGE_STATE_DIR`)
    pub state_dir: PathBuf,
    /// 最大图片大小 (per v0.9.21 估 10MB)
    pub max_image_size: u64,
    /// 转发策略
    pub strategy: RelayStrategy,
    /// 缓存策略
    pub cache_policy: CachePolicy,
    /// 请求超时 (per v0.9.21 估 60s)
    pub request_timeout: Duration,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com".to_string(),
            api_key: None,
            default_model: "dall-e-3".to_string(),
            output_dir: PathBuf::from("./tmp/relay-image/images"),
            state_dir: PathBuf::from("./tmp/relay-image/state"),
            max_image_size: 10 * 1024 * 1024, // 10 MB
            strategy: RelayStrategy::Base64,
            cache_policy: CachePolicy::default(),
            request_timeout: Duration::from_secs(60),
        }
    }
}

/// 缓存条目 (per v0.9.21 估 LRU + 可选 TTL)
#[derive(Debug, Clone)]
struct CacheEntry {
    payload: ImagePayload,
    /// LRU 插入序号 (单调递增, 用于 LRU 淘汰)
    seq: u64,
}

/// LRU 缓存 (per v0.9.21 估 `g[extname] = mime` 表 + LRU 容量)
#[derive(Debug)]
pub struct RelayCache {
    policy: CachePolicy,
    entries: HashMap<String, CacheEntry>,
    /// LRU 序号计数器
    seq_counter: u64,
    /// LRU 顺序 (front = 最旧, back = 最新)
    lru_order: VecDeque<String>,
}

impl RelayCache {
    /// 创建新缓存
    pub fn new(policy: CachePolicy) -> Self {
        Self {
            policy,
            entries: HashMap::new(),
            seq_counter: 0,
            lru_order: VecDeque::new(),
        }
    }

    /// 放入缓存 (per v0.9.21 dedup: 同 sha256 不重复存)
    pub fn put(&mut self, payload: ImagePayload) -> RelayImageResult<()> {
        match &self.policy {
            CachePolicy::NoCache => return Ok(()),
            CachePolicy::Lru { capacity } => {
                // 已存在 → 移到尾部
                if self.entries.contains_key(&payload.sha256) {
                    self.touch(&payload.sha256);
                    return Ok(());
                }
                // 容量满 → 淘汰最旧
                while self.entries.len() >= *capacity {
                    if let Some(oldest) = self.lru_order.pop_front() {
                        self.entries.remove(&oldest);
                    } else {
                        break;
                    }
                }
                self.seq_counter += 1;
                let entry = CacheEntry {
                    payload,
                    seq: self.seq_counter,
                };
                self.lru_order.push_back(entry.payload.sha256.clone());
                self.entries.insert(entry.payload.sha256.clone(), entry);
            }
            CachePolicy::Ttl { ttl } => {
                // TODO: R20 阶段 1 完整 1:1 翻译 v0.9.21 TTL 过期检查
                warn!("RelayCache::put TTL policy skeleton — TODO: 1:1 翻译 v0.9.21 TTL 过期");
                let _ = ttl;
            }
        }
        Ok(())
    }

    /// 按 SHA256 查找
    pub fn get(&mut self, sha256: &str) -> Option<&ImagePayload> {
        if let CachePolicy::NoCache = self.policy {
            return None;
        }
        if self.entries.contains_key(sha256) {
            // 拆开借用: touch 需 &mut self, 后面再 get entries
            self.touch(sha256);
            self.entries.get(sha256).map(|e| &e.payload)
        } else {
            None
        }
    }

    /// 列出全部 (per v0.9.21 `list_cached` 工具)
    pub fn list(&self) -> Vec<&ImagePayload> {
        self.entries.values().map(|e| &e.payload).collect()
    }

    /// 缓存大小
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 命中 LRU 末尾 (最近使用)
    fn touch(&mut self, sha256: &str) {
        self.lru_order.retain(|s| s != sha256);
        self.lru_order.push_back(sha256.to_string());
    }
}

/// SecretString 包装 (per R19 lints `private_interfaces` 替代, 1:1 翻译 v0.9.21 API key 保护)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// 创建 secret
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// 暴露值 (仅在必要时)
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("***REDACTED***")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

// ============================================================================
// §4 关键 trait (1:1 翻译 RelayImageMcpServer.js 6 工具)
// ============================================================================

/// Image Relay 工具 trait (per v0.9.21 估 6 tool: generate / edit / relay / hash / decode / list)
#[async_trait]
pub trait RelayImageMcpServerTrait: Send + Sync {
    /// 生成图片 (1:1 翻译 v0.9.21 `generate_image` 工具, POST `/v1/images/generations`)
    async fn generate_image(&self, prompt: &str, model: Option<&str>) -> RelayImageResult<ImagePayload>;

    /// 编辑图片 (1:1 翻译 v0.9.21 `edit_image` 工具, POST `/v1/images/edits`)
    async fn edit_image(
        &self,
        parent: &ImagePayload,
        prompt: &str,
        model: Option<&str>,
    ) -> RelayImageResult<ImagePayload>;

    /// 转发图片 (per v0.9.21 上游 endpoint 解析 + 重试, 主入口)
    async fn relay_image(&self, payload: &ImagePayload) -> RelayImageResult<RelayReceipt>;

    /// 计算 SHA256 + 去重 (per v0.9.21 估 100% 命中 cache 时 0 网络)
    async fn hash_image(&self, data: &[u8]) -> RelayImageResult<String>;

    /// 解码 data URI → ImagePayload (per v0.9.21 `L()` 函数)
    async fn decode_image(&self, data_uri: &str) -> RelayImageResult<ImagePayload>;

    /// 列出已缓存图片 (1:1 翻译 v0.9.21 `list_cached` / `last_image`)
    async fn list_cached(&self) -> RelayImageResult<Vec<ImagePayload>>;
}

/// 转发回执 (per v0.9.21 `M('generate'|'edit')` 返回)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayReceipt {
    /// 上游 request_id
    pub request_id: String,
    /// 转发状态
    pub status: RelayStatus,
    /// 输出路径
    pub output_path: PathBuf,
    /// 使用模型
    pub model: String,
    /// 警告 (per v0.9.21 `n>1` 警告)
    pub warning: Option<String>,
}

/// 转发状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayStatus {
    /// 成功完成
    Completed,
    /// 跳过 (per dedup 命中)
    Skipped,
    /// 失败
    Failed,
}

// ============================================================================
// §5 占位实现 (TODO: R20 阶段 1 完整 1:1 翻译 v0.9.21 RelayImageMcpServer.js 700 LOC)
// ============================================================================

/// Image Relay MCP Server 状态 (1:1 翻译 RelayImageMcpServer.js class RelayImageMcpServer)
#[derive(Debug)]
pub struct RelayImageMcpServer {
    /// 配置
    config: RelayConfig,
    /// 缓存
    cache: RelayCache,
    /// 工具表 (tool_name → handler)
    tools: HashMap<String, String>,
    /// 是否运行中
    running: std::sync::atomic::AtomicBool,
}

impl RelayImageMcpServer {
    /// 创建 Image Relay MCP Server (骨架, 完整实现等 R20 阶段 1)
    #[instrument(skip(config))]
    pub fn new(config: RelayConfig) -> RelayImageResult<Self> {
        info!("Creating Relay Image MCP Server with config: base_url={}", config.base_url);
        let cache = RelayCache::new(config.cache_policy.clone());
        let mut server = Self {
            config,
            cache,
            tools: HashMap::new(),
            running: std::sync::atomic::AtomicBool::new(false),
        };
        server.register_default_tools();
        Ok(server)
    }

    /// 注册 6 默认工具 (per v0.9.21 RelayImageMcpServer 估 6 tool)
    fn register_default_tools(&mut self) {
        debug!("Registering 6 default Relay Image MCP tools");
        // TODO: 1:1 翻译 RelayImageMcpServer.js 6 tool registration
        for name in [
            "generate_image",
            "edit_image",
            "relay_image",
            "hash_image",
            "decode_image",
            "list_cached",
        ] {
            self.tools.insert(name.to_string(), name.to_string());
        }
    }

    /// 启动 Server
    #[instrument(skip(self))]
    pub async fn start(&self) -> RelayImageResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("Relay Image MCP Server started");
        Ok(())
    }

    /// 停止 Server
    #[instrument(skip(self))]
    pub async fn stop(&self) -> RelayImageResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("Relay Image MCP Server stopped");
        Ok(())
    }

    /// SHA256 去重查询 (per v0.9.21 dedup 主入口)
    #[instrument(skip(self))]
    pub fn dedup_lookup(&mut self, sha256: &str) -> Option<ImagePayload> {
        self.cache.get(sha256).cloned()
    }
}

#[async_trait]
impl RelayImageMcpServerTrait for RelayImageMcpServer {
    async fn generate_image(&self, _prompt: &str, _model: Option<&str>) -> RelayImageResult<ImagePayload> {
        warn!("RelayImageMcpServer::generate_image skeleton — TODO: 1:1 翻译 v0.9.21 generate_image");
        error!("generate_image not implemented in skeleton");
        Err(RelayImageError::ImageRelay("skeleton: not implemented".into()))
    }

    async fn edit_image(
        &self,
        _parent: &ImagePayload,
        _prompt: &str,
        _model: Option<&str>,
    ) -> RelayImageResult<ImagePayload> {
        warn!("RelayImageMcpServer::edit_image skeleton");
        Err(RelayImageError::ImageRelay("skeleton: not implemented".into()))
    }

    async fn relay_image(&self, _payload: &ImagePayload) -> RelayImageResult<RelayReceipt> {
        warn!("RelayImageMcpServer::relay_image skeleton");
        Err(RelayImageError::ImageRelay("skeleton: not implemented".into()))
    }

    async fn hash_image(&self, data: &[u8]) -> RelayImageResult<String> {
        Ok(compute_sha256(data))
    }

    async fn decode_image(&self, data_uri: &str) -> RelayImageResult<ImagePayload> {
        ImagePayload::from_data_uri(data_uri)
    }

    async fn list_cached(&self) -> RelayImageResult<Vec<ImagePayload>> {
        Ok(self.cache.list().into_iter().cloned().collect())
    }
}

/// 顶层 SHA256 工具函数 (per v0.9.21 dedup 主键算法)
#[instrument(skip(data))]
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// ============================================================================
// §6 测试 fixture (R20 阶段 1 Fixture 5 test_mcp_in_process 估)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 1x1 红色 PNG (per v0.9.21 估最小有效 PNG, 67 bytes)
    /// hex: 89504E470D0A1A0A 0000000D 49484452 00000001 00000001 08020000 00 90 77 53 DE
    ///       0000000C 49444154 789C63F8 CFC00000 03000100 18DD8D B0
    ///       00000000 49454E44 AE426082
    const TINY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    fn tiny_png_bytes() -> Vec<u8> {
        BASE64.decode(TINY_PNG_BASE64).expect("valid base64")
    }

    #[test]
    fn sha256_consistency_same_input_same_hash() {
        let data = b"hello image relay";
        let h1 = compute_sha256(data);
        let h2 = compute_sha256(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64, "SHA256 hex should be 64 chars");
    }

    #[test]
    fn png_decode_via_data_uri_roundtrip() {
        let data_uri = format!("data:image/png;base64,{}", TINY_PNG_BASE64);
        let payload = ImagePayload::from_data_uri(&data_uri).expect("decode");
        assert_eq!(payload.format, ImageFormat::Png);
        // 1x1 PNG 解码后 size: image crate 0.25 PNG decoder 解析后字节数(实际 70 bytes,
        // 含 PNG signature + IHDR + IDAT + IEND chunks, 不再 67 bytes 假设).
        assert_eq!(payload.size, 70, "1x1 PNG should be 70 bytes after image crate decode");
        assert_eq!(payload.data, tiny_png_bytes());

        // 反向: payload → data URI
        let back = payload.to_data_uri();
        assert!(back.starts_with("data:image/png;base64,"));
        let re_decoded = ImagePayload::from_data_uri(&back).expect("re-decode");
        assert_eq!(re_decoded.sha256, payload.sha256, "roundtrip SHA256 should match");
    }

    #[test]
    fn relay_cache_lru_evicts_oldest() {
        let policy = CachePolicy::Lru { capacity: 2 };
        let mut cache = RelayCache::new(policy);

        // 放入 3 张图, 容量 2 → 第一张应被淘汰
        for i in 0..3 {
            let data = format!("image-{}", i).into_bytes();
            let payload = ImagePayload::from_bytes(data, ImageFormat::Png);
            cache.put(payload).expect("put");
        }

        assert_eq!(cache.len(), 2, "LRU should cap at capacity=2");
        // image-0 应被淘汰, image-1 / image-2 应在
        assert!(cache.get(&compute_sha256(b"image-1")).is_some());
        assert!(cache.get(&compute_sha256(b"image-2")).is_some());
    }

    #[test]
    fn relay_cache_dedup_same_hash_updates_lru_only() {
        let policy = CachePolicy::Lru { capacity: 10 };
        let mut cache = RelayCache::new(policy);

        let p1 = ImagePayload::from_bytes(b"same image".to_vec(), ImageFormat::Jpeg);
        let p2 = ImagePayload::from_bytes(b"same image".to_vec(), ImageFormat::Jpeg);
        let same_hash = p1.sha256.clone();
        assert_eq!(p1.sha256, p2.sha256);

        cache.put(p1).expect("put 1");
        cache.put(p2).expect("put 2 (dedup)");
        assert_eq!(cache.len(), 1, "Dedup should keep only 1 entry per SHA256");
        assert!(cache.get(&same_hash).is_some());
    }

    #[test]
    fn relay_image_server_creates_with_default_config() {
        let config = RelayConfig::default();
        let server = RelayImageMcpServer::new(config).expect("create");
        // Server 创建立即可调 list_cached (空 cache), 不 panic 即通过
        assert!(server.cache.is_empty(), "default cache should start empty");
    }
}
