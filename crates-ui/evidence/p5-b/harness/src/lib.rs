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

// ⭐⭐⭐ **本 crate 是 wasm 专用**（E-168）。
//
// ## 为什么
//
// 它依赖的两个模块都是 **wasm32 专属**：
// - `apeireth_ui_dom::web`（真实 DOM 后端，`web-sys`）
// - `apeireth_ui_app::wasm_host`（wasm 薄壳）
//
// ⇒ ⇒ **native 目标下这两个模块根本不参与编译**
// ⇒ ⇒ ⇒ 若不门控，`cargo clippy`（native）会报**一堆 `unresolved import`**
// ⇒ ⇒ ⇒ 看起来像「32 个 clippy 错误」，实际是「**这个 crate 不能编到 native**」。
//
// ⇒ ⇒ 这里**显式门控**：native 目标下只留一条**说人话**的说明
// ⇒ ⇒ ⇒ 免得下一个人以为欠了一屁股 clippy 债（E-167 时确实这么误会过）。
//
// ✅ **验证目标只有 wasm32** ⇒ ⇒ ⇒ 门禁跑的是
// `cargo clippy --target wasm32-unknown-unknown`（**0 错误**）。
#![cfg(target_arch = "wasm32")]

use apeireth_ui_gpu::{
    BackendPreference, BlendMode, CanvasSizeSource, Engine, EngineOptions, SceneDescriptor,
    ScenePass, ScenePipeline, SurfaceTargetInput, Topology, UniformValues, VertexFormat,
    INTRO_BACKDROP_FRAGMENT_GLSL, INTRO_BACKDROP_UNIFORM_NAMES, INTRO_BACKDROP_VERTEX_GLSL,
    INTRO_SPRITE_COORD_FRAGMENT_GLSL, INTRO_SPRITE_FLAT_FRAGMENT_GLSL, INTRO_SPRITE_FRAGMENT_GLSL,
    INTRO_SPRITE_PROBE_FRAGMENT_GLSL, INTRO_SPRITE_UNIFORM_NAMES, INTRO_SPRITE_VERTEX_GLSL,
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
    // ⚠️ 该字段当前只**记录**（供将来查「这次跑的是哪些 pass」）
    //   ⇒ ⇒ 暂时没有读取方 ⇒ ⇒ 显式标注，避免被当成「漏用」。
    #[allow(dead_code)]
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
                    // ⚠️ **不用通配分支**：`DeviceEvent` 只有 `Error` / `Lost`
                    //   两个变体（`apeireth-ui-gpu/src/engine.rs`，**无 cfg 区分**）
                    //   ⇒ ⇒ 通配分支**不可达** ⇒ ⇒ 写出来会被 clippy 判 `unreachable_patterns`
                    //   ⇒ ⇒ 这里**穷尽匹配**：将来**新增变体**时，这里会**编译报错**
                    //   ⇒ ⇒ ⇒ **强制**处理新事件（比静默落到 `other` 更安全）。
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
        let scene = SceneDescriptor::new(INTRO_BACKDROP_VERTEX_GLSL, INTRO_BACKDROP_FRAGMENT_GLSL)
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
            SpriteTopology::InstancedQuad => {
                SceneDescriptor::new(INTRO_SPRITE_VERTEX_GLSL, sprite_fragment)
                    .with_instance_data(seeds)
                    .with_instance_format(VertexFormat::Vec4)
                    .with_topology(Topology::InstancedQuad)
                    .with_blend(BlendMode::Additive)
            }
            // 点图元路径（WebGL 快路径）：粒子属性走**顶点缓冲**。
            SpriteTopology::PointList => {
                SceneDescriptor::new(INTRO_SPRITE_VERTEX_GLSL, sprite_fragment)
                    .with_vertex_data(seeds)
                    .with_vertex_format(VertexFormat::Vec4)
                    .with_topology(Topology::PointList)
                    .with_blend(BlendMode::Additive)
            }
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
                out.push_str(&format!(
                    "\"{name}\":{}",
                    format_f64_components(&components)
                ));
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
            out.push_str(&format!(
                "\"{name}\":{}",
                format_f64_components(&components)
            ));
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
            let _ =
                js_sys::Reflect::set(&object, &JsValue::from_str(key), &JsValue::from_f64(value));
        }
        object.into()
    };

    let dom =
        WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom::new 失败：{error}")))?;
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
            failures.push(format!(
                "G1 子盒高度不一致：engine={engine:?} js={reference:?}"
            ));
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
            failures.push(format!(
                "G6 换算错误：device={device:?} 期望宽度={expected}"
            ));
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
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("payload"),
        &JsValue::from_str(&payload),
    );
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
        let _ = js_sys::Reflect::set(
            &result,
            &JsValue::from_str("device_input"),
            &box_json(&rect),
        );
    }
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **CLS（布局位移）观测探针** —— L1-G7 的**最后一段浏览器证据**。
///
/// ## 它证明什么
///
/// 引擎的 `LayoutShiftTracker` **吃的是真实 DOM 几何**（`WebDom::bounding_box`
/// ⇒ `getBoundingClientRect`），不是脚本化的假测量
/// ⇒ ⇒ 所以「用户真的看得见的跳动」能被引擎算出来，
/// 而「亚像素噪声」与「视口外的移动」**不会被记成缺陷**。
///
/// ## 三条对照（缺一不可）
///
/// | 场景 | 期望 | 为什么要这条 |
/// |---|---|---|
/// | **真实位移**（插入 100px 高的占位 ⇒ 被追踪元素下移 100px）| 得分 **> 0** | 证明**能测出来** |
/// | **亚像素位移**（下移 0.5px，低于 1px 阈值）| 得分 **= 0** | 证明**不把渲染噪声当缺陷**（E-54 那类 1 LSB 差异） |
/// | **视口外的元素**（放到 y=100000）| 得分 **= 0** | 证明**只计用户看得见的位移** |
///
/// ⚠️ 前两条是「阳性 / 阴性」成对；第三条防「把不可见区域也算进去」。
/// ⇒ ⇒ 只有三条**同时成立**，CLS 观测才算真的可用（否则就是**一个永远返回 0 的摆设**）。
#[wasm_bindgen]
pub fn l1_layout_shift_probe(container_id: &str) -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom as _;
    use apeireth_ui_dom::layout_shift::LayoutShiftTracker;
    use apeireth_ui_dom::renderer::Renderer;
    use apeireth_ui_dom::vnode::VNode;

    let mut failures: Vec<String> = Vec::new();

    let dom = apeireth_ui_dom::web::WebDom::new()
        .map_err(|error| JsValue::from_str(&format!("WebDom::new: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    let root_id = format!("{container_id}-cls-root");
    dom.set_attribute(&root, "id", &root_id)
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    // ⚠️⚠️ **必须钉在视口内**（2026-11-06 实测踩过）：探针页 body 上方已有大量内容
    //   ⇒ 直接 appendChild 会让被测元素落在 y≈1250
    //   ⇒ 而 CLS 得分算「并集 ∩ **视口**」⇒ 元素在视口外 ⇒ 影响占比恒 0
    //   ⇒ **得分恒 0**（看着像「观测坏了」，实则元素根本不在视口里）。
    dom.set_attribute(
        &root,
        "style",
        "position: fixed; top: 0; left: 0; z-index: -1;",
    )
    .map_err(|error| JsValue::from_str(&format!("设 style 失败：{error}")))?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("无 document"))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let mut renderer = apeireth_ui_dom::renderer::Renderer::new(dom, root);

    // ⭐⭐ 视口必须用**真实窗口尺寸**：得分 = 并集∩视口/视口面积 × 位移/长边
    //   ⇒ 视口填错 ⇒ 要么把可见位移算成 0（视口比实际小），要么被稀释到看不出。
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("无 window"))?;
    let vw = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let vh = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let (vw, vh) = if vw.is_finite() && vw > 0.0 && vh.is_finite() && vh > 0.0 {
        (vw, vh)
    } else {
        (800.0, 600.0)
    };
    let mut tracker = LayoutShiftTracker::new(vw, vh);

    // 树：div(列) -> [占位 div, 被追踪 span]
    let build = |spacer_height: &str, tracked_style: &str| {
        let spacer_style = format!("height: {spacer_height};");
        VNode::element("div")
            .child(
                VNode::element("div")
                    .attr("style", spacer_style.as_str())
                    .build(),
            )
            .child(
                VNode::element("span")
                    .attr("style", tracked_style)
                    .child(VNode::text("tracked"))
                    .build(),
            )
            .build()
    };

    // ① 挂载：占位 0px ⇒ 被追踪 span 在顶部
    renderer
        .mount(build("0px", "display:block; height:50px; width:100px;"))
        .map_err(|error| JsValue::from_str(&format!("mount: {error:?}")))?;
    let box_before = renderer
        .bounding_box_at_path(&[1])
        .map_err(|error| JsValue::from_str(&format!("bbox(baseline): {error:?}")))?;
    let baseline = renderer
        .observe_layout_shift_at_path(&mut tracker, 1, &[1])
        .map_err(|error| JsValue::from_str(&format!("observe(baseline): {error:?}")))?;
    // ② 占位变 100px ⇒ span **真的**下移 100px
    renderer
        .patch(build("100px", "display:block; height:50px; width:100px;"))
        .map_err(|error| JsValue::from_str(&format!("patch(shift): {error:?}")))?;
    let box_after = renderer
        .bounding_box_at_path(&[1])
        .map_err(|error| JsValue::from_str(&format!("bbox(shift): {error:?}")))?;
    let shifted = renderer
        .observe_layout_shift_at_path(&mut tracker, 1, &[1])
        .map_err(|error| JsValue::from_str(&format!("observe(shift): {error:?}")))?;

    if baseline != 0.0 {
        failures.push(format!("首次观测只应建基线，实际 {baseline}"));
    }
    if !(shifted > 0.0) {
        failures.push(format!(
            "真实下移 100px 却测得 {shifted} ⇒ ⇒ **CLS 观测没有吃真实几何**"
        ));
    }

    // ③ 反向对照 A：**亚像素**位移（0.5px < 1px 阈值）⇒ ⇒ 必须 0
    let before_subpixel = tracker.total();
    renderer
        .patch(build("100.5px", "display:block; height:50px; width:100px;"))
        .map_err(|error| JsValue::from_str(&format!("patch(subpixel): {error:?}")))?;
    let subpixel = renderer
        .observe_layout_shift_at_path(&mut tracker, 1, &[1])
        .map_err(|error| JsValue::from_str(&format!("observe(subpixel): {error:?}")))?;
    if subpixel != 0.0 {
        failures.push(format!(
            "亚像素位移不得计分（阈值 1px），实际 {subpixel}"
        ));
    }
    let subpixel_did_not_accumulate = (tracker.total() - before_subpixel).abs() < 1e-12;
    if !subpixel_did_not_accumulate {
        failures.push(String::from("亚像素位移被累加进了总分"));
    }

    // ④ 反向对照 B：**前后都在视口外** => => 必须 0
    //
    // ⚠️ 口径（写错会得到一个「永远通过」的假判据）：得分用
    //   **before ∪ after ∩ 视口** => => 从「可见位置」移到视口外时，**并集**仍含
    //   可见的那部分 => => 那是**正确计分**（用户确实看见它不见了）
    //   => => 所以负对照必须让**两次观测都在视口外**，才真正验证「视口外不计分」。
    let mut offscreen_tracker = LayoutShiftTracker::new(vw, vh);
    // 先把元素挪到很远（**新** tracker ⇒ 首观测只建基线、不计分）
    renderer
        .patch(build("100000px", "display:block; height:50px; width:100px;"))
        .map_err(|error| JsValue::from_str(&format!("patch(offscreen-base): {error:?}")))?;
    let offscreen_baseline = renderer
        .observe_layout_shift_at_path(&mut offscreen_tracker, 2, &[1])
        .map_err(|error| JsValue::from_str(&format!("observe(offscreen-base): {error:?}")))?;
    // 再挪 500px（仍在视口外）
    renderer
        .patch(build("100500px", "display:block; height:50px; width:100px;"))
        .map_err(|error| JsValue::from_str(&format!("patch(offscreen-move): {error:?}")))?;
    let offscreen = renderer
        .observe_layout_shift_at_path(&mut offscreen_tracker, 2, &[1])
        .map_err(|error| JsValue::from_str(&format!("observe(offscreen-move): {error:?}")))?;
    if offscreen_baseline != 0.0 {
        failures.push(format!("视口外基线应为 0，实际 {offscreen_baseline}"));
    }
    if offscreen != 0.0 {
        failures.push(format!("**两次都在视口外**的位移不得计分，实际 {offscreen}"));
    }
    if offscreen_tracker.total().abs() > 1e-12 {
        failures.push(String::from("视口外位移被累加进了总分"));
    }

    // ⑤ 观测次数必须真的增加（防「一次都没观测却报 0 = 看着正常」）
    let samples_ok = tracker.samples() >= 3 && offscreen_tracker.samples() >= 2;
    if !samples_ok {
        failures.push(format!(
            "观测次数不足（主 {} < 3 或 视口外 {} < 2）⇒ ⇒ 结论不可用",
            tracker.samples(),
            offscreen_tracker.samples()
        ));
    }

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("vw"), &JsValue::from_f64(vw));
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("vh"), &JsValue::from_f64(vh));
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
        &JsValue::from_str("baseline_score"),
        &JsValue::from_f64(baseline),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("shift_score"),
        &JsValue::from_f64(shifted),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("subpixel_score"),
        &JsValue::from_f64(subpixel),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("offscreen_score"),
        &JsValue::from_f64(offscreen),
    );
    let box_to_json = |b: apeireth_ui_dom::dom::ElementBox| {
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("x"), &JsValue::from_f64(b.x));
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("y"), &JsValue::from_f64(b.y));
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("w"), &JsValue::from_f64(b.width));
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str("h"), &JsValue::from_f64(b.height));
        o
    };
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("box_before"),
        &box_to_json(box_before),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("box_after"),
        &box_to_json(box_after),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("total"),
        &JsValue::from_f64(tracker.total()),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("samples"),
        &JsValue::from_f64(tracker.samples() as f64),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **纹理上传端到端探针**（L1-G7 浏览器证据）。
///
/// 链路：真实引擎（`Engine::create`）→ `GpuTextureUploader`（**引擎内**实现，无需适配器）
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
    use apeireth_ui_gpu::resource::{upload_decoded, ResourceRegistry, ResourceState};
    use apeireth_ui_gpu::upload::GpuTextureUploader;

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
    let image = apeireth_ui_gpu::resource::ImageData::new(3, 2, vec![200u8; 3 * 2 * 4]);
    let mut adapter = GpuTextureUploader::new(&mut store, engine.device(), engine.queue());
    let upload_ok = upload_decoded(&mut registry, &mut adapter, id, image).is_ok();
    let uploaded = store
        .get(id.raw())
        .map(|texture| (texture.width, texture.height));
    let ready = matches!(registry.state(id), Some(ResourceState::Ready));
    if !upload_ok || uploaded != Some((3, 2)) || !ready {
        failures.push(format!(
            "G7 上传端到端失败：upload_ok={upload_ok} uploaded={uploaded:?} ready={ready}"
        ));
    }

    // ⭐ 反向：坏数据必须被拒且**不得**标 Ready（不能把坏数据传进 GPU）
    let bad_id = registry.register();
    let bad = apeireth_ui_gpu::resource::ImageData::new(2, 2, vec![0u8; 3]);
    let mut adapter = GpuTextureUploader::new(&mut store, engine.device(), engine.queue());
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
    renderer.mount(build("hello")).expect("挂载必须成功");

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
    renderer.focus_at_path(&[0, 0]).expect("聚焦输入框必须成功");
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
        if let Some(element) = document.query_selector("#l1-int-list").ok().flatten() {
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
    // ⭐ 断言**引擎算出的窗口行数**与**DOM 实际行数**一致（此前该值只赋值、不校验）。
    if mounted_rows == 0 || mounted_rows > 20 {
        failures.push(format!(
            "④ 窗口行数异常：引擎算出 {mounted_rows} 行（应 O(窗口)，远小于 1000）"
        ));
    }
    if mounted_rows as u32 != mounted_dom {
        failures.push(format!(
            "④ 引擎窗口行数（{mounted_rows}）与 DOM 实际行数（{mounted_dom}）不一致"
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
// 演示第一屏真正会用到的那几件事：
// ①**受控输入**（值由状态给，不由 DOM 自己跑）；②**虚拟列表**（留白交给引擎）；
// ③**滚动联动**（信号 + 真实位置）；④**错误边界**（`render_guarded`）；
// ⑤**状态变了重渲染**（`patch` 差分）。
//
// ⚠️ 这里的调度器是**示例级**的（JS 侧 `setInterval` 调 `w0_tick`）——
// 生产应换成正式调度器；引擎侧契约不变（**状态 → 重建树 → patch**）。

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
    /// ⭐ 诊断用：`w0_first_screen` 里「挂监听后立刻派发 scroll」的 `peek()` 结果。
    static SCROLL_PROBE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
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
    /// 列表项总数（1000 行）。
    pub row_count: usize,
    /// ⭐ 列表**还没窗口化**（首屏挂载后等宿主在 paint 之后调 `w0_windowize`）。
    pub pending_windowize: bool,
}

/// 建示例屏的 VNode 树（**纯函数** ⇒ 可单独测试）。
pub fn build_demo_tree(
    state: &Rc<RefCell<DemoState>>,
    rows: &[String],
) -> apeireth_ui_dom::vnode::VNode {
    // ⚠️ `rows` **故意不用**：列表内容**完全由窗口化负责**（E-162 的根本修法）。
    //   保留参数是为了**不破坏调用点签名**（`w0_first_screen` / `w0_tick` 都传了它）
    //   ⇒ ⇒ 将来若要恢复占位行，调用点不用改。
    let _ = rows;
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
        .child(build_list_node())
        .build()
}

/// ⭐⭐⭐⭐ **列表容器**（E-165：内容**交给虚拟化**）。
///
/// ## 为什么树里**只有容器、没有行**
///
/// 引擎的「虚拟化接管」保证（E-165）：`patch_windowed_at_path` 会**自动**给该容器
/// 打上 `virtualized` 标记 ⇒ ⇒ ⇒ **整树 `patch` 会跳过它的子节点差分**
/// ⇒ ⇒ ⇒ 已挂载的窗口内容**原样保留** ⇒ ⇒ ⇒ 两件事一起消失：
/// ① 整树重建**清空**窗口 ⇒ ⇒ 滚动位置不再丢（E-164）；
/// ② 整树重建**覆盖**窗口 ⇒ ⇒ 虚拟列表不再塌陷（E-161）。
///
/// ⇒ ⇒ 所以树里**不需要**再带一份窗口内容 ⇒ ⇒ ⇒
/// ⇒ ⇒ ⇒ 「树与 state 同步窗口」那套机制（E-163 的补丁）**已删**。
///
/// ⚠️ 容器**必须**在树里（否则 `patch_windowed_at_path` 的路径找不到父节点）。
fn build_list_node() -> apeireth_ui_dom::vnode::VNode {
    use apeireth_ui_dom::vnode::VNode;
    VNode::element("div")
        .attr("id", "w0-list")
        .attr("style", "height: 200px; overflow-y: auto;")
        .build()
}

/// ⭐⭐⭐ **虚拟列表的窗口化渲染**（**首屏与 tick 共用** ⇒ ⇒ 两条路径口径一致）。
///
/// ## 为什么必须共用（E-161 修的真缺陷）
///
/// `build_demo_tree` 生成的列表里只有 **1 个占位行**（`rows.first()`）
/// ⇒ ⇒ 首屏挂载后用 `patch_windowed_at_path` 把它换成**窗口化内容**
/// （窗口内的行 + 前后留白）。
///
/// ⚠️⚠️ **原缺陷**：`w0_tick` 用 `build_demo_tree` 重建整棵树 ⇒ ⇒ 列表又变回
/// **1 个占位行** ⇒ ⇒ `patch` 把窗口化结果**整个覆盖掉**
/// ⇒ ⇒ ⇒ **虚拟列表在第一次交互（输入/滚动）后就塌成 1 行**
/// ⇒ ⇒ ⇒ 用户看到的列表**不再滚动、不再虚拟化**。
///
/// ⇒ ⇒ 修法：**每次改完树都要重新窗口化** ⇒ ⇒ ⇒ 抽成本函数 ⇒ ⇒ 两条路径共用
/// ⇒ ⇒ ⇒ 口径一致，不会再出现「一条路径窗口化、另一条不」的偏差。
fn render_windowed_list(
    renderer: &mut apeireth_ui_dom::Renderer<apeireth_ui_dom::web::WebDom>,
    list_path: &[usize],
    item_count: usize,
) -> Result<(), JsValue> {
    use apeireth_ui_dom::vnode::VNode;
    let heights = vec![40.0_f64; item_count];
    renderer
        .patch_windowed_at_path(list_path, &heights, 2, |index| {
            VNode::element("div")
                .attr("class", "row")
                .attr("style", "height: 40px")
                .child(VNode::text(format!("行 {index}")))
                .build()
        })
        .map_err(|error| JsValue::from_str(&format!("窗口化失败：{error}")))?;
    Ok(())
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
    // ⭐⭐ **优先复用页面里已有的 `#w0-root`**（E-163）。
    //
    // ## 为什么
    //
    // `web/w0/index.html` 里**本来就有** `<div id="w0-root"></div>`（页面骨架的一部分）
    // ⇒ ⇒ 若这里**再建一个**同 id 的容器 ⇒ ⇒ 页面上会有**两个 `#w0-root`**
    // ⇒ ⇒ ⇒ ① id 重复（无效 HTML）；② 任何按 `#w0-root` 选择的东西会**选到两个**
    // ⇒ ⇒ ⇒ ③ 更要命：**测量会被算重**（E-163 的三次自造假象之一）。
    //
    // ⇒ ⇒ **有就复用，没有才建**。
    let root = match document.query_selector("#w0-root").ok().flatten() {
        Some(existing) => existing.into(),
        None => {
            let created = dom
                .create_element("div")
                .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
            dom.set_attribute(&created, "id", "w0-root")
                .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
            if let Some(body) = document.body() {
                let _ = body.append_child(&created);
            }
            created
        }
    };
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

    // ⭐⭐⭐ **虚拟列表：首屏挂载后**同步**窗口化**（E-162 的实测结论）。
    //
    // ## 为什么是同步（而不是延后到 paint 之后）
    //
    // E-160 曾以为「延后 ⇒ 那次强制回流并入浏览器首次绘制 ⇒ ⇒ 省掉 88 ms」
    // ⇒ ⇒ **E-162 在干净页面上实测推翻了这个结论**：
    // 延后路径的「用户看到列表」时刻 = **58~75 ms**，同步路径 = **20 ms**
    // ⇒ ⇒ ⇒ **延后反而更晚**（多等两帧，而布局还是要做）。
    //
    // ⇒ ⇒ ⇒ 而且那 58~88 ms 本来就是**测量环境污染**：
    // E-159/E-160 是在 l1 页面测的，那里已有其它探针留下的 **37,476 个节点**
    // ⇒ ⇒ 首次 layout 要把它们全排一遍
    // ⇒ ⇒ ⇒ E-162 的污染实验直接证实：干净页 **15.7 ms** / 加 20,000 节点后 **140.2 ms**
    // ⇒ ⇒ ⇒ ⇒ **那不是我们列表的成本**。
    //
    // ⇒ ⇒ **结论（E-162，三次复现）**：首屏**立即窗口化**。
    //
    // ## 曾经以为「必须延后一帧」（E-160 / E-162 初稿），**已被实测推翻**
    //
    // 那两个结论都建立在**一次**测量上：
    // ①「延后能省 88 ms」⇒ ⇒ ⇒ **假**（那 58~88 ms 是**测量环境污染**）；
    // ②「同步会算错窗口（只有 1 行）」⇒ ⇒ ⇒ **不可复现**（三次复现都是 7 行）。
    //
    // ⇒ ⇒ ⇒ **可复现**的结论（三次一致）：
    // - 立即窗口化 ⇒ ⇒ 窗口**正确**（7 行）、t_dom ≈ **18~20 ms**；
    // - 延后一帧 ⇒ ⇒ 窗口**正确**（7 行）、t_dom ≈ **41~42 ms**；
    // ⇒ ⇒ ⇒ ⇒ **延后没有任何收益，只是晚 20~35 ms** ⇒ ⇒ ⇒ ⇒ **立即窗口化**。
    //
    // ⇒ ⇒ 保留 [`w0_windowize`] 供宿主在**别**的场景显式调用
    // ⇒ ⇒（例如「先渲染骨架、后补数据」这类**已知**要延后的场景）。
    render_windowed_list(&mut renderer, &[1], 1000)?;
    // ⭐⭐ **诊断（E-164）**：监听挂上之后**立刻**派发一次 `scroll` ⇒ ⇒ 看 `peek()`。
    //   ⇒ ⇒ 与「窗口化之前」那次对比 ⇒ ⇒ 定位是「挂上就没生效」
    //   ⇒ ⇒ 还是「窗口化把监听弄丢了」。
    if let Some(container) = document.query_selector("#w0-root #w0-list").ok().flatten() {
        if let Ok(event) = web_sys::Event::new("scroll") {
            let _ = container.dispatch_event(&event);
        }
    }
    let scroll_peek_after_windowize =
        DEMO.with(|slot| slot.borrow().as_ref().map(|demo| demo.scroll.peek()));
    SCROLL_PROBE.set(scroll_peek_after_windowize.unwrap_or(false));

    DEMO.with(|slot| {
        *slot.borrow_mut() = Some(DemoRenderer {
            renderer,
            state,
            scroll,
            list_path: vec![1],
            row_count: 1000,
            // ⭐ 首屏已**立即**窗口化（E-162 实测最优）⇒ ⇒ 不是 pending。
            pending_windowize: false,
        });
    });

    // ⭐ 诊断：窗口化之后的列表状态（行数 / 容器高度）⇒ ⇒ 一眼看出窗口算得对不对。
    let list_children = document
        .query_selector_all("#w0-root #w0-list > *")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);
    let list_client_height = document
        .query_selector("#w0-root #w0-list")
        .ok()
        .flatten()
        .and_then(|element| {
            js_sys::Reflect::get(&element, &JsValue::from_str("clientHeight"))
                .ok()
                .and_then(|value| value.as_f64())
        })
        .unwrap_or(-1.0);

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mounted"),
        &JsValue::from_bool(true),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("list_children"),
        &JsValue::from_f64(list_children as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("list_client_height"),
        &JsValue::from_f64(list_client_height),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐⭐ **最小复现：清空列表 ⇒ 再填充 ⇒ DOM 会不会翻倍**（E-163）。
///
/// ## 为什么要这个探针
///
/// w0 页面实测：DOM 里出现了**两套**完整的窗口化内容（7 行 + 留白 × 2）
/// ⇒ ⇒ ⇒ **疑似引擎缺陷**：`patch_children_at_path` 在「清空 ⇒ 重新填充」时
/// ⇒ ⇒ ⇒ **旧节点没被移除**（DOM 泄漏 + 视觉重复）。
///
/// ⇒ ⇒ 本探针在**干净环境**里做最小复现：
/// ① 挂载 3 个子节点 ⇒ ⇒ 报告 DOM 子节点数；
/// ② 用 **0** 个子节点替换（清空）⇒ ⇒ 报告 DOM 子节点数（应 = 0）；
/// ③ 再用 3 个子节点填充 ⇒ ⇒ 报告 DOM 子节点数（应 = 3，**不该是 6**）。
#[wasm_bindgen]
pub fn l1_patch_children_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::vnode::VNode;
    use apeireth_ui_dom::web::WebDom;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "patch-children-probe-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let count = || -> f64 {
        document
            .query_selector_all("#patch-children-probe-root #probe-list > *")
            .ok()
            .map(|list| list.length())
            .unwrap_or(0) as f64
    };
    let make_row = |index: usize| {
        VNode::element("div")
            .attr("class", "probe-row")
            .child(VNode::text(format!("行 {index}")))
            .build()
    };

    let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
    // ① 挂载：容器（`section`）里带 3 个子节点。
    renderer
        .render_guarded(
            VNode::element("section")
                .attr("id", "probe-section")
                .child(
                    VNode::element("div")
                        .attr("id", "probe-list")
                        .child(make_row(0))
                        .child(make_row(1))
                        .child(make_row(2))
                        .build(),
                )
                .build(),
        )
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;
    let after_mount = count();

    // ② 清空：用**整树 `patch`**（与产品路径同一条路）⇒ ⇒ 新树里列表**没有行**。
    let empty_tree = VNode::element("section")
        .attr("id", "probe-section")
        .child(VNode::element("div").attr("id", "probe-list").build())
        .build();
    renderer
        .patch(empty_tree)
        .map_err(|error| JsValue::from_str(&format!("整树清空失败：{error}")))?;
    let after_clear = count();

    // ③ 再填充：整树 `patch`，新树里列表**有 3 行**。
    let full_tree = VNode::element("section")
        .attr("id", "probe-section")
        .child(
            VNode::element("div")
                .attr("id", "probe-list")
                .child(make_row(0))
                .child(make_row(1))
                .child(make_row(2))
                .build(),
        )
        .build();
    renderer
        .patch(full_tree)
        .map_err(|error| JsValue::from_str(&format!("整树重建失败：{error}")))?;
    let after_refill = count();

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("after_mount"),
        &JsValue::from_f64(after_mount),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("after_clear"),
        &JsValue::from_f64(after_clear),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("after_refill"),
        &JsValue::from_f64(after_refill),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐⭐ **滚动信号探针**（E-164）：引擎**到底有没有收到 `scroll` 事件**？
///
/// ## 它回答什么
///
/// w0 页面上滚动后窗口**不动**（首行仍是「行 0」）⇒ ⇒ 两种可能：
/// ①引擎**没收到** `scroll` 事件（`ScrollSignal` 的 `dirty` 从没被置位）；
/// ②收到了，但 `w0_tick` 没据此重新窗口化。
///
/// ⇒ ⇒ 本探针直接读 `ScrollSignal::peek()` ⇒ ⇒ **区分这两种可能**。
/// ⭐⭐⭐ **焦点与模态浏览器探针**（E-171 / E-172）。
///
/// ## 它验什么
///
/// E-171 的键盘语义层**全部在引擎侧**（`focus_order` / `focus_next` / `open_modal`）
/// ⇒ ⇒ **native 单测测不到「真实浏览器 + 真实 keydown」这条路**
/// ⇒ ⇒ 必须**真实浏览器 + 真实派发**才验得住。
///
/// 1. **Tab 导航**：真实派发 `keydown(Tab)` ⇒ ⇒ 引擎的 `focus_next` 把焦点移到下一个
///    可聚焦元素 ⇒ ⇒ 断言「焦点**真的动了**」（`document.activeElement` 逐字比对）；
/// 2. **Shift+Tab 反向**：真实派发带 Shift 的 Tab ⇒ ⇒ 焦点往回走；
/// 3. **模态陷阱**：开模态（只允许弹层里的元素）⇒ ⇒ Tab ⇒ ⇒ 焦点**不得**跳出弹层；
/// 4. **空列表窗口化不 panic**（E-172）：空高度向量 ⇒ ⇒ **优雅返回**而不是崩。
#[wasm_bindgen]
pub fn l1_focus_modal_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::vnode::VNode;
    use apeireth_ui_dom::web::WebDom;
    use std::cell::RefCell;
    use std::rc::Rc;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ⓪ 建**独立**挂载点（不碰页面其它探针的状态 ⇒ ⇒ 不互相污染）
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "focus-modal-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    // ⓪⓪ 引擎放进 `Rc<RefCell<>>` ⇒ ⇒ 事件处理器能拿到 `&mut`。
    let renderer_cell: Rc<RefCell<apeireth_ui_dom::Renderer<WebDom>>> =
        Rc::new(RefCell::new(apeireth_ui_dom::Renderer::new(dom, root)));
    renderer_cell
        .borrow_mut()
        .render_guarded(
            VNode::element("div")
                .child(
                    VNode::element("button")
                        .attr("id", "fm-b1")
                        .attr("class", "fm-btn")
                        .child(VNode::text("B1"))
                        .build(),
                )
                .child(
                    VNode::element("span")
                        .attr("id", "fm-skip")
                        .child(VNode::text("不可聚焦"))
                        .build(),
                )
                // ⭐⭐⭐ **E-181 可见性**：`class="fm-hidden"` ⇒ ⇒ 页面里用
                //   **真实 class 规则** `.fm-hidden { display: none; }` 把它藏起来
                //   ⇒ ⇒ 隐藏**只能**由浏览器的**计算样式**判出来（引擎自己
                //   不知道 class 规则）⇒ ⇒ ⇒ 这条路**只有浏览器能验**。
                //   ⇒ ⇒ 它在 Tab 序列里必须**被跳过**（⇒ ⇒ Tab 从 fm-b1 直接
                //   落到 fm-b2）⇒ ⇒ 「可见性判据坏掉」⇒ ⇒ 下面那条断言立刻红。
                .child(
                    VNode::element("button")
                        .attr("id", "fm-hidden")
                        .attr("class", "fm-hidden")
                        .child(VNode::text("隐藏按钮"))
                        .build(),
                )
                .child(
                    VNode::element("input")
                        .attr("id", "fm-b2")
                        .attr("class", "fm-btn")
                        .build(),
                )
                .child(
                    VNode::element("div")
                        .attr("id", "fm-modal")
                        .child(
                            VNode::element("button")
                                .attr("id", "fm-inside")
                                .attr("class", "fm-btn")
                                .child(VNode::text("内"))
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;

    // ⭐⭐⭐ 装载**引擎的键盘导航装置**（E-174）⇒ ⇒ Tab 从此**走引擎**
    //   ⇒ ⇒ 浏览器的原生 Tab **不再插手** ⇒ ⇒ 焦点陷阱才在真实浏览器里成立。
    let installed = apeireth_ui_dom::keyboard::KeyNavigation::install(renderer_cell.clone())
        .map_err(|error| JsValue::from_str(&format!("装载键盘导航失败：{error}")))?;

    // ⓫ 派发辅助：真实 `KeyboardEvent`（带 Shift / 不带）
    let make_key = |key: &str, shift: bool| -> Result<web_sys::KeyboardEvent, JsValue> {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(key);
        init.set_code(key);
        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_shift_key(shift);
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .map_err(|error| JsValue::from_str(&format!("建键盘事件失败：{error:?}")))
    };

    // 读当前焦点（真实 `document.activeElement`）
    let active_id = || -> String {
        document
            .active_element()
            .map(|element| element.id())
            .unwrap_or_default()
    };

    // ① 初始焦点在第一个按钮（路径 [0,0]）
    renderer_cell
        .borrow_mut()
        .focus_at_path(&[0])
        .map_err(|error| JsValue::from_str(&format!("聚焦失败：{error}")))?;
    let focus_after_first = active_id();

    // ② 派发 Tab ⇒ ⇒ 引擎应把焦点移到下一个**可见**的可聚焦元素
    //    （跳过 span ⇒ **跳过 class 驱动的隐藏按钮** ⇒ ⇒ 落到 input）
    let tab_event = make_key("Tab", false)?;
    let first_button = document
        .query_selector("#focus-modal-root #fm-b1")
        .ok()
        .flatten()
        .ok_or_else(|| JsValue::from_str("找不到第一个按钮"))?;
    first_button.dispatch_event(&tab_event)?;
    let focus_after_tab = active_id();

    // ③ 派发 Shift+Tab ⇒ ⇒ 焦点往回走（回到第一个按钮）
    let shift_tab_event = make_key("Tab", true)?;
    first_button.dispatch_event(&shift_tab_event)?;
    let focus_after_shift_tab = active_id();

    // ④⭐ 开模态（只允许弹层里的按钮）⇒ ⇒ Tab ⇒ ⇒ 焦点**不得**跳出弹层
    //
    // ⚠ 弹层在**子节点下标 4**（E-181 插入了隐藏按钮 ⇒ ⇒ 后面全部后移一位）。
    renderer_cell
        .borrow_mut()
        .open_modal(&[&[4, 0]], &[4], Some(&[4, 0]))
        .map_err(|error| JsValue::from_str(&format!("开模态失败：{error}")))?;
    let modal_tab_event = make_key("Tab", false)?;
    let inside = document
        .query_selector("#focus-modal-root #fm-inside")
        .ok()
        .flatten()
        .ok_or_else(|| JsValue::from_str("找不到弹层按钮"))?;
    inside.dispatch_event(&modal_tab_event)?;
    let focus_in_modal = active_id();

    // ⑤ 关模态 ⇒ ⇒ 焦点归还
    let restored = renderer_cell.borrow_mut().close_modal().ok().flatten();
    let focus_after_close = active_id();

    // ⑥⭐ 空列表窗口化（E-172）：空高度向量 ⇒ ⇒ **优雅返回**而不是崩
    let empty_window_ok = {
        let mut guard = renderer_cell.borrow_mut();
        guard
            .patch_windowed_at_path(&[4], &[], 0, |_| VNode::element("div").build())
            .is_ok()
    };

    // ⓷ 组装结果
    let result = js_sys::Object::new();
    let set = |key: &str, value: JsValue| {
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str(key), &value);
    };
    set("installed", JsValue::from_bool(installed));
    set("focus_after_first", JsValue::from_str(&focus_after_first));
    set("focus_after_tab", JsValue::from_str(&focus_after_tab));
    set(
        "focus_after_shift_tab",
        JsValue::from_str(&focus_after_shift_tab),
    );
    set("focus_in_modal", JsValue::from_str(&focus_in_modal));
    set("focus_after_close", JsValue::from_str(&focus_after_close));
    set("modal_restored", JsValue::from_bool(restored.is_some()));
    set("empty_window_ok", JsValue::from_bool(empty_window_ok));

    // ⭐ 判据
    let ok = installed
        && focus_after_first == "fm-b1"      // ① 初始焦点在第一个按钮
        && focus_after_tab == "fm-b2"       // ② Tab ⇒ ⇒ 跳到 input（跳过 span **与隐藏按钮**）
        && focus_after_shift_tab == "fm-b1" // ③ Shift+Tab ⇒ ⇒ 回到第一个
        && focus_in_modal == "fm-inside"    // ④ 模态内 Tab ⇒ ⇒ 焦点**留在**弹层里
        && focus_after_close == "fm-b1"     // ⑤ 关模态 ⇒ ⇒ 焦点归还
        && empty_window_ok; // ⑥ 空列表不崩
    set("ok", JsValue::from_bool(ok));
    Ok(result.into())
}

#[wasm_bindgen]
pub fn l1_scroll_signal_probe() -> Result<JsValue, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ⓪ ⭐⭐ **最小实验**：自己建一个滚动容器 + 挂监听 ⇒ ⇒ 不依赖 w0 的任何状态。
    //
    //   ⇒ ⇒ 这样能区分「引擎的 scroll 监听本身坏了」与「w0 的某个状态不对」。
    let min_scroll_ok = {
        use apeireth_ui_dom::dom::Dom;
        use apeireth_ui_dom::vnode::VNode;
        use apeireth_ui_dom::web::WebDom;
        let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
        let root = dom
            .create_element("div")
            .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
        dom.set_attribute(&root, "id", "min-scroll-root")
            .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
        if let Some(body) = document.body() {
            let _ = body.append_child(&root);
        }
        let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
        renderer
            .render_guarded(
                VNode::element("section")
                    .child(
                        VNode::element("div")
                            .attr("id", "min-scroll-list")
                            .attr("style", "height: 100px; overflow-y: auto;")
                            .child(
                                VNode::element("div")
                                    .attr("style", "height: 5000px")
                                    .build(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;
        let signal = renderer
            .listen_scroll_at_path(&[0])
            .map_err(|error| JsValue::from_str(&format!("监听滚动失败：{error}")))?;
        let container = document
            .query_selector("#min-scroll-root #min-scroll-list")
            .ok()
            .flatten();
        let Some(container) = container else {
            return Err(JsValue::from_str("找不到最小滚动容器"));
        };
        // ① 手动派发
        let event = web_sys::Event::new("scroll")
            .map_err(|error| JsValue::from_str(&format!("建事件失败：{error:?}")))?;
        container.dispatch_event(&event)?;
        let after_manual = signal.peek();
        // ② 真实滚动
        container.set_scroll_top(2000);
        let after_real_scroll = signal.peek();
        // ③ 读回 scrollTop（看真实滚动是否真的发生）
        let read_back = f64::from(container.scroll_top());
        (after_manual, after_real_scroll, read_back)
    };

    // ⓪⓪ ⭐⭐ **不重新挂载**！直接用**页面现成**的示例屏（l1 页面里
    //   `l1_w0_window_probe` 已经挂过一次）⇒ ⇒ 派发 scroll ⇒ ⇒ 读**那一个** signal。
    //
    // ⚠️⚠️ **上一版探针的错误**：这里又调了一次 `w0_first_screen()`
    //   ⇒ ⇒ 它**重新挂载**并**替换了 DEMO 槽**（新的 `ScrollSignal`）
    //   ⇒ ⇒ 而派发是发给**页面里的 DOM 节点**的
    //   ⇒ ⇒ ⇒ 读到的 signal 与被派发的节点**对不上号** ⇒ ⇒ 「收不到」是**假象**。
    // ⇒ ⇒ ⇒ **教训**：探针**自己制造的状态**会污染后续观测 ⇒ ⇒ 别重复挂载。
    // ①b ⭐ 立刻读一次 `peek()` ⇒ ⇒ 看「挂载后」引擎是否已收到过 scroll。
    let peek_right_after_mount = DEMO.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|demo| demo.scroll.peek())
            .unwrap_or(false)
    });
    // ② 挂一个**独立的**滚动监听器（在同一个列表容器上）。
    let container = document.query_selector("#w0-root #w0-list").ok().flatten();
    let Some(container) = container else {
        return Err(JsValue::from_str("找不到 #w0-list"));
    };
    // 用引擎自己的 API 挂监听 ⇒ ⇒ 拿到 `ScrollSignal`。
    // ⚠️ 引擎的监听是**按已挂载树**挂的 ⇒ ⇒ 这里读 DEMO 里那个渲染器的 signal。
    let scroll_signal = DEMO.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|demo| demo.scroll.peek())
            .unwrap_or(false)
    });
    let _ = container;

    // ③ 派发一个真实 `scroll` 事件 ⇒ ⇒ 再读一次 `peek()`。
    let before = scroll_signal;
    // ⭐ 记录派发**前后**的 DOM 状态（供对照）。
    let scroll_event = web_sys::Event::new("scroll")
        .map_err(|error| JsValue::from_str(&format!("建 scroll 事件失败：{error:?}")))?;
    container.dispatch_event(&scroll_event)?;
    let after = DEMO.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|demo| demo.scroll.peek())
            .unwrap_or(false)
    });

    // ⭐⭐ **诊断：引擎挂监听的到底是哪个节点？**
    //
    // 用**引擎自己的 `bounding_box_at_path`** 拿它「以为」的节点的盒子
    // ⇒ ⇒ 再和 DOM 查询到的 `#w0-list` 的盒子比对
    // ⇒ ⇒ 两者**相同** ⇒ ⇒ 引擎找对了节点 ⇒ ⇒ 问题在别处；
    // ⇒ ⇒ 两者**不同** ⇒ ⇒ 路径 [1] 指向的不是列表容器 ⇒ ⇒ **根因**。
    let engine_box = DEMO.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|demo| demo.renderer.bounding_box_at_path(&[1]).ok())
    });
    let dom_box = container
        .dyn_ref::<web_sys::Element>()
        .map(|element| element.get_bounding_client_rect());
    let dom_box_height = dom_box.as_ref().map(|rect| rect.height()).unwrap_or(-1.0);
    // ⭐ 逐项打印（引擎 vs DOM）⇒ ⇒ 看出差在哪一项。
    let box_detail = format!(
        "engine=({:.1},{:.1},{:.1}x{:.1}) dom=({:.1},{:.1},{:.1}x{:.1})",
        engine_box.map(|b| b.x).unwrap_or(-1.0),
        engine_box.map(|b| b.y).unwrap_or(-1.0),
        engine_box.map(|b| b.width).unwrap_or(-1.0),
        engine_box.map(|b| b.height).unwrap_or(-1.0),
        dom_box.as_ref().map(|r| r.x()).unwrap_or(-1.0),
        dom_box.as_ref().map(|r| r.y()).unwrap_or(-1.0),
        dom_box.as_ref().map(|r| r.width()).unwrap_or(-1.0),
        dom_box.as_ref().map(|r| r.height()).unwrap_or(-1.0),
    );
    let boxes_match = match (engine_box, dom_box.as_ref()) {
        (Some(a), Some(b)) => {
            (a.x - b.x()).abs() < 0.5
                && (a.y - b.y()).abs() < 0.5
                && (a.width - b.width()).abs() < 0.5
                && (a.height - b.height()).abs() < 0.5
        }
        _ => false,
    };

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("boxes_match"),
        &JsValue::from_bool(boxes_match),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("box_detail"),
        &JsValue::from_str(&box_detail),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("engine_box"),
        &match engine_box {
            Some(box_) => JsValue::from_f64(box_.height),
            None => JsValue::from_f64(-1.0),
        },
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("dom_box_height"),
        &JsValue::from_f64(dom_box_height),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("peek_right_after_mount"),
        &JsValue::from_bool(peek_right_after_mount),
    );
    // ⭐ 最小实验的三项结果。
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("w0_peek_after_windowize"),
        &JsValue::from_bool(SCROLL_PROBE.with(|slot| slot.get())),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("min_after_manual"),
        &JsValue::from_bool(min_scroll_ok.0),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("min_after_real_scroll"),
        &JsValue::from_bool(min_scroll_ok.1),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("min_read_back"),
        &JsValue::from_f64(min_scroll_ok.2),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("peek_before"),
        &JsValue::from_bool(before),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("peek_after"),
        &JsValue::from_bool(after),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **键盘语义浏览器探针**（E-169）。
///
/// ## 它验什么
///
/// 缺陷 ① 的修复点**在 `web.rs`**（浏览器侧）⇒ ⇒ **native 单测测不到**
/// ⇒ ⇒ 必须**真实浏览器 + 真实派发**才验得住。
///
/// 1. **键盘事件的修饰键**：真实派发 `keydown`（Shift / 无 Shift）
///    ⇒ ⇒ 引擎收到的 `shift_key()` 必须**分别是 true / false**
///    ⇒ ⇒ ⇒ 旧口径（修饰键只在 MouseEvent 分支填）这里**恒为 false** ⇒ ⇒ **必红**；
/// 2. **IME 组合期标记**：真实派发带 `isComposing` 的 `keydown`
///    ⇒ ⇒ 引擎收到的 `is_composing()` 必须**为 true** ⇒ ⇒ ⇒ 应用才能判
///    ⇒ ⇒ ⇒ 「这次 Enter 是选词还是提交」（r10 §5.2）。
#[wasm_bindgen]
pub fn l1_keyboard_semantics_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::{Dom, EventKind};
    use apeireth_ui_dom::events::ElementHandler;
    use apeireth_ui_dom::vnode::VNode;
    use apeireth_ui_dom::web::WebDom;
    use std::cell::RefCell;
    use std::rc::Rc;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ⓪ 建一个**独立**的挂载点（不碰页面上其它探针的状态 ⇒ ⇒ 不互相污染）
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "kbd-semantics-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    // ⓪⓪ 记录引擎收到的载荷
    /// 引擎收到的一次键盘事件摘要：`(key, shift, ctrl, is_composing)`。
    type KeyEventSeen = (String, bool, bool, bool);

    let seen: Rc<RefCell<Option<KeyEventSeen>>> = Rc::new(RefCell::new(None));
    let sink = seen.clone();
    let handler = ElementHandler::new(move |event| {
        *sink.borrow_mut() = Some((
            event.key().unwrap_or_default().to_owned(),
            event.shift_key(),
            event.ctrl_key(),
            event.is_composing(),
        ));
    });

    let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
    renderer
        .render_guarded(
            VNode::element("section")
                .child(
                    VNode::element("input")
                        .attr("id", "kbd-semantics-input")
                        .handler(EventKind::KeyDown, handler)
                        .build(),
                )
                .build(),
        )
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;

    let input = document
        .query_selector("#kbd-semantics-root #kbd-semantics-input")
        .ok()
        .flatten()
        .ok_or_else(|| JsValue::from_str("找不到探针输入框"))?;

    // ⓫ 造一个键盘事件（`KeyboardEventInit` ⇒ ⇒ 能设 `shiftKey` 与 `isComposing`）
    let make_key_event =
        |shift: bool, composing: bool| -> Result<web_sys::KeyboardEvent, JsValue> {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Enter");
            init.set_code("Enter");
            init.set_bubbles(true);
            init.set_cancelable(true);
            init.set_shift_key(shift);
            init.set_ctrl_key(false);
            init.set_meta_key(false);
            init.set_alt_key(false);
            init.set_is_composing(composing);
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                .map_err(|error| JsValue::from_str(&format!("建键盘事件失败：{error:?}")))
        };

    // ① 派发**不带** Shift 的 `keydown(Enter)`
    let plain = make_key_event(false, false)?;
    input.dispatch_event(&plain)?;
    let plain_seen = seen.borrow().clone();

    // ② 派发**带** Shift 的 `keydown(Enter)`
    let shifted = make_key_event(true, false)?;
    input.dispatch_event(&shifted)?;
    let shifted_seen = seen.borrow().clone();

    // ③ 派发**组合期**的 `keydown(Enter)`（`isComposing = true`）
    //    ⇒ ⇒ 模拟中文输入法「选词确认」那一刻 ⇒ ⇒ 引擎必须看到 `is_composing = true`
    let composing_event = make_key_event(false, true)?;
    input.dispatch_event(&composing_event)?;
    let composing_seen = seen.borrow().clone();

    // ⓷ 组装结果（**零 serde**：用 `js_sys::Object` + `Reflect::set`）
    let result = js_sys::Object::new();
    let set = |key: &str, value: JsValue| {
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str(key), &value);
    };
    match &plain_seen {
        Some((key, shift, ctrl, composing)) => {
            set("plain_key", JsValue::from_str(key));
            set("plain_shift", JsValue::from_bool(*shift));
            set("plain_ctrl", JsValue::from_bool(*ctrl));
            set("plain_composing", JsValue::from_bool(*composing));
        }
        None => set("plain_missing", JsValue::from_bool(true)),
    }
    match &shifted_seen {
        Some((key, shift, ctrl, composing)) => {
            set("shifted_key", JsValue::from_str(key));
            set("shifted_shift", JsValue::from_bool(*shift));
            set("shifted_ctrl", JsValue::from_bool(*ctrl));
            set("shifted_composing", JsValue::from_bool(*composing));
        }
        None => set("shifted_missing", JsValue::from_bool(true)),
    }
    match &composing_seen {
        Some((key, shift, ctrl, composing)) => {
            set("composing_key", JsValue::from_str(key));
            set("composing_flag", JsValue::from_bool(*composing));
            let _ = (shift, ctrl);
        }
        None => set("composing_missing", JsValue::from_bool(true)),
    }
    // ⭐ 判据：**Shift 版必须带 shift=true**，**无 Shift 版必须带 shift=false**
    //   ⇒ ⇒ 旧口径（修饰键只在 MouseEvent 分支填）⇒ ⇒ 两个都是 false ⇒ ⇒ **必红**。
    let ok = plain_seen.is_some()
        && shifted_seen.is_some()
        && composing_seen.is_some()
        && plain_seen.as_ref().is_some_and(|item| !item.1)
        && shifted_seen.as_ref().is_some_and(|item| item.1)
        && composing_seen.as_ref().is_some_and(|item| item.3);
    set("ok", JsValue::from_bool(ok));
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **对照：挂载与窗口化在**同一个 wasm 调用**里**（E-162 的关键证据）。
///
/// ## 它回答什么
///
/// 「窗口算错（只有 1 行）」到底是怎么发生的？
///
/// 假设：`w0_first_screen` 早期版本在**同一个 wasm 调用**里
/// ⇒ ⇒ 「挂载 + 窗口化」⇒ ⇒ 此刻浏览器**还没做任何布局**
/// ⇒ ⇒ 读 `client_height` ⇒ ⇒ **拿到 0** ⇒ ⇒ 窗口只剩 1 行。
///
/// ⇒ ⇒ 本导出**专门复现**那个情形 ⇒ ⇒ 用它与「宿主在帧后调 `w0_windowize`」
/// ⇒ ⇒ 对比 ⇒ ⇒ ⇒ **同一个 wasm 调用内挂载+窗口化 ⇒ 窗口算错**（正确性缺陷）。
#[wasm_bindgen]
pub fn w0_first_screen_sync_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::web::WebDom;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "sync-probe-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let state = Rc::new(RefCell::new(DemoState::default()));
    let rows: Vec<String> = (0..1000).map(|index| format!("行 {index}")).collect();
    let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
    // ① 挂载
    renderer
        .render_guarded(build_demo_tree(&state, &rows))
        .map_err(|error| JsValue::from_str(&format!("渲染失败：{error}")))?;
    // ② ⭐ **同一个 wasm 调用内**窗口化（这是要复现的情形）。
    render_windowed_list(&mut renderer, &[1], 1000)?;

    let rows_in_dom = document
        .query_selector_all("#sync-probe-root #w0-list .row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("rows"),
        &JsValue::from_f64(rows_in_dom as f64),
    );
    // ⭐ 把**容器实际高度**也带出来 ⇒ ⇒ 一眼看出是不是 `client_height = 0`。
    let client_height = document
        .query_selector("#sync-probe-root #w0-list")
        .ok()
        .flatten()
        .and_then(|element| {
            js_sys::Reflect::get(&element, &JsValue::from_str("clientHeight"))
                .ok()
                .and_then(|value| value.as_f64())
        })
        .unwrap_or(-1.0);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("client_height"),
        &JsValue::from_f64(client_height),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **窗口化首屏的虚拟列表**（宿主在浏览器 paint 之后调；E-160）。
///
/// ## 为什么单独一个入口
///
/// `patch_windowed_at_path` 要读**真实布局度量** ⇒ ⇒ **触发强制回流**
/// ⇒ ⇒ 冷启动实测 **58~88 ms**（浏览器首次排版 + 字体度量）
/// ⇒ ⇒ 延后到 paint 之后 ⇒ ⇒ 实测 **0 ms**（E-160）。
///
/// ⇒ ⇒ **调度归宿主**：引擎不猜「什么时候该窗口化」
/// ⇒ ⇒ ⇒ 宿主（JS）在 `requestAnimationFrame` 后调本函数即可。
///
/// ## 语义
///
/// - **幂等**：已经窗口化过 ⇒ ⇒ 直接返回 `ok: true`（不重复读度量）；
/// - **兜底**：若宿主从不调它，`w0_tick` 在首次重渲染时也会窗口化
///   ⇒ ⇒ ⇒ 列表不会永远空着。
#[wasm_bindgen]
pub fn w0_windowize() -> Result<JsValue, JsValue> {
    let mut windowized = false;
    let mut already = false;
    DEMO.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(demo) = guard.as_mut() else {
            return;
        };
        if !demo.pending_windowize {
            already = true;
            return;
        }
        match render_windowed_list(&mut demo.renderer, &demo.list_path, demo.row_count) {
            Ok(()) => {
                demo.pending_windowize = false;
                windowized = true;
            }
            Err(error) => {
                // ⭐ 不静默：把错误带出去（宿主能看到「窗口化没成功」）。
                let _ = error;
            }
        }
    });

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("windowized"),
        &JsValue::from_bool(windowized),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("already"),
        &JsValue::from_bool(already),
    );
    Ok(result.into())
}

/// ⭐ **tick**：状态变了就重渲染（示例级调度器；生产换正式调度器）。
#[wasm_bindgen]
pub fn w0_tick() -> Result<JsValue, JsValue> {
    let mut redrawn = false;
    let mut scroll_seen = false;
    // ⭐ 窗口化失败**不静默**（E-161）⇒ ⇒ 带出去给调用方看。
    let mut window_error: Option<String> = None;
    // ⭐⭐ patch 失败**也不静默**（E-163）⇒ ⇒ 之前 `let _ =` 把它吞了。
    let mut patch_error: Option<String> = None;
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
            // ⭐⭐⭐ **滚动 ⇒ 先更新窗口，再重建树**（E-164 的关键顺序）。
            //
            // ## 为什么顺序不能反
            //
            // 树里带的是 **`state.window`**（当前窗口）⇒ ⇒ 若**先** patch（用旧窗口）
            // ⇒ ⇒ **再**窗口化（算出新窗口）⇒ ⇒ ⇒ 两者**不一致**
            // ⇒ ⇒ ⇒ 下一个 tick 的 patch 又会把列表改回旧窗口 ⇒ ⇒ ⇒ **窗口来回跳**。
            //
            // ⇒ ⇒ **先窗口化（更新 `state.window`）⇒ ⇒ 再重建树 ⇒ ⇒ 再 patch**
            // ⇒ ⇒ ⇒ 树里的窗口 = DOM 里的窗口 ⇒ ⇒ **一致且稳定** ✓
            //
            // ⚠️ 窗口化要读布局度量 ⇒ ⇒ **一次强制回流**
            // ⇒ ⇒ ⇒ **只在滚动时**才做（输入变化**不**触发 ⇒ ⇒ 零额外回流）。
            let need_windowize = scroll_seen || demo.pending_windowize;
            if need_windowize {
                if let Err(error) =
                    render_windowed_list(&mut demo.renderer, &demo.list_path, demo.row_count)
                {
                    // ⭐ 不静默吞掉：把错误**带出去**（`w0_tick` 的结果里有 `window_error`）。
                    window_error = Some(format!("{error:?}"));
                }
                demo.pending_windowize = false;
            }
            let tree = build_demo_tree(&demo.state, &[]);
            // ⚠️ 受控写回在 IME 组合期会被**门住**（引擎负责补写）。
            // ⭐⭐ **错误不再静默吞掉**（E-163）：此前 `let _ = ...` 把 patch 的失败
            //   ⇒ ⇒ 一起吞了 ⇒ ⇒ 于是「列表内容没变」这件事**看起来**像是正常行为
            //   ⇒ ⇒ ⇒ 掩盖了真正的失败原因。
            if let Err(error) = demo.renderer.patch(tree) {
                patch_error = Some(format!("{error:?}"));
            }
            // ⭐⭐⭐ **只在「窗口需要变」时才重新窗口化**（E-162 的根本修法）。
            //
            // ## 触发条件
            //
            // - **滚动了**（`scroll_seen`）⇒ ⇒ 窗口位置变了 ⇒ ⇒ **必须**重新窗口化；
            // - **首屏还没窗口化**（`pending_windowize`）⇒ ⇒ **必须**窗口化一次；
            // - **只是输入变了** ⇒ ⇒ 列表内容**不受影响**
            //   ⇒ ⇒ **不重新窗口化**（省掉一次强制回流）。
            //
            // ## 为什么必须这样分
            //
            // `patch_windowed_at_path` 要读布局度量 ⇒ ⇒ **每次都调 = 每次都强制回流**
            // ⇒ ⇒ 大页面上单次可达 **140 ms**（E-162 实测）
            // ⇒ ⇒ ⇒ 每 50 ms 一次 tick 若都调 ⇒ ⇒ 主线程被占满
            // ⇒ ⇒ ⇒ ⇒ 宿主自己的 `requestAnimationFrame` **饿死**（实测帧驱动停摆）。
            //
            // ⇒ ⇒ 配合「`build_demo_tree` 的列表**不含行**」⇒ ⇒
            // ⇒ ⇒ **整树重建碰不到窗口化结果** ⇒ ⇒ 输入变化**零额外回流** ✓
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
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("window_error"),
        &match &window_error {
            Some(message) => JsValue::from_str(message),
            None => JsValue::NULL,
        },
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("patch_error"),
        &match &patch_error {
            Some(message) => JsValue::from_str(message),
            None => JsValue::NULL,
        },
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **W0 首屏窗口化保持性探针**（2026-11-03；E-161）。
///
/// ## 它查什么
///
/// 怀疑：`w0_tick()` 用 `build_demo_tree(&state, &[])` 重建整棵树
/// ⇒ ⇒ 那棵树的列表里只有 **1 个空行**（`rows.first()` 是 `None` ⇒ 空的）
/// ⇒ ⇒ ⇒ **`patch` 会覆盖掉 `patch_windowed_at_path` 的窗口化结果**
/// ⇒ ⇒ ⇒ 虚拟列表在首次 tick 之后**塌成 1 行**。
///
/// ⇒ ⇒ 若属实 ⇒ ⇒ **上屏第一屏的参考实现是坏的**（虚拟列表根本没生效）
/// ⇒ ⇒ ⇒ 而这**不是**性能问题，是**功能缺陷**。
#[wasm_bindgen]
pub fn l1_w0_window_probe() -> Result<JsValue, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ① 挂载示例首屏（与产品路径**同一套**代码）。
    let _ = w0_first_screen()?;

    // ①b ⭐ 宿主在**一帧后**调 `w0_windowize`（E-162：同步窗口化会算错窗口）。
    //    本探针是**同步**的 ⇒ ⇒ 这里直接调（此时浏览器已 layout 过一次
    //    ⇒ ⇒ 因为前面的探针已经触发过 layout）⇒ ⇒ 窗口算得对。
    let _ = w0_windowize()?;

    // ② 窗口化之后的窗口行数。
    let rows_after_mount = document
        .query_selector_all("#w0-root #w0-list .row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);

    // ③ 跑一次 tick —— 见下方（**先置脏**，否则是假阴性）。

    // ④ tick 之后的窗口行数。
    let rows_after_tick = document
        .query_selector_all("#w0-root #w0-list .row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);

    // ⭐ 诊断：把 `#w0-list` 的**子节点结构**（类名 + 文本前 12 字）列出来
    //   ⇒ ⇒ 一眼看出 tick 之后**剩下的是什么**（spacer？行？混合？）。
    let dump_children = || -> String {
        let mut parts: Vec<String> = Vec::new();
        // ⚠️ 用 `query_selector_all("#w0-list > *")` 取**直接子节点**
        //   （`Element::children()` 需要 `web-sys` 的 `HtmlElement` feature
        //   ⇒ ⇒ 不为此加 feature ⇒ ⇒ **依赖面不动**）。
        if let Ok(children) = document.query_selector_all("#w0-root #w0-list > *") {
            for index in 0..children.length() {
                if let Some(child) = children.item(index) {
                    // ⚠️ `item()` 返回 `Node`（不是 `Element`）⇒ ⇒ 用 `Reflect` 取属性
                    //   （避免为此引入 `Element` 转换与新 feature）。
                    let class = js_sys::Reflect::get(&child, &JsValue::from_str("className"))
                        .ok()
                        .and_then(|value| value.as_string())
                        .unwrap_or_default();
                    let text = js_sys::Reflect::get(&child, &JsValue::from_str("textContent"))
                        .ok()
                        .and_then(|value| value.as_string())
                        .unwrap_or_default();
                    let short: String = text.chars().take(12).collect();
                    parts.push(format!("{class}[{short}]"));
                }
            }
        }
        parts.join(", ")
    };
    let dump_mount = dump_children();
    DEMO.with(|slot| {
        if let Some(demo) = slot.borrow().as_ref() {
            demo.state.borrow_mut().dirty = true;
        }
    });
    let tick_result = w0_tick()?;
    let dump_tick = dump_children();

    let mut failures: Vec<String> = Vec::new();
    if rows_after_mount < 2 {
        failures.push(format!(
            "首屏刚挂载时窗口行数 = {rows_after_mount}（应 O(窗口) ⇒ ≥ 2）"
        ));
    }
    if rows_after_tick < 2 {
        failures.push(format!(
            "⭐ **tick 之后窗口行数 = {rows_after_tick}** ⇒ ⇒ 虚拟列表**被覆盖塌陷**了"
        ));
    }
    // ⭐⭐ **防回归的强断言**：tick 之后行数必须与**挂载后**一致
    //   ⇒ ⇒ 「窗口化被整树重建覆盖」这类缺陷**必然被抓到**
    //   ⇒ ⇒ （只断言「≥ 2」会放过「从 7 行塌成 3 行」这种部分塌陷）。
    if rows_after_tick != rows_after_mount {
        failures.push(format!(
            "⭐ **tick 之后窗口行数（{rows_after_tick}）与挂载后（{rows_after_mount}）不一致**             ⇒ ⇒ 整树重建**覆盖了窗口化结果**"
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
        &JsValue::from_str("rows_after_mount"),
        &JsValue::from_f64(rows_after_mount as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("rows_after_tick"),
        &JsValue::from_f64(rows_after_tick as f64),
    );
    // ⭐ 把 tick 的返回值带出来（**含 patch_error / window_error**）⇒ ⇒ 出错能看见。
    // ⭐ tick 的关键字段（**逐字段取**，不用 `JSON::stringify`——它要 `JsString`）。
    for (key, value) in [
        ("tick_redrawn", "redrawn"),
        ("tick_scroll_seen", "scroll_seen"),
    ] {
        let field =
            js_sys::Reflect::get(&tick_result, &JsValue::from_str(value)).unwrap_or(JsValue::NULL);
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str(key), &field);
    }
    for (key, value) in [
        ("tick_window_error", "window_error"),
        ("tick_patch_error", "patch_error"),
    ] {
        let field =
            js_sys::Reflect::get(&tick_result, &JsValue::from_str(value)).unwrap_or(JsValue::NULL);
        let _ = js_sys::Reflect::set(&result, &JsValue::from_str(key), &field);
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("dump_mount"),
        &JsValue::from_str(&dump_mount),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("dump_tick"),
        &JsValue::from_str(&dump_tick),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **「延后一帧再窗口化」实验探针**（2026-11-03；E-160）。
///
/// ## 它回答什么
///
/// E-159 定位到：首屏挂载的 88 ms 是**浏览器首次 layout**（读度量 ⇒ 强制回流）
/// ⇒ ⇒ **冷 88 ms / 热 0 ms**。
///
/// ⭐ 关键问题：**这 88 ms 能不能避免？**
///
/// 假设：若宿主策略是「**先渲染容器，不窗口化**；等浏览器**画完一帧**后再窗口化」
/// ⇒ ⇒ 那时浏览器**已经做过一次 layout** ⇒ ⇒ ⇒ 延后窗口化的那次度量读取
/// ⇒ ⇒ ⇒ **应该接近 0** ⇒ ⇒ ⇒ ⇒ **88 ms 整个消失**（不是变快，是**并入**了
/// ⇒ ⇒ ⇒ ⇒ 浏览器本来就要付的首次绘制代价）。
///
/// ## 两步用法（与 `l1_app_probe` / `_after_frame` 同模式）
///
/// 1. `l1_deferred_window_probe()` —— 挂载容器（**不**窗口化），返回挂载耗时；
/// 2. JS 侧等一帧后调 `l1_deferred_window_probe_after_frame()` —— 窗口化并量耗时。
#[wasm_bindgen]
pub fn l1_deferred_window_probe() -> Result<JsValue, JsValue> {
    // ⚠️ `Dom` trait 必须引（`create_element` / `set_attribute` 是 trait 方法）。
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::web::WebDom;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let now_ms = || js_sys::Date::now();

    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "deferred-window-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let state = Rc::new(RefCell::new(DemoState::default()));
    let rows: Vec<String> = (0..1000).map(|index| format!("行 {index}")).collect();

    // ① 只挂载容器（**不**窗口化）⇒ ⇒ 浏览器尚未被迫做滚动容器的 layout。
    let mount_start = now_ms();
    let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
    renderer
        .render_guarded(build_demo_tree(&state, &rows))
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;
    let mount_millis = now_ms() - mount_start;

    // 把渲染器留在全局，**帧后**再用（窗口化要读真实度量）。
    DEFERRED.with(|slot| {
        *slot.borrow_mut() = Some(DeferredRenderer { renderer });
    });

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mount_millis"),
        &JsValue::from_f64(mount_millis),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **「延后一帧再窗口化」实验探针 · 帧后**（2026-11-03；E-160）。
#[wasm_bindgen]
pub fn l1_deferred_window_probe_after_frame() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::vnode::VNode;

    let now_ms = || js_sys::Date::now();
    let mut failures: Vec<String> = Vec::new();

    let mut measured = 0.0_f64;
    DEFERRED.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(deferred) = guard.as_mut() else {
            failures.push(String::from("⚠️ 没找到延后窗口化的渲染器（第一步没跑？）"));
            return;
        };
        let heights = vec![40.0_f64; 1000];
        // ⭐ **窗口计算**（帧后）：这次读度量应该**很快**（浏览器已 layout 过）。
        let plan_start = now_ms();
        let plan = deferred
            .renderer
            .virtual_window_for(&[1], &heights, 2)
            .map_err(|error| JsValue::from_str(&format!("窗口计算失败：{error}")));
        let plan_millis = now_ms() - plan_start;
        let Ok(plan) = plan else {
            failures.push(String::from("窗口计算失败"));
            return;
        };
        // 写入窗口行（spacer + 行）。
        let write_start = now_ms();
        let mut rows = Vec::new();
        if plan.offset > 0.0 {
            rows.push(
                VNode::element("div")
                    .attr("class", "vlist-spacer-before")
                    .attr("style", format!("height: {}px", plan.offset))
                    .build(),
            );
        }
        for index in plan.first..=plan.last {
            rows.push(
                VNode::element("div")
                    .attr("class", "row")
                    .attr("style", "height: 40px")
                    .child(VNode::text(format!("行 {index}")))
                    .build(),
            );
        }
        let total: f64 = heights.iter().sum();
        let window_height: f64 = heights[plan.first..=plan.last].iter().sum();
        let spacer_after = (total - plan.offset - window_height).max(0.0);
        if spacer_after > 0.0 {
            rows.push(
                VNode::element("div")
                    .attr("class", "vlist-spacer-after")
                    .attr("style", format!("height: {spacer_after}px"))
                    .build(),
            );
        }
        let write_result = deferred.renderer.patch_children_at_path(&[1], rows);
        let write_millis = now_ms() - write_start;
        if write_result.is_err() {
            failures.push(String::from("窗口写入失败"));
        }
        measured = plan_millis + write_millis;
        DEFERRED_PLAN_MS.with(|slot| slot.set(plan_millis));
        DEFERRED_WRITE_MS.with(|slot| slot.set(write_millis));
    });

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
        &JsValue::from_str("deferred_millis"),
        &JsValue::from_f64(measured),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("plan_millis"),
        &JsValue::from_f64(DEFERRED_PLAN_MS.with(|slot| slot.get())),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("write_millis"),
        &JsValue::from_f64(DEFERRED_WRITE_MS.with(|slot| slot.get())),
    );
    Ok(result.into())
}

/// 延后窗口化实验的渲染器暂存（线程局部单例）。
struct DeferredRenderer {
    renderer: apeireth_ui_dom::Renderer<apeireth_ui_dom::web::WebDom>,
}

thread_local! {
    static DEFERRED: RefCell<Option<DeferredRenderer>> = const { RefCell::new(None) };
    /// 帧后窗口计算的耗时（供结果对象读取）。
    static DEFERRED_PLAN_MS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// 帧后窗口写入的耗时。
    static DEFERRED_WRITE_MS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// ⭐⭐⭐ **真实负载基线探针**（2026-11-03；E-158）。
///
/// ## 它测什么
///
/// §8 的性能数字此前都来自**合成压测树**（37,476 节点）⇒ ⇒ E-155 已更正：
/// **真实首屏**是**虚拟列表窗口化**（只几十节点）⇒ ⇒ 首屏**没有卡点**。
///
/// ⭐ 本探针量的是**真实用户交互路径**的三段耗时：
///
/// | 段 | 是什么 | 为什么重要 |
/// |---|---|---|
/// | `mount_millis` | 首屏**首次挂载**（含 1000 行虚拟列表的窗口化计算） | 用户看到的**第一帧** |
/// | `input_redraw_millis` | 改**输入框**后的一次重渲染（真实 `input` 事件驱动） | 最常见的交互 |
/// | `scroll_redraw_millis` | **滚动**后的一次重渲染（真实 `scroll` 事件驱动） | 列表场景的核心交互 |
///
/// ## 为什么用**放大测量**
///
/// 浏览器侧**拿不到可靠的亚毫秒时钟**（E-156 实测：`performance.now()` 反复
/// `eval` 读数恒为 0.1 µs；函数引用法恒为 0）⇒ ⇒ 改用**毫秒时钟 + 连做 N 次后除以 N**
/// ⇒ ⇒ 累计量够大 ⇒ ⇒ 读数才有意义（⚠️ 精度为**量级**）。
#[wasm_bindgen]
pub fn l1_real_load_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_dom::dom::Dom;
    use apeireth_ui_dom::vnode::VNode;
    use apeireth_ui_dom::web::WebDom;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ⚠️ 毫秒时钟（浏览器侧无可靠亚毫秒时钟 ⇒ E-156）。
    let now_ms = || js_sys::Date::now();

    // ① 首屏挂载：容器 + 1000 行虚拟列表（**窗口化** ⇒ DOM 里只有窗口内的行）。
    let dom = WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let root = dom
        .create_element("div")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    dom.set_attribute(&root, "id", "real-load-root")
        .map_err(|error| JsValue::from_str(&format!("设 id 失败：{error}")))?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    let mut renderer = apeireth_ui_dom::Renderer::new(dom, root);
    let state = Rc::new(RefCell::new(DemoState::default()));

    // ⭐ 窗口化渲染的闭包（**交互重渲染时也必须用它**）。
    //
    // ⚠️⚠️ **踩过的坑**：最初只在**挂载时**调 `patch_windowed_at_path`，
    //   交互重渲染却用 `build_demo_tree` ⇒ ⇒ 那棵树里含**1 行占位行**
    //   ⇒ ⇒ **覆盖掉了窗口化结果** ⇒ ⇒ DOM 里只剩 1 行
    //   ⇒ ⇒ 数字完全失真（且断言没抓到 ⇒ 因为「1 行」也满足 `> 0`）。
    //   ⇒ ⇒ ⇒ 现在**所有**列表渲染都走这个闭包 ⇒ ⇒ 窗口化不会被覆盖。
    let heights = vec![40.0_f64; 1000];
    let render_list = |renderer: &mut apeireth_ui_dom::Renderer<WebDom>| {
        renderer
            .patch_windowed_at_path(&[1], &heights, 2, |index| {
                VNode::element("div")
                    .attr("class", "row")
                    .attr("style", "height: 40px")
                    .child(VNode::text(format!("行 {index}")))
                    .build()
            })
            .map_err(|error| JsValue::from_str(&format!("窗口化失败：{error}")))
    };

    // 挂载：容器 + 窗口化列表（与 `w0_first_screen()` 同形）。
    //
    // ⭐⭐ **分段计时**（2026-11-03；E-159）：把 58 ms 拆成四段，
    // ⇒ ⇒ 才能知道**该优化哪一段**（否则只有一个总数，无从下手）。
    //
    // ⚠️ 注意顺序：`build_demo_tree` 之前还有 `rows` 的构造（1000 个 String）
    //   ⇒ ⇒ 那部分**不在**下面四段里 ⇒ ⇒ 单独量。
    let rows_start = now_ms();
    let rows: Vec<String> = (0..1000).map(|index| format!("行 {index}")).collect();
    let rows_millis = now_ms() - rows_start;

    // ① 首次挂载（`render_guarded`）：建 DOM + 写属性 + 写文本。
    let mount_start = now_ms();
    renderer
        .render_guarded(build_demo_tree(&state, &rows))
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;
    let guarded_millis = now_ms() - mount_start;

    // ② 窗口化：**再拆两段**（E-159）：
    //    ②a **窗口计算**（`virtual_window_for`：含真实 DOM 度量 ⇒ 触发 layout）
    //    ②b **写入**（`patch_children_at_path`：写 spacer + 窗口行）
    //    ⇒ ⇒ 两者相加应 ≈ 原来的 `window_millis` ⇒ ⇒ 定位到具体那一段。
    let window_start = now_ms();
    let window_plan_start = now_ms();
    let _plan = renderer
        .virtual_window_for(&[1], &heights, 2)
        .map_err(|error| JsValue::from_str(&format!("窗口计算失败：{error}")))?;
    let window_plan_millis = now_ms() - window_plan_start;

    // ⭐ **再算一次**（热）：若第一次是「冷」代价 ⇒ ⇒ 第二次应接近 0
    //   ⇒ ⇒ 就能区分「**每次**都慢」与「**只有第一次**慢」。
    let window_plan2_start = now_ms();
    let _plan2 = renderer
        .virtual_window_for(&[1], &heights, 2)
        .map_err(|error| JsValue::from_str(&format!("窗口计算(2)失败：{error}")))?;
    let window_plan2_millis = now_ms() - window_plan2_start;

    let window_write_start = now_ms();
    render_list(&mut renderer)?;
    let window_write_millis = now_ms() - window_write_start;

    let window_millis = now_ms() - window_start;

    let mount_millis = now_ms() - mount_start;

    // ② 输入后重渲染：改状态 ⇒ 重建树 ⇒ patch（**放大测量**：连做 N 次）。
    const INPUT_ROUNDS: usize = 200;
    let input_start = now_ms();
    for round in 0..INPUT_ROUNDS {
        state.borrow_mut().name = format!("名字 {round}");
        state.borrow_mut().dirty = true;
        let tree = build_demo_tree(&state, &rows);
        renderer
            .patch(tree)
            .map_err(|error| JsValue::from_str(&format!("输入重渲染失败：{error}")))?;
        // ⚠️ 必须**再窗口化一次**（否则被上一步的占位行覆盖）。
        render_list(&mut renderer)?;
    }
    let input_redraw_millis = (now_ms() - input_start) / INPUT_ROUNDS as f64;

    // ③ 滚动后重渲染：改滚动位置 ⇒ 重建树 ⇒ patch（同样放大）。
    const SCROLL_ROUNDS: usize = 200;
    let scroll_start = now_ms();
    for round in 0..SCROLL_ROUNDS {
        state.borrow_mut().scroll_top = (round % 100) as f64 * 40.0;
        state.borrow_mut().dirty = true;
        let tree = build_demo_tree(&state, &rows);
        renderer
            .patch(tree)
            .map_err(|error| JsValue::from_str(&format!("滚动重渲染失败：{error}")))?;
        // ⚠️ 同上：必须**再窗口化一次**。
        render_list(&mut renderer)?;
    }
    let scroll_redraw_millis = (now_ms() - scroll_start) / SCROLL_ROUNDS as f64;

    // 断言：首屏挂载后**窗口内**才有行（**不是** 1000 行）⇒ ⇒ 虚拟列表真的在起作用。
    // ⚠️ 行容器是 `build_demo_tree` 自己建的 **`#w0-list`**（不是我的根节点）
    //   ⇒ ⇒ 选择器必须指向它 ⇒ ⇒ 否则统计到 0。
    let mounted_rows = document
        .query_selector_all("#w0-list .row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);
    // ⚠️ 断言**收紧**：容器 200px / 行高 40px ⇒ 窗口约 5 行（+overscan 2）
    //   ⇒ ⇒ **必须 ≥ 2 行** ⇒ ⇒ 「1 行」这种「窗口化被覆盖」的情况**要被抓到**
    //   ⇒ ⇒ （此前 `> 0` 的断言让「1 行」蒙混过关 ⇒ 数字失真却没报错）。
    if !(2..=40).contains(&mounted_rows) {
        failures.push(format!(
            "首屏窗口化异常：DOM 行数 = {mounted_rows}（应 O(窗口)，远小于 1000）"
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
        &JsValue::from_str("mount_millis"),
        &JsValue::from_f64(mount_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("input_redraw_millis"),
        &JsValue::from_f64(input_redraw_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("scroll_redraw_millis"),
        &JsValue::from_f64(scroll_redraw_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mounted_rows"),
        &JsValue::from_f64(mounted_rows as f64),
    );
    // ⭐ 分段计时（E-159）：挂载这一段拆成「构造行数据 / 首次挂载 / 窗口化」。
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("rows_millis"),
        &JsValue::from_f64(rows_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("guarded_millis"),
        &JsValue::from_f64(guarded_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("window_millis"),
        &JsValue::from_f64(window_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("window_plan_millis"),
        &JsValue::from_f64(window_plan_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("window_write_millis"),
        &JsValue::from_f64(window_write_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("window_plan2_millis"),
        &JsValue::from_f64(window_plan2_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("rounds"),
        &JsValue::from_f64((INPUT_ROUNDS + SCROLL_ROUNDS) as f64),
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

    // ── ④ ⭐⭐⭐⭐ **全链闭环**（G4 的「输入 ⇒ 状态 ⇒ DOM」，三跳一次跑完）───
    //
    // ①②③ 各自只验**一段**：
    //   ① 信号 → 首渲染 DOM；② 信号变 → DOM 变；③ 真实 `input` → 信号变。
    // ⇒ ⇒ 但**没有一条**验「真实输入**经状态加工后**回到 DOM」
    //   ⇒ ⇒ 而那正是「受控输入」的全部价值所在 —— 三跳里任何一跳断掉，
    //   用户都会看到「我打的字没变成我该看到的东西」。
    //
    // ⇒ ⇒ 这里模拟应用的**真实加工**（状态加个后缀）⇒ ⇒ 再重渲染
    //   ⇒ ⇒ 断言 DOM 显示的是**加工后**的值 ⇒ ⇒ 且**不需要**手工再写一次 DOM。
    let before_loop = value_of("#l1-bind-input");
    // 第一跳已完成（③ 把 DOM 的 "uplink" 写进了信号）。这里做第二、三跳：
    // ②' 状态**被应用加工**（真实应用会做 trim / 大小写 / 拼接等）
    signal.update(|value| format!("{value}-applied"));
    renderer
        .patch(
            VNode::element("input")
                .attr("id", "l1-bind-input")
                .bind(Binding::text(signal.clone()))
                .build(),
        )
        .map_err(|error| JsValue::from_str(&format!("打补丁失败：{error}")))?;
    // ③' DOM 必须跟着变成**加工后**的值
    let after_loop = value_of("#l1-bind-input");
    if after_loop.as_deref() != Some("uplink-applied") {
        failures.push(format!(
            "④ 全链闭环未闭合：输入⇒状态⇒DOM 之后 DOM = {after_loop:?}（期望 uplink-applied，             加工前 {before_loop:?}）⇒ ⇒ 「我打的字没变成我该看到的东西」"
        ));
    }
    // ⭐ 反向：信号本身也必须真的是加工后的值（防「DOM 对了但状态没对」）
    if signal.get() != "uplink-applied" {
        failures.push(format!("④ 状态未记录加工结果：{}", signal.get()));
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
        &JsValue::from_str("loop_before"),
        &JsValue::from_str(before_loop.as_deref().unwrap_or("<无>")),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("loop_after"),
        &JsValue::from_str(after_loop.as_deref().unwrap_or("<无>")),
    );
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
    container
        .set_attribute("id", "l1-app-root")
        .map_err(|error| JsValue::from_str(&format!("设容器 id 失败：{error:?}")))?;
    body.append_child(&container)
        .map_err(|error| JsValue::from_str(&format!("挂容器失败：{error:?}")))?;

    // ② boot
    wasm_host::boot("#l1-app-root")
        .map_err(|error| JsValue::from_str(&format!("boot 失败：{error:?}")))?;
    let input_value = || -> Option<String> {
        let element = document
            .query_selector("#l1-app-root input")
            .ok()
            .flatten()?;
        js_sys::Reflect::get(&element, &JsValue::from_str("value"))
            .ok()
            .and_then(|value| value.as_string())
    };
    let booted = input_value();
    if booted.as_deref() != Some("宿主已就绪") {
        failures.push(format!("① boot 后输入框值不对：{booted:?}"));
    }

    // ③ ⭐⭐ **批次边界渲染**：N 次状态变更 ⇒ **只渲染一次**（不抖）
    //    （core 的 effect 同步重跑 ⇒ ⇒ 若每次都 patch ⇒ N 次渲染 ⇒ 抖
    //      ⇒ ⇒ effect 只**标脏**，真正的 patch 由批次边界合并成**一次**）
    let renders_before = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;
    for text in ["一", "二", "三"] {
        wasm_host::set_demo_text(text)
            .map_err(|error| JsValue::from_str(&format!("set_demo_text 失败：{error:?}")))?;
    }
    // 变更后、批次边界前 ⇒ DOM **不应**变（渲染被合并了）
    let before_flush = input_value();
    if before_flush.as_deref() == Some("三") {
        failures.push(String::from(
            "② 批次边界前不应渲染（渲染必须合并到边界 ⇒ 否则一帧多次变更 ⇒ 多次渲染 ⇒ 抖）",
        ));
    }
    let flushed =
        wasm_host::flush().map_err(|error| JsValue::from_str(&format!("flush 失败：{error:?}")))?;
    if !flushed {
        failures.push(String::from("② 批次边界应当渲染一次"));
    }
    let renders_after = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;
    if renders_after != renders_before + 1 {
        failures.push(format!(
            "② 3 次变更 ⇒ 只应渲染 1 次；实际 {renders_before} → {renders_after}"
        ));
    }
    let after = input_value();
    if after.as_deref() != Some("三") {
        failures.push(format!(
            "② 批次边界后 DOM 应是**最新**值：三；实际 {after:?}"
        ));
    }
    // ③b 再次 flush ⇒ 幂等（无变更 ⇒ 不该再渲染）
    let flushed_again =
        wasm_host::flush().map_err(|error| JsValue::from_str(&format!("flush 失败：{error:?}")))?;
    if flushed_again {
        failures.push(String::from("②b 无变更时 flush 不应再渲染"));
    }
    let renders_final = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;
    if renders_final != renders_after {
        failures.push(format!("②b 幂等失败：{renders_after} → {renders_final}"));
    }

    // ③b 手动路径仍在：`rerender()` 可显式触发一次
    wasm_host::rerender()
        .map_err(|error| JsValue::from_str(&format!("rerender 失败：{error:?}")))?;

    // ③c ⭐ **帧驱动**：再改一次信号 ⇒ **不**调 flush ⇒ 交给 rAF 帧循环
    //    （⇒ 「自动」= 宿主**不需要记得调**；帧后探针会断言它已更新）
    wasm_host::set_demo_text("帧后更新")
        .map_err(|error| JsValue::from_str(&format!("set_demo_text 失败：{error:?}")))?;

    // ⚠️ dispose 段挪到独立探针（`l1_app_dispose_probe`）⇒ ⇒ 因为
    //   dispose 会**卸载 DOM** ⇒ ⇒ 若在帧后探针之前执行 ⇒ ⇒ 帧后探针就看不到输入框了。

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

/// ⭐ **探针阶段标记**（诊断用）：把当前阶段写进 `window.__P5B_ASYNC_STAGE__`。
///
/// ## 为什么要它（2026-11-05 实测踩过）
///
/// ⇒ ⇒ 探针若在 wasm 里**挂住**（没有抛错、也没有返回）⇒ ⇒ JS 侧只看到
/// 「await 永远不落地」⇒ ⇒⇒ **什么都看不出来** ⇒ ⇒⇒⇒ 只能瞎猜。
/// ⇒ ⇒⇒ 阶段标记让「挂在哪一步」变成**可观测**的。
fn mark_async_stage(stage: &str) {
    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("__P5B_ASYNC_STAGE__"),
            &JsValue::from_str(stage),
        );
    }
}

/// ⭐ E-182：异步数据线探针的**跨调用状态**（arm → collect 两段之间）。
struct AsyncProbeState {
    /// 宿主容器（collect 时移除）。
    container: web_sys::Element,
    /// 对照组：真实到达顺序（arm 时排的定时器往里写）。
    arrivals: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    /// 对照组定时器闭包（**必须保活**到 collect）。
    #[allow(dead_code)]
    control_timers: Vec<Closure<dyn FnMut()>>,
}

thread_local! {
    /// 当前正在跑的异步探针状态（`None` = 没 arm 过）。
    static ASYNC_PROBE: std::cell::RefCell<Option<AsyncProbeState>> =
        const { std::cell::RefCell::new(None) };
}

/// ⭐⭐⭐ **异步数据线探针（第一段：armed）**（E-182）。
///
/// ## 为什么拆成两段（**不是**为了好看）
///
/// 2026-11-05 实测：**异步 wasm 导出**（`async fn` 导出 ⇒ JS 侧拿到 Promise）
/// 在 **WebKitGTK** 里**永远不落地** ⇒ ⇒ 页面 `await` 挂住
/// ⇒ ⇒⇒ 而且**事件循环也被冻住**（超时定时器都不跑）⇒ ⇒ 页面停在上一份报告
/// ⇒ ⇒⇒⇒ 于是「探针挂了」和「探针没跑」**无法区分**。
/// ⇒ ⇒⇒ ⇒ 改成**两段同步导出**：① arm（同步回来）② 页面自己等 900ms ③ collect（同步回来）
/// ⇒ ⇒⇒⇒ 任何一段挂住 ⇒ ⇒ **页面侧一眼看出是哪一段**。
///
/// ## 它做什么
///
/// ① 建容器 + `boot` 参考宿主（宿主内部接了一条**异步数据线**）；
/// ② 先发「alpha」（**慢**：600ms，模拟冷缓存）、紧接着发「beta」（**快**：150ms）
/// ③ 并用**同样的延时**排一组对照定时器 ⇒ ⇒ 记录**真实到达顺序**。
///
/// # Errors
///
/// 建容器 / boot / 发查询失败 ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn l1_async_line_arm() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    mark_async_stage("arm:start");
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
    container
        .set_attribute("id", "l1-async-root")
        .map_err(|error| JsValue::from_str(&format!("设容器 id 失败：{error:?}")))?;
    body.append_child(&container)
        .map_err(|error| JsValue::from_str(&format!("挂容器失败：{error:?}")))?;

    // ② boot 参考宿主（它内部接了一条**异步数据线**）
    mark_async_stage("arm:boot");
    wasm_host::boot("#l1-async-root")
        .map_err(|error| JsValue::from_str(&format!("boot 失败：{error:?}")))?;
    mark_async_stage("arm:booted");

    // ③ 造乱序：先发**慢**的 alpha，再发**快**的 beta ⇒ ⇒ beta **先**回来
    mark_async_stage("arm:query-alpha");
    wasm_host::set_demo_query("alpha")
        .map_err(|error| JsValue::from_str(&format!("set_demo_query(alpha) 失败：{error:?}")))?;
    mark_async_stage("arm:query-beta");
    wasm_host::set_demo_query("beta")
        .map_err(|error| JsValue::from_str(&format!("set_demo_query(beta) 失败：{error:?}")))?;

    // ④ 对照组：**同样的延时**再排两条定时器 ⇒ ⇒ 记录**真实到达顺序**
    //   ⇒ ⇒ 若顺序是 [beta, alpha] ⇒ ⇒ 「后发先至」成立
    //   ⇒ ⇒⇒ 若顺序是 [alpha, beta] ⇒ ⇒ 乱序这个前提不成立 ⇒ ⇒ 正跑段什么也证明不了。
    let arrivals: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut control_timers: Vec<Closure<dyn FnMut()>> = Vec::new();
    for (label, delay) in [("alpha", 600i32), ("beta", 150i32)] {
        let arrivals_for_timer = arrivals.clone();
        let closure = Closure::<dyn FnMut()>::new(move || {
            arrivals_for_timer.borrow_mut().push(label.to_owned());
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                delay,
            );
        }
        control_timers.push(closure);
    }

    ASYNC_PROBE.with(|slot| {
        *slot.borrow_mut() = Some(AsyncProbeState {
            container,
            arrivals,
            control_timers,
        });
    });
    mark_async_stage("arm:done");

    let result = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("armed"),
        &JsValue::from_bool(true),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **异步数据线探针（第二段：collect）**（E-182）—— 「旧回调不能覆盖新工作」的证据。
///
/// ## 它断言什么
///
/// 真定时器造出**乱序**（alpha 慢、beta 快 ⇒ ⇒ beta 先回来、alpha 后回来）后：
/// ① **DOM 必须显示 beta**（后到的 alpha **不许**覆盖它）；
/// ② `resolved == 2`（两条**都**回来了）+ `accepted == 1`（只上屏一条）
/// + `rejected == 1`（有一条**被驳回**）⇒ ⇒ 三个数一起才排除「alpha 根本没回来」；
/// ③ 对照组：真实到达顺序必须是 `[beta, alpha]`（**后发先至**）
/// ⇒ ⇒⇒ 否则「乱序」这个前提不成立 ⇒ ⇒ ①②**什么也证明不了** ⇒ ⇒ **必须报红**。
///
/// ## ⚠️ 为什么还要 `naiveLastWins`
///
/// 把「对照组实测的到达顺序」直接套到「谁最后到谁上屏」这条**朴素策略**上
/// ⇒ ⇒ 得到**没有仲裁时**屏幕上会是什么（实测是 **alpha**）
/// ⇒ ⇒⇒ 这就把「仲裁是唯一让它不发生的机制」写成了**可复核的数字**，而不是一句断言。
///
/// # Errors
///
/// 没有 arm 过 / 读 DOM / 计数失败 ⇒ 返回 JS 错误。
#[wasm_bindgen]
pub fn l1_async_line_collect() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    mark_async_stage("collect:start");
    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    // ① 正跑段判据：DOM 上的结果文本 + 三个计数
    let results_text = document
        .query_selector("#app-demo-results")
        .ok()
        .flatten()
        .and_then(|element| element.text_content())
        .unwrap_or_default();
    let stats = wasm_host::async_line_stats()
        .map_err(|error| JsValue::from_str(&format!("async_line_stats 失败：{error:?}")))?;
    let stat = |key: &str| -> f64 {
        js_sys::Reflect::get(&stats, &JsValue::from_str(key))
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(-1.0)
    };
    let resolved = stat("resolved");
    let accepted = stat("accepted");
    let rejected = stat("rejected");
    mark_async_stage("collect:read");

    if !results_text.starts_with("beta") {
        failures.push(format!(
            "① 过期结果覆盖了新结果：DOM 显示 {results_text:?}（必须是 beta 的结果 \
             ⇒ ⇒ 说明仲裁没挡住后到的 alpha）"
        ));
    }
    if resolved != 2.0 {
        failures.push(format!(
            "② 两条请求**都**应当回来：resolved={resolved}（必须是 2 \
             ⇒ ⇒ 否则「显示 beta」可能只是 alpha 没回来 ⇒ ⇒ 判据不成立）"
        ));
    }
    if accepted != 1.0 {
        failures.push(format!("② 只应上屏一条：accepted={accepted}（必须是 1）"));
    }
    if rejected != 1.0 {
        failures.push(format!(
            "② 后到的过期结果**必须被驳回**：rejected={rejected}（必须是 1）"
        ));
    }

    // ② 对照组判据：真实到达顺序必须是「后发先至」
    let state = ASYNC_PROBE.with(|slot| slot.borrow_mut().take());
    let Some(state) = state else {
        return Err(JsValue::from_str(
            "异步数据线探针：没有先 arm 就 collect（顺序错了）",
        ));
    };
    let order = state.arrivals.borrow().clone();
    let order_ok =
        order == vec![String::from("beta"), String::from("alpha")];
    if !order_ok {
        failures.push(format!(
            "③ 对照组：到达顺序应为 [beta, alpha]（后发先至），实测 {order:?} \
             ⇒ ⇒ 乱序这个前提不成立 ⇒ 正跑段的「显示 beta」不能证明仲裁生效"
        ));
    }
    // 「没有仲裁」时会显示什么：**按到达顺序覆盖** ⇒ ⇒ 最后到的是谁，屏幕上就是谁
    let naive_last = order.last().cloned().unwrap_or_default();

    // ③ 收尾：销毁宿主 + 移除容器（不污染后续探针）
    mark_async_stage("collect:dispose");
    let _ = wasm_host::dispose_screen();
    if let Some(body) = document.body() {
        let _ = body.remove_child(&state.container);
    }
    drop(state);
    mark_async_stage("collect:done");

    let result = js_sys::Object::new();
    let failures_array = js_sys::Array::new();
    for item in &failures {
        failures_array.push(&JsValue::from_str(item));
    }
    let order_array = js_sys::Array::new();
    for label in &order {
        order_array.push(&JsValue::from_str(label));
    }
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("ok"),
        &JsValue::from_bool(failures.is_empty()),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("failures"), &failures_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("resultsText"),
        &JsValue::from_str(&results_text),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("resolved"),
        &JsValue::from_f64(resolved),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("accepted"),
        &JsValue::from_f64(accepted),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("rejected"),
        &JsValue::from_f64(rejected),
    );
    let _ = js_sys::Reflect::set(&result, &JsValue::from_str("arrivalOrder"), &order_array);
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("naiveLastWins"),
        &JsValue::from_str(&naive_last),
    );
    Ok(result.into())
}

/// ⭐⭐⭐⭐ **宿主「帧后」探针**（在浏览器的一帧之后调用）。
///
/// ## 它验证什么
///
/// **rAF 帧驱动**：状态变更 ⇒ **宿主不需要手动调 `flush`** ⇒ **下一帧**自动更新 DOM。
/// ⇒ ⇒ 「自动」不再是「宿主记得调」，而是**引擎自己在帧边界收口**。
#[wasm_bindgen]
pub fn l1_app_probe_after_frame() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    let value = document
        .query_selector("#l1-app-root input")
        .ok()
        .flatten()
        .and_then(|element| {
            js_sys::Reflect::get(&element, &JsValue::from_str("value"))
                .ok()
                .and_then(|value| value.as_string())
        });
    if value.as_deref() != Some("帧后更新") {
        failures.push(format!(
            "⭐ 帧后 DOM 应被**自动**更新为「帧后更新」；实际 {value:?}（⇒ rAF 帧驱动未生效）"
        ));
    }
    let renders = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;
    if renders < 2 {
        failures.push(format!(
            "⭐ 帧驱动应当至少渲染过一次（首屏之外）；实际 renders={renders}"
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
        &JsValue::from_str("render_count"),
        &JsValue::from_f64(renders as f64),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **宿主「销毁」探针**（在帧后探针之后调用）。
///
/// 验证 `dispose_screen` ⇒ **DOM 被卸载**（宿主换屏时必须走这一步，
/// 否则旧屏的焦点陷阱 / IME 待写队列会渗进新屏）。
#[wasm_bindgen]
pub fn l1_app_dispose_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;

    wasm_host::dispose_screen()
        .map_err(|error| JsValue::from_str(&format!("dispose 失败：{error:?}")))?;
    let leftover = document.query_selector("#l1-app-root input").ok().flatten();
    if leftover.is_some() {
        failures.push(String::from("dispose 后输入框仍在 DOM 里"));
    }
    // dispose 时的渲染次数（供「帧循环已停止」探针比较）。
    let renders_at_dispose = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;

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
        &JsValue::from_str("renders_at_dispose"),
        &JsValue::from_f64(renders_at_dispose as f64),
    );
    Ok(result.into())
}

/// ⭐⭐⭐ **帧循环已停止**探针（dispose 之后再调用）。
///
/// ## 它验证什么
///
/// `dispose_screen` 之后，帧循环必须**真的停**（`running` 标志 + 取消已排的那一帧）
/// ⇒ ⇒ 否则宿主换屏后，旧循环还在每帧跑（浪费 + 潜在的跨屏渲染）。
#[wasm_bindgen]
pub fn l1_app_frame_stopped_probe(before: usize) -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    let mut failures: Vec<String> = Vec::new();
    let now = wasm_host::render_count()
        .map_err(|error| JsValue::from_str(&format!("render_count 失败：{error:?}")))?;
    if now != before {
        failures.push(format!(
            "⭐ dispose 后帧循环必须停止（渲染次数不得增长）；dispose 前={before} 现在={now}"
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
    Ok(result.into())
}

/// ⭐⭐⭐ **反复换屏回归**（dispose ⇒ 再 boot ⇒ …）。
///
/// ## 它验证什么
///
/// 宿主换屏是**常态**（路由切换）⇒ ⇒ 每换一次都会 `dispose` 旧屏 + `boot` 新屏
/// ⇒ ⇒ 若 `dispose` 没清干净（焦点陷阱 / IME 队列 / 帧循环 / 捕获错误）
/// ⇒ ⇒ 旧屏的残留会**渗进**新屏 ⇒ ⇒ 且问题往往**延迟**出现（很难定位）。
/// ⇒ ⇒ 这里连做 **3 轮**「boot ⇒ 改信号 ⇒ 渲染 ⇒ dispose ⇒ 卸载」
/// ⇒ ⇒ 每轮都断言「渲染生效 + DOM 真的被卸载」⇒ ⇒ 证**反复换屏不累积问题**。
#[wasm_bindgen]
pub fn l1_app_reboot_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_app::wasm_host;

    let mut failures: Vec<String> = Vec::new();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("没有 document"))?;
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("没有 body"))?;

    for round in 1..=3u32 {
        // ① 每轮用**独立**容器（避免上一轮的残留干扰断言）。
        let container = document
            .create_element("div")
            .map_err(|error| JsValue::from_str(&format!("建容器失败：{error:?}")))?;
        let container_id = format!("l1-app-reboot-{round}");
        container
            .set_attribute("id", &container_id)
            .map_err(|error| JsValue::from_str(&format!("设容器 id 失败：{error:?}")))?;
        body.append_child(&container)
            .map_err(|error| JsValue::from_str(&format!("挂容器失败：{error:?}")))?;

        // ② boot
        wasm_host::boot(&format!("#{container_id}"))
            .map_err(|error| JsValue::from_str(&format!("第 {round} 轮 boot 失败：{error:?}")))?;

        // ③ 改信号 ⇒ 显式 flush ⇒ DOM 更新（批边界路径）
        let text = format!("第{round}轮");
        wasm_host::set_demo_text(&text)
            .map_err(|error| JsValue::from_str(&format!("第 {round} 轮改信号失败：{error:?}")))?;
        wasm_host::flush()
            .map_err(|error| JsValue::from_str(&format!("第 {round} 轮 flush 失败：{error:?}")))?;
        let value = document
            .query_selector(&format!("#{container_id} input"))
            .ok()
            .flatten()
            .and_then(|element| {
                js_sys::Reflect::get(&element, &JsValue::from_str("value"))
                    .ok()
                    .and_then(|value| value.as_string())
            });
        if value.as_deref() != Some(text.as_str()) {
            failures.push(format!("第 {round} 轮渲染未生效：{value:?}"));
        }

        // ④ dispose ⇒ DOM 真的被卸载（**残留会渗进下一轮** ⇒ 必须每轮都干净）
        wasm_host::dispose_screen().map_err(|error| {
            JsValue::from_str(&format!("第 {round} 轮 dispose 失败：{error:?}"))
        })?;
        let leftover = document
            .query_selector(&format!("#{container_id} input"))
            .ok()
            .flatten();
        if leftover.is_some() {
            failures.push(format!("第 {round} 轮 dispose 后输入框仍在 DOM 里"));
        }
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

/// ⭐⭐⭐⭐ **规模基线探针**（「整树重建 + 差分 patch」的成本）。
///
/// ## 它回答什么
///
/// 当前引擎的更新路径是「**整树重建** → **差分 patch**」
/// ⇒ ⇒ 每次状态变更都要**重新构造整棵视图树**，再由 patch **逐节点比对**。
/// ⇒ ⇒ 成本随**树规模**线性增长 ⇒ ⇒ 大界面（长列表 / 大面板）会越来越贵。
///
/// ⇒ ⇒ 这里给出**可复跑的数字**：节点数、挂载耗时、单次更新耗时。
/// ⚠️ 这些数字**机器相关** ⇒ ⇒ **不作门禁**，只作**决策依据**
/// （「细粒度更新」值不值得做 ⇒ ⇒ 用数字说话，不靠猜）。
///
/// ## 顺带验证行为
///
/// 更新后**只有目标叶子的文本变了**，其余节点**逐字未变**
/// ⇒ ⇒ 证明差分 patch 的**语义**是对的（不是「整棵重刷」）。
#[wasm_bindgen]
pub fn l1_perf_probe() -> Result<JsValue, JsValue> {
    use apeireth_ui_core::Runtime;
    use apeireth_ui_dom::binding::Binding;
    use apeireth_ui_dom::dom::Dom;
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
    if let Some(body) = document.body() {
        let _ = body.append_child(&root);
    }

    // ⭐ 树规模：宽 × 深 ⇒ 元素数 ≈ WIDTH^DEPTH × 2。
    //   取 **8 × 4 ⇒ 约 8 千个元素**（够暴露「整树重建」的成本，又不至于把浏览器卡住）。
    const WIDTH: usize = 8;
    const DEPTH: usize = 4;

    let runtime = Runtime::new();
    let leaf = runtime.signal(String::from("初始"));
    let mut renderer = Renderer::new(dom, root);

    // 构树：递归生成（WIDTH 个兄弟 × DEPTH 层），**只有最深的一个叶子**带绑定。
    fn build(
        width: usize,
        depth: usize,
        leaf: &apeireth_ui_core::Signal<String>,
        first: bool,
    ) -> VNode {
        let mut children = Vec::with_capacity(width);
        for index in 0..width {
            if depth == 0 {
                children.push(
                    VNode::element("span")
                        .attr("data-i", index.to_string())
                        .build(),
                );
                if first {
                    children.push(
                        VNode::element("input")
                            .attr("id", "perf-leaf")
                            .bind(Binding::text(leaf.clone()))
                            .build(),
                    );
                }
            } else {
                children.push(build(width, depth - 1, leaf, first && index == 0));
            }
        }
        VNode::element("div")
            .attr("class", "perf-row")
            .child(VNode::Fragment(children))
            .build()
    }

    let view = || build(WIDTH, DEPTH, &leaf, true);

    // ⚠️ 计时用 `js_sys::Date::now()`（**毫秒**精度）⇒ ⇒ 不依赖 wasm32 的
    //   `Instant` 平台支持（那条路在部分 wasm 环境会 panic）。
    let start_mount = js_sys::Date::now();
    renderer
        .mount(view())
        .map_err(|error| JsValue::from_str(&format!("挂载失败：{error}")))?;
    let mount_millis = js_sys::Date::now() - start_mount;

    // ⭐ 单次更新（改一个叶子信号 ⇒ 整树重建 + 差分 patch）—— **拆成两段计时**：
    //   ①「构造」：Rust 侧重建整棵视图树（纯 CPU）；
    //   ②「patch」：差分比对 + 写 DOM（含 JS 边界往返）。
    //   ⇒ ⇒ 数字才**能指明该优化哪一半**（不猜）。
    let start_update = js_sys::Date::now();
    leaf.set(String::from("更新后"));
    renderer.reset_patch_stats();
    let start_build = js_sys::Date::now();
    let new_view = view();
    let build_millis = js_sys::Date::now() - start_build;
    renderer
        .patch(new_view)
        .map_err(|error| JsValue::from_str(&format!("打补丁失败：{error}")))?;
    let update_millis = js_sys::Date::now() - start_update;
    // ⭐⭐ patch 统计（**不抖**的观测量 ⇒ ⇒ 「收窄比对范围」可验证）
    let stats = renderer.patch_stats();

    // 行为断言：目标叶子确实更新了
    let leaf_value = document
        .query_selector("#perf-leaf")
        .ok()
        .flatten()
        .and_then(|element| {
            js_sys::Reflect::get(&element, &JsValue::from_str("value"))
                .ok()
                .and_then(|value| value.as_string())
        });
    if leaf_value.as_deref() != Some("更新后") {
        failures.push(format!("目标叶子未更新：{leaf_value:?}"));
    }

    // ⭐ 节点规模：**真实 DOM 节点数**（`document.querySelectorAll('*')`）
    //   ⇒ ⚠️ **不能**用 `.perf-row` 选择器计数当「元素数」⇒ ⇒ 那是**某类**元素的数量
    //   ⇒ ⇒ 与引擎的「比对节点数」**口径不同** ⇒ ⇒ 会得出错误的倍率（2026-11-01 踩过）。
    let total_elements = document
        .query_selector_all("*")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);
    let row_elements = document
        .query_selector_all(".perf-row")
        .ok()
        .map(|list| list.length())
        .unwrap_or(0);

    // ⭐⭐ **真实首屏形状**的同形状测量（2026-11-03；E-155）。
    //
    // 上面那段是**合成压测树**（WIDTH×DEPTH ⇒ 37,458 个真实 DOM 节点）
    // ⇒ ⇒ 它的「单次更新」数字**不代表首屏**（首屏是虚拟列表窗口化 ⇒ 只几十节点）
    // ⇒ ⇒ 这里**另测**首屏形状 ⇒ ⇒ 浏览器侧也有同形状数字。
    //
    // 形状与 `w0_first_screen()` 一致：容器 + 窗口内 N 行（行 = 1 元素 + 1 文本）。
    let screen_rows: usize = 22;
    let screen_nodes = screen_rows * 2 + 2;
    let screen_view = || {
        let rows: Vec<VNode> = (0..screen_rows)
            .map(|index| {
                VNode::element("div")
                    .attr("class", "row")
                    .attr("style", "height: 40px")
                    .child(VNode::text(format!("行 {index}")))
                    .build()
            })
            .collect();
        VNode::element("div")
            .attr("id", "w0-screen-root")
            .child(
                VNode::element("div")
                    .attr("class", "list")
                    .child(VNode::Fragment(rows))
                    .build(),
            )
            .build()
    };
    let screen_dom =
        WebDom::new().map_err(|error| JsValue::from_str(&format!("WebDom: {error}")))?;
    let screen_root = screen_dom
        .create_element("main")
        .map_err(|error| JsValue::from_str(&format!("建根失败：{error}")))?;
    let mut screen_renderer = Renderer::new(screen_dom, screen_root);
    screen_renderer
        .mount(screen_view())
        .map_err(|error| JsValue::from_str(&format!("首屏形状挂载失败：{error}")))?;

    // 改**一个叶子**（第 0 行的文本）⇒ 量一次 patch。
    //
    // ⚠️ 用 `performance.now()`（**亚毫秒**）而不是 `Date::now()`（**毫秒**）：
    // 首屏形状的 patch 只有几十微秒 ⇒ ⇒ 毫秒精度会**读出 0**（E-156 实测踩到）。
    // ⇒ 用 `js_sys::eval` 调浏览器的 `performance.now()`（`js-sys` 已有，
    // ⇒ ⇒ **不必**给 `web-sys` 加 `Performance` feature ⇒ ⇒ 不动依赖面）。
    //
    // ⚠️⚠️ **重复多次取中位数**：单次 `eval` 调用本身有开销（首次还要编译表达式）
    // ⇒ ⇒ 单次测量会把**测量开销**算进去 ⇒ ⇒ 本段跑 **9 次**取**中位数**。
    //
    // ⚠️⚠️⚠️ **浏览器侧亚毫秒计时的三次失败尝试**（E-156 全程留档）：
    //
    // ① `js_sys::eval("performance.now()")` 反复求值 ⇒ ⇒ 实测两次间隔恒为
    //    **0.1 µs** ⇒ ⇒ 读数是「精度被截断到同一 tick」的**假值**；
    // ② 改成 **eval 一次拿函数引用** + `Function::call0` ⇒ ⇒ 实测 `clock_probe = 0`
    //    ⇒ ⇒ `dyn_ref::<Function>` 在此环境下拿不到可调用对象 ⇒ ⇒ **恒 0**；
    // ③ 结论：**浏览器侧**在本环境下**拿不到可靠的亚毫秒时钟**
    //    ⇒ ⇒ 改用 **毫秒时钟 + 放大测量**（`Date::now()` 连续 N 次后相减再除以 N）
    //    ⇒ ⇒ N 大到累计量足够 ⇒ ⇒ 读数才有意义。
    //
    // ⚠️ 代价：**单次 patch 的精度 ≈ 总时长 / N** ⇒ ⇒ 数字要读作**量级**，不是精确值。
    let perf_now = js_sys::Date::now;
    //
    // ⚠️⚠️ **放大测量**：浏览器的 `performance.now()` 精度受**安全策略**限制
    //   （实测会被降到 ~100 µs 粒度）⇒ ⇒ 单次几十微秒的 patch **读出来是 0**。
    // ⇒ ⇒ **连做 N 次再计时、除以 N** ⇒ ⇒ 累计量够大 ⇒ ⇒ 读数才有意义。
    const SCREEN_ROUNDS: usize = 200;
    let mut screen_samples: Vec<f64> = Vec::with_capacity(9);
    for round in 0..9 {
        let start_screen = perf_now();
        let mut screen_changed = screen_view();
        if let VNode::Element(root_element) = &mut screen_changed {
            if let Some(VNode::Element(list)) = root_element.children.first_mut() {
                if let Some(VNode::Fragment(rows)) = list.children.first_mut() {
                    if let Some(VNode::Element(row)) = rows.first_mut() {
                        if let Some(VNode::Text(text)) = row.children.first_mut() {
                            // ⚠️ 每次改**不同**的文本 ⇒ 保证每次 patch 都真的有写入
                            //   ⇒ 否则第 2 次起 patch 树与旧树相同 ⇒ ⇒ 耗时趋近 0
                            //   ⇒ ⇒ 会把「无写入的 patch」误读成「首屏很快」（E-156 实测踩到）。
                            text.data = format!("行 0（改了 {round}）");
                        }
                    }
                }
            }
        }
        // ⚠️ **连做 N 次**（每次改不同文本 ⇒ 每次都有真实写入）⇒ ⇒ 计时够大才读得准。
        for inner in 0..SCREEN_ROUNDS {
            if let VNode::Element(root_element) = &mut screen_changed {
                if let Some(VNode::Element(list)) = root_element.children.first_mut() {
                    if let Some(VNode::Fragment(rows)) = list.children.first_mut() {
                        if let Some(VNode::Element(row)) = rows.first_mut() {
                            if let Some(VNode::Text(text)) = row.children.first_mut() {
                                text.data = format!("行 0（改了 {round}-{inner}）");
                            }
                        }
                    }
                }
            }
            screen_renderer
                .patch(screen_changed.clone())
                .map_err(|error| JsValue::from_str(&format!("首屏形状 patch 失败：{error}")))?;
        }
        // ⚠️ `perf_now` 现在是**毫秒** ⇒ ⇒ 单次 patch 的毫秒数。
        let elapsed = (perf_now() - start_screen) / SCREEN_ROUNDS as f64;
        screen_samples.push(elapsed);
        if round == 0 || round == 8 {
            // 首次含 `eval` 编译开销 ⇒ ⇒ 打印出来供判读
            let _ = js_sys::Reflect::set(
                &js_sys::Object::new(),
                &JsValue::from_str("noop"),
                &JsValue::from_f64(elapsed),
            );
        }
    }
    // ⭐ 取**中位数**（去掉首次的 eval 编译开销与偶发抖动）。
    screen_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let screen_update_millis_direct = screen_samples[screen_samples.len() / 2];
    // ⭐ 诊断：时钟**绝对值**与最小样本（供判读「时钟没走」还是「差值被吞」）。
    let screen_clock_probe = perf_now();
    let screen_min_sample = screen_samples[0];
    // ⭐ 保留**毫秒**（与既有字段同口径）**和**亚毫秒（首屏形状才读得准）。
    let screen_update_millis = screen_update_millis_direct;
    let screen_update_micros = screen_update_millis * 1000.0;

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
        &JsValue::from_str("width"),
        &JsValue::from_f64(WIDTH as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("depth"),
        &JsValue::from_f64(DEPTH as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("elements"),
        &JsValue::from_f64(total_elements as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("row_elements"),
        &JsValue::from_f64(row_elements as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("mount_millis"),
        &JsValue::from_f64(mount_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("update_millis"),
        &JsValue::from_f64(update_millis),
    );
    // ⭐ 真实首屏形状的同形状数字（E-155）：与上面的压测树数字**并列**。
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_rows"),
        &JsValue::from_f64(screen_rows as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_nodes"),
        &JsValue::from_f64(screen_nodes as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_update_millis"),
        &JsValue::from_f64(screen_update_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_update_micros"),
        &JsValue::from_f64(screen_update_micros),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_clock_probe"),
        &JsValue::from_f64(screen_clock_probe),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("screen_min_sample"),
        &JsValue::from_f64(screen_min_sample),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("build_millis"),
        &JsValue::from_f64(build_millis),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("compared_nodes"),
        &JsValue::from_f64(stats.compared_nodes as f64),
    );
    let _ = js_sys::Reflect::set(
        &result,
        &JsValue::from_str("written_nodes"),
        &JsValue::from_f64(stats.written_nodes as f64),
    );
    Ok(result.into())
}
