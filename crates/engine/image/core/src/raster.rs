// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 栅格图像面：编解码进出 + 与引擎像素面（Surface）的薄桥。
//!
//! [`RasterImage`] 是工具面/界面消费的主类型：解码进、编码出，几何与滤镜便利
//! 方法委托给引擎面（见 [`crate::geometry`] / [`crate::filters`]）。桥接只做
//! 「交错字节 ↔ 瓦片面」的搬运，不改写引擎像素逻辑。

use apeireth_image_codecs::{ChannelLayout, EncodeOptions, Format, Image, SampleType};
use apeireth_image_color::{ColorMode, PixelFormat};
use apeireth_image_geom::Rect;
use apeireth_image_raster::Surface;

use crate::error::ImageError;

/// 编码目标格式（第一批：平面位图四格式；其余走 [`crate::engine`] 直取）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// PNG（无损，含调色板/交错等选项面由引擎直取）。
    Png,
    /// JPEG（有损）。
    Jpeg,
    /// TIFF（多页/深位深面由引擎直取）。
    Tiff,
    /// WebP。
    WebP,
}

impl From<OutputFormat> for Format {
    fn from(f: OutputFormat) -> Format {
        match f {
            OutputFormat::Png => Format::Png,
            OutputFormat::Jpeg => Format::Jpeg,
            OutputFormat::Tiff => Format::Tiff,
            OutputFormat::WebP => Format::WebP,
        }
    }
}

/// 一张平面栅格图（像素 + 嵌入元数据），接口层的主消费类型。
#[derive(Debug, Clone, PartialEq)]
pub struct RasterImage(pub(crate) Image);

/// 解码任意受支持格式的字节流（格式自动识别）。
pub fn decode(bytes: &[u8]) -> Result<RasterImage, ImageError> {
    Ok(RasterImage(apeireth_image_codecs::decode(bytes)?))
}

/// 编码为目标格式字节流。
pub fn encode(image: &RasterImage, format: OutputFormat) -> Result<Vec<u8>, ImageError> {
    Ok(apeireth_image_codecs::encode(&image.0, format.into(), &EncodeOptions::default())?)
}

impl RasterImage {
    /// 由交错 RGBA8 像素行主序构建。
    pub fn from_rgba8(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, ImageError> {
        Ok(RasterImage(Image::from_u8(width, height, ChannelLayout::Rgba, pixels)?))
    }

    /// 宽（像素）。
    pub fn width(&self) -> u32 {
        self.0.width()
    }

    /// 高（像素）。
    pub fn height(&self) -> u32 {
        self.0.height()
    }

    /// 尺寸 `(w, h)`。
    pub fn dimensions(&self) -> (u32, u32) {
        (self.0.width(), self.0.height())
    }

    /// 交错原始字节（原生端序采样，行主序）。
    pub fn data(&self) -> &[u8] {
        self.0.data()
    }

    /// 通道布局。
    pub fn layout(&self) -> ChannelLayout {
        self.0.layout()
    }

    /// 嵌入的 ICC 描述（若有）。
    pub fn icc(&self) -> Option<&[u8]> {
        self.0.icc.as_deref()
    }

    /// 转成引擎瓦片像素面（第一批桥：Gray/GrayA/Rgb/Rgba 全采样深；CMYK/Lab 走
    /// [`crate::engine`] 直取）。
    pub fn to_surface(&self) -> Result<Surface, ImageError> {
        let fmt = pixel_format(self.0.layout(), self.0.sample_type())?;
        let mut s = Surface::new(fmt);
        let r = Rect::from_xywh(0, 0, self.0.width(), self.0.height());
        s.write_interleaved(r, self.0.data());
        Ok(s)
    }

    /// 从引擎瓦片像素面取回平面图（第一层桥的逆向；取 `region` 区域）。
    pub fn from_surface(surface: &Surface, region: Rect) -> Result<Self, ImageError> {
        if region.width() == 0 || region.height() == 0 {
            return Err(ImageError::InvalidArgument("empty region".into()));
        }
        let bytes = surface.try_to_interleaved(region).map_err(|e| ImageError::Allocation(format!("{e:?}")))?;
        let fmt = surface.format();
        let (layout, sample) = channel_layout(fmt)?;
        Ok(RasterImage(Image::from_raw(region.width(), region.height(), layout, sample, bytes)?))
    }

    /// 几何裁切（像素拷贝，区域须落在图内）。
    pub fn cropped(&self, x: u32, y: u32, w: u32, h: u32) -> Result<RasterImage, ImageError> {
        let s = self.to_surface()?;
        let out = crate::geometry::crop(&s, self.bounds(), Rect::from_xywh(x as i32, y as i32, w, h))?;
        RasterImage::from_surface(&out, Rect::from_xywh(0, 0, w, h))
    }

    /// 缩放到目标尺寸（双三次重采样）。
    pub fn resized(&self, width: u32, height: u32) -> Result<RasterImage, ImageError> {
        let s = self.to_surface()?;
        let out = crate::geometry::resize(&s, self.bounds(), apeireth_image_geom::Size::new(width, height))?;
        RasterImage::from_surface(&out, Rect::from_xywh(0, 0, width, height))
    }

    /// 高斯模糊（radius = 标准差，像素）。
    pub fn blurred(&self, radius: f32) -> Result<RasterImage, ImageError> {
        let s = self.to_surface()?;
        let out = crate::filters::blur(&s, self.bounds(), radius);
        RasterImage::from_surface(&out, self.bounds())
    }

    /// USM 锐化（amount %，radius 像素，threshold 层级）。
    pub fn sharpened(&self, amount: f32, radius: f32, threshold: f32) -> Result<RasterImage, ImageError> {
        let s = self.to_surface()?;
        let out = crate::filters::sharpen(&s, self.bounds(), amount, radius, threshold);
        RasterImage::from_surface(&out, self.bounds())
    }

    /// 本图的逻辑区域（`(0,0,w,h)`）。
    fn bounds(&self) -> Rect {
        Rect::from_xywh(0, 0, self.0.width(), self.0.height())
    }
}

fn pixel_format(layout: ChannelLayout, sample: SampleType) -> Result<PixelFormat, ImageError> {
    let mode = match layout {
        ChannelLayout::Gray | ChannelLayout::GrayA => ColorMode::Grayscale,
        ChannelLayout::Rgb | ChannelLayout::Rgba => ColorMode::Rgb,
        other => return Err(ImageError::Unsupported(format!("{other:?} layout: bridge covers Gray/GrayA/Rgb/Rgba"))),
    };
    let alpha = matches!(layout, ChannelLayout::GrayA | ChannelLayout::Rgba);
    Ok(PixelFormat { mode, sample: sample_type(sample)?, alpha })
}

fn channel_layout(fmt: PixelFormat) -> Result<(ChannelLayout, SampleType), ImageError> {
    let layout = match (fmt.mode, fmt.alpha) {
        (ColorMode::Grayscale, false) => ChannelLayout::Gray,
        (ColorMode::Grayscale, true) => ChannelLayout::GrayA,
        (ColorMode::Rgb, false) => ChannelLayout::Rgb,
        (ColorMode::Rgb, true) => ChannelLayout::Rgba,
        (other, _) => return Err(ImageError::Unsupported(format!("{other:?} mode: bridge covers Grayscale/Rgb"))),
    };
    Ok((layout, codec_sample(fmt.sample)))
}

fn sample_type(s: SampleType) -> Result<apeireth_image_color::SampleType, ImageError> {
    Ok(match s {
        SampleType::U8 => apeireth_image_color::SampleType::U8,
        SampleType::U16 => apeireth_image_color::SampleType::U16,
        SampleType::F32 => apeireth_image_color::SampleType::F32,
        other => return Err(ImageError::Unsupported(format!("{other:?} sample: bridge covers U8/U16/F32"))),
    })
}

fn codec_sample(s: apeireth_image_color::SampleType) -> SampleType {
    match s {
        apeireth_image_color::SampleType::U8 => SampleType::U8,
        apeireth_image_color::SampleType::U16 => SampleType::U16,
        apeireth_image_color::SampleType::F32 => SampleType::F32,
    }
}
