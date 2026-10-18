// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 引擎直取口：未经门面包装的下层能力，按中性模块名暴露。
//!
//! 承诺（O-5 不假装）：这里的 API 是 vendored 引擎家族的原始面，直接消费即
//! 自担其演进；门面已覆盖的主链优先走 [`crate`] 顶层接口。

/// 图像算法（滤镜/重采样/选区/修饰）。
pub use apeireth_image_algo as algo;
/// ICC 色彩管理（剖面解析/写出/变换）。
pub use apeireth_image_cms as cms;
/// 平面位图编解码。
pub use apeireth_image_codecs as codecs;
/// 色彩与混合数学（像素格式/混合模式）。
pub use apeireth_image_color as color;
/// CPU 合成器。
pub use apeireth_image_compose as compose;
/// 文档模型（图层/蒙版/调整/效果，纯数据）。
pub use apeireth_image_doc as doc;
/// 几何基元（矩形/尺寸/变换）。
pub use apeireth_image_geom as geom;
/// 分层文档格式（PSD/PSB 读写）。
pub use apeireth_image_psd as psd;
/// 像素面（写时复制瓦片）。
pub use apeireth_image_raster as raster;
/// 矢量路径/形状/描边栅格化。
pub use apeireth_image_vector as vector;
