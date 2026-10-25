# apeireth-gateway

> Apeireth OpenClaw-mode gateway: single long-lived daemon hosting multiple Node adapters (TUI/HTTP/Desktop/Mobile/CLI), Agent workspace, Skills, and DM access security. Borrows the 'single process, multi-LLM, multi-channel' architecture from OpenClaw, lifted to Rust with compile-time-guaranteed single-instance mode.

Apeireth **v2.0.0-rc.4** 工作区 crate（30 members 之一）。`src` 现有 **32 个公开模块**（逐字取自 `src/lib.rs` 的 `pub mod`，2026-10-25 校）：

- **鉴权 / 多租户 / 管理面**：`admin` · `client_tokens` · `domain_admin` · `domain_context` ·
  `user_admin` · `principal_forget`
- **请求入口与错误面**：`canonical_entry` · `events` · `error_codes` · `error_frame`
- **流式与打断**：`duplex_gateway` · `barge_in`
- **面板与遥测**：`panels` · `panel_metrics` · `panel_memory_update` · `task_panel` ·
  `trait_panel` · `care_panel` · `skill_shelf` · `ember_hud_driver` · `presence`
- **外部通道 / 文件 / 远程**：`file_fetcher` · `remote_access` · `im_bridge` · `im_channels` ·
  `im_device` · `im_approval` · `im_proposal`
- **运行保障与审计**：`resident` · `shutdown` · `chain_audit` · `session_settings`

> 模块清单以 `src/lib.rs` 的 `pub mod` 为唯一真源（本行于 2026-10-25 按实际 32 个模块订正；
> 此前写「1.0 工作区 / 10 个模块」是 v1.0 时代口径，已过时）。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
