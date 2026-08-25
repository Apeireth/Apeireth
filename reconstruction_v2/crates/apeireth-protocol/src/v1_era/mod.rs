//! v1 era module — apeireth-protocol v1 transcription (R17 era files)
//!
//! Transcribed from `crates/_archived/v1.0-legacy/apeireth-protocol/src/`
//! preserving v1 structure and behavior. v2 protocol modules (normalized,
//! adapters, ws, voice) coexist alongside these.

pub mod adapter;
pub mod adapters;
pub mod bridge;
pub mod bridge_ext;
pub mod error;
pub mod gateway;
pub mod normalized;
pub mod ws_v1;

pub use adapter::ProtocolAdapter;
pub use bridge::{
    AnthropicMessagesBridge, GeminiBridge, OpenAiChatBridge, OpenAiResponsesBridge, ProtocolBridge,
    decode_for_kind, encode_for_kind, endpoint_path_for_kind,
};
pub use error::ProtocolError;
pub use gateway::{ProtocolGateway, ProtocolKind};
pub use normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedToolChoice, NormalizedUsage, ToolCall,
};
pub use ws_v1::WsFrame as V1WsFrame;
