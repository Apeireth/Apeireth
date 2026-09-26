//! 全双工打断响应 (Barge-in) 基准: 流原子取消 + `tokio::Notify` 广播到观察者。
//!
//! 口径 (与 README「Stream cancellation + tokio::Notify broadcast」行同义):
//! 每个操作 = 一次打断**往返**: 预注册流会话并让 1 个等待者停在
//! `wait_for_interrupt` 上 (停泊 2 ms 不计时) → 计时起点 → `interrupt()`
//! (原子置位 + 原因记账 + notify_waiters) → 等待者被唤醒并回报观察到打断
//! (oneshot) → 计时终点。注册/停泊/清理不计时。异步多线程 tokio 运行时
//! (广播唤醒含任务调度开销 = 真实端到端语义)。
//!
//! 诚实边界: 等待者停泊依赖 2 ms 让步窗口, 极端调度延迟下若未及停泊,
//! `wait_for_interrupt` 的入口检查会立即返回 (仍是正确语义, 只是走的快路径);
//! 单等待者口径, 多等待者广播扇出成本随等待者数线性增长。

use std::hint::black_box;
use std::time::{Duration, Instant};

use apeireth_gateway::{BargeInController, InterruptReason};

use crate::assembly::tokio_runtime;
use crate::support::{BenchConfig, Outcome, Target};

/// 每操作打断往返数 (batch)。
const BATCH: usize = 1;

/// 运行 Barge-in 打断基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let rt = tokio_runtime();
    let controller = BargeInController::new();
    let samples = cfg.samples(200);
    let warmup = cfg.samples(10);
    let counter = std::cell::Cell::new(0u64);

    let mut run_cycle = || -> f64 {
        let n = counter.get();
        counter.set(n + 1);
        let session = format!("bench-sess-{n}");
        let handle = controller.register_stream(&session);
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let waiter_handle = handle.clone();
        rt.spawn(async move {
            waiter_handle.wait_for_interrupt().await;
            let _ = tx.send(());
        });
        // 停泊窗口 (不计时): 让等待者挂上 notified()。
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(2)).await });

        let start = Instant::now();
        let hit = controller.interrupt(&session, InterruptReason::VoiceBargeIn);
        assert!(hit, "打断必须命中活跃流");
        rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(2), rx)
                .await
                .expect("等待者必须观察到打断")
                .expect("等待者回报通道");
        });
        let ns = start.elapsed().as_nanos() as f64;
        controller.cleanup(&session);
        black_box(handle.is_interrupted());
        ns
    };

    for _ in 0..warmup {
        run_cycle();
    }
    let mut samples_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        samples_ns.push(run_cycle());
    }

    vec![Outcome::from_ns(
        "barge-in-roundtrip",
        "全双工打断响应 (流取消 + Notify 广播)",
        "interrupt() 原子取消 + notify_waiters 广播 → 等待者 observe 到打断 的端到端往返单次",
        "us",
        BATCH,
        warmup,
        samples_ns,
        Some(Target {
            label: "< 1.0 ms".to_string(),
            value: 1000.0,
        }),
        "含 tokio 任务唤醒调度; 会话注册/等待者停泊(2ms)/清理不计时; 单等待者口径; 多线程异步运行时",
    )]
}
