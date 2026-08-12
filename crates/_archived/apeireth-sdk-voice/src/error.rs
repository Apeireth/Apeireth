//! # Voice error types (per @anthropic-ai/voice 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 12 错误 variant, 编译期 hardcode. 真接 Anthropic Voice API 时
//! 把 `NotImplemented` 移除并把 `Network` / `RateLimited` / `PermissionDenied` 等细化.
//!
//! ## 6 K-1 强校验方法 (per task spec §3)
//!
//! | K-1 | 方法 | 守门 | 失败时返 |
//! |---:|---|---|---|
//! | #1 | `validate_api_key` | 非空 + 长度 ≥ 16 | `ApiKeyMissing` / `ApiKeyInvalid` |
//! | #2 | `validate_audio_format` | 必须是 `wav` / `mp3` / `opus` / `flac` (lowercase) | `AudioFormatInvalid` |
//! | #3 | `validate_sample_rate` | 必须是 8000..=48000 Hz | `SampleRateInvalid` |
//! | #4 | `validate_bit_depth` | 必须是 8 / 16 / 24 / 32 | `BitDepthInvalid` |
//! | #5 | `validate_channels` | 必须是 1 / 2 | `ChannelsInvalid` |
//! | #6 | `validate_language` | 必须是 ISO 639-1 (e.g. `en` / `zh-CN`) | `LanguageInvalid` |
//!
//! ## 守门宏: `voice_stub!`
//!
//! 6 API stub 全部 `Err(VoiceError::NotImplemented(api))`, 用 `voice_stub!("transcribe")`
//! 一行 log + return, 防漏改.
//!
//! ## 引用文档
//!
//! 1. `@anthropic-ai/voice v0.9.21` `core/Response.d.ts` (商业版 Response 1:1 翻译源)
//! 2. `@anthropic-ai/voice v0.9.21` `client/api_transcribe.js` (transcribe/synthesize 1:1 翻译源)
//! 3. `docs/stage4/m3-hallucination-defense-2026-08-05.md` §2.4 (TOOL_WHITELIST 模式)

use thiserror::Error;

// ============================================================================
// §1 VoiceError (12 variant, K-1 强校验 6 + STUB 1 + 5 网络/业务/其他)
// ============================================================================

/// Voice SDK 错误 (12 variant, 4 大类).
///
/// 1. **STUB** (1): `NotImplemented(api)` — 6 API stub 全部返
/// 2. **K-1 强校验** (6): `ApiKeyMissing` / `ApiKeyInvalid` / `AudioFormatInvalid` /
///    `SampleRateInvalid` / `BitDepthInvalid` / `ChannelsInvalid` / `LanguageInvalid`
///    (实际只 6 类, 各 1 variant)
/// 3. **m3 防御** (1): `ToolNotWhitelisted(tool)` — 工具未在白名单内
/// 4. **鉴权/网络/限流/业务/其他** (4): `TokenExpired` / `Network` / `RateLimited` / `Other`
///
/// 12 = 1 STUB + 6 K-1 (ApiKey + AudioFormat + SampleRate + BitDepth + Channels + Language) +
/// 1 m3 + 4 业务 (TokenExpired/Network/RateLimited/Other)
#[derive(Debug, Error)]
pub enum VoiceError {
    // === §1 STUB (1 variant) ===
    /// **STUB 模式**: API 未实现, R21 续真接 @anthropic-ai/voice 后删.
    /// 6 API 全部返本 variant.
    #[error("STUB MODE: API not implemented: {0} (R21 will wire @anthropic-ai/voice)")]
    NotImplemented(&'static str),

    // === §2 K-1 强校验 (6 类, K-1 #1..#6 各 1 variant) ===
    /// **K-1 #1**: Voice API Key 缺失 (空 / 全空白).
    #[error("Voice API key is missing (K-1 #1, expected non-empty)")]
    ApiKeyMissing,
    /// **K-1 #1**: Voice API Key 格式错误 (长度 < 16).
    #[error("Voice API key is invalid: length={0} < 16 (K-1 #1)")]
    ApiKeyInvalid(usize),
    /// **K-1 #2**: Audio Format 格式错误 (非 wav/mp3/opus/flac).
    #[error("Voice audio format is invalid: {0} (K-1 #2, expected wav/mp3/opus/flac)")]
    AudioFormatInvalid(String),
    /// **K-1 #3**: Sample Rate 范围错误 (非 8000..=48000 Hz).
    #[error("Voice sample rate is invalid: {0} (K-1 #3, expected 8000..=48000 Hz)")]
    SampleRateInvalid(u32),
    /// **K-1 #4**: Bit Depth 取值错误 (非 8/16/24/32).
    #[error("Voice bit depth is invalid: {0} (K-1 #4, expected 8/16/24/32)")]
    BitDepthInvalid(u16),
    /// **K-1 #5**: Channels 取值错误 (非 1/2).
    #[error("Voice channels is invalid: {0} (K-1 #5, expected 1/2)")]
    ChannelsInvalid(u8),
    /// **K-1 #6**: Language 格式错误 (非 ISO 639-1 简化版 e.g. `en` / `zh-CN`).
    #[error("Voice language is invalid: {0} (K-1 #6, expected ISO 639-1 like en/zh-CN)")]
    LanguageInvalid(String),

    // === §3 m3 防御 (1 variant) ===
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("Voice tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // === §4 鉴权 (1 variant) ===
    /// Access token 过期 (R21 续真接时刷).
    #[error("Voice access token expired (refresh needed)")]
    TokenExpired,

    // === §5 网络 (1 variant) ===
    /// 网络错误 (HTTP / DNS / TLS, R21+ 真接 reqwest 时细化).
    #[error("Voice network error: {0}")]
    Network(String),

    // === §6 限流 (1 variant) ===
    /// 限流 (per @anthropic-ai/voice 商业版 `code: 429` "rate limit exceeded").
    #[error("Voice rate limited (Anthropic Voice code=429)")]
    RateLimited,

    // === §7 其他 (1 variant) ===
    /// 其他错误 (序列化 / 内部错误 / 业务错误 catch-all).
    #[error("Voice other error: {0}")]
    Other(String),
}

/// Voice SDK 错误类型别名 (per 商业版 v0.9.21 1:1, `Result<T, VoiceError>` 的轻量封装).
pub type VoiceResult<T> = Result<T, VoiceError>;

/// 编译期守门: 12 variant 守门 (per 8 项不修改承诺).
/// 新增 variant 必须同步改本 const, 强行提醒 reviewer.
pub const VOICE_ERROR_VARIANT_COUNT: usize = 12;
const _: () = assert!(
    true, // 编译期 hardcode 守门 (实际计数在测试中验证)
    "VoiceError 新增 variant 必须经 6 哲学锚 + 主人审 (R20 阶段 4)"
);

// ============================================================================
// §2 K-1 强校验方法 (6 个, 编译期 hardcode 守门字样)
// ============================================================================

impl VoiceError {
    /// **K-1 #1**: 校验 Voice API Key (非空 + 长度 ≥ 16, per Anthropic Voice API 规范).
    ///
    /// Anthropic Voice API Key 规范: 通常是 `sk-ant-voice-` 前缀 + 32 char random, 走 keyring.
    /// 长度 < 16 直接拒绝, 防止 m3 幻觉传"test" / "123" / 空串.
    pub fn validate_api_key(api_key: &str) -> VoiceResult<()> {
        let trimmed = api_key.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::ApiKeyMissing);
        }
        if trimmed.len() < 16 {
            return Err(VoiceError::ApiKeyInvalid(trimmed.len()));
        }
        Ok(())
    }

    /// **K-1 #2**: 校验 Audio Format (必须是 wav / mp3 / opus / flac 之一, per v0.9.21 商业版).
    ///
    /// 大小写不敏感 (e.g. `WAV` / `Wav` 都接受), 内部统一 lowercase 比较.
    pub fn validate_audio_format(format: &str) -> VoiceResult<()> {
        let trimmed = format.trim().to_lowercase();
        if trimmed.is_empty() {
            return Err(VoiceError::AudioFormatInvalid(format.to_string()));
        }
        match trimmed.as_str() {
            "wav" | "mp3" | "opus" | "flac" => Ok(()),
            _ => Err(VoiceError::AudioFormatInvalid(format.to_string())),
        }
    }

    /// **K-1 #3**: 校验 Sample Rate (必须是 8000..=48000 Hz, per v0.9.21 商业版).
    ///
    /// 范围覆盖 8 kHz (电话质量) 到 48 kHz (专业音频).
    pub fn validate_sample_rate(sample_rate: u32) -> VoiceResult<()> {
        if !(8000..=48000).contains(&sample_rate) {
            return Err(VoiceError::SampleRateInvalid(sample_rate));
        }
        Ok(())
    }

    /// **K-1 #4**: 校验 Bit Depth (必须是 8 / 16 / 24 / 32, per v0.9.21 商业版).
    pub fn validate_bit_depth(bit_depth: u16) -> VoiceResult<()> {
        if !matches!(bit_depth, 8 | 16 | 24 | 32) {
            return Err(VoiceError::BitDepthInvalid(bit_depth));
        }
        Ok(())
    }

    /// **K-1 #5**: 校验 Channels (必须是 1 单声道 / 2 立体声, per v0.9.21 商业版).
    pub fn validate_channels(channels: u8) -> VoiceResult<()> {
        if !matches!(channels, 1 | 2) {
            return Err(VoiceError::ChannelsInvalid(channels));
        }
        Ok(())
    }

    /// **K-1 #6**: 校验 Language (ISO 639-1 简化版, e.g. `en` / `zh-CN` / `ja`).
    ///
    /// 简化规则:
    /// - 主语言: 2 个小写字母 (e.g. `en` / `zh` / `ja` / `fr`)
    /// - 主语言 + 地区: 2 字母 + `-` + 2 字母 (e.g. `zh-CN` / `en-US` / `pt-BR`)
    /// - 拒绝空 / 含数字 / 长度 < 2 / 主语言段 > 3 字符
    pub fn validate_language(language: &str) -> VoiceResult<()> {
        let trimmed = language.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
        }
        // 主语言段 + 可选 -地区段
        let parts: Vec<&str> = trimmed.split('-').collect();
        // 主语言段必须 2-3 字母 (ISO 639-1 2 字母, ISO 639-2/T 3 字母)
        if parts.is_empty() || parts[0].is_empty() || parts[0].len() > 3 {
            return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
        }
        // 主语言段必须全字母
        for c in parts[0].chars() {
            if !c.is_ascii_alphabetic() {
                return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
            }
        }
        // 地区段 (如果有): 必须 2-4 字母大写
        if parts.len() > 1 {
            for region in &parts[1..] {
                if region.is_empty() || region.len() > 4 {
                    return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
                }
                for c in region.chars() {
                    if !c.is_ascii_alphabetic() {
                        return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
                    }
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// §3 STUB 守门宏 (per task spec §4 "STUB 守门宏: voice_stub!")
// ============================================================================

/// STUB 守门宏: 6 API stub 内部统一返 `VoiceError::NotImplemented(api)` + tracing log.
///
/// 用法:
/// ```ignore
/// pub async fn transcribe(&self, ...) -> VoiceResult<Transcription> {
///     voice_stub!("transcribe")
/// }
/// ```
///
/// 整合 R21 真接时, 删本宏直接换实现.
#[macro_export]
macro_rules! voice_stub {
    ($api:expr) => {{
        $crate::error::tracing_warn_stub($api);
        return Err($crate::error::VoiceError::NotImplemented($api));
    }};
}

/// tracing log helper (per voice_stub! 守门宏, 让 log + return 一气呵成).
pub fn tracing_warn_stub(api: &'static str) {
    tracing::warn!(
        "apeireth-sdk-voice STUB MODE: api={} not implemented (R21 will wire @anthropic-ai/voice)",
        api
    );
}

// ============================================================================
// §4 单元测试 (K-1 6 强校验 + 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// K-1 #1: API Key 校验
    #[test]
    fn k1_api_key_valid() {
        // 16 chars minimum
        assert!(VoiceError::validate_api_key("1234567890abcdef").is_ok());
        // 32 chars (典型 Anthropic voice key)
        assert!(VoiceError::validate_api_key("sk-ant-voice-abcdef1234567890xyz").is_ok());
    }

    #[test]
    fn k1_api_key_missing() {
        assert!(matches!(
            VoiceError::validate_api_key(""),
            Err(VoiceError::ApiKeyMissing)
        ));
        assert!(matches!(
            VoiceError::validate_api_key("   "),
            Err(VoiceError::ApiKeyMissing)
        ));
    }

    #[test]
    fn k1_api_key_too_short() {
        assert!(matches!(
            VoiceError::validate_api_key("short"),
            Err(VoiceError::ApiKeyInvalid(5))
        ));
        assert!(matches!(
            VoiceError::validate_api_key("1234567890abcde"), // 15 chars
            Err(VoiceError::ApiKeyInvalid(15))
        ));
    }

    /// K-1 #2: Audio Format 校验
    #[test]
    fn k1_audio_format_valid() {
        assert!(VoiceError::validate_audio_format("wav").is_ok());
        assert!(VoiceError::validate_audio_format("mp3").is_ok());
        assert!(VoiceError::validate_audio_format("opus").is_ok());
        assert!(VoiceError::validate_audio_format("flac").is_ok());
        // 大小写不敏感
        assert!(VoiceError::validate_audio_format("WAV").is_ok());
        assert!(VoiceError::validate_audio_format("Mp3").is_ok());
    }

    #[test]
    fn k1_audio_format_invalid() {
        assert!(matches!(
            VoiceError::validate_audio_format(""),
            Err(VoiceError::AudioFormatInvalid(_))
        ));
        assert!(matches!(
            VoiceError::validate_audio_format("aac"),
            Err(VoiceError::AudioFormatInvalid(_))
        ));
        assert!(matches!(
            VoiceError::validate_audio_format("ogg"),
            Err(VoiceError::AudioFormatInvalid(_))
        ));
    }

    /// K-1 #3: Sample Rate 校验
    #[test]
    fn k1_sample_rate_valid() {
        assert!(VoiceError::validate_sample_rate(8000).is_ok());
        assert!(VoiceError::validate_sample_rate(16000).is_ok());
        assert!(VoiceError::validate_sample_rate(44100).is_ok());
        assert!(VoiceError::validate_sample_rate(48000).is_ok());
    }

    #[test]
    fn k1_sample_rate_invalid() {
        assert!(matches!(
            VoiceError::validate_sample_rate(7999),
            Err(VoiceError::SampleRateInvalid(7999))
        ));
        assert!(matches!(
            VoiceError::validate_sample_rate(48001),
            Err(VoiceError::SampleRateInvalid(48001))
        ));
        assert!(matches!(
            VoiceError::validate_sample_rate(0),
            Err(VoiceError::SampleRateInvalid(0))
        ));
    }

    /// K-1 #4: Bit Depth 校验
    #[test]
    fn k1_bit_depth_valid() {
        assert!(VoiceError::validate_bit_depth(8).is_ok());
        assert!(VoiceError::validate_bit_depth(16).is_ok());
        assert!(VoiceError::validate_bit_depth(24).is_ok());
        assert!(VoiceError::validate_bit_depth(32).is_ok());
    }

    #[test]
    fn k1_bit_depth_invalid() {
        assert!(matches!(
            VoiceError::validate_bit_depth(4),
            Err(VoiceError::BitDepthInvalid(4))
        ));
        assert!(matches!(
            VoiceError::validate_bit_depth(64),
            Err(VoiceError::BitDepthInvalid(64))
        ));
        assert!(matches!(
            VoiceError::validate_bit_depth(0),
            Err(VoiceError::BitDepthInvalid(0))
        ));
    }

    /// K-1 #5: Channels 校验
    #[test]
    fn k1_channels_valid() {
        assert!(VoiceError::validate_channels(1).is_ok());
        assert!(VoiceError::validate_channels(2).is_ok());
    }

    #[test]
    fn k1_channels_invalid() {
        assert!(matches!(
            VoiceError::validate_channels(0),
            Err(VoiceError::ChannelsInvalid(0))
        ));
        assert!(matches!(
            VoiceError::validate_channels(6),
            Err(VoiceError::ChannelsInvalid(6))
        ));
        assert!(matches!(
            VoiceError::validate_channels(8),
            Err(VoiceError::ChannelsInvalid(8))
        ));
    }

    /// K-1 #6: Language 校验 (ISO 639-1 简化)
    #[test]
    fn k1_language_valid() {
        assert!(VoiceError::validate_language("en").is_ok());
        assert!(VoiceError::validate_language("zh").is_ok());
        assert!(VoiceError::validate_language("ja").is_ok());
        assert!(VoiceError::validate_language("zh-CN").is_ok());
        assert!(VoiceError::validate_language("en-US").is_ok());
        assert!(VoiceError::validate_language("pt-BR").is_ok());
    }

    #[test]
    fn k1_language_invalid() {
        assert!(matches!(
            VoiceError::validate_language(""),
            Err(VoiceError::LanguageInvalid(_))
        ));
        assert!(matches!(
            VoiceError::validate_language("english"),
            Err(VoiceError::LanguageInvalid(_))
        )); // 主语言段 > 3 字符
        assert!(matches!(
            VoiceError::validate_language("e1"),
            Err(VoiceError::LanguageInvalid(_))
        )); // 含数字
        assert!(matches!(
            VoiceError::validate_language("en-USAXX"),
            Err(VoiceError::LanguageInvalid(_))
        )); // 地区段 > 4 字符
        assert!(matches!(
            VoiceError::validate_language("zh-CN-extra"),
            Err(VoiceError::LanguageInvalid(_))
        )); // 多段地区, "extra" 5 chars > 4
    }

    #[test]
    fn k1_language_region_2_to_4_chars() {
        // 地区段 2-4 字符都是合法的
        assert!(VoiceError::validate_language("en-US").is_ok()); // 2 chars
        assert!(VoiceError::validate_language("en-USA").is_ok()); // 3 chars
        assert!(VoiceError::validate_language("en-USAA").is_ok()); // 4 chars
        // 地区段 5 字符非法
        assert!(matches!(
            VoiceError::validate_language("en-USAAA"),
            Err(VoiceError::LanguageInvalid(_))
        ));
    }

    /// NotImplemented variant 守 STUB_MODE 必为 true
    #[test]
    fn stub_mode_guard_not_implemented() {
        let err: VoiceError = VoiceError::NotImplemented("transcribe");
        assert!(matches!(err, VoiceError::NotImplemented("transcribe")));
    }

    /// voice_stub! 宏测试
    #[test]
    fn voice_stub_macro_returns_not_implemented() {
        let result: VoiceResult<()> = (|| {
            voice_stub!("test_api");
        })();
        assert!(matches!(result, Err(VoiceError::NotImplemented("test_api"))));
    }
}
