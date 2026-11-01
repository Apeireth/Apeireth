# apeireth-protocol

> Apeireth R17 战役 1-1: LLM 协议归一化层 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini), 字段级**自研实现**, 对齐业界通行 wire format 语义. R37-1 bridge facade (4 Bridge struct 砍 router 中间层) + R20 阶段 2 WS 8 帧协议 (AuthFrame/CloseFrame/ErrorFrame/Ping/StreamChunk/StreamEnd/ToolInvoke/ToolResult). src 目录当前 16 个条目 (acp / adapter / adapters/ / bridge / bridge_ext / canonical/ / error / gateway / lib / normalized / organ_kani_proofs / p2p_mesh / retry / usage / ws_session / ws_v1). 测试数字单一真源见 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md)（本文不抄写）。

apeireth-protocol 是 Apeireth v2 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../../docs/README.md)。
