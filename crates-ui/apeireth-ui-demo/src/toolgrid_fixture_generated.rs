//! 工具卡面固定数据（**自动生成** —— 勿手改）。
//!
//! 来源：`crates-ui/apeireth-ui-demo/harness/fixtures/toolgrid-cases.json`
//! 生成器：`crates-ui/apeireth-ui-demo/harness/scripts/extract-toolgrid.mjs`

use super::toolgrid_fixture::ToolGridCase;
use apeireth_ui_kit::tool_grid::ToolSummary;

/// 对照用的固定工具清单（两侧喂同一份数据）。
pub fn toolgrid_tools() -> Vec<ToolSummary> {
    vec![
        ToolSummary {
            name: String::from("fs.read"),
            description: Some(String::from("读取工作区内的文件内容，按路径返回文本。")),
            available: true,
        },
        ToolSummary {
            name: String::from("fs.write"),
            description: Some(String::from("把内容写入工作区内的指定路径。")),
            available: true,
        },
        ToolSummary {
            name: String::from("shell.exec"),
            description: Some(String::from(
                "在工作区内执行受控的 shell 命令，输出按行返回。",
            )),
            available: true,
        },
        ToolSummary {
            name: String::from("memory.search"),
            description: Some(String::from("在长期记忆里检索与查询相关的片段。")),
            available: false,
        },
        ToolSummary {
            name: String::from("audit.read"),
            description: Some(String::from("读取审计日志的最近记录。")),
            available: false,
        },
        ToolSummary {
            name: String::from("web.fetch"),
            description: Some(String::from("抓取远端页面并转成纯文本。")),
            available: true,
        },
    ]
}

/// 对照用的固定用例（每个用例 = 一个搜索词）。
pub fn toolgrid_cases() -> Vec<ToolGridCase> {
    vec![
        ToolGridCase {
            id: String::from("all"),
            query: String::from(""),
        },
        ToolGridCase {
            id: String::from("by-name"),
            query: String::from("fs"),
        },
        ToolGridCase {
            id: String::from("by-description"),
            query: String::from("读取"),
        },
        ToolGridCase {
            id: String::from("no-match"),
            query: String::from("无此工具"),
        },
    ]
}
