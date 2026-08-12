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

#![warn(missing_docs)]

pub mod hash;
pub mod exif;
pub mod ocr;
pub mod router;
pub mod mcp;
pub mod compat;
pub mod enhanced;

pub use hash::{ImageHash, perceptual_hash};
pub use exif::ExifData;
pub use ocr::{OcrResult, ocr_extract};
pub use router::{ImageRouter, ProcessOp};
pub use mcp::{ImageProcessMcp, ImageProcessTool};
pub use compat::{ImageProcessCommand, ImageProcessCompatRouter, IMAGEPROC_COMMAND_COUNT};
pub use enhanced::EnhancedImageProcess;

/// R141 deliverables for image-process:
/// - 5 modules (hash / exif / ocr / router / mcp) + compat + enhanced
pub const R141_IMAGE_PROC_DELIVERABLES: usize = 7;