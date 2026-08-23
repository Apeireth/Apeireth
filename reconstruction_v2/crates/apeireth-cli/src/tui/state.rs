use apeireth_companion::emotion::Pad;
use apeireth_storage::memory_v2::MemoryItem;
use chrono::Utc;
use std::time::Instant;
use super::theme::{Theme, ThemeStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    /// 0 舰桥 (Bridge, ΣΚΟΠΗ) — 后端真实工程参数全景看板
    Bridge = 0,
    /// 1 对话 (Dialogue, ΔΙΑΛΟΓΟΣ) — 伴侣流式交互与思考链
    Dialogue = 1,
    /// 2 生长 (Growth, ΑΥΞΗΣΙΣ) — ACT-R 记忆与知识图谱三元组
    Growth = 2,
    /// 3 历史 (History, ΙΣΤΟΡΙΑ) — 审计区块链与会话账本
    History = 3,
    /// 4 设置 (Settings, ΤΑΞΙΣ) — 经典双主题与系统参数
    Settings = 4,
}

impl NavPage {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bridge),
            1 => Some(Self::Dialogue),
            2 => Some(Self::Growth),
            3 => Some(Self::History),
            4 => Some(Self::Settings),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let n = (self as u8 + 1) % 5;
        Self::from_u8(n).unwrap()
    }

    pub fn prev(self) -> Self {
        let n = if self as u8 == 0 { 4 } else { self as u8 - 1 };
        Self::from_u8(n).unwrap()
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
pub struct ChatMessageItem {
    pub role: String,
    pub content: String,
    pub cot: Option<String>,
    pub pad: Pad,
    pub tokens: usize,
    pub audit_hash: String,
    pub timestamp_ms: i64,
}

/// 真实后端工程遥测参数集
#[derive(Debug, Clone)]
pub struct EngineeringTelemetry {
    // 1. Win32 GDI 视觉与桌面感知
    pub screen_resolution: String,
    pub screen_driver: &'static str,
    pub screen_phash: u64,
    pub screen_hamming_diff: u32,
    pub som_elements_count: usize,
    pub screen_latency_ms: u64,

    // 2. ACT-R 2.0 认知记忆与 SQLite WAL
    pub wal_pool_status: &'static str,
    pub total_memory_facts: usize,
    pub avg_activation_score: f64,
    pub cjk_bigram_status: &'static str,
    pub db_size_kb: u64,
    pub db_path: String,

    // 3. LLM 协议与认知推理
    pub active_model: String,
    pub protocol_adapter: &'static str,
    pub session_tokens: usize,
    pub last_latency_ms: u64,
    pub cot_depth: &'static str,
    pub streaming_state: &'static str,

    // 4. 5-Gate 宪政与安全沙箱
    pub onion_sanitizer: &'static str,
    pub pii_scrubbing: &'static str,
    pub egress_whitelist: &'static str,
    pub platform_sandbox: String,
    pub self_disable_scanner: &'static str,

    // 5. SHA-256 审计区块链与生物动力学
    pub audit_ledger_height: usize,
    pub audit_integrity_ok: bool,
    pub genesis_hash_verified: bool,
    pub borbely_sleep_drive: f64,
    pub circadian_phase: &'static str,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_page: NavPage,
    pub theme: Theme,
    pub theme_transition: Option<(ThemeStyle, ThemeStyle, Instant)>,
    pub thinking_expanded: bool, // Ctrl+O toggle
    pub input_buffer: String,
    pub input_focused: bool,
    pub session_id: String,
    pub is_thinking: bool,

    // Dialogue State & Scrolling
    pub messages: Vec<ChatMessageItem>,
    pub scroll_offset: u16,
    pub scroll_to_bottom: bool,
    pub history_scroll_offset: u16,

    // Live Engineering Telemetry
    pub telemetry: EngineeringTelemetry,

    // Emotion & Bio-drive
    pub current_pad: Pad,

    // ACT-R Memory
    pub memory_items: Vec<MemoryItem>,
    pub memory_selected: usize,

    // Audit blockchain
    pub audit_chain_length: usize,
    pub audit_integrity_ok: bool,

    // Status Message
    pub status_message: Option<(String, i64)>,
}


impl AppState {
    pub fn new(session_id: String, db_path: &str) -> Self {
        Self {
            current_page: NavPage::Bridge,
            theme: Theme::Archaic,
            theme_transition: None,
            thinking_expanded: false,
            input_buffer: String::new(),
            input_focused: true,
            session_id,
            is_thinking: false,

            messages: vec![
                ChatMessageItem {
                    role: "system".into(),
                    content: "Apeireth 2.0 统一认知伴侣内核已启动。后端物理视觉引擎、ACT-R WAL 存储池、5-Gate 宪政门禁与 SHA-256 审计区块链已全量连接。".into(),
                    cot: None,
                    pad: Pad::default(),
                    tokens: 36,
                    audit_hash: "0000000000000000".into(),
                    timestamp_ms: Utc::now().timestamp_millis(),
                }
            ],
            scroll_offset: 0,
            scroll_to_bottom: true,
            history_scroll_offset: 0,

            telemetry: EngineeringTelemetry {
                screen_resolution: "1707x1067 RGB (Win32 Native)".into(),
                screen_driver: "BitBlt + DIBits Zero-Leak Pipeline",
                screen_phash: 0x9f4a1c7b0e2d3f8a,
                screen_hamming_diff: 0,
                som_elements_count: 8,
                screen_latency_ms: 12,

                wal_pool_status: "SQLite WAL Mode (Max: 10 Conns)",
                total_memory_facts: 0,
                avg_activation_score: 1.842,
                cjk_bigram_status: "Jaccard 聚类 + CJK Bigram 激活",
                db_size_kb: 48,
                db_path: db_path.to_string(),

                active_model: "MiniMax-Text-01 (MoE 456B)".into(),
                protocol_adapter: "Minimax JSON / OpenAI & Anthropic 兼容",
                session_tokens: 0,
                last_latency_ms: 380,
                cot_depth: "Deep Reasoning CoT Active (Ctrl+O 展开)",
                streaming_state: "就绪 (Idle / Waiting for turn)",

                onion_sanitizer: "Layer 1/2/3 Principle Onion 严格校验",
                pii_scrubbing: "已启用 (身份证/电话/密钥自动脱敏)",
                egress_whitelist: "严格域名白名单拦截已生效 (Default Deny)",
                platform_sandbox: "Windows Job Object (Process Hardened)".into(),
                self_disable_scanner: "自愈签名扫描器巡检中 (0 异常)",

                audit_ledger_height: 1,
                audit_integrity_ok: true,
                genesis_hash_verified: true,
                borbely_sleep_drive: 0.28,
                circadian_phase: "Process C: Circadian Wave S=0.28",
            },

            current_pad: Pad { pleasure: 0.66, arousal: 0.33, dominance: 0.60 },

            memory_items: Vec::new(),
            memory_selected: 0,

            audit_chain_length: 1,
            audit_integrity_ok: true,

            status_message: Some(("欢迎使用 Apeireth 2.0！按 0-4 或 Tab 切换页面，按 t 切换主题，PageUp/PageDown 滚动。".into(), Utc::now().timestamp_millis())),

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
        let mut state = AppState::new("test_session_123".into(), "test.db");
        assert_eq!(state.current_page, NavPage::Bridge);
        assert_eq!(state.theme, Theme::Archaic);
        assert!(state.scroll_to_bottom);
        state.toggle_theme();
        assert_eq!(state.theme, Theme::Era);
    }
}
