// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 滤镜操作面：第一批收口「模糊 / 锐化」两个高频入口；
//! 噪声/扭曲/风格化等其余滤镜经 [`crate::engine::algo`] 直取。
//!
//! 与引擎面一致：作用区域由调用方以 `region` 指明（瓦片面不自带逻辑边界）。

use apeireth_image_algo::{apply, FilterParams};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

/// 高斯模糊（`radius` = 标准差，像素，引擎面有效域 0.1–1000）。
pub fn blur(surface: &Surface, region: Rect, radius: f32) -> Surface {
    apply(surface, &FilterParams::GaussianBlur { radius }, region, region, None)
}

/// USM 锐化（`amount` %、`radius` 像素、`threshold` 层级）。
pub fn sharpen(surface: &Surface, region: Rect, amount: f32, radius: f32, threshold: f32) -> Surface {
    apply(surface, &FilterParams::UnsharpMask { amount, radius, threshold }, region, region, None)
}
