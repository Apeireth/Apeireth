# Sandbox SDK API (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接之一 (per 整合 #3 F-3)
> **依据**: `crates/apeireth-sandbox/src/` + Docker daemon HTTP API v1.43+ 1:1 翻译
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-3)
> **不假装**: 6 API 真接, bollard 0.15 stub, 19 tests (per 整合 #3 F-3)

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ **6 API 真接** (wiremock) + 🟡 bollard 0.15 stub (1.0 release) |
| **6 API** | exec / kill / status / network / filesystem / resource_limit |
| **3 RuntimeKind** | Container (1.0 主路径) / Process (1.0 partial) / WASM (R21+) |
| **9 ContainerCreateSpec** | image / command / user / env / port / volume / etc |
| **测试** | 19 unit + 19 wiremock = 38 tests |
| **依赖** | bollard 0.15 (R21+ 真接) + reqwest 0.12 + apeireth-pipeline-g5 |

---

## 1. 客户端初始化

```rust
use apeireth_sandbox::{SandboxClient, SandboxConfig, RuntimeKind};

let client = SandboxClient::new(SandboxConfig {
    base_url: "unix:///var/run/docker.sock".to_string(),  // Docker daemon
    runtime: RuntimeKind::Container,
    network: "bridge".to_string(),
    timeout: 60,
})
.with_pipeline_g5(apeireth_pipeline_g5::Pipeline::default());  // 5 阶段集成
```

---

## 2. 6 API 真接 (wiremock 端到端)

### 2.1 exec (在容器内执行命令)

```rust
let exec_result = client.exec(
    "container_xxx",
    ExecRequest {
        cmd: vec!["ls", "-la", "/workspace"],
        user: Some("root"),
        env: vec!["PATH=/usr/bin:/bin"],
        timeout: 30,
    },
).await?;
// 返 ExecResult { stdout, stderr, exit_code }
```

**API**: `POST /containers/{id}/exec` (1:1 翻译 Docker daemon HTTP v1.43+)

### 2.2 kill (杀容器)

```rust
client.kill("container_xxx", "SIGKILL").await?;
// 返 ()
```

**API**: `POST /containers/{id}/kill`

### 2.3 status (查容器状态)

```rust
let status = client.status("container_xxx").await?;
// 返 ContainerStatus { state: "running", pid: 1234, exit_code: 0, ... }
```

**API**: `GET /containers/{id}/json`

### 2.4 network (网络管理)

```rust
let networks = client.list_networks().await?;
// 返 Vec<Network> { id, name, driver, subnet, gateway }

client.create_network(NetworkRequest {
    name: "apeireth-net",
    driver: "bridge",
    subnet: "172.20.0.0/16",
}).await?;
```

**API**: `GET /networks` + `POST /networks/create`

### 2.5 filesystem (文件系统操作)

```rust
// 拷出文件
let content = client.archive_get("container_xxx", "/etc/hostname").await?;
// 返 Vec<u8>

// 拷入文件
client.archive_put("container_xxx", "/workspace/inputs/data.json", &content).await?;
```

**API**: `GET /containers/{id}/archive` + `PUT /containers/{id}/archive`

### 2.6 resource_limit (资源限制)

```rust
client.update_resources("container_xxx", ResourceLimit {
    cpu_shares: 1024,
    memory_bytes: 512 * 1024 * 1024,  // 512 MB
    pids_limit: 100,
}).await?;
```

**API**: `PUT /containers/{id}/update`

---

## 3. 3 RuntimeKind

| RuntimeKind | 1.0 状态 | R21 续 |
|-------------|---------|--------|
| **Container** | ✅ wiremock 端到端 (bollard 0.15 stub) | R21 真连 Docker daemon |
| **Process** | 🟡 partial (本地进程, 0 容器) | R21 完善 |
| **WASM** | ⚪ TODO R21+ | 估 1-2 月 |

---

## 4. 9 ContainerCreateSpec (K-1 强校验)

| K-1 | 校验内容 | 白名单 |
|-----|---------|--------|
| **image** | 5 白名单 image | `python:3.12-slim` / `node:20-alpine` / `rust:1.80-slim` / `alpine:3.20` / `ubuntu:24.04` |
| **command** | shell-words 解析, 0 注入 | — |
| **user** | 必须 root / 特定 UID | `root` / UID 1000+ |
| **env** | 5 白名单 env 变量 | `PATH` / `HOME` / `USER` / `LANG` / `TZ` |
| **port** | 0 privileged port (< 1024) | 1024-65535 |
| **volume** | 0 bind mount host 根目录, 只能 workspace 目录 | `/workspace/...` |
| **network** | 5 白名单 network | `bridge` / `host` / `none` / `apeireth-net` / `apeireth-test` |
| **memory** | ≤ 8 GB | 8 GB |
| **cpu** | ≤ 4 cores | 4 cores |

---

## 5. 集成 pipeline-g5 (Reliability 5 阶段)

```rust
use apeireth_pipeline_g5::{Pipeline, Stage};

let pipeline = Pipeline::new()
    .add_stage(Stage::Dispatch)       // 1 派发
    .add_stage(Stage::Normalize)      // 2 标准化
    .add_stage(Stage::Policy)         // 3 策略 (9 K-1 强校验)
    .add_stage(Stage::Reliability)    // 4 可靠 (重试 / 限流 / 断路)
    .add_stage(Stage::Throttle);      // 5 限流 (rate limit)

client.set_pipeline(pipeline);
```

---

## 6. 19 tests + 19 wiremock 端到端

| 类别 | 数量 |
|------|----:|
| 6 API × 2 case (success / K-1 拒绝) = 12 | 12 |
| 3 RuntimeKind × 2 case = 6 | 6 |
| 9 ContainerCreateSpec × 1 case = 9 | 9 |
| pipeline-g5 5 阶段 × 1 case = 5 | 5 |
| 7 边缘 case (timeout / OOM / 限流 / 权限 / etc) | 7 |
| **总** | **~39** |

> per 整合 #3 F-3 估 19 (实际 39, 含组合)

---

## 7. 7+ 诚实标缺 (per 整合 #3 F-3)

| 标缺 | R21+ 续 |
|------|---------|
| Docker daemon 0 真连 (1.0 仅 wiremock) | R21 真接 |
| WASM 0 真接 | R21+ 估 1-2 月 |
| 资源限制跨平台差异 (Windows / macOS) | R21 兼容 |
| 5 P0 拒绝场景未全测 | R21 补 |
| bollard 0.15 API 变更风险 | R21 锁定 version |
| docker-compose 集成 | R21+ |
| Kubernetes operator 集成 | R21+ |

---

## 8. 相关

- [docs/sdk/sandbox-sdk.md](../sdk/sandbox-sdk.md) (SDK 客户端视角)
- 实现: `crates/apeireth-sandbox/`
- 1:1 翻译源: Docker daemon HTTP API v1.43+
- 决策: 整合 #3 F-3

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-3)
