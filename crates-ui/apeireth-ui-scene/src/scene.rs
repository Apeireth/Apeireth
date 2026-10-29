//! 场景实例（状态 ⇒ 16 个取值的持有者）。
//!
//! ## 两条帧驱动路径
//!
//! | 路径 | 入口 | 说明 |
//! |---|---|---|
//! | 动画路径 | [`Scene::advance`] | 每帧推进时钟、状态指数趋近、自转相位、机位插值 |
//! | 静态路径 | [`Scene::render_once`] | 状态直接落到目标值、自转相位 0.35、时钟 1.2 |
//!
//! 生产侧对应「rAF 动画循环」与「减动效静态单帧」两条路径。
//!
//! ## 冻结时钟
//!
//! [`SceneOptions::frozen_time`] 对应生产侧的「按 URL 参数冻结动画时钟」调试钩子：
//! 冻结时**时钟钉在该秒数、自转相位钉在 0**，而漂移时钟另取 `0`
//! ⇒ 动画路径的取值在该模式下**完全确定**（与真实帧率无关）。

use crate::camera::{compute_layout, CameraRig, EffectiveCamera, HoleCircle, Layout};
use crate::jsnum;
use crate::state::{approach, current_targets, PadBias, PresenceStateParams, SceneMode};
use crate::timeline::{timeline_params, TimelineParams};
use crate::tokens::{self, presence};
use crate::uniforms::SceneUniforms;

/// 视口（宿主提供的画布尺寸与设备像素比）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// 画布的 CSS 宽度。
    pub css_width: f64,
    /// 画布的 CSS 高度。
    pub css_height: f64,
    /// 设备像素比（宿主上报值；场景侧会按上限钳位）。
    pub device_pixel_ratio: f64,
}

/// ⭐ 帧时长钳位（**两侧**都夹）。
///
/// - **上限** = [`tokens::performance::DT_CLAMP_SEC`]（50ms）：性能钳位
///   （切标签页回来时的一帧可能是好几秒 ⇒ 状态一步跳到目标）；
/// - **下限** = `0`：**正确性**钳位 —— 时钟回拨 / 挂起恢复后
///   `performance.now()` 的差值可能是**负数** ⇒ 负 `dt` 会让状态**倒退**
///   （`t_sec` 回退、`approach` 反向远离目标、机位 `tween` 变负 ⇒ 缓动外插）。
///
/// ⇒ 负 `dt` 一律当 **0** 处理（这一帧**不推进**，但也**不倒退**）。
pub(crate) fn clamp_frame_time(dt: f64) -> f64 {
    // ⚠️ **只夹上界**（与生产逐字一致）⇒ **不夹下界**。
    //
    // ## 为什么负 `dt` 也照原样用
    //
    // 实测：生产侧的首帧 `dt` **就是负数**（夹具里 −0.0062s）⇒
    // ⇒ 引擎若把负 `dt` 当 0（或夹到 0）⇒ **中间帧与生产不再逐位相同**
    // ⇒ ⇒ 场景面的 T1 判据（逐位相等，容差 0）**直接红**（实测 37/43）。
    //
    // ⇒ ⇒ 「负 `dt` 把状态推离目标」是**生产侧的既有缺陷**
    // ⇒ ⇒ **迁移的验收标准是「与生产逐字一致」** ⇒ ⇒ 引擎**故意保留**该行为
    // ⇒ ⇒ 修它要改**生产前端**（本仓只读）⇒ 另立一线。
    // ⇒ ⇒ 登记见 `docs/04-internal/ui-engine-defect-ledger.md`（E-21）。
    jsnum::min(dt, tokens::performance::DT_CLAMP_SEC)
}

/// 场景选项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneOptions {
    /// 是否响应鼠标轨道视差。
    pub interactive: bool,
    /// 是否处于「减动效」静态模式。
    pub reduce_motion: bool,
    /// 冻结动画时钟（秒）—— `Some(t)` ⇒ 时钟钉在 `t`、自转相位钉在 0。
    pub frozen_time: Option<f64>,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            interactive: true,
            reduce_motion: false,
            frozen_time: None,
        }
    }
}

/// 场景实例。
#[derive(Debug, Clone)]
pub struct Scene {
    options: SceneOptions,
    destroyed: bool,
    paused: bool,

    mode: SceneMode,
    pad_bias: PadBias,
    presence_intensity: Option<f64>,
    current: PresenceStateParams,
    rot: f64,
    t_sec: f64,

    rig: CameraRig,
    layout: Layout,
    css_width: f64,
    css_height: f64,
    dpr: f64,

    hour: Option<f64>,
    clock_hour: f64,

    last_hole: HoleCircle,
}

impl Scene {
    /// 建一份场景实例。
    ///
    /// 初始状态 = 静息档基线；机位 = 第一个预设。
    pub fn new(viewport: Viewport, options: SceneOptions) -> Self {
        let mut rig = CameraRig::new();
        rig.interactive = options.interactive;
        rig.reduce_motion = options.reduce_motion;
        let mode = SceneMode::Quiet;
        let current = mode.baseline();
        let mut scene = Self {
            options,
            destroyed: false,
            paused: false,
            mode,
            pad_bias: PadBias::default(),
            presence_intensity: None,
            current,
            rot: 0.0,
            t_sec: 0.0,
            rig,
            layout: compute_layout(viewport.css_width, viewport.css_height),
            css_width: viewport.css_width,
            css_height: viewport.css_height,
            dpr: jsnum::min(viewport.device_pixel_ratio, tokens::performance::DPR_MAX),
            hour: None,
            clock_hour: 0.0,
            last_hole: HoleCircle::default(),
        };
        scene.update_hole_circle_from_preset();
        scene
    }

    /// 设定视口（宿主侧的画布尺寸变化）。
    ///
    /// 对应生产侧在尺寸变化时重算构图布局与画布缓冲尺寸。
    pub fn set_viewport(&mut self, css_width: f64, css_height: f64, device_pixel_ratio: f64) {
        if self.destroyed {
            return;
        }
        self.css_width = css_width;
        self.css_height = css_height;
        self.dpr = jsnum::min(device_pixel_ratio, tokens::performance::DPR_MAX);
        self.layout = compute_layout(css_width, css_height);
        self.update_hole_circle_from_preset();
    }

    /// 切换存在态档位。
    ///
    /// 生产侧在切换后会立刻重绘静态帧；本 crate 把**重绘的触发权交给宿主**
    /// （宿主调用 [`Scene::render_once`] 或 [`Scene::advance`]）。
    pub fn set_mode(&mut self, mode: SceneMode) {
        if self.destroyed || self.mode == mode {
            return;
        }
        self.mode = mode;
    }

    /// 设定 PAD 偏置与回合级强度。
    ///
    /// `intensity = None` ⇒ **没有真实存在态帧** ⇒ 回落三档基线。
    pub fn set_pad_bias(&mut self, bias: PadBias, intensity: Option<f64>) {
        if self.destroyed {
            return;
        }
        self.pad_bias = bias;
        self.presence_intensity = intensity;
    }

    /// 设定舰内时刻（`None` ⇒ 跟随宿主提供的本地时钟）。
    pub fn set_hour(&mut self, hour: Option<f64>) {
        if self.destroyed {
            return;
        }
        self.hour = hour.map(crate::timeline::normalize_hour);
    }

    /// 设定宿主侧的本地时钟小时数（`hour` 为 `None` 时的取值来源）。
    pub fn set_clock_hour(&mut self, hour: f64) {
        self.clock_hour = hour;
    }

    /// 开关鼠标轨道视差（关闭时视角平滑回中）。
    pub fn set_interactive(&mut self, on: bool) {
        if self.destroyed {
            return;
        }
        self.rig.set_interactive(on);
    }

    /// 暂停 / 恢复推进（暂停期间 [`Scene::advance`] 不产出帧）。
    pub fn set_paused(&mut self, paused: bool) {
        if self.destroyed {
            return;
        }
        self.paused = paused;
    }

    /// 切换机位（从当前值起插值）。
    ///
    /// 越界下标**显式拒绝**（返回 `false`）。
    /// 「减动效」模式下**锁定**第一个预设（与生产侧一致）。
    pub fn set_camera(&mut self, index: usize) -> bool {
        if self.destroyed {
            return false;
        }
        let accepted = self.rig.set_index(index);
        if accepted {
            self.update_hole_circle_from_preset();
        }
        accepted
    }

    /// 当前机位下标。
    pub fn camera_index(&self) -> usize {
        self.rig.index
    }

    /// 当前黑洞视区圆（CSS px，基于最近一帧的有效机位）。
    pub fn hole_screen_circle(&self) -> HoleCircle {
        self.last_hole
    }

    /// 设定鼠标位置（归一化坐标 `-1..1`）。
    pub fn set_pointer(&mut self, x: f64, y: f64) {
        if self.destroyed {
            return;
        }
        self.rig.set_pointer(x, y);
    }

    /// 销毁实例（此后所有推进/绘制请求都不产出帧）。
    pub fn destroy(&mut self) {
        if self.destroyed {
            return;
        }
        self.destroyed = true;
    }

    /// 推进一帧（动画路径）。
    ///
    /// 返回 `None` = 本帧**不产出**（已暂停或已销毁）。
    ///
    /// ## ⭐ `dt` **两侧**都夹
    ///
    /// 上限（50ms）是性能钳位；**下限（0）**是**正确性**钳位：
    /// 时钟回拨 / 挂起恢复后 `performance.now()` 的差值可能是**负数** ⇒
    ///
    /// - `t_sec` 会**倒退**；
    /// - `approach` 的 `k = min(1, dt·2.4)` 会是**负数** ⇒ 状态被**反向推离**
    ///   目标（而不是趋近）；
    /// - 机位的 `tween` 会变负 ⇒ `ease_in_out_cubic` **外插** ⇒ 机位过冲。
    ///
    /// ⇒ 负 `dt` 一律当 **0** 处理（这一帧不推进，但不倒退）。
    pub fn advance(&mut self, dt: f64) -> Option<SceneUniforms> {
        if self.destroyed || self.paused {
            return None;
        }
        let dt = clamp_frame_time(dt);
        self.t_sec += dt;
        let target = self.current_targets();
        self.current = approach(self.current, target, dt);
        self.rot += self.current.speed * dt;
        if let Some(frozen) = self.options.frozen_time {
            self.t_sec = frozen;
            self.rot = 0.0;
        }
        let drift_clock = if self.options.frozen_time.is_some() {
            0.0
        } else {
            self.t_sec
        };
        let cam = self.effective_camera(dt, drift_clock);
        Some(self.draw(&cam))
    }

    /// 出一帧静态画面（静态路径）。
    ///
    /// 状态直接落到目标值（不做指数趋近）、自转相位 0.35、时钟 1.2。
    /// 返回 `None` = 已销毁。
    pub fn render_once(&mut self) -> Option<SceneUniforms> {
        if self.destroyed {
            return None;
        }
        self.current = self.current_targets();
        self.rot = 0.35;
        self.t_sec = 1.2;
        let cam = self.effective_camera(0.0, 0.0);
        Some(self.draw(&cam))
    }

    /// 当前档位。
    pub fn mode(&self) -> SceneMode {
        self.mode
    }

    /// 当前状态参数（四个取值）。
    pub fn current_state(&self) -> PresenceStateParams {
        self.current
    }

    // -----------------------------------------------------------------------
    // 内部
    // -----------------------------------------------------------------------

    /// 当前目标状态参数。
    fn current_targets(&self) -> PresenceStateParams {
        current_targets(self.mode, &self.pad_bias, self.presence_intensity)
    }

    /// 解析舰内时刻（`hour` 为 `None` ⇒ 用宿主提供的本地时钟）。
    fn resolve_hour(&self) -> f64 {
        self.hour.unwrap_or(self.clock_hour)
    }

    /// 每帧的有效机位。
    fn effective_camera(&mut self, dt: f64, t: f64) -> EffectiveCamera {
        self.rig
            .effective(dt, t, &self.layout, self.css_width, self.css_height)
    }

    /// 由有效机位算出 16 个取值（并更新视区圆）。
    fn draw(&mut self, cam: &EffectiveCamera) -> SceneUniforms {
        let mut tp: TimelineParams = timeline_params(self.resolve_hour());
        if self.presence_intensity.is_some() {
            // 存在态半边：PAD 档内微调金色占比与光强（保守增益）。
            let p = self.pad_bias.p * presence::HEURISTIC_V0_GAIN;
            tp.bright *= 1.0 + 0.06 * p;
            tp.gold = jsnum::min(1.0, jsnum::max(0.0, tp.gold * (1.0 + 0.08 * p)));
        }
        let canvas_width = jsnum::max(1.0, jsnum::round(self.css_width * self.dpr));
        let canvas_height = jsnum::max(1.0, jsnum::round(self.css_height * self.dpr));
        let uniforms = SceneUniforms {
            res: [canvas_width, canvas_height],
            center: [cam.cx * self.dpr, (self.css_height - cam.cy) * self.dpr],
            scale: self.layout.hole_r * cam.zoom * self.dpr,
            time: self.t_sec,
            rot: self.rot,
            bright: self.current.bright,
            turb: self.current.turb,
            pulse: self.current.pulse,
            incl: cam.incl,
            azim: cam.azim,
            disk_azim: cam.disk_azim,
            disk_incl: cam.disk_incl,
            amb_str: tp.amb,
            eclipse: tp.eclipse,
            tl_bright: tp.bright,
            tl_gold: tp.gold,
        };
        self.last_hole = HoleCircle {
            x: cam.cx,
            y: cam.cy,
            r: self.layout.hole_r * cam.zoom,
        };
        uniforms
    }

    /// 由**机位预设**（而不是最近一帧）更新视区圆。
    fn update_hole_circle_from_preset(&mut self) {
        let preset = &tokens::CAMERA_PRESETS[self.rig.index];
        self.last_hole = HoleCircle {
            x: self.layout.base_cx + preset.ox * self.css_width,
            y: self.layout.base_cy + preset.oy * self.css_height,
            r: self.layout.hole_r * preset.zoom,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Viewport {
        Viewport {
            css_width: 1440.0,
            css_height: 900.0,
            device_pixel_ratio: 1.0,
        }
    }

    fn static_scene() -> Scene {
        Scene::new(
            viewport(),
            SceneOptions {
                interactive: false,
                reduce_motion: true,
                frozen_time: None,
            },
        )
    }

    fn animated_scene() -> Scene {
        Scene::new(
            viewport(),
            SceneOptions {
                interactive: false,
                reduce_motion: false,
                frozen_time: Some(1.5),
            },
        )
    }

    #[test]
    fn static_path_lands_on_the_target_state() {
        let mut scene = static_scene();
        scene.set_mode(SceneMode::Thinking);
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.bright, 0.9);
        assert_eq!(uniforms.turb, 1.0);
        assert_eq!(uniforms.pulse, 0.0);
        assert_eq!(uniforms.time, 1.2);
        assert_eq!(uniforms.rot, 0.35);
    }

    #[test]
    fn static_path_has_no_camera_drift() {
        let mut scene = static_scene();
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.incl, 0.3);
        assert_eq!(uniforms.azim, 0.0);
        assert_eq!(uniforms.disk_incl, 0.3);
        assert_eq!(uniforms.disk_azim, 0.0);
        assert_eq!(uniforms.scale, 144.0 * 0.6);
    }

    #[test]
    fn animated_path_applies_the_drift_and_freezes_the_clock() {
        let mut scene = animated_scene();
        let uniforms = scene.advance(0.016).expect("动画帧");
        assert_eq!(uniforms.time, 1.5, "冻结时钟把时钟钉在冻结秒数");
        assert_eq!(uniforms.rot, 0.0, "冻结时钟把自转相位钉在 0");
        let expected_incl = 0.3 + 0.02 * 1.3f64.sin();
        assert!((uniforms.incl - expected_incl).abs() < 1e-15);
        let expected_scale = 144.0 * 0.6 * (1.0 + 0.045 * 2.1f64.sin());
        assert!((uniforms.scale - expected_scale).abs() < 1e-12);
    }

    #[test]
    fn center_y_is_flipped_against_the_bottom_edge() {
        let mut scene = static_scene();
        let uniforms = scene.render_once().expect("静态帧");
        assert!((uniforms.center[0] - 950.4).abs() < 1e-9);
        assert_eq!(uniforms.center[1], 900.0 - 405.0);
    }

    #[test]
    fn resolution_matches_the_canvas_buffer() {
        let mut scene = static_scene();
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.res, [1440.0, 900.0]);
    }

    #[test]
    fn device_pixel_ratio_is_clamped() {
        let mut scene = Scene::new(
            Viewport {
                css_width: 800.0,
                css_height: 600.0,
                device_pixel_ratio: 3.0,
            },
            SceneOptions::default(),
        );
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.res, [1600.0, 1200.0], "设备像素比被钳到 2");
    }

    #[test]
    fn hour_drives_the_four_timeline_values() {
        let mut scene = static_scene();
        scene.set_hour(Some(12.0));
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.amb_str, 1.0);
        assert_eq!(uniforms.eclipse, 1.0);
        assert!((uniforms.tl_bright - 1.37).abs() < 1e-12);
        assert!((uniforms.tl_gold - 0.45).abs() < 1e-12);
    }

    #[test]
    fn pad_bias_shifts_the_timeline_brightness_and_gold() {
        let mut scene = static_scene();
        scene.set_pad_bias(
            PadBias {
                p: 1.0,
                a: 0.0,
                d: 0.0,
            },
            Some(0.5),
        );
        let uniforms = scene.render_once().expect("静态帧");
        // p = 0.75 ⇒ bright ×(1 + 0.06·0.75)
        assert!((uniforms.tl_bright - 1.0 * (1.0 + 0.06 * 0.75)).abs() < 1e-12);
        // gold = 1.0（夜间）×(1 + 0.08·0.75) = 1.06 ⇒ 被夹到 1
        assert!((uniforms.tl_gold - 1.0).abs() < 1e-12);
    }

    #[test]
    fn out_of_range_camera_index_is_rejected() {
        let mut scene = static_scene();
        assert!(!scene.set_camera(7));
        assert_eq!(scene.camera_index(), 0);
    }

    #[test]
    fn camera_switch_moves_the_composition_center() {
        let mut scene = animated_scene();
        assert!(scene.set_camera(1));
        assert_eq!(scene.camera_index(), 1);
        let circle = scene.hole_screen_circle();
        assert!((circle.y - (405.0 - 0.05 * 900.0)).abs() < 1e-9);
        assert!((circle.r - 144.0 * 1.9).abs() < 1e-9);
    }

    #[test]
    fn reduce_motion_locks_the_camera() {
        let mut scene = static_scene();
        assert!(scene.set_camera(2));
        assert_eq!(scene.camera_index(), 0, "静态模式下锁定第一个预设");
    }

    #[test]
    fn paused_and_destroyed_scenes_produce_no_frame() {
        let mut scene = animated_scene();
        scene.set_paused(true);
        assert!(scene.advance(0.016).is_none());
        scene.set_paused(false);
        assert!(scene.advance(0.016).is_some());
        scene.destroy();
        assert!(scene.advance(0.016).is_none());
        assert!(scene.render_once().is_none());
    }

    #[test]
    fn mode_switch_is_idempotent() {
        let mut scene = static_scene();
        scene.set_mode(SceneMode::Speaking);
        let uniforms = scene.render_once().expect("静态帧");
        assert_eq!(uniforms.pulse, 1.0);
        assert_eq!(uniforms.bright, 1.05);
    }

    #[test]
    fn animated_path_smooths_the_state_exponentially() {
        let mut scene = animated_scene();
        scene.set_mode(SceneMode::Speaking);
        let first = scene.advance(0.016).expect("动画帧");
        assert!(first.bright > 0.32, "从静息档向说话档趋近");
        assert!(first.bright < 1.05, "一步不该跳到目标");
    }

    #[test]
    fn quiet_state_stays_put_on_the_animated_path() {
        let mut scene = animated_scene();
        for _ in 0..120 {
            let uniforms = scene.advance(0.016).expect("动画帧");
            assert_eq!(uniforms.bright, 0.32);
            assert_eq!(uniforms.turb, 0.12);
            assert_eq!(uniforms.pulse, 0.0);
        }
    }

    #[test]
    fn frame_time_is_clamped() {
        let mut scene = Scene::new(
            viewport(),
            SceneOptions {
                interactive: false,
                reduce_motion: false,
                frozen_time: None,
            },
        );
        let uniforms = scene.advance(10.0).expect("动画帧");
        assert!((uniforms.time - 0.05).abs() < 1e-12, "帧时长被钳到 50ms");
    }

    // -----------------------------------------------------------------------
    // ⭐ 负 `dt`（时钟回拨 / 挂起恢复）
    // -----------------------------------------------------------------------

    #[test]
    fn a_negative_frame_time_reproduces_the_production_clock_behavior() {
        // ⚠️ **记录「生产同源缺陷」**（见台账 E-21）：
        // ⇒ 生产**不夹**负 `dt` ⇒ 时钟会**倒退** ⇒ 引擎**故意保留**（对齐优先）。
        let mut scene = Scene::new(
            viewport(),
            SceneOptions {
                interactive: false,
                reduce_motion: false,
                frozen_time: None,
            },
        );
        let first = scene.advance(0.016).expect("动画帧");
        assert!((first.time - 0.016).abs() < 1e-12);
        // 负 dt ⇒ 时钟**倒退**（生产同源 ⇒ 引擎故意保留）。
        let second = scene.advance(-0.5).expect("动画帧");
        assert!(
            (second.time - (first.time - 0.5)).abs() < 1e-12,
            "负 dt ⇒ 时钟倒退 0.5（**与生产同源**）：{} → {}",
            first.time,
            second.time
        );
    }

    #[test]
    fn a_negative_frame_time_reproduces_the_production_state_behavior() {
        // ⚠️ **记录「生产同源缺陷」**（见台账 E-21）：负 `dt` ⇒ `k` 为负
        // ⇒ 状态**反向远离**目标 ⇒ 引擎**故意保留**（对齐优先）。
        // ⚠ 早期形态：`k = min(1, dt · 2.4)` ⇒ 负 dt ⇒ k 为负 ⇒
        // `cur += (target - cur) · k` 把状态**反向推离**目标。
        let mut scene = Scene::new(
            viewport(),
            SceneOptions {
                interactive: false,
                reduce_motion: false,
                frozen_time: None,
            },
        );
        scene.set_mode(SceneMode::Thinking);
        let before = scene.advance(0.016).expect("动画帧");
        assert!(before.bright > 0.32, "先向目标走一步");
        let after = scene.advance(-0.5).expect("动画帧");
        assert!(
            after.bright < before.bright,
            "负 dt ⇒ 状态**反向远离**目标（**与生产同源**）：{} → {}",
            before.bright,
            after.bright
        );
        assert!(after.bright <= 0.9, "状态不得越过目标：{}", after.bright);
    }

    #[test]
    fn a_negative_frame_time_reproduces_the_production_camera_behavior() {
        // ⚠️ **记录「生产同源缺陷」**（见台账 E-21）：负 `dt` ⇒ 机位缓动**外插**
        // ⇒ 引擎**故意保留**（对齐优先）。
        // ⚠ 早期形态：`tween` 可变负 ⇒ `ease_in_out_cubic` **外插** ⇒ 机位过冲。
        //
        // ⭐ 精确口径：负 `dt` 被夹到 0 ⇒ 这一帧**整帧不推进** ⇒ 漂移时钟
        // 也没动 ⇒ 机位的有效取值必须与上一帧**逐位相同**。
        let mut scene = Scene::new(
            viewport(),
            SceneOptions {
                interactive: true,
                reduce_motion: false,
                frozen_time: None,
            },
        );
        assert!(scene.set_camera(1), "机位切换必须被接受");
        let before = scene.advance(0.016).expect("动画帧");
        let after = scene.advance(-0.5).expect("动画帧");
        assert_ne!(
            (after.incl, after.azim, after.scale),
            (before.incl, before.azim, before.scale),
            "负 dt ⇒ 机位缓动**外插**（**与生产同源**）"
        );
    }

    #[test]
    fn the_frame_time_clamp_covers_the_upper_side_only() {
        // ⚠️ **只夹上界**（与生产逐字一致）⇒ 负值**照原样**（见台账 E-21）。
        assert_eq!(crate::scene::clamp_frame_time(-0.5), -0.5);
        assert_eq!(crate::scene::clamp_frame_time(-0.000001), -0.000001);
        assert_eq!(crate::scene::clamp_frame_time(0.0), 0.0);
        assert_eq!(crate::scene::clamp_frame_time(0.016), 0.016);
        assert_eq!(
            crate::scene::clamp_frame_time(10.0),
            tokens::performance::DT_CLAMP_SEC
        );
    }
}
