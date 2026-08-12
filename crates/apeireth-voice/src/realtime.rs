//! # Apeireth Realtime Protocol
//!
//! OpenAI Realtime API protocol support - bidirectional streaming audio,
//! multimodal model dispatch, ephemeral token creation, function calling,
//! image input, and session lifecycle management.
//!
//! ## Borrowed Reference (per O-5)
//!
//! Inspired by the **OpenAI Realtime API** (GA 2024-12, protocol v1).
//! Realtime is OpenAI bidirectional WebSocket protocol for low-latency
//! speech-to-speech model interaction. We mirror its event schemas and
//! session lifecycle, with adaptations for local-first orchestration.
//!
//! Borrowed elements:
//! - 3 model dispatch: gpt-realtime / gpt-realtime-mini / gpt-4o-realtime
//! - 128K token context window (shared across all 3 models)
//! - Server-side VAD (voice activity detection, turn_detection config)
//! - WebSocket event protocol: session.update / input_audio_buffer.append /
//!   response.create / conversation.item.create / response.audio.delta
//! - Ephemeral tokens (server-minted, single-session, short-lived)
//! - Function calling via realtime response.tool_call
//! - Image input (multimodal conversation.item.input_image)
//!
//! 0 external dep - built on reqwest + tokio (already in workspace).
//! Audio codec (PCM16 / G.711 / Opus) handling lives above this layer.
//!
//! ## Self-host / remote dispatch
//!
//! The protocol is provider-agnostic at the type level. Real session I/O
//! is delegated to upstream providers (Apeireth own apeireth-api gateway,
//! or any OpenAI-Realtime-compatible endpoint). This module is the schema
//! + lifecycle + dispatch surface.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use thiserror::Error;


// ============================================================================
// §1 Compile-time hardcodes
// ============================================================================

/// Realtime protocol schema version. Mirrors OpenAI Realtime GA 2024-12.
pub const REALTIME_SCHEMA_VERSION: &str = "1";

/// Default base URL for ephemeral token endpoint (provider-agnostic).
pub const REALTIME_DEFAULT_TOKEN_ENDPOINT: &str = "/v1/realtime/sessions";

/// Default session TTL (ephemeral token lifetime, per OpenAI Realtime spec).
pub const REALTIME_DEFAULT_SESSION_TTL: Duration = Duration::from_secs(60 * 60);

/// Maximum session TTL (per OpenAI Realtime spec, 1 hour hard cap).
pub const REALTIME_MAX_SESSION_TTL: Duration = Duration::from_secs(60 * 60);

/// 128K token context window (shared across all 3 models, per spec).
pub const REALTIME_CONTEXT_WINDOW_TOKENS: u32 = 128_000;

/// Compile-time gate: 3 models in dispatch enum (no runtime extension).
pub const REALTIME_MODEL_COUNT: usize = 3;

// Constraint marker: token endpoint is /v1/realtime/sessions (v1 protocol).
// The literal is hardcoded in REALTIME_DEFAULT_TOKEN_ENDPOINT above; string
// comparison is non-const so we cannot enforce equality at compile time without
// unstable features (see rust-lang/rust#143874).

// ============================================================================
// §2 Error type
// ============================================================================

/// Realtime protocol errors (8 variants, per lark 1:1 pattern).
#[derive(Debug, Error)]
pub enum RealtimeError {
    /// Invalid session config (e.g. unknown model, empty tools).
    #[error("invalid realtime config: {0}")]
    InvalidConfig(String),
    /// Session not found or expired.
    #[error("realtime session not found or expired: {0}")]
    SessionExpired(String),
    /// Ephemeral token mint failed (HTTP / provider error).
    #[error("ephemeral token mint failed: {0}")]
    TokenMintFailed(String),
    /// Unsupported model dispatch.
    #[error("unsupported realtime model: {0}")]
    UnsupportedModel(String),
    /// Event schema violation (e.g. unknown event type).
    #[error("realtime event schema error: {0}")]
    EventSchema(String),
    /// Audio buffer size limit exceeded.
    #[error("audio buffer too large: got {got} bytes, max {max} bytes")]
    AudioBufferTooLarge { got: usize, max: usize },
    /// Image input size limit exceeded.
    #[error("image input too large: got {got} bytes, max {max} bytes")]
    ImageTooLarge { got: usize, max: usize },
    /// Provider returned non-zero status.
    #[error("realtime provider error: status={status}, body={body}")]
    Provider { status: u16, body: String },
}

pub type RealtimeResult<T> = Result<T, RealtimeError>;


// ============================================================================
// §3 Model dispatch (3-model enum)
// ============================================================================

/// 3-model realtime dispatch. Compile-time hardcode; no runtime extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeModel {
    /// gpt-realtime - flagship multimodal realtime model (128K context).
    #[serde(rename = "gpt-realtime")]
    GptRealtime,
    /// gpt-realtime-mini - cost-optimized realtime model (128K context).
    #[serde(rename = "gpt-realtime-mini")]
    GptRealtimeMini,
    /// gpt-4o-realtime - legacy 4o realtime (retained for back-compat).
    #[serde(rename = "gpt-4o-realtime")]
    Gpt4oRealtime,
}

impl RealtimeModel {
    /// Wire identifier (per OpenAI Realtime API model field).
    pub fn as_str(&self) -> &'static str {
        match self {
            RealtimeModel::GptRealtime => "gpt-realtime",
            RealtimeModel::GptRealtimeMini => "gpt-realtime-mini",
            RealtimeModel::Gpt4oRealtime => "gpt-4o-realtime",
        }
    }

    /// Per-model max output token cap (128K context window minus prompt reservation).
    pub fn max_output_tokens(&self) -> u32 {
        match self {
            RealtimeModel::GptRealtime => 4_096,
            RealtimeModel::GptRealtimeMini => 4_096,
            RealtimeModel::Gpt4oRealtime => 4_096,
        }
    }

    /// Per-model audio sample rate (Hz).
    pub fn audio_sample_rate_hz(&self) -> u32 {
        match self {
            RealtimeModel::GptRealtime => 24_000,
            RealtimeModel::GptRealtimeMini => 24_000,
            RealtimeModel::Gpt4oRealtime => 24_000,
        }
    }

    /// Per-model cost tier (1=cheapest, 3=flagship).
    pub fn cost_tier(&self) -> u8 {
        match self {
            RealtimeModel::GptRealtimeMini => 1,
            RealtimeModel::Gpt4oRealtime => 2,
            RealtimeModel::GptRealtime => 3,
        }
    }
}

impl Default for RealtimeModel {
    fn default() -> Self {
        RealtimeModel::GptRealtime
    }
}

impl fmt::Display for RealtimeModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Compile-time gate: exactly 3 models in dispatch enum.
pub const SUPPORTED_REALTIME_MODELS: &[RealtimeModel] = &[
    RealtimeModel::GptRealtime,
    RealtimeModel::GptRealtimeMini,
    RealtimeModel::Gpt4oRealtime,
];
const _: () = assert!(SUPPORTED_REALTIME_MODELS.len() == REALTIME_MODEL_COUNT);

// ============================================================================
// §4 Voice / modality config
// ============================================================================

/// Output voice (per OpenAI Realtime API voice field).
/// 6 voices - alloy / ash / coral / echo / sage / verse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RealtimeVoice {
    Alloy,
    Ash,
    Coral,
    Echo,
    Sage,
    Verse,
}

impl RealtimeVoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            RealtimeVoice::Alloy => "alloy",
            RealtimeVoice::Ash => "ash",
            RealtimeVoice::Coral => "coral",
            RealtimeVoice::Echo => "echo",
            RealtimeVoice::Sage => "sage",
            RealtimeVoice::Verse => "verse",
        }
    }
}

impl Default for RealtimeVoice {
    fn default() -> Self {
        RealtimeVoice::Alloy
    }
}

pub const SUPPORTED_REALTIME_VOICES: &[RealtimeVoice] = &[
    RealtimeVoice::Alloy,
    RealtimeVoice::Ash,
    RealtimeVoice::Coral,
    RealtimeVoice::Echo,
    RealtimeVoice::Sage,
    RealtimeVoice::Verse,
];
const _: () = assert!(SUPPORTED_REALTIME_VOICES.len() == 6);

/// Modality (input/output audio + text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RealtimeModality {
    Text,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeModalities {
    pub input: Vec<RealtimeModality>,
    pub output: Vec<RealtimeModality>,
}

impl Default for RealtimeModalities {
    fn default() -> Self {
        Self {
            input: vec![RealtimeModality::Text, RealtimeModality::Audio],
            output: vec![RealtimeModality::Audio, RealtimeModality::Text],
        }
    }
}

/// Audio format for input/output (per OpenAI Realtime audio.format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeAudioFormat {
    /// PCM16, 24kHz, mono (default).
    Pcm16,
    /// G.711 u-law, 8kHz (telephony).
    G711Ulaw,
    /// G.711 A-law, 8kHz (telephony EU).
    G711Alaw,
}

impl Default for RealtimeAudioFormat {
    fn default() -> Self {
        RealtimeAudioFormat::Pcm16
    }
}

// ============================================================================
// §5 Turn detection / VAD
// ============================================================================

/// Server-side VAD (voice activity detection) configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnDetection {
    /// VAD type (per OpenAI Realtime turn_detection.type).
    #[serde(rename = "type")]
    pub kind: TurnDetectionKind,
    /// Activation threshold (0.0~1.0, default 0.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
    /// Silence duration to trigger turn end (ms, default 500).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<u32>,
    /// Prefix padding (ms, audio included before speech start, default 300).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnDetectionKind {
    /// Server-side VAD (default for realtime).
    ServerVad,
    /// Disabled - manual turn management via input_audio_buffer.commit.
    Disabled,
}

impl Default for TurnDetection {
    fn default() -> Self {
        Self {
            kind: TurnDetectionKind::ServerVad,
            threshold: Some(0.5),
            silence_duration_ms: Some(500),
            prefix_padding_ms: Some(300),
        }
    }
}

// ============================================================================
// §6 Function tool definition (for realtime function calling)
// ============================================================================

/// Function tool exposed to the realtime model (per OpenAI tools[]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeTool {
    /// Tool type (only function supported in realtime GA).
    #[serde(rename = "type")]
    pub kind: String,
    /// Tool name (must match [a-zA-Z0-9_-]{1,64}).
    pub name: String,
    /// Human-readable description (model uses to decide when to call).
    pub description: String,
    /// JSON schema for tool parameters.
    pub parameters: serde_json::Value,
}

impl RealtimeTool {
    /// Create a function tool with auto-set type field.
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            kind: "function".to_string(),
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

// ============================================================================
// §7 Session config (builder pattern)
// ============================================================================

/// Realtime session configuration (per OpenAI Realtime session.update event).
///
/// Use the builder pattern via RealtimeSessionConfig::new and chainable
/// setters. Default: gpt-realtime model, alloy voice, server VAD, audio+text modalities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionConfig {
    pub model: RealtimeModel,
    pub voice: RealtimeVoice,
    pub modalities: RealtimeModalities,
    pub audio_format: RealtimeAudioFormat,
    pub turn_detection: TurnDetection,
    pub tools: Vec<RealtimeTool>,
    pub tool_choice: Option<String>,
    pub instructions: Option<String>,
    pub temperature: f32,
    pub max_output_tokens: u32,
    /// Session TTL (ephemeral token lifetime, default 1h, max 1h).
    pub session_ttl: Duration,
    /// Custom metadata (provider-agnostic).
    pub metadata: HashMap<String, String>,
}

impl RealtimeSessionConfig {
    /// Default config builder (gpt-realtime, alloy, server VAD, audio+text).
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, m: RealtimeModel) -> Self {
        self.model = m;
        self
    }

    pub fn voice(mut self, v: RealtimeVoice) -> Self {
        self.voice = v;
        self
    }

    pub fn instructions(mut self, s: impl Into<String>) -> Self {
        self.instructions = Some(s.into());
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = t.clamp(0.0, 2.0);
        self
    }

    pub fn max_output_tokens(mut self, n: u32) -> Self {
        self.max_output_tokens = n.min(self.model.max_output_tokens());
        self
    }

    pub fn add_tool(mut self, tool: RealtimeTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn tool_choice_auto(mut self) -> Self {
        self.tool_choice = Some("auto".to_string());
        self
    }

    pub fn session_ttl(mut self, d: Duration) -> Self {
        self.session_ttl = d.min(REALTIME_MAX_SESSION_TTL);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Validate config. Returns Err(InvalidConfig) for invalid combinations.
    pub fn validate(&self) -> RealtimeResult<()> {
        if self.tools.len() > 128 {
            return Err(RealtimeError::InvalidConfig(format!(
                "too many tools: {} (max 128)",
                self.tools.len()
            )));
        }
        for tool in &self.tools {
            if tool.name.is_empty() || tool.name.len() > 64 {
                return Err(RealtimeError::InvalidConfig(format!(
                    "tool name must be 1..=64 chars: {}",
                    tool.name
                )));
            }
            if tool.kind != "function" {
                return Err(RealtimeError::InvalidConfig(format!(
                    "only function tool type supported: got {}",
                    tool.kind
                )));
            }
        }
        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err(RealtimeError::InvalidConfig(format!(
                "temperature out of range: {}",
                self.temperature
            )));
        }
        if self.session_ttl > REALTIME_MAX_SESSION_TTL {
            return Err(RealtimeError::InvalidConfig(format!(
                "session_ttl exceeds max: {:?} > {:?}",
                self.session_ttl, REALTIME_MAX_SESSION_TTL
            )));
        }
        Ok(())
    }
}

impl Default for RealtimeSessionConfig {
    fn default() -> Self {
        Self {
            model: RealtimeModel::default(),
            voice: RealtimeVoice::default(),
            modalities: RealtimeModalities::default(),
            audio_format: RealtimeAudioFormat::default(),
            turn_detection: TurnDetection::default(),
            tools: Vec::new(),
            tool_choice: Some("auto".to_string()),
            instructions: None,
            temperature: 0.8,
            max_output_tokens: RealtimeModel::default().max_output_tokens(),
            session_ttl: REALTIME_DEFAULT_SESSION_TTL,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// §8 Ephemeral token (server-minted, single-session)
// ============================================================================

/// Ephemeral token returned by POST /v1/realtime/sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralToken {
    /// Opaque token string (e.g. ek_...).
    pub value: String,
    /// Token expiry (absolute SystemTime).
    pub expires_at: SystemTime,
    /// Associated session ID (provider-assigned).
    pub session_id: String,
    /// Model bound to this token.
    pub model: RealtimeModel,
    /// Token TTL (relative Duration).
    pub ttl: Duration,
}

impl EphemeralToken {
    /// Check if token is expired (relative to now).
    pub fn is_expired(&self, now: SystemTime) -> bool {
        self.expires_at <= now
    }

    /// Remaining validity (None if expired).
    pub fn remaining(&self, now: SystemTime) -> Option<Duration> {
        self.expires_at.duration_since(now).ok()
    }
}

/// Request body for ephemeral token mint (POST /v1/realtime/sessions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralTokenRequest {
    pub model: RealtimeModel,
    pub voice: RealtimeVoice,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ttl_seconds: Option<u64>,
}

impl From<&RealtimeSessionConfig> for EphemeralTokenRequest {
    fn from(cfg: &RealtimeSessionConfig) -> Self {
        Self {
            model: cfg.model,
            voice: cfg.voice,
            instructions: cfg.instructions.clone(),
            session_ttl_seconds: Some(cfg.session_ttl.as_secs()),
        }
    }
}

// ============================================================================
// §9 Realtime events (WebSocket protocol schema)
// ============================================================================

/// Server -> client events (subset - full protocol has 30+; we expose the 10 most useful).
/// Per OpenAI Realtime protocol v1: event type uses dotted notation (e.g. `session.created`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    /// Session created successfully (sent on connect).
    #[serde(rename = "session.created")]
    SessionCreated {
        session_id: String,
        model: RealtimeModel,
        expires_at: SystemTime,
    },
    /// Session updated (echo of session.update).
    #[serde(rename = "session.updated")]
    SessionUpdated { session_id: String },
    /// Audio delta (incremental PCM16 audio chunk).
    #[serde(rename = "response.audio.delta")]
    AudioDelta {
        item_id: String,
        content_index: u32,
        delta: Vec<u8>,
    },
    /// Audio done (full audio chunk complete).
    #[serde(rename = "response.audio.done")]
    AudioDone { item_id: String },
    /// Text delta (transcript / response text).
    #[serde(rename = "response.text.delta")]
    TextDelta {
        item_id: String,
        content_index: u32,
        delta: String,
    },
    /// Function call from model.
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCall {
        item_id: String,
        call_id: String,
        name: String,
        arguments: String,
    },
    /// Speech started (server VAD detected speech).
    #[serde(rename = "input_audio_buffer.speech_started")]
    SpeechStarted { item_id: String, audio_start_ms: u32 },
    /// Speech stopped (server VAD detected silence).
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    SpeechStopped { item_id: String, audio_end_ms: u32 },
    /// Error from server.
    #[serde(rename = "error")]
    Error { code: String, message: String },
    /// Conversation interrupted (user spoke over assistant).
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    Interrupted { item_id: String, audio_end_ms: u32 },
}

/// Client -> server events (subset - 8 most common).
/// Per OpenAI Realtime protocol v1: event type uses dotted notation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    /// Update session config mid-stream.
    #[serde(rename = "session.update")]
    SessionUpdate { config: RealtimeSessionConfig },
    /// Append audio to input buffer.
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend { audio: Vec<u8> },
    /// Commit input buffer (force model turn).
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit,
    /// Clear input buffer.
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear,
    /// Create response (force model to respond).
    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
    },
    /// Add conversation item (text / image / function result).
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate { item: ConversationItem },
    /// Truncate conversation item (per OpenAI realtime spec).
    #[serde(rename = "conversation.item.truncate")]
    ConversationItemTruncate {
        item_id: String,
        content_index: u32,
        audio_end_ms: u32,
    },
}

/// Conversation item (text / image / function result / function call).
/// Per OpenAI Realtime protocol v1: type uses dotted notation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConversationItem {
    /// User text message (per OpenAI Realtime: audio messages go through
    /// `InputAudioBufferAppend` + `ConversationItemCreate`, not a separate type).
    #[serde(rename = "message")]
    UserMessage {
        role: String,
        content: String,
    },
    /// User image input (multimodal, base64 PNG/JPEG).
    #[serde(rename = "conversation.item.input_image")]
    InputImage {
        role: String,
        /// Base64-encoded image data (PNG / JPEG / WEBP).
        image_base64: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Function tool call result.
    #[serde(rename = "conversation.item.function_call_output")]
    FunctionCallOutput {
        call_id: String,
        output: String,
    },
}

// ============================================================================
// §10 Image input helper (multimodal) + base64 encoder
// ============================================================================

/// Maximum image input size (5 MB, per OpenAI Realtime spec).
pub const REALTIME_MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Maximum audio buffer size per InputAudioBufferAppend (15 MB, per spec).
pub const REALTIME_MAX_AUDIO_BUFFER_BYTES: usize = 15 * 1024 * 1024;

/// Encode image bytes as base64 multimodal conversation item.
pub fn encode_image_input(image_bytes: &[u8]) -> RealtimeResult<ConversationItem> {
    if image_bytes.is_empty() {
        return Err(RealtimeError::ImageTooLarge {
            got: 0,
            max: REALTIME_MAX_IMAGE_BYTES,
        });
    }
    if image_bytes.len() > REALTIME_MAX_IMAGE_BYTES {
        return Err(RealtimeError::ImageTooLarge {
            got: image_bytes.len(),
            max: REALTIME_MAX_IMAGE_BYTES,
        });
    }
    let encoded = simple_base64_encode(image_bytes);
    Ok(ConversationItem::InputImage {
        role: "user".to_string(),
        image_base64: encoded,
        detail: Some("auto".to_string()),
    })
}

/// Encode audio bytes for input buffer (validates size).
pub fn encode_audio_append(audio_bytes: &[u8]) -> RealtimeResult<ClientEvent> {
    if audio_bytes.is_empty() {
        return Err(RealtimeError::AudioBufferTooLarge {
            got: 0,
            max: REALTIME_MAX_AUDIO_BUFFER_BYTES,
        });
    }
    if audio_bytes.len() > REALTIME_MAX_AUDIO_BUFFER_BYTES {
        return Err(RealtimeError::AudioBufferTooLarge {
            got: audio_bytes.len(),
            max: REALTIME_MAX_AUDIO_BUFFER_BYTES,
        });
    }
    Ok(ClientEvent::InputAudioBufferAppend {
        audio: audio_bytes.to_vec(),
    })
}

// ============================================================================
// §11 Minimal base64 encoder (RFC 4648 standard, no external dep)
// ============================================================================
//
// 0 external dep ponytail ceiling:
// Base64 is RFC 4648 standard; trivial inline impl avoids base64 crate (340KB).
// Tests below verify correctness against canonical RFC 4648 §10 fixtures.

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn simple_base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let b0 = input[i];
        let b1 = input[i + 1];
        let b2 = input[i + 2];
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(BASE64_ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[(triple & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let b0 = input[i];
        let triple = (b0 as u32) << 16;
        out.push(BASE64_ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = input[i];
        let b1 = input[i + 1];
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8);
        out.push(BASE64_ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(BASE64_ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

// ============================================================================
// §12 Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_count_compile_time() {
        assert_eq!(SUPPORTED_REALTIME_MODELS.len(), 3);
        assert_eq!(REALTIME_MODEL_COUNT, 3);
    }

    #[test]
    fn test_model_dispatch_strings() {
        assert_eq!(RealtimeModel::GptRealtime.as_str(), "gpt-realtime");
        assert_eq!(
            RealtimeModel::GptRealtimeMini.as_str(),
            "gpt-realtime-mini"
        );
        assert_eq!(RealtimeModel::Gpt4oRealtime.as_str(), "gpt-4o-realtime");
    }

    #[test]
    fn test_model_max_output_tokens_capped() {
        for m in SUPPORTED_REALTIME_MODELS {
            assert!(m.max_output_tokens() <= REALTIME_CONTEXT_WINDOW_TOKENS);
        }
    }

    #[test]
    fn test_model_audio_sample_rate_uniform() {
        for m in SUPPORTED_REALTIME_MODELS {
            assert_eq!(m.audio_sample_rate_hz(), 24_000);
        }
    }

    #[test]
    fn test_cost_tier_ordering() {
        // mini < 4o < flagship
        assert!(
            RealtimeModel::GptRealtimeMini.cost_tier()
                < RealtimeModel::Gpt4oRealtime.cost_tier()
        );
        assert!(
            RealtimeModel::Gpt4oRealtime.cost_tier() < RealtimeModel::GptRealtime.cost_tier()
        );
    }

    #[test]
    fn test_voice_count() {
        assert_eq!(SUPPORTED_REALTIME_VOICES.len(), 6);
    }

    #[test]
    fn test_voice_default_alloy() {
        assert_eq!(RealtimeVoice::default(), RealtimeVoice::Alloy);
        assert_eq!(RealtimeVoice::Alloy.as_str(), "alloy");
    }

    #[test]
    fn test_session_config_default() {
        let cfg = RealtimeSessionConfig::default();
        assert_eq!(cfg.model, RealtimeModel::GptRealtime);
        assert_eq!(cfg.voice, RealtimeVoice::Alloy);
        assert_eq!(cfg.audio_format, RealtimeAudioFormat::Pcm16);
        assert_eq!(cfg.turn_detection.kind, TurnDetectionKind::ServerVad);
        assert!(cfg.tools.is_empty());
        assert_eq!(cfg.tool_choice.as_deref(), Some("auto"));
        assert_eq!(cfg.session_ttl, REALTIME_DEFAULT_SESSION_TTL);
    }

    #[test]
    fn test_session_config_builder() {
        let cfg = RealtimeSessionConfig::new()
            .model(RealtimeModel::GptRealtimeMini)
            .voice(RealtimeVoice::Sage)
            .instructions("You are a helpful voice assistant.")
            .temperature(0.7)
            .add_tool(RealtimeTool::function(
                "get_weather",
                "Get current weather for a city",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" }
                    },
                    "required": ["city"]
                }),
            ))
            .metadata("session_tag", "voice_smoke_test");

        assert_eq!(cfg.model, RealtimeModel::GptRealtimeMini);
        assert_eq!(cfg.voice, RealtimeVoice::Sage);
        assert_eq!(cfg.tools.len(), 1);
        assert_eq!(cfg.tools[0].name, "get_weather");
        assert_eq!(
            cfg.metadata.get("session_tag").map(|s| s.as_str()),
            Some("voice_smoke_test")
        );
    }

    #[test]
    fn test_session_config_validate_ok() {
        let cfg = RealtimeSessionConfig::new()
            .add_tool(RealtimeTool::function("test", "test tool", serde_json::json!({})));
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_session_config_validate_too_many_tools() {
        let mut cfg = RealtimeSessionConfig::new();
        for i in 0..129 {
            cfg = cfg.add_tool(RealtimeTool::function(
                format!("t_{}", i),
                "x",
                serde_json::json!({}),
            ));
        }
        assert!(matches!(
            cfg.validate(),
            Err(RealtimeError::InvalidConfig(_))
        ));
    }

    #[test]
    fn test_session_config_validate_tool_name_length() {
        let cfg = RealtimeSessionConfig::new().add_tool(RealtimeTool::function(
            "",
            "empty name",
            serde_json::json!({}),
        ));
        assert!(matches!(
            cfg.validate(),
            Err(RealtimeError::InvalidConfig(_))
        ));

        let long_name = "x".repeat(65);
        let cfg = RealtimeSessionConfig::new().add_tool(RealtimeTool::function(
            long_name,
            "too long",
            serde_json::json!({}),
        ));
        assert!(matches!(
            cfg.validate(),
            Err(RealtimeError::InvalidConfig(_))
        ));
    }

    #[test]
    fn test_session_config_temperature_clamp() {
        let cfg = RealtimeSessionConfig::new().temperature(5.0);
        assert!(cfg.temperature <= 2.0);
        let cfg = RealtimeSessionConfig::new().temperature(-1.0);
        assert!(cfg.temperature >= 0.0);
    }

    #[test]
    fn test_session_ttl_max_cap() {
        let cfg =
            RealtimeSessionConfig::new().session_ttl(Duration::from_secs(7 * 60 * 60)); // 7h, exceeds 1h cap
        assert!(cfg.session_ttl <= REALTIME_MAX_SESSION_TTL);
    }

    #[test]
    fn test_ephemeral_token_expiry() {
        let now = SystemTime::now();
        let token = EphemeralToken {
            value: "ek_test".to_string(),
            expires_at: now + Duration::from_secs(60),
            session_id: "sess_123".to_string(),
            model: RealtimeModel::GptRealtime,
            ttl: Duration::from_secs(60),
        };
        assert!(!token.is_expired(now));
        assert!(token.is_expired(now + Duration::from_secs(120)));
        assert!(token.remaining(now).is_some());
        assert!(token.remaining(now + Duration::from_secs(120)).is_none());
    }

    #[test]
    fn test_ephemeral_token_request_from_config() {
        let cfg = RealtimeSessionConfig::new()
            .model(RealtimeModel::GptRealtimeMini)
            .voice(RealtimeVoice::Echo)
            .instructions("test");
        let req: EphemeralTokenRequest = (&cfg).into();
        assert_eq!(req.model, RealtimeModel::GptRealtimeMini);
        assert_eq!(req.voice, RealtimeVoice::Echo);
        assert_eq!(req.instructions.as_deref(), Some("test"));
        assert_eq!(
            req.session_ttl_seconds,
            Some(REALTIME_DEFAULT_SESSION_TTL.as_secs())
        );
    }

    #[test]
    fn test_server_event_session_created_serde() {
        let ev = ServerEvent::SessionCreated {
            session_id: "sess_1".to_string(),
            model: RealtimeModel::GptRealtime,
            expires_at: SystemTime::UNIX_EPOCH,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"session.created\""));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ServerEvent::SessionCreated { .. }));
    }

    #[test]
    fn test_server_event_audio_delta_serde() {
        let ev = ServerEvent::AudioDelta {
            item_id: "item_1".to_string(),
            content_index: 0,
            delta: vec![0x00, 0x01, 0x02],
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"response.audio.delta\""));
        assert!(json.contains("\"item_id\":\"item_1\""));
    }

    #[test]
    fn test_server_event_function_call_serde() {
        let ev = ServerEvent::FunctionCall {
            item_id: "item_1".to_string(),
            call_id: "call_1".to_string(),
            name: "get_weather".to_string(),
            arguments: "{\"city\":\"Beijing\"}".to_string(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"response.function_call_arguments.done\""));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        if let ServerEvent::FunctionCall { name, .. } = parsed {
            assert_eq!(name, "get_weather");
        } else {
            panic!("expected FunctionCall");
        }
    }

    #[test]
    fn test_client_event_session_update_serde() {
        let cfg = RealtimeSessionConfig::new();
        let ev = ClientEvent::SessionUpdate { config: cfg.clone() };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"session.update\""));
        let parsed: ClientEvent = serde_json::from_str(&json).unwrap();
        if let ClientEvent::SessionUpdate { config } = parsed {
            assert_eq!(config, cfg);
        } else {
            panic!("expected SessionUpdate");
        }
    }

    #[test]
    fn test_client_event_response_create_serde() {
        let ev = ClientEvent::ResponseCreate {
            instructions: Some("answer briefly".to_string()),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"type\":\"response.create\""));
    }

    #[test]
    fn test_conversation_item_user_message_serde() {
        let item = ConversationItem::UserMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"message\""));
    }

    #[test]
    fn test_conversation_item_input_image_serde() {
        let item = ConversationItem::InputImage {
            role: "user".to_string(),
            image_base64: "iVBORw0KGgo=".to_string(),
            detail: Some("high".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"conversation.item.input_image\""));
        assert!(json.contains("\"detail\":\"high\""));
    }

    #[test]
    fn test_encode_audio_append_ok() {
        let audio = vec![0u8; 100];
        let ev = encode_audio_append(&audio).unwrap();
        if let ClientEvent::InputAudioBufferAppend { audio: a } = ev {
            assert_eq!(a.len(), 100);
        } else {
            panic!("expected InputAudioBufferAppend");
        }
    }

    #[test]
    fn test_encode_audio_append_empty_rejected() {
        assert!(matches!(
            encode_audio_append(&[]),
            Err(RealtimeError::AudioBufferTooLarge { .. })
        ));
    }

    #[test]
    fn test_encode_audio_append_oversize_rejected() {
        let huge = vec![0u8; REALTIME_MAX_AUDIO_BUFFER_BYTES + 1];
        assert!(matches!(
            encode_audio_append(&huge),
            Err(RealtimeError::AudioBufferTooLarge { .. })
        ));
    }

    #[test]
    fn test_encode_image_input_ok() {
        let img = vec![0u8; 1024];
        let item = encode_image_input(&img).unwrap();
        if let ConversationItem::InputImage {
            image_base64,
            detail,
            ..
        } = item
        {
            assert!(!image_base64.is_empty());
            assert_eq!(detail.as_deref(), Some("auto"));
        } else {
            panic!("expected InputImage");
        }
    }

    #[test]
    fn test_encode_image_input_empty_rejected() {
        assert!(matches!(
            encode_image_input(&[]),
            Err(RealtimeError::ImageTooLarge { .. })
        ));
    }

    #[test]
    fn test_encode_image_input_oversize_rejected() {
        let huge = vec![0u8; REALTIME_MAX_IMAGE_BYTES + 1];
        assert!(matches!(
            encode_image_input(&huge),
            Err(RealtimeError::ImageTooLarge { .. })
        ));
    }

    #[test]
    fn test_simple_base64_encode_empty() {
        assert_eq!(simple_base64_encode(&[]), "");
    }

    #[test]
    fn test_simple_base64_encode_one_byte() {
        // "f" -> "Zg=="
        assert_eq!(simple_base64_encode(b"f"), "Zg==");
    }

    #[test]
    fn test_simple_base64_encode_two_bytes() {
        // "fo" -> "Zm8="
        assert_eq!(simple_base64_encode(b"fo"), "Zm8=");
    }

    #[test]
    fn test_simple_base64_encode_three_bytes() {
        // "foo" -> "Zm9v"
        assert_eq!(simple_base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_simple_base64_encode_canonical_fixtures() {
        // RFC 4648 §10 test vectors
        assert_eq!(simple_base64_encode(b""), "");
        assert_eq!(simple_base64_encode(b"f"), "Zg==");
        assert_eq!(simple_base64_encode(b"fo"), "Zm8=");
        assert_eq!(simple_base64_encode(b"foo"), "Zm9v");
        assert_eq!(simple_base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(simple_base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(simple_base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_simple_base64_encode_longer() {
        // "Hello, World!" -> "SGVsbG8sIFdvcmxkIQ=="
        assert_eq!(
            simple_base64_encode(b"Hello, World!"),
            "SGVsbG8sIFdvcmxkIQ=="
        );
    }

    #[test]
    fn test_session_config_serde_roundtrip() {
        let cfg = RealtimeSessionConfig::new()
            .model(RealtimeModel::GptRealtime)
            .voice(RealtimeVoice::Coral)
            .instructions("Be brief");
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RealtimeSessionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn test_tool_function_helper() {
        let tool = RealtimeTool::function(
            "search",
            "Search the web",
            serde_json::json!({"type": "object", "properties": {}}),
        );
        assert_eq!(tool.kind, "function");
        assert_eq!(tool.name, "search");
    }
}

