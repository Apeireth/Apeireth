use apeireth_companion::emotion::Pad;
use apeireth_storage::memory_v2::MemoryItem;
use chrono::Utc;
use std::time::Instant;
use super::theme::{Theme, ThemeStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    /// 0 舰桥 (Bridge, ΣΚΟΠΗ) — 9 器官仪表盘与系统全景
    Bridge = 0,
    /// 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ) — 伴侣流式交互与思考链
    Dialogue = 1,
    /// 2 生长 (Growth, ΑΥΞΗΣΙΣ) — 知识沉淀与能力进化
    Growth = 2,
    /// 3 历史 (History, ΙΣΤΟΡΙΑ) — 审计区块链与会话账本
    History = 3,
    /// 4 设置 (Settings, ΤΑΞΙΣ) — 主题与系统参数
    Settings = 4,
}

impl NavPage {
    pub fn next(self) -> Self {
        match self {
            NavPage::Bridge => NavPage::Dialogue,
            NavPage::Dialogue => NavPage::Growth,
            NavPage::Growth => NavPage::History,
            NavPage::History => NavPage::Settings,
            NavPage::Settings => NavPage::Bridge,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            NavPage::Bridge => NavPage::Settings,
            NavPage::Dialogue => NavPage::Bridge,
            NavPage::Growth => NavPage::Dialogue,
            NavPage::History => NavPage::Growth,
            NavPage::Settings => NavPage::History,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            NavPage::Bridge => "0 舰桥 ΣΚΟΠΗ",
            NavPage::Dialogue => "1 对话 ΔΙΑΛΟΓΟΣ",
            NavPage::Growth => "2 生长 ΑΥΞΗΣΙΣ",
            NavPage::History => "3 历史 ΙΣΤΟΡΙΑ",
            NavPage::Settings => "4 设置 ΤΑΞΙΣ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrganCard {
    pub name: &'static str,
    pub metaphor: &'static str,
    pub primary: &'static str,
    pub health: f64,
}


#[derive(Debug, Clone)]
pub struct ChatMessageItem {
    pub role: String,
    pub content: String,
    pub cot: Option<String>,
    pub pad: Pad,
    pub tokens: usize,
    pub audit_hash: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_page: NavPage,
    pub theme: Theme,
    pub theme_transition: Option<(ThemeStyle, ThemeStyle, Instant)>, // (from, to, start)
    pub thinking_expanded: bool, // Ctrl+O toggle
    pub input_buffer: String,
    pub session_id: String,
    pub is_thinking: bool,

    // Dialogue State
    pub messages: Vec<ChatMessageItem>,
    pub message_scroll: usize,

    // 9 Organs
    pub organs: Vec<OrganCard>,

    // Emotion & Bio-drive
    pub current_pad: Pad,
    pub sleep_drive: f64,

    // ACT-R Memory
    pub memory_items: Vec<MemoryItem>,
    pub memory_selected: usize,

    // Audit Blockchain
    pub audit_chain_length: usize,
    pub audit_integrity_ok: bool,

    // Status Message
    pub status_message: Option<(String, i64)>,
}

impl AppState {
    pub fn new(session_id: String) -> Self {
        Self {
            current_page: NavPage::Bridge,
            theme: Theme::Archaic,
            theme_transition: None,
            thinking_expanded: false,
            input_buffer: String::new(),
            session_id,
            is_thinking: false,

            messages: vec![
                ChatMessageItem {
                    role: "system".into(),
                    content: "Apeireth 2.0 Living Companion OS Online. All 9 organs active.".into(),
                    cot: None,
                    pad: Pad::default(),
                    tokens: 24,
                    audit_hash: "0000000000000000".into(),
                    timestamp_ms: Utc::now().timestamp_millis(),
                }
            ],
            message_scroll: 0,

            organs: vec![
                OrganCard { name: "眼 Eye", metaphor: "Win32 GDI 物理屏幕", primary: "1707x1067 RGB", health: 1.0 },
                OrganCard { name: "耳 Ear", metaphor: "WebAudio 全双工", primary: "VAD 能量实时探测", health: 1.0 },
                OrganCard { name: "手 Hand", metaphor: "MCP 外部工具总线", primary: "JSON-RPC 2024-11-05", health: 1.0 },
                OrganCard { name: "脑 Brain", metaphor: "MiniMax + ACT-R 混合内核", primary: "CoT 认知推理活跃", health: 1.0 },
                OrganCard { name: "口 Voice", metaphor: "TTS 声学输出", primary: "低延迟端到端", health: 1.0 },
                OrganCard { name: "心 Heart", metaphor: "PAD 情绪与 Plutchik", primary: "三维向量动力学", health: 0.95 },
                OrganCard { name: "忆 Memory", metaphor: "ACT-R SQLite WAL 池", primary: "动态激活度遗忘衰减", health: 1.0 },
                OrganCard { name: "身 Body", metaphor: "Windows Job 沙箱", primary: "资源限制与反越狱", health: 1.0 },
                OrganCard { name: "神 Mind", metaphor: "5-Gate 宪政守护", primary: "不可篡改 SHA-256 账本", health: 1.0 },
            ],

            current_pad: Pad { pleasure: 0.66, arousal: 0.33, dominance: 0.60 },
            sleep_drive: 0.28,

            memory_items: Vec::new(),
            memory_selected: 0,

            audit_chain_length: 1,
            audit_integrity_ok: true,

            status_message: Some(("欢迎使用 Apeireth 2.0 伴侣系统！按 0-4 或 Tab 切换页面，按 t 切换主题。".into(), Utc::now().timestamp_millis())),
        }
    }

    pub fn toggle_theme(&mut self) {
        let from_style = self.current_style();
        self.theme = self.theme.toggle();
        let to_style = ThemeStyle::of(self.theme);
        self.theme_transition = Some((from_style, to_style, Instant::now()));
        self.set_status(format!("已切换主题至 {}", self.theme.display_label()));
    }

    pub fn current_style(&self) -> ThemeStyle {
        if let Some((from, to, start)) = self.theme_transition {
            let elapsed = start.elapsed().as_millis() as f64;
            let progress = elapsed / 200.0;
            if progress >= 1.0 {
                to
            } else {
                ThemeStyle::interpolate(from, to, progress)
            }
        } else {
            ThemeStyle::of(self.theme)
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Utc::now().timestamp_millis()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        let mut state = AppState::new("test_session_123".into());
        assert_eq!(state.current_page, NavPage::Bridge);
        assert_eq!(state.theme, Theme::Archaic);
        state.toggle_theme();
        assert_eq!(state.theme, Theme::Era);
    }
}
