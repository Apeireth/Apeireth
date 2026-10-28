//! ⭐ **窗口化（虚拟列表）原语** —— 台账 L1-G5 的第一片。
//!
//! ## 为什么归引擎
//!
//! 长会话 / 长列表要能滚动，就必须「只渲染可见窗口」
//! ⇒ ⇒ 但「当前滚动位置对应哪几项、第一项要偏移多少」是**纯计算 + 与渲染帧耦合**的事
//! ⇒ ⇒ 让每个列表各写一遍，必然各错一种（差一项、错位、滚到底不显示最后一行）。
//!
//! ## 本模块的边界（0 装纪律）
//!
//! 这里**只做窗口计算**：给定「每项高度 + 滚动位置 + 视口高度」⇒ 算出
//! **可见索引区间**与**窗口相对滚动容器的偏移**
//! ⇒ ⇒ 真正的「只挂载这几项」由渲染器/上层完成（**当前未接线**，见四级口径）。
//!
//! 引擎**不做排版**（ADR-0015）：每项高度由测量（[`crate::dom::Dom::bounding_box`]）或
//! 预估策略提供，本模块不猜。

/// 窗口计划：**可见区间**（闭区间）+ 窗口相对滚动容器的纵向偏移（CSS 像素）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VirtualPlan {
    /// 第一个可见项的下标（已含 overscan）。
    pub first: usize,
    /// 最后一个可见项的下标（**闭区间**；空列表时 `last < first`）。
    pub last: usize,
    /// 窗口相对于滚动容器内容顶部的偏移（CSS 像素）
    /// ⇒ 上层把它作为窗口容器的 `translateY` / `top`。
    pub offset: f64,
    /// 窗口内项数（**显式字段**：空窗口时 `first/last` 无意义，不能靠它们推导）。
    pub count: usize,
}

impl VirtualPlan {
    /// 窗口内的项数（空窗口 ⇒ 0）。
    pub fn len(&self) -> usize {
        self.count
    }

    /// 是否为空窗口。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 把每项高度**净化**为「非负有限值」。
///
/// ⚠️ 非有限值（NaN/∞）会让后续所有累加变成 NaN ⇒ ⇒ 整条窗口计算静默失效
/// ⇒ ⇒ 这里显式替换为 0，并由 [`plan_virtual_window`] 把零高度项当作**零占用**处理
/// （仍可被渲染，只是不占空间）。
fn sanitized(heights: &[f64]) -> Vec<f64> {
    heights
        .iter()
        .map(|height| {
            if height.is_finite() && *height > 0.0 {
                *height
            } else {
                0.0
            }
        })
        .collect()
}

/// 计算可见窗口。
///
/// - `item_heights`：每项高度（CSS 像素；非法值按 0 处理）；
/// - `scroll_top`：滚动容器当前滚动位置（CSS 像素；负数按 0 处理）；
/// - `viewport_height`：滚动容器可视高度（CSS 像素；非法/负数按 0 处理）；
/// - `overscan`：上下各多渲染几项（减少快速滚动时的空白）。
///
/// ## 语义
///
/// 1. 可见判据：项与 `[scroll_top, scroll_top + viewport_height)` **有交叠**即可见；
/// 2. 零高度项不占空间 ⇒ 只有「视口高度 > 0 且滚动位置落在其累计位置」时才可能可见
///    （本实现对零高度项采取「不主动纳入」策略：不可见，避免窗口被大量零高度项撑爆）；
/// 3. `overscan` 在两端**夹住**列表边界；
/// 4. 滚动超出内容末尾 ⇒ 窗口落在最后若干项上（**不是空窗口**）；
/// 5. 内容总高度为 0 或视口高度为 0 ⇒ **空窗口**。
pub fn plan_virtual_window(
    item_heights: &[f64],
    scroll_top: f64,
    viewport_height: f64,
    overscan: usize,
) -> VirtualPlan {
    let heights = sanitized(item_heights);
    let empty = VirtualPlan {
        first: 0,
        last: 0,
        offset: 0.0,
        count: 0,
    };
    if heights.is_empty() {
        return empty;
    }
    let scroll = if scroll_top.is_finite() && scroll_top > 0.0 {
        scroll_top
    } else {
        0.0
    };
    let viewport = if viewport_height.is_finite() && viewport_height > 0.0 {
        viewport_height
    } else {
        0.0
    };
    if viewport == 0.0 {
        return empty;
    }

    // 前缀和：prefix[i] = 前 i 项的总高度。
    let mut prefix = Vec::with_capacity(heights.len() + 1);
    prefix.push(0.0_f64);
    for height in &heights {
        let next = prefix.last().copied().unwrap_or(0.0) + *height;
        prefix.push(next);
    }
    let total = prefix[prefix.len() - 1];

    // 滚动超出内容末尾 ⇒ 夹到末尾（窗口落在最后若干项，而不是空窗口）。
    let scroll = scroll.min((total - viewport).max(0.0));
    let window_top = scroll;
    let window_bottom = scroll + viewport;

    // 找第一个「底部 > 窗口顶部」的项（零高度项底部 == 顶部 ⇒ 不算）。
    let mut first = heights.len();
    for index in 0..heights.len() {
        if prefix[index + 1] > window_top && heights[index] > 0.0 {
            first = index;
            break;
        }
    }
    if first == heights.len() {
        // 窗口顶部已在内容之后（理论上已被 clamp 排除）⇒ 落在最后一项。
        first = heights.len().saturating_sub(1);
    }

    // 找最后一个「顶部 < 窗口底部」的项（闭区间）。
    let mut last = first;
    for index in first..heights.len() {
        if prefix[index] < window_bottom && heights[index] > 0.0 {
            last = index;
        }
    }

    // overscan 夹到边界。
    let first = first.saturating_sub(overscan);
    let last = (last + overscan).min(heights.len() - 1);

    VirtualPlan {
        first,
        last,
        offset: prefix[first],
        count: last - first + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空列表 ⇒ 空窗口（`last < first`），`len()` 为 0。
    #[test]
    fn empty_list_yields_empty_window() {
        let plan = plan_virtual_window(&[], 0.0, 500.0, 2);
        assert!(plan.is_empty());
        assert_eq!(plan.len(), 0);
        assert_eq!(plan.offset, 0.0);
    }

    /// 视口高度为 0 或非法 ⇒ 空窗口（不猜）。
    #[test]
    fn zero_or_illegal_viewport_yields_empty_window() {
        for viewport in [0.0, -10.0, f64::NAN, f64::INFINITY] {
            let plan = plan_virtual_window(&[50.0; 10], 0.0, viewport, 0);
            assert!(plan.is_empty(), "viewport={viewport} 应为空窗口");
        }
    }

    /// 典型情形：每项 50、视口 200、滚动 0 ⇒ 可见 4 项（0..=3）。
    #[test]
    fn basic_window_covers_exactly_the_viewport() {
        let heights = vec![50.0; 10];
        let plan = plan_virtual_window(&heights, 0.0, 200.0, 0);
        assert_eq!((plan.first, plan.last), (0, 3));
        assert_eq!(plan.len(), 4);
        assert_eq!(plan.offset, 0.0);
    }

    /// 滚动到中间 ⇒ 窗口跟着移动，`offset` = 第一项的累计位置。
    #[test]
    fn scrolling_moves_the_window_and_its_offset() {
        let heights = vec![50.0; 10];
        let plan = plan_virtual_window(&heights, 120.0, 200.0, 0);
        // 滚动 120 ⇒ 第 2 项（100..150）与窗口相交 ⇒ first=2
        assert_eq!(plan.first, 2);
        assert_eq!(plan.last, 6); // 窗口底部 320 ⇒ 第 6 项（300..350）相交
        assert_eq!(plan.offset, 100.0);
    }

    /// ⭐ **滚动超出末尾 ⇒ 窗口落在最后若干项**（不是空窗口 —— 常见的"滚到底白屏"缺陷）。
    #[test]
    fn overscroll_clamps_to_the_tail_instead_of_going_blank() {
        let heights = vec![50.0; 10];
        let plan = plan_virtual_window(&heights, 9_999.0, 200.0, 0);
        assert!(!plan.is_empty(), "超出末尾不得变成空窗口");
        assert_eq!(plan.last, 9, "必须包含最后一项");
        assert_eq!(plan.first, 6, "窗口应从能填满视口的位置开始");
        assert_eq!(plan.offset, 300.0);
    }

    /// overscan 在两端**夹住**边界（不得越界/下溢）。
    #[test]
    fn overscan_is_clamped_at_both_ends() {
        let heights = vec![50.0; 3];
        let plan = plan_virtual_window(&heights, 0.0, 100.0, 10);
        assert_eq!(plan.first, 0, "前向 overscan 不得下溢");
        assert_eq!(plan.last, 2, "后向 overscan 不得越界");
    }

    /// 非有限/负数高度不会把整条计算变成 NaN。
    #[test]
    fn illegal_heights_do_not_poison_the_plan() {
        let heights = vec![50.0, f64::NAN, 50.0, -10.0, 50.0];
        let plan = plan_virtual_window(&heights, 0.0, 100.0, 0);
        assert!(plan.offset.is_finite(), "offset 不得为 NaN");
        assert!(plan.first <= plan.last);
        // 50+0+50 = 100 ⇒ 前三个（含零高度项）覆盖视口
        assert_eq!(plan.first, 0);
        assert_eq!(plan.last, 2);
    }

    /// 分数高度：窗口边界按**交叠**判定（不是整除）。
    #[test]
    fn fractional_heights_use_overlap_semantics() {
        let heights = vec![33.3, 33.3, 33.3, 33.3];
        let plan = plan_virtual_window(&heights, 40.0, 40.0, 0);
        // 40..80 与第 1 项(33.3..66.6) 相交 ⇒ first=1；与第 2 项(66.6..99.9) 相交 ⇒ last=2
        assert_eq!((plan.first, plan.last), (1, 2));
        assert!((plan.offset - 33.3).abs() < 1e-9);
    }
}
