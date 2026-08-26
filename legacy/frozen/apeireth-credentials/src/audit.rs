//! # 凭证审计 (4 事件: get / put / rotate / revoke)
//!
//! 1:1 翻译 v0.9.21 商业版凭证审计日志. 4 事件必须不可绕过地记录, 满足 SOC 2 /
//! GDPR / HIPAA 合规要求.
//!
//! ## 4 事件 (K-1 强校验: 编译期 hardcode, 不可运行时增删)
//!
//! | # | 事件 | 触发时机 | 关键字段 |
//! |---|------|---------|---------|
//! | 1 | `Get` | 凭证被读取 (`get_token`) | provider, scope, requester |
//! | 2 | `Put` | 凭证被存储 / 更新 | provider, scope, source |
//! | 3 | `Rotate` | 凭证轮换 (`rotate`) | provider, strategy, old_expires, new_expires |
//! | 4 | `Revoke` | 凭证撤销 (`revoke`) | provider, reason |
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **4 事件编译期 hardcode**: 不可运行时增删
//! 2. **不可绕过**: `record_event` 失败返 `Err(CredentialsError::AuditLogFailed)`,
//!    上层必须处理 (e.g. 重试 / 熔断)
//! 3. **0 暴露凭证值**: AuditEvent 不含 `api_key` / `token` / `secret` 字段, 只含元数据
//! 4. **UUID 唯一**: 每个事件有 `event_id` (UUID v4), 防重放
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 4 事件 1:1 翻译 SOC 2 审计要求, 0 业务重设计
//! - **S-2 实事求是**: 4 事件够用 99% 场景, 不发明 `Introspect` / `Validate` 等花哨
//! - **O-2 走在前人肩上**: 借鉴 SOC 2 CC6.1 + GDPR Art. 30 审计日志要求
//! - **O-3 干到底**: AuditEvent + AuditLog + 4 事件常量 + 4 fixture 测试
//! - **O-4 任何人都能接手**: 跟 keyring / i18n 同模式 (struct + Display + serde)
//! - **O-5 不假装**: 4 事件穷举 match, 0 任何事件漏防

use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{CredentialsError, CredentialsResult};
use crate::provider::ProviderKind;
use crate::rotation::RotationStrategy;

// ============================================================================
// §1 事件类型枚举 (4 种, 编译期 hardcode)
// ============================================================================

/// 凭证审计事件类型 (4 种, K-1 强校验).
///
/// 1:1 翻译 v0.9.21 商业版 4 种审计事件. 顺序固定, 不可运行时增删.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventKind {
    /// **Get**: 凭证被读取 (`get_token`).
    Get,
    /// **Put**: 凭证被存储 / 更新.
    Put,
    /// **Rotate**: 凭证轮换 (`rotate`).
    Rotate,
    /// **Revoke**: 凭证撤销 (`revoke`).
    Revoke,
}

impl AuditEventKind {
    /// 事件字符串 (snake_case).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Put => "put",
            Self::Rotate => "rotate",
            Self::Revoke => "revoke",
        }
    }

    /// 4 事件名 (编译期 hardcode).
    pub const ALL_NAMES: [&'static str; 4] = ["get", "put", "rotate", "revoke"];

    /// 4 事件 (K-1 强校验: 编译期 hardcode).
    pub const COUNT: usize = 4;
    /// 4 事件 (K-1 强校验: 编译期 hardcode).
    #[must_use]
    pub fn all() -> [AuditEventKind; 4] {
        [Self::Get, Self::Put, Self::Rotate, Self::Revoke]
    }
}

impl std::fmt::Display for AuditEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 AuditEvent 结构 (无敏感字段, 仅元数据)
// ============================================================================

/// 审计事件 (1 条).
///
/// 0 暴露凭证: 不含 `api_key` / `token` / `secret` 字段, 只含元数据.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    /// 事件唯一 ID (UUID v4, 防重放).
    pub event_id: Uuid,
    /// 事件类型.
    pub kind: AuditEventKind,
    /// 事件时间戳 (UTC, ISO 8601).
    pub timestamp: DateTime<Utc>,
    /// Provider 类型.
    pub provider: ProviderKind,
    /// 操作者 (e.g. user_id / service_name), 用于审计追踪.
    pub requester: String,
    /// 事件消息 (人类可读, 不含凭证值).
    pub message: String,
    /// 旧过期时间 (Rotate 事件用).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_expires_at: Option<DateTime<Utc>>,
    /// 新过期时间 (Rotate 事件用).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_expires_at: Option<DateTime<Utc>>,
    /// 撤销原因 (Revoke 事件用).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    /// 轮换策略 (Rotate 事件用, 描述字符串).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_strategy: Option<String>,
}

impl AuditEvent {
    /// 构造新事件 (自动生成 event_id + timestamp).
    #[must_use]
    pub fn new(kind: AuditEventKind, provider: ProviderKind, requester: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            kind,
            timestamp: Utc::now(),
            provider,
            requester: requester.into(),
            message: message.into(),
            old_expires_at: None,
            new_expires_at: None,
            revoke_reason: None,
            rotation_strategy: None,
        }
    }

    /// 构造 Get 事件.
    #[must_use]
    pub fn get(provider: ProviderKind, requester: impl Into<String>) -> Self {
        Self::new(
            AuditEventKind::Get,
            provider,
            requester,
            "credential token retrieved",
        )
    }

    /// 构造 Put 事件.
    #[must_use]
    pub fn put(provider: ProviderKind, requester: impl Into<String>, source: impl Into<String>) -> Self {
        Self::new(
            AuditEventKind::Put,
            provider,
            requester,
            format!("credential stored from source={}", source.into()),
        )
    }

    /// 构造 Rotate 事件.
    #[must_use]
    pub fn rotate(
        provider: ProviderKind,
        requester: impl Into<String>,
        strategy: RotationStrategy,
        old_expires: Option<DateTime<Utc>>,
        new_expires: Option<DateTime<Utc>>,
    ) -> Self {
        let mut ev = Self::new(
            AuditEventKind::Rotate,
            provider,
            requester,
            format!("credential rotated via {}", strategy.describe()),
        );
        ev.old_expires_at = old_expires;
        ev.new_expires_at = new_expires;
        ev.rotation_strategy = Some(strategy.describe());
        ev
    }

    /// 构造 Revoke 事件.
    #[must_use]
    pub fn revoke(provider: ProviderKind, requester: impl Into<String>, reason: impl Into<String>) -> Self {
        let reason_str = reason.into();
        let mut ev = Self::new(
            AuditEventKind::Revoke,
            provider,
            requester,
            format!("credential revoked, reason={reason_str}"),
        );
        ev.revoke_reason = Some(reason_str);
        ev
    }
}

// ============================================================================
// §3 AuditLog (内存 ring buffer, 线程安全)
// ============================================================================

/// 默认 ring buffer 大小 (1000 事件, 满后覆盖最早).
pub const DEFAULT_AUDIT_LOG_CAPACITY: usize = 1000;

/// 审计日志 (内存 ring buffer, 线程安全).
///
/// R21+ 真接商业版时, 此 struct 可加 `flush_to_remote` 异步上传到
/// CloudWatch / Stackdriver / Azure Monitor 等 SIEM 系统.
pub struct AuditLog {
    /// 内部 ring buffer.
    events: Arc<Mutex<VecDeque<AuditEvent>>>,
    /// 最大容量.
    capacity: usize,
}

impl AuditLog {
    /// 构造 (默认容量 1000).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_AUDIT_LOG_CAPACITY)
    }

    /// 构造指定容量.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    /// **核心**: 记录 1 个事件.
    ///
    /// 当前 skeleton 阶段: 写入内存 ring buffer + `info!` 日志.
    /// R21+ 真接: 加远程 SIEM 推送 (CloudWatch / Stackdriver).
    pub async fn record(&self, event: AuditEvent) -> CredentialsResult<()> {
        // 0 暴露凭证值: 验证 event.message 不含敏感字段 (P0 安全铁律)
        if event.message.contains("sk-") || event.message.contains("token=") {
            warn!(
                event_id = %event.event_id,
                kind = %event.kind,
                "audit: P0 安全铁律违反 — message 包含疑似凭证, 已拒绝记录"
            );
            return Err(CredentialsError::AuditLogFailed {
                event: event.kind.as_str().to_string(),
                reason: "P0 安全铁律: message must not contain credential values".to_string(),
            });
        }
        info!(
            event_id = %event.event_id,
            kind = %event.kind,
            provider = %event.provider,
            requester = %event.requester,
            "audit: event recorded"
        );
        let mut events = self.events.lock().await;
        if events.len() >= self.capacity {
            events.pop_front(); // 覆盖最早
        }
        events.push_back(event);
        Ok(())
    }

    /// 列出所有事件 (按时间顺序).
    pub async fn list(&self) -> Vec<AuditEvent> {
        self.events.lock().await.iter().cloned().collect()
    }

    /// 列出指定 Provider 的事件.
    pub async fn list_by_provider(&self, provider: ProviderKind) -> Vec<AuditEvent> {
        self.events
            .lock()
            .await
            .iter()
            .filter(|e| e.provider == provider)
            .cloned()
            .collect()
    }

    /// 列出指定类型的事件.
    pub async fn list_by_kind(&self, kind: AuditEventKind) -> Vec<AuditEvent> {
        self.events
            .lock()
            .await
            .iter()
            .filter(|e| e.kind == kind)
            .cloned()
            .collect()
    }

    /// 当前事件数.
    pub async fn len(&self) -> usize {
        self.events.lock().await.len()
    }

    /// 是否为空.
    pub async fn is_empty(&self) -> bool {
        self.events.lock().await.is_empty()
    }

    /// 清空.
    pub async fn clear(&self) {
        self.events.lock().await.clear();
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("capacity", &self.capacity)
            .finish()
    }
}

// ============================================================================
// §4 编译期守门 (4 事件对齐)
// ============================================================================

/// 4 事件 (K-1 强校验: 编译期 hardcode).
pub const AUDIT_EVENT_KINDS: &[AuditEventKind] = &[
    AuditEventKind::Get,
    AuditEventKind::Put,
    AuditEventKind::Rotate,
    AuditEventKind::Revoke,
];

const _: () = assert!(AUDIT_EVENT_KINDS.len() == AuditEventKind::COUNT);
const _: () = assert!(AuditEventKind::ALL_NAMES.len() == AuditEventKind::COUNT);

// ============================================================================
// §5 单元测试 (4 事件 fixture + P0 安全铁律)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_4_event_kinds() {
        // 4 事件全部存在
        assert_eq!(AuditEventKind::all().len(), 4);
        assert!(AuditEventKind::all().contains(&AuditEventKind::Get));
        assert!(AuditEventKind::all().contains(&AuditEventKind::Put));
        assert!(AuditEventKind::all().contains(&AuditEventKind::Rotate));
        assert!(AuditEventKind::all().contains(&AuditEventKind::Revoke));
    }

    #[tokio::test]
    async fn test_audit_get_event() {
        let log = AuditLog::new();
        let ev = AuditEvent::get(ProviderKind::Anthropic, "user-001");
        log.record(ev).await.expect("record get");
        let events = log.list().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, AuditEventKind::Get);
        assert_eq!(events[0].provider, ProviderKind::Anthropic);
        assert_eq!(events[0].requester, "user-001");
    }

    #[tokio::test]
    async fn test_audit_put_event() {
        let log = AuditLog::new();
        let ev = AuditEvent::put(ProviderKind::OpenAI, "user-002", "keyring");
        log.record(ev).await.expect("record put");
        let events = log.list_by_kind(AuditEventKind::Put).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider, ProviderKind::OpenAI);
        assert!(events[0].message.contains("keyring"));
    }

    #[tokio::test]
    async fn test_audit_rotate_event() {
        let log = AuditLog::new();
        let old_expires = Utc::now() - chrono::Duration::days(1);
        let new_expires = Utc::now() + chrono::Duration::days(30);
        let ev = AuditEvent::rotate(
            ProviderKind::Google,
            "service-001",
            crate::rotation::time_default(),
            Some(old_expires),
            Some(new_expires),
        );
        log.record(ev).await.expect("record rotate");
        let events = log.list_by_kind(AuditEventKind::Rotate).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].old_expires_at, Some(old_expires));
        assert_eq!(events[0].new_expires_at, Some(new_expires));
        assert!(events[0].rotation_strategy.is_some());
    }

    #[tokio::test]
    async fn test_audit_revoke_event() {
        let log = AuditLog::new();
        let ev = AuditEvent::revoke(
            ProviderKind::Azure,
            "admin-001",
            "key compromise suspected",
        );
        log.record(ev).await.expect("record revoke");
        let events = log.list_by_kind(AuditEventKind::Revoke).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].revoke_reason.as_deref(), Some("key compromise suspected"));
    }

    #[tokio::test]
    async fn test_audit_p0_redaction() {
        // P0 安全铁律: 事件 message 不允许含 "sk-" 或 "token=" (疑似凭证)
        let log = AuditLog::new();
        let ev = AuditEvent::new(
            AuditEventKind::Get,
            ProviderKind::Anthropic,
            "user-001",
            "leaked: token=sk-12345",
        );
        let err = log.record(ev).await.unwrap_err();
        assert!(matches!(err, CredentialsError::AuditLogFailed { .. }));
    }

    #[tokio::test]
    async fn test_audit_list_by_provider() {
        let log = AuditLog::new();
        log.record(AuditEvent::get(ProviderKind::Anthropic, "u1")).await.unwrap();
        log.record(AuditEvent::get(ProviderKind::OpenAI, "u2")).await.unwrap();
        log.record(AuditEvent::get(ProviderKind::Anthropic, "u3")).await.unwrap();
        let anthropic_events = log.list_by_provider(ProviderKind::Anthropic).await;
        assert_eq!(anthropic_events.len(), 2);
        let openai_events = log.list_by_provider(ProviderKind::OpenAI).await;
        assert_eq!(openai_events.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_capacity_overflow() {
        // ring buffer 满后覆盖最早
        let log = AuditLog::with_capacity(3);
        for i in 0..5 {
            log.record(AuditEvent::get(ProviderKind::Local, format!("u{i}"))).await.unwrap();
        }
        assert_eq!(log.len().await, 3, "capacity must be respected");
        let events = log.list().await;
        // 最后 3 个事件保留 (u2 / u3 / u4)
        assert_eq!(events[0].requester, "u2");
        assert_eq!(events[1].requester, "u3");
        assert_eq!(events[2].requester, "u4");
    }

    #[tokio::test]
    async fn test_audit_clear() {
        let log = AuditLog::new();
        log.record(AuditEvent::get(ProviderKind::Anthropic, "u1")).await.unwrap();
        assert!(!log.is_empty().await);
        log.clear().await;
        assert!(log.is_empty().await);
    }

    #[test]
    fn test_audit_event_id_unique() {
        // 每个事件 event_id 必须不同
        let ev1 = AuditEvent::get(ProviderKind::Anthropic, "u1");
        let ev2 = AuditEvent::get(ProviderKind::Anthropic, "u1");
        assert_ne!(ev1.event_id, ev2.event_id);
    }

    #[test]
    fn test_audit_event_serde_roundtrip() {
        let ev = AuditEvent::rotate(
            ProviderKind::Google,
            "service-001",
            crate::rotation::hybrid_default(),
            Some(Utc::now() - chrono::Duration::days(1)),
            Some(Utc::now() + chrono::Duration::days(30)),
        );
        let json = serde_json::to_string(&ev).expect("serialize");
        let parsed: AuditEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, ev);
    }
}
