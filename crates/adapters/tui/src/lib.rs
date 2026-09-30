//! # apeireth-tui — 终端驾驶舱 (P1)
//!
//! 纯 Rust 全屏 TUI 客户端 (重度用户操控台), 连既有 gateway/CLI 后端。
//!
//! 职责边界 (0 重复实现): 会话生命周期、治理判定、工具执行、模型路由全部
//! 留在既有后端; 本 crate 只是它们的呈现与转发层 —— 与桌面端共用同一后端
//! 数据面, 一套语义两处上屏。
//!
//! P1 交付面:
//! - 面板 1 主对话区: 流式 Markdown 渲染 (代码高亮 / 工具卡片) + 多会话页签;
//! - 面板 2 实时状态条: 上下文用量条 / token 计数 / 回合耗时 / 缓存命中率
//!   (接口无此字段时诚实显示 "—") / 活动指示;
//! - 面板 4 会话管理: `/resume` `/new` `/compact` `/export` + 模型热切换;
//! - 快捷键: `/` 命令补全、`?` 帮助浮层、Ctrl+C 两段式退出 (先断流再退出);
//! - 驾驶舱氛围: 发光描边 / 扫描线 / 等宽跳动数字, 含 reduced-motion 档。
//!
//! P1 显式留桩 (下批接线):
//! - 面板 3 内部过程频道 —— 接口桩 `未接线`, 下批消费 events 面只读投影;
//! - 面板 5 时间倒带 (双 Esc / `/rewind`) —— 接口桩 `未接线`, 下批接
//!   continuation / 会话 fork 确定性重放。

#![forbid(unsafe_code)]

pub mod backend;
pub mod command;
pub mod effects;
pub mod gateway_http;
pub mod keys;
pub mod markdown;
pub mod render;
pub mod state;
pub mod theme;
