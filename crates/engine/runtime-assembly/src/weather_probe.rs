//! 自述面 · 天气提醒计数（独立文件）。
//!
//! 原来住在 `canonical/self_status_source.rs` 里，但**天气提醒计数与自述源
//! 不是同一职责** —— 它读的是设备侧天气信号账（`SignalLedger`），
//! 自述源读的是别的账。放在一起会让那个文件（在 arch_guard 尺寸基线内，
//! 只减不增）为一个不相关的职责占行。
//!
//! **只回计数，不回内容**。

use std::path::Path;

/// 天气提醒条数（读持久信号账；账不存在 = 0，如实不编）。
pub fn weather_reminder_count(data_dir: &Path) -> Result<u64, String> {
    let ledger = apeireth_tools_canonical::device::weather::SignalLedger::under_data_dir(data_dir);
    if !ledger.path().exists() {
        return Ok(0);
    }
    let scan = ledger.scan()?;
    Ok(scan.records.len() as u64)
}
