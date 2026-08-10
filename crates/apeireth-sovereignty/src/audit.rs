//! Sovereignty 审计日志 — 4 事件类型 + 5 鉴权级别 + 3 K-1 强校验 (R20 阶段 6 估补)
//!
//! **职责** (本模块 flesh out 估补, lib.rs LOCKED 不重 export, integration test 通过 `#[path]` 引入):
//! - **4 事件类型**: `Access` (读取) / `Modify` (修改) / `Delete` (删除) / `Export` (导出)
//! - **5 鉴权级别**: `Read` / `Write` / `Admin` / `Owner` / `Root`
//!   (按降权顺序: Read < Write < Admin < Owner < Root)
//! - **3 K-1 强校验** (任何 record 必须满足, 否则 `Err(AuditError::K1Violation)`):
//!   1. **K-1.a** — `actor` 非空 (谁做的)
//!   2. **K-1.b** — `resource` 非空 (动了什么)
//!   3. **K-1.c** — `level >= event.required_min_level` (级别匹配事件严重度)
//!
//! **6 哲学锚穿透** (R20 阶段 6 §"6 哲学锚 + 8 项不修改承诺"):
//! - **主 22:33 ASI 北极星** — 审计可追溯 → 任何治理动作事后可还原
//! - **主 17:43 实事求是** — 5 鉴权级别是真实权限层级映射, 非装饰
//! - **主 17:58 不假装** — `try_record` 返回 `Err` 表达真实失败, 不 silent pass
//! - **主 19:33 走在前人肩上** — 复用 `chrono::Utc::now()` 已有依赖, 不引 time / time-macros
//! - **主 23:44 干到底** — 3 K-1 强校验在 `try_record` 一处集中执行, 不分散到调用方
//! - **主 00:56 任何人都能接手** — 公开 API 简单直白, 错误用 `thiserror`, 文档化每个类型
//!
//! **8 项不修改承诺**:
//! - ✅ 编译期 hardcode: 事件数 = 4, 鉴权级别数 = 5, K-1 强校验数 = 3
//! - ✅ 0 触碰 LOCKED: 本文件独立, 不动 lib.rs / Cargo.toml / 24 LOCKED crate
//! - ✅ 0 依赖 NewAPI
//! - ✅ 0 重复造轮子: 复用 `serde::Serialize` + `thiserror::Error` + `chrono::Utc::now()`
//! - ✅ 诚实标缺: ❌ 不持久化 (无 Redis / SQLite), 仅 in-memory; 真生产应接 outbox
//!
//! **诚实登记**:
//! - ❌ **不持久化** — 仅 in-memory; 真生产应通过 outbox 模式接 Redis / Kafka / ClickHouse
//! - ❌ **不假装有签名** — 数字签名见 `signature.rs` (本 crate)
//! - ❌ **不假装抗篡改** — 真抗篡改需 Merkle 树 + 外部 timestamp authority
//!
//! **用法** (integration test 模式, 不走 lib re-export):
//! ```ignore
//! // #[path = "../src/audit.rs"]
//! // mod audit;
//! use audit::{AuditEvent, AuditLevel, AuditLog, EventKind};
//!
//! let mut log = AuditLog::new();
//! let event = AuditEvent::new(
//!     EventKind::Modify,
//!     "alice",
//!     "principle_onion.E_layer",
//!     AuditLevel::Owner,
//!     "modify E principle",
//! );
//! log.try_record(event).expect("K-1 强校验通过");
//! assert_eq!(log.len(), 1);
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 编译时 hardcode: 4 事件 / 5 鉴权级 / 3 K-1 强校验
// ============================================================

/// 事件类型数 (编译时硬编码: Access / Modify / Delete / Export = 4)
pub const EVENT_KIND_COUNT_HARDCODE: usize = 4;

/// 鉴权级别数 (编译时硬编码: Read / Write / Admin / Owner / Root = 5)
pub const AUDIT_LEVEL_COUNT_HARDCODE: usize = 5;

/// K-1 强校验数 (编译时硬编码: actor 非空 / resource 非空 / level 匹配 = 3)
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

/// 4 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// 读取 — 任何读操作 (审计查询 / 状态查询)
    Access,
    /// 修改 — 任何写操作 (配置变更 / 状态变更)
    Modify,
    /// 删除 — 任何删除操作 (资源销毁 / 记录删除)
    Delete,
    /// 导出 — 任何数据外带 (导出报告 / 跨边界)
    Export,
}

impl EventKind {
    /// 该事件要求的最低鉴权级别 (K-1.c 强校验的依据)
    ///
    /// 严重度映射 (按降权顺序 Read < Write < Admin < Owner < Root):
    /// - `Access` → Read (只读最低)
    /// - `Modify` → Write
    /// - `Delete` → Admin (删除是高破坏性)
    /// - `Export` → Owner (跨边界数据外带最敏感)
    pub fn required_min_level(self) -> AuditLevel {
        match self {
            EventKind::Access => AuditLevel::Read,
            EventKind::Modify => AuditLevel::Write,
            EventKind::Delete => AuditLevel::Admin,
            EventKind::Export => AuditLevel::Owner,
        }
    }

    /// 字符串 ID (用于序列化 / 审计查询)
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Access => "access",
            EventKind::Modify => "modify",
            EventKind::Delete => "delete",
            EventKind::Export => "export",
        }
    }
}

/// 5 鉴权级别 (按降权顺序: Root > Owner > Admin > Write > Read)
///
/// 数值越大权限越高 — `PartialOrd` 直接比较数值即可得到权限高低。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditLevel {
    /// 只读 — 看
    Read = 1,
    /// 写 — 改
    Write = 2,
    /// 管理员 — 删
    Admin = 3,
    /// 主人 — 跨边界导出
    Owner = 4,
    /// Root — 紧急 / 不可逆 (编译时 hardcode 最高)
    Root = 5,
}

impl AuditLevel {
    /// 字符串 ID
    pub fn as_str(self) -> &'static str {
        match self {
            AuditLevel::Read => "read",
            AuditLevel::Write => "write",
            AuditLevel::Admin => "admin",
            AuditLevel::Owner => "owner",
            AuditLevel::Root => "root",
        }
    }
}

/// 审计错误
#[derive(Debug, Error, PartialEq)]
pub enum AuditError {
    /// K-1.a 强校验失败 — actor 非空
    #[error("K-1.a 强校验失败: actor 字段为空 (审计必须记录谁做的)")]
    K1ActorEmpty,
    /// K-1.b 强校验失败 — resource 非空
    #[error("K-1.b 强校验失败: resource 字段为空 (审计必须记录动了什么)")]
    K1ResourceEmpty,
    /// K-1.c 强校验失败 — level 不满足事件要求
    #[error(
        "K-1.c 强校验失败: 事件 {event:?} 要求至少 {required:?} 级, 实际为 {actual:?}"
    )]
    K1LevelInsufficient {
        /// 事件类型
        event: EventKind,
        /// 要求最低级
        required: AuditLevel,
        /// 实际提供
        actual: AuditLevel,
    },
}

/// 单条审计事件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件 ID (UUID 风格, 调用方提供; 留空则自动生成)
    pub id: String,
    /// 事件类型
    pub kind: EventKind,
    /// 执行者 (人类 ID / AI ID / 服务 ID) — K-1.a
    pub actor: String,
    /// 涉及资源 (路径 / 对象 / 字段名) — K-1.b
    pub resource: String,
    /// 执行者鉴权级别 — K-1.c
    pub level: AuditLevel,
    /// 人类可读理由
    pub reason: String,
    /// 时间戳 (epoch ms)
    pub timestamp_ms: i64,
}

impl AuditEvent {
    /// 构造审计事件 (timestamp 自动用 `chrono::Utc::now().timestamp_millis()`)
    pub fn new(
        kind: EventKind,
        actor: impl Into<String>,
        resource: impl Into<String>,
        level: AuditLevel,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("audit-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            kind,
            actor: actor.into(),
            resource: resource.into(),
            level,
            reason: reason.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 自定义 ID (供测试或重放场景)
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// 3 K-1 强校验 (集中执行, 任何一条失败 → Err)
    ///
    /// - **K-1.a**: actor 非空
    /// - **K-1.b**: resource 非空
    /// - **K-1.c**: level >= event.required_min_level
    pub fn validate_k1(&self) -> Result<(), AuditError> {
        if self.actor.trim().is_empty() {
            return Err(AuditError::K1ActorEmpty);
        }
        if self.resource.trim().is_empty() {
            return Err(AuditError::K1ResourceEmpty);
        }
        let required = self.kind.required_min_level();
        if self.level < required {
            return Err(AuditError::K1LevelInsufficient {
                event: self.kind,
                required,
                actual: self.level,
            });
        }
        Ok(())
    }
}

/// 审计日志 (in-memory)
#[derive(Debug, Clone, Default)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    /// 新建空日志
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录事件 (先 K-1 强校验, 通过则 push, 失败返回 Err 不 push)
    pub fn try_record(&mut self, event: AuditEvent) -> Result<(), AuditError> {
        event.validate_k1()?;
        self.events.push(event);
        Ok(())
    }

    /// 便捷构造 + 记录 (K-1 失败返回 Err)
    pub fn record(
        &mut self,
        kind: EventKind,
        actor: impl Into<String>,
        resource: impl Into<String>,
        level: AuditLevel,
        reason: impl Into<String>,
    ) -> Result<(), AuditError> {
        let event = AuditEvent::new(kind, actor, resource, level, reason);
        self.try_record(event)
    }

    /// 当前事件数
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 按 actor 过滤
    pub fn filter_by_actor(&self, actor: &str) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.actor == actor).collect()
    }

    /// 按 kind 过滤
    pub fn filter_by_kind(&self, kind: EventKind) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.kind == kind).collect()
    }

    /// 全列表引用
    pub fn all(&self) -> &[AuditEvent] {
        &self.events
    }

    /// 清空 (仅测试用, 真生产不应清空审计)
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

const _: () = {
    assert!(EVENT_KIND_COUNT_HARDCODE == 4);
    assert!(AUDIT_LEVEL_COUNT_HARDCODE == 5);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_count_is_4() {
        assert_eq!(EVENT_KIND_COUNT_HARDCODE, 4);
        // 4 事件: Access / Modify / Delete / Export
        assert_eq!(EventKind::Access.as_str(), "access");
        assert_eq!(EventKind::Modify.as_str(), "modify");
        assert_eq!(EventKind::Delete.as_str(), "delete");
        assert_eq!(EventKind::Export.as_str(), "export");
    }

    #[test]
    fn audit_level_count_is_5_and_ordering() {
        assert_eq!(AUDIT_LEVEL_COUNT_HARDCODE, 5);
        // 5 鉴权级, 降权顺序 Root > Owner > Admin > Write > Read
        assert!(AuditLevel::Root > AuditLevel::Owner);
        assert!(AuditLevel::Owner > AuditLevel::Admin);
        assert!(AuditLevel::Admin > AuditLevel::Write);
        assert!(AuditLevel::Write > AuditLevel::Read);
    }

    #[test]
    fn k1_strict_checks_three_failures() {
        // K-1.a: actor 空
        let e1 = AuditEvent::new(EventKind::Access, "  ", "r", AuditLevel::Read, "x");
        assert_eq!(e1.validate_k1(), Err(AuditError::K1ActorEmpty));

        // K-1.b: resource 空
        let e2 = AuditEvent::new(EventKind::Access, "alice", "", AuditLevel::Read, "x");
        assert_eq!(e2.validate_k1(), Err(AuditError::K1ResourceEmpty));

        // K-1.c: level 不足 (Delete 要求 Admin, 给 Read)
        let e3 = AuditEvent::new(EventKind::Delete, "alice", "r", AuditLevel::Read, "x");
        assert_eq!(
            e3.validate_k1(),
            Err(AuditError::K1LevelInsufficient {
                event: EventKind::Delete,
                required: AuditLevel::Admin,
                actual: AuditLevel::Read,
            })
        );
    }

    #[test]
    fn audit_log_try_record_passes_and_rejects() {
        let mut log = AuditLog::new();
        assert!(log.is_empty());

        // 通过 K-1
        log.record(EventKind::Access, "alice", "principle_onion", AuditLevel::Owner, "audit")
            .unwrap();
        assert_eq!(log.len(), 1);

        // level 不足 (Modify 要求 Write, 给 Read)
        let res = log.record(EventKind::Modify, "alice", "principle_onion", AuditLevel::Read, "x");
        assert!(res.is_err());
        assert_eq!(log.len(), 1); // 未 push

        // 过滤
        assert_eq!(log.filter_by_actor("alice").len(), 1);
        assert_eq!(log.filter_by_kind(EventKind::Access).len(), 1);
        assert_eq!(log.filter_by_kind(EventKind::Delete).len(), 0);
    }
}
