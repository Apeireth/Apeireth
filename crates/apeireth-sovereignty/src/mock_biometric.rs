//! Mock Biometric Provider — Rust 内真实实现 (无 PyO3 / 无外部 SDK)
//!
//! **用途**:
//! - 测试场景: 默认 OK / 模拟胁迫 / 模拟失败 / 模拟不可用
//! - 离线场景: `is_available() = false`
//!
//! **未来真实实现**:
//! - `WindowsHelloProvider` 通过 `windows-rs` crate 调用 WinRT API (不通过 PyO3)
//! - `Fido2Provider` 通过 `libfido2-sys` crate 调用 FIDO2 (Rust FFI, 不通过 PyO3)
//! - `MasterKeyProvider` 通过 PKCS#11 / 智能卡 crate

use crate::ha::{BiometricProvider, BiometricResult};
use std::collections::HashMap;
use std::sync::Mutex;

/// 胁迫行为 — 控制 MockBiometric 在特定 human_id 下模拟胁迫检测。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoercionBehavior {
    /// 正常认证
    Normal,
    /// 模拟胁迫 (返回 CoercionDetected)
    Coerce,
    /// 模拟失败
    Fail,
    /// 模拟不可用
    Unavailable,
}

/// Mock Biometric Provider 配置。
#[derive(Debug, Clone)]
pub struct MockBiometricBehavior {
    /// per-human-id 行为
    per_human: HashMap<String, CoercionBehavior>,
    /// 默认行为
    default_behavior: CoercionBehavior,
    /// 是否整体可用 (false = 离线场景)
    available: bool,
}

impl Default for MockBiometricBehavior {
    fn default() -> Self {
        Self {
            per_human: HashMap::new(),
            default_behavior: CoercionBehavior::Normal,
            available: true,
        }
    }
}

impl MockBiometricBehavior {
    /// 创建新配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置默认行为
    pub fn with_default(mut self, behavior: CoercionBehavior) -> Self {
        self.default_behavior = behavior;
        self
    }

    /// 设置特定 human 行为
    pub fn with_human(mut self, human_id: impl Into<String>, behavior: CoercionBehavior) -> Self {
        self.per_human.insert(human_id.into(), behavior);
        self
    }

    /// 设置整体可用性 (false = 离线)
    pub fn with_available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// 获取特定 human 的行为
    pub fn behavior_for(&self, human_id: &str) -> CoercionBehavior {
        self.per_human
            .get(human_id)
            .copied()
            .unwrap_or(self.default_behavior)
    }
}

/// Mock Biometric Provider — Rust 内真实实现, 用于测试 + 离线场景.
pub struct MockBiometric {
    behavior: Mutex<MockBiometricBehavior>,
    provider_name: String,
}

impl MockBiometric {
    /// 创建默认 mock (所有 human 正常认证)
    pub fn new() -> Self {
        Self {
            behavior: Mutex::new(MockBiometricBehavior::default()),
            provider_name: "mock-biometric".to_string(),
        }
    }

    /// 创建带配置的 mock
    pub fn with_behavior(behavior: MockBiometricBehavior) -> Self {
        Self {
            behavior: Mutex::new(behavior),
            provider_name: "mock-biometric".to_string(),
        }
    }

    /// 创建离线 mock (不可用)
    pub fn offline() -> Self {
        Self {
            behavior: Mutex::new(MockBiometricBehavior::new().with_available(false)),
            provider_name: "mock-biometric-offline".to_string(),
        }
    }

    /// 动态修改行为 (供测试)
    pub fn set_behavior(&self, human_id: &str, behavior: CoercionBehavior) {
        let mut b = self.behavior.lock().expect("biometric poisoned");
        b.per_human.insert(human_id.to_string(), behavior);
    }

    /// 当前行为快照
    pub fn current_behavior(&self) -> MockBiometricBehavior {
        self.behavior.lock().expect("biometric poisoned").clone()
    }

    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

impl Default for MockBiometric {
    fn default() -> Self {
        Self::new()
    }
}

impl BiometricProvider for MockBiometric {
    fn authenticate(&self, human_id: &str) -> BiometricResult {
        let b = self.behavior.lock().expect("biometric poisoned");
        if !b.available {
            return BiometricResult::Unavailable {
                reason: "提供者整体不可用 (离线模式)".into(),
            };
        }
        let behavior = b.behavior_for(human_id);
        let at_ms = self.now_ms();
        drop(b); // 早释放锁
        match behavior {
            CoercionBehavior::Normal => BiometricResult::Authenticated {
                confidence: 0.95,
                at_ms,
            },
            CoercionBehavior::Coerce => BiometricResult::CoercionDetected {
                stress_level: 0.88,
                at_ms,
            },
            CoercionBehavior::Fail => BiometricResult::Failed {
                reason: "模拟认证失败".into(),
                at_ms,
            },
            CoercionBehavior::Unavailable => BiometricResult::Unavailable {
                reason: "特定 human 模拟不可用".into(),
            },
        }
    }

    fn is_available(&self) -> bool {
        self.behavior.lock().expect("biometric poisoned").available
    }

    fn provider_name(&self) -> &str {
        &self.provider_name
    }
}
