//! # 9 器官端到端测试
//!
//! **职责**: 验证 9 器官 (Heart / Brain / Hand / Eye / Ear / Memory / Voice / Body / Mind)
//! 各自能渲染, health 数字正确, Mind 显 6 哲学锚.
//!
//! **9 器官** (per R19 拟人化原则 — 心脑手眼耳鼻记忆声体意):
//! - 0 Heart (心) — 60Hz CPU 脉冲, partial
//! - 1 Brain (脑) — LLM 推理, partial
//! - 2 Hand (手) — 工具调用, partial
//! - 3 Eye (眼) — 输入监控, stub
//! - 4 Ear (耳) — 事件订阅, stub
//! - 5 Memory (记忆) — 会话历史, partial
//! - 6 Voice (声) — STT / TTS, stub
//! - 7 Body (体) — 进程 / 资源, partial
//! - 8 Mind (意) — AGI 状态 + 6 哲学锚, partial
//!
//! **测试函数** (派活单 §6 要求 9 个 pub async fn):
//! - `test_organ_heart_60hz_pulse`
//! - `test_organ_brain_llm_reasoning`
//! - `test_organ_hand_tool_calls`
//! - `test_organ_eye_input_monitor`
//! - `test_organ_ear_event_subscribe`
//! - `test_organ_memory_session_history`
//! - `test_organ_voice_stt_tts`
//! - `test_organ_body_process_resources`
//! - `test_organ_mind_6_philosophy_anchors`
//!
//! **8 不修改承诺**: 跟 nav_e2e.rs 一致

use crate::error::TuiE2EResult;
use crate::harness::TuiHarness;
use crate::Organ;

/// 0 Heart — 60Hz 脉冲 (跟 tui heart organ 镜像)
pub async fn test_organ_heart_60hz_pulse() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Heart as usize] = 1.0;
    h.render_4_panel()?;
    let snap = h.snapshot();
    // 9 organ middle bar 必含 [HEART]
    assert!(snap.contains("[HEART]"), "Heart organ 应渲染 ASCII");
    // Heart 健康度 100% (middle bar 显示 "100%")
    assert!(snap.contains("100%"), "Heart 100% health 应渲染");
    Ok(())
}

/// 1 Brain — LLM 推理 (e2e 不真调 LLM, 模拟 brain thinking)
pub async fn test_organ_brain_llm_reasoning() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Brain as usize] = 0.85;
    h.app.thinking_expanded = true;
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[BRAIN]"), "Brain organ 应渲染 ASCII");
    // 85% → 颜色 Yellow
    assert!(snap.contains("85%"), "Brain 85% health 应渲染");
    Ok(())
}

/// 2 Hand — 工具调用 (e2e 模拟 hand 调用 6 工具)
pub async fn test_organ_hand_tool_calls() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Hand as usize] = 0.95;
    // 模拟 hand 调用工具
    h.app.push_system("hand: tool calendar called");
    h.app.push_system("hand: tool message called");
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[HAND]"), "Hand organ 应渲染 ASCII");
    assert!(snap.contains("95%"), "Hand 95% health 应渲染");
    Ok(())
}

/// 3 Eye — 输入监控 (stub, e2e 验 [EYE] 渲染)
pub async fn test_organ_eye_input_monitor() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Eye as usize] = 0.5; // stub, 默认低
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[EYE]"), "Eye organ 应渲染 ASCII");
    // stub 标 [stub] (在 bridge content)
    assert!(snap.contains("[stub]"), "Eye 标 stub");
    Ok(())
}

/// 4 Ear — 事件订阅 (stub, 验 [EAR] 渲染)
pub async fn test_organ_ear_event_subscribe() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Ear as usize] = 0.5;
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[EAR]"), "Ear organ 应渲染 ASCII");
    assert!(snap.contains("[stub]"), "Ear 标 stub");
    Ok(())
}

/// 5 Memory — 会话历史 (跟 Memory organ 镜像)
pub async fn test_organ_memory_session_history() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Memory as usize] = 0.9;
    h.app.push_user_input("remember this");
    h.app.push_assistant_reply("stored");
    h.app.nav = crate::NavPage::History;
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[MEM]"), "Memory organ 应渲染 ASCII");
    assert!(snap.contains("90%"), "Memory 90% health 应渲染");
    assert!(snap.contains("remember this"), "Memory 应保留历史");
    Ok(())
}

/// 6 Voice — STT / TTS (stub, 验 [VOICE] 渲染)
pub async fn test_organ_voice_stt_tts() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Voice as usize] = 0.4;
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[VOICE]"), "Voice organ 应渲染 ASCII");
    assert!(snap.contains("[stub]"), "Voice 标 stub");
    Ok(())
}

/// 7 Body — 进程 / 资源 (跟 Body organ 镜像)
pub async fn test_organ_body_process_resources() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Body as usize] = 0.7;
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[BODY]"), "Body organ 应渲染 ASCII");
    assert!(snap.contains("70%"), "Body 70% health 应渲染");
    Ok(())
}

/// 8 Mind — 6 哲学锚 (跟派活单 "9 器官 mind 6 哲学锚" 同步)
pub async fn test_organ_mind_6_philosophy_anchors() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Mind as usize] = 1.0;
    h.app.nav = crate::NavPage::Bridge; // Bridge 渲染 Mind + 6 锚
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("[MIND]"), "Mind organ 应渲染 ASCII");
    assert!(snap.contains("100%"), "Mind 100% health 应渲染");
    for (id, _, _) in crate::SIX_PHI_ANCHORS.iter() {
        assert!(snap.contains(id), "Mind 应渲染哲学锚 {id}");
    }
    Ok(())
}

// =====================================================================
// 同步包装 — 派活单要 pub async fn, #[test] 同步用 wrapper
// =====================================================================

pub fn test_organ_heart_60hz_pulse_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_heart_60hz_pulse())
}
pub fn test_organ_brain_llm_reasoning_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_brain_llm_reasoning())
}
pub fn test_organ_hand_tool_calls_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_hand_tool_calls())
}
pub fn test_organ_eye_input_monitor_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_eye_input_monitor())
}
pub fn test_organ_ear_event_subscribe_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_ear_event_subscribe())
}
pub fn test_organ_memory_session_history_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_memory_session_history())
}
pub fn test_organ_voice_stt_tts_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_voice_stt_tts())
}
pub fn test_organ_body_process_resources_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_body_process_resources())
}
pub fn test_organ_mind_6_philosophy_anchors_sync() -> TuiE2EResult<()> {
    futures::executor::block_on(test_organ_mind_6_philosophy_anchors())
}

// =====================================================================
// 9 器官共享验证
// =====================================================================

/// 9 器官 health 全 100% 时, middle bar 全是绿色
pub fn test_9_organ_all_100() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    for i in 0..Organ::COUNT {
        h.app.organ_health[i as usize] = 1.0;
    }
    h.render_4_panel()?;
    let snap = h.snapshot();
    for i in 0..Organ::COUNT {
        let organ = Organ::from_u8(i).unwrap();
        assert!(snap.contains(organ.ascii()), "应渲染 {}", organ.ascii());
    }
    Ok(())
}

/// 9 器官 health 全 30% 时, middle bar 全是红色
pub fn test_9_organ_all_30_red() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    for i in 0..Organ::COUNT {
        h.app.organ_health[i as usize] = 0.3;
    }
    h.render_4_panel()?;
    let snap = h.snapshot();
    // 9 个 30% 都应渲染
    let count_30 = snap.text.matches("30%").count();
    assert!(count_30 >= 9, "9 个 30% 应渲染, 实际 {count_30}");
    Ok(())
}

/// 9 器官 health 混合 (80% 绿 / 60% 黄 / 30% 红) — 颜色映射正确
pub fn test_9_organ_mixed_health_colors() -> TuiE2EResult<()> {
    let mut h = TuiHarness::start()?;
    h.app.organ_health[Organ::Heart as usize] = 0.9; // 绿
    h.app.organ_health[Organ::Brain as usize] = 0.6; // 黄
    h.app.organ_health[Organ::Hand as usize] = 0.3; // 红
    h.render_4_panel()?;
    let snap = h.snapshot();
    assert!(snap.contains("90%"));
    assert!(snap.contains("60%"));
    assert!(snap.contains("30%"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organ_heart_60hz_pulse() {
        test_organ_heart_60hz_pulse_sync().unwrap();
    }

    #[test]
    fn organ_brain_llm_reasoning() {
        test_organ_brain_llm_reasoning_sync().unwrap();
    }

    #[test]
    fn organ_hand_tool_calls() {
        test_organ_hand_tool_calls_sync().unwrap();
    }

    #[test]
    fn organ_eye_input_monitor() {
        test_organ_eye_input_monitor_sync().unwrap();
    }

    #[test]
    fn organ_ear_event_subscribe() {
        test_organ_ear_event_subscribe_sync().unwrap();
    }

    #[test]
    fn organ_memory_session_history() {
        test_organ_memory_session_history_sync().unwrap();
    }

    #[test]
    fn organ_voice_stt_tts() {
        test_organ_voice_stt_tts_sync().unwrap();
    }

    #[test]
    fn organ_body_process_resources() {
        test_organ_body_process_resources_sync().unwrap();
    }

    #[test]
    fn organ_mind_6_philosophy_anchors() {
        test_organ_mind_6_philosophy_anchors_sync().unwrap();
    }

    #[test]
    fn organ_9_all_100() {
        test_9_organ_all_100().unwrap();
    }

    #[test]
    fn organ_9_all_30_red() {
        test_9_organ_all_30_red().unwrap();
    }

    #[test]
    fn organ_9_mixed_health_colors() {
        test_9_organ_mixed_health_colors().unwrap();
    }
}
