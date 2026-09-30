//! 驾驶舱状态机: 面板状态、命令链路、两段式退出、连接失败即帧。
//!
//! 纯逻辑 (无 I/O): [`App::feed`] 吃入输入、产出 [`Effect`]; I/O 由
//! [`crate::effects`] 在后端端口上执行后回灌结果。测试因此可以全程用
//! [`crate::backend::MockBackend`] 走通会话管理命令链路。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::backend::{
    FinishKind, ModelInfo, SessionMeta, ToolEvent, ToolEventKind, TurnDelta, TurnRequest,
    UsageSnapshot,
};
use crate::command::{self, Command};
use crate::keys::{self, Action, KeyMode};
use crate::theme::{DigitTicker, MotionMode};

/// 面板 5 时间倒带的接口桩文案 (显式「未接线」)。
pub const REWIND_STUB_NOTE: &str =
    "时间倒带: 未接线 (本批接口桩; 下批接 continuation / 会话 fork 确定性重放)";

/// 面板 3 内部过程频道的接口桩文案 (显式「未接线」)。
pub const CHANNEL_STUB_NOTE: &str = "内部过程频道: 未接线 (本批接口桩; 下批消费 events 面只读投影)";

/// 退出阶段 (Ctrl+C 两段式: 先断流再退出)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitStage {
    /// 正常运行。
    #[default]
    Running,
    /// 第一段完成: 已切断活动流。
    StreamCut,
    /// 第二段完成: 退出驾驶舱。
    Exiting,
}

/// 连接状态 (启动即探活, 失败即帧)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// 探活中。
    #[default]
    Checking,
    /// 已连接。
    Connected,
    /// 连接失败 (错误帧直接上屏, 不白屏)。
    Failed {
        /// 失败细节。
        detail: String,
    },
}

/// 活动指示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivityState {
    /// 空闲。
    #[default]
    Idle,
    /// 流式回合进行中。
    Streaming,
}

/// 系统注记级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteLevel {
    /// 信息。
    Info,
    /// 告警。
    Warn,
    /// 错误。
    Error,
}

/// 系统注记。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemNote {
    /// 级别。
    pub level: NoteLevel,
    /// 文案。
    pub text: String,
}

/// 工具卡片状态徽章。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// 执行中。
    Running,
    /// 成功。
    Ok,
    /// 失败。
    Failed,
}

impl ToolStatus {
    /// 徽章文案。
    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Ok => "OK",
            Self::Failed => "FAILED",
        }
    }
}

/// 工具卡片 (工具名 + 状态徽章 + 耗时)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCard {
    /// 工具名 (能力 id)。
    pub name: String,
    /// 状态徽章。
    pub status: ToolStatus,
    /// 开始时刻 (epoch 毫秒)。
    pub started_ms: u64,
    /// 耗时 (毫秒); 未收口为 None。
    pub duration_ms: Option<u64>,
}

/// 助手消息 (流式 Markdown 正文 + 工具卡片 + 计量)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantMessage {
    /// 累积正文 (逐增量流式渲染)。
    pub text: String,
    /// 是否仍在流式。
    pub streaming: bool,
    /// 工具卡片。
    pub tools: Vec<ToolCard>,
    /// 计量。
    pub usage: Option<UsageSnapshot>,
    /// 收口方式。
    pub finish: Option<FinishKind>,
}

impl AssistantMessage {
    /// 一条刚开始流式的消息。
    pub fn streaming_message() -> Self {
        Self {
            text: String::new(),
            streaming: true,
            tools: Vec::new(),
            usage: None,
            finish: None,
        }
    }
}

/// 会话消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// 用户输入。
    User(String),
    /// 助手回复。
    Assistant(AssistantMessage),
    /// 系统注记。
    System(SystemNote),
}

/// 进行中回合的令牌。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTurn {
    /// 回合纪元 (断流后旧纪元增量一律丢弃)。
    pub epoch: u64,
}

/// 会话页签。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTab {
    /// 页签标题。
    pub title: String,
    /// 后端会话 id; 未建立为 None。
    pub session_id: Option<String>,
    /// 消息流。
    pub messages: Vec<Message>,
    /// 滚动偏移 (行)。
    pub scroll: u16,
    /// 进行中回合。
    pub pending: Option<PendingTurn>,
    /// 回合纪元计数。
    pub turn_epoch: u64,
}

impl SessionTab {
    /// 新页签。
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            session_id: None,
            messages: Vec::new(),
            scroll: 0,
            pending: None,
            turn_epoch: 0,
        }
    }

    /// 开一个新回合, 返回纪元。
    pub fn begin_turn(&mut self) -> u64 {
        self.turn_epoch = self.turn_epoch.wrapping_add(1);
        self.pending = Some(PendingTurn {
            epoch: self.turn_epoch,
        });
        self.turn_epoch
    }

    /// 当前回合纪元是否仍有效。
    pub fn epoch_live(&self, epoch: u64) -> bool {
        self.pending.map(|pending| pending.epoch) == Some(epoch)
    }
}

/// 输入行。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Composer {
    /// 输入文本。
    pub text: String,
    /// 是否处于命令模式。
    pub command_mode: bool,
}

/// 浮层。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// 帮助浮层。
    Help,
}

/// 实时状态条模型 (面板 2)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusModel {
    /// 连接状态。
    pub connection: ConnectionState,
    /// 当前模型覆盖。
    pub model: Option<String>,
    /// 上下文窗口 (未知为 None, 上屏诚实显示 "?")。
    pub context_window: Option<u32>,
    /// 最近计量。
    pub usage: Option<UsageSnapshot>,
    /// 最近回合耗时 (毫秒)。
    pub turn_latency_ms: Option<u64>,
    /// 活动指示。
    pub activity: ActivityState,
    /// 最近错误。
    pub last_error: Option<String>,
}

/// 输入 (按键 + 后端链路回灌)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// 按键。
    Key(KeyEvent),
    /// 后端探活成功。
    ConnectionOk,
    /// 后端探活失败 (即帧错误)。
    ConnectionFailed {
        /// 失败细节。
        detail: String,
    },
    /// 模型列表已取回。
    ModelsLoaded(Vec<ModelInfo>),
    /// 会话账本已取回。
    SessionsLoaded(Vec<SessionMeta>),
    /// 模型切换已生效。
    ModelSwitched {
        /// 会话 id。
        session: Option<String>,
        /// 模型名。
        model: String,
    },
    /// 回合增量。
    TurnDelta {
        /// 页签下标。
        tab: usize,
        /// 回合纪元。
        epoch: u64,
        /// 增量。
        delta: TurnDelta,
    },
    /// 回合收口。
    TurnFinished {
        /// 页签下标。
        tab: usize,
        /// 回合纪元。
        epoch: u64,
        /// 结果。
        outcome: crate::backend::TurnOutcome,
    },
    /// 回合失败。
    TurnFailed {
        /// 页签下标。
        tab: usize,
        /// 回合纪元。
        epoch: u64,
        /// 错误文案。
        message: String,
    },
    /// 压缩完成。
    CompactDone {
        /// 页签下标。
        tab: usize,
        /// 压缩前消息数。
        before: usize,
        /// 压缩后消息数。
        after: usize,
    },
    /// 压缩失败。
    CompactFailed {
        /// 页签下标。
        tab: usize,
        /// 错误文案。
        message: String,
    },
    /// 导出完成。
    ExportDone {
        /// 页签下标。
        tab: usize,
        /// 落盘路径。
        path: String,
    },
    /// 导出失败。
    ExportFailed {
        /// 页签下标。
        tab: usize,
        /// 错误文案。
        message: String,
    },
    /// 后端链路失败 (通用)。
    BackendFailed {
        /// 哪条链路。
        what: String,
        /// 错误文案。
        message: String,
    },
    /// 动效帧。
    Tick,
}

/// 输出效果 (由链路执行层在后端端口上执行)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// 发送回合。
    SendTurn {
        /// 页签下标。
        tab: usize,
        /// 回合纪元。
        epoch: u64,
        /// 请求。
        request: TurnRequest,
    },
    /// 刷新会话账本。
    RefreshSessions,
    /// 拉取模型列表。
    ListModels,
    /// 会话级模型热切换。
    SwitchModel {
        /// 会话 id。
        session: Option<String>,
        /// 目标模型 (None = 复位默认)。
        model: Option<String>,
    },
    /// 压缩会话上下文。
    Compact {
        /// 页签下标。
        tab: usize,
        /// 会话 id。
        session: String,
    },
    /// 导出会话记录 (本地落盘)。
    Export {
        /// 页签下标。
        tab: usize,
        /// 路径。
        path: String,
        /// 已渲染的 Markdown。
        markdown: String,
    },
    /// 重新探活。
    Reconnect,
}

/// 驾驶舱应用状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// 后端端点 (上屏用)。
    pub endpoint: String,
    /// 会话页签 (面板 1 多会话页签)。
    pub tabs: Vec<SessionTab>,
    /// 活动页签下标。
    pub active: usize,
    /// 输入行。
    pub composer: Composer,
    /// 状态条 (面板 2)。
    pub status: StatusModel,
    /// 浮层。
    pub overlay: Option<Overlay>,
    /// 退出阶段。
    pub exit: ExitStage,
    /// 动效档。
    pub motion: MotionMode,
    /// 会话账本缓存 (`/resume` 数据源)。
    pub session_catalog: Vec<SessionMeta>,
    /// 模型列表缓存。
    pub model_catalog: Vec<ModelInfo>,
    /// 待确认的续接目标。
    pub pending_resume: Option<String>,
    /// 双 Esc 检测 (Esc Esc 时间倒带)。
    pub last_key_was_escape: bool,
    /// 页签计数器。
    pub next_session_no: u32,
    /// 氛围跳动数字。
    pub ticker: DigitTicker,
    /// 帧计数。
    pub frame: u64,
}

impl App {
    /// 构建驾驶舱: 单页签 + 探活中状态 (连接失败也第一帧上屏)。
    pub fn new(endpoint: impl Into<String>, motion: MotionMode) -> Self {
        Self {
            endpoint: endpoint.into(),
            tabs: vec![SessionTab::new("会话 1")],
            active: 0,
            composer: Composer::default(),
            status: StatusModel::default(),
            overlay: None,
            exit: ExitStage::Running,
            motion,
            session_catalog: Vec::new(),
            model_catalog: Vec::new(),
            pending_resume: None,
            last_key_was_escape: false,
            next_session_no: 1,
            ticker: DigitTicker::default(),
            frame: 0,
        }
    }

    /// 按键入口。
    pub fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        self.feed(Input::Key(key))
    }

    /// 喂入一个输入, 产出待执行效果。
    pub fn feed(&mut self, input: Input) -> Vec<Effect> {
        match input {
            Input::Key(key) => self.on_key(key),
            Input::ConnectionOk => {
                self.status.connection = ConnectionState::Connected;
                self.note(NoteLevel::Info, format!("后端已连接: {}", self.endpoint));
                Vec::new()
            }
            Input::ConnectionFailed { detail } => {
                // 连接失败即帧: 状态与错误注记在第一次渲染前就绪, 不白屏。
                self.status.connection = ConnectionState::Failed {
                    detail: detail.clone(),
                };
                self.status.last_error = Some(detail.clone());
                self.note(
                    NoteLevel::Error,
                    format!("连接失败: {detail} —— 本地页签仍可浏览; 后端恢复后用 /resume 续接"),
                );
                Vec::new()
            }
            Input::ModelsLoaded(models) => {
                let names: Vec<String> = models.iter().map(|model| model.id.clone()).collect();
                self.model_catalog = models;
                self.note(
                    NoteLevel::Info,
                    format!("可用模型 {} 个: {}", names.len(), names.join(", ")),
                );
                Vec::new()
            }
            Input::SessionsLoaded(sessions) => self.on_sessions_loaded(sessions),
            Input::ModelSwitched { session, model } => {
                self.status.model = Some(model.clone());
                let scope = session.unwrap_or_else(|| "请求覆盖".to_string());
                self.note(
                    NoteLevel::Info,
                    format!("模型已切换: {model} (作用域: {scope})"),
                );
                Vec::new()
            }
            Input::TurnDelta { tab, epoch, delta } => {
                self.on_turn_delta(tab, epoch, delta);
                Vec::new()
            }
            Input::TurnFinished {
                tab,
                epoch,
                outcome,
            } => self.on_turn_finished(tab, epoch, outcome),
            Input::TurnFailed {
                tab,
                epoch,
                message,
            } => {
                if self.finalize_turn(tab, epoch) {
                    self.status.activity = ActivityState::Idle;
                    self.status.last_error = Some(message.clone());
                    self.note(NoteLevel::Error, format!("回合失败: {message}"));
                }
                Vec::new()
            }
            Input::CompactDone {
                tab: _,
                before,
                after,
            } => {
                self.note(
                    NoteLevel::Info,
                    format!("上下文压缩完成: {before} → {after} 条消息"),
                );
                Vec::new()
            }
            Input::CompactFailed { tab: _, message } => {
                self.note(NoteLevel::Error, format!("上下文压缩未完成: {message}"));
                Vec::new()
            }
            Input::ExportDone { tab: _, path } => {
                self.note(NoteLevel::Info, format!("会话记录已导出: {path}"));
                Vec::new()
            }
            Input::ExportFailed { tab: _, message } => {
                self.note(NoteLevel::Error, format!("会话导出失败: {message}"));
                Vec::new()
            }
            Input::BackendFailed { what, message } => {
                self.note(NoteLevel::Error, format!("{what} 失败: {message}"));
                Vec::new()
            }
            Input::Tick => {
                self.frame = self.frame.wrapping_add(1);
                self.ticker.advance();
                Vec::new()
            }
        }
    }

    fn on_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        let action = keys::map_key(key, self.key_mode());
        if !matches!(action, Action::Escape | Action::Ignored) {
            self.last_key_was_escape = false;
        }
        match action {
            Action::QuitStep => self.quit_step(),
            Action::OpenCommand => {
                self.composer.command_mode = true;
                self.composer.text = "/".to_string();
                Vec::new()
            }
            Action::ToggleHelp => {
                self.overlay = if self.overlay == Some(Overlay::Help) {
                    None
                } else {
                    Some(Overlay::Help)
                };
                Vec::new()
            }
            Action::Submit => self.submit(),
            Action::Escape => self.escape(),
            Action::PageUp => {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    tab.scroll = tab.scroll.saturating_add(5);
                }
                Vec::new()
            }
            Action::PageDown => {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    tab.scroll = tab.scroll.saturating_sub(5);
                }
                Vec::new()
            }
            Action::NextTab => self.cycle_tab(1),
            Action::PrevTab => self.cycle_tab(-1),
            Action::Backspace => {
                self.composer.text.pop();
                Vec::new()
            }
            Action::Complete => self.complete_input(),
            Action::Char(ch) => {
                self.composer.text.push(ch);
                Vec::new()
            }
            Action::Ignored => Vec::new(),
        }
    }

    fn key_mode(&self) -> KeyMode {
        if self.composer.command_mode || !self.composer.text.is_empty() {
            KeyMode::Input
        } else {
            KeyMode::Normal
        }
    }

    /// Ctrl+C 两段式: 第一段断流 (切断本地收流), 第二段退出。
    fn quit_step(&mut self) -> Vec<Effect> {
        match self.exit {
            ExitStage::Running => {
                self.exit = ExitStage::StreamCut;
                for tab in &mut self.tabs {
                    if tab.pending.is_some() {
                        tab.turn_epoch = tab.turn_epoch.wrapping_add(1);
                        tab.pending = None;
                        if let Some(Message::Assistant(assistant)) = tab.messages.last_mut() {
                            if assistant.streaming {
                                assistant.streaming = false;
                            }
                        }
                    }
                }
                self.status.activity = ActivityState::Idle;
                self.note(
                    NoteLevel::Warn,
                    "已断流 (第一段): 本地收流已切断; 再次 Ctrl+C 退出驾驶舱",
                );
                Vec::new()
            }
            ExitStage::StreamCut => {
                self.exit = ExitStage::Exiting;
                Vec::new()
            }
            ExitStage::Exiting => Vec::new(),
        }
    }

    fn escape(&mut self) -> Vec<Effect> {
        if self.overlay.is_some() {
            self.overlay = None;
            self.last_key_was_escape = false;
            return Vec::new();
        }
        if self.composer.command_mode {
            self.composer.command_mode = false;
            self.composer.text.clear();
            self.last_key_was_escape = false;
            return Vec::new();
        }
        if self.last_key_was_escape {
            self.last_key_was_escape = false;
            // 面板 5 时间倒带: 接口桩, 显式「未接线」。
            self.note(NoteLevel::Warn, REWIND_STUB_NOTE);
        } else {
            self.last_key_was_escape = true;
        }
        Vec::new()
    }

    fn submit(&mut self) -> Vec<Effect> {
        if self.composer.command_mode {
            let text = self.composer.text.clone();
            self.composer.text.clear();
            self.composer.command_mode = false;
            self.overlay = None;
            return match command::parse_command(&text) {
                Ok(parsed) => self.dispatch_command(parsed),
                Err(error) => {
                    self.note(NoteLevel::Error, error.message());
                    Vec::new()
                }
            };
        }
        let text = self.composer.text.trim().to_string();
        if text.is_empty() {
            return Vec::new();
        }
        self.composer.text.clear();
        self.send_turn(text)
    }

    fn send_turn(&mut self, text: String) -> Vec<Effect> {
        let tab_index = self.active;
        let model = self.status.model.clone();
        let session = self.tabs[tab_index].session_id.clone();
        let tab = &mut self.tabs[tab_index];
        tab.messages.push(Message::User(text.clone()));
        tab.messages
            .push(Message::Assistant(AssistantMessage::streaming_message()));
        let epoch = tab.begin_turn();
        self.status.activity = ActivityState::Streaming;
        self.status.last_error = None;
        vec![Effect::SendTurn {
            tab: tab_index,
            epoch,
            request: TurnRequest {
                session,
                input: text,
                model,
            },
        }]
    }

    fn dispatch_command(&mut self, parsed: Command) -> Vec<Effect> {
        match parsed {
            Command::Help => {
                self.overlay = Some(Overlay::Help);
                Vec::new()
            }
            Command::New => {
                self.next_session_no += 1;
                let title = format!("会话 {}", self.next_session_no);
                self.tabs.push(SessionTab::new(title));
                self.active = self.tabs.len() - 1;
                self.note(NoteLevel::Info, "已新建会话页签");
                Vec::new()
            }
            Command::Resume { target } => {
                match target {
                    Some(id) => {
                        self.pending_resume = Some(id.clone());
                        self.note(NoteLevel::Info, format!("正在向会话账本确认 {id} …"));
                    }
                    None => {
                        self.note(NoteLevel::Info, "正在刷新会话账本…");
                    }
                }
                vec![Effect::RefreshSessions]
            }
            Command::Compact => {
                let tab_index = self.active;
                match self.tabs[tab_index].session_id.clone() {
                    Some(session) => vec![Effect::Compact {
                        tab: tab_index,
                        session,
                    }],
                    None => {
                        self.note(
                            NoteLevel::Warn,
                            "当前页签还没有会话 id: 先发一个回合建立会话, 或 /resume 续接",
                        );
                        Vec::new()
                    }
                }
            }
            Command::Export { path } => {
                let tab_index = self.active;
                let path = path.unwrap_or_else(|| default_export_path(&self.tabs[tab_index]));
                let markdown = export_markdown(&self.tabs[tab_index]);
                self.note(NoteLevel::Info, format!("正在导出会话记录到 {path} …"));
                vec![Effect::Export {
                    tab: tab_index,
                    path,
                    markdown,
                }]
            }
            Command::Model { name } => match name {
                Some(model) => {
                    self.status.model = Some(model.clone());
                    let session = self.tabs[self.active].session_id.clone();
                    match session {
                        Some(id) => {
                            self.note(
                                NoteLevel::Info,
                                format!("正在切换模型: {model} (会话 {id}) …"),
                            );
                            vec![Effect::SwitchModel {
                                session: Some(id),
                                model: Some(model),
                            }]
                        }
                        None => {
                            self.note(
                                NoteLevel::Info,
                                format!(
                                    "模型覆盖已设为 {model} (随请求生效; 会话建立后可持久化到会话设置)"
                                ),
                            );
                            Vec::new()
                        }
                    }
                }
                None => vec![Effect::ListModels],
            },
            Command::Rewind => {
                self.note(NoteLevel::Warn, REWIND_STUB_NOTE);
                Vec::new()
            }
            Command::Motion { mode } => match mode {
                Some(name) => {
                    if let Some(motion) = MotionMode::parse(&name) {
                        self.motion = motion;
                        self.note(NoteLevel::Info, format!("动效档已切换: {}", motion.label()));
                    }
                    Vec::new()
                }
                None => {
                    self.note(
                        NoteLevel::Info,
                        format!("当前动效档: {}", self.motion.label()),
                    );
                    Vec::new()
                }
            },
            Command::Quit => {
                self.exit = ExitStage::Exiting;
                Vec::new()
            }
        }
    }

    fn complete_input(&mut self) -> Vec<Effect> {
        if !self.composer.command_mode {
            return Vec::new();
        }
        let matches = command::complete_command(&self.composer.text);
        if let Some(first) = matches.first() {
            self.composer.text = format!("/{}", first.name);
        }
        Vec::new()
    }

    fn cycle_tab(&mut self, step: i32) -> Vec<Effect> {
        let len = self.tabs.len() as i32;
        if len == 0 {
            return Vec::new();
        }
        let current = self.active as i32;
        let next = (current + step).rem_euclid(len);
        self.active = next as usize;
        Vec::new()
    }

    fn on_sessions_loaded(&mut self, sessions: Vec<SessionMeta>) -> Vec<Effect> {
        self.session_catalog = sessions.clone();
        match self.pending_resume.take() {
            Some(target) => match sessions.iter().find(|meta| meta.id == target) {
                Some(meta) => {
                    let title = meta.title.clone().unwrap_or_else(|| "无标题".to_string());
                    let tab = &mut self.tabs[self.active];
                    tab.session_id = Some(meta.id.clone());
                    tab.title = title.clone();
                    self.note(NoteLevel::Info, format!("已续接会话 {} ({title})", meta.id));
                }
                None => {
                    self.note(
                        NoteLevel::Error,
                        format!("会话 {target} 不在账本中 (账本 {} 条)", sessions.len()),
                    );
                }
            },
            None => {
                self.note(
                    NoteLevel::Info,
                    format!(
                        "会话账本已刷新: {} 条 (用 /resume <会话id> 续接)",
                        sessions.len()
                    ),
                );
            }
        }
        Vec::new()
    }

    fn on_turn_delta(&mut self, tab_index: usize, epoch: u64, delta: TurnDelta) {
        let Some(tab) = self.tabs.get_mut(tab_index) else {
            return;
        };
        // 断流后 (纪元失配) 的增量一律丢弃。
        if !tab.epoch_live(epoch) {
            return;
        }
        let Some(Message::Assistant(assistant)) = tab.messages.last_mut() else {
            return;
        };
        match delta {
            TurnDelta::Text(chunk) => assistant.text.push_str(&chunk),
            TurnDelta::Tool(event) => apply_tool_event(assistant, event),
            TurnDelta::Usage(usage) => {
                assistant.usage = Some(usage);
                self.status.usage = Some(usage);
            }
        }
    }

    fn on_turn_finished(
        &mut self,
        tab_index: usize,
        epoch: u64,
        outcome: crate::backend::TurnOutcome,
    ) -> Vec<Effect> {
        if !self.finalize_turn(tab_index, epoch) {
            return Vec::new();
        }
        if let Some(tab) = self.tabs.get_mut(tab_index) {
            if tab.session_id.is_none() {
                tab.session_id = Some(outcome.session.clone());
            }
        }
        self.status.activity = ActivityState::Idle;
        self.status.usage = Some(outcome.usage);
        self.status.turn_latency_ms = Some(outcome.latency_ms);
        if outcome.finish == FinishKind::ApprovalRequired {
            self.note(
                NoteLevel::Warn,
                "回合挂起等待审批 (审批处置通道下批接线; 可先用后端命令行处置)",
            );
        }
        Vec::new()
    }

    /// 收口进行中回合; 返回该回合是否仍被认领。
    fn finalize_turn(&mut self, tab_index: usize, epoch: u64) -> bool {
        let Some(tab) = self.tabs.get_mut(tab_index) else {
            return false;
        };
        if !tab.epoch_live(epoch) {
            return false;
        }
        tab.pending = None;
        if let Some(Message::Assistant(assistant)) = tab.messages.last_mut() {
            assistant.streaming = false;
        }
        true
    }

    /// 系统注记进活动页签。
    pub fn note(&mut self, level: NoteLevel, text: impl Into<String>) {
        let message = Message::System(SystemNote {
            level,
            text: text.into(),
        });
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.messages.push(message);
        }
    }

    /// 是否有流式回合在途。
    pub fn is_streaming(&self) -> bool {
        self.tabs.iter().any(|tab| tab.pending.is_some())
    }
}

fn apply_tool_event(assistant: &mut AssistantMessage, event: ToolEvent) {
    match event.kind {
        ToolEventKind::Started => assistant.tools.push(ToolCard {
            name: event.name,
            status: ToolStatus::Running,
            started_ms: event.at_ms,
            duration_ms: None,
        }),
        ToolEventKind::Completed { ok } => {
            if let Some(card) = assistant
                .tools
                .iter_mut()
                .rev()
                .find(|card| card.name == event.name && card.status == ToolStatus::Running)
            {
                card.status = if ok {
                    ToolStatus::Ok
                } else {
                    ToolStatus::Failed
                };
                card.duration_ms = Some(event.at_ms.saturating_sub(card.started_ms));
            }
        }
    }
}

/// 默认导出路径。
pub fn default_export_path(tab: &SessionTab) -> String {
    let id = tab.session_id.as_deref().unwrap_or("draft");
    format!("apeireth-session-{id}.md")
}

/// 会话记录 → Markdown 导出文本。
pub fn export_markdown(tab: &SessionTab) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", tab.title));
    if let Some(id) = &tab.session_id {
        out.push_str(&format!("<!-- session: {id} -->\n\n"));
    }
    for message in &tab.messages {
        match message {
            Message::User(text) => {
                out.push_str("## 用户\n\n");
                out.push_str(text);
                out.push_str("\n\n");
            }
            Message::Assistant(assistant) => {
                out.push_str("## 助手\n\n");
                for tool in &assistant.tools {
                    let duration = tool
                        .duration_ms
                        .map(|ms| format!("{ms}ms"))
                        .unwrap_or_else(|| "进行中".to_string());
                    out.push_str(&format!(
                        "- 工具 `{}` — {} — {}\n",
                        tool.name,
                        tool.status.label(),
                        duration
                    ));
                }
                if !assistant.tools.is_empty() {
                    out.push('\n');
                }
                out.push_str(&assistant.text);
                out.push_str("\n\n");
            }
            Message::System(note) => {
                out.push_str(&format!("> {}\n\n", note.text));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyEventKind, KeyEventState};

    use super::*;
    use crate::backend::{BackendCall, CompactReport, MockBackend, TurnOutcome};
    use crate::effects;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn ctrl_c() -> KeyEvent {
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn type_line(app: &mut App, line: &str) -> Vec<Effect> {
        let mut effects = Vec::new();
        for ch in line.chars() {
            effects.extend(app.handle_key(key(KeyCode::Char(ch))));
        }
        effects.extend(app.handle_key(key(KeyCode::Enter)));
        effects
    }

    fn drive(app: &mut App, backend: &mut MockBackend, effects: Vec<Effect>) {
        let mut queue: Vec<Effect> = effects;
        let mut rounds = 0;
        while !queue.is_empty() {
            rounds += 1;
            assert!(rounds < 8, "命令链路出现环形效果");
            let mut inputs = Vec::new();
            for effect in queue.drain(..) {
                effects::run(effect, backend, &mut |input| inputs.push(input));
            }
            for input in inputs {
                queue.extend(app.feed(input));
            }
        }
    }

    /// 两段式退出: 第一段断流, 第二段退出; 断流后旧纪元增量被丢弃。
    #[test]
    fn two_stage_ctrl_c_cuts_stream_then_exits() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        let mut backend = MockBackend::with_reply("流式回复");
        let effects = type_line(&mut app, "完成的回合");
        drive(&mut app, &mut backend, effects);
        assert!(app.tabs[0].messages.len() >= 2);

        // 起一个在途回合 (效果暂不执行, 保持 pending)。
        let _in_flight = type_line(&mut app, "在途回合");
        let epoch = app.tabs[0].pending.expect("回合在途").epoch;
        assert_eq!(app.exit, ExitStage::Running);

        // 第一段: 断流。
        app.handle_key(ctrl_c());
        assert_eq!(app.exit, ExitStage::StreamCut);
        assert!(app.tabs[0].pending.is_none());
        let text_before = render_plain(&app);
        assert!(text_before.contains("已断流"));

        // 断流后旧纪元增量被丢弃。
        app.feed(Input::TurnDelta {
            tab: 0,
            epoch,
            delta: TurnDelta::Text("断流后的增量".to_string()),
        });
        assert!(!render_plain(&app).contains("断流后的增量"));
        // 旧纪元收口帧也不再被认领。
        let follow = app.feed(Input::TurnFinished {
            tab: 0,
            epoch,
            outcome: TurnOutcome {
                session: "s".to_string(),
                text: String::new(),
                usage: UsageSnapshot::default(),
                served_by: String::new(),
                rounds: 1,
                latency_ms: 1,
                finish: FinishKind::Stop,
            },
        });
        assert!(follow.is_empty());

        // 第二段: 退出。
        app.handle_key(ctrl_c());
        assert_eq!(app.exit, ExitStage::Exiting);
    }

    /// 连接失败即帧: 探活失败后状态与错误帧立刻就绪 (首帧不白屏)。
    #[test]
    fn connection_failure_is_framed_immediately() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        assert_eq!(app.status.connection, ConnectionState::Checking);
        app.feed(Input::ConnectionFailed {
            detail: "connection refused".to_string(),
        });
        assert!(matches!(
            app.status.connection,
            ConnectionState::Failed { .. }
        ));
        let plain = render_plain(&app);
        assert!(plain.contains("连接失败"));
        assert!(plain.contains("connection refused"));
    }

    /// 会话管理命令链路 (mock 后端): /resume /compact /model /new /export 全程走端口。
    #[test]
    fn session_commands_flow_through_backend_chain() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        let mut backend = MockBackend::with_reply("好的");
        backend.sessions = vec![SessionMeta {
            id: "abc-123".to_string(),
            title: Some("演练会话".to_string()),
            updated_at: 0,
            message_count: 3,
        }];
        backend.compact_report = Some(CompactReport {
            session: "abc-123".to_string(),
            before_messages: 6,
            after_messages: 2,
        });
        backend.models = vec![ModelInfo {
            id: "demo-model".to_string(),
            provider: "mock.provider".to_string(),
            description: None,
        }];

        // /resume abc-123 → 拉账本 → 绑定页签会话。
        let effects = type_line(&mut app, "/resume abc-123");
        drive(&mut app, &mut backend, effects);
        assert!(backend.calls.contains(&BackendCall::ListSessions));
        assert_eq!(app.tabs[app.active].session_id.as_deref(), Some("abc-123"));
        assert_eq!(app.tabs[app.active].title, "演练会话");

        // /compact → 压缩链路进后端。
        let effects = type_line(&mut app, "/compact");
        drive(&mut app, &mut backend, effects);
        assert!(backend.calls.contains(&BackendCall::Compact {
            session: "abc-123".to_string()
        }));
        assert!(render_plain(&app).contains("上下文压缩完成"));

        // /model demo-model → 会话级热切换链路。
        let effects = type_line(&mut app, "/model demo-model");
        drive(&mut app, &mut backend, effects);
        assert!(backend.calls.contains(&BackendCall::SetModel {
            session: Some("abc-123".to_string()),
            model: Some("demo-model".to_string())
        }));
        assert_eq!(app.status.model.as_deref(), Some("demo-model"));

        // /model (无参) → 模型列表链路。
        let effects = type_line(&mut app, "/model");
        drive(&mut app, &mut backend, effects);
        assert!(backend.calls.contains(&BackendCall::ListModels));

        // /new → 新页签。
        let effects = type_line(&mut app, "/new");
        drive(&mut app, &mut backend, effects);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active, 1);

        // /export → 本地落盘导出。
        let dir = tempfile::tempdir().expect("临时目录");
        let path = dir.path().join("out.md");
        let effects = type_line(&mut app, &format!("/export {}", path.display()));
        drive(&mut app, &mut backend, effects);
        let written = fs_err::read_to_string(&path).expect("导出文件");
        assert!(written.contains("# 会话 2"));
    }

    /// 时间倒带接口桩: 双 Esc 与 /rewind 都显式「未接线」。
    #[test]
    fn rewind_stub_is_explicitly_marked() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.handle_key(key(KeyCode::Esc));
        assert!(!render_plain(&app).contains("未接线"));
        app.handle_key(key(KeyCode::Esc));
        assert!(render_plain(&app).contains("时间倒带: 未接线"));
        let effects = type_line(&mut app, "/rewind");
        drive(&mut app, &mut MockBackend::default(), effects);
        assert!(render_plain(&app).contains(REWIND_STUB_NOTE));
    }

    /// 双 Esc 检测是「连续两次」: 中间隔了别的键就不触发。
    #[test]
    fn rewind_needs_two_consecutive_escapes() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Char('x')));
        app.handle_key(key(KeyCode::Backspace));
        app.handle_key(key(KeyCode::Esc));
        assert!(!render_plain(&app).contains("时间倒带: 未接线"));
    }

    /// 流式增量 + 工具卡片: 名称/徽章/耗时三要素齐上。
    #[test]
    fn streaming_deltas_and_tool_cards_land_in_transcript() {
        let mut app = App::new("http://127.0.0.1:8080", MotionMode::Full);
        let mut backend = MockBackend::default();
        backend.turn_deltas = vec!["流式".to_string(), "回复".to_string()];
        backend.turn_tools = vec![
            ToolEvent {
                kind: ToolEventKind::Started,
                name: "tool.demo".to_string(),
                at_ms: 1_000,
            },
            ToolEvent {
                kind: ToolEventKind::Completed { ok: true },
                name: "tool.demo".to_string(),
                at_ms: 1_342,
            },
        ];
        let effects = type_line(&mut app, "跑一个工具");
        drive(&mut app, &mut backend, effects);
        let plain = render_plain(&app);
        assert!(plain.contains("流式回复"));
        assert!(plain.contains("tool.demo"));
        assert!(plain.contains("OK"));
        assert!(plain.contains("342ms"));
        assert!(app.status.turn_latency_ms.is_some());
    }

    /// 把当前页签消息流拍平成纯文本 (断言用)。
    fn render_plain(app: &App) -> String {
        let mut out = String::new();
        for tab in &app.tabs {
            for message in &tab.messages {
                match message {
                    Message::User(text) => out.push_str(text),
                    Message::Assistant(assistant) => {
                        out.push_str(&assistant.text);
                        for tool in &assistant.tools {
                            out.push_str(&tool.name);
                            out.push_str(tool.status.label());
                            if let Some(ms) = tool.duration_ms {
                                out.push_str(&format!("{ms}ms"));
                            }
                        }
                    }
                    Message::System(note) => out.push_str(&note.text),
                }
                out.push('\n');
            }
        }
        out
    }

    /// 键盘事件类型守卫: 事件枚举可被驱动 (编译期面)。
    #[test]
    fn key_events_round_trip_through_feed() {
        let mut app = App::new("mock://x", MotionMode::Reduced);
        let effects = app.feed(Input::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::empty(),
        }));
        assert!(effects.is_empty());
        assert_eq!(app.composer.text, "");
        // Event 枚举占位: 终端循环按 Event 分发 (编译期断言形状可用)。
        let event = Event::Key(key(KeyCode::Char('a')));
        if let Event::Key(inner) = event {
            let typed = app.handle_key(inner);
            assert!(typed.is_empty());
            assert_eq!(app.composer.text, "a");
        }
    }
}
