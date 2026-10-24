//! 工具卡面固定数据（**两侧同源**）。
//!
//! 真源 = `harness/fixtures/toolcard-cases.json`（JSON）；Rust 侧由
//! `harness/scripts/extract-assets.mjs` **构建期生成**
//! （`toolcard_fixture_generated.rs`）⇒ 不手抄、不漂移（抽取脚本的 `--check`
//! 模式可复核漂移）。

use apeireth_ui_kit::tool_card::{ToolCallDetails, ToolCallLifecycleCall};

pub use super::toolcard_fixture_generated::tool_card_cases;

/// 一组对照用例（工具卡 + 生命周期卡）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCardCase {
    /// 用例标识。
    pub id: String,
    /// 工具卡数据。
    pub tool_call: ToolCallDetails,
    /// 生命周期卡数据。
    pub lifecycle_call: ToolCallLifecycleCall,
}

#[cfg(test)]
mod tests {
    //! fixture 的形状断言（数据本身由生成物提供）。

    use super::*;

    #[test]
    fn the_fixed_data_covers_five_cases() {
        let cases = tool_card_cases();
        assert_eq!(cases.len(), 5, "固定数据：5 组用例");
    }

    #[test]
    fn the_fixed_data_covers_every_tool_status() {
        let cases = tool_card_cases();
        let statuses: Vec<_> = cases.iter().map(|case| case.tool_call.status).collect();
        for status in [
            apeireth_ui_kit::tool_card::ToolCallStatus::Pending,
            apeireth_ui_kit::tool_card::ToolCallStatus::Running,
            apeireth_ui_kit::tool_card::ToolCallStatus::Succeeded,
            apeireth_ui_kit::tool_card::ToolCallStatus::Failed,
            apeireth_ui_kit::tool_card::ToolCallStatus::Cancelled,
        ] {
            assert!(
                statuses.contains(&status),
                "固定数据缺状态 {status:?}：{statuses:?}"
            );
        }
    }

    #[test]
    fn the_fixed_data_covers_every_lifecycle_status() {
        let cases = tool_card_cases();
        let statuses: Vec<_> = cases
            .iter()
            .map(|case| case.lifecycle_call.status)
            .collect();
        for status in [
            apeireth_ui_kit::tool_card::ToolCallLifecycleStatus::Pending,
            apeireth_ui_kit::tool_card::ToolCallLifecycleStatus::Running,
            apeireth_ui_kit::tool_card::ToolCallLifecycleStatus::Success,
            apeireth_ui_kit::tool_card::ToolCallLifecycleStatus::Error,
        ] {
            assert!(
                statuses.contains(&status),
                "固定数据缺状态 {status:?}：{statuses:?}"
            );
        }
    }

    #[test]
    fn the_fixed_data_covers_the_argument_result_error_and_raw_variants() {
        let cases = tool_card_cases();
        assert!(
            cases.iter().any(|case| case.tool_call.args.is_some()),
            "有用例带参数对象"
        );
        assert!(
            cases.iter().any(|case| case.tool_call.raw_args.is_some()),
            "有用例带原始参数文本"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.tool_call.result_summary.is_some()),
            "有用例带结果摘要"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.tool_call.result_full.is_some()),
            "有用例带完整结果"
        );
        assert!(
            cases.iter().any(|case| case.tool_call.error.is_some()),
            "有用例带错误信息"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.lifecycle_call.arguments.is_some()),
            "有用例带生命周期参数"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.lifecycle_call.result.is_some()),
            "有用例带生命周期结果"
        );
    }
}
