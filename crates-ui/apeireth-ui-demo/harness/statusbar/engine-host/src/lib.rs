//! 状态条面对照的 wasm 宿主（**独立 crate**）。
//!
//! ## 诚实边界
//!
//! - 本 crate 是**证据用**的对照宿主（形状照 `crates-ui/evidence/p5-b/harness/`）
//!   ⇒ **不是** `crates-ui` workspace 的成员 ⇒ 不进 CI 门禁、不进 deny 图。
//! - `wasm_app` 整体被 `#[cfg(target_arch = "wasm32")]` 隔离 ⇒ **native 编译
//!   不参与** ⇒ 它的行为**未经 native 侧验证**（浏览器内的验证由本面的对照
//!   harness 承担：像素 / DOM / 逐帧）。
//! - **真后端未接线**：数据是**固定 fixture**（引擎不碰 HTTP / SSE 契约）。

pub mod fixture;
pub mod fixture_generated;

#[cfg(target_arch = "wasm32")]
pub mod wasm_app;

#[cfg(test)]
mod tests {
    //! 固定数据的**结构**断言（native 侧；证明两侧喂的是同一份数据的形状）。

    use super::*;

    #[test]
    fn the_fixture_has_cases_and_each_case_has_four_cells() {
        let cases = fixture_generated::status_bar_cases();
        assert!(!cases.is_empty(), "固定数据不得为空");
        for case in &cases {
            assert_eq!(case.items.len(), 4, "用例 {} 的格数", case.id);
        }
    }

    #[test]
    fn the_fixture_covers_every_tone_and_badge_variant() {
        use apeireth_ui_kit::status_bar::{StatusBadgeVariant, StatusTone};

        let cases = fixture_generated::status_bar_cases();
        let tones: Vec<StatusTone> = cases
            .iter()
            .flat_map(|case| case.items.iter().map(|cell| cell.tone))
            .collect();
        assert!(tones.contains(&StatusTone::Quiet), "覆盖安静态");
        assert!(tones.contains(&StatusTone::Warn), "覆盖待完善态");
        assert!(tones.contains(&StatusTone::Danger), "覆盖危险态");

        let variants: Vec<StatusBadgeVariant> =
            cases.iter().map(|case| case.badge_variant).collect();
        for variant in [
            StatusBadgeVariant::Amber,
            StatusBadgeVariant::Green,
            StatusBadgeVariant::Blue,
            StatusBadgeVariant::Danger,
            StatusBadgeVariant::Neutral,
            StatusBadgeVariant::Dim,
        ] {
            assert!(variants.contains(&variant), "覆盖徽标变体 {variant:?}");
        }
    }

    #[test]
    fn the_fixture_covers_clickable_and_simulated_cells() {
        let cases = fixture_generated::status_bar_cases();
        assert!(
            cases
                .iter()
                .any(|case| case.items.iter().any(|cell| cell.clickable)),
            "覆盖可点的格"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.items.iter().any(|cell| !cell.clickable)),
            "覆盖不可点的格"
        );
        assert!(
            cases
                .iter()
                .any(|case| case.items.iter().any(|cell| cell.sim)),
            "覆盖 SIM 标注"
        );
    }
}
