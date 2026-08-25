//! OrganStateRegistry — 9 器官 state 聚合.
use crate::organ::{OrganStub, ORGAN_COUNT};

/// 9器官 state 注册表 — 字段硬编码 (改 1 器官 = 改 1 字段 + 1 match arm).
pub const REGISTRY_ORGAN_COUNT: usize = 9;

#[derive(Debug)]
pub struct OrganStateRegistry {
    pub heart:  OrganStub,
    pub brain:  OrganStub,
    pub hand:   OrganStub,
    pub eye:    OrganStub,
    pub ear:    OrganStub,
    pub memory: OrganStub,
    pub voice:  OrganStub,
    pub body:   OrganStub,
    pub mind:   OrganStub,
}

impl OrganStateRegistry {
    /// 默认空 9 organ 注册表 (callers fill 9 fields).
    pub fn new(heart: OrganStub, brain: OrganStub, hand: OrganStub, eye: OrganStub, ear: OrganStub, memory: OrganStub, voice: OrganStub, body: OrganStub, mind: OrganStub) -> Self {
        Self { heart, brain, hand, eye, ear, memory, voice, body, mind }
    }

    /// Count via constant (always 9).
    pub fn organ_count(&self) -> usize { ORGAN_COUNT }
}

/// Builder for OrganStateRegistry.
pub struct OrganStateRegistryBuilder {
    pub heart:  Option<OrganStub>,
    pub brain:  Option<OrganStub>,
    pub hand:   Option<OrganStub>,
    pub eye:    Option<OrganStub>,
    pub ear:    Option<OrganStub>,
    pub memory: Option<OrganStub>,
    pub voice:  Option<OrganStub>,
    pub body:   Option<OrganStub>,
    pub mind:   Option<OrganStub>,
}

impl OrganStateRegistryBuilder {
    pub fn new() -> Self {
        Self { heart: None, brain: None, hand: None, eye: None, ear: None, memory: None, voice: None, body: None, mind: None }
    }

    pub fn heart(mut self, s: OrganStub) -> Self { self.heart = Some(s); self }
    pub fn brain(mut self, s: OrganStub) -> Self { self.brain = Some(s); self }
    pub fn hand(mut self, s: OrganStub) -> Self { self.hand = Some(s); self }
    pub fn eye(mut self, s: OrganStub) -> Self { self.eye = Some(s); self }
    pub fn ear(mut self, s: OrganStub) -> Self { self.ear = Some(s); self }
    pub fn memory(mut self, s: OrganStub) -> Self { self.memory = Some(s); self }
    pub fn voice(mut self, s: OrganStub) -> Self { self.voice = Some(s); self }
    pub fn body(mut self, s: OrganStub) -> Self { self.body = Some(s); self }
    pub fn mind(mut self, s: OrganStub) -> Self { self.mind = Some(s); self }

    pub fn build(self) -> Option<OrganStateRegistry> {
        Some(OrganStateRegistry::new(
            self.heart?, self.brain?, self.hand?, self.eye?, self.ear?, self.memory?, self.voice?, self.body?, self.mind?,
        ))
    }
}

impl Default for OrganStateRegistryBuilder {
    fn default() -> Self { Self::new() }
}
