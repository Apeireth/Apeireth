# apeireth-ui-gpu

Apeireth 全栈纯 Rust 前端引擎的 **GPU 桥**：GLSL 前置变换 + uniform 绑定表 + 双后端渲染入口（状态 → 像素）。

## 在引擎里的位置

引擎依赖拓扑（ADR-0015 §3.3，单向下行无环）：

```
ui-macro(0) ← ui-core(1) ← {ui-dom(2), ui-gpu(2)} ← ui-kit(3) ← ui(4, 门面) ← ui-demo(5)
```

`apeireth-ui-gpu` 是**拓扑秩 2**，与 `ui-dom` **平级**（不是父子）：

- 只回答「**状态 → 像素**」；**不改着色器数学**（电影感载体属引擎之外，引擎一行都不碰）。

## 已实现（IMPLEMENTED：库级已实现，未接线进生产路径）

- **GLSL ES 3.0 → naga IR 前置变换器**：三步机械变换（删 `#version 300 es` 行 + 把 N 个 plain uniform **合并进一个 std140 uniform 块** + 必要时补 16 字节对齐的 padding 成员）⇒ **纯文本变换**，native 可单测，不需 GPU。*为什么必须合并成一块*：naga 给每个 uniform 一个独立 binding ⇒ WebGL2 后端每个着色阶段只有 12–14 个 uniform buffer（GLES3 规范下限 12）⇒「逐个注入 binding」在 WebGL 后端**必然失败**。
- **uniform 名 → 索引映射**：naga 会抹掉 uniform 名 ⇒ 引擎自维护映射表，按**索引**绑定。
- **双后端引擎入口**：`Engine::create(canvas)` 严格走 5 步（探测 → 建 Instance → 建表面 → 带 `compatible_surface` 申请 adapter → 申请设备），后端在碰画布之前定死。
- **显式色彩契约**：显式改选**非 sRGB** 表面格式（wgpu 默认把 sRGB 排在 `formats[0]` ⇒ 两个后端会画出不同像素）。
- **设备错误不 panic**：校验错误收进队列并以 `GpuError` 返回。
- **片元坐标语义修正**：GLSL ES 的 `gl_FragCoord` y 轴向上，而两个后端都向下 ⇒ 变换器在**源级**把坐标取法改成 `vec4(x, height - y, z, w)`（**不改任何表达式与数值**）。

## GPU 后端与目标

- **双后端**：`webgl` + `webgpu` **编译期共存、运行期二选一**（宿主能力探测）⇒ 引擎走 `new_instance_with_webgpu_detection`，**禁** `Backends::all()`。
- **着色器前端**：`glsl` 与 `wgsl` **必须同开**（上游 naga 30.0.1 的 GLSL 前端无 ES profile ⇒ 单独开 `glsl-in` 编译不过；本 crate 直接依赖恒开 `wgsl-in` 把该缺陷消掉）。
- **宿主目标**：浏览器 WebView；native 侧只跑纯逻辑单测（**native 侧不是画面验证**）。

## 画面级证据（逐像素对照链）

- `crates-ui/evidence/p5-gpu/`：同一 wasm 产物里 WebGL 与 WebGPU 各自用生产版 GLSL 跑出真画面，两张画布**逐像素完全一致**（判据与截图见目录内 `report.json` / `README.md`）。
- `crates-ui/evidence/p5-b/`：生产版场景类与引擎用**同一批 uniform 取值**画出的画面**逐像素对照**，两个后端各 **0 / 576,000** 像素不同（差分阈值：任一通道绝对差 > 2/255）。

## 未接线 / 未实测（诚实边界）

- **uniform 取值来源仍未接线（按设计如此）**：引擎**不知道**场景语义 ⇒ 取值必须由**宿主**喂。本批证明的是「同一批取值 ⇒ 同一张画面」，**不是**「引擎自己算得出取值」。
- **未在 Tauri WebView 内验证**：只实测过 Linux 无头 Chromium；macOS WKWebView / Windows WebView2 / Linux WebKitGTK **均未实测**。
- **顶点阶段的 uniform 未接线**：对「顶点阶段有 uniform」的着色器**显式报错**（不静默忽略）。

## Install

```toml
[dependencies]
apeireth-ui-gpu = "2.0.0-rc.4"
```

## License

Apache-2.0 OR MIT

## Links

- crate 目录：`crates-ui/apeireth-ui-gpu`
- 证据链：`crates-ui/evidence/p5-gpu/`、`crates-ui/evidence/p5-b/`
- 决策真源：`docs/adr/0015-pure-rust-ui-engine.md`
