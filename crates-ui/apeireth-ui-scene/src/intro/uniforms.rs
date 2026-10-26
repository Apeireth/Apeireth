//! 片头「火之文明史」的 uniform 取值容器（背景 matte + 火焰粒子两套）。
//!
//! ## 为什么按**索引**写（不是按名字）
//!
//! 引擎的 uniform 映射表（`apeireth_ui_gpu::SHADER_UNIFORM_NAMES`）是**黑洞
//! 场景**的 16 项 ⇒ 片头这两套 uniform（背景 27 项 / 粒子 40 项）**不在表里**
//! ⇒ ⇒ 按**索引**写（索引 = uniform 在块里的**成员序** = **声明序**）⇒
//! 与引擎的 `glsl::fragment_uniform_block_members` 的成员序**必然一致**。
//!
//! ## 口径
//!
//! - 取值在本层是 **f64**（与生产脚本运行时同口径）；
//! - 写进引擎的 uniform 缓冲时**收口成 f32**（着色器取值本来就是单精度）
//!   ⇒ ⇒ 与生产侧收口后的结果**逐位相同**。

use apeireth_ui_gpu::{BindingError, UniformValues};

use super::choreography::{BgParams, FireParams, OriginTrack, Vec2, Vec3};

/// 背景 matte 的 uniform 名（生产 `BG_UNIFORMS` 的 27 项，**声明序**）。
///
/// ⚠ **单一真源在引擎侧**（[`apeireth_ui_gpu::shaders::INTRO_BACKDROP_UNIFORM_NAMES`]）
///   ⇒ ⇒ 本层与引擎的布局**不可能**对不上。
pub const BACKDROP_UNIFORM_NAMES: [&str; 27] =
    apeireth_ui_gpu::shaders::INTRO_BACKDROP_UNIFORM_NAMES;

/// 火焰粒子的 uniform 名（生产 `FIRE_UNIFORMS` 的 40 项，**声明序**）。
///
/// ⚠ **单一真源在引擎侧**（[`apeireth_ui_gpu::shaders::INTRO_SPRITE_UNIFORM_NAMES`]）。
pub const SPRITE_UNIFORM_NAMES: [&str; 40] = apeireth_ui_gpu::shaders::INTRO_SPRITE_UNIFORM_NAMES;

/// 背景 matte 的一组 uniform 取值（生产 `BG_UNIFORMS` 的 27 项）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropUniforms {
    /// 画布尺寸（设备像素）。
    pub res: Vec2,
    /// 视口宽高比（CSS 宽 / CSS 高）。
    pub aspect: f64,
    /// 全局时钟（秒）。
    pub time: f64,
    /// 星野强度。
    pub stars: f64,
    /// 星点水平拉伸。
    pub star_stretch: f64,
    /// 银河带强度。
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
    /// 弹体形变。
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

impl BackdropUniforms {
    /// 由「画布尺寸 + 视口宽高比 + 全局时钟 + 背景参数」拼出取值。
    ///
    /// ⚠ 对齐生产 `drawBg` 的 uniform 写入顺序。
    pub fn new(res: Vec2, aspect: f64, time: f64, params: &BgParams) -> Self {
        Self {
            res,
            aspect,
            time,
            stars: params.stars,
            star_stretch: params.star_stretch,
            milky: params.milky,
            moon: params.moon,
            moon_pos: params.moon_pos,
            moon_r: params.moon_r,
            furnace: params.furnace,
            furnace_r: params.furnace_r,
            engine: params.engine,
            shock: params.shock,
            shock_apex: params.shock_apex,
            body: params.body,
            body_pos: params.body_pos,
            body_len: params.body_len,
            body_molten: params.body_molten,
            body_morph: params.body_morph,
            planet_hint: params.planet_hint,
            ground_glow: params.ground_glow,
            ground_pos: params.ground_pos,
            core_glow: params.core_glow,
            core_pos: params.core_pos,
            flash: params.flash,
            flash_pos: params.flash_pos,
            vig: params.vig,
        }
    }

    /// 按**声明序**逐项写入引擎的 uniform 集合（索引 = 声明序）。
    ///
    /// # Errors
    ///
    /// 引擎侧的 uniform 写入失败（缓冲太小等）→ [`BindingError`]。
    pub fn write_into(&self, values: &mut UniformValues) -> Result<(), BindingError> {
        // ⚠ **栈上**缓冲（每项最多 2 个分量）⇒ 每帧**零堆分配**。
        let mut buffer = [[0.0f32; 2]; 27];
        let mut lengths = [0usize; 27];
        let mut put = |index: usize, components: &[f32]| {
            buffer[index][..components.len()].copy_from_slice(components);
            lengths[index] = components.len();
        };
        put(0, &[self.res[0] as f32, self.res[1] as f32]);
        put(1, &[self.aspect as f32]);
        put(2, &[self.time as f32]);
        put(3, &[self.stars as f32]);
        put(4, &[self.star_stretch as f32]);
        put(5, &[self.milky as f32]);
        put(6, &[self.moon as f32]);
        put(7, &[self.moon_pos[0] as f32, self.moon_pos[1] as f32]);
        put(8, &[self.moon_r as f32]);
        put(9, &[self.furnace as f32]);
        put(10, &[self.furnace_r as f32]);
        put(11, &[self.engine as f32]);
        put(12, &[self.shock as f32]);
        put(13, &[self.shock_apex[0] as f32, self.shock_apex[1] as f32]);
        put(14, &[self.body as f32]);
        put(15, &[self.body_pos[0] as f32, self.body_pos[1] as f32]);
        put(16, &[self.body_len as f32]);
        put(17, &[self.body_molten as f32]);
        put(18, &[self.body_morph as f32]);
        put(19, &[self.planet_hint as f32]);
        put(20, &[self.ground_glow as f32]);
        put(21, &[self.ground_pos[0] as f32, self.ground_pos[1] as f32]);
        put(22, &[self.core_glow as f32]);
        put(23, &[self.core_pos[0] as f32, self.core_pos[1] as f32]);
        put(24, &[self.flash as f32]);
        put(25, &[self.flash_pos[0] as f32, self.flash_pos[1] as f32]);
        put(26, &[self.vig as f32]);
        for index in 0..27 {
            values.set(index as u32, &buffer[index][..lengths[index]]);
        }
        Ok(())
    }

    /// 诊断视图：(名字, 分量) 列表（顺序 = 声明序）。
    ///
    /// ⚠ **诊断用途**（每次调用会分配）⇒ 不要放在每帧热路径上。
    pub fn entries(&self) -> Vec<(&'static str, Vec<f64>)> {
        vec![
            ("uRes", self.res.to_vec()),
            ("uAspect", vec![self.aspect]),
            ("uTime", vec![self.time]),
            ("uStars", vec![self.stars]),
            ("uStarStretch", vec![self.star_stretch]),
            ("uMilky", vec![self.milky]),
            ("uMoon", vec![self.moon]),
            ("uMoonPos", self.moon_pos.to_vec()),
            ("uMoonR", vec![self.moon_r]),
            ("uFurnace", vec![self.furnace]),
            ("uFurnaceR", vec![self.furnace_r]),
            ("uEngine", vec![self.engine]),
            ("uShock", vec![self.shock]),
            ("uShockApex", self.shock_apex.to_vec()),
            ("uBody", vec![self.body]),
            ("uBodyPos", self.body_pos.to_vec()),
            ("uBodyLen", vec![self.body_len]),
            ("uBodyMolten", vec![self.body_molten]),
            ("uBodyMorph", vec![self.body_morph]),
            ("uPlanetHint", vec![self.planet_hint]),
            ("uGroundGlow", vec![self.ground_glow]),
            ("uGroundPos", self.ground_pos.to_vec()),
            ("uCoreGlow", vec![self.core_glow]),
            ("uCorePos", self.core_pos.to_vec()),
            ("uFlash", vec![self.flash]),
            ("uFlashPos", self.flash_pos.to_vec()),
            ("uVig", vec![self.vig]),
        ]
    }
}

/// 火焰粒子的一组 uniform 取值（生产 `FIRE_UNIFORMS` 的 40 项）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteUniforms {
    /// 视口宽高比。
    pub aspect: f64,
    /// 设备像素比（生产 `min(devicePixelRatio, PERFORMANCE.dprMax)`）。
    pub dpr: f64,
    /// 本拍本地时间（秒）。
    pub time: f64,
    /// 发射原点。
    pub p0: Vec2,
    /// 弹道速度。
    pub vel: Vec2,
    /// 弹道加速度。
    pub acc: Vec2,
    /// 主喷射方向。
    pub dir: Vec2,
    /// 方向散布（弧度）。
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
    /// 出生尺寸。
    pub size0: f64,
    /// 消亡尺寸。
    pub size1: f64,
    /// 尺寸抖动幅度。
    pub size_var: f64,
    /// 重力。
    pub grav: Vec2,
    /// 阻力。
    pub drag: f64,
    /// 湍流幅度。
    pub turb: f64,
    /// 湍流频率。
    pub turb_freq: f64,
    /// 湍流时间缩放。
    pub turb_time: f64,
    /// 活跃粒子比例。
    pub count01: f64,
    /// 是否一次性迸发。
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
    /// 本拍的权重（crossfade 时 = 拍权重）。
    pub alpha: f64,
    /// 热色。
    pub col_hot: Vec3,
    /// 中段色。
    pub col_mid: Vec3,
    /// 尾色。
    pub col_tail: Vec3,
}

/// [`SpriteUniforms::new`] 的**参数包**。
///
/// ## 为什么要包起来
///
/// 原来 `new` 有 **8 个**位置参数（`clippy::too_many_arguments` ⇒ 上限 7）
/// ⇒ ⇒ 且 8 个 `f64` 在调用点**极易传错顺序**（编译器**不报错**）
/// ⇒ ⇒ 打包成具名结构 ⇒ ⇒ 调用点**自带名字** ⇒ ⇒ 顺序错了**一眼可见**。
#[derive(Debug, Clone, Copy)]
pub struct SpriteUniformsArgs<'a> {
    /// 火焰参数（按拍取值）。
    pub params: &'a FireParams,
    /// 弹道。
    pub track: &'a OriginTrack,
    /// ⭐ **本拍本地时间**（秒）—— 生产 `fire.ts` 的 **sprite pass** 的 `uTime`。
    ///
    /// ⚠⚠️ **口径与背景 pass 不同**（这是 2026-10-26 实测钉死的）：
    /// - 生产 `drawBg`  ⇒ `gl.uniform1f(u.uTime, tGlobal);` ⇒ **背景用全局时间**；
    /// - 生产 `drawFire`⇒ `gl.uniform1f(u.uTime, localT);`  ⇒ **粒子用拍本地时间**。
    ///
    /// ⇒ ⇒ 引擎的 sprite pass 必须用**拍本地时间**（`localT = t - start + XF/2`）
    /// ⇒ ⇒ 用全局时间会**错**（虽然在一拍上可能偶然相等）。
    ///
    /// 生产原文（`FIRE_VERT`）：
    /// ```text
    /// float birth  = uBurst > 0.5 ? r2 * (radial ? min(uBurstSpan, 0.18) : uBurstSpan) : 0.0;
    /// float ageSec = uBurst > 0.5 ? (uTime - birth) : fract(uTime / life + r2) * life;
    /// ```
    /// ⇒ ⇒ 非 burst 时 `birth = 0` ⇒ ⇒ 粒子年龄**只由 `uTime` 的相位**决定。
    pub local_t: f64,
    /// 增益乘子。
    pub gain_mul: f64,
    /// 数量乘子。
    pub count_mul: f64,
    /// 本拍权重（crossfade 时 = 拍权重）。
    pub alpha: f64,
    /// 视口宽高比。
    pub aspect: f64,
    /// 设备像素比。
    pub dpr: f64,
}

impl SpriteUniforms {
    /// 由「参数 + 弹道 + 本拍本地时间 + 增益乘子 + 权重」拼出取值。
    ///
    /// ⚠ 对齐生产 `drawFire` 的 uniform 写入顺序与算法：
    ///   `count = p.count01 * countMul`；`uGain = p.gain * gainMul`。
    pub fn new(args: &SpriteUniformsArgs<'_>) -> Self {
        let SpriteUniformsArgs {
            params,
            track,
            local_t: _,
            gain_mul,
            count_mul,
            alpha,
            aspect,
            dpr,
        } = *args;
        Self {
            aspect,
            dpr,
            // ⭐ **拍本地时间**（生产 `drawFire` 的 `uTime` 口径）⇒ ⇒ 见 `SpriteUniformsArgs::local_t`。
            time: args.local_t,
            p0: track.p,
            vel: track.v,
            acc: track.a,
            dir: params.dir,
            spread: params.spread,
            spawn_r: params.spawn_r,
            speed: params.speed,
            speed_var: params.speed_var,
            inherit: params.inherit,
            life: params.life,
            life_var: params.life_var,
            size0: params.size0,
            size1: params.size1,
            size_var: params.size_var,
            grav: params.grav,
            drag: params.drag,
            turb: params.turb,
            turb_freq: params.turb_freq,
            turb_time: params.turb_time,
            count01: params.count01 * count_mul,
            burst: params.burst,
            burst_span: params.burst_span,
            radial01: params.radial01,
            radial_speed: params.radial_speed,
            radial_drag: params.radial_drag,
            radial_life: params.radial_life,
            soft01: params.soft01,
            soft_size: params.soft_size,
            soft_gain: params.soft_gain,
            streak: params.streak,
            twinkle: params.twinkle,
            gain: params.gain * gain_mul,
            ember01: params.ember01,
            alpha,
            col_hot: params.col_hot,
            col_mid: params.col_mid,
            col_tail: params.col_tail,
        }
    }

    /// 按**声明序**逐项写入引擎的 uniform 集合（索引 = 声明序）。
    ///
    /// # Errors
    ///
    /// 引擎侧的 uniform 写入失败 → [`BindingError`]。
    pub fn write_into(&self, values: &mut UniformValues) -> Result<(), BindingError> {
        // ⚠ **栈上**缓冲（每项最多 3 个分量）⇒ 每帧**零堆分配**。
        let mut buffer = [[0.0f32; 3]; 40];
        let mut lengths = [0usize; 40];
        let mut put = |index: usize, components: &[f32]| {
            buffer[index][..components.len()].copy_from_slice(components);
            lengths[index] = components.len();
        };
        put(0, &[self.aspect as f32]);
        put(1, &[self.dpr as f32]);
        put(2, &[self.time as f32]);
        put(3, &[self.p0[0] as f32, self.p0[1] as f32]);
        put(4, &[self.vel[0] as f32, self.vel[1] as f32]);
        put(5, &[self.acc[0] as f32, self.acc[1] as f32]);
        put(6, &[self.dir[0] as f32, self.dir[1] as f32]);
        put(7, &[self.spread as f32]);
        put(8, &[self.spawn_r as f32]);
        put(9, &[self.speed as f32]);
        put(10, &[self.speed_var as f32]);
        put(11, &[self.inherit as f32]);
        put(12, &[self.life as f32]);
        put(13, &[self.life_var as f32]);
        put(14, &[self.size0 as f32]);
        put(15, &[self.size1 as f32]);
        put(16, &[self.size_var as f32]);
        put(17, &[self.grav[0] as f32, self.grav[1] as f32]);
        put(18, &[self.drag as f32]);
        put(19, &[self.turb as f32]);
        put(20, &[self.turb_freq as f32]);
        put(21, &[self.turb_time as f32]);
        put(22, &[self.count01 as f32]);
        put(23, &[self.burst as f32]);
        put(24, &[self.burst_span as f32]);
        put(25, &[self.radial01 as f32]);
        put(26, &[self.radial_speed as f32]);
        put(27, &[self.radial_drag as f32]);
        put(28, &[self.radial_life as f32]);
        put(29, &[self.soft01 as f32]);
        put(30, &[self.soft_size as f32]);
        put(31, &[self.soft_gain as f32]);
        put(32, &[self.streak as f32]);
        put(33, &[self.twinkle as f32]);
        put(34, &[self.gain as f32]);
        put(35, &[self.ember01 as f32]);
        put(36, &[self.alpha as f32]);
        put(
            37,
            &[
                self.col_hot[0] as f32,
                self.col_hot[1] as f32,
                self.col_hot[2] as f32,
            ],
        );
        put(
            38,
            &[
                self.col_mid[0] as f32,
                self.col_mid[1] as f32,
                self.col_mid[2] as f32,
            ],
        );
        put(
            39,
            &[
                self.col_tail[0] as f32,
                self.col_tail[1] as f32,
                self.col_tail[2] as f32,
            ],
        );
        for index in 0..40 {
            values.set(index as u32, &buffer[index][..lengths[index]]);
        }
        Ok(())
    }

    /// 诊断视图：(名字, 分量) 列表（顺序 = 声明序）。
    ///
    /// ⚠ **诊断用途**（每次调用会分配）⇒ 不要放在每帧热路径上。
    pub fn entries(&self) -> Vec<(&'static str, Vec<f64>)> {
        vec![
            ("uAspect", vec![self.aspect]),
            ("uDpr", vec![self.dpr]),
            ("uTime", vec![self.time]),
            ("uP0", self.p0.to_vec()),
            ("uVel", self.vel.to_vec()),
            ("uAcc", self.acc.to_vec()),
            ("uDir", self.dir.to_vec()),
            ("uSpread", vec![self.spread]),
            ("uSpawnR", vec![self.spawn_r]),
            ("uSpeed", vec![self.speed]),
            ("uSpeedVar", vec![self.speed_var]),
            ("uInherit", vec![self.inherit]),
            ("uLife", vec![self.life]),
            ("uLifeVar", vec![self.life_var]),
            ("uSize0", vec![self.size0]),
            ("uSize1", vec![self.size1]),
            ("uSizeVar", vec![self.size_var]),
            ("uGrav", self.grav.to_vec()),
            ("uDrag", vec![self.drag]),
            ("uTurb", vec![self.turb]),
            ("uTurbFreq", vec![self.turb_freq]),
            ("uTurbTime", vec![self.turb_time]),
            ("uCount01", vec![self.count01]),
            ("uBurst", vec![self.burst]),
            ("uBurstSpan", vec![self.burst_span]),
            ("uRadial01", vec![self.radial01]),
            ("uRadialSpeed", vec![self.radial_speed]),
            ("uRadialDrag", vec![self.radial_drag]),
            ("uRadialLife", vec![self.radial_life]),
            ("uSoft01", vec![self.soft01]),
            ("uSoftSize", vec![self.soft_size]),
            ("uSoftGain", vec![self.soft_gain]),
            ("uStreak", vec![self.streak]),
            ("uTwinkle", vec![self.twinkle]),
            ("uGain", vec![self.gain]),
            ("uEmber01", vec![self.ember01]),
            ("uAlpha", vec![self.alpha]),
            ("uColHot", self.col_hot.to_vec()),
            ("uColMid", self.col_mid.to_vec()),
            ("uColTail", self.col_tail.to_vec()),
        ]
    }
}
