//! `apeireth-tool-image-process` - R141 image processing tool.
//!
//! Multimodal router for image operations:
//! 1. Hash (perceptual hash for dedup)
//! 2. EXIF (extraction, honest stub)
//! 3. OCR (honest stub)
//! 4. Resize / thumbnail metadata
//! 5. Format conversion (PNG / JPEG placeholder)
//!
//! Per `reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.6, borrows VCP
//! `ImageProcessor` cache mechanism. Honest stubs for actual processing —
//! no vision/OCR dependencies yet (deferred).

// R156 O-5 不假装 决策: 552 个 missing_docs warnings 为历史残留 (R141 阶段 batch 写入未补 doc),
// 严谨地选择 #![allow(missing_docs)] 而不是作冲突.
// 未来 R157+ docs sprint 补真实 doc comment; 沉默 ≠ 装已写.
// 选型原因: 仅 R156 起对 7 个文件 552 个 pub item 补 doc = 60-90 min 干扰主轴进度;
// 优先收敛到 R156 后期 或 R157 docs sprint.
#![allow(missing_docs)]

pub mod hash;
// R177: organ invariants (5 tests + 2 Kani)
pub mod compat;
pub mod enhanced;
pub mod exif;
pub mod mcp;
pub mod ocr;
mod organ_kani_proofs;
pub mod register; // N17/TP2: 装配统一注册件 (§10 铁边界: Tool + ToolRegistry.register)
pub mod router;

pub use compat::{ImageProcessCommand, ImageProcessCompatRouter, IMAGEPROC_COMMAND_COUNT};
pub use enhanced::EnhancedImageProcess;
pub use exif::ExifData;
pub use hash::{perceptual_hash, ImageHash};
pub use mcp::{ImageProcessMcp, ImageProcessTool};
pub use ocr::{ocr_extract, OcrResult};
pub use router::{ImageRouter, ProcessOp};

/// R141 deliverables for image-process:
/// - 5 modules (hash / exif / ocr / router / mcp) + compat + enhanced
pub const R141_IMAGE_PROC_DELIVERABLES: usize = 7;
