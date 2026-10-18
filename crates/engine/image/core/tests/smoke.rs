// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! 接口冒烟测试：钉住门面主链的真实行为（编解码 / 几何 / 滤镜 / 图层 / 分层格式）。
//! 用例只走 `apeireth-image-core` 公开面——这正是后续工具面的消费路径。

use apeireth_image_core::{decode, encode, flatten, ImageError, LayerDocument, OutputFormat, RasterImage};

fn checker_rgba8(w: u32, h: u32) -> Vec<u8> {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for _y in 0..h {
        for x in 0..w {
            let v = if x < w / 2 { 0 } else { 255 };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    px
}

#[test]
fn png_roundtrip_preserves_pixels() {
    let pixels: Vec<u8> = (0..4u8).flat_map(|i| [i * 60, 255 - i * 60, 128, 255]).collect();
    let img = RasterImage::from_rgba8(2, 2, pixels.clone()).expect("build");
    let bytes = encode(&img, OutputFormat::Png).expect("encode png");
    let back = decode(&bytes).expect("decode png");
    assert_eq!(back.dimensions(), (2, 2));
    assert_eq!(back.data(), pixels.as_slice(), "PNG 无损往返逐字节一致");
}

#[test]
fn decode_garbage_is_an_error_not_a_panic() {
    let r = decode(b"this is definitely not an image");
    assert!(matches!(r, Err(ImageError::Codec(_))), "{r:?}");
}

#[test]
fn crop_returns_the_requested_region() {
    let mut pixels = Vec::new();
    for y in 0..4u32 {
        for x in 0..4u32 {
            pixels.extend_from_slice(&[(x * 40) as u8, (y * 40) as u8, 0, 255]);
        }
    }
    let img = RasterImage::from_rgba8(4, 4, pixels).expect("build");
    let out = img.cropped(1, 2, 2, 2).expect("crop");
    assert_eq!(out.dimensions(), (2, 2));
    assert_eq!(out.data()[0], 40, "x=1 → R=40");
    assert_eq!(out.data()[1], 80, "y=2 → G=80");
}

#[test]
fn crop_rejects_an_empty_region() {
    let img = RasterImage::from_rgba8(2, 2, vec![0; 16]).expect("build");
    assert!(matches!(img.cropped(0, 0, 0, 2), Err(ImageError::InvalidArgument(_))));
}

#[test]
fn resize_scales_dimensions() {
    let img = RasterImage::from_rgba8(2, 2, checker_rgba8(2, 2)).expect("build");
    let out = img.resized(4, 6).expect("resize");
    assert_eq!(out.dimensions(), (4, 6));
}

#[test]
fn blur_softens_a_hard_edge() {
    let img = RasterImage::from_rgba8(8, 8, checker_rgba8(8, 8)).expect("build");
    let back = img.blurred(2.0).expect("blur");
    assert_eq!(back.dimensions(), (8, 8), "模糊不改尺寸");
    let edge = back.data()[((4 * 8 + 3) * 4) as usize];
    assert!(edge > 0 && edge < 255, "硬边被软化，实测 edge={edge}");
}

#[test]
fn sharpen_changes_edge_pixels() {
    // 中灰阶梯边（64|192）：纯黑白边经 USM 过冲会被 0/255 钳位吃掉，钉行为要用灰阶。
    let px = soft_step_rgba8(8, 8);
    let img = RasterImage::from_rgba8(8, 8, px.clone()).expect("build");
    let back = img.sharpened(150.0, 2.0, 0.0).expect("sharpen");
    assert_eq!(back.dimensions(), (8, 8));
    assert_ne!(back.data(), px.as_slice(), "USM 锐化确实改写了像素");
}

fn soft_step_rgba8(w: u32, h: u32) -> Vec<u8> {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for _y in 0..h {
        for x in 0..w {
            let v = if x < w / 2 { 64 } else { 192 };
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    px
}

#[test]
fn layer_flatten_blends_over_the_white_background() {
    let red: Vec<u8> = (0..2u32).flat_map(|_| [255, 0, 0, 255]).collect();
    let layer = RasterImage::from_rgba8(2, 1, red).expect("build");
    let mut doc = LayerDocument::new("smoke", 2, 1);
    doc.add_raster_layer("red", &layer, 0.5).expect("push layer");
    assert_eq!(doc.layer_count(), 2, "白底背景层 + 新加栅格层");
    let flat = flatten(&doc);
    assert_eq!((flat.width, flat.height), (2, 1));
    let p = &flat.pixels[0..4];
    assert_eq!(p[0], 255, "红通道满值");
    assert!((127..=129).contains(&p[1]), "半透明红叠白底 → 绿≈128，实测 {}", p[1]);
    assert!((127..=129).contains(&p[2]), "半透明红叠白底 → 蓝≈128，实测 {}", p[2]);
}

#[test]
fn psd_roundtrip_is_byte_stable() {
    use apeireth_image_core::engine::psd::{LayerSpec, PixelData, PsdBuilder, PsdFile};

    let mut b = PsdBuilder::new(2, 1);
    b.push_layer(LayerSpec::new("Red", 0, 0, 2, 1, PixelData::Rgba8(vec![255, 0, 0, 255, 255, 0, 0, 128])));
    b.composite(PixelData::Rgba8(vec![255, 0, 0, 255, 255, 0, 0, 128]));
    let bytes = b.to_bytes().expect("write psd");

    let file = PsdFile::from_bytes(&bytes).expect("read psd");
    assert_eq!(file.to_bytes().expect("rewrite"), bytes, "分层文档字节级稳定往返");
    let layer = file.layer(0).expect("layer 0");
    assert_eq!(layer.name(), "Red");
    assert_eq!(layer.rgba8().expect("pixels").data[7], 128);
}
