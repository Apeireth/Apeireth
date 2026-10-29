//! 片头「火之文明史」的**每拍编舞**（生产 `intro/fire.ts` 的 Rust 对齐实现）。
//!
//! ## 对齐口径
//!
//! 本模块是生产 `frontend/companion-desktop/src/lib/intro/fire.ts` 的
//! **逐值对齐**移植：`FIRE_TABLE`（每拍一套参数）、`dynamics`（弹道 p/v/a 与
//! 动态乘子）、`bgFor`（背景特征参数）、`mixBg`（拍间 crossfade 的数值 lerp）
//! —— **一字不改**地搬过来 ⇒ ⇒ 同一时刻算出的 uniform 取值与生产侧**逐位
//! 相同**。
//!
//! ## 职责边界
//!
//! - 本模块只算**取值**（宿主侧场景语义）；
//! - 着色器**不在**本模块（GLSL 资产在 `apeireth-ui-gpu`）；
//! - 绘制**不在**本模块（由 `apeireth-ui-gpu` 承担）。
//!
//! ## 数值口径
//!
//! 生产侧跑在 JS 运行时里 ⇒ 数值是 **f64**；本模块也用 **f64** ⇒ 写进
//! uniform 缓冲时统一收口成 f32 ⇒ ⇒ 与生产侧收口后的结果**逐位相同**。

use super::timeline::{BeatWeight, IntroBeatId};

/// 二维向量（f64）。
pub type Vec2 = [f64; 2];

/// 三维向量（f64）。
pub type Vec3 = [f64; 3];

// ---------------------------------------------------------------------------
// 设计令牌（生产 `scene/tokens.ts` 的取值，逐字照搬）
// ---------------------------------------------------------------------------

/// 白热（生产 `COLOR_PRESENCE.whiteHotRgb`）。
pub const HOT: Vec3 = [1.0, 0.95, 0.82];
/// 金（生产 `COLOR_PRESENCE.goldRgb`）。
pub const GOLD: Vec3 = [1.0, 0.824, 0.478];
/// 琥珀（生产 `COLOR_PRESENCE.amberRgb`）。
pub const AMBER: Vec3 = [0.91, 0.639, 0.239];
/// 火星尾色（生产 `fire.ts` 的 `EMBER_TAIL`）。
pub const EMBER_TAIL: Vec3 = [0.45, 0.16, 0.04];
/// 熔融中段色（生产 `fire.ts` 的 `MOLTEN_MID`）。
pub const MOLTEN_MID: Vec3 = [1.0, 0.86, 0.55];
/// 熔融尾色（生产 `fire.ts` 的 `MOLTEN_TAIL`）。
pub const MOLTEN_TAIL: Vec3 = [0.70, 0.35, 0.10];

// ---------------------------------------------------------------------------
// 参数结构（生产 `fire.ts` 的 `BgParams` / `FireParams`）
// ---------------------------------------------------------------------------

/// 背景 matte 特征参数（全部可数值 lerp；强度类 0 = 关闭）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BgParams {
    /// 星野强度。
    pub stars: f64,
    /// 星点水平拉伸（速度线）。
    pub star_stretch: f64,
    /// 银河带（右侧）。
    pub milky: f64,
    /// 月球暗盘。
    pub moon: f64,
    /// 月球位置。
    pub moon_pos: Vec2,
    /// 月球半径。
    pub moon_r: f64,
    /// 炉膛铁环。
    pub furnace: f64,
    /// 炉膛铁环半径。
    pub furnace_r: f64,
    /// 发动机气缸剖面。
    pub engine: f64,
    /// 激波锥弧。
    pub shock: f64,
    /// 激波锥顶点。
    pub shock_apex: Vec2,
    /// 弹体 / 火箭 capsule。
    pub body: f64,
    /// 弹体位置。
    pub body_pos: Vec2,
    /// 弹体长度。
    pub body_len: f64,
    /// 熔融金流占比。
    pub body_molten: f64,
    /// 弹体形变（0 子弹 → 1 火箭）。
    pub body_morph: f64,
    /// 落幅行星 / 星环暗示。
    pub planet_hint: f64,
    /// 营火地面光。
    pub ground_glow: f64,
    /// 营火地面光位置。
    pub ground_pos: Vec2,
    /// 火心体积光。
    pub core_glow: f64,
    /// 火心位置。
    pub core_pos: Vec2,
    /// 闪光。
    pub flash: f64,
    /// 闪光位置。
    pub flash_pos: Vec2,
    /// 暗角。
    pub vig: f64,
}

/// 火焰粒子 uniform 编舞（每拍一套）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireParams {
    /// 静态拍的发射原点（动态拍由弹道覆盖）。
    pub origin: Vec2,
    /// 主喷射方向。
    pub dir: Vec2,
    /// 方向散布（弧度，±）。
    pub spread: f64,
    /// 发射口半径抖动。
    pub spawn_r: f64,
    /// 初速。
    pub speed: f64,
    /// 初速抖动幅度。
    pub speed_var: f64,
    /// 继承弹道速度比例。
    pub inherit: f64,
    /// 寿命。
    pub life: f64,
    /// 寿命抖动幅度。
    pub life_var: f64,
    /// 出生尺寸（CSS px）。
    pub size0: f64,
    /// 消亡尺寸。
    pub size1: f64,
    /// 尺寸抖动幅度。
    pub size_var: f64,
    /// 重力。
    pub grav: Vec2,
    /// 阻力。
    pub drag: f64,
    /// 湍流位移幅度。
    pub turb: f64,
    /// 湍流频率。
    pub turb_freq: f64,
    /// 湍流时间缩放。
    pub turb_time: f64,
    /// 活跃粒子比例。
    pub count01: f64,
    /// 是否一次性迸发（1 = 是）。
    pub burst: f64,
    /// 迸发时间跨度。
    pub burst_span: f64,
    /// 放射环粒子占比。
    pub radial01: f64,
    /// 放射环初速。
    pub radial_speed: f64,
    /// 放射环阻力。
    pub radial_drag: f64,
    /// 放射环寿命。
    pub radial_life: f64,
    /// 大软光点占比。
    pub soft01: f64,
    /// 大软光点尺寸。
    pub soft_size: f64,
    /// 大软光点增益。
    pub soft_gain: f64,
    /// 速度向拉伸。
    pub streak: f64,
    /// 闪烁幅度。
    pub twinkle: f64,
    /// 整体亮度增益。
    pub gain: f64,
    /// 火星子类占比。
    pub ember01: f64,
    /// 热色。
    pub col_hot: Vec3,
    /// 中段色。
    pub col_mid: Vec3,
    /// 尾色。
    pub col_tail: Vec3,
}

/// 动态弹道（moving origin 拍）的 p / v / a。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginTrack {
    /// 位置。
    pub p: Vec2,
    /// 速度。
    pub v: Vec2,
    /// 加速度。
    pub a: Vec2,
}

// ---------------------------------------------------------------------------
// 每拍编舞表（生产 `FIRE_TABLE`）
// ---------------------------------------------------------------------------

/// 每拍一套的粒子编舞（生产 `fire.ts` 的 `FIRE_TABLE`）—— **按拍序**。
///
/// ⚠ 逐值照搬生产侧；顺序必须与
/// [`super::timeline::INTRO_BEAT_IDS`] 一致 ⇒ 由单测钉住。
pub const FIRE_TABLE: [FireParams; 10] = [
    // 1 打火石：纯黑中一簇金色火花迸溅（放射 + 微重力下坠 + 强 streak）。
    FireParams {
        origin: [0.0, -0.02],
        dir: [0.0, 1.0],
        spread: 2.6,
        spawn_r: 0.012,
        speed: 1.7,
        speed_var: 0.75,
        inherit: 0.0,
        life: 0.65,
        life_var: 0.5,
        size0: 20.0,
        size1: 2.0,
        size_var: 0.4,
        grav: [0.0, -1.6],
        drag: 1.2,
        turb: 0.10,
        turb_freq: 2.0,
        turb_time: 1.0,
        count01: 0.8,
        burst: 1.0,
        burst_span: 0.32,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.0,
        soft_size: 0.0,
        soft_gain: 0.0,
        streak: 1.3,
        twinkle: 0.6,
        ember01: 0.15,
        gain: 1.35,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
    // 2 营火：中心偏下，火焰升腾 + 大软光点体积感 + 火星子类。
    FireParams {
        origin: [0.0, -0.30],
        dir: [0.0, 1.0],
        spread: 0.30,
        spawn_r: 0.045,
        speed: 0.62,
        speed_var: 0.5,
        inherit: 0.0,
        life: 1.15,
        life_var: 0.45,
        size0: 8.0,
        size1: 1.6,
        size_var: 0.5,
        grav: [0.0, 0.35],
        drag: 0.6,
        turb: 0.24,
        turb_freq: 2.6,
        turb_time: 1.0,
        count01: 0.8,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.22,
        soft_size: 100.0,
        soft_gain: 0.045,
        streak: 0.15,
        twinkle: 0.35,
        ember01: 0.12,
        gain: 1.0,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: EMBER_TAIL,
    },
    // 3 蒸汽机：炉膛圆口内翻滚（宽散布 + 大软点 + 铁环 matte）。
    FireParams {
        origin: [0.0, -0.03],
        dir: [0.0, 1.0],
        spread: 0.9,
        spawn_r: 0.10,
        speed: 0.42,
        speed_var: 0.6,
        inherit: 0.0,
        life: 0.95,
        life_var: 0.5,
        size0: 14.0,
        size1: 3.0,
        size_var: 0.5,
        grav: [0.0, 0.12],
        drag: 0.9,
        turb: 0.42,
        turb_freq: 3.0,
        turb_time: 1.2,
        count01: 0.85,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.30,
        soft_size: 130.0,
        soft_gain: 0.05,
        streak: 0.10,
        twinkle: 0.25,
        ember01: 0.05,
        gain: 0.88,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: EMBER_TAIL,
    },
    // 4 内燃机：气缸内燃烧火球（紧致 + 高湍流 + JS 侧脉动 gain）。
    FireParams {
        origin: [0.0, 0.0],
        dir: [0.0, 1.0],
        spread: 2.8,
        spawn_r: 0.07,
        speed: 0.34,
        speed_var: 0.6,
        inherit: 0.0,
        life: 0.70,
        life_var: 0.4,
        size0: 14.0,
        size1: 4.0,
        size_var: 0.5,
        grav: [0.0, 0.0],
        drag: 1.3,
        turb: 0.60,
        turb_freq: 4.0,
        turb_time: 1.6,
        count01: 0.75,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.40,
        soft_size: 120.0,
        soft_gain: 0.07,
        streak: 0.05,
        twinkle: 0.15,
        ember01: 0.0,
        gain: 0.9,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
    // 5 底火出膛：放射环（radial 子类）+ 弹身环火（随加速弹体后掠）。
    FireParams {
        origin: [0.0, 0.0],
        dir: [-1.0, 0.10],
        spread: 0.5,
        spawn_r: 0.03,
        speed: 0.8,
        speed_var: 0.4,
        inherit: 0.85,
        life: 0.5,
        life_var: 0.4,
        size0: 7.0,
        size1: 1.2,
        size_var: 0.4,
        grav: [0.0, 0.0],
        drag: 0.8,
        turb: 0.25,
        turb_freq: 3.0,
        turb_time: 1.5,
        count01: 0.9,
        burst: 1.0,
        burst_span: 1.05,
        radial01: 0.45,
        radial_speed: 2.6,
        radial_drag: 2.0,
        radial_life: 1.1,
        soft01: 0.10,
        soft_size: 90.0,
        soft_gain: 0.06,
        streak: 0.7,
        twinkle: 0.3,
        ember01: 0.10,
        gain: 1.1,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
    // 6 子弹飞行：火焰尾迹 + 激波锥（相机跟拍，弹体居中）。
    FireParams {
        origin: [0.26, 0.0],
        dir: [-1.0, 0.0],
        spread: 0.14,
        spawn_r: 0.02,
        speed: 1.05,
        speed_var: 0.35,
        inherit: 0.1,
        life: 0.85,
        life_var: 0.4,
        size0: 8.0,
        size1: 1.4,
        size_var: 0.4,
        grav: [0.0, 0.02],
        drag: 0.4,
        turb: 0.45,
        turb_freq: 3.2,
        turb_time: 2.0,
        count01: 0.9,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.18,
        soft_size: 90.0,
        soft_gain: 0.05,
        streak: 0.85,
        twinkle: 0.2,
        ember01: 0.2,
        gain: 1.05,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: EMBER_TAIL,
    },
    // 7 变形：熔成金色流线（更高湍流 + 熔融色 + 弹体 morph）。
    FireParams {
        origin: [0.28, 0.0],
        dir: [-1.0, 0.0],
        spread: 0.16,
        spawn_r: 0.03,
        speed: 1.0,
        speed_var: 0.35,
        inherit: 0.1,
        life: 0.8,
        life_var: 0.4,
        size0: 10.0,
        size1: 1.6,
        size_var: 0.4,
        grav: [0.0, 0.0],
        drag: 0.4,
        turb: 0.75,
        turb_freq: 3.6,
        turb_time: 2.4,
        count01: 0.9,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.22,
        soft_size: 100.0,
        soft_gain: 0.06,
        streak: 0.8,
        twinkle: 0.15,
        ember01: 0.15,
        gain: 1.1,
        col_hot: HOT,
        col_mid: MOLTEN_MID,
        col_tail: MOLTEN_TAIL,
    },
    // 8 划月：火箭跨屏金线（低自速 + 零继承 → 尾迹留成线）。
    FireParams {
        origin: [0.0, 0.0],
        dir: [-1.0, -0.18],
        spread: 0.10,
        spawn_r: 0.008,
        speed: 0.3,
        speed_var: 0.4,
        inherit: 0.05,
        life: 0.55,
        life_var: 0.3,
        size0: 6.0,
        size1: 1.0,
        size_var: 0.4,
        grav: [0.0, 0.0],
        drag: 0.1,
        turb: 0.18,
        turb_freq: 3.0,
        turb_time: 1.5,
        count01: 0.7,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.10,
        soft_size: 60.0,
        soft_gain: 0.05,
        streak: 0.95,
        twinkle: 0.1,
        ember01: 0.1,
        gain: 1.15,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
    // 9 抵达：金线弧线减速划向行星/星环方向（长尾 + 随 tau 衰减）。
    FireParams {
        origin: [0.0, 0.0],
        dir: [-1.0, -0.3],
        spread: 0.08,
        spawn_r: 0.006,
        speed: 0.2,
        speed_var: 0.3,
        inherit: 0.05,
        life: 1.1,
        life_var: 0.3,
        size0: 5.0,
        size1: 0.8,
        size_var: 0.4,
        grav: [0.0, 0.0],
        drag: 0.1,
        turb: 0.12,
        turb_freq: 3.0,
        turb_time: 1.2,
        count01: 0.5,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.08,
        soft_size: 50.0,
        soft_gain: 0.04,
        streak: 0.95,
        twinkle: 0.05,
        ember01: 0.08,
        gain: 1.05,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
    // 10 落幅：粒子归零，只剩背景暗示与 DOM 淡出。
    FireParams {
        origin: [0.0, 0.0],
        dir: [0.0, 1.0],
        spread: 0.0,
        spawn_r: 0.0,
        speed: 0.0,
        speed_var: 0.0,
        inherit: 0.0,
        life: 1.0,
        life_var: 0.0,
        size0: 0.0,
        size1: 0.0,
        size_var: 0.0,
        grav: [0.0, 0.0],
        drag: 0.0,
        turb: 0.0,
        turb_freq: 1.0,
        turb_time: 1.0,
        count01: 0.0,
        burst: 0.0,
        burst_span: 0.0,
        radial01: 0.0,
        radial_speed: 0.0,
        radial_drag: 0.0,
        radial_life: 0.5,
        soft01: 0.0,
        soft_size: 0.0,
        soft_gain: 0.0,
        streak: 0.0,
        twinkle: 0.0,
        ember01: 0.0,
        gain: 0.0,
        col_hot: HOT,
        col_mid: GOLD,
        col_tail: AMBER,
    },
];

// ---------------------------------------------------------------------------
// 工具函数（生产 `fire.ts` 的同名函数，逐字照搬）
// ---------------------------------------------------------------------------

/// 与生产 `clamp01` 一致。
pub fn clamp01(v: f64) -> f64 {
    // ⚠️ 用 `clamp`（不是 `min().max()`）：后者在 **NaN** 输入下会给出 `0.0`
    // ⇒ ⇒ 与生产的 `clamp01`（`Math.min(1, Math.max(0, v))`）**逐字同形**
    // ⇒ ⇒ 顺序保持「先上界后下界」⇒ ⇒ NaN 的处置也一致。
    v.clamp(0.0, 1.0)
}

/// 与生产 `sstep01(a, b, x)` 一致（注意：与 `timeline::sstep` **同名不同义**
/// —— 生产侧这两个函数**都存在**，这里保持各自的口径）。
pub fn sstep01(a: f64, b: f64, x: f64) -> f64 {
    let t = clamp01((x - a) / (b - a));
    t * t * (3.0 - 2.0 * t)
}

/// 与生产 `easeInOutCubic` 一致。
pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (2.0 - 2.0 * t).powi(3) / 2.0
    }
}

/// 与生产 `easeOutCubic` 一致。
pub fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

/// 与生产 `lerp` 一致。
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// 与生产 `lerpVec2` 一致。
pub fn lerp_vec2(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t)]
}

/// 数值微分弹道采样（生产 `trackAt`，`h = 8ms`）。
///
/// ⚠ 逐字照搬：中心差分（`v`）与二阶差分（`a`）⇒ 与生产侧**逐位相同**
///   （同样的 f64 操作、同样的顺序）。
pub fn track_at(path: &dyn Fn(f64) -> Vec2, t: f64) -> OriginTrack {
    let h = 0.008f64;
    let p = path(t);
    let pf = path(t + h);
    // ⚠ 生产写的是 `fn(Math.max(0, t - h))` ⇒ 逐字照搬（**不是** `t.max(0) - h`）。
    let pb = path((t - h).max(0.0));
    OriginTrack {
        p,
        v: [(pf[0] - pb[0]) / (2.0 * h), (pf[1] - pb[1]) / (2.0 * h)],
        a: [
            (pf[0] - 2.0 * p[0] + pb[0]) / (h * h),
            (pf[1] - 2.0 * p[1] + pb[1]) / (h * h),
        ],
    }
}

/// 静态弹道（生产 `staticTrack`）。
pub fn static_track(p: Vec2) -> OriginTrack {
    OriginTrack {
        p,
        v: [0.0, 0.0],
        a: [0.0, 0.0],
    }
}

// ---------------------------------------------------------------------------
// 每拍动态（生产 `dynamics`）
// ---------------------------------------------------------------------------

/// 一拍的动态编舞结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatDynamics {
    /// 弹道（p / v / a）。
    pub track: OriginTrack,
    /// 亮度乘子。
    pub gain_mul: f64,
    /// 粒子数乘子。
    pub count_mul: f64,
}

/// 拍 id + 本地时间 → 弹道 p/v/a 与动态乘子（生产 `dynamics`）。
///
/// ⚠ `aspect` 是**视口宽高比**（生产 `this.aspect` = CSS 宽 / CSS 高）。
pub fn dynamics(beat: &BeatWeight, aspect: f64) -> BeatDynamics {
    let lt = beat.local_t;
    let tau = beat.tau;
    match beat.id {
        IntroBeatId::Spark => BeatDynamics {
            track: static_track(FIRE_TABLE[0].origin),
            gain_mul: 1.0,
            count_mul: 1.0,
        },
        IntroBeatId::Campfire => BeatDynamics {
            track: static_track(FIRE_TABLE[1].origin),
            gain_mul: 1.0,
            count_mul: 1.0,
        },
        IntroBeatId::Furnace => BeatDynamics {
            track: static_track(FIRE_TABLE[2].origin),
            gain_mul: 1.0,
            count_mul: 1.0,
        },
        IntroBeatId::Engine => BeatDynamics {
            // 点火脉动（概念图 beat-04：火光脉动）。
            track: static_track(FIRE_TABLE[3].origin),
            gain_mul: 1.0 + 0.30 * (lt * 13.0).sin(),
            count_mul: 1.0,
        },
        IntroBeatId::Primer => {
            // 子弹加速旋出：x = 0.34·s²（s 截断在拍内）。
            let dur = 1.2f64;
            let fn_track = |t: f64| -> Vec2 {
                let s = (t.max(0.0).min(dur)) / dur;
                [0.34 * s * s, 0.0]
            };
            BeatDynamics {
                track: track_at(&fn_track, lt),
                gain_mul: 1.0,
                count_mul: 1.0,
            }
        }
        IntroBeatId::Flight => {
            let fn_track =
                |t: f64| -> Vec2 { [0.26 + 0.008 * (t * 2.6).sin(), 0.010 * (t * 2.1).sin()] };
            BeatDynamics {
                track: track_at(&fn_track, lt),
                gain_mul: 1.0,
                count_mul: 1.0,
            }
        }
        IntroBeatId::Morph => {
            let fn_track =
                |t: f64| -> Vec2 { [0.28 + 0.008 * (t * 2.4).sin(), 0.010 * (t * 1.9).sin()] };
            BeatDynamics {
                track: track_at(&fn_track, lt),
                gain_mul: 1.0,
                count_mul: 1.0,
            }
        }
        IntroBeatId::Moon => {
            // 火箭跨屏：左下 → 右上掠月，轻微上弧。
            let dur = 2.0f64;
            let p0: Vec2 = [-0.95 * aspect, -0.55];
            let p1: Vec2 = [0.85 * aspect, 0.38];
            let fn_track = |t: f64| -> Vec2 {
                let s = clamp01(t / dur);
                [
                    lerp(p0[0], p1[0], s),
                    lerp(p0[1], p1[1], s) + 0.22 * (std::f64::consts::PI * s).sin(),
                ]
            };
            BeatDynamics {
                track: track_at(&fn_track, lt),
                gain_mul: 1.0,
                count_mul: 1.0,
            }
        }
        IntroBeatId::Arrival => {
            // 金线弧线减速（ease-out = 抵达感），弯向行星/星环方向。
            let dur = 2.5f64;
            let p0: Vec2 = [0.80 * aspect, 0.45];
            let c1: Vec2 = [0.15 * aspect, 0.60];
            let c2: Vec2 = [-0.35 * aspect, 0.25];
            let p1: Vec2 = [-0.62 * aspect, -0.30];
            let fn_track = |t: f64| -> Vec2 {
                let s = ease_out_cubic(clamp01(t / dur));
                let u = 1.0 - s;
                [
                    u * u * u * p0[0]
                        + 3.0 * u * u * s * c1[0]
                        + 3.0 * u * s * s * c2[0]
                        + s * s * s * p1[0],
                    u * u * u * p0[1]
                        + 3.0 * u * u * s * c1[1]
                        + 3.0 * u * s * s * c2[1]
                        + s * s * s * p1[1],
                ]
            };
            BeatDynamics {
                track: track_at(&fn_track, lt),
                gain_mul: 1.0 - 0.55 * sstep01(0.55, 1.0, tau),
                count_mul: 1.0 - 0.6 * tau,
            }
        }
        IntroBeatId::Landing => BeatDynamics {
            track: static_track([0.0, 0.0]),
            gain_mul: 0.0,
            count_mul: 0.0,
        },
    }
}

// ---------------------------------------------------------------------------
// 背景特征参数（生产 `bgFor` + `mixBg`）
// ---------------------------------------------------------------------------

/// 拍 id + 弹道 → 背景 matte 参数（生产 `bgFor`）。
pub fn bg_for(beat: &BeatWeight, track: &OriginTrack, aspect: f64) -> BgParams {
    let lt = beat.local_t;
    let tau = beat.tau;
    let mut base = BgParams {
        stars: 0.0,
        star_stretch: 0.0,
        milky: 0.0,
        moon: 0.0,
        moon_pos: [-0.42 * aspect, 0.38],
        moon_r: 0.52,
        furnace: 0.0,
        furnace_r: 0.40,
        engine: 0.0,
        shock: 0.0,
        shock_apex: [0.46, 0.0],
        body: 0.0,
        body_pos: [0.32, 0.0],
        body_len: 0.15,
        body_molten: 0.0,
        body_morph: 0.0,
        planet_hint: 0.0,
        ground_glow: 0.0,
        ground_pos: [0.0, -0.55],
        core_glow: 0.0,
        core_pos: [0.0, 0.0],
        flash: 0.0,
        flash_pos: [0.0, 0.0],
        vig: 0.42,
    };
    match beat.id {
        IntroBeatId::Spark => {
            base.flash = 0.9 * (-((lt - 0.05).max(0.0)) * 5.0).exp();
            base.flash_pos = track.p;
            base.core_glow = 0.45 * (-((lt - 0.05).max(0.0)) * 4.0).exp() + 0.34;
            base.core_pos = track.p;
        }
        IntroBeatId::Campfire => {
            base.ground_glow = 0.55;
            base.core_glow = 0.5;
            base.core_pos = [track.p[0], track.p[1] + 0.10];
        }
        IntroBeatId::Furnace => {
            base.furnace = 1.0;
            base.core_glow = 0.55;
            base.core_pos = track.p;
        }
        IntroBeatId::Engine => {
            base.engine = 1.0;
            base.core_glow = 0.7 * (1.0 + 0.25 * (lt * 13.0).sin());
            base.core_pos = track.p;
            base.flash = 0.5 * (-((lt - 0.1).max(0.0)) * 6.0).exp();
            base.flash_pos = track.p;
        }
        IntroBeatId::Primer => {
            base.flash = 1.1 * (-((lt - 0.05).max(0.0)) * 5.0).exp();
            base.flash_pos = track.p;
            base.core_glow = 0.9;
            base.core_pos = track.p;
            let bw = sstep01(0.2, 0.5, tau);
            base.body = bw;
            base.body_pos = [track.p[0] + 0.075, track.p[1]];
            base.shock = 0.5 * sstep01(0.55, 1.0, tau);
            base.shock_apex = [track.p[0] + 0.20, track.p[1]];
        }
        IntroBeatId::Flight => {
            base.stars = 0.8;
            base.star_stretch = 1.0;
            base.body = 1.0;
            base.body_pos = [track.p[0] + 0.075, track.p[1]];
            base.shock = 1.0;
            base.shock_apex = [track.p[0] + 0.22, track.p[1]];
            base.core_glow = 0.9;
            base.core_pos = [track.p[0] + 0.04, track.p[1]];
        }
        IntroBeatId::Morph => {
            base.stars = 0.8;
            base.star_stretch = 1.0;
            let m = ease_in_out_cubic(tau);
            base.body = 1.0;
            base.body_len = 0.15 + 0.05 * m;
            base.body_morph = m;
            base.body_molten = 0.15 + 0.6 * tau;
            base.body_pos = [track.p[0] + 0.075 + 0.04 * m, track.p[1]];
            base.shock = 1.0 - 0.7 * tau;
            base.shock_apex = [track.p[0] + 0.26, track.p[1]];
            base.core_glow = 0.75;
            base.core_pos = [track.p[0] + 0.05, track.p[1]];
        }
        IntroBeatId::Moon => {
            base.stars = 1.0;
            base.star_stretch = 0.25;
            base.milky = 1.0;
            base.moon = sstep01(0.0, 0.35, tau);
            base.body = 1.0 - sstep01(0.0, 0.2, tau);
            base.body_pos = track.p;
            base.body_len = 0.19;
            base.body_morph = 1.0;
            base.core_glow = 1.3;
            base.core_pos = track.p;
        }
        IntroBeatId::Arrival => {
            base.stars = 1.0;
            base.star_stretch = 0.1;
            base.milky = 0.35;
            base.planet_hint = sstep01(0.15, 0.85, tau);
            base.core_glow = 1.1 * (1.0 - 0.75 * tau);
            base.core_pos = track.p;
        }
        IntroBeatId::Landing => {
            base.stars = 1.0;
            base.milky = 0.3;
            base.planet_hint = 1.0;
        }
    }
    base
}

/// 拍间 crossfade 的背景参数数值 lerp（生产 `mixBg`）。
///
/// ⚠ 生产：`t = bw / (aw + bw)` ⇒ **权重比**（不是 `aw/(aw+bw)`）⇒ 逐字照搬。
pub fn mix_bg(a: &BgParams, aw: f64, b: &BgParams, bw: f64) -> BgParams {
    let t = bw / (aw + bw);
    BgParams {
        stars: lerp(a.stars, b.stars, t),
        star_stretch: lerp(a.star_stretch, b.star_stretch, t),
        milky: lerp(a.milky, b.milky, t),
        moon: lerp(a.moon, b.moon, t),
        moon_pos: lerp_vec2(a.moon_pos, b.moon_pos, t),
        moon_r: lerp(a.moon_r, b.moon_r, t),
        furnace: lerp(a.furnace, b.furnace, t),
        furnace_r: lerp(a.furnace_r, b.furnace_r, t),
        engine: lerp(a.engine, b.engine, t),
        shock: lerp(a.shock, b.shock, t),
        shock_apex: lerp_vec2(a.shock_apex, b.shock_apex, t),
        body: lerp(a.body, b.body, t),
        body_pos: lerp_vec2(a.body_pos, b.body_pos, t),
        body_len: lerp(a.body_len, b.body_len, t),
        body_molten: lerp(a.body_molten, b.body_molten, t),
        body_morph: lerp(a.body_morph, b.body_morph, t),
        planet_hint: lerp(a.planet_hint, b.planet_hint, t),
        ground_glow: lerp(a.ground_glow, b.ground_glow, t),
        ground_pos: lerp_vec2(a.ground_pos, b.ground_pos, t),
        core_glow: lerp(a.core_glow, b.core_glow, t),
        core_pos: lerp_vec2(a.core_pos, b.core_pos, t),
        flash: lerp(a.flash, b.flash, t),
        flash_pos: lerp_vec2(a.flash_pos, b.flash_pos, t),
        vig: lerp(a.vig, b.vig, t),
    }
}
