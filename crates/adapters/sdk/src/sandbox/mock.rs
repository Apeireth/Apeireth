//! # Mock orchestration service — 外部编排服务的 mock 边界
//!
//! [`MockOrchestrationService`] 按协议契约模拟外部编排服务的行为面
//! (创建/终止/销毁/巡检/列举/等待/日志), 供客户端协议层测试与本地演练用。
//! 它收发**协议帧 JSON**, 内部使用同一套编解码器, 因此客户端的编解码、
//! 版本校验、回声校验都在真实路径上。
//!
//! **诚实边界**: 这是 mock, 不是编排服务的真实实现。它只保证"协议契约上
//! 服务应该怎么回答", 不模拟任何真实容器/虚拟机运行时; 客户端代码对
//! mock 与真实服务一视同仁 (同一协议帧、同一错误闭合词表、同一 deadline)。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use uuid::Uuid;

use crate::sandbox::protocol::{
    self, OpResult, OrchestrationRequest, OrchestrationResponse, WireErrorCode,
};
use crate::sandbox::runtime::SandboxStatus;
use crate::sandbox::transport::{OrchestrationTransport, TransportError};
use crate::sandbox::{
    ExitCode, LogStream, LogStreamEvent, SandboxHandle, SANDBOX_MAX_LOG_CHUNK_BYTES,
};

fn now() -> SystemTime {
    SystemTime::now()
}

fn exit_code_of(handle: &SandboxHandle) -> ExitCode {
    match handle.exit_code {
        Some(0) | None => ExitCode::Ok,
        Some(code) => ExitCode::Failed(code),
    }
}

#[derive(Debug, Default)]
struct MockState {
    sandboxes: HashMap<Uuid, SandboxHandle>,
    log_lines: Vec<String>,
    requests_seen: Vec<OrchestrationRequest>,
    server_quota: Option<usize>,
    fail_next: Option<WireErrorCode>,
    delay: Option<Duration>,
    closed: bool,
}

/// 协议契约级 mock: 实现 [`OrchestrationTransport`], 内部状态线程安全。
#[derive(Debug, Default)]
pub struct MockOrchestrationService {
    state: Mutex<MockState>,
}

impl MockOrchestrationService {
    /// 新建空 mock (无配额上限、无延迟、无脚本失败)。
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, MockState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 脚本化: 下一次调用返回指定闭合错误码 (一次性)。
    pub fn fail_next(&self, code: WireErrorCode) {
        self.lock().fail_next = Some(code);
    }

    /// 脚本化: 每次调用先延迟 (用于 deadline 超时测试)。
    pub fn set_delay(&self, delay: Duration) {
        self.lock().delay = Some(delay);
    }

    /// 服务端并发配额上限 (None = 不限)。
    pub fn set_server_quota(&self, max_sandboxes: usize) {
        self.lock().server_quota = Some(max_sandboxes);
    }

    /// 追加一行脚本日志 (stream_logs 按序返回)。
    pub fn push_log_line(&self, line: impl Into<String>) {
        self.lock().log_lines.push(line.into());
    }

    /// 关闭传输 (后续调用返 [`TransportError::Closed`])。
    pub fn close(&self) {
        self.lock().closed = true;
    }

    /// 已见过的请求 (协议一致性断言用)。
    pub fn requests_seen(&self) -> Vec<OrchestrationRequest> {
        self.lock().requests_seen.clone()
    }

    /// 当前服务端活跃沙箱数。
    pub fn active_count(&self) -> usize {
        self.lock().sandboxes.len()
    }

    /// 服务端句柄快照 (None = 无此沙箱)。
    pub fn server_handle(&self, id: &Uuid) -> Option<SandboxHandle> {
        self.lock().sandboxes.get(id).cloned()
    }

    /// 直接改写服务端状态 (模拟状态漂移 / 状态巡检场景)。
    pub fn force_status(&self, id: &Uuid, status: SandboxStatus) -> bool {
        let mut state = self.lock();
        if let Some(handle) = state.sandboxes.get_mut(id) {
            handle.status = status;
            if status.is_terminal() {
                handle.finished_at = Some(now());
            }
            true
        } else {
            false
        }
    }

    /// 把服务端记录的启动时间回拨 (模拟超龄沙箱 / 状态巡检回收场景)。
    pub fn force_age(&self, id: &Uuid, age: Duration) -> bool {
        let mut state = self.lock();
        if let Some(handle) = state.sandboxes.get_mut(id) {
            handle.started_at = now().checked_sub(age).unwrap_or_else(now);
            true
        } else {
            false
        }
    }

    /// 让服务端"丢失"某沙箱记录 (模拟服务端回收后客户端残留)。
    pub fn drop_server(&self, id: &Uuid) -> bool {
        self.lock().sandboxes.remove(id).is_some()
    }
}

fn chunk_event(sandbox_id: Uuid, seq: u64, line: &str) -> LogStreamEvent {
    // 单 chunk 字节上限, 在 UTF-8 边界截断。
    let mut cut = line.len().min(SANDBOX_MAX_LOG_CHUNK_BYTES);
    while cut > 0 && !line.is_char_boundary(cut) {
        cut -= 1;
    }
    LogStreamEvent {
        sandbox_id,
        stream_id: Uuid::new_v4(),
        stream: LogStream::Stdout,
        data: line.as_bytes()[..cut].to_vec(),
        seq,
        timestamp: now(),
    }
}

fn err_response(code: WireErrorCode, detail: String) -> OrchestrationResponse {
    OrchestrationResponse::Err { code, detail }
}

fn ok(result: OpResult) -> OrchestrationResponse {
    OrchestrationResponse::Ok { result }
}

/// 服务端协议契约处理 (纯逻辑, 与传输无关)。
fn serve(state: &mut MockState, request: OrchestrationRequest) -> OrchestrationResponse {
    state.requests_seen.push(request.clone());
    if let Some(code) = state.fail_next.take() {
        return err_response(code, format!("scripted failure: {}", code.as_str()));
    }
    match request {
        OrchestrationRequest::Create { sandbox_id, config } => {
            if state.sandboxes.contains_key(&sandbox_id) {
                return err_response(
                    WireErrorCode::InvalidState,
                    format!("sandbox {sandbox_id} already exists"),
                );
            }
            if let Some(cap) = state.server_quota {
                if state.sandboxes.len() >= cap {
                    return err_response(
                        WireErrorCode::QuotaExceeded,
                        format!("server quota {cap} reached"),
                    );
                }
            }
            let handle = SandboxHandle {
                id: sandbox_id,
                status: SandboxStatus::Running,
                runtime: config.runtime,
                isolation: config.isolation,
                started_at: now(),
                finished_at: None,
                exit_code: None,
                error: None,
            };
            state.sandboxes.insert(sandbox_id, handle.clone());
            ok(OpResult::Created { handle })
        }
        OrchestrationRequest::Terminate { sandbox_id, signal } => {
            let Some(handle) = state.sandboxes.get_mut(&sandbox_id) else {
                return err_response(WireErrorCode::NotFound, sandbox_id.to_string());
            };
            handle.status = SandboxStatus::Stopped;
            handle.finished_at = Some(now());
            handle.exit_code = Some(if signal == Some(9) { 137 } else { 0 });
            ok(OpResult::Terminated {
                sandbox_id,
                status: SandboxStatus::Stopped,
            })
        }
        OrchestrationRequest::Destroy {
            sandbox_id,
            release_resources: _,
        } => {
            if state.sandboxes.remove(&sandbox_id).is_none() {
                return err_response(WireErrorCode::NotFound, sandbox_id.to_string());
            }
            ok(OpResult::Destroyed { sandbox_id })
        }
        OrchestrationRequest::Inspect { sandbox_id } => {
            let Some(handle) = state.sandboxes.get(&sandbox_id) else {
                return err_response(WireErrorCode::NotFound, sandbox_id.to_string());
            };
            ok(OpResult::Inspected {
                handle: handle.clone(),
            })
        }
        OrchestrationRequest::List => {
            let mut handles: Vec<SandboxHandle> = state.sandboxes.values().cloned().collect();
            handles.sort_by_key(|handle| handle.id);
            ok(OpResult::Listed { handles })
        }
        OrchestrationRequest::Wait {
            sandbox_id,
            timeout_ms: _,
        } => {
            let Some(handle) = state.sandboxes.get(&sandbox_id) else {
                return err_response(WireErrorCode::NotFound, sandbox_id.to_string());
            };
            let finished = handle.status.is_terminal();
            ok(OpResult::Waited {
                sandbox_id,
                exit_code: finished.then(|| exit_code_of(handle)),
                finished,
            })
        }
        OrchestrationRequest::Logs {
            sandbox_id,
            since_seq,
            max_chunks,
        } => {
            if !state.sandboxes.contains_key(&sandbox_id) {
                return err_response(WireErrorCode::NotFound, sandbox_id.to_string());
            }
            let total = state.log_lines.len() as u64;
            if since_seq >= total {
                // 空批收尾 (last=true), 表示当前无更多日志。
                return ok(OpResult::LogsChunks {
                    events: Vec::new(),
                    last: true,
                });
            }
            let end = (since_seq + max_chunks.max(1)).min(total);
            let events: Vec<LogStreamEvent> = state.log_lines[since_seq as usize..end as usize]
                .iter()
                .enumerate()
                .map(|(offset, line)| chunk_event(sandbox_id, since_seq + offset as u64, line))
                .collect();
            ok(OpResult::LogsChunks {
                events,
                last: end >= total,
            })
        }
    }
}

#[async_trait]
impl OrchestrationTransport for MockOrchestrationService {
    async fn call(&self, request_frame: String) -> Result<String, TransportError> {
        let delay = {
            let mut state = self.lock();
            if state.closed {
                return Err(TransportError::Closed);
            }
            state.delay
        };
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }

        // 帧解码失败: 以无法回声 request_id 的错误帧响应 (客户端侧会按
        // 协议违规收口), 不静默丢弃。
        let frame = match protocol::decode_request(&request_frame) {
            Ok(frame) => frame,
            Err(_) => {
                return protocol::encode_response(
                    Uuid::nil(),
                    err_response(WireErrorCode::Internal, "malformed request frame".into()),
                )
                .map_err(|_| TransportError::Unavailable("response encode failed".into()));
            }
        };

        let response = serve(&mut self.lock(), frame.body);
        protocol::encode_response(frame.request_id, response)
            .map_err(|_| TransportError::Unavailable("response encode failed".into()))
    }
}
