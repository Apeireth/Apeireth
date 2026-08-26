# apeireth-tool-image-gen

> Apeireth image generation tool (ImageGenProvider trait, Mock + OpenAI DALL-E + Stability AI + MiniMax-Image providers, compatible adapter layer)

apeireth-tool-image-gen 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (10 src 文件 / 28 测试 + 2 Kani proof)

- `src/lib.rs` — 入口 re-export (ToolBridge 装配)
- `src/provider.rs` — ImageGenProvider trait + 4 测试
- `src/generators.rs` — Mock + OpenAI DALL-E + Stability AI + MiniMax-Image providers + ProviderRegistry + 2 测试
- `src/params.rs` — ImageGenParams (serde derive) + 4 测试
- `src/result.rs` — ImageGenResult / ImageArtifact + 2 测试
- `src/mcp.rs` — MCP server (2 工具: ImageGenerate/ListProviders) + 5 测试
- `src/register.rs` — ToolBridge catalog 接入 + 1 测试
- `src/compat.rs` — 兼容层 adapter + 3 测试
- `src/enhanced.rs` — enhanced 路径 (含可选 provider 路由) + 2 测试
- `src/organ_kani_proofs.rs` — image-gen organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
