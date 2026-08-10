# Sandbox SDK stub

> **依据**: `crates/apeireth-sdk-sandbox/src/lib.rs` 实际实现
> **最后更新**: 2026-08-05
> **状态**: 🟡 stub（进程隔离 stub，完整执行 stub）

---

## 1. 概览

**功能**: 沙盒代码执行（per `code_exec` tool）
**目标**: isolate / wasmtime / firecracker（per 路线图 19 路）
**1.0 状态**: stub

---

## 2. API

### 2.1 进程沙盒

```rust
use apeireth_sdk_sandbox::{Sandbox, SandboxConfig};

let sandbox = Sandbox::new(SandboxConfig {
    runtime: Runtime::Process,
    memory_limit_mb: 512,
    cpu_limit: 1.0,
    timeout_seconds: 30,
    network: NetworkPolicy::Denied,  // 默认禁网
});

let result = sandbox.execute("python3", r#"
print("hello")
"#, &[], &[]).await?;

// 1.0 stub: NotImplemented
```

### 2.2 容器沙盒

```rust
let sandbox = Sandbox::new(SandboxConfig {
    runtime: Runtime::Container { image: "python:3.12".into() },
    ..Default::default()
});

let result = sandbox.execute("python3", "print('hi')", &[], &[]).await?;
// 1.0 stub
```

### 2.3 WASM 沙盒

```rust
let sandbox = Sandbox::new(SandboxConfig {
    runtime: Runtime::Wasm { module: wasm_bytes.to_vec() },
    ..Default::default()
});

let result = sandbox.execute_wasm("main", &[]).await?;
// 1.0 stub
```

---

## 3. 三档 Runtime

| Runtime | 隔离强度 | 性能 | 启动 | R21 计划 |
|---|---|---|---|---|
| `Process` | 弱 (UID 隔离) | 1.0x | <10ms | R21 early |
| `Container` | 中 (namespace + cgroup) | 0.95x | ~100ms | R21 mid |
| `Wasm` | 强 (wasmtime) | 0.6x | <5ms | R21 late |
| `Firecracker` | 极强 (microVM) | 0.85x | ~150ms | R22+ |

---

## 4. 4 守门

```rust
pub struct SandboxConfig {
    pub runtime: Runtime,
    pub memory_limit_mb: u32,
    pub cpu_limit: f32,           // 1.0 = 1 核
    pub timeout_seconds: u32,
    pub network: NetworkPolicy,   // Denied / Allowlist / Allowed
    pub filesystem: FsPolicy,     // ReadOnly / ReadWrite(paths)
    pub env: Vec<(String, String)>,
}
```

**per APEIRETH-CONVENTIONS.md §9 6 哲学锚**:
- 进程隔离 → 防止代码逃逸
- 网络默认禁 → 防止外联
- 文件系统默认只读 → 防止篡改
- 资源硬限 → 防止 DoS

---

## 5. 与 code_exec 工具关系

`/v1/tools/code_exec/invoke` 1.0 走 `apeireth-tools::code_exec` 直调，**不走 Sandbox SDK**。
R21 切换：code_exec tool → Sandbox SDK → 三档 runtime 可选。

---

## 6. 错误

```rust
pub enum Error {
    NotImplemented(&'static str),
    ResourceLimit(String),
    Timeout,
    NetworkDenied,
    ExecutionFailed(String),
}
```

---

## 7. R21 计划

| Runtime | R21 实装 | 估时 |
|---|---|---|
| `Process` | tokio::process + rlimit | 1 owner × 1 周 |
| `Container` | bollard (Docker API) | 2 owner × 1 周 |
| `Wasm` | wasmtime | 1 owner × 1 周 |
| `Firecracker` | firecracker-rs | 2 owner × 1 周 (R22+) |

**总估**: 6 owner × 1 周

---

## 8. 不假装

- ✅ API 签名清楚
- ✅ 4 守门 policy 类型定义
- 🟡 4 Runtime 真接 R21/R22
- ✅ 不假装已实现

---

## 9. 相关

- 实现: `crates/apeireth-sdk-sandbox/src/lib.rs`
- code_exec tool: `crates/apeireth-tools/src/code_exec.rs`
- 决策: R20 阶段 1 拍板"SDK stub 留 R21 续"
- 路线图: `docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md` (19 路含 wasmtime / qdrant)
