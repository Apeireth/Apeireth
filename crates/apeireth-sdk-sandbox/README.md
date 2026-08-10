# apeireth-sdk-sandbox

> **⚠️ STUB MODE**: R20 阶段 4 效果, 6 工具全部返 `SandboxError::NotImplemented`. 修改需经 6 哲学锚 + 主人审.

## 概述

`apeireth-sdk-sandbox` 是 Apeireth 平台的 **沙箱 SDK**, 1:1 翻译 v0.9.21 商业版
`@anthropic-ai/sandbox` 进程隔离 / 资源限制 / 安全策略 API 表面.

**当前状态**: STUB skeleton (R20 阶段 4 效果, 整合 #X sub-agent 派活中).
**真实实现**: 留 R21+ 真接 docker / firecracker / gvisor.

## 6 核心 API (1:1 翻译 v0.9.21 商业版)

| 工具 (商业版) | 1:1 翻译 | R21+ 真实实现 |
|--------------|---------|--------------|
| `spawn` | `SandboxSdk::spawn` | bollard `Docker::create_container` / firecracker `VM::start` / runsc |
| `kill` | `SandboxSdk::kill` | bollard `Docker::kill` / firecracker `VM::stop` / runsc |
| `wait` | `SandboxSdk::wait` | tokio::select + `container.wait` |
| `getStatus` | `SandboxSdk::get_status` | bollard `Docker::inspect_container` / `VM::state` |
| `streamLogs` | `SandboxSdk::stream_logs` | bollard `Docker::logs(stream=true)` |
| `cleanup` | `SandboxSdk::cleanup` | bollard `Docker::remove_container` (含 volume) |

## 3 运行时 (K-1 强校验 #2)

- `Docker` (默认, per v0.9.21 商业版 `runtime: "docker"`)
- `Firecracker` (microVM, per `runtime: "firecracker"`)
- `Gvisor` (用户态内核, per `runtime: "gvisor"`)

## 3 隔离级别 (K-1 强校验 #3)

- `Process` (Linux namespace + seccomp + cgroup, 默认)
- `Container` (Docker / gVisor runsc)
- `Vm` (Firecracker microVM, KVM)

## 5 资源限制 (K-1 强校验 #1)

| 字段 | 范围 | 单位 |
|------|------|------|
| `cpu_cores` | [0.1, 64.0] | 核 |
| `memory_bytes` | [16 MiB, 256 GiB] | bytes |
| `io_bandwidth_bps` | [1 MiB/s, 10 GiB/s] | bytes/sec |
| `network_bandwidth_bps` | [1 MiB/s, 10 GiB/s] | bytes/sec |
| `tmp_bytes` | [1 MiB, 100 GiB] | bytes |

## 6 K-1 强校验

1. **镜像名**: 非空, 含 tag, registry 在白名单 (8 个)
2. **命令**: 非空, 不含 shell 注入字符 (`; & | $` > < \n \0`)
3. **user**: 非空, 禁止 root / admin / Administrator / SYSTEM
4. **env**: KEY 不在禁列 (LD_PRELOAD / LD_LIBRARY_PATH / PATH / PYTHONPATH / 等 10 个)
5. **端口**: container_port 范围 1-65535, 单沙箱 ≤ 16 端口映射
6. **卷挂载**: 源路径前缀在白名单 (5 个), 目标必须绝对路径, 单沙箱 ≤ 32 挂载

## 🧭 6 哲学锚 (per task spec)

1. **S-1 不漂移**: 0 假装已实现, 6 API 全部返 `NotImplemented`.
2. **S-2 编译期 hardcode**: `STUB_MODE = true` 不可运行时改.
3. **O-2 工程铁律**: 0 引重复造轮子的 dep (bollard / firecracker-rs / runsc).
4. **O-3 m3 防御**: 6 工具白名单 + `validate_tool_call` schema 校验.
5. **O-4 不假装可观测**: 失败时返 `NotImplemented` + `tracing::warn!`.
6. **O-5 K-1 强校验**: 6 字段 `validate()` 走 6 K-1 检查.

## 🔒 8 项不修改承诺 (per task spec)

1. `version.workspace = true` ✅
2. `edition.workspace = true` ✅
3. `rust-version.workspace = true` ✅
4. `license.workspace = true` ✅
5. `authors.workspace = true` ✅
6. deps 用 `{ workspace = true }` ✅
7. 不修改 workspace Cargo.toml (由整合 #X sub-agent 加 member) ⏳
8. 不引 unsafe (workspace `#![deny(unsafe_code)]` 继承) ✅

## 模块结构

```
crates/apeireth-sdk-sandbox/
├── Cargo.toml                      (8 项承诺 workspace version)
├── README.md                       (本文档)
├── src/
│   ├── lib.rs                      (793 行, 6 哲学锚 + 8 承诺 + 5 K-1 字样 + 6 API stub)
│   ├── error.rs                    (10 variant SandboxError)
│   ├── runtime.rs                  (3 RuntimeKind + 3 IsolationLevel + 6 SandboxStatus)
│   ├── isolation.rs                (IsolationConfig + SandboxRuntime trait)
│   ├── resource.rs                 (5 资源限制 + ResourceUsage)
│   └── policy.rs                   (6 K-1 强校验 + VolumeMount + PortMapping)
├── tests/
│   └── test_sandbox_in_process.rs  (355 行, 15 测试)
└── examples/
    └── sandbox_demo.rs             (8 段 demo)
```

## 集成路径 (R21+)

```text
apeireth-tool-runtime
         ↓
apeireth-sdk-sandbox (本 crate)
         ↓
   ┌─────┴─────┬─────────┐
   │           │         │
Docker     Firecracker  gVisor
(bollard)  (KVM)        (runsc)
```

## 状态

⏳ STUB skeleton (R20 阶段 4 效果, 主人 2026-08-05 派 #X sub-agent 干).
整合 #X sub-agent 1 commit 落地时, 改 `STUB_MODE = false` + 真接 bollard / firecracker-rs / runsc.
