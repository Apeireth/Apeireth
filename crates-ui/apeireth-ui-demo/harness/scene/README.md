# 场景层语义 · 对照 harness（逐值 + 逐像素）

**一句话**：把**生产场景类**（`frontend/companion-desktop/src/lib/scene/blackhole.ts`，
**只读引入，一行未改**）在真实浏览器里跑出来的 **16 个 uniform 取值**，与
**Rust 场景层语义 crate**（`apeireth-ui-scene`）**自己算出来的**同一组取值
**逐值比较**，再用同一组取值把两侧的画面**逐像素比较**。

> 本 harness 证明的是整条链：
> **「状态 ⇒ 16 个 uniform ⇒ 画面」**在 Rust 侧**与生产侧完全一致**。

---

## 1. 这一批到底证明了什么

| 命题 | 状态 | 依据 |
|---|---|---|
| 生产场景类可在本页**按路径只读引入** | ✅ 实测 | 参考页直接 import 生产源，vite 只转译不打补丁 |
| 生产侧**实际用的那一帧**的 16 个 uniform 取值可被**原样抓取** | ✅ 实测 | CDP `Page.addScriptToEvaluateOnNewDocument` 注入录制器，在 `drawArrays` 时刻快照 |
| Rust 侧能**自己算出**同一组取值（不靠抓取值回放） | ✅ 实测 | `apeireth-ui-scene` 独立算 ⇒ 与生产侧**逐位相等** |
| 同一组取值 ⇒ 两侧画面**逐像素一致** | ✅ 实测 | 0 差异像素 / 1,296,000 |

## 2. 两条帧驱动路径（用例矩阵的两个维度）

| 路径 | 生产侧 | Rust 侧 | 确定性 |
|---|---|---|---|
| `static` | 减动效静态单帧（状态**直接**落到目标值、时钟 1.2、自转相位 0.35、**不施加**自主漂移） | `Scene::render_once()` | **完全确定**（与帧率无关） |
| `animated` | rAF 动画循环（状态**指数趋近**、机位**插值**、自主漂移；`?t0=` 冻结时钟与自转相位） | `Scene::advance(dt)` **重放生产侧实测抓到的帧时长序列** | **逐步一致**（同一段 `dt` ⇒ 同一串状态） |

⚠️ 动画路径的**状态平滑依赖真实帧间隔** ⇒ 两侧要一致，就必须喂**同一段帧时长**。
录制器因此额外抓了 `requestAnimationFrame` 的时间戳序列与起始时钟
⇒ 驱动脚本按生产侧**同一算式**（`(now - lastT) / 1000`，上限 50ms）还原出
`dt` 序列 ⇒ Rust 侧逐步重放 ⇒ 状态与机位**逐位**一致。

## 3. 文件

| 文件 | 内容 |
|---|---|
| `cases.json` | 用例矩阵（模式 × 时刻 × 机位 × 存在态） |
| `drive.py` | CDP 驱动（**纯 websocket-client + Pillow**：无 Playwright / 不下载浏览器 / 不装 npm 包） |
| `build.sh` | 一键复跑（wasm → wasm-bindgen → vite build → 对照） |
| `harness/` | 验证宿主 crate（wasm-bindgen 包装；**独立 workspace**，不进依赖图 / 不进 deny 图 / 不进 CI 门禁） |
| `web/reference/` | 参考页（生产场景，只读引入） |
| `web/engine/` | 引擎页（wasm 宿主：Rust 侧自算取值 → 引擎画布） |
| `../evidence/scene/` | 证据（`report.json` = 43 个用例的**全部**逐值 / 逐像素数据 + 代表性用例的画布 + 控制台日志） |

⚠️ **画布只留代表性用例**（3 个用例 × 生产侧 / 引擎 WebGL / 引擎 WebGPU 三张），
避免入库体积膨胀；**43 个用例的完整画布**可用 `drive.py` 一键重出（见 §4）。
**数字证据（`report.json`）是全量的**。

## 4. 复现

```bash
# 1) 构建 wasm 宿主（wasm32，独立 workspace）
cd crates-ui/apeireth-ui-demo/harness/scene/harness
cargo build --release --target wasm32-unknown-unknown --locked

# 2) wasm-bindgen（版本必须与 crates-ui 的 wasm-bindgen 依赖精确一致：0.2.129）
#    （本机默认在主仓的 target/ui-probe-scratch/bin/bin/；可用 WASM_BINDGEN=<路径> 指定）
<path-to>/wasm-bindgen --target web --no-typescript \
    --out-dir ../web/public/engine/pkg \
    target/wasm32-unknown-unknown/release/scene_harness.wasm

# 3) vite build（参考页 + 引擎页）
cd ../web && npx --no-install vite build
#    （无 node_modules 时复用生产前端的**只读**依赖目录：
#      ln -s ../../../../../frontend/companion-desktop/node_modules node_modules）

# 4) 跑对照（CDP ⇒ 抓取值 ⇒ 还原 dt 序列 ⇒ 引擎页自算取值 ⇒ 逐值 + 逐像素）
cd .. && python3 drive.py --out ../evidence/scene --web ./web/dist
```

`build.sh` 把 4 步串成一条命令。

⚠️ **两个已踩过的坑（复现时注意）**：

1. `public/` 是 **vite build 时**才拷进 `dist/` 的 ⇒ 改了 wasm 必须**重跑
   `vite build`**，否则引擎页会拿到**上一轮**的 wasm；
2. 静态服务器必须发 `Cache-Control: no-store`（驱动已加）⇒ 否则浏览器可能命中
   HTTP 缓存，把「换了 wasm」变成「没换 wasm」的假象。

## 5. 口径

| 项 | 值 |
|---|---|
| 浏览器 | **Chromium 154.0.8037.92**（`/usr/bin/chromium`，`--headless=new`） |
| 画布 | 1440×900 设备像素（CSS 尺寸 = 设备尺寸，dpr = 1，CDP `Emulation.setDeviceMetricsOverride` 固定） |
| 时区 | UTC（进程环境变量固定） |
| 逐值容差 | 浮点**逐位相等**优先；否则 ≤ `1e-6` |
| 逐像素阈值 | 任一通道绝对差 > `2/255` 记「不同」；判定上限 差异占比 ≤ 0.1% |

## 6. 诚实边界

- ✅ **已实测**：Linux 无头 Chromium 154 + 软件适配器（SwiftShader）上，
  场景取值链在真实浏览器里与生产侧**逐位相等**、画面**逐像素一致**。
- ⚠️ **native 单测 ≠ 浏览器验证**：native 侧只覆盖纯逻辑（取值算式 / 映射 /
  布局 / 插值 / 缓动）；「与生产侧一致」这条结论**只**来自本目录。
- ⚠️ **Canvas 2D 降级路径未纳入本对照**：生产场景类在无 WebGL2 时会切到
  Canvas 2D 静态渲染；本 crate 不含该路径（**wgpu 在无 WebGL2 时同样不可用**
  ⇒ 降级路径**无法**由本 crate 取代 ⇒ 仍留生产侧）。
- ⚠️ **未在 Tauri WebView 内验证**（本批是普通浏览器页面，不是 Tauri 窗口）。
- ⚠️ **未覆盖**：macOS WKWebView / Windows WebView2 / Linux WebKitGTK；
  片头火焰、粒子系统、`SceneLayer` / `PlanetLayer` 的组件包装（本批范围外）。
- ✅ **生产前端全程只读**：只 `import` / 读，**零写入**。
