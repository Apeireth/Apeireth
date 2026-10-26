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

/// 已建的引擎 + 管线（**可带一条精灵管线**：片头是**两个 pass**）。
struct EngineEntry {
    /// 引擎本体。
    engine: Engine,
    /// 主 pass 的管线（背景 / 全屏三角形）。
    pipeline: ScenePipeline,
    /// 主 pass 的 uniform 取值。
    values: UniformValues,
    /// 精灵 pass（`GL_POINTS`）的管线与取值 —— `None` = 只有主 pass。
    sprite: Option<(ScenePipeline, UniformValues)>,
}

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
            entry
                .engine
                .draw_frame(&mut entry.pipeline, &entry.values)
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
                pipeline,
                values,
                sprite: None,
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

/// ⭐ 片头背景 matte 的 uniform 取值（生产 `fire.ts` 的 `BG_UNIFORMS`）。
///
/// 按**索引**（= 声明序）写入引擎的 uniform 集合 ⇒ 与引擎的
/// `glsl::fragment_uniform_block_members` 成员序**必然一致**。
#[wasm_bindgen]
pub async fn run_intro(
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

    let scene = SceneDescriptor::new(
        INTRO_BACKDROP_VERTEX_GLSL,
        INTRO_BACKDROP_FRAGMENT_GLSL,
    )
    .with_vertex_data(FULLSCREEN_TRIANGLE.to_vec());
    let mut pipeline = engine
        .create_scene_pipeline(&scene)
        .map_err(|e| JsValue::from_str(&format!("pipeline: {e}")))?;

    // uniform：按**索引**（= 声明序）写 ⇒ 名字 → 索引走引擎自维护的表。
    let mut values = UniformValues::new();
    let mut used: Vec<String> = Vec::with_capacity(INTRO_BACKDROP_UNIFORM_NAMES.len());
    for (index, name) in INTRO_BACKDROP_UNIFORM_NAMES.iter().enumerate() {
        let components = read_uniform(&uniforms, name)?;
        values.set(index as u32, &components);
        used.push(format!("\"{name}\":{}", format_components(&components)));
    }

    // ⭐ 火焰粒子 pass（**可选**）：生产侧同一帧还有一层 `GL_POINTS`。
    //   - 顶点阶段有 40 个 uniform ⇒ 走引擎的**顶点 uniform 块**（binding = 1）；
    //   - 顶点属性是 `vec4`（`aSeed`，4096 个）⇒ stride 16；
    //   - 拓扑是**点列表** ⇒ 只在 WebGL（GLES）后端可用（WebGPU 没有点尺寸）；
    //   - 混合是**加法**（`ONE/ONE`）⇒ 与生产的 `gl.blendFunc(ONE, ONE)` 同义。
    let mut sprite_pipeline: Option<ScenePipeline> = None;
    let mut sprite_values: Option<UniformValues> = None;
    if let Some(sprite_uniforms) = js_sys::Reflect::get(&uniforms, &JsValue::from_str("__sprite"))
        .ok()
        .filter(|value| !value.is_null() && !value.is_undefined())
    {
        let sprite_scene = SceneDescriptor::new(
            INTRO_SPRITE_VERTEX_GLSL,
            INTRO_SPRITE_FRAGMENT_GLSL,
        )
        .with_vertex_data(apeireth_ui_scene::intro::sprites::sprite_seeds())
        .with_vertex_format(VertexFormat::Vec4)
        .with_topology(Topology::PointList)
        .with_blend(BlendMode::Additive);
        let pipeline = engine
            .create_scene_pipeline(&sprite_scene)
            .map_err(|e| JsValue::from_str(&format!("sprite pipeline: {e}")))?;
        let mut values = UniformValues::new();
        for (index, name) in INTRO_SPRITE_UNIFORM_NAMES.iter().enumerate() {
            let components = read_uniform(&sprite_uniforms, name)?;
            values.set(index as u32, &components);
        }
        sprite_values = Some(values);
        sprite_pipeline = Some(pipeline);
    }

    let mut drawn = 0u32;
    let mut last = None;
    for _ in 0..frames.max(1) {
        let frame = match (&mut sprite_pipeline, &sprite_values) {
            (Some(sprite_pipeline), Some(sprite_values)) => {
                let mut passes = [
                    ScenePass {
                        pipeline: &mut pipeline,
                        uniforms: &values,
                        vertex_uniforms: None,
                    },
                    ScenePass {
                        pipeline: sprite_pipeline,
                        uniforms: sprite_values,
                        vertex_uniforms: Some(sprite_values),
                    },
                ];
                engine
                    .draw_passes(&mut passes)
                    .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?
            }
            _ => engine
                .draw_frame(&mut pipeline, &values)
                .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?,
        };
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
                sprite: match (sprite_pipeline, sprite_values) {
                    (Some(pipeline), Some(values)) => Some((pipeline, values)),
                    _ => None,
                },
            },
        );
    });
    Ok(format!(
        concat!(
            "{{\"backend_requested\":\"{}\",\"backend_actual\":\"{}\",",
            "\"surface_format\":\"{}\",\"encoding\":\"{}\",",
            "\"width\":{},\"height\":{},\"frames_drawn\":{},",
            "\"vertex_count\":{},\"uniform_members\":{},",
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
        frame.uniform_members,
        used.join(","),
    ))
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
            match &mut entry.sprite {
                Some((sprite_pipeline, sprite_values)) => {
                    let mut passes = [
                        ScenePass {
                            pipeline: &mut entry.pipeline,
                            uniforms: &entry.values,
                            vertex_uniforms: None,
                        },
                        ScenePass {
                            pipeline: sprite_pipeline,
                            uniforms: sprite_values,
                            vertex_uniforms: Some(sprite_values),
                        },
                    ];
                    entry
                        .engine
                        .draw_passes(&mut passes)
                        .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
                }
                None => {
                    entry
                        .engine
                        .draw_frame(&mut entry.pipeline, &entry.values)
                        .map_err(|e| JsValue::from_str(&format!("draw: {e}")))?;
                }
            }
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
    let mut out = String::from("{\"beats\":");
    out.push_str(&frame.sprites.len().to_string());
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
    for (index, sprite) in frame.sprites.iter().enumerate() {
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
