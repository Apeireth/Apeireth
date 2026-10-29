//! 创作工具面 W1 (`tool.image_ops`) 行为钉子: 逐操作像素断言 / 参数校验矩阵 /
//! 敏感面拒 / 炸弹图上限 / 格式白名单 / 管道顺序 / 原子写中途失败原状 /
//! 观测门禁 (未读不得覆盖写) / 脱敏口径 (路径不出帧) / 开关 fail-closed。
//!
//! 断言全部走**像素级**真值 (fixture 像素逐个选定, 输出逐像素核对), 不钉字面。

/// 本集成测试二进制内的 env 锁（进程级全局量 ⇒ 串行）。
/// 放在**模块级**（不在测试函数内）：同文件多个测试函数共用同一把。
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use std::path::Path;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_tools_canonical::{
    ImageOpsActivity, ImageOpsTool, ObservedGate, IMAGE_OPS_ENABLE_ENV, MAX_PIPELINE_STEPS,
    OPERATION_VOCABULARY,
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};

fn tool(root: &Path) -> ImageOpsTool {
    ImageOpsTool::new(
        root.to_path_buf(),
        Arc::new(ObservedGate::new()),
        Arc::new(ImageOpsActivity::new()),
    )
}

fn call_id() -> String {
    format!("call_{}", std::process::id())
}

async fn invoke(tool: &ImageOpsTool, arguments: Value) -> ToolResult {
    let call = ToolCall {
        id: call_id(),
        name: "image_ops".into(),
        arguments,
    };
    tool.invoke(&call).await
}

fn args(input: &str, output: &str, operations: Value) -> Value {
    json!({ "input": input, "output": output, "operations": operations })
}

/// 像素按坐标函数生成 (逐像素可预测)。
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

fn pixel(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    img.get_pixel(x, y).0
}

/// 特征像素 (逐坐标唯一, 位置一挪就能看出来)。
fn feature_pixel(x: u32, y: u32) -> Rgba<u8> {
    Rgba([(x * 40 + 10) as u8, (y * 40 + 20) as u8, 60, 255])
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

/// ① resize: 尺寸精确 + 纯色图逐像素保色 (三角核对均匀色不漂移)。
#[tokio::test]
async fn resize_changes_dims_and_keeps_uniform_color() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([200, 100, 50, 255])),
        &root.join("in.png"),
    );
    let result = invoke(
        &tool(root),
        args(
            "in.png",
            "out.png",
            json!([{ "op": "resize", "width": 2, "height": 2 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["output_size"]["width"], 2);
    assert_eq!(report["output_size"]["height"], 2);
    assert_eq!(report["operations"], json!(["resize"]));
    assert_eq!(report["overwrote"], false);
    let out = read_rgba(&root.join("out.png"));
    assert_eq!(out.dimensions(), (2, 2));
    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(
                pixel(&out, x, y),
                [200, 100, 50, 255],
                "uniform color at {x},{y}"
            );
        }
    }
}

/// ② crop / rotate / flip: 逐像素位置钉死 (位置一挪就红)。
#[tokio::test]
async fn crop_rotate_flip_pin_pixel_positions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(&image_with(3, 2, feature_pixel), &root.join("in.png"));
    let tool = tool(root);

    // crop (1,0,2,2): 取右下四像素, 位置逐个核对。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "crop.png",
            json!([{ "op": "crop", "x": 1, "y": 0, "width": 2, "height": 2 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let out = read_rgba(&root.join("crop.png"));
    assert_eq!(out.dimensions(), (2, 2));
    assert_eq!(pixel(&out, 0, 0), feature_pixel(1, 0).0);
    assert_eq!(pixel(&out, 1, 0), feature_pixel(2, 0).0);
    assert_eq!(pixel(&out, 0, 1), feature_pixel(1, 1).0);
    assert_eq!(pixel(&out, 1, 1), feature_pixel(2, 1).0);

    // rotate 90 (顺时针): 3x2 -> 2x3, dst(x,y) = src(y, srcH-1-x)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "rot.png",
            json!([{ "op": "rotate", "angle_deg": 90 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let out = read_rgba(&root.join("rot.png"));
    assert_eq!(out.dimensions(), (2, 3));
    for y in 0..3 {
        for x in 0..2 {
            assert_eq!(
                pixel(&out, x, y),
                feature_pixel(y, 2 - 1 - x).0,
                "rotate90 mapping at {x},{y}"
            );
        }
    }

    // flip horizontal: 镜像逐像素。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "flip.png",
            json!([{ "op": "flip", "axis": "horizontal" }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let out = read_rgba(&root.join("flip.png"));
    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(
                pixel(&out, x, y),
                feature_pixel(3 - 1 - x, y).0,
                "flip mapping at {x},{y}"
            );
        }
    }
}

/// ③ brightness / contrast / grayscale: 通道算术逐值钉死 (饱和与舍入如实)。
#[tokio::test]
async fn brightness_contrast_grayscale_pin_channel_math() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(2, 2, |_, _| Rgba([100, 200, 50, 255])),
        &root.join("in.png"),
    );
    let tool = tool(root);

    // brightness +50: 逐通道加, alpha 不动; 饱和另测 (200+50 未到顶)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "bright.png",
            json!([{ "op": "brightness", "amount": 50 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("bright.png")), 0, 0),
        [150, 250, 100, 255]
    );

    // 饱和: amount=100 时 200+100 封顶 255。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "sat.png",
            json!([{ "op": "brightness", "amount": 100 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("sat.png")), 0, 0),
        [200, 255, 150, 255]
    );

    // contrast 2.0: (v-128)*2+128, 四舍五入 + 饱和 (200 -> 372 封顶 255)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "contrast.png",
            json!([{ "op": "contrast", "factor": 2.0 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("contrast.png")), 0, 0),
        [72, 255, 0, 255]
    );

    // grayscale: (77r + 150g + 29b + 128) >> 8 = (7700+30000+1450+128)>>8 = 153。
    let result = invoke(
        &tool,
        args("in.png", "gray.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("gray.png")), 0, 0),
        [153, 153, 153, 255]
    );
}

/// ④ blur / sharpen: 邻域真的被带到 (不是恒等), 远角不动。
#[tokio::test]
async fn blur_and_sharpen_touch_neighbors() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // 亮斑图: 5x5 黑底中心白点。
    let dot = image_with(5, 5, |x, y| {
        if (x, y) == (2, 2) {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    });
    write_png(&dot, &root.join("dot.png"));
    let tool = tool(root);

    let result = invoke(
        &tool,
        args(
            "dot.png",
            "blur.png",
            json!([{ "op": "blur", "sigma": 1.0 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let blurred = read_rgba(&root.join("blur.png"));
    let center = u16::from(pixel(&blurred, 2, 2)[0]);
    let neighbor = u16::from(pixel(&blurred, 2, 1)[0]);
    assert!(center < 255, "blur 必须摊开中心白点: center={center}");
    assert!(neighbor > 0, "blur 必须把能量带到邻域: neighbor={neighbor}");

    // 阶跃图: 左半 100 / 右半 150 的竖直棱。
    let step = image_with(8, 8, |x, _| {
        let value = if x < 4 { 100 } else { 150 };
        Rgba([value, value, value, 255])
    });
    write_png(&step, &root.join("step.png"));
    let result = invoke(
        &tool,
        args(
            "step.png",
            "sharp.png",
            json!([{ "op": "sharpen", "sigma": 1.0, "threshold": 0 }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let sharp = read_rgba(&root.join("sharp.png"));
    let bright_side = u16::from(pixel(&sharp, 4, 4)[0]);
    let dark_side = u16::from(pixel(&sharp, 3, 4)[0]);
    let far_corner = u16::from(pixel(&sharp, 0, 0)[0]);
    assert!(bright_side > 150, "棱亮侧必须被抬起来: {bright_side}");
    assert!(dark_side < 100, "棱暗侧必须被压下去: {dark_side}");
    assert_eq!(far_corner, 100, "远角均匀区不动");
}

/// ⑤ watermark_text: 字框内落笔 (白), 框外一个像素不动。
#[tokio::test]
async fn watermark_text_draws_only_inside_the_glyph_box() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(20, 20, |_, _| Rgba([0, 0, 0, 255])),
        &root.join("in.png"),
    );
    let result = invoke(
        &tool(root),
        args(
            "in.png",
            "wm.png",
            json!([{
                "op": "watermark_text",
                "text": "A",
                "x": 2,
                "y": 2,
                "scale": 2
            }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let out = read_rgba(&root.join("wm.png"));
    // 'A' 字模首行 ".###.": 缩放 2 后 (4,2) 起是落笔方块。
    assert_eq!(pixel(&out, 4, 2), [255, 255, 255, 255], "字框内必须落笔");
    assert_eq!(pixel(&out, 2, 2), [0, 0, 0, 255], "字模空位不动");
    assert_eq!(pixel(&out, 15, 15), [0, 0, 0, 255], "字框外不动");
    assert_eq!(pixel(&out, 0, 0), [0, 0, 0, 255], "远角不动");
}

/// ⑥ format_convert: 白名单四格式逐一真编码真回读 (jpeg 有损容差如实)。
#[tokio::test]
async fn format_convert_roundtrips_the_whitelisted_formats() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(8, 8, |_, _| Rgba([200, 100, 50, 255])),
        &root.join("in.png"),
    );
    for (format, extension) in [
        ("png", "png"),
        ("jpeg", "jpg"),
        ("webp", "webp"),
        ("bmp", "bmp"),
    ] {
        let output = format!("out.{extension}");
        let result = invoke(
            &tool(root),
            args(
                "in.png",
                &output,
                json!([{ "op": "format_convert", "format": format }]),
            ),
        )
        .await;
        assert!(result.is_ok(), "{format}: {}", result.render());
        let report: Value = serde_json::from_str(&result.render()).unwrap();
        assert_eq!(
            report["output_format"], format,
            "ext=.jpg 时输出格式仍报 jpeg"
        );
        let decoded = image::open(root.join(&output))
            .unwrap_or_else(|error| panic!("{format} output must decode: {error}"));
        assert_eq!((decoded.width(), decoded.height()), (8, 8), "{format} dims");
        let rgba = decoded.to_rgba8();
        let [r, g, b, a] = pixel(&rgba, 4, 4);
        if format == "jpeg" {
            // jpeg 有损: 容差 ±20 如实, 不装无损。
            assert!((i16::from(r) - 200).abs() <= 20, "jpeg r={r}");
            assert!((i16::from(g) - 100).abs() <= 20, "jpeg g={g}");
            assert!((i16::from(b) - 50).abs() <= 20, "jpeg b={b}");
        } else {
            assert_eq!([r, g, b, a], [200, 100, 50, 255], "{format} 无损逐像素");
        }
    }
}

/// ⑦ 参数校验矩阵: 词表外名 / 缺参 / 多余参 / 越界值 / 空管道 / 超长管道 /
/// 坏形状, 一律拒绝即帧, 且不落盘。
#[tokio::test]
async fn parameter_validation_matrix_rejects_every_malformed_operation() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([10, 20, 30, 255])),
        &root.join("in.png"),
    );
    let tool = tool(root);
    let cases: Vec<(Value, &str)> = vec![
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "rescale", "width": 2, "height": 2 }]),
            ),
            "闭合词表",
        ),
        (
            args("in.png", "out.png", json!([{ "op": "resize", "width": 2 }])),
            "height",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "grayscale", "amount": 10 }]),
            ),
            "amount",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "rotate", "angle_deg": 45 }]),
            ),
            "angle_deg",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "flip", "axis": "diagonal" }]),
            ),
            "axis",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "brightness", "amount": 256 }]),
            ),
            "amount",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "contrast", "factor": 3.5 }]),
            ),
            "factor",
        ),
        (
            args("in.png", "out.png", json!([{ "op": "blur", "sigma": 0.0 }])),
            "sigma",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "sharpen", "sigma": 1.0, "threshold": 300 }]),
            ),
            "threshold",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "crop", "x": -1, "y": 0, "width": 2, "height": 2 }]),
            ),
            "image_ops.param_rejected",
        ),
        (
            args(
                "in.png",
                "out.png",
                json!([{ "op": "watermark_text", "text": "hi", "x": 0, "y": 0, "scale": 9 }]),
            ),
            "scale",
        ),
        (args("in.png", "out.png", json!([])), "空管道"),
        (
            args(
                "in.png",
                "out.png",
                json!(vec![json!({ "op": "grayscale" }); MAX_PIPELINE_STEPS + 1]),
            ),
            "step cap",
        ),
    ];
    for (arguments, needle) in cases {
        let result = invoke(&tool, arguments).await;
        assert_frame(&result, "image_ops.param_rejected", needle);
        assert!(
            !root.join("out.png").exists(),
            "拒绝即帧的调用不得落盘 ({needle})"
        );
    }

    // 坏形状 (缺 input 字段): serde 面也必须是帧。
    let result = invoke(&tool, json!({ "output": "out.png", "operations": [] })).await;
    assert!(!result.is_ok(), "{}", result.render());
    assert!(result.render().contains("invalid image_ops parameters"));
}

/// ⑧ 敏感面拒: 凭据/密钥面读取拒绝走读工具同一帧 (pre_deny 语义)。
#[tokio::test]
async fn sensitive_surface_reads_are_denied_with_the_credential_frame() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("creds.json"), b"{\"token\":\"x\"}").unwrap();
    std::fs::write(root.join("server.pem"), b"-----BEGIN-----").unwrap();
    std::fs::write(root.join(".env"), b"SECRET=1").unwrap();
    let tool = tool(root);
    for input in ["creds.json", "server.pem", ".env"] {
        let result = invoke(
            &tool,
            args(input, "out.png", json!([{ "op": "grayscale" }])),
        )
        .await;
        assert!(!result.is_ok(), "{input} 必须拒");
        let rendered = result.render();
        assert!(
            rendered.contains("pipeline.pre_deny"),
            "凭据面帧 = 读工具同一帧: {rendered}"
        );
        assert!(
            rendered.contains("凭据面不可读"),
            "拒绝即帧要说真因: {rendered}"
        );
        assert!(!root.join("out.png").exists(), "拒绝后不得落盘");
    }
}

/// ⑨ 炸弹图三层上限: 字节上限 (按实读判定) + 解码期维度上限 + 像素上限 ——
/// 三层各自拒绝即帧, 不解码出整幅、不落盘。
#[tokio::test]
async fn bomb_inputs_are_rejected_by_byte_and_dimension_caps() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // (a) 字节上限: 32x32 纯色 png 一定 > 64 字节。
    write_png(
        &image_with(32, 32, |_, _| Rgba([9, 9, 9, 255])),
        &root.join("big.png"),
    );
    let small = ImageOpsTool::new(
        root.to_path_buf(),
        Arc::new(ObservedGate::new()),
        Arc::new(ImageOpsActivity::new()),
    )
    .with_max_input_bytes(64);
    let result = invoke(
        &small,
        args("big.png", "out.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert_frame(&result, "image_ops.limit_exceeded", "byte limit");

    // (b) 解码期维度上限: 头部宣称 20000x20000 的 png (炸弹图: 小文件大图幅)。
    std::fs::write(root.join("bomb.png"), bomb_png(20_000, 20_000)).unwrap();
    let result = invoke(
        &tool(root),
        args("bomb.png", "out.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert_frame(&result, "image_ops.limit_exceeded", "上限");
    assert!(!root.join("out.png").exists(), "炸弹图不得产出");
}

/// ⑩ 格式白名单: 输入内容白名单外拒; 输出扩展名映射到白名单外同样拒。
#[tokio::test]
async fn non_whitelisted_formats_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // gif 头 (内容嗅探即拒, 无需完整字节流)。
    let mut gif = b"GIF89a".to_vec();
    gif.extend_from_slice(&[0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    std::fs::write(root.join("in.gif"), &gif).unwrap();
    let tool = tool(root);
    let result = invoke(
        &tool,
        args("in.gif", "out.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert_frame(&result, "image_ops.format_rejected", "whitelist");

    // 合法 png 内容 + 白名单外扩展名输出: 同样拒 (不写驴唇马嘴扩展名)。
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 2, 3, 255])),
        &root.join("in.png"),
    );
    let result = invoke(
        &tool,
        args("in.png", "out.gif", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert_frame(&result, "image_ops.format_rejected", "whitelist");
    assert!(!root.join("out.gif").exists(), "拒绝后不得落盘");
}

/// ⑪ 管道顺序: 同一操作集按给定顺序应用, 顺序不同结果不同 (逐像素钉值)。
#[tokio::test]
async fn pipeline_applies_operations_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(2, 2, |_, _| Rgba([100, 200, 50, 255])),
        &root.join("in.png"),
    );
    let tool = tool(root);

    // 亮度→对比度: (150,250,100) -> ((150-128)*2+128, 封顶, (100-128)*2+128)。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "a.png",
            json!([
                { "op": "brightness", "amount": 50 },
                { "op": "contrast", "factor": 2.0 }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("a.png")), 0, 0),
        [172, 255, 72, 255]
    );

    // 对比度→亮度: (72,255,0) -> (122,255,50)。顺序不同, 结果逐像素不同。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "b.png",
            json!([
                { "op": "contrast", "factor": 2.0 },
                { "op": "brightness", "amount": 50 }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    assert_eq!(
        pixel(&read_rgba(&root.join("b.png")), 0, 0),
        [122, 255, 50, 255]
    );

    // 几何顺序: 先裁后放 = 4x4, 先放后裁 = 2x2。
    let result = invoke(
        &tool,
        args(
            "in.png",
            "c.png",
            json!([
                { "op": "crop", "x": 0, "y": 0, "width": 1, "height": 1 },
                { "op": "resize", "width": 4, "height": 4 }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["output_size"]["width"], 4);

    let result = invoke(
        &tool,
        args(
            "in.png",
            "d.png",
            json!([
                { "op": "resize", "width": 4, "height": 4 },
                { "op": "crop", "x": 0, "y": 0, "width": 2, "height": 2 }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["output_size"]["width"], 2);
    assert_eq!(
        report["operations"],
        json!(["resize", "crop"]),
        "汇报按应用顺序"
    );
}

/// ⑫ 原子写: 管道中途失败 = 目标原状 + 无临时文件残留 (全无产出, 不留半成品)。
#[tokio::test]
async fn mid_pipeline_failure_leaves_the_original_and_no_residue() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 1, 1, 255])),
        &root.join("in.png"),
    );
    std::fs::write(root.join("out.png"), b"ORIGINAL").unwrap();

    let result = invoke(
        &tool(root),
        args(
            "in.png",
            "out.png",
            json!([
                { "op": "grayscale" },
                { "op": "crop", "x": 3, "y": 3, "width": 5, "height": 5 }
            ]),
        ),
    )
    .await;
    assert_frame(&result, "image_ops.param_rejected", "crop");
    assert_eq!(
        std::fs::read(root.join("out.png")).unwrap(),
        b"ORIGINAL",
        "中途失败目标必须原状"
    );
    let residue: Vec<String> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| name.contains(".tmp-"))
        .collect();
    assert!(residue.is_empty(), "原子写不得留临时文件残留: {residue:?}");
}

/// ⑬ 观测门禁 (与读工具同一张观测表): 未读不得覆盖写; 同调用读写 (原地编辑)
/// 与写后回填的覆盖写放行。
#[tokio::test]
async fn overwrite_needs_prior_observation_and_in_place_edit_works() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([7, 8, 9, 255])),
        &root.join("in.png"),
    );
    std::fs::write(root.join("out.png"), b"EXTERNAL").unwrap();
    let tool = tool(root);

    // 盲覆盖: 目标存在且从未读过 -> 拒绝即帧 + 原状。
    let result = invoke(
        &tool,
        args("in.png", "out.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert_frame(&result, "image_ops.write_denied", "从未被读取观测");
    assert_eq!(std::fs::read(root.join("out.png")).unwrap(), b"EXTERNAL");

    // 原地编辑: 输入 = 输出 (同调用先读后写, 观测在场) -> 放行。
    let result = invoke(
        &tool,
        args("in.png", "in.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["overwrote"], true);
    assert_eq!(
        pixel(&read_rgba(&root.join("in.png")), 0, 0),
        [8, 8, 8, 255]
    );

    // 写后回填观测: 工具写过的目标再覆盖写放行 (同会话连写无需重读)。
    let result = invoke(
        &tool,
        args("in.png", "out2.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let result = invoke(
        &tool,
        args(
            "in.png",
            "out2.png",
            json!([{ "op": "brightness", "amount": 10 }]),
        ),
    )
    .await;
    assert!(
        result.is_ok(),
        "写后回填的覆盖写必须放行: {}",
        result.render()
    );
    let report: Value = serde_json::from_str(&result.render()).unwrap();
    assert_eq!(report["overwrote"], true);
}

/// ⑭ 脱敏口径: 帧与汇报只回工作区相对拼写, 工作区根的绝对拼写不出帧;
/// 越界/敏感类拒绝帧不回显路径。
#[tokio::test]
async fn frames_and_reports_never_echo_absolute_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let outside = tempfile::tempdir().unwrap();
    write_png(
        &image_with(4, 4, |_, _| Rgba([1, 1, 1, 255])),
        &root.join("in.png"),
    );
    write_png(
        &image_with(4, 4, |_, _| Rgba([2, 2, 2, 255])),
        &outside.path().join("out.png"),
    );
    let tool = tool(root);
    let root_raw = root.to_string_lossy().to_string();
    let root_slash = root_raw.replace('\\', "/");

    // 成功帧: 绝对拼写输入也只回相对拼写。
    let absolute_input = root.join("in.png").to_string_lossy().to_string();
    let absolute_output = root.join("abs-out.png").to_string_lossy().to_string();
    let result = invoke(
        &tool,
        args(
            &absolute_input,
            &absolute_output,
            json!([{ "op": "grayscale" }]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(
        !rendered.contains(&root_raw),
        "绝对根拼写不得出帧: {rendered}"
    );
    assert!(
        !rendered.contains(&root_slash),
        "斜杠归一形态同样不出帧: {rendered}"
    );
    assert!(rendered.contains("abs-out.png"), "只回相对拼写: {rendered}");

    // 越界拒绝帧: 不回显任何路径 (不回显工作区外拼写, 也不回显根)。
    let outside_path = outside.path().join("out.png").to_string_lossy().to_string();
    let result = invoke(
        &tool,
        args(&outside_path, "out.png", json!([{ "op": "grayscale" }])),
    )
    .await;
    assert!(!result.is_ok(), "{}", result.render());
    let rendered = result.render();
    assert!(
        rendered.contains("image_ops.outside_workspace"),
        "越界拒绝帧要有稳定 code: {rendered}"
    );
    assert!(
        !rendered.contains(&outside_path),
        "越界路径不得出帧: {rendered}"
    );
    assert!(
        !rendered.contains(&root_raw),
        "工作区根不得出帧: {rendered}"
    );
}

/// ⑮ 创作节计数器: 调用/步数/拒绝逐项真值, 只计数不回路径 (与自述创作节同源)。
#[tokio::test]
async fn activity_counts_track_operations_without_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_png(
        &image_with(4, 4, |_, _| Rgba([3, 3, 3, 255])),
        &root.join("in.png"),
    );
    let activity = Arc::new(ImageOpsActivity::new());
    let tool = ImageOpsTool::new(
        root.to_path_buf(),
        Arc::new(ObservedGate::new()),
        Arc::clone(&activity),
    );

    let result = invoke(
        &tool,
        args(
            "in.png",
            "out.png",
            json!([
                { "op": "grayscale" },
                { "op": "resize", "width": 2, "height": 2 }
            ]),
        ),
    )
    .await;
    assert!(result.is_ok(), "{}", result.render());
    let result = invoke(&tool, json!({ "bad": "shape" })).await;
    assert!(!result.is_ok());

    let status = activity.status();
    assert_eq!(status.calls, 2);
    assert_eq!(status.ops_applied, 2);
    assert_eq!(status.denied, 1);
    assert_eq!(status.last_call_ops.as_deref(), Some("grayscale -> resize"));
    assert!(status.last_op_at_ms.is_some(), "最近操作时刻如实");
    assert_eq!(
        status.by_operation.len(),
        OPERATION_VOCABULARY.len(),
        "闭合词表逐操作计数不省行"
    );
    let grayscale = status
        .by_operation
        .iter()
        .find(|row| row.operation == "grayscale")
        .unwrap();
    assert_eq!(grayscale.count, 1);
    let note = status.note.clone().unwrap_or_default();
    assert!(note.contains("进程内计数"), "计数口径如实注明: {note}");

    // 脱敏口径: 创作节序列化零路径 (只回操作名)。
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(
        !serialized.contains(&root.to_string_lossy().to_string()),
        "{serialized}"
    );
    assert!(!serialized.contains(".png"), "创作节零文件名: {serialized}");
}

/// ⑯ 开关 fail-closed: `APEIRETH_ENABLE_IMAGE_OPS` 只认显式 "1" (语义真源同款),
/// 其余取值一律关 (默认关)。
#[tokio::test]
async fn image_ops_switch_fails_closed_on_anything_but_one() {
    let _guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::env::remove_var(IMAGE_OPS_ENABLE_ENV);
    assert!(
        !apeireth_tools_canonical::image_ops_enabled_from_env(),
        "未设 = 关"
    );
    for value in ["0", "true", "yes", " 2 ", "11"] {
        std::env::set_var(IMAGE_OPS_ENABLE_ENV, value);
        assert!(
            !apeireth_tools_canonical::image_ops_enabled_from_env(),
            "must refuse {value:?}"
        );
    }
    std::env::set_var(IMAGE_OPS_ENABLE_ENV, "1");
    assert!(apeireth_tools_canonical::image_ops_enabled_from_env());
    std::env::set_var(IMAGE_OPS_ENABLE_ENV, " 1 ");
    assert!(apeireth_tools_canonical::image_ops_enabled_from_env());
    std::env::remove_var(IMAGE_OPS_ENABLE_ENV);
}

/// 生成「头大身小」的炸弹图 png: 完整签名 + IHDR (宣称 width×height) + 最小
/// IDAT + IEND, 逐块 CRC32 合法 —— 解码器读到 IHDR 即触上限拒绝。
fn bomb_png(width: u32, height: u32) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = u32::MAX;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut for_crc = Vec::new();
        for_crc.extend_from_slice(kind);
        for_crc.extend_from_slice(data);
        out.extend_from_slice(&crc32(&for_crc).to_be_bytes());
        out
    }
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit RGBA, deflate, adaptive, no interlace
    png.extend(chunk(b"IHDR", &ihdr));
    // 最小合法 zlib 流 (空 deflate 块 + adler32), 让 info 头读取完整通过。
    png.extend(chunk(
        b"IDAT",
        &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
    ));
    png.extend(chunk(b"IEND", &[]));
    png
}
