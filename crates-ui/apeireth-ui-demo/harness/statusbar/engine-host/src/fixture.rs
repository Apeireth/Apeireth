//! 固定数据的**类型**（数据本体由抽取器生成 ⇒ `fixture_generated.rs`）。
//!
//! 真源 = `fixtures/statusbar-cases.json`（参考侧直接 import 该 JSON）⇒
//! 两侧喂**同一份**。

use apeireth_ui_kit::status_bar::{StatusBadgeSize, StatusBadgeVariant, StatusTone};

/// 对照用例里的**一格**。
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBarCaseItem {
    /// 列表身份。
    pub key: String,
    /// 条上小字。
    pub text: String,
    /// 色态。
    pub tone: StatusTone,
    /// 悬停解释。
    pub title: String,
    /// 可点 = 是入口。
    pub clickable: bool,
    /// 「模拟值」标注。
    pub sim: bool,
}

/// 对照用例（状态条四格 + 一枚徽标）。
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBarCase {
    /// 用例标识。
    pub id: String,
    /// 状态条上的四格。
    pub items: Vec<StatusBarCaseItem>,
    /// 徽标变体。
    pub badge_variant: StatusBadgeVariant,
    /// 徽标文案。
    pub badge_label: String,
    /// 徽标尺寸。
    pub badge_size: StatusBadgeSize,
}
