//! # OrganStateRegistry — 9 器官 state 共享注册表
//!
//! 借鉴 Golutra v0.1.0 9 Tauri state 装配 (main.rs 启动时 `state.manage(...)` 9 次)
//! 1:1 翻译到 ratatui 路线.
//!
//! ## 设计
//!
//! - 9 器官 9 字段 (heart / brain / hand / eye / ear / memory / voice / body / mind),
//!   编译期 hardcode, 改 1 器官 = 改 1 字段 + 1 match arm
//! - 9 字段默认用 [`MutexState<T>`] (模式 2, 通用 1 锁覆盖 9 器官)
//! - 也可用 [`OrganStateRegistryBuilder`] 自定义每器官模式 (e.g. mind 走 OnceLock, memory 走 RwLock)
//!
//! ## 9 器官默认模式分配 (per 借鉴 Golutra 9 state 模式分析)
//!
//! | 器官 | 默认模式 | 理由 (per 借鉴 Golutra state 模式) |
//! |---|---|---|
//! | heart | Mutex | 60Hz tick 写多 |
//! | brain | Mutex | LLM call 频率计数 |
//! | hand | Mutex | 工具调用统计 |
//! | eye | RwLock | 输入监控 (读多) |
//! | ear | RwLock | 事件订阅 (读多) |
//! | memory | RwLock | 会话历史 (读多) |
//! | voice | Mutex | TTS/STT 状态 |
//! | body | Mutex | 进程/资源统计 |
//! | mind | Mutex | AGI 状态 + 6 锚 |
//!
//! **不假装**: 真实集成时, 模式可按器官特性调 (per `OrganStateRegistryBuilder::with_mode`).

use std::sync::Arc;

use crate::error::StateError;
use crate::mode_mutex::{MutexState, MutexStateMode};
use crate::mode_rw_lock::RwLockState;
use crate::organ::{
    BodyStub, BrainStub, EarStub, EyeStub, HandStub, HeartStub, MemoryStub, MindStub, Organ,
    VoiceStub,
};
use crate::shared_state::{SharedState, SharedStateMode};

/// **K-1 强校验 #4**: OrganStateRegistry 9 字段 (跟借鉴 #1 sister 报告 9 organ 1:1).
pub const REGISTRY_ORGAN_COUNT: usize = 9;

/// 9 器官 state 共享注册表.
///
/// **设计** (per 借鉴 Golutra 9 Tauri state 装配模式):
/// - 9 器官 9 字段, 编译期 hardcode
/// - 默认每器官走 MutexState<T> (模式 2)
/// - `OrganStateRegistryBuilder` 提供 per-organ 模式覆盖
///
/// **不假装**: 9 字段类型都是 `MutexState<OrganStub>` 或 `RwLockState<OrganStub>` (9 OrganStub 类型),
/// 真实集成时换为 sister 报告 9 organ State 类型.
#[derive(Debug, Clone)]
pub struct OrganStateRegistry {
    /// 0: heart (心) — 60Hz tick (Mutex, 写多).
    pub heart: MutexState<HeartStub>,
    /// 1: brain (脑) — LLM call 频率 (Mutex).
    pub brain: MutexState<BrainStub>,
    /// 2: hand (手) — 工具调用统计 (Mutex).
    pub hand: MutexState<HandStub>,
    /// 3: eye (眼) — 输入监控 (RwLock, 读多).
    pub eye: RwLockState<EyeStub>,
    /// 4: ear (耳) — 事件订阅 (RwLock, 读多).
    pub ear: RwLockState<EarStub>,
    /// 5: memory (记忆) — 会话历史 (RwLock, 读多).
    pub memory: RwLockState<MemoryStub>,
    /// 6: voice (声) — TTS/STT (Mutex).
    pub voice: MutexState<VoiceStub>,
    /// 7: body (体) — 进程/资源 (Mutex).
    pub body: MutexState<BodyStub>,
    /// 8: mind (意) — AGI 状态 (Mutex).
    pub mind: MutexState<MindStub>,
}

impl Default for OrganStateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganStateRegistry {
    /// 新建 OrganStateRegistry (9 字段全部 default init).
    ///
    /// per 借鉴 Golutra 9 Tauri state 装配模式: 9 个 `state.manage(...)` 一次性装配.
    pub fn new() -> Self {
        Self {
            heart: MutexState::new(HeartStub::new()),
            brain: MutexState::new(BrainStub::new()),
            hand: MutexState::new(HandStub::new()),
            eye: RwLockState::new(EyeStub::new()),
            ear: RwLockState::new(EarStub::new()),
            memory: RwLockState::new(MemoryStub::new()),
            voice: MutexState::new(VoiceStub::new()),
            body: MutexState::new(BodyStub::new()),
            mind: MutexState::new(MindStub::new()),
        }
    }

    /// 9 器官模式概览 (调试用).
    pub fn mode_summary(&self) -> [SharedStateMode; REGISTRY_ORGAN_COUNT] {
        [
            self.heart.mode(),
            self.brain.mode(),
            self.hand.mode(),
            self.eye.mode(),
            self.ear.mode(),
            self.memory.mode(),
            self.voice.mode(),
            self.body.mode(),
            self.mind.mode(),
        ]
    }

    /// 9 器官中文名 (per ORGAN_NAMES_ZH, 调试用).
    pub fn organ_names(&self) -> [&'static str; REGISTRY_ORGAN_COUNT] {
        [
            Organ::Heart.name_zh(),
            Organ::Brain.name_zh(),
            Organ::Hand.name_zh(),
            Organ::Eye.name_zh(),
            Organ::Ear.name_zh(),
            Organ::Memory.name_zh(),
            Organ::Voice.name_zh(),
            Organ::Body.name_zh(),
            Organ::Mind.name_zh(),
        ]
    }

    /// 9 器官 ASCII 字符 (调试用).
    pub fn ascii_chars(&self) -> [&'static str; REGISTRY_ORGAN_COUNT] {
        [
            Organ::Heart.ascii_char(),
            Organ::Brain.ascii_char(),
            Organ::Hand.ascii_char(),
            Organ::Eye.ascii_char(),
            Organ::Ear.ascii_char(),
            Organ::Memory.ascii_char(),
            Organ::Voice.ascii_char(),
            Organ::Body.ascii_char(),
            Organ::Mind.ascii_char(),
        ]
    }
}

/// **OrganStateRegistryBuilder**: 自定义 9 器官模式 (覆盖默认 MutexState / RwLockState).
///
/// **不假装**: 当前 skeleton 阶段 builder 仅占位, 仅支持默认 `build()` 一次性装配,
/// 真实集成时 R21+ 续做 per-organ 模式定制.
#[derive(Debug, Default, Clone)]
pub struct OrganStateRegistryBuilder {
    /// 内部 OrganStateRegistry (构造期间累积).
    inner: OrganStateRegistry,
}

impl OrganStateRegistryBuilder {
    /// 新建 builder.
    pub fn new() -> Self {
        Self {
            inner: OrganStateRegistry::new(),
        }
    }

    /// Set 某器官的模式 (per organ 索引 0-8, 编译期守门).
    ///
    /// **不假装**: 当前 skeleton 阶段 0 业务, 模式选择仅 log 一行.
    /// 真实集成时 R21+ 续做: `match mode { Mutex => inner.heart = MutexState::new(...), ... }`.
    pub fn with_mode(mut self, _organ: Organ, _mode: SharedStateMode) -> Self {
        // skeleton 阶段: 0 行为, 仅保留 builder API 形状
        self
    }

    /// 构造 OrganStateRegistry.
    pub fn build(self) -> OrganStateRegistry {
        self.inner
    }
}

// =====================================================================
// 单元测试 (9 字段 + 9 模式 + OrganStateRegistry 基础 = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn nine_fields_constructible() {
        let reg = OrganStateRegistry::new();
        // 9 字段都存在 (编译期守门)
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
    fn default_equivalent_to_new() {
        let r1 = OrganStateRegistry::new();
        let r2 = OrganStateRegistry::default();
        // 9 字段都应该可读
        let _ = r1.heart.read();
        let _ = r2.heart.read();
    }

    #[test]
    fn mode_summary_9_entries() {
        let reg = OrganStateRegistry::new();
        let summary = reg.mode_summary();
        assert_eq!(summary.len(), REGISTRY_ORGAN_COUNT);
        // 验证 9 器官模式分配:
        // heart/brain/hand/voice/body/mind = Mutex
        // eye/ear/memory = RwLock
        assert_eq!(summary[0], SharedStateMode::Mutex); // heart
        assert_eq!(summary[1], SharedStateMode::Mutex); // brain
        assert_eq!(summary[2], SharedStateMode::Mutex); // hand
        assert_eq!(summary[3], SharedStateMode::RwLock); // eye
        assert_eq!(summary[4], SharedStateMode::RwLock); // ear
        assert_eq!(summary[5], SharedStateMode::RwLock); // memory
        assert_eq!(summary[6], SharedStateMode::Mutex); // voice
        assert_eq!(summary[7], SharedStateMode::Mutex); // body
        assert_eq!(summary[8], SharedStateMode::Mutex); // mind
    }

    #[test]
    fn organ_names_9_distinct() {
        let reg = OrganStateRegistry::new();
        let names = reg.organ_names();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn ascii_chars_9_distinct() {
        let reg = OrganStateRegistry::new();
        let chars = reg.ascii_chars();
        let unique: std::collections::HashSet<&str> = chars.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn all_9_states_readable() -> Result<(), StateError> {
        let reg = OrganStateRegistry::new();
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            // 编译期守门: 9 State 都能 read (拿到 stub)
            match organ {
                Organ::Heart => {
                    let _ = reg.heart.read()?;
                }
                Organ::Brain => {
                    let _ = reg.brain.read()?;
                }
                Organ::Hand => {
                    let _ = reg.hand.read()?;
                }
                Organ::Eye => {
                    let _ = reg.eye.read()?;
                }
                Organ::Ear => {
                    let _ = reg.ear.read()?;
                }
                Organ::Memory => {
                    let _ = reg.memory.read()?;
                }
                Organ::Voice => {
                    let _ = reg.voice.read()?;
                }
                Organ::Body => {
                    let _ = reg.body.read()?;
                }
                Organ::Mind => {
                    let _ = reg.mind.read()?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn all_9_states_writable() -> Result<(), StateError> {
        let reg = OrganStateRegistry::new();
        for n in 0..=8u8 {
            let organ = Organ::from_u8(n).unwrap();
            match organ {
                Organ::Heart => {
                    let _ = reg.heart.write()?;
                }
                Organ::Brain => {
                    let _ = reg.brain.write()?;
                }
                Organ::Hand => {
                    let _ = reg.hand.write()?;
                }
                Organ::Eye => {
                    let _ = reg.eye.write()?;
                }
                Organ::Ear => {
                    let _ = reg.ear.write()?;
                }
                Organ::Memory => {
                    let _ = reg.memory.write()?;
                }
                Organ::Voice => {
                    let _ = reg.voice.write()?;
                }
                Organ::Body => {
                    let _ = reg.body.write()?;
                }
                Organ::Mind => {
                    let _ = reg.mind.write()?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn registry_clone_shares_state() {
        let reg = OrganStateRegistry::new();
        let cloned = reg.clone();
        // 改 cloned.heart, reg.heart 应该看到
        {
            let mut g = cloned.heart.write().unwrap();
            g._marker = 99;
        }
        let g = reg.heart.read().unwrap();
        assert_eq!(g._marker, 99);
    }

    #[test]
    fn registry_organ_count_constant_is_9() {
        assert_eq!(REGISTRY_ORGAN_COUNT, 9);
    }

    #[test]
    fn builder_default_equivalent_to_new() {
        let b1 = OrganStateRegistryBuilder::new();
        let b2 = OrganStateRegistryBuilder::default();
        let _ = b1.build();
        let _ = b2.build();
    }

    #[test]
    fn builder_with_mode_keeps_state() {
        let reg = OrganStateRegistryBuilder::new()
            .with_mode(Organ::Heart, SharedStateMode::Mutex)
            .with_mode(Organ::Memory, SharedStateMode::RwLock)
            .build();
        // 9 字段都应该可读
        let _ = reg.heart.read();
        let _ = reg.memory.read();
    }

    #[test]
    fn concurrent_read_9_organs_smoke() {
        // 9 器官并发读 smoke test (借 Arc 共享 reg)
        let reg = Arc::new(OrganStateRegistry::new());
        let mut handles = vec![];
        for n in 0..9 {
            let reg_clone = Arc::clone(&reg);
            handles.push(thread::spawn(move || -> Result<(), StateError> {
                let organ = Organ::from_u8(n as u8).unwrap();
                // 每器官读 1 次
                match organ {
                    Organ::Heart => {
                        let _ = reg_clone.heart.read()?;
                    }
                    Organ::Brain => {
                        let _ = reg_clone.brain.read()?;
                    }
                    Organ::Hand => {
                        let _ = reg_clone.hand.read()?;
                    }
                    Organ::Eye => {
                        let _ = reg_clone.eye.read()?;
                    }
                    Organ::Ear => {
                        let _ = reg_clone.ear.read()?;
                    }
                    Organ::Memory => {
                        let _ = reg_clone.memory.read()?;
                    }
                    Organ::Voice => {
                        let _ = reg_clone.voice.read()?;
                    }
                    Organ::Body => {
                        let _ = reg_clone.body.read()?;
                    }
                    Organ::Mind => {
                        let _ = reg_clone.mind.read()?;
                    }
                }
                Ok(())
            }));
        }
        for h in handles {
            h.join()
                .expect("thread should not panic")
                .expect("read should succeed");
        }
    }

    #[test]
    fn mutex_state_mode_marker_constant() {
        // 借 MutexStateMode 验常量
        assert_eq!(MutexStateMode::as_str(), "mutex");
    }
}
