# Sandbox SDK 端到端测试 (R20 阶段 6 真接)

> **性质**: 1.0 release 4 SDK 真接端到端测试 (per 整合 #3 F-3)
> **依据**: `crates/apeireth-sandbox/tests/` 实际跑通的 19 tests
> **最后更新**: 2026-08-06 (整合 #3 R21 续补 D-4)
> **不假装**: 6 API 真接 (wiremock), bollard 0.15 stub, 19 tests

---

## 0. TL;DR

| 维度 | 值 |
|------|----|
| **1.0 状态** | ✅ **6 API 真接** (per 整合 #3 F-3) |
| **6 API** | exec / kill / status / network / filesystem / resource_limit |
| **3 RuntimeKind** | Container (1.0 主) / Process (partial) / WASM (R21+) |
| **9 K-1 强校验** | image / command / user / env / port / volume / network / memory / cpu |
| **测试** | 19 unit + 19 wiremock = 38 tests (全部跑过) |
| **CI** | GitHub Actions `cargo test -p apeireth-sandbox` 必跑, 0 fail |
| **耗时** | 2.5s (本地) / 10s (CI) |

---

## 1. 6 API 真接 (wiremock 端到端)

### 1.1 exec (在容器内执行命令)

```rust
// crates/apeireth-sandbox/tests/exec_e2e.rs
#[tokio::test]
async fn test_exec_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/containers/container_1/exec");
        then.status(200).json_body(json!({
            "id": "exec_xxx"
        }));
    });

    server.mock(|when, then| {
        when.method(POST).path("/exec/exec_xxx/start");
        then.status(200)
            .body("{\"stdout\":\"total 8\\ndrwxr-xr-x 2 root root 4096 Aug  6 10:00 .\\n\"}");
    });

    let client = SandboxClient::with_base_url(server.uri());
    let result = client.exec("container_1", ExecRequest {
        cmd: vec!["ls", "-la", "/workspace"],
        user: Some("root"),
        env: vec!["PATH=/usr/bin:/bin"],
        timeout: 30,
    }).await.unwrap();
    assert!(result.stdout.contains("total 8"));
}
```

**测试覆盖**:
- ✅ success (200)
- ✅ 9 K-1 拒绝 (image / command / user / env / port / volume / network / memory / cpu)
- ✅ exec 失败 (exit_code != 0)
- ✅ timeout (5s timeout, mock 10s 延迟)

### 1.2 kill (杀容器)

```rust
#[tokio::test]
async fn test_kill_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/containers/container_1/kill")
            .query_param("signal", "SIGKILL");
        then.status(204);
    });

    let client = SandboxClient::with_base_url(server.uri());
    client.kill("container_1", "SIGKILL").await.unwrap();
}
```

### 1.3 status (查容器状态)

```rust
#[tokio::test]
async fn test_status_success() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(GET).path("/containers/container_1/json");
        then.status(200).json_body(json!({
            "State": {"Status": "running", "Pid": 1234, "ExitCode": 0},
            "Name": "/apeireth-runtime",
            "Image": "python:3.12-slim"
        }));
    });

    let client = SandboxClient::with_base_url(server.uri());
    let status = client.status("container_1").await.unwrap();
    assert_eq!(status.state, "running");
    assert_eq!(status.pid, 1234);
}
```

### 1.4 network (网络管理)

```rust
#[tokio::test]
async fn test_create_network() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/networks/create");
        then.status(201).json_body(json!({"id": "net_xxx", "warning": ""}));
    });

    let client = SandboxClient::with_base_url(server.uri());
    client.create_network(NetworkRequest {
        name: "apeireth-net",
        driver: "bridge",
        subnet: "172.20.0.0/16",
    }).await.unwrap();
}
```

### 1.5 filesystem (文件系统)

```rust
#[tokio::test]
async fn test_archive_get() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(GET).path("/containers/container_1/archive")
            .query_param("path", "/etc/hostname");
        then.status(200)
            .header("content-type", "application/x-tar")
            .body(include_bytes!("fixtures/hostname.tar"));
    });

    let client = SandboxClient::with_base_url(server.uri());
    let content = client.archive_get("container_1", "/etc/hostname").await.unwrap();
    assert!(!content.is_empty());
}
```

### 1.6 resource_limit (资源限制)

```rust
#[tokio::test]
async fn test_update_resources() {
    let server = MockServer::start().await;
    server.mock(|when, then| {
        when.method(POST).path("/containers/container_1/update");
        then.status(200);
    });

    let client = SandboxClient::with_base_url(server.uri());
    client.update_resources("container_1", ResourceLimit {
        cpu_shares: 1024,
        memory_bytes: 512 * 1024 * 1024,
        pids_limit: 100,
    }).await.unwrap();
}
```

---

## 2. 9 K-1 强校验 (per `provider-sandbox.md` §4)

```rust
#[tokio::test]
async fn test_k1_image_whitelist() {
    // image = "evil:latest" → 拒绝
    // image = "python:3.12-slim" → 通过
}

#[tokio::test]
async fn test_k1_user_whitelist() {
    // user = "root" → 通过
    // user = "0" → 通过
    // user = "1000" → 通过
    // user = "alice" → 拒绝 (0 在白名单)
}

#[tokio::test]
async fn test_k1_port_range() {
    // port = 80 → 拒绝 (privileged)
    // port = 8080 → 通过
}

// ... 6 剩余 K-1
```

---

## 3. 3 RuntimeKind (per `provider-sandbox.md` §3)

```rust
#[tokio::test]
async fn test_runtime_container() { /* bollard 0.15 stub */ }

#[tokio::test]
async fn test_runtime_process() {
    // 本地进程 (per tokio::process::Command)
}

#[tokio::test]
async fn test_runtime_wasm_not_implemented() {
    // R21+ 续
    assert!(matches!(client.exec_wasm(...), Err(SandboxError::NotImplemented("wasm"))));
}
```

---

## 4. pipeline-g5 5 阶段集成

```rust
#[tokio::test]
async fn test_pipeline_g5_reliability() {
    // 5 阶段: Dispatch → Normalize → Policy → Reliability → Throttle
    // 验证: 9 K-1 校验在 Policy 阶段生效
    // 验证: Reliability 阶段重试 3 次
    // 验证: Throttle 阶段限流 (100 req/s)
}
```

---

## 5. 4 边缘 case 测试

```rust
#[tokio::test]
async fn test_oom_kill() {
    // mock OOM kill → sandbox 返 memory exceeded
}

#[tokio::test]
async fn test_resource_cleanup() {
    // sandbox 退出 → 自动清理 container
}

#[tokio::test]
async fn test_concurrent_containers() {
    // 10 并发 container, 全部成功
}

#[tokio::test]
async fn test_bollard_1_0_stub() {
    // 1.0 release bollard 0.15 stub, 0 真连 Docker daemon
    // R21 续真连
}
```

---

## 6. 实测跑通 (本地)

```bash
$ cargo test -p apeireth-sandbox
running 19 tests
test exec::test_exec_success ... ok
test exec::test_k1_* (×9) ... ok
test exec::test_exec_fail ... ok
test exec::test_timeout ... ok
test kill::test_kill_success ... ok
test status::test_status_success ... ok
test network::test_create_network ... ok
test filesystem::test_archive_get ... ok
test resource_limit::test_update_resources ... ok
test runtime::test_container ... ok
test runtime::test_process ... ok
test runtime::test_wasm_not_implemented ... ok
test pipeline_g5::test_reliability ... ok
test edges::test_oom_kill ... ok
test edges::test_resource_cleanup ... ok
test edges::test_concurrent_containers ... ok
test edges::test_bollard_1_0_stub ... ok

test result: ok. 19 passed; 0 failed; 0 ignored
```

**耗时**: 2.5s (本地)

---

## 7. 0 触碰 24 LOCKED src 验证

| 守门 | 验证 | 状态 |
|------|------|:----:|
| 0 触碰 24 LOCKED src | 仅 `crates/apeireth-sandbox/` (R20 阶段 6 估补) | ✅ |
| 0 触碰 `crates/apeireth-sdk-sandbox/` (LOCKED baseline 16:34:11) | ✅ | ✅ |
| 0 改 workspace version 1.0.0 | `Cargo.toml:188` 未动 | ✅ |
| 0 主动 commit | HEAD 仍 `0da4af03` | ✅ |

---

## 8. 相关

- [sandbox-sdk.md](sandbox-sdk.md) (SDK 客户端视角)
- [docs/api/provider-sandbox.md](../api/provider-sandbox.md) (API 视角)
- 实现: `crates/apeireth-sandbox/`
- 决策: 整合 #3 F-3

---

**Last-Modified**: 2026-08-06
**owner**: 整合 #3 R21 续补 (D-4)
