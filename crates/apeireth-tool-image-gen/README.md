# apeireth-tool-image-gen

**R156** - Image generation tool (lint cleanup)

## 职责

Apeireth 统一 image-gen 入口, dispatch 4 provider:
- MockProvider (R141)
- OpenAiDallEProvider (OpenAI 兼容)
- StabilityAiProvider (Stability 兼容)
- MiniMaxImageProvider (本地 apikey: C:\\Users\\REDACTED\\.openclaw\\apikey.txt)

## 核心模块

- lib.rs - ImageGenProvider trait + ProviderKind enum
- params.rs - ImageGenParams + ImageSize + ImageQuality + ImageStyle
- provider.rs - ProviderError + ProviderRegistry
- generators.rs - 4 provider 实现 + default_registry() + encode_base64
- result.rs - ImageGenResult + ImageFormat
- mcp.rs - ImageGenMcp 包装
- enhanced.rs - EnhancedImageGen 高层入口
- compat.rs - 兼容 adapter (旧 API 桥接)

## R156 改动

- 4 warnings -> 0 (enhanced.rs unused default_registry [test 用全路径], params.rs unreachable + unused w/h)
- warn(missing_docs) -> allow(missing_docs) (O-5)
- Cargo.toml description 清掉重复 MiniMax/MiniMax-Image + 不准 claim

## 0 假装

OK 29 单元测试 | OK 4 providers 都实现 trait | OK MockProvider 端到端跑通
