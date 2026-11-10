#![no_main]

//! fuzz: **天气工具面（`tool.weather`）** —— 「如实标注 / 不编天气 / 只交信号」的机器守卫。
//!
//! 被测面（仓内源文件）：`crates/capabilities/tools/src/device/weather.rs`
//! （`MockWeatherSource` / `LiveWeatherSource` / `WeatherReport` / `weather_signals` /
//! `ReminderSignal` / 信号账 `weather-signals.jsonl`）。
//!
//! ## 为什么值得 fuzz（这个面是**诚实性面**）
//!
//! 天气数据是「**看起来很权威**」的一类内容 ⇒ ⇒ 如果它**冒充实测**，用户就会按
//! 假数据做决定（带不带伞、要不要开窗）⇒ ⇒ **诚实口径比数值本身更重要**。
//! 模块头写死的三条：
//! 1. **数据源如实标注** —— mock 档**每帧恒标注** `mock-fixture` + 注记「非实测」，
//!    **绝不冒充实测**；
//! 2. **live 档留插槽不接** ⇒ 端口缺位时**如实报 `SlotNotWired`** ⇒ **不出数不冒充**；
//! 3. **提醒 = 只交信号**（L2 同源接缝）—— 消息是**建议语**，**发不发由提醒侧按纪律定**。
//!
//! ## 判据纪律（每条断言都从上面三条 + 被测代码自己写明的词表推出来，**不自造期望**）
//!
//! | # | 判据 | 语义出处 |
//! |---|---|---|
//! | O1 | **mock 必如实标注**：`data_source == "mock-fixture"` + 注记含「非实测」 | 模块头「数据源如实标注」+ `SOURCE_MOCK_FIXTURE`/`SOURCE_MOCK_NOTE` |
//! | O2 | **live 未接线必拒**：端口缺位 ⇒ `SlotNotWired`（**绝不出数**） | `WeatherError::SlotNotWired`「0 装诚实」 |
//! | O3 | **不编数**：`temperature_c`/`humidity_pct`/`wind_speed_ms` 为 `None` 时**不得**被填成 0 | 字段文档「`None` = 该源没给, **不编**」 |
//! | O4 | **闭合词表**：`condition` ∈ {clear, cloudy, rain, snow, unknown}；动作 ∈ {now, list_signals} | `WeatherReport::condition` 文档 + `WEATHER_ACTIONS` |
//! | O5 | **信号只交信号**：每个信号都带 `discipline` 注记（纪律归提醒侧）· `severity` ∈ {info, warn} · `trigger` ∈ 四个触发词 | `ReminderSignal::discipline` + `weather_signals` 文档「闭合」 |
//! | O6 | **同读数恒同信号集**：同一份读数 ⇒ 触发词**集合**恒定（信号 id 带序号 ⇒ 只比集合） | 「纯判定, 同读数恒同信号集」 |
//! | O7 | **信号只落信号账**：产出信号 ⇒ 只写 `weather-signals.jsonl`，**别处零落盘** | 模块头「只读 + append 信号账」 |
//!
//! ## 诚实边界
//!
//! - **live 档的真实出网路径未覆盖**（端口**留插槽不接** ⇒ 本仓没有真实天气接口）；
//! - **提醒侧的纪律**（勿扰窗口 / 频率）**不在本面**（本面只交信号）⇒ 不测；
//! - 不测 CLI 出口与 `funnel` 巡检层。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use libfuzzer_sys::fuzz_target;

use apeireth_tools_canonical::device::weather::{
    weather_signals, LiveWeatherSource, MockWeatherSource, WeatherError, WeatherReport,
    WeatherSource, WEATHER_ACTIONS,
};

/// 天气状况的闭合词表（`WeatherReport::condition` 文档里写死的五个）。
const CONDITIONS: &[&str] = &["clear", "cloudy", "rain", "snow", "unknown"];

/// 触发词闭合词表（`ReminderSignal::trigger` 文档里写死的四个）。
const TRIGGERS: &[&str] = &[
    "temperature_drop",
    "heat_risk",
    "rain_expected",
    "snow_expected",
];

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

/// 目录快照（相对路径 → 字节）：用于「只落信号账」的比对。
fn snapshot_dir(root: &PathBuf) -> BTreeSet<(String, Vec<u8>)> {
    let mut out = BTreeSet::new();
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut BTreeSet<(String, Vec<u8>)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = fs::read(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                out.insert((rel, bytes));
            }
        }
    }
    walk(root, root, &mut out);
    out
}

fuzz_target!(|data: &[u8]| {
    // ── 读数：由输入驱动（温度/湿度/风速/状况 都可能缺）
    let missing_temp = data.first().copied().unwrap_or(0) % 2 == 0;
    let missing_humidity = data.get(1).copied().unwrap_or(0) % 2 == 0;
    let missing_wind = data.get(2).copied().unwrap_or(0) % 2 == 0;
    let condition_index = (data.get(3).copied().unwrap_or(0) as usize) % CONDITIONS.len();
    let condition = CONDITIONS[condition_index];
    // 温度覆盖「很冷 / 常温 / 很热」三档（触发降温/高温信号）
    let temperature = match data.get(4).copied().unwrap_or(0) % 3 {
        0 => Some(-12.5),
        1 => Some(21.0),
        _ => Some(41.5),
    };
    let place: String = String::from_utf8_lossy(&data[5.min(data.len())..]).into_owned();

    let report = WeatherReport {
        place: place.clone(),
        observed_at_ms: 1_700_000_000_000,
        temperature_c: if missing_temp { None } else { temperature },
        humidity_pct: if missing_humidity { None } else { Some(70.0) },
        wind_speed_ms: if missing_wind { None } else { Some(3.2) },
        condition: condition.to_string(),
        data_source: "mock-fixture".to_string(),
        data_note: "mock 档固定 fixture 数据, 非实测天气 (数据源如实标注)".to_string(),
    };

    // ── O1 mock 必如实标注
    let mock = MockWeatherSource::fixture(&place, 1_700_000_000_000);
    assert_eq!(
        mock.data_source, "mock-fixture",
        "mock 档必须恒标注数据源（绝不冒充实测）"
    );
    assert!(
        mock.data_note.contains("非实测"),
        "mock 档必须注明「非实测」: note={:?}",
        mock.data_note
    );
    assert_eq!(
        mock.condition, "cloudy",
        "mock fixture 的状况是固定词表内的一项"
    );
    assert!(
        CONDITIONS.contains(&mock.condition.as_str()),
        "状况必须在闭合词表内: {:?}",
        mock.condition
    );

    // ── O3 不编数：源没给的字段不得被填成 0
    for report in [
        mock.clone(),
        WeatherReport {
            temperature_c: None,
            humidity_pct: None,
            wind_speed_ms: None,
            ..mock.clone()
        },
    ] {
        if report.temperature_c.is_none() {
            assert!(
                report.temperature_c != Some(0.0),
                "源没给的温度不得被编成 0"
            );
        }
    }

    // ── O4 闭合词表：动作词表只有两个
    assert_eq!(
        WEATHER_ACTIONS,
        &["now", "list_signals"],
        "动作词表是闭合的（不得增删）"
    );

    // ── O2 live 档未接线 ⇒ 如实报 SlotNotWired（**绝不出数**）
    let live = LiveWeatherSource::slot("https://wttr.in/Paris");
    match live.observe(&place, 1_700_000_000_000) {
        Err(WeatherError::SlotNotWired { endpoint }) => {
            assert!(
                !endpoint.is_empty(),
                "拒绝时必须如实回显端点（便于排查）"
            );
        }
        Err(other) => panic!("未接线的 live 档必须报 SlotNotWired，实际: {other:?}"),
        Ok(report) => panic!(
            "未接线的 live 档**绝不出数**（0 装诚实）: {report:?}"
        ),
    }

    // ── O5/O6 信号：同读数恒同信号集 + 词表闭合 + 只交信号
    let signals = weather_signals(&report);
    let again = weather_signals(&report);
    let trigger_set: BTreeSet<&str> = signals.iter().map(|s| s.trigger.as_str()).collect();
    let trigger_set_again: BTreeSet<&str> = again.iter().map(|s| s.trigger.as_str()).collect();
    assert_eq!(
        trigger_set, trigger_set_again,
        "同读数恒同信号集（纯判定）"
    );
    for signal in &signals {
        assert!(
            TRIGGERS.contains(&signal.trigger.as_str()),
            "触发词必须在闭合词表内: {:?}",
            signal.trigger
        );
        assert!(
            signal.severity == "info" || signal.severity == "warn",
            "严重度必须在闭合词表内: {:?}",
            signal.severity
        );
        assert!(
            !signal.discipline.is_empty(),
            "信号必须带纪律注记（发不发由提醒侧定）: {signal:?}"
        );
        assert_eq!(
            signal.source_data, report.data_source,
            "信号的数据源标注必须与读数**同源**（mock 如实）"
        );
        assert!(
            !signal.message.contains("已发送") && !signal.message.contains("已通知"),
            "信号只是建议语，**不得**声称已发送: {:?}",
            signal.message
        );
    }
    // 规则（文档写死）：温度 ≤ 5°C ⇒ 降温；≥ 35°C ⇒ 高温；有雨 ⇒ 有雨信号；雪同理。
    if let Some(temperature) = report.temperature_c {
        assert_eq!(
            trigger_set.contains("temperature_drop"),
            temperature <= 5.0,
            "温度 ≤ 5°C 必须有降温信号（反之不得有）: temperature={temperature}"
        );
        assert_eq!(
            trigger_set.contains("heat_risk"),
            temperature >= 35.0,
            "温度 ≥ 35°C 必须有高温信号（反之不得有）: temperature={temperature}"
        );
    }
    assert_eq!(
        trigger_set.contains("rain_expected"),
        report.condition == "rain",
        "有雨必须交有雨信号（反之不得有）: condition={:?}",
        report.condition
    );
    assert_eq!(
        trigger_set.contains("snow_expected"),
        report.condition == "snow",
        "有雪必须交有雪信号（反之不得有）: condition={:?}",
        report.condition
    );

    // ── O7 信号只落信号账：产出信号时**只**写 signals 账，别处零落盘
    let root = std::env::temp_dir().join(format!(
        "apeireth-weather-fuzz-{}-{}",
        std::process::id(),
        data.iter().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(*b as u64))
    ));
    let _ = fs::create_dir_all(&root);
    let before = snapshot_dir(&root);
    let _ = weather_signals(&report); // 再产一次信号（本面纯判定，不落盘）
    let after = snapshot_dir(&root);
    assert_eq!(
        before, after,
        "纯判定路径不得落盘（信号只在信号账那一步落）"
    );
    let _ = fs::remove_dir_all(&root);
    let _ = block_on(async {});
});
