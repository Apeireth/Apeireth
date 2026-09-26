//! 本地类型检查 / 冒烟桩 crate 的入口。
//!
//! `mirror` 模块 = research/verification/kani/src/lib.rs 的同一份源文件
//! (#[path] 零复制); 其内部相对路径仍以该文件所在目录为基准, canonical
//! 引用与 harness 引用全部解析到同一份真实代码。`--cfg kani` 下 harness
//! 模块参与编译, 由 kani 桩提供 `kani::any` / `#[kani::proof]`。

#![allow(dead_code, unused_imports, unused_variables, unused_qualifications)]

#[path = "../../kani/src/lib.rs"]
pub mod mirror;
