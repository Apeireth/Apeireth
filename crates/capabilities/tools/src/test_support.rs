//! 测试共享支持 (D26, 2026-10-21): 回环 bind 重试助手。
//!
//! 背景: Windows 上 `TcpListener::bind("127.0.0.1:0")` 偶发 `WSAEACCES`
//! (`os error 10013`) —— 系统挑中的临时端口恰好落在被 Hyper-V / WinNAT /
//! 其它组件保留的区间内 (失败帧见 `artifacts/wave-bio-w32d-workspace-test.log`:
//! `egress::tests::direct_ip_transport_works` 等)。这是**环境侧瞬时现象**,
//! 不是测试逻辑错误: 重试一次会换到另一个临时端口。
//!
//! 纪律 (D26): **只**对 Windows `WSAEACCES (10013)` 做有界重试 (最多 8 次,
//! 每次间隔 5ms); 其它错误原样返回; 非 Windows 平台无此现象, 直通首个结果。
//! 重试次数耗尽是环境侧故障, 由调用方 `.unwrap()` 如实失败, 不静默 pass。
//!
//! 用法 (单源 helper, 不复制实现):
//! - crate 内单测: `crate::test_support::bind_loopback_ephemeral().await?`
//! - 本 crate 集成测试: `tests/common/mod.rs` 用
//!   `#[path = "../../src/test_support.rs"] mod test_support;` 引入同一文件并
//!   `pub use test_support::bind_loopback_ephemeral;`, 测试文件写 `mod common;`
//!   后调 `common::bind_loopback_ephemeral().await?`。
//!
//! 其它 crate 若出现同形态 flake, 建议按同一模式在各自 crate 内放一份
//! (仓库不新增 test-support crate: 不改 `workspace.members` 口径, 不引外部依赖)。
#![allow(dead_code)]

use std::io;
use std::time::Duration;

/// crate 级 env 锁：所有改进程 env 的测试**共用同一把**。
///
/// **为什么需要它（2026-10-25 实测）**：`std::env::set_var` 在**多线程进程**里
/// 不是线程安全的 —— Rust 官方文档明确：并发 `getenv`/`setenv` 可致未定义行为。
/// `cargo test --lib` 会把一个 crate 的**全部单测跑在同一进程**，
/// 于是不同模块**各自声明**的 `static ENV_LOCK` **互不排斥**：
/// 某模块写 env 的瞬间，别的模块的测试可能读到瞬态值。
///
/// **本 crate 的约定**：crate 内**只允许这一把** crate 级锁；
/// 模块内再声明私有 `static ENV_LOCK` = 破坏互斥。
/// 本模块的守卫测试检查这条约定，并实测锁的互斥。
#[cfg(test)]
pub(crate) static CRATE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 取 crate 级 env 锁。
///
/// 中毒时取内层：一次测试失败不该把后续所有测试连带锁死。
#[cfg(test)]
pub(crate) fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    CRATE_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 设进程 env（`None` = 移除）。
#[cfg(test)]
pub(crate) fn set_env(name: &str, value: Option<&str>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

/// Windows `WSAEACCES`: 临时端口撞上系统保留区间。
const WSAEACCES: i32 = 10013;

/// 绑定 `127.0.0.1:0`, 仅对 Windows `WSAEACCES (10013)` 做有界重试。
pub async fn bind_loopback_ephemeral() -> io::Result<tokio::net::TcpListener> {
    const MAX_ATTEMPTS: u32 = 8;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match tokio::net::TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => return Ok(listener),
            Err(error) if is_reserved_port_error(&error) && attempt < MAX_ATTEMPTS => {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

/// 仅 Windows `WSAEACCES` 算「临时端口撞保留区间」; 其它错误不重试。
fn is_reserved_port_error(error: &io::Error) -> bool {
    cfg!(windows) && error.raw_os_error() == Some(WSAEACCES)
}

/// 通用「开关 fail-closed」断言（**持 crate 级 env 锁**）。
///
/// 覆盖五段，与各模块原本逐份手写的断言**逐条等价**：
/// 1. 未设 = 关；2. 一组杂值全关；3. 显式 `"1"` = 开；
/// 4. 显式 `"1"` = **配置可构造**（`config_constructible` 传 `Some` 时）；
/// 5. `" 1 "`（前后空白）= 开。
///
/// `config_constructible` 传 `Some` 时额外断言「开关关 ⇒ 配置**不构造**」，
/// 对齐「开关关 ⇒ 该工具面不注册」的 fail-closed 语义；
/// 传 `None` 则只断言开关本身。
///
/// **为什么要共享**（2026-10-25）：这份断言此前在 5 个模块里**各抄一份**，
/// 每份还各自持一把**私有** env 锁 —— 而私有锁**跨模块不互斥**
/// （`cargo test --lib` 同进程），实测会偶发失败。
/// 抽成一份 + 共用 crate 级锁，一次解决「重复」与「不互斥」两件事。
#[cfg(test)]
pub(crate) fn assert_switch_fails_closed_except_exact_one(
    env_name: &str,
    bad_values: &[&str],
    is_on: impl Fn() -> bool,
    config_constructible: Option<fn() -> bool>,
) {
    let _guard = lock_env();
    let assert_off = |value: &str| {
        assert!(!is_on(), "{env_name} 必须拒绝 {value:?}");
        if let Some(config_ok) = config_constructible {
            assert!(!config_ok(), "{env_name} 必须拒绝 {value:?}（配置不构造）");
        }
    };
    set_env(env_name, None);
    assert_off("未设");
    for value in bad_values {
        set_env(env_name, Some(value));
        assert_off(value);
    }
    set_env(env_name, Some("1"));
    assert!(is_on(), "{env_name} 显式 1 必须开");
    if let Some(config_ok) = config_constructible {
        assert!(config_ok(), "{env_name} 显式 1 = 配置构造");
    }
    set_env(env_name, Some(" 1 "));
    assert!(is_on(), "{env_name} 前后空白容忍");
    set_env(env_name, None);
}

/// 私有 env 锁的**代码**模式。
///
/// **运行时拼出来**，不是字面量 —— 否则本守卫的源码里就会出现这个模式，
/// 把自己也命中（第一版就踩了这个：守卫报的是它自己那几行）。
fn private_lock_needle() -> String {
    format!("static ENV_{}", "LOCK")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 助手本身可用: 返回已绑定且端口非 0 的回环 listener。
    #[tokio::test]
    async fn bind_loopback_ephemeral_returns_live_loopback_listener() {
        let listener = bind_loopback_ephemeral()
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local_addr");
        assert!(addr.ip().is_loopback(), "expected loopback, got {addr}");
        assert_ne!(addr.port(), 0, "ephemeral bind must pick a real port");
    }

    /// 重试条件收窄: 仅 Windows WSAEACCES (10013); AddrInUse / 其它错误不重试。
    #[test]
    fn is_reserved_port_error_is_narrow_to_windows_wsaeeacces() {
        let wsaeeacces = io::Error::from_raw_os_error(10013);
        let addrinuse = io::Error::from_raw_os_error(10048);
        let other = io::Error::new(io::ErrorKind::ConnectionRefused, "synthetic");
        assert_eq!(is_reserved_port_error(&wsaeeacces), cfg!(windows));
        assert!(!is_reserved_port_error(&addrinuse));
        assert!(!is_reserved_port_error(&other));
    }

    /// 守卫：crate 源码内**不得**再有私有 `static ENV_LOCK`。
    ///
    /// 私有锁是本缺陷的**复发路径**：它看起来是"加了保护"，
    /// 实际只锁住自己那个文件，跨模块竞争照旧（`cargo test --lib` 同进程）。
    /// 只看**代码行**（跳过注释行，注释里可以正常谈论这个模式）。
    #[test]
    fn no_private_env_lock_remains_in_crate_sources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut hits: Vec<String> = Vec::new();
        let mut stack = vec![root.join("src")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (idx, line) in text.lines().enumerate() {
                    let trimmed = line.trim_start();
                    // 注释行不算（文档里可以提及该模式）。
                    if trimmed.starts_with("//") || trimmed.starts_with("*") {
                        continue;
                    }
                    if line.contains(&private_lock_needle()) {
                        hits.push(format!(
                            "{}:{}: {}",
                            path.strip_prefix(root).unwrap_or(&path).display(),
                            idx + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
        assert!(
            hits.is_empty(),
            "crate 内不得声明私有 `{}`（跨模块不互斥）:\n{}",
            private_lock_needle(),
            hits.join("\n")
        );
    }

    /// 守卫：每个集成测试二进制**最多一把** env 锁。
    ///
    /// 多于一把 = 同文件内多个测试函数各自持锁 ⇒ 互不排斥。
    #[test]
    fn each_integration_test_binary_has_at_most_one_env_lock() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut over: Vec<String> = Vec::new();
        let mut stack = vec![root.join("tests")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let count = text.matches(private_lock_needle().as_str()).count();
                if count > 1 {
                    over.push(format!("{}: {count} 把", path.display()));
                }
            }
        }
        assert!(
            over.is_empty(),
            "同一集成测试二进制里多于一把 env 锁（文件内互不排斥）:\n{}",
            over.join("\n")
        );
    }

    /// 守卫：crate 级锁**真的互斥**（第二个线程必须等第一个释放）。
    #[test]
    fn crate_env_lock_actually_excludes() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let holding = Arc::new(AtomicBool::new(false));
        let saw_holding = Arc::new(AtomicBool::new(false));
        let holder_flag = holding.clone();
        let observer_flag = saw_holding.clone();

        // Establish ownership before starting the contender. The channel confirms
        // an actual rejected acquisition instead of depending on scheduling delays.
        let guard = lock_env();
        holding.store(true, Ordering::SeqCst);
        let (blocked_tx, blocked_rx) = std::sync::mpsc::channel();
        let observer = std::thread::spawn(move || {
            assert!(matches!(
                CRATE_ENV_LOCK.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            blocked_tx.send(()).expect("report blocked contender");
            let guard = lock_env();
            // 拿到锁时，另一个线程应当**不在**持锁中。
            observer_flag.store(!holder_flag.load(Ordering::SeqCst), Ordering::SeqCst);
            drop(guard);
        });

        blocked_rx
            .recv()
            .expect("contender must observe the held lock");
        // 观察者此时必须被挡住。
        assert!(
            !saw_holding.load(Ordering::SeqCst),
            "第二个线程在第一个持锁期间拿到了锁 ⇒ 锁无效"
        );
        holding.store(false, Ordering::SeqCst);
        drop(guard);
        observer.join().expect("观察线程 join");
        assert!(
            saw_holding.load(Ordering::SeqCst),
            "第一个释放后第二个线程应能拿到锁"
        );
    }
}
