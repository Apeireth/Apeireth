#![no_main]

//! fuzz: **守护看家面（`tool.guard_watch`）** —— 「只看不动 / 缺读数不编造」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/device/guard_watch.rs`
//! （`GuardCondition::evaluate` / `GuardMatch` / `GuardWindow::validate` /
//! `GuardWatchActivity` 的时序窗口 / 动作词表）。
//!
//! ## 为什么值得 fuzz（这个面会**产出告警** ⇒ 告警进 LLM/关怀候选链）
//!
//! 模块头写死的四条：
//! 1. **缺读数 = `Unknown`，不告警不编造**（`GuardMatch::Unknown` **永不告警**）；
//! 2. **只看不动**（零设备动作；闭合动作词表只有 `list_rules` / `watch` / `list_alerts`）；
//! 3. **时序条件**：`sustain_minutes` 连续命中窗口 + `active_window` 生效时段
//!    （**跨午夜 = 起 > 止**）；坏配置**拒开也拒写**；
//! 4. **两闸齐开**才有守护面（`GUARD_WATCH_ENABLE_ENV` **依赖**
//!    `DEVICE_SENSE_ENABLE_ENV`，默认关 fail-closed）。
//!
//! ## 判据纪律（每条断言都从被测代码自己写明的分支/词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **三态闭合**：条件判定只能是 `Met` / `NotMet` / `Unknown{reason}`；**Unknown 的 reason 非空** | `GuardMatch` 定义 + `unknown(reason)` |
//! | O2 | **Unknown 永不告警**：`is_met()` 对 `Unknown` 必须为 false | `is_met` 文档「Unknown 不算命中」 |
//! | O3 | **缺读数不编造**：温度条件遇到**缺 `temperature_c`** ⇒ `Unknown`（**不是** `NotMet`）；门磁同理 | `evaluate` 的 `None => unknown(...)` 分支 |
//! | O4 | **阈值语义**：`>` / `<` 严格不等（**等于阈值 ⇒ NotMet**） | `evaluate` 的 `value > celsius` / `value < celsius` |
//! | O5 | **门磁语义**：`door_open` 读数 == 目标态才命中（open=false 时读 false 也命中） | `value == open` |
//! | O6 | **离线语义**：`Offline` 条件 ⇒ `online == false` 命中（与读数缺位无关） | `Self::Offline => from_bool(!state.online)` |
//! | O7 | **闭合动作词表**：`GUARD_WATCH_ACTIONS` 恰 3 项且互异 | `GUARD_WATCH_ACTIONS` 常量 |
//! | O8 | **坏时段拒开**：分钟数 ≥ 1440 ⇒ `validate()` 必须 Err（**不静默夹取**） | `GuardWindow::validate` |
//! | O9 | **跨午夜 = 起 > 止**：`start > end` 必须是**合法**配置（不拒） | `GuardWindow` 文档「跨午夜 = 起 > 止」 |
//!
//! ## 诚实边界
//!
//! - **不测告警落账面**（`guard-alerts.jsonl` 追加）—— 那是另一条面；
//!   本 target 只测**纯判定**（条件 + 时段 + 词表）；
//! - **不测 `watch` 动作的端到端**（需要设备表 + 适配器 + 活动状态）；
//! - **不测开关**（依赖进程级 env ⇒ 属 `env-race` 清单面，不是本面）。

use std::collections::BTreeMap;

use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

use apeireth_tools_canonical::device::adapter::DeviceState;
use apeireth_tools_canonical::device::guard_watch::{
    GuardCondition, GuardMatch, GuardWindow, GUARD_WATCH_ACTIONS,
};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    /// 无依赖的极简 `block_on`（抄 `credentials.rs` 的同款：**0 引新外部 dep**）。
    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(NoopWaker));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 造一个带指定读数的设备状态。
fn state_with(readings: &[(&str, Value)], online: bool) -> DeviceState {
    let mut map = BTreeMap::new();
    for (key, value) in readings {
        map.insert((*key).to_string(), value.clone());
    }
    DeviceState {
        device_id: "dev-1".to_string(),
        kind: apeireth_tools_canonical::device::registry::DeviceKind::Other,
        adapter: "mock".to_string(),
        online,
        readings: map,
        observed_at_ms: 1_700_000_000_000,
        source_note: "fuzz-fixture".to_string(),
    }
}

fuzz_target!(|data: &[u8]| {
    let b = |i: usize| -> u8 { data.get(i).copied().unwrap_or(0) };
    let _ = block_on(async {});

    // ── 读数：温度可缺 / 可为非数值 / 可有；门磁同理；online 可真可假
    let temperature: Option<Value> = match b(0) % 4 {
        0 => None,
        1 => Some(json!("not-a-number")), // 类型不符 ⇒ 也当缺位
        2 => Some(json!(30.0)),
        _ => Some(json!(b(1) as f64)),
    };
    let door: Option<Value> = match b(2) % 4 {
        0 => None,
        1 => Some(json!(42)), // 类型不符
        2 => Some(json!(true)),
        _ => Some(json!(false)),
    };
    let online = b(3) % 2 == 0;

    let mut readings: Vec<(&str, Value)> = Vec::new();
    if let Some(value) = temperature.clone() {
        readings.push(("temperature_c", value));
    }
    if let Some(value) = door.clone() {
        readings.push(("door_open", value));
    }
    let state = state_with(&readings, online);

    // ── 条件：四种闭合形态
    let threshold = (b(4) as f64) - 10.0;
    let condition = match b(5) % 4 {
        0 => GuardCondition::TemperatureAbove { celsius: threshold },
        1 => GuardCondition::TemperatureBelow { celsius: threshold },
        2 => GuardCondition::DoorOpen { open: b(6) % 2 == 0 },
        _ => GuardCondition::Offline,
    };

    let matched = condition.evaluate(&state);

    // ── O1 三态闭合 + Unknown 的 reason 非空
    match &matched {
        GuardMatch::Met | GuardMatch::NotMet => {}
        GuardMatch::Unknown { reason } => {
            assert!(
                !reason.trim().is_empty(),
                "Unknown 必须带真因（不得空消息）: condition={:?}",
                condition
            );
        }
    }

    // ── O2 Unknown 永不告警
    assert!(
        !matches!(matched, GuardMatch::Unknown { .. }) || !matched.is_met(),
        "Unknown 永不告警（is_met 必须为 false）"
    );

    // ── O3 缺读数不编造（缺位 / 类型不符 ⇒ Unknown，**不是** NotMet）
    let has_temp = temperature
        .as_ref()
        .and_then(|v| v.as_f64())
        .is_some();
    let has_door = door.as_ref().and_then(|v| v.as_bool()).is_some();
    match &condition {
        GuardCondition::TemperatureAbove { .. } | GuardCondition::TemperatureBelow { .. } => {
            if !has_temp {
                assert!(
                    matches!(matched, GuardMatch::Unknown { .. }),
                    "温度读数缺位/类型不符必须判 Unknown（不编造）: matched={matched:?}"
                );
            }
        }
        GuardCondition::DoorOpen { .. } => {
            if !has_door {
                assert!(
                    matches!(matched, GuardMatch::Unknown { .. }),
                    "门磁读数缺位/类型不符必须判 Unknown（不编造）: matched={matched:?}"
                );
            }
        }
        GuardCondition::Offline => {}
    }

    // ── O4 阈值语义：严格不等
    if let (Some(temp), GuardCondition::TemperatureAbove { celsius }) = (
        temperature.as_ref().and_then(|v| v.as_f64()),
        &condition,
    ) {
        assert_eq!(
            matched.is_met(),
            temp > *celsius,
            "温度上阈值必须是严格 >（等于阈值不命中）: temp={temp} celsius={celsius}"
        );
    }
    if let (Some(temp), GuardCondition::TemperatureBelow { celsius }) = (
        temperature.as_ref().and_then(|v| v.as_f64()),
        &condition,
    ) {
        assert_eq!(
            matched.is_met(),
            temp < *celsius,
            "温度下阈值必须是严格 <（等于阈值不命中）: temp={temp} celsius={celsius}"
        );
    }

    // ── O5 门磁语义：读数 == 目标态才命中
    if let (Some(value), GuardCondition::DoorOpen { open }) = (
        door.as_ref().and_then(|v| v.as_bool()),
        &condition,
    ) {
        assert_eq!(
            matched.is_met(),
            value == *open,
            "门磁必须「读数 == 目标态」才命中: value={value} open={open}"
        );
    }

    // ── O6 离线语义：只看 online（与读数缺位无关）
    if let GuardCondition::Offline = &condition {
        assert_eq!(
            matched.is_met(),
            !online,
            "离线条件必须按 online 判定（与读数缺位无关）: online={online}"
        );
    }

    // ── O7 闭合动作词表
    assert_eq!(
        GUARD_WATCH_ACTIONS,
        &["list_rules", "watch", "list_alerts"],
        "动作词表是闭合的（不得增删）"
    );
    let mut unique = GUARD_WATCH_ACTIONS.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        GUARD_WATCH_ACTIONS.len(),
        "动作词表各项互异"
    );

    // ── O8 坏时段拒开（不静默夹取）
    let bad_minutes = b(7) as u32 % 4;
    let start = if bad_minutes == 0 { 1440 } else { 10 };
    let end = if bad_minutes == 1 { 1441 } else { 20 };
    let window = GuardWindow {
        start_minutes: start,
        end_minutes: end,
    };
    if start >= 24 * 60 || end >= 24 * 60 {
        assert!(
            window.validate().is_err(),
            "分钟数 ≥ 1440 必须拒开（不静默夹取）: {window:?}"
        );
    } else {
        assert!(
            window.validate().is_ok(),
            "合法分钟数必须通过校验: {window:?}"
        );
    }

    // ── O9 跨午夜 = 起 > 止（必须合法）
    let overnight = GuardWindow {
        start_minutes: 23 * 60,
        end_minutes: 60,
    };
    assert!(
        overnight.validate().is_ok(),
        "跨午夜（起 > 止）必须是**合法**配置（不得拒开）: {overnight:?}"
    );

    // ── 条件词（落盘 / 工具结果 / 自述共用）必须非空且稳定
    let label = condition.label();
    assert!(!label.is_empty(), "条件词不得为空");
    assert_eq!(label, condition.label(), "条件词必须稳定（纯函数）");
});
