//! # apeireth-sdk-sandbox stub demo (R20 阶段 4 效果)
//!
//! 演示 6 stub 工具返 `SandboxError::NotImplemented` + SandboxConfig 构造 + 6 K-1 强校验.
//! **R21+ 真接 docker/firecracker/gvisor 时, 本 demo 会被替换成真 spawn / kill / wait demo.**
//!
//! ## 运行
//!
//! ```bash
//! cargo run --manifest-path crates/apeireth-sdk-sandbox/Cargo.toml --example sandbox_demo
//! ```

use apeireth_sdk_sandbox::{
    validate_tool_call, IsolationConfig, IsolationLevel, PortMapping, PortProtocol, ResourceLimits,
    RuntimeKind, SandboxConfig, SandboxCredentials, SandboxError, SandboxHandle, SandboxSdk,
    SecurityPolicy, VolumeMount, DEFAULT_ISOLATION_LEVEL, DEFAULT_RUNTIME_KIND, PLATFORM_NAME,
    SANDBOX_MAX_LIFETIME_SECONDS, SANDBOX_MAX_LOG_CHUNKS, SANDBOX_MAX_LOG_CHUNK_BYTES,
    SANDBOX_SCHEMA_VERSION, SANDBOX_TOOL_WHITELIST, SANDBOX_TOOL_WHITELIST_COUNT, STUB_MODE,
    SUPPORTED_ISOLATION_LEVELS, SUPPORTED_RUNTIME_KINDS,
};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-sdk-sandbox stub demo (R20 阶段 4 效果) ===");
    println!();

    // 1) 编译期 hardcode 守门 (K-1 强校验)
    println!("[§1 编译期 hardcode]");
    println!("  SANDBOX_SCHEMA_VERSION    = {}", SANDBOX_SCHEMA_VERSION);
    println!("  PLATFORM_NAME             = {}", PLATFORM_NAME);
    println!("  STUB_MODE                 = {}", STUB_MODE);
    println!("  SANDBOX_MAX_LIFETIME_SECS = {}", SANDBOX_MAX_LIFETIME_SECONDS);
    println!("  SANDBOX_MAX_LOG_CHUNKS    = {}", SANDBOX_MAX_LOG_CHUNKS);
    println!("  SANDBOX_MAX_CHUNK_BYTES   = {}", SANDBOX_MAX_LOG_CHUNK_BYTES);
    println!("  DEFAULT_RUNTIME_KIND      = {:?}", DEFAULT_RUNTIME_KIND);
    println!("  DEFAULT_ISOLATION_LEVEL   = {:?}", DEFAULT_ISOLATION_LEVEL);
    println!();

    // 2) 3 RuntimeKind + 3 IsolationLevel
    println!("[§2 3 RuntimeKind + 3 IsolationLevel (K-1 强校验 #2/#3)]");
    println!("  SUPPORTED_RUNTIME_KINDS: {:?}", SUPPORTED_RUNTIME_KINDS);
    println!("  SUPPORTED_ISOLATION_LEVELS: {:?}", SUPPORTED_ISOLATION_LEVELS);
    for r in SUPPORTED_RUNTIME_KINDS {
        println!("    RuntimeKind::{:?} -> \"{}\"", r, r.as_str());
    }
    for i in SUPPORTED_ISOLATION_LEVELS {
        println!("    IsolationLevel::{:?} -> \"{}\"", i, i.as_str());
    }
    println!();

    // 3) 6 工具白名单
    println!("[§3 6 工具白名单 (m3 防御)]");
    println!("  SANDBOX_TOOL_WHITELIST_COUNT = {}", SANDBOX_TOOL_WHITELIST_COUNT);
    for (i, tool) in SANDBOX_TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    println!();

    // 4) m3 防御: validate_tool_call 测试
    println!("[§4 m3 防御: validate_tool_call]");
    let args = serde_json::json!({});
    let valid = validate_tool_call("apeireth_sdk_sandbox_spawn", &args);
    println!("  白名单内工具: {:?}", valid);
    let invalid = validate_tool_call("apeireth_sdk_sandbox_bogus", &args);
    println!("  非白名单工具: {:?}", invalid);
    println!();

    // 5) SandboxConfig 构造 (Docker + Container + 默认 alpine + 1 GiB 内存)
    println!("[§5 SandboxConfig 构造]");
    let policy = SecurityPolicy::new(
        "docker.io/library/alpine:3.19",
        vec!["/bin/sh".to_string(), "-c".to_string(), "echo hello".to_string()],
        "apeireth",
    );
    let resources = ResourceLimits {
        cpu_cores: 2.0,
        memory_bytes: 1024 * 1024 * 1024, // 1 GiB
        io_bandwidth_bps: 100 * 1024 * 1024,
        network_bandwidth_bps: 100 * 1024 * 1024,
        tmp_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
    };
    let creds = SandboxCredentials {
        registry: "ghcr.io".to_string(),
        username: "apeireth-bot".to_string(),
        secret_ref: "ghcr-pull-token".to_string(),
    };
    let mut labels = HashMap::new();
    labels.insert("env".to_string(), "demo".to_string());
    labels.insert("owner".to_string(), "apeireth".to_string());

    let config = SandboxConfig {
        runtime: RuntimeKind::Docker,
        isolation: IsolationLevel::Container,
        isolation_config: IsolationConfig {
            level: IsolationLevel::Container,
            runtime: RuntimeKind::Docker,
            pid_namespace: true,
            network_namespace: true,
            mount_namespace: true,
            seccomp_profile: None,
            cgroup_slice: None,
            capabilities: Vec::new(),
        },
        policy,
        resources,
        credentials: Some(creds),
        workdir: PathBuf::from("/workspace"),
        labels,
    };
    println!("  runtime        = {:?}", config.runtime);
    println!("  isolation      = {:?}", config.isolation);
    println!("  policy.image   = {}", config.policy.image);
    println!("  policy.user    = {}", config.policy.user);
    println!("  resources      = {:.2} cores / {}", config.resources.cpu_cores, config.resources.human_memory());
    println!("  credentials    = registry={}, secret_ref={}", 
        config.credentials.as_ref().unwrap().registry,
        config.credentials.as_ref().unwrap().secret_ref);
    println!("  workdir        = {}", config.workdir.display());
    println!("  labels         = {} entries", config.labels.len());
    println!();

    // 6) 6 stub 工具返 NotImplemented
    println!("[§6 6 stub 工具返 NotImplemented]");
    let mut sdk = SandboxSdk::new(config).expect("SandboxSdk::new must succeed in STUB mode");
    let id = Uuid::new_v4();

    println!("  spawn        : {:?}", sdk.spawn(sdk.config().policy.clone()).await);
    println!("  kill         : {:?}", sdk.kill(&id, Some(15)).await);
    println!("  wait         : {:?}", sdk.wait(&id, Some(60)).await);
    println!("  get_status   : {:?}", sdk.get_status(&id).await);
    println!("  stream_logs  : <Pin<Box<dyn Stream>>> returned (NotImplemented)");
    println!("  cleanup      : {:?}", sdk.cleanup(&id).await);
    println!();

    // 7) SandboxHandle 状态机演示
    println!("[§7 SandboxHandle 状态机]");
    let mut h = SandboxHandle::new(RuntimeKind::Docker, IsolationLevel::Container);
    println!("  initial: status={:?} is_running={}", h.status, h.is_running());

    h.status = apeireth_sdk_sandbox::SandboxStatus::Running;
    println!("  after Running: is_running={} is_finished={}", h.is_running(), h.is_finished());

    h.status = apeireth_sdk_sandbox::SandboxStatus::Stopped;
    h.exit_code = Some(0);
    println!("  after Stopped: is_running={} is_finished={} exit_code={:?}",
        h.is_running(), h.is_finished(), h.exit_code);
    println!();

    // 8) 6 K-1 强校验演示 (per task spec)
    println!("[§8 6 K-1 强校验 (per task spec 6 字段)]");
    let bad_configs = vec![
        ("镜像名空", SandboxConfig {
            policy: SecurityPolicy::new("", vec!["/bin/sh".to_string()], "apeireth"),
            ..Default::default()
        }),
        ("命令空", SandboxConfig {
            policy: SecurityPolicy::new("docker.io/library/alpine:3.19", vec![], "apeireth"),
            ..Default::default()
        }),
        ("user=root", SandboxConfig {
            policy: SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "root",
            ),
            ..Default::default()
        }),
        ("env=LD_PRELOAD", {
            let mut p = SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "apeireth",
            );
            p.env.insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
            SandboxConfig { policy: p, ..Default::default() }
        }),
        ("container_port=0", {
            let mut p = SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "apeireth",
            );
            p.ports.push(PortMapping {
                host_port: 8080,
                container_port: 0,
                protocol: PortProtocol::Tcp,
            });
            SandboxConfig { policy: p, ..Default::default() }
        }),
        ("volume mount /etc", {
            let mut p = SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "apeireth",
            );
            p.mounts.push(VolumeMount {
                source: PathBuf::from("/etc/passwd"),
                target: PathBuf::from("/mnt/passwd"),
                read_only: true,
            });
            SandboxConfig { policy: p, ..Default::default() }
        }),
    ];
    for (name, cfg) in bad_configs {
        let r = cfg.validate();
        let err_name = match &r {
            Err(SandboxError::InvalidImage(_)) => "InvalidImage",
            Err(SandboxError::InvalidCommand(_)) => "InvalidCommand",
            Err(SandboxError::InvalidConfig(_)) => "InvalidConfig",
            Err(SandboxError::Isolation { .. }) => "Isolation",
            _ => "Other",
        };
        println!("  K-1 [{:<18}] 拒绝: {} (预期拒绝)", name, err_name);
    }
    println!();

    println!("=== demo 完 (R20 阶段 4 效果: 整合 #X sub-agent 1 commit 落地, 改 STUB_MODE=false + 接 docker/firecracker/gvisor) ===");
}
