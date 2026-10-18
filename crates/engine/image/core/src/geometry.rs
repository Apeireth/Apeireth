// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 几何操作面：裁切与缩放（引擎重采样），第一批只做正交几何；
//! 旋转/透视/形变等经 [`crate::engine::algo`] 直取。
//!
//! 像素面是瓦片粒度的（引擎面 256px 瓦片），**逻辑区域由调用方指明**——
//! 这与引擎面自身的 area/bounds 传参风格一致，不替调用方猜内容边界。

use apeireth_image_algo::resample::{resize_surface, Resample};
use apeireth_image_geom::{Rect, Size};
use apeireth_image_raster::Surface;

use crate::error::ImageError;

/// 裁切出 `region`（须落在 `source` 内且非空），返回逻辑区域为 `(0,0,w,h)` 的新像素面。
pub fn crop(surface: &Surface, source: Rect, region: Rect) -> Result<Surface, ImageError> {
    if region.width() == 0 || region.height() == 0 {
        return Err(ImageError::InvalidArgument("empty crop region".into()));
    }
    let inside = region.x0 >= source.x0 && region.y0 >= source.y0 && region.x1 <= source.x1 && region.y1 <= source.y1;
    if !inside {
        return Err(ImageError::InvalidArgument(format!("crop region {region:?} leaves source {source:?}")));
    }
    let bytes = surface.try_to_interleaved(region).map_err(|e| ImageError::Allocation(format!("{e:?}")))?;
    let mut out = Surface::new(surface.format());
    out.write_interleaved(Rect::from_xywh(0, 0, region.width(), region.height()), &bytes);
    Ok(out)
}

/// 把 `source` 区域缩放到 `target` 尺寸（双三次重采样）。
pub fn resize(surface: &Surface, source: Rect, target: Size) -> Result<Surface, ImageError> {
    if target.width == 0 || target.height == 0 {
        return Err(ImageError::InvalidArgument("resize target must be non-zero".into()));
    }
    if source.width() == 0 || source.height() == 0 {
        return Err(ImageError::InvalidArgument("empty source region".into()));
    }
    let sx = f64::from(target.width) / f64::from(source.width());
    let sy = f64::from(target.height) / f64::from(source.height());
    Ok(resize_surface(surface, sx, sy, Resample::Bicubic))
}
