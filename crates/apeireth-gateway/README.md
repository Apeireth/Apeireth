# apeireth-gateway

> Apeireth OpenClaw-mode single long-lived Gateway (R174 / stage 6 §3) — 单一常驻进程承载多 Node 适配器 (TUI/HTTP/Desktop/Mobile/CLI)。

## 是什么

一个 **单一常驻进程**, 拥有一个 root `Session`, 注册任意数量的入站 `Node` 适配器,
施加 DM 访问安全 (`AccessPolicy` + `DmScope`), 并通过可插拔 `Transport` 注册表
把 payload 转发进 9-organ 系统。

## 借鉴 OpenClaw (Rust 化 + 编译期保证)

- "single process, multi-LLM, multi-channel" → `GatewayMode::SingleProcess` (本 crate 唯一模式)
- per-channel transport → `Transport` trait + registry
- per-user scope → `DmScope` + `AccessPolicy`

## 非目标 / 0 漂移

- 0 改任何 LOCKED crate (council / runtime / supervisor / ...)
- 0 新增外部依赖 (仅 tokio/serde/parking_lot/uuid/chrono, 与 rate-limiter 同)
- 0 LLM 调用 / 0 上游渲染 (交给 `apeireth-api`)
- `#![deny(unsafe_code)]`

## 状态

Apeireth workspace 成员 (81 members, 0 orphan)。

**No-fake**: 7 模块 (详见 src/lib.rs 顶部 doc)。GatewayMode::SingleProcess 为唯一实装模式。
**Run-no-fear**: `cargo check --workspace` 0 errors。

## 入口

- `Cargo.toml`: 见 [dependencies](Cargo.toml)
- `src/lib.rs`: 顶部 doc comment 是模块级总览

## 参见

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth 文档归位映射](../../docs/document-relocation-map.md)
