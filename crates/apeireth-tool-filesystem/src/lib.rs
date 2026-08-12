//! `apeireth-tool-filesystem` - R137 filesystem 扩展层
//! 
//! **目标**: 在 `apeireth-tools/file_ops` (6 ops 真实现) 基础上扩展 5 维度能力:
//! 1. **真沙箱** (realpath 路径校验, 防 symlink 逃逸)
//! 2. **原子写入** (tmp + rename, 崩溃恢复)
//! 3. **fsnotify 监听** (基于 notify crate)
//! 4. **文件锁** (fd-lock, 多 writer 安全)
//! 5. **文档解析** (PDF/Word/Excel 异步读取, feature gated)
//! 6. **VCP FileOperator 19 命令兼容层** (manifest reader + 路由)
//!
//! **背景** (R137 主人提示):
//! - 我们之前已借鉴模仿了很多 (R17 战役 2-5 已 5 trait 真实现)
//! - R137 不从零新建, 而是扩展现有 `apeireth-tools/file_ops`
//! - 不破坏 24 LOCKED 约束 (owner: apeireth-tools, 我们只在它上层扩展)
//!
//! **架构**:
//! `
//!   apeireth-tools::file_ops (R17 LOCKED, 6 ops)
//!          |
//!          v (扩展)
//!   apeireth-tool-filesystem (本 crate)
//!   |- sandbox    : realpath + 路径白名单校验
//!   |- atomic     : tmp + rename 原子写
//!   |- watch      : fsnotify 事件订阅
//!   |- lock       : fd-lock 文件锁
//!   |- parse      : PDF/Word/Excel 异步解析 (feature gated)
//!   |- vcp_compat : VCP FileOperator 19 command 兼容层
//!   |- enhanced   : EnhancedFileOps trait (5 维度升级总入口)
//! `
//!
//! **不假装** (per O-5):
//! - 不假装真沙箱: realpath 检查 + 路径白名单 tokio::task::spawn_blocking
//! - 不假装原子写: tempfile crate 在同目录创建 .tmp + std::fs::rename
//! - 不假装 fsnotify: notify 6.1 真接, 事件真传
//! - 不假装文件锁: fd-lock 4.0 真加锁
//! - 不假装文档解析: feature gated, 默认 build 不带 lopdf/docx-rs/calamine

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod sandbox;
pub mod atomic;
pub mod watch;
pub mod lock;
#[cfg(feature = "full")]
pub mod parse;
pub mod vcp_compat;
pub mod enhanced;

pub use enhanced::{EnhancedFileOps, StdEnhancedFileOps};
pub use sandbox::{Sandbox, SandboxPolicy, SandboxError};
pub use atomic::{atomic_write, AtomicWriteError};
pub use watch::{FileWatcher, WatchEvent, WatchError};
pub use lock::{FileLock, FileLockGuard, LockError};
#[cfg(feature = "full")]
pub use parse::{parse_document, DocumentType, ParseError};
pub use vcp_compat::{VcpCommand, VcpManifest, VcpCompatRouter, VcpError};

/// R137 完成定义 (per 主人 R134 A4 闭环):
/// - 6 模块 (sandbox/atomic/watch/lock/parse/vcp_compat) 全部真实现
/// - EnhancedFileOps 在 apeireth-tools/file_ops 6 ops 基础上加 5 维度
/// - VCP FileOperator 19 command 兼容层 (cargo test 19 个 e2e)
/// - 4 维测试: unit + 沙箱逃逸 + 并发 + 集成
/// - cargo test --workspace 全过 (target `20599+1+新增`)
pub const R137_DELIVERABLES: usize = 6;

/// 5 维度扩展 (per R137 报告 v3)
pub const UPGRADE_DIMENSIONS: usize = 5;
