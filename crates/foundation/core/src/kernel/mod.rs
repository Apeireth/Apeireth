//! Canonical primitives — the stable vocabulary every Apeireth subsystem shares.
//!
//! # What belongs here
//!
//! Identifiers, time, lifecycle, events, annotations, and the errors those
//! produce. Nothing else. The test for membership is: *would two unrelated
//! subsystems both need this in order to talk to each other?* If only one
//! subsystem needs it, it belongs to that subsystem.
//!
//! # What must never be added here
//!
//! HTTP, LLM implementations, SQLite, tools, memory engines, gateways, MCP
//! transports, companion cognition, provider implementations. Each of those has
//! an owner elsewhere; see `ARCHITECTURE.md`.
//!
//! # Why this is a submodule rather than the crate root
//!
//! `apeireth-core` presently also carries a large body of historical work —
//! memory items, philosophy gates, permission onions, a cognitive lifecycle —
//! re-exported at the crate root and depended on by 38 crates. That content does
//! not satisfy the rule above, but it cannot be evicted without breaking the
//! workspace, and doing so is tracked as a migration item rather than performed
//! here.
//!
//! Confining the canonical vocabulary to `apeireth_core::kernel` gives new code
//! an unambiguous namespace today, and keeps `pub use kernel::*` off the crate
//! root so that the legacy re-exports cannot silently shadow a primitive. When
//! the legacy content has moved to its rightful owners, this module becomes the
//! crate.
//!
//! # Determinism
//!
//! Nothing here reads the wall clock implicitly. [`Event`] and [`Timestamp`] take
//! a `&dyn Clock`, which is what lets the end-to-end runtime test assert on
//! timing without sleeping.

pub mod error;
pub mod event;
pub mod ids;
pub mod lifecycle;
pub mod memory;
pub mod metadata;
pub mod time;

pub use error::{CoreError, CoreResult};
pub use event::{Event, Topic};
pub use ids::{ApprovalId, CapabilityId, ModelId, PluginId, RequestId, SessionId, TaskId, TraceId};
pub use lifecycle::Lifecycle;
pub use metadata::Metadata;
pub use time::{system_clock, Clock, SystemClock, Timestamp, VirtualClock};
// Domain types (Episode / Note / Session / IdentityCard / Migration) live in
// `crate::kernel::memory` — accessed as `apeireth_core::kernel::memory::Episode`.
// NOT re-exported at kernel root (would conflict with `apeireth_core::Episode`
// legacy compat at root). v2.0.0-rc 阶段: 12 consumer 批量迁 kernel::memory
// 后, lib.rs `pub use memory::*` 删除, kernel 顶层 re-export 打开.

// P-arch (2026-08-27): core drain 第一阶段.
// 真实 drain（删 root 的 `pub use memory::*` 等）需要 12 个 crate 改路径,
// 排期 v2.1。v2.0 alpha 的最小可用做法：让 `apeireth_core::kernel` 路径
// 也能拿到 legacy 域类型 (Episode/Note/Session/IdentityCard) — 与 root 路径
// 指向**同一类型** (alias, 不是新类型), 0 改其他 crate, 0 破 API.
// 新代码: `use apeireth_core::kernel::Episode` 表达 v2 canonical 意图.
// 旧代码: `use apeireth_core::Episode` 继续工作 (root 的 `pub use memory::*`).
// 详细架构: docs/04-internal/v2-unabsorbed-features.md §P1 + ROADMAP §5.
//
// 7 键 + 13 键 验证层类型: 已在 13 键 LOCKED 范畴, 0 改.
pub use crate::gate::Gate as GateTrait;
pub use crate::memory::Episode;
pub use crate::memory::IdentityCard;
pub use crate::memory::Migration as CarrierMigration;
pub use crate::memory::Note;
pub use crate::memory::Session;
pub use crate::onion::{PermissionOnion, PrincipleOnion, PrincipleLayer};
pub use crate::philosophy::{PhilosophyGuard, PhilosophyKey, PhilosophyVerdict, VerdictCache};

#[cfg(test)]
mod tests {
    use super::*;

    /// The primitives have to compose without any subsystem's help; if this stops
    /// being true, something layered has leaked into core.
    #[test]
    fn the_primitives_compose_into_a_correlated_record() {
        let clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );

        let session = SessionId::new();
        let trace = TraceId::new();
        let plugin = PluginId::new("builtin.calculator").unwrap();
        let capability = CapabilityId::new("tool.calculator").unwrap();
        let model = ModelId::new("fake-model-1").unwrap();

        let state = Lifecycle::Registered
            .transition_to(plugin.as_str(), Lifecycle::Initializing)
            .unwrap()
            .transition_to(plugin.as_str(), Lifecycle::Active)
            .unwrap();
        assert!(state.is_dispatchable());

        let event = Event::new(
            Topic::new("runtime.capability.dispatched").unwrap(),
            trace,
            &clock,
            serde_json::json!({
                "session": session.to_string(),
                "capability": capability.as_str(),
                "model": model.as_str(),
            }),
        )
        .with_metadata("plugin", plugin.as_str());

        assert_eq!(event.trace, trace);
        assert_eq!(event.metadata.get("plugin"), Some("builtin.calculator"));
        assert_eq!(capability.kind_segment(), "tool");
    }

    /// P-arch (2026-08-27): core drain 第一阶段验证.
    /// kernel re-export 的 legacy 域类型 = 同一类型 (非新类型).
    /// 0 触碰公开签名; v2.1 drain 时 12 个 consumer 仅改 `use` 行即可.
    #[test]
    fn kernel_legacy_aliases_point_to_the_same_types_as_root() {
        let _e_via_root: crate::memory::Episode = Episode {
            id: "ep-drain".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "core drain test".into(),
            session_id: "sess-drain".into(),
        };
        // 编译通过 = 类型完全一致 (trait object 同一 ID)

        // 13 键哲学层别名也可达
        let _key: PhilosophyKey = PhilosophyKey::NotClone;
        // VerdictCache 别名
        let cache = VerdictCache::new();
        let _via_root_cache: crate::philosophy::VerdictCache = cache;
    }

    /// 13 键 / 守门 / 洋葱 全部通过 kernel 可达.
    #[test]
    fn kernel_principle_and_governance_aliases_resolve() {
        // 13 键编译期断言仍通过
        let count: u8 = crate::ALL_THIRTEEN_KEYS
            .iter()
            .map(|k| k.group_id())
            .filter(|g| *g >= 1 && *g <= 7)
            .count() as u8;
        assert_eq!(count, 13);

        // 原则洋葱通过 kernel 路径
        let _layer: PrincipleLayer = PrincipleLayer {
            name: "E".to_string(),
            description: "existence".to_string(),
            hardcoded: true,
        };
        // 权限洋葱同路径
        let _permission: PermissionOnion = PermissionOnion {
            l0: crate::onion::PermissionLayer {
                name: "L0".to_string(),
                description: "HA".to_string(),
                requires_ha: true,
            },
            l1: crate::onion::PermissionLayer {
                name: "L1".to_string(),
                description: "controlled write".to_string(),
                requires_ha: false,
            },
            l2: crate::onion::PermissionLayer {
                name: "L2".to_string(),
                description: "important ops".to_string(),
                requires_ha: false,
            },
            l3: crate::onion::PermissionLayer {
                name: "L3".to_string(),
                description: "critical ops".to_string(),
                requires_ha: false,
            },
            l4: crate::onion::PermissionLayer {
                name: "L4".to_string(),
                description: "core upgrade".to_string(),
                requires_ha: false,
            },
            l5: crate::onion::PermissionLayer {
                name: "L5".to_string(),
                description: "nuclear".to_string(),
                requires_ha: false,
            },
        };
    }
}
