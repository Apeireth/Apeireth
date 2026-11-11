//! 工具卡面固定数据（**两侧同源**）。
//!
//! 真源 = `harness/fixtures/toolgrid-cases.json`（JSON）；Rust 侧由
//! `harness/scripts/extract-toolgrid.mjs` **构建期生成**
//! （`toolgrid_fixture_generated.rs`）⇒ 不手抄、不漂移（抽取脚本的 `--check`
//! 模式可复核漂移）。

use apeireth_ui_kit::tool_grid::ToolSummary;

pub use super::toolgrid_fixture_generated::{toolgrid_cases, toolgrid_tools};

/// 一组对照用例（**一个搜索词**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolGridCase {
    /// 用例标识。
    pub id: String,
    /// 搜索词（空串 ⇒ 不过滤）。
    pub query: String,
}

/// 固定工具清单（供测试直接取用）。
pub fn tools() -> Vec<ToolSummary> {
    toolgrid_tools()
}

#[cfg(test)]
mod tests {
    //! fixture 的形状断言（数据本身由生成物提供）。

    use super::*;
    use apeireth_ui_kit::tool_grid::filter_tools;

    #[test]
    fn the_fixed_data_covers_six_tools() {
        assert_eq!(toolgrid_tools().len(), 6, "固定数据：6 条工具");
    }

    #[test]
    fn the_fixed_data_covers_both_availability_states() {
        let tools = toolgrid_tools();
        assert!(tools.iter().any(|tool| tool.available));
        assert!(tools.iter().any(|tool| !tool.available));
    }

    #[test]
    fn every_tool_has_a_description() {
        for tool in toolgrid_tools() {
            assert!(
                tool.description.is_some(),
                "工具 `{}` 缺描述：{tool:?}",
                tool.name
            );
        }
    }

    #[test]
    fn the_fixed_data_covers_four_search_cases() {
        assert_eq!(toolgrid_cases().len(), 4, "固定数据：4 组用例");
    }

    #[test]
    fn the_cases_cover_the_empty_the_matching_and_the_missing_queries() {
        let cases = toolgrid_cases();
        assert!(
            cases.iter().any(|case| case.query.is_empty()),
            "有不过滤的用例"
        );
        assert!(
            cases.iter().any(|case| !case.query.is_empty()
                && !filter_tools(&toolgrid_tools(), &case.query).is_empty()),
            "有命中非空的用例"
        );
        assert!(
            cases.iter().any(|case| !case.query.is_empty()
                && filter_tools(&toolgrid_tools(), &case.query).is_empty()),
            "有命中为空的用例"
        );
    }
}
