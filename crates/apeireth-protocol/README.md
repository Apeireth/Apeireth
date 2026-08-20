# apeireth-protocol

> Apeireth R17 战役 1-1: LLM 协议归一化层 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini), 字段级借鉴开源 protocol-bridge 真代码. R37-1 bridge facade (4 Bridge struct 砍 router 中间层) + R20 阶段 2 WS 8 帧协议 (AuthFrame/CloseFrame/ErrorFrame/Ping/StreamChunk/StreamEnd/ToolInvoke/ToolResult). src 模块 9 个 (lib + adapter + adapters/ + bridge + bridge_ext + error + gateway + normalized + organ_kani_proofs + ws_v1). 测试数 (#[test]): 51 in-src + 27 集成 (tests/wire_format.rs + tests/wire_format_ext.rs, 4 协议 golden + 边界).

apeireth-protocol 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
