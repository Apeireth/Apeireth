//! 4 协议 adapter

pub mod anthropic_messages;
pub mod gemini;
pub mod openai_chat;
pub mod openai_responses;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use gemini::GeminiAdapter;
pub use openai_chat::OpenAiChatAdapter;
pub use openai_responses::OpenAiResponsesAdapter;
