//! ⭐ **布局位移观测（CLS）** —— 台账 L1-G7 的「CLS 观测」部分。
//!
//! ## 为什么归引擎
//!
//! 「布局跳动」是**用户可见、但各处不可测**的质量问题：
//! 它发生在**两帧之间**，需要「上一帧的几何」与「这一帧的几何」才能算出来
//! ⇒ ⇒ 只有引擎（持有逐帧几何与渲染节奏）能观测
//! ⇒ ⇒ 交给每个页面自己监控 ⇒ ⇒ 没人做，或各做一套算法不可比。
//!
//! ## 算法（与 web-vitals 的 CLS 同构，**但不依赖任何外部库**）
//!
//! 对每个被追踪的元素，比较**上一帧**与**这一帧**的元素盒：
//!
//! - **影响面积占比**（impact fraction）= `上一帧 ∪ 这一帧 ∩ 视口` 的面积 / 视口面积；
//! - **位移占比**（distance fraction）= `max(|dx|, |dy|) / max(视口宽, 视口高)`；
//! - **本次得分** = 影响面积占比 × 位移占比。
//!
//! 累加所有「非预期位移」即得 CLS 近似值。
//!
//! ## ⚠️ 两条重要口径（避免把噪声当缺陷）
//!
//! 1. **亚像素位移不计数**：渲染噪声、抗锯齿舍入会产生 < 1px 的几何抖动
//!    ⇒ ⇒ 低于 [`DEFAULT_SHIFT_THRESHOLD_PX`] 的位移**不计分**
//!    （否则我们会把自己 E-54 那类 1 LSB 差异记成"布局跳动"）；
//! 2. **视口外元素不计数**：用户看不到的位移没有体验影响。
//!
//! 本模块**只观测**：不修正布局、不隐藏跳变、不决定阈值（阈值可配，默认给一个保守值）。

use std::collections::BTreeMap;

use crate::dom::ElementBox;

/// 默认的「算作位移」阈值（CSS 像素）：低于它视为渲染噪声。
///
/// 取 1.0px：亚像素噪声（<1px）不计分，而 1px 以上对用户已是可见跳变。
pub const DEFAULT_SHIFT_THRESHOLD_PX: f64 = 1.0;

/// 被追踪元素的稳定标识（由上层决定：key、路径编号均可）。
pub type TrackId = u64;

/// ⭐ **布局位移追踪器**。
#[derive(Debug)]
pub struct LayoutShiftTracker {
    viewport_width: f64,
    viewport_height: f64,
    threshold_px: f64,
    last: BTreeMap<TrackId, ElementBox>,
    total: f64,
    worst: f64,
    samples: usize,
}

impl LayoutShiftTracker {
    /// 按视口尺寸建追踪器（非法尺寸按 0 处理 ⇒ 影响占比恒 0，不会除零）。
    pub fn new(viewport_width: f64, viewport_height: f64) -> Self {
        Self {
            viewport_width: sanitize(viewport_width),
            viewport_height: sanitize(viewport_height),
            threshold_px: DEFAULT_SHIFT_THRESHOLD_PX,
            last: BTreeMap::new(),
            total: 0.0,
            worst: 0.0,
            samples: 0,
        }
    }

    /// 设定位移阈值（CSS 像素；非法值忽略）。
    pub fn set_threshold(&mut self, threshold_px: f64) {
        if threshold_px.is_finite() && threshold_px >= 0.0 {
            self.threshold_px = threshold_px;
        }
    }

    /// 视口尺寸。
    pub fn viewport(&self) -> (f64, f64) {
        (self.viewport_width, self.viewport_height)
    }

    /// 观测一次元素几何；返回**本次**位移得分（0.0 = 没有非预期位移）。
    ///
    /// 首次观测只记录基线 ⇒ 返回 0。
    pub fn observe(&mut self, id: TrackId, rect: ElementBox) -> f64 {
        self.samples += 1;
        let Some(previous) = self.last.insert(id, rect) else {
            return 0.0;
        };
        let dx = rect.x - previous.x;
        let dy = rect.y - previous.y;
        if dx.abs() < self.threshold_px && dy.abs() < self.threshold_px {
            return 0.0;
        }
        let score = self.score(previous, rect);
        if score > 0.0 {
            self.total += score;
            if score > self.worst {
                self.worst = score;
            }
        }
        score
    }

    /// 累计 CLS（本追踪器看到的总分）。
    pub fn total(&self) -> f64 {
        self.total
    }

    /// 单次最大得分（定位"哪一次最糟"用）。
    pub fn worst(&self) -> f64 {
        self.worst
    }

    /// 已观测次数（判据/日志用；`0` 表示**没有任何观测** ⇒ 结论不可用）。
    pub fn samples(&self) -> usize {
        self.samples
    }

    /// 遗忘某个元素（卸载时调用，避免把"卸载"记成位移）。
    pub fn forget(&mut self, id: TrackId) {
        self.last.remove(&id);
    }

    /// 清空累计（宿主上报后可重置窗口）。
    pub fn reset(&mut self) {
        self.total = 0.0;
        self.worst = 0.0;
        self.samples = 0;
        self.last.clear();
    }

    fn score(&self, before: ElementBox, after: ElementBox) -> f64 {
        let viewport_area = self.viewport_width * self.viewport_height;
        if viewport_area <= 0.0 {
            return 0.0;
        }
        let impact =
            union_area_in_viewport(before, after, self.viewport_width, self.viewport_height)
                / viewport_area;
        if impact <= 0.0 {
            return 0.0;
        }
        let distance = (after.x - before.x).abs().max((after.y - before.y).abs())
            / self.viewport_width.max(self.viewport_height);
        impact * distance
    }
}

fn sanitize(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

/// 两个盒的**并集**与视口相交后的面积。
fn union_area_in_viewport(
    a: ElementBox,
    b: ElementBox,
    viewport_width: f64,
    viewport_height: f64,
) -> f64 {
    let left = a.x.min(b.x).max(0.0);
    let top = a.y.min(b.y).max(0.0);
    let right = a.right().max(b.right()).min(viewport_width);
    let bottom = a.bottom().max(b.bottom()).min(viewport_height);
    let width = (right - left).max(0.0);
    let height = (bottom - top).max(0.0);
    width * height
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 首次观测只建基线 ⇒ 得分 0。
    #[test]
    fn first_observation_is_only_a_baseline() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 800.0);
        let score = tracker.observe(1, ElementBox::new(0.0, 100.0, 200.0, 50.0));
        assert_eq!(score, 0.0);
        assert_eq!(tracker.samples(), 1, "观测次数必须被记录");
    }

    /// ⭐ **亚像素位移不计数**（渲染噪声不是布局跳动）。
    #[test]
    fn subpixel_shifts_are_ignored() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 800.0);
        tracker.observe(1, ElementBox::new(10.0, 10.0, 100.0, 40.0));
        let score = tracker.observe(1, ElementBox::new(10.0, 10.4, 100.0, 40.0));
        assert_eq!(score, 0.0, "0.4px 位移应视为噪声");
        assert_eq!(tracker.total(), 0.0);
    }

    /// 明显位移 ⇒ 得分 = 影响占比 × 位移占比（手算可复核）。
    #[test]
    fn visible_shift_scores_impact_times_distance() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 1000.0);
        // 基线：元素 100×100，位于 (0, 0)
        tracker.observe(1, ElementBox::new(0.0, 0.0, 100.0, 100.0));
        // 下移 100 ⇒ 并集 = 宽 100 × 高 200 = 20000 ⇒ 影响占比 = 20000/1000000 = 0.02
        // 距离占比 = max(0, 100)/1000 = 0.1
        // 得分 = 0.02 × 0.1 = 0.002
        let score = tracker.observe(1, ElementBox::new(0.0, 100.0, 100.0, 100.0));
        assert!((score - 0.002).abs() < 1e-9, "实际得分 {score}");
        assert!((tracker.total() - 0.002).abs() < 1e-9);
        assert!((tracker.worst() - 0.002).abs() < 1e-9);
    }

    /// 视口外的位移不计数（用户看不见）。
    #[test]
    fn shifts_outside_the_viewport_are_not_scored() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 800.0);
        tracker.observe(1, ElementBox::new(0.0, 2000.0, 100.0, 100.0));
        let score = tracker.observe(1, ElementBox::new(0.0, 2100.0, 100.0, 100.0));
        assert_eq!(score, 0.0, "视口外位移不应计分");
    }

    /// 累计与 worst 跟踪；`reset` 清空。
    #[test]
    fn totals_accumulate_and_reset() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 1000.0);
        tracker.observe(1, ElementBox::new(0.0, 0.0, 100.0, 100.0));
        let first = tracker.observe(1, ElementBox::new(0.0, 100.0, 100.0, 100.0));
        let second = tracker.observe(1, ElementBox::new(0.0, 200.0, 100.0, 100.0));
        assert!(second > 0.0 && first > 0.0);
        assert!((tracker.total() - (first + second)).abs() < 1e-9);
        assert_eq!(tracker.worst().max(first), tracker.worst());
        tracker.reset();
        assert_eq!(tracker.total(), 0.0);
        assert_eq!(tracker.samples(), 0);
    }

    /// 卸载（forget）后重新出现 ⇒ 只算基线，不把"卸载/挂载"记成位移。
    #[test]
    fn forget_prevents_unmount_from_being_scored() {
        let mut tracker = LayoutShiftTracker::new(1000.0, 1000.0);
        tracker.observe(1, ElementBox::new(0.0, 0.0, 100.0, 100.0));
        tracker.forget(1);
        let score = tracker.observe(1, ElementBox::new(0.0, 300.0, 100.0, 100.0));
        assert_eq!(score, 0.0, "卸载后重新出现应视为新基线");
    }

    /// 非法视口 ⇒ 不除零、得分为 0（但也**记录观测次数**，便于发现"没在观测"）。
    #[test]
    fn illegal_viewport_yields_zero_score_without_panicking() {
        let mut tracker = LayoutShiftTracker::new(0.0, f64::NAN);
        tracker.observe(1, ElementBox::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(
            tracker.observe(1, ElementBox::new(0.0, 100.0, 10.0, 10.0)),
            0.0
        );
        assert_eq!(tracker.samples(), 2);
    }
}
