//! 创作工具面 W1 (`tool.image_ops`) 换创作内核验收面:
//! ① 换核前后像素级一致性抽样 —— 几何类 (crop/rotate/flip) 与通道算术类
//!    (brightness/contrast/grayscale) 逐像素**全等** (换核前参照 = 换核前同款
//!    调用在 dev 依赖图像库上重算); 重采样/滤镜类 (resize/blur/sharpen) 内核
//!    换档 (线性核/引擎高斯/引擎 USM), 逐像素全等不成立, 如实钉**偏差上界 +
//!    不变量** (纯色不漂移 / 尺寸精确 / 能量摊布)。
//! ② 新操作行为钉 —— `layers_composite` (核面合成数学, 逐像素钉值) 与
//!    `psd_read_info` (核面分层文档只读探针), 同款参数校验矩阵拒绝即帧。
//!
//! 断言全部走**像素级**真值, 不钉字面。

use std::path::Path;
use std::sync::Arc;

use apeireth_image_core::engine::psd::{ColorMode, LayerSpec, PixelData, PsdBuilder, Version};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::{
    ImageOperation, ImageOpsActivity, ImageOpsTool, ObservedGate, OperationSpec,
    MAX_PSD_LAYER_NAMES, OPERATION_VOCABULARY,
};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};

fn tool(root: &Path) -> ImageOpsTool {
    ImageOpsTool::new(
        root.to_path_buf(),
        Arc::new(ObservedGate::new()),
        Arc::new(ImageOpsActivity::new()),
    )
}

async fn invoke(tool: &ImageOpsTool, arguments: Value) -> ToolResult {
    let call = ToolCall {
        id: format!("call_{}", std::process::id()),
        name: "image_ops".into(),
        arguments,
    };
    tool.invoke(&call).await
}

fn args(input: &str, output: &str, operations: Value) -> Value {
    json!({ "input": input, "output": output, "operations": operations })
}

fn image_with(width: u32, height: u32, pixel: impl Fn(u32, u32) -> Rgba<u8>) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, pixel(x, y));
        }
    }
    img
}

fn write_png(img: &RgbaImage, path: &Path) {
    img.save(path).expect("fixture png");
}

fn read_rgba(path: &Path) -> RgbaImage {
    image::open(path).expect("decode output").to_rgba8()
}

/// 逐坐标唯一特征像素 (含 alpha 渐变 —— 预乘/反预乘路径也钉进来)。
fn feature_pixel(x: u32, y: u32) -> Rgba<u8> {
    Rgba([
        (x * 31 + 7) as u8,
        (y * 47 + 13) as u8,
        (x * 17 + y * 23 + 29) as u8,
        (255 - ((x + y) * 19) as u8),
    ])
}

fn assert_frame(result: &ToolResult, code: &str, needle: &str) {
    assert!(
        !result.is_ok(),
        "expected refusal frame, got {}",
        result.render()
    );
    let rendered = result.render();
    assert!(
        rendered.starts_with(&format!("error: {code}")),
        "frame code must be {code}: {rendered}"
    );
    assert!(
        rendered.contains(needle),
        "frame must name {needle}: {rendered}"
    );
}

fn max_deviation(a: &RgbaImage, b: &RgbaImage) -> u16 {
    assert_eq!(
        a.dimensions(),
        b.dimensions(),
        "dims must match before diffing"
    );
    let mut worst = 0u16;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for (ca, cb) in pa.0.into_iter().zip(pb.0) {
            worst = worst.max(ca.abs_diff(cb).into());
        }
    }
    worst
}

// ── 换核前参照 (换核前实现同款调用, dev 依赖图像库重算) ─────────────────────

fn pre_swap_resize(img: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    image::imageops::resize(img, width, height, FilterType::Triangle)
}

fn pre_swap_blur(img: &RgbaImage, sigma: f32) -> RgbaImage {
    image::imageops::blur(img, sigma)
}

fn pre_swap_sharpen(img: &RgbaImage, sigma: f32, threshold: i32) -> RgbaImage {
    image::imageops::unsharpen(img, sigma, threshold)
}

/// 换核前 jpeg 编码参照 (换核前同款调用: 白底合成 + 质量 90 编码 + 解码)。
fn pre_swap_jpeg_roundtrip(img: &RgbaImage) -> RgbaImage {
    use image::ImageEncoder;
    let (width, height) = img.dimensions();
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for pixel in img.pixels() {
        let Rgba([r, g, b, a]) = *pixel;
        let alpha = f64::from(a) / 255.0;
        let blend =
            |channel: u8| (f64::from(channel) * alpha + 255.0 * (1.0 - alpha)).round() as u8;
        rgb.push(blend(r));
        rgb.push(blend(g));
        rgb.push(blend(b));
    }
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 90)
        .write_image(&rgb, width, height, image::ExtendedColorType::Rgb8)
        .expect("reference jpeg encode");
    image::load_from_memory(&encoded)
        .expect("reference jpeg decode")
        .to_rgba8()
}

/// ① 换核前后一致性·几何类: crop/rotate/flip 是像素置换, 换核前后必须
///    **逐像素全等** (含 alpha 渐变样本, 预乘往返不得留痕)。
#[tokio::test]
async fn core_swap_geometry_ops_match_the_pre_swap_reference_pixel_for_pixel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let source = image_with(7, 5, feature_pixel);
    write_png(&source, &root.join("in.png"));
    let tool = tool(root);

    let cases: Vec<(&str, Value, RgbaImage)> = vec![
        (
            "crop.png",
            json!([{ "op": "crop", "x": 2, "y": 1, "width": 4, "height": 3 }]),
            image::imageops::crop_imm(&source, 2, 1, 4, 3).to_image(),
        ),
        (
            "rot90.png",
            json!([{ "op": "rotate", "angle_deg": 90 }]),
            image::imageops::rotate90(&source),
        ),
        (
            "rot180.png",
            json!([{ "op": "rotate", "angle_deg": 180 }]),
            image::imageops::rotate180(&source),
        ),
        (
            "rot270.png",
            json!([{ "op": "rotate", "angle_deg": 270 }]),
            image::imageops::rotate270(&source),
        ),
        (
            "fliph.png",
            json!([{ "op": "flip", "axis": "horizontal" }]),
            image::imageops::flip_horizontal(&source),
        ),
        (
            "flipv.png",
            json!([{ "op": "flip", "axis": "vertical" }]),
            image::imageops::flip_vertical(&source),
        ),
    ];
    for (output, operations, expected) in cases {
        let result = invoke(&tool, args("in.png", output, operations)).await;
        assert!(result.is_ok(), "{output}: {}", result.render());
        let produced = read_rgba(&root.join(output));
        assert_eq!(
            produced, expected,
            "{output}: 换核前后几何类必须逐像素全等 (含 alpha)"
        );
    }
}

/// ② 换核前后一致性·通道算术类: brightness/contrast/grayscale 保留自研,
///    与换核前同公式独立重算逐像素全等 (饱和与舍入如实)。
#[tokio::test]
async fn core_swap_channel_math_matches_the_pre_swap_reference_pixel_for_pixel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let source = image_with(5, 4, feature_pixel);
    write_png(&source, &root.join("in.png"));
    let tool = tool(root);

    // brightness +37 (饱和加, alpha 不动)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "b.png",
            json!([{ "op": "brightness", "amount": 37 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let expected = image_with(5, 4, |x, y| {
        let Rgba([r, g, b, a]) = feature_pixel(x, y);
        let clamp = |v: u8| (i32::from(v) + 37).clamp(0, 255) as u8;
        Rgba([clamp(r), clamp(g), clamp(b), a])
    });
    assert_eq!(
        read_rgba(&root.join("b.png")),
        expected,
        "brightness 换核前后全等"
    );

    // contrast 1.5 (以 128 为轴线性拉伸, 四舍五入 + 饱和, alpha 不动)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "c.png",
            json!([{ "op": "contrast", "factor": 1.5 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let expected = image_with(5, 4, |x, y| {
        let Rgba([r, g, b, a]) = feature_pixel(x, y);
        let stretch = |v: u8| {
            let s = (f64::from(v) - 128.0).mul_add(1.5, 128.0).round();
            s.clamp(0.0, 255.0) as u8
        };
        Rgba([stretch(r), stretch(g), stretch(b), a])
    });
    assert_eq!(
        read_rgba(&root.join("c.png")),
        expected,
        "contrast 换核前后全等"
    );

    // grayscale ((77r + 150g + 29b + 128) >> 8, alpha 不动)。
    let result = invoke(
        &tool,
        args("in.png", "g.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let expected = image_with(5, 4, |x, y| {
        let Rgba([r, g, b, a]) = feature_pixel(x, y);
        let weighted = 77 * u32::from(r) + 150 * u32::from(g) + 29 * u32::from(b) + 128;
        let value = (weighted >> 8).min(255) as u8;
        Rgba([value, value, value, a])
    });
    assert_eq!(
        read_rgba(&root.join("g.png")),
        expected,
        "grayscale 换核前后全等"
    );
}

/// ③ 换核前后一致性·resize: 内核换档 (三角核 → 核面画布内线性核), 逐像素
///    全等不成立 —— 如实钉契约不变量 (尺寸精确 / 纯色逐像素不漂移) + 偏差
///    上界 (抽样实测: 非均匀图 17x13→9x7 与 2 倍放大, 每通道偏差 27/255 与
///    1/255, 上界登记 32/255)。
#[tokio::test]
async fn core_swap_resize_keeps_the_contract_within_the_documented_kernel_delta() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let gradient = image_with(17, 13, |x, y| {
        Rgba([
            (x * 15) as u8,
            (y * 19) as u8,
            (x * 7 + y * 11) as u8,
            (200 + (x % 2) * 55) as u8,
        ])
    });
    write_png(&gradient, &root.join("grad.png"));
    write_png(
        &image_with(6, 6, |_, _| Rgba([200, 100, 50, 255])),
        &root.join("flat.png"),
    );
    let tool = tool(root);

    for (width, height) in [(9u32, 7u32), (34, 26)] {
        let result = invoke(
            &tool,
            args(
                "grad.png",
                "out.png",
                json!([{ "op": "resize", "width": width, "height": height }]),
            ),
        )
        .await;
        assert!(result.is_ok(), "{}", result.render());
        let produced = read_rgba(&root.join("out.png"));
        assert_eq!(produced.dimensions(), (width, height), "尺寸精确");
        let reference = pre_swap_resize(&gradient, width, height);
        let worst = max_deviation(&produced, &reference);
        assert!(
            worst <= 32,
            "换核前后 resize 每通道偏差 {worst} 超出登记上界 32 (三角核 vs 线性核; 抽样实测 27)"
        );
    }

    // 契约不变量: 纯色图缩放逐像素不漂移 (含 alpha)。
    let result = invoke(
        &tool,
        args(
            "flat.png",
            "flat-out.png",
            json!([{ "op": "resize", "width": 3, "height": 5 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let produced = read_rgba(&root.join("flat-out.png"));
    for y in 0..5 {
        for x in 0..3 {
            assert_eq!(
                image::GenericImageView::get_pixel(&produced, x, y).0,
                [200, 100, 50, 255],
                "纯色缩放不得漂移 at {x},{y}"
            );
        }
    }
}

/// ④ 换核前后一致性·blur/sharpen: 内核换档 (库高斯/USM → 引擎高斯/USM,
///    apply_in 边像素外扩语义对齐), 如实钉契约不变量 (纯色不变, 逐像素精确)
///    + 偏差上界 (抽样实测: blur 每通道 1/255 → 上界 4, sharpen 每通道
///    0/255 → 上界 2)。
#[tokio::test]
async fn core_swap_blur_and_sharpen_keep_the_contract_within_the_documented_kernel_delta() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mixed = image_with(16, 16, |x, y| {
        Rgba([(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8, 255])
    });
    write_png(&mixed, &root.join("mixed.png"));
    write_png(
        &image_with(6, 6, |_, _| Rgba([90, 140, 190, 255])),
        &root.join("flat.png"),
    );
    let tool = tool(root);

    // blur sigma 1.2: 与换核前参照的偏差上界。
    let result = invoke(
        &tool,
        args(
            "mixed.png",
            "blur.png",
            json!([{ "op": "blur", "sigma": 1.2 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let produced = read_rgba(&root.join("blur.png"));
    let worst = max_deviation(&produced, &pre_swap_blur(&mixed, 1.2));
    assert!(
        worst <= 4,
        "换核前后 blur 每通道偏差 {worst} 超出登记上界 4 (抽样实测 1)"
    );

    // sharpen sigma 1.0 / threshold 4: 与换核前参照的偏差上界。
    let result = invoke(
        &tool,
        args(
            "mixed.png",
            "sharp.png",
            json!([{ "op": "sharpen", "sigma": 1.0, "threshold": 4 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let produced = read_rgba(&root.join("sharp.png"));
    let worst = max_deviation(&produced, &pre_swap_sharpen(&mixed, 1.0, 4));
    assert!(
        worst <= 2,
        "换核前后 sharpen 每通道偏差 {worst} 超出登记上界 2 (抽样实测 0)"
    );

    // 契约不变量: 纯色图模糊/锐化逐像素不变 (精确)。
    for (output, operations) in [
        ("flat-blur.png", json!([{ "op": "blur", "sigma": 1.0 }])),
        (
            "flat-sharp.png",
            json!([{ "op": "sharpen", "sigma": 1.0, "threshold": 0 }]),
        ),
    ] {
        let result = invoke(&tool, args("flat.png", output, operations)).await;
        assert!(result.is_ok(), "{output}: {}", result.render());
        let produced = read_rgba(&root.join(output));
        for y in 0..6 {
            for x in 0..6 {
                assert_eq!(
                    image::GenericImageView::get_pixel(&produced, x, y).0,
                    [90, 140, 190, 255],
                    "纯色 {output} 不得漂移 at {x},{y}"
                );
            }
        }
    }
}

/// ⑤ 换核前后一致性·编解码: 无损三档 (png/webp/bmp) 往返逐像素全等;
///    jpeg 有损不冒充无损 —— 换核前后**同款编码**(白底合成 + q90)的偏差
///    上界如实钉 (均匀内容 ±20 容差另由金标准⑥钉住)。
#[tokio::test]
async fn core_swap_lossless_encodes_roundtrip_pixel_exact_and_jpeg_stays_in_tolerance() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let source = image_with(8, 8, feature_pixel);
    write_png(&source, &root.join("in.png"));
    let tool = tool(root);

    for (format, extension) in [
        ("png", "png"),
        ("webp", "webp"),
        ("bmp", "bmp"),
        ("jpeg", "jpg"),
    ] {
        let output = format!("out.{extension}");
        let result = invoke(
            &tool,
            args(
                "in.png",
                &output,
                json!([{ "op": "format_convert", "format": format }]),
            ),
        )
        .await;
        assert!(result.is_ok(), "{format}: {}", result.render());
        let produced = read_rgba(&root.join(&output));
        if format == "jpeg" {
            // 本例高频合成图是最坏情况, 上界取实测值 + 余量, 不粉饰。
            let worst = max_deviation(&produced, &pre_swap_jpeg_roundtrip(&source));
            assert!(
                worst <= 12,
                "换核前后 jpeg 编码每通道偏差 {worst} 超出登记上界 12 (抽样实测 6)"
            );
        } else {
            assert_eq!(produced, source, "{format} 无损往返逐像素全等");
        }
    }
}

/// ⑥ layers_composite: 核面合成数学逐像素钉值 (中点 / 全不透明 / 全透明 /
///    半透明源叠加), 框外像素一个不动。
#[tokio::test]
async fn layers_composite_blends_the_overlay_with_the_core_formula_and_leaves_the_rest_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(8, 8, |_, _| Rgba([100, 100, 100, 255])),
        &root.join("base.png"),
    );
    write_png(
        &image_with(4, 4, |_, _| Rgba([202, 202, 202, 255])),
        &root.join("over.png"),
    );
    // 半透明叠加图 (a = 128) 与全透明叠加图 (a = 0)。
    write_png(
        &image_with(2, 2, |_, _| Rgba([202, 202, 202, 128])),
        &root.join("over-half.png"),
    );
    write_png(
        &image_with(2, 2, |_, _| Rgba([202, 202, 202, 0])),
        &root.join("over-clear.png"),
    );
    let tool = tool(root);

    // opacity 0.5: 中点合成 (100+202)/2 = 151, alpha 合成 = 1.0 → 255。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "mid.png",
            json!([{
                "op": "layers_composite",
                "overlay": "over.png",
                "x": 2,
                "y": 3,
                "opacity": 0.5
            }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let mid = read_rgba(&root.join("mid.png"));
    assert_eq!(
        image::GenericImageView::get_pixel(&mid, 3, 4).0,
        [151, 151, 151, 255],
        "中点合成逐像素钉值"
    );
    assert_eq!(
        image::GenericImageView::get_pixel(&mid, 1, 3).0,
        [100, 100, 100, 255],
        "图层框外一个像素不动"
    );
    assert_eq!(
        image::GenericImageView::get_pixel(&mid, 7, 7).0,
        [100, 100, 100, 255],
        "远角不动"
    );

    // opacity 1.0 + 全透明叠加像素: 底图不动; 不透明叠加像素: 叠加值直出。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "full.png",
            json!([
                {
                    "op": "layers_composite",
                    "overlay": "over-clear.png",
                    "x": 0,
                    "y": 0,
                    "opacity": 1.0
                },
                {
                    "op": "layers_composite",
                    "overlay": "over.png",
                    "x": 4,
                    "y": 4,
                    "opacity": 1.0
                }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let full = read_rgba(&root.join("full.png"));
    assert_eq!(
        image::GenericImageView::get_pixel(&full, 0, 0).0,
        [100, 100, 100, 255],
        "全透明叠加像素不动底图"
    );
    assert_eq!(
        image::GenericImageView::get_pixel(&full, 5, 5).0,
        [202, 202, 202, 255],
        "不透明叠加像素直出"
    );

    // opacity 1.0 + 半透明源 (a = 128): W3C/PDF 源叠加
    // out = (1 − 128/255)·100/255 + (128/255)·202/255 ≈ 151.2/255 → 151;
    // αo = 128/255 + 1·(1 − 128/255) = 1 → 255。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "half.png",
            json!([{
                "op": "layers_composite",
                "overlay": "over-half.png",
                "x": 1,
                "y": 1,
                "opacity": 1.0
            }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let half = read_rgba(&root.join("half.png"));
    assert_eq!(
        image::GenericImageView::get_pixel(&half, 1, 1).0,
        [151, 151, 151, 255],
        "半透明源叠加逐像素钉值 (W3C/PDF 公式)"
    );
}

/// ⑦ layers_composite 参数校验矩阵 + 受控读面: 缺参/多参/越界值/越界图层/
///    白名单外/敏感面/缺失文件一律拒绝即帧; 管道中途拒绝 = 目标原状。
#[tokio::test]
async fn layers_composite_rejects_malformed_parameters_and_uncontrolled_overlays() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(8, 8, |_, _| Rgba([10, 20, 30, 255])),
        &root.join("base.png"),
    );
    write_png(
        &image_with(4, 4, |_, _| Rgba([200, 200, 200, 255])),
        &root.join("over.png"),
    );
    std::fs::write(root.join("bad.gif"), b"GIF89a").unwrap();
    std::fs::write(root.join(".env"), b"SECRET=1").unwrap();
    let tool = tool(root);

    let composite = |overlay: Value, x: Value, y: Value, opacity: Value| {
        json!([{
            "op": "layers_composite",
            "overlay": overlay,
            "x": x,
            "y": y,
            "opacity": opacity
        }])
    };

    // 缺参 (无 opacity)。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            json!([{ "op": "layers_composite", "overlay": "over.png", "x": 0, "y": 0 }]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "opacity");

    // 多余参。
    let mut extra = composite(json!("over.png"), json!(0), json!(0), json!(1.0));
    extra[0]["threshold"] = json!(3);
    let result = invoke(&tool, args("base.png", "out.png", extra)).await;
    assert_frame(&result, "image_ops.param_rejected", "threshold");

    // 越界值 (opacity 1.5 / -0.1 / 空 overlay)。
    for opacity in [json!(1.5), json!(-0.1)] {
        let result = invoke(
            &tool,
            args(
                "base.png",
                "out.png",
                composite(json!("over.png"), json!(0), json!(0), opacity),
            ),
        )
        .await;
        assert_frame(&result, "image_ops.param_rejected", "opacity");
    }
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            composite(json!(""), json!(0), json!(0), json!(1.0)),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "overlay");

    // 图层越界 (4x4 @ 7,7 于 8x8)。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            composite(json!("over.png"), json!(7), json!(7), json!(1.0)),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "layers_composite");
    assert!(!root.join("out.png").exists(), "拒绝即帧不得落盘");

    // 白名单外叠加图。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            composite(json!("bad.gif"), json!(0), json!(0), json!(1.0)),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.format_rejected", "whitelist");

    // 敏感面叠加图 (凭据帧)。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            composite(json!(".env"), json!(0), json!(0), json!(1.0)),
        ),
    )
    .await;
    assert_frame(&result, "pipeline.pre_deny", "凭据面不可读");

    // 缺失叠加图。
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            composite(json!("none.png"), json!(0), json!(0), json!(1.0)),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.read_failed", "not found");

    // 管道中途拒绝 = 目标原状 (灰度已应用但合成越界, 全无产出)。
    std::fs::write(root.join("out.png"), b"ORIGINAL").unwrap();
    let result = invoke(
        &tool,
        args(
            "base.png",
            "out.png",
            json!([
                { "op": "grayscale" },
                {
                    "op": "layers_composite",
                    "overlay": "over.png",
                    "x": 7,
                    "y": 7,
                    "opacity": 1.0
                }
            ]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "layers_composite");
    assert_eq!(
        std::fs::read(root.join("out.png")).unwrap(),
        b"ORIGINAL",
        "中途失败目标必须原状"
    );
}

/// 均匀像素缓冲 (count 个 rgba, 行主序交错)。
fn flat_pixels(rgba: [u8; 4], count: usize) -> Vec<u8> {
    std::iter::repeat(rgba).take(count).flatten().collect()
}

/// 构造一个真 PSD (核面文档构建器): 两个命名图层 + 合并像素。
fn sample_psd() -> Vec<u8> {
    let mut builder = PsdBuilder::new(6, 4)
        .color_mode(ColorMode::Rgb)
        .depth(8)
        .version(Version::Psd);
    builder.push_layer(LayerSpec::new(
        "alpha",
        0,
        0,
        6,
        4,
        PixelData::Rgba8(flat_pixels([255, 0, 0, 255], 24)),
    ));
    builder.push_layer(LayerSpec::new(
        "beta",
        1,
        1,
        3,
        2,
        PixelData::Rgba8(flat_pixels([0, 0, 255, 255], 6)),
    ));
    builder.composite(PixelData::Rgba8(flat_pixels([128, 128, 128, 255], 24)));
    builder.to_bytes().expect("sample psd")
}

/// ⑧ psd_read_info: 真文档头与图层清单逐项钉值 (相对路径出帧, 只读)。
#[tokio::test]
async fn psd_read_info_reports_header_and_layers_for_a_real_document() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 2, 3, 255])),
        &root.join("in.png"),
    );
    std::fs::write(root.join("doc.psd"), sample_psd()).unwrap();
    let tool = tool(root);

    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([
                { "op": "psd_read_info", "path": "doc.psd" },
                { "op": "grayscale" }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["operations"], json!(["psd_read_info", "grayscale"]));
    let infos = report["psd_infos"].as_array().expect("probe section");
    assert_eq!(infos.len(), 1);
    let info = &infos[0];
    assert_eq!(info["path"], "doc.psd", "探针路径只回相对拼写");
    assert_eq!(info["width"], 6);
    assert_eq!(info["height"], 4);
    assert_eq!(info["depth"], 8);
    assert_eq!(info["color_mode"], "Rgb");
    assert_eq!(info["version"], "psd");
    assert_eq!(info["layer_count"], 2);
    assert_eq!(info["layer_names"], json!(["alpha", "beta"]));
    assert_eq!(info["layer_names_capped"], false);
    // 像素面不受探针影响: 只有 grayscale 生效。
    let out = read_rgba(&root.join("out.png"));
    assert_eq!(
        image::GenericImageView::get_pixel(&out, 0, 0).0,
        [2, 2, 2, 255],
        "探针零像素效果"
    );

    // 脱敏口径: 帧与探针都不得回显工作区根绝对拼写。
    let rendered = result.render();
    let root_raw = root.to_string_lossy().to_string();
    assert!(
        !rendered.contains(&root_raw),
        "绝对根拼写不得出帧: {rendered}"
    );
}

/// ⑨ psd_read_info 参数校验与坏文档: 缺参/多余参/空路径/坏字节/敏感面/
///    缺失文件一律拒绝即帧。
#[tokio::test]
async fn psd_read_info_rejects_malformed_parameters_and_documents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 2, 3, 255])),
        &root.join("in.png"),
    );
    std::fs::write(root.join("bad.psd"), b"NOT A PSD").unwrap();
    std::fs::write(root.join(".env"), b"SECRET=1").unwrap();
    let tool = tool(root);

    // 缺参。
    let result = invoke(
        &tool,
        args("in.png", "out.png", json!([{ "op": "psd_read_info" }])),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "path");

    // 多余参。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{ "op": "psd_read_info", "path": "doc.psd", "x": 0 }]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "x");

    // 空路径。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{ "op": "psd_read_info", "path": "  " }]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "path");

    // 坏文档字节。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{ "op": "psd_read_info", "path": "bad.psd" }]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.decode_failed", "psd parse failed");

    // 敏感面 (凭据帧)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{ "op": "psd_read_info", "path": ".env" }]),
        ),
    )
    .await;
    assert_frame(&result, "pipeline.pre_deny", "凭据面不可读");

    // 缺失文件。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{ "op": "psd_read_info", "path": "none.psd" }]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.read_failed", "not found");
    assert!(!root.join("out.png").exists(), "拒绝即帧不得落盘");
}

/// ⑩ psd_read_info 探针可复用且按管道顺序汇报; 图层名截断不静默 (上限常量)。
#[tokio::test]
async fn psd_read_info_is_non_destructive_and_reports_in_pipeline_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([8, 8, 8, 255])),
        &root.join("in.png"),
    );
    std::fs::write(root.join("a.psd"), sample_psd()).unwrap();
    // 70 图层文档 (超上限 64): 截断须显式标记。
    let mut builder = PsdBuilder::new(2, 2)
        .color_mode(ColorMode::Rgb)
        .depth(8)
        .version(Version::Psb);
    for index in 0..70u32 {
        builder.push_layer(LayerSpec::new(
            format!("layer-{index}"),
            0,
            0,
            2,
            2,
            PixelData::Rgba8(flat_pixels([0, 0, 0, 255], 4)),
        ));
    }
    builder.composite(PixelData::Rgba8(flat_pixels([0, 0, 0, 255], 4)));
    std::fs::write(root.join("many.psd"), builder.to_bytes().unwrap()).unwrap();
    let tool = tool(root);

    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([
                { "op": "psd_read_info", "path": "a.psd" },
                { "op": "brightness", "amount": 2 },
                { "op": "psd_read_info", "path": "many.psd" }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(
        report["operations"],
        json!(["psd_read_info", "brightness", "psd_read_info"])
    );
    let infos = report["psd_infos"].as_array().expect("probe section");
    assert_eq!(infos.len(), 2, "探针按管道顺序逐条汇报");
    assert_eq!(infos[0]["path"], "a.psd");
    assert_eq!(infos[1]["path"], "many.psd");
    assert_eq!(infos[1]["version"], "psb");
    assert_eq!(infos[1]["layer_count"], 70);
    assert_eq!(
        infos[1]["layer_names"].as_array().unwrap().len(),
        MAX_PSD_LAYER_NAMES
    );
    assert_eq!(infos[1]["layer_names_capped"], true, "截断不静默");
}

/// ⑪ 新词表成员同款闭合规则: 声明面枚举 = 词表全集; 新操作缺参即拒、表外
///    参数集即拒、serde 面未知字段即拒。
#[tokio::test]
async fn new_vocabulary_members_are_declared_and_parse_under_the_same_closed_rules() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 1, 1, 255])),
        &root.join("in.png"),
    );
    let tool = tool(root);

    // 声明面: 操作枚举 = 闭合词表全集 (13 项, 逐一对齐)。
    let declaration = tool.declaration();
    let rendered = serde_json::to_value(&declaration).expect("declaration json");
    let declared: Vec<String> = rendered["parameters"]["properties"]["operations"]["items"]
        ["properties"]["op"]["enum"]
        .as_array()
        .expect("op enum")
        .iter()
        .map(|entry| entry.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        declared.len(),
        OPERATION_VOCABULARY.len(),
        "声明枚举与闭合词表同长"
    );
    for name in OPERATION_VOCABULARY {
        assert!(declared.contains(&name.to_string()), "声明缺 {name}");
    }

    // 解析面: 新操作缺参即拒 (与既有操作同款)。
    for name in ["layers_composite", "psd_read_info"] {
        let error = ImageOperation::parse(&OperationSpec {
            op: name.to_string(),
            ..OperationSpec::default()
        })
        .unwrap_err();
        assert!(
            error.frame().starts_with("image_ops.param_rejected"),
            "{name} 缺参必须拒绝即帧: {}",
            error.frame()
        );
    }

    // serde 面: 未知字段即拒 (无静默忽略)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([{
                "op": "layers_composite",
                "overlay": "in.png",
                "x": 0,
                "y": 0,
                "opacity": 1.0,
                "bogus": 1
            }]),
        ),
    )
    .await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(
        result.render().contains("invalid image_ops parameters"),
        "未知字段必须走 serde 拒绝帧: {}",
        result.render()
    );
}
