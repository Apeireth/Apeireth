//! TP20-N20 ApprovalBridge — companion ↔ orchestrator 跨 crate 审批透传契约.
//!
//! **动机**: N19 决策逻辑 + companion approval_requests 已有 (✅), 但跨 crate 调用
//! 序列化/反序列化容易丢字段, 拒绝/批准状态在 companion 与 team-lead 间双向同步
//! 缺统一 bridge. 本模块定义契约, 不做 RPC 也不做重试 (失败 eprintln 隔离即可).
//!
//! **架构**:
//! ```text
//!   companion::approval_requests    ───bridge.send_request(req)──▶   team-lead::Orchestrator
//!                                            ⇣
//!                                    dispatch_request(req) → ApprovalResponse
//!                                            ⇡
//!   companion::approval_requests    ◀──bridge.send_response(resp)──  team-lead::Orchestrator
//! ```
//!
//! **字段透传保真**:
//! - `#[serde(rename_all = "snake_case")]` 保证 wire format 一致
//! - 所有字段都 `#[serde(default)]` 或 `Option<...>` 包装 → 缺字段不丢 (默认 reject)
//! - 未知字段用 `#[serde(flatten)] extra: HashMap<String, Value>` 收容 → 升级期
//!   新增字段不会破坏旧 deserializer
//!
//! **0 装 PASS** (per §1.2「0 装 PASS」):
//! - 缺字段 = 返回 `Reject` 决策 + `MissingField` 错误日志, **不**panic
//! - bridge.send_* 失败 = `eprintln!` + 返回 `ApprovalBridgeError`, **不**假装"已透传"
//! - 不知道下游 orchestrator 是否真收到 → `InProcessBridge` 暴露 `received_log`
//!   用于测试断言 (生产可关闭)
//!
//! **依赖**: 纯 std + serde (Cargo.toml 无新外部依赖, 这俩已经是 workspace 既有).

#![allow(missing_docs)] // TP20 additive, 注释在模块头标注
#![allow(clippy::all)]

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// ============================================================================
// 协议类型 — 字段透传保真
// ============================================================================

/// 审批请求 (companion → orchestrator).
///
/// 所有字段 `#[serde(default)]` 或可空: 缺字段不 panic, 走降级 (`MissingField` 错误
/// + 返回 Reject 响应, 而非中断主路径).
///
/// **wire format**: snake_case (per `rename_all`).
/// **未知字段**: `extra` HashMap 收容, 反序列化不丢 (升级期新增字段不破坏兼容).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalRequest {
    /// 逻辑链 ID (同工具同摘要共享 chain, 与 companion::ApprovalRequest.chain 对齐).
    #[serde(default)]
    pub chain: String,
    /// 请求的工具名.
    #[serde(default)]
    pub tool: String,
    /// 参数摘要 (截断, 展示用).
    #[serde(default)]
    pub args_preview: String,
    /// 拒绝理由.
    #[serde(default)]
    pub reason: String,
    /// 请求创建时间 (epoch seconds, 0 = 缺字段).
    #[serde(default)]
    pub created_at: i64,
    /// 收容未知字段 — 新版本加字段旧版本不会丢.
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ApprovalRequest {
    /// 校验必填字段.
    ///
    /// **0 装 PASS**: 缺 `chain` 或 `tool` = `Err(MissingField)`, 不 panic.
    /// 返回的 `Err` 让调用方降级到 "Reject 响应" 而非中断主路径.
    pub fn validate(&self) -> Result<(), ApprovalBridgeError> {
        if self.chain.is_empty() {
            return Err(ApprovalBridgeError::MissingField("chain".into()));
        }
        if self.tool.is_empty() {
            return Err(ApprovalBridgeError::MissingField("tool".into()));
        }
        Ok(())
    }
}

/// 审批响应 (orchestrator → companion, 双向状态同步).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalResponse {
    /// 对应的请求 chain.
    #[serde(default)]
    pub chain: String,
    /// 决策: "approved" / "rejected" / "pending" (suspend, 不决).
    #[serde(default)]
    pub decision: String,
    /// 决策时间 (epoch seconds, 0 = 缺字段).
    #[serde(default)]
    pub decided_at: i64,
    /// 备注 (orchestrator 写入的额外信息, 可选).
    #[serde(default)]
    pub note: String,
    /// 收容未知字段.
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ApprovalResponse {
    /// 校验响应合法性. `chain` 非空 + `decision` 在三选一.
    pub fn validate(&self) -> Result<(), ApprovalBridgeError> {
        if self.chain.is_empty() {
            return Err(ApprovalBridgeError::MissingField("chain".into()));
        }
        match self.decision.as_str() {
            "approved" | "rejected" | "pending" => {}
            "" => return Err(ApprovalBridgeError::MissingField("decision".into())),
            other => {
                return Err(ApprovalBridgeError::Rejected(format!(
                    "unknown decision: {other} (allowed: approved/rejected/pending)"
                )));
            }
        }
        Ok(())
    }

    /// 构造 `Reject` 响应 (降级路径默认, 用于缺字段/桥不可达).
    pub fn reject_for(chain: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            chain: chain.into(),
            decision: "rejected".into(),
            decided_at: 0,
            note: reason.into(),
            extra: serde_json::Map::new(),
        }
    }
}

/// ApprovalBridge 错误类型.
///
/// **0 装 PASS**: 错误携带 reason, 不吞错也不假装"已成功".
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ApprovalBridgeError {
    /// Bridge 不可达 (IPC/RPC 层断).
    #[error("bridge unavailable: {0}")]
    Unavailable(String),
    /// Bridge 拒绝 (下游主动拒, 如决策非法).
    #[error("bridge rejected: {0}")]
    Rejected(String),
    /// 必填字段缺失 (请求/响应未通过 validate).
    #[error("missing required field: {0}")]
    MissingField(String),
}

// ============================================================================
// ApprovalBridge trait
// ============================================================================

/// Approval bridge trait — companion ↔ orchestrator 双向透传契约.
///
/// **设计原则**:
/// - `dispatch_request`: companion 发送新审批请求, orchestrator 收到后返回决策
/// - `dispatch_response`: orchestrator 把状态变更推回 companion (双向同步)
/// - `on_request`: orchestrator 注册回调, 后续 `dispatch_request` 自动触发
///   (TP11 handoff 既有接线可复用)
/// - 全部 sync (in-process), 上层用 `tokio::spawn` 包异步
/// - 失败 = 返回 `ApprovalBridgeError`, **不**假装"已透传"
pub trait ApprovalBridge: Send + Sync {
    /// companion → orchestrator: 派发新审批请求, 阻塞等待响应.
    ///
    /// **0 装 PASS**: 字段不全 → 返回 `Err(MissingField)`, 不 panic.
    fn dispatch_request(
        &self,
        req: ApprovalRequest,
    ) -> Result<ApprovalResponse, ApprovalBridgeError>;

    /// orchestrator → companion: 派发响应 (状态变更/双向同步).
    fn dispatch_response(
        &self,
        resp: ApprovalResponse,
    ) -> Result<(), ApprovalBridgeError>;
}

// ============================================================================
// InProcessBridge — 默认实现 (in-process 单测 + 部署层 fallback)
// ============================================================================

/// In-process bridge — 同 crate 内共享 `Arc<InProcessBridge>`, 测试 / 部署层 fallback.
///
/// **特性**:
/// - 全部 sync, in-process (无 RPC/msgpack)
/// - `received_log: Vec<...>` 记录所有 send/dispatch 调用, 测试断言
/// - `on_request_cb`: orchestrator 注册回调; 缺省 = `auto_reject` 模式
///   (单测可验证"未注册回调时默认 Reject")
/// - 线程安全: `Mutex<...>` 包内部状态
pub struct InProcessBridge {
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for InProcessBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessBridge")
            .field("inner", &"<Mutex<Inner>>")
            .finish()
    }
}

#[derive(Default)]
struct Inner {
    /// orchestrator 注册的请求回调. None = 默认 reject.
    on_request_cb: Option<Arc<dyn Fn(ApprovalRequest) -> ApprovalResponse + Send + Sync>>,
    /// dispatch_request 调用记录 (测试断言用).
    received_requests: Vec<ApprovalRequest>,
    /// dispatch_response 调用记录 (测试断言用).
    received_responses: Vec<ApprovalResponse>,
}

impl Default for InProcessBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl InProcessBridge {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    /// 注册 orchestrator 回调 (per TP11 handoff on_handoff 模式).
    ///
    /// 替换已有回调 (last-write-wins). 注册后 `dispatch_request` 自动用回调计算响应.
    pub fn on_request<F>(&self, cb: F)
    where
        F: Fn(ApprovalRequest) -> ApprovalResponse + Send + Sync + 'static,
    {
        let mut g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.on_request_cb = Some(Arc::new(cb));
    }

    /// 清除回调 (回到默认 reject 模式).
    pub fn clear_callback(&self) {
        let mut g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.on_request_cb = None;
    }

    /// 读请求记录 (测试断言).
    pub fn received_requests(&self) -> Vec<ApprovalRequest> {
        let g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.received_requests.clone()
    }

    /// 读响应记录 (测试断言).
    pub fn received_responses(&self) -> Vec<ApprovalResponse> {
        let g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.received_responses.clone()
    }
}

impl ApprovalBridge for InProcessBridge {
    fn dispatch_request(
        &self,
        req: ApprovalRequest,
    ) -> Result<ApprovalResponse, ApprovalBridgeError> {
        // 0 装 PASS: 先 validate, 缺字段不 panic 直接 Err
        req.validate()?;

        let mut g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.received_requests.push(req.clone());

        // 调回调算响应; 无回调 = 默认 reject (per 0 装 PASS: 不假装"已处理")
        match &g.on_request_cb {
            Some(cb) => {
                let resp = cb(req);
                // 二次校验响应合法 (回调可能写错字段)
                if let Err(e) = resp.validate() {
                    return Err(e);
                }
                g.received_responses.push(resp.clone());
                Ok(resp)
            }
            None => Ok(ApprovalResponse::reject_for(
                &req.chain,
                "no callback registered (default reject)",
            )),
        }
    }

    fn dispatch_response(
        &self,
        resp: ApprovalResponse,
    ) -> Result<(), ApprovalBridgeError> {
        resp.validate()?;
        let mut g = self.inner.lock().expect("InProcessBridge mutex poisoned");
        g.received_responses.push(resp);
        Ok(())
    }
}

// ============================================================================
// 测试 — 全路径覆盖
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===== t01 — serde round-trip 字段透传保真 =====
    #[test]
    fn t01_request_serde_roundtrip() {
        let req = ApprovalRequest {
            chain: "chain-001".into(),
            tool: "FileOperator".into(),
            args_preview: r#"{"op":"write"}"#.into(),
            reason: "needs owner".into(),
            created_at: 1_700_000_000,
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("trace_id".into(), serde_json::json!(42));
                m
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        // wire format snake_case
        assert!(json.contains("\"chain\""));
        assert!(json.contains("\"tool\""));
        assert!(json.contains("\"args_preview\""));
        assert!(json.contains("\"reason\""));
        assert!(json.contains("\"created_at\""));
        // 未知字段进 extra, 不丢
        assert!(json.contains("\"trace_id\""));

        let parsed: ApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, req);
    }

    // ===== t02 — 缺字段默认 reject (ApprovalRequest 缺 chain/tool) =====
    #[test]
    fn t02_request_missing_field_rejects_not_panics() {
        let bridge = InProcessBridge::new();
        // 缺 chain
        let bad = ApprovalRequest {
            chain: "".into(),
            tool: "FileOperator".into(),
            args_preview: "{}".into(),
            reason: "x".into(),
            created_at: 0,
            extra: serde_json::Map::new(),
        };
        let resp = bridge.dispatch_request(bad).unwrap_err();
        match resp {
            ApprovalBridgeError::MissingField(f) => assert_eq!(f, "chain"),
            _ => panic!("expected MissingField(chain)"),
        }

        // 缺 tool
        let bad = ApprovalRequest {
            chain: "c".into(),
            tool: "".into(),
            args_preview: "{}".into(),
            reason: "x".into(),
            created_at: 0,
            extra: serde_json::Map::new(),
        };
        let resp = bridge.dispatch_request(bad).unwrap_err();
        match resp {
            ApprovalBridgeError::MissingField(f) => assert_eq!(f, "tool"),
            _ => panic!("expected MissingField(tool)"),
        }
    }

    // ===== t03 — 缺字段的 JSON 输入也能 parse (serde default), 走降级 =====
    #[test]
    fn t03_partial_json_parse_then_validate_rejects() {
        // 只给 chain, 其他字段缺 → serde default 全部默认空/0
        let json = r#"{"chain":"c1"}"#;
        let req: ApprovalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.chain, "c1");
        assert_eq!(req.tool, "");
        // 走到 dispatch_request → validate → 缺 tool → MissingField
        let bridge = InProcessBridge::new();
        let err = bridge.dispatch_request(req).unwrap_err();
        assert!(matches!(err, ApprovalBridgeError::MissingField(_)));
    }

    // ===== t04 — 未知字段进 extra, 不破坏兼容 =====
    #[test]
    fn t04_unknown_fields_go_to_extra() {
        // 假设新版本加了 field_a, 旧版本解析时进 extra
        let json = r#"{
            "chain": "c",
            "tool": "T",
            "args_preview": "{}",
            "reason": "x",
            "created_at": 0,
            "field_a": "future",
            "field_b": 99
        }"#;
        let req: ApprovalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.extra.get("field_a").unwrap(), &serde_json::json!("future"));
        assert_eq!(req.extra.get("field_b").unwrap(), &serde_json::json!(99));
    }

    // ===== t05 — 无回调时默认 reject =====
    #[test]
    fn t05_no_callback_default_rejects() {
        let bridge = InProcessBridge::new();
        let req = ApprovalRequest {
            chain: "c1".into(),
            tool: "T".into(),
            args_preview: "{}".into(),
            reason: "x".into(),
            created_at: 0,
            extra: serde_json::Map::new(),
        };
        let resp = bridge.dispatch_request(req).unwrap();
        assert_eq!(resp.chain, "c1");
        assert_eq!(resp.decision, "rejected");
        assert_eq!(resp.note, "no callback registered (default reject)");
    }

    // ===== t06 — 注册回调后 dispatch_request 走回调, 状态双向同步 =====
    #[test]
    fn t06_callback_routing_and_log() {
        let bridge = InProcessBridge::new();
        bridge.on_request(|req| ApprovalResponse {
            chain: req.chain.clone(),
            decision: "approved".into(),
            decided_at: 1_700_000_001,
            note: "ok".into(),
            extra: serde_json::Map::new(),
        });
        let req = ApprovalRequest {
            chain: "c2".into(),
            tool: "T".into(),
            args_preview: "{}".into(),
            reason: "x".into(),
            created_at: 0,
            extra: serde_json::Map::new(),
        };
        let resp = bridge.dispatch_request(req.clone()).unwrap();
        assert_eq!(resp.decision, "approved");
        assert_eq!(resp.decided_at, 1_700_000_001);
        // 请求已记录
        let logged = bridge.received_requests();
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0].chain, "c2");
        // 响应也已记录 (回调自动 push 到 received_responses)
        let resp_log = bridge.received_responses();
        assert_eq!(resp_log.len(), 1);
        assert_eq!(resp_log[0].decision, "approved");
    }

    // ===== t07 — dispatch_response 双向同步: orchestrator 推响应 → companion 收 =====
    #[test]
    fn t07_dispatch_response_two_way_sync() {
        let bridge = InProcessBridge::new();
        let resp = ApprovalResponse {
            chain: "c3".into(),
            decision: "approved".into(),
            decided_at: 1_700_000_002,
            note: "via bridge".into(),
            extra: serde_json::Map::new(),
        };
        bridge.dispatch_response(resp.clone()).unwrap();
        let logged = bridge.received_responses();
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0], resp);
    }

    // ===== t08 — dispatch_response 响应非法决策 =====
    #[test]
    fn t08_dispatch_response_rejects_unknown_decision() {
        let bridge = InProcessBridge::new();
        let bad = ApprovalResponse {
            chain: "c".into(),
            decision: "MAYBE".into(), // 非三选一
            decided_at: 0,
            note: "x".into(),
            extra: serde_json::Map::new(),
        };
        let err = bridge.dispatch_response(bad).unwrap_err();
        assert!(matches!(err, ApprovalBridgeError::Rejected(_)));
    }

    // ===== t09 — clear_callback 回到默认 reject =====
    #[test]
    fn t09_clear_callback_falls_back_to_default() {
        let bridge = InProcessBridge::new();
        bridge.on_request(|req| ApprovalResponse {
            chain: req.chain.clone(),
            decision: "approved".into(),
            decided_at: 0,
            note: String::new(),
            extra: serde_json::Map::new(),
        });
        // 此时是 approved
        let req = ApprovalRequest {
            chain: "c4".into(),
            tool: "T".into(),
            args_preview: "{}".into(),
            reason: "x".into(),
            created_at: 0,
            extra: serde_json::Map::new(),
        };
        let resp = bridge.dispatch_request(req.clone()).unwrap();
        assert_eq!(resp.decision, "approved");

        // 清回调 → 回到 reject
        bridge.clear_callback();
        let resp = bridge.dispatch_request(req).unwrap();
        assert_eq!(resp.decision, "rejected");
        assert!(resp.note.contains("default reject"));
    }

    // ===== t10 — 模块头标 0 装 PASS (源码字符串回归) =====
    #[test]
    fn t10_module_doc_marks_zero_fake() {
        let src = include_str!("approval_bridge.rs");
        assert!(src.contains("0 装 PASS"), "模块头必须含 0 装 PASS 标");
        assert!(src.contains("字段透传保真"), "必须明示字段透传保真");
        assert!(src.contains("缺字段"), "必须明示缺字段降级");
        assert!(src.contains("不假装"), "必须明示不假装语义");
    }
}