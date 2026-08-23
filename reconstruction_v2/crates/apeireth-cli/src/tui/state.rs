use apeireth_companion::emotion::Pad;
use apeireth_storage::memory_v2::MemoryItem;
use chrono::Utc;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dialogue,
    Memory,
    Organs,
    Factory,
    Governance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Left,
    Center,
    Right,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Search,
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
pub struct FactoryTaskItem {
    pub id: String,
    pub requirement: String,
    pub branch: String,
    pub diff_content: String,
    pub passed: bool,
    pub test_count: usize,
    pub pending_approval: bool,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_tab: ActiveTab,
    pub focus_pane: FocusPane,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub session_id: String,
    pub is_thinking: bool,

    // Dialogue State
    pub messages: Vec<ChatMessageItem>,
    pub message_scroll: usize,

    // Emotion & Bio-drive State
    pub current_pad: Pad,
    pub pad_history: Vec<f64>,
    pub sleep_drive: f64,
    pub sleep_history: Vec<f64>,

    // Memory State
    pub memory_items: Vec<MemoryItem>,
    pub memory_selected: usize,
    pub memory_search_query: String,

    // Factory Tasks State
    pub factory_tasks: Vec<FactoryTaskItem>,
    pub factory_selected: usize,

    // Organ State
    pub screen_width: u32,
    pub screen_height: u32,
    pub screen_elements_count: usize,
    pub last_phash: u64,
    pub vad_energy: f64,
    pub tool_execution_logs: Vec<String>,

    // Governance State
    pub audit_chain_length: usize,
    pub audit_integrity_ok: bool,
    pub gates_passed_count: usize,

    // Notifications & Modals
    pub status_message: Option<(String, i64)>, // (msg, timestamp)
    pub pending_modal: Option<String>,
}

impl AppState {
    pub fn new(session_id: String) -> Self {
        Self {
            active_tab: ActiveTab::Dialogue,
            focus_pane: FocusPane::Input,
            input_mode: InputMode::Editing,
            input_buffer: String::new(),
            input_cursor: 0,
            session_id,
            is_thinking: false,

            messages: vec![
                ChatMessageItem {
                    role: "system".into(),
                    content: "Apeireth 2.0 Living Companion connected. All organs online (Win32 GDI, ACT-R Memory, 5-Gate Governance, Software Factory, MCP Transport).".into(),
                    cot: None,
                    pad: Pad::default(),
                    tokens: 32,
                    audit_hash: "0000000000000000".into(),
                    timestamp_ms: Utc::now().timestamp_millis(),
                }
            ],
            message_scroll: 0,

            current_pad: Pad { pleasure: 0.65, arousal: 0.35, dominance: 0.60 },
            pad_history: vec![0.5, 0.52, 0.58, 0.61, 0.63, 0.65],
            sleep_drive: 0.28,
            sleep_history: vec![0.15, 0.18, 0.22, 0.25, 0.28],

            memory_items: Vec::new(),
            memory_selected: 0,
            memory_search_query: String::new(),

            factory_tasks: vec![
                FactoryTaskItem {
                    id: "task_init_check".into(),
                    requirement: "Egress Whitelist & GDI Screen Capture integration".into(),
                    branch: "factory/task_init_check".into(),
                    diff_content: "+pub fn verify_screen_capture() -> bool { true }\n+pub fn enforce_egress() -> bool { true }".into(),
                    passed: true,
                    test_count: 68,
                    pending_approval: false,
                }
            ],
            factory_selected: 0,

            screen_width: 1707,
            screen_height: 1067,
            screen_elements_count: 8,
            last_phash: 0x9f4a1c7b0e2d3f8a,
            vad_energy: 0.0,
            tool_execution_logs: vec![
                "GDI Screen Capture initialized (1707x1067)".into(),
                "ACT-R Memory WAL Pool attached".into(),
                "MCP Hub StdioTransport ready".into(),
            ],

            audit_chain_length: 1,
            audit_integrity_ok: true,
            gates_passed_count: 5,

            status_message: Some(("Welcome to Apeireth 2.0 TUI! Press F1-F5 to switch tabs, Tab to switch panes.".into(), Utc::now().timestamp_millis())),
            pending_modal: None,
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
        assert_eq!(state.active_tab, ActiveTab::Dialogue);
        assert_eq!(state.focus_pane, FocusPane::Input);
        assert_eq!(state.session_id, "test_session_123");

        state.set_status("Test status");
        assert_eq!(state.status_message.unwrap().0, "Test status");
    }
}

