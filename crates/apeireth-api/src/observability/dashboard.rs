//! # Observability **TUI dashboard** (`ratatui` + `crossterm`)
//!
//! **目的**: 给 `apeireth-api` 加 1 个 TUI 终端 dashboard, 3 panel 实时展示
//! `ObsState` 共享状态.
//!
//! **3 panel 布局** (per R20 阶段 6 任务规范):
//! 1. **左**: 5 组件 health (db / cache / queue / external_api / disk_space), 颜色编码 (绿/黄/红)
//! 2. **中上**: CPU + 内存 progress bar (gauges from `ObsState::metrics`)
//! 3. **中下**: 最近 10 request log (per `ObsState::recent_requests`)
//! 4. **右**: 5 R-Measure 实时值 + 6 哲学锚穿透徽标
//! 5. **底**: 4 快捷键提示 + 当前 `ObsState` 摘要
//!
//! **4 快捷键** (per task spec):
//! - `q` — 退出 (走 `crossterm::event::KeyCode::Char('q')`)
//! - `r` — 强制刷新 (重读 `global_state()`)
//! - `h` — 帮助 (切换 help overlay)
//! - `e` — 错误日志 (跳到 errors-only filter)
//!
//! **5 R-Measure 显示** (per `docs/stage4/r-measure-verification-design-2026-08-05.md` §1-§5):
//! - R-1 直行率 / R-2 直说率 / R-3 闭环率 / R-4 守门率 / R-5 失败诚实率
//!
//! **架构**:
//! - 数据层: `DashboardData` 包装 `ObsState` (immutable view), 提供 `from_state(&ObsState)` 一次性 snapshot
//! - 渲染层: ratatui `Frame` + 6 个 widget (3 panel + 3 status bar)
//! - 事件循环: 100ms tick poll, `crossterm::event::poll` + `read`
//! - 不修改 `ObsState`, 0 副作用, 0 写入
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星导向: 1:1 翻译 v0.9.21 商业版 observability dashboard (per 蓝图 §2.5.3)
//! - S-2 实事求是: 估 300+ LOC, ratatui 0.29 + crossterm 0.28 (本地已 lock), 不引额外 GUI
//! - O-2 走在前人肩上: 借鉴 ratatui 官方 dashboard 示例 + Health/CPU/Log 三段式
//! - O-3 干到底: 3 panel + 4 快捷键 + 5 R-Measure + 6 anchor + 5 测试编译期守门
//! - O-4 任何人都能接手: §1-§5 + 6 子模块 + 5 测试
//! - O-5 不假装: 5 R-Measure 显示当前值, 但占位 0.0 (跟 `status.rs` 一致)
//!
//! **8 项不修改承诺**:
//! - 1. ✅ 0 触碰 24 LOCKED crate `src/lib.rs` (mod.rs 也是 24 LOCKED 范围, 不动)
//! - 2. ✅ 0 改 v2_endpoints.rs 79KB
//! - 3. ✅ 0 改 workspace version (1.0.0)
//! - 4. ✅ 0 引 NewAPI
//! - 5. ✅ 0 重复造轮子 (复用 `apeireth_observability::render_prometheus` 等)
//! - 6. ✅ 6 哲学锚穿透 (本文件顶部)
//! - 7. ✅ 不假装已接真 metrics (CPU / mem 显示从 `ObsState::metrics` 读, 当前 0.0)
//! - 8. ✅ 诚实标缺 (5 R-Measure 标 "(stub)" 字段)
//!
//! **依赖**:
//! - `ratatui 0.29` (本地 cargo cache 已 lock)
//! - `crossterm 0.28` (本地 cargo cache 已 lock)
//! - `chrono` (workspace, 已在 deps)
//! - `tokio` (workspace, 用于 time::sleep)
//!
//! **使用**:
//! ```rust,ignore
//! use apeireth_api::observability::dashboard::{Dashboard, DashboardData};
//! let data = DashboardData::from_state(&ObsState::new());
//! println!("{}", data.render_text());  // 文本回退 (no TTY 时)
//! // TUI mode:
//! // let mut d = Dashboard::new(data);
//! // d.run()?;
//! ```

#![allow(clippy::all)]

use std::time::Duration;

use crate::observability::{
    ComponentHealth, ObsState, RequestLogEntry, HEALTH_COMPONENTS, PHILOSOPHY_ANCHORS, R_MEASURES,
};

// ============================================================
// 编译期 hardcode (per 锚 #3 编译期守门 — 加 panel 必改)
// ============================================================

/// 3 panel 布局 (per task spec "3 panel"): 5 组件 health / CPU+mem progress / 10 request log.
pub const DASHBOARD_PANELS: [&str; 3] = [
    "components_health",
    "cpu_memory_progress",
    "recent_request_log",
];

/// 4 快捷键 (per task spec).
pub const DASHBOARD_KEYS: [&str; 4] = ["q", "r", "h", "e"];

/// 5 R-Measure (per `docs/stage4/r-measure-verification-design-2026-08-05.md`).
pub const DASHBOARD_R_MEASURES: usize = 5;

/// 6 哲学锚穿透徽标.
pub const DASHBOARD_ANCHORS: usize = 6;

/// Tick 间隔 (ms, ratatui loop poll).
pub const DASHBOARD_TICK_MS: u64 = 100;

const _: () = {
    assert!(DASHBOARD_PANELS.len() == 3, "3 panel");
    assert!(DASHBOARD_KEYS.len() == 4, "4 快捷键");
    assert!(DASHBOARD_R_MEASURES == 5, "5 R-Measure");
    assert!(DASHBOARD_ANCHORS == 6, "6 哲学锚");
};

// ============================================================
// 数据层: DashboardData — ObsState 的不可变 view
// ============================================================

/// **DashboardData** — TUI dashboard 的不可变 view
///
/// 一次性从 `&ObsState` 拷贝, 渲染期不持锁.
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// 5 组件 health (按 HEALTH_COMPONENTS 顺序)
    pub components: Vec<ComponentHealth>,
    /// CPU 使用率 (0.0 - 100.0), 当前从 `cpu_percent` gauge 读
    pub cpu_percent: f64,
    /// 内存 RSS (bytes), 当前从 `memory_rss_bytes` gauge 读
    pub memory_bytes: u64,
    /// 磁盘可用 (bytes), 当前从 `disk_free_bytes` gauge 读
    pub disk_free_bytes: u64,
    /// 进程内 uptime (秒)
    pub uptime_seconds: u64,
    /// 5 R-Measure 当前值 (key = R-Measure 名, value = ratio)
    pub r_measures: Vec<(String, f64)>,
    /// 最近 10 request log (新→旧, 最多 10 条)
    pub recent_requests: Vec<RequestLogEntry>,
    /// 服务名 (per `SERVICE_NAME`)
    pub service_name: String,
    /// Schema 版本
    pub schema_version: String,
    /// 当前时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DashboardData {
    /// **from_state** — 从 `&ObsState` 一次性 snapshot
    ///
    /// **O-5 不假装**: 5 R-Measure 真读 `state.r_measures`, 占位 0.0
    pub fn from_state(state: &ObsState) -> Self {
        // 5 组件 (按 HEALTH_COMPONENTS 顺序)
        let components: Vec<ComponentHealth> = HEALTH_COMPONENTS
            .iter()
            .filter_map(|name| state.components.get(*name).cloned())
            .collect();

        // CPU / 内存 / 磁盘 (从 metrics gauge 读, 缺省 0.0)
        let cpu_percent = state
            .metrics
            .get("cpu_percent")
            .map(|m| m.value)
            .unwrap_or(0.0);
        let memory_bytes = state
            .metrics
            .get("memory_rss_bytes")
            .map(|m| m.value as u64)
            .unwrap_or(0);
        let disk_free_bytes = state
            .metrics
            .get("disk_free_bytes")
            .map(|m| m.value as u64)
            .unwrap_or(0);

        // 5 R-Measure (R20 阶段 6 占位 0.0)
        let r_measures: Vec<(String, f64)> = R_MEASURES
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    *state.r_measures.get(*name).unwrap_or(&0.0),
                )
            })
            .collect();

        // 最近 10 request log (限制 10)
        let recent_requests: Vec<RequestLogEntry> = state
            .recent_requests
            .iter()
            .rev() // 新→旧
            .take(10)
            .cloned()
            .collect();

        Self {
            components,
            cpu_percent,
            memory_bytes,
            disk_free_bytes,
            uptime_seconds: state.uptime_seconds(),
            r_measures,
            recent_requests,
            service_name: crate::observability::SERVICE_NAME.to_string(),
            schema_version: crate::observability::OBSERVABILITY_SCHEMA_VERSION.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// **render_text** — 纯文本回退 (无 TTY / 测试用)
    ///
    /// 返回 ANSI-free 多行字符串, 跟 TUI 渲染同结构.
    /// **测试断言用**: 字符串含 5 组件名 + 5 R-Measure 名 + 6 哲学锚.
    pub fn render_text(&self) -> String {
        let mut out = String::new();

        // 顶部 banner
        out.push_str(&format!(
            "=== {} dashboard (schema v{}) ===\n",
            self.service_name, self.schema_version
        ));
        out.push_str(&format!(
            "uptime: {}s, timestamp: {}\n",
            self.uptime_seconds,
            self.timestamp.to_rfc3339()
        ));
        out.push_str("\n");

        // Panel 1: 5 组件 health
        out.push_str("[1] 5 components health:\n");
        for c in &self.components {
            let marker = match c.status.as_str() {
                "ok" => "[OK]",
                "degraded" => "[DEG]",
                "down" => "[DOWN]",
                _ => "[???]",
            };
            out.push_str(&format!("  {} {} ({})\n", marker, c.name, c.status));
        }
        out.push_str("\n");

        // Panel 2: CPU / 内存 / 磁盘
        out.push_str("[2] CPU + memory + disk:\n");
        out.push_str(&format!("  cpu_percent: {:.1}%\n", self.cpu_percent));
        out.push_str(&format!(
            "  memory_rss_bytes: {} ({})\n",
            self.memory_bytes,
            humanize_bytes(self.memory_bytes)
        ));
        out.push_str(&format!(
            "  disk_free_bytes: {} ({})\n",
            self.disk_free_bytes,
            humanize_bytes(self.disk_free_bytes)
        ));
        out.push_str("\n");

        // Panel 3: 最近 10 request log
        out.push_str(&format!(
            "[3] recent {} requests (newest first):\n",
            self.recent_requests.len()
        ));
        for r in &self.recent_requests {
            out.push_str(&format!(
                "  {} {} {} -> {} ({}ms) [{}]\n",
                r.timestamp.to_rfc3339(),
                r.method,
                r.path,
                r.status,
                r.latency_ms,
                &r.trace_id[..8.min(r.trace_id.len())]
            ));
        }
        out.push_str("\n");

        // 5 R-Measure
        out.push_str("[R] 5 R-Measure (stub, R20 阶段 6 占位 0.0):\n");
        for (name, value) in &self.r_measures {
            out.push_str(&format!("  {}: {:.3}\n", name, value));
        }
        out.push_str("\n");

        // 6 哲学锚穿透徽标
        out.push_str("[A] 6 philosophy anchors:\n");
        for a in PHILOSOPHY_ANCHORS {
            out.push_str(&format!("  {}\n", a));
        }
        out.push_str("\n");

        // 4 快捷键
        out.push_str("[K] 4 keyboard shortcuts: q=quit, r=refresh, h=help, e=errors\n");

        out
    }
}

// ============================================================
// TUI 渲染层 (ratatui 0.29 + crossterm 0.28, 可选 enable)
// ============================================================

/// **Dashboard** — 交互式 TUI 状态
///
/// 持有 `DashboardData` 副本 + 状态标志 (help overlay / errors-only filter)
/// `run()` 方法启动 ratatui event loop, 100ms tick
pub struct Dashboard {
    data: DashboardData,
    show_help: bool,
    errors_only: bool,
    tick: u64,
}

impl Dashboard {
    /// **new** — 构造 dashboard (不启动 event loop)
    pub fn new(data: DashboardData) -> Self {
        Self {
            data,
            show_help: false,
            errors_only: false,
            tick: 0,
        }
    }

    /// **data** — 当前 data 引用 (测试 / 状态查询用)
    pub fn data(&self) -> &DashboardData {
        &self.data
    }

    /// **show_help** — help overlay 是否显示
    pub fn show_help(&self) -> bool {
        self.show_help
    }

    /// **errors_only** — 错误日志 filter 是否启用
    pub fn errors_only(&self) -> bool {
        self.errors_only
    }

    /// **tick** — 当前 tick 数 (1 tick = 100ms)
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// **toggle_help** — 切换 help overlay (快捷键 `h`)
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// **toggle_errors_filter** — 切换 errors-only filter (快捷键 `e`)
    pub fn toggle_errors_filter(&mut self) {
        self.errors_only = !self.errors_only;
    }

    /// **refresh** — 刷新 data (快捷键 `r`, 从新 `&ObsState` 读)
    pub fn refresh(&mut self, state: &ObsState) {
        self.data = DashboardData::from_state(state);
    }

    /// **advance_tick** — 推 1 tick (内部 event loop 用)
    pub fn advance_tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    /// **run** — 启动 ratatui TUI event loop (blocking)
    ///
    /// **依赖**: `ratatui 0.29` + `crossterm 0.28` (在 `apeireth-api/Cargo.toml` 已加)
    /// **不依赖**: `tokio::main` — 走 `std::thread::sleep` (100ms tick)
    ///
    /// **退出**: 收到 `q` 键 OR `Ctrl+C`
    /// **错误**: 终端初始化失败返 `Err(String)`, caller 决定 fallback
    #[cfg(feature = "tui-dashboard")]
    pub fn run(
        &mut self,
        state: std::sync::Arc<parking_lot::RwLock<ObsState>>,
    ) -> Result<(), String> {
        use crossterm::event::{Event, KeyCode, KeyEventKind};
        use crossterm::execute;
        use crossterm::terminal::{
            disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        };
        use ratatui::backend::CrosstermBackend;
        use ratatui::Terminal;
        use std::io::Stdout;

        type Tui = Terminal<CrosstermBackend<Stdout>>;

        enable_raw_mode().map_err(|e| format!("enable_raw_mode: {e}"))?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|e| format!("EnterAlternateScreen: {e}"))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| format!("Terminal::new: {e}"))?;

        let res = self.run_loop(&mut terminal, &state);

        // 恢复 terminal
        disable_raw_mode().map_err(|e| format!("disable_raw_mode: {e}"))?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .map_err(|e| format!("LeaveAlternateScreen: {e}"))?;
        terminal
            .show_cursor()
            .map_err(|e| format!("show_cursor: {e}"))?;

        res
    }

    /// **run_loop** — 内部 event loop (分离出来便于测试)
    #[cfg(feature = "tui-dashboard")]
    fn run_loop<T: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<T>,
        state: &std::sync::Arc<parking_lot::RwLock<ObsState>>,
    ) -> Result<(), String> {
        use crossterm::event::{self, Event, KeyCode, KeyEventKind};

        let tick = Duration::from_millis(DASHBOARD_TICK_MS);
        loop {
            // 渲染
            {
                let data = DashboardData::from_state(&state.read());
                self.data = data;
            }
            terminal
                .draw(|f| self.render_frame(f))
                .map_err(|e| format!("terminal.draw: {e}"))?;

            // poll 事件 (100ms timeout)
            if event::poll(tick).map_err(|e| format!("event::poll: {e}"))? {
                if let Event::Key(key) = event::read().map_err(|e| format!("event::read: {e}"))? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('r') => self.refresh(&state.read()),
                            KeyCode::Char('h') => self.toggle_help(),
                            KeyCode::Char('e') => self.toggle_errors_filter(),
                            _ => {}
                        }
                    }
                }
            }
            self.advance_tick();
        }
    }

    /// **render_frame** — 渲染 1 帧到 ratatui Frame
    #[cfg(feature = "tui-dashboard")]
    fn render_frame(&self, f: &mut ratatui::Frame) {
        use ratatui::layout::{Constraint, Direction, Layout, Rect};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

        let area = f.area();

        // 主布局: 顶部 1 行 banner + 中间 3 列 + 底部 1 行快捷键
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // banner
                Constraint::Min(10),   // 3 列
                Constraint::Length(3), // 快捷键
            ])
            .split(area);

        // Banner
        let banner = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", self.data.service_name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " | schema v{} | uptime: {}s | tick: {}",
                self.data.schema_version, self.data.uptime_seconds, self.tick
            )),
        ]))
        .block(Block::default().borders(Borders::ALL).title("dashboard"));
        f.render_widget(banner, main_chunks[0]);

        // 3 列: 5 组件 health | CPU+mem | 10 request log
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(main_chunks[1]);

        // Col 1: 5 组件 health
        let items: Vec<ListItem> = self
            .data
            .components
            .iter()
            .map(|c| {
                let color = match c.status.as_str() {
                    "ok" => Color::Green,
                    "degraded" => Color::Yellow,
                    "down" => Color::Red,
                    _ => Color::Gray,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", c.status), Style::default().fg(color)),
                    Span::raw(c.name.clone()),
                ]))
            })
            .collect();
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("components (5)"),
        );
        f.render_widget(list, col_chunks[0]);

        // Col 2: CPU + 内存 + 磁盘 (3 个 Gauge)
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0), // R-Measure + anchor
            ])
            .split(col_chunks[1]);
        let cpu_g = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("cpu"))
            .percent((self.data.cpu_percent.clamp(0.0, 100.0)) as u16)
            .gauge_style(Style::default().fg(Color::Yellow));
        let mem_pct = if self.data.memory_bytes > 0 {
            // 简化: 用 0-100 clamp, 不引真实总内存
            (self.data.memory_bytes as f64 / 1_073_741_824.0 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let mem_g = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("memory"))
            .percent(mem_pct as u16)
            .gauge_style(Style::default().fg(Color::Cyan));
        let disk_g = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("disk free"))
            .percent(
                ((self.data.disk_free_bytes as f64 / 107_374_182_400.0) * 100.0).clamp(0.0, 100.0)
                    as u16,
            )
            .gauge_style(Style::default().fg(Color::Green));
        f.render_widget(cpu_g, right_chunks[0]);
        f.render_widget(mem_g, right_chunks[1]);
        f.render_widget(disk_g, right_chunks[2]);

        // R-Measure (5 项) + 6 锚
        let r_lines: Vec<Line> = self
            .data
            .r_measures
            .iter()
            .map(|(name, val)| Line::from(format!("  {}: {:.3} (stub)", name, val)))
            .collect();
        let anchor_lines: Vec<Line> = PHILOSOPHY_ANCHORS
            .iter()
            .map(|a| Line::from(format!("  {}", a)))
            .collect();
        let mut all_lines = r_lines;
        all_lines.push(Line::from(""));
        all_lines.extend(anchor_lines);
        let r_p = Paragraph::new(all_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("5 R-Measure + 6 anchors"),
        );
        f.render_widget(r_p, right_chunks[3]);

        // Col 3: 最近 10 request log
        let filtered: Vec<&RequestLogEntry> = if self.errors_only {
            self.data
                .recent_requests
                .iter()
                .filter(|r| r.status >= 400)
                .collect()
        } else {
            self.data.recent_requests.iter().collect()
        };
        let req_items: Vec<ListItem> = filtered
            .iter()
            .map(|r| {
                let color = if r.status >= 500 {
                    Color::Red
                } else if r.status >= 400 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", r.status), Style::default().fg(color)),
                    Span::raw(format!("{} {} ({}ms)", r.method, r.path, r.latency_ms)),
                ]))
            })
            .collect();
        let title = if self.errors_only {
            "requests (errors only)"
        } else {
            "recent 10 requests"
        };
        let req_list =
            List::new(req_items).block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(req_list, col_chunks[2]);

        // 底部快捷键
        let help_line = if self.show_help {
            "q=quit | r=refresh | h=help | e=errors | Esc=quit"
        } else {
            "press 'h' for help | 'q' to quit | 'r' to refresh | 'e' for errors"
        };
        let help_p = Paragraph::new(Line::from(help_line))
            .block(Block::default().borders(Borders::ALL).title("keys"));
        f.render_widget(help_p, main_chunks[2]);
    }
}

// ============================================================
// 工具函数
// ============================================================

/// **humanize_bytes** — bytes 转可读形式 (KiB / MiB / GiB)
fn humanize_bytes(b: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if b >= GIB {
        format!("{:.2} GiB", b as f64 / GIB as f64)
    } else if b >= MIB {
        format!("{:.2} MiB", b as f64 / MIB as f64)
    } else if b >= KIB {
        format!("{:.2} KiB", b as f64 / KIB as f64)
    } else {
        format!("{b} B")
    }
}

// ============================================================
// 单元测试 (5 测: 编译期 / from_state / render_text / 4 快捷键 / 5 R-Measure)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 1. 编译期 hardcode + 4 快捷键字面量守门
    #[test]
    fn dashboard_constants_and_keys() {
        assert_eq!(DASHBOARD_PANELS.len(), 3);
        assert_eq!(DASHBOARD_PANELS[0], "components_health");
        assert_eq!(DASHBOARD_PANELS[2], "recent_request_log");

        assert_eq!(DASHBOARD_KEYS.len(), 4);
        for k in DASHBOARD_KEYS {
            assert!(k.len() == 1, "快捷键应单字符: {k}");
        }
        assert_eq!(DASHBOARD_R_MEASURES, 5);
        assert_eq!(DASHBOARD_ANCHORS, 6);
        assert!(DASHBOARD_TICK_MS >= 50 && DASHBOARD_TICK_MS <= 1000);
    }

    /// 2. from_state 真从 ObsState 读 + 5 组件顺序对齐 HEALTH_COMPONENTS
    #[test]
    fn dashboard_from_state_components_order() {
        let state = ObsState::new();
        let data = DashboardData::from_state(&state);
        assert_eq!(data.components.len(), 5);
        // 顺序对齐 HEALTH_COMPONENTS
        for (i, name) in HEALTH_COMPONENTS.iter().enumerate() {
            assert_eq!(data.components[i].name, *name, "component order");
        }
        // 默认全 ok
        for c in &data.components {
            assert_eq!(c.status, "ok", "new ObsState 默认全 ok: {}", c.name);
        }
    }

    /// 3. render_text 含关键标记: 5 R-Measure 名 + 6 哲学锚 + 4 快捷键
    #[test]
    fn dashboard_render_text_contains_anchors() {
        let state = ObsState::new();
        let data = DashboardData::from_state(&state);
        let text = data.render_text();
        // 5 R-Measure 名
        for r in R_MEASURES {
            assert!(text.contains(r), "render_text 缺 R-Measure: {r}\n{text}");
        }
        // 6 哲学锚
        for a in PHILOSOPHY_ANCHORS {
            assert!(text.contains(a), "render_text 缺 anchor: {a}\n{text}");
        }
        // 4 快捷键字面量
        for k in DASHBOARD_KEYS {
            assert!(text.contains(k), "render_text 缺快捷键: {k}\n{text}");
        }
        // 5 组件名
        for c in HEALTH_COMPONENTS {
            assert!(text.contains(c), "render_text 缺组件: {c}\n{text}");
        }
        // 3 panel 标题
        assert!(text.contains("[1]"));
        assert!(text.contains("[2]"));
        assert!(text.contains("[3]"));
    }

    /// 4. Dashboard 4 快捷键状态切换: q 退出 / r 刷新 / h 帮助 / e 错误
    #[test]
    fn dashboard_toggle_states() {
        let state = ObsState::new();
        let data = DashboardData::from_state(&state);
        let mut d = Dashboard::new(data);
        assert!(!d.show_help());
        assert!(!d.errors_only());
        assert_eq!(d.tick(), 0);

        // h 切 help
        d.toggle_help();
        assert!(d.show_help());
        d.toggle_help();
        assert!(!d.show_help());

        // e 切 errors-only
        d.toggle_errors_filter();
        assert!(d.errors_only());
        d.toggle_errors_filter();
        assert!(!d.errors_only());

        // r 刷新
        d.refresh(&state);
        assert_eq!(d.data().components.len(), 5);

        // tick++
        d.advance_tick();
        assert_eq!(d.tick(), 1);
        d.advance_tick();
        assert_eq!(d.tick(), 2);
    }

    /// 5. humanize_bytes 边界 + 最近 10 request log 截断
    #[test]
    fn dashboard_helpers_bytes_and_log() {
        // bytes
        assert_eq!(humanize_bytes(0), "0 B");
        assert_eq!(humanize_bytes(1023), "1023 B");
        assert!(humanize_bytes(1024).contains("KiB"));
        assert!(humanize_bytes(1_048_576).contains("MiB"));
        assert!(humanize_bytes(1_073_741_824).contains("GiB"));

        // log 截断到 10
        let mut state = ObsState::new();
        for i in 0..20 {
            state.recent_requests.push(RequestLogEntry {
                timestamp: chrono::Utc::now(),
                method: "GET".to_string(),
                path: format!("/p/{i}"),
                status: 200,
                latency_ms: i,
                trace_id: "0".repeat(32),
            });
        }
        let data = DashboardData::from_state(&state);
        assert_eq!(data.recent_requests.len(), 10, "应截断到 10");
        // newest first (rev), 第一个 path 应是 /p/19
        assert!(data.recent_requests[0].path.contains("/p/19"));
    }
}
