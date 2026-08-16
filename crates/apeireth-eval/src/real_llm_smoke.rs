//! R32-3-1: 真接 LLM smoke eval — 调 MiniMax Anthropic API 真跑 1 个 round-trip
//!
//! **目标**: 跟 R32-3 `smoke_task` (stub F 0 LLM) 并列, 加 `real_llm_smoke` 真接 1 个
//! LLM call, 验证:
//! - 7 阶段 metric (apikey / conventions / prompt / http / shape / content / token)
//! - response shape 跟 Anthropic Messages API spec 一致 (content blocks + usage)
//! - token 报数跟 response.usage.input_tokens + output_tokens 对齐
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 R32-3 smoke_task (stub 版仍保留, 给 CI 0-网络环境用)
//! - 0 改 R32-2 tool_loop / R33-1 conventions_scanner (只 import 复用)
//! - 0 改 apeireth-pipeline (直调 HttpClient, 因 Pipeline 走 Bearer auth, MiniMax Anthropic 走 x-api-key)
//! - 0 引入网络阻塞 (走 tokio async + HttpClient Keep-Alive 5 字段)
//!
//! **借鉴锚 (S-1)**:
//! - Anthropic Messages API spec (2023-06-01 version): model / max_tokens / system / messages
//!   + response: content[] (text / tool_use blocks) + usage.input_tokens / output_tokens
//! - MiniMax 兼容文档 (2026-08-09 拍板): base URL `https://api.minimaxi.com`,
//!   `ANTHROPIC_API_KEY` env, `x-api-key: <key>` header
//! - VCP `chatCompletionHandler.js` token 报数 5 字段语义

use std::path::Path;
use std::time::{Duration, Instant};

use apeireth_http_client::HttpClient;
use apeireth_tools::ProjectConventions;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// MiniMax Anthropic API 文档基准 (per https://platform.minimaxi.com/docs/api-reference/text-anthropic-api)
///
/// **Base URL**: `https://api.minimaxi.com` (Anthropic SDK 风格, 实际 endpoint 拼 `/anthropic/v1/messages` per docs ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic)
/// **鉴权**: `x-api-key: <api_key>` + `anthropic-version: 2023-06-01`
/// **模型**: M3 / M2.7 / M2.7-highspeed / M2.5 / M2.5-highspeed / M2.1 / M2.1-highspeed / M2
pub const MINIMAX_BASE_URL: &str = "https://api.minimaxi.com";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";
/// 默认 endpoint (Anthropic SDK 路径前缀)
pub const ANTHROPIC_MESSAGES_PATH: &str = "/anthropic/v1/messages";

/// 默认模型 (高 TPS 100, 204800 context, 适合 CI smoke)
pub const DEFAULT_MODEL: &str = "MiniMax-M2.7-highspeed";

/// 默认 apikey 加载位置 (per 主人 8/9 拍板: openclaw 下 apikey.txt 优先)
/// Windows 优先: `.openclaw\apikey.txt`
/// Linux/Mac 兼容: `$HOME/.openclaw/apikey.txt`
pub const DEFAULT_APIKEY_PATHS: &[&str] = &[
    "C:\\Users\\REDACTED\\.openclaw\\apikey.txt",
    "C:\\Users\\REDACTED\\apikey-ultra.txt",
];

/// 默认 user message (短, 1 sentence, 容易验 response 非空)
pub const DEFAULT_USER_MESSAGE: &str =
    "Reply with exactly 1 short sentence confirming you received the system prompt.";

// ============================================================
// Real LLM smoke report (7 阶段 metric)
// ============================================================

/// 7 阶段真 LLM eval metric
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RealLlmSmokeReport {
    /// 阶段 1: apikey 加载成功 (env / file / explicit 之一)
    pub apikey_loaded: bool,
    /// 阶段 2: conventions_scanner 抽到 workspace metadata (0 scan_error)
    pub conventions_scanned: bool,
    /// 阶段 3: Aider-style system block 拼成 (含 # 项目约定 + # 风格提示)
    pub prompt_built: bool,
    /// 阶段 4: HTTP request 成功 (status 200)
    pub http_request_ok: bool,
    /// 阶段 5: response shape 合法 (Anthropic Messages spec: content[] + usage + model + stop_reason)
    pub response_shape_valid: bool,
    /// 阶段 6: response content 至少 1 个 text block 且 text 非空
    pub content_non_empty: bool,
    /// 阶段 7: token 报数记录 (usage.input_tokens + output_tokens > 0)
    pub token_usage_recorded: bool,

    /// 抽到的 conventions (R33-1, 给 reporting 用; #[serde(skip)] 因 ProjectConventions 不 impl Serialize)
    #[serde(skip)]
    pub conventions: Option<ProjectConventions>,
    /// 拼好的 system block (Aider-style, 给 reporting 用)
    pub system_block: String,
    /// 实际请求的 model
    pub model: String,
    /// 实际请求的 base URL
    pub base_url: String,
    /// 响应里 LLM 返的 model 字段 (验证 server 跟 client 一致)
    pub response_model: Option<String>,
    /// 响应里 stop_reason (Anthropic: end_turn / max_tokens / stop_sequence / tool_use)
    pub stop_reason: Option<String>,
    /// response 第 1 个 text block 的 content (摘录前 200 字符, 给 reporting 用)
    pub response_text_excerpt: String,
    /// token 报数 (Anthropic usage 字段 1:1)
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// 请求耗时 (从 send() 到收到完整 body)
    pub latency_ms: u64,
    /// HTTP status code
    pub http_status: u16,
    /// error (any stage fail 记录, 不 panic)
    pub error: Option<String>,
}

impl RealLlmSmokeReport {
    /// 7 阶段全 pass?
    pub fn all_pass(&self) -> bool {
        self.apikey_loaded
            && self.conventions_scanned
            && self.prompt_built
            && self.http_request_ok
            && self.response_shape_valid
            && self.content_non_empty
            && self.token_usage_recorded
    }

    /// pass rate (0.0 - 1.0)
    pub fn pass_rate(&self) -> f64 {
        let n = 7.0;
        let pass = [
            self.apikey_loaded,
            self.conventions_scanned,
            self.prompt_built,
            self.http_request_ok,
            self.response_shape_valid,
            self.content_non_empty,
            self.token_usage_recorded,
        ]
        .iter()
        .filter(|x| **x)
        .count() as f64;
        pass / n
    }

    /// 7 阶段 → EvalScore (跟 R23 6 module aggregation 对齐)
    pub fn to_eval_scores(&self) -> Vec<crate::EvalScore> {
        vec![
            crate::EvalScore::new("apikey_loaded", if self.apikey_loaded { 1.0 } else { 0.0 }),
            crate::EvalScore::new(
                "conventions_scanned",
                if self.conventions_scanned { 1.0 } else { 0.0 },
            ),
            crate::EvalScore::new("prompt_built", if self.prompt_built { 1.0 } else { 0.0 }),
            crate::EvalScore::new(
                "http_request_ok",
                if self.http_request_ok { 1.0 } else { 0.0 },
            ),
            crate::EvalScore::new(
                "response_shape_valid",
                if self.response_shape_valid { 1.0 } else { 0.0 },
            ),
            crate::EvalScore::new(
                "content_non_empty",
                if self.content_non_empty { 1.0 } else { 0.0 },
            ),
            crate::EvalScore::new(
                "token_usage_recorded",
                if self.token_usage_recorded { 1.0 } else { 0.0 },
            ),
        ]
    }

    fn all_pass_count(&self) -> usize {
        [
            self.apikey_loaded,
            self.conventions_scanned,
            self.prompt_built,
            self.http_request_ok,
            self.response_shape_valid,
            self.content_non_empty,
            self.token_usage_recorded,
        ]
        .iter()
        .filter(|x| **x)
        .count()
    }
}

// ============================================================
// Apikey 加载 (3 源: explicit > env > file)
// ============================================================

/// 加载 apikey: 优先级 explicit arg > env var > file path
///
/// **优先级**:
/// 1. `explicit` 参数非空 → 用之 (源 "explicit")
/// 2. `APEIRETH_MINIMAX_API_KEY` env var 非空 → 用之 (源 "env")
/// 3. `DEFAULT_APIKEY_PATHS` 任一文件存在且非空 → 用之 (源 "file:<path>")
///
/// **返回**: `(api_key, source)` 元组; `Err(String)` 给 `run_real_llm_smoke` 包成 `report.error`
pub fn load_api_key(explicit: Option<&str>) -> Result<(String, String), String> {
    if let Some(k) = explicit {
        if !k.trim().is_empty() {
            return Ok((k.trim().to_string(), "explicit".to_string()));
        }
    }
    if let Ok(k) = std::env::var("APEIRETH_MINIMAX_API_KEY") {
        if !k.trim().is_empty() {
            return Ok((k.trim().to_string(), "env".to_string()));
        }
    }
    for path in DEFAULT_APIKEY_PATHS {
        if let Ok(content) = std::fs::read_to_string(path) {
            let k = content.trim().to_string();
            if !k.is_empty() {
                return Ok((k, format!("file:{}", path)));
            }
        }
    }
    Err(format!(
        "apikey 3 源全 miss: explicit=None/empty, env=APEIRETH_MINIMAX_API_KEY={}, files={:?}",
        std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_else(|_| "<unset>".to_string()),
        DEFAULT_APIKEY_PATHS
    ))
}

// ============================================================
// Anthropic Messages API request / response types
// ============================================================

/// Anthropic Messages API request body (字段级 per spec)
///
/// 完整 spec: <https://docs.anthropic.com/en/api/messages>
#[derive(Debug, Clone, Serialize)]
pub(crate) struct AnthropicMessagesRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) system: Option<&'a str>,
    pub(crate) messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AnthropicMessage {
    pub(crate) role: &'static str,
    pub(crate) content: String,
}

/// Anthropic Messages API response (字段级 1:1, 1:1)
///
/// 我们只 deserialize 需要的字段, 其他忽略 (forward compat)
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AnthropicMessagesResponse {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) stop_reason: Option<String>,
    pub(crate) content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    pub(crate) usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) thinking: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AnthropicUsage {
    pub(crate) input_tokens: u32,
    pub(crate) output_tokens: u32,
    #[serde(default)]
    pub(crate) cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    pub(crate) cache_read_input_tokens: Option<u32>,
}

// ============================================================
// Real LLM smoke 主入口
// ============================================================

/// 配置 (defaultable, 给 test 注入 base_url / model)
#[derive(Debug, Clone)]
pub struct RealLlmConfig {
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    /// Anthropic Messages API path (默认 `/v1/messages`)
    pub messages_path: String,
    /// 可选 timeout (默认 30s)
    pub timeout: Duration,
    /// 默认 system prompt prefix (拼接 Aider block 在前)
    pub system_prefix: String,
    /// user message (短, 1 sentence)
    pub user_message: String,
}

impl Default for RealLlmConfig {
    fn default() -> Self {
        Self {
            base_url: MINIMAX_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 64,
            temperature: Some(1.0),
            messages_path: ANTHROPIC_MESSAGES_PATH.to_string(),
            timeout: Duration::from_secs(30),
            system_prefix: "You are a helpful assistant. Answer concisely (1 sentence max)."
                .to_string(),
            user_message: DEFAULT_USER_MESSAGE.to_string(),
        }
    }
}

/// 跑 1 个真 LLM smoke: 扫 conventions + 拼 prompt + 调 MiniMax /v1/messages + 验证 response
///
/// **不真接时会失败**: 缺 apikey / 网络不通 / model 不存在 / response shape 不对 都返
/// `error: Some(...)` + `all_pass() = false`, 不 panic
///
/// **借鉴**: OpenAI Evals 7 阶段 + Anthropic Evals 7 阶段 + R23 6 module aggregation 接口
pub async fn run_real_llm_smoke(
    workspace_root: &Path,
    api_key: Option<&str>,
    config: Option<RealLlmConfig>,
) -> RealLlmSmokeReport {
    let cfg = config.unwrap_or_default();
    let mut report = RealLlmSmokeReport {
        model: cfg.model.clone(),
        base_url: cfg.base_url.clone(),
        ..Default::default()
    };

    // 阶段 1: apikey 加载
    let api_key = match load_api_key(api_key) {
        Ok((k, _src)) => {
            report.apikey_loaded = true;
            k
        }
        Err(e) => {
            report.error = Some(format!("apikey_loaded: {e}"));
            return report;
        }
    };

    // 阶段 2: conventions_scanner
    let conv = ProjectConventions::scan(workspace_root);
    report.conventions_scanned = conv.scan_error.is_none();
    if !report.conventions_scanned {
        report.error = Some(format!(
            "conventions_scanned: {}",
            conv.scan_error.clone().unwrap_or_default()
        ));
        return report;
    }
    report.conventions = Some(conv.clone());

    // 阶段 3: prompt_built (Aider-style system block + 默认 system prompt)
    let block = conv.to_system_prompt_block();
    let system_prompt = format!("{}\n\n{}", cfg.system_prefix, block);
    report.prompt_built = block.contains("# 项目约定") && block.contains("# 风格提示");
    report.system_block = system_prompt.clone();
    if !report.prompt_built {
        report.error = Some("prompt_built: Aider block 缺 # 项目约定 或 # 风格提示".to_string());
        return report;
    }

    // 阶段 4-7: HTTP call + response validation
    let http = match HttpClient::with_chat_defaults() {
        Ok(h) => h,
        Err(e) => {
            report.error = Some(format!("http_request_ok: HttpClient 构造失败: {e}"));
            return report;
        }
    };
    let url = format!("{}{}", cfg.base_url, cfg.messages_path);
    let req_body = AnthropicMessagesRequest {
        model: &cfg.model,
        max_tokens: cfg.max_tokens,
        system: Some(&system_prompt),
        messages: vec![AnthropicMessage {
            role: "user",
            content: cfg.user_message.clone(),
        }],
        temperature: cfg.temperature,
    };

    let req_builder = http
        .reqwest_client()
        .post(&url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("Content-Type", "application/json")
        .timeout(cfg.timeout)
        .json(&req_body);

    let _guard = http.pool().enter().await;
    let start = Instant::now();
    let response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            report.latency_ms = start.elapsed().as_millis() as u64;
            report.error = Some(format!("http_request_ok: send 失败: {e}"));
            return report;
        }
    };
    let status = response.status();
    report.http_status = status.as_u16();
    report.http_request_ok = status.is_success();
    let latency = start.elapsed();
    report.latency_ms = latency.as_millis() as u64;

    if !report.http_request_ok {
        let body = response.text().await.unwrap_or_default();
        report.error = Some(format!(
            "http_request_ok: status={} body={}",
            report.http_status,
            body.chars().take(300).collect::<String>()
        ));
        return report;
    }

    let body_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            report.error = Some(format!("response_shape_valid: read body 失败: {e}"));
            return report;
        }
    };

    let parsed: AnthropicMessagesResponse = match serde_json::from_str(&body_text) {
        Ok(r) => r,
        Err(e) => {
            report.error = Some(format!(
                "response_shape_valid: JSON parse 失败: {e}, body 前 200: {}",
                body_text.chars().take(200).collect::<String>()
            ));
            return report;
        }
    };

    // 阶段 5: response shape (content[] + usage + model + stop_reason)
    report.response_model = parsed.model.clone();
    report.stop_reason = parsed.stop_reason.clone();
    report.response_shape_valid = !parsed.content.is_empty()
        && parsed.usage.is_some()
        && parsed.model.is_some()
        && parsed.stop_reason.is_some();

    if !report.response_shape_valid {
        report.error = Some(format!(
            "response_shape_valid: content.len()={}, usage={:?}, model={:?}, stop_reason={:?}",
            parsed.content.len(),
            parsed
                .usage
                .as_ref()
                .map(|u| (u.input_tokens, u.output_tokens)),
            parsed.model,
            parsed.stop_reason
        ));
        return report;
    }

    // 阶段 6: content_non_empty (至少 1 text block 非空)
    let first_text = parsed.content.iter().find_map(|b| b.text.clone());
    report.response_text_excerpt = first_text
        .clone()
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect::<String>();
    report.content_non_empty = report.response_text_excerpt.len() > 0;

    if !report.content_non_empty {
        report.error = Some(format!(
            "content_non_empty: 无 text block (response content: {:?})",
            parsed
                .content
                .iter()
                .map(|b| (
                    b.kind.as_str(),
                    b.text
                        .as_deref()
                        .map(|t| t.chars().take(50).collect::<String>())
                ))
                .collect::<Vec<_>>()
        ));
        return report;
    }

    // 阶段 7: token_usage_recorded
    if let Some(usage) = parsed.usage {
        report.input_tokens = usage.input_tokens;
        report.output_tokens = usage.output_tokens;
        report.token_usage_recorded = usage.input_tokens > 0 && usage.output_tokens > 0;
    }

    if !report.token_usage_recorded {
        report.error = Some(format!(
            "token_usage_recorded: input={}, output={}",
            report.input_tokens, report.output_tokens
        ));
    }

    report
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod real_llm_smoke_tests {
    use super::*;
    use apeireth_tools::ProjectConventions;

    // ---- load_api_key ----

    #[test]
    fn load_api_key_explicit_wins() {
        let (k, src) = load_api_key(Some("sk-test-123")).unwrap();
        assert_eq!(k, "sk-test-123");
        assert_eq!(src, "explicit");
    }

    #[test]
    fn load_api_key_explicit_whitespace_trimmed() {
        let (k, _) = load_api_key(Some("  sk-test  \n")).unwrap();
        assert_eq!(k, "sk-test");
    }

    #[test]
    fn load_api_key_explicit_empty_falls_through() {
        // 设了空 explicit, 应该 fallback (env unset, file 可能 miss)
        let r = load_api_key(Some(""));
        // 行为依赖环境: 不强求 result, 但 explicit="" 不该返 ""
        if let Ok((k, _)) = r {
            assert!(!k.is_empty());
        }
    }

    // ---- RealLlmSmokeReport helpers ----

    #[test]
    fn report_all_pass_requires_all_7() {
        let mut r = RealLlmSmokeReport::default();
        assert!(!r.all_pass());
        r.apikey_loaded = true;
        assert!(!r.all_pass());
        r.conventions_scanned = true;
        r.prompt_built = true;
        r.http_request_ok = true;
        r.response_shape_valid = true;
        r.content_non_empty = true;
        r.token_usage_recorded = true;
        assert!(r.all_pass());
        assert_eq!(r.pass_rate(), 1.0);
    }

    #[test]
    fn report_pass_rate_partial() {
        let mut r = RealLlmSmokeReport::default();
        r.apikey_loaded = true;
        r.conventions_scanned = true;
        // 5 阶段 fail
        assert_eq!(r.all_pass_count(), 2);
        assert_eq!(r.pass_rate(), 2.0 / 7.0);
    }

    // ---- Response shape validation (no network) ----

    #[test]
    fn parses_valid_anthropic_response() {
        let body = json!({
            "id": "msg_test123",
            "model": "MiniMax-M2.7-highspeed",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "ok confirmed"}],
            "usage": {"input_tokens": 42, "output_tokens": 5}
        });
        let parsed: AnthropicMessagesResponse = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.model.as_deref(), Some("MiniMax-M2.7-highspeed"));
        assert_eq!(parsed.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(parsed.content.len(), 1);
        assert_eq!(parsed.usage.unwrap().input_tokens, 42);
    }

    #[test]
    fn parses_response_with_thinking_block() {
        let body = json!({
            "id": "msg_x",
            "model": "MiniMax-M3",
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": "step 1..."},
                {"type": "text", "text": "answer"}
            ],
            "usage": {"input_tokens": 100, "output_tokens": 10}
        });
        let parsed: AnthropicMessagesResponse = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.content.len(), 2);
        let texts: Vec<_> = parsed
            .content
            .iter()
            .filter_map(|b| b.text.clone())
            .collect();
        assert_eq!(texts, vec!["answer".to_string()]);
    }

    #[test]
    fn parses_response_with_cache_tokens() {
        let body = json!({
            "model": "MiniMax-M3",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "ok"}],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_creation_input_tokens": 100,
                "cache_read_input_tokens": 200
            }
        });
        let parsed: AnthropicMessagesResponse = serde_json::from_value(body).unwrap();
        let u = parsed.usage.unwrap();
        assert_eq!(u.cache_creation_input_tokens, Some(100));
        assert_eq!(u.cache_read_input_tokens, Some(200));
    }

    #[test]
    fn parses_response_with_tool_use_block() {
        let body = json!({
            "model": "MiniMax-M2.7",
            "stop_reason": "tool_use",
            "content": [
                {"type": "text", "text": "let me call a tool"},
                {"type": "tool_use", "id": "toolu_1", "name": "scan", "input": {"path": "."}}
            ],
            "usage": {"input_tokens": 50, "output_tokens": 20}
        });
        let parsed: AnthropicMessagesResponse = serde_json::from_value(body).unwrap();
        assert_eq!(parsed.stop_reason.as_deref(), Some("tool_use"));
        // text block is first
        assert_eq!(
            parsed.content[0].text.as_deref(),
            Some("let me call a tool")
        );
    }

    // ---- eval scores ----

    #[test]
    fn to_eval_scores_matches_phases() {
        let mut r = RealLlmSmokeReport::default();
        r.apikey_loaded = true;
        r.conventions_scanned = true;
        r.prompt_built = true;
        let scores = r.to_eval_scores();
        assert_eq!(scores.len(), 7);
        assert_eq!(scores[0].dimension, "apikey_loaded");
        assert_eq!(scores[0].value, 1.0);
        // 后面 5 个 0.0
        for s in &scores[3..] {
            assert_eq!(s.value, 0.0);
        }
    }

    #[test]
    fn to_eval_scores_skip_conventions_for_serde() {
        // ProjectConventions 不 impl Serialize, 但加了 #[serde(skip)] 应可 round-trip
        let mut r = RealLlmSmokeReport::default();
        r.conventions = Some(ProjectConventions::default());
        let json = serde_json::to_string(&r).unwrap();
        let back: RealLlmSmokeReport = serde_json::from_str(&json).unwrap();
        // conventions 字段被 skip, 应该是 None
        assert!(back.conventions.is_none());
    }

    // ---- Constants ----

    #[test]
    fn constants_match_minimax_docs() {
        // 防止 spec 漂移
        assert_eq!(MINIMAX_BASE_URL, "https://api.minimaxi.com");
        assert_eq!(ANTHROPIC_VERSION, "2023-06-01");
        assert_eq!(ANTHROPIC_MESSAGES_PATH, "/anthropic/v1/messages");
        assert_eq!(DEFAULT_MODEL, "MiniMax-M2.7-highspeed");
    }

    #[test]
    fn default_paths_include_openclaw_apikey() {
        // 主人 8/9 拍板: openclaw 下 apikey.txt 优先
        assert!(DEFAULT_APIKEY_PATHS.iter().any(|p| p.contains(".openclaw")));
    }

    #[test]
    fn default_config_uses_minimax() {
        let cfg = RealLlmConfig::default();
        assert_eq!(cfg.base_url, MINIMAX_BASE_URL);
        assert_eq!(cfg.model, DEFAULT_MODEL);
        assert_eq!(cfg.messages_path, ANTHROPIC_MESSAGES_PATH);
    }
}
