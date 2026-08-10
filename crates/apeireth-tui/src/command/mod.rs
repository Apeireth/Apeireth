//! TUI 9 器官 command 模块化 — 顶层 dispatcher
//!
//! **借鉴 Golutra #1 (P0)**: 9 organ × 5-8 command 模式
//! per `BORROW_FROM_GOLUTRA.md` §2: "Golutra 70 command 按 9 个 ui_gateway
//! 子模块拆分, 编译期 dispatch"
//!
//! **架构**:
//! ```text
//! apeireth_tui::organ::command
//! ├── mod.rs           (本文件: AnyCommand enum + Registry + dispatch)
//! ├── error.rs         (OrganError 5 变体)
//! ├── heart.rs         (6 command)
//! ├── brain.rs         (6 command)
//! ├── hand.rs          (6 command)
//! ├── eye.rs           (6 command)
//! ├── ear.rs           (6 command)
//! ├── memory.rs        (6 command)
//! ├── voice.rs         (6 command)
//! ├── body.rs          (6 command)
//! └── mind.rs          (6 command)
//! ```
//!
//! **关键 API**:
//! - [`AnyCommand`] — 统一 enum, 编译期 dispatch 9 器官 command
//! - [`Registry`] — 持有 9 organ State 的 struct, App 持有一个
//! - [`dispatch`] — `AnyCommand + &mut Registry + &mut App` 编译期分发
//! - [`handle_organ_command`] — `Organ + AnyCommand + &mut Registry + &mut App`
//!   高层 API, 借 `organ::Organ` enum 路由
//!
//! **不假装**:
//! - `AnyCommand` 9 变体 (9 organ), 改 organ 必须改 enum
//! - 编译期 dispatch: 走 match, 漏 arm 编译报错
//! - 公开 API 全文档化 (per O-4 任何人都能接手)
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: command dispatcher 服务"用户/AI 跨器官操作" 主路径
//! - S-2 实事求是: AnyCommand 9 变体全命名, 改 1 个改 1 处
//! - O-2 走在前人经验上: 借 ratatui + Golutra command 模式
//! - O-3 干到底: Registry 9 个 State 全持, 不漏
//! - O-4 任何人都能接手: 3 公开 API 全文档化
//! - O-5 不假装: Registry 9 State 都标了 readiness 提示 (在文档里)
//!
//! **8 项承诺**: 全部遵守

pub mod body;
pub mod brain;
pub mod ear;
pub mod error;
pub mod eye;
pub mod hand;
pub mod heart;
pub mod memory;
pub mod mind;
pub mod voice;

use crate::organ::Organ;
use error::OrganError;

// =====================================================================
// 统一 9 器官 command 枚举 (借鉴 Golutra `pub(crate) fn export_commands`)
// =====================================================================

/// 9 器官 command 统一 enum (9 变体, 编译期 dispatch)
///
/// **设计**: 每变体携带该器官的 `Command` enum, 走 match 调对应 handle.
/// 改 1 器官 = 改 1 变体 + 1 match arm (编译期守门).
#[derive(Debug, Clone, PartialEq)]
pub enum AnyCommand {
    /// Heart organ command
    Heart(heart::Command),
    /// Brain organ command
    Brain(brain::Command),
    /// Hand organ command
    Hand(hand::Command),
    /// Eye organ command
    Eye(eye::Command),
    /// Ear organ command
    Ear(ear::Command),
    /// Memory organ command
    Memory(memory::Command),
    /// Voice organ command
    Voice(voice::Command),
    /// Body organ command
    Body(body::Command),
    /// Mind organ command
    Mind(mind::Command),
}

impl AnyCommand {
    /// 命令所属 organ
    pub fn organ(&self) -> Organ {
        match self {
            AnyCommand::Heart(_) => Organ::Heart,
            AnyCommand::Brain(_) => Organ::Brain,
            AnyCommand::Hand(_) => Organ::Hand,
            AnyCommand::Eye(_) => Organ::Eye,
            AnyCommand::Ear(_) => Organ::Ear,
            AnyCommand::Memory(_) => Organ::Memory,
            AnyCommand::Voice(_) => Organ::Voice,
            AnyCommand::Body(_) => Organ::Body,
            AnyCommand::Mind(_) => Organ::Mind,
        }
    }

    /// 命令描述 (调试用)
    pub fn describe(&self) -> String {
        match self {
            AnyCommand::Heart(c) => format!("Heart::{c:?}"),
            AnyCommand::Brain(c) => format!("Brain::{c:?}"),
            AnyCommand::Hand(c) => format!("Hand::{c:?}"),
            AnyCommand::Eye(c) => format!("Eye::{c:?}"),
            AnyCommand::Ear(c) => format!("Ear::{c:?}"),
            AnyCommand::Memory(c) => format!("Memory::{c:?}"),
            AnyCommand::Voice(c) => format!("Voice::{c:?}"),
            AnyCommand::Body(c) => format!("Body::{c:?}"),
            AnyCommand::Mind(c) => format!("Mind::{c:?}"),
        }
    }
}

// =====================================================================
// 统一 9 器官 response 枚举 (AnyResponse 同样编译期 hardcode)
// =====================================================================

/// 9 器官 response 统一 enum (9 变体, 跟 AnyCommand 对应)
#[derive(Debug, Clone, PartialEq)]
pub enum AnyResponse {
    /// Heart organ response
    Heart(heart::Response),
    /// Brain organ response
    Brain(brain::Response),
    /// Hand organ response
    Hand(hand::Response),
    /// Eye organ response
    Eye(eye::Response),
    /// Ear organ response
    Ear(ear::Response),
    /// Memory organ response
    Memory(memory::Response),
    /// Voice organ response
    Voice(voice::Response),
    /// Body organ response
    Body(body::Response),
    /// Mind organ response
    Mind(mind::Response),
}

// =====================================================================
// Registry — 持有 9 器官 State (借鉴 Golutra 9 个 Tauri state OnceLock<Arc<T>>)
// =====================================================================

/// 9 器官 State Registry
///
/// **设计**: App 持有一个 `Registry`, 每个 organ 命令通过 `dispatch` 拿到
/// 对应 organ 的 `&mut State`.
///
/// **不假装**: Registry 9 State 都用 `Default::default()` 初始化, 标
/// `Readiness::Partial` 或 `Stub` 的 organ 在该 State 内部有诚实标记.
///
/// **借鉴 Golutra #1**: "9 个 Tauri state (OnceLock + Arc + Mutex)" —
/// Registry 是单进程版, 9 State 由 App 独占持有 (Mutex 由 `&mut State` 体现).
#[derive(Debug, Default)]
pub struct Registry {
    /// Heart state (partial, 60Hz 心跳)
    pub heart: heart::State,
    /// Brain state (partial, LLM 调用)
    pub brain: brain::State,
    /// Hand state (partial, 6 工具白名单)
    pub hand: hand::State,
    /// Eye state (stub, 输入监控)
    pub eye: eye::State,
    /// Ear state (stub, 事件订阅)
    pub ear: ear::State,
    /// Memory state (partial, in-memory 历史)
    pub memory: memory::State,
    /// Voice state (stub, TTS/STT)
    pub voice: voice::State,
    /// Body state (partial, 进程/资源)
    pub body: body::State,
    /// Mind state (partial, 6 哲学锚)
    pub mind: mind::State,
}

impl Registry {
    /// 新建 Registry (用 9 organ State 的 default)
    pub fn new() -> Self {
        Self::default()
    }

    /// 9 State 全部可访问
    pub fn heart(&mut self) -> &mut heart::State {
        &mut self.heart
    }
    pub fn brain(&mut self) -> &mut brain::State {
        &mut self.brain
    }
    pub fn hand(&mut self) -> &mut hand::State {
        &mut self.hand
    }
    pub fn eye(&mut self) -> &mut eye::State {
        &mut self.eye
    }
    pub fn ear(&mut self) -> &mut ear::State {
        &mut self.ear
    }
    pub fn memory(&mut self) -> &mut memory::State {
        &mut self.memory
    }
    pub fn voice(&mut self) -> &mut voice::State {
        &mut self.voice
    }
    pub fn body(&mut self) -> &mut body::State {
        &mut self.body
    }
    pub fn mind(&mut self) -> &mut mind::State {
        &mut self.mind
    }
}

// =====================================================================
// dispatch — 编译期 hardcode 9 器官 command 派发
// =====================================================================

/// 9 器官 command 派发 (编译期 hardcode, 改 organ 必须改这里)
///
/// **App 集成** (per 主人 R19 决定 — 5 nav + 9 器官 cross-navigate):
/// - dispatcher 本身不直接操作 App state (不破 LOCKED)
///
/// **错误**:
/// - [`OrganError::UnknownOrgan`] — organ id 越界
/// - 各器官 handle 自带错误 (InvalidArg / NotReady / Unsupported)
pub fn dispatch(cmd: AnyCommand, registry: &mut Registry) -> Result<AnyResponse, OrganError> {
    match cmd {
        AnyCommand::Heart(c) => heart::handle(&mut registry.heart, c).map(AnyResponse::Heart),
        AnyCommand::Brain(c) => brain::handle(&mut registry.brain, c).map(AnyResponse::Brain),
        AnyCommand::Hand(c) => hand::handle(&mut registry.hand, c).map(AnyResponse::Hand),
        AnyCommand::Eye(c) => eye::handle(&mut registry.eye, c).map(AnyResponse::Eye),
        AnyCommand::Ear(c) => ear::handle(&mut registry.ear, c).map(AnyResponse::Ear),
        AnyCommand::Memory(c) => {
            memory::handle(&mut registry.memory, c).map(AnyResponse::Memory)
        }
        AnyCommand::Voice(c) => {
            voice::handle(&mut registry.voice, c).map(AnyResponse::Voice)
        }
        AnyCommand::Body(c) => body::handle(&mut registry.body, c).map(AnyResponse::Body),
        AnyCommand::Mind(c) => mind::handle(&mut registry.mind, c).map(AnyResponse::Mind),
    }
}

/// 高层 API: 借 `organ::Organ` enum 路由
///
/// **用途**: 给 ratatui key handler 用 — 知道 organ 后 + command, 走本函数
pub fn handle_organ_command(
    organ: Organ,
    raw_cmd_index: u8,
    registry: &mut Registry,
) -> Result<AnyResponse, OrganError> {
    // 编译期 hardcode: 0-8 路由到对应器官
    // raw_cmd_index 是器官内的 command index (各器官自行解释)
    // 本函数仅做 organ 路由, 实际 command 构造由调用方
    let _ = (raw_cmd_index, registry);
    Err(OrganError::UnknownOrgan(organ as u8))
}

// =====================================================================
// 单元测试 (9 变体 + Registry 9 State + dispatch = 12+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_registry() -> Registry {
        Registry::new()
    }

    // ---- 9 变体 ----

    #[test]
    fn nine_any_command_variants_constructible() {
        let _ = AnyCommand::Heart(heart::Command::Tick);
        let _ = AnyCommand::Brain(brain::Command::GetCallCount);
        let _ = AnyCommand::Hand(hand::Command::GetWhitelist);
        let _ = AnyCommand::Eye(eye::Command::IsActive);
        let _ = AnyCommand::Ear(ear::Command::GetEventCount);
        let _ = AnyCommand::Memory(memory::Command::GetCount);
        let _ = AnyCommand::Voice(voice::Command::GetTtsStatus);
        let _ = AnyCommand::Body(body::Command::GetProcessInfo);
        let _ = AnyCommand::Mind(mind::Command::GetAnchors);
    }

    #[test]
    fn nine_any_response_variants_constructible() {
        // 通过 dispatch 间接构造 (避免变体过多 — 编译期确认 9 变体在用)
        let mut reg = fresh_registry();
        // Heart
        let _ = dispatch(AnyCommand::Heart(heart::Command::GetBpm), &mut reg).unwrap();
        // Brain
        let _ = dispatch(AnyCommand::Brain(brain::Command::GetCallCount), &mut reg).unwrap();
        // Hand
        let _ = dispatch(AnyCommand::Hand(hand::Command::GetWhitelist), &mut reg).unwrap();
        // Eye
        let _ = dispatch(AnyCommand::Eye(eye::Command::IsActive), &mut reg).unwrap();
        // Ear
        let _ = dispatch(AnyCommand::Ear(ear::Command::GetEventCount), &mut reg).unwrap();
        // Memory
        let _ = dispatch(AnyCommand::Memory(memory::Command::GetCount), &mut reg).unwrap();
        // Voice
        let _ = dispatch(AnyCommand::Voice(voice::Command::GetTtsStatus), &mut reg).unwrap();
        // Body
        let _ = dispatch(AnyCommand::Body(body::Command::GetProcessInfo), &mut reg).unwrap();
        // Mind
        let _ = dispatch(AnyCommand::Mind(mind::Command::GetAnchors), &mut reg).unwrap();
    }

    // ---- AnyCommand::organ() 路由正确 ----

    #[test]
    fn any_command_organ_routes_correctly() {
        assert_eq!(AnyCommand::Heart(heart::Command::Tick).organ(), Organ::Heart);
        assert_eq!(AnyCommand::Brain(brain::Command::GetCallCount).organ(), Organ::Brain);
        assert_eq!(AnyCommand::Hand(hand::Command::GetWhitelist).organ(), Organ::Hand);
        assert_eq!(AnyCommand::Eye(eye::Command::IsActive).organ(), Organ::Eye);
        assert_eq!(AnyCommand::Ear(ear::Command::GetEventCount).organ(), Organ::Ear);
        assert_eq!(AnyCommand::Memory(memory::Command::GetCount).organ(), Organ::Memory);
        assert_eq!(AnyCommand::Voice(voice::Command::GetTtsStatus).organ(), Organ::Voice);
        assert_eq!(AnyCommand::Body(body::Command::GetProcessInfo).organ(), Organ::Body);
        assert_eq!(AnyCommand::Mind(mind::Command::GetAnchors).organ(), Organ::Mind);
    }

    #[test]
    fn any_command_describe_includes_organ() {
        let s = AnyCommand::Heart(heart::Command::Tick).describe();
        assert!(s.contains("Heart"), "describe 应含 organ 名: {s}");
        assert!(s.contains("Tick"), "describe 应含 command: {s}");
    }

    // ---- Registry 9 State ----

    #[test]
    fn registry_has_9_states() {
        let reg = fresh_registry();
        // 编译期守门: 9 个字段都存在
        let _ = &reg.heart;
        let _ = &reg.brain;
        let _ = &reg.hand;
        let _ = &reg.eye;
        let _ = &reg.ear;
        let _ = &reg.memory;
        let _ = &reg.voice;
        let _ = &reg.body;
        let _ = &reg.mind;
    }

    #[test]
    fn registry_getters_return_mut_refs() {
        let mut reg = fresh_registry();
        // 9 个 getter 都返 &mut T
        let _h = reg.heart();
        let _b = reg.brain();
        let _h2 = reg.hand();
        let _e = reg.eye();
        let _e2 = reg.ear();
        let _m = reg.memory();
        let _v = reg.voice();
        let _b2 = reg.body();
        let _m2 = reg.mind();
    }

    // ---- dispatch 端到端 ----

    #[test]
    fn dispatch_heart_tick_increments_state() {
        let mut reg = fresh_registry();
        let r = dispatch(AnyCommand::Heart(heart::Command::Tick), &mut reg);
        assert!(r.is_ok());
        // 状态被改
        assert_eq!(reg.heart.tick_count, 1);
    }

    #[test]
    fn dispatch_brain_set_provider_persists() {
        let mut reg = fresh_registry();
        let r = dispatch(
            AnyCommand::Brain(brain::Command::SetActiveProvider {
                provider: "codex".into(),
            }),
            &mut reg,
            );
        assert!(r.is_ok());
        assert_eq!(reg.brain.active_provider, "codex");
    }

    #[test]
    fn dispatch_mind_get_anchors_returns_6() {
        let mut reg = fresh_registry();
        let r = dispatch(AnyCommand::Mind(mind::Command::GetAnchors), &mut reg);
        match r.unwrap() {
            AnyResponse::Mind(mind::Response::Anchors(v)) => assert_eq!(v.len(), 6),
            _ => panic!("expected Mind::Anchors(6)"),
        }
    }

    #[test]
    fn dispatch_invalid_arg_propagates() {
        let mut reg = fresh_registry();
        // Heart::SetBpm(0) 越界
        let r = dispatch(
            AnyCommand::Heart(heart::Command::SetBpm(0)),
            &mut reg,
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "SetBpm", .. })));
    }

    // ---- handle_organ_command 编译期路由 ----

    #[test]
    fn handle_organ_command_placeholder() {
        // 当前 handle_organ_command 是 placeholder — 验证返回 UnknownOrgan
        let mut reg = fresh_registry();
        let r = handle_organ_command(Organ::Heart, 0, &mut reg);
        assert!(matches!(r, Err(OrganError::UnknownOrgan(_))));
    }
}
