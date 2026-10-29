# P5-b · 生产对照 —— 引擎画面 vs 生产版场景画面（逐像素）

**一句话**：把**生产版场景类**（`frontend/companion-desktop/src/lib/scene/blackhole.ts`，
**只读引入，一行未改**）在真实浏览器里跑出的画面，与 Rust 引擎（`apeireth-ui-gpu`）
用**同一批 uniform 取值**画出的画面**逐像素对照** —— **两个后端各 0 / 576,000
像素不同**（差分阈值：任一通道绝对差 > 2/255）。

> 本批同时**发现并修掉了引擎的一个真缺陷**：两个后端都把 GLSL ES 的片元坐标
> y 轴搞反了 ⇒ 同一份着色器源在引擎里画出来是**上下颠倒**的。详见
> [§4 本批发现并修掉的缺陷](#4-本批发现并修掉的缺陷片元坐标-y-轴)。

---

## 1. 这一批到底证明了什么

| 命题 | 状态 | 依据 |
|---|---|---|
| 生产版场景类可在本页**按路径只读引入**（依赖链浏览器安全） | ✅ 实测 | 参考页直接 import 生产源，vite 只转译不打补丁 |
| 生产版**实际用的那一帧**的 16 个 uniform 取值可被**原样抓取** | ✅ 实测 | CDP `Page.addScriptToEvaluateOnNewDocument` 注入录制器，在 `drawArrays` 时刻快照 |
| 同一批取值 ⇒ 引擎画面与生产画面**逐像素一致**（WebGL 后端） | ✅ 实测 | 0 / 576,000 不同 |
| 同一批取值 ⇒ 引擎画面与生产画面**逐像素一致**（WebGPU 后端） | ✅ 实测 | 0 / 576,000 不同 |
| 引擎能**自己算出**这些取值 | ❌ **不做**（按设计） | 取值来源属**宿主侧**场景语义；引擎只做「uniform → 着色器」的管道 |

**口径**：「一致」= 同一批 uniform 取值 + 同一份着色器源 + 同一块画布尺寸 ⇒
两侧画布逐像素相同。**不含**「取值链一致」——那是宿主的事。

## 2. 文件

| 文件 | 内容 |
|---|---|
| `production.canvas.png` | **生产侧**画布（960×600，dpr=1 ⇒ 截图 1:1 无缩放） |
| `webgl.canvas.png` | **引擎侧**画布（WebGL 后端 / wgpu-core GLES） |
| `webgpu.canvas.png` | **引擎侧**画布（WebGPU 后端 / 浏览器 WebGPU） |
| `reference.page.png` / `engine.page.png` | 两页整页截图 |
| `uniforms.json` | **抓到的 16 个 uniform 取值**（原样，未四舍五入） |
| `report.json` | **结构化判据**（环境 / 两次运行的实测口径 / 逐像素结果 / 判据布尔） |
| `console.txt` | 浏览器控制台 + 页面错误 + 引擎逐步骤日志 |
| `drive.py` | CDP 驱动（**纯 websocket-client + Pillow**：无 Playwright / 不下载浏览器 / 不装 npm 包） |
| `build.sh` | 一键复跑（wasm → wasm-bindgen → vite build → 对照） |
| `web/` | vite 工程（参考页 = 生产场景；引擎页 = wasm 宿主） |
| `harness/` | 验证宿主 crate（wasm-bindgen 包装；**独立 workspace**，不进依赖图 / 不进 deny 图 / 不进 CI 门禁） |

## 3. 实测环境与判据

| 项 | 值 |
|---|---|
| 浏览器 | **Chromium 154.0.8037.92**（`/usr/bin/chromium`，`--headless=new`） |
| WebGL 适配器 | `ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero)), SwiftShader driver)` |
| WebGPU 适配器 | 浏览器 WebGPU（软件适配器，`--enable-unsafe-webgpu --use-webgpu-adapter=swiftshader`） |
| 编译目标 | `wasm32-unknown-unknown`（release） |
| 宿主目标 | `x86_64-unknown-linux-gnu` |
| 画布 | 960×600 设备像素，dpr = 1（CDP `Emulation.setDeviceMetricsOverride` 固定） |
| 冻结时钟 | `?t0=1.5`（生产版的调试钩子：`tSec = 1.5`、`rot = 0`） |
| 日期 | 2026-10-24 |

### 结构化判据（`report.json` → `criteria`）

| 判据 | 实测值 |
|---|---|
| `uniform_count` | **16**（名字与分量数逐项核对：`uRes`/`uCenter` 是 vec2，其余 14 个是 f32） |
| `reference_fallback` | `false`（生产侧走的是 WebGL2 路径，不是 Canvas 2D 降级） |
| `reference_canvas` / `reference_dpr` | `[960, 600]` / `1` |
| `webgl_frames_drawn` / `webgpu_frames_drawn` | `3` / `3` |
| `webgl_encoding` / `webgpu_encoding` | `raw_bytes` / `raw_bytes`（**两侧同一种字节编码**） |
| `webgl_surface_format` / `webgpu_surface_format` | `Rgba8Unorm` / `Rgba8Unorm` |
| `production_vs_engine_webgl.differing_pixels` | **0** / 576,000（占比 **0.000000**，上限 0.1%） |
| `production_vs_engine_webgl.max_channel_delta` | **0** |
| `production_vs_engine_webgpu.differing_pixels` | **0** / 576,000（占比 **0.000000**，上限 0.1%） |
| `production_vs_engine_webgpu.max_channel_delta` | **0 或 1**（见 §6 诚实边界） |
| `verdict` | **`pass`** |

## 4. 本批发现并修掉的缺陷（片元坐标 y 轴）

**现象**：首轮对照里，生产侧画面与引擎侧画面 **97.6%** 的像素不同。

**定位**：把引擎侧画面**上下翻转**后再比 ⇒ **0 差异**（`max_channel_delta = 0`）
⇒ 差异**只有**「整幅镜像」这一个成因 ⇒ 引擎把片元坐标的 y 轴搞反了。

**根因**（上游事实，非本 crate 的偏好）：

- GLSL ES 3.0 规定 `gl_FragCoord` 原点在**左下角**、y 轴**向上**；
- **WebGPU 后端**：`@builtin(position)` 原点在**左上角**（WebGPU 规范）⇒ y 轴向下；
- **WebGL 后端**：wgpu-hal 的 GL 路径把主渲染**上下翻转**存进交换链纹理
  （`wgpu-hal-30.0.1/src/gles/web.rs:301-312`：「GL's presentation is not
  flipped, but main rendering is. Therefore, we Y-flip the output positions
  in the shader, and also this blit.」）⇒ 片元里读到的 `gl_FragCoord.y` 与
  **画布上从上到下的行号**同向。

⇒ 两个后端都把 y 轴变成**向下**，而 GLSL ES 规定**向上** ⇒ 同一份 GLSL 源
在引擎里画出来的画面是**上下颠倒**的。

**修法**（引擎侧，`crates-ui/apeireth-ui-gpu/src/glsl.rs`）：在**前置变换器**里
做**方言外壳层**的坐标语义修正 —— 把 `gl_FragCoord` 换成
`vec4(x, height - y, z, w)`（`height` = 表面高度）。**不改任何表达式与数值**，
只改坐标的取法；顶点阶段不碰（顶点阶段没有 `gl_FragCoord`）。

修完后：生产侧 vs 引擎侧（WebGL）**0/576,000**，生产侧 vs 引擎侧（WebGPU）
**0/576,000** ⇒ 判据 `pass`。

## 5. 抓到的 16 个 uniform 取值（原样）

`uniforms.json` 的内容（一次实测样本；**未四舍五入**，按 JSON double 原样
往返喂给引擎）：

```json
{
  "uRes":       [960, 600],
  "uCenter":    [633.6, 330],
  "uScale":     [59.83743867835388],
  "uTime":      [1.5],
  "uRot":       [0],
  "uBright":    [0.32],
  "uTurb":      [0.12],
  "uPulse":     [0],
  "uIncl":      [0.31927116370834385],
  "uAzim":      [0],
  "uDiskAzim":  [0],
  "uDiskIncl":  [0.31927116370834385],
  "uAmbStr":    [0.7833872166649256],
  "uEclipse":   [0],
  "uTlBright":  [1.1723451876662836],
  "uTlGold":    [0.5691370308342909]
}
```

**取值的来源（供复核，不是本批的主张）**：

- `uTime = 1.5`、`uRot = 0` ← 生产版的 `?t0=` 冻结钩子；
- `uBright / uTurb / uPulse` ← 生产版的静息档基线（`0.32 / 0.12 / 0`）；
  **注意**：冻结时钟**只**钉住 `tSec` 与 `rot`，这三个值仍按真实 dt 平滑推进
  ⇒ 本次实测它们恰好停在基线值（目标值 = 初值 ⇒ 无位移），**不假设**它们恒定；
- `uIncl / uDiskIncl / uAzim / uDiskAzim / uScale` ← 机位预设 0 + 自主漂移在
  `t = 0` 处的取值；
- `uAmbStr / uEclipse / uTlBright / uTlGold` ← **本地时钟**对应的时间线照明
  ⇒ **随抓取时刻变化**（本批每次抓取都是一组自洽的取值）。

## 6. 复现

```bash
# 1) 构建 wasm 宿主（wasm32，独立 workspace）
cd crates-ui/evidence/p5-b/harness
cargo build --release --target wasm32-unknown-unknown --locked

# 2) wasm-bindgen（版本必须与 crates-ui 的 wasm-bindgen 依赖精确一致：0.2.129）
<path-to>/wasm-bindgen --target web --no-typescript \
    --out-dir ../web/public/engine/pkg \
    target/wasm32-unknown-unknown/release/p5b_harness.wasm

# 3) vite build（参考页 + 引擎页）
cd ../web && npx --no-install vite build
#    （无 node_modules 时复用生产前端的**只读**依赖目录：
#      ln -s ../../../../frontend/companion-desktop/node_modules node_modules）

# 4) 跑对照（CDP ⇒ 抓取值 ⇒ 渲染 ⇒ 逐像素差分）
cd .. && python3 drive.py --out ./evidence --web ./web/dist
```

`build.sh` 把 4 步串成一条命令。驱动会：起本地静态服务器 → 用系统 Chromium
打开参考页 → 注入 uniform 录制器 → 冻结生产侧渲染循环 → 抓 16 个取值 + 截生产
侧画布 → 把取值**原样**注入引擎页 → 两个后端各画 3 帧 → 截两张画布 → PIL 逐
像素差分 → 写 `report.json` / `uniforms.json` / `console.txt`。

⚠️ **两个已踩过的坑（复现时注意）**：

1. `public/` 是 **vite build 时**才拷进 `dist/` 的 ⇒ 改了 wasm 必须**重跑
   `vite build`**，否则引擎页会拿到**上一轮**的 wasm；
2. 静态服务器必须发 `Cache-Control: no-store`（驱动已加）⇒ 否则浏览器可能命中
   HTTP 缓存，把「换了 wasm」变成「没换 wasm」的假象。

## 7. 诚实边界

- ✅ **已实测**：Linux 无头 Chromium 154 + 软件适配器（SwiftShader）上，
  生产侧画面与引擎侧画面在两个后端**逐像素一致**（0/576,000）。
- ❌ **未实测**：macOS WKWebView / Windows WebView2 / Linux WebKitGTK；
  ❌ **未在 Tauri WebView 内验证**（本批是普通浏览器页面，不是 Tauri 窗口）。
- ❌ **未做**：把**取值来源**搬进引擎。引擎**不知道**构图比例、机位、presence
  档位、时间线照明 ⇒ 取值必须由宿主喂。本批证明的是「同一批取值 ⇒ 同一张画面」。
- ⚠️ **两个后端不是逐位相同**：WebGPU 后端偶尔出现 `max_channel_delta = 1`
  的像素（**低于**差分阈值 2 ⇒ 仍记「相同」）。本批口径是「任一通道绝对差
  > 2/255 记不同」，与 P3/P5 一致；**不是**逐位相等的主张。
- ⚠️ **冻结时钟不等于冻结一切**：`?t0=` 只钉住 `tSec` 与 `rot`；
  `cur.bright / cur.turb / cur.pulse` 仍按真实 dt 平滑推进（本次实测恰好停在
  基线值）。本批的取值**一律实测抓取**，不做任何假设。
- ⚠️ **生产侧取值随本地时钟变化**：`uAmbStr / uTlBright / uTlGold` 来自本地
  时刻 ⇒ 每轮抓取的数值不同（但每轮**自洽**：同一组取值同时喂给两侧）。
- ⚠️ **native 单测 ≠ 浏览器验证**：native 侧只覆盖纯逻辑（变换 / 映射 / 布局 /
  策略 / 坐标语义）；画面级结论**只**来自本目录。
- ⚠️ **截图口径**：画布用 `Page.captureScreenshot` 的 clip 截取，dpr 固定为 1
  ⇒ 截图 1:1 对应绘制缓冲；两侧走**同一条**截图/合成路径 ⇒ 口径可比。
- ✅ **生产前端全程只读**：只 `import` / 读，**零写入**（见 §8）。

## 8. 纪律声明

- **生产前端只读**：`frontend/companion-desktop/**` **零写入**（参考页按路径
  `import` 生产源；vite 只做 TS→JS 转译与打包）。
- **主仓 0 改动**：本批的写入只落在 `crates-ui/evidence/p5-b/**` 与
  `crates-ui/apeireth-ui-gpu/src/**`。
- **LOCKED 5 项 0 触碰**、**禁区 0 触碰**、**未 commit**。
