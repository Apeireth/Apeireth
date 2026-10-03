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
use crate::telemetry::{GovVerdict, GovernanceEvent, TelemetrySnapshot, LAMP_SLOTS};

/// 记忆件数计数窗口 (既有契约列表上限 500; 页满 = 窗口饱和, 不冒充总数)。
const EPISODE_COUNT_WINDOW: usize = 500;

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

    fn fetch_telemetry(&mut self) -> BackendResult<TelemetrySnapshot> {
        let mut snapshot = TelemetrySnapshot::default();
        let mut any_response = false;
        let mut unreachable: Option<BackendError> = None;

        // 会话计数: 会话账本全量列表 (计数端点) → 真实总数。
        match self.get("/v1/panel/sessions") {
            Ok(body) => {
                any_response = true;
                match parse_json(&body) {
                    Ok(value) => match value.get("sessions").and_then(Value::as_array) {
                        Some(items) => snapshot.ledger.sessions = Some(items.len() as u64),
                        None => snapshot
                            .notes
                            .push("会话计数: 响应缺 sessions[] → 显式留空".to_string()),
                    },
                    Err(error) => snapshot.notes.push(format!("会话计数: {error}")),
                }
            }
            Err(error) => {
                note_section(&mut snapshot.notes, "会话计数", &error, &mut unreachable);
            }
        }

        // 记忆/保护计数: 记忆件列表窗口 (既有契约无总数端点) —— 窗口未饱和
        // = 页面全量 = 真实总数; 窗口饱和一律显式留空, 不冒充总数。
        match self.get(&format!(
            "/v1/panel/memory/episodes?limit={EPISODE_COUNT_WINDOW}"
        )) {
            Ok(body) => {
                any_response = true;
                match parse_json(&body) {
                    Ok(value) => match value.get("episodes").and_then(Value::as_array) {
                        Some(items) => {
                            let (memories, protected, notes) =
                                ledger_from_episodes(items, EPISODE_COUNT_WINDOW);
                            snapshot.ledger.memories = memories;
                            snapshot.ledger.protected = protected;
                            snapshot.notes.extend(notes);
                        }
                        None => snapshot
                            .notes
                            .push("记忆计数: 响应缺 episodes[] → 显式留空".to_string()),
                    },
                    Err(error) => snapshot.notes.push(format!("记忆计数: {error}")),
                }
            }
            Err(error) => {
                note_section(&mut snapshot.notes, "记忆计数", &error, &mut unreachable);
            }
        }

        // 教训计数: 自述探测口未上 HTTP 契约 → 显式「未接线」, 不编数。
        snapshot
            .notes
            .push("教训计数: 未接线 (自述探测口未上 HTTP 契约)".to_string());

        // 治理灯阵: 安全护栏最近事件 (绿=放行/琥珀=审批/红=拒绝)。
        match self.get(&format!("/v1/panel/safety/guard/events?limit={LAMP_SLOTS}")) {
            Ok(body) => {
                any_response = true;
                match parse_json(&body) {
                    Ok(value) => match value.get("events").and_then(Value::as_array) {
                        Some(items) => {
                            let (events, unknown) = governance_from_events(items);
                            snapshot.governance = events;
                            if unknown > 0 {
                                snapshot
                                    .notes
                                    .push(format!("治理灯阵: {unknown} 个未知判定不上灯 (不猜色)"));
                            }
                        }
                        None => snapshot
                            .notes
                            .push("治理灯阵: 响应缺 events[] → 暗格".to_string()),
                    },
                    Err(error) => snapshot.notes.push(format!("治理灯阵: {error}")),
                }
            }
            Err(error) => {
                note_section(&mut snapshot.notes, "治理灯阵", &error, &mut unreachable);
            }
        }

        // 整体连不上 (一节都没回) 才报错; 否则逐节诚实回灌。
        if !any_response {
            if let Some(error) = unreachable {
                return Err(error);
            }
        }
        Ok(snapshot)
    }
}

/// 单节拉取失败的诚实记录: 备注 + 首个「连不上」错误留作整体错误。
fn note_section(
    notes: &mut Vec<String>,
    what: &str,
    error: &BackendError,
    unreachable: &mut Option<BackendError>,
) {
    notes.push(format!("{what}: {error}"));
    if matches!(error, BackendError::Unreachable { .. }) && unreachable.is_none() {
        *unreachable = Some(error.clone());
    }
}

/// 记忆件列表 → 记忆/保护计数 (窗口未饱和 = 页面全量 = 真实总数)。
fn ledger_from_episodes(items: &[Value], window: usize) -> (Option<u64>, Option<u64>, Vec<String>) {
    if items.len() >= window {
        return (
            None,
            None,
            vec![format!(
                "记忆/保护计数: 记忆页窗口饱和 (≥{window}), 既有契约无总数端点 → 显式留空"
            )],
        );
    }
    let memories = Some(items.len() as u64);
    let protected_flagged = items
        .iter()
        .filter(|item| {
            item.get("protected")
                .map(Value::is_boolean)
                .unwrap_or(false)
        })
        .count();
    if protected_flagged == items.len() {
        let protected = items
            .iter()
            .filter(|item| item.get("protected") == Some(&Value::Bool(true)))
            .count() as u64;
        (memories, Some(protected), Vec::new())
    } else {
        (
            memories,
            None,
            vec!["保护计数: 记忆页缺保护标记字段 → 显式留空".to_string()],
        )
    }
}

/// 治理事件流 → 灯阵事件 (未知判定不猜色不上灯, 返回未上灯数)。
fn governance_from_events(items: &[Value]) -> (Vec<GovernanceEvent>, usize) {
    let mut events = Vec::new();
    let mut unknown = 0usize;
    for item in items {
        let Some(verdict) = item
            .get("decision")
            .and_then(Value::as_str)
            .and_then(GovVerdict::parse)
        else {
            unknown += 1;
            continue;
        };
        events.push(GovernanceEvent {
            at_ms: item
                .get("timestamp_ms")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            capability: item
                .get("capability_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            verdict,
        });
    }
    events.sort_by_key(|event| event.at_ms);
    (events, unknown)
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

    /// 记忆件计数: 窗口未饱和 = 页面全量 = 真实总数; 饱和/缺字段显式留空。
    #[test]
    fn episode_counts_are_exact_only_inside_a_complete_window() {
        let items = vec![
            json!({"id": "e1", "protected": true}),
            json!({"id": "e2", "protected": false}),
            json!({"id": "e3", "protected": true}),
        ];
        let (memories, protected, notes) = ledger_from_episodes(&items, 500);
        assert_eq!(memories, Some(3));
        assert_eq!(protected, Some(2), "保护计数只数显式 true");
        assert!(notes.is_empty());

        // 窗口饱和: 不冒充总数, 两行都显式留空 + 备注。
        let full_window = vec![json!({"protected": false}); 500];
        let (memories, protected, notes) = ledger_from_episodes(&full_window, 500);
        assert_eq!(memories, None);
        assert_eq!(protected, None);
        assert!(notes[0].contains("窗口饱和"), "{notes:?}");

        // 保护标记缺失: 记忆计数照给, 保护计数留空 (不猜)。
        let mixed = vec![json!({"id": "e1", "protected": true}), json!({"id": "e2"})];
        let (memories, protected, notes) = ledger_from_episodes(&mixed, 500);
        assert_eq!(memories, Some(2));
        assert_eq!(protected, None);
        assert!(notes[0].contains("保护标记"), "{notes:?}");
    }

    /// 治理事件映射: 三色判定各有归宿, 未知判定不上灯 (不猜色)。
    #[test]
    fn guard_events_map_to_lamp_verdicts_without_guessing() {
        let items = vec![
            json!({"timestamp_ms": 300, "capability_id": "tool.c", "decision": "deny"}),
            json!({"timestamp_ms": 100, "capability_id": "tool.a", "decision": "allow"}),
            json!({"timestamp_ms": 200, "capability_id": "tool.b", "decision": "require_approval"}),
            json!({"timestamp_ms": 400, "capability_id": "tool.d", "decision": "maybe"}),
        ];
        let (events, unknown) = governance_from_events(&items);
        assert_eq!(unknown, 1, "未知判定不猜色不上灯");
        let order: Vec<(i64, GovVerdict)> = events
            .iter()
            .map(|event| (event.at_ms, event.verdict))
            .collect();
        assert_eq!(
            order,
            vec![
                (100, GovVerdict::Allow),
                (200, GovVerdict::Approve),
                (300, GovVerdict::Reject),
            ],
            "灯阵事件按时间序回灌"
        );
    }
}
