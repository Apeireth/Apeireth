//! 既有 gateway 后端的 HTTP/SSE 适配 (契约投影, 0 业务语义重复)。
//!
//! 全部端点形状来自既有网关契约: `/health`、`/v1/models`、
//! `/v1/panel/sessions`、`/v1/sessions/{id}/settings`、
//! `/v1/chat/completions` (`stream: true` 的逐增量 SSE)。请求/响应/错误帧
//! 形状都以既有契约为准; 本模块只做编解码与错误归一。

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::backend::{
    BackendError, BackendResult, CockpitBackend, FinishKind, ModelInfo, SessionMeta, ToolEvent,
    ToolEventKind, TurnDelta, TurnOutcome, TurnRequest, UsageSnapshot,
};

/// 网关 HTTP 后端。
#[derive(Debug)]
pub struct HttpGatewayBackend {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl HttpGatewayBackend {
    /// 连接既有 gateway (形如 `http://127.0.0.1:8080`)。
    pub fn new(endpoint: &str) -> BackendResult<Self> {
        let trimmed = endpoint.trim().trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            return Err(BackendError::Unreachable {
                endpoint: endpoint.to_string(),
                detail: "端点为空".to_string(),
            });
        }
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|error| BackendError::Unreachable {
                endpoint: trimmed.clone(),
                detail: error.to_string(),
            })?;
        Ok(Self {
            endpoint: trimmed,
            client,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.endpoint, path)
    }

    fn get(&self, path: &str) -> BackendResult<String> {
        let response = self
            .client
            .get(self.url(path))
            .send()
            .map_err(|error| unreachable_error(&self.endpoint, error))?;
        read_body(response)
    }
}

impl CockpitBackend for HttpGatewayBackend {
    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn health(&mut self) -> BackendResult<()> {
        self.get("/health").map(|_| ())
    }

    fn list_models(&mut self) -> BackendResult<Vec<ModelInfo>> {
        let body = self.get("/v1/models")?;
        let value: Value = parse_json(&body)?;
        let items = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| BackendError::Protocol("模型列表缺 data[]".to_string()))?;
        Ok(items
            .iter()
            .map(|item| ModelInfo {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                provider: item
                    .get("owned_by")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .collect())
    }

    fn list_sessions(&mut self) -> BackendResult<Vec<SessionMeta>> {
        let body = self.get("/v1/panel/sessions?limit=50")?;
        let value: Value = parse_json(&body)?;
        let items = value
            .get("sessions")
            .and_then(Value::as_array)
            .ok_or_else(|| BackendError::Protocol("会话账本缺 sessions[]".to_string()))?;
        Ok(items
            .iter()
            .map(|item| SessionMeta {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                updated_at: item.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
                message_count: item
                    .get("message_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            })
            .collect())
    }

    fn set_model(&mut self, session: Option<&str>, model: Option<&str>) -> BackendResult<String> {
        // 会话级热切换走既有会话设置链 (PATCH /v1/sessions/{id}/settings)。
        let Some(session) = session else {
            return Err(BackendError::NotWired {
                capability: "session.settings",
                hint: "未绑定会话时只做请求级模型覆盖, 会话级设置需要先建立会话".to_string(),
            });
        };
        let body = json!({ "model": model });
        let response = self
            .client
            .patch(self.url(&format!("/v1/sessions/{session}/settings")))
            .json(&body)
            .send()
            .map_err(|error| unreachable_error(&self.endpoint, error))?;
        let text = read_body(response)?;
        let value: Value = parse_json(&text)?;
        let applied = value
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| model.map(str::to_string))
            .unwrap_or_else(|| "默认模型".to_string());
        Ok(applied)
    }

    fn send_turn(
        &mut self,
        request: TurnRequest,
        sink: &mut dyn FnMut(TurnDelta),
    ) -> BackendResult<TurnOutcome> {
        let mut body = Map::new();
        body.insert("stream".to_string(), Value::Bool(true));
        let mut message = Map::new();
        message.insert("role".to_string(), Value::String("user".to_string()));
        message.insert("content".to_string(), Value::String(request.input.clone()));
        body.insert(
            "messages".to_string(),
            Value::Array(vec![Value::Object(message)]),
        );
        if let Some(session) = &request.session {
            body.insert("session_id".to_string(), Value::String(session.clone()));
        }
        if let Some(model) = &request.model {
            body.insert("model".to_string(), Value::String(model.clone()));
        }

        let response = self
            .client
            .post(self.url("/v1/chat/completions"))
            .json(&Value::Object(body))
            .send()
            .map_err(|error| unreachable_error(&self.endpoint, error))?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().unwrap_or_default();
            return Err(decode_error_frame(status, &text));
        }

        let mut text = String::new();
        let mut usage = UsageSnapshot::default();
        let mut session = request.session.clone().unwrap_or_default();
        let mut served_by = String::new();
        let mut rounds = 0u32;
        let mut finish = FinishKind::Stop;

        let reader = BufReader::new(response);
        for line in reader.lines() {
            let line = line.map_err(|error| BackendError::Protocol(error.to_string()))?;
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload == "[DONE]" {
                break;
            }
            if payload.is_empty() {
                continue;
            }
            let chunk: Value = parse_json(payload)?;
            if let Some(delta) = chunk
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
            {
                if !delta.is_empty() {
                    text.push_str(delta);
                    sink(TurnDelta::Text(delta.to_string()));
                }
            }
            match chunk
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
            {
                Some("approval_required") => finish = FinishKind::ApprovalRequired,
                Some("stop") => finish = FinishKind::Stop,
                _ => {}
            }
            if let Some(meta) = chunk.get("apeireth").filter(|meta| meta.is_object()) {
                if let Some(id) = meta.get("session_id").and_then(Value::as_str) {
                    session = id.to_string();
                }
                if let Some(provider) = meta.get("served_by").and_then(Value::as_str) {
                    served_by = provider.to_string();
                }
                if let Some(value) = meta.get("rounds").and_then(Value::as_u64) {
                    rounds = u32::try_from(value).unwrap_or(u32::MAX);
                }
                if let Some(events) = meta.get("events").and_then(Value::as_array) {
                    for event in events {
                        if let Some(tool) = parse_tool_event(event) {
                            sink(TurnDelta::Tool(tool));
                        }
                    }
                }
                if let Some(raw) = meta.get("usage") {
                    if let Some(parsed) = parse_usage(raw) {
                        usage = parsed;
                        sink(TurnDelta::Usage(parsed));
                    }
                }
            }
        }

        Ok(TurnOutcome {
            session,
            text,
            usage,
            served_by,
            rounds,
            latency_ms: 0,
            finish,
        })
    }

    fn compact_session(&mut self, _session: &str) -> BackendResult<crate::backend::CompactReport> {
        // 0 假装: 既有网关暂无会话压缩端点, 命令链路形状已定, 端点就绪即通。
        Err(BackendError::NotWired {
            capability: "session.compact",
            hint: "既有网关暂无会话压缩端点; 端点就绪后 /compact 直接走该链路".to_string(),
        })
    }
}

fn parse_tool_event(event: &Value) -> Option<ToolEvent> {
    let name = event.get("tool_name").and_then(Value::as_str)?;
    if name.is_empty() {
        return None;
    }
    let kind = match event.get("event").and_then(Value::as_str)? {
        "tool_started" => ToolEventKind::Started,
        "tool_completed" => ToolEventKind::Completed { ok: true },
        "tool_failed" => ToolEventKind::Completed { ok: false },
        _ => return None,
    };
    Some(ToolEvent {
        kind,
        name: name.to_string(),
        at_ms: now_ms(),
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// 计量解析 (契约字段 + 兼容别名; 缺失字段按 0)。
fn parse_usage(raw: &Value) -> Option<UsageSnapshot> {
    let field = |name: &str, alias: &str| -> u32 {
        raw.get(name)
            .or_else(|| raw.get(alias))
            .and_then(Value::as_u64)
            .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
            .unwrap_or(0)
    };
    let prompt = field("prompt_tokens", "input_tokens");
    let completion = field("completion_tokens", "output_tokens");
    let total = raw
        .get("total_tokens")
        .and_then(Value::as_u64)
        .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
        .unwrap_or_else(|| prompt.saturating_add(completion));
    Some(UsageSnapshot {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: total,
        // 既有接口无缓存命中字段 → None, 上屏诚实 "—"。
        cache_hit_rate: None,
    })
}

fn parse_json(text: &str) -> BackendResult<Value> {
    serde_json::from_str(text).map_err(|error| BackendError::Protocol(error.to_string()))
}

fn read_body(response: reqwest::blocking::Response) -> BackendResult<String> {
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if status.is_success() {
        Ok(text)
    } else {
        Err(decode_error_frame(status.as_u16(), &text))
    }
}

fn unreachable_error(endpoint: &str, error: reqwest::Error) -> BackendError {
    BackendError::Unreachable {
        endpoint: endpoint.to_string(),
        detail: error.to_string(),
    }
}

/// 既有错误帧契约: `{"error": {"message", "code", "solution"}}`。
fn decode_error_frame(status: u16, body: &str) -> BackendError {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(frame) = value.get("error").filter(|frame| frame.is_object()) {
            return BackendError::Http {
                status,
                code: frame
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                message: frame
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                solution: frame
                    .get("solution")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            };
        }
    }
    let snippet: String = body.chars().take(200).collect();
    BackendError::Http {
        status,
        code: "http_error".to_string(),
        message: snippet,
        solution: "检查后端日志与端点可达性".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 计量解析: 契约字段与兼容别名都认, 缺缓存字段如实留空。
    #[test]
    fn usage_parsing_is_tolerant_and_honest() {
        let parsed = parse_usage(&json!({
            "prompt_tokens": 12, "completion_tokens": 34, "total_tokens": 46
        }))
        .expect("计量");
        assert_eq!(parsed.total_tokens, 46);
        assert_eq!(parsed.cache_hit_rate, None);

        let aliased =
            parse_usage(&json!({ "input_tokens": 7, "output_tokens": 3 })).expect("别名计量");
        assert_eq!(aliased.total_tokens, 10);
    }

    /// 错误帧解码: 契约形状优先, 裸文本兜底。
    #[test]
    fn error_frames_decode_to_typed_errors() {
        let typed = decode_error_frame(
            401,
            r#"{"error":{"message":"未配置 API 密钥","code":"auth_missing_key","solution":"配置密钥"}}"#,
        );
        match typed {
            BackendError::Http { status, code, .. } => {
                assert_eq!(status, 401);
                assert_eq!(code, "auth_missing_key");
            }
            other => panic!("预期 Http 错误帧, 得到 {other:?}"),
        }
        let bare = decode_error_frame(502, "upstream down");
        assert!(matches!(bare, BackendError::Http { status: 502, .. }));
    }

    /// 工具事件映射: 三种事件名各有归宿。
    #[test]
    fn tool_events_map_from_execution_events() {
        let started = parse_tool_event(&json!({
            "event": "tool_started", "tool_name": "tool.demo", "tool_call_id": "c1", "round": 1
        }))
        .expect("started");
        assert_eq!(started.kind, ToolEventKind::Started);
        let failed = parse_tool_event(&json!({
            "event": "tool_failed", "tool_name": "tool.demo"
        }))
        .expect("failed");
        assert_eq!(failed.kind, ToolEventKind::Completed { ok: false });
        assert!(parse_tool_event(&json!({ "event": "presence_state" })).is_none());
    }
}
