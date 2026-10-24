//! 场景层语义（**uniform 取值链 + 生命周期**）—— 宿主侧。
//!
//! ## 职责边界
//!
//! 本 crate 是**生产场景类**的「状态 ⇒ 16 个 uniform 取值」这条链在 Rust 侧的
//! 对齐实现（取值与生产侧**逐位**相同，见下方证据）。
//! ⇒ 着色器**不在**本 crate（由 naga 在构建期从 GLSL 转译）；
//! ⇒ 绘制**不在**本 crate（由 `apeireth-ui-gpu` 承担）。
//!
//! ## 为什么独立成一个 crate
//!
//! - ADR-0015 §2.6 明确：uniform 的**取值来源属宿主侧场景语义**，引擎只做
//!   「uniform → 着色器」的管道 ⇒ 本 crate 是**宿主侧**，不应塞进引擎；
//! - 依赖方向保持**单向**（场景 → 引擎）⇒ 引擎不因场景而膨胀；
//! - 未来若走**原生窗口路径**，宿主侧场景语义需要**跨宿主复用** ⇒ 独立 crate 便于搬运。
//!
//! ## 模块
//!
//! | 模块 | 内容 |
//! |---|---|
//! | [`tokens`] | 设计令牌（机位预设、构图比例、漂移、动效、性能钳位、三档基线） |
//! | [`timeline`] | 舰内时刻 ⇒ 四个时间线取值（纯函数） |
//! | [`state`] | 存在态三档 + PAD 映射 + 指数趋近 |
//! | [`camera`] | 机位预设 / 插值 / 漂移 / 鼠标视差 / 构图布局 |
//! | [`scene`] | 场景实例：生命周期 + 两条帧驱动路径 |
//! | [`uniforms`] | 16 个取值的容器与投递（写进引擎的 uniform 集合） |
//! | [`jsnum`] | 与宿主脚本引擎一致的数值原语（NaN / `±0` / 取整口径） |
//!
//! ## 诚实边界
//!
//! - 本 crate **不**含 Canvas 2D 降级路径（生产场景类有）⇒ 降级路径仍留生产侧
//!   （wgpu 在无 WebGL2 时同样不可用 ⇒ 降级路径**无法**由本 crate 取代）。
//! - 本 crate **不**含粒子系统（属另一模块）。
//! - 本 crate **不**含片头火焰（属另一模块）。
//! - 机位预设 / 状态档位的**展示文案**不参与任何取值 ⇒ **未**纳入本 crate。
//! - 本 crate 的取值**已在真实浏览器里与生产侧逐值对照**（证据见
//!   `crates-ui/apeireth-ui-demo/harness/evidence/scene/`）。

#![forbid(unsafe_code)]

pub mod camera;
pub mod jsnum;
pub mod scene;
pub mod state;
pub mod timeline;
pub mod tokens;
pub mod uniforms;

/// 逐值对照的**原生可复跑**夹具（生产侧实测抓取值的快照）。
///
/// ⚠️ 只在测试期编译：夹具里的数值是**记录**，不是逻辑。
#[cfg(test)]
mod parity;

pub use camera::{CameraRig, CameraState, EffectiveCamera, HoleCircle, Layout};
pub use scene::{Scene, SceneOptions, Viewport};
pub use state::{PadBias, PresenceStateParams, SceneMode};
pub use timeline::{timeline_params, TimelineParams};
pub use uniforms::SceneUniforms;
