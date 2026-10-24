# P5 · GPU 桥能力层 —— 真浏览器验证证据

**一句话**：同一个 wasm 产物里，**WebGL 后端**与 **WebGPU 后端**各自用**生产版
GLSL 着色器**跑出真画面（surface → pipeline → draw → present，各 3 帧），
两张画布**逐像素完全一致**。

> ⚠️ **这不等于「与生产画面逐帧一致」**：生产版的场景参数语义、uniform 取值
> 来源、画面比对属 **P5-b**。本证据只证明**能力层**（变换器 + 绑定表 +
> 双后端渲染入口 + 着色器资产）在真实浏览器里成立。

## 文件

| 文件 | 内容 |
|---|---|
| `webgl.canvas.png` | **WebGL 后端**（wgpu-core / GLES）的画布截图（482×362 裁切） |
| `webgpu.canvas.png` | **WebGPU 后端**（浏览器 WebGPU）的画布截图（482×362 裁切） |
| `page.png` | 整页截图（两个画布并排 + 控制台输出） |
| `report.json` | **结构化判据**（环境 / 两次运行的实测口径 / 判据布尔 / 像素对照结果） |
| `console.txt` | 浏览器控制台 + 页面错误 + 逐步骤日志 |
| `drive.py` | CDP 驱动（**纯 websocket-client**：无 Playwright / 不下载浏览器 / 不装 npm 包） |
| `harness/` | 验证宿主 crate（wasm-bindgen 包装；**独立 workspace**，不进依赖图） |

## 实测环境

| 项 | 值 |
|---|---|
| 浏览器 | **Chromium 154.0.8037.92**（`/usr/bin/chromium`，`--headless=new`） |
| WebGL 适配器 | `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)` |
| WebGPU 适配器 | 浏览器 WebGPU（软件适配器，`--enable-unsafe-webgpu --use-webgpu-adapter=swiftshader`） |
| 编译目标 | `wasm32-unknown-unknown`（release） |
| 宿主目标 | `x86_64-unknown-linux-gnu` |
| 日期 | 2026-10-24 |

## 结构化判据（`report.json` → `criteria`）

| 判据 | 实测值 | 含义 |
|---|---|---|
| `both_backends_created` | `true` | 两个后端都建出了引擎 |
| `webgl_no_error` / `webgpu_no_error` | `true` / `true` | 两边都没有报错 |
| `webgl_frames_drawn` / `webgpu_frames_drawn` | `3` / `3` | 各自真画了 3 帧 |
| `webgl_canvas_not_blank` / `webgpu_canvas_not_blank` | `true` / `true` | 画布非空白（非黑像素占比 0.979） |
| `webgl_encoding` / `webgpu_encoding` | `raw_bytes` / `raw_bytes` | **两边同一种字节编码**（显式色彩契约生效） |
| `encoding_parity` | `true` | 编码方式一致 |
| `backend_pixel_parity.differing_pixels` | `0` / 174484 | **逐像素零差异** |
| `backend_pixel_parity.max_channel_delta` | `0` | 最大通道差 = 0 |
| `verdict` | **`pass`** | 全部判据通过 |

## 三条实测事实（截图/日志之外的可复核结论）

1. **后端可用格式不同 ⇒ 画布字节会不同（色彩管线的根因）**
   - WebGL 后端：`available_formats = [Rgba8UnormSrgb, Rgba8Unorm, Rgba16Float]`
   - WebGPU 后端：`available_formats = [Rgba8Unorm, Bgra8Unorm, Rgba16Float]`
   - wgpu 会把 **sRGB 格式排在前面**（`wgpu-core-30.0.1/src/device/global.rs:56`
     的 `sort_by_key(|fc| !fc.format.is_srgb())`）⇒ 直接取 `formats[0]` 会在
     WebGL 后端多做一次 linear→sRGB 编码：**同一片元值 0.35 → 字节 160 vs 89**。
   - ⇒ 引擎**显式改选非 sRGB 格式**（[`select_surface_format`]），本证据里两边
     都落到 `Rgba8Unorm` + `raw_bytes` ⇒ 像素零差异。

2. **WebGL2 后端装不下 16 个独立 uniform buffer**（实测报错原文见 `console.txt`
   的历史轮次与本目录 README 的说明）：
   ```
   Too many bindings of type UniformBuffers in Stage ShaderStages(FRAGMENT),
   limit is 14, count was 16
   ```
   ⇒ 前置变换器必须把 N 个 plain uniform **合并进一个 std140 uniform 块**
   （X3），并补一个 16 字节对齐的 padding 成员（WebGL2 要求块大小是 16 的倍数：
   `the type given for group 0 binding 0 has a size of 72 … must have a size that
   is a multiple of 16 bytes`）。

3. **后端必须在碰画布之前定死**：一个 canvas 只能绑一种 context 类型。

## 复现

**一键**（推荐）：

```bash
cd crates-ui/evidence/p5-gpu && ./build.sh
```

⚠️ **产物必须落在本目录**：`index.html` 按**同目录相对路径**引用 `./p5_gpu_harness.js`，
而 `drive.py` 服务的正是本目录 ⇒ 把 wasm-bindgen 产物放到别处（如 `pkg/`）会得到 **404**，
且**画面判据会静默变 `null`** —— 看起来像「渲染失败」，实际是「文件没被找到」。
（2026-10-24 实测踩到；`build.sh` 已把这个约束固化。）

手动步骤（与 `build.sh` 等价）：

```bash
# 1) 构建验证宿主（wasm32，独立 workspace）
cd crates-ui/evidence/p5-gpu/harness
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir ../ \
    target/wasm32-unknown-unknown/release/p5_gpu_harness.wasm

# 2) 把 index.html 放到同一目录（页面本体见 crates-ui/evidence/p5-gpu/ 的说明）
#    —— index.html 需要从 p5_gpu_harness.js 引入 wasm 模块

# 3) 跑 CDP 驱动
cd crates-ui/evidence/p5-gpu
python3 drive.py --out ./evidence
```

`drive.py` 会：起一个本地静态服务器 → 用系统 Chromium（`--headless=new`）打开
页面 → 等两个后端各自画完 → 截**画布裁切**与**整页** → 逐像素分析 PNG →
写 `report.json` / `console.txt`。

## 诚实边界

- ✅ **已实测**：Linux 无头 Chromium 154 上，WebGL 与 WebGPU 两个后端各自用
  **生产版 GLSL 着色器**跑出真画面，且两张画布逐像素一致。
- ❌ **未实测**：macOS WKWebView / Windows WebView2 / Linux WebKitGTK。
- ❌ **未做**：与生产版画面的**逐帧一致**（uniform 取值来源 + 场景参数语义）——
  属 **P5-b**。本证据里的 uniform 取值是一组**固定可复现**的取值，不是生产版
  的实时取值链。
- ❌ **native 侧不是画面验证**：native 单测只覆盖纯逻辑（变换 / 映射 / 布局 /
  策略）；画面级结论只来自本目录。
