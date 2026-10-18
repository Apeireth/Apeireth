// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 图像核心接口层（image core facade）——引擎级创作内核的中性消费口。
//!
//! 本层给后续工具面（如图像操作工具）与界面一个**稳定、中性**的 API：调用方
//! 只依赖本 crate 的类型与函数，不必直接接触下层引擎家族。下层是按「文件 1:1
//! 可对账」方式 vendored 的图像引擎家族（`apeireth-image-*`：几何 / 色彩管理 /
//! 像素面 / 文档模型 / 矢量 / 合成 / 滤镜 / 编解码 / 分层文档格式），
//! 溯源、许可与四关质检见 `docs/04-internal/license-port/`。
//!
//! 分层承诺（O-5 不假装）：
//! - 本层是**薄门面**：高层便利函数 + 稳定 re-export，不重写引擎逻辑；
//! - 尚未包装的引擎能力可经 `engine` 模块拿到原始 crate（消费方自担引擎 API 演进）；
//! - 接口冒烟测试见 `tests/smoke.rs`。

#![forbid(unsafe_code)]

pub mod engine;

mod error;
mod filters;
mod geometry;
mod layers;
mod raster;

pub use error::ImageError;
pub use filters::{blur, sharpen};
pub use geometry::{crop, resize};
pub use layers::{flatten, LayerDocument};
pub use raster::{decode, encode, OutputFormat, RasterImage};
