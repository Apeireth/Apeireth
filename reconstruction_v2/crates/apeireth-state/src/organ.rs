//! 9 organ enum + 9 OrganStub types (编译期 hardcode).
use std::sync::Arc;

/// 9器官总计数.
pub const ORGAN_COUNT: usize = 9;

/// 9器官中文名.
pub const ORGAN_NAMES_ZH: [&str; 9] = ["心", "脑", "手", "眼", "耳", "记忆", "声", "身", "意"];

/// 9器官 ASCII 单字符.
pub const ORGAN_ASCII_CHARS: [char; 9] = ['h', 'b', 'H', 'y', 'e', 'm', 'v', 'B', 'M'];

/// 9器官变体.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Organ { Heart, Brain, Hand, Eye, Ear, Memory, Voice, Body, Mind }

impl Organ {
    pub const ALL: [Organ; 9] = [Self::Heart, Self::Brain, Self::Hand, Self::Eye, Self::Ear, Self::Memory, Self::Voice, Self::Body, Self::Mind];

    pub fn as_str(&self) -> &'static str {
        match self { Self::Heart=>"heart", Self::Brain=>"brain", Self::Hand=>"hand", Self::Eye=>"eye", Self::Ear=>"ear", Self::Memory=>"memory", Self::Voice=>"voice", Self::Body=>"body", Self::Mind=>"mind" }
    }
}

/// 9 OrganStub trait (R21+ 真接具体 organ state).
pub trait OrganImpl: Send + Sync + std::fmt::Debug {
    fn organ(&self) -> Organ;
}

/// Convenience: dyn trait object for all organ stubs.
pub type OrganStub = Arc<dyn OrganImpl>;
pub type HeartStub  = OrganStub;
pub type BrainStub  = OrganStub;
pub type HandStub   = OrganStub;
pub type EyeStub    = OrganStub;
pub type EarStub    = OrganStub;
pub type MemoryStub = OrganStub;
pub type VoiceStub  = OrganStub;
pub type BodyStub   = OrganStub;
pub type MindStub   = OrganStub;