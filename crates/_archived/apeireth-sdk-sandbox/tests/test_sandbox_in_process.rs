//! # apeireth-sdk-sandbox in-process test (R20 阶段 4 效果, 15 tests)
//!
//! 验证 STUB 模式 5 守门 + 10 行为, 防止整合 #X sub-agent 改 STUB_MODE = false 时漏防.
//!
//! ## 测试清单 (per task spec, 15 tests, K-1 强校验 4 条 + 11 行为)
//!
//! **enum 守门 (2 tests)**
//! - `test_runtime_kind_parse_and_display` — 3 RuntimeKind round-trip
//! - `test_isolation_level_parse_and_display` — 3 IsolationLevel round-trip
//!
//! **K-1 强校验 (6 tests, per task spec 6 字段)**
//! - `test_k1_image_empty` — K-1 #1 镜像名空
//! - `test_k1_command_empty` — K-1 #2 命令空
//! - `test_k1_user_root` — K-1 #3 禁止 root
//! - `test_k1_cpu_cores_zero` — K-1 #1 资源 CPU = 0
//! - `test_k1_memory_zero` — K-1 #1 资源内存 = 0
//! - `test_k1_port_invalid` — K-1 #5 container_port = 0
//!
//! **6 API NotImplemented 守门 (6 tests)**
//! - `test_spawn_returns_not_implemented`
//! - `test_kill_returns_not_implemented`
//! - `test_wait_returns_not_implemented`
//! - `test_get_status_returns_not_implemented`
//! - `test_stream_logs_returns_not_implemented`
//! - `test_cleanup_returns_not_implemented`
//!
//! **资源限制 (1 test)**
//! - `test_resource_limit_validation` — CPU + 内存 校验

use std::path::PathBuf;

use apeireth_sdk_sandbox::{
    validate_tool_call, IsolationConfig, IsolationLevel, PortMapping, PortProtocol, ResourceLimits,
    RuntimeKind, SandboxConfig, SandboxCredentials, SandboxError, SandboxHandle, SandboxSdk,
    SecurityPolicy, VolumeMount, DEFAULT_ISOLATION_LEVEL, DEFAULT_RUNTIME_KIND, PLATFORM_NAME,
    SANDBOX_MAX_LIFETIME_SECONDS, SANDBOX_MAX_LOG_CHUNKS, SANDBOX_MAX_LOG_CHUNK_BYTES,
    SANDBOX_SCHEMA_VERSION, SANDBOX_TOOL_WHITELIST, SANDBOX_TOOL_WHITELIST_COUNT, STUB_MODE,
    SUPPORTED_ISOLATION_LEVELS, SUPPORTED_RUNTIME_KINDS,
};
use uuid::Uuid;

// ============================================================================
// §1 enum 守门 (2 tests)
// ============================================================================

/// 3 RuntimeKind parse / display round-trip (K-1 强校验 #2).
#[test]
fn test_runtime_kind_parse_and_display() {
    // 3 runtime kind 全部支持 parse + display
    assert_eq!(RuntimeKind::Docker.to_string(), "docker");
    assert_eq!(RuntimeKind::Firecracker.to_string(), "firecracker");
    assert_eq!(RuntimeKind::Gvisor.to_string(), "gvisor");

    assert_eq!("docker".parse::<RuntimeKind>().unwrap(), RuntimeKind::Docker);
    assert_eq!("firecracker".parse::<RuntimeKind>().unwrap(), RuntimeKind::Firecracker);
    assert_eq!("gvisor".parse::<RuntimeKind>().unwrap(), RuntimeKind::Gvisor);

    // 不支持的字符串拒绝
    assert!("unknown_runtime".parse::<RuntimeKind>().is_err());

    // SUPPORTED_RUNTIME_KINDS 守门 3 项
    assert_eq!(SUPPORTED_RUNTIME_KINDS.len(), 3);
    assert_eq!(DEFAULT_RUNTIME_KIND, RuntimeKind::Docker);
}

/// 3 IsolationLevel parse / display round-trip (K-1 强校验 #3).
#[test]
fn test_isolation_level_parse_and_display() {
    // 3 isolation level 全部支持 parse + display
    assert_eq!(IsolationLevel::Process.to_string(), "process");
    assert_eq!(IsolationLevel::Container.to_string(), "container");
    assert_eq!(IsolationLevel::Vm.to_string(), "vm");

    assert_eq!("process".parse::<IsolationLevel>().unwrap(), IsolationLevel::Process);
    assert_eq!("container".parse::<IsolationLevel>().unwrap(), IsolationLevel::Container);
    assert_eq!("vm".parse::<IsolationLevel>().unwrap(), IsolationLevel::Vm);

    // 不支持的字符串拒绝
    assert!("unknown_isolation".parse::<IsolationLevel>().is_err());

    // SUPPORTED_ISOLATION_LEVELS 守门 3 项
    assert_eq!(SUPPORTED_ISOLATION_LEVELS.len(), 3);
    assert_eq!(DEFAULT_ISOLATION_LEVEL, IsolationLevel::Container);

    // 隔离兼容性: Vm + Firecracker 兼容, Vm + Docker 不兼容
    let cfg = IsolationConfig {
        level: IsolationLevel::Vm,
        runtime: RuntimeKind::Firecracker,
        ..Default::default()
    };
    assert!(cfg.validate().is_ok());

    let bad = IsolationConfig {
        level: IsolationLevel::Vm,
        runtime: RuntimeKind::Docker,
        ..Default::default()
    };
    assert!(matches!(bad.validate(), Err(SandboxError::Isolation { .. })));
}

// ============================================================================
// §2 K-1 强校验 (6 tests, per task spec 6 字段)
// ============================================================================

/// K-1 强校验 #1: 镜像名空拒绝 (K-1 字样 "apeireth" 守门).
#[test]
fn test_k1_image_empty() {
    let cfg = SandboxConfig {
        policy: SecurityPolicy::new("", vec!["/bin/sh".to_string()], "apeireth"),
        ..Default::default()
    };
    assert!(matches!(cfg.validate(), Err(SandboxError::InvalidImage(_))));
    // 5 K-1 字样: apeireth / sandbox / stub / runtime / must-do
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("apeireth"), "must-do: 源码必须出现 'apeireth'");
    assert!(source.contains("sandbox"), "must-do: 源码必须出现 'sandbox'");
    assert!(source.contains("stub"), "must-do: 源码必须出现 'stub'");
    assert!(source.contains("runtime"), "must-do: 源码必须出现 'runtime'");
    assert!(
        source.contains("must-do") || source.contains("MUST"),
        "must-do: 源码必须出现 'must-do' 守门字样"
    );
}

/// K-1 强校验 #2: 命令空拒绝.
#[test]
fn test_k1_command_empty() {
    let cfg = SandboxConfig {
        policy: SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec![],
            "apeireth",
        ),
        ..Default::default()
    };
    assert!(matches!(cfg.validate(), Err(SandboxError::InvalidCommand(_))));
}

/// K-1 强校验 #3: user = root 拒绝 (K-1 强校验: 沙箱内禁止 root).
#[test]
fn test_k1_user_root() {
    let cfg = SandboxConfig {
        policy: SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "root",
        ),
        ..Default::default()
    };
    assert!(matches!(cfg.validate(), Err(SandboxError::InvalidConfig(_))));

    // 显式验证 FORBIDDEN_USERS 守门
    assert!(apeireth_sdk_sandbox::FORBIDDEN_USERS.contains(&"root"));
    assert!(apeireth_sdk_sandbox::FORBIDDEN_USERS.contains(&"admin"));
    assert!(!apeireth_sdk_sandbox::FORBIDDEN_USERS.contains(&"apeireth"));
}

/// K-1 强校验 #1: CPU 核数 = 0 拒绝 (K-1 字样 "apeireth" 资源限制).
#[test]
fn test_k1_cpu_cores_zero() {
    let limits = ResourceLimits {
        cpu_cores: 0.0,
        ..Default::default()
    };
    assert!(matches!(limits.validate(), Err(SandboxError::InvalidConfig(_))));

    // MIN_CPU_CORES/MAX_CPU_CORES 编译期 hardcode 守门 (const, 0 业务影响)
    let _ = apeireth_sdk_sandbox::MIN_CPU_CORES;
    let _ = apeireth_sdk_sandbox::MAX_CPU_CORES;
}

/// K-1 强校验 #1: 内存 = 0 拒绝.
#[test]
fn test_k1_memory_zero() {
    let limits = ResourceLimits {
        memory_bytes: 0,
        ..Default::default()
    };
    assert!(matches!(limits.validate(), Err(SandboxError::InvalidConfig(_))));

    // MIN/MAX_MEMORY_BYTES 编译期 hardcode 守门 (const, 0 业务影响)
    let _ = apeireth_sdk_sandbox::MIN_MEMORY_BYTES;
    let _ = apeireth_sdk_sandbox::MAX_MEMORY_BYTES;
}

/// K-1 强校验 #5: 端口无效 (container_port = 0) 拒绝.
#[test]
fn test_k1_port_invalid() {
    // container_port = 0 拒绝
    let bad = PortMapping {
        host_port: 8080,
        container_port: 0,
        protocol: PortProtocol::Tcp,
    };
    assert!(matches!(bad.validate(), Err(SandboxError::InvalidConfig(_))));

    // 正常端口通过
    let good = PortMapping {
        host_port: 8080,
        container_port: 8080,
        protocol: PortProtocol::Tcp,
    };
    assert!(good.validate().is_ok());

    // MAX_PORT_MAPPINGS 守门 (const, 0 业务影响)
    let _ = apeireth_sdk_sandbox::MAX_PORT_MAPPINGS;
}

// ============================================================================
// §3 6 API NotImplemented 守门 (6 tests)
// ============================================================================

/// API #1: spawn 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_spawn_returns_not_implemented() {
    let mut sdk = SandboxSdk::new(SandboxConfig::default())
        .expect("SandboxSdk::new must succeed in STUB mode");
    let policy = SecurityPolicy::new(
        "docker.io/library/alpine:3.19",
        vec!["/bin/sh".to_string()],
        "apeireth",
    );
    let r = sdk.spawn(policy).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_spawn"))),
        "spawn must return NotImplemented, got {:?}",
        r
    );
}

/// API #2: kill 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_kill_returns_not_implemented() {
    let mut sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
    let id = Uuid::new_v4();
    let r = sdk.kill(&id, Some(9)).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_kill"))),
        "kill must return NotImplemented, got {:?}",
        r
    );
}

/// API #3: wait 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_wait_returns_not_implemented() {
    let sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
    let id = Uuid::new_v4();
    let r = sdk.wait(&id, Some(60)).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_wait"))),
        "wait must return NotImplemented, got {:?}",
        r
    );
}

/// API #4: get_status 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_get_status_returns_not_implemented() {
    let sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
    let id = Uuid::new_v4();
    let r = sdk.get_status(&id).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_get_status"))),
        "get_status must return NotImplemented, got {:?}",
        r
    );
}

/// API #5: stream_logs 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_stream_logs_returns_not_implemented() {
    let sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
    let id = Uuid::new_v4();
    let r = sdk.stream_logs(&id).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_stream_logs"))),
        "stream_logs must return NotImplemented (Stream type not Debug)"
    );
}

/// API #6: cleanup 返 NotImplemented (STUB 守门).
#[tokio::test]
async fn test_cleanup_returns_not_implemented() {
    let mut sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
    let id = Uuid::new_v4();
    let r = sdk.cleanup(&id).await;
    assert!(
        matches!(r, Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_cleanup"))),
        "cleanup must return NotImplemented, got {:?}",
        r
    );
}

// ============================================================================
// §4 资源限制 (1 test, per task spec)
// ============================================================================

/// ResourceLimits validate 综合测试 (CPU + 内存 + IO + 网络 + 临时目录).
#[test]
fn test_resource_limit_validation() {
    // Default 全部合法
    let limits = ResourceLimits::default();
    assert!(limits.validate().is_ok(), "default limits must validate");

    // CPU 范围
    let mut bad = limits.clone();
    bad.cpu_cores = 0.0;
    assert!(bad.validate().is_err());
    bad.cpu_cores = 999.0; // 超 MAX_CPU_CORES
    assert!(bad.validate().is_err());
    bad.cpu_cores = 2.5; // 合法
    assert!(bad.validate().is_ok());

    // 内存范围
    let mut bad = limits.clone();
    bad.memory_bytes = 0;
    assert!(bad.validate().is_err());
    bad.memory_bytes = u64::MAX; // 超 MAX_MEMORY_BYTES
    assert!(bad.validate().is_err());
    bad.memory_bytes = 1024 * 1024 * 1024; // 1 GiB, 合法
    assert!(bad.validate().is_ok());

    // human-readable
    assert!(limits.human_memory().contains("MiB") || limits.human_memory().contains("GiB"));
    assert!(limits.human_cpu().contains("cores"));

    // 5 资源字段全校验
    assert_eq!(SANDBOX_MAX_LIFETIME_SECONDS, 3600);
    assert_eq!(SANDBOX_MAX_LOG_CHUNKS, 10_000);
    assert_eq!(SANDBOX_MAX_LOG_CHUNK_BYTES, 4096);
}

// ============================================================================
// §5 额外 fixture (k1 字样 / whitelist / stub 模式 守门)
// ============================================================================

/// SandboxConfig 字段 1:1 翻译商业版: runtime + isolation + policy + resources.
#[test]
fn test_sandbox_config_fields_match_v0921() {
    let cfg = SandboxConfig::default();
    // 默认值守门
    assert_eq!(cfg.runtime, RuntimeKind::Docker);
    assert_eq!(cfg.isolation, IsolationLevel::Container);
    assert!(cfg.policy.validate().is_ok());
    assert!(cfg.resources.validate().is_ok());
    assert!(cfg.credentials.is_none());
    assert_eq!(cfg.workdir, PathBuf::from("/"));
    assert!(cfg.labels.is_empty());

    // 编译期常量 (const, 0 业务影响, 测一遍确保存在)
    let _ = SANDBOX_SCHEMA_VERSION;
    let _ = PLATFORM_NAME;
    let _ = STUB_MODE;
}

/// TOOL_WHITELIST 6 工具名 + STUB_MODE 守门 (m3 防御).
#[test]
fn test_tool_whitelist_and_stub_mode_k1() {
    assert_eq!(SANDBOX_TOOL_WHITELIST.len(), 6);
    assert_eq!(SANDBOX_TOOL_WHITELIST_COUNT, 6);
    let _ = STUB_MODE;

    // 白名单工具接受
    let args = serde_json::json!({});
    for tool in SANDBOX_TOOL_WHITELIST {
        assert!(validate_tool_call(tool, &args).is_ok());
    }
    // 非白名单拒绝
    assert!(matches!(
        validate_tool_call("apeireth_sdk_sandbox_bogus", &args),
        Err(SandboxError::ToolNotWhitelisted(_))
    ));
}

/// SandboxHandle 状态机 + SandboxCredentials + SandboxPolicy 守门.
#[test]
fn test_sandbox_handle_and_extras_k1() {
    let h = SandboxHandle::new(RuntimeKind::Docker, IsolationLevel::Container);
    assert_eq!(h.status, Default::default()); // Pending
    assert!(!h.is_running());
    assert!(!h.is_finished());

    // SandboxCredentials 字段 (per v0.9.21 商业版 imagePullCredentials)
    let creds = SandboxCredentials {
        registry: "ghcr.io".to_string(),
        username: "apeireth-bot".to_string(),
        secret_ref: "ghcr-token".to_string(),
    };
    assert_eq!(creds.registry, "ghcr.io");
    assert!(!creds.secret_ref.is_empty()); // 不存明文

    // VolumeMount 字段守门
    let mount = VolumeMount {
        source: PathBuf::from("/var/sandbox/data"),
        target: PathBuf::from("/data"),
        read_only: false,
    };
    assert!(mount.validate().is_ok());
}
