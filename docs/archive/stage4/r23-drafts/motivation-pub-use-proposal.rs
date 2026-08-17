//! R23 pub use 顶层导出草稿（apeireth-motivation）。
//!
//! **8 项承诺**: 全部遵守。**不假装**: 本文件仅为草稿，**未应用**到 lib.rs。
//! **不修改承诺 (LOCKED)**: 草稿独立成文，待主人/Mavis 在 R23 拍板后由他们显式写入。

//! # 建议加在 crates/apeireth-motivation/src/lib.rs 末尾（pub mod 声明之后）

pub use crate::{
    DriveKind, Evidence, EvidenceKind, ExternalDrive, InternalDrive, Modality,
    MultimodalIntent, MotivationDrive, MotivationError, MotivationScore, SGI,
    SGIContent, SGIStructured, WriteResult,
};
