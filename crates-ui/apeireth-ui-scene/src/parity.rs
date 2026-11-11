//! 逐值对照的**原生可复跑**夹具。
//!
//! 本模块的每一个取值都是**生产侧在真实浏览器里实测抓到的**原始数值
//! （f64，未四舍五入），落盘于
//! `crates-ui/apeireth-ui-demo/harness/evidence/scene/report.json`
//! （浏览器证据由 `crates-ui/apeireth-ui-demo/harness/scene/drive.py` 产出）。
//!
//! 本模块的测试断言：本 crate 算出的 16 个取值与那些抓取值**逐位相等**
//! （`assert_eq!` 逐位比较，不设容差）⇒ **不跑浏览器也能复核逐值结论**。
//!
//! ## 诚实边界
//!
//! - 浏览器证据才是「与生产侧一致」的**主证据**；本模块是它的**原生快照**，
//!   用于快速回归与代码评审；
//! - 静态路径的用例**完全确定**（与帧率无关）；动画路径的用例带**帧时长序列**
//!   （生产侧实测抓到的 rAF 时间戳还原）⇒ 状态指数趋近逐步一致。

use crate::scene::{Scene, SceneOptions, Viewport};
use crate::state::{PadBias, SceneMode};

/// 一条被记录的用例（生产侧实测抓取值）。
struct Recorded {
    /// 用例标识。
    id: &'static str,
    /// 档位。
    mode: SceneMode,
    /// 舰内时刻。
    hour: f64,
    /// 机位下标。
    camera: usize,
    /// 静态路径（`true`）/ 动画路径（`false`）。
    static_path: bool,
    /// 存在态 PAD（`None` ⇒ 没有真实存在态帧）。
    pad: Option<PadBias>,
    /// 回合级强度（`None` ⇒ 没有真实存在态帧）。
    intensity: Option<f64>,
    /// 动画路径的帧时长序列（秒）。
    dts: &'static [f64],
    /// 生产侧实测抓到的 16 个取值（名字序 = 着色器声明序）。
    uniforms: [(&'static str, &'static [f64]); 16],
}

/// 用例 `static-quiet-h0900`：static 路径 / quiet 档 / 时刻 9 / 机位 0。
const REC_STATIC_QUIET_H0900: Recorded = Recorded {
    id: "static-quiet-h0900",
    mode: SceneMode::Quiet,
    hour: 9.0,
    camera: 0,
    static_path: true,
    pad: None,
    intensity: None,
    dts: &[],
    uniforms: [
        ("uRes", &[1440.0, 900.0]),
        ("uCenter", &[950.4000000000001, 495.0]),
        ("uScale", &[86.39999999999999]),
        ("uTime", &[1.2]),
        ("uRot", &[0.35]),
        ("uBright", &[0.32]),
        ("uTurb", &[0.12]),
        ("uPulse", &[0.0]),
        ("uIncl", &[0.3]),
        ("uAzim", &[0.0]),
        ("uDiskAzim", &[0.0]),
        ("uDiskIncl", &[0.3]),
        ("uAmbStr", &[1.0]),
        ("uEclipse", &[0.0]),
        ("uTlBright", &[1.22]),
        ("uTlGold", &[0.44999999999999996]),
    ],
};

/// 用例 `static-thinking-h0625`：static 路径 / thinking 档 / 时刻 6.25 / 机位 0。
const REC_STATIC_THINKING_H0625: Recorded = Recorded {
    id: "static-thinking-h0625",
    mode: SceneMode::Thinking,
    hour: 6.25,
    camera: 0,
    static_path: true,
    pad: None,
    intensity: None,
    dts: &[],
    uniforms: [
        ("uRes", &[1440.0, 900.0]),
        ("uCenter", &[950.4000000000001, 495.0]),
        ("uScale", &[86.39999999999999]),
        ("uTime", &[1.2]),
        ("uRot", &[0.35]),
        ("uBright", &[0.9]),
        ("uTurb", &[1.0]),
        ("uPulse", &[0.0]),
        ("uIncl", &[0.3]),
        ("uAzim", &[0.0]),
        ("uDiskAzim", &[0.0]),
        ("uDiskIncl", &[0.3]),
        ("uAmbStr", &[0.5]),
        ("uEclipse", &[0.0]),
        ("uTlBright", &[1.11]),
        ("uTlGold", &[0.725]),
    ],
};

/// 用例 `static-speaking-h1200`：static 路径 / speaking 档 / 时刻 12 / 机位 0。
const REC_STATIC_SPEAKING_H1200: Recorded = Recorded {
    id: "static-speaking-h1200",
    mode: SceneMode::Speaking,
    hour: 12.0,
    camera: 0,
    static_path: true,
    pad: None,
    intensity: None,
    dts: &[],
    uniforms: [
        ("uRes", &[1440.0, 900.0]),
        ("uCenter", &[950.4000000000001, 495.0]),
        ("uScale", &[86.39999999999999]),
        ("uTime", &[1.2]),
        ("uRot", &[0.35]),
        ("uBright", &[1.05]),
        ("uTurb", &[0.35]),
        ("uPulse", &[1.0]),
        ("uIncl", &[0.3]),
        ("uAzim", &[0.0]),
        ("uDiskAzim", &[0.0]),
        ("uDiskIncl", &[0.3]),
        ("uAmbStr", &[1.0]),
        ("uEclipse", &[1.0]),
        ("uTlBright", &[1.3699999999999999]),
        ("uTlGold", &[0.44999999999999996]),
    ],
};

/// 用例 `static-pad-thinking-h0900`：static 路径 / thinking 档 / 时刻 9 / 机位 0。
const REC_STATIC_PAD_THINKING_H0900: Recorded = Recorded {
    id: "static-pad-thinking-h0900",
    mode: SceneMode::Thinking,
    hour: 9.0,
    camera: 0,
    static_path: true,
    pad: Some(PadBias {
        p: 0.6,
        a: -0.3,
        d: 0.2,
    }),
    intensity: Some(0.8),
    dts: &[],
    uniforms: [
        ("uRes", &[1440.0, 900.0]),
        ("uCenter", &[950.4000000000001, 495.0]),
        ("uScale", &[86.39999999999999]),
        ("uTime", &[1.2]),
        ("uRot", &[0.35]),
        ("uBright", &[0.8492500000000001]),
        ("uTurb", &[0.6000000000000001]),
        ("uPulse", &[0.0]),
        ("uIncl", &[0.3]),
        ("uAzim", &[0.0]),
        ("uDiskAzim", &[0.0]),
        ("uDiskIncl", &[0.3]),
        ("uAmbStr", &[1.0]),
        ("uEclipse", &[0.0]),
        ("uTlBright", &[1.25294]),
        ("uTlGold", &[0.46619999999999995]),
    ],
};

/// 用例 `anim-quiet-cam2-h0900`：animated 路径 / quiet 档 / 时刻 9 / 机位 2。
const REC_ANIM_QUIET_CAM2_H0900: Recorded = Recorded {
    id: "anim-quiet-cam2-h0900",
    mode: SceneMode::Quiet,
    hour: 9.0,
    camera: 2,
    static_path: false,
    pad: None,
    intensity: None,
    dts: &[
        -0.006200000001490116,
        0.0166,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.033299999999999955,
        0.05,
        0.033299999999999955,
        0.033300000000000066,
        0.05,
        0.03339999999999998,
        0.04989999999999998,
        0.05,
        0.049999999999999885,
        0.05,
        0.016700000000000045,
        0.033299999999999955,
        0.05,
        0.016600000000000135,
        0.05,
        0.05,
        0.033300000000000184,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.04990000000000009,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.01659999999999991,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
        0.05,
    ],
    uniforms: [
        ("uRes", &[1440.0, 900.0]),
        ("uCenter", &[950.4000000000001, 468.0]),
        ("uScale", &[137.62610896021394]),
        ("uTime", &[1.5]),
        ("uRot", &[0.0]),
        ("uBright", &[0.32]),
        ("uTurb", &[0.12]),
        ("uPulse", &[0.0]),
        ("uIncl", &[0.8192711637083439]),
        ("uAzim", &[0.4]),
        ("uDiskAzim", &[0.4]),
        ("uDiskIncl", &[0.8192711637083439]),
        ("uAmbStr", &[1.0]),
        ("uEclipse", &[0.0]),
        ("uTlBright", &[1.22]),
        ("uTlGold", &[0.44999999999999996]),
    ],
};

/// 被记录的用例清单。
const RECORDED: [&Recorded; 5] = [
    &REC_STATIC_QUIET_H0900,
    &REC_STATIC_THINKING_H0625,
    &REC_STATIC_SPEAKING_H1200,
    &REC_STATIC_PAD_THINKING_H0900,
    &REC_ANIM_QUIET_CAM2_H0900,
];

/// 视口（画布 1440×900 设备像素，dpr = 1 —— 与浏览器对照同一口径）。
const VIEWPORT: Viewport = Viewport {
    css_width: 1440.0,
    css_height: 900.0,
    device_pixel_ratio: 1.0,
};

/// 按夹具重放本 crate 的取值链。
fn computed(recorded: &Recorded) -> Vec<(&'static str, Vec<f64>)> {
    let mut scene = Scene::new(
        VIEWPORT,
        SceneOptions {
            interactive: false,
            reduce_motion: recorded.static_path,
            frozen_time: if recorded.static_path {
                None
            } else {
                Some(1.5)
            },
        },
    );
    scene.set_mode(recorded.mode);
    scene.set_hour(Some(recorded.hour));
    if let Some(pad) = recorded.pad {
        scene.set_pad_bias(pad, recorded.intensity);
    }
    assert!(scene.set_camera(recorded.camera), "机位下标必须被接受");

    let uniforms = if recorded.static_path {
        scene.render_once().expect("静态帧")
    } else {
        let mut last = None;
        for dt in recorded.dts {
            last = scene.advance(*dt);
        }
        last.expect("动画帧")
    };
    uniforms.entries()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_cases_are_reproduced_bit_for_bit() {
        for recorded in RECORDED {
            let computed = computed(recorded);
            assert_eq!(computed.len(), 16, "{}: 取值个数不对", recorded.id);
            for (index, (name, components)) in computed.iter().enumerate() {
                let (expected_name, expected) = recorded.uniforms[index];
                assert_eq!(*name, expected_name, "{}: 取值顺序不对", recorded.id);
                assert_eq!(
                    components.as_slice(),
                    expected,
                    "{}: 取值 `{}` 与生产侧实测值不逐位相等",
                    recorded.id,
                    name
                );
            }
        }
    }

    #[test]
    fn every_recorded_case_has_sixteen_values() {
        for recorded in RECORDED {
            assert_eq!(recorded.uniforms.len(), 16, "{}: 夹具不完整", recorded.id);
            assert_eq!(
                recorded.uniforms[0].0, "uRes",
                "{}: 第一个取值必须是 uRes",
                recorded.id
            );
            assert_eq!(
                recorded.uniforms[15].0, "uTlGold",
                "{}: 最后一个取值必须是 uTlGold",
                recorded.id
            );
        }
    }

    #[test]
    fn animated_cases_carry_the_frame_delta_sequence() {
        for recorded in RECORDED {
            if recorded.static_path {
                assert!(
                    recorded.dts.is_empty(),
                    "{}: 静态路径没有帧时长",
                    recorded.id
                );
            } else {
                assert!(
                    !recorded.dts.is_empty(),
                    "{}: 动画路径必须带帧时长",
                    recorded.id
                );
                // ⚠️ 帧时长只有**上限**（50ms）没有下限 ⇒ 首帧可以是负数：
                // 浏览器的 rAF 时间戳是该帧的**起始时刻**，可能早于
                // `performance.now()`（startLoop 里取的起始时钟）⇒ 首帧 dt 为负。
                // 生产侧就是这个值 ⇒ 重放必须逐位复现（不得夹到 0）。
                assert!(
                    recorded.dts.iter().all(|dt| *dt <= 0.05),
                    "{}: 帧时长不得超过 50ms 上限",
                    recorded.id
                );
            }
        }
    }
}
