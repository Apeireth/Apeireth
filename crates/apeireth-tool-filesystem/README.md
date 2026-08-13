# apeireth-tool-filesystem

**R137** — 文件系统操作工具

## 职责

吸收 VCP FileOperator / FileListGenerator / FileTreeGenerator / ImageFileServer / CapturePreprocessor 5 职责, 但用 Rust trait 拆分为 9 sub-module.

## 核心模块

- read.rs / write.rs / edit.rs / list.rs / search.rs / diff.rs / sandbox.rs / watch.rs / image_server.rs

## 借鉴 vs 超越

VCP FileOperator 70KB 5 职责塞一进程 → 我们 9 模块分离, 单一职责.

## 0 假装

✅ 10 单元测试 | ✅ fsnotify 真实集成 | ✅ sandbox 路径校验

## R162 lint cleanup

88 -> 0 warnings. Unused `PathBuf` import removed from lock.rs.
