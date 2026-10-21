//! 集成测试共享夹具 (D26, 2026-10-21)。
//!
//! 回环 bind 助手与 crate 内单测共用**同一份源文件** (`src/test_support.rs`),
//! 用 `#[path]` 引入, 不复制实现、不新增 workspace member。
//! 用法: 测试文件顶部 `mod common;`, 然后 `common::bind_loopback_ephemeral().await`。
//!
//! 为什么不在各测试文件里直接 `TcpListener::bind(...).unwrap()`: Windows 上
//! 临时端口偶发落进系统保留区间 → `WSAEACCES (os error 10013)`;
//! 该助手只对这一类错误有界重试, 其它错误照常失败 (见 `src/test_support.rs`)。
#![allow(dead_code)]

#[path = "../../src/test_support.rs"]
mod test_support;

pub use test_support::bind_loopback_ephemeral;
