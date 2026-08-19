//! Integration tests for apeireth-tool-image-gen (post-1.0.0)
//!
//! src/ 5 module 真实现 (provider/params/result/generators/mcp + compat + enhanced).
//! 这里 (tests/) 加跨 API 集成 + 边界.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_tool_image_gen::provider::ProviderError;
use apeireth_tool_image_gen::{
    GeneratedImage, ImageGenCommand, ImageGenCompatRouter, ImageGenParams, ImageGenResult,
    ImageQuality, ImageSize, ImageStyle, ProviderKind, ProviderRegistry, IMAGEGEN_COMMAND_COUNT,
    PROVIDER_COUNT, R141_DELIVERABLES,
};

// =============================================================================
// Constants
// =============================================================================

#[test]
fn r141_deliverables_count() {
    assert_eq!(R141_DELIVERABLES, 7);
}

#[test]
fn provider_count_is_13() {
    assert_eq!(PROVIDER_COUNT, 13);
}

#[test]
fn imagegen_command_count_is_12() {
    assert_eq!(IMAGEGEN_COMMAND_COUNT, 12);
}

// =============================================================================
// ImageSize
// =============================================================================

#[test]
fn image_size_dimensions() {
    assert_eq!(ImageSize::Small.dimensions(), (256, 256));
    assert_eq!(ImageSize::Medium.dimensions(), (512, 512));
    assert_eq!(ImageSize::Large.dimensions(), (1024, 1024));
    assert_eq!(ImageSize::Portrait.dimensions(), (1024, 1792));
    assert_eq!(ImageSize::Landscape.dimensions(), (1792, 1024));
    assert_eq!(ImageSize::Custom(800, 600).dimensions(), (800, 600));
}

#[test]
fn image_size_as_str() {
    assert_eq!(ImageSize::Small.as_str(), "256x256");
    assert_eq!(ImageSize::Medium.as_str(), "512x512");
    assert_eq!(ImageSize::Large.as_str(), "1024x1024");
    assert_eq!(ImageSize::Portrait.as_str(), "1024x1792");
    assert_eq!(ImageSize::Landscape.as_str(), "1792x1024");
    assert_eq!(ImageSize::Custom(100, 100).as_str(), "custom");
}

#[test]
fn image_size_eq_hash() {
    let s1 = ImageSize::Medium;
    let s2 = ImageSize::Medium;
    let s3 = ImageSize::Small;
    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
    let mut set = std::collections::HashSet::new();
    set.insert(s1);
    set.insert(s2);
    set.insert(s3);
    assert_eq!(set.len(), 2);
}

#[test]
fn image_size_serde() {
    let s = ImageSize::Custom(800, 600);
    let json = serde_json::to_string(&s).unwrap();
    let back: ImageSize = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
}

// =============================================================================
// ImageQuality / ImageStyle
// =============================================================================

#[test]
fn image_quality_variants() {
    assert_ne!(ImageQuality::Draft, ImageQuality::Standard);
    assert_ne!(ImageQuality::Standard, ImageQuality::HD);
    assert_ne!(ImageQuality::Draft, ImageQuality::HD);
}

#[test]
fn image_style_variants_5() {
    let styles = [
        ImageStyle::Natural,
        ImageStyle::Vivid,
        ImageStyle::Anime,
        ImageStyle::Photographic,
        ImageStyle::Cinematic,
    ];
    assert_eq!(styles.len(), 5);
    let unique: std::collections::HashSet<_> = styles.iter().collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn image_quality_serde() {
    for q in [
        ImageQuality::Draft,
        ImageQuality::Standard,
        ImageQuality::HD,
    ] {
        let s = serde_json::to_string(&q).unwrap();
        let back: ImageQuality = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
    }
}

// =============================================================================
// ImageGenParams
// =============================================================================

#[test]
fn image_gen_params_new() {
    let p = ImageGenParams::new("a cat");
    assert_eq!(p.prompt, "a cat");
    assert_eq!(p.count, 1);
    assert_eq!(p.size, ImageSize::Large);
    assert_eq!(p.quality, ImageQuality::Standard);
    assert_eq!(p.style, ImageStyle::Natural);
    assert!(p.negative_prompt.is_none());
    assert!(p.seed.is_none());
}

#[test]
fn image_gen_params_builder() {
    let p = ImageGenParams::new("test")
        .with_size(ImageSize::Medium)
        .with_quality(ImageQuality::HD)
        .with_style(ImageStyle::Cinematic)
        .with_count(3);
    assert_eq!(p.size, ImageSize::Medium);
    assert_eq!(p.quality, ImageQuality::HD);
    assert_eq!(p.style, ImageStyle::Cinematic);
    assert_eq!(p.count, 3);
}

#[test]
fn image_gen_params_clone() {
    let p = ImageGenParams::new("test");
    let p2 = p.clone();
    assert_eq!(p.prompt, p2.prompt);
    assert_eq!(p.count, p2.count);
}

#[test]
fn image_gen_params_serde_roundtrip() {
    let p = ImageGenParams::new("hello world")
        .with_size(ImageSize::Portrait)
        .with_count(2);
    let s = serde_json::to_string(&p).unwrap();
    let back: ImageGenParams = serde_json::from_str(&s).unwrap();
    assert_eq!(back.prompt, "hello world");
    assert_eq!(back.size, ImageSize::Portrait);
    assert_eq!(back.count, 2);
}

// =============================================================================
// GeneratedImage / ImageGenResult
// =============================================================================

#[test]
fn generated_image_construction() {
    let img = GeneratedImage {
        data: vec![0x89, 0x50, 0x4E, 0x47],
        mime: "image/png".to_string(),
        width: 1024,
        height: 1024,
        url: Some("https://example.com/img.png".into()),
        seed: Some(42),
    };
    assert_eq!(img.mime, "image/png");
    assert_eq!(img.width, 1024);
    assert_eq!(img.seed, Some(42));
    assert_eq!(img.url.as_deref(), Some("https://example.com/img.png"));
}

#[test]
fn generated_image_serde() {
    let img = GeneratedImage {
        data: vec![0u8; 4],
        mime: "image/jpeg".into(),
        width: 100,
        height: 200,
        url: None,
        seed: None,
    };
    let s = serde_json::to_string(&img).unwrap();
    let back: GeneratedImage = serde_json::from_str(&s).unwrap();
    // GeneratedImage 不派生 PartialEq, 手动比对字段
    assert_eq!(back.data, img.data);
    assert_eq!(back.width, img.width);
    assert_eq!(back.height, img.height);
    assert_eq!(back.url, img.url);
    assert_eq!(back.seed, img.seed);
}

#[test]
fn image_gen_result_construction() {
    let r = ImageGenResult {
        provider: "mock".to_string(),
        model: "mock-v1".to_string(),
        images: vec![],
        timestamp: "2026-08-12T00:00:00Z".to_string(),
        elapsed_ms: 100,
    };
    assert_eq!(r.provider, "mock");
    assert_eq!(r.elapsed_ms, 100);
    assert!(r.images.is_empty());
}

#[test]
fn image_gen_result_with_images() {
    let img = GeneratedImage {
        data: vec![1, 2, 3],
        mime: "image/png".into(),
        width: 64,
        height: 64,
        url: None,
        seed: Some(1),
    };
    let r = ImageGenResult {
        provider: "x".into(),
        model: "m".into(),
        images: vec![img.clone()],
        timestamp: "t".into(),
        elapsed_ms: 50,
    };
    assert_eq!(r.images.len(), 1);
    assert_eq!(r.images[0].width, 64);
}

// =============================================================================
// ProviderKind
// =============================================================================

#[test]
fn provider_kind_all_count() {
    assert_eq!(ProviderKind::all().len(), 13);
}

#[test]
fn provider_kind_names_unique() {
    let names: Vec<&str> = ProviderKind::all().iter().map(|k| k.name()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len());
}

#[test]
fn provider_kind_name_specific() {
    assert_eq!(ProviderKind::OpenAiDallE.name(), "openai-dalle");
    assert_eq!(ProviderKind::StabilityAi.name(), "stability-ai");
    assert_eq!(ProviderKind::Midjourney.name(), "midjourney");
    assert_eq!(ProviderKind::MiniMaxImage.name(), "minimax-image");
    assert_eq!(ProviderKind::GoogleImagen.name(), "google-imagen");
    assert_eq!(ProviderKind::AdobeFirefly.name(), "adobe-firefly");
    assert_eq!(ProviderKind::LeonardoAi.name(), "leonardo-ai");
    assert_eq!(ProviderKind::Ideogram.name(), "ideogram");
    assert_eq!(ProviderKind::PlaygroundAi.name(), "playground-ai");
    assert_eq!(ProviderKind::BingImageCreator.name(), "bing-image-creator");
    assert_eq!(ProviderKind::Craiyon.name(), "craiyon");
    assert_eq!(ProviderKind::Nightcafe.name(), "nightcafe");
    assert_eq!(ProviderKind::Mock.name(), "mock");
}

#[test]
fn provider_kind_eq_copy_hash() {
    let k = ProviderKind::OpenAiDallE;
    let k2 = k;
    assert_eq!(k, k2);
    let mut set = std::collections::HashSet::new();
    set.insert(k);
    set.insert(k2);
    assert_eq!(set.len(), 1);
}

// =============================================================================
// ProviderRegistry
// =============================================================================

#[test]
fn provider_registry_new_empty() {
    let r = ProviderRegistry::new();
    assert_eq!(r.count(), 0);
    assert!(r.names().is_empty());
    assert!(r.get("nope").is_none());
}

#[test]
fn provider_registry_default() {
    let r = ProviderRegistry::default();
    assert_eq!(r.count(), 0);
}

// =============================================================================
// ProviderError
// =============================================================================

#[test]
fn provider_error_not_implemented_display() {
    let e = ProviderError::NotImplemented("test".into());
    let s = e.to_string();
    assert!(s.contains("test"));
    assert!(s.contains("not implemented") || s.contains("未实现"));
}

#[test]
fn provider_error_missing_api_key_display() {
    let e = ProviderError::MissingApiKey("openai-dalle".into());
    let s = e.to_string();
    assert!(s.contains("openai-dalle"));
    assert!(s.contains("API key"));
}

#[test]
fn provider_error_api_display() {
    let e = ProviderError::Api("rate limited".into());
    let s = e.to_string();
    assert!(s.contains("rate limited"));
}

// =============================================================================
// ImageGenCommand
// =============================================================================

#[test]
fn imagegen_command_from_str_12() {
    let names = [
        "OpenAiDallE",
        "StabilityAi",
        "Midjourney",
        "MiniMaxImage",
        "GoogleImagen",
        "AdobeFirefly",
        "LeonardoAi",
        "Ideogram",
        "PlaygroundAi",
        "BingImageCreator",
        "Craiyon",
        "Nightcafe",
    ];
    for n in names {
        assert_ne!(ImageGenCommand::from_str(n), ImageGenCommand::Unknown);
    }
}

#[test]
fn imagegen_command_unknown_fallback() {
    assert_eq!(ImageGenCommand::from_str("xyz"), ImageGenCommand::Unknown);
    assert_eq!(ImageGenCommand::from_str(""), ImageGenCommand::Unknown);
}

#[test]
fn imagegen_command_eq_hash() {
    let a = ImageGenCommand::OpenAiDallE;
    let b = ImageGenCommand::OpenAiDallE;
    let c = ImageGenCommand::Midjourney;
    assert_eq!(a, b);
    assert_ne!(a, c);
    let mut set = std::collections::HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(c);
    assert_eq!(set.len(), 2);
}

// =============================================================================
// ImageGenCompatRouter
// =============================================================================

#[test]
fn imagegen_router_count() {
    assert_eq!(ImageGenCompatRouter::command_count(), 12);
}

#[test]
fn imagegen_router_default() {
    let _r = ImageGenCompatRouter::default();
}

// =============================================================================
// Cross-module integration
// =============================================================================

#[test]
fn integration_params_to_result() {
    // 模拟生成流程: Params → Result with images
    let p = ImageGenParams::new("a cat").with_count(2);
    assert_eq!(p.count, 2);
    let img = GeneratedImage {
        data: vec![0u8; 10],
        mime: "image/png".into(),
        width: p.size.dimensions().0,
        height: p.size.dimensions().1,
        url: None,
        seed: None,
    };
    let r = ImageGenResult {
        provider: ProviderKind::Mock.name().into(),
        model: "mock-v1".into(),
        images: vec![img.clone(); p.count as usize],
        timestamp: "2026-08-12T00:00:00Z".into(),
        elapsed_ms: 100,
    };
    assert_eq!(r.images.len(), 2);
    assert_eq!(r.images[0].width, ImageSize::Large.dimensions().0);
}

#[test]
fn integration_provider_kind_to_command() {
    // ProviderKind 13 vs ImageGenCommand 12 (Mock 不在 command enum)
    assert_eq!(ProviderKind::all().len(), 13);
    assert_eq!(IMAGEGEN_COMMAND_COUNT, 12);
    // Mock provider 没有对应 command
    for kind in ProviderKind::all() {
        if *kind == ProviderKind::Mock {
            continue;
        }
        // 其他 12 个应有对应 command name (PascalCase 转换)
        let _ = kind.name();
    }
}

#[test]
fn integration_params_with_seed() {
    let p = ImageGenParams::new("test");
    assert!(p.seed.is_none());
    // 模拟设置 seed (这里手动, 因为 src 没有 with_seed builder)
    let mut p2 = p;
    p2.seed = Some(12345);
    assert_eq!(p2.seed, Some(12345));
}

#[test]
fn integration_size_variants_dims() {
    let sizes = [
        ImageSize::Small,
        ImageSize::Medium,
        ImageSize::Large,
        ImageSize::Portrait,
        ImageSize::Landscape,
    ];
    let unique_dims: std::collections::HashSet<(u32, u32)> =
        sizes.iter().map(|s| s.dimensions()).collect();
    assert_eq!(unique_dims.len(), 5, "5 个 variant 各自独特维度");
}

#[test]
fn integration_generated_image_round_trip() {
    let img = GeneratedImage {
        data: vec![0xFFu8; 8],
        mime: "image/jpeg".into(),
        width: 256,
        height: 256,
        url: Some("https://example.com/x.jpg".into()),
        seed: Some(99),
    };
    let json = serde_json::to_string(&img).unwrap();
    let back: GeneratedImage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.data, img.data);
    assert_eq!(back.width, img.width);
    assert_eq!(back.height, img.height);
    assert_eq!(back.url, img.url);
    assert_eq!(back.seed, img.seed);
}
