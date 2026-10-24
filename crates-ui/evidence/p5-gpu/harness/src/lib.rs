//! P5 浏览器验证宿主（**scratch**，不入库）。
//!
//! 职责：把引擎（`apeireth-ui-gpu`）驱动到真实浏览器里，各画一个真画面，
//! 并把**实测口径**（后端 / 表面格式 / 编码方式 / 帧数）回报给 CDP 驱动。
//!
//! 本文件不含任何业务语义：uniform 取值是「一组固定的可复现取值」，
//! 目的是让两个后端画**同一张图**以便逐像素对照（P5-b 才做与生产版
//! 逐帧一致）。

use apeireth_ui_gpu::{
    BackendPreference, Engine, EngineOptions, SceneDescriptor, SurfaceTargetInput, UniformValues,
    SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// 调试用日志（scratch 宿主专用）。
fn trace(message: &str) {
    let _ = web_sys::console::log_1(&JsValue::from_str(&format!("[harness] {message}")));
}

/// 全屏三角形（与生产版一致：`[-1,-1, 3,-1, -1,3]`）。
const FULLSCREEN_TRIANGLE: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

/// 在浏览器宿主里建引擎 → 建管线 → 画 N 帧 → 回报实测口径。
///
/// 返回一行 JSON（字段见 `report.json` 的判据定义）。
#[wasm_bindgen]
pub async fn run(
    canvas: web_sys::HtmlCanvasElement,
    backend: &str,
    frames: u32,
) -> Result<String, JsValue> {
    let preference = match backend {
        "webgl" => BackendPreference::WebGl,
        "webgpu" => BackendPreference::WebGpu,
        "auto" => BackendPreference::Auto,
        other => return Err(JsValue::from_str(&format!("unknown backend: {other}"))),
    };

    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let options = EngineOptions {
        backend: preference,
        width,
        height,
    };

    trace(&format!("create engine: backend={backend}"));
    let engine = Engine::create(SurfaceTargetInput::HtmlCanvas(canvas.clone()), options)
        .await
        .map_err(|e| JsValue::from_str(&format!("engine: {e}")))?;
    trace("engine created");
    let report = engine.report();

    // 画布尺寸（设备像素）→ 场景几何量。取值口径与生产版一致：
    //   桌面布局 cxRatio 0.66 / cyRatio 0.45 / holeRadiusRatio 0.16，
    //   机位预设 0（远眺）incl 0.3 / zoom 0.6。
    let layout_cx = 0.66 * width as f32;
    let layout_cy = 0.45 * height as f32;
    let hole_radius = (width.min(height) as f32) * 0.16;
    let zoom = 0.6_f32;

    let scene = SceneDescriptor::new(SCENE_VERTEX_GLSL, SCENE_FRAGMENT_GLSL)
        .with_vertex_data(FULLSCREEN_TRIANGLE.to_vec());
    let mut pipeline = engine
        .create_scene_pipeline(&scene)
        .map_err(|e| JsValue::from_str(&format!("pipeline: {e}")))?;
    trace("pipeline created");

    // 一组固定、可复现的 uniform 取值（binding 索引寻址）。
    let mut values = UniformValues::new();
    values.set(0, &[width as f32, height as f32]); // uRes
    values.set(1, &[layout_cx, height as f32 - layout_cy]); // uCenter
    values.set(2, &[hole_radius * zoom]); // uScale
    values.set(3, &[12.0]); // uTime
    values.set(4, &[0.0]); // uRot
    values.set(5, &[0.32]); // uBright
    values.set(6, &[0.12]); // uTurb
    values.set(7, &[0.0]); // uPulse
    values.set(8, &[0.3]); // uIncl
    values.set(9, &[0.0]); // uAzim
    values.set(10, &[0.0]); // uDiskAzim
    values.set(11, &[0.3]); // uDiskIncl
    values.set(12, &[1.0]); // uAmbStr
    values.set(13, &[0.0]); // uEclipse
    values.set(14, &[1.0]); // uTlBright
    values.set(15, &[0.5]); // uTlGold

    let mut drawn = 0u32;
    let mut last = None;
    for _ in 0..frames.max(1) {
        let frame = engine
            .draw_frame(&mut pipeline, &values)
            .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
        trace(&format!("frame {drawn} drawn"));
        last = Some(frame);
        drawn += 1;
        // 让出到浏览器任务队列 ⇒ 合成器有机会把这帧贴到画布上。
        yield_to_browser().await;
    }

    let frame = last.ok_or_else(|| JsValue::from_str("no frame drawn"))?;
    let color = report.color;

    Ok(format!(
        concat!(
            "{{\"backend_requested\":\"{}\",\"backend_actual\":\"{}\",",
            "\"webgpu_supported\":{},\"adapter_name\":\"{}\",\"adapter_vendor\":{},",
            "\"device_type\":\"{}\",\"surface_format\":\"{}\",\"encoding\":\"{}\",",
            "\"available_formats\":[{}],\"width\":{},\"height\":{},",
            "\"frames_drawn\":{},\"vertex_count\":{},\"uniform_members\":{}}}"
        ),
        match preference {
            BackendPreference::Auto => "auto",
            BackendPreference::WebGl => "webgl",
            BackendPreference::WebGpu => "webgpu",
        },
        format!("{:?}", report.backend),
        report.webgpu_supported,
        escape(&report.adapter_name),
        report.adapter_vendor,
        format!("{:?}", report.device_type),
        format!("{:?}", color.surface_format),
        match color.encoding {
            apeireth_ui_gpu::OutputEncoding::RawBytes => "raw_bytes",
            apeireth_ui_gpu::OutputEncoding::SrgbEncoded => "srgb_encoded",
        },
        color
            .available_formats
            .iter()
            .map(|f| format!("\"{:?}\"", f))
            .collect::<Vec<_>>()
            .join(","),
        report.width,
        report.height,
        drawn,
        frame.vertex_count,
        frame.uniform_members,
    ))
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

/// JSON 字符串转义（只处理本宿主用到的字符集）。
fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
