//! fuzz: 模型回程解码面（`decode_for_kind`）。
//!
//! **面**：4 个 HTTP LLM 协议的 decode 入口。输入是**外部网络回来的 JSON**——
//! 这是整个系统最不受控的一段字节流。
//!
//! **判据**：不得 panic。允许（且期望）返回结构化 `ProtocolError`：
//! 「解析失败」是正常结果，「解析崩溃」才是缺陷。
#![no_main]

use apeireth_protocol::bridge::decode_for_kind;
use apeireth_protocol::gateway::ProtocolKind;
use libfuzzer_sys::fuzz_target;

const KINDS: [ProtocolKind; 7] = [
    ProtocolKind::OpenAiChat,
    ProtocolKind::OpenAiResponses,
    ProtocolKind::AnthropicMessages,
    ProtocolKind::Gemini,
    ProtocolKind::Acp,
    ProtocolKind::Mcp,
    ProtocolKind::OpenClawGateway,
];

fuzz_target!(|data: &[u8]| {
    // 先过 serde_json（这一步本身也不该 panic）
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) else {
        return;
    };
    for kind in KINDS {
        // 只要不 panic 就算通过；错误结果是有意义的正常输出
        let _ = decode_for_kind(kind, &value);
    }
});
