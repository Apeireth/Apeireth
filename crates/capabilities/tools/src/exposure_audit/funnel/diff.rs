//! 守望巡检漏斗 · 报告段的增量视角 (与上次巡检 diff: 新增 / 消失 / 未变)。
//!
//! diff 只比**二次确认候选** (复核段放行的键), 不比原始发现 —— 瞬时态误报
//! 不进增量账。基线 = 上一次**完成**巡检留下的键账 (断点续查的中间态不换基线,
//! 未完成的会话不与上次比, 如实不产出 diff)。

use std::collections::BTreeSet;

/// 增量 diff (新增 / 消失 / 未变; 键序稳定排序)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSummary {
    /// 基线巡检 id (首次巡检 = None, 全部按新增计)。
    pub baseline_patrol_id: Option<String>,
    /// 新增: 本次确认、上次没有。
    pub new: Vec<String>,
    /// 消失: 上次报过、本次未确认 (已收口或已转瞬时)。
    pub gone: Vec<String>,
    /// 未变: 两次都在 (暴露面未收敛也未恶化)。
    pub unchanged: Vec<String>,
}

impl DiffSummary {
    /// 从两次键账算增量 (集合语义, 同键只计一次)。
    pub fn between(
        baseline_patrol_id: Option<String>,
        previous: &[String],
        current: &[String],
    ) -> Self {
        let before: BTreeSet<&str> = previous.iter().map(String::as_str).collect();
        let after: BTreeSet<&str> = current.iter().map(String::as_str).collect();
        let new = after
            .difference(&before)
            .map(|key| (*key).to_string())
            .collect();
        let gone = before
            .difference(&after)
            .map(|key| (*key).to_string())
            .collect();
        let unchanged = before
            .intersection(&after)
            .map(|key| (*key).to_string())
            .collect();
        Self {
            baseline_patrol_id,
            new,
            gone,
            unchanged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_splits_new_gone_and_unchanged_stably() {
        let previous = vec!["a|1".to_string(), "b|2".to_string()];
        let current = vec!["b|2".to_string(), "c|3".to_string()];
        let diff = DiffSummary::between(Some("patrol-1".to_string()), &previous, &current);
        assert_eq!(diff.new, vec!["c|3"]);
        assert_eq!(diff.gone, vec!["a|1"]);
        assert_eq!(diff.unchanged, vec!["b|2"]);
        assert_eq!(diff.baseline_patrol_id.as_deref(), Some("patrol-1"));
    }

    #[test]
    fn first_patrol_has_no_baseline_and_marks_everything_new() {
        let diff = DiffSummary::between(None, &[], &["a|1".to_string()]);
        assert_eq!(diff.baseline_patrol_id, None);
        assert_eq!(diff.new, vec!["a|1"]);
        assert!(diff.gone.is_empty());
        assert!(diff.unchanged.is_empty());
    }
}
