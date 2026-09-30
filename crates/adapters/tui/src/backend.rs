//! 后端链路: 驾驶舱 → 既有 gateway/CLI 后端的驱动投影。
//!
//! 会话语义、治理判定、工具执行、模型路由全部在后端; 本层只定义「驾驶舱
//! 需要什么」的最小端口 (与桌面端共用同一后端, 0 重复实现)。测试用
//! [`MockBackend`] 把命令链路全程记录下来。

/// token 计量快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageSnapshot {
    /// 输入 token。
    pub prompt_tokens: u32,
    /// 输出 token。
    pub completion_tokens: u32,
    /// 总计。
    pub total_tokens: u32,
    /// 缓存命中率 (接口无此字段时为 None, 上屏诚实显示 "—")。
    pub cache_hit_rate: Option<u32>,
}

/// 模型条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// 模型 id。
    pub id: String,
    /// 提供方。
    pub provider: String,
    /// 展示名 / 说明。
    pub description: Option<String>,
}

/// 会话账本条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    /// 会话 id。
    pub id: String,
    /// 标题。
    pub title: Option<String>,
    /// 最近更新时间 (epoch 毫秒)。
    pub updated_at: i64,
    /// 消息条数。
    pub message_count: usize,
}

/// 工具卡片事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolEvent {
    /// 事件阶段。
    pub kind: ToolEventKind,
    /// 工具名 (能力 id)。
    pub name: String,
    /// 事件时刻 (epoch 毫秒, 后端提供; 工具卡片算耗时用)。
    pub at_ms: u64,
}

/// 工具事件阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEventKind {
    /// 已派发, 执行中。
    Started,
    /// 已收口。
    Completed {
        /// 是否成功。
        ok: bool,
    },
}

/// 一次回合请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRequest {
    /// 既有会话 id; 缺省由后端新建。
    pub session: Option<String>,
    /// 用户输入。
    pub input: String,
    /// 模型覆盖。
    pub model: Option<String>,
}

/// 流式增量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnDelta {
    /// 文本增量 (逐 token)。
    Text(String),
    /// 工具卡片事件。
    Tool(ToolEvent),
    /// 计量快照 (终帧)。
    Usage(UsageSnapshot),
}

/// 回合收口方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishKind {
    /// 正常收口。
    Stop,
    /// 挂起等待审批。
    ApprovalRequired,
}

/// 回合结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    /// 实际使用的会话 id。
    pub session: String,
    /// 最终全文。
    pub text: String,
    /// 计量。
    pub usage: UsageSnapshot,
    /// 服务方能力 id。
    pub served_by: String,
    /// 提供方往返轮数。
    pub rounds: u32,
    /// 回合端到端耗时 (毫秒, 由链路执行层计量)。
    pub latency_ms: u64,
    /// 收口方式。
    pub finish: FinishKind,
}

/// 压缩报告 (后端接口就绪后透传; 链路形状先固定)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactReport {
    /// 会话 id。
    pub session: String,
    /// 压缩前消息数。
    pub before_messages: usize,
    /// 压缩后消息数。
    pub after_messages: usize,
}

/// 后端链路错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendError {
    /// 连不上后端。
    #[error("连接失败 [{endpoint}]: {detail}")]
    Unreachable {
        /// 端点。
        endpoint: String,
        /// 细节。
        detail: String,
    },
    /// 后端返回错误帧。
    #[error("后端错误帧 [HTTP {status} {code}]: {message} ({solution})")]
    Http {
        /// HTTP 状态。
        status: u16,
        /// 机器码。
        code: String,
        /// 人话消息。
        message: String,
        /// 处置建议。
        solution: String,
    },
    /// 能力未接线 (0 假装: 后端没有该端点就明说)。
    #[error("能力未接线 [{capability}]: {hint}")]
    NotWired {
        /// 能力 id。
        capability: &'static str,
        /// 处置建议。
        hint: String,
    },
    /// 响应形状不符合契约。
    #[error("响应不符合契约: {0}")]
    Protocol(String),
}

/// 后端结果别名。
pub type BackendResult<T> = Result<T, BackendError>;

/// 驾驶舱后端端口 (最小面)。
pub trait CockpitBackend: Send {
    /// 后端端点 (上屏用)。
    fn endpoint(&self) -> &str;

    /// 探活 (启动即探活: 连接失败也要第一帧上屏)。
    fn health(&mut self) -> BackendResult<()>;

    /// 可用模型 (模型热切换数据源)。
    fn list_models(&mut self) -> BackendResult<Vec<ModelInfo>>;

    /// 会话账本 (`/resume` 数据源)。
    fn list_sessions(&mut self) -> BackendResult<Vec<SessionMeta>>;

    /// 会话级模型热切换 (走既有会话设置链)。
    fn set_model(&mut self, session: Option<&str>, model: Option<&str>) -> BackendResult<String>;

    /// 发送一个回合, 增量经 `sink` 流出。
    fn send_turn(
        &mut self,
        request: TurnRequest,
        sink: &mut dyn FnMut(TurnDelta),
    ) -> BackendResult<TurnOutcome>;

    /// 压缩会话上下文。
    fn compact_session(&mut self, session: &str) -> BackendResult<CompactReport>;
}

/// 链路调用记录 (mock 后端的证据链)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendCall {
    /// 探活。
    Health,
    /// 列模型。
    ListModels,
    /// 列会话。
    ListSessions,
    /// 会话级模型切换。
    SetModel {
        /// 会话 id。
        session: Option<String>,
        /// 目标模型 (None = 复位默认)。
        model: Option<String>,
    },
    /// 发送回合。
    SendTurn {
        /// 会话 id。
        session: Option<String>,
        /// 输入。
        input: String,
        /// 模型覆盖。
        model: Option<String>,
    },
    /// 压缩会话。
    Compact {
        /// 会话 id。
        session: String,
    },
}

/// 脚本化 mock 后端: 记录全部调用, 回放预置增量。
#[derive(Debug, Default)]
pub struct MockBackend {
    /// 调用记录。
    pub calls: Vec<BackendCall>,
    /// 探活失败原因 (None = 成功)。
    pub fail_health: Option<String>,
    /// 预置会话账本。
    pub sessions: Vec<SessionMeta>,
    /// 预置模型列表。
    pub models: Vec<ModelInfo>,
    /// 回合文本增量脚本。
    pub turn_deltas: Vec<String>,
    /// 回合工具事件脚本。
    pub turn_tools: Vec<ToolEvent>,
    /// 回合计量。
    pub turn_usage: UsageSnapshot,
    /// 回合最终文本 (缺省 = 增量拼接)。
    pub turn_text: Option<String>,
    /// 实际服务的会话 id (缺省 = 请求会话或 `mock-session`)。
    pub served_session: Option<String>,
    /// 压缩报告。
    pub compact_report: Option<CompactReport>,
    /// 压缩失败原因。
    pub fail_compact: Option<String>,
    /// 回合失败原因。
    pub fail_turn: Option<String>,
    /// 列表失败原因。
    pub fail_list: Option<String>,
}

impl MockBackend {
    /// 便捷构造: 一个默认回合。
    pub fn with_reply(reply: &str) -> Self {
        Self {
            turn_deltas: vec![reply.to_string()],
            ..Self::default()
        }
    }
}

impl CockpitBackend for MockBackend {
    fn endpoint(&self) -> &str {
        "mock://cockpit"
    }

    fn health(&mut self) -> BackendResult<()> {
        self.calls.push(BackendCall::Health);
        match &self.fail_health {
            Some(detail) => Err(BackendError::Unreachable {
                endpoint: self.endpoint().to_string(),
                detail: detail.clone(),
            }),
            None => Ok(()),
        }
    }

    fn list_models(&mut self) -> BackendResult<Vec<ModelInfo>> {
        self.calls.push(BackendCall::ListModels);
        if let Some(detail) = &self.fail_list {
            return Err(BackendError::Protocol(detail.clone()));
        }
        Ok(self.models.clone())
    }

    fn list_sessions(&mut self) -> BackendResult<Vec<SessionMeta>> {
        self.calls.push(BackendCall::ListSessions);
        if let Some(detail) = &self.fail_list {
            return Err(BackendError::Protocol(detail.clone()));
        }
        Ok(self.sessions.clone())
    }

    fn set_model(&mut self, session: Option<&str>, model: Option<&str>) -> BackendResult<String> {
        self.calls.push(BackendCall::SetModel {
            session: session.map(str::to_string),
            model: model.map(str::to_string),
        });
        Ok(model.unwrap_or("默认模型").to_string())
    }

    fn send_turn(
        &mut self,
        request: TurnRequest,
        sink: &mut dyn FnMut(TurnDelta),
    ) -> BackendResult<TurnOutcome> {
        self.calls.push(BackendCall::SendTurn {
            session: request.session.clone(),
            input: request.input.clone(),
            model: request.model.clone(),
        });
        if let Some(detail) = &self.fail_turn {
            return Err(BackendError::Protocol(detail.clone()));
        }
        for tool in &self.turn_tools {
            sink(TurnDelta::Tool(tool.clone()));
        }
        let mut text = String::new();
        for delta in &self.turn_deltas {
            text.push_str(delta);
            sink(TurnDelta::Text(delta.clone()));
        }
        sink(TurnDelta::Usage(self.turn_usage));
        Ok(TurnOutcome {
            session: self
                .served_session
                .clone()
                .or_else(|| request.session.clone())
                .unwrap_or_else(|| "mock-session".to_string()),
            text: self.turn_text.clone().unwrap_or(text),
            usage: self.turn_usage,
            served_by: "mock.provider".to_string(),
            rounds: 1,
            latency_ms: 0,
            finish: FinishKind::Stop,
        })
    }

    fn compact_session(&mut self, session: &str) -> BackendResult<CompactReport> {
        self.calls.push(BackendCall::Compact {
            session: session.to_string(),
        });
        if let Some(detail) = &self.fail_compact {
            return Err(BackendError::Protocol(detail.clone()));
        }
        Ok(self.compact_report.clone().unwrap_or(CompactReport {
            session: session.to_string(),
            before_messages: 0,
            after_messages: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// mock 后端把会话管理链路逐跳记录下来, 增量按序流出。
    #[test]
    fn mock_backend_records_calls_and_streams_deltas() {
        let mut backend = MockBackend::with_reply("你好");
        backend.sessions.push(SessionMeta {
            id: "s-1".to_string(),
            title: Some("t".to_string()),
            updated_at: 0,
            message_count: 1,
        });
        backend.health().unwrap();
        let sessions = backend.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        let mut deltas = Vec::new();
        let outcome = backend
            .send_turn(
                TurnRequest {
                    session: Some("s-1".to_string()),
                    input: "hi".to_string(),
                    model: None,
                },
                &mut |delta| deltas.push(delta),
            )
            .unwrap();
        assert_eq!(outcome.session, "s-1");
        assert_eq!(
            deltas,
            vec![
                TurnDelta::Text("你好".to_string()),
                TurnDelta::Usage(UsageSnapshot::default()),
            ]
        );
        assert!(backend.calls.contains(&BackendCall::Health));
        assert!(backend.calls.contains(&BackendCall::ListSessions));
        assert!(backend.calls.contains(&BackendCall::SendTurn {
            session: Some("s-1".to_string()),
            input: "hi".to_string(),
            model: None,
        }));
    }
}
