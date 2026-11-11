//! 片头「火之文明史」—— 时间轴 + 每拍编舞 + uniform 取值。
//!
//! ## 这是什么
//!
//! 生产 `frontend/companion-desktop/src/lib/intro/fire.ts` 的**宿主侧场景
//! 语义**在 Rust 侧的对齐实现：
//!
//! | 生产（TS） | 本模块（Rust） |
//! |---|---|
//! | `intro/timeline.ts` 的 `INTRO_BEATS` / `activeBeats` | [`timeline`] |
//! | `fire.ts` 的 `FIRE_TABLE` / `dynamics` / `bgFor` / `mixBg` | [`choreography`] |
//! | `fire.ts` 的 `drawBg` / `drawFire` 的 uniform 写入 | [`uniforms`] |
//!
//! ⇒ **同一时刻算出的 uniform 取值与生产侧逐位相同**（f64 计算 ⇒ 收口 f32
//! 后逐位相同）。
//!
//! ## 职责边界（与本 crate 的其余部分一致）
//!
//! - 本模块只算**取值**（宿主侧场景语义）；
//! - 着色器**不在**本模块（GLSL 资产在 `apeireth-ui-gpu`）；
//! - 绘制**不在**本模块（由 `apeireth-ui-gpu` 承担）。
//!
//! ## 诚实边界
//!
//! - 本模块**只**对齐**取值**；「取值 ⇒ 像素」的那一段（着色器是否逐字等价、
//!   两个后端是否画出同一张图）由 `apeireth-ui-gpu` 与浏览器实测负责；
//! - 生产侧的**粒子位置**是在**顶点着色器里**解算的（`aSeed` + uniform 编舞）
//!   ⇒ 本模块对齐的是「顶点着色器的**输入**」⇒ 顶点着色器本身的等价性由
//!   GLSL 资产的逐字对照负责。

pub mod choreography;
pub mod sprites;
pub mod timeline;
pub mod uniforms;

use apeireth_ui_gpu::UniformValues;

use choreography::{bg_for, dynamics, mix_bg, BgParams, FireParams};

use timeline::{active_beats, BeatWeight, IntroBeatId, INTRO_BEAT_IDS};
use uniforms::{BackdropUniforms, SpriteUniforms, SpriteUniformsArgs};

/// 视口上下文（画布尺寸 / 宽高比 / DPR）。
///
/// ## 为什么 `res`（设备像素）与 `aspect`（CSS 宽高比）**分开**
///
/// 生产侧：画布的**设备像素**尺寸 = `round(cssW * dpr)`，而 `aspect` 是
/// **CSS 宽 / CSS 高** ⇒ 两者在 DPR ≠ 1 时**不相等** ⇒ ⇒ 必须分开存
/// （本模块**照搬**生产口径，不擅自统一）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntroViewport {
    /// 画布设备像素尺寸（`(宽, 高)`）。
    pub res: [f64; 2],
    /// 视口宽高比（CSS 宽 / CSS 高）。
    pub aspect: f64,
    /// 设备像素比（生产 `min(devicePixelRatio, PERFORMANCE.dprMax)`）。
    pub dpr: f64,
}

/// 一帧的完整编舞结果（背景 + 每拍一组粒子）。
#[derive(Debug, Clone, PartialEq)]
pub struct IntroFrame {
    /// 背景 matte 的 uniform 取值（`None` = 此刻**没有活跃拍** ⇒ 不画）。
    pub backdrop: Option<BackdropUniforms>,
    /// 粒子 pass 的 uniform 取值（**每拍一组**，crossfade 期间两组）。
    pub sprites: Vec<SpriteUniforms>,
}

impl IntroFrame {
    /// ⭐ **生产真正会画的那几组**粒子（生产 `drawFire` 的跳过判据）。
    ///
    /// 生产 `fire.ts` 的 `drawFire`：
    /// ```text
    /// const count = p.count01 * countMul;
    /// if (count <= 0.001 || alpha <= 0.001) return;
    /// ```
    ///
    /// ⇒ **粒子数为 0**（落幅拍 `count01 = 0`）或**拍权重为 0** 的那一组，
    ///   生产侧**根本不画** ⇒ ⇒ 逐像素对照时引擎侧也必须**不画**
    ///   （否则多画一层 ⇒ 多一层加法 ⇒ 像素不同）。
    ///
    /// ⚠ [`IntroFrame::sprites`] 保留**全部活跃拍**（编舞事实）⇒
    ///   「编舞有几拍」与「画面画了几层」是**两件事** ⇒ 分开暴露。
    pub fn drawn_sprites(&self) -> Vec<&SpriteUniforms> {
        self.sprites
            .iter()
            .filter(|sprite| sprite.count01 > 0.001 && sprite.alpha > 0.001)
            .collect()
    }
}

/// 时刻 `t` 的完整编舞（生产 `render(t)` 的等价物）。
///
/// ## 与生产的对应关系
///
/// 生产 `render(t)`：
///
/// 1. `beats = activeBeats(t)`；空 ⇒ 直接返回（**不画**）；
/// 2. `tracks = beats.map(dynamics)`；
/// 3. `bgs = beats.map(bgFor)`；两拍 ⇒ `mixBg(bgs[0], beats[0].weight, bgs[1], beats[1].weight)`；
/// 4. `drawBg(mixed, t)`；
/// 5. 每拍各 `drawFire(FIRE_TABLE[b.id], dyn.track, b.weight, dyn.gainMul, dyn.countMul, b.localT)`。
///
/// ⇒ 本函数把 1–5 的**取值**部分一字不改地搬过来。
pub fn intro_frame(t: f64, viewport: &IntroViewport) -> Option<IntroFrame> {
    let beats = active_beats(t);
    if beats.is_empty() {
        return None;
    }
    let tracks: Vec<_> = beats
        .iter()
        .map(|beat| dynamics(beat, viewport.aspect))
        .collect();
    let bgs: Vec<BgParams> = beats
        .iter()
        .zip(tracks.iter())
        .map(|(beat, motion)| bg_for(beat, &motion.track, viewport.aspect))
        .collect();
    let mixed = if bgs.len() == 1 {
        bgs[0]
    } else {
        mix_bg(&bgs[0], beats[0].weight, &bgs[1], beats[1].weight)
    };

    let backdrop = BackdropUniforms::new(viewport.res, viewport.aspect, t, &mixed);
    let sprites: Vec<SpriteUniforms> = beats
        .iter()
        .zip(tracks.iter())
        .map(|(beat, motion)| {
            let params = fire_params(beat.id);
            SpriteUniforms::new(&SpriteUniformsArgs {
                params,
                track: &motion.track,
                // ⭐ **拍本地时间**（生产 `drawFire` 的 `uTime` 口径）⇒ ⇒ 不是全局 `t`。
                local_t: beat.local_t,
                gain_mul: motion.gain_mul,
                count_mul: motion.count_mul,
                alpha: beat.weight,
                aspect: viewport.aspect,
                dpr: viewport.dpr,
            })
        })
        .collect();
    Some(IntroFrame {
        backdrop: Some(backdrop),
        sprites,
    })
}

/// 拍 id → 该拍的粒子编舞参数（生产 `FIRE_TABLE[b.id]`）。
pub fn fire_params(id: IntroBeatId) -> &'static FireParams {
    let index = INTRO_BEAT_IDS
        .iter()
        .position(|candidate| *candidate == id)
        .expect("拍 id 必在拍表里");
    &choreography::FIRE_TABLE[index]
}

/// 把一帧的编舞写进引擎的 uniform 集合。
///
/// ## ⚠ 索引空间是**分开**的
///
/// 背景与粒子是**两条管线**（两套 uniform 块）⇒ ⇒ 两个 uniform 集合
/// （`UniformValues`）**各写各的** ⇒ 索引都从 0 起（各自的声明序）。
///
/// # Errors
///
/// 引擎侧的 uniform 写入失败 → [`apeireth_ui_gpu::BindingError`]。
pub fn write_frame(
    frame: &IntroFrame,
    backdrop_values: &mut UniformValues,
    sprite_values: &mut UniformValues,
) -> Result<(), apeireth_ui_gpu::BindingError> {
    if let Some(backdrop) = &frame.backdrop {
        backdrop.write_into(backdrop_values)?;
    }
    for sprite in &frame.sprites {
        sprite.write_into(sprite_values)?;
    }
    Ok(())
}

/// 活跃拍集合（诊断用；生产 `activeBeats`）。
pub fn beats_at(t: f64) -> Vec<BeatWeight> {
    active_beats(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fire_table_is_aligned_with_the_beat_ids() {
        // ⭐ 拍表与拍序表必须**一一对应**（顺序错了 ⇒ 编舞整段错）。
        assert_eq!(choreography::FIRE_TABLE.len(), INTRO_BEAT_IDS.len());
        for (index, id) in INTRO_BEAT_IDS.iter().enumerate() {
            assert_eq!(fire_params(*id), &choreography::FIRE_TABLE[index]);
        }
    }

    #[test]
    fn the_landing_beat_draws_no_particles() {
        // ⭐ 落幅拍的 `count01 = 0` ⇒ 生产侧 `drawFire` 直接 return
        //   （`count <= 0.001`）⇒ ⇒ **只有背景** ⇒ 这是「背景 pass 可逐像素
        //   对照」的那一拍（浏览器实测用）。
        let viewport = IntroViewport {
            res: [960.0, 600.0],
            aspect: 960.0 / 600.0,
            dpr: 1.0,
        };
        let frame = intro_frame(timeline::INTRO_TOTAL, &viewport).expect("落幅拍活跃");
        assert_eq!(frame.sprites.len(), 1);
        assert_eq!(frame.sprites[0].count01, 0.0);
        assert_eq!(frame.sprites[0].gain, 0.0);
        assert!(frame.backdrop.is_some());
    }

    #[test]
    fn no_active_beat_means_no_frame() {
        // 生产 `render`：`if (!beats.length) return;` ⇒ 不画任何东西。
        let viewport = IntroViewport {
            res: [960.0, 600.0],
            aspect: 1.6,
            dpr: 1.0,
        };
        assert!(intro_frame(-100.0, &viewport).is_none());
    }
}
