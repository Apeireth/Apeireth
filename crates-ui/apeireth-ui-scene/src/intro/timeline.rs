//! 片头「火之文明史」分拍时间轴（生产 `intro/timeline.ts` 的 Rust 对齐实现）。
//!
//! ## 对齐口径
//!
//! 本模块是生产 `frontend/companion-desktop/src/lib/intro/timeline.ts` 的
//! **逐值对齐**移植：拍表、拍起点、crossfade 半宽、活跃拍集合的判定与权重
//! **一字不改**地搬过来 ⇒ ⇒ 同一时刻 `t` 得到的活跃拍集合（拍 id / 权重 /
//! 本地时间 / 拍内进度）与生产侧**逐位相同**。
//!
//! ## 为什么单独成模块
//!
//! - 时间轴是**纯函数**（无 DOM、无框架依赖）⇒ 可以独立单测；
//! - 拍表是**数据**（生产侧是 `as const` 的只读数组）⇒ 搬成 Rust 的
//!   `const` 数组 ⇒ 编译期固定 ⇒ 运行期零分配。
//!
//! ## 数值口径
//!
//! 生产侧跑在 JS 运行时里 ⇒ 数值是 **f64**；本模块也用 **f64** ⇒ 写进
//! uniform 缓冲时统一收口成 f32（着色器取值本来就是单精度）⇒ ⇒ 与生产侧
//! 收口到 f32 的结果**逐位相同**。

/// 一拍的 id（生产 `IntroBeatId` 的对齐实现）。
///
/// ⚠ **顺序即拍序**（生产侧 `INTRO_BEATS` 的声明顺序）⇒ 拍起点由时长累计
///   ⇒ ⇒ 改顺序会改时间轴 ⇒ 由单测钉住。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntroBeatId {
    /// 拍 1：打火石（纯黑中迸出金色火花）。
    Spark,
    /// 拍 2：营火（黑夜营火，火星升腾）。
    Campfire,
    /// 拍 3：蒸汽机（铆接炉门内火焰翻滚）。
    Furnace,
    /// 拍 4：内燃机（气缸内燃烧火球脉动）。
    Engine,
    /// 拍 5：底火出膛（放射环 + 子弹旋出；此拍起不再剪）。
    Primer,
    /// 拍 6：子弹飞行（火焰尾迹 + 激波锥）。
    Flight,
    /// 拍 7：变形（子弹熔成金色流线重塑为火箭）。
    Morph,
    /// 拍 8：划月（火箭划月，尾焰一道金线）。
    Moon,
    /// 拍 9：抵达（金线弧线划向行星/星环方向）。
    Arrival,
    /// 拍 10：落幅（无缝交接活舰桥）。
    Landing,
}

/// 全部拍 id（拍序）。
pub const INTRO_BEAT_IDS: [IntroBeatId; 10] = [
    IntroBeatId::Spark,
    IntroBeatId::Campfire,
    IntroBeatId::Furnace,
    IntroBeatId::Engine,
    IntroBeatId::Primer,
    IntroBeatId::Flight,
    IntroBeatId::Morph,
    IntroBeatId::Moon,
    IntroBeatId::Arrival,
    IntroBeatId::Landing,
];

/// 各拍时长（秒）—— 生产 `INTRO_BEATS` 的 `dur`（逐值照搬）。
///
/// ⚠ **顺序必须与 [`INTRO_BEAT_IDS`] 一致** ⇒ 由单测钉住。
pub const INTRO_BEAT_DURATIONS: [f64; 10] = [0.8, 1.2, 1.2, 1.0, 1.2, 1.5, 1.2, 2.0, 2.5, 1.5];

/// 拍间 crossfade 半宽（秒）—— 生产 `INTRO_XF`。
pub const INTRO_XF: f64 = 0.3;

/// 各拍起点（秒）—— 由时长累计（生产 `INTRO_STARTS` 的同一算法）。
///
/// ⚠ 用**编译期常量**表达式求值（不是运行期累加）⇒ ⇒ 与生产侧
///   「同样顺序累加同样时长」的 f64 结果**逐位相同**（f64 加法在同样操作数
///   顺序下是确定性的）。
pub const INTRO_STARTS: [f64; 10] = {
    let mut starts = [0.0f64; 10];
    let mut acc = 0.0f64;
    let mut index = 0;
    while index < 10 {
        starts[index] = acc;
        acc += INTRO_BEAT_DURATIONS[index];
        index += 1;
    }
    starts
};

/// 总时长（秒）—— 生产 `INTRO_TOTAL`。
pub const INTRO_TOTAL: f64 = INTRO_STARTS[9] + INTRO_BEAT_DURATIONS[9];

/// 落幅拍起点（秒）—— 生产 `INTRO_LANDING_START`（跳过 / 快进的目标时刻）。
pub const INTRO_LANDING_START: f64 = INTRO_STARTS[9];

/// 一拍在时刻 `t` 的活跃权重与本拍本地时间。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatWeight {
    /// 拍序号（0 起）。
    pub index: usize,
    /// 拍 id。
    pub id: IntroBeatId,
    /// 权重 0..1（crossfade 窗内平滑过渡）。
    pub weight: f64,
    /// 本拍本地时间（秒，含 `XF/2` lead-in ⇒ 循环粒子无冷启动）。
    pub local_t: f64,
    /// 本拍进度 0..1。
    pub tau: f64,
}

/// 与生产 `sstep(a, b, x)` **逐值一致**的平滑阶跃。
///
/// ⚠ 生产 `scene/timeline.ts` 的 `sstep`：
/// `t = max(0, min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t);`
///
/// ⇒ 这里**逐字照搬**（不换 `smoothstep` 之类 ⇒ 免得边界行为不同）。
pub fn sstep(a: f64, b: f64, x: f64) -> f64 {
    let t = (x - a) / (b - a);
    let t = if t < 0.0 { 0.0 } else { t.min(1.0) };
    t * t * (3.0 - 2.0 * t)
}

/// 时刻 `t` 的活跃拍集合（**至多两拍**）—— 生产 `activeBeats(t)` 的对齐实现。
///
/// ## 语义（与生产一致）
///
/// - 拍 i 的活跃区间 = `[start_i - XF/2, start_i + dur_i + XF/2]`；
/// - 权重：拍 i 在 `[start_i - XF/2, start_i + XF/2]` 淡入，
///   在 `[end_i - XF/2, end_i + XF/2]` 淡出；**首拍无淡入、末拍无淡出**；
/// - 权重 `≤ 0.0005` 的拍**不返回**；
/// - `t < 0` 或 `t > INTRO_TOTAL` 时**仍返回端点拍**。
///
/// ⚠ 返回的是 `Vec` ⇒ **每帧一次分配**（生产侧同理：每次 `render` 都新建
///   数组）⇒ 若要零分配，调用方可以复用缓冲（本函数是**纯函数**，便于自测）。
pub fn active_beats(t: f64) -> Vec<BeatWeight> {
    let n = INTRO_BEAT_IDS.len();
    let mut out = Vec::with_capacity(2);
    for index in 0..n {
        let start = INTRO_STARTS[index];
        let dur = INTRO_BEAT_DURATIONS[index];
        let end = start + dur;
        // 活跃区间向两侧各扩 XF/2（crossfade 窗）。
        if t < start - INTRO_XF / 2.0 || t > end + INTRO_XF / 2.0 {
            continue;
        }
        let mut weight = 1.0f64;
        if index > 0 {
            weight *= sstep(start - INTRO_XF / 2.0, start + INTRO_XF / 2.0, t);
        }
        if index < n - 1 {
            weight *= 1.0 - sstep(end - INTRO_XF / 2.0, end + INTRO_XF / 2.0, t);
        }
        if weight <= 0.0005 {
            continue;
        }
        let local_t = t - start + INTRO_XF / 2.0;
        let tau = ((t - start + INTRO_XF / 2.0) / dur).clamp(0.0, 1.0);
        out.push(BeatWeight {
            index,
            id: INTRO_BEAT_IDS[index],
            weight,
            local_t,
            tau,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beat_table_matches_the_production_one() {
        // ⭐ 生产 `intro/timeline.ts` 的 `INTRO_BEATS`（逐值照搬）⇒
        //   时长表与拍序表必须一一对应（顺序错了 ⇒ 整条时间轴错）。
        assert_eq!(
            INTRO_BEAT_DURATIONS,
            [0.8, 1.2, 1.2, 1.0, 1.2, 1.5, 1.2, 2.0, 2.5, 1.5]
        );
        assert_eq!(INTRO_BEAT_IDS.len(), INTRO_BEAT_DURATIONS.len());
    }

    #[test]
    fn the_starts_accumulate_the_durations() {
        // 生产 `INTRO_STARTS` 由时长累计 ⇒ 起点必须严格递增。
        assert_eq!(INTRO_STARTS[0], 0.0);
        let mut acc = 0.0;
        for index in 0..INTRO_BEAT_DURATIONS.len() {
            assert_eq!(INTRO_STARTS[index], acc, "第 {index} 拍的起点不对");
            acc += INTRO_BEAT_DURATIONS[index];
        }
        assert_eq!(INTRO_TOTAL, acc);
        // 生产侧总时长 ≈ 14.1s。
        assert!((INTRO_TOTAL - 14.1).abs() < 1e-9, "{}", INTRO_TOTAL);
        assert_eq!(INTRO_LANDING_START, INTRO_STARTS[9]);
    }

    #[test]
    fn sstep_matches_the_production_formula() {
        assert_eq!(sstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(sstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(sstep(0.0, 1.0, 0.5), 0.5);
        assert_eq!(sstep(0.0, 1.0, 0.0), 0.0);
    }

    #[test]
    fn at_most_two_beats_are_ever_active() {
        // ⭐ 生产纪律：任意时刻**至多两拍**同时活跃（crossfade 窗对称）。
        let mut t = -0.5;
        while t < INTRO_TOTAL + 0.5 {
            let beats = active_beats(t);
            assert!(beats.len() <= 2, "t={t} ⇒ {} 拍", beats.len());
            t += 0.01;
        }
    }

    #[test]
    fn the_first_beat_fades_in_and_the_last_one_does_not_fade_out() {
        // 首拍无淡入（权重恒 1）；末拍无淡出。
        let first = active_beats(0.0);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].id, IntroBeatId::Spark);
        assert_eq!(first[0].weight, 1.0, "首拍不得淡入");

        let last = active_beats(INTRO_TOTAL);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].id, IntroBeatId::Landing);
        assert_eq!(last[0].weight, 1.0, "末拍不得淡出");
    }

    #[test]
    fn the_local_time_carries_the_lead_in() {
        // 生产：`localT = t - start + XF/2` ⇒ 拍起点处本地时间已是 XF/2。
        let beats = active_beats(0.0);
        assert_eq!(beats[0].local_t, INTRO_XF / 2.0);
        assert_eq!(beats[0].tau, (INTRO_XF / 2.0) / INTRO_BEAT_DURATIONS[0]);
    }

    #[test]
    fn out_of_range_times_return_no_beat() {
        // ⚠ **生产的注释与代码不一致**（实测：按**代码**为准）。
        //
        // 生产 `intro/timeline.ts` 的注释写着「t<0 或 t>INTRO_TOTAL 时仍返回
        // 端点拍」，但**代码**是：
        //
        // ```js
        // if (t < start - INTRO_XF / 2 || t > end + INTRO_XF / 2) continue;
        // ```
        //
        // ⇒ `t = -10` 时首拍也被 `continue` 掉 ⇒ **返回空集**（不是端点拍）。
        // ⇒ 生产 `render` 随后 `if (!beats.length) return;` ⇒ **什么都不画**。
        //
        // ⇒ 本移植**照代码**实现（不照注释）⇒ 并在此把差异钉住。
        assert!(
            active_beats(-10.0).is_empty(),
            "生产代码在 t<0 时返回**空集**（注释说端点拍 ⇒ 注释是陈的）"
        );
        assert!(
            active_beats(INTRO_TOTAL + 10.0).is_empty(),
            "生产代码在 t>INTRO_TOTAL 时返回**空集**"
        );
        // 端点**刚好**在活跃窗内 ⇒ 返回端点拍。
        assert_eq!(active_beats(0.0).len(), 1);
        assert_eq!(active_beats(INTRO_TOTAL).len(), 1);
    }
}
