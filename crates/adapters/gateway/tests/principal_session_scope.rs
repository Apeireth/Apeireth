//! 「**主体 → 会话**」归属写面**槽位模型**的钉桩测试（2026-11-05）。
//!
//! # 这一批钉的是**槽位模型**本身
//!
//! 旧模型是**进程级单槽** ⇒ 并行用例互相顶替 / 一个用例的 `Drop` 误杀另一个用例的
//! 写面 / 串数据。新模型（方案 B′）分两层：
//!
//! | 层 | 载体 | 谁用 |
//! |---|---|---|
//! | **全局默认槽** | 进程级 `static RwLock` | **生产**（跨线程、跨请求） |
//! | **作用域内槽** | **线程局部**栈 | **测试**（各用例各推各的层） |
//!
//! **读侧规则**：**先看作用域内栈 ⇒ 没有才回落全局默认槽**。
//!
//! # 为什么全局默认槽必须是**进程级**
//!
//! 生产读侧在 **HTTP handler**（`apeireth-gateway` 的 `post(native_chat)` ⇒ tokio
//! worker 线程），装入侧在**启动线程** ⇒ ⇒ **两者不是同一个线程**
//! ⇒ ⇒ 写面必须**跨线程可见** ⇒ ⇒ **不能**改成线程局部
//! （否则 handler 线程读不到 ⇒ 归属**被静默丢弃** ⇒ 最坏的一类故障）。
//!
//! ⇒ ⇒ 本文件用**真跨线程**（`std::thread::spawn`）钉住这一点。
//!
//! # 隔离纪律
//!
//! 本文件**碰全局默认槽**的用例 ⇒ 串行化（`DEFAULT_SLOT_LOCK`）⇒
//! ⇒ `cargo test`（同进程多线程）与 `cargo nextest`（一用例一进程）都稳定。

use std::sync::{mpsc, Arc, Barrier, Mutex};

use apeireth_runtime::canonical::principal_sessions::{self, PrincipalSessionRecorder};

/// 碰**全局默认槽**的用例 ⇒ 串行化（`cargo test` 同进程多线程时防互踩）。
///
/// `cargo nextest` 是「一用例一进程」⇒ 该锁在此处是**空操作**（不掩盖任何问题）。
static DEFAULT_SLOT_LOCK: Mutex<()> = Mutex::new(());

/// 假写面：记下每一次登记调用（带标签 ⇒ 断言消息能说清是哪一个）。
#[derive(Debug)]
struct Recorder {
    label: String,
    calls: Mutex<Vec<(String, String)>>,
}

impl Recorder {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().expect("记录锁").clone()
    }

    fn label(&self) -> &str {
        &self.label
    }
}

impl PrincipalSessionRecorder for Recorder {
    fn record(&self, principal_id: &str, session_id: &str) -> Result<(), String> {
        self.calls
            .lock()
            .expect("记录锁")
            .push((principal_id.to_string(), session_id.to_string()));
        Ok(())
    }
}

/// 取全局默认槽的**独占**锁（毒了就取内层 ⇒ 前一个用例 panic 不卡死后面）。
fn lock_default_slot() -> std::sync::MutexGuard<'static, ()> {
    DEFAULT_SLOT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// ⭐ **测试 1：作用域正确 —— A 用例装的 recorder 不泄漏到 B 用例**。
///
/// 两个**真线程** = 两个并行用例 ⇒ 各自 `enter_scope` 各自的写面
/// ⇒ ⇒ 断言**互不可见**：A 的登记只落在 A 的写面，B 的只落在 B 的写面。
///
/// 第三个**无作用域**的线程 ⇒ 必须**回落全局默认槽** ⇒ 证明
/// 「作用域内槽」既不互相串、也不污染默认槽。
#[test]
fn scoped_recorder_isolated_between_parallel_scopes() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    let default_slot = Arc::new(Recorder::new("default"));
    principal_sessions::install_principal_session_recorder(default_slot.clone());

    let rec_a = Arc::new(Recorder::new("A"));
    let rec_b = Arc::new(Recorder::new("B"));
    let barrier = Arc::new(Barrier::new(3));

    let handle_a = {
        let rec_a = rec_a.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let _scope = principal_sessions::enter_scope(rec_a.clone());
            barrier.wait();
            principal_sessions::record_principal_session("user-a", "session-a").expect("A 登记")
        })
    };
    let handle_b = {
        let rec_b = rec_b.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let _scope = principal_sessions::enter_scope(rec_b.clone());
            barrier.wait();
            principal_sessions::record_principal_session("user-b", "session-b").expect("B 登记")
        })
    };
    let handle_c = {
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            // **无作用域**的线程 ⇒ 必须走全局默认槽。
            barrier.wait();
            assert!(
                principal_sessions::scoped_recorder().is_none(),
                "无作用域的线程不得看见别人的作用域层"
            );
            principal_sessions::record_principal_session("user-c", "session-c").expect("C 登记")
        })
    };

    assert!(handle_a.join().expect("A 线程"), "A 必须登记成功");
    assert!(handle_b.join().expect("B 线程"), "B 必须登记成功");
    assert!(handle_c.join().expect("C 线程"), "C 必须登记成功");

    assert_eq!(
        rec_a.calls(),
        vec![("user-a".to_string(), "session-a".to_string())],
        "{} 用例的登记只许落在自己的写面上（不得串到 B / 默认槽）",
        rec_a.label()
    );
    assert_eq!(
        rec_b.calls(),
        vec![("user-b".to_string(), "session-b".to_string())],
        "{} 用例的登记只许落在自己的写面上（不得串到 A / 默认槽）",
        rec_b.label()
    );
    assert_eq!(
        default_slot.calls(),
        vec![("user-c".to_string(), "session-c".to_string())],
        "{} 必须只收到无作用域线程那一次登记（A/B 的作用域层不得溢出到这里）",
        default_slot.label()
    );
}

/// ⭐ **测试 2：Drop 不误杀 —— A 用例结束不会清掉 B 用例的 recorder**。
///
/// A 退出作用域（守卫 Drop）之后：
/// * **B 的作用域层必须还在** ⇒ B 仍登记到**自己的**写面；
/// * **全局默认槽必须还在** ⇒ A 自己退出作用域后仍能登记（走默认槽）。
#[test]
fn scope_guard_drop_does_not_kill_other_scope_or_default_slot() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    let default_slot = Arc::new(Recorder::new("default"));
    principal_sessions::install_principal_session_recorder(default_slot.clone());

    let rec_a = Arc::new(Recorder::new("A"));
    let rec_b = Arc::new(Recorder::new("B"));

    let (tx_ready_a, rx_ready_a) = mpsc::channel::<()>();
    let (tx_ready_b, rx_ready_b) = mpsc::channel::<()>();
    let (tx_dropped, rx_dropped) = mpsc::channel::<()>();

    let handle_b = {
        let rec_b = rec_b.clone();
        std::thread::spawn(move || {
            let _scope = principal_sessions::enter_scope(rec_b.clone());
            tx_ready_b.send(()).expect("B 就绪");
            // 等 A 结束（并已 Drop 守卫）⇒ 再读。
            rx_dropped.recv().expect("A 已结束");
            let scope_alive = principal_sessions::scoped_recorder().is_some();
            let ok = principal_sessions::record_principal_session("user-b", "session-b")
                .expect("B 登记");
            (ok, scope_alive)
        })
    };

    let handle_a = {
        let rec_a = rec_a.clone();
        std::thread::spawn(move || {
            let _scope = principal_sessions::enter_scope(rec_a.clone());
            tx_ready_a.send(()).expect("A 就绪");
            // 等 B 也进作用域 ⇒ 两个用例**同时**在场。
            rx_ready_b.recv().expect("B 就绪");
            drop(_scope);
            tx_dropped.send(()).expect("通知 B");
            // A 已退出作用域 ⇒ 读侧**回落全局默认槽** ⇒ 仍能登记。
            principal_sessions::record_principal_session("user-a", "session-a").expect("A 登记")
        })
    };

    rx_ready_a.recv().expect("A 就绪");
    assert!(handle_a.join().expect("A 线程"), "A 退出作用域后仍能登记");
    let (b_ok, b_scope_alive) = handle_b.join().expect("B 线程");

    assert!(b_ok, "B 必须仍能登记");
    assert!(
        b_scope_alive,
        "A 的守卫 Drop **只准弹自己那一层** ⇒ 不得误杀 B 的作用域层"
    );
    assert_eq!(
        rec_b.calls(),
        vec![("user-b".to_string(), "session-b".to_string())],
        "{} 的登记必须仍落在自己的写面上（A 的 Drop 不得改写它）",
        rec_b.label()
    );
    assert_eq!(
        default_slot.calls(),
        vec![("user-a".to_string(), "session-a".to_string())],
        "{} 必须仍然可用（A 的守卫 Drop **只准弹自己那一层** ⇒ 不得清全局默认槽）",
        default_slot.label()
    );
    assert!(
        rec_a.calls().is_empty(),
        "{} 在作用域内没有登记过（退出作用域后才登记 ⇒ 走默认槽）",
        rec_a.label()
    );
}

/// ⭐ **测试 3：原语义不回退 —— 单线程 / 普通场景行为完全不变**。
///
/// 装入 ⇒ 登记；空主体 / 空会话 ⇒ 不登记（fail-closed）；卸下 ⇒ 回到未装入。
#[test]
fn default_slot_semantics_unchanged() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    // 未装入 ⇒ 不登记。
    assert!(
        principal_sessions::current_recorder().is_none(),
        "未装入 ⇒ 读侧必须拿到 None"
    );
    assert!(
        !principal_sessions::record_principal_session("user-a", "session-a").expect("未装入"),
        "未装入 ⇒ 不得登记"
    );

    // 装入 ⇒ 登记。
    let recorder = Arc::new(Recorder::new("default"));
    principal_sessions::install_principal_session_recorder(recorder.clone());
    assert!(principal_sessions::current_recorder().is_some());
    assert!(principal_sessions::default_recorder().is_some());
    assert!(
        principal_sessions::scoped_recorder().is_none(),
        "没进作用域 ⇒ 作用域内槽必须为空"
    );
    assert!(principal_sessions::record_principal_session("user-a", "session-a").expect("登记"));
    assert_eq!(recorder.calls().len(), 1);

    // 空主体 / 空会话 ⇒ 不登记（fail-closed：宁可归属不可判定，也不要错归属）。
    assert!(!principal_sessions::record_principal_session("", "session-a").expect("空主体"));
    assert!(!principal_sessions::record_principal_session("user-a", "  ").expect("空会话"));
    assert_eq!(
        recorder.calls().len(),
        1,
        "{} : 空主体 / 空会话不得产生登记",
        recorder.label()
    );

    // 卸下 ⇒ 回到「未装入 ⇒ 不登记」。
    principal_sessions::clear_default_principal_session_recorder();
    assert!(principal_sessions::current_recorder().is_none());
    assert!(principal_sessions::default_recorder().is_none());
    assert!(!principal_sessions::record_principal_session("user-a", "session-a").expect("已卸下"));
}

/// ⭐⭐ **测试 4：跨线程传递不破 —— 装配在主线程、读在 worker 线程**。
///
/// 这是**生产读侧**的同构场景：CLI 在**启动线程**装入写面 ⇒ HTTP handler 在
/// **tokio worker 线程**读 ⇒ ⇒ 必须读得到（命中**全局默认槽**）。
///
/// ⚠️ **真跨线程**（`std::thread::spawn`）⇒ 不是同线程模拟。
#[test]
fn default_slot_visible_from_other_thread() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    let recorder = Arc::new(Recorder::new("default"));
    // **装配在当前（主）线程**。
    principal_sessions::install_principal_session_recorder(recorder.clone());
    let install_thread = std::thread::current().id();

    // **读在另一个真线程**。
    let handle = {
        std::thread::spawn(move || {
            let read_thread = std::thread::current().id();
            let has_scope = principal_sessions::scoped_recorder().is_some();
            let ok = principal_sessions::record_principal_session("user-x", "session-x")
                .expect("worker 登记");
            (ok, read_thread, has_scope)
        })
    };
    let (ok, read_thread, worker_has_scope) = handle.join().expect("worker 线程");

    assert_ne!(
        read_thread, install_thread,
        "本用例必须是**真跨线程**（装配线程与读线程不得相同）"
    );
    assert!(
        ok,
        "worker 线程必须读得到全局默认槽（否则生产归属会被**静默丢弃**）"
    );
    assert!(
        !worker_has_scope,
        "worker 线程没有作用域 ⇒ 读侧必须走全局默认槽"
    );
    assert_eq!(
        recorder.calls(),
        vec![("user-x".to_string(), "session-x".to_string())],
        "{} : 跨线程那次登记必须真的落到写面上",
        recorder.label()
    );
}

/// ⭐ **测试 5：旧名薄壳 —— `clear_principal_session_recorder()` 仍可用**。
///
/// 语义 = 清**默认槽**；⇒ **不动作用域内槽**（作用域槽由守卫按层释放）。
#[test]
#[allow(deprecated)]
fn deprecated_clear_shell_clears_default_slot() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    let recorder = Arc::new(Recorder::new("default"));
    principal_sessions::install_principal_session_recorder(recorder.clone());
    assert!(principal_sessions::current_recorder().is_some());

    // 旧名薄壳 ⇒ 转发到「清默认槽」。
    principal_sessions::clear_principal_session_recorder();
    assert!(
        principal_sessions::current_recorder().is_none(),
        "旧名薄壳的语义 = 清**默认槽**"
    );
    assert!(principal_sessions::default_recorder().is_none());

    // 薄壳**不动作用域内槽**。
    let scoped = Arc::new(Recorder::new("scoped"));
    let _scope = principal_sessions::enter_scope(scoped.clone());
    principal_sessions::install_principal_session_recorder(recorder.clone());
    principal_sessions::clear_principal_session_recorder();
    assert!(
        principal_sessions::default_recorder().is_none(),
        "旧名薄壳必须清掉默认槽"
    );
    assert!(
        principal_sessions::scoped_recorder().is_some(),
        "旧名薄壳**不得**动作用域内槽"
    );
    assert!(principal_sessions::record_principal_session("user-s", "session-s").expect("登记"));
    assert_eq!(
        scoped.calls(),
        vec![("user-s".to_string(), "session-s".to_string())],
        "{} : 作用域内的登记必须仍落在自己的写面上",
        scoped.label()
    );
}

/// ⭐ **补充：嵌套作用域取栈顶（最内层）**。
#[test]
fn nested_scopes_use_innermost_layer() {
    let _lock = lock_default_slot();
    principal_sessions::clear_default_principal_session_recorder();

    let outer = Arc::new(Recorder::new("outer"));
    let inner = Arc::new(Recorder::new("inner"));
    {
        let _outer_scope = principal_sessions::enter_scope(outer.clone());
        assert!(principal_sessions::record_principal_session("user-o", "session-o").expect("外层"));
        {
            let _inner_scope = principal_sessions::enter_scope(inner.clone());
            assert!(
                principal_sessions::record_principal_session("user-i", "session-i").expect("内层")
            );
        }
        // 内层守卫已 Drop ⇒ 回落**外层**（不是默认槽）。
        assert!(
            principal_sessions::record_principal_session("user-o2", "session-o2").expect("外层")
        );
    }
    // 外层守卫已 Drop ⇒ 回落默认槽（未装入 ⇒ 不登记）。
    assert!(
        !principal_sessions::record_principal_session("user-x", "session-x").expect("无作用域"),
        "两层作用域都退出后必须回落默认槽（未装入 ⇒ 不登记）"
    );

    assert_eq!(outer.calls().len(), 2, "{} : 外层两次", outer.label());
    assert_eq!(inner.calls().len(), 1, "{} : 内层一次", inner.label());
}
