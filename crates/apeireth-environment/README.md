# apeireth-environment

> Apeireth terminal sandboxes (R173 / Stage2 §3) — 6 个终端后端执行沙箱。

## 6 backend

| Backend | 用途 | 状态 |
|---|---|---|
| Local | 本机执行 (default) | 真实现 |
| Docker | 容器化执行 (linux + seccomp) | 真实现 |
| SSH | 远程执行 (rust SSH client) | 真实现 |
| Daytona | dev environment SaaS (HTTP REST API) | 远程 stub |
| Modal | serverless Python (HTTP REST API) | 远程 stub |
| Singularity | HPC container (subprocess interface) | 远程 stub |

## 设计约束 (不漂移)

- 0 改 `apeireth-tool-shell` 任何已实装类型
- 0 副作用: 每个 backend `execute()` 返回 `Result`, 不直接 IO
- 借鉴 `apeireth-tool-shell` 的 `sandbox.rs` 设计模式
- `#![deny(unsafe_code)]`

## 状态

Apeireth workspace 成员 (81 members, 0 orphan)。

**No-fake**: 6 backend trait + Local/Docker/SSH 真实现; Daytona/Modal/Singularity 为远程 stub (未装真实现)。
**Run-no-fear**: `cargo check --workspace` 0 errors。

## 入口

- `Cargo.toml`: 见 [dependencies](Cargo.toml)
- `src/lib.rs`: 顶部 doc comment 是模块级总览

## 参见

- [Apeireth conventions](../../docs/conventions/README.md)
- [Apeireth 文档归位映射](../../docs/document-relocation-map.md)
