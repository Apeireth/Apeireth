# M 轴 · 3D 高斯溅射隔离试验田实测（2026-10-26）

> **本文回答 D19 的下一步**：`wgpu-3dgs-core` / `wgpu-3dgs-viewer` 0.8.0 的
> **「先证明能 headless 出帧」**这条前置，**现在有实测结论了**（不再是侦察报告的推断）。
>
> - 侦察报告（2026-10-18）：[m-axis-3d-recon-wgpu3dgs-2026-10-18.md](m-axis-3d-recon-wgpu3dgs-2026-10-18.md)
> - 试验田源码：[m-axis-trial-2026-10-26/](m-axis-trial-2026-10-26/)（**独立 cargo 工作区**）
> - 债栏：`docs/04-internal/backend-enhancement/00-backlog.md` 的 **D19** 行

## 结论（先行）

**A1 / A2 / A4 / A5 四条判据全部通过** ⇒ **「headless 出帧 + 字节可复现」这条前置被实证**。

**但 D19 的裁定不变：维持「暂不立项」（不进主仓依赖图）。**
理由不是「能不能跑」，而是**搬入主仓的代价**：
① 依赖闭包**实测 178 个 crate 在本机实际编出**（`cargo` 解析出的 lock 共 232 个 package，含各平台变体）；
② `deny.toml` 的 `multiple-versions = "deny"` 与 `wesl`（单依赖带进 104 包）冲突；
③ 上游单人维护、0.8.0 预 1.0、每 3 个月跟一次 `wgpu` 大版本 ⇒ 必须 vendored 隔离。

⇒ **一句话**：**技术可行 = 已证实；该不该进主仓 = 仍然不该**（代价与收益不成比例）。

## 实测环境（如实标注）

| 项 | 值 |
|---|---|
| OS | Linux（本机，**非** owner 的 Windows 机） |
| GPU | **AMD Radeon 8060S Graphics (RADV STRIX_HALO)**，`IntegratedGpu`，backend **Vulkan** |
| Vulkan | **1.4.354**（另有 **llvmpipe** CPU 设备在同机可见 ⇒ CPU 回退路径存在，但本轮**未跑** CPU 回退） |
| 依赖 | `wgpu 30.0.1` / `wgpu-3dgs-core 0.8.0` / `wgpu-3dgs-viewer 0.8.0` / `glam 0.33` |
| 点云 | **自建** 2000 个高斯（球面壳 + 色相环；位置/颜色全由索引算出 ⇒ 无随机 ⇒ 可复现） |
| 相机 | `pos=(0,0,-3)`、`yaw=0`、`pitch=0`、垂直 FOV 60°、z 区间 `0.1..1e4`（**全写死**） |
| 目标 | 1280×720 `Rgba8UnormSrgb` 离屏纹理，**全链路无 `Surface`** |

## 判据逐条

| # | 判据（抄侦察报告 §10） | 门 | 实测 | 结论 |
|---|---|---|---|---|
| A1 | 固定 ply + 固定相机 + 1280×720 + `Rgba8UnormSrgb` + **不开窗口** | 必须成立 | 成立（headless，adapter = 集显，纹理自建） | ✅ |
| A2 | 同机同驱动连续两次运行 PNG 字节一致 | SHA-256 相等 | **6 次运行 SHA-256 全同**（`f8b5b6d8…84d93`）⇒ **比降级判据（PSNR ≥50 dB）更严地通过** | ✅ |
| A3 | 回归基线 | 与首版 golden PSNR ≥45 dB | **字节级** ⇒ 基线即 SHA-256 | ✅ |
| A4 | 画面非空 | 覆盖率 ≥5%，中心 ±5% | 覆盖率 **0.1895**，中心区命中 **0.9439** | ✅ |
| A5 | 性能基线 | 720p 单帧 < 5 s；100 万点不 OOM | **0.28–1.18 s**（含 viewer 构建 + 3 次 WESL 编译 + 排序 + 渲染 + 回读 + 写 PNG）；峰值 RSS **88 MB**；**100 万点未验** | ✅（前半） |

**golden SHA-256**：`f8b5b6d8de31d5b3cc0c3b2a56ad61a9b5be6ab6487b9eabf78e013131584d93`
⇒ 字节级回归基准 ⇒ **不必提交 PNG 本体**，重跑比 hash 即可。

## 搬入主仓的代价（本轮实测，把侦察报告的估算换掉）

| 指标 | 侦察报告（2026-10-18 估算） | 本轮实测 | 差异 |
|---|---|---|---|
| 依赖闭包 | 84–85 包（手工数「普通依赖」子图） | `Cargo.lock` **232 个 package**；本机 Linux **实际编出 178 个 crate** | 口径不同（lock 含各平台变体），但**量级明显更大** |
| `glam` 重复 | 报告称 `thiserror`/`foldhash`/`bit-set` 需 skip | **`glam` 本身就要两份**（若 pin 0.30）⇒ 必须 pin 0.33 才只有一份 | **新增一条 deny 风险**（`deny.toml` `multiple-versions = "deny"`） |
| 纹理回读 | 库**无**纹理回读 | 确认：**必须自己写** `copy_texture_to_buffer` + 256B 行对齐（本次 1280×4 = 5120，天然对齐） | 一致 |

⚠️ **一处自我更正**：我最初把「`glam` 两版本」当成闭包缺陷记下，**后来发现那是我自己 pin `glam = "0.30"` 造成的**
（两个 3dgs crate 都要 0.33）⇒ **不是上游闭包的问题**。已在试验田 `Cargo.toml` 里 pin 成 0.33 并在注释里写明。

## 四个真实踩坑（省下一个人一天）

1. **`required_limits` 不能用 `downlevel_webgl2_defaults()`** ⇒ 它把 storage-buffer 上限设成 **0**
   ⇒ `Viewer::new` 直接报 `PreprocessorCreate(ModelSizeExceedsDeviceLimit { device_limit: 0 })`。
   ⇒ 用 `adapter.limits()`。
2. **相机朝向是反的直觉**（源码 `camera.rs::get_forward`）：
   `forward = (cos(pitch)·sin(yaw), sin(pitch), cos(pitch)·cos(yaw))` ⇒ **`yaw = 0` 时朝 +Z 望**
   ⇒ 相机放在 **+Z** 会**背对**点云 ⇒ **全黑帧**（本轮踩到，靠 A4 的覆盖率 0.0000 暴露出来）。
3. **`unmap` 前必须先释放映射视图** ⇒ 否则 `You cannot unmap a buffer that still has accessible mapped views`。
4. **wgpu 30 的 API 变化**（相对 wgpu 25/26 的常见写法）：
   `Instance::default()`（不是 `Instance::new(&InstanceDescriptor::default())`）、
   `RequestAdapterOptions` 多 `apply_limit_buckets`、
   `DeviceDescriptor` 多 `experimental_features`、
   `poll(PollType::wait_indefinitely())`、`on_uncaptured_error(Arc::new(..))`。

## 本轮的**边界**（不夸大）

- 只证明「**能 headless 出帧、且字节可复现**」⇒ **不**证明「应该搬进主仓」。
- **只在本机 Linux + 一块 AMD 集显上验过** ⇒ **跨驱动 / 跨平台 / Windows 机**的可复现性**未验**。
- **CPU 回退路径**（llvmpipe / lavapipe）**未跑**（本轮走了真 GPU）。
- **规模只到 2000 点** ⇒ A5 的「100 万点不 OOM」**未验**（88 MB 是 2000 点的数）。
- **WESL 解释执行的启动成本**在本轮被总耗时（0.28–1.18 s）掩盖，**未单独拆解**。

## 下一步（若要继续推进，按代价从小到大）

1. **补 CPU 回退跑一次**（`force_fallback_adapter = true`）⇒ 证明「普通家用机无独显也能出帧」—— 这是产品红线级的判据。
2. **补 100 万点规模**（合成大点云）⇒ 验 A5 后半 + 峰值 RSS 曲线。
3. **补跨平台**：在 owner 的 Windows 机上跑同一程序（**同一份代码**）⇒ 验 SHA-256 是否仍一致。
4. 以上三条都过之后，再谈「要不要 vendored 隔离进主仓」—— **那一步要过 `cargo deny` 与 `deny.toml` 的 skip 决策**，
   属于**主仓依赖图**的改动，仍需单独拍板。

## 附：复现命令

```bash
cd docs/design/m-axis-trial-2026-10-26
cargo run --release -- /tmp/m-axis-trial/out.png
sha256sum /tmp/m-axis-trial/out.png
# 期望: f8b5b6d8de31d5b3cc0c3b2a56ad61a9b5be6ab6487b9eabf78e013131584d93
```
