//! 并行调用分类调度的机制验收: 判定器失败关闭、互斥屏障、有界滚动池、
//! 模型序提交、取消补果、纯并行全速、混合流、零调用/全串行边界。
//!
//! 编排纪律: 执行体用信号量式手语 (通道 + 会合) 精确控制启动与完成次序,
//! 负面断言前留让步窗口, 不依赖脆弱的绝对时序。

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::deadline::live_deadline_timers;
use apeireth_orchestration::{
    CallScheduler, CancelKind, ConcurrencyClass, ConcurrencySafety, SafetyRule, SafetyWhitelist,
    ScheduledCall, SchedulerConfig, SchedulerConfigError, SlotOutcome,
};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};

/// 把一次执行装成 `'static` 结果 future, 供 [`apeireth_orchestration::CallRunner`] 交付。
fn boxed<R: Send + 'static>(
    future: impl Future<Output = R> + Send + 'static,
) -> Pin<Box<dyn Future<Output = R> + Send>> {
    Box::pin(future)
}

/// 只放行 `par` 工具的判定器; `excl` 一律失败关闭。
fn par_only_safety() -> Arc<dyn ConcurrencySafety> {
    Arc::new(SafetyWhitelist::new().allow(SafetyRule::for_tool("par")))
}

fn calls(specs: &[(&str, &str)]) -> Vec<ScheduledCall> {
    specs
        .iter()
        .enumerate()
        .map(|(index, (id, tool))| ScheduledCall::new(*id, *tool, json!({ "index": index })))
        .collect()
}

// ---------------------------------------------------------------------------
// 判定器
// ---------------------------------------------------------------------------

#[test]
fn classifier_fails_closed_without_explicit_allow() {
    let blank = SafetyWhitelist::new();
    assert!(
        !blank.is_concurrency_safe("read", &json!({ "path": "a" })),
        "空白名单不得放行任何调用"
    );
    assert_eq!(
        blank.classify("read", &json!({ "path": "a" })),
        ConcurrencyClass::Exclusive,
        "失败关闭即独占"
    );

    let whitelist = SafetyWhitelist::new()
        .allow(SafetyRule::for_tool("read"))
        .allow(SafetyRule::for_tool_when("fetch", |args| {
            args.get("method").and_then(serde_json::Value::as_str) == Some("GET")
        }));

    assert!(whitelist.is_concurrency_safe("read", &json!({ "path": "a" })));
    assert!(whitelist.is_concurrency_safe("fetch", &json!({ "method": "GET" })));
    assert!(
        !whitelist.is_concurrency_safe("fetch", &json!({ "method": "POST" })),
        "参数谓词不满足即失败关闭"
    );
    assert!(
        !whitelist.is_concurrency_safe("write", &json!({ "path": "a" })),
        "白名单外的工具不得放行"
    );
    // 判定只看 (工具, 参数): 同输入必须同判定, 调度才可复现。
    assert!(whitelist.is_concurrency_safe("read", &json!({ "path": "a" })));
    assert_eq!(
        whitelist.classify("read", &json!({ "path": "a" })),
        ConcurrencyClass::Parallel
    );
}

#[test]
fn scheduler_rejects_a_pool_that_admits_nothing() {
    let Err(error) = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 0,
            wind_down: Duration::from_millis(50),
        },
    ) else {
        panic!("并发上限为 0 的调度器必须被拒绝");
    };
    assert_eq!(error, SchedulerConfigError::NonPositiveParallelism);
}

// ---------------------------------------------------------------------------
// 批次调度
// ---------------------------------------------------------------------------

#[tokio::test]
async fn exclusive_call_is_a_mutual_exclusion_barrier() {
    // [并行, 独占, 并行]: 独占调用前后成串行屏障, 与在飞调用互斥。
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 2,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "par"), ("c1", "excl"), ("c2", "par")]);

    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let mut release_txs = Vec::new();
    let mut release_slots = Vec::new();
    for _ in 0..3 {
        let (tx, rx) = oneshot::channel();
        release_txs.push(Some(tx));
        release_slots.push(Some(rx));
    }
    let release_slots: Arc<Mutex<Vec<Option<oneshot::Receiver<()>>>>> =
        Arc::new(Mutex::new(release_slots));

    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let started_tx = started_tx.clone();
        let release_slots = Arc::clone(&release_slots);
        boxed(async move {
            started_tx.send(index).unwrap();
            let slot = release_slots.lock().unwrap()[index].take().unwrap();
            slot.await.ok();
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);

    // P0 先飞; 屏障未排空前 E1 与 P2 都不得启动。
    assert_eq!(started_rx.recv().await, Some(0));
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        started_rx.try_recv().is_err(),
        "独占调用等待期间不得启动任何其它调用"
    );

    release_txs[0].take().unwrap().send(()).unwrap();
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!((index, outcome.completed()), (0, Some(0)));

    // E1 独占执行: 只有它自己在飞, P2 仍卡在屏障后。
    assert_eq!(started_rx.recv().await, Some(1));
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        started_rx.try_recv().is_err(),
        "独占调用在飞时并行调用不得越过屏障"
    );

    release_txs[1].take().unwrap().send(()).unwrap();
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!((index, outcome.completed()), (1, Some(1)));

    assert_eq!(started_rx.recv().await, Some(2));
    release_txs[2].take().unwrap().send(()).unwrap();
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!((index, outcome.completed()), (2, Some(2)));
    assert!(batch.next_outcome().await.is_none());
}

#[tokio::test]
async fn rolling_pool_never_exceeds_its_bound() {
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 2,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "par"), ("c1", "par"), ("c2", "par"), ("c3", "par")]);

    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let runner_in_flight = Arc::clone(&in_flight);
    let runner_peak = Arc::clone(&peak);
    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let in_flight = Arc::clone(&runner_in_flight);
        let peak = Arc::clone(&runner_peak);
        boxed(async move {
            let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(25)).await;
            in_flight.fetch_sub(1, Ordering::SeqCst);
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);
    let mut delivered = Vec::new();
    while let Some((index, outcome)) = batch.next_outcome().await {
        delivered.push((index, outcome.completed().unwrap()));
    }

    assert_eq!(
        delivered,
        vec![(0, 0), (1, 1), (2, 2), (3, 3)],
        "超限排队不丢弃"
    );
    assert_eq!(
        peak.load(Ordering::SeqCst),
        2,
        "滚动池并发峰值恰为上限: 既不超发, 也滚满"
    );
}

#[tokio::test]
async fn results_commit_in_model_order_when_completion_order_differs() {
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 3,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "par"), ("c1", "par"), ("c2", "par")]);

    let completion_order: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let runner_order = Arc::clone(&completion_order);
    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let completion_order = Arc::clone(&runner_order);
        boxed(async move {
            // 完成次序被强制为 2, 1, 0 (倒序)。
            let delay = 60u64 - (index as u64) * 25;
            tokio::time::sleep(Duration::from_millis(delay)).await;
            completion_order.lock().unwrap().push(index);
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);
    let mut delivered = Vec::new();
    while let Some((index, outcome)) = batch.next_outcome().await {
        delivered.push((index, outcome.completed().unwrap()));
    }

    assert_eq!(
        *completion_order.lock().unwrap(),
        vec![2, 1, 0],
        "完成次序确实是倒序"
    );
    assert_eq!(
        delivered,
        vec![(0, 0), (1, 1), (2, 2)],
        "乱序完成也必须按模型原始顺序交付"
    );
}

#[tokio::test]
async fn abort_synthesizes_outcomes_and_winds_down_started_calls() {
    let baseline = live_deadline_timers();
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 1,
            wind_down: Duration::from_millis(150),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "par"), ("c1", "par"), ("c2", "par")]);

    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (release_tx, release_rx) = oneshot::channel::<()>();
    let release_rx = Arc::new(Mutex::new(Some(release_rx)));

    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let started_tx = started_tx.clone();
        let release_rx = Arc::clone(&release_rx);
        boxed(async move {
            started_tx.send(index).unwrap();
            match index {
                0 => {
                    let slot = release_rx.lock().unwrap().take().unwrap();
                    slot.await.ok();
                    0
                }
                // 永不完成: 收场窗口到期后必须被取消。
                1 => std::future::pending().await,
                _ => 2,
            }
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);

    assert_eq!(started_rx.recv().await, Some(0));
    release_tx.send(()).unwrap();
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!((index, outcome.completed()), (0, Some(0)));

    // 滚动启动 c1; c2 排队 (并发上限 1)。
    assert_eq!(started_rx.recv().await, Some(1));

    batch.abort();
    assert!(batch.is_aborted());
    assert_eq!(
        live_deadline_timers(),
        baseline + 1,
        "已启动调用的收场走统一超时件"
    );

    // 模型序交付: 已启动的补收场超时成因, 未启动的补未启动成因。
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!(
        (index, outcome.cancel_kind()),
        (1, Some(CancelKind::WindDownExpired))
    );
    let (index, outcome) = batch.next_outcome().await.unwrap();
    assert_eq!(
        (index, outcome.cancel_kind()),
        (2, Some(CancelKind::NotStarted))
    );
    assert!(batch.next_outcome().await.is_none(), "每个调用恰好一条结果");

    drop(batch);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(live_deadline_timers(), baseline, "收场窗口定时器随批次清理");
}

#[tokio::test]
async fn all_parallel_batch_runs_at_full_width() {
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 4,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "par"), ("c1", "par"), ("c2", "par"), ("c3", "par")]);

    // 四个调用全部会合: 只有四个同时在飞才可能到齐。
    let rendezvous = Arc::new(tokio::sync::Barrier::new(4));
    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let rendezvous = Arc::clone(&rendezvous);
        boxed(async move {
            rendezvous.wait().await;
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);
    let delivered = tokio::time::timeout(Duration::from_secs(5), async {
        let mut delivered = Vec::new();
        while let Some((index, outcome)) = batch.next_outcome().await {
            delivered.push((index, outcome.completed().unwrap()));
        }
        delivered
    })
    .await
    .expect("四个并行调用必须同时在飞才能会合");

    assert_eq!(delivered, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
}

#[tokio::test]
async fn mixed_stream_keeps_barriers_and_pool_bounds_together() {
    // [并行, 并行, 独占, 并行, 并行], 并发上限 2。
    let scheduler = CallScheduler::new(
        par_only_safety(),
        SchedulerConfig {
            max_parallel: 2,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[
        ("c0", "par"),
        ("c1", "par"),
        ("c2", "excl"),
        ("c3", "par"),
        ("c4", "par"),
    ]);

    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let runner_log = Arc::clone(&log);
    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let log = Arc::clone(&runner_log);
        boxed(async move {
            log.lock().unwrap().push(format!("start:{index}"));
            tokio::time::sleep(Duration::from_millis(30)).await;
            log.lock().unwrap().push(format!("end:{index}"));
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);
    let mut delivered = Vec::new();
    while let Some((index, outcome)) = batch.next_outcome().await {
        delivered.push((index, outcome.completed().unwrap()));
    }
    assert_eq!(delivered, vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]);

    let log = log.lock().unwrap().clone();
    let at = |needle: &str| {
        log.iter()
            .position(|entry| entry == needle)
            .unwrap_or_else(|| panic!("{needle} 不在事件日志中: {log:?}"))
    };

    // 同段并行: 首对与次对各自重叠。
    assert!(at("start:1") < at("end:0"), "并行段内必须真重叠");
    assert!(at("start:4") < at("end:3"), "屏障后的并行段内必须真重叠");
    // 独占屏障: 前段全定局才启动, 未收场后段不启动。
    assert!(at("end:0") < at("start:2") && at("end:1") < at("start:2"));
    assert!(at("end:2") < at("start:3") && at("end:2") < at("start:4"));
}

#[tokio::test]
async fn zero_call_batch_yields_no_outcomes() {
    let scheduler = CallScheduler::new(par_only_safety(), SchedulerConfig::default()).unwrap();
    let runner = Arc::new(|_index: usize, _call: &ScheduledCall| boxed(async { 0usize }));

    let mut batch = scheduler.start_batch(Vec::new(), runner);
    assert!(batch.is_empty());
    assert_eq!(batch.len(), 0);
    assert!(!batch.is_aborted());
    assert!(batch.next_outcome().await.is_none(), "零调用即刻收束");
}

#[tokio::test]
async fn all_exclusive_batch_runs_strictly_serial() {
    // 并发上限给到 4 也没用: 全独占 = 全串行。
    let scheduler = CallScheduler::new(
        Arc::new(SafetyWhitelist::new()),
        SchedulerConfig {
            max_parallel: 4,
            wind_down: Duration::from_millis(200),
        },
    )
    .unwrap();
    let batch_calls = calls(&[("c0", "excl"), ("c1", "excl"), ("c2", "excl")]);

    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let runner_log = Arc::clone(&log);
    let runner = Arc::new(move |index: usize, _call: &ScheduledCall| {
        let log = Arc::clone(&runner_log);
        boxed(async move {
            log.lock().unwrap().push(format!("start:{index}"));
            tokio::time::sleep(Duration::from_millis(10)).await;
            log.lock().unwrap().push(format!("end:{index}"));
            index
        })
    });

    let mut batch = scheduler.start_batch(batch_calls, runner);
    let mut delivered = Vec::new();
    while let Some((index, outcome)) = batch.next_outcome().await {
        delivered.push((index, outcome.completed().unwrap()));
    }

    assert_eq!(delivered, vec![(0, 0), (1, 1), (2, 2)]);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["start:0", "end:0", "start:1", "end:1", "start:2", "end:2"],
        "全独占必须严格串行"
    );
}

#[tokio::test]
async fn slot_outcomes_expose_real_results_and_cancel_kinds() {
    let completed = SlotOutcome::<usize>::Completed(7);
    assert_eq!(completed.clone().completed(), Some(7));
    assert_eq!(completed.cancel_kind(), None);

    let cancelled = SlotOutcome::<usize>::Cancelled(CancelKind::NotStarted);
    assert_eq!(cancelled.clone().completed(), None);
    assert_eq!(cancelled.cancel_kind(), Some(CancelKind::NotStarted));
}
