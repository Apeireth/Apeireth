//! OpenAI-compatible embeddings transport for semantic memory recall.
//!
//! 2026-10-06 W2 接线批: 差距审计 (`docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md`
//! §7) 实证生产召回的语义向量阶段从未激活 —— `ProductionBackends.embedding_provider`
//! 在 CLI 组装根恒为 `None`, 且全仓库只有 `NoEmbeddingProvider` 与测试 fake。
//! 本模块是**第一个真实现**, 按 `apeireth_memory::scope` 的边界契约落地:
//!
//! > Memory never constructs HTTP clients or reads credentials;
//! > Assembly injects an implementation.
//!
//! 即: 本实现只做 vendor transport (reqwest + OpenAI `POST /embeddings` 归一化),
//! 凭据与端点由组装根 (adapters/cli) 从环境注入; memory crate 不动。
//!
//! **四级口径**: IMPLEMENTED ✅ / PRODUCTION WIRED ✅ (经 `APEIRETH_EMBEDDING_URL`
//! + `APEIRETH_EMBEDDING_MODEL` 旋钮注入组装根) / DEFAULT ENABLED ❌ (无旋钮不构造,
//! 召回静默走词法回退 `used_lexical_fallback = true`) / HARDWARE VALIDATED n/a。
//!
//! **0 装边界**:
//! - 只支持 OpenAI 形状响应 (`data[0].embedding` 为数字数组); 其他 vendor 形状
//!   返回 `InvalidVector`, **不猜**。
//! - `embed` 每次一个文本 (trait 契约), 候选逐条嵌入 = 逐条 HTTP; 批量化留待
//!   trait 升级 (不在此处私自加平行 API)。
//! - 非 2xx 一律 `Unavailable`, 带状态码与响应体片段; 不重试 (重试策略归上层)。

use apeireth_memory::{EmbeddingError, EmbeddingProvider};
use apeireth_plugin::Secret;
use async_trait::async_trait;
use serde_json::Value;

/// Environment variable naming the embeddings endpoint base URL.
pub const EMBEDDING_URL_ENV: &str = "APEIRETH_EMBEDDING_URL";
/// Environment variable naming the embedding model id.
pub const EMBEDDING_MODEL_ENV: &str = "APEIRETH_EMBEDDING_MODEL";
/// Optional environment variable carrying the bearer token for the endpoint.
pub const EMBEDDING_KEY_ENV: &str = "APEIRETH_EMBEDDING_KEY";

/// OpenAI-compatible embeddings endpoint transport.
///
/// Owns its vendor transport (a `reqwest::Client` and the `POST /embeddings`
/// request/response shape), mirroring the `canonical_*` provider capabilities.
///
/// # Secret handling (H9)
///
/// The bearer token is held as `Option<Secret>` exactly like the canonical
/// providers' resolver path (`credentials.rs`): `Secret`'s `Debug` prints
/// `Secret(<redacted>)`, so a `debug!`/`{:?}` on this long-lived struct can no
/// longer write `APEIRETH_EMBEDDING_KEY` into logs or an error panel. The
/// plain `Option<String>` field this replaces was the only secret-bearing
/// field in the crate without that protection.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleEmbeddingProvider {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<Secret>,
}

impl OpenAiCompatibleEmbeddingProvider {
    /// Build a provider for `base_url` (without the trailing `/embeddings`).
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Result<Self, EmbeddingError> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        let model = model.into().trim().to_string();
        if base_url.is_empty() {
            return Err(EmbeddingError::Unavailable("empty embedding url".into()));
        }
        if model.is_empty() {
            return Err(EmbeddingError::Unavailable("empty embedding model".into()));
        }
        // M6: 禁重定向 —— vendor embeddings 是一次性 POST, 从不 30x。reqwest
        // 0.12 跨主机重定向只删 Authorization/Cookie 等固定集合, 带 bearer
        // token 的端点被导向另一主机时 key 会跟着转发; 直接拒绝重定向。
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                EmbeddingError::Unavailable(format!("reqwest client build failed: {error}"))
            })?;
        Ok(Self {
            http,
            base_url,
            model,
            api_key: api_key.map(Secret::new),
        })
    }

    /// Construct from the environment. `APEIRETH_EMBEDDING_URL` and
    /// `APEIRETH_EMBEDDING_MODEL` are required; `APEIRETH_EMBEDDING_KEY` is
    /// optional (some local endpoints need no bearer token).
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let url = std::env::var(EMBEDDING_URL_ENV)
            .map_err(|_| EmbeddingError::Unavailable(format!("missing {EMBEDDING_URL_ENV}")))?;
        let model = std::env::var(EMBEDDING_MODEL_ENV)
            .map_err(|_| EmbeddingError::Unavailable(format!("missing {EMBEDDING_MODEL_ENV}")))?;
        let key = std::env::var(EMBEDDING_KEY_ENV).ok();
        Self::new(url, model, key)
    }

    fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.base_url)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatibleEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let request = self
            .http
            .post(self.embeddings_url())
            .json(&serde_json::json!({ "input": text, "model": self.model }));
        let request = match &self.api_key {
            Some(key) => request.bearer_auth(key.expose()),
            None => request,
        };
        let response = request.send().await.map_err(|error| {
            EmbeddingError::Unavailable(format!("embeddings request failed: {error}"))
        })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            EmbeddingError::Unavailable(format!("embeddings body read failed: {error}"))
        })?;
        if !status.is_success() {
            return Err(classify_status(status, body));
        }
        parse_embedding_body(&body)
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// Pure parser for the OpenAI embeddings response shape
/// (`{"data": [{"embedding": [f64, ...]}]}`). Kept side-effect free for tests.
pub fn parse_embedding_body(body: &str) -> Result<Vec<f32>, EmbeddingError> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| EmbeddingError::InvalidVector(format!("invalid JSON: {error}")))?;
    let entries = value
        .get("data")
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| EmbeddingError::InvalidVector("missing data array".into()))?;
    let embedding = entries[0]
        .get("embedding")
        .and_then(Value::as_array)
        .filter(|vector| !vector.is_empty())
        .ok_or_else(|| EmbeddingError::InvalidVector("missing data[0].embedding".into()))?;
    let mut out = Vec::with_capacity(embedding.len());
    for entry in embedding {
        let value = entry.as_f64().ok_or_else(|| {
            EmbeddingError::InvalidVector(format!("non-numeric embedding entry: {entry}"))
        })?;
        out.push(value as f32);
    }
    Ok(out)
}

fn classify_status(status: reqwest::StatusCode, body: String) -> EmbeddingError {
    let snippet: String = body.chars().take(200).collect();
    EmbeddingError::Unavailable(format!("embeddings provider HTTP {status}: {snippet}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_shaped_embedding() {
        let body = r#"{"data":[{"embedding":[0.25,-0.5,1.0],"index":0}],"model":"m"}"#;
        let vector = parse_embedding_body(body).expect("valid body");
        assert_eq!(vector, vec![0.25, -0.5, 1.0]);
    }

    #[test]
    fn rejects_invalid_json() {
        let error = parse_embedding_body("not json").expect_err("must fail");
        assert!(
            matches!(error, EmbeddingError::InvalidVector(_)),
            "{error:?}"
        );
    }

    #[test]
    fn rejects_missing_data_array() {
        let error = parse_embedding_body(r#"{"object":"list"}"#).expect_err("must fail");
        assert!(
            matches!(error, EmbeddingError::InvalidVector(_)),
            "{error:?}"
        );
    }

    #[test]
    fn rejects_empty_embedding() {
        let error = parse_embedding_body(r#"{"data":[{"embedding":[]}]}"#).expect_err("must fail");
        assert!(
            matches!(error, EmbeddingError::InvalidVector(_)),
            "{error:?}"
        );
    }

    #[test]
    fn rejects_non_numeric_entries() {
        let error =
            parse_embedding_body(r#"{"data":[{"embedding":[1.0,"x"]}]}"#).expect_err("must fail");
        assert!(
            matches!(error, EmbeddingError::InvalidVector(_)),
            "{error:?}"
        );
    }

    #[test]
    fn constructor_rejects_empty_url_and_model() {
        assert!(OpenAiCompatibleEmbeddingProvider::new("", "m", None).is_err());
        assert!(OpenAiCompatibleEmbeddingProvider::new("http://x", "", None).is_err());
    }

    #[test]
    fn constructor_trims_trailing_slash_and_reports_model() {
        let provider = OpenAiCompatibleEmbeddingProvider::new("http://x/v1/", " emb-model ", None)
            .expect("provider builds");
        assert_eq!(provider.embeddings_url(), "http://x/v1/embeddings");
        assert_eq!(provider.model_id(), "emb-model");
    }

    /// H9 防泄露回归: 长生命周期结构体上的 API key 不得以明文出现在任何
    /// `Debug` 输出里 (与 credentials.rs `the_resolver_does_not_carry_or_
    /// print_secrets` 对称)。CLI 组装根构造后以 `Arc<dyn EmbeddingProvider>`
    /// 长期驻留, 任一上层 `{:?}` 都会经过这里。
    #[test]
    fn debug_and_display_never_print_the_embedding_key() {
        let provider = OpenAiCompatibleEmbeddingProvider::new(
            "http://x/v1",
            "emb-model",
            Some("sk-embedding-super-secret".to_string()),
        )
        .expect("provider builds");
        let printed = format!("{provider:?}");
        assert!(
            !printed.contains("sk-embedding-super-secret"),
            "Debug must not carry the key: {printed}"
        );
        // 字段仍存在 (结构未被误删), 只是值被脱敏。
        assert!(printed.contains("OpenAiCompatibleEmbeddingProvider"), "{printed}");
    }

    #[test]
    fn a_missing_key_still_builds_and_bears_no_token() {
        // 本地/无鉴权端点: key 缺失是合法配置, 构造必须成功且不携带秘密。
        let provider = OpenAiCompatibleEmbeddingProvider::new("http://x/v1", "m", None)
            .expect("provider builds without a key");
        assert!(provider.api_key.is_none());
        assert!(!format!("{provider:?}").contains("sk-"));
    }
}
