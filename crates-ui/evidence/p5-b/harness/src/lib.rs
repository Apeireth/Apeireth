//! P5-b 浏览器验证宿主（**scratch**，不入库）。
//!
//! 职责：把引擎（`apeireth-ui-gpu`）用**生产侧实测抓到的 uniform 取值**
//! 驱动到真实浏览器里，两个后端各画一张画布，供逐像素对照。
//!
//! ## 纪律
//!
//! - 本文件**不含任何场景语义**：uniform 取值全部来自宿主注入的 JS 对象
//!   （生产侧抓到的原样数值），引擎只做「uniform → 着色器」的管道；
//! - uniform **按名字**写入（`set_by_name`），顺序以引擎的
//!   `SHADER_UNIFORM_NAMES` 为准；
//! - 顶点数据与生产版逐值一致（全屏三角形 `[-1,-1, 3,-1, -1,3]`）。

use apeireth_ui_gpu::{
    BackendPreference, Engine, EngineOptions, SceneDescriptor, ScenePipeline, SurfaceTargetInput,
    UniformValues, SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL, SHADER_UNIFORM_NAMES,
};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// ⭐ **已建的引擎与管线**（供 [`redraw`] **复用**）。
///
/// ## 为什么需要它
///
/// 浏览器的 WebGL **drawing buffer 默认不保留**（`preserveDrawingBuffer: false`）：
/// **每次合成之后** buffer 被清空 ⇒ 下一次截图触发的合成拿到的就是**空**。
/// ⇒ 「画完 3 帧就停」⇒ 停后浏览器发生一次合成 ⇒ buffer 被清 ⇒ 之后截图**全白**
/// ⇒ 逐像素差 99.94% ⇒ **被误判为「引擎画错」**（2026-10-25 CI 实测，run 40170312024）。
///
/// ⇒ 修法是**持续重绘** ⇒ 而持续重绘**不能**每次都重建 engine + pipeline
/// （重建极重 ⇒ 实测 `Page.captureScreenshot` **超时**）⇒ ⇒ **建一次、复用多次**。
///
/// ⚠️ wasm 是**单线程**的 ⇒ 用 [`thread_local!`] 存放即可（无需 `Send`/`Sync`）。
type EngineSlot = RefCell<HashMap<String, (Engine, ScenePipeline, UniformValues)>>;

thread_local! {
    static ENGINE: EngineSlot = RefCell::new(HashMap::new());
}

/// 调试用日志（scratch 宿主专用）。
fn trace(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(&format!("[p5b] {message}")));
}

/// 全屏三角形（与生产版一致：`[-1,-1, 3,-1, -1,3]`）。
const FULLSCREEN_TRIANGLE: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

/// ⭐ **复用已建的引擎与管线**再画 N 帧（**不重建** GPU 资源）。
///
/// ## 为什么需要
///
/// 浏览器的 WebGL **drawing buffer 默认不保留** ⇒ 合成后被清空
/// ⇒ 「画完就停」⇒ 之后截图**全白**（2026-10-25 CI 实测，99.94% 差异）。
/// ⇒ 驱动需要「**截图前重画**」⇒ 而重画**不能**每次重建 engine + pipeline
/// （重建极重 ⇒ 实测 `Page.captureScreenshot` **超时**）⇒ ⇒ **复用**。
///
/// ## 口径
///
/// uniform **不重新读取** ⇒ 用**首次**注入的那一批（原样）⇒ 画面**恒定**
/// ⇒ ⇒ 截图拿到什么与「画了几帧」无关 ⇒ **对时序不敏感**。
#[wasm_bindgen]
pub async fn redraw(backend: &str, frames: u32) -> Result<String, JsValue> {
    let mut drawn = 0u32;
    ENGINE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some((engine, pipeline, values)) = guard.get_mut(backend) else {
            return Err(JsValue::from_str(&format!(
                "redraw: 后端 `{backend}` 的引擎尚未建立（先调 run）"
            )));
        };
        for _ in 0..frames.max(1) {
            engine
                .draw_frame(pipeline, values)
                .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
            drawn += 1;
        }
        Ok(())
    })?;
    for _ in 0..drawn {
        // 让出到浏览器任务队列 ⇒ 合成器有机会把这帧贴到画布上。
        yield_to_browser().await;
    }
    Ok(format!("{{\"frames_drawn\":{drawn}}}"))
}

/// 在浏览器宿主里建引擎 → 建管线 → 用宿主给的 uniform 画 N 帧 → 回报实测口径。
///
/// `uniforms` 是宿主注入的 JS 对象：形如 `{"uRes": [960, 600], "uScale": [57.6], …}`，
/// 键名必须覆盖 [`SHADER_UNIFORM_NAMES`] 的全部 16 项。
///
/// 返回一行 JSON（字段见 `report.json` 的判据定义）。
#[wasm_bindgen]
pub async fn run(
    canvas: web_sys::HtmlCanvasElement,
    backend: &str,
    frames: u32,
    uniforms: JsValue,
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

    trace(&format!(
        "create engine: backend={backend} {width}x{height}"
    ));
    let engine = Engine::create(SurfaceTargetInput::HtmlCanvas(canvas.clone()), options)
        .await
        .map_err(|e| JsValue::from_str(&format!("engine: {e}")))?;
    trace("engine created");
    let report = engine.report();

    let scene = SceneDescriptor::new(SCENE_VERTEX_GLSL, SCENE_FRAGMENT_GLSL)
        .with_vertex_data(FULLSCREEN_TRIANGLE.to_vec());
    let mut pipeline = engine
        .create_scene_pipeline(&scene)
        .map_err(|e| JsValue::from_str(&format!("pipeline: {e}")))?;
    trace("pipeline created");

    // uniform：按名字从宿主注入的对象里取值，**原样**喂进引擎（不四舍五入、
    // 不替换成「好看」的数）。
    let mut values = UniformValues::new();
    let mut used: Vec<String> = Vec::with_capacity(SHADER_UNIFORM_NAMES.len());
    for name in SHADER_UNIFORM_NAMES {
        let components = read_uniform(&uniforms, name)?;
        values
            .set_by_name(name, &components)
            .map_err(|e| JsValue::from_str(&format!("uniform `{name}`: {e}")))?;
        used.push(format!("\"{name}\":{}", format_components(&components)));
    }
    trace(&format!("uniforms bound: {}", used.join(" ")));

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

    // ⭐ 留存**引擎与管线** ⇒ [`redraw`] 可复用（不必每次重建 GPU 资源）。
    ENGINE.with(|slot| {
        slot.borrow_mut()
            .insert(String::from(backend), (engine, pipeline, values));
    });

    Ok(format!(
        concat!(
            "{{\"backend_requested\":\"{}\",\"backend_actual\":\"{}\",",
            "\"webgpu_supported\":{},\"adapter_name\":\"{}\",\"adapter_vendor\":{},",
            "\"device_type\":\"{}\",\"surface_format\":\"{}\",\"encoding\":\"{}\",",
            "\"available_formats\":[{}],\"width\":{},\"height\":{},",
            "\"frames_drawn\":{},\"vertex_count\":{},\"uniform_members\":{},",
            "\"uniforms_used\":{{{}}}}}"
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
        used.join(","),
    ))
}

/// 从宿主注入的 JS 对象里读一个 uniform 的分量列表。
fn read_uniform(uniforms: &JsValue, name: &str) -> Result<Vec<f32>, JsValue> {
    let key = JsValue::from_str(name);
    let value = js_sys::Reflect::get(uniforms, &key)
        .map_err(|e| JsValue::from_str(&format!("uniform `{name}` read: {e:?}")))?;
    if value.is_null() || value.is_undefined() {
        return Err(JsValue::from_str(&format!("uniform `{name}` missing")));
    }
    let array = js_sys::Array::from(&value);
    let mut out = Vec::with_capacity(array.length() as usize);
    for item in array.iter() {
        let number = item
            .as_f64()
            .ok_or_else(|| JsValue::from_str(&format!("uniform `{name}` has a non-number")))?;
        out.push(number as f32);
    }
    if out.is_empty() {
        return Err(JsValue::from_str(&format!("uniform `{name}` is empty")));
    }
    Ok(out)
}

/// 把分量列表格式化成可读文本（报告用；f32 的十进制表示，足够复现）。
fn format_components(components: &[f32]) -> String {
    let parts: Vec<String> = components
        .iter()
        .map(|value| format!("{value:?}"))
        .collect();
    format!("[{}]", parts.join(","))
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
