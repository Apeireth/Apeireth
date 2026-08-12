# apeireth-council

> Apeireth 智囊团 7 强制 Advisor + 按住机制 + 拟人化 synthesis + **真 LLM 后端** (R131 验证) — R14 Phase 5 P22.

## Status

Part of the Apeireth workspace (74 active crate after R128 94鈫?5 merge).

**No-fake**: every public type or trait documented in this crate is real.
**Run-no-fear**: cargo check --workspace passes (0 errors).

## R131 真接 LLM 后端 (P0-3 修复同步)

7 advisor 可通过 `LlmAdvisorBackend` 真接 LLM (替换默���的 keyword mock):

```rust
use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_council::{CouncilMember, CouncilMemberDeliberator, LlmAdvisorBackend};

let cfg = AnthropicCompatibleConfig::new(
    api_key,
    "https://api.minimaxi.com/anthropic",
    vec!["MiniMax-M3".to_string()],
);
let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);
let backend: Arc<dyn MockLlmProvider> = Arc::new(LlmAdvisorBackend::new(provider));

let deliberator = CouncilMemberDeliberator::new(members)
    .with_mock_llm(backend)
    .with_max_rounds(3);
```

实测 3 round × 3 member = 10.5s 端到端 (MiniMax-M3 通过 Anthropic 协议).

**P0-3 警告**: `MockLlmProvider` trait 自 1.2.0 已 `#[deprecated]`. 它不是真 LLM, 是脚本匹配. 真 LLM 走 `LlmAdvisorBackend` + `apeireth_api::llm::LlmProvider`.

完整 demo: `cargo run -p apeireth-council --release --example council_member_deliberation_demo`

---

## Where to start

- Cargo.toml: see [dependencies](Cargo.toml) for upstream crate.
- src/lib.rs: see top-level doc comment for module-level overview.

## See also

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth roadmap](../../docs/pages-source/roadmap.md)

---

_Auto-generated README per R128 batch (2026-08-12). Last-modified tracked in git log._