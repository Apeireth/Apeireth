//! # State Sharing Demo — 1 个完整 9 器官并发访问例子
//!
//! per 任务 spec: "1 状态共享例子" (1 完整 9 器官 state 共享演示).
//!
//! ## 演示
//!
//! 1. 创建 `OrganStateRegistry` (9 器官默认 MutexState / RwLockState)
//! 2. 演示 3 模式基础:
//!    - `OnceLockState<Config>` — 进程级配置
//!    - `MutexState<Counter>` — 跨线程计数器
//!    - `RwLockState<History>` — 跨线程读多写少
//! 3. 演示 9 器官并发读写 (3 个线程各做不同器官操作)
//! 4. 演示 `SharedState<T>` trait dispatch (3 模式 enum match)
//!
//! ## 跑
//!
//! ```bash
//! cargo run -p apeireth-state --example state_sharing_demo
//! ```

use apeireth_state::{
    BodyStub, BrainStub, EarStub, EyeStub, HandStub, HeartStub, MemoryStub, MindStub, MutexState,
    OnceLockState, Organ, OrganStateRegistry, RwLockState, SharedState, SharedStateMode,
    VoiceStub,
};
use std::sync::Arc;
use std::thread;

// 演示用业务类型
#[derive(Debug, Clone, PartialEq, Eq)]
struct AppConfig {
    platform: String,
    version: String,
}

#[derive(Debug, Default, Clone)]
struct CallCounter {
    count: u32,
    last_provider: String,
}

#[derive(Debug, Default, Clone)]
struct History {
    entries: Vec<String>,
}

fn main() {
    println!("===========================================");
    println!("apeireth-state: State Sharing Demo");
    println!("===========================================\n");

    // ------------------------------------------------------------------
    // 演示 1: OnceLockState<Config> — 进程级配置 (启动时 init 一次)
    // ------------------------------------------------------------------
    println!("--- Demo 1: OnceLockState<Config> (模式 1: 进程全局 lazy init) ---");
    let config_state: OnceLockState<AppConfig> = OnceLockState::new();
    println!("  Before init: is_initialized={}", config_state.is_initialized());

    config_state
        .init(AppConfig {
            platform: "apeireth".to_string(),
            version: "0.1.0".to_string(),
        })
        .expect("init should succeed");

    println!("  After init: is_initialized={}", config_state.is_initialized());
    let cfg = config_state.get().expect("get should be Some after init");
    println!("  Config: platform={}, version={}\n", cfg.platform, cfg.version);

    // ------------------------------------------------------------------
    // 演示 2: MutexState<Counter> — 跨线程互斥计数器
    // ------------------------------------------------------------------
    println!("--- Demo 2: MutexState<Counter> (模式 2: 跨线程互斥) ---");
    let counter = MutexState::new(CallCounter::default());
    let counter_handle = counter.handle();

    // 5 个线程并发 +1
    let mut handles = vec![];
    for i in 0..5 {
        let h = Arc::clone(&counter_handle);
        handles.push(thread::spawn(move || {
            let mut g = h.lock().expect("lock should succeed");
            g.count += 1;
            g.last_provider = format!("provider_{i}");
        }));
    }
    for h in handles {
        h.join().expect("thread should not panic");
    }
    let g = counter.read().expect("read should succeed");
    println!("  5 threads +1, final count={}, last_provider={}\n", g.count, g.last_provider);

    // ------------------------------------------------------------------
    // 演示 3: RwLockState<History> — 读多写少
    // ------------------------------------------------------------------
    println!("--- Demo 3: RwLockState<History> (模式 3: 跨线程读写锁) ---");
    let history = RwLockState::new(History::default());
    let hist_handle = history.handle();

    // 3 个 writer 线程各加 2 条
    let mut writers = vec![];
    for w in 0..3 {
        let h = Arc::clone(&hist_handle);
        writers.push(thread::spawn(move || {
            let mut g = h.write().expect("write should succeed");
            g.entries.push(format!("writer_{w}_entry_1"));
            g.entries.push(format!("writer_{w}_entry_2"));
        }));
    }
    for w in writers {
        w.join().expect("writer thread should not panic");
    }

    // 5 个 reader 线程各读 1 次
    let mut readers = vec![];
    for _r in 0..5 {
        let h = Arc::clone(&hist_handle);
        readers.push(thread::spawn(move || {
            let g = h.read().expect("read should succeed");
            g.entries.len()
        }));
    }
    let mut total_read = 0;
    for r in readers {
        total_read += r.join().expect("reader thread should not panic");
    }
    let g = history.read().expect("read should succeed");
    println!(
        "  3 writers × 2 entries = 6, 5 readers total_read={}, actual entries={}\n",
        total_read / 5,
        g.entries.len()
    );

    // ------------------------------------------------------------------
    // 演示 4: OrganStateRegistry 9 器官聚合
    // ------------------------------------------------------------------
    println!("--- Demo 4: OrganStateRegistry 9 器官聚合 ---");
    let registry = OrganStateRegistry::new();
    let names = registry.organ_names();
    let chars = registry.ascii_chars();
    let summary = registry.mode_summary();
    println!("  9 器官 (name_zh, ascii, mode):");
    for i in 0..9 {
        let _organ = Organ::from_u8(i as u8).expect("0-8 valid");
        println!(
            "    [{}] {} ({}) = {:?}",
            i,
            chars[i],
            names[i],
            summary[i]
        );
    }
    println!();

    // ------------------------------------------------------------------
    // 演示 5: 9 器官并发访问 (3 线程各做不同器官操作)
    // ------------------------------------------------------------------
    println!("--- Demo 5: 9 器官并发访问 (3 线程) ---");
    let reg_arc = Arc::new(registry);
    let mut workers = vec![];

    // Thread 1: 操作 heart (MutexState) 60Hz tick (模拟)
    let reg1 = Arc::clone(&reg_arc);
    workers.push(thread::spawn(move || {
        for _ in 0..5 {
            let mut g = reg1.heart.write().expect("heart write should succeed");
            g._marker = g._marker.wrapping_add(1);
        }
        "heart 5 ticks OK"
    }));

    // Thread 2: 操作 memory (RwLockState) append 3 条
    let reg2 = Arc::clone(&reg_arc);
    workers.push(thread::spawn(move || {
        for _i in 0..3 {
            let mut g = reg2.memory.write().expect("memory write should succeed");
            g._marker = g._marker.wrapping_add(1);
        }
        "memory 3 appends OK"
    }));

    // Thread 3: 操作 brain + mind 读 (验证 RwLock 读并发)
    let reg3 = Arc::clone(&reg_arc);
    workers.push(thread::spawn(move || {
        let _b = reg3.brain.read().expect("brain read should succeed");
        let _m = reg3.mind.read().expect("mind read should succeed");
        "brain + mind reads OK"
    }));

    for w in workers {
        let msg = w.join().expect("worker should not panic");
        println!("  Thread: {msg}");
    }
    println!();

    // ------------------------------------------------------------------
    // 演示 6: SharedState<T> trait dispatch (3 模式 enum match)
    // ------------------------------------------------------------------
    println!("--- Demo 6: SharedState<T> trait dispatch (3 模式 enum match) ---");

    // 构造 3 模式各 1 个 state
    let once_state: OnceLockState<u32> = OnceLockState::new();
    once_state.init(123).expect("init");
    let mutex_state: MutexState<u32> = MutexState::new(456);
    let rw_state: RwLockState<u32> = RwLockState::new(789);

    // 用 enum dispatch 读 3 个 state
    let modes = [SharedStateMode::OnceLock, SharedStateMode::Mutex, SharedStateMode::RwLock];
    for m in &modes {
        let v: u32 = match m {
            SharedStateMode::OnceLock => *once_state.read().expect("once read"),
            SharedStateMode::Mutex => *mutex_state.read().expect("mutex read"),
            SharedStateMode::RwLock => *rw_state.read().expect("rw read"),
        };
        println!("  mode={m:?} -> value={v}");
    }
    println!();

    // ------------------------------------------------------------------
    // 演示 7: 9 OrganStub 编译期 hardcode (per 借鉴 #1 sister 报告 9 organ)
    // ------------------------------------------------------------------
    println!("--- Demo 7: 9 OrganStub 编译期 hardcode ---");
    let stubs: [(&str, &dyn std::fmt::Debug); 9] = [
        ("HeartStub", &HeartStub::new()),
        ("BrainStub", &BrainStub::new()),
        ("HandStub", &HandStub::new()),
        ("EyeStub", &EyeStub::new()),
        ("EarStub", &EarStub::new()),
        ("MemoryStub", &MemoryStub::new()),
        ("VoiceStub", &VoiceStub::new()),
        ("BodyStub", &BodyStub::new()),
        ("MindStub", &MindStub::new()),
    ];
    for (name, stub) in &stubs {
        println!("  {name:13} = {stub:?}");
    }
    println!();

    println!("===========================================");
    println!("apeireth-state: Demo complete (7 sections, 9 organ cross-mode)");
    println!("===========================================");
}
