//! R1b-1a · 机制信号桥（ADR-0009 D2，内部私有）。
//!
//! 这是**唯一**把既有钩子事件翻成 [`NervousSignal`] 的位置：装配根把本模块
//! 放在模块表**首位**，其余机制（R1b-2 起）只订阅总线，不再各自扫描钩子。
//!
//! 映射面严格等于 ADR-0009 D2 冻结的两类（来源已在生产派发环里真实存在）：
//!
//! | 钩子 | 上下文事实 | 信号 |
//! |---|---|---|
//! | `AfterToolResult` | `ModuleContext::tool_result: Some(&ToolResult)` | [`NervousSignal::ToolOutcome`] |
//! | `OnError` | `ModuleContext::error: Some(&str)` 且文本非空白 | [`NervousSignal::ErrorObserved`] |
//! | 其余钩子 | — | `None`（不编造信号） |
//!
//! `ToolOutcome.tool` 的解析与迁移前机制钩子口径一致：结果名优先，缺省回退
//! 调用名（两者都空 = `None`，不编名字）；否则只订阅总线的机制会静默丢掉
//! 「结果未带名字」的归属。
//!
//! 行为边界：
//! - 本模块**永不改变回合走向**：`on_hook` 恒返回 [`ModuleOutcome::continue_`]，
//!   发布失败/零订阅者都不影响调用方；
//! - `OnError` 文本缺失或空白 → 不发信号（不从缺失数据编造通用错误）；
//! - 发布是同步调用：订阅者只在当前线程做轻量推进，不得阻塞、不得再进回合。

use apeireth_protocol::canonical::{ToolCall, ToolResult};

use super::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use super::nervous_bus::{NervousBus, NervousSignal};

/// 桥模块的稳定 id（装配根按 id 去重校验；不得与既有模块重名）。
pub const NERVOUS_BRIDGE_MODULE_ID: &str = "nervous.bridge";

/// 把一次钩子调用映射为总线信号；`None` = 该钩子不是信号源（ADR-0009 D2）。
///
/// `tool_call` 只用于工具名回退：`ToolResult::name` 缺省时取调用名（与迁移前
/// 机制钩子的解析顺序一致）；其余字段只读结果事实，不从调用侧编造结局。
pub fn bridge_signal(
    hook: HookPoint,
    tool_call: Option<&ToolCall>,
    tool_result: Option<&ToolResult>,
    error: Option<&str>,
) -> Option<NervousSignal> {
    match hook {
        HookPoint::AfterToolResult => {
            let result = tool_result?;
            let name = result
                .name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .or_else(|| {
                    tool_call
                        .map(|call| call.name.as_str())
                        .filter(|name| !name.trim().is_empty())
                });
            Some(NervousSignal::tool_outcome(
                result.is_ok(),
                name,
                result.outcome.is_retryable(),
            ))
        }
        HookPoint::OnError => {
            let message = error?;
            if message.trim().is_empty() {
                return None;
            }
            Some(NervousSignal::error_observed(message))
        }
        _ => None,
    }
}

/// HookPoint → [`NervousSignal`] 的唯一桥模块（同步发布，不改变回合走向）。
#[derive(Clone)]
pub struct NervousBridgeModule {
    bus: NervousBus,
    manifest: ModuleManifest,
}

impl NervousBridgeModule {
    /// 构造：`bus` 是装配根持有的同一条总线（`clone` 共享）。
    pub fn new(bus: NervousBus) -> Self {
        Self {
            bus,
            manifest: ModuleManifest::new(NERVOUS_BRIDGE_MODULE_ID, "Nervous Bridge"),
        }
    }

    /// 本模块发布所用总线（装配根取回同一句柄给机制订阅）。
    pub fn bus(&self) -> &NervousBus {
        &self.bus
    }
}

impl std::fmt::Debug for NervousBridgeModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NervousBridgeModule")
            .field("manifest", &self.manifest)
            .field("bus", &self.bus)
            .finish()
    }
}

#[async_trait::async_trait]
impl BehaviorModule for NervousBridgeModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if let Some(signal) = bridge_signal(hook, ctx.tool_call, ctx.tool_result, ctx.error) {
            self.bus.publish(&signal);
        }
        Ok(ModuleOutcome::continue_())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::Value;

    use super::*;
    use crate::canonical::nervous_bus::NervousSignalKind;

    fn ok_result(name: &str) -> ToolResult {
        ToolResult::ok("call_ok", Value::Null).with_name(name)
    }

    fn retryable_error(name: &str) -> ToolResult {
        ToolResult::retryable_error("call_retry", "timeout").with_name(name)
    }

    fn permanent_error(name: &str) -> ToolResult {
        ToolResult::permanent_error("call_perm", "bad argument").with_name(name)
    }

    #[test]
    fn module_manifest_is_stable() {
        let module = NervousBridgeModule::new(NervousBus::new());
        assert_eq!(module.manifest().id, NERVOUS_BRIDGE_MODULE_ID);
        assert_eq!(module.manifest().name, "Nervous Bridge");
    }

    #[test]
    fn after_tool_result_maps_success_facts_only() {
        let result = ok_result("fs.read");
        let signal = bridge_signal(HookPoint::AfterToolResult, None, Some(&result), None)
            .expect("AfterToolResult 必须产生信号");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: true,
                tool: Some("fs.read".to_string()),
                retryable: false,
            }
        );
        assert_eq!(signal.kind(), NervousSignalKind::ToolOutcome);
    }

    #[test]
    fn after_tool_result_maps_retryability_verbatim() {
        let signal = bridge_signal(
            HookPoint::AfterToolResult,
            None,
            Some(&retryable_error("shell.exec")),
            None,
        )
        .expect("失败结果同样是观测事实");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: false,
                tool: Some("shell.exec".to_string()),
                retryable: true,
            }
        );

        let signal = bridge_signal(
            HookPoint::AfterToolResult,
            None,
            Some(&permanent_error("shell.exec")),
            None,
        )
        .expect("永久失败也是观测事实");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: false,
                tool: Some("shell.exec".to_string()),
                retryable: false,
            }
        );
    }

    #[test]
    fn after_tool_result_without_tool_result_is_not_a_signal() {
        assert_eq!(
            bridge_signal(HookPoint::AfterToolResult, None, None, Some("ignored")),
            None
        );
    }

    #[test]
    fn on_error_maps_non_blank_text() {
        let signal = bridge_signal(HookPoint::OnError, None, None, Some("provider timeout"))
            .expect("OnError 有文本必须产生信号");
        assert_eq!(
            signal,
            NervousSignal::ErrorObserved {
                message: "provider timeout".to_string(),
            }
        );
        assert_eq!(signal.kind(), NervousSignalKind::ErrorObserved);
    }

    #[test]
    fn on_error_does_not_fabricate_from_missing_or_blank_text() {
        assert_eq!(bridge_signal(HookPoint::OnError, None, None, None), None);
        assert_eq!(
            bridge_signal(HookPoint::OnError, None, None, Some("")),
            None
        );
        assert_eq!(
            bridge_signal(HookPoint::OnError, None, None, Some("   \n\t")),
            None
        );
    }

    #[test]
    fn tool_name_falls_back_to_the_call_name_verbatim() {
        let call = ToolCall {
            id: "call_fallback".to_string(),
            name: "filesystem".to_string(),
            arguments: Value::Null,
        };
        let mut unnamed = ToolResult::ok("call_fallback", Value::Null);
        unnamed.name = None;
        let signal = bridge_signal(
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&unnamed),
            None,
        )
        .expect("结果无名仍应产生信号");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: true,
                tool: Some("filesystem".to_string()),
                retryable: false,
            }
        );

        // 结果名是空白 = 缺省 → 回退调用名，不把空白当名字。
        let mut blank = ToolResult::ok("call_fallback", Value::Null);
        blank.name = Some("   ".to_string());
        let signal = bridge_signal(HookPoint::AfterToolResult, Some(&call), Some(&blank), None)
            .expect("空白结果名同样产生信号");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: true,
                tool: Some("filesystem".to_string()),
                retryable: false,
            }
        );

        // 两边都缺名 = None（下游如实跳过归属，不编名字）。
        let bare_call = ToolCall {
            id: "call_none".to_string(),
            name: String::new(),
            arguments: Value::Null,
        };
        let bare = ToolResult::ok("call_none", Value::Null);
        let signal = bridge_signal(
            HookPoint::AfterToolResult,
            Some(&bare_call),
            Some(&bare),
            None,
        )
        .expect("仍产生结局信号");
        assert_eq!(
            signal,
            NervousSignal::ToolOutcome {
                ok: true,
                tool: None,
                retryable: false,
            }
        );
    }

    #[test]
    fn other_hooks_are_never_signal_sources() {
        let result = ok_result("fs.read");
        for hook in [
            HookPoint::TurnStart,
            HookPoint::BeforeModelCall,
            HookPoint::AfterModelResponse,
            HookPoint::BeforeToolCall,
            HookPoint::BeforeFinalCommit,
            HookPoint::AfterTurn,
        ] {
            assert_eq!(
                bridge_signal(hook, None, Some(&result), Some("boom")),
                None,
                "{hook:?} 不在 D2 冻结的信号源里"
            );
        }
    }

    #[test]
    fn cloned_bridge_shares_bus() {
        let bus = NervousBus::new();
        let first = NervousBridgeModule::new(bus.clone());
        let second = NervousBridgeModule::new(first.bus().clone());
        assert_eq!(first.bus().publish(&NervousSignal::error_observed("x")), 0);
        assert_eq!(
            second
                .bus()
                .published_count(NervousSignalKind::ErrorObserved),
            1
        );
        assert!(first.bus().is_idle());
    }

    #[test]
    fn synthetic_signal_is_published_synchronously() {
        let bus = NervousBus::new();
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = Arc::clone(&observed);
        bus.subscribe(NervousSignalKind::ToolOutcome, move |signal| {
            sink.lock().expect("锁未中毒").push(signal.clone());
        });
        assert_eq!(
            bus.publish(&NervousSignal::tool_outcome(true, Some("fs.read"), false)),
            1
        );
        let observed = observed.lock().expect("锁未中毒");
        assert_eq!(observed.len(), 1);
        assert_eq!(bus.delivered_total(), 1);
    }
}
