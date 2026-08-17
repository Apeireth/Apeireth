# 身 (Body) 器官 API

> **性质**: 9 器官之一 (per 整合 #3 C-1 借 Golutra #1)
> **对应 crate**: `apeireth-sandbox` (R20 阶段 6 真接, 6 API)
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **TUI 短单字**: 身 / **i18n 解剖名词**: 身体

---

## 0. 概览

| 维度 | 值 |
|------|----|
| **器官名** | body (身 / 身体) |
| **6 command** | exec / kill / status / network / filesystem / resource_limit |
| **关键 dep** | tokio 1.40 / reqwest 0.12 / bollard 0.15 (R21+) / apeireth-pipeline-g5 |
| **状态** | ✅ R20 阶段 6 真接 (per 整合 #3 F-3) |
| **i18n 状态** | G-1 续补 (per 整合 #3 G-2) |

---

## 1. 6 command

| command | 用途 | i18n key (中文) |
|---------|------|----------------|
| `exec` | 执行 (在容器 / 进程) | 执行 |
| `kill` | 杀掉 (运行中) | 杀 |
| `status` | 状态 (运行中 / 退出) | 状态 |
| `network` | 网络 (列 / 创 / 删) | 网络 |
| `filesystem` | 文件系统 (拷出 / 拷入) | 文件系统 |
| `resource_limit` | 资源限制 (CPU / 内存) | 资源限制 |

---

## 2. API 调用

```rust
use apeireth_sandbox::organ::body::{Body, ExecRequest, ExecResult};

let body = Body::new();
let result = body.exec(
    "container_xxx",
    ExecRequest {
        cmd: vec!["ls", "-la", "/workspace"],
        user: Some("root"),
        env: vec!["PATH=/usr/bin:/bin"],
        timeout: 30,
    },
).await?;
// ExecResult { stdout, stderr, exit_code }
```

---

## 3. 6 API (per `provider-sandbox.md` §2)

| API | HTTP | 1.0 状态 |
|-----|------|---------|
| **exec** | `POST /containers/{id}/exec` | ✅ wiremock |
| **kill** | `POST /containers/{id}/kill` | ✅ wiremock |
| **status** | `GET /containers/{id}/json` | ✅ wiremock |
| **network** | `GET /networks` + `POST /networks/create` | ✅ wiremock |
| **filesystem** | `GET/PUT /containers/{id}/archive` | ✅ wiremock |
| **resource_limit** | `PUT /containers/{id}/update` | ✅ wiremock |

---

## 4. 3 RuntimeKind

| RuntimeKind | 1.0 状态 |
|-------------|---------|
| **Container** (Docker) | ✅ wiremock, bollard 0.15 stub |
| **Process** (本地进程) | 🟡 partial |
| **WASM** | ⚪ R21+ |

---

## 5. 9 ContainerCreateSpec (per `provider-sandbox.md` §4)

9 K-1 强校验: image / command / user / env / port / volume / network / memory / cpu

---

## 6. TUI 9 器官 集成 (per 整合 #3 C-1)

```rust
// crates/apeireth-tui/src/organ/command/body.rs
impl Command for BodyCommand {
    fn name(&self) -> &str { "body" }  // i18n 改 async t() per G-2
    fn run(&self, args: &[String]) -> CommandResult { /* exec / kill / etc */ }
}
```

---

## 7. 相关

- [docs/api/provider-sandbox.md](provider-sandbox.md) (Sandbox SDK 视角)
- 实现: `crates/apeireth-sandbox/`
- 决策: 整合 #3 C-1 + F-3 + G-2

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
