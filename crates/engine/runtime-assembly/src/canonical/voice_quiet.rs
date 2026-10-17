//! 发声面勿扰窗口的同源接缝 (V3-b 发声 × V3-a 关怀纪律)。
//!
//! # 同源裁定 (先读对方的口径)
//!
//! 安静纪律要求发声面的勿扰窗口与关怀纪律**同一个窗口**:
//!
//! 1. `care_rules.json` 的打扰纪律 ([`CareDiscipline`]) **配了**勿扰窗口 →
//!    **引用它的窗口** (同一窗口值 + 同一跨午夜语义 + 同一 UTC 当日分钟
//!    时间基准, [`QuietHours`] 只承接不另造);
//! 2. `care_rules` 未落 / 未配窗口 → 回落发声面的**最小自建 env**
//!    (`APEIRETH_VOICE_QUIET_HOURS`, 缺省 `23:00-08:00`, 与关怀纪律默认窗口
//!    同口径);
//! 3. `care_rules` 存在但拒开 (坏配置) → **错误即帧式拒开**: 拒绝解析并报
//!    真因, 不静默回退 (不静默降级纪律)。
//!
//! [`CareDiscipline`]: apeireth_organ::care::CareDiscipline

use std::path::Path;

use apeireth_organ::care::CareStore;
use apeireth_tools_canonical::voice::{QuietHours, QuietWindow};

/// 从关怀纪律解析发声面勿扰窗口 (同源接缝的唯一入口)。
///
/// 返回的 [`QuietHours`] 即发声面生效窗口 (风险钩子 / 边界 Guard / 工具面
/// 三处共用同一实例语义)。
pub fn voice_quiet_hours_from_care_rules(data_dir: impl AsRef<Path>) -> Result<QuietHours, String> {
    let store = CareStore::new(data_dir.as_ref());
    match store.load() {
        Ok(Some(body)) => {
            match (
                body.discipline.quiet_start_minutes,
                body.discipline.quiet_end_minutes,
            ) {
                (Some(start), Some(end)) => {
                    let window = QuietWindow::new(start, end).map_err(|error| {
                        format!("care_rules 勿扰窗口不合法 (拒绝解析, 如实报): {error}")
                    })?;
                    Ok(QuietHours::windowed(window))
                }
                // 关怀纪律未配窗口 → 发声面最小自建 env 兜底 (安静纪律不落空)。
                _ => QuietHours::from_env(),
            }
        }
        Ok(None) => QuietHours::from_env(),
        Err(error) => Err(format!(
            "care_rules.json refused while resolving the voice quiet window: {error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_organ::care::{CareDiscipline, CareRulesBody};

    fn data_dir_with(body: CareRulesBody) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        CareStore::new(dir.path())
            .save(&body)
            .expect("care_rules fixture saves");
        dir
    }

    #[test]
    fn configured_care_quiet_window_wins() {
        let dir = data_dir_with(CareRulesBody {
            discipline: CareDiscipline {
                quiet_start_minutes: Some(22 * 60),
                quiet_end_minutes: Some(6 * 60),
                ..CareDiscipline::default()
            },
            ..CareRulesBody::default()
        });
        let quiet = voice_quiet_hours_from_care_rules(dir.path()).expect("bridge");
        assert!(quiet.suppressed_at(23 * 60), "同源: 关怀窗口即发声窗口");
        assert!(quiet.suppressed_at(3 * 60));
        assert!(!quiet.suppressed_at(12 * 60));
    }

    #[test]
    fn missing_or_unwindowed_care_rules_fall_back_to_the_minimal_env() {
        let empty = tempfile::tempdir().expect("tempdir");
        let quiet = voice_quiet_hours_from_care_rules(empty.path()).expect("no care_rules");
        assert_eq!(
            quiet.window().map(|window| window.describe()),
            Some(apeireth_tools_canonical::voice::VOICE_QUIET_HOURS_DEFAULT.to_string()),
            "care_rules 未落 = 最小自建 env 缺省窗口兜底"
        );

        // 显式未配窗口 (缺省默认已同源 23:00-8:00, 这里钉「未配 → env 兜底」支路)。
        let unwindowed = data_dir_with(CareRulesBody {
            discipline: CareDiscipline {
                quiet_start_minutes: None,
                quiet_end_minutes: None,
                ..CareDiscipline::default()
            },
            ..CareRulesBody::default()
        });
        let quiet = voice_quiet_hours_from_care_rules(unwindowed.path()).expect("unwindowed");
        assert!(quiet.suppressed_at(23 * 60), "未配窗口也走 env 兜底");
    }

    /// 同源钉 (设计裁定 R1⑭): `care_rules` 打扰纪律的默认窗口与发声面最小
    /// 自建 env 的默认窗口必须是同一个窗口 (两处同改不漂移) + 同一判定口径。
    #[test]
    fn care_discipline_default_and_voice_quiet_default_share_one_window() {
        let care = CareDiscipline::default();
        let (start, end) = (
            care.quiet_start_minutes.expect("默认有窗"),
            care.quiet_end_minutes.expect("默认有窗"),
        );
        let window = QuietHours::default().window().expect("发声面缺省有窗");
        assert_eq!(
            (window.start_min(), window.end_min()),
            (start, end),
            "两处默认勿扰窗口必须同源同值"
        );
        assert_eq!(window.describe(), "23:00-08:00", "裁定口径 = 23:00-8:00");
        for minute in [0, 3 * 60, 8 * 60 - 1, 8 * 60, 12 * 60, 23 * 60] {
            assert_eq!(
                care.in_quiet_window(minute),
                window.contains(minute),
                "第 {minute} 分钟两处判定必须一致 (同一跨午夜语义)"
            );
        }
    }

    #[test]
    fn a_refused_care_rules_document_never_silently_falls_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("care_rules.json"), "{not json").expect("corrupt");
        let error = voice_quiet_hours_from_care_rules(dir.path()).expect_err("坏配置拒开");
        assert!(
            error.contains("refused"),
            "坏配置必须拒开并报真因, 不静默回退: {error}"
        );
    }
}
