# apeireth-llm-iface

> R179 P0-3: LLM 抽象接口 (ChatMessage / LlmRequest / LlmProvider / LlmError). 拆 apeireth-memory <-> apeireth-api 编译期边. src 模块 3 个: error / lib / traits. 测试数(单测标注): **17** (per 2026-08-19 zero-test P0 audit 落地, 0 装 PASS 严守: 8 硬墙里 A3 13 键 verdict cache 跟 LlmError 的 retryable 判定强相关, retrier 必须严格按 is_retryable() 决定是否重试 AuthFailed/Config 这种永久错 — 所以这 3 个方法 0 假装测齐). 接口 crate 0 业务逻辑, 但 retrier contract 强相关, 测这 17 个 is_retryable / backoff / provider / builder / capability / mock 合约 0 假装严守.

apeireth-llm-iface 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
