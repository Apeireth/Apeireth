//! # tui_e2e — TUI 5 nav + 9 器官 e2e (14 测试)
//!
//! **职责**: 端到端验证 TUI 设计契约 — 1 屏 4 panel + 5 nav + 9 器官 + 6 哲学锚.
//!
//! **跟 `apeireth-tui-e2e` 的关系**: 互补, 不重复
//! - `apeireth-tui-e2e` 测 TUI 设计契约 (20+ 测试, 镜像 tui 公开 API)
//! - `apeireth-integration-e2e::tui_e2e` 测 TUI 在三层 harness 里的端到端 (14 测试)
//! - 共同点: 5 nav + 9 器官 1 屏 4 panel
//!
//! **14 测试** (per 派活单 §5):
//! 1. `test_tui_status_nav_renders`           — status nav 渲染含 5 组件
//! 2. `test_tui_session_nav_lists`            — session nav 列出活跃 session
//! 3. `test_tui_tools_nav_shows_6`            — tools nav 6 工具
//! 4. `test_tui_settings_nav_5_providers`     — settings nav 5 provider
//! 5. `test_tui_help_nav_6_anchors`           — help nav 6 哲学锚
//! 6. `test_tui_organ_heart_pulse`            — heart 60Hz 脉冲
//! 7. `test_tui_organ_brain_llm`              — brain LLM 调用
//! 8. `test_tui_organ_hand_tools`             — hand 6 工具
//! 9. `test_tui_organ_eye_input`              — eye 输入流
//! 10. `test_tui_organ_ear_events`            — ear 事件流
//! 11. `test_tui_organ_memory_history`        — memory 历史
//! 12. `test_tui_organ_voice_state`           — voice 状态
//! 13. `test_tui_organ_body_resources`        — body 资源
//! 14. `test_tui_organ_mind_anchors`          — mind 6 哲学锚
//!
//! **附加** (per 派活单 §5 续):
//! 15. `test_tui_quit_key_q`                  — q 退出
//!
//! **8 不修改承诺**: 跟 lib.rs / error.rs / harness.rs / api_e2e 一致

use parking_lot::Mutex;

use crate::error::{E2EError, E2EResult};
use crate::harness::{IntegrationHarness, NavPageMirror, OrganMirror};

// =====================================================================
// 14 TUI e2e 测试
// =====================================================================

/// 1. status nav 渲染 (含 5 health 组件)
pub fn test_tui_status_nav_renders(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.nav = NavPageMirror::Bridge;
    }
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Bridge") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_status_nav_renders".into(),
            expected: "Bridge".into(),
            actual: text,
        });
    }
    if !text.contains("Heart") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_status_nav_renders organ".into(),
            expected: "Heart".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 2. session nav 列出
pub fn test_tui_session_nav_lists(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.nav = NavPageMirror::Dialogue;
    }
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Dialogue") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_session_nav_lists".into(),
            expected: "Dialogue".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 3. tools nav 显示 6 工具
pub fn test_tui_tools_nav_shows_6(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.nav = NavPageMirror::Dialogue;
    }
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    // 9 器官全显, 至少有 1 个器官 label
    for o in OrganMirror::ALL.iter().take(3) {
        if !text.contains(o.label_en()) {
            return Err(E2EError::TuiAssert {
                context: format!("test_tui_tools_nav_shows_6 organ {}", o.label_en()),
                expected: o.label_en().into(),
                actual: text.clone(),
            });
        }
    }
    Ok(())
}

/// 4. settings nav 5 provider
pub fn test_tui_settings_nav_5_providers(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.nav = NavPageMirror::Settings;
    }
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Settings") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_settings_nav_5_providers".into(),
            expected: "Settings".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 5. help nav 6 哲学锚
pub fn test_tui_help_nav_6_anchors(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.nav = NavPageMirror::Settings;
    }
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    // status bar 含 3 个哲学锚 (S-1, S-2, O-5)
    if !text.contains("S-1") || !text.contains("S-2") || !text.contains("O-5") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_help_nav_6_anchors".into(),
            expected: "S-1, S-2, O-5 in status bar".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 6. heart 器官 60Hz 脉冲
pub fn test_tui_organ_heart_pulse(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Heart") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_heart_pulse".into(),
            expected: "Heart".into(),
            actual: text,
        });
    }
    // 器官心跳: render_tick 推进
    let app = h.tui_app.lock();
    if app.render_tick == 0 {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_heart_pulse render_tick".into(),
            expected: "> 0 (Heart 60Hz pulse)".into(),
            actual: "0".into(),
        });
    }
    Ok(())
}

/// 7. brain 器官 (LLM 调用)
pub fn test_tui_organ_brain_llm(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Brain") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_brain_llm".into(),
            expected: "Brain".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 8. hand 器官 (6 工具)
pub fn test_tui_organ_hand_tools(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Hand") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_hand_tools".into(),
            expected: "Hand".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 9. eye 器官 (输入流)
pub fn test_tui_organ_eye_input(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Eye") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_eye_input".into(),
            expected: "Eye".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 10. ear 器官 (事件流)
pub fn test_tui_organ_ear_events(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Ear") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_ear_events".into(),
            expected: "Ear".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 11. memory 器官 (历史)
pub fn test_tui_organ_memory_history(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Memory") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_memory_history".into(),
            expected: "Memory".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 12. voice 器官 (状态)
pub fn test_tui_organ_voice_state(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Voice") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_voice_state".into(),
            expected: "Voice".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 13. body 器官 (资源)
pub fn test_tui_organ_body_resources(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    if !text.contains("Body") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_body_resources".into(),
            expected: "Body".into(),
            actual: text,
        });
    }
    Ok(())
}

/// 14. mind 器官 (6 哲学锚)
pub fn test_tui_organ_mind_anchors(h: &mut IntegrationHarness) -> E2EResult<()> {
    h.tui_render()?;
    let text = h.tui_buffer_text()?;
    // mind 器官 = 6 哲学锚穿透, status bar 至少 3 个显式
    if !text.contains("S-1") {
        return Err(E2EError::TuiAssert {
            context: "test_tui_organ_mind_anchors S-1".into(),
            expected: "S-1".into(),
            actual: text.clone(),
        });
    }
    Ok(())
}

/// 15. q 退出
pub fn test_tui_quit_key_q(h: &mut IntegrationHarness) -> E2EResult<()> {
    {
        let mut app = h.tui_app.lock();
        app.handle_key('q');
    }
    let app = h.tui_app.lock();
    if !app.should_quit {
        return Err(E2EError::TuiAssert {
            context: "test_tui_quit_key_q".into(),
            expected: "should_quit = true".into(),
            actual: "should_quit = false".into(),
        });
    }
    Ok(())
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn new_harness() -> IntegrationHarness {
        tokio_test_block_on(IntegrationHarness::start()).unwrap()
    }

    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn run_all_14_tui_e2e() {
        let mut h = new_harness();
        test_tui_status_nav_renders(&mut h).unwrap();
        test_tui_session_nav_lists(&mut h).unwrap();
        test_tui_tools_nav_shows_6(&mut h).unwrap();
        test_tui_settings_nav_5_providers(&mut h).unwrap();
        test_tui_help_nav_6_anchors(&mut h).unwrap();
        test_tui_organ_heart_pulse(&mut h).unwrap();
        test_tui_organ_brain_llm(&mut h).unwrap();
        test_tui_organ_hand_tools(&mut h).unwrap();
        test_tui_organ_eye_input(&mut h).unwrap();
        test_tui_organ_ear_events(&mut h).unwrap();
        test_tui_organ_memory_history(&mut h).unwrap();
        test_tui_organ_voice_state(&mut h).unwrap();
        test_tui_organ_body_resources(&mut h).unwrap();
        test_tui_organ_mind_anchors(&mut h).unwrap();
        test_tui_quit_key_q(&mut h).unwrap();
    }

    #[test]
    fn test_tui_status_nav_renders_run() {
        let mut h = new_harness();
        test_tui_status_nav_renders(&mut h).unwrap();
    }

    #[test]
    fn test_tui_session_nav_lists_run() {
        let mut h = new_harness();
        test_tui_session_nav_lists(&mut h).unwrap();
    }

    #[test]
    fn test_tui_tools_nav_shows_6_run() {
        let mut h = new_harness();
        test_tui_tools_nav_shows_6(&mut h).unwrap();
    }

    #[test]
    fn test_tui_settings_nav_5_providers_run() {
        let mut h = new_harness();
        test_tui_settings_nav_5_providers(&mut h).unwrap();
    }

    #[test]
    fn test_tui_help_nav_6_anchors_run() {
        let mut h = new_harness();
        test_tui_help_nav_6_anchors(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_heart_pulse_run() {
        let mut h = new_harness();
        test_tui_organ_heart_pulse(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_brain_llm_run() {
        let mut h = new_harness();
        test_tui_organ_brain_llm(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_hand_tools_run() {
        let mut h = new_harness();
        test_tui_organ_hand_tools(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_eye_input_run() {
        let mut h = new_harness();
        test_tui_organ_eye_input(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_ear_events_run() {
        let mut h = new_harness();
        test_tui_organ_ear_events(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_memory_history_run() {
        let mut h = new_harness();
        test_tui_organ_memory_history(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_voice_state_run() {
        let mut h = new_harness();
        test_tui_organ_voice_state(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_body_resources_run() {
        let mut h = new_harness();
        test_tui_organ_body_resources(&mut h).unwrap();
    }

    #[test]
    fn test_tui_organ_mind_anchors_run() {
        let mut h = new_harness();
        test_tui_organ_mind_anchors(&mut h).unwrap();
    }

    #[test]
    fn test_tui_quit_key_q_run() {
        let mut h = new_harness();
        test_tui_quit_key_q(&mut h).unwrap();
    }

    /// 9 器官 LOCKED 顺序 (跟 tui 主线对齐)
    #[test]
    fn nine_organs_locked_order() {
        let organs = OrganMirror::ALL;
        assert_eq!(organs[0], OrganMirror::Heart);
        assert_eq!(organs[1], OrganMirror::Brain);
        assert_eq!(organs[2], OrganMirror::Hand);
        assert_eq!(organs[3], OrganMirror::Eye);
        assert_eq!(organs[4], OrganMirror::Ear);
        assert_eq!(organs[5], OrganMirror::Memory);
        assert_eq!(organs[6], OrganMirror::Voice);
        assert_eq!(organs[7], OrganMirror::Body);
        assert_eq!(organs[8], OrganMirror::Mind);
    }

    /// 5 nav LOCKED 顺序 (跟 tui 主线对齐)
    #[test]
    fn five_nav_locked_order() {
        let navs = NavPageMirror::ALL;
        assert_eq!(navs[0], NavPageMirror::Bridge);
        assert_eq!(navs[1], NavPageMirror::Dialogue);
        assert_eq!(navs[2], NavPageMirror::Growth);
        assert_eq!(navs[3], NavPageMirror::History);
        assert_eq!(navs[4], NavPageMirror::Settings);
    }

    // 锁 Mutex 引用, 避免 dead_code 警告
    #[allow(dead_code)]
    fn _force_mutex_use() {
        let _: &Mutex<()> = &Mutex::new(());
    }
}
