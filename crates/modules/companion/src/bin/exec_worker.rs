//! `exec_worker` bin — 执行体隔离的 per-call 子进程.
//!
//! 被 `ToolBridge` (隔离模式) spawn: 读一行 JSON 请求 → 执行工具 → 输出一行响应 → 退出.
//! 零状态, 跑完即退, 宿主可超时 kill.

use apeireth_companion::exec_worker::run;

#[tokio::main]
async fn main() {
    run().await;
}
