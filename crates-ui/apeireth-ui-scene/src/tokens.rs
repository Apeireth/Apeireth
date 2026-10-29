//! 场景设计令牌（数值常量）。
//!
//! 这一组常量是**取值链的原料**：机位预设、构图比例、漂移与视差幅度、
//! 动效时长、性能钳位、presence 三档基线。
//!
//! ## 来源纪律
//!
//! - 数值与生产侧的设计令牌**逐字一致**（含小数位数）—— 任何一处改动都会
//!   让「状态 ⇒ uniform」的逐值对照漂移；
//! - 令牌里的**展示文案**（机位名、状态名）**不参与任何取值** ⇒ 本模块
//!   **不携带**它们（不属于取值链，且引擎树不出现展示词汇）。

/// 圆周率的两倍（与 `Math.PI * 2` 同值）。
pub const TAU: f64 = std::f64::consts::PI * 2.0;

/// 动效令牌。
pub mod motion {
    /// 机位过渡时长（秒）—— 缓动为 easeInOutCubic。
    pub const CAMERA_TWEEN_SEC: f64 = 2.6;
    /// presence 状态参数的指数趋近速率（/s）。
    pub const STATE_SMOOTHING_RATE: f64 = 2.4;
    /// 鼠标视差平滑速率（/s）。
    pub const MOUSE_SMOOTHING_RATE: f64 = 2.5;
}

/// 性能钳位令牌。
pub mod performance {
    /// 设备像素比上限。
    pub const DPR_MAX: f64 = 2.0;
    /// 单帧时长上限（秒）—— 帧时长钳位。
    pub const DT_CLAMP_SEC: f64 = 0.05;
}

/// presence 三档基线。
pub mod presence {
    /// 静息档：转速 / 亮度 / 湍流 / 呼吸。
    pub const QUIET: [f64; 4] = [0.015, 0.32, 0.12, 0.0];
    /// 思考档：转速 / 亮度 / 湍流 / 呼吸。
    pub const THINKING: [f64; 4] = [0.38, 0.9, 1.0, 0.0];
    /// 说话档：转速 / 亮度 / 湍流 / 呼吸。
    pub const SPEAKING: [f64; 4] = [0.16, 1.05, 0.35, 1.0];
    /// presence 接线增益（PAD / 强度进入映射表前的保守缩放）。
    pub const HEURISTIC_V0_GAIN: f64 = 0.75;
}

/// 机位倾角的活动范围。
pub mod incl_clamp {
    /// 下限。
    pub const MIN: f64 = 0.06;
    /// 上限。
    pub const MAX: f64 = 1.2;
}

/// 鼠标轨道视差幅度。
pub mod parallax {
    /// 方位角幅度。
    pub const AZIMUTH_RANGE: f64 = 0.055;
    /// 倾角幅度。
    pub const INCLINATION_RANGE: f64 = 0.022;
}

/// 构图布局比例。
pub mod layout {
    /// 移动端断点（CSS px）。
    pub const BREAKPOINT_PX: f64 = 820.0;
    /// 桌面端构图中心比例（屏宽）。
    pub const DESKTOP_CX_RATIO: f64 = 0.66;
    /// 桌面端构图中心比例（屏高）。
    pub const DESKTOP_CY_RATIO: f64 = 0.45;
    /// 桌面端黑洞视区半径比例（取短边）。
    pub const DESKTOP_HOLE_RADIUS_RATIO: f64 = 0.16;
    /// 移动端构图中心比例（屏宽）。
    pub const MOBILE_CX_RATIO: f64 = 0.5;
    /// 移动端构图中心比例（屏高）。
    pub const MOBILE_CY_RATIO: f64 = 0.28;
    /// 移动端黑洞视区半径比例（取短边）。
    pub const MOBILE_HOLE_RADIUS_RATIO: f64 = 0.19;
}

/// 一个机位预设。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPreset {
    /// 预设序号。
    pub id: u32,
    /// 盘面倾角压扁系数（小 = 平视）。
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

/// 三个机位预设（远眺 / 临渊 / 俯瞰）。
///
/// ⚠️ 展示文案**不**在本表内（不参与取值链）。
pub const CAMERA_PRESETS: [CameraPreset; 3] = [
    CameraPreset {
        id: 0,
        incl: 0.3,
        zoom: 0.6,
        azim: 0.0,
        ox: 0.0,
        oy: 0.0,
    },
    CameraPreset {
        id: 1,
        incl: 0.115,
        zoom: 1.9,
        azim: -0.1,
        ox: 0.0,
        oy: -0.05,
    },
    CameraPreset {
        id: 2,
        incl: 0.8,
        zoom: 0.92,
        azim: 0.4,
        ox: 0.0,
        oy: 0.03,
    },
];

/// 一组正弦漂移的参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftSpec {
    /// 幅度。
    pub amplitude: f64,
    /// 周期（秒）。
    pub period_sec: f64,
    /// 相位。
    pub phase: f64,
}

/// 三组正弦漂移（周期互质）。
pub mod drift {
    use super::DriftSpec;

    /// 方位角漂移。
    pub const AZIMUTH: DriftSpec = DriftSpec {
        amplitude: 0.05,
        period_sec: 47.0,
        phase: 0.0,
    };
    /// 倾角漂移。
    pub const INCLINATION: DriftSpec = DriftSpec {
        amplitude: 0.02,
        period_sec: 37.0,
        phase: 1.3,
    };
    /// 视距漂移（乘性）。
    pub const ZOOM: DriftSpec = DriftSpec {
        amplitude: 0.045,
        period_sec: 53.0,
        phase: 2.1,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_camera_presets_are_ordered_by_id() {
        assert_eq!(CAMERA_PRESETS.len(), 3);
        for (index, preset) in CAMERA_PRESETS.iter().enumerate() {
            assert_eq!(preset.id as usize, index, "预设序号必须连续");
        }
    }

    #[test]
    fn camera_preset_values_are_the_accepted_ones() {
        assert_eq!(CAMERA_PRESETS[0].incl, 0.3);
        assert_eq!(CAMERA_PRESETS[0].zoom, 0.6);
        assert_eq!(CAMERA_PRESETS[1].incl, 0.115);
        assert_eq!(CAMERA_PRESETS[1].zoom, 1.9);
        assert_eq!(CAMERA_PRESETS[1].oy, -0.05);
        assert_eq!(CAMERA_PRESETS[2].incl, 0.8);
        assert_eq!(CAMERA_PRESETS[2].zoom, 0.92);
        assert_eq!(CAMERA_PRESETS[2].azim, 0.4);
        assert_eq!(CAMERA_PRESETS[2].oy, 0.03);
    }

    #[test]
    fn drift_specs_keep_their_accepted_numbers() {
        assert_eq!(drift::AZIMUTH.amplitude, 0.05);
        assert_eq!(drift::AZIMUTH.period_sec, 47.0);
        assert_eq!(drift::INCLINATION.amplitude, 0.02);
        assert_eq!(drift::INCLINATION.period_sec, 37.0);
        assert_eq!(drift::INCLINATION.phase, 1.3);
        assert_eq!(drift::ZOOM.amplitude, 0.045);
        assert_eq!(drift::ZOOM.period_sec, 53.0);
        assert_eq!(drift::ZOOM.phase, 2.1);
    }

    #[test]
    fn motion_and_performance_clamps_are_the_accepted_ones() {
        assert_eq!(motion::CAMERA_TWEEN_SEC, 2.6);
        assert_eq!(motion::STATE_SMOOTHING_RATE, 2.4);
        assert_eq!(motion::MOUSE_SMOOTHING_RATE, 2.5);
        assert_eq!(performance::DPR_MAX, 2.0);
        assert_eq!(performance::DT_CLAMP_SEC, 0.05);
        assert_eq!(incl_clamp::MIN, 0.06);
        assert_eq!(incl_clamp::MAX, 1.2);
    }

    #[test]
    fn presence_baselines_are_the_accepted_ones() {
        assert_eq!(presence::QUIET, [0.015, 0.32, 0.12, 0.0]);
        assert_eq!(presence::THINKING, [0.38, 0.9, 1.0, 0.0]);
        assert_eq!(presence::SPEAKING, [0.16, 1.05, 0.35, 1.0]);
        assert_eq!(presence::HEURISTIC_V0_GAIN, 0.75);
    }

    #[test]
    fn layout_ratios_are_the_accepted_ones() {
        assert_eq!(layout::BREAKPOINT_PX, 820.0);
        assert_eq!(layout::DESKTOP_CX_RATIO, 0.66);
        assert_eq!(layout::DESKTOP_CY_RATIO, 0.45);
        assert_eq!(layout::DESKTOP_HOLE_RADIUS_RATIO, 0.16);
        assert_eq!(layout::MOBILE_CX_RATIO, 0.5);
        assert_eq!(layout::MOBILE_CY_RATIO, 0.28);
        assert_eq!(layout::MOBILE_HOLE_RADIUS_RATIO, 0.19);
    }

    #[test]
    fn tau_is_two_pi() {
        assert_eq!(TAU, std::f64::consts::PI * 2.0);
    }
}
