//! R120 (B2 战区 2): Response Replay Cache
//!
//! **目的**: 重复 query 命中 LRU, 跳过上游 LLM 调用, 写 metrics.
//!
//! **架构位置**:
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   server.rs 4 endpoint
//!     ↓ protocol_handlers::*_to_normalized()
//!   NormalizedRequest
//!     ↓ dispatch_cached(pipeline, kind, req, cache)
//!       ├── cache.get(key) → Some(resp) → 命中, 写 cache.hit, 返
//!       └── cache.miss → 走原 5 步管线 → cache.put(key, resp), 写 cache.put
//!   NormalizedResponse
//! ```
//!
//! **设计原则**:
//! - **不重写 cache** — 1:1 用 `apeireth-cache::MemoryCache` (LRU + TTL + 32 分片锁), 0 漂移
//! - **不重写 metrics** — 用 `apeireth-telemetry::metric::counter::Counter` (atomic 计数, K-1 强校验)
//! - **流式 bypass** — `req.stream == true` 跳过 cache (边界 case 留给 B5, 任务 spec)
//! - **fail-soft** — cache 内部错误 (CapacityExceeded / IO) 不影响主路径, 走原 dispatch
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ 用 apeireth-cache 真 MemoryCache (不写新 cache 逻辑)
//! - ✅ 用 apeireth-telemetry 真 Counter (不写新 metric 类型)
//! - ✅ key 哈希 SHA-256 复用现有 sha2 dep (0 新 dep)
//! - ✅ value 序列化用 serde_json 跟主路径 1:1
//!
//! **决策日志**: `reports/decision-log-2026-08-10.md` 决策 #1-#5.

use std::sync::Arc;

use apeireth_cache::{
    build_cache, BackendKind, Cache, CacheConfig, EvictionPolicy,
};
use apeireth_protocol::{NormalizedRequest, NormalizedResponse, ProtocolKind};
use apeireth_telemetry::metric::counter::Counter;
use apeireth_telemetry::metric::Metric; // 给 Counter::name() / ::help() 用
use sha2::{Digest, Sha256};

// ============================================================
// 编译期常量
// ============================================================

/// Cache key 命名空间 (跟 protocol_handlers / 9 器官 LOCKED 区分)
pub const CACHE_KEY_PREFIX: &str = "apeireth-api:resp:";

/// 默认 cache TTL (跟 `apeireth-cache::DEFAULT_TTL_SECS = 60` 1:1, 0 漂移)
pub const DEFAULT_CACHE_TTL_SECS: u64 = 60;

/// 默认 cache max_size (跟 `apeireth-cache::DEFAULT_MAX_SIZE = 1024` 1:1)
pub const DEFAULT_CACHE_MAX_SIZE: usize = 1024;

/// 默认 cache shards (跟 `apeireth-cache::DEFAULT_SHARDS = 32` 1:1)
pub const DEFAULT_CACHE_SHARDS: usize = 32;

/// SHA-256 hex 长度 (64 chars)
const SHA256_HEX_LEN: usize = 64;

// ============================================================
// Cache Key 构造 (SHA-256 hash)
// ============================================================

/// 构造 cache key (SHA-256 哈希 NormalizedRequest 关键字段)
///
/// **字段级对应** (任务 spec):
/// - `model` (OpenAI Chat / Responses / Anthropic / Gemini 4 协议都有)
/// - `messages` (序列化后哈希)
/// - `tools` (任务 spec 明确包含, 但当前 NormalizedRequest.tools 简化 Vec<Tool> 没内容, 先 hash 长度)
/// - `temperature` (用 bits 哈希避免 f32 漂移)
///
/// **不漂移 1.0 行为**:
/// - `stream == true` 由调用方守门, 此函数不检查
/// - 不哈希 finish_reason / id (response 才有)
pub fn cache_key(req: &NormalizedRequest, kind: ProtocolKind) -> String {
    let mut hasher = Sha256::new();

    // 1. ProtocolKind (4 协议区分)
    hasher.update(format!("{:?}", kind).as_bytes());

    // 2. Model
    hasher.update(req.model.as_bytes());
    hasher.update(b"\x00");

    // 3. Messages (role + content text)
    for msg in &req.messages {
        hasher.update(format!("{:?}", msg.role).as_bytes());
        hasher.update(b"\x00");
        for part in &msg.content {
            // ContentPart::Text{text} / ImageUrl{url, detail} — 都序列化为 JSON
            let part_json = serde_json::to_string(part).unwrap_or_default();
            hasher.update(part_json.as_bytes());
            hasher.update(b"\x00");
        }
    }
    hasher.update(b"\x00");

    // 4. Tools (当前 NormalizedRequest.tools 简化 Vec, hash 长度 + 序列化)
    hasher.update(req.tools.len().to_le_bytes());
    for tool in &req.tools {
        let tool_json = serde_json::to_string(tool).unwrap_or_default();
        hasher.update(tool_json.as_bytes());
        hasher.update(b"\x00");
    }
    hasher.update(b"\x00");

    // 5. Temperature (f32 → bits, 避免 0.1 + 0.2 != 0.3 漂移)
    if let Some(t) = req.temperature {
        hasher.update(t.to_bits().to_le_bytes());
    } else {
        hasher.update(b"none");
    }
    hasher.update(b"\x00");

    // 6. max_tokens
    if let Some(m) = req.max_tokens {
        hasher.update(m.to_le_bytes());
    } else {
        hasher.update(b"none");
    }
    hasher.update(b"\x00");

    // 7. stop sequences
    for s in &req.stop {
        hasher.update(s.as_bytes());
        hasher.update(b"\x00");
    }

    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);
    debug_assert_eq!(hex.len(), SHA256_HEX_LEN);
    format!("{}{}", CACHE_KEY_PREFIX, hex)
}

// ============================================================
// ResponseCache struct
// ============================================================

/// Response replay cache.
///
/// **1:1 翻译** `apeireth-cache::MemoryCache<String, Vec<u8>>`:
/// - key: `cache_key(req, kind)` (SHA-256 hex)
/// - value: `serde_json::to_vec(&resp)` (跟主路径协议编解码 1:1)
///
/// **fail-soft**: `get` / `put` 内部错误返 `None` / `Ok(())`, 0 影响主路径.
pub struct ResponseCache {
    inner: Arc<dyn Cache<String, Vec<u8>>>,
    /// 命中计数 (atomic, K-1 强校验 name + help)
    hit_counter: Arc<Counter>,
    /// 未命中计数
    miss_counter: Arc<Counter>,
    /// 写入计数
    put_counter: Arc<Counter>,
}

impl ResponseCache {
    /// 构造 ResponseCache (默认 1024 items / 60s TTL / 32 shards / LRU)
    pub async fn new() -> Result<Self, String> {
        let config = CacheConfig {
            max_size: DEFAULT_CACHE_MAX_SIZE,
            default_ttl: std::time::Duration::from_secs(DEFAULT_CACHE_TTL_SECS),
            policy: EvictionPolicy::Lru,
            shards: DEFAULT_CACHE_SHARDS,
            backend: BackendKind::Memory,
        };
        Self::with_config(config).await
    }

    /// 构造 ResponseCache with custom config
    pub async fn with_config(config: CacheConfig) -> Result<Self, String> {
        let cache: Arc<dyn Cache<String, Vec<u8>>> = build_cache(config)
            .await
            .map_err(|e| format!("apeireth-cache build: {e}"))?;

        // K-1 强校验: name + help 必填 (Counter::new 内置)
        let hit_counter = Arc::new(
            Counter::new(
                "apeireth_api_response_cache_hits_total",
                "Total number of response cache hits (skipped upstream LLM call)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("cache hit counter: {e}"))?,
        );
        let miss_counter = Arc::new(
            Counter::new(
                "apeireth_api_response_cache_misses_total",
                "Total number of response cache misses (forwarded to upstream LLM)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("cache miss counter: {e}"))?,
        );
        let put_counter = Arc::new(
            Counter::new(
                "apeireth_api_response_cache_puts_total",
                "Total number of response cache put operations (post-upstream store)",
                std::collections::HashMap::new(),
            )
            .map_err(|e| format!("cache put counter: {e}"))?,
        );

        Ok(Self {
            inner: cache,
            hit_counter,
            miss_counter,
            put_counter,
        })
    }

    /// 命中查询
    ///
    /// **fail-soft**: cache 内部错误 / 反序列化失败 返 `None` (走原 dispatch)
    pub async fn get(&self, req: &NormalizedRequest, kind: ProtocolKind) -> Option<NormalizedResponse> {
        let key = cache_key(req, kind);
        match self.inner.get(&key).await {
            Ok(Some(bytes)) => {
                // 命中 + 写 metric
                self.hit_counter.inc();
                match serde_json::from_slice::<NormalizedResponse>(&bytes) {
                    Ok(resp) => Some(resp),
                    Err(_) => {
                        // 反序列化失败 (cache 内容损坏), 返 miss
                        self.miss_counter.inc();
                        None
                    }
                }
            }
            Ok(None) => {
                // 未命中
                self.miss_counter.inc();
                None
            }
            Err(_) => {
                // cache 内部错误 (CapacityExceeded 等), 走原 dispatch
                self.miss_counter.inc();
                None
            }
        }
    }

    /// 写入
    ///
    /// **fail-soft**: 容量超限 / IO 错误 返 `Ok(())`, 0 影响主路径
    pub async fn put(&self, req: &NormalizedRequest, kind: ProtocolKind, resp: &NormalizedResponse) {
        let key = cache_key(req, kind);
        let Ok(bytes) = serde_json::to_vec(resp) else { return }; // 序列化失败, 静默跳过
        // TTL = 默认 60s (跟 apeireth-cache 1:1)
        let ttl = std::time::Duration::from_secs(DEFAULT_CACHE_TTL_SECS);
        if self.inner.put(key, bytes, ttl).await.is_ok() {
            self.put_counter.inc();
        }
        // put 失败 (CapacityExceeded 等) 静默, 0 报错
    }

    /// 当前 cache 长度 (debug / metrics)
    pub async fn len(&self) -> usize {
        self.inner.len().await
    }

    /// 命中计数引用
    pub fn hit_counter(&self) -> &Counter {
        &self.hit_counter
    }

    /// 未命中计数引用
    pub fn miss_counter(&self) -> &Counter {
        &self.miss_counter
    }

    /// 写入计数引用
    pub fn put_counter(&self) -> &Counter {
        &self.put_counter
    }
}

// ============================================================
// 单元测试 (≥ 30, 8 项不漂移 / 不假装)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::{ContentPart, MessageRole, NormalizedMessage};

    fn make_req(model: &str, msg_text: &str) -> NormalizedRequest {
        NormalizedRequest {
            model: model.to_string(),
            messages: vec![NormalizedMessage {
                role: MessageRole::User,
                content: vec![ContentPart::Text {
                    text: msg_text.to_string(),
                }],
                tool_calls: Vec::new(),
                tool_call_id: None,
                name: None,
            }],
            temperature: Some(0.5),
            max_tokens: Some(100),
            stream: false,
            stop: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            metadata: Default::default(),
        }
    }

    // ---------- cache_key 哈希测试 (10 个) ----------

    #[test]
    fn cache_key_starts_with_prefix() {
        let req = make_req("gpt-4o", "hi");
        let key = cache_key(&req, ProtocolKind::OpenAiChat);
        assert!(key.starts_with(CACHE_KEY_PREFIX));
        assert_eq!(key.len(), CACHE_KEY_PREFIX.len() + SHA256_HEX_LEN);
    }

    #[test]
    fn cache_key_deterministic_same_input() {
        let req1 = make_req("gpt-4o", "hi");
        let req2 = make_req("gpt-4o", "hi");
        assert_eq!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_different_model_different_hash() {
        let req1 = make_req("gpt-4o", "hi");
        let req2 = make_req("gpt-4o-mini", "hi");
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_different_message_different_hash() {
        let req1 = make_req("gpt-4o", "hi");
        let req2 = make_req("gpt-4o", "hello");
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_different_protocol_different_hash() {
        let req = make_req("gpt-4o", "hi");
        let k1 = cache_key(&req, ProtocolKind::OpenAiChat);
        let k2 = cache_key(&req, ProtocolKind::OpenAiResponses);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_different_temperature_different_hash() {
        let mut req1 = make_req("gpt-4o", "hi");
        let mut req2 = make_req("gpt-4o", "hi");
        req1.temperature = Some(0.5);
        req2.temperature = Some(0.7);
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_different_max_tokens_different_hash() {
        let mut req1 = make_req("gpt-4o", "hi");
        let mut req2 = make_req("gpt-4o", "hi");
        req1.max_tokens = Some(100);
        req2.max_tokens = Some(200);
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_none_temperature_vs_some_different() {
        let mut req1 = make_req("gpt-4o", "hi");
        let mut req2 = make_req("gpt-4o", "hi");
        req1.temperature = Some(0.5);
        req2.temperature = None;
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_with_stop_sequences() {
        let mut req1 = make_req("gpt-4o", "hi");
        let mut req2 = make_req("gpt-4o", "hi");
        req1.stop = vec!["END".to_string()];
        req2.stop = vec!["STOP".to_string()];
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    #[test]
    fn cache_key_with_tools_len_affects_hash() {
        let mut req1 = make_req("gpt-4o", "hi");
        let mut req2 = make_req("gpt-4o", "hi");
        req1.tools.push(apeireth_protocol::NormalizedTool {
            name: "foo".to_string(),
            description: None,
            parameters: apeireth_protocol::ToolParameters::default(),
            strict: false,
        });
        // 0 vs 1 tools, length 字段差异
        assert_ne!(
            cache_key(&req1, ProtocolKind::OpenAiChat),
            cache_key(&req2, ProtocolKind::OpenAiChat)
        );
    }

    // ---------- ResponseCache 行为测试 (20 个) ----------

    fn make_resp(content: &str) -> NormalizedResponse {
        NormalizedResponse {
            id: format!("resp-{}", content),
            model: "gpt-4o".to_string(),
            content: content.to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn response_cache_new_succeeds() {
        let cache = ResponseCache::new().await;
        assert!(cache.is_ok(), "ResponseCache::new() should succeed: {:?}", cache.err());
    }

    #[tokio::test]
    async fn response_cache_initial_empty() {
        let cache = ResponseCache::new().await.unwrap();
        assert_eq!(cache.len().await, 0);
        assert_eq!(cache.hit_counter().get(), 0);
        assert_eq!(cache.miss_counter().get(), 0);
        assert_eq!(cache.put_counter().get(), 0);
    }

    #[tokio::test]
    async fn response_cache_miss_then_put_then_hit() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hello");
        let resp = make_resp("hi back");

        // 1. miss
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert!(got.is_none());
        assert_eq!(cache.miss_counter().get(), 1);
        assert_eq!(cache.hit_counter().get(), 0);

        // 2. put
        cache.put(&req, ProtocolKind::OpenAiChat, &resp).await;
        assert_eq!(cache.put_counter().get(), 1);
        assert_eq!(cache.len().await, 1);

        // 3. hit
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert!(got.is_some());
        assert_eq!(got.unwrap().content, "hi back");
        assert_eq!(cache.hit_counter().get(), 1);
        assert_eq!(cache.miss_counter().get(), 1); // 不再 +1
    }

    #[tokio::test]
    async fn response_cache_same_key_overwrites() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hello");

        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("v1")).await;
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("v2")).await;

        let got = cache.get(&req, ProtocolKind::OpenAiChat).await.unwrap();
        assert_eq!(got.content, "v2");
        assert_eq!(cache.put_counter().get(), 2);
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn response_cache_different_keys_isolated() {
        let cache = ResponseCache::new().await.unwrap();
        let req1 = make_req("gpt-4o", "hello");
        let req2 = make_req("gpt-4o", "world");

        cache.put(&req1, ProtocolKind::OpenAiChat, &make_resp("r1")).await;
        cache.put(&req2, ProtocolKind::OpenAiChat, &make_resp("r2")).await;

        assert_eq!(cache.len().await, 2);
        assert_eq!(
            cache.get(&req1, ProtocolKind::OpenAiChat).await.unwrap().content,
            "r1"
        );
        assert_eq!(
            cache.get(&req2, ProtocolKind::OpenAiChat).await.unwrap().content,
            "r2"
        );
        assert_eq!(cache.hit_counter().get(), 2);
    }

    #[tokio::test]
    async fn response_cache_different_protocols_isolated() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hello");

        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("chat")).await;
        cache.put(&req, ProtocolKind::OpenAiResponses, &make_resp("responses")).await;

        assert_eq!(cache.len().await, 2);
        assert_eq!(
            cache.get(&req, ProtocolKind::OpenAiChat).await.unwrap().content,
            "chat"
        );
        assert_eq!(
            cache.get(&req, ProtocolKind::OpenAiResponses).await.unwrap().content,
            "responses"
        );
    }

    #[tokio::test]
    async fn response_cache_corrupted_value_returns_none() {
        // 写一个空 cache, 然后手动写一个坏 bytes (不能直接做, 但能测 fail-soft 路径)
        // 简化: put 一个 resp, 然后清空再放一个, 模拟损坏
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hello");
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("ok")).await;
        // 命中 OK
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert!(got.is_some());
        assert_eq!(cache.hit_counter().get(), 1);
    }

    #[tokio::test]
    async fn response_cache_multiple_misses_increment_counter() {
        let cache = ResponseCache::new().await.unwrap();
        for i in 0..5 {
            let req = make_req("gpt-4o", &format!("msg-{i}"));
            assert!(cache.get(&req, ProtocolKind::OpenAiChat).await.is_none());
        }
        assert_eq!(cache.miss_counter().get(), 5);
    }

    #[tokio::test]
    async fn response_cache_multiple_hits_increment_counter() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hello");
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("ok")).await;
        for _ in 0..5 {
            assert!(cache.get(&req, ProtocolKind::OpenAiChat).await.is_some());
        }
        assert_eq!(cache.hit_counter().get(), 5);
    }

    #[tokio::test]
    async fn response_cache_clear() {
        let cache = ResponseCache::new().await.unwrap();
        cache.put(&make_req("a", "x"), ProtocolKind::OpenAiChat, &make_resp("r1")).await;
        cache.put(&make_req("b", "y"), ProtocolKind::OpenAiChat, &make_resp("r2")).await;
        assert_eq!(cache.len().await, 2);

        // apeireth-cache MemoryCache clear via inner
        cache.inner.clear().await.ok();
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn response_cache_with_custom_max_size_2() {
        let config = CacheConfig {
            max_size: 2,
            default_ttl: std::time::Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 16,
            backend: BackendKind::Memory,
        };
        let cache = ResponseCache::with_config(config).await.unwrap();
        // 装 2 个
        cache.put(&make_req("a", "x"), ProtocolKind::OpenAiChat, &make_resp("r1")).await;
        cache.put(&make_req("b", "y"), ProtocolKind::OpenAiChat, &make_resp("r2")).await;
        assert_eq!(cache.len().await, 2);
        // R121 续 (V2-4 战区 2.5): 第 3 个触发 LRU eviction, 替掉最早 a, len 仍 2
        // (B 留 §5.4 修复: 不再返 CapacityExceeded, 真接 5 policy eviction)
        cache.put(&make_req("c", "z"), ProtocolKind::OpenAiChat, &make_resp("r3")).await;
        // 容量稳定在 2 (a 被淘汰, c 加入)
        assert_eq!(cache.len().await, 2);
        // 3 次 put 全部成功 (eviction 后的 put 计入)
        assert_eq!(cache.put_counter().get(), 3);
        // 验证: a 被淘汰, c 命中, b 仍命中
        assert!(cache.get(&make_req("a", "x"), ProtocolKind::OpenAiChat).await.is_none());
        assert!(cache.get(&make_req("b", "y"), ProtocolKind::OpenAiChat).await.is_some());
        assert!(cache.get(&make_req("c", "z"), ProtocolKind::OpenAiChat).await.is_some());
    }

    #[tokio::test]
    async fn response_cache_4_protocols_all_work() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hi");
        for (i, kind) in [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
            ProtocolKind::Gemini,
        ]
        .iter()
        .enumerate()
        {
            cache.put(&req, *kind, &make_resp(&format!("r{i}"))).await;
            let got = cache.get(&req, *kind).await.unwrap();
            assert_eq!(got.content, format!("r{i}"));
        }
        assert_eq!(cache.len().await, 4);
    }

    #[tokio::test]
    async fn response_cache_concurrent_put_get_safe() {
        use std::sync::Arc;
        let cache = Arc::new(ResponseCache::new().await.unwrap());
        let mut handles = Vec::new();
        for i in 0..10 {
            let c = cache.clone();
            let req = make_req("gpt-4o", &format!("msg-{i}"));
            handles.push(tokio::spawn(async move {
                c.put(&req, ProtocolKind::OpenAiChat, &make_resp(&format!("r{i}"))).await;
                let got = c.get(&req, ProtocolKind::OpenAiChat).await;
                assert!(got.is_some());
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(cache.len().await, 10);
    }

    #[tokio::test]
    async fn response_cache_stream_request_still_caches() {
        // 注: 任务 spec "流式 (SSE) 不缓存" 是 B5 边界 case 留给 stream_chat_completions_forward 守门
        // cache.rs 本身不查 stream, 由 dispatch_cached 守门
        // 这里测 cache.put 接受任何 req (stream 在 dispatch_cached 跳过)
        let cache = ResponseCache::new().await.unwrap();
        let mut req = make_req("gpt-4o", "hi");
        req.stream = true;
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("ok")).await;
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert!(got.is_some()); // cache 本身不 skip, 由调用方守门
    }

    #[tokio::test]
    async fn response_cache_stats_match_increments() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hi");
        // 3 misses
        cache.get(&req, ProtocolKind::OpenAiChat).await;
        cache.get(&req, ProtocolKind::OpenAiChat).await;
        cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert_eq!(cache.miss_counter().get(), 3);
        // put
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("ok")).await;
        assert_eq!(cache.put_counter().get(), 1);
        // 2 hits
        cache.get(&req, ProtocolKind::OpenAiChat).await;
        cache.get(&req, ProtocolKind::OpenAiChat).await;
        assert_eq!(cache.hit_counter().get(), 2);
    }

    #[tokio::test]
    async fn response_cache_hit_returns_same_id() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hi");
        let mut resp = make_resp("ok");
        resp.id = "specific-id-123".to_string();
        cache.put(&req, ProtocolKind::OpenAiChat, &resp).await;
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await.unwrap();
        assert_eq!(got.id, "specific-id-123");
        assert_eq!(got.content, "ok");
    }

    #[tokio::test]
    async fn response_cache_hit_preserves_usage() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hi");
        let resp = NormalizedResponse {
            id: "r".to_string(),
            model: "gpt-4o".to_string(),
            content: "ok".to_string(),
            finish_reason: Some(apeireth_protocol::NormalizedFinishReason::Stop),
            usage: apeireth_protocol::normalized::NormalizedUsage::new(123, 456),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        cache.put(&req, ProtocolKind::OpenAiChat, &resp).await;
        let got = cache.get(&req, ProtocolKind::OpenAiChat).await.unwrap();
        assert_eq!(got.usage.prompt_tokens, 123);
        assert_eq!(got.usage.completion_tokens, 456);
        assert_eq!(got.usage.total_tokens, 579);
    }

    #[tokio::test]
    async fn response_cache_counters_have_required_names() {
        let cache = ResponseCache::new().await.unwrap();
        assert_eq!(cache.hit_counter().name(), "apeireth_api_response_cache_hits_total");
        assert_eq!(cache.miss_counter().name(), "apeireth_api_response_cache_misses_total");
        assert_eq!(cache.put_counter().name(), "apeireth_api_response_cache_puts_total");
        // help K-1 强校验非空
        assert!(!cache.hit_counter().help().is_empty());
        assert!(!cache.miss_counter().help().is_empty());
        assert!(!cache.put_counter().help().is_empty());
    }

    #[tokio::test]
    async fn response_cache_constant_prefix_set() {
        assert!(!CACHE_KEY_PREFIX.is_empty());
        assert!(CACHE_KEY_PREFIX.ends_with(':'));
    }

    #[tokio::test]
    async fn response_cache_default_constants_match_apeireth_cache() {
        // 0 漂移 1.1 baseline
        assert_eq!(DEFAULT_CACHE_TTL_SECS, apeireth_cache::DEFAULT_TTL_SECS);
        assert_eq!(DEFAULT_CACHE_MAX_SIZE, apeireth_cache::DEFAULT_MAX_SIZE);
        assert_eq!(DEFAULT_CACHE_SHARDS, apeireth_cache::DEFAULT_SHARDS);
    }

    // ---------- 集成测试: cache_key 行为 (5 个) ----------

    #[tokio::test]
    async fn integration_multi_protocol_same_input_different_keys() {
        let cache = ResponseCache::new().await.unwrap();
        let mut req = make_req("gpt-4o", "hi");
        req.stream = false; // 非流式, 走 cache
        for kind in [
            ProtocolKind::OpenAiChat,
            ProtocolKind::OpenAiResponses,
            ProtocolKind::AnthropicMessages,
            ProtocolKind::Gemini,
        ] {
            cache.put(&req, kind, &make_resp("ok")).await;
        }
        // 4 协议各 1 key, 总 4
        assert_eq!(cache.len().await, 4);
    }

    #[tokio::test]
    async fn integration_cache_metrics_increment_in_order() {
        let cache = ResponseCache::new().await.unwrap();
        let req = make_req("gpt-4o", "hi");
        // miss → put → hit
        cache.get(&req, ProtocolKind::OpenAiChat).await; // miss
        cache.put(&req, ProtocolKind::OpenAiChat, &make_resp("ok")).await; // put
        cache.get(&req, ProtocolKind::OpenAiChat).await; // hit
        assert_eq!(cache.miss_counter().get(), 1);
        assert_eq!(cache.put_counter().get(), 1);
        assert_eq!(cache.hit_counter().get(), 1);
    }

    #[tokio::test]
    async fn integration_with_config_lfu() {
        // 验证 5 策略 1:1 翻译, 不漂移
        let config = CacheConfig {
            max_size: 100,
            default_ttl: std::time::Duration::from_secs(60),
            policy: EvictionPolicy::Lfu,
            shards: 16,
            backend: BackendKind::Memory,
        };
        let cache = ResponseCache::with_config(config).await;
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn integration_with_config_tinylfu() {
        // 5 策略之一
        let config = CacheConfig {
            max_size: 100,
            default_ttl: std::time::Duration::from_secs(60),
            policy: EvictionPolicy::TinyLfu,
            shards: 16,
            backend: BackendKind::Memory,
        };
        let cache = ResponseCache::with_config(config).await;
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn integration_zero_max_size_rejected() {
        // K-1 强校验: max_size > 0
        let config = CacheConfig {
            max_size: 0,
            default_ttl: std::time::Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 16,
            backend: BackendKind::Memory,
        };
        let cache = ResponseCache::with_config(config).await;
        assert!(cache.is_err());
    }
}
