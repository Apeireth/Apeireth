# apeireth-provider

> Apeireth 规范 provider 能力层 —— 把各厂商 wire 协议翻译成规范协议，并以 `ProviderCapability` 插件对外暴露。

`apeireth-provider` 只做一件事：厂商协议 ↔ 规范协议的翻译，外加凭据解析。路由、会话、
治理与工具执行都归规范运行时所有，本 crate 不重复实现这些语义。

## 主入口

- `src/lib.rs` — 模块清单（11 个 `mod` 声明，唯一真源）。
- `src/canonical_anthropic.rs` — `AnthropicProviderCapability`（`ProviderCapability` 的实现）
  与 `AnthropicProviderPlugin`（`from_env()` 按凭据解析装配）；`src/canonical_minimax.rs` —
  `MinimaxProviderCapability` / `MinimaxProviderPlugin`；`src/canonical_openai_compatible.rs` —
  `OpenAiCompatibleProviderCapability` / `OpenAiCompatibleProviderPlugin`（覆盖 DeepSeek 等任意
  OpenAI 形状端点）。
- `src/provider_model.rs` — `ProviderModel` 与 `find_model()`：规范模型 id ↔ 厂商 wire 名的映射。
- `src/credentials.rs` — `EnvCredentialResolver`（`new()` / `with_mapping()` / `env_var_for()`）
  与逻辑凭据名常量 `MINIMAX_API_KEY` / `ANTHROPIC_API_KEY` / `OPENAI_COMPATIBLE_API_KEY`。
- `src/embeddings.rs` — `OpenAiCompatibleEmbeddingProvider` 与 `parse_embedding_body()`：OpenAI
  形状的 embeddings 传输（端点 / 模型 / key 由 `APEIRETH_EMBEDDING_URL` / `APEIRETH_EMBEDDING_MODEL` / `APEIRETH_EMBEDDING_KEY` 指定）。
- `src/openai_chat.rs` — OpenAI Chat Completions 的公共 wire 转换；`src/reasoning_adapter.rs` — 推理字段归一化；`src/minimax_llm_factory.rs` /
  `src/openai_compatible_llm_factory.rs` — LLM 工厂实现；`src/llm_factory_adapters.rs`（`pub(crate)`）— 工厂共享的转换与错误映射。

## 测试

```
cargo test -p apeireth-provider --locked
```

集成测试在 `crates/engine/provider/tests/`（6 个测试文件）。其中
`tests/openai_compatible_live.rs` 的两条 live 用例标了 `#[ignore]`，默认 `cargo test` 不跑，
需手动 `cargo test -p apeireth-provider --test openai_compatible_live -- --ignored --nocapture`
并自备 `OPENAI_API_KEY`；测试数量以 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md) 为单一真源。

## 已知边界

- 凭据解析目前只读环境变量：`EnvCredentialResolver` 把逻辑名映射到环境变量；OS keyring、
  加密文件与 KMS 后端在 `apeireth-credentials` 里已有，但接到 `CredentialResolver` 后面仍是待办。
- embeddings 传输只认 OpenAI 形状响应（`data[0].embedding` 必须是数字数组），其它厂商形状一律返回 `InvalidVector`，不猜、不降级。
- 本 crate 没有 `[features]` 段（没有能力门控）；provider 插件是否被装配由组装根（`apeireth-runtime-assembly`）决定；此前本 README 写的「6 Provider」名单已不是当前模块构成。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
