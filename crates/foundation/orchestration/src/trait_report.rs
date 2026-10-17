//! 性格周报 (裁定 ⑤) —— 成长计既有账的**只读汇总**。
//!
//! **数据 = 成长计既有账** (自校准的沉积记录 [`TuningRecord`], 落盘在
//! `tuning-log.jsonl`): 本模块只做纯汇总, **不新造账** —— 没有自己的存储面。
//!
//! **口径 (逐项钉死)**:
//! - 窗口 = 近 [`TRAIT_REPORT_WINDOW_DAYS`] 天: `[now - 7d, now]` **两端含**;
//!   时间戳在未来的记录不进窗 (不编造"还没发生"的移动);
//! - **移动量** `net_delta` = 窗内该杆 `Σ(next - previous)` (带符号净移动);
//!   `move_count` = 窗内实际移动 (next != previous) 的沉积记录数; 零移动留痕
//!   (已达上下限 / 已达每日漂移上限 / 锁定跳过) 不计移动;
//! - **移动原因** `top_reasons` = 窗内**实际移动**的沉积记录按 reason 归并
//!   计数的前 [`TRAIT_REPORT_TOP_REASONS`] 条 (计数降序, 同数按字典序升序,
//!   确定性可复核);
//! - **锁定态** = [`TraitLocks`] 真值; 被锁定跳过的沉积留痕单独计数
//!   (`locked_skipped`), 不混进移动;
//! - `last_move` / [`recent_moves`] = 最近一次(几条) **实际移动** (自述性格节
//!   的"最近移动"同源)。
//!
//! 全确定性、无 IO、无时钟: `now_ms` 由调用方给 (测试可钉死)。

use serde::{Deserialize, Serialize};

use crate::self_tuning::{
    is_trait_locked_skip, TraitLocks, TunableParam, TuningRecord, TuningValues, DAY_MS,
};

/// 性格周报的窗口天数 (近 7 天)。
pub const TRAIT_REPORT_WINDOW_DAYS: i64 = 7;

/// 移动原因排行的条数上限 (top N)。
pub const TRAIT_REPORT_TOP_REASONS: usize = 5;

/// 自述性格节「最近移动」的条数上限。
pub const TRAIT_RECENT_MOVES: usize = 4;

/// 判定一条记录是否**实际移动**了杆值 (零移动留痕不算移动)。
pub fn moved(record: &TuningRecord) -> bool {
    (record.next - record.previous).abs() > 1e-9
}

/// 一次滑杆移动 (来自成长计既有账的一条实际移动沉积记录)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitMove {
    /// 被移动的滑杆。
    pub param: TunableParam,
    /// 滑杆中文名 (展示用, 与调参面板同名)。
    pub label: String,
    /// 移动时刻 (epoch 毫秒)。
    pub at_epoch_ms: i64,
    /// 移动量 (带符号: next - previous)。
    pub delta: f64,
    /// 移动原因 (沉积记录的 reason 原文)。
    pub reason: String,
}

impl TraitMove {
    /// 从一条实际移动的沉积记录投影。
    pub fn from_record(record: &TuningRecord) -> Self {
        Self {
            param: record.param,
            label: record.param.label().to_string(),
            at_epoch_ms: record.at_epoch_ms,
            delta: record.next - record.previous,
            reason: record.reason.clone(),
        }
    }
}

/// 移动原因排行的一行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitReasonCount {
    /// 移动原因 (沉积记录的 reason 原文)。
    pub reason: String,
    /// 窗内出现次数。
    pub count: u64,
}

/// 单根滑杆的周报行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitSliderReport {
    /// 滑杆。
    pub param: TunableParam,
    /// 滑杆中文名。
    pub label: String,
    /// 当前生效值。
    pub value: f64,
    /// 锁定态 (locked = 成长沉积不再移动该杆)。
    pub locked: bool,
    /// 窗内净移动量 = `Σ(next - previous)`。
    pub net_delta: f64,
    /// 窗内实际移动的沉积记录数。
    pub move_count: u64,
    /// 窗内被锁定跳过的沉积审计留痕数。
    pub locked_skipped: u64,
    /// 窗内最近一次实际移动 (`None` = 窗内没动过)。
    pub last_move: Option<TraitMove>,
}

/// 性格周报 (近 7 天四杆移动量 / 移动原因 top / 锁定态)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitReport {
    /// 窗口天数。
    pub window_days: i64,
    /// 窗口起点 (epoch 毫秒, 含)。
    pub window_from_ms: i64,
    /// 窗口终点 (epoch 毫秒, 含; = `generated_at_ms`)。
    pub window_to_ms: i64,
    /// 汇总时刻 (epoch 毫秒, 调用方给)。
    pub generated_at_ms: i64,
    /// 四根滑杆的周报行 ([`TunableParam::ALL`] 顺序)。
    pub sliders: Vec<TraitSliderReport>,
    /// 窗内成长计记录总数 (含零移动留痕)。
    pub records_in_window: u64,
    /// 窗内锁定跳过的沉积审计留痕总数。
    pub locked_skipped: u64,
    /// 移动原因 top ([`TRAIT_REPORT_TOP_REASONS`] 条封顶)。
    pub top_reasons: Vec<TraitReasonCount>,
}

impl TraitReport {
    /// 从成长计既有账 (沉积记录) 汇总周报。纯函数: 同输入同输出。
    pub fn build(
        records: &[TuningRecord],
        locks: &TraitLocks,
        values: TuningValues,
        now_ms: i64,
    ) -> Self {
        let window_from_ms = now_ms - TRAIT_REPORT_WINDOW_DAYS * DAY_MS;
        let in_window: Vec<&TuningRecord> = records
            .iter()
            .filter(|record| record.at_epoch_ms >= window_from_ms && record.at_epoch_ms <= now_ms)
            .collect();

        let mut sliders = Vec::with_capacity(TunableParam::ALL.len());
        let mut locked_skipped_total = 0u64;
        let mut reason_counts: std::collections::BTreeMap<String, u64> =
            std::collections::BTreeMap::new();
        for record in &in_window {
            if is_trait_locked_skip(record) {
                locked_skipped_total += 1;
                continue;
            }
            if moved(record) {
                *reason_counts.entry(record.reason.clone()).or_insert(0) += 1;
            }
        }

        for param in TunableParam::ALL {
            let mut net_delta = 0.0;
            let mut move_count = 0u64;
            let mut locked_skipped = 0u64;
            let mut last_move: Option<(u64, TraitMove)> = None;
            for record in &in_window {
                if record.param != param {
                    continue;
                }
                if is_trait_locked_skip(record) {
                    locked_skipped += 1;
                    continue;
                }
                if moved(record) {
                    net_delta += record.next - record.previous;
                    move_count += 1;
                    let candidate = (record.seq, TraitMove::from_record(record));
                    let newer = last_move.as_ref().is_none_or(|current: &(u64, TraitMove)| {
                        (candidate.1.at_epoch_ms, candidate.0) > (current.1.at_epoch_ms, current.0)
                    });
                    if newer {
                        last_move = Some(candidate);
                    }
                }
            }
            sliders.push(TraitSliderReport {
                param,
                label: param.label().to_string(),
                value: values.get(param),
                locked: locks.is_locked(param),
                net_delta,
                move_count,
                locked_skipped,
                last_move: last_move.map(|(_, record)| record),
            });
        }

        let mut top_reasons: Vec<TraitReasonCount> = reason_counts
            .into_iter()
            .map(|(reason, count)| TraitReasonCount { reason, count })
            .collect();
        top_reasons.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
        top_reasons.truncate(TRAIT_REPORT_TOP_REASONS);

        Self {
            window_days: TRAIT_REPORT_WINDOW_DAYS,
            window_from_ms,
            window_to_ms: now_ms,
            generated_at_ms: now_ms,
            sliders,
            records_in_window: in_window.len() as u64,
            locked_skipped: locked_skipped_total,
            top_reasons,
        }
    }
}

/// 「最近移动」: 最近 `limit` 条**实际移动**的沉积记录 (新 → 旧; 未来时刻不计)。
///
/// 自述性格节的"最近移动"与周报的 `last_move` 同一账本同一判据, 不另造口径。
pub fn recent_moves(records: &[TuningRecord], now_ms: i64, limit: usize) -> Vec<TraitMove> {
    let mut moved_records: Vec<&TuningRecord> = records
        .iter()
        .filter(|record| record.at_epoch_ms <= now_ms && moved(record))
        .collect();
    moved_records.sort_by(|a, b| (b.at_epoch_ms, b.seq).cmp(&(a.at_epoch_ms, a.seq)));
    moved_records
        .into_iter()
        .take(limit)
        .map(TraitMove::from_record)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_tuning::TRAIT_LOCKED_SKIP_PREFIX;

    fn record(
        seq: u64,
        param: TunableParam,
        previous: f64,
        next: f64,
        reason: &str,
        at_epoch_ms: i64,
    ) -> TuningRecord {
        TuningRecord {
            seq,
            param,
            previous,
            next,
            reason: reason.to_string(),
            at_epoch_ms,
        }
    }

    fn skip_record(seq: u64, param: TunableParam, at_epoch_ms: i64) -> TuningRecord {
        record(
            seq,
            param,
            1.0,
            1.0,
            &format!("{TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 测试意图"),
            at_epoch_ms,
        )
    }

    const NOW: i64 = 70 * DAY_MS;

    // 1. 七天窗 + 四杆移动量: 窗内计入、窗外与未来剔除、两端含、零移动留痕不计。
    #[test]
    fn weekly_report_seven_day_window_and_movement_amounts() {
        let records = vec![
            // 窗内: MemoryFade 移动两次 (+0.25, -0.5) → 净 -0.25, 移动 2 次。
            record(
                1,
                TunableParam::MemoryFade,
                1.0,
                1.25,
                "上调一档",
                NOW - DAY_MS,
            ),
            record(
                2,
                TunableParam::MemoryFade,
                1.25,
                0.75,
                "下调半档",
                NOW - 3 * DAY_MS,
            ),
            // 窗内: ToneSaturation 移动一次。
            record(
                3,
                TunableParam::ToneSaturation,
                1.0,
                1.2,
                "过平淡",
                NOW - 2 * DAY_MS,
            ),
            // 窗内零移动留痕 (已达上限): 不计移动。
            record(
                4,
                TunableParam::ToneSaturation,
                1.2,
                1.2,
                "已达参数上限: x",
                NOW - 2 * DAY_MS,
            ),
            // 窗内锁定跳过: 只计 locked_skipped。
            skip_record(5, TunableParam::MemoryFade, NOW - DAY_MS),
            // 窗口左端 (含): 恰在 now-7d 的记录计入。
            record(
                6,
                TunableParam::CuriosityStrength,
                1.0,
                1.1,
                "左端",
                NOW - 7 * DAY_MS,
            ),
            // 窗外 (左端前 1ms): 剔除。
            record(
                7,
                TunableParam::CuriosityStrength,
                1.1,
                3.0,
                "窗外",
                NOW - 7 * DAY_MS - 1,
            ),
            // 未来时刻: 剔除 (不编造未发生的移动)。
            record(
                8,
                TunableParam::ConsolidationCadence,
                1.0,
                9.0,
                "未来",
                NOW + 1,
            ),
        ];
        let locks = TraitLocks {
            memory_fade: true,
            ..TraitLocks::default()
        };
        let report = TraitReport::build(&records, &locks, TuningValues::baseline(), NOW);

        assert_eq!(report.window_days, 7);
        assert_eq!(report.window_from_ms, NOW - 7 * DAY_MS);
        assert_eq!(report.window_to_ms, NOW);
        assert_eq!(report.records_in_window, 6, "窗内 6 条 (含留痕)");
        assert_eq!(report.locked_skipped, 1);

        let by_param = |param: TunableParam| {
            report
                .sliders
                .iter()
                .find(|row| row.param == param)
                .expect("四杆齐")
                .clone()
        };
        let fade = by_param(TunableParam::MemoryFade);
        assert!(fade.locked, "锁定态如实上报");
        assert!((fade.net_delta - (-0.25)).abs() < 1e-9, "净移动量 = -0.25");
        assert_eq!(fade.move_count, 2);
        assert_eq!(fade.locked_skipped, 1);
        let last = fade.last_move.expect("最近移动在场");
        assert_eq!(last.at_epoch_ms, NOW - DAY_MS, "最近移动 = 窗内最新一次");
        assert!((last.delta - 0.25).abs() < 1e-9);
        assert_eq!(last.reason, "上调一档");

        let tone = by_param(TunableParam::ToneSaturation);
        assert!(!tone.locked);
        assert!((tone.net_delta - 0.2).abs() < 1e-9, "零移动留痕不计移动量");
        assert_eq!(tone.move_count, 1, "零移动留痕不计移动次数");

        let curiosity = by_param(TunableParam::CuriosityStrength);
        assert!((curiosity.net_delta - 0.1).abs() < 1e-9, "左端含、窗外剔");
        assert_eq!(curiosity.move_count, 1);

        let cadence = by_param(TunableParam::ConsolidationCadence);
        assert_eq!(cadence.move_count, 0, "未来记录不计");
        assert_eq!(cadence.net_delta, 0.0);
        assert!(cadence.last_move.is_none());
        assert_eq!(report.sliders.len(), 4, "四杆一行不多不少");
    }

    // 2. 移动原因 top: 只数实际移动的沉积记录, 计数降序 + 同数字典序, 5 条封顶。
    #[test]
    fn weekly_report_top_reasons_rank_moved_deposits_only() {
        let mut records = Vec::new();
        for index in 0..7u64 {
            records.push(record(
                index,
                TunableParam::MemoryFade,
                1.0,
                1.1,
                "reason-b",
                NOW - DAY_MS,
            ));
        }
        for index in 7..10u64 {
            records.push(record(
                index,
                TunableParam::MemoryFade,
                1.1,
                1.2,
                "reason-a",
                NOW - DAY_MS,
            ));
        }
        // 零移动留痕 (reason-c) 与锁定跳过: 都不算"移动原因"。
        records.push(record(
            20,
            TunableParam::MemoryFade,
            1.2,
            1.2,
            "reason-c",
            NOW - DAY_MS,
        ));
        records.push(skip_record(21, TunableParam::MemoryFade, NOW - DAY_MS));
        // 超过 5 种原因时只留前 5。
        for (index, reason) in ["reason-d", "reason-e", "reason-f", "reason-g"]
            .into_iter()
            .enumerate()
        {
            records.push(record(
                30 + index as u64,
                TunableParam::ToneSaturation,
                1.0,
                1.1,
                reason,
                NOW - DAY_MS,
            ));
        }

        let report = TraitReport::build(
            &records,
            &TraitLocks::default(),
            TuningValues::baseline(),
            NOW,
        );
        assert_eq!(report.top_reasons.len(), 5, "top 5 封顶");
        assert_eq!(report.top_reasons[0].reason, "reason-b");
        assert_eq!(report.top_reasons[0].count, 7);
        assert_eq!(report.top_reasons[1].reason, "reason-a");
        assert_eq!(report.top_reasons[1].count, 3);
        // 同计数按字典序升序 (确定性): d/e/f/g 各 1, 取前三 = d、e、f。
        let tail: Vec<&str> = report.top_reasons[2..]
            .iter()
            .map(|r| r.reason.as_str())
            .collect();
        assert_eq!(tail, vec!["reason-d", "reason-e", "reason-f"]);
        assert!(
            !report
                .top_reasons
                .iter()
                .any(|row| row.reason == "reason-c"
                    || row.reason.starts_with(TRAIT_LOCKED_SKIP_PREFIX)),
            "零移动留痕与锁定跳过不得混进移动原因"
        );
    }

    // 3. 最近移动: 新 → 旧、条数上限、零移动留痕不计、未来时刻不计。
    #[test]
    fn recent_moves_are_newest_first_limited_and_skip_untouched() {
        let records = vec![
            record(
                1,
                TunableParam::MemoryFade,
                1.0,
                1.1,
                "一",
                NOW - 5 * DAY_MS,
            ),
            record(
                2,
                TunableParam::ToneSaturation,
                1.0,
                0.9,
                "二",
                NOW - 3 * DAY_MS,
            ),
            record(3, TunableParam::MemoryFade, 1.1, 1.2, "三", NOW - DAY_MS),
            skip_record(4, TunableParam::MemoryFade, NOW - 1),
            record(
                5,
                TunableParam::CuriosityStrength,
                1.0,
                5.0,
                "未来",
                NOW + 10,
            ),
        ];
        let moves = recent_moves(&records, NOW, TRAIT_RECENT_MOVES);
        let reasons: Vec<&str> = moves.iter().map(|m| m.reason.as_str()).collect();
        assert_eq!(reasons, vec!["三", "二", "一"], "新 → 旧, 跳过/未来不计");
        assert_eq!(moves[0].param, TunableParam::MemoryFade);
        assert_eq!(moves[0].label, "遗忘衰减强度");
        let limited = recent_moves(&records, NOW, 1);
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].reason, "三");
    }

    // 4. 空账 = 如实空周报 (四杆全 0 移动, 不编数)。
    #[test]
    fn empty_ledger_reports_zeros_not_invented_numbers() {
        let report = TraitReport::build(&[], &TraitLocks::default(), TuningValues::baseline(), NOW);
        assert_eq!(report.records_in_window, 0);
        assert_eq!(report.locked_skipped, 0);
        assert!(report.top_reasons.is_empty());
        for row in &report.sliders {
            assert_eq!(row.net_delta, 0.0);
            assert_eq!(row.move_count, 0);
            assert!(!row.locked);
            assert!(row.last_move.is_none());
        }
    }
}
