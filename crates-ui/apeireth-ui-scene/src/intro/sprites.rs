//! 片头火焰粒子的**顶点属性**（每粒一个 `vec4` 种子）—— 生产侧算法的逐值移植。
//!
//! ## 生产侧怎么生成种子
//!
//! 生产 `intro/fire.ts` 的 `initGL()`：
//!
//! ```js
//! const seeds = new Float32Array(MAX_PARTICLES * 4);
//! let s = 1234567;
//! for (let i = 0; i < seeds.length; i++) {
//!   // xorshift32 确定性伪随机
//!   s ^= s << 13; s ^= s >>> 17; s ^= s << 5;
//!   seeds[i] = ((s >>> 0) % 100000) / 100000;
//! }
//! ```
//!
//! ⇒ **确定性** ⇒ 引擎侧可以**逐值复现** ⇒ ⇒ 粒子的初始状态与生产**一致**。
//!
//! ## ⚠ 数值口径（**必须**逐位相同）
//!
//! - JS 的位运算是 **32 位**：`s << 13` 是 int32 的左移（高位被丢弃）⇒ Rust
//!   侧用 `u32` 的 `<<`/`^`/`>>>`（`u32` 的移位与截断语义**逐位相同**）；
//! - `s >>> 17` 是**逻辑**右移 ⇒ Rust 的 `u32 >> 17` ✓；
//! - `((s >>> 0) % 100000) / 100000` ⇒ 先取模（`u32`）⇒ 再**除成 f64** ⇒
//!   最后存进 `Float32Array` ⇒ **收口成 f32** ⇒ Rust 侧照做（`as f32`）。
//!
//! ⚠ 若这里错一位 ⇒ 粒子的初始分布就变 ⇒ 画面**完全不同** ⇒ ⇒ 由单测
//!   `the_seed_generator_reproduces_the_production_sequence` 钉住（对照生产
//!   源码里的同一段循环）。

/// 粒子上限（生产 `MAX_PARTICLES`）。
pub const MAX_PARTICLES: usize = 4096;

/// 种子缓冲的**长度**（分量数 = 粒子数 × 4）。
pub const SEED_COMPONENTS: usize = MAX_PARTICLES * 4;

/// 生成粒子的**种子属性**（`vec4` × [`MAX_PARTICLES`]，**展平成 f32 数组**）。
///
/// ## ⚠ 每帧**不要**重算
///
/// 种子是**静态**的（生产侧在 `initGL` 里算一次 ⇒ 顶点缓冲里常驻）⇒ ⇒
/// 调用方应当**算一次、缓存起来**（本函数每次调用会分配一个新 `Vec`）。
pub fn sprite_seeds() -> Vec<f32> {
    let mut seeds = Vec::with_capacity(SEED_COMPONENTS);
    let mut state: u32 = 1_234_567;
    for _ in 0..SEED_COMPONENTS {
        // ⚠ 与生产**逐字**一致：xorshift32（`^= <<13`、`^= >>>17`、`^= <<5`）。
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let value = (state % 100_000) as f64 / 100_000.0;
        seeds.push(value as f32);
    }
    seeds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_generator_reproduces_the_production_sequence() {
        // ⭐ 与生产 `initGL()` 的那段循环**逐值对照**。
        //
        // 口径：生产侧的同一段循环在 **Node（v22.23.1）** 里跑出来的前几个值是
        // `0.34981998801231384, 0.2462799996137619, 0.37299999594688416, …`
        // （**f32** 值）⇒ ⇒ 本实现必须**逐位相同** ⇒ ⇒ 用**精确相等**断言
        // （f32 → f64 的转换是精确的 ⇒ 相等即逐位相同）。
        let seeds = sprite_seeds();
        assert_eq!(seeds.len(), SEED_COMPONENTS);
        let expected = [
            0.34981998801231384f64,
            0.2462799996137619f64,
            0.37299999594688416f64,
            0.8909400105476379f64,
        ];
        for (index, want) in expected.iter().enumerate() {
            assert_eq!(
                seeds[index] as f64, *want,
                "第 {index} 个种子与生产不一致（差一位 ⇒ 粒子分布全变）"
            );
        }
        // 全部落在 [0, 1) ⇒ 生产用 `% 100000` 保证了这一点。
        for value in &seeds {
            assert!(*value >= 0.0 && *value < 1.0, "种子越界：{value}");
        }
    }

    #[test]
    fn the_seed_generator_is_deterministic() {
        // ⭐ 刷新不重排（生产注释：「确定性 hash，刷新不重排」）⇒ ⇒ 两次调用
        //   必须**逐值相同**。
        assert_eq!(sprite_seeds(), sprite_seeds());
    }

    #[test]
    fn the_seed_values_match_a_reference_implementation() {
        // ⭐ 用**另一份写法**（显式的位运算展开）复算前 64 个 ⇒ 交叉验证。
        let seeds = sprite_seeds();
        let mut state: u32 = 1_234_567;
        for (index, seed) in seeds.iter().enumerate().take(64) {
            state ^= state.wrapping_shl(13);
            state ^= state.wrapping_shr(17);
            state ^= state.wrapping_shl(5);
            let expected = ((state % 100_000) as f64 / 100_000.0) as f32;
            assert_eq!(*seed, expected, "第 {index} 个种子不对");
        }
    }
}
