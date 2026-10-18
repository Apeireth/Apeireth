# M 轴 · 3D 高斯溅射在场表现层 · `wgpu-3dgs` 两 crate 侦察（2026-10-18）

> 侦察对象：crates.io 上的 `wgpu-3dgs-core` 0.8.0 与 `wgpu-3dgs-viewer` 0.8.0（上一批遗留的「下一个候选」）。
> 取材方式：按 [github-fetch-guide.md](../04-internal/github-fetch-guide.md) §〇 选路——**Rust crate 源码走 crates.io tarball**，文档/元数据走 crates.io API 与 docs.rs；**未触碰 GitHub 域**。
> 边界：**只做静态读码 + 元数据**，**未编译、未运行**这两个 crate；除本文件外**未改动本仓库任何文件**（含 `crates/**`、`Cargo.toml`、`Cargo.lock`）。
> 上游语境：`docs/design/m-axis-3d-recon-2026-10-18.md`（同批上一个候选已判「不建议搬」，本报告不重复其结论）。

## 结论（先行）

- **建议：搬（有条件）** —— 两 crate 是纯库、无窗口依赖、`render` 直接收 `TextureView`，**headless 离屏可用**；许可与我们的双协议完全相容。
- **许可 `MIT OR Apache-2.0`**（可择 Apache-2.0 取专利授权；两包均**无 NOTICE**，无 NOTICE 传递义务）；**规模**：`src/**` 共 32 个 `.rs` / **8,451 行**（含空行），shader 11 个文件 / 1,524 行，另有 examples 10 文件 / 1,731 行。
- **最大风险是依赖闭包不是画质**：一阶即拉入 `wgpu 30`（默认开全后端）+ **运行时 WESL 编译器**，发布 lockfile 的全平台并集 **222（core）/ 226（viewer）** 个包，与本仓 `deny.toml` 的 `multiple-versions = "deny"` 存在**可预见的冲突**（未实测）。

---

## 1. 元数据

| 项 | `wgpu-3dgs-core` | `wgpu-3dgs-viewer` |
|---|---|---|
| 最新版本 | **0.8.0** | **0.8.0** |
| 发布时刻（UTC） | 2026-08-23 09:56:44 | 2026-08-23 10:10:33 |
| 总下载 / 近 90 天下载 | **2,095** / 571 | **7,268** / 4,301 |
| 许可（SPDX，crates.io 字段） | **`MIT OR Apache-2.0`** | **`MIT OR Apache-2.0`** |
| 是否 yanked | 否（0.4.0–0.8.0 共 6 个版本全 `yanked=False`） | 否（0.1.0–0.8.0 共 10 个版本全 `yanked=False`） |
| 仓库 / 主页 | `https://github.com/LioQing/wgpu-3dgs-core` | `https://github.com/LioQing/wgpu-3dgs-viewer` |
| 发布快照 commit（`.cargo_vcs_info.json`） | `d98ccc8f033ac060cf41401ee01087d5afd81124` | `e4b3127f4043cb53a8ce19c56af49bc9cf942b4d` |
| 维护者 | `LioQing`（crates.io owner，docs.rs Owners 同） | `LioQing` |
| docs.rs 文档覆盖率 | **39.39%** | **57.89%** |
| 首次发布 | 2025-09-20（0.4.0 起） | 2025-02-27（0.1.0 起） |
| edition / rust-version | 2024 / 未声明 | 2024 / 未声明 |

**许可文件原文（两包内容一致，SHA-256 相同）**

`LICENSE-MIT` 开头 3 行：

```text
MIT License

Copyright (c) 2025 Lio Qing
```

`LICENSE-APACHE` 开头 3 行：

```text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/
```

| 文件 | 字节 | SHA-256 |
|---|---|---|
| `LICENSE-MIT` | 1,065 | `D5D786111EDFCC4A496754D70E9579A8CD2C74C82C4E43CCEA8ED09FFD81BFE7` |
| `LICENSE-APACHE` | 11,338 | `E7F74376EF43142436EB693EAB4B66DD09AB0D0FFAD2907994E41AB5456E2826` |
| `NOTICE` | — | **两包均不存在** |

复现命令：

```powershell
$ua = @{'User-Agent'='apeireth-build/2.0 (vendor-research)'}
$j = (Invoke-WebRequest -UseBasicParsing -Headers $ua `
  -Uri 'https://crates.io/api/v1/crates/wgpu-3dgs-core').Content | ConvertFrom-Json
$j.crate.max_version, $j.crate.downloads, $j.crate.repository
$j.versions | Select-Object num, created_at, yanked, license, edition
```

**未查到**：GitHub 侧 star / issue / 提交频率（按指引不访问 GitHub 域，未取）。可查到的事实是：两仓共用一个 owner，CHANGELOG 的 PR 编号最大到 `#31`，属**单人维护的小体量项目**；发布节奏为 0.7.0（2026-05-15）→ 0.8.0（2026-08-23），**每版都跟一次 `wgpu` 大版本**（29→30）。

## 2. 依赖清单 / 平台绑定 / headless

**`wgpu-3dgs-core` 的 13 个普通依赖**（`Cargo.toml` 与 crates.io `/dependencies` 双向核对）：

```text
async-trait 0.1   bytemuck 1.25(derive,extern_crate_alloc,min_const_generics)
flate2 1.1        glam 0.33(bytemuck)      half 2.7(bytemuck)   itertools 0.15
log 0.4           oneshot 0.2(async,std)   paste 1.0            ply-rs 0.1
thiserror 2.0     wesl 0.4                 wgpu 30.0
dev: assert_matches 1.5, pollster 1.0
```

**`wgpu-3dgs-viewer`**：`bitflags 2.13`、`bytemuck`、`glam`、`half`、`log`、`oneshot`、`paste`、`thiserror`、`wesl`、`wgpu`、`wgpu-3dgs-core 0.8`；可选 `wgpu-3dgs-editor 0.8`（仅 `editor`/`selection` feature）。**`winit` 只是 dev-dependency**（给 examples 用），**库本体不依赖任何窗口 crate**。

```toml
# wgpu-3dgs-viewer-0.8.0/Cargo.toml（节选）
[features]
editor = ["dep:wgpu-3dgs-editor"]
multi-model = []
selection = ["editor"]
viewer-selection = ["selection"]

[dependencies.wgpu]
version = "30.0"     # ← 未关默认 feature
[dev-dependencies.winit]
version = "0.30"
```

**平台绑定**：不硬绑单一图形 API，但也**没有给我们裁剪的余地**。`wgpu 30.0.0` 的默认 feature 集（crates.io API 实测）是：

```text
std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu
```

两 crate 都以默认 feature 依赖 `wgpu`；Cargo feature 只增不减，**我们在自己 crate 里 `default-features = false` 也压不掉 ash / objc2-metal / gles 这些后端**，除非 vendored 改 `Cargo.toml`。运行时选后端，Windows 上可用 DX12 或 Vulkan。

**headless**：**有，而且是库的天然形态**。`src/**` 全量搜索 `Surface|surface|Window|winit` 只命中 1 处（`wgpu::TextureUsages::RENDER_ATTACHMENT` 的注释），**库代码 0 处创建 surface / instance / window**；渲染入口只收 `&wgpu::Device`、`&wgpu::Queue`、`&mut CommandEncoder`、`&TextureView`。

## 3. 代码规模

| 范围 | 文件数 | 行数（含空行） | 字节 |
|---|---|---|---|
| core `src/**/*.rs` | 13 | **4,082** | — |
| viewer `src/**/*.rs` | 19 | **4,369** | — |
| **src 合计** | **32** | **8,451** | — |
| core `examples/*.rs` | 4 | 213 | — |
| viewer `examples/**/*.rs` | 6 | 1,518 | — |
| **含 examples 的 `.rs` 合计** | **42** | **10,182** | 347,544 |
| core shader（`.wesl`） | 3 | 323 | 8,314 |
| viewer shader（7 `.wesl` + 1 `.wgsl`） | 8 | 1,201 | 42,470 |
| **shader 合计** | **11** | **1,524** | 50,784 |

口径说明：行数用 `(Get-Content $f).Count`（**含空行**）。注意 `Measure-Object -Line` 会**跳过空行**（同文件实测 17 vs 20），本报告不采用该口径，避免少报。

复现命令（本轮实际执行）：

```powershell
$tmp = Join-Path $env:TEMP 'crates-recon'
foreach ($d in 'wgpu-3dgs-core-0.8.0','wgpu-3dgs-viewer-0.8.0') {
  $root = Join-Path $tmp $d
  $src = Get-ChildItem (Join-Path $root 'src') -Recurse -Filter *.rs -File
  $bytes = ($src | Measure-Object -Property Length -Sum).Sum
  $lines = 0; foreach ($f in $src) { $lines += (Get-Content $f.FullName).Count }   # 含空行
  "$d  rs=$($src.Count)  bytes=$bytes  lines=$lines"
}
```

结果：`core rs=13 bytes=143078 lines=4082`、`viewer rs=19 bytes=204466 lines=4369`（viewer 的 204,466 B 含 examples；src-only 行数为上表值）。

## 4. 架构与公开 API 面

**分层**（`src/lib.rs` 的模块表即分层表）：

- **core = 数据面**：`gaussian`（`Gaussian` 值类型 + 12 种 `GaussianPodWith*Configs` 的 `repr(C)` GPU 布局）+ `buffer`（`GaussiansBuffer` / `ModelTransformBuffer` / `GaussianTransformBuffer`，含 `BufferWrapper::download` 回读）+ `source_format`（`ply` / `spz`）+ `compute_bundle`（WESL→WGSL→`ComputePipeline` 的构建器）+ `shader`（WESL 包常量）。
- **viewer = 渲染面**：`Camera`/`CameraBuffer` → `Preprocessor`（视锥剔除 + indirect args + 深度）→ `RadixSorter`（GPU 基数排序）→ `Renderer`（splat/ellipse/point 三种显示模式）→ `Viewer`（把以上串成一次 `render`）；`multi-model`、`selection` 为可选 feature，默认关。

**公开 API 面（docs.rs 0.8.0 首页 Items 链接逐项计数）**

| crate | struct | enum | trait | type alias | module | re-export |
|---|---|---|---|---|---|---|
| core | 39 | 27 | 9 | 0 | 1（`shader`） | 3（`glam` / `wesl` / `wgpu`） |
| viewer | 18 | 4 | 1 | 2 | 2（`shader`、`selection`） | 2（`core`、`editor`） |

core 的关键项：`Gaussians`、`GaussiansSource`、`Gaussian`、`GaussiansBuffer`、`ComputeBundle`/`ComputeBundleBuilder`、`GaussianPod`/`IterGaussian`/`BufferWrapper`。
viewer 的关键项：`Viewer`、`ViewerCreateOptions`、`Camera`、`CameraTrait`、`Preprocessor`、`RadixSorter`、`Renderer`、`DefaultGaussianPod`（=`GaussianPodWithShSingleCov3dSingleConfigs`）。

**shader 是否内嵌：是，且是运行期编译**（不是 build script）：

- 所有 shader 以 `include_str!` 编进二进制：core 3 个 `.wesl`（`gaussian` / `gaussian_transform` / `model_transform`），viewer 7 个 `.wesl` + 1 个 `radix_sort.wgsl`；无 `build.rs`（`Cargo.toml` 明写 `build = false`）。
- 运行时由 `wesl 0.4` 把 WESL 编译成 WGSL：`renderer.rs`/`preprocessor.rs` 走 `wesl::compile_sourcemap(...)` + `wesl_utils::resolver()`（把两个 crate 的 `CodegenPkg` 常量的 shader 源喂给 `wesl::PkgResolver`，**不读文件系统**）。

```rust
// wgpu-3dgs-viewer-0.8.0/src/wesl_utils.rs（节选）
pub fn resolver() -> wesl::PkgResolver {
    let mut resolver = wesl::PkgResolver::new();
    // ... 注册 wgpu-3dgs-viewer / wgpu-3dgs-core 两个 CodegenPkg
}
```

**compute shader 排序：自带**。`RadixSorter` 的 `radix_sort.wgsl` 529 行（23,120 B，含空行口径），`Viewer::render` 每帧调用一次：

```rust
// wgpu-3dgs-viewer-0.8.0/src/lib.rs:271-280
pub fn render(&self, encoder: &mut wgpu::CommandEncoder, texture_view: &wgpu::TextureView) {
    self.preprocessor.preprocess(encoder, self.gaussians_buffer.len() as u32);
    self.radix_sorter.sort(encoder, &self.radix_sort_indirect_args_buffer);
    self.renderer.render(encoder, texture_view, &self.indirect_args_buffer);
}
```

## 5. 资产格式与内存量级

**支持读**（`Gaussians::read_from_file(path, GaussiansSource)`）：

| 格式 | 支持情况 | 证据 |
|---|---|---|
| `.ply`（3DGS 的 Inria 布局） | **读 + 写**；`f_dc_0..2` / `f_rest_0..44` / `opacity` / `scale_0..2` / `rot_0..3` 属性名直读；ASCII / binary little-endian / binary big-endian 三种编码 | `source_format/ply.rs`，`PlyHeader::{Inria, Custom}`（Custom 允许属性顺序不同） |
| `.spz` | **读 + 写**；magic `0x5053474e`（`"NGSP"`），支持 version `1..=3`，量化存储 | `source_format/spz.rs:458-461` |
| `.splat` / SOG / glTF / 压缩 PLY | **不支持**（`GaussiansSource` 只有 `Internal` / `Ply` / `Spz` 三个变体，`source_format/` 下也只有 `ply.rs`、`spz.rs`） | 同上 |

**单点属性布局与内存**（`repr(C)`，按字段宽度推算，**未运行 `size_of`**）：

| Pod 类型 | 组成（字节） | 单点 |
|---|---|---|
| `PlyGaussianPod`（读 PLY 的中间态） | pos 12 + normal 12 + color 12 + sh 180 + alpha 4 + scale 12 + rot 16 | **248 B** |
| `DefaultGaussianPod` = ShSingle + Cov3dSingle，`padding_size=1` | pos 12 + color RGBA8 4 + sh `[Vec3;15]` 180 + cov3d `[f32;6]` 24 + pad 4 | **224 B** |
| ShHalf + Cov3dHalf，`padding_size=2` | 16 + 92 + 12 + 8 | **128 B** |
| ShNorm8 + Cov3dHalf，`padding_size=1` | 16 + 48 + 12 + 4 | **80 B** |

量级：100 万点 → 读 PLY 峰值约 **248 MB**，上 GPU 默认 **224 MB**（half 128 MB / norm8 80 MB）。`GaussiansBuffer` 是**单个 storage buffer**，默认 limits 下单个 storage binding 有长度上限（历史默认 128 MiB 量级；wgpu 30 `Limits::default()` 的确切数值本轮未逐字段核验），**百万点级需要显式申请更高 limits**（examples 里就是 `required_limits: adapter.limits()`）。

## 6. 渲染可用性：能否只当库用、不启窗口

**能**，但「读回像素」这半截要我们自己写。

**库内已有的离屏路径**（具体类型/函数）：

```rust
// 1) 读点云（不碰窗口）
let gaussians = wgpu_3dgs_core::Gaussians::read_from_file(path, GaussiansSource::Ply)?;
// 2) 建 viewer：只给 device + 目标纹理格式 + 点云
let mut viewer = wgpu_3dgs_viewer::Viewer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb, &gaussians)?;
// 3) 相机：位置/朝向/z 区间/垂直 FOV
let mut camera = wgpu_3dgs_viewer::Camera::new(0.1..1e4, 60f32.to_radians());
camera.pos = glam::Vec3::new(0.0, 1.0, 4.0);
viewer.update_camera(&queue, &camera, glam::UVec2::new(1280, 720));
// 4) 建好 viewer 后至少调一次（不调则尺寸/SH 阶数/显示模式保持 0）
viewer.update_gaussian_transform(&queue, 1.0, GaussianDisplayMode::Splat,
    GaussianShDegree::new(3).unwrap(), false, GaussianMaxStdDev::new(2.0).unwrap());
// 5) 渲到我们自己的纹理
let mut encoder = device.create_command_encoder(&Default::default());
viewer.render(&mut encoder, &offscreen_view);
```

- **离屏渲染接口有**：`Viewer::render(&self, &mut CommandEncoder, &TextureView)`；`RendererCreateOptions` 允许指定 `texture_format`、可选 `depth_stencil`、`color_write_mask`、`pipeline_cache`；默认 `multisample = 1`、`blend = ALPHA_BLENDING`。
- **纹理 readback 接口没有**：库只提供 **buffer** 级回读（`BufferWrapper::download` / `map_download_with_poll_type`，用于 selection 等 buffer），**没有** `copy_texture_to_buffer` 封装。要做 PNG 得自己写约 60–80 行：建 `RENDER_ATTACHMENT | COPY_SRC` 纹理 → `copy_texture_to_buffer`（每行按 256 B 对齐）→ `map_async` → 去 padding → 编码 PNG。
- **资源底线**：单个 compute shader stage 绑定 **6 个 storage buffer**（`Preprocessor` 6、`RadixSorter` 6、`Renderer` 2），wgpu 原生默认上限 8 → 可用；**WebGL2 downlevel（上限 4）不可用**。
- **启动成本**：`Viewer::new` 内部要做 3 次 WESL 编译（pre/mid/post）+ 1 次渲染管线创建；WESL 是解释执行的编译器，**首次建 viewer 的 CPU 开销不可忽略**（未实测）。

## 7. 依赖闭包风险

**估算口径**（写清楚，避免被当成实测）：解包后的 `Cargo.lock` 是发布时的完整解析图；我按 `[[package]]` 建图，从**该 crate 的普通依赖**出发做 BFS，得到闭包。`Cargo.lock` **不区分 normal / build**，所以闭包**含 build-dependencies**；且它是**全平台并集**（Android / Apple / wasm / Wayland 的包都在里面），**Windows 实编集比这个数小，未逐 target 实测**。

| 指标 | core | viewer |
|---|---|---|
| 普通依赖闭包（全平台并集，含 build-deps） | **222** | **226** |
| 相对本仓现有 `Cargo.lock`（606 包）的**新增**包（按 name+version 严格比对） | **84** | **85** |
| 其中 `wgpu` 子树 | 123 | 123 |
| 其中 `wesl` 子树 | **104** | 104 |

**最扎眼的一条**：`wesl` 一个依赖就带进 **104 个包**（lalrpop、logos、pulldown-cmark、annotate-snippets、petgraph、regex、sha3、serde、walkdir…），占闭包近一半；而它是**普通依赖、无 feature 开关**——想省掉只能自己改代码（把 WESL 预编译成 WGSL 常量，按 4 种 SH × 3 种 cov3d = 12 个变体各存一份），这属于搬入后第二阶段的事，不是接线能规避的。

**与 `deny.toml` 的冲突（读配置推算，未跑 `cargo deny`）**：本仓 `[bans] multiple-versions = "deny"`，`[graph] targets` 含 `aarch64-linux-android`；发布 lockfile 里出现多版本的有 `bitflags 1.3.2/2.13.1`、`thiserror 1.0.69/2.0.20`、`hashbrown 0.15/0.16/0.17`、`foldhash 0.1/0.2`、`bit-set 0.9/0.10`、`itertools 0.14/0.15`、`syn 2/3`。其中 `hashbrown`/`bitflags`/`itertools`/`syn`/`indexmap`/`windows-*` 已在 `skip` 里，**`thiserror`（来自 Android 目标的 `ndk 0.9`）、`foldhash`、`bit-set` 不在**——接进来大概率要补 `skip` 条目，否则 `cargo deny check bans` 会红。

**其他闭包事实**：`ply-rs 0.1.3`（MIT，2019 年代的老包）的构建期链带进 `skeptic`/`error-chain`/`peg` 等（发布 lock 不区分 normal/build，此处未逐条确认依赖 kind）；`skeptic` 属**构建期**用途，不进运行时，但会拉长编译时间并增加供应链面。

## 8. 许可义务

- **SPDX：`MIT OR Apache-2.0`**（crates.io 字段、两包 `Cargo.toml`、docs.rs 页头三处一致），**双许可可择一**。我们主仓是 `Apache-2.0 OR MIT`，**完全相容**，且选 Apache-2.0 可拿到显式专利授权。
- **无 NOTICE 文件**（两包均无）→ Apache-2.0 §4(d) 的 NOTICE 传递义务**不触发**；但仍须：
  1. 随分发附上所选许可的**完整文本**（`LICENSE-APACHE` 或 `LICENSE-MIT`）；
  2. **保留版权与归属声明**（MIT：`Copyright (c) 2025 Lio Qing`；Apache-2.0 §4(c)：保留所有版权/专利/商标/归属声明）；
  3. 对**修改过的文件**做**修改声明**（Apache-2.0 §4(b)）；若我们做中性改名（crate 名、WESL `crate_name`、shader 模块路径），就属于修改，须在文件头或 NOTICE 中写明。
- **copyleft 排查**：新增包里抽样 8 个高风险项（`r-efi`、`redox_syscall`、`khronos-egl`、`spirv`、`ply-rs`、`skeptic`、`error-chain`、`zmij`）全为 MIT / Apache-2.0 / 双许可；`r-efi` 是 `MIT OR Apache-2.0 OR LGPL-2.1-or-later` 三选一，取 MIT 即可，**不构成 LGPL 义务**。**未做全量 SPDX 核验**（那要跑 `cargo deny`，本批不改 `Cargo.toml`，故未跑）——接线的第一批必须补跑。
- **资产许可**：`examples/model.ply`（3,758 B）与 `examples/model.spz`（139 B）随包分发，**来源未在包内注明**；若我们要拿它当测试基准，须在 `OSS_NOTICE.md` 记「上游示例资产，来源未注明」，或干脆自建测试点云。
- **本仓既有机制**：`THIRD-PARTY-NOTICES.md`（cargo-about 生成，561 crates）、`OSS_NOTICE.md`、`deny.toml` 白名单已含 `MIT`/`Apache-2.0`；搬入后需**重新生成**第三方清单并登记溯源，这是许可证搬运线的既有工序。

## 9. 与我们现状的兼容面

**先纠一个前提**：任务书写「我们 workspace 是 edition 2024」——**实测根 `Cargo.toml` 是 `edition = "2021"`**（第 66 行），`rust-version = "1.97"`，`rust-toolchain.toml` 钉 `1.97.1`；这两个 crate 是 **edition 2024**。edition 是**逐 crate** 的，**混用不冲突**；edition 2024 要求 rustc ≥ 1.85，`wgpu 30.0.0` 自报 `rust-version = 1.87.0`，我们 1.97.1 **都满足**。

**现有 wgpu 依赖：没有**。全仓搜索只命中 1 条**注释**：

```text
$ git grep -n -i "wgpu" -- Cargo.toml crates
crates/engine/image/cms/src/lut3d.rs:65:  /// Little-endian RGBA16F texels (8 bytes each), e.g. for `wgpu::TextureFormat::Rgba16Float`.
```

`Cargo.lock`（本批读取时 606 包）里 `wgpu`/`wgpu-core`/`wgpu-hal`/`naga`/`ash`/`glam`/`oneshot`/`ply-rs`/`wesl` **全部 absent**；`bytemuck`/`log`/`thiserror`/`flate2`/`half`/`itertools`/`paste`/`bitflags`/`async-trait` **present**（版本不完全相同，需统一）。

**最小接线面（3–5 条）**

1. **落位**：按许可证搬运线惯例以 vendored 成员进 workspace（中性命名 `apeireth-3dgs-core` / `apeireth-3dgs-viewer`），`version.workspace` / `license.workspace` 继承；改名会连带改 `wesl::CodegenPkg::crate_name`、`ModulePath` 字符串与 shader 里的包名——**不是只改 `Cargo.toml`**。
2. **GPU 会话**：`wgpu::Instance` → `request_adapter`（`compatible_surface: None`）→ `request_device`（`required_limits: adapter.limits()`，百万点必须放开 storage binding 上限）；`wgpu` 版本以 `wgpu_3dgs_core::wgpu` 的 re-export 为准，避免双版本。
3. **数据**：`Gaussians::read_from_file(path, GaussiansSource::Ply)`（或 SPZ）；需要写盘时用 `write_to_file`。
4. **渲染**：`Viewer::new` + `Camera`/`update_camera`（每帧）+ `update_gaussian_transform`（建好后至少一次）+ `render` 到自建 `TextureView`。
5. **回读**：自写 `copy_texture_to_buffer` + 256 B 行对齐 + `map_async` → PNG；库不提供纹理回读。

**治理面**：`wgpu` 树会让 `Cargo.lock` 增加约 **84–85** 个包，`deny.toml` 的 `bans.skip` 与 `THIRD-PARTY-NOTICES.md` 都要同步更新——这是一次**需要 owner 拍板的新外部依赖引入**，不是普通接线。

## 10. 结论与第一阶段验收标准

**结论：搬（有条件）**。理由是三条硬证据：① 纯库、无窗口、`render` 收 `TextureView`，headless 是它的原生用法；② `MIT OR Apache-2.0` 与我们的双协议相容、无 NOTICE 传递；③ 8,451 行 src + 1,524 行 shader 里包含 GPU 基数排序与 12 种量化布局，**自己重写的成本远高于搬入**。

**条件**（缺一不搬）：vendored 而非 crates.io 直依赖（隔离上游每 3 个月一次的 `wgpu` 大版本跳动）；`deny.toml` 补 `skip` 并跑通 `cargo deny check`；`THIRD-PARTY-NOTICES.md` 重新生成；第一阶段只接 headless，不接窗口。

**第一阶段验收标准（可测）**

| # | 标准 | 判定 |
|---|---|---|
| A1 | 固定 `.ply`（建议自建，不用上游示例资产）+ 固定相机（`pos`/`yaw`/`pitch`/`fov`/`z` 全部写死）+ 1280×720 + `Rgba8UnormSrgb` + **不开窗口**（无 `Surface` 创建），输出 PNG | `Viewer::new` → `render` → 回读 全链路无 `winit`/`Surface` 符号，PNG 可打开 |
| A2 | **同机同驱动连续两次运行，PNG 字节一致** | `Get-FileHash` SHA-256 相等；若不成立，降级为「≥99.9% 像素差 ≤1 LSB 且 PSNR ≥ 50 dB」并如实记录 |
| A3 | 回归基线 | 与 A1 首版 golden 比 **PSNR ≥ 45 dB**（或 SSIM ≥ 0.99），阈值在第一次验收时由 owner 定稿 |
| A4 | 画面非空 | alpha 覆盖率 ≥ 5%，点云包围盒中心落在画面中心 ±5% |
| A5 | 性能基线 | 720p 单帧（含排序）在目标机 < 5 s；100 万点加载不 OOM（记录峰值 RSS） |
| A6 | 治理 | `cargo deny check` 0 violation、`cargo test --workspace --locked -j 2` 0 failed（搬入批次执行，本批不跑） |

**如果 owner 不接受 `wgpu 30` 全后端 + WESL 104 包的代价**，退路是「只参考」：拿走 `radix_sort.wgsl` 的排序思路、`GaussianPod` 的 12 种量化布局与 PLY/SPZ 解析规则，自研渲染层——但那是数倍工作量，且要自己解决排序正确性与 SH 求值。

---

## 附录：本轮实测

### A. 通道与耗时（2026-10-18 14:49–14:54 +08:00）

| 通道 | 对象 | 结果 | 耗时 |
|---|---|---|---|
| crates.io API `GET /api/v1/crates/{name}` | 两 crate 元数据 | **200** | 未单独计时 |
| crates.io API `GET /api/v1/crates/{name}/0.8.0/dependencies` | 依赖 kind/feature | **200** | 未单独计时 |
| crates.io API `GET /api/v1/crates/wgpu/30.0.0` | wgpu 默认 feature | **200** | 未单独计时 |
| crates.io **tarball** `GET /api/v1/crates/{name}/0.8.0/download` | core 52,445 B + viewer 75,677 B | **200** | **8,586 ms（两包合计，Stopwatch）** |
| `tar -xzf` 解包 | 两包 | 成功 | < 1 s |
| docs.rs `web_fetch` 首页 ×2 | API 面 | **200** | 未单独计时 |
| docs.rs `Invoke-WebRequest` 首页 ×2（复核 items 计数） | API 面 | **200** | 未单独计时 |
| 抽样 crates.io 版本元数据 ×8 | 许可/描述 | **200** | ~2 s（含 250 ms 间隔） |

解包命令（本轮实际执行）：

```powershell
$tmp = Join-Path $env:TEMP 'crates-recon'; New-Item -ItemType Directory -Force $tmp | Out-Null
$ua = @{'User-Agent'='apeireth-build/2.0 (vendor-research)'}
Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 -Headers $ua `
  -Uri "https://crates.io/api/v1/crates/wgpu-3dgs-core/0.8.0/download" -OutFile "$tmp\core.crate"
Invoke-WebRequest -UseBasicParsing -TimeoutSec 30 -Headers $ua `
  -Uri "https://crates.io/api/v1/crates/wgpu-3dgs-viewer/0.8.0/download" -OutFile "$tmp\viewer.crate"
tar -xzf "$tmp\core.crate" -C $tmp; tar -xzf "$tmp\viewer.crate" -C $tmp
```

### B. 失败尝试（如实登记）

- **GitHub / raw.githubusercontent.com：一次都没试**（按 `github-fetch-guide.md` §〇 与任务书，该域 `web_fetch` 定死失败）。
- **crates.io / docs.rs：0 失败、0 重试**，全部 200。
- 工具层小磕碰 1 次：用通配符路径 `Get-ChildItem "$tmp\wgpu-3dgs-*\src" -Recurse` 列 shader 时返回空，改用显式路径重跑成功（非通道故障）。

### C. 拿到但未使用的材料

- 两包各自带 `Cargo.lock`（54,658 B / 89,713 B）：**只用于依赖闭包估算**，未据此推断任何编译结果。
- 上游 `CHANGELOG.md`（4,357 B / 5,732 B）：用于确认版本节奏与依赖升级路径（wgpu 26→27→28→29→30）。
- `coverage/README.md`：覆盖率徽章数据，本轮未取具体百分比。

### D. 材料受限（如实标注）

- **GitHub star / issue / 最近提交 / CI 状态：未查到**（未访问 GitHub 域）。
- **Windows-only 实际编译包数：未实测**（未跑 `cargo tree` / `cargo build`，遵守「不编译」约束）；222/226 是**全平台并集上界**。
- **`cargo deny check`：未实测**（会改依赖图，本批禁止）；第 7 节的冲突是**读配置 + 读 lockfile 的推算**。
- **运行时行为（WESL 编译耗时、帧率、显存峰值、两次运行是否字节一致）：未实测**（未编译未运行）；第 10 节的阈值是**建议值**，不是实测值。
- **全量 SPDX 核验：未做**，只抽了 8 个高风险包。

> 0 装声明：本报告所有版本号、下载量、许可、行数、依赖数均来自本轮实际执行命令的输出，命令与 URL 已随文给出；所有推算（内存字节数、依赖闭包、deny 冲突、验收阈值）均标明「推算 / 未实测」。未编译、未运行这两个 crate，未改动本仓库除本文件外的任何文件，未新增任何依赖。
