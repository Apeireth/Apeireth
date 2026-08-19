# apeireth-pipeline

> Apeireth R17 战役 1-3 主 chat 管线 (借鉴 §6.2.2 #15/#17/#19/#20: token 预算三层 / placeholder 递归 / Force-Translate / 15s 抑制窗口) + R122 tiktoken (BPE 精确) + R126-1 Provider Registry (LiteLLM ⏳ 限流 = 准备) + R157 g5_chat_bridge (chat 5-step → pipeline-g5 5-stage). src 模块 14 个 (13 modules: force_translate / g5_chat_bridge / model_router / model_router_kani / organ_kani_proofs / placeholder / provider_registry / retry_suppression / role_divider / streaming / tiktoken_counter / token_budget / tool_loop). 测试数 (#[test]): 165 in-src + 20 集成 (tests/pipeline.rs wiremock e2e).

apeireth-pipeline 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
