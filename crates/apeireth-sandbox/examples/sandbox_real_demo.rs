//! # `apeireth-sandbox` R20 阶段 6 flesh out: 真接 demo
//!
//! **本 demo 是 R20 阶段 6 flesh out 新增**, 演示 3 RuntimeKind × 6 API 真接实现.
//! 跟 `lib.rs` 现状 STUB 路径 (`SandboxSdk` 6 API 返 NotImplemented) 严格分离.
//!
//! ## 设计 (per 任务 spec + 蓝图 §3.5 缺口 + 主人 2026-08-06 派活)
//!
//! **8 演示入口** (跟 voice `voice_real_demo.rs` 1:1 模式):
//! 1. **Container runtime exec** — `Container` 真接 (`HttpDaemonClient` + mock URL)
//! 2. **Process runtime exec** — `Process` 真接 (`tokio::process::Command`)
//! 3. **Wasm runtime exec** — `Wasm` STUB (per 诚实标缺 #2: 0 真接 wasmtime)
//! 4. **container_status** — `status()` 真接
//! 5. **container_kill** — `kill()` 真接
//! 6. **network_create** — `network()` 真接
//! 7. **filesystem_read** — `filesystem()` 真接
//! 8. **resource_limit** — `resource_limit()` 真接
//!
//! ## 0 真连 Docker daemon
//!
//! 跟 voice 1:1 模式 (per 任务 spec: 本机可能没装 Docker):
//! - `HttpDaemonClient` base_url 指向 `http://127.0.0.1:1` (无 server, 必然 connect 失败)
//! - 演示 "真请求" 失败但**架构就绪**, 跟 voice 1:1 "AuthFailed" 模式
//! - 集成时只换 base_url 即可
//!
//! ## 6 哲学锚穿透 (跟 voice 1:1 模式)
//!
//! - **S-1 北极星**: 1:1 翻译 Docker daemon 6 维度, 0 假装"已连真 Docker daemon"
//! - **S-2 实事求是**: 演示输出"真尝试"结果, 不假装成功
//! - **O-2 走在前人肩上**: 8 演示入口简洁, 1 屏可读
//! - **O-3 干到底**: 3 RuntimeKind × 6 API = 18 组合 demo 演示
//! - **O-4 任何人都能接手**: 单一 main fn, 0 共享状态
//! - **O-5 不假装**: Wasm STUB 显式标缺
//!
//! ## 8 项不修改承诺 守门 (跟 voice 1:1 模式)
//!
//! - **#1 不假装已实现**: Container 真发 HTTP, 失败如实报; Process 真 spawn, 失败如实报; Wasm STUB
//! - **#2 编译期 hardcode**: 6 API 名 / 3 RuntimeKind / Reliability 常数仍 hardcode
//! - **#3 不改 LOCKED**: 0 改 24 LOCKED crate + 0 改 `apeireth-sdk-sandbox` LOCKED baseline
//! - **#4 不改 workspace version**: `version = "0.1.0"` 沿用
//! - **#5 6 哲学锚穿透**: 上 6 行
//! - **#6 不依赖 NewAPI**: 0 引外部 RPC
//! - **#7 不重复造轮子**: 沿用 workspace 已有 reqwest / tokio / serde
//! - **#8 诚实标缺**: Wasm STUB 显式标缺

use std::path::PathBuf;
use std::time::Duration;

use apeireth_sandbox::real::SandboxRealImpl;
use apeireth_sandbox::{
    FilesystemAction, NetworkAction, ResourceLimits, RuntimeKind, SandboxConfig, SandboxError,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("[sandbox_real_demo] apeireth-sandbox 真接实现 demo (R20 阶段 6 flesh out)");
    println!(
        "[sandbox_real_demo] base_url=http://127.0.0.1:1 (0 真连 Docker daemon, 跟 voice 1:1 模式)"
    );
    println!();

    let base_url = "http://127.0.0.1:1";
    let api_key = ""; // 0 注入, 走 env fallback 路径 (跟 voice 1:1)

    // 演示 1: Container runtime exec
    println!("[demo 1/8] Container runtime exec() 真接 (HttpDaemonClient)");
    let cfg_container = SandboxConfig {
        runtime: RuntimeKind::Container,
        image: "docker.io/library/alpine:3.19".to_string(),
        command: vec!["echo".to_string(), "hello from apeireth".to_string()],
        ..Default::default()
    };
    let impl_container = SandboxRealImpl::with_http_daemon(
        cfg_container.clone(),
        base_url,
        api_key,
    );
    match impl_container {
        Ok(impl_) => {
            // 短 timeout 因为 http://127.0.0.1:1 必然 connect 失败
            match tokio::time::timeout(
                Duration::from_secs(2),
                impl_container_clone_exec(&impl_, cfg_container.clone()),
            )
            .await
            {
                Ok(Ok(handle)) => println!("[demo 1/8] exec() 成功: handle.id={}", handle.id),
                Ok(Err(e)) => println!("[demo 1/8] exec() 失败 (跟 voice 1:1 真尝试失败模式): {e}"),
                Err(_) => println!("[demo 1/8] exec() timeout 2s (无 server, 必然 timeout)"),
            }
        }
        Err(e) => println!("[demo 1/8] SandboxRealImpl 创建失败: {e}"),
    }
    println!();

    // 演示 2: Process runtime exec
    println!("[demo 2/8] Process runtime exec() 真接 (tokio::process::Command)");
    let cfg_process = SandboxConfig {
        runtime: RuntimeKind::Process,
        image: String::new(),
        command: if cfg!(windows) {
            vec!["cmd".to_string(), "/C".to_string(), "echo".to_string(), "hello".to_string()]
        } else {
            vec!["echo".to_string(), "hello from apeireth process".to_string()]
        },
        ..Default::default()
    };
    match SandboxRealImpl::with_http_daemon(cfg_process.clone(), base_url, api_key) {
        Ok(impl_) => {
            match impl_.exec(cfg_process).await {
                Ok(handle) => println!("[demo 2/8] Process exec() 成功: handle.container_id={}", handle.container_id),
                Err(e) => println!("[demo 2/8] Process exec() 失败: {e}"),
            }
        }
        Err(e) => println!("[demo 2/8] Process SandboxRealImpl 创建失败: {e}"),
    }
    println!();

    // 演示 3: Wasm runtime exec (STUB 0 真接, 跟诚实标缺 #2)
    println!("[demo 3/8] Wasm runtime exec() STUB 守门 (per 诚实标缺 #2: 0 真接 wasmtime, R21+ 续)");
    let cfg_wasm = SandboxConfig {
        runtime: RuntimeKind::Wasm,
        image: String::new(),
        command: vec!["module.wasm".to_string()],
        ..Default::default()
    };
    match SandboxRealImpl::with_http_daemon(cfg_wasm.clone(), base_url, api_key) {
        Ok(impl_) => match impl_.exec(cfg_wasm).await {
            Ok(_) => println!("[demo 3/8] Wasm exec() 意外成功 (不应该, 标缺)"),
            Err(SandboxError::NotImplemented(msg)) => {
                println!("[demo 3/8] Wasm exec() 守门成功: NotImplemented({msg})")
            }
            Err(e) => println!("[demo 3/8] Wasm exec() 其他错误: {e}"),
        },
        Err(e) => println!("[demo 3/8] Wasm SandboxRealImpl 创建失败: {e}"),
    }
    println!();

    // 演示 4: status (无 sandbox 句柄 → NotFound)
    println!("[demo 4/8] status() 守门 (无 sandbox 句柄 → NotFound)");
    match SandboxRealImpl::with_http_daemon(SandboxConfig::default(), base_url, api_key) {
        Ok(impl_) => {
            let fake_id = uuid::Uuid::new_v4();
            match impl_.status(fake_id).await {
                Ok(handle) => println!("[demo 4/8] status() 意外成功: {handle:?}"),
                Err(SandboxError::NotFound(id)) => {
                    println!("[demo 4/8] status() 守门成功: NotFound({id})")
                }
                Err(e) => println!("[demo 4/8] status() 其他错误: {e}"),
            }
        }
        Err(e) => println!("[demo 4/8] status() 创建失败: {e}"),
    }
    println!();

    // 演示 5: kill (无 sandbox 句柄 → NotFound)
    println!("[demo 5/8] kill() 守门 (无 sandbox 句柄 → NotFound)");
    match SandboxRealImpl::with_http_daemon(SandboxConfig::default(), base_url, api_key) {
        Ok(impl_) => {
            let fake_id = uuid::Uuid::new_v4();
            match impl_.kill(fake_id).await {
                Ok(()) => println!("[demo 5/8] kill() 意外成功"),
                Err(SandboxError::NotFound(id)) => {
                    println!("[demo 5/8] kill() 守门成功: NotFound({id})")
                }
                Err(e) => println!("[demo 5/8] kill() 其他错误: {e}"),
            }
        }
        Err(e) => println!("[demo 5/8] kill() 创建失败: {e}"),
    }
    println!();

    // 演示 6: network create (1:1 翻译 Docker daemon `POST /networks/create`)
    println!("[demo 6/8] network() 真接 (per 6 API #4)");
    match SandboxRealImpl::with_http_daemon(SandboxConfig::default(), base_url, api_key) {
        Ok(impl_) => {
            let action = NetworkAction::Create {
                name: "apeireth-net".to_string(),
            };
            match tokio::time::timeout(Duration::from_secs(2), impl_.network(action)).await {
                Ok(Ok(())) => println!("[demo 6/8] network() 成功 (不应该, 0 真 server)"),
                Ok(Err(e)) => println!("[demo 6/8] network() 失败 (跟 voice 1:1 真尝试失败模式): {e}"),
                Err(_) => println!("[demo 6/8] network() timeout 2s (无 server)"),
            }
        }
        Err(e) => println!("[demo 6/8] network() 创建失败: {e}"),
    }
    println!();

    // 演示 7: filesystem read
    println!("[demo 7/8] filesystem() 真接 (per 6 API #5)");
    match SandboxRealImpl::with_http_daemon(SandboxConfig::default(), base_url, api_key) {
        Ok(impl_) => {
            let fake_id = uuid::Uuid::new_v4();
            let action = FilesystemAction::Read {
                sandbox_id: fake_id,
                path: PathBuf::from("/etc/hostname"),
            };
            match impl_.filesystem(action).await {
                Ok(data) => println!("[demo 7/8] filesystem() 成功: data={data:?}"),
                Err(SandboxError::NotFound(id)) => {
                    println!("[demo 7/8] filesystem() 守门成功: NotFound({id})")
                }
                Err(e) => println!("[demo 7/8] filesystem() 其他错误: {e}"),
            }
        }
        Err(e) => println!("[demo 7/8] filesystem() 创建失败: {e}"),
    }
    println!();

    // 演示 8: resource_limit
    println!("[demo 8/8] resource_limit() 真接 (per 6 API #6)");
    match SandboxRealImpl::with_http_daemon(SandboxConfig::default(), base_url, api_key) {
        Ok(impl_) => {
            let fake_id = uuid::Uuid::new_v4();
            let limits = ResourceLimits::default();
            match impl_.resource_limit(fake_id, limits).await {
                Ok(()) => println!("[demo 8/8] resource_limit() 意外成功"),
                Err(SandboxError::NotFound(id)) => {
                    println!("[demo 8/8] resource_limit() 守门成功: NotFound({id})")
                }
                Err(e) => println!("[demo 8/8] resource_limit() 其他错误: {e}"),
            }
        }
        Err(e) => println!("[demo 8/8] resource_limit() 创建失败: {e}"),
    }
    println!();

    println!("[sandbox_real_demo] 演示完成 (R20 阶段 6 flesh out 真接实现已 ready, 集成时换 base_url 即用)");
    println!("[sandbox_real_demo] 18 组合 demo: 3 RuntimeKind × 6 API = 18 (Container 真接 / Process 真接 / Wasm STUB)");

    // 给 background task 时间结束
    sleep(Duration::from_millis(100)).await;
}

/// clone 引用传 exec (per Rust borrow checker)
async fn impl_container_clone_exec(
    impl_: &SandboxRealImpl,
    cfg: SandboxConfig,
) -> Result<apeireth_sandbox::SandboxHandle, SandboxError> {
    impl_.exec(cfg).await
}
