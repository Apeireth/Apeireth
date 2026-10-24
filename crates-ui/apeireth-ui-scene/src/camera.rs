//! 机位：预设、插值、漂移、鼠标轨道视差、构图布局。
//!
//! ## 两条角度通道
//!
//! - `incl` / `azim`：**含**鼠标轨道视差（驱动星野三层视差）；
//! - `disk_incl` / `disk_azim`：**不含**鼠标视差（驱动盘面姿态）——
//!   盘面与黑洞本体对鼠标零响应。
//!
//! ## 构图
//!
//! 黑洞构图中心永远钉死（`ox` / `oy` 只来自机位预设）；视区半径 = 布局半径 × 视距。

use crate::jsnum;
use crate::tokens::{self, CAMERA_PRESETS};

/// 机位五元组。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// 盘面倾角压扁系数。
    pub incl: f64,
    /// 视距缩放。
    pub zoom: f64,
    /// 轨道方位角。
    pub azim: f64,
    /// 构图横向偏移（屏宽比例）。
    pub ox: f64,
    /// 构图纵向偏移（屏高比例）。
    pub oy: f64,
}

impl CameraState {
    /// 由预设构造。
    pub fn from_preset(preset: &tokens::CameraPreset) -> Self {
        Self {
            incl: preset.incl,
            zoom: preset.zoom,
            azim: preset.azim,
            ox: preset.ox,
            oy: preset.oy,
        }
    }
}

/// 每帧算出的有效机位。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveCamera {
    /// 视角倾角（含鼠标视差）。
    pub incl: f64,
    /// 视距缩放（含乘性漂移）。
    pub zoom: f64,
    /// 视角方位角（含鼠标视差）。
    pub azim: f64,
    /// 盘面姿态倾角（不含鼠标视差）。
    pub disk_incl: f64,
    /// 盘面姿态方位角（不含鼠标视差）。
    pub disk_azim: f64,
    /// 构图中心 x（CSS px）。
    pub cx: f64,
    /// 构图中心 y（CSS px）。
    pub cy: f64,
}

/// 构图布局（CSS px）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// 构图中心 x。
    pub base_cx: f64,
    /// 构图中心 y。
    pub base_cy: f64,
    /// 黑洞视区半径。
    pub hole_r: f64,
}

/// 由视口尺寸算构图布局。
///
/// - 短边 `≤ 820px` ⇒ 移动端比例；否则桌面端比例；
/// - 视区半径 = 短边 × 比例。
pub fn compute_layout(css_w: f64, css_h: f64) -> Layout {
    let short = jsnum::min(css_w, css_h);
    if css_w <= tokens::layout::BREAKPOINT_PX {
        Layout {
            base_cx: css_w * tokens::layout::MOBILE_CX_RATIO,
            base_cy: css_h * tokens::layout::MOBILE_CY_RATIO,
            hole_r: short * tokens::layout::MOBILE_HOLE_RADIUS_RATIO,
        }
    } else {
        Layout {
            base_cx: css_w * tokens::layout::DESKTOP_CX_RATIO,
            base_cy: css_h * tokens::layout::DESKTOP_CY_RATIO,
            hole_r: short * tokens::layout::DESKTOP_HOLE_RADIUS_RATIO,
        }
    }
}

/// 缓动（easeInOutCubic）。
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// 机位装置（可变状态）。
#[derive(Debug, Clone, PartialEq)]
pub struct CameraRig {
    /// 当前预设下标。
    pub index: usize,
    /// 当前机位五元组（插值结果）。
    pub current: CameraState,
    /// 插值起点。
    pub from: CameraState,
    /// 插值进度（`1` = 已到位）。
    pub tween: f64,
    /// 鼠标当前位置（归一化）。
    pub mouse_x: f64,
    /// 鼠标当前位置（归一化）。
    pub mouse_y: f64,
    /// 鼠标目标位置（归一化）。
    pub mouse_target_x: f64,
    /// 鼠标目标位置（归一化）。
    pub mouse_target_y: f64,
    /// 是否响应鼠标轨道视差。
    pub interactive: bool,
    /// 是否处于「减动效」静态模式（静态模式**不**施加自主漂移与鼠标视差）。
    pub reduce_motion: bool,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraRig {
    /// 建一份装置（初始 = 第一个预设）。
    pub fn new() -> Self {
        let initial = CameraState::from_preset(&CAMERA_PRESETS[0]);
        Self {
            index: 0,
            current: initial,
            from: initial,
            tween: 1.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_target_x: 0.0,
            mouse_target_y: 0.0,
            interactive: true,
            reduce_motion: false,
        }
    }

    /// 切换机位（从当前值起插值）。
    ///
    /// 「减动效」模式下**锁定**第一个预设（与生产侧一致）。
    /// 越界下标**显式拒绝**（返回 `false`），不静默回落。
    pub fn set_index(&mut self, index: usize) -> bool {
        if index >= CAMERA_PRESETS.len() {
            return false;
        }
        let locked = if self.reduce_motion { 0 } else { index };
        self.index = locked;
        self.from = self.current;
        self.tween = if self.reduce_motion { 1.0 } else { 0.0 };
        true
    }

    /// 开关鼠标轨道视差（关闭时视角平滑回中 ⇒ 目标归零）。
    pub fn set_interactive(&mut self, on: bool) {
        self.interactive = on;
        if !on {
            self.mouse_target_x = 0.0;
            self.mouse_target_y = 0.0;
        }
    }

    /// 设定鼠标目标位置（归一化坐标，`-1..1`）。
    pub fn set_pointer(&mut self, x: f64, y: f64) {
        if self.interactive {
            self.mouse_target_x = x;
            self.mouse_target_y = y;
        }
    }

    /// 推进一帧的机位状态，算出有效机位。
    ///
    /// - `dt`：帧时长（秒）；
    /// - `t`：漂移时钟（秒）；
    /// - `layout`：构图布局；
    /// - `css_w` / `css_h`：视口尺寸（CSS px）。
    pub fn effective(
        &mut self,
        dt: f64,
        t: f64,
        layout: &Layout,
        css_w: f64,
        css_h: f64,
    ) -> EffectiveCamera {
        if self.tween < 1.0 {
            self.tween = jsnum::min(1.0, self.tween + dt / tokens::motion::CAMERA_TWEEN_SEC);
            let e = ease_in_out_cubic(self.tween);
            let target = &CAMERA_PRESETS[self.index];
            self.current.incl = self.from.incl + (target.incl - self.from.incl) * e;
            self.current.zoom = self.from.zoom + (target.zoom - self.from.zoom) * e;
            self.current.azim = self.from.azim + (target.azim - self.from.azim) * e;
            self.current.ox = self.from.ox + (target.ox - self.from.ox) * e;
            self.current.oy = self.from.oy + (target.oy - self.from.oy) * e;
        }

        let mut drift_az = 0.0;
        let mut drift_incl = 0.0;
        let mut drift_zoom = 1.0;
        let mut par_az = 0.0;
        let mut par_incl = 0.0;
        if !self.reduce_motion {
            drift_az = tokens::drift::AZIMUTH.amplitude
                * ((t * tokens::TAU) / tokens::drift::AZIMUTH.period_sec
                    + tokens::drift::AZIMUTH.phase)
                    .sin();
            drift_incl = tokens::drift::INCLINATION.amplitude
                * ((t * tokens::TAU) / tokens::drift::INCLINATION.period_sec
                    + tokens::drift::INCLINATION.phase)
                    .sin();
            drift_zoom = 1.0
                + tokens::drift::ZOOM.amplitude
                    * ((t * tokens::TAU) / tokens::drift::ZOOM.period_sec
                        + tokens::drift::ZOOM.phase)
                        .sin();
            let k = jsnum::min(1.0, dt * tokens::motion::MOUSE_SMOOTHING_RATE);
            self.mouse_x += (self.mouse_target_x - self.mouse_x) * k;
            self.mouse_y += (self.mouse_target_y - self.mouse_y) * k;
            par_az = -self.mouse_x * tokens::parallax::AZIMUTH_RANGE;
            par_incl = -self.mouse_y * tokens::parallax::INCLINATION_RANGE;
        }

        EffectiveCamera {
            incl: jsnum::min(
                tokens::incl_clamp::MAX,
                jsnum::max(
                    tokens::incl_clamp::MIN,
                    self.current.incl + drift_incl + par_incl,
                ),
            ),
            zoom: self.current.zoom * drift_zoom,
            azim: self.current.azim + drift_az + par_az,
            disk_incl: jsnum::min(
                tokens::incl_clamp::MAX,
                jsnum::max(tokens::incl_clamp::MIN, self.current.incl + drift_incl),
            ),
            disk_azim: self.current.azim + drift_az,
            cx: layout.base_cx + self.current.ox * css_w,
            cy: layout.base_cy + self.current.oy * css_h,
        }
    }
}

/// 黑洞视区圆（CSS px）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HoleCircle {
    /// 圆心 x。
    pub x: f64,
    /// 圆心 y。
    pub y: f64,
    /// 半径。
    pub r: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_layout_is_the_accepted_ratio() {
        let layout = compute_layout(1440.0, 900.0);
        assert!((layout.base_cx - 950.4).abs() < 1e-9);
        assert!((layout.base_cy - 405.0).abs() < 1e-9);
        assert!((layout.hole_r - 144.0).abs() < 1e-9);
    }

    #[test]
    fn mobile_layout_switches_at_the_breakpoint() {
        let layout = compute_layout(800.0, 600.0);
        assert!((layout.base_cx - 400.0).abs() < 1e-9);
        assert!((layout.base_cy - 168.0).abs() < 1e-9);
        assert!((layout.hole_r - 114.0).abs() < 1e-9);
        // 断点本身（`<=`）仍走移动端比例。
        let at_breakpoint = compute_layout(820.0, 600.0);
        assert!((at_breakpoint.base_cx - 410.0).abs() < 1e-9);
        let above_breakpoint = compute_layout(821.0, 600.0);
        assert!((above_breakpoint.base_cx - 821.0 * 0.66).abs() < 1e-9);
    }

    #[test]
    fn easing_hits_the_three_waypoints() {
        assert_eq!(ease_in_out_cubic(0.0), 0.0);
        assert_eq!(ease_in_out_cubic(0.5), 0.5);
        assert_eq!(ease_in_out_cubic(1.0), 1.0);
        assert!((ease_in_out_cubic(0.25) - 0.0625).abs() < 1e-15);
    }

    #[test]
    fn rig_starts_at_the_first_preset() {
        let rig = CameraRig::new();
        assert_eq!(rig.index, 0);
        assert_eq!(rig.current, CameraState::from_preset(&CAMERA_PRESETS[0]));
        assert_eq!(rig.tween, 1.0);
    }

    #[test]
    fn out_of_range_camera_index_is_rejected() {
        let mut rig = CameraRig::new();
        assert!(!rig.set_index(3));
        assert!(!rig.set_index(99));
        assert_eq!(rig.index, 0, "被拒绝的切换不得改动当前机位");
    }

    #[test]
    fn tween_reaches_the_target_exactly() {
        let mut rig = CameraRig::new();
        assert!(rig.set_index(1));
        let layout = compute_layout(1440.0, 900.0);
        let mut last = None;
        for _ in 0..300 {
            last = Some(rig.effective(0.016, 0.0, &layout, 1440.0, 900.0));
        }
        assert_eq!(rig.tween, 1.0, "插值必须走完");
        let target = &CAMERA_PRESETS[1];
        let drift_incl = 0.02 * 1.3f64.sin();
        let frame = last.expect("frame");
        assert!(
            (frame.disk_incl - (rig.current.incl + drift_incl)).abs() < 1e-15,
            "盘面倾角 = 机位倾角 + 漂移"
        );
        assert!((rig.current.incl - target.incl).abs() < 1e-12);
        assert!((rig.current.zoom - target.zoom).abs() < 1e-12);
        assert!((rig.current.azim - target.azim).abs() < 1e-12);
        assert!((rig.current.oy - target.oy).abs() < 1e-12);
    }

    #[test]
    fn reduce_motion_locks_the_first_preset_and_drops_the_drift() {
        let mut rig = CameraRig::new();
        rig.reduce_motion = true;
        assert!(rig.set_index(2));
        assert_eq!(rig.index, 0, "减动效模式下锁定第一个预设");
        assert_eq!(rig.tween, 1.0);
        let layout = compute_layout(1440.0, 900.0);
        let cam = rig.effective(0.0, 0.0, &layout, 1440.0, 900.0);
        assert_eq!(cam.incl, CAMERA_PRESETS[0].incl, "静态模式不施加漂移");
        assert_eq!(cam.zoom, CAMERA_PRESETS[0].zoom);
        assert_eq!(cam.azim, CAMERA_PRESETS[0].azim);
        assert_eq!(cam.disk_incl, CAMERA_PRESETS[0].incl);
        assert_eq!(cam.disk_azim, CAMERA_PRESETS[0].azim);
    }

    #[test]
    fn drift_is_applied_on_the_animated_path() {
        let mut rig = CameraRig::new();
        let layout = compute_layout(1440.0, 900.0);
        let cam = rig.effective(0.016, 0.0, &layout, 1440.0, 900.0);
        let expected_incl = CAMERA_PRESETS[0].incl + 0.02 * 1.3f64.sin();
        assert!((cam.incl - expected_incl).abs() < 1e-15);
        let expected_zoom = CAMERA_PRESETS[0].zoom * (1.0 + 0.045 * 2.1f64.sin());
        assert!((cam.zoom - expected_zoom).abs() < 1e-15);
        assert_eq!(cam.azim, 0.0, "方位角漂移在 t=0 处为 0");
    }

    #[test]
    fn mouse_parallax_only_moves_the_starfield_channels() {
        let mut rig = CameraRig::new();
        rig.set_pointer(1.0, 1.0);
        let layout = compute_layout(1440.0, 900.0);
        for _ in 0..200 {
            rig.effective(0.016, 0.0, &layout, 1440.0, 900.0);
        }
        let cam = rig.effective(0.016, 0.0, &layout, 1440.0, 900.0);
        let drift_incl = 0.02 * 1.3f64.sin();
        let par_incl = -rig.mouse_y * tokens::parallax::INCLINATION_RANGE;
        assert!(par_incl < 0.0, "鼠标视差把视角压下来");
        assert!(
            (cam.incl - (rig.current.incl + drift_incl + par_incl)).abs() < 1e-15,
            "视角倾角含鼠标视差"
        );
        assert!(
            (cam.disk_incl - (rig.current.incl + drift_incl)).abs() < 1e-15,
            "盘面倾角不含鼠标视差"
        );
        assert!(
            (cam.disk_azim - rig.current.azim).abs() < 1e-15,
            "盘面方位角不含鼠标视差"
        );
    }

    #[test]
    fn turning_parallax_off_recenters_the_view() {
        let mut rig = CameraRig::new();
        rig.set_pointer(1.0, 1.0);
        rig.set_interactive(false);
        assert_eq!(rig.mouse_target_x, 0.0);
        assert_eq!(rig.mouse_target_y, 0.0);
    }
}
