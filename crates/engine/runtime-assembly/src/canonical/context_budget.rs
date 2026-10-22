//! 上下文预算旋钮解析（**单一真源**）：`APEIRETH_CONTEXT_BUDGET_CHARS`。
//!
//! 背景（Wave 4 SG-14  homeostasis 行为接线）：cli 侧早已有该旋钮（组装器注入
//! 上下文块的总字符预算），但装配根从未把它交给生物装配面模块
//! （`BioAssemblyModule`），导致「上下文压力 → 调节策略」在内核里从未在
//! 生产回合里真的发生。本模块把解析逻辑收成**一处**，cli 与装配根都转调
//! 这里，避免两套解析口径漂移（同一 env 两读两错是最难查的接线债）。
//!
//! 语义（与 cli 既有解析逐字对齐，历史口径不许漂移）：
//! - 未设 / 空白 / 非法数字 / `0` = 默认 [`DEFAULT_CONTEXT_BUDGET_CHARS`]；
//! - 合法正数 = 原值（不夹上限；上限由组装器自己的预算纪律负责）；
//! - 只读 env，无时钟、无 I/O、无副作用（测试可用显式值入口，不碰进程 env）。

/// 预算旋钮名。
pub const CONTEXT_BUDGET_ENV: &str = "APEIRETH_CONTEXT_BUDGET_CHARS";

/// 默认预算（字符；与 runtime 组装器默认同一真源，本模块不复制数值）。
pub use apeireth_runtime::DEFAULT_CONTEXT_BUDGET_CHARS;

/// 由显式 env 值解析预算（`None` = 未设）。
///
/// 未设 / 空 / 非法 / `0` → 默认值；不做 trim 之外的任何猜测。
#[must_use]
pub fn context_budget_chars_from_env_value(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|&chars| chars > 0)
        .unwrap_or(DEFAULT_CONTEXT_BUDGET_CHARS)
}

/// 从进程 env 解析预算（cli 与装配根共用这一入口）。
#[must_use]
pub fn context_budget_chars_from_env() -> usize {
    context_budget_chars_from_env_value(std::env::var(CONTEXT_BUDGET_ENV).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_empty_invalid_and_zero_fall_back_to_the_default() {
        for raw in [
            None,
            Some(""),
            Some("   "),
            Some("abc"),
            Some("0"),
            Some("-5"),
        ] {
            assert_eq!(
                context_budget_chars_from_env_value(raw),
                DEFAULT_CONTEXT_BUDGET_CHARS,
                "raw={raw:?} 必须回落默认值"
            );
        }
    }

    #[test]
    fn positive_values_are_parsed_verbatim_with_surrounding_whitespace_trimmed() {
        assert_eq!(context_budget_chars_from_env_value(Some("4096")), 4096);
        assert_eq!(context_budget_chars_from_env_value(Some(" 8192 ")), 8192);
        assert_eq!(context_budget_chars_from_env_value(Some("1")), 1);
    }
}
