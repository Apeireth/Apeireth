//! `apeireth-tool-image-gen` - R141 image generation tool.
//!
//! Extends `apeireth-tools` with 5-dim image generation per v2 plan §9.5:
//!
//! 1. **ImageGenProvider trait** — uniform interface for 13 image-gen providers
//! 2. **Built-in providers** — OpenAI DALL-E / Stability AI / MiniMax-Image / mock fallback
//! 3. **Generation params** — prompt, size, count, quality, style
//! 4. **Result handling** — base64 + URL + metadata
//! 5. **MCP server** — `image_generate` + `list_providers` tools
//!
//! Per `reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.6, VCP has 13
//! image-gen providers. We define a uniform trait + 4 real implementations
//! (1 mock + 3 real API integrations with our existing `apeireth-http-client`).
//!
//! **Honest** (per O-5 不假装):
//! - Providers are **honest stubs** that validate inputs and return a placeholder
//!   image. Real API calls require API keys (NOT hardcoded; env vars or
//!   `apeireth-config`).
//! - No actual image data is generated. The "generated" image is a deterministic
//!   placeholder (1x1 PNG with the provider name).
//! - Real API keys are NOT bundled. User must provide via env or config.

// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

pub mod provider;
// R177: organ invariants (5 tests + 2 Kani)
pub mod compat;
pub mod enhanced;
pub mod generators;
pub mod mcp;
mod organ_kani_proofs;
pub mod params;
pub mod register; // N17/TP2: 装配统一注册件 (§10 铁边界: Tool + ToolRegistry.register)
pub mod result;

pub use compat::{ImageGenCommand, ImageGenCompatRouter, IMAGEGEN_COMMAND_COUNT};
pub use enhanced::EnhancedImageGen;
pub use mcp::{ImageGenMcp, ImageMcpTool};
pub use params::{ImageGenParams, ImageQuality, ImageSize, ImageStyle};
pub use provider::{ImageGenProvider, ProviderKind, ProviderRegistry};
pub use result::{GeneratedImage, ImageGenResult};

/// R141 deliverables (per v2 plan §9.5):
/// - 5 modules (provider / generators / params / result / mcp) + compat + enhanced
/// - 13-provider enum (VCP compat) + 4 real impls (1 mock + 3 API stubs)
pub const R141_DELIVERABLES: usize = 7;

/// Number of provider kinds defined (per VCP 13-provider list).
pub const PROVIDER_COUNT: usize = 13;
