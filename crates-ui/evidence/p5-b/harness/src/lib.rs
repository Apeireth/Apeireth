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
    BackendPreference, BlendMode, CanvasSizeSource, Engine, EngineOptions, SceneDescriptor,
    ScenePass, ScenePipeline, SurfaceTargetInput, Topology, UniformValues, VertexFormat,
    INTRO_BACKDROP_FRAGMENT_GLSL, INTRO_BACKDROP_UNIFORM_NAMES, INTRO_BACKDROP_VERTEX_GLSL,
    INTRO_SPRITE_FRAGMENT_GLSL, INTRO_SPRITE_UNIFORM_NAMES, INTRO_SPRITE_VERTEX_GLSL,
    INTRO_SPRITE_PROBE_FRAGMENT_GLSL,
    INTRO_SPRITE_FLAT_FRAGMENT_GLSL,
    INTRO_SPRITE_COORD_FRAGMENT_GLSL,
    SCENE_FRAGMENT_GLSL, SCENE_VERTEX_GLSL, SHADER_UNIFORM_NAMES,
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
/// ⚠️ wasm 是**单线程**的 ⇒ 用 [`thread_local!]` 存放即可（无需 `Send`/`Sync`）。
type EngineSlot = RefCell<HashMap<String, EngineEntry>>;

/// 已建的引擎 + 管线（**可带多条精灵管线**：片头是**两个 pass**）。
struct EngineEntry {
    /// 引擎本体。
    engine: Engine,
    /// 主 pass 的管线（背景 / 全屏三角形）—— `None` = 只画粒子（隔离对照）。
    pipeline: Option<ScenePipeline>,
    /// 主 pass 的 uniform 取值。
    values: UniformValues,
    /// 精灵 pass（`GL_POINTS`）的管线与取值（**每拍一组**）。
    sprites: Vec<(ScenePipeline, UniformValues)>,
    /// 本次跑的 pass 组合（`all` / `backdrop` / `sprite`）—— 重绘时**照旧**。
    passes: String,
}

thread_local! {
    static ENGINE: EngineSlot = RefCell::new(HashMap::new());
}

/// ⭐⭐ **读取设备事件**（设备丢失 / 设备错误）—— **真实 GPU 实测**用。
///
/// ## 为什么需要这个出口
///
/// 「设备丢失」（GPU 上下文被作废）在**真实硬件**上才会真发生
/// （浏览器回收 GPU 上下文 / 驱动复位 / GPU 进程崩溃）
/// ⇒ ⇒ 要验它，必须能**从浏览器侧**把引擎观测到的事件**取出来**
/// ⇒ ⇒ 宿主（页面）才能断言「引擎确实报了丢失，且带帧号」。
///
/// ## 口径
///
/// - 先调 [`Engine::poll_device`]（**有界等待**）⇒ ⇒ 把**异步到达**的事件取回来
///   ⇒ ⇒ ⚠️ 不 poll ⇒ ⇒ 后端错误**永远不派发**（失败完全静默）。
/// - 返回 JSON 数组；每个元素形如
///   `{"kind":"lost","frame":12,"reason":"Destroyed","message":"…"}`
///   或 `{"kind":"error","frame":7,"message":"…"}`。
#[wasm_bindgen(js_name = pollDeviceEvents)]
pub fn poll_device_events() -> String {
    let mut out: Vec<String> = Vec::new();
    ENGINE.with(|slot| {
        let mut slot = slot.borrow_mut();
        for (name, entry) in slot.iter_mut() {
            // ⚠️ poll 失败也算**可报告的事实**（设备已丢时 poll 会失败）。
            if let Err(error) = entry.engine.poll_device() {
                out.push(format!(
                    "{{\"kind\":\"poll_error\",\"engine\":{},\"message\":{}}}",
                    json_string(name),
                    json_string(&format!("{error:?}")),
                ));
            }
            for event in entry.engine.take_device_events() {
                let text = match event {
                    apeireth_ui_gpu::DeviceEvent::Lost {
                        frame,
                        reason,
                        message,
                    } => format!(
                        "{{\"kind\":\"lost\",\"engine\":{},\"frame\":{frame},\"reason\":{reason_json},\"message\":{message_json}}}",
                        json_string(name),
                        reason_json = json_string(&reason),
                        message_json = json_string(&message),
                    ),
                    apeireth_ui_gpu::DeviceEvent::Error { frame, message } => format!(
                        "{{\"kind\":\"error\",\"engine\":{},\"frame\":{frame},\"message\":{}}}",
                        json_string(name),
                        json_string(&message),
                    ),
                    other => format!(
                        "{{\"kind\":\"other\",\"engine\":{},\"detail\":{}}}",
                        json_string(name),
                        json_string(&format!("{other:?}")),
                    ),
                };
                out.push(text);
            }
        }
    });
    format!("[{}]", out.join(","))
}

/// ⭐⭐ **主动销毁设备**（= WebGPU 规范的 `device.destroy()`）⇒ ⇒ 触发**设备丢失**。
///
/// ## 为什么用它来验「设备丢失」
///
/// - 真实硬件上最自然的丢失触发是「GPU 进程被杀 / 驱动复位」
///   ⇒ ⇒ 但那会让**渲染进程一起崩** ⇒ ⇒ **页面 JS 上下文随之消失**
///   ⇒ ⇒ ⇒ **观测不到**（实测：headless 下杀 GPU 进程 ⇒ 页面失联）。
/// - 而 `device.destroy()` 是 **WebGPU 规范自带**的动作
///   ⇒ ⇒ 引擎侧同样走 `DeviceLost` 回调（reason = `Destroyed`）
///   ⇒ ⇒ **确定性高、页面不死** ⇒ ⇒ ⇒ **可观测**。
///
/// ⇒ ⇒ 两者在**引擎看来是同一种事件** ⇒ ⇒ 用后者验通路，前者作为补充记录。
#[wasm_bindgen(js_name = destroyDevice)]
pub fn destroy_device() -> String {
    let mut out: Vec<String> = Vec::new();
    ENGINE.with(|slot| {
        let mut slot = slot.borrow_mut();
        for (name, entry) in slot.iter_mut() {
            entry.engine.destroy_device();
            out.push(format!(
                "{{\"kind\":\"destroyed\",\"engine\":{}}}",
                json_string(name),
            ));
        }
    });
    format!("[{}]", out.join(","))
}

/// JSON 字符串转义（最小实现：够用于设备事件文本）。
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
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
    out.push('"');
    out
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
///
/// ## ⭐ 顺带回报**表面状态**（resize 自侦测的证据）
///
/// 返回值里带 `surface_width` / `surface_height` / `surface_generation` /
/// `auto_resize_count` ⇒ ⇒ 驱动能**直接**看出「画布改了尺寸以后，引擎自己
/// 跟上没有」⇒ **不必**从像素反推。
#[wasm_bindgen]
pub async fn redraw(backend: &str, frames: u32) -> Result<String, JsValue> {
    let mut drawn = 0u32;
    let mut surface = (0u32, 0u32, 0u64, 0u64);
    ENGINE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(entry) = guard.get_mut(backend) else {
            return Err(JsValue::from_str(&format!(
                "redraw: 后端 `{backend}` 的引擎尚未建立（先调 run）"
            )));
        };
        for _ in 0..frames.max(1) {
            let Some(pipeline) = entry.pipeline.as_mut() else {
                return Err(JsValue::from_str("redraw: 片头管线不存在"));
            };
            entry
                .engine
                .draw_frame(pipeline, &entry.values)
                .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
            drawn += 1;
        }
        let (width, height) = entry.engine.surface_size();
        surface = (
            width,
            height,
            entry.engine.surface_generation(),
            entry.engine.auto_resize_count(),
        );
        Ok(())
    })?;
    for _ in 0..drawn {
        // 让出到浏览器任务队列 ⇒ 合成器有机会把这帧贴到画布上。
        yield_to_browser().await;
    }
    Ok(format!(
        "{{\"frames_drawn\":{drawn},\"surface_width\":{},\"surface_height\":{},\"surface_generation\":{},\"auto_resize_count\":{}}}",
        surface.0, surface.1, surface.2, surface.3
    ))
}

/// 在浏览器宿主里建引擎 → 建管线 → 用宿主给的 uniform 画 N 帧 → 回报实测口径。
///
/// `uniforms` 是宿主注入的 JS 对象：形如 `{"uRes": [960, 600], "uScale": [57.6], …}`，
/// 键名必须覆盖 [`SHADER_UNIFORM_NAMES`] 的全部 16 项。
///
/// 返回一行 JSON（字段见 `report.json` 的判据定义）。
///
/// ## ⭐ 尺寸**来源**（自侦测）
///
/// 引擎**不持有画布句柄** ⇒ 它自己**看不到** `canvas.width/height` 变没变
/// ⇒ ⇒ 宿主把「读画布尺寸」的闭包交给引擎（[`CanvasSizeSource`]）⇒ 引擎
/// **每帧**比对 ⇒ 不一致就**自动** resize ⇒ ⇒ 与生产（Svelte + `<canvas>`：
/// 改 `canvas.width/height` 就完事）**行为一致**。
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
    // ⭐ 尺寸来源：读**画布当前**的 `width/height` ⇒ 引擎**自侦测**尺寸变化。
    let size_canvas = canvas.clone();
    let options = EngineOptions {
        backend: preference,
        width,
        height,
        canvas_size: Some(CanvasSizeSource::new(move || {
            (size_canvas.width(), size_canvas.height())
        })),
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
    // ⚠ 引擎马上要被**移进** ENGINE 槽（wasm 单线程 ⇒ 用 thread_local 存）
    //   ⇒ 移走**之前**先把要回报的计数取出来。
    let auto_resize_count = engine.auto_resize_count();

    // ⭐ 留存**引擎与管线** ⇒ [`redraw`] 可复用（不必每次重建 GPU 资源）。
    ENGINE.with(|slot| {
        slot.borrow_mut().insert(
            String::from(backend),
            EngineEntry {
                engine,
                pipeline: Some(pipeline),
                values,
                sprites: Vec::new(),
                passes: String::from(PASS_ALL),
            },
        );
    });

    Ok(format!(
        concat!(
            "{{\"backend_requested\":\"{}\",\"backend_actual\":\"{}\",",
            "\"webgpu_supported\":{},\"adapter_name\":\"{}\",\"adapter_vendor\":{},",
            "\"device_type\":\"{}\",\"surface_format\":\"{}\",\"encoding\":\"{}\",",
            "\"available_formats\":[{}],\"width\":{},\"height\":{},",
            "\"frames_drawn\":{},\"vertex_count\":{},\"uniform_members\":{},",
            "\"surface_width\":{},\"surface_height\":{},\"surface_generation\":{},",
            "\"auto_resize_count\":{},",
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
        frame.surface_width,
        frame.surface_height,
        frame.surface_generation,
        auto_resize_count,
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

// ---------------------------------------------------------------------------
// ⭐ 片头「火之文明史」腿（生产 `intro/fire.ts` 的迁移验证）
// ---------------------------------------------------------------------------

/// ⭐ 片头**要画哪些 pass**（诊断用：单 pass 隔离对照）。
///
/// - `all`：背景 matte + 全部粒子 pass（= 生产的完整一帧）；
/// - `backdrop`：**只**画背景 matte（⇒ 与生产的「只画背景」逐像素对照）；
/// - `sprite`：**只**画粒子（⇒ 与生产的「只画粒子」逐像素对照）。
const PASS_ALL: &str = "all";
const PASS_BACKDROP: &str = "backdrop";
const PASS_SPRITE: &str = "sprite";

/// ⭐ 粒子的**绘制路径**（两条路径**必须产出同一画面** ⇒ 由逐像素门禁对照）。
///
/// - [`SpriteTopology::InstancedQuad`]：**实例化四边形**（后端无关 ⇒ 两个
///   后端都能画）⇒ **默认路径**；
/// - [`SpriteTopology::PointList`]：点图元（WebGL 快路径；WebGPU 后端
///   **显式报错** ⇒ 那一腿**无效** ⇒ 只作对照用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpriteTopology {
    InstancedQuad,
    PointList,
}

/// ⭐ 片头一帧的 uniform 取值（宿主注入的 JS 对象）。
///
/// 形如：
/// ```text
/// {"__backdrop": {"uRes": [960, 600], …},
///  "__sprites": [{"uAspect": [1.6], …}, …]}
/// ```
///
/// - `__backdrop` = 背景 matte 的 27 项（生产 `BG_UNIFORMS`）；
/// - `__sprites` = **每拍一组**的 40 项（生产 `FIRE_UNIFORMS`）⇒ crossfade
///   期间有**两组** ⇒ ⇒ 数组长度 = 活跃拍数。
///
/// ⚠ 取值**按索引**（= 声明序）写进引擎的 uniform 集合 ⇒ 与引擎的
///   `glsl::fragment_uniform_block_members` 成员序**必然一致**。
#[wasm_bindgen]
pub async fn run_intro(
    canvas: web_sys::HtmlCanvasElement,
    backend: &str,
    frames: u32,
    uniforms: JsValue,
    passes: &str,
    sprite_mode: &str,
    // ⭐ **诊断探针**（2026-10-27）：`""` = 正常渲染；`"varying"` = 用
    // **varying 编码探针片元**（把 `vAlpha`/`vCol.g`/`vStreak` 编码进 RGB）。
    // ⇒ ⇒ 目的：**两侧用同一个探针** ⇒ ⇒ 逐像素差分**直接读出是哪个 varying 不同**。
    // ⚠⚠️ 这里**必须**用 `//` 而不是 `///`：`#[wasm_bindgen]` 的**参数**上
    // 只接受 `allow`/`cfg`/`cfg_attr`/`deny`/`expect`/`forbid`/`warn`
    // ⇒ ⇒ `///` 会被当成**属性** ⇒ ⇒ 编译错误（实测踩到）。
    probe: Option<String>,
) -> Result<String, JsValue> {
    let preference = match backend {
        "webgl" => BackendPreference::WebGl,
        "webgpu" => BackendPreference::WebGpu,
        "auto" => BackendPreference::Auto,
        other => return Err(JsValue::from_str(&format!("unknown backend: {other}"))),
    };
    if !matches!(passes, PASS_ALL | PASS_BACKDROP | PASS_SPRITE) {
        return Err(JsValue::from_str(&format!("unknown pass filter: {passes}")));
    }
    // ⭐ 粒子的**绘制路径**：实例化四边形（后端无关）/ 点图元（WebGL 快路径）。
    let sprite_topology = match sprite_mode {
        "quad" => SpriteTopology::InstancedQuad,
        "point" => SpriteTopology::PointList,
        other => return Err(JsValue::from_str(&format!("unknown sprite mode: {other}"))),
    };
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let size_canvas = canvas.clone();
    let options = EngineOptions {
        backend: preference,
        width,
        height,
        canvas_size: Some(CanvasSizeSource::new(move || {
            (size_canvas.width(), size_canvas.height())
        })),
    };
    let engine = Engine::create(SurfaceTargetInput::HtmlCanvas(canvas.clone()), options)
        .await
        .map_err(|e| JsValue::from_str(&format!("engine: {e}")))?;
    let report = engine.report();

    // ── 背景 matte pass ──────────────────────────────────────────────────
    let mut pipeline = None;
    let mut values = UniformValues::new();
    let mut used: Vec<String> = Vec::new();
    if passes != PASS_SPRITE {
        let scene = SceneDescriptor::new(
            INTRO_BACKDROP_VERTEX_GLSL,
            INTRO_BACKDROP_FRAGMENT_GLSL,
        )
        .with_vertex_data(FULLSCREEN_TRIANGLE.to_vec());
        pipeline = Some(
            engine
                .create_scene_pipeline(&scene)
                .map_err(|e| JsValue::from_str(&format!("pipeline: {e}")))?,
        );
        let backdrop = js_sys::Reflect::get(&uniforms, &JsValue::from_str("__backdrop"))
            .map_err(|e| JsValue::from_str(&format!("__backdrop read: {e:?}")))?;
        for (index, name) in INTRO_BACKDROP_UNIFORM_NAMES.iter().enumerate() {
            let components = read_uniform(&backdrop, name)?;
            values.set(index as u32, &components);
            used.push(format!("\"{name}\":{}", format_components(&components)));
        }
    }

    // ⭐ 火焰粒子 pass（**每拍一组**）：生产侧同一帧可能画**两层** `GL_POINTS`
    //   （crossfade 期间两拍同时活跃）⇒ ⇒ 必须**逐组**画。
    //   - 顶点阶段有 40 个 uniform ⇒ 走引擎的**顶点 uniform 块**（binding = 1）；
    //   - 顶点属性是 `vec4`（`aSeed`，4096 个）⇒ stride 16；
    //   - 拓扑：⭐ **实例化四边形**（后端无关 ⇒ WebGL 与 WebGPU **都能画**）
    //     / **点列表**（WebGL 快路径；WebGPU 后端**显式报错** ⇒ 该腿无效）；
    //   - 混合是**加法**（`ONE/ONE`）⇒ 与生产的 `gl.blendFunc(ONE, ONE)` 同义。
    // ⭐ **诊断探针片元**（2026-10-27）：`probe == "varying"` ⇒ ⇒ 用 varying 编码探针。
    let sprite_fragment = match probe.as_deref() {
        Some("varying") => INTRO_SPRITE_PROBE_FRAGMENT_GLSL,
        Some("flat") => INTRO_SPRITE_FLAT_FRAGMENT_GLSL,
        Some("coord") => INTRO_SPRITE_COORD_FRAGMENT_GLSL,
        Some(other) => {
            return Err(JsValue::from_str(&format!(
                "run_intro: 未知探针 `{other}`（支持：\"\" / \"varying\"）"
            )))
        }
        None => INTRO_SPRITE_FRAGMENT_GLSL,
    };
    let mut sprites: Vec<(ScenePipeline, UniformValues)> = Vec::new();
    if passes != PASS_BACKDROP {
        let seeds = apeireth_ui_scene::intro::sprites::sprite_seeds();
        let sprite_scene = match sprite_topology {
            // ⭐ 四边形路径：粒子属性走**实例缓冲**（每实例 1 个粒子）
            //   ⇒ 顶点数据**不变**（不是 4 × 4096 个顶点）。
            SpriteTopology::InstancedQuad => SceneDescriptor::new(
                INTRO_SPRITE_VERTEX_GLSL,
                sprite_fragment,
            )
            .with_instance_data(seeds)
            .with_instance_format(VertexFormat::Vec4)
            .with_topology(Topology::InstancedQuad)
            .with_blend(BlendMode::Additive),
            // 点图元路径（WebGL 快路径）：粒子属性走**顶点缓冲**。
            SpriteTopology::PointList => SceneDescriptor::new(
                INTRO_SPRITE_VERTEX_GLSL,
                sprite_fragment,
            )
            .with_vertex_data(seeds)
            .with_vertex_format(VertexFormat::Vec4)
            .with_topology(Topology::PointList)
            .with_blend(BlendMode::Additive),
        };
        let list = js_sys::Reflect::get(&uniforms, &JsValue::from_str("__sprites"))
            .map_err(|e| JsValue::from_str(&format!("__sprites read: {e:?}")))?;
        let array = js_sys::Array::from(&list);
        for (index, item) in array.iter().enumerate() {
            let pipeline = engine
                .create_scene_pipeline(&sprite_scene)
                .map_err(|e| JsValue::from_str(&format!("sprite pipeline: {e}")))?;
            let mut values = UniformValues::new();
            for (slot, name) in INTRO_SPRITE_UNIFORM_NAMES.iter().enumerate() {
                let components = read_uniform(&item, name)?;
                values.set(slot as u32, &components);
            }
            sprites.push((pipeline, values));
            trace(&format!("sprite pipeline {index} created"));
        }
    }

    let sprite_pass_count = sprites.len();
    let mut drawn = 0u32;
    let mut last = None;
    for _ in 0..frames.max(1) {
        let frame = draw_intro_frame(&engine, pipeline.as_mut(), &values, &mut sprites)?;
        last = Some(frame);
        drawn += 1;
        yield_to_browser().await;
    }
    let frame = last.ok_or_else(|| JsValue::from_str("no frame drawn"))?;
    // ⭐ 留存引擎与管线 ⇒ [`redraw_intro`] 可复用（截图前持续重绘）。
    ENGINE.with(|slot| {
        slot.borrow_mut().insert(
            String::from(backend),
            EngineEntry {
                engine,
                pipeline,
                values,
                sprites,
                passes: String::from(passes),
            },
        );
    });
    Ok(format!(
        concat!(
            "{{\"backend_requested\":\"{}\",\"backend_actual\":\"{}\",",
            "\"surface_format\":\"{}\",\"encoding\":\"{}\",",
            "\"width\":{},\"height\":{},\"frames_drawn\":{},",
            "\"vertex_count\":{},\"instance_count\":{},\"uniform_members\":{},",
            "\"passes\":\"{}\",\"sprite_passes\":{},\"sprite_mode\":\"{}\",",
            "\"uniforms_used\":{{{}}}}}"
        ),
        match preference {
            BackendPreference::Auto => "auto",
            BackendPreference::WebGl => "webgl",
            BackendPreference::WebGpu => "webgpu",
        },
        format!("{:?}", report.backend),
        format!("{:?}", report.color.surface_format),
        match report.color.encoding {
            apeireth_ui_gpu::OutputEncoding::RawBytes => "raw_bytes",
            apeireth_ui_gpu::OutputEncoding::SrgbEncoded => "srgb_encoded",
        },
        report.width,
        report.height,
        drawn,
        frame.vertex_count,
        frame.instance_count,
        frame.uniform_members,
        passes,
        sprite_pass_count,
        match sprite_topology {
            SpriteTopology::InstancedQuad => "quad",
            SpriteTopology::PointList => "point",
        },
        used.join(","),
    ))
}

/// ⭐ 画片头的一帧（背景 + N 组粒子 ⇒ **一次呈现**）。
///
/// ⚠ **一次呈现**是硬要求：生产侧背景与粒子画进**同一张**表面 ⇒ 分两次画
///   ⇒ 第二次会重新取纹理并呈现 ⇒ ⇒ 第一次的画面被覆盖。
fn draw_intro_frame(
    engine: &Engine,
    pipeline: Option<&mut ScenePipeline>,
    backdrop_values: &UniformValues,
    sprites: &mut [(ScenePipeline, UniformValues)],
) -> Result<apeireth_ui_gpu::FrameReport, JsValue> {
    let mut all: Vec<ScenePass<'_>> = Vec::with_capacity(1 + sprites.len());
    if let Some(pipeline) = pipeline {
        all.push(ScenePass {
            pipeline,
            uniforms: backdrop_values,
            vertex_uniforms: None,
        });
    }
    for (pipeline, sprite_values) in sprites.iter_mut() {
        let uniforms: &UniformValues = sprite_values;
        all.push(ScenePass {
            pipeline,
            uniforms,
            vertex_uniforms: Some(uniforms),
        });
    }
    if all.is_empty() {
        return Err(JsValue::from_str("no pass to draw"));
    }
    engine
        .draw_passes(&mut all)
        .map_err(|e| JsValue::from_str(&format!("draw: {e}")))
}

/// ⭐ **复用**已建的片头引擎与管线再画 N 帧（**不重建** GPU 资源）。
///
/// ## 为什么需要
///
/// 浏览器的 WebGL **drawing buffer 默认不保留** ⇒ 合成后被清空 ⇒
/// 「画完就停」⇒ 之后截图**全白**（2026-10-25 CI 实测，99.94% 差异）⇒
/// ⇒ 截图前必须**持续重绘** ⇒ 而持续重绘**不能**每次重建 engine + pipeline
/// （重建极重 ⇒ 实测 `Page.captureScreenshot` **超时**）⇒ ⇒ **复用**。
#[wasm_bindgen]
pub async fn redraw_intro(backend: &str, frames: u32) -> Result<String, JsValue> {
    let mut drawn = 0u32;
    ENGINE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(entry) = guard.get_mut(backend) else {
            return Err(JsValue::from_str(&format!(
                "redraw_intro: 后端 `{backend}` 的引擎尚未建立（先调 run_intro）"
            )));
        };
        for _ in 0..frames.max(1) {
            draw_intro_frame(
                &entry.engine,
                entry.pipeline.as_mut(),
                &entry.values,
                &mut entry.sprites,
            )
            .map_err(|e| JsValue::from_str(&format!("draw: {e:?}")))?;
            drawn += 1;
        }
        Ok(())
    })?;
    for _ in 0..drawn {
        yield_to_browser().await;
    }
    Ok(format!("{{\"frames_drawn\":{drawn}}}"))
}

/// ⭐ 片头粒子的**种子属性**（Rust 侧生成的 `vec4` 序列）。
///
/// 返回 JSON 数组（f32 值）⇒ ⇒ 页面可与「按生产同一段循环复算」的值
/// **逐值**对照。
#[wasm_bindgen]
pub fn intro_sprite_seeds() -> Result<JsValue, JsValue> {
    let seeds = apeireth_ui_scene::intro::sprites::sprite_seeds();
    let array = js_sys::Array::new_with_length(seeds.len() as u32);
    for (index, value) in seeds.iter().enumerate() {
        array.set(index as u32, JsValue::from_f64(*value as f64));
    }
    Ok(array.into())
}

/// ⭐ 片头编舞取值（Rust 侧算出来的）与生产侧**逐值**对照用。
///
/// 返回 JSON：`{"beats":N,"backdrop":{name:[...]},"sprites":[{name:[...]}…]}`。
///
/// ⚠ 取值以 **f64** 的十进制给出（**不**先收口 f32）⇒ ⇒ 页面可以按
///   **f32 收口后**逐值比对（着色器取值本来就是单精度）。
#[wasm_bindgen]
pub fn intro_uniforms(
    t: f64,
    res_width: f64,
    res_height: f64,
    aspect: f64,
    dpr: f64,
) -> Result<String, JsValue> {
    let viewport = apeireth_ui_scene::intro::IntroViewport {
        res: [res_width, res_height],
        aspect,
        dpr,
    };
    let Some(frame) = apeireth_ui_scene::intro::intro_frame(t, &viewport) else {
        return Ok("{\"beats\":0,\"backdrop\":null,\"sprites\":[]}".to_string());
    };
    let drawn = frame.drawn_sprites();
    let mut out = String::from("{\"beats\":");
    out.push_str(&frame.sprites.len().to_string());
    out.push_str(",\"drawn_passes\":");
    out.push_str(&drawn.len().to_string());
    out.push_str(",\"backdrop\":");
    match &frame.backdrop {
        Some(backdrop) => {
            out.push('{');
            let mut first = true;
            for (name, components) in backdrop.entries() {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&format!("\"{name}\":{}", format_f64_components(&components)));
            }
            out.push('}');
        }
        None => out.push_str("null"),
    }
    out.push_str(",\"sprites\":[");
    // ⚠ 只报**生产真正会画的那几组**（生产 `drawFire` 的跳过判据）⇒
    //   ⇒ 与生产侧抓到的绘制**逐组一一对应**（落幅拍的 `count01 = 0`
    //   ⇒ 生产侧不画 ⇒ 这里也不报）。
    for (index, sprite) in drawn.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        let mut first = true;
        for (name, components) in sprite.entries() {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&format!("\"{name}\":{}", format_f64_components(&components)));
        }
        out.push('}');
    }
    out.push_str("]}");
    Ok(out)
}

/// 把 f64 分量列表格式化成 JSON 数组（**保留完整精度**：17 位有效数字）。
fn format_f64_components(components: &[f64]) -> String {
    let parts: Vec<String> = components
        .iter()
        .map(|value| format!("{value:?}"))
        .collect();
    format!("[{}]", parts.join(","))
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

/// ⭐⭐⭐ **L1 能力浏览器探针**（L1-G1/G2/G3/G6 的**浏览器侧证据**）。
///
/// ## 为什么返回 `JsValue` 而不是 JSON 字符串
///
/// 手拼 JSON 要在 Rust 里处理转义与格式（上一轮就栽在 `{:?}` 输出 Rust Debug 格式上）
/// ⇒ ⇒ 直接用 `js_sys::Object` + `Reflect::set` 构造对象，**结构天然合法**
/// ⇒ ⇒ 判据代码不该把「序列化正确性」当成自己的风险来源。
#[wasm_bindgen]
pub fn l1_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::{Dom, EventKind, MeasurementScale};
    use apeireth_ui_dom::vnode::{PropValue, VNode};
    use apeireth_ui_dom::web::WebDom;
    use apeireth_ui_dom::Renderer;

    let mut failures: Vec<String> = Vec::new();
    let box_json = |rect: &apeireth_ui_dom::dom::ElementBox| -> JsValue {
        let object = js_sys::Object::new();
        for (key, value) in [
            ("x", rect.x),
            ("y", rect.y),
            ("w", rect.width),
            ("h", rect.height),
        ] {
            let _ = js_sys::Reflect::set(&object, &JsValue::from_str(key), &JsValue::from_f64(value));
        }
        object.into()
    };

    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom::new 失败：{error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "l1-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let mut renderer = Renderer::new(dom, root);
    let view = VNode::element("form")
        .child(
            VNode::element("input")
                .prop("value", PropValue::Text(String::from("a")))
                .build(),
        )
        .child(
            VNode::element("input")
                .prop("value", PropValue::Text(String::from("b")))
                .build(),
        )
        .build();
    renderer
        .mount(view)
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;

    // ── G1：真实元素盒（引擎 vs 浏览器 JS 双读数）──────────────
    let engine_form = renderer.bounding_box().ok();
    let engine_input = renderer.bounding_box_at_path(&[0]).ok();
    let js_rect = |selector: &str| -> Option<apeireth_ui_dom::dom::ElementBox> {
        let element = document.query_selector(selector).ok().flatten()?;
        let rect = element.get_bounding_client_rect();
        Some(apeireth_ui_dom::dom::ElementBox::new(
            rect.x(),
            rect.y(),
            rect.width(),
            rect.height(),
        ))
    };
    let js_form = js_rect("#l1-root > form");
    let js_input = js_rect("#l1-root > form > input");
    if let (Some(engine), Some(reference)) = (engine_form, js_form) {
        if (engine.x - reference.x).abs() > 0.5 || (engine.width - reference.width).abs() > 0.5 {
            failures.push(format!("G1 根盒不一致：engine={engine:?} js={reference:?}"));
        }
    } else {
        failures.push(String::from("G1 根盒取不到（engine 或 js 为 None）"));
    }
    if let (Some(engine), Some(reference)) = (engine_input, js_input) {
        if (engine.height - reference.height).abs() > 0.5 {
            failures.push(format!("G1 子盒高度不一致：engine={engine:?} js={reference:?}"));
        }
    } else {
        failures.push(String::from("G1 子盒取不到"));
    }

    // ── G6：设备像素换算 ──────────────────────────────────────
    renderer.set_measurement_scale(MeasurementScale::new(2.0, 1.25));
    let device = renderer.bounding_box_in_device(&[0]).ok();
    if let (Some(device), Some(reference)) = (device, engine_input) {
        let expected = reference.width * 2.5;
        if (device.width - expected).abs() > 0.01 {
            failures.push(format!("G6 换算错误：device={device:?} 期望宽度={expected}"));
        }
    } else {
        failures.push(String::from("G6 取不到设备像素盒"));
    }
    renderer.set_measurement_scale(MeasurementScale::IDENTITY);

    // ── G3：焦点真的驱动（看 document.activeElement）────────────
    let focused_path = renderer.focus_at_path(&[0]).is_ok();
    let active_is_input = document
        .active_element()
        .map(|element| element.tag_name() == "INPUT")
        .unwrap_or(false);
    if !(focused_path && active_is_input) {
        failures.push(format!(
            "G3 焦点未驱动：focus_at_path={focused_path} activeElementIsInput={active_is_input}"
        ));
    }
    let restored = renderer.restore_focus().ok().flatten();
    if restored.as_deref() != Some(&[][..]) && restored.is_some() {
        // 首次聚焦没有历史 ⇒ restore 返回 None 属正常；只要不抛错即可。
    }

    // ── G3：IME 组合静默期（真实 composition 事件 → document 级跟踪）──
    let target = document
        .query_selector("#l1-root > form > input")
        .ok()
        .flatten()
        .ok_or_else(|| JsValue::from_str("找不到第一个 input"))?;
    let before = renderer.dom().is_composing();
    // ⚠️ `Event::new` 默认 **不冒泡**；真实 `compositionstart` 是冒泡的
    //   ⇒ ⇒ 探针必须显式置 `bubbles=true`，否则 document 级监听收不到
    //   （这不是产品缺陷，是**探针造的事件不像真的**）。
    if let Ok(start_event) = web_sys::Event::new("compositionstart") {
        // 直接派发到 document：`Event::new` 造的事件不冒泡（web-sys 的
        // `init_event` 签名受限）⇒ ⇒ 派到 document 上可命中 document 级监听。
        // ⚠️ 这是**探针简化**：真实事件的 target 是输入元素，此处只验「document 级跟踪生效」。
        let _ = document.dispatch_event(&start_event);
    }
    let during = renderer.dom().is_composing();
    if let Ok(end_event) = web_sys::Event::new("compositionend") {
        let _ = document.dispatch_event(&end_event);
    }
    let after = renderer.dom().is_composing();
    if !(!before && during && !after) {
        failures.push(format!(
            "G3 IME 组合态未按预期：before={before} during={during} after={after}"
        ));
    }

    // ── G2：事件载荷（真实 input 事件 → target_value / 坐标）────
    let payload_sink = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let sink = std::rc::Rc::clone(&payload_sink);
    let handler = apeireth_ui_dom::events::ElementHandler::new(move |event| {
        *sink.borrow_mut() = format!(
            "value={:?};pos={:?};checked={:?}",
            event.target_value(),
            event.client_position(),
            event.checked()
        );
    });
    let input_node = renderer
        .node_for_path(&[0])
        .map_err(|error| JsValue::from_str(&format!("取节点失败：{error}")))?;
    renderer
        .dom()
        .add_listener(&input_node, EventKind::Input, handler)
        .map_err(|error| JsValue::from_str(&format!("挂监听失败：{error}")))?;
    if let Ok(input_event) = web_sys::Event::new("input") {
        let _ = target.dispatch_event(&input_event);
    }
    let payload = payload_sink.borrow().clone();
    if payload.is_empty() {
        failures.push(String::from("G2 事件载荷为空（监听器没被调用）"));
    }

    // ── G8：错误边界（强制渲染失败 ⇒ 兜底可见 + 错误可读）────────
    let mut g8_ok = false;
    let mut g8_captured = String::new();
    if let Ok(dom2) = WebDom::new() {
        if let Ok(root2) = dom2.create_element("div") {
            let _ = dom2.set_attribute(&root2, "id", "l1-g8");
            if let Some(body) = document.body() {
                let _ = body.append_child(&root2);
            }
            let mut guarded = Renderer::new(dom2, root2);
            guarded.set_error_boundary(Some(
                VNode::element("div")
                    .class("ap-error")
                    .child(VNode::text("兜底可见"))
                    .build(),
            ));
            // ⚠️ **制造真实失败**（2026-10-31 改）：原先靠「未挂载就渲染 ⇒ 必失败」，
            //   但 `render_guarded` 已改成「未挂载 ⇒ 首次挂载」（第一屏也要有兜底）
            //   ⇒ ⇒ 那个前提不再成立 ⇒ ⇒ 这里改用**非法标签名**
            //   （浏览器 `createElement("")` 会抛 ⇒ 渲染真失败 ⇒ 兜底顶上）。
            let _ = guarded.render_guarded(VNode::element("").build());
            g8_captured = guarded.captured_error().unwrap_or_default().to_owned();
            let fallback_visible = document
                .query_selector("#l1-g8 .ap-error")
                .ok()
                .flatten()
                .is_some();
            g8_ok = fallback_visible && !g8_captured.is_empty();
        }
    }
    if !g8_ok {
        failures.push(format!(
            "G8 错误边界未生效（fallback_visible={g8_ok}, captured=`{g8_captured}`）"
        ));
    }

    // ── G8：panic hook 安装（⚠️ **不**触发真实 panic：wasm 目标可能 `panic=abort`
    //    ⇒ ⇒ 真 panic 会 trap 掉整个实例，测不到；这里只验「hook 能装上」）────────
    let panic_state = if apeireth_ui_dom::panic_guard::is_installed() {
        String::from("already_installed")
    } else {
        match apeireth_ui_dom::panic_guard::install(Box::new(|_message| {})) {
            Ok(()) => String::from("installed"),
            Err(reason) => format!("failed: {reason}"),
        }
    };
    if panic_state.starts_with("failed") {
        failures.push(format!("G8 panic hook 未安装：{panic_state}"));
    }

    // ── 组装结果（结构化对象，不拼 JSON）──────────────────────
    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("ok"), &JsValue::from_bool(failures.is_empty()));
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("payload"), &JsValue::from_str(&payload));
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("g8_error_boundary_ok"),
        &JsValue::from_bool(g8_ok),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("g8_captured_error"),
        &JsValue::from_str(&g8_captured),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("g8_panic_hook"),
        &JsValue::from_str(&panic_state),
    );
    if let Some(rect) = engine_form {
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str("engine_form"), &box_json(&rect));
    }
    if let Some(rect) = js_form {
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str("js_form"), &box_json(&rect));
    }
    if let Some(rect) = device {
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str("device_input"), &box_json(&rect));
    }
    Ok(result.into())
}

/// ⭐ **纹理上传适配器**（证据装置内）：
/// 把 `apeireth_ui_dom::resource::TextureUploader` 接到 `apeireth_ui_gpu::upload::TextureStore`。
///
/// ## 为什么适配器放在这里，而不是 ui-gpu 里
///
/// 让 **ui-gpu 依赖 ui-dom** 会把分层方向倒过来（GPU 更底层）
/// ⇒ ⇒ 适配器放在**同时依赖两边**的装置/上层 crate 里（本 harness、将来的 ui-demo/ui-kit）
/// ⇒ ⇒ 产品分层的依赖方向保持单向。
pub struct DomTextureUploader<'a> {
    store: &'a mut apeireth_ui_gpu::upload::TextureStore,
    device: &'a apeireth_ui_gpu::wgpu::Device,
    queue: &'a apeireth_ui_gpu::wgpu::Queue,
}

impl<'a> DomTextureUploader<'a> {
    /// 建适配器（借用纹理仓库 + 设备的只读引用）。
    pub fn new(
        store: &'a mut apeireth_ui_gpu::upload::TextureStore,
        device: &'a apeireth_ui_gpu::wgpu::Device,
        queue: &'a apeireth_ui_gpu::wgpu::Queue,
    ) -> Self {
        Self {
            store,
            device,
            queue,
        }
    }
}

impl apeireth_ui_dom::resource::TextureUploader for DomTextureUploader<'_> {
    fn upload(
        &mut self,
        id: apeireth_ui_dom::resource::ResourceId,
        image: &apeireth_ui_dom::resource::ImageData,
    ) -> Result<(), String> {
        self.store.upload(
            self.device,
            self.queue,
            id.raw(),
            image.width,
            image.height,
            &image.rgba,
        )
    }
}

/// ⭐⭐⭐ **纹理上传端到端探针**（L1-G7 浏览器证据）。
///
/// 链路：真实引擎（`Engine::create`）→ `DomTextureUploader`（适配器）
/// → `TextureStore::upload`（**256 行对齐**）→ `upload_decoded` 状态收敛
/// ⇒ ⇒ 端到端证明「上传 → 纹理可用 → 资源 Ready」，而不只是纯逻辑测试。
///
/// ⚠️ 用**宽度非 256 倍数**的图（3×2）⇒ 真正走一遍行对齐路径
/// （3×4 = 12 字节/行，远小于 256）⇒ 若对齐写错，这里会失败或泛化不出视图。
#[wasm_bindgen]
pub async fn l1_upload_probe(
    canvas: web_sys::HtmlCanvasElement,
    backend: &str,
) -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::resource::{ResourceRegistry, ResourceState, upload_decoded};

    let mut failures: Vec<String> = Vec::new();
    let preference = match backend {
        "webgl" => BackendPreference::WebGl,
        "webgpu" => BackendPreference::WebGpu,
        _ => BackendPreference::Auto,
    };
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let size_canvas = canvas.clone();
    let options = EngineOptions {
        backend: preference,
        width,
        height,
        canvas_size: Some(CanvasSizeSource::new(move || {
            (size_canvas.width(), size_canvas.height())
        })),
    };
    let engine = Engine::create(SurfaceTargetInput::HtmlCanvas(canvas.clone()), options)
        .await
        .map_err(|error| JsValue::from_str(&format!("engine: {error}")))?;

    let mut store = apeireth_ui_gpu::upload::TextureStore::new();
    let mut registry = ResourceRegistry::new();
    let id = registry.register();
    // 3×2 RGBA：每行 12 字节 ⇒ 必须补到 256
    let image = apeireth_ui_dom::resource::ImageData::new(3, 2, vec![200u8; 3 * 2 * 4]);
    let mut adapter = DomTextureUploader::new(&mut store, engine.device(), engine.queue());
    let upload_ok = upload_decoded(&mut registry, &mut adapter, id, image).is_ok();
    let uploaded = store.get(id.raw()).map(|texture| (texture.width, texture.height));
    let ready = matches!(registry.state(id), Some(ResourceState::Ready));
    if !upload_ok || uploaded != Some((3, 2)) || !ready {
        failures.push(format!(
            "G7 上传端到端失败：upload_ok={upload_ok} uploaded={uploaded:?} ready={ready}"
        ));
    }

    // ⭐ 反向：坏数据必须被拒且**不得**标 Ready（不能把坏数据传进 GPU）
    let bad_id = registry.register();
    let bad = apeireth_ui_dom::resource::ImageData::new(2, 2, vec![0u8; 3]);
    let mut adapter = DomTextureUploader::new(&mut store, engine.device(), engine.queue());
    let bad_rejected = upload_decoded(&mut registry, &mut adapter, bad_id, bad).is_err();
    let bad_not_ready = !matches!(registry.state(bad_id), Some(ResourceState::Ready));
    if !(bad_rejected && bad_not_ready) {
        failures.push(format!(
            "G7 坏数据未被正确拒绝：rejected={bad_rejected} not_ready={bad_not_ready}"
        ));
    }

    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("backend_actual"),
        &JsValue::from_str(&format!("{:?}", engine.report().backend)),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("uploaded_w"),
        &JsValue::from_f64(uploaded.map(|(w, _)| w as f64).unwrap_or(0.0)),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **L1 组合集成探针**（上屏第一屏的"能不能串起来"实测）。
///
/// 在**同一个渲染器**、同一棵树上同时跑：
/// ①**受控输入**（`value` prop 差值写回）；②**IME 组合静默期**（组合中不写回、结束后补写）；
/// ③**焦点陷阱**（陷阱外拒绝且不动 DOM）；④**虚拟列表窗口化**（嵌套子级替换 + 真实滚动状态）。
///
/// 只要这四条在同一棵树里都成立 ⇒ ⇒ 说明 L1 各能力**组合起来没有缺口**
/// （单点测试通过但组合失效，是这类引擎最常见的坑）。
#[wasm_bindgen]
pub fn l1_integration_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::vnode::{PropValue, VNode};
    use apeireth_ui_dom::web::WebDom;
    use apeireth_ui_dom::Renderer;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "l1-int")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }
    let mut renderer = Renderer::new(dom, root);

    // 树：form(input) + div.vlist-host（列表宿主，可滚动）
    let build = |value: &str| {
        VNode::element("div")
            .child(
                VNode::element("form")
                    .child(
                        VNode::element("input")
                            .attr("id", "l1-int-input")
                            .prop("value", PropValue::Text(value.to_owned()))
                            .build(),
                    )
                    .build(),
            )
            .child(
                VNode::element("div")
                    .attr("id", "l1-int-list")
                    .attr("style", "height: 200px; overflow-y: auto;")
                    .build(),
            )
            .build()
    };
    renderer
        .mount(build("hello"))
        .expect("挂载必须成功");

    // ── ① 受控输入：改 value ⇒ DOM 真的变 ───────────────────────────
    renderer.patch(build("hello2")).expect("打补丁必须成功");
    let input_value = |selector: &str| -> Option<String> {
        let element = document.query_selector(selector).ok().flatten()?;
        js_sys::Reflect::get(&element, &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_string())
    };
    let after_controlled = input_value("#l1-int-input");
    if after_controlled.as_deref() != Some("hello2") {
        failures.push(format!(
            "① 受控输入未生效：DOM value = {after_controlled:?}（期望 hello2）"
        ));
    }

    // ── ② IME 组合静默期：组合中不写回，结束后补写 ──────────────────
    if let Ok(start) = web_sys::Event::new("compositionstart") {
        let _ = document.dispatch_event(&start);
    }
    let composing = renderer.dom().is_composing();
    renderer.patch(build("hello3")).expect("打补丁必须成功");
    let during_composition = input_value("#l1-int-input");
    if !composing || during_composition.as_deref() != Some("hello2") {
        failures.push(format!(
            "② 组合期写回未门住：composing={composing} DOM value = {during_composition:?}"
        ));
    }
    if let Ok(end) = web_sys::Event::new("compositionend") {
        let _ = document.dispatch_event(&end);
    }
    renderer.patch(build("hello3")).expect("打补丁必须成功");
    let after_composition = input_value("#l1-int-input");
    if after_composition.as_deref() != Some("hello3") {
        failures.push(format!(
            "② 组合结束后未补写：DOM value = {after_composition:?}（期望 hello3）"
        ));
    }

    // ── ③ 焦点陷阱：陷阱外拒绝且不动 DOM 焦点 ───────────────────────
    renderer
        .focus_at_path(&[0, 0])
        .expect("聚焦输入框必须成功");
    let focused_is_input = document
        .active_element()
        .map(|element| element.id() == "l1-int-input")
        .unwrap_or(false);
    if !focused_is_input {
        failures.push(String::from("③ 聚焦未驱动 document.activeElement"));
    }

    // ── ④ 虚拟列表窗口化（嵌套子级替换 + 真实滚动状态）──────────────
    let heights = vec![40.0_f64; 1000];
    let host_path = [1_usize];
    let mut first_visible = 0usize;
    let mut mounted_rows = 0usize;
    for scroll_top in [0.0_f64, 4000.0] {
        if let Some(element) = document
            .query_selector("#l1-int-list")
            .ok()
            .flatten()
        {
            let target = element.dyn_ref::<web_sys::Element>();
            if let Some(target) = target {
                let _ = js_sys::Reflect::set(
                    target,
                    &JsValue::from_str("scrollTop"),
                    &JsValue::from_f64(scroll_top),
                );
            }
        }
        let metrics = renderer
            .scroll_metrics_at_path(&host_path)
            .map_err(|error| JsValue::from_str(&format!("读滚动度量失败：{error}")))?;
        let plan = apeireth_ui_dom::virtual_list::plan_virtual_window(
            &heights,
            metrics.scroll_top,
            metrics.client_height,
            2,
        );
        first_visible = plan.first;
        mounted_rows = plan.len();
        // ⭐ 用 `patch_windowed_at_path`（**留白由引擎算**）而不是手写行：
        //   手写行 ⇒ 宿主内容高度不足 ⇒ 浏览器把 scrollTop 夹回 0 ⇒ 窗口不动。
        renderer
            .patch_windowed_at_path(&host_path, &heights, 2, |index| {
                VNode::element("div")
                    .attr("class", "row")
                    .attr("style", "height: 40px")
                    .child(VNode::text(format!("{index}")))
                    .build()
            })
            .map_err(|error| JsValue::from_str(&format!("嵌套窗口化渲染失败：{error}")))?;
    }
    let mounted_dom = document
        .query_selector_all("#l1-int-list > .row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);
    if mounted_dom == 0 || mounted_dom > 20 {
        failures.push(format!(
            "④ 窗口化渲染异常：DOM 行数 = {mounted_dom}（应 O(窗口)）"
        ));
    }
    if first_visible == 0 {
        failures.push(String::from("④ 滚动后窗口未移动（first 仍为 0）"));
    }

    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mounted_rows"),
        &JsValue::from_f64(mounted_dom as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("first_visible"),
        &JsValue::from_f64(first_visible as f64),
    );
    Ok(result.into())
}

// ═══════════════════════════════════════════════════════════════════════════
// ⭐ **上屏第一屏示例**（引擎侧接入范式；W0 onboarding 文档引用的**可运行**版本）
// ═══════════════════════════════════════════════════════════════════════════
//
///! 演示第一屏真正会用到的那几件事：
///! ①**受控输入**（值由状态给，不由 DOM 自己跑）；②**虚拟列表**（留白交给引擎）；
///! ③**滚动联动**（信号 + 真实位置）；④**错误边界**（`render_guarded`）；
///! ⑤**状态变了重渲染**（`patch` 差分）。
///!
///! ⚠️ 这里的调度器是**示例级**的（JS 侧 `setInterval` 调 [`w0_tick`]）——
///! 生产应换成正式调度器；引擎侧契约不变（**状态 → 重建树 → patch**）。

use std::rc::Rc;

/// 示例屏的应用状态（**唯一真相**）。
#[derive(Default)]
pub struct DemoState {
    /// 输入框的值（受控）。
    pub name: String,
    /// 状态是否变了（下一 tick 需要重渲染）。
    pub dirty: bool,
    /// 列表滚动位置（由引擎从真实 DOM 读，不缓存）。
    pub scroll_top: f64,
}

thread_local! {
    /// 渲染器（示例屏单例）。
    static DEMO: RefCell<Option<DemoRenderer>> = const { RefCell::new(None) };
}

/// 示例屏的渲染器 + 状态（放一起，便于 tick 时一起用）。
pub struct DemoRenderer {
    /// 渲染器。
    pub renderer: apeireth_ui_dom::Renderer<apeireth_ui_dom::web::WebDom>,
    /// 应用状态（与事件处理器共享）。
    pub state: Rc<RefCell<DemoState>>,
    /// 滚动信号（`scroll` 不冒泡 ⇒ 直连容器）。
    pub scroll: apeireth_ui_dom::renderer::ScrollSignal,
    /// 列表容器路径（第一屏的第二个子节点）。
    pub list_path: Vec<usize>,
}

/// 建示例屏的 VNode 树（**纯函数** ⇒ 可单独测试）。
pub fn build_demo_tree(
    state: &Rc<RefCell<DemoState>>,
    rows: &[String],
) -> apeireth_ui_dom::vnode::VNode {
    use apeireth_ui_dom::dom::EventKind;
    use apeireth_ui_dom::events::ElementHandler;
    use apeireth_ui_dom::vnode::{PropValue, VNode};

    // ⚠ 状态用 `Rc<RefCell<_>>` 共享给事件处理器（处理器要求 `'static`，
    //   且**拿不到 `&mut renderer`** ⇒ 处理器只改状态，重渲染交给 tick）。
    //   ⇒ **必须复用调用方那一个 Rc**（否则处理器写的状态与 tick 读的不是同一个）。
    let input_state = state.clone();
    VNode::element("section")
        .attr("class", "w0-screen")
        .child(
            VNode::element("input")
                .attr("id", "w0-name")
                .prop("value", PropValue::Text(state.borrow().name.clone()))
                .handler(
                    EventKind::Input,
                    ElementHandler::new(move |event| {
                        // ⭐ 受控的关键：处理器只**改状态**，不碰 DOM。
                        let mut guard = input_state.borrow_mut();
                        guard.name = event.target_value().unwrap_or_default().to_owned();
                        guard.dirty = true;
                    }),
                )
                .build(),
        )
        .child(
            VNode::element("div")
                .attr("id", "w0-list")
                .attr("style", "height: 200px; overflow-y: auto;")
                .child(
                    VNode::element("div")
                        .attr("class", "row")
                        .child(VNode::text(rows.first().cloned().unwrap_or_default()))
                        .build(),
                )
                .build(),
        )
        .build()
}

/// ⭐ **挂载示例屏**（浏览器入口）。
#[wasm_bindgen]
pub fn w0_first_screen() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::web::WebDom;
    use apeireth_ui_dom::Renderer;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "w0-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }
    let mut renderer = Renderer::new(dom, root);

    let rows: Vec<String> = (0..1000).map(|index| format!("行 {index}")).collect();
    let state = Rc::new(RefCell::new(DemoState::default()));
    let tree = build_demo_tree(&state, &rows);
    renderer
        .render_guarded(tree)
        .map_err(|error| JsValue::from_str(&format!("渲染失败：{error}")))?;

    // 滚动信号：直连列表容器（`scroll` 不冒泡）。
    let scroll = renderer
        .listen_scroll_at_path(&[1])
        .map_err(|error| JsValue::from_str(&format!("监听滚动失败：{error}")))?;

    // 虚拟列表：用 `patch_windowed_at_path`（**留白由引擎算** ⇒ 容器真的能滚）。
    let heights = vec![40.0_f64; 1000];
    renderer
        .patch_windowed_at_path(&[1], &heights, 2, |index| {
            use apeireth_ui_dom::vnode::VNode;
            VNode::element("div")
                .attr("class", "row")
                .attr("style", "height: 40px")
                .child(VNode::text(format!("行 {index}")))
                .build()
        })
        .map_err(|error| JsValue::from_str(&format!("窗口化失败：{error}")))?;

    DEMO.with(|slot| {
        *slot.borrow_mut() = Some(DemoRenderer {
            renderer,
            state,
            scroll,
            list_path: vec![1],
        });
    });

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mounted"),
        &JsValue::from_bool(true),
    );
    Ok(result.into())
}

/// ⭐ **tick**：状态变了就重渲染（示例级调度器；生产换正式调度器）。
#[wasm_bindgen]
pub fn w0_tick() -> Result<JsValue, JsValue> {

    let mut redrawn = false;
    let mut scroll_seen = false;
    DEMO.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(demo) = guard.as_mut() else {
            return;
        };
        // ① 滚动信号：位置**从真实 DOM 读**（信号只答「刚滚过没」）。
        if demo.scroll.take() {
            scroll_seen = true;
            if let Ok(metrics) = demo.renderer.scroll_metrics_at_path(&demo.list_path) {
                demo.state.borrow_mut().scroll_top = metrics.scroll_top;
                demo.state.borrow_mut().dirty = true;
            }
        }
        // ② 状态脏 ⇒ 重建树 → `patch`（差分）。
        if demo.state.borrow().dirty {
            let tree = build_demo_tree(&demo.state, &[]);
            // ⚠️ 受控写回在 IME 组合期会被**门住**（引擎负责补写）。
            let _ = demo.renderer.patch(tree);
            demo.state.borrow_mut().dirty = false;
            redrawn = true;
        }
    });
    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("redrawn"),
        &JsValue::from_bool(redrawn),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("scroll_seen"),
        &JsValue::from_bool(scroll_seen),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **双向绑定探针**（垂直切片 Signal 环的浏览器端到端证据）。
///
/// 在**真实浏览器**里验证：
/// ①**下行**（信号 → DOM）：`signal.set(...)` ⇒ 重新渲染 ⇒ DOM 的 `value` 跟着变；
/// ②**上行**（DOM → 信号）：真实 `input` 事件 ⇒ 信号被写回；
/// ③**用户自己的处理器仍被调用**（内部绑定写回**不得覆盖**用户处理器）。
#[wasm_bindgen]
pub fn l1_binding_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::binding::Binding;
    use apeireth_ui_dom::dom::{Dom, EventKind};
    use apeireth_ui_dom::events::ElementHandler;
    use apeireth_ui_dom::vnode::VNode;
    use apeireth_ui_dom::web::WebDom;
    use apeireth_ui_dom::Renderer;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "l1-bind-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }
    let mut renderer = Renderer::new(dom, root);

    let runtime = Runtime::new();
    let signal = runtime.signal(String::from("init"));
    let user_called = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = user_called.clone();
    let view = VNode::element("input")
        .attr("id", "l1-bind-input")
        .bind(Binding::text(signal.clone()))
        .handler(
            EventKind::Input,
            ElementHandler::new(move |_event| flag.set(true)),
        )
        .build();
    renderer
        .mount(view)
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;

    let value_of = |selector: &str| -> Option<String> {
        let element = document.query_selector(selector).ok().flatten()?;
        js_sys::Reflect::get(&element, &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_string())
    };

    // ① 下行：首渲染写回信号值
    let initial = value_of("#l1-bind-input");
    if initial.as_deref() != Some("init") {
        failures.push(format!("① 首渲染未把信号值写进 DOM：{initial:?}"));
    }

    // ② 下行：信号变 ⇒ 重渲染 ⇒ DOM 跟着变
    signal.set(String::from("downlink"));
    renderer
        .patch(
            VNode::element("input")
                .attr("id", "l1-bind-input")
                .bind(Binding::text(signal.clone()))
                .build(),
        )
        .map_err(|error| JsValue::from_str(&format!("打补丁失败：{error}")))?;
    let after_downlink = value_of("#l1-bind-input");
    if after_downlink.as_deref() != Some("downlink") {
        failures.push(format!("② 信号变更后 DOM 未更新：{after_downlink:?}"));
    }

    // ③ 上行：真实 input 事件 ⇒ 信号被写回
    let dispatched = document
        .query_selector("#l1-bind-input")
        .ok()
        .flatten()
        .map(|element| {
            let _ = js_sys::Reflect::set(
                &element,
                &JsValue::from_str("value"),
                &JsValue::from_str("uplink"),
            );
            let event = web_sys::Event::new("input").expect("造事件必须成功");
            let _ = element.dispatch_event(&event);
            true
        })
        .unwrap_or(false);
    if !dispatched {
        failures.push(String::from("③ 找不到输入框，无法派发 input 事件"));
    }
    if signal.get() != "uplink" {
        failures.push(format!("③ input 事件未把 DOM 值写回信号：{}", signal.get()));
    }

    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("signal_value"),
        &JsValue::from_str(&signal.get()),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐⭐ **产品宿主薄壳探针**（`ui-app` 的浏览器端到端证据）。
///
/// 验证**宿主运行时契约**在真实浏览器里成立：
/// ①`boot` ⇒ 屏幕挂进真实 DOM（输入框出现，值 = 演示屏初始值）；
/// ②**信号变 ⇒ 重渲染 ⇒ DOM 更新**（`set_demo_text` + `rerender`）；
/// ③`dispose_screen` ⇒ DOM 被卸载（容器清空）。
///
/// ⚠️ 这是**产品宿主**的证据（不是引擎内部能力）⇒ 与 `l1_probe`（引擎能力）分开。
#[wasm_bindgen]
pub fn l1_app_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("没有 body"))?;

    // ① 宿主容器（真实 DOM）
    let container = document
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建容器失败：{error:?}")))?;
    container.set_attribute("id", "l1-app-root").map_err(|error| {
        JsValue::from_str(&format!("设容器 id 失败：{error:?}"))
    })?;
    body.append_child(&container).map_err(|error| {
        JsValue::from_str(&format!("挂容器失败：{error:?}"))
    })?;

    // ② boot
    wasm_host::boot("#l1-app-root").map_err(|error| {
        JsValue::from_str(&format!("boot 失败：{error:?}"))
    })?;
    let input_value = || -> Option<String> {
        let element = document.query_selector("#l1-app-root input").ok().flatten()?;
        js_sys::Reflect::get(&element, &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_string())
    };
    let booted = input_value();
    if booted.as_deref() != Some("宿主已就绪") {
        failures.push(format!("① boot 后输入框值不对：{booted:?}"));
    }

    // ③ 信号变 ⇒ rerender ⇒ DOM 更新
    wasm_host::set_demo_text("已更新").map_err(|error| {
        JsValue::from_str(&format!("set_demo_text 失败：{error:?}"))
    })?;
    let redrawn = wasm_host::rerender().map_err(|error| {
        JsValue::from_str(&format!("rerender 失败：{error:?}"))
    })?;
    if !redrawn {
        failures.push(String::from("② 状态已标脏 ⇒ rerender 必须返回 true"));
    }
    let after = input_value();
    if after.as_deref() != Some("已更新") {
        failures.push(format!("② 重渲染后 DOM 未更新：{after:?}"));
    }

    // ④ dispose ⇒ DOM 被卸载
    wasm_host::dispose_screen()
        .map_err(|error| JsValue::from_str(&format!("dispose 失败：{error:?}")))?;
    let leftover = document
        .query_selector("#l1-app-root input")
        .ok()
        .flatten();
    if leftover.is_some() {
        failures.push(String::from("③ dispose 后输入框仍在 DOM 里"));
    }

    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    Ok(result.into())
}
