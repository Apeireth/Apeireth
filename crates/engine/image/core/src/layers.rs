// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 图层文档面：建文档、叠图层、合成出图（引擎合成器）。
//!
//! 第一批只收口「栅格图层 + 合成」主链；调整图层/图层样式/蒙版/矢量图层等
//! 能力在 [`crate::engine::doc`] / [`crate::engine::compose`] 直取。

use apeireth_image_color::{Color, ColorMode, SampleType};
use apeireth_image_doc::{Document, Layer, LayerContent, Size};
use apeireth_image_raster::Rgba8Image;

use crate::error::ImageError;
use crate::raster::RasterImage;

/// 分层文档（接口层壳；内核是引擎文档模型）。
#[derive(Debug, Clone)]
pub struct LayerDocument {
    doc: Document,
    width: u32,
    height: u32,
}

impl LayerDocument {
    /// 新建画布（白底、RGB、U8）。
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self { doc: Document::with_background(name, Size::new(width, height), ColorMode::Rgb, SampleType::U8, Color::WHITE), width, height }
    }

    /// 追加一个栅格图层（`opacity`：0.0 全透明 – 1.0 不透明）。
    pub fn add_raster_layer(&mut self, name: &str, image: &RasterImage, opacity: f32) -> Result<(), ImageError> {
        let surface = image.to_surface()?;
        let mut layer = Layer::new(name, LayerContent::Raster(surface));
        layer.opacity = opacity.clamp(0.0, 1.0);
        self.doc.layers.push(layer);
        Ok(())
    }

    /// 图层数。
    pub fn layer_count(&self) -> usize {
        self.doc.layers.len()
    }

    /// 尺寸 `(w, h)`。
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 引擎文档模型直取（调整图层等进阶面用）。
    pub fn engine_document(&self) -> &Document {
        &self.doc
    }
}

/// 合成出图（引擎合成器，直通 RGBA8）。
pub fn flatten(doc: &LayerDocument) -> Rgba8Image {
    apeireth_image_compose::flatten(&doc.doc).to_rgba8()
}
