//! `apeireth-companion::screen_perception` — 连续感知②: 屏幕显著性事件.
//!
//! ## 定位 (主人 2026-08-18: "屏幕感知可以做…很基础的功能, 为后面做铺垫")
//!
//! 连续感知轻层 (非全时录像): **显著性事件** — 窗口切换 / 应用聚焦 / 长时间无操作.
//! 事件喂 [`crate::bus 事件桥`] (System 类) → PerceptionGate 门控 → 她"看见你什么时候在干什么".
//! 终极形态 (愿景): 她看着你干活 → 自己学习理解 → 主动提出帮忙.
//!
//! ## 0 装 PASS
//!
//! - [`ScreenEventSource`] trait 口已备 (Windows 前台窗口轮询 = 实现点; 跨平台: macOS/Linux 对应 API).
//! - 默认 [`NoopScreenSource`] 诚实返回空 (未接不假装看见).
//! - Mock 走通显著性判定机制.

/// 屏幕显著性事件类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEventKind {
    /// 前台窗口切换 (她注意到你换了窗口).
    WindowSwitch,
    /// 应用聚焦 (长时间停在某应用 = 深度投入).
    AppFocus,
    /// 空闲开始 (长时间无操作).
    IdleStart,
    /// 空闲结束 (你回来了).
    IdleResume,
}

/// 一条屏幕显著性事件.
#[derive(Debug, Clone)]
pub struct ScreenEvent {
    pub kind: ScreenEventKind,
    /// 应用/窗口标识 (如 "cargo", "vscode").
    pub app: String,
    pub at_ms: i64,
}

/// 屏幕事件源 trait 口 (实现点: Windows GetForegroundWindow 轮询等).
pub trait ScreenEventSource: Send + Sync + std::fmt::Debug {
    /// 轮询一次: 返回自上次轮询以来的显著性事件.
    fn poll(&mut self) -> Vec<ScreenEvent>;
}

/// 默认实现: 未接 → 空 (0 装: 不假装看见屏幕).
#[derive(Debug, Default)]
pub struct NoopScreenSource;

impl ScreenEventSource for NoopScreenSource {
    fn poll(&mut self) -> Vec<ScreenEvent> {
        Vec::new()
    }
}

/// 屏幕感知器 (确定性: 事件透传 + 显著性打分).
#[derive(Debug)]
pub struct ScreenPerception {
    source: Box<dyn ScreenEventSource>,
    /// 空闲检测: 距上次事件超过 idle_threshold_ms → IdleStart.
    pub idle_threshold_ms: i64,
    last_event_ms: Option<i64>,
}

impl ScreenPerception {
    pub fn new(source: Box<dyn ScreenEventSource>) -> Self {
        Self {
            source,
            idle_threshold_ms: 5 * 60 * 1000, // 5 分钟无操作 = 空闲 (待拟合)
            last_event_ms: None,
        }
    }

    /// 轮询: 源事件 + 空闲检测 → 显著性事件流.
    pub fn poll_events(&mut self) -> Vec<ScreenEvent> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut out = self.source.poll();
        // 空闲检测 (确定性, 不依赖源)
        if let Some(last) = self.last_event_ms {
            if now - last > self.idle_threshold_ms {
                out.push(ScreenEvent {
                    kind: ScreenEventKind::IdleStart,
                    app: "system".into(),
                    at_ms: now,
                });
            }
        }
        if !out.is_empty() {
            self.last_event_ms = Some(now);
        }
        out
    }

    /// 显著性打分 ``[0,1]``: 她该多在意这条事件.
    /// WindowSwitch 低 (你只是切窗), AppFocus 中, IdleResume 高 (你回来了).
    pub fn significance(&self, e: &ScreenEvent) -> f64 {
        match e.kind {
            ScreenEventKind::WindowSwitch => 0.2,
            ScreenEventKind::AppFocus => 0.5,
            ScreenEventKind::IdleStart => 0.3,
            ScreenEventKind::IdleResume => 0.8,
        }
    }

    /// 值得感知 (喂 PerceptionGate 前的最小过滤).
    pub fn should_perceive(&self, e: &ScreenEvent) -> bool {
        self.significance(e) >= 0.3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性 Mock 源.
    #[derive(Debug)]
    struct MockSource {
        events: Vec<ScreenEvent>,
    }

    impl ScreenEventSource for MockSource {
        fn poll(&mut self) -> Vec<ScreenEvent> {
            std::mem::take(&mut self.events)
        }
    }

    fn ev(kind: ScreenEventKind, app: &str) -> ScreenEvent {
        ScreenEvent {
            kind,
            app: app.into(),
            at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    #[test]
    fn events_pass_through_with_significance() {
        let mut p = ScreenPerception::new(Box::new(MockSource {
            events: vec![ev(ScreenEventKind::AppFocus, "vscode")],
        }));
        let events = p.poll_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].app, "vscode");
        assert!(p.should_perceive(&events[0]));
        assert!(!p.should_perceive(&ev(ScreenEventKind::WindowSwitch, "x")));
    }

    #[test]
    fn idle_detection_after_threshold() {
        let mut p = ScreenPerception::new(Box::new(MockSource { events: Vec::new() }));
        p.idle_threshold_ms = 1000;
        // 模拟: 设置 last_event_ms 到过去
        p.last_event_ms = Some(chrono::Utc::now().timestamp_millis() - 5000);
        let events = p.poll_events();
        assert!(events.iter().any(|e| e.kind == ScreenEventKind::IdleStart));
    }

    #[test]
    fn noop_source_is_honest() {
        let mut p = ScreenPerception::new(Box::new(NoopScreenSource));
        assert!(p.poll_events().is_empty(), "未接不假装看见屏幕");
    }

    #[test]
    fn significance_ranking() {
        let p = ScreenPerception::new(Box::new(NoopScreenSource));
        let idle = ev(ScreenEventKind::IdleResume, "s");
        let switch = ev(ScreenEventKind::WindowSwitch, "s");
        assert!(p.significance(&idle) > p.significance(&switch));
        assert_eq!(p.significance(&switch), 0.2);
    }
}
