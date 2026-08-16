//! Apeireth R19 TUI — App 状态机
//!
//! **职责**:
//! - 5 nav 状态 (0/1/2/3/4 = Bridge/Dialogue/Growth/History/Settings)
//! - 当前主题 + 模式 + breath/splash 开关
//! - 对话历史 (气泡)
//! - 输入框缓冲
//!
//! **W3.6 平滑过渡**: `theme_from` / `theme_to` / `theme_transition_start` 三件套 +
//! `begin_theme_transition()` / `current_style()` / `finish_theme_transition_if_done()`
//! 帮手, 200ms RGB 线性插值 (走 `ThemeStyle::interpolate`), 主循环每帧重画.

use std::time::Instant;

use crate::theme::{Theme, ThemeStyle};

/// W3.6: 主题切换的过渡时长 (ms). 200ms = 4s 呼吸的 1/20, 用户能感知但不打断节奏.
/// 编译期 hardcode (任务规范), 不暴露配置.
pub const THEME_TRANSITION_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPage {
    Bridge = 0,
    Dialogue = 1,
    Growth = 2,
    History = 3,
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

    #[allow(dead_code)]
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Bridge => "舰桥",
            Self::Dialogue => "对话",
            Self::Growth => "生长",
            Self::History => "历史",
            Self::Settings => "设置",
        }
    }

    pub fn label_greek(self) -> &'static str {
        match self {
            Self::Bridge => "ΣΚΟΠΗ",
            Self::Dialogue => "ΔΙΑΛΟΓΟΣ",
            Self::Growth => "ΑΥΞΗΣΙΣ",
            Self::History => "ΙΣΤΟΡΙΑ",
            Self::Settings => "ΤΑΞΙΣ",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Focus,
    Inspire,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Inspire => "inspire",
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            Self::Focus => Self::Inspire,
            Self::Inspire => Self::Focus,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Zh,
    En,
}

impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            Self::Zh => Self::En,
            Self::En => Self::Zh,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
}

#[derive(Debug)]
pub struct App {
    pub nav: NavPage,
    pub theme: Theme,
    pub mode: Mode,
    pub language: Language,
    pub splash_enabled: bool,
    pub breath_enabled: bool,
    /// R26-2: settings 页内选项光标 (0..5, 0=mode 1=theme 2=splash 3=breath 4=language)
    /// [j][k] / [Up]/[Down] 切换; 被选中项在 settings 页里金色高亮
    pub settings_cursor: u8,
    /// R26-2: 启动 splash 是否正在显示 (按 splash_enabled 初始化, 按任意键退出)
    /// (运行时状态, 不持久化; 启动时 from settings.splash_enabled)
    pub splash_active: bool,
    /// R26-2: 呼吸节律动画相位 [0.0, 2*PI), 主循环每 tick 推进
    /// tick_rate 250ms + 步进 0.06 ≈ 2.6s 完整呼吸周期 (吸气 -> 屏息 -> 呼气 -> 屏息)
    /// 渲染时 `((phase.sin() + 1.0) * 4.0) as usize % 8` 取 ⠀⠁⠃⠇⠧⠷⠿⠿ 字符
    pub breath_phase: f32,
    /// 对话页输入缓冲 (按 char 存, 不用 String 是因为按 char 索引切分方便)
    pub input_buf: Vec<char>,
    /// W2.6 新增: 输入框光标位置 (char 索引, 0..=input_buf.len())
    /// Left/Right 调这个, Char 插入 + 1, Backspace 删 cursor 之前的字符
    pub input_cursor: usize,
    /// 对话历史
    pub chat_history: Vec<ChatMessage>,
    /// Thinking 链是否展开 (Ctrl+O 切换, 借鉴 Claude Code)
    pub thinking_expanded: bool,
    /// 是否正在等 LLM (W2.3 新增: spinner + 异步 chat 期间 true)
    pub processing: bool,
    /// spinner 帧 (每 250ms tick 切换 0/1/2/3, ⟳ → ◐ → ◑ → ◒)
    pub spinner_frame: u8,
    /// 异步 chat 完成后接收 LLM 回复的 channel
    /// (None = 不在等; Some(rx) = 在等)
    pub chat_rx: Option<std::sync::mpsc::Receiver<String>>,
    /// W3 #1 流式: 累积 streaming chunks (None = 不在流式; Some(s) = 正在累积)
    /// 处理期显示 partial 状态, 收完所有 chunks 后 commit 到 chat_history
    pub streaming_message: Option<String>,
    /// R30 P4: 本轮 AI 调用的工具事件 (灰色行渲染用)
    /// 收完所有 chunks 后 commit 到 chat_history 作为 system 消息, 然后清空
    pub tool_events: Vec<crate::backend::ToolCallEvent>,
    /// R26-3-fixes: AI 处理期间用户提前输入的内容
    /// - None: 没有预充输入
    /// - Some(s): AI 完成后自动提交 (Claude Code / ChatGPT 风格)
    pub pending_input: Option<String>,
    /// R26-3-fixes: 输入焦点模式 (true = 数字键进 input_buf, false = 数字键切 nav)
    /// - 进入 Dialogue 页默认 true
    /// - 鼠标点击 input 区域 -> true, 点击 chat 历史 -> false
    /// - Esc -> false
    /// - AI 处理完 -> true
    pub input_focused: bool,
    /// R26-3-fixes: 输入历史 (只存发过的, FIFO 100 条, 持久化到 ~/.openclaw/apeireth-tui-input-history.txt)
    pub input_history: Vec<String>,
    /// R26-3-fixes: 当前在 input_history 的导航位置
    /// - None: 不在导航 (正常输入)
    /// - Some(idx): 正显示 input_history[idx]
    pub history_idx: Option<usize>,
    /// R26-3-fixes: TUI 内置选区 (chat 历史拖拽选, 按视觉行粒度)
    /// - None: 无选区
    /// - Some((line_a, line_b)): chat_line_map 索引范围 (含两端, 即视觉行号)
    ///   复制时拼接 chat_line_map[lo..=hi] 的 text
    ///   按视觉行 (不按字符) 是因为 Paragraph Wrap 让 char→row 映射很复杂,
    ///   按行粒度用户能拖选区跨多条消息的若干行, 不再"一选就整条"
    pub selection: Option<((usize, usize), (usize, usize))>,
    /// R26-3-fixes: 渲染时计算, 每个 visual line 对应 (msg_idx, 该消息内字符偏移)
    /// 用于选区高亮 (msg_idx + char_idx -> visual line/col)
    pub chat_line_map: Vec<crate::pages::dialogue::LineInfo>, // visual_line_index → (msg_idx, text)

    /// R26-3-fixes: 复制反馈 ("已复制 N 字符", 0.5s 后清)
    pub copy_feedback: Option<(String, std::time::Instant)>,
    /// R26-3-fixes: LLM max_tokens (默认 8192, Settings 可配 1-32768)
    pub max_tokens: u32,
    /// R27 C 方案: 当前 API base_url (跟 LlmConfig.base_url 同步, status_bar 显示)
    pub api_url: String,
    /// R27 C 方案: apeireth-api daemon 连通状态 (主循环每 5s 刷新)
    pub api_online: bool,
    /// 是否退出
    pub should_quit: bool,
    /// 启动时间 (R26-2 改: W2 splash 动画已接, 字段保留供 future)
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// 重新渲染计数 (用于主题切换时强制重画)
    pub render_tick: u64,
    /// 历史页 / 对话页 滚动偏移 (R26 新增: PageUp/PageDown 支持)
    /// 显礼块 (auto-scroll) 等同状态: scroll_offset + scroll_to_bottom
    /// - scroll_to_bottom=true (default): 实时锚到最底 (新消息自动升窗)
    /// - scroll_to_bottom=false: 锚到 scroll_offset, 用户可顾分看早期内容
    pub scroll_offset: u16,
    /// R26-3-fixes: 实时锚到底标志 (默认 true, Claude Code 风格)
    pub scroll_to_bottom: bool,
    // ---- W3.6 主题切换平滑过渡字段 ----
    // (R26: theme_from 已删, 渐变插值仍用 theme_to 即可; 老实现两字段冗余)
    /// 渐变目标主题 (新主题). 与 `theme` 在渐变期间相同, 渐变结束后保留作 `theme_from` 来源参考.
    pub theme_to: Theme,
    /// 渐变开始时刻. `None` = 不在渐变 (用 `ThemeStyle::of(theme)` 静态渲染).
    /// `Some(instant)` = 在 200ms 渐变期内, 用 `ThemeStyle::interpolate(from, to, progress)`.
    pub theme_transition_start: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            nav: NavPage::Bridge,  // 默认舰桥 (首页)
            theme: Theme::Archaic, // 默认古朴金
            mode: Mode::Focus,
            language: Language::Zh,
            splash_enabled: true,
            breath_enabled: true,
            // R26-2: settings cursor 默认 1 (theme) (旧 UX 默认高亮项)
            settings_cursor: 1,
            // R26-2: 启动 splash_active = splash_enabled (主人默认开)
            splash_active: true,
            // R26-2: 呼吸相位从 0 起 (主循环 tick 推进)
            breath_phase: 0.0,
            input_buf: Vec::new(),
            input_cursor: 0,
            chat_history: Vec::new(),
            thinking_expanded: false,
            processing: false,
            spinner_frame: 0,
            chat_rx: None,
            streaming_message: None,
            tool_events: Vec::new(), // R30 P4: 工具事件累积 (灰色行渲染)
            pending_input: None,     // R26-3-fixes: 预充输入 buffer
            should_quit: false,
            started_at: chrono::Utc::now(),
            render_tick: 0,
            scroll_offset: 0,
            scroll_to_bottom: true, // R26-3-fixes: default 锚到底
            // W3.6: 初始无渐变, theme_to = 当前 theme (R26: theme_from 字段已删)
            theme_to: Theme::Archaic,
            theme_transition_start: None,
            input_focused: true, // R26-3-fixes: 进 Dialogue 默认 focus
            input_history: crate::persistence::load_input_history(),
            history_idx: None,
            selection: None,
            chat_line_map: Vec::new(),
            copy_feedback: None,
            max_tokens: 8192, // R26-3-fixes: 默认 8192 (用户可在 Settings 改)
            // R27 C 方案: api_url / api_online 在 main 启动后 health check 填充
            api_url: String::new(),
            api_online: false,
        }
    }

    pub fn push_user_input(&mut self, content: String) {
        if !content.trim().is_empty() {
            self.chat_history.push(ChatMessage {
                role: "user".into(),
                content,
            });
        }
    }

    pub fn push_assistant_reply(&mut self, content: String) {
        self.chat_history.push(ChatMessage {
            role: "assistant".into(),
            content,
        });
    }

    #[allow(dead_code)]
    pub fn push_system(&mut self, content: String) {
        self.chat_history.push(ChatMessage {
            role: "system".into(),
            content,
        });
    }

    // ---- W3.6 主题切换平滑过渡 (按 `t` 键触发) ----

    /// 启动一次 200ms 平滑过渡到 `new_theme` (W3.6).
    ///
    /// 立即把 `theme` 切到新值 (这样 nav_bar 之类显示字符立即刷新),
    /// 同时记录旧主题到 `theme_from` + 设置 `theme_to` + 启动计时.
    /// 主循环每帧调 `current_style()` 拿插值后的 ThemeStyle, 直到渐变结束.
    ///
    /// 如果当前已经在渐变中, **不重置起点**: 让上一次渐变自然结束, 然后下次
    /// 按 `t` 才会启动新渐变. (避免连续按 `t` 抖动颜色, 体现 R19 用户预期:
    /// 主题切换应该是 "看得见" 的渐变, 不是疯狂闪.)
    pub fn begin_theme_transition(&mut self, new_theme: Theme) {
        // 已经在渐变, 不重置 (避免在 200ms 内连续按 `t` 时颜色被卡在中间)
        if self.theme_transition_start.is_some() {
            return;
        }
        // (R26: theme_from 字段已删, 渐变 from = 当前 theme_to 在 current_style 中读)
        // 立即切到新主题 (避免字符/border_type 在 200ms 内显示 "老" 的)
        self.theme = new_theme;
        self.theme_to = new_theme;
        self.theme_transition_start = Some(Instant::now());
    }

    /// 渲染时拿当前应该用的 ThemeStyle (W3.6).
    ///
    /// - 渐变中: `ThemeStyle::interpolate(from, to, progress)` (RGB 线性插值)
    /// - 渐变结束 / 从未开始: `ThemeStyle::of(theme)` (静态)
    ///
    /// 注意: 本方法仅 `&self`, **不会** 自动清 `theme_transition_start`.
    /// 清理由 `finish_theme_transition_if_done()` 在 `run_app` 主循环里做
    /// (因为 immutable borrow 不能改 self, mut borrow 在 ui() 之后).
    pub fn current_style(&self) -> ThemeStyle {
        // R26: theme_from 字段已删, 渐变 from 反查不可达 -> 简化为静态.
        // 保留 self.theme_transition_start 字段给未来 W3.6+ 真渐变 (raintail 重写).
        let _in_transition = self.theme_transition_start.is_some();
        ThemeStyle::of(self.theme)
    }
    ///
    /// 主循环每帧调一次, 如果渐变已经走完 200ms, 清掉 `theme_transition_start`.
    /// 这样 `current_style()` 后续就返回静态 `ThemeStyle::of(theme)`, 不再每帧重算.
    /// 必须 `&mut self` — 这是本方法唯一改 self 的地方.
    pub fn finish_theme_transition_if_done(&mut self) {
        if let Some(start) = self.theme_transition_start {
            if start.elapsed().as_millis() as u64 >= THEME_TRANSITION_MS {
                self.theme_transition_start = None;
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl Mode {
    /// R26: 中文显示 (UI 用, 不改 canonical label() / 持久化 keys)
    pub fn display_label(self) -> &'static str {
        match self {
            Self::Focus => "专注",
            Self::Inspire => "灵感",
        }
    }
}

impl Language {
    /// R26: 中文显示 (UI 用, 不改 canonical label() / 持久化 keys)
    pub fn display_label(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "英文",
        }
    }
}
