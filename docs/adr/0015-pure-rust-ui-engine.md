# 0015. 全栈纯 Rust 前端引擎 —— 自研响应式 + DOM 绑定层，独立 workspace 立项

- **Status**: Accepted
- **Date**: 2026-10-24
- **Owners**: owner 拍板（2026-10-24 口述：「全栈纯 Rust，先试点，渲染引擎永远自己维护没问题，自研就单独立项」）；实施线 = 前端重构项目组
- **Supersedes**: —
- **契约影响**: `docs/architecture/contract.toml` §[workspace]（新增独立 workspace 登记）、§[vendored]（新增 `ui_engine` 族）、§[extension_points]（新增 UI 组件扩展点）；§[deps] **0 改动**（独立 workspace 不进主依赖图）
- **例外关联**: **无**（见 §6.2「破 0 引新外部 dep 纪律」的定性）
- **预算净额**: 依赖边 **±0**（主 workspace 生产边 0 增 0 减）/ 开关 **±0** / 主 workspace crate **±0**（30 members 不变）/ 新增独立 workspace crate **+7** / 文件尺寸 **±0**（主仓现有文件 0 行改动，除本文与配套文档）

---

## 1. 背景与问题

### 1.1 现状实测（2026-10-24，`frontend/companion-desktop/`）

| 项 | 实测值 |
|---|---|
| `.svelte` 文件 | 62 个 / 51,742 行（含 `.ts`） |
| `.ts` 文件 | 纯逻辑 TS（不 import svelte）**23,309 行** |
| 测试套件 | `tests/*.mjs` **84 个**，其中 **38 个用 `readFileSync` 读源码文本做断言**（源码镜像），47 个真 import 生产模块 |
| 巨型文件 | `SettingsView.svelte` **5,314 行** / `App.svelte` **3,608 行**（70 个 `$state`）/ `runtime.ts` **2,829 行** |
| Tauri Rust 宿主 | `src-tauri/src/` **13,476 行**（45 个 `#[tauri::command]`） |
| 场景层 | `scene/blackhole.ts` **1,037 行**（含 **2 条 GLSL 程序**：vertex `:131` / fragment `:256`）、`three` 0.186 + `@pixiv/three-vrm` 3.5.5 |

### 1.2 三个真问题

1. **渲染层与测试层被编译边界割裂**：`.svelte` 文件无法被 node 直接 import ⇒ 38 个测试只能读源码文本断言，钉的是**字面量与物理位置**（`app.includes('await deleteSession(id, {')`）。任何重构（哪怕纯搬移、语义等价）都会打碎它们。
2. **契约漂移风险**：前端手写 TS 类型（`runtime.ts` 2,829 行）对上 Rust 后端结构，字段名对不上时运行时才暴露。
3. **双语言工具链**：前端独立 pnpm workspace + 独立 `[workspace]` 的 Tauri crate，两套构建、两套依赖、两套 CI job。

### 1.3 为什么现在必须决策

前端是 V1 施工序里唯一未开工的一格（「上屏面兑现」），且 `capabilities.accountability`、`apeireth egress impact` 等本周新能力已无上屏面可挂（D9/D11 债）。**继续在 Svelte 上堆功能，会让「渲染层不可测」这个结构性问题随功能量线性放大。** 越晚换，迁移面越大。

---

## 2. 决策

**自研一个纯 Rust 的响应式 + DOM 绑定引擎，编译到 `wasm32-unknown-unknown`，跑在 Tauri 现有 WebView 宿主里。以独立 cargo workspace `crates-ui/` 立项，不进主 workspace。**

### 2.1 引擎边界（硬约束，写入契约）

> **引擎只做三件事：响应式调度 / DOM 绑定 / 过渡时序。**
>
> **不写 CSS 引擎、不写滤镜层、不碰 shader、不碰排版、不碰主题。**

**理由**：电影感（设计系统 §1「电影感，不是软件感」）的载体经实测全部在**引擎之外**——

| 层 | 载体 | 实测 | 引擎是否触碰 |
|---|---|---|:--:|
| GPU 场景 | `blackhole.ts` 1,037 行 / 2 条 GLSL | WebGL2 | ❌ 零改写 |
| 场景合成 | `SceneLayer.svelte` 全屏 canvas `position:absolute; inset:0` | CSS 定位 | ❌ 零改写 |
| 暗角 + 颗粒 | `#vignette` DOM 节点 + `shell.css:62` | CSS | ❌ 零改写 |
| 面板模糊 | `backdrop-filter: blur(8/10/12/14/20/24px)` **80 处** | CSS | ❌ 零改写 |
| 排版 | 衬线栈 / 行高 2.1 / 字距 0.13em / 幽灵编号 | CSS | ❌ 零改写 |
| 主题 | 14 套主题 + 三调性 + `--ap-*` 令牌 | CSS + `theme.ts` | ❌ 零改写 |

引擎替代的只是「谁决定 DOM 长什么样」——**上屏像素仍由 WebView 的 GPU + CSS 合成器产出**。

### 2.2 自研目标边界：不是实现 Svelte

> **口径修正（2026-10-24 实测复核，lead 亲验）**：本文初稿曾按「`on:` / `in:`」的 Svelte 4 语法统计特性使用面。**复核发现本仓用的是 Svelte 5 新语法**，Svelte 4 语法在本仓**几乎不存在**（`on:` 精确匹配仅 2 处、`in:`/`out:` 精确匹配 **0 处**）。下表是复核后的**准确口径**（统计范围：`frontend/companion-desktop/src/`，`.svelte` 文件）。

| 用到（须实现） | 处数 | 文件数 | 备注 |
|---|---:|---:|---|
| `{#if}` 条件块 | 377 | 48 | |
| `$state` | 376 | 45 | |
| `$derived` | 162 | 36 | |
| `{#each}` 列表 | 111 | 35 | |
| `$props` | 55 | 55 | |
| `$effect` | 40 | 19 | |
| `bind:` 双向绑定 | 70 | 27 | |
| **Svelte 5 事件属性（`onclick=` / `onkeydown=` 等）** | **403** | — | 分布：`onclick` 304 / `onkeydown` 41 / `oninput` 24 / `onchange` 13 / `onfocusout` 9 / 其余 12 |

| 不用（**不实现**） | 实测处数 |
|---|---:|
| `$bindable` / `{#await}` / `use:` / `context=` / `setContext` / `$inspect` | **0** |
| `{#snippet}` | 2（可退化为普通函数） |
| **Svelte 框架过渡指令（`transition:` / `in:` / `out:` 作为 Svelte 指令）** | **0** |
| `{#animate}` 动画块 / `animate:` 指令 | **0** |

**关键结论（本 ADR 最重要的技术事实）**：

> **本仓的 86 处 `transition:` 全部是「纯 CSS `transition` 属性」，不是 Svelte 过渡指令。**
> 例：`src/App.svelte:3295` `transition: opacity 0.8s ease;`、`src/App.svelte:3411` `transition: border-color 0.2s ease, color 0.2s ease, ...`。
>
> **这意味着：框架级的进入/离场过渡（`in:` / `out:`）在本仓用量为 0。**

**这大幅改变了风险画像**：我此前判断「277 处 `in:` 过渡是电影感的关键、须专门做过渡时序引擎」——**该判断基于错误的语法口径，现予更正**。

修正后的结论：

1. **动画责任完全在 CSS**（86 处纯 CSS `transition` + `transform` / `opacity` 关键帧）⇒ 落在引擎边界之外（§2.1），**引擎无需实现过渡时序引擎**
2. **引擎要实现的是「事件绑定」**（403 处 `on*` 属性），但这是 DOM 事件委托的常规工作，不涉及时序
3. **真正需要自研的只剩**：响应式调度（`$state` / `$derived` / `$effect`）+ DOM 差分 + 事件委托 + 双向绑定 + 组件模型

**工作量随之下降**：自研目标从「响应式 + DOM 绑定 + 过渡时序引擎」收敛为「**响应式 + DOM 绑定**」，引擎代码量估 9,000–18,000 → **7,000–14,000 行**。

**但结论不变**：CSS 动画责任在引擎外 ⇒ 视觉零改写的保证反而更强（动画根本不经引擎）。

### 2.3 视觉风险：动画责任完全在 CSS（引擎边界之外）

设计系统 §7.1 对动效有硬纪律（界面过渡 ≥2s easeInOutCubic、状态参数趋近指数平滑 rate 2.4/s、说话呼吸 2.8s 循环）。经复核（§2.2 口径修正），**这些动画在本仓全部由 CSS 实现**（86 处纯 CSS `transition`），**不经框架**。

因此：

- 引擎**不实现**过渡时序引擎（§2.2 修正后的收敛结论）
- **P3 试点保留「逐帧对照」验收门**（成本低、收益高）：因为动画虽不经引擎，但**组件挂载/卸载时机**仍可能影响 CSS 过渡的触发（例如条件块销毁过早 ⇒ 离场动画丢失）。逐帧对照正是为了钉住这一点。
- 电影感的真实载体仍是 §2.1 表中的 GPU + CSS 合成层，**引擎一行都不碰**

### 2.4 crate 划分（`crates-ui/` 独立 workspace，7 crate）

```
crates-ui/
├── apeireth-ui/         门面：re-export 各层
├── apeireth-ui-core/    响应式核心：信号 / 依赖图 / 调度 / effect
├── apeireth-ui-dom/     DOM 绑定：差分 / 事件委托 / 双向绑定 / 列表
├── apeireth-ui-macro/   过程宏 #[component]
├── apeireth-ui-gpu/     wgpu 桥（黑洞场景）
├── apeireth-ui-kit/     组件库
└── apeireth-ui-demo/    试点应用（审批卡）
```

### 2.5 形态留门：原生路径

第一阶段形态 = **Rust → wasm32 → Tauri WebView**。理由：Tauri 宿主 13,476 行（托盘 / 单实例 / 开机自启 / OS keychain / 透明窗 / 后端自拉起 / 快捷窗 / 桌宠穿透）**全复用**。

`apeireth-ui-gpu` 的 wgpu 抽象使**原生窗口路径**（弃 WebView）成为第二阶段选项，不需重写引擎。**第二阶段不在本 ADR 承诺范围**，另出 ADR。

### 2.6 P0 探测实测结论（2026-10-24，证据链见 §8.2）

P0 探测子线在 `target/ui-probe-scratch/`（独立 scratch，0 写主仓）实测 6 项，**结论对本 ADR 的成本模型有决定性影响**：

| # | 项 | 实测结论 |
|---|---|---|
| 1 | `wasm-bindgen` / `web-sys` 版本配对 | **`web-sys` 对 `wasm-bindgen` 是精确硬钉 1:1**：`0.2.129 ⟷ js-sys 0.3.106 ⟷ web-sys 0.3.106`。错配（0.2.129 + 0.3.104）在**依赖解析期**直接失败 |
| 2 | `wgpu 30.0.1` 在 wasm32 | ✅ `webgl` 与 `webgpu` 两个后端**均编译通过** |
| 3 | **naga 能否保住现有 GLSL** | ✅ **能，着色器主体不需要重写**。但需 **机械变换**（删 `#version 300 es` 行 + uniform 绑定声明改造）—— ⚠️ **2026-10-24 P5 实测修订：原记的「2 步」不足，见 §2.6.3** |
| 4 | 依赖树 | ✅ **极其干净**：wgpu-webgl 68 crate，**唯一多版本包只有 `syn`（2.0.119 + 3.0.9）**；`hashbrown`/`bitflags`/`itertools`/`miniz_oxide`/`ahash`/`crossbeam-*` 实测**未进树** |
| 5 | 产物体积 | wgpu-webgl **4,534,454 B** / wgpu-webgpu **2,860,060 B**（release .wasm，含真实导出非空壳） |
| 6 | `wasm-bindgen-cli` | ✅ 44.68 s 装上 0.2.129，wasm→js 转换 + node 加载实测通过 |

**三条由此产生的硬约束（写入 P5 GPU 阶段的前置设计）**：

1. **naga 的 GLSL 前端无 ES profile**（`naga-30.0.1/src/front/glsl/parser.rs:267` 只收 `#version 440|450|460` + `core`）⇒ 引擎须内置 GLSL ES 3.0 → naga IR 的前置变换。
2. **`glsl` feature 必须与 `wgsl` 同开**（上游 `naga-30.0.1/src/front/mod.rs:5` 把 `interpolator` 模块 gate 在 `spv-in`/`wgsl-in` 后，而 `front/glsl/variables.rs:467` 无条件调用 ⇒ 只开 `glsl-in` 编译不过）。**这是上游缺陷，30.0.1 未修。**
3. **naga 的 IR 把每个 GLSL uniform 变成独立 binding**（`layout(std140) uniform` 块，binding 0..15），再吐出时 **uniform 名被抹成 `_group_0_binding_N_fs`** ⇒ 引擎须自行维护 **uniform 名 → binding 索引**映射表（`blackhole.ts:337` 的 `UNIFORM_NAMES` 正好可用）。

**正面结论**：WebGL 路线**全程留在 GLSL ES，不经过 WGSL** ⇒ **着色器主体的光学数值零改动风险**，P5 的 GPU 阶段成本远低于初估。

### 2.6.2 第 2 轮探测：双后端共存 + 运行时选后端（2026-10-24，**首次画面级实测**）

第 1 轮只测到「编译期与解析期」。第 2 轮（scratch `target/ui-probe2-scratch/`，`README.md` 完整报告 + `run-all.sh` 一键复现）**在本机 Chromium 154 无头浏览器里真跑出画面**，并回答了「双后端能否共存 + 运行时能否按能力自动选」。

| # | 项 | 实测结论 |
|---|---|---|
| 7 | **双后端编译期共存** | ✅ **可行**。同一份 `src/lib.rs`（三配置逐字节相同），`features = ["webgl","webgpu","glsl","wgsl","naga-ir"]` → **exit 0 / 零 warning / 无 feature 冲突 / 无符号冲突**。共存成立的原因：`wgpu-30.0.1/build.rs:38-46` 的 `wgpu_core` cfg alias 含 `webgl`，`webgpu` 与 `wgpu_core` 两个 `#[cfg]` 块在 wasm 上同时在场 |
| 8 | **产物体积（同一份源码三配置，release wasm32）** | `webgpu-only` **3,017,454 B** / `webgl-only` **5,611,114 B** / **`both` 6,311,484 B**；共存相对 webgl-only **+700,370 B 裸 / +153,595 B gzip**；唯一多版本包仍只有 `syn`（2.0.119 + 3.0.9）。⚠️ 与第 1 轮数字（webgl 4,534,454 / webgpu 2,860,060 B）**不同工程**（本轮 naga 开了 `glsl-out`），量级一致，不构成矛盾 |
| 9 | **运行时自动回落** | ❌ **wgpu 不会自动回落**。后端在 **`Instance::new` 那一刻一次定死**（`wgpu-30.0.1/src/api/instance.rs:63-99`）；`request_adapter` 阶段**没有任何 WebGPU→WebGL 回落**（`backend/webgpu.rs:1661-1709`，null adapter → `backend/webgpu.rs:1001-1029` → `NotFound` 即终止） |
| 10 | **宿主能力判据** | ⚠️ **`navigator.gpu` 存在性是弱判据，不够**。实测本机 Chromium 154：`navigator.gpu` 存在（`[object GPU]`）但 `requestAdapter()` 返回 **null** ⇒ `is_browser_webgpu_supported() = false`。正确判据是 **`requestAdapter()` resolve 出非 null adapter**，即 wgpu 自己的实现（`wgpu-30.0.1/src/util/init.rs:79-99`） |
| 11 | **wgpu 官方推荐写法** | ✅ 用 **`wgpu::util::new_instance_with_webgpu_detection`**（`wgpu-30.0.1/src/util/init.rs:116-128`）—— 探测不通过时**由它**把 `Backends::BROWSER_WEBGPU` 从 `InstanceDescriptor` 里摘掉。wgpu-types 文档自己写明弱判据不够（`wgpu-types-30.0.1/src/instance.rs:30-42`：「the browser may define the `navigator.gpu` object, but be unable to create any WebGPU adapters」） |
| 12 | **真实浏览器画面** | ✅ **两个后端都渲染出画面**（真 surface → pipeline → draw → present，各 3 帧）。WebGL 路径 `backend=Gl`（ANGLE/SwiftShader，format `Rgba8UnormSrgb`）、WebGPU 路径 `backend=BrowserWebGpu`（format `Rgba8Unorm`），两张 canvas 截图**肉眼一致**（洋红三角形 + 深蓝 clear 色）。驱动方式：**系统 Chromium + 自写 CDP 驱动**（无需 Playwright / 无需下载浏览器 / 无 npm 安装） |

**由此新增的四条硬约束（写入 P5 GPU 阶段的实现规范）**：

4. **禁止直接 `Instance::new(Backends::all())`** —— 那正是实测会走进 `NotFound` 死路的写法（宿主有 `navigator.gpu` 但拿不到 adapter 时）。统一走 `wgpu::util::new_instance_with_webgpu_detection`。
5. **WebGL 后端没有 `compatible_surface` 就产不出 adapter**（`wgpu-hal-30.0.1/src/gles/web.rs:123-144` 在 `surface_hint == None` 时返回空）⇒ 必须**先建 surface，再带 `compatible_surface` 申请 adapter**。实测 `no_adapter_backends: Backends(GL)`。
6. **一个 canvas 只能绑一种 context 类型** ⇒ **后端必须在碰 canvas 之前定死**（先用 WebGPU 后端 `create_surface` 再用 WebGL 后端会报 `canvas already in use`）。
7. **`required_limits` 必须取 `adapter.limits()`** —— WebGL2 无 compute，`Limits::default()` 要 `max_compute_workgroups_per_dimension = 65535`，实测被拒（`LimitsExceeded`）。

**第 2 轮追加的 GLSL 链路实测（7 个测试全绿）**：本仓真实 2 条 shader（`blackhole.{vert,frag}` 逐字抽出）→ FRAG **16 uniform / 6 function** → naga IR **globals=18 / fns=6** → `naga::back::glsl::Writer`（`Version::Embedded{300, is_webgl:true}`）吐出 **17,099 字节 `#version 300 es`** → 再解析**成功 + 校验成功**。三点如实标注：

- ⚠️ **naga 自己吐出的 `#version 300 es`，naga 自己的 GLSL frontend 解析不了**（`front/glsl/parser.rs:267` + `:290` → `InvalidVersion(300)` / `InvalidProfile("es")`）⇒ **回环校验必须再剥一次 version 行**（第 1 轮报告说「能 re-parse」，那次也是先剥了 version —— 结论成立但条件此前未写明）。
- ⚠️ **回环不是等形变换**：re-parse 后是 globals=20 / fns=7（不是 18/6）—— naga GLSL backend 把每个 plain uniform 降级成独立匿名 std140 块、`hash21` → `hash21_`、`main` 拆成 `main_1` + `main` 并把中间量提升为全局。
- ⚠️ **uniform 名字必丢**（变 `_group_0_binding_<N>_fs`）⇒ **JS/Rust 侧必须按 binding index 绑定，不能按名字**。

**口径更正（lead 与 owner 讨论中的一句话，落到本 ADR）**：此前口头表述「WebGPU 在 Linux 没有」**不准确**。准确表述是：**宿主 WebView 的 WebGPU 支持度随引擎而异**（Chromium 系完整支持 / WebKit 系部分支持），且 **wgpu 的 WebGPU 后端在「有 `navigator.gpu` 但拿不到 adapter」的宿主上会直接失败而非回落**。这与 wgpu 自身的跨平台能力无关（wgpu native 端在 Linux 走 Vulkan 是一等公民）—— 受限的是「wasm 里跑 wgpu 时宿主 WebView 给不给 WebGPU」这一层。

**本轮的诚实边界**：

- ✅ **已升级**：从「编译期与解析期实测」升级到「**画面级实测**」（真实浏览器 + 真实 surface + 真实 draw + 截图）。
- ❌ **仍未实测**：**macOS WKWebView / Windows WebView2 / Linux WebKitGTK 上均未实测**（环境只有 Linux Chromium）。Safari 无 WebGPU，WKWebView 路径大概率只能落 WebGL2，**需在真机复验**。
- ❌ **色彩管线未对齐**：WebGPU 后端 `format=Rgba8Unorm` 与 WebGL 的 `Rgba8UnormSrgb` 不同，linear↔sRGB 转换需在 P5 显式处理。
- ❌ **未跑本仓真实黑洞场景**：第 2 轮渲染的是探针三角形，**不是** `blackhole.{vert,frag}` 的画面（那两条 shader 只做了 naga 解析/回环层面的验证）。

---

### 2.6.3 第 3 轮实测（P5 GPU 桥落地）：**X2 被实测推翻，改为 X3**

P5 把 GPU 桥真正实现出来时，**实测推翻了第 1/2 轮写下的「X1 + X2 两步机械变换」**中的 X2。

| # | 项 | 实测结论 |
|---|---|---|
| 13 | **X2（每个 uniform 逐个注入 `layout(binding=N)`）** | ❌ **在 WebGL2 后端必然失败**。本仓 16 个 uniform ⇒ 16 个 uniform buffer，而 **WebGL2 后端每阶段只有 14 个**（本机实测 14；GLES3 规范下限 12）。实测报错原文：`Too many bindings of type UniformBuffers in Stage ShaderStages(FRAGMENT), limit is 14, count was 16` |
| 14 | **X3（合并成一个 std140 uniform 块）** | ✅ **可行**。把 N 个 plain uniform **合并进一个** `layout(std140, binding=0) uniform EngineUniforms { … };` ⇒ 只需 1 个 binding。**成员名 / 表达式 / 数值逐字不动**（GLSL 允许块成员不带实例名直接访问，已用单测钉住 naga 前端确实支持）⇒ **着色器正文里的 `uRes` 等标识符不用改** |
| 15 | **块大小须为 16 的倍数** | ⚠️ WebGL2 要求 uniform 块大小是 16 的倍数，否则设备报 `BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED`。实测 72 字节被拒 ⇒ 引擎须按需补一个 `vec4` **padding 成员** |
| 16 | **两后端的表面格式不同 ⇒ 画面会不同** | ⚠️ wgpu 把 sRGB 格式排在 `formats[0]`（`wgpu-core-30.0.1/src/device/global.rs:56`）⇒ 直接取会让**同一片元值 0.35 在两后端变成字节 160 vs 89**。⇒ 引擎**显式改选非 sRGB 格式**（`Rgba8Unorm` + `raw_bytes`），并把该口径记进 `ColorProfile`。实测修正后：**两后端逐像素完全一致** |
| 17 | **画面级实测（生产版着色器）** | ✅ WebGL 后端（`backend=Gl`）与 WebGPU 后端（`backend=BrowserWebGpu`）**各自用生产版的 2 条 GLSL 跑出真画面**（黑洞 + 吸积盘 + 星野 + 环境蓝光），各 3 帧；两张画布**逐像素零差异**（**174,484 像素比较 / 0 个不同 / 最大通道差 0**）—— 主代理用 PIL **独立重算**复核（`getbbox()` = None，各通道极值 (0,0)） |

**由此产生的三条实现约束**：

5. **着色器变换链是 X1 + X3，不是 X1 + X2**（X2 保留为可测基准，但生产路径不用）。前置编译器仍只改**声明外壳**，**光学数值与表达式逐字不动**。
6. **uniform 块大小须补齐到 16 的倍数**（补 `vec4` padding 成员）—— 否则 WebGL2 后端直接拒绝。
7. **两后端必须显式选定同一种表面格式**（本仓取 `Rgba8Unorm` + `raw_bytes`）—— 否则同一片元值在两后端落到不同字节 ⇒ 逐帧对照会出现**假差异**。

**第 3 轮的诚实边界**：

- ✅ **已实测**：两个后端各自用**生产版 GLSL** 跑出真画面，且**逐像素完全一致**。
- ❌ **未实测**：macOS WKWebView / Windows WebView2 / Linux WebKitGTK **均未实测**（环境只有 Linux Chromium）。
- ❌ **P5-b 未做**：黑洞场景的**完整复刻**（uniform 取值来源 = 生产版的实时取值链、场景参数语义、**与生产画面逐帧一致**）属下一批 ⇒ **本轮不主张「与生产画面一致」**（证据里的 uniform 取值是一组固定可复现的取值）。
- ❌ **顶点阶段 uniform 未接线**（显式报错，不静默忽略）。

---

### 2.6.4 第 4 轮实测（P5-b 生产对照）：**引擎画面 == 生产版场景画面**，并抓到一个真缺陷

P5-b 把「生产版场景类（**只读引入**）在真实浏览器里跑出的画面」与「Rust 引擎用**同一批
uniform 取值**画出的画面」逐像素对照 ⇒ **两个后端各 0 / 576,000 像素不同**（差分阈值
任一通道绝对差 > 2/255）。**这是本项目迄今最强的一条画面级证据**：不是「探针三角形」、
不是「固定 uniform」，而是**生产版场景 + 生产版实际取值**。

#### 取值怎么对齐的（**不把业务语义搬进引擎**）
生产版的 uniform 取值链依赖 presence 状态机 / PAD bias / 相机动画 / 主题参数 —— 那些是
**宿主侧**的事，引擎不该知道（引擎里不出现业务名词）。⇒ 本轮的做法是：
**用 CDP 在页面 JS 运行前注入录制器**（钩 `WebGL2RenderingContext.prototype` 的
`getUniformLocation` / `uniform1f|2f|3f|4f` / `drawArrays`）⇒ **抓生产版自己那一帧真正用的
16 个值** ⇒ 原样喂给引擎 ⇒ 两侧同帧同值。

#### ⭐ 本轮抓到的真缺陷：`gl_FragCoord` 的 y 轴在**两个后端都被搞反**

- **现象**：首轮对照 **97.6% 像素不同**（562,044 / 576,000）；把引擎侧画面**上下翻转**后再比 ⇒
  **0 差异** ⇒ 差异**只有**「整幅镜像」一个成因。
- **根因**（上游事实，非本仓偏好）：
  - GLSL ES 3.0 规定 `gl_FragCoord` 原点在**左下角**、y 轴**向上**；
  - **WebGPU 后端**：`@builtin(position)` 原点在**左上角**（WebGPU 规范）⇒ y 向下；
  - **WebGL 后端**：wgpu-hal 的 GL 路径把主渲染**上下翻转**存进交换链纹理
    （`wgpu-hal-30.0.1/src/gles/web.rs:301-312` 原文：「GL's presentation is not flipped,
    but main rendering is. Therefore, we Y-flip the output positions in the shader, and also
    this blit.」）⇒ 片元里读到的 `gl_FragCoord.y` 与**画布上从上到下的行号**同向。
  ⇒ **两个后端都把 y 轴变成向下** ⇒ 同一份 GLSL 源在引擎里画出来**上下颠倒**。
- **修法**（引擎前置变换器，方言外壳层）：源级把 `gl_FragCoord` 换成
  `vec4(gl_FragCoord.x, height - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w)`
  （`height` = 表面高度）。**不改任何表达式与数值**，只改坐标的取法；顶点阶段不碰；
  **不插行** ⇒ 变换前后**行号一一对应**（错误信息仍指回原始行）；**词边界感知**
  （`myGl_FragCoord` / `gl_FragCoordExtra` 不受影响）。
- **意义**：**若没有这道逐像素对照，引擎会把上下颠倒的黑洞发出去** —— 而单测（native 纯逻辑）
  **永远抓不到它**，肉眼在缩略图上也未必立刻察觉。这是「视觉层零改写」这条纪律的**物理保证**
  在最后一环落地。

#### 由此产生的第 4 条实现约束

8. **片元坐标语义必须由引擎显式对齐**（本仓取「GLSL ES 的 y 向上」⇒ 源级翻转 `gl_FragCoord.y`）。
   **不修的后果不是「报错」而是「画面上下颠倒」** —— 这是最难靠日志发现的一类缺陷，
   只能靠**与参照实现逐像素对照**钉住。

#### 第 4 轮的诚实边界

- ✅ **已实测**：Linux 无头 **Chromium 154** + SwiftShader 软件适配器（WebGL 与 WebGPU **都是软件
  后端**）上，生产侧与引擎侧逐像素一致（两腿各 0 / 576,000）。
- ❌ **未在 Tauri WebView 内验证**（本轮是普通浏览器页面，不是 Tauri 窗口）。
- ❌ **未实测**：macOS WKWebView / Windows WebView2 / Linux WebKitGTK。
- ❌ **未做（按设计）**：把**取值来源**搬进引擎 ⇒ 引擎不知道构图比例 / 机位 / presence 档位 /
  时间线照明。本轮证明的是「**同一批取值 ⇒ 同一张画面**」这条**管道**，**不是**「引擎自己算得出取值」。
- ⚠️ **两个后端不是逐位相同**：WebGPU 后端个别轮次出现 `max_channel_delta = 1` 的像素
  （**低于**差分阈值 2 ⇒ 仍记「相同」）。本轮口径是「任一通道绝对差 > 2/255 记不同」
  （与 P3 / P5 一致），**不是**逐位相等的主张。
- ⚠️ **冻结时钟不等于冻结一切**：`?t0=` 只钉住 `tSec` 与 `rot`；`cur.bright` / `cur.turb` /
  `cur.pulse` 仍按真实 dt 平滑推进 ⇒ **取值一律实测抓取，不做任何假设**。
- ⚠️ **生产侧取值随本地时钟变化**（`uAmbStr` / `uTlBright` / `uTlGold` 来自时间线照明）⇒ 每轮数值
  不同，但每轮**自洽**（同一组值同时喂给两侧）。
- ⚠️ **截图口径**：用 `Page.captureScreenshot` 的 clip 截画布、dpr 固定 1 ⇒ 1:1 对应绘制缓冲；
  两侧走同一条截图/合成路径 ⇒ 口径可比。**未**用 `readPixels` 做第二来源交叉验证（登记为未做）。
- ❌ **CI job 未在 runner 上实测过**：首次执行可能暴露 runner 镜像差异（那时它会**红**，
  而红是真信号）。判定逻辑按**实际可验证的部分**：`webgpu_comparison_verifiable` 显式标注
  WebGPU 腿是否可验；不可验时**不是「通过」也不是「失败」**（记 `null` + 一句说明），
  判定退到 WebGL 腿（**必过**）。降级路径用 `--no-webgpu` 开关**可被跑到**（已实测：
  WebGL 0 / 576,000、WebGPU 腿记「不可验证」、verdict 仍 pass）。


**措辞校正（lead 亲验后改写，0 装诚实）**：`blackhole.ts` 的 **1,037 行是 TS 文件总行数**，其中绝大部分是 JS 胶水，**真正的 GLSL 是 2 条程序**（vertex `:131` / fragment `:256`，合计约 200 行 GLSL）。因此准确表述是：

- ✅ **「着色器主体不需要重写」成立且已实测**（2 条 GLSL 程序经 naga 往返后全部保住）
- ❌ **「1,037 行 GLSL 原样复用」不成立**（该数字是 TS 文件行数，不是 GLSL 行数）

**诚实边界**：第 1 轮为**编译期与解析期**实测。⚠️ **该边界已被第 2 轮部分推翻**（见 §2.6.2）：双后端共存、运行时选后端、以及**画面渲染**均已在真实无头 Chromium 里实测通过；但**其余宿主（WKWebView / WebView2 / WebKitGTK）仍未实测**，且**未跑本仓真实黑洞场景**。

---

## 3. O-6 三阶审查

### 3.1 总体最优

**问**：在「V1 上屏面兑现」这条当前波次里，自研渲染引擎是不是最优切入点？

**答**：不是唯一最优，但**在给定约束下是最优**。

**拒的 alternative A：继续在 Svelte 上堆功能（0 成本）**
- 拒理由：`SettingsView.svelte` 5,314 行 / `App.svelte` 3,608 行 / 70 个 `$state` 的状态泥潭，叠加 38 个源码镜像测试的地雷阵——**任何后续功能开发都在给一个测不了的结构加重量**。V1 要交付的是「上屏面兑现」，功能面会持续变大，结构债的现值只会升。
- 为什么不选它作为主路径：它把决策推给未来，而未来迁移面更大。O-6「等以后做是借口」。

**拒的 alternative B：直接换原生 GUI（egui/iced）**
- 拒理由：① 电影感载体（2 条手写 GLSL 程序）须转 WGSL 并重新调参（吸积盘 smoothstep 混合曲线 / 双多普勒增亮 / 色温梯度）；② 中文排版必须内嵌 CJK 字体（Noto Serif CJK SC ≈ 20MB+），而现状**零字体资产、全靠系统字体栈**；③ Tauri 13,476 行宿主能力（透明窗圆角透视 / 桌宠穿透 / OS keychain / 单实例）须重写；④ 与设计系统总纲「面板语言会把「他」降格成「它」」正面冲突；⑤ `egui` 官方文档明载默认字体**仅支持拉丁与西里尔**，CJK 需手工装字体。
- 为什么不选：它把「视觉是否退化」这个**不可逆的高风险**绑在立项第一天。

**拒的 alternative C：Leptos/Dioxus → WASM（用现成框架而非自研）**
- 拒理由：**本仓已经试过一次并归档**。`legacy/donor/apeireth-web` = Leptos 0.7 SSR + WASM，4,010 行，其 `src/lib.rs:6` 自述组件停在 stub（「R19+ 升级到 Leptos view! 时填」），未走完即随 v1 冻结归档。**这不是推测，是仓内既成事实。**
- 为什么不选：现成框架的组件模型与本项目「显影式空间界面」范式（`docs/design/00-PHILOSOPHY.md`）不匹配；且仍需承担 WASM 侧的 canvas 3D 集成成本。

**为什么本方案最优**：它是**唯一同时满足四个约束**的路径——① 源码 100% Rust ② 视觉层零改写（电影感不冒风险）③ Tauri 宿主全复用 ④ 组件成为 Rust 函数，测试基座可从"钉源码文本"升级为"钉行为"（直接解掉 D3 债与 38 个镜像测试的地雷）。

**代价诚实登记**：引擎永久自维护（owner 已明示接受）；工作量 7,000–14,000 行（§2.2 口径修正后收敛）；P4 是硬闸门，试点不过即停。

### 3.2 系统最优

**问**：在 `foundation → engine → capabilities → adapters` 的依赖图里，这个引擎放哪一层最合适？

**答**：**不进主依赖图**——独立 cargo workspace `crates-ui/`，与主 workspace 0 依赖边。

**拒的 alternative A：作为 `adapters` 层的一个 crate 进主 workspace**
- 拒理由：① 主 workspace 的 `Cargo.lock` 有 547 包、`deny.toml` 的 `multiple-versions = "deny"` 是业界顶尖标准策略；引擎的 wgpu/naga/wasm-bindgen 树（**实测包数待 P0 探测确认**）极可能引入 `hashbrown` / `bitflags` / `itertools` / `miniz_oxide` 等多版本，直接撞红；② 进主 workspace 就进 `contract.toml` 的 `[deps]` 层规则与 `[extension_points]`，成为主架构的长期负债；③ 主 workspace 的 arch_guard 门禁（30 crates / 111 边 / 尺寸棘轮）会立刻对引擎文件生效，与「引擎自由生长」的迭代节奏冲突。
- 为什么不选：把一个**尚在证明期**的东西写进**主架构契约**，违反契约棘轮原则（允许集合只减不增）。

**拒的 alternative B：作为 `adapters` 层的 crate 进主 workspace，但 exclude 出主门禁**
- 拒理由：这是 `contract.toml` `[vendored]` 段为图像家族设计的模式（`image_family` 族，R4a-1 只做主门禁隔离）。但图像家族是**已交付的静态内核**（129,828 行，已冻结），引擎是**持续迭代中的活代码**——用静态资产的门禁模式管活代码，会让「独立」名存实亡（代码在主 workspace 里，依赖边仍进主图）。

**为什么不选**：独立 workspace 是真独立——`cargo metadata` 不含它 ⇒ 主 `arch_guard` 0 影响（已实测：`arch_guard.py` 只扫根 workspace 的 `cargo metadata`）。代价是**引擎失去主门禁保护**，因此本 ADR 强制要求为 `crates-ui/` 建**独立门禁 CI**（`ui-engine-ci.yml`：check / clippy -D warnings / fmt --check / test 四 job）。

**系统位置结论**：引擎是**前端展示层的实现细节**，与主后端通过 HTTP/SSE 契约交互（`frontend-data-contract.md` 已定义）——它不是后端的一层，不该进后端的层规则。

### 3.3 架构最优

**问**：在单向依赖 + 扩展点约束下，公开 API 形状与 crate 边界是否最优？

**答**：7 crate 的划分按**依赖方向**而非功能命名：

```
ui-macro (proc-macro)  ←  最底层，零内部依赖
ui-core (响应式)        ←  只依赖 ui-macro（可选）
ui-dom  (DOM 绑定)      ←  依赖 ui-core
ui-gpu  (GPU 桥)        ←  依赖 ui-core（状态）+ wgpu
ui-kit  (组件库)        ←  依赖 ui-dom + ui-gpu
ui      (门面)          ←  re-export 全部
ui-demo (试点)          ←  依赖 ui + ui-kit
```

**单向下行，无环**。`ui-gpu` 与 `ui-dom` **平级**（GPU 场景与 DOM 是两个正交投影，不是父子）——这与 `00-PHILOSOPHY.md` §2「四个投影显影同一份状态」一致：presence 数据同时驱动 DOM 显影与 GPU 显影，两条路都只依赖 `ui-core` 的状态。

**拒的 alternative A：单一 crate（`apeireth-ui` 一个包全装）**
- 拒理由：响应式核心（可纯逻辑测试，不需 DOM）与 DOM 绑定（需 web-sys）必须分离，否则**核心层永远无法脱离 WebView 单测**——而核心层是整个引擎里最难测、边角最多的部分。这条边界是本 ADR 最核心的架构约束。

**拒的 alternative B：`ui-dom` 与 `ui-gpu` 合并为 `ui-render`**
- 拒理由：DOM 差分与 GPU 绘制是完全不同的失效模式（前者是逻辑 bug，后者是着色/时序 bug），合并后无法分别建立门禁与回归基线；且 GPU 路径在 P5 之前根本不启用，合并会让 P1/P2 被一个未启用的依赖拖累。

**公开 API 形状**：状态用**信号（Signal）** 而非闭包 getter 暴露；`ui-core` **0 依赖 web-sys**（保证可 native 单测）；`ui-dom` 是唯一接触 DOM 的一层。

**依赖纪律**：P0 阶段各 crate `[dependencies]` 只允许 workspace 内部 path 依赖；外部依赖（wgpu / wasm-bindgen / web-sys）的**引入时机与版本由 P0 探测结论决定**（P0 探测子线实测中），并逐个过 `deny.toml` 许可白名单。

---

## 4. 迁移计划（分步、每步可独立合并、可回滚）

详见 `docs/04-internal/ui-engine-plan.md` §3。骨架：

| 阶段 | 目标 | 关键验收门 |
|---|---|---|
| **P0** | 立项（本文 + 架构文档 + 骨架 + CI + 契约登记） | 本 ADR 落 accepted；`crates-ui/` 四门禁绿；主仓 `arch_guard --strict` 0 违规 |
| **P1** | `ui-core` 响应式核心（信号 / 依赖图 / 调度 / effect） | ≥20 例**真测试**（native 跑，不需浏览器）；`ui-core` 0 依赖 web-sys |
| **P2** | `ui-dom` 绑定层（差分 / 事件委托 / 双向绑定） | 差分正确性测试；事件委托语义测试（403 处 `on*` 属性）；CSS 过渡触发时机测试（组件挂载/卸载时机 ⇒ 动画是否丢失） |
| **P3** | `ui-macro` 过程宏 + 审批卡试点 | **像素对照 ≤1%** + **逐帧对照**（钉住 CSS 动画触发时机）+ **owner 电影感主观验收** |
| **P4** | 试点复盘 go/no-go | **硬闸门**：不过即停，不硬推 |
| **P5+** | 扩面（组件库 / 迁移 / GPU 场景） | 每批独立验收；另出 ADR |

**P3 试点组件选型**：**审批卡**（`ApprovalRequestCard`）——输入输出明确、有真后端（`/v1/apeireth/approval-requests` + SSE `approval_required`）、有现有 Svelte 版可逐像素对照、交互真实、且是范式 §6 原则 4「判断即仪式」的代表。

---

## 5. 回滚

| 阶段 | 回滚动作 | 回滚后状态 |
|---|---|---|
| P0 | `git revert` 独立 commit；删 `crates-ui/` 目录 + `ui-engine-ci.yml` | 主仓完全回到原状（主 workspace 0 改动） |
| P1–P2 | `git revert` 对应 commit | 同上；`crates-ui/` 停在上一阶段 |
| P3 | 同上 | 同上；**现有 Svelte 前端零改动**（试点是独立 demo，不动 `frontend/companion-desktop/`） |
| P4 后 | 停止推进，不回滚（引擎已交付但未接线） | 前端仍在 Svelte 上运行 |

**关键性质**：本方案**全程不修改 `frontend/companion-desktop/`**。Svelte 前端在引擎完全交付并通过 P3 对照之前，**一直是生产在跑的那一份**。因此任何阶段的回滚成本 = 一个 `git revert`，无生产影响。

---

## 6. 后果（Consequences）

### 6.1 正面

- **契约唯一真源**：Rust 结构 → UI 组件，编译期保证，消灭 TS 手写类型与 Rust 后端的漂移。
- **测试基座升级**：组件 = Rust 函数，`cargo test` 直接调 ⇒ 38 个源码镜像测试可逐步退休（D3 债的根因解法）。
- **源码 100% Rust**：node/pnpm 从构建链消失（`pnpm check` / `pnpm test` / `svelte-check` 全部退役）。
- **视觉零风险**：GPU + CSS 合成层 0 改写（§2.1 表）。
- **保留全部既有资产**：Tauri 宿主 13,476 行、2 条手写 GLSL 程序（`blackhole.ts:131/256`）、14 套主题、三调性、令牌体系。
- **可回滚**：主 workspace 全程 0 改动。

### 6.2 负面 / 接受的代价

| 代价 | 状态 |
|---|---|
| **引擎永久自维护** | owner 已明示接受（2026-10-24）。这是产品级长期承诺，不是技术债。 |
| **破「0 引新外部 dep」纪律** | 本仓 commit 模板必带项（`AGENTS.md` §4 / MANIFESTO §8.2「0 引新外部 dep (Cargo.lock 0 行 diff)」）。**owner 已拍板豁免**（2026-10-24：「全栈纯 Rust / 自研就单独立项」）。**定性（非架构例外）**：实测 `scripts/arch_guard.py` 的违规类别全集为 `direction / structural / size / caliber / duplicate / edge / env / exception` —— **不含外部依赖类别**，即该纪律是**commit 正文诚实声明**而非架构契约约束，故**不登记 `exceptions.toml`**（`exceptions.toml` 只收「必须暂时违反契约」的条目，登记一个不存在的架构违规是伪造账）。落点改为：①本 ADR §6.2 如实登记豁免事实；②计划书立专项债，逐个外部依赖过 `deny.toml` 许可白名单；③每批 commit 正文照常声明「0 引新外部 dep」的**例外明细**（引入了什么、为什么、许可为何合规）。 |
| **依赖树撞 `multiple-versions = "deny"`** | 独立 workspace 不进主图；但独立 workspace 内仍需 `deny.toml` 的 skip 登记（P0 探测结论回来后定）。 |
| **工作量 7,000–14,000 行** | 估时（主代理主观加权，非实测）。§2.2 口径修正后由 9,000–18,000 收敛而来（取消过渡时序引擎）。 |
| **与主线争人** | 独立立项 + 独立施工线，与问责闭环 / 发布收口并行。 |
| **「全栈纯 Rust」不等于「无 WebView」** | 第一阶段仍是 Tauri WebView 宿主（§2.5）；WebView 是系统组件，不是我们的代码。去掉它是第二阶段，另出 ADR。 |

### 6.3 后续 stage

- **P1 起**：每阶段独立施工线 + 独立 write_scope（不重叠），每批 1 commit。
- **P5+**：组件库补齐 → 逐面迁移（建议序：审批卡 → 工具卡 → 状态条 → 治理卷宗 → 设置页三页）。
- **第二阶段决策点**：P4 复盘时一并评估「是否走原生窗口路径」。

---

## 7. 验收（P0 批）

- [x] `python scripts/arch_guard.py --strict` 通过且**基线净额 ≤ 0**（主 workspace 30 crates / 111 边 0 改动）
- [x] `crates-ui/` 独立门禁四 job 绿：check / clippy `-D warnings` / fmt `--check` / test
- [x] `cargo deny check` 通过（新依赖树许可白名单 + 多版本 skip 登记）
- [x] 契约登记：`contract.toml` §[workspace] / §[vendored] / §[extension_points] 更新
- [x] 例外登记：**不适用**（破 0-dep 纪律非架构契约违规，理由见 §6.2；`exceptions.toml` 不收伪造条目）
- [x] 文档随行：`docs/design/ui-engine-architecture.md` + `docs/04-internal/ui-engine-plan.md`
- [x] LOCKED 5 项 0 触碰（9 哲学锚本体 / 14 键 / 3 不可变脊柱 / workspace.version / R11 baseline）
- [x] 0 触碰主 `frontend/companion-desktop/`

---

## 8. 参考与出处

| 事实 | 出处 |
|---|---|
| 引擎边界（视觉层零改写） | 实测 `frontend/companion-desktop/src/lib/scene/blackhole.ts:131,256`（2 条 GLSL）、`src/lib/design/shell.css:62`（#vignette）、`src/lib/design/base.css:416-417`（backdrop-filter 24px）、`src/lib/design/tokens.css:118-120`（字体栈） |
| Svelte 特性使用面 | 实测 `grep -rE` 于 `frontend/companion-desktop/src/`（2026-10-24） |
| Leptos 先例失败 | `legacy/donor/apeireth-web/src/lib.rs:6`（「组件 stub」）+ `legacy/donor/apeireth-web/Cargo.toml:55`（leptos 0.7） |
| 独立 workspace 先例 | `frontend/companion-desktop/src-tauri/Cargo.toml:1-4`（独立 `[workspace]` 隔离说明） |
| 独立门禁 CI 先例 | `.github/workflows/companion-desktop-ci.yml`（独立 workspace 三 job 结构） |
| 契约登记位置 | `docs/architecture/contract.toml` §[workspace] / §[vendored]（图像家族独立登记先例） / §[extension_points] |
| 例外预算与规则 | `docs/architecture/exceptions.toml:1-16`（权重 / 期限 / 净零规则） |
| wgpu GLSL 支持 | wgpu 30.0.1 官方文档（`glsl` feature，经 naga 转译）；P0 探测实测结论见 §2.6 与 `docs/04-internal/ui-engine-plan.md` §7 |
| 电影感定义与载体 | `docs/design/01-DESIGN-SYSTEM.md` §1 / §2 / §5 / §7.1；`docs/design/00-PHILOSOPHY.md` §2 / §6 / §7 |

### 8.2 P0 探测证据链

| 证据 | 位置 / 命令 |
|---|---|
| 探测工程与完整证据链 | `target/ui-probe-scratch/README.md`（**gitignore 目录，不入库**）；一键复验 `bash target/ui-probe-scratch/run-all.sh` |
| 从 `blackhole.ts` 逐字抽出的 VERT / FRAG | `target/ui-probe-scratch/assets/blackhole.{vert,frag}` |
| naga GLSL 行为测试（7 例，含 2 例「预期失败」断言） | `target/ui-probe-scratch/naga-glsl/` —— `cargo test` 实测 5 + 2 = **7 passed / 0 failed** |
| 版本硬钉矩阵 | crates.io 稀疏索引实测（`web-sys 0.3.104` 要求 `wasm-bindgen =0.2.127`；`0.2.129` ↔ `js-sys/web-sys 0.3.106`） |
| naga 上游缺陷定位 | `naga-30.0.1/src/front/glsl/parser.rs:267`（无 ES profile）、`src/front/mod.rs:5` + `src/front/glsl/variables.rs:467`（`glsl-in` 单独开编译不过）、`src/front/glsl/variables.rs:618`（uniform 必须 `layout(binding=N)`） |
| wgpu WebGL 后端吐 GLSL ES 的路径 | `wgpu-hal-30.0.1/src/gles/device.rs:364`（`naga::back::glsl::Writer`）—— 探测子线实测同一条路吐出 17,099 字节 `#version 300 es` 且能被 naga 重新解析校验 |
| 依赖树实测 | `cargo tree --duplicates` + `Cargo.lock` 解析：wgpu-webgl 108 条目 / 68 crate；唯一多版本 = `syn`（2.0.119 + 3.0.9）；`jni-sys` 双版本仅出现于 native 目标产物，wasm 侧 0 多版本 |
| 产物体积 | `cargo build --release --target wasm32-unknown-unknown` 实测字节数（§2.6 表） |

---

_本文档由前端重构项目组（总工：lead）撰写于 2026-10-24，经 owner 口述拍板「全栈纯 Rust / 先试点 / 永久自维护接受 / 单独立项」后转 Accepted。后续按本文与 `docs/04-internal/ui-engine-plan.md` 执行，O-6 doctrine 真兑现。_
