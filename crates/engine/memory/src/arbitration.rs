//! `apeireth-memory::arbitration` — HASH-SQL 唯一事实时间线与不可篡改审计仲裁机 (Arbitration Engine).
//!
//! ## 核心哲学 (S-2 实事求是 + O-1 安全优先)
//! 多源输入（前端桌面、CLI、多 Agent 协作、网关）在进入认知与持久化时，
//! 必须通过统一的确定性哈希链仲裁：
//! - 每一项事件在产生时计算严格规范化的 SHA-256 内容哈希 (`content_hash`)；
//! - 串联上一条记录的哈希 (`prev_hash`) 形成防篡改审计链条；
//! - 仲裁时以 `(timestamp_ms, content_hash, seq)` 三元组确定唯一不可篡改的事实时间线；
//! - 任何外界注入或数据库直接篡改均可通过 `verify_chain` 和 Merkle Root 瞬时检出。
//!
//! ## 安全与纯粹性
//! - 纯 Safe Rust (`#![deny(unsafe_code)]`)，0 未定义行为；
//! - 哈希比较采用常数时间比较 (`constant_time_eq`) 杜绝时序侧信道攻击；
//! - 内存与持久化双向对齐，0 外部不可信 C-FFI。

#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// 仲裁错误.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArbitrationError {
    #[error("哈希链断裂: 序号 {seq} 处的期望前驱哈希为 {expected}, 实际为 {actual}")]
    BrokenHashChain {
        seq: u64,
        expected: String,
        actual: String,
    },
    #[error("内容哈希校验失败: 序号 {seq} 处内容被篡改 (期望 {expected}, 计算值 {calculated})")]
    ContentTampered {
        seq: u64,
        expected: String,
        calculated: String,
    },
    #[error("输入有效载荷为空")]
    EmptyPayload,
}

/// 跨前端、终端、多 Agent 与系统的统一事件源.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSource {
    /// 前端桌面端伴侣 (Svelte 5 / Tauri 2).
    Frontend,
    /// 命令行 CLI.
    Cli,
    /// HTTP / SSE 网关.
    Gateway,
    /// Agent 间协作 (Subagent 委派).
    AgentComm,
    /// 系统内部心跳 / 调度 / 做梦.
    System,
    /// 外部插件 / MCP 工具.
    External,
}

impl EventSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::Cli => "cli",
            Self::Gateway => "gateway",
            Self::AgentComm => "agent_comm",
            Self::System => "system",
            Self::External => "external",
        }
    }
}

/// 单条仲裁事件记录.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArbitrationEvent {
    /// 单调自增序号 (从 1 起始).
    pub seq: u64,
    /// 事件源.
    pub source: EventSource,
    /// 所属会话 ID.
    pub session_id: String,
    /// 事件类型 (如 "message_input", "tool_call", "dream_cycle", "memory_append").
    pub event_type: String,
    /// 规范化 JSON 载荷.
    pub payload_json: String,
    /// 当前记录的内容 SHA-256 哈希.
    pub content_hash: String,
    /// 上一条记录的 SHA-256 哈希 (首条记录为 64 个 '0').
    pub prev_hash: String,
    /// 时间戳 (ms).
    pub timestamp_ms: i64,
}

fn encode_hex(bytes: impl AsRef<[u8]>) -> String {
    use std::fmt::Write;
    let bytes = bytes.as_ref();
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

impl ArbitrationEvent {
    /// 计算单条记录的规范内容哈希.
    pub fn compute_hash(
        seq: u64,
        source: EventSource,
        session_id: &str,
        event_type: &str,
        payload_json: &str,
        prev_hash: &str,
        timestamp_ms: i64,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(seq.to_be_bytes());
        hasher.update(source.as_str().as_bytes());
        hasher.update(session_id.as_bytes());
        hasher.update(event_type.as_bytes());
        hasher.update(payload_json.as_bytes());
        hasher.update(prev_hash.as_bytes());
        hasher.update(timestamp_ms.to_be_bytes());
        encode_hex(hasher.finalize())
    }
}

/// 完整性验证结果报告.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityReport {
    /// 校验通过的记录总数.
    pub verified_count: usize,
    /// 整体 Merkle 根哈希.
    pub merkle_root: String,
    /// 链条是否完整无篡改.
    pub is_valid: bool,
}

/// 常数时间字符串比较 (防时序侧信道).
fn constant_time_eq_str(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// HASH-SQL 仲裁机引擎.
#[derive(Debug, Clone, Default)]
pub struct ArbitrationEngine {
    events: Arc<Mutex<Vec<ArbitrationEvent>>>,
}

impl ArbitrationEngine {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 仲裁并追加一条新事件.
    pub fn append_event(
        &self,
        source: EventSource,
        session_id: &str,
        event_type: &str,
        payload_json: &str,
    ) -> Result<ArbitrationEvent, ArbitrationError> {
        let payload_json = payload_json.trim();
        if payload_json.is_empty() {
            return Err(ArbitrationError::EmptyPayload);
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let mut lock = self.events.lock().unwrap();
        let seq = (lock.len() as u64) + 1;
        let prev_hash = if let Some(last) = lock.last() {
            last.content_hash.clone()
        } else {
            "0".repeat(64)
        };

        let content_hash = ArbitrationEvent::compute_hash(
            seq,
            source,
            session_id,
            event_type,
            payload_json,
            &prev_hash,
            now_ms,
        );

        let event = ArbitrationEvent {
            seq,
            source,
            session_id: session_id.to_string(),
            event_type: event_type.to_string(),
            payload_json: payload_json.to_string(),
            content_hash,
            prev_hash,
            timestamp_ms: now_ms,
        };

        lock.push(event.clone());
        Ok(event)
    }

    /// 校验全量历史事件链的哈希完整性.
    pub fn verify_integrity(&self) -> Result<IntegrityReport, ArbitrationError> {
        let lock = self.events.lock().unwrap();
        let mut expected_prev_hash = "0".repeat(64);

        for event in lock.iter() {
            // 1. 验证前驱哈希连续性
            if !constant_time_eq_str(&event.prev_hash, &expected_prev_hash) {
                return Err(ArbitrationError::BrokenHashChain {
                    seq: event.seq,
                    expected: expected_prev_hash,
                    actual: event.prev_hash.clone(),
                });
            }

            // 2. 重新计算并验证内容哈希
            let computed = ArbitrationEvent::compute_hash(
                event.seq,
                event.source,
                &event.session_id,
                &event.event_type,
                &event.payload_json,
                &event.prev_hash,
                event.timestamp_ms,
            );

            if !constant_time_eq_str(&event.content_hash, &computed) {
                return Err(ArbitrationError::ContentTampered {
                    seq: event.seq,
                    expected: event.content_hash.clone(),
                    calculated: computed,
                });
            }

            expected_prev_hash = event.content_hash.clone();
        }

        let merkle_root = Self::compute_merkle_root(&lock);

        Ok(IntegrityReport {
            verified_count: lock.len(),
            merkle_root,
            is_valid: true,
        })
    }

    /// 计算一组事件的 Merkle Root.
    pub fn compute_merkle_root(events: &[ArbitrationEvent]) -> String {
        if events.is_empty() {
            return "0".repeat(64);
        }

        let mut current_layer: Vec<String> =
            events.iter().map(|e| e.content_hash.clone()).collect();

        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            for chunk in current_layer.chunks(2) {
                let mut hasher = Sha256::new();
                if chunk.len() == 2 {
                    hasher.update(chunk[0].as_bytes());
                    hasher.update(chunk[1].as_bytes());
                } else {
                    // 奇数节点自身与自身哈希
                    hasher.update(chunk[0].as_bytes());
                    hasher.update(chunk[0].as_bytes());
                }
                next_layer.push(encode_hex(hasher.finalize()));
            }
            current_layer = next_layer;
        }

        current_layer[0].clone()
    }

    /// 获取事件总数.
    pub fn count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

// ============================================================
// 单元测试集
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_verify_hash_chain() {
        let engine = ArbitrationEngine::new();
        assert_eq!(engine.count(), 0);

        let ev1 = engine
            .append_event(
                EventSource::Frontend,
                "sess_1",
                "chat_message",
                r#"{"text":"你好，阿佩瑞斯"}"#,
            )
            .unwrap();

        assert_eq!(ev1.seq, 1);
        assert_eq!(ev1.prev_hash, "0".repeat(64));

        let ev2 = engine
            .append_event(
                EventSource::AgentComm,
                "sess_1",
                "subagent_decision",
                r#"{"action":"analyze_repo"}"#,
            )
            .unwrap();

        assert_eq!(ev2.seq, 2);
        assert_eq!(ev2.prev_hash, ev1.content_hash);

        let report = engine.verify_integrity().unwrap();
        assert!(report.is_valid);
        assert_eq!(report.verified_count, 2);
        assert_ne!(report.merkle_root, "0".repeat(64));
    }

    #[test]
    fn test_tamper_detection_catches_mutation() {
        let engine = ArbitrationEngine::new();
        engine
            .append_event(EventSource::Gateway, "s1", "m1", r#"{"k":"v1"}"#)
            .unwrap();
        engine
            .append_event(EventSource::Gateway, "s1", "m2", r#"{"k":"v2"}"#)
            .unwrap();

        // 模拟外部恶意篡改第一条记录的载荷
        {
            let mut lock = engine.events.lock().unwrap();
            lock[0].payload_json = r#"{"k":"malicious_tamper"}"#.to_string();
        }

        let err = engine.verify_integrity().unwrap_err();
        match err {
            ArbitrationError::ContentTampered { seq, .. } => {
                assert_eq!(seq, 1);
            }
            _ => panic!("Expected ContentTampered error"),
        }
    }
}
