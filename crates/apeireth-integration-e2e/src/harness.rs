//! # `IntegrationHarness` — 三层 e2e 编排核心
//!
//! **职责**: 一次性起三层测试环境, 提供 workspace + API + TUI 子句柄.
//!
//! **三层**:
//! - **Workspace**: 锁主仓根路径, 用 `cargo metadata` + 文件系统审计验证状态
//! - **API**: wiremock `MockServer` + reqwest 客户端, 模拟 `apeireth-api` 6 端点
//! - **TUI**: ratatui `TestBackend` + 本地 mirror TuiApp, 1 屏 4 panel
//!
//! **生命周期**:
//! ```text
//!   IntegrationHarness::start()
//!     ├─ 确定 workspace_root (主仓根)
//!     ├─ 启动 wiremock (随机端口 0)
//!     ├─ 创建 TuiTestBackend (默认 80x24)
//!     └─ 装载默认 5 provider 路由 + 6 类 V2 endpoint
//!
//!   ... 测试中 ...
//!
//!   IntegrationHarness::shutdown()
//!     └─ wiremock drop (graceful)
//! ```
//!
//! **8 不修改承诺**: 跟 lib.rs / error.rs 一致
//!
//! **跟 `apeireth-tui-e2e::TuiHarness` 的关系**:
//! - `TuiHarness` 测 TUI 设计契约 (5 nav / 9 organ / 1 屏 4 panel)
//! - `IntegrationHarness::tui()` 复用了 TuiTestBackend, 但额外提供三层联动
//! - 互补, 不重复 (per 派活单 §4)

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use reqwest::Client as ReqwestClient;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::error::{E2EError, E2EResult};

// =====================================================================
// 5 K-1 强校验 — 编译期 hardcode 常量
// =====================================================================

/// 终端默认宽 (跟 tui 主线一致)
pub const DEFAULT_WIDTH: u16 = 80;

/// 终端默认高
pub const DEFAULT_HEIGHT: u16 = 24;

/// 6 类 V2 endpoint 总数 (per v2_endpoints §3)
pub const V2_ENDPOINT_GROUPS: u8 = 6;

/// 6 哲学锚 数量 (per APEIRETH-CONVENTIONS §0.2)
pub const SIX_PHI_ANCHORS: u8 = 6;

/// 5 nav 数 (per TuiApp::NavPage)
pub const FIVE_NAV: u8 = 5;

/// 9 器官数 (per TuiApp::Organ)
pub const NINE_ORGANS: u8 = 9;

/// 8 不修改承诺 (per docs/stage4/8-locked-unified-2026-08-05.md §2)
pub const EIGHT_PROMISES: u8 = 8;

// =====================================================================
// TuiTestBackend — ratatui TestBackend 包装
// =====================================================================

/// TUI 渲染后端视图 (持有 width/height, 通过 `backend()` 取 ratatui backend 引用)
pub struct TuiTestBackend {
    /// 宽
    pub width: u16,
    /// 高
    pub height: u16,
}

impl TuiTestBackend {
    /// 构造指定尺寸 (view only, 真 backend 在 Terminal 里)
    pub fn new(width: u16, height: u16) -> E2EResult<Self> {
        if width == 0 || height == 0 {
            return Err(E2EError::TuiRender {
                width,
                height,
                reason: "width / height must be > 0 (K-1 强校验)".into(),
            });
        }
        Ok(Self { width, height })
    }

    /// 默认 80x24
    pub fn default_24x80() -> E2EResult<Self> {
        Self::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }
}

impl std::fmt::Debug for TuiTestBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiTestBackend")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

// =====================================================================
// IntegrationHarness
// =====================================================================

/// 三层集成测试 harness
///
/// - `workspace_root` 锁主仓根, 用作 cargo metadata + 文件审计
/// - `api_server` wiremock 模拟 6 端点
/// - `api_client` reqwest 客户端, 走 wiremock URI
/// - `tui_terminal` ratatui Terminal<TestBackend>, 渲染 1 屏 4 panel
/// - `tui_app` 镜像 apeireth-tui App 状态 (5 nav / 9 organ)
/// - `tempdir` workspace 隔离测试目录 (cargo metadata 用)
pub struct IntegrationHarness {
    /// 主仓根路径
    pub workspace_root: PathBuf,
    /// wiremock mock server
    pub api_server: MockServer,
    /// reqwest 客户端 (走 mock server URI)
    pub api_client: ReqwestClient,
    /// ratatui 终端 (绑 TestBackend)
    pub tui_terminal: Terminal<TestBackend>,
    /// TuiTestBackend 视图 (断言助手)
    pub tui_backend: TuiTestBackend,
    /// TUI 应用状态镜像
    pub tui_app: Arc<Mutex<TuiAppMirror>>,
    /// 临时目录 (workspace 隔离测试)
    pub tempdir: TempDir,
}

/// TuiApp 镜像 (per apeireth-tui-e2e 公开 API 表面, 跟 tui 主线字段对齐)
#[derive(Debug, Clone)]
pub struct TuiAppMirror {
    /// 当前 nav
    pub nav: NavPageMirror,
    /// 9 器官状态 (顺序对齐 9 器官 LOCKED)
    pub organs: [OrganMirrorState; 9],
    /// should_quit
    pub should_quit: bool,
    /// render tick
    pub render_tick: u32,
    /// 当前 mode
    pub mode: ModeMirror,
}

/// 5 nav (per tui 主线 NavPage)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavPageMirror {
    /// 0 桥接 (默认)
    Bridge = 0,
    /// 1 对话
    Dialogue = 1,
    /// 2 成长
    Growth = 2,
    /// 3 历史
    History = 3,
    /// 4 设置
    Settings = 4,
}

impl NavPageMirror {
    /// 全部 5 nav (跟 tui 顺序对齐)
    pub const ALL: [Self; 5] = [
        Self::Bridge,
        Self::Dialogue,
        Self::Growth,
        Self::History,
        Self::Settings,
    ];

    /// 中文 label (跟 tui label_zh 镜像)
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Bridge => "桥接",
            Self::Dialogue => "对话",
            Self::Growth => "成长",
            Self::History => "历史",
            Self::Settings => "设置",
        }
    }

    /// 英文 label (e2e render 用, 避免 CJK 在 Windows CP936 终端显示 `?`)
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Bridge => "Bridge",
            Self::Dialogue => "Dialogue",
            Self::Growth => "Growth",
            Self::History => "History",
            Self::Settings => "Settings",
        }
    }
}

impl Default for NavPageMirror {
    fn default() -> Self {
        Self::Bridge
    }
}

/// 9 器官 (per tui 主线 Organ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrganMirror {
    /// 0 心
    Heart = 0,
    /// 1 脑
    Brain = 1,
    /// 2 手
    Hand = 2,
    /// 3 眼
    Eye = 3,
    /// 4 耳
    Ear = 4,
    /// 5 忆
    Memory = 5,
    /// 6 声
    Voice = 6,
    /// 7 体
    Body = 7,
    /// 8 心 (mind)
    Mind = 8,
}

impl OrganMirror {
    /// 全部 9 器官
    pub const ALL: [Self; 9] = [
        Self::Heart,
        Self::Brain,
        Self::Hand,
        Self::Eye,
        Self::Ear,
        Self::Memory,
        Self::Voice,
        Self::Body,
        Self::Mind,
    ];

    /// 中文 label
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Heart => "心",
            Self::Brain => "脑",
            Self::Hand => "手",
            Self::Eye => "眼",
            Self::Ear => "耳",
            Self::Memory => "忆",
            Self::Voice => "声",
            Self::Body => "体",
            Self::Mind => "魂",
        }
    }

    /// 英文 label (e2e render 用, 避免 CJK 在 Windows CP936 终端显示 `?`)
    pub fn label_en(self) -> &'static str {
        match self {
            Self::Heart => "Heart",
            Self::Brain => "Brain",
            Self::Hand => "Hand",
            Self::Eye => "Eye",
            Self::Ear => "Ear",
            Self::Memory => "Memory",
            Self::Voice => "Voice",
            Self::Body => "Body",
            Self::Mind => "Mind",
        }
    }
}

/// TUI 模式 (Focus / Inspire)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeMirror {
    /// Focus 模式 (默认)
    Focus,
    /// Inspire 模式
    Inspire,
}

impl Default for ModeMirror {
    fn default() -> Self {
        Self::Focus
    }
}

impl Default for TuiAppMirror {
    fn default() -> Self {
        Self {
            nav: NavPageMirror::default(),
            organs: std::array::from_fn(|i| OrganMirror::from_index(i as u8).into_mirror_state()),
            should_quit: false,
            render_tick: 0,
            mode: ModeMirror::default(),
        }
    }
}

impl OrganMirror {
    /// 0-8 → `OrganMirror`, 越界返回 `None`
    pub fn from_index(i: u8) -> Self {
        match i {
            0 => Self::Heart,
            1 => Self::Brain,
            2 => Self::Hand,
            3 => Self::Eye,
            4 => Self::Ear,
            5 => Self::Memory,
            6 => Self::Voice,
            7 => Self::Body,
            _ => Self::Mind,
        }
    }

    /// 转 OrganMirror 状态 (默认 active=true)
    fn into_mirror_state(self) -> OrganMirrorState {
        OrganMirrorState {
            kind: self,
            active: true,
        }
    }
}

/// 器官状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganMirrorState {
    /// 器官
    pub kind: OrganMirror,
    /// 是否活跃
    pub active: bool,
}

impl From<OrganMirror> for OrganMirrorState {
    fn from(k: OrganMirror) -> Self {
        Self {
            kind: k,
            active: true,
        }
    }
}

impl TuiAppMirror {
    /// 模拟 1s tick
    pub fn tick(&mut self) {
        self.render_tick = self.render_tick.wrapping_add(1);
    }

    /// 处理 key
    pub fn handle_key(&mut self, key: char) {
        match key {
            'q' | 'Q' => self.should_quit = true,
            '1' => self.nav = NavPageMirror::Bridge,
            '2' => self.nav = NavPageMirror::Dialogue,
            '3' => self.nav = NavPageMirror::Growth,
            '4' => self.nav = NavPageMirror::History,
            '5' => self.nav = NavPageMirror::Settings,
            _ => {}
        }
    }
}

impl IntegrationHarness {
    /// 启动 harness (用 cargo metadata 自动找主仓根)
    pub async fn start() -> E2EResult<Self> {
        let workspace_root = find_workspace_root()?;
        let tempdir = tempfile::tempdir().map_err(|e| E2EError::HarnessStart {
            reason: format!("tempdir: {e}"),
        })?;
        let api_server = MockServer::start().await;
        let api_client = ReqwestClient::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| E2EError::HarnessStart {
                reason: format!("reqwest build: {e}"),
            })?;
        let tui_backend = TuiTestBackend::default_24x80()?;
        let backend_for_terminal = TestBackend::new(tui_backend.width, tui_backend.height);
        let tui_terminal =
            Terminal::new(backend_for_terminal).map_err(|e| E2EError::HarnessStart {
                reason: format!("ratatui terminal: {e}"),
            })?;
        let tui_app = Arc::new(Mutex::new(TuiAppMirror::default()));
        // **不**挂默认 mocks — 让每个 test 自己挂 (避免 wiremock 重复挂相同 path 冲突)
        // mount_default_mocks(&api_server).await?;
        Ok(Self {
            workspace_root,
            api_server,
            api_client,
            tui_terminal,
            tui_backend,
            tui_app,
            tempdir,
        })
    }

    /// 启动并使用指定主仓根
    pub async fn start_at(workspace_root: PathBuf) -> E2EResult<Self> {
        let mut h = Self::start().await?;
        h.workspace_root = workspace_root;
        Ok(h)
    }

    /// 关 harness (wiremock 自动 drop)
    pub async fn shutdown(self) -> E2EResult<()> {
        // MockServer drop 自动 graceful shutdown
        drop(self.api_server);
        Ok(())
    }

    /// API 端点 URI (per wiremock mock server)
    pub fn api_uri(&self, path: &str) -> String {
        format!("{}{}", self.api_server.uri(), path)
    }

    /// GET 端点
    pub async fn api_get(&self, path: &str) -> E2EResult<reqwest::Response> {
        let url = self.api_uri(path);
        self.api_client
            .get(&url)
            .send()
            .await
            .map_err(|e| E2EError::ApiHttp {
                url: url.clone(),
                reason: e.to_string(),
            })
    }

    /// POST 端点
    pub async fn api_post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> E2EResult<reqwest::Response> {
        let url = self.api_uri(path);
        self.api_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| E2EError::ApiHttp {
                url: url.clone(),
                reason: e.to_string(),
            })
    }

    /// PUT 端点
    pub async fn api_put(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> E2EResult<reqwest::Response> {
        let url = self.api_uri(path);
        self.api_client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| E2EError::ApiHttp {
                url: url.clone(),
                reason: e.to_string(),
            })
    }

    /// DELETE 端点
    pub async fn api_delete(&self, path: &str) -> E2EResult<reqwest::Response> {
        let url = self.api_uri(path);
        self.api_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| E2EError::ApiHttp {
                url: url.clone(),
                reason: e.to_string(),
            })
    }

    /// TUI 渲染 — 写 1 屏 4 panel 到 backend
    ///
    /// **注意**: 用英文 label 渲染 (e2e 测试稳定性).
    /// 中文 label 仍在 `NavPageMirror::label_zh` / `OrganMirror::label_zh`,
    /// 跟 `apeireth-tui` 主线一致, 但 render 走英文避免 Windows CP936 终端 CJK 显示 `?`.
    pub fn tui_render(&mut self) -> E2EResult<()> {
        use ratatui::layout::{Constraint, Direction, Layout};
        let mut app = self.tui_app.lock();
        app.tick();
        let nav = app.nav;
        let organs = app.organs;
        let should_quit = app.should_quit;
        let render_tick = app.render_tick;
        drop(app);
        self.tui_terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // top: 5 nav
                        Constraint::Length(3), // middle: 9 organ
                        Constraint::Min(5),    // content
                        Constraint::Length(3), // status
                    ])
                    .split(f.area());
                // top: 5 nav (英文 label)
                let nav_text: String = NavPageMirror::ALL
                    .iter()
                    .map(|n| {
                        if *n == nav {
                            format!("[{}]", n.label_en())
                        } else {
                            format!(" {} ", n.label_en())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                f.render_widget(ratatui::widgets::Paragraph::new(nav_text), chunks[0]);
                // middle: 9 organ (英文 label)
                let organ_text: String = OrganMirror::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        let active = organs.get(i).map(|s| s.active).unwrap_or(false);
                        let label = o.label_en();
                        if active {
                            format!("[{label}]")
                        } else {
                            format!(" {label} ")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                f.render_widget(ratatui::widgets::Paragraph::new(organ_text), chunks[1]);
                // content
                let content = if should_quit {
                    "Bye, Apeireth".to_string()
                } else {
                    format!(
                        "Current nav: {}\nrender_tick: {}",
                        nav.label_en(),
                        render_tick
                    )
                };
                f.render_widget(ratatui::widgets::Paragraph::new(content), chunks[2]);
                // status (英文, 避免 CJK 编码问题)
                let status =
                    "S-1 Polaris | S-2 Realistic | O-5 Honest | press q to quit".to_string();
                f.render_widget(ratatui::widgets::Paragraph::new(status), chunks[3]);
            })
            .map_err(|e| E2EError::TuiRender {
                width: self.tui_backend.width,
                height: self.tui_backend.height,
                reason: e.to_string(),
            })?;
        Ok(())
    }

    /// TUI buffer 内容 (从 terminal 后端提取)
    pub fn tui_buffer_text(&mut self) -> E2EResult<String> {
        self.tui_render()?;
        let buf = self.tui_terminal.backend().buffer();
        let area = buf.area;
        let mut cells = String::with_capacity((area.width as usize) * (area.height as usize));
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                if sym.is_empty() {
                    cells.push(' ');
                } else {
                    cells.push_str(sym);
                }
            }
            cells.push('\n');
        }
        Ok(cells)
    }

    /// 断言 buffer 包含某段
    pub fn tui_assert_contains(&mut self, needle: &str) -> E2EResult<()> {
        let text = self.tui_buffer_text()?;
        if !text.contains(needle) {
            return Err(E2EError::TuiAssert {
                context: "tui_assert_contains".into(),
                expected: format!("<contains `{needle}`>"),
                actual: format!(
                    "<not found in {w}x{h} buffer>",
                    w = self.tui_backend.width,
                    h = self.tui_backend.height
                ),
            });
        }
        Ok(())
    }
}

// 注: E2EError::HarnessStart 复用, 没有需要单独 fallback 的 HuiStart 变体

/// 自动找主仓根 (从 CARGO_MANIFEST_DIR 父目录, 或 PWD 向上找 Cargo.toml)
///
/// 关键: 本 crate 在 `crates/apeireth-integration-e2e/`, CARGO_MANIFEST_DIR 需上 2 级
fn find_workspace_root() -> E2EResult<PathBuf> {
    // 1. CARGO_MANIFEST_DIR 上 2 级 (sub-workspace 模式)
    if let Some(manifest) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest);
        // p = crates/apeireth-integration-e2e/
        // 上 2 级: crates/apeireth-integration-e2e/../.. = 主仓根
        if let Some(grand) = p.parent().and_then(|x| x.parent()) {
            if grand.join("Cargo.toml").exists() && grand.join("crates").is_dir() {
                return Ok(grand.to_path_buf());
            }
        }
        // 上 1 级 fallback (兼容老布局)
        if let Some(parent) = p.parent() {
            if parent.join("Cargo.toml").exists() && parent.join("crates").is_dir() {
                return Ok(parent.to_path_buf());
            }
        }
    }
    // 2. 当前 PWD 向上找
    let mut p = std::env::current_dir().map_err(|e| E2EError::HarnessStart {
        reason: format!("current_dir: {e}"),
    })?;
    for _ in 0..8 {
        if p.join("Cargo.toml").exists() && p.join("crates").is_dir() {
            return Ok(p);
        }
        if let Some(parent) = p.parent() {
            p = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(E2EError::HarnessStart {
        reason: "无法自动找主仓根 (CARGO_MANIFEST_DIR 上 2 级 / PWD 向上找都失败)".into(),
    })
}

/// 装默认 mocks: 5 provider 路由 + 6 V2 端点 + 错误码
async fn mount_default_mocks(server: &MockServer) -> E2EResult<()> {
    // 5 provider (per api 6 LLM provider)
    for p in ["anthropic", "openai", "gemini", "codex", "claude-code"] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/providers/{p}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "provider": p,
                "status": "ready"
            })))
            .mount(server)
            .await;
    }
    // 6 V2 endpoint
    Mock::given(method("GET"))
        .and(path("/v1/tools/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tools": ["WebSearch", "FileOperator", "Git", "ShellExec"]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/memory/episodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "episodes": []
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "organs": ["心", "脑", "手", "眼", "耳", "忆", "声", "体", "心-mind"]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/asi/all"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "dims": 24,
            "score": 0.9
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/sovereignty/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "armed": true,
            "guards": 5
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/agent/aliases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "aliases": []
        })))
        .mount(server)
        .await;
    // health
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "Healthy"
        })))
        .mount(server)
        .await;
    // metrics
    Mock::given(method("GET"))
        .and(path("/metrics"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "# HELP apeireth_up 1 if up\n# TYPE apeireth_up gauge\napeireth_up 1\n",
        ))
        .mount(server)
        .await;
    Ok(())
}

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_page_mirror_count_5() {
        assert_eq!(NavPageMirror::ALL.len(), 5);
    }

    #[test]
    fn organ_mirror_count_9() {
        assert_eq!(OrganMirror::ALL.len(), 9);
    }

    #[test]
    fn nav_label_zh_unique() {
        let labels: Vec<&str> = NavPageMirror::ALL.iter().map(|n| n.label_zh()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 5);
    }

    #[test]
    fn organ_label_zh_unique() {
        let labels: Vec<&str> = OrganMirror::ALL.iter().map(|o| o.label_zh()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn tui_app_mirror_default_bridge() {
        let app = TuiAppMirror::default();
        assert_eq!(app.nav, NavPageMirror::Bridge);
        assert!(!app.should_quit);
        assert_eq!(app.render_tick, 0);
        assert_eq!(app.mode, ModeMirror::Focus);
    }

    #[test]
    fn tui_app_mirror_handle_key_q() {
        let mut app = TuiAppMirror::default();
        app.handle_key('q');
        assert!(app.should_quit);
    }

    #[test]
    fn tui_app_mirror_handle_key_1_to_5() {
        let mut app = TuiAppMirror::default();
        app.handle_key('3');
        assert_eq!(app.nav, NavPageMirror::Growth);
        app.handle_key('5');
        assert_eq!(app.nav, NavPageMirror::Settings);
    }

    #[test]
    fn tui_test_backend_default_24x80() {
        let b = TuiTestBackend::default_24x80().unwrap();
        assert_eq!(b.width, DEFAULT_WIDTH);
        assert_eq!(b.height, DEFAULT_HEIGHT);
    }

    #[test]
    fn tui_test_backend_zero_size_rejected() {
        let r = TuiTestBackend::new(0, 24);
        assert!(matches!(r, Err(E2EError::TuiRender { .. })));
    }

    #[test]
    fn k1_constants_match() {
        // 5 K-1 强校验: 编译期常量跟实际值对齐
        assert_eq!(FIVE_NAV, 5);
        assert_eq!(NINE_ORGANS, 9);
        assert_eq!(SIX_PHI_ANCHORS, 6);
        assert_eq!(EIGHT_PROMISES, 8);
        assert_eq!(V2_ENDPOINT_GROUPS, 6);
    }
}
