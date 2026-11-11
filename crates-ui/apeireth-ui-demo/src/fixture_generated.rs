//! 固定数据（**自动生成** —— 勿手改）。
//!
//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/pilot-request.json`
//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-assets.mjs`

use crate::approval_card::ApprovalItem;

/// 对照用的固定审批项（两侧喂同一份数据）。
pub fn pilot_item() -> ApprovalItem {
    ApprovalItem {
        title: Some(String::from("读取卷宗目录")),
        command_text: Some(String::from("apeireth fs list --scope workspace")),
        arguments_summary: Some(String::from("{\"path\": \"/卷宗/工作区\", \"depth\": 1}")),
        reason: Some(String::from("需要列出目录才能定位目标文件")),
        created_at: Some(1761354600000),
        sandbox: Some(String::from("工作区沙箱")),
        cwd: Some(String::from("/卷宗/工作区")),
        isolation: Some(String::from("文件可写 / 网络断")),
    }
}
