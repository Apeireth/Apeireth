# M 轴 · 3D 在场能力 · 外部候选仓库可行性侦察（2026-10-18）

> 侦察对象：`https://github.com/ooolabdev/ooosplat`（Apache-2.0，2k★ 量级）
> 取材方式：按 [github-fetch-guide.md](../04-internal/github-fetch-guide.md) §〇 选路；本轮实走「节点池浅克隆」+「直连 curl」双通道，附录给实测。
> 边界：仅读源码与元数据，**未编译、未运行**该仓库任何程序；未改动本仓库任何代码。

## 结论（先行）

- **不建议搬**（作为伙伴的 3D 在场表现层）：该仓库**不含 Rust 渲染器**——光栅化、排序、shader 全在 TypeScript + PlayCanvas（`node_modules` 依赖），Rust 侧 0 行 GPU 代码、0 个 shader 文件，搬过来拿不到任何一帧画面。
- **建议只参考**：值得抄的是它的「引擎编排 + 项目/任务状态机 + PLY 流式后处理 + 多格式导出」这套工程做法，不是它的渲染层。
- 许可：**Apache-2.0 + NOTICE + 独立商标政策**（修改版不得沿用 OOOSplat 名字/图标）；随包第三方为 MIT/BSD-3/LGPL-3.0/MPL-2.0 混合，但引擎二进制不入 Git。
- 规模：**单 crate、无 workspace**，Rust 63 文件 / **35,057 行**，TypeScript 141 文件 / **17,175 行**。
- 最大风险：**M 轴核心能力（画面）在本仓库不存在**；其次是 GPU 无 CPU 回退（纯 CPU/无独显机器直接不可用）。

---

## 1. 仓库现状

**默认分支** `main`；**最近提交** `410859b91850a7b3591878809f3bf11febe3fb64`，`2026-10-18 14:13:40 +0800`，标题 `feat: export Gaussian models as PLY, compressed PLY, SPZ and SOG`；**star 2031**；**许可 Apache-2.0**。

复现命令（本轮实测）：

```powershell
git clone --depth 1 https://ghfast.top/https://github.com/ooolabdev/ooosplat.git   # 7,219 ms / 392 文件
git -C ooosplat log -1 --format='%H %ad %s' --date=iso
curl.exe -sS https://api.github.com/repos/ooolabdev/ooosplat        # 直连 200
```

API 返回（2026-10-18 14:44 前后 +08:00 取的快照）：

| 字段 | 值 |
|---|---|
| `stargazers_count` | 2031 |
| `forks_count` / `subscribers_count` | 243 / 10 |
| `open_issues_count` | 24 |
| `created_at` | 2026-08-15T15:48:10Z |
| `pushed_at` | 2026-10-18T06:13:45Z |
| `license.spdx_id` | Apache-2.0 |
| `language` / `topics` | Rust / `3d`, `gaussian-splatting` |
| `size` | 12,724 KB |
| `archived` | false |

**维护活跃度**：仓库 2026-08-15 建，约 9 周；`stats/participation` 近 10 周每周提交数为 `1,7,7,6,10,5,1,22,26,8`（`owner` 52 周合计 83）；`/commits?per_page=50` 显示 50 次提交落在 **2026-10-02 → 2026-10-18**，近 7 天 9 次，作者只有 `ooolabdev` 一个账号。

**LICENSE 开头三行（逐字，含行首空格；第 1 行为空行）**：

```text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/
```

**NOTICE 全文 7 行**（关键两条）：

```text
OOOSplat
Copyright 2026 ooolabdev

Licensed under the Apache License, Version 2.0.
...
The Apache License does not grant permission to use the OOOSplat name, logo,
icons, or other brand features as trademarks. See TRADEMARK_POLICY.md.
```

**活跃度判断**：上游很活跃（9 周 2k★，近 30 天 50 次提交），但 **bus factor = 1 个组织账号**，版本仍在 `0.6.1`（ROADMAP 自述「最新正式发布版本 0.5.0」），API/目录结构变动风险高。

---

## 2. 代码规模

```powershell
git ls-files | Measure-Object -Line                 # 392
(git ls-files '*.rs').Count                         # 63
git ls-files '*.rs' | % { (Get-Content $_ | Measure-Object -Line).Lines } | Measure -Sum   # 35,057
(git ls-files '*.ts' '*.tsx').Count                 # 141
git ls-files '*Cargo.toml'                          # 仅 src-tauri/Cargo.toml
```

- **workspace 数量 = 0**：全仓只有一个 `Cargo.toml`（`src-tauri/`），`package.name = "ooo-splat"`，`version = "0.6.1"`，**单 crate**（`lib` + 两个 `bin`：`ooo-splat`、`splatstudio`）。任务书假设的「workspace 有几个 crate」在本仓库不成立。
- 量级：Rust **35,057 行 / 63 文件**；TypeScript **17,175 行 / 141 文件**（`src/` + `html-viewer/` + `server/`）；`.mjs` 脚本 38 个；`Cargo.lock` 610 个包。

Rust 侧模块分布（`git ls-files 'src-tauri/src/*.rs'` 按二级目录汇总；表内合计 62 文件 / 35,034 行，另加 `src-tauri/build.rs` 1 文件 / 23 行，整体为 63 文件 / 35,057 行）：

| 模块 | 文件 | 行数 | 职责一句话 |
|---|---:|---:|---|
| `pipeline/` | 8 | 7,226 | 视频/图片→COLMAP→Brush→PLY 的阶段编排、进度、效果统计 |
| `engines/` | 8 | 4,965 | 外部引擎（ffmpeg / COLMAP / Brush）探测、版本校验、进程启动与 GPU 选择 |
| `commands/` | 3 | 4,442 | Tauri IPC：任务、模型导出、离线 HTML 导出 |
| `tasks/` | 4 | 3,516 | 任务队列、日志、通知、测试 |
| `project/` | 4 | 2,160 | 项目目录、元数据、SQLite 目录（catalog） |
| `reconstruction/` | 7 | 2,074 | **PLY 解析/校验/流式裁剪变换导出**、COLMAP 模型读取 |
| `video/` | 5 | 1,841 | ffprobe 探测、抽帧计划、图片序列 |
| （根） | 5 | 1,803 | `lib.rs` / `error.rs` / `imports.rs` 等 |
| `telemetry/` `diagnostics/` `presets/` `process/` `mcp/` `planner/` `updates/` `bin/` | 18 | 7,007 | 遥测、诊断、质量档位、进程资源、MCP 服务、补帧规划、更新检查、CLI |

---

## 3. 架构分层（搬不搬得动的关键）

| 层 | 载体 | 位置 | 是否可搬 |
|---|---|---|---|
| **纯计算/核心** | Rust（`reconstruction/`） | `src-tauri/src/reconstruction/` 2,074 行 | ✅ 无窗口、无 GPU，唯一真正可独立搬的部分 |
| 编排/状态机 | Rust（`pipeline/` `tasks/` `engines/` `project/`） | 约 1.8 万行 | ⚠️ 与外置引擎强耦合（ffmpeg/COLMAP/Brush 命令行契约） |
| **窗口/事件循环/UI** | React 19 + Tauri（`tao`/`wry`/WebView2） | `src/` 131 文件 + `src-tauri` Tauri 胶水 | ❌ 桌面 App 形态，非库 |
| **GPU 后端** | **TypeScript + PlayCanvas**（WebGL2 / WebGPU） | `package.json` 的 `playcanvas 2.21.4`、`@playcanvas/react 0.11.5`、`@playcanvas/splat-transform 3.10.1` | ❌ 光栅化代码不在仓库里（在 npm 依赖中） |
| 训练 GPU | 外置二进制 **Brush**（Vulkan/Metal） | `engines/manifest*.json` 下载，不入 Git | ❌ 第三方程序，非源码 |

**关键事实**：`src-tauri/Cargo.toml` 的依赖表里没有任何渲染栈——`tauri 2`、`rusqlite 0.32.1(bundled)`、`rmcp =3.5.1`、`axum =0.8.9`、`reqwest 0.12(rustls)`、`image 0.25`、`sysinfo =0.39.6`、`windows-sys 0.61`；`Cargo.lock` 610 个包里 **没有 `wgpu`/`wgpu-core`/`wgpu-hal`/`naga`/`ash`/`glow`**，只有 `tao`/`wry`（窗口与 WebView）。

```text
$ Select-String src-tauri/Cargo.lock '^name = "(wgpu|winit|ash|glow|naga|tao|wry)"'
name = "tao"
name = "wry"
```

Rust 中 `wgpu` 字样共 12 处命中：11 处是**字符串字面量**（传给 Brush 的环境变量名与日志过滤串）、1 处是 `catalog.rs:652` 的 `trash::delete_all` 误命中 `ash`；**0 处依赖、0 处 GPU API 调用**：

```text
brush.rs:14:  "brush_cli=info,brush_process=info,cubecl_wgpu=info,burn_wgpu=info"
brush.rs:86:  OsString::from("CUBECL_WGPU_DEFAULT_DEVICE"),
runtime.rs:275: tracker.line("GPU initialization completed in 1s: backend=Vulkan device=NVIDIA RTX ...")
```

---

## 4. wgpu / wgpu 版本 / GPU 依赖 / CPU 回退

- **我们没有 wgpu 可搬**：仓库无 `wgpu` 依赖（上节证据），无 `wgpu` 版本可言。
- **窗口/事件循环**：Tauri 2（`tao` + `wry` + WebView2 Runtime，README「支持 WebView2 Runtime」）。
- **GPU 硬绑路径**：
  - 预览：PlayCanvas 必须拿到 **WebGPU 或 WebGL2**，否则直接抛错——`html-viewer/viewer.ts:69`（摘录，省略部分参数）：
    ```ts
    const device = await createGraphicsDevice(canvas, { deviceTypes: [DEVICETYPE_WEBGPU, DEVICETYPE_WEBGL2], ... });
    if (!device.isWebGPU && !device.isWebGL2) throw new Error(zh ? "浏览器不支持所需图形功能。" : ...);
    ```
  - 训练：Brush 走 **Vulkan（Windows/Linux）/ Metal（macOS）**，`brush.rs` 强制注入 `CUBECL_WGPU_DEFAULT_DEVICE=DiscreteGpu(0)`，日志固定写 `backend=Vulkan`。
- **CPU 回退**：
  - ✅ COLMAP 有 CPU 回退（README：「不满足要求时程序会自动使用 CPU」；macOS 用 Ceres CPU 构建）——但那是**相机重建**，不是渲染。
  - ❌ Brush **无 CPU 回退**（README 系统要求：「Brush 训练需要可用的 GPU 图形后端，建议使用独立显卡」；Ubuntu 节：「CPU-only 软件图形后端尚未验证」）。
  - ❌ 预览层**无 CPU 回退**（无 WebGL2/WebGPU 即失败）。
- **对我们产品的含义**：M 轴要覆盖「无独显、甚至纯 CPU 机器」的话，本仓库的整条预览/训练链在这些机器上**直接不可用**，不是性能降级问题。

---

## 5. 资产格式与内存量级

**读**（`src-tauri/src/imports.rs:181-190`）：

```rust
if name == "meta.json" { return Some(ImportedGaussianFormat::Sog); }
match path.extension()... {
    "ply" => Ply, "sog" => Sog, "spz" => Spz, "splat" => Splat, "ksplat" => Ksplat, _ => return None,
}
```

- `.ply` 由**自有 Rust 解析器**读（`reconstruction/ply.rs` 头解析 + `splat_transform.rs::parse_layout`）。
- `.spz` / `.sog` / `.splat` / `.ksplat`：Rust 只做**目录/引用安全性检查**（SOG 的 `meta.json` 引用不许越界、不许符号链接），实际解码交给前端 `@playcanvas/splat-transform`。

**写**（`reconstruction/splat_transform.rs` + `commands/model_export.rs`）：`ply` / `compressedPly` / `spz` / `sog` 四种。**编码不在 Rust**：Rust 只按 `CHUNK = 1,024,000` 字节分块落盘、限制总大小 `file_size*4 + 64 MiB`、导出后回读校验点数与 SH 阶；SPZ/SOG 的字节由 TS worker（`gaussianExport.worker.ts`，引 `@playcanvas/splat-transform/lib/webp.wasm`）生成。

**每点字节数**（由代码推导，可复算）：`parse_layout` 对每个 `property float` 记 `stride += 4`（`splat_transform.rs:245`），必需属性 14 个（`x,y,z,f_dc_0..2,opacity,scale_0..2,rot_0..3`），`bands()` 把 `f_rest_` 数量映射为 0/9/24/45 → `sh_bands` 0/1/2/3：

| SH 档（`f_rest_` 个数） | 属性数 | 字节/点（float32，无法线） |
|---|---:|---:|
| bands 0（无 `f_rest_`） | 14 | 56 B |
| bands 1（9 个） | 23 | 92 B |
| bands 2（24 个） | 38 | 152 B |
| **bands 3（45 个，3DGS 常用档）** | **59** | **236 B** |
| 若上游写入法线 `nx,ny,nz` | 62 | 248 B |

**单场景内存量级**（`commands/model_export.rs:140-152`，原文公式）：

```rust
fn estimate(info: &PlyInfo) -> u64 {
    info.file_size.saturating_mul(6)
        .saturating_add(info.splat_count.saturating_mul(128))
        .saturating_add(64 * MIB)
}
fn memory_budget(available: Option<u64>) -> u64 {
    available.map(|n| (n / 4).min(512 * MIB)).unwrap_or(256 * MIB)
}
```

即：**峰值 ≈ PLY 文件大小 ×6 + 点数 ×128 B + 64 MiB**，预算上限 **512 MiB**（默认 256 MiB）。代入 100 万点 SH3（236 B/点，文件 236 MB）：估算 ≈ 1.42 GB + 128 MB + 64 MB ≈ **1.6 GB**，远超 512 MiB 预算，`compression_allowed` 直接为 false。仓库 ROADMAP 的 P0 条目（issue #21「WebView2 加载超过 1 GB PLY 的内存峰值和 OOM 闪退」）与此一致。

预览侧还有一条硬上限（`html-viewer/viewer.ts:78`）：`Math.ceil(Math.sqrt(splatCount)) > device.maxTextureSize` 即报「模型超出显卡纹理容量」——说明 splat 数据以**纹理**形式上传 GPU。

---

## 6. 渲染路径

- **排序**：**不是 compute shader 排序**。用的是 PlayCanvas 2.21.4 的 **CPU sorter**（`src/components/GaussianViewer/PreviewSortCoordinator.ts:16-19` 原文注释）：
  ```ts
  /** Application-local compatibility bridge for PlayCanvas 2.21.4's CPU sorter.
   * A busy sorter can silently drop a camera request, while its manager still advances
   * the last-camera cache. Keep that request, and acknowledge only applied results.
   * No Gaussian data or order buffers are copied by this bridge. */
  ```
  本仓库在该文件里做的是**猴补丁式桥接**（包装 `gsplatDirector.getCameraData` / `layer.gsplatManager.createSorter` / `sorter.setSortParams`），保证排序请求不丢，不实现排序算法本身。
- **storage buffer 原子操作**：仓库内**无**相关代码（无 wgpu/无 shader 文件，`git ls-files` 中 `.wgsl/.glsl/.vert/.frag/.comp/.hlsl/.metal` 命中数为 **0**）。PlayCanvas 内部实现不在仓库、不可审计。
- **可替换接缝**：只有 PlayCanvas 暴露的注入点——`component.setWorkBufferModifier({ glsl, wgsl })`（`GaussianViewer.tsx:589`、`html-viewer/viewer.ts:135`），注入的是**内嵌在 TS 字符串里的 GLSL/WGSL**（`src/components/GaussianViewer/PreviewAnimation.ts:94` 定义 `PREVIEW_ANIMATION_GLSL`）。
  → 这**不是**「可替换成我们自己的 rasterizer 的接缝」：它只能改 PlayCanvas 的 work-buffer pass 片段着色逻辑；要换光栅化器等于**整换 PlayCanvas 引擎**（跨语言、跨运行时）。
- **离线 HTML 导出**：Rust 侧只做字符串拼装——`html_export.rs:432-433` `include_str!("../../../html-viewer/style.css")` / `body.html`，外加 `.cache/html-viewer/runtime.js`（Vite 以 IIFE 打出的 viewer bundle，内含 PlayCanvas）。**Rust 不含任何一帧像素生成逻辑**。

---

## 7. 许可义务

**Apache-2.0 需要保留/声明什么**：

1. `LICENSE` 全文随分发保留（§4(a)）；
2. **`NOTICE` 必须随分发传递**（§4(d)）——本仓库 NOTICE 只有 7 行（OOOSplat / Copyright 2026 ooolabdev / 指向 `licenses/THIRD_PARTY_NOTICES.txt` / 商标保留声明）；
3. **修改过的文件必须加显著修改声明**（§4(b)）——注意：**63 个 `.rs` 文件中带版权/SPDX/Apache 头的 = 0 个**（实测 `rs_with_license_header=0 / 63`），搬进来必须由我们补头；
4. 专利授权与终止条款（§3）随代码走；
5. 商标**不在** Apache 授权范围内：`TRADEMARK_POLICY.md` 明确「修改版不得把 OOOSplat 用作产品或项目名，不得使用官方 logo/icon」，且「基于 OOOSplat」类表述必须标明非官方。这与本仓库「零第三方产品名」的中性表述纪律方向一致，但它是**对方的硬义务**，不只是我们的风格。

**第三方资产逐项**（`licenses/` 共 11 个文件，无 `.gitmodules`、无 vendored 子模块）：

| 组件 | 许可 | 分发状态 | 对我们的影响 |
|---|---|---|---|
| Brush（训练器） | Apache-2.0 | **二进制，不入 Git**（`engines/README.md`：「Native binaries are not stored in Git」） | 拉二进制才产生分发义务 |
| COLMAP 4.2.1 | BSD-3-Clause | 二进制，不入 Git | 同上；含 Caspar(Apache-2.0)/Ceres |
| FFmpeg / FFprobe | Windows LGPL-3.0-or-later；macOS LGPL-2.1-or-later（禁用 GPL/nonfree） | 二进制，不入 Git | **LGPL 分发义务（含替换/重链接要求）**，仅在拉二进制时触发 |
| PlayCanvas Engine | MIT | npm 依赖 | 前端预览/离线 HTML |
| `@playcanvas/splat-transform` | MIT | npm 依赖 | SPZ/SOG/压缩 PLY 编码 + SH 旋转算法来源 |
| Mediabunny | MPL-2.0 | npm 依赖 | 视频编码（WebCodecs 封装） |
| Niantic SPZ | MIT | 格式实现参考 | SPZ 读写 |
| NVIDIA CUDA Runtime | 专有可再分发 | 二进制 | COLMAP CUDA 构建 |
| Caspar（Symforce） | Apache-2.0 | 随 COLMAP 二进制 | 稀疏求解 |

**许可死角（必须写进结论）**：`src-tauri/src/reconstruction/splat_transform.rs:3` 原文——

```rust
//! SH rotation follows the MIT-licensed PlayCanvas splat-transform implementation.
```

即 **MIT 派生算法落在 Apache-2.0 文件内**（MIT 与 Apache-2.0 兼容，但 MIT 要求保留其版权声明）。搬这一个文件，必须同时搬 `licenses/PlayCanvas-MIT.txt` 并在该文件头保留 MIT 归属，否则就是许可缺口。另 `splat_transform.rs:681` 也有「real-SH recurrence used by PlayCanvas splat-transform」的注释，属同一处归属。

---

## 8. 移植切分建议

**先纠正一个前提**：不存在「≤5 个 crate / ≤2 万行」的切法——仓库只有 1 个 crate。真正的问题是**切哪一层**，而唯一无窗口、无 GPU、可独立跑的只有 `reconstruction/` 的 PLY 后处理。

**第一批可独立搬的最小内核（建议）**：

| 文件 | 行数 | 依赖 |
|---|---:|---|
| `src-tauri/src/reconstruction/ply.rs` | 148 | `serde` |
| `src-tauri/src/reconstruction/splat_transform.rs` | 1,009 | `uuid` + `ply` + `error` + 两个类型 + 系统 `MoveFileExW` |
| `src-tauri/src/reconstruction/splat_transform/export_view.rs` | 279 | 同上（流式 seek 视图、内存预算） |
| `src-tauri/src/reconstruction/edit_mask.rs` | 179 | `serde` |
| `src-tauri/src/error.rs` | 35 | `thiserror` |
| 自写/内联类型（抄 `project/metadata.rs:27-95` 的 `GaussianTransform` + `GaussianCrop`，约 70 行） | ~70 | 无（切断 `chrono`/`uuid` 项目元数据/`rusqlite`） |
| **合计** | **≈ 1,720 行** | **0 GPU、0 窗口、0 外部进程** |

**能不能 headless 跑通：能。** 理由（代码证据）：纯 `std::fs` + `BufReader` 流式，`MAX_HEADER_BYTES = 1024*1024`、`ROWS_PER_CHUNK = 4096`；入口是 `export_transformed_ply_to(source, output, transform, edits, progress, check)`，只吃路径、吐路径；唯一平台相关是 Windows 原子发布 `MoveFileExW`（非 Windows 走 `fs::rename`）。它不产生任何图像/帧缓冲，只产出**变换后的 PLY 字节流**。

**它给不了 M 轴要的东西**：这是资产后处理（裁剪/删除/整体变换/格式导出），不是「3D 在场表现」。预览层（`GaussianViewer.tsx` 1,661 行 + 同目录共 47 个文件 + PlayCanvas + WASM 编解码）是 **TS/JS 跨语言栈**，无法搬进 Rust 产品线；`html-viewer/viewer.ts` 只有 175 行，因为它几乎全部委托给 PlayCanvas。

**若目标真是「Rust 渲染器」，本轮 crates.io 检索（`https://crates.io/api/v1/crates?q=gaussian-splatting&sort=downloads`，2026-10-18）给出的候选**（**许可与可用性本轮未查**，下一批必须单独核）：

| crate | 最新版 | 下载量 | 最近更新 | 说明 |
|---|---:|---:|---|---|
| `bevy_gaussian_splatting` | 8.0.2 | 47,347 | 2026-08-31 | bevy 插件，渲染管线挂在 bevy |
| `wgpu-3dgs-viewer` | 0.8.0 | 7,268 | 2026-08-23 | 「A 3D Gaussian splatting viewing library written in Rust using wgpu」 |
| `wgpu-3dgs-core` | 0.8.0 | 2,095 | 2026-08-23 | 同作者核心库 |
| `spz` / `spz_rs` | 0.0.6 / 0.1.0 | 2,289 / 916 | 2025-12-23 / 2025-02-22 | SPZ 格式读写 |

---

## 9. 关键风险

1. **仓库里没有渲染器（决定性）**：Rust 侧 0 行 GPU 代码、0 个 shader 文件；光栅化与排序在 PlayCanvas（npm 依赖，MIT，不在仓库内）。搬这个仓库**拿不到任何像素生成能力**，M 轴交付为零。
2. **GPU 版本锁 + 无 CPU 回退**：预览必须 WebGPU/WebGL2；训练必须 Vulkan/Metal（Brush）。README 明确「CPU-only 软件图形后端尚未验证」。产品要在无独显/纯 CPU 机器降级，这条链直接不可用，不是慢。
3. **行数超预算**：全仓 Rust 35,057 行 + TS 17,175 行，任何「整仓搬」都远超单批 ≤2 万行预算；而砍到 ≤2 千行只剩 PLY 后处理，与 M 轴目标不匹配。
4. **许可死角**：MIT 派生 SH 旋转算法混在 Apache-2.0 文件里（需保留 PlayCanvas MIT 声明）；63 个源文件零版权头（必须自己补 + 声明修改）；商标政策禁止修改版沿用 OOOSplat 名/图标；FFmpeg LGPL 与 CUDA 专有运行时只在拉二进制时触发，但 `engines/manifest*.json` 会把人往那条路上带。
5. **上游活跃但单点**：9 周 2k★、近 30 天 50 次提交、单一 `ooolabdev` 账号、0.6.1 未转正。随时可能重构目录与 IPC 契约，跟随成本高。
6. **shader 不可审计**：全部 shader 以 TS 字符串（`PreviewAnimation.ts:94`）或 PlayCanvas 引擎内部形式存在，做安全/合规审计时无法给出「我们分发的着色器源码」清单。

---

## 10. 结论

**建议只参考，不建议搬。**

- **不建议搬**（作为 3D 在场表现层）：M 轴的核心是画面，而画面不在这个仓库里；搬过来只能得到一套「视频→COLMAP→Brush→PLY→导出」的桌面 App 编排，与「伙伴的 3D 在场」目标错位，还要额外背 GPU 硬依赖与商标/许可义务。
- **建议只参考**：
  - 工程做法：`engines/` 的「二进制不入 Git + manifest 锁版本/哈希 + 启动前 verify」策略；`pipeline/` 的阶段状态机与进度事件；`tasks/logs.rs` 的日志环形缓冲；`project/` 的原子发布（`MoveFileExW` + 回读校验）。
  - 资产层：`reconstruction/` 的**流式、非破坏、带内存预算**的 PLY 变换导出（1,720 行，可独立搬，见 §8）——若我们将来需要「伙伴 3D 资产的裁剪/压缩/多格式导出」，这是目前看到的最干净的一块。
  - 产品口径：ROADMAP 里「超大 PLY 预览 OOM」这类真实边界，可直接拿来做我们 M 轴的压力测试用例设计参考。

**若搬（只搬 §8 的资产内核），第一阶段验收标准（可测）**：

1. **字节级正确性**：给定 100,000 点 SH3 测试 PLY（236 B/点），跑「整体变换 + Box 裁剪 + 删除掩码」组合，输出 PLY 的点数、属性集、SH 阶与期望一致，且**变换只应用一次**（与上游 `edited_export_keeps_sh_and_applies_filter_and_transform_only_once` 的断言语义对齐），逐行字节 diff = 0。
2. **内存门禁**：处理过程中 RSS 峰值 ≤ `file_size×6 + splat_count×128 + 64 MiB`（上游 `estimate()` 公式）实测值 ±10%；超预算时必须走「拒绝压缩 / 明确报错」路径而不是 OOM。
3. **零新依赖**：除 `serde`/`uuid`/`thiserror` 外 0 个新增第三方 crate；`cargo tree` diff 可复核。
4. **无窗口可跑**：在**无 GPU、无显示**环境（容器/CI，`DISPLAY` 未设）`cargo test` 全绿，且不链接任何图形库。
5. **许可门禁**：搬入文件全部带 SPDX 头 + 修改声明；`THIRD_PARTY_NOTICES` 增补 PlayCanvas MIT 归属；仓库内不出现 OOOSplat 名称/图标（中性表述纪律）。

> 若 M 轴最终要的是「能出画面」，正确动作不是搬这个仓库，而是**另立一批**评估 `wgpu-3dgs-core`/`wgpu-3dgs-viewer`（先核许可、再核无独显回退），届时验收才谈得上「给定测试 `.ply` 输出 1280×720 PNG，与参考图 PSNR ≥ X」。**本报告不预设该 X 值**——参考图与基线本轮都没有材料，标「未查到」，需在渲染器专项批里先建立。

---

## 附录：本轮实测

### A. 节点池测活（2026-10-18 14:41 +08:00，§三配方，`ThrottleLimit 12`，单节点 `--max-time 8`）

探测 URL：`https://raw.githubusercontent.com/github/gitignore/main/Rust.gitignore`；总墙钟 **10,454 ms**；**活 20 / 32**。

| 节点 | 延迟 ms | 节点 | 延迟 ms |
|---|---:|---|---:|
| git.yylx.win | 856 | gh-proxy.com | 1,987 |
| ghfast.top | 1,007 | github.ednovas.xyz | 1,987 |
| ghm.078465.xyz | 1,271 | git.tangbai.cc | 2,121 |
| gh.catmak.name | 1,374 | ghpxy.hwinzniej.top | 2,720 |
| ghfile.geekertao.top | 1,439 | gh.nxnow.top | 2,789 |
| ghproxy.net | 1,489 | jiashu.1win.eu.org | 3,089 |
| gh.chjina.com | 1,686 | tvv.tw | 3,340 |
| gh.dpik.top | 1,698 | fastgit.cc | 3,382 |
| gh.noki.icu | 1,722 | git.669966.xyz | 3,787 |
| gh.monlor.com | 1,796 | ghp.keleyaa.com | 4,312 |

**死 12 个**：`gh.927223.xyz`、`github.chenc.dev`、`ghproxy.imciel.com`、`gitproxy.mrhjx.cn`、`g.blfrp.cn`、`github.geekery.cn`、`gh.b52m.cn`、`github.xxlab.tech`、`ghproxy.cxkpro.top`、`ghproxy.monkeyray.net`、`github.dpik.top`、`gh.ddlc.top`。

### B. 实际成功取到内容的通道

| 目标 | 通道 | 结果 | 耗时 |
|---|---|---|---|
| 整仓源码（392 文件） | `git clone --depth 1 https://ghfast.top/https://github.com/ooolabdev/ooosplat.git` | ✅ 成功（`git.yylx.win` 先试失败：`info/refs not valid`） | **7,219 ms** |
| 仓库元数据（star/license/分支） | ① 节点 `ghm.078465.xyz` 代理 `api.github.com` → 200；② **`curl.exe` 直连 `api.github.com` → 200** | ✅ 两路都通，数值一致 | < 1 s（直连） |
| 提交列表（50 条） | `curl.exe https://api.github.com/repos/.../commits?per_page=100` → 15 s 超时（282 KB 未读完）→ 降为 `per_page=50` | ✅ 第二次成功 196,294 B | ~5 s |
| raw 文件直连 | `curl.exe -o NUL -w '%{http_code}' https://raw.githubusercontent.com/ooolabdev/ooosplat/main/LICENSE` | ✅ **200** | < 2 s |
| crates.io 是否已发布 `ooosplat` | `https://crates.io/api/v1/crates/ooosplat` | ❌ **404**（未发布 → 无 tarball 通道） | ~1 s |
| crates.io 候选检索 | `https://crates.io/api/v1/crates?q=gaussian-splatting` | ✅ 200 | ~1 s |
| GitHub API 限速 | `https://api.github.com/rate_limit` | `core 56/60`（匿名额度，省用） | < 1 s |

**新发现（建议回写 `github-fetch-guide.md` §〇，本报告只记录不改指南）**：教程写「`web_fetch` 对 GitHub 域定死 `fetch failed`，GitHub 内容别指望 `web_fetch`」——本轮复核成立（未再试 `web_fetch`，遵批令）。但 **`pwsh` + `curl.exe` 直连 `api.github.com` 与 `raw.githubusercontent.com` 均返回 200**，即「harness `web_fetch` 的链路」不通不等于「本机链路」不通。批量取材仍建议节点池（稳、可 `git clone`），但单文件/元数据取证可先试直连 `curl`，省节点往返。

### C. 取到的原始材料清单（本地临时目录，不入库）

```text
%TEMP%\m-axis-recon\ooosplat\        # ghfast.top 浅克隆，depth=1，HEAD=410859b，392 tracked files
%TEMP%\m-axis-recon\repo.json        # /repos/ooolabdev/ooosplat（5,793 B）
%TEMP%\m-axis-recon\commits.json     # /commits?per_page=50（196,294 B）
%TEMP%\m-axis-recon\part.json        # /stats/participation
```

本批对 Apeireth-rust 工作区的唯一写入是**新增本报告**（`git status --porcelain` 中本批对应项仅 `?? m-axis-3d-recon-2026-10-18.md`）；同一快照里的其他 `M`/`??` 项（`crates/**`、`artifacts/**`、`docs/04-internal/**`）属并行施工线，非本批产出，本批未触碰。

### D. 材料受限（如实标注）

- **未实测运行**该仓库程序（未 `cargo build`、未 `npm i`、未下载任何引擎二进制），所有性能/内存数字均为**源码公式推导**，非实测；
- 未取得 release 资产、未审计 PlayCanvas 引擎源码（npm 未安装），故 PlayCanvas 内部排序/光栅化实现**未查到**（只知其 API 与注释层面事实）；
- `stats/participation` 的 `all` 与 `owner` 数组合计不一致（近 10 周 `all` 求和 93，`owner` 52 周 83），已按 API 原值记录，未做归一；
- 报告落盘时点：2026-10-18 14:48 +08:00，本轮总耗时约 7.5 分钟（测活 10.5 s + 克隆 7.2 s + 取证与阅读约 6 min）。

> 0 装声明：本报告为外部仓库侦察，未改动本仓库任何代码；未实测运行该仓库程序。
