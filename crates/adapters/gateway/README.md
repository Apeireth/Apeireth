# apeireth-gateway

> Apeireth OpenClaw-mode gateway: single long-lived daemon hosting multiple Node adapters (TUI/HTTP/Desktop/Mobile/CLI), Agent workspace, Skills, and DM access security. Borrows the 'single process, multi-LLM, multi-channel' architecture from OpenClaw, lifted to Rust with compile-time-guaranteed single-instance mode.

Apeireth v2 工作区 crate。测试数字单一真源见 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md)（本文不抄写）。

Apeireth **v2.0.0-rc.5** 工作区 crate（31 members 之一）。`src` 现有 **34 个公开模块**（逐字取自 `src/lib.rs` 的 `pub mod`，2026-11-06 合并后校）：

- **鉴权 / 多租户 / 管理面**：`admin` · `client_tokens` · `domain_admin` · `domain_context` ·
  `user_admin` · `principal_forget`
- **请求入口与错误面**：`canonical_entry` · `events` · `error_codes` · `error_frame`
- **流式与打断**：`duplex_gateway` · `barge_in`
- **面板与遥测**：`panels` · `panel_metrics` · `panel_memory_update` · `task_panel` ·
  `trait_panel` · `care_panel` · `skill_shelf` · `ember_hud_driver` · `presence`
- **外部通道 / 文件 / 远程**：`file_fetcher` · `remote_access` · `im_bridge` · `im_channels` ·
  `im_device` · `im_approval` · `im_proposal`
- **运行保障与审计**：`resident` · `shutdown` · `chain_audit` · `session_settings`
- **决策与归属**：`device_decision` · `principal_attribution`

> 模块清单以 `src/lib.rs` 的 `pub mod` 为唯一真源（本行于 2026-11-06 按合并后实际 34 个模块订正；
> 此前写「1.0 工作区 / 10 个模块」是 v1.0 时代口径，2026-10-25 的 32 个模块口径也已被合并后的
> `device_decision` / `principal_attribution` 两项取代）。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
