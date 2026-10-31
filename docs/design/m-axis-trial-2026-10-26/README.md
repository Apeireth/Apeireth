# M 轴 · 3D 高斯溅射隔离试验田（2026-10-26）

D19 裁定的落地物：**独立 cargo 工作区**（不进主仓依赖图、不碰主 `Cargo.lock`、不碰 `deny.toml`）。

## 跑法

```bash
cd docs/design/m-axis-trial-2026-10-26
cargo run --release -- /tmp/m-axis-trial/out.png
```

- 输入：程序**自建**固定点云 `fixed.ply`（2000 个高斯，位置/颜色全由索引算出 ⇒ 无随机）。
- 相机：`pos = (0,0,-3)`、`yaw = 0`、`pitch = 0`、垂直 FOV 60°（**全写死**）。
- 输出：1280×720 `Rgba8UnormSrgb` PNG（**全链路无 `Surface`** ⇒ headless）。

## 验收判据与实测结果（2026-10-26，Linux + AMD Radeon 8060S / Vulkan 1.4.354）

| # | 判据 | 门 | 实测 | 结论 |
|---|---|---|---|---|
| A1 | 固定 ply + 固定相机 + 1280×720 + 无窗口 | 必须成立 | 成立 | ✅ |
| A2 | 同机连续两次 PNG 字节一致 | SHA-256 相等 | **6 次运行 SHA-256 全同** | ✅ |
| A4 | 画面非空 | 覆盖率 ≥5%，中心 ±5% | 覆盖率 **0.1895**，中心命中 **0.9439** | ✅ |
| A5 | 720p 单帧（含 viewer 构建） | < 5 s | **0.28–1.18 s**，峰值 RSS **88 MB** | ✅ |

**golden SHA-256**：`f8b5b6d8de31d5b3cc0c3b2a56ad61a9b5be6ab6487b9eabf78e013131584d93`
（字节级回归基准 ⇒ 不必提交 PNG 本体；重跑比 hash 即可）

## 四个真实踩坑（写在这里省下一个人一天）

1. **`required_limits` 不能用 `downlevel_webgl2_defaults()`** ⇒ 它的 storage-buffer 上限是 **0**
   ⇒ `Viewer::new` 报 `ModelSizeExceedsDeviceLimit { device_limit: 0 }`。
   ⇒ 用 `adapter.limits()`。
2. **相机朝向**（源码 `camera.rs::get_forward`）：`forward = (cos(pitch)·sin(yaw), sin(pitch), cos(pitch)·cos(yaw))`
   ⇒ **`yaw = 0` 时朝 +Z 望** ⇒ 相机放在 **+Z** 会**背对**点云 ⇒ **全黑帧**（我踩了）。
3. **`unmap` 前必须先释放映射视图** ⇒ 否则 `You cannot unmap a buffer that still has accessible mapped views`。
4. **wgpu 30 的 API 变化**：`Instance::default()`（不是 `Instance::new(&InstanceDescriptor::default())`）、
   `RequestAdapterOptions` 多 `apply_limit_buckets`、`DeviceDescriptor` 多 `experimental_features`、
   `poll(PollType::wait_indefinitely())`、`on_uncaptured_error(Arc::new(..))`。

## 边界（不夸大）

- 本试验田只证明「**能 headless 出帧、且字节可复现**」，
  **不**证明「应该搬进主仓」—— 搬入代价由 [m-axis-3d-recon-wgpu3dgs-2026-10-18.md](../m-axis-3d-recon-wgpu3dgs-2026-10-18.md) 单独记账。
- **本机只测了 1 万点以内的规模**（2000 点）⇒ 「100 万点不 OOM」（A5 后半）**未验**。
- **只在本机 Linux + 一块 AMD 集显上验过** ⇒ 跨驱动/跨平台可复现性**未验**。
