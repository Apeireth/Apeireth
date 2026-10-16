//! 任务输出环 + 双游标 + 完成通知 的行为测试。
//!
//! 覆盖: 环有界淘汰与省略记账 / read_at 不推进消费游标 / 双游标独立 /
//! 完成通知恰好一次且先释放等待方再发事件 / awaited 去重 /
//! max_consecutive_wakes 封自激链 / 取消后已产出输出可读 /
//! 并发多观察者一消费者序列化正确。

use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use apeireth_orchestration::job_board::{
    BoardConfig, BoardError, CompletionListener, JobBoard, JobHandle, JobId, JobOutcome, JobStatus,
    OwnerId, SegmentState, WakeDecision,
};
use apeireth_orchestration::job_ring::JobRing;

/// 计数唤醒: 记录唤醒次数与"唤醒/事件"投递顺序。
#[derive(Default)]
struct CountingWake {
    count: AtomicUsize,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl Wake for CountingWake {
    fn wake(self: Arc<Self>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.log.lock().unwrap().push("wake");
    }
}

fn counter(log: &Arc<Mutex<Vec<&'static str>>>) -> Arc<CountingWake> {
    Arc::new(CountingWake {
        count: AtomicUsize::new(0),
        log: Arc::clone(log),
    })
}

fn count(counter: &Arc<CountingWake>) -> usize {
    counter.count.load(Ordering::SeqCst)
}

#[test]
fn ring_evicts_oldest_chunks_when_over_capacity_and_marks_lossy() {
    let mut ring = JobRing::new(10);
    assert_eq!(ring.push(b"aaaaa"), 0);
    assert_eq!(ring.push(b"bbbbb"), 5);
    assert!(!ring.is_lossy(), "未超限不淘汰");
    assert_eq!(ring.push(b"ccccc"), 10); // 总量 15 > 10: 丢最旧整段
    assert!(ring.is_lossy(), "超限淘汰必须留下 lossy 标志");
    assert_eq!(ring.omitted_chunks(), 1);
    assert_eq!(ring.omitted_bytes(), 5);
    assert_eq!(ring.base_offset(), 5);
    assert_eq!(ring.end_offset(), 15);
    assert_eq!(ring.retained_bytes(), 10, "总量上限硬性成立");
    let slice = ring.read_at(0);
    assert!(slice.omitted_before, "被淘汰区间的读取显式标注省略");
    assert_eq!(slice.start, 5);
    assert_eq!(slice.bytes, b"bbbbbccccc");
}

#[test]
fn ring_trims_front_of_a_single_oversize_chunk() {
    let mut ring = JobRing::new(10);
    ring.push(b"0123456789abcdef"); // 单段 16 字节 > 上限 10: 截弃最旧前缀
    assert!(ring.is_lossy());
    assert_eq!(ring.omitted_chunks(), 0, "截前缀不是整段丢弃");
    assert_eq!(ring.omitted_bytes(), 6);
    assert_eq!(ring.base_offset(), 6);
    assert_eq!(ring.retained_bytes(), 10);
    assert_eq!(ring.read_at(0).bytes, b"6789abcdef");
}

#[test]
fn read_at_never_advances_the_consumer_cursor() {
    let mut ring = JobRing::new(100);
    ring.push(b"hello-world");
    assert_eq!(ring.consumer_cursor(), 0);
    let first = ring.read_at(0);
    assert_eq!(first.bytes, b"hello-world");
    assert_eq!(ring.consumer_cursor(), 0, "read_at 绝不推进消费游标");
    let second = ring.read_at(6);
    assert_eq!(second.bytes, b"world");
    assert_eq!(ring.consumer_cursor(), 0);
    let _ = ring.consume(3);
    assert_eq!(ring.consumer_cursor(), 3);
    ring.read_at(0);
    assert_eq!(ring.consumer_cursor(), 3, "观察读取不动已有消费进度");
}

#[test]
fn dual_cursors_advance_independently() {
    let mut ring = JobRing::new(100);
    ring.push(b"0123456789");
    // 观察游标 4 先读 (不挪消费游标)
    assert_eq!(ring.read_at(4).bytes, b"456789");
    // 消费游标独立推进
    assert_eq!(ring.consume(3).bytes, b"012");
    assert_eq!(ring.consumer_cursor(), 3);
    // 消费推进不挪动观察游标; 同一偏移可反复观察
    assert_eq!(ring.read_at(4).bytes, b"456789");
    assert_eq!(ring.read_at(4).bytes, b"456789");
    // 消费走完剩余
    assert_eq!(ring.consume(100).bytes, b"3456789");
    assert_eq!(ring.consumer_cursor(), 10);
}

#[test]
fn completion_notification_is_exactly_once_and_releases_waiters_before_events() {
    let board = JobBoard::new(BoardConfig::default());
    let handle = board.open_job(OwnerId::new("notify-owner"), 1);
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let waker_counter = counter(&log);
    let waker = Waker::from(Arc::clone(&waker_counter));
    let mut cx = Context::from_waker(&waker);
    let mut waiter = pin!(board.wait_for_completion(handle.id).unwrap());
    assert!(waiter.as_mut().poll(&mut cx).is_pending());
    let log_for_listener = Arc::clone(&log);
    let listener: CompletionListener = Arc::new(move |_outcome: &JobOutcome| {
        log_for_listener.lock().unwrap().push("event");
    });
    board.on_completion(handle.id, listener).unwrap();

    let report = board.settle(&handle).unwrap();
    assert_eq!(report.waiters_released, 1);
    assert_eq!(report.outcome.status, JobStatus::Completed);
    assert_eq!(count(&waker_counter), 1);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["wake", "event"],
        "任务结算先释放等待方, 再发事件"
    );

    // 重复结算被终态挡住, 不产生第二次通知
    assert!(matches!(
        board.settle(&handle),
        Err(BoardError::AlreadySettled(_))
    ));
    assert!(matches!(
        board.cancel(&handle),
        Err(BoardError::AlreadySettled(_))
    ));
    assert_eq!(count(&waker_counter), 1);
    assert_eq!(log.lock().unwrap().len(), 2);

    // 已唤醒的等待方再轮询直接取结果, 不重复唤醒
    assert!(matches!(waiter.as_mut().poll(&mut cx), Poll::Ready(_)));
    assert_eq!(count(&waker_counter), 1);
}

#[test]
fn awaited_marker_deduplicates_notifications_per_waiter() {
    let board = JobBoard::new(BoardConfig::default());
    let handle = board.open_job(OwnerId::new("dedup-owner"), 0);
    let mut first = pin!(board.wait_for_completion(handle.id).unwrap());
    let mut second = pin!(board.wait_for_completion(handle.id).unwrap());
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let counter_first = counter(&log);
    let counter_second = counter(&log);
    let waker_first = Waker::from(Arc::clone(&counter_first));
    let waker_second = Waker::from(Arc::clone(&counter_second));
    let mut cx_first = Context::from_waker(&waker_first);
    let mut cx_second = Context::from_waker(&waker_second);

    // 同一等待方反复轮询只登记一个等待槽 (awaited 标记去重)
    for _ in 0..5 {
        assert!(first.as_mut().poll(&mut cx_first).is_pending());
        assert!(second.as_mut().poll(&mut cx_second).is_pending());
    }
    let report = board.settle(&handle).unwrap();
    assert_eq!(report.waiters_released, 2, "轮询次数不放大通知次数");
    assert_eq!(count(&counter_first), 1);
    assert_eq!(count(&counter_second), 1);
}

#[test]
fn max_consecutive_wakes_seals_the_self_excitation_chain() {
    let board = JobBoard::new(BoardConfig {
        ring_capacity_bytes: 1024,
        max_consecutive_wakes: 2,
    });
    let owner = OwnerId::new("idle-owner");
    // 自激链: 每次结算都对空闲 owner 发 follow-up 唤醒, 唤醒后无进展又结算一次
    for _ in 0..2 {
        let handle = board.open_job(owner.clone(), 0);
        assert_eq!(
            board.settle(&handle).unwrap().owner_wake,
            Some(WakeDecision::Proceed)
        );
    }
    assert_eq!(board.consecutive_wakes(&owner), 2);
    // 超上限即抑制, 且计数不再增长 (自激链被封)
    for _ in 0..2 {
        let handle = board.open_job(owner.clone(), 0);
        assert_eq!(
            board.settle(&handle).unwrap().owner_wake,
            Some(WakeDecision::Suppressed)
        );
    }
    assert_eq!(board.consecutive_wakes(&owner), 2);
    // 真实进展清零计数, 唤醒恢复
    board.record_progress(&owner);
    let handle = board.open_job(owner.clone(), 0);
    assert_eq!(
        board.settle(&handle).unwrap().owner_wake,
        Some(WakeDecision::Proceed)
    );
}

#[test]
fn busy_owner_gets_no_follow_up_wake() {
    let board = JobBoard::new(BoardConfig::default());
    let owner = OwnerId::new("busy-owner");
    let running = board.open_job(owner.clone(), 1);
    assert_eq!(board.start_next_segment(&running).unwrap(), Some(0));
    // owner 还有已启动段: 不发 follow-up 唤醒
    let other = board.open_job(owner.clone(), 0);
    assert_eq!(board.settle(&other).unwrap().owner_wake, None);
    // 段完成、任务结算后 owner 空闲, follow-up 唤醒恢复
    board.finish_segment(&running, 0).unwrap();
    board.settle(&running).unwrap();
    let third = board.open_job(owner.clone(), 0);
    assert_eq!(
        board.settle(&third).unwrap().owner_wake,
        Some(WakeDecision::Proceed)
    );
}

#[test]
fn cancel_skips_unstarted_segments_and_keeps_produced_output_readable() {
    let board = JobBoard::new(BoardConfig {
        ring_capacity_bytes: 4096,
        max_consecutive_wakes: 3,
    });
    let owner = OwnerId::new("cancel-owner");
    let handle = board.open_job(owner, 2);
    assert_eq!(board.start_next_segment(&handle).unwrap(), Some(0));
    board.append(&handle, b"partial output").unwrap();

    let report = board.cancel(&handle).unwrap();
    assert_eq!(report.outcome.status, JobStatus::Cancelled);
    assert_eq!(report.outcome.skipped_segments, 1, "取消只影响未启动的段");
    assert_eq!(
        board.segment_states(handle.id).unwrap(),
        vec![SegmentState::Started, SegmentState::Skipped],
        "已启动段不被追溯"
    );

    // 已产出的输出保留在环内可读 (观察 + 消费两条路都通)
    assert_eq!(
        board.read_at(handle.id, 0).unwrap().bytes,
        b"partial output"
    );
    assert_eq!(
        board.consume(handle.id, 64).unwrap().bytes,
        b"partial output"
    );

    // 终态任务不再接受新输出、不再启动段
    assert!(matches!(
        board.append(&handle, b"late"),
        Err(BoardError::AlreadySettled(_))
    ));
    assert!(matches!(
        board.start_next_segment(&handle),
        Err(BoardError::AlreadySettled(_))
    ));
}

#[test]
fn mutation_requires_the_owning_handle_while_observation_is_by_id() {
    let board = JobBoard::new(BoardConfig::default());
    let handle = board.open_job(OwnerId::new("owner-a"), 1);
    let impostor = JobHandle::new(handle.id, OwnerId::new("owner-b"));
    board.append(&handle, b"data").unwrap();

    // 观察按 id: 无需句柄
    assert_eq!(board.read_at(handle.id, 0).unwrap().bytes, b"data");
    assert_eq!(board.ring_stats(handle.id).unwrap().end_offset, 4);

    // 写入 / 取消 / 结算认句柄: owner 不符一律拒绝
    assert!(matches!(
        board.append(&impostor, b"x"),
        Err(BoardError::WrongOwner { .. })
    ));
    assert!(matches!(
        board.cancel(&impostor),
        Err(BoardError::WrongOwner { .. })
    ));
    assert!(matches!(
        board.settle(&impostor),
        Err(BoardError::WrongOwner { .. })
    ));

    // 未知 id 明确报错
    assert!(matches!(
        board.read_at(JobId(999), 0),
        Err(BoardError::UnknownJob(_))
    ));

    // 真 owner 可结算
    assert_eq!(
        board.settle(&handle).unwrap().outcome.status,
        JobStatus::Completed
    );
}

#[test]
fn late_waiter_and_late_listener_stay_exactly_once() {
    let board = JobBoard::new(BoardConfig::default());
    let handle = board.open_job(OwnerId::new("late-owner"), 0);
    board.settle(&handle).unwrap();

    // 结算后登记的等待方直接取结果, 不产生新唤醒
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let waker_counter = counter(&log);
    let waker = Waker::from(Arc::clone(&waker_counter));
    let mut cx = Context::from_waker(&waker);
    let mut waiter = pin!(board.wait_for_completion(handle.id).unwrap());
    assert!(matches!(
        waiter.as_mut().poll(&mut cx),
        Poll::Ready(outcome) if outcome.status == JobStatus::Completed
    ));
    assert_eq!(count(&waker_counter), 0);

    // 结算后登记的监听器立即、且只调用一次
    let events = Arc::new(AtomicUsize::new(0));
    let events_for_listener = Arc::clone(&events);
    let listener: CompletionListener = Arc::new(move |_outcome: &JobOutcome| {
        events_for_listener.fetch_add(1, Ordering::SeqCst);
    });
    board.on_completion(handle.id, listener).unwrap();
    assert_eq!(events.load(Ordering::SeqCst), 1);
}

#[test]
fn concurrent_observers_and_one_consumer_serialize_correctly() {
    const CHUNKS: usize = 200;
    let board = Arc::new(JobBoard::new(BoardConfig {
        ring_capacity_bytes: 1 << 20,
        max_consecutive_wakes: 3,
    }));
    let handle = board.open_job(OwnerId::new("stream-owner"), 1);
    let expected: Vec<u8> = (0..CHUNKS)
        .flat_map(|index| format!("chunk-{index:03};").into_bytes())
        .collect();
    let done = Arc::new(AtomicBool::new(false));

    // 一消费者 (消费游标推进)
    let consumer_board = Arc::clone(&board);
    let consumer_id = handle.id;
    let consumer_expected = expected.clone();
    let consumer_done = Arc::clone(&done);
    let consumer = std::thread::spawn(move || {
        let mut collected = Vec::new();
        while collected.len() < consumer_expected.len() {
            let slice = consumer_board.consume(consumer_id, 37).unwrap();
            if slice.is_empty() {
                if consumer_done.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::yield_now();
                continue;
            }
            collected.extend_from_slice(&slice.bytes);
        }
        collected
    });

    // 三观察者 (read_at, 互不干扰)
    let observers: Vec<_> = (0..3)
        .map(|_| {
            let observer_board = Arc::clone(&board);
            let observer_id = handle.id;
            let observer_done = Arc::clone(&done);
            let observer_target = expected.len();
            std::thread::spawn(move || {
                let mut collected = Vec::new();
                let mut offset = 0u64;
                while collected.len() < observer_target {
                    let slice = observer_board.read_at(observer_id, offset).unwrap();
                    if slice.is_empty() {
                        if observer_done.load(Ordering::SeqCst) {
                            break;
                        }
                        std::thread::yield_now();
                        continue;
                    }
                    offset += slice.bytes.len() as u64;
                    collected.extend_from_slice(&slice.bytes);
                }
                collected
            })
        })
        .collect();

    // 生产者 (主线程) 逐段追加
    for chunk in expected.chunks(17) {
        board.append(&handle, chunk).unwrap();
    }
    done.store(true, Ordering::SeqCst);

    let consumed = consumer.join().unwrap();
    assert_eq!(consumed, expected, "消费者经消费游标读到完整字节序列");
    let stats = board.ring_stats(handle.id).unwrap();
    assert_eq!(stats.consumer_cursor, expected.len() as u64);
    assert!(!stats.lossy, "容量充足时不得误丢");
    for (index, observer) in observers.into_iter().enumerate() {
        assert_eq!(
            observer.join().unwrap(),
            expected,
            "观察者 {index} 读到完整字节序列且互不干扰"
        );
    }
}
