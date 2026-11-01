//! 场景对照 harness 的浏览器宿主（wasm）。
//!
//! 职责：**用场景层语义 crate 算出的 16 个取值**驱动引擎画到真实画布上，
//! 并把「算出来的取值」原样报回去 ⇒ 驱动脚本可以拿它与生产侧抓到的取值
//! **逐值**比较，再把两张画布**逐像素**比较。
//!
//! ## 纪律
//!
//! - 取值**全部**来自 `apeireth-ui-scene`（宿主侧语义 crate）—— 本宿主
//!   **不含任何场景语义**（没有一行手写的 uniform 数值）；
//! - uniform **按名字**写进引擎（`set_by_name`），顺序以引擎的映射表为准；
//! - 顶点数据与生产版逐值一致（全屏三角形 `[-1,-1, 3,-1, -1,3]`）；
//! - 动画路径的**帧时长序列**由驱动注入（生产侧实测抓到的 rAF 时间戳序列）
//!   ⇒ 状态指数趋近与机位插值在两侧**逐步一致**（不是「各自跑各自的」）。

use apeireth_ui_gpu::{
    BackendPreference, CanvasSizeSource, Engine, EngineOptions, SceneDescriptor, SurfaceTargetInput,
    UniformValues, SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL,
};
use apeireth_ui_scene::{PadBias, Scene, SceneMode, SceneOptions, Viewport};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// 调试用日志。
fn trace(message: &str) {
    let _ = web_sys::console::log_1(&JsValue::from_str(&format!("[scene] {message}")));
}

/// 全屏三角形（与生产版一致：`[-1,-1, 3,-1, -1,3]`）。
const FULLSCREEN_TRIANGLE: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

/// 跑一个用例：算取值 → 写进引擎 → 画 N 帧 → 回报取值与实测口径。
///
/// 参数：
///
/// - `mode`：`quiet` / `thinking` / `speaking`；
/// - `hour`：舰内时刻（`None` ⇒ 宿主本地时钟）；
/// - `camera`：机位下标（静态路径下会按生产侧规则锁定到第一个预设）；
/// - `path`：`static`（静态单帧）/ `animated`（动画路径）；
/// - `frozen_time`：冻结时钟（秒，`None` ⇒ 不冻结）；
/// - `dts`：动画路径的帧时长序列（秒）；
/// - `pad_p` / `pad_a` / `pad_d` / `intensity`：存在态 PAD 与强度
///   （`intensity` 为 `None` ⇒ 没有真实存在态帧）。
///
/// 返回一行 JSON（含 16 个取值的 f64 口径与 f32 口径）。
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub async fn run_case(
    canvas: web_sys::HtmlCanvasElement,
    backend: &str,
    mode: &str,
    hour: Option<f64>,
    camera: u32,
    path: &str,
    frozen_time: Option<f64>,
    css_width: f64,
    css_height: f64,
    dpr: f64,
    dts: js_sys::Array,
    pad_p: f64,
    pad_a: f64,
    pad_d: f64,
    intensity: Option<f64>,
    frames: u32,
) -> Result<String, JsValue> {
    let preference = match backend {
        "webgl" => BackendPreference::WebGl,
        "webgpu" => BackendPreference::WebGpu,
        "auto" => BackendPreference::Auto,
        other => return Err(JsValue::from_str(&format!("unknown backend: {other}"))),
    };
    let scene_mode = match mode {
        "quiet" => SceneMode::Quiet,
        "thinking" => SceneMode::Thinking,
        "speaking" => SceneMode::Speaking,
        other => return Err(JsValue::from_str(&format!("unknown mode: {other}"))),
    };
    let static_path = match path {
        "static" => true,
        "animated" => false,
        other => return Err(JsValue::from_str(&format!("unknown path: {other}"))),
    };

    // ── 1) 取值链：场景层语义 crate ───────────────────────────────────────
    let viewport = Viewport {
        css_width,
        css_height,
        device_pixel_ratio: dpr,
    };
    let options = SceneOptions {
        interactive: false,
        reduce_motion: static_path,
        frozen_time,
    };
    let mut scene = Scene::new(viewport, options);
    scene.set_mode(scene_mode);
    if let Some(value) = hour {
        scene.set_hour(Some(value));
    }
    if intensity.is_some() {
        scene.set_pad_bias(
            PadBias {
                p: pad_p,
                a: pad_a,
                d: pad_d,
            },
            intensity,
        );
    }
    if !scene.set_camera(camera as usize) {
        return Err(JsValue::from_str(&format!(
            "camera index rejected: {camera}"
        )));
    }

    let mut scene_frames = 0usize;
    let uniforms = if static_path {
        let uniforms = scene
            .render_once()
            .ok_or_else(|| JsValue::from_str("static frame unavailable"))?;
        scene_frames = 1;
        uniforms
    } else {
        let mut last = None;
        for item in dts.iter() {
            let dt = item
                .as_f64()
                .ok_or_else(|| JsValue::from_str("dt entry is not a number"))?;
            let uniforms = scene
                .advance(dt)
                .ok_or_else(|| JsValue::from_str("animated frame unavailable"))?;
            last = Some(uniforms);
            scene_frames += 1;
        }
        last.ok_or_else(|| JsValue::from_str("no animated frame"))?
    };

    // ── 2) 引擎：取值 → 着色器 ───────────────────────────────────────────
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let engine_options = EngineOptions {
        backend: preference,
        width,
        height,
        // ⭐ **画布尺寸自侦测**（2026-10-25 的 resize 闭合物料）：
        // 引擎**每帧**比对画布实际尺寸 ⇒ ⇒ 宿主改 `canvas.width/height` 时
        // ⇒ 引擎**自己**跟上（重配表面 + 换交换链）⇒ ⇒ 行为与生产**同形**
        // （生产也是「改 canvas 尺寸就完事」，什么都不调）。
        // ⇒ ⇒ `None` ⇒ 不自侦测（原生路径）。
        canvas_size: Some(CanvasSizeSource::new({
            let canvas = canvas.clone();
            move || {
                (
                    u32::try_from(canvas.width().max(1)).unwrap_or(1),
                    u32::try_from(canvas.height().max(1)).unwrap_or(1),
                )
            }
        })),
    };
    trace(&format!(
        "engine: backend={backend} {width}x{height} mode={mode} path={path}"
    ));
    let engine = Engine::create(SurfaceTargetInput::HtmlCanvas(canvas.clone()), engine_options)
        .await
        .map_err(|e| JsValue::from_str(&format!("engine: {e}")))?;
    let report = engine.report();

    let scene_descriptor = SceneDescriptor::new(SCENE_VERTEX_GLSL, SCENE_FRAGMENT_GLSL)
        .with_vertex_data(FULLSCREEN_TRIANGLE.to_vec());
    let mut pipeline = engine
        .create_scene_pipeline(&scene_descriptor)
        .map_err(|e| JsValue::from_str(&format!("pipeline: {e}")))?;

    let mut values = UniformValues::new();
    uniforms
        .write_into(&mut values)
        .map_err(|e| JsValue::from_str(&format!("uniform binding: {e}")))?;

    let mut drawn = 0u32;
    let mut frame_report = None;
    for _ in 0..frames.max(1) {
        let frame = engine
            .draw_frame(&mut pipeline, &values)
            .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
        frame_report = Some(frame);
        drawn += 1;
        yield_to_browser().await;
    }
    let frame = frame_report.ok_or_else(|| JsValue::from_str("no frame drawn"))?;

    // ── 3) 回报：取值（f64 口径 + f32 口径）+ 实测口径 ────────────────────
    let f64_entries = uniforms.entries();
    let f32_entries = uniforms.entries_f32();
    let mut json = String::new();
    json.push('{');
    json.push_str(&format!(
        "\"backend_requested\":\"{}\",\"backend_actual\":\"{:?}\",",
        match preference {
            BackendPreference::Auto => "auto",
            BackendPreference::WebGl => "webgl",
            BackendPreference::WebGpu => "webgpu",
        },
        report.backend
    ));
    json.push_str(&format!(
        "\"encoding\":\"{}\",\"surface_format\":\"{:?}\",",
        match report.color.encoding {
            apeireth_ui_gpu::OutputEncoding::RawBytes => "raw_bytes",
            apeireth_ui_gpu::OutputEncoding::SrgbEncoded => "srgb_encoded",
        },
        report.color.surface_format
    ));
    json.push_str(&format!(
        "\"width\":{},\"height\":{},\"frames_drawn\":{},\"vertex_count\":{},\"uniform_members\":{},",
        report.width, report.height, drawn, frame.vertex_count, frame.uniform_members
    ));
    json.push_str(&format!("\"scene_frames\":{scene_frames},"));
    json.push_str(&format!(
        "\"canvas_width\":{},\"canvas_height\":{},",
        width, height
    ));
    json.push_str("\"uniforms\":{");
    for (index, (name, components)) in f64_entries.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&format!("\"{name}\":{}", json_components(components)));
    }
    json.push_str("},\"uniforms_f32\":{");
    for (index, (name, components)) in f32_entries.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&format!("\"{name}\":{}", json_components_f32(components)));
    }
    json.push_str("}}");
    trace(&format!("drawn {drawn} frames, scene frames {scene_frames}"));
    Ok(json)
}

/// f64 分量列表 → JSON 数组文本（短表示，可原样往返）。
fn json_components(components: &[f64]) -> String {
    let mut out = String::from("[");
    for (index, value) in components.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        if !value.is_finite() {
            // 非有限值在 JSON 里无法表示 ⇒ 显式报错（不写「NaN」这种假值）。
            out.push_str("null");
            continue;
        }
        out.push_str(&format!("{value:?}"));
    }
    out.push(']');
    out
}

/// f32 分量列表 → JSON 数组文本。
fn json_components_f32(components: &[f32]) -> String {
    let mut out = String::from("[");
    for (index, value) in components.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        if !value.is_finite() {
            out.push_str("null");
            continue;
        }
        out.push_str(&format!("{value:?}"));
    }
    out.push(']');
    out
}

/// 让出到浏览器的**任务队列**（宏任务）—— 微任务不足以保证合成器已出帧。
async fn yield_to_browser() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let fired = web_sys::window().map(|window| {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
                .is_ok()
        });
        if !fired.unwrap_or(false) {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let _ = JsFuture::from(promise).await;
}
