# R183 GitHub 优秀项目调研 — tui 模块 (终端 UI 架构)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R183
> **日期**: 2026-08-13
> **范围**: apeireth-tui 当前架构 + 终端 UI 设计模式 SOTA
> **状态**: 调研为升级预备. 主人 R176 指示 \"先不接前端, 我们就纯后端调试补弱\". TUI 调研是为未来 R220+ 接入阶段准备.

---

## 0. 现状

apeireth-tui 53 src 文件, 255KB:
- 5 pages: bridge / dialogue / growth / history / settings
- 8 command (身体器官化): body / brain / ear / eye / hand / heart / memory / mind / voice
- 5 nav: help / session / settings / status / tools
- runtime_bridge.rs (R155 桥已建) — TUI 与真后端 runtime 集成入口
- 5 KB theme / observability / config_watcher
- backend.rs / cognition_live.rs — backend 抽象 + cognition 实时显示

**已用库**: ratatui + crossterm (Rust TUI 事实标准)
**架构**: 8 器官化命令 / 多 page / nav router / 5 KB observability
**当前状态**: 骨架可跑, 未接真后端 (R176 决定)

---

## 1. Rust TUI 生态

### 1.1 ratatui (ratatui/ratatui) — **当前在用, 强化**

- **GitHub**: https://github.com/ratatui/ratatui
- **Stars**: 13K+ (前 tui-rs)
- **License**: MIT
- **定位**: Rust TUI 事实标准
- **核心能力**:
  - immediate mode 渲染 (vs retained mode)
  - Layout 引擎 (Flexbox-style)
  - Widget 库 (Chart / Table / List / Paragraph / ...)
  - 跨 backend (crossterm / termion / termwiz)
  - async/await 友好

**强化方向**:
- 升级到最新 ratatui (0.28+ 拿到新 widget)
- 启用 tui-input (输入框)
- 启用 tui-textarea (多行编辑)
- 启用 ratatui-image (screenshot 显示 — 配合 tool-browser)
- 启用 tui-tree-widget (关系图可视化)

### 1.2 cursive (gyscos/cursive) — 备选

- **License**: MIT
- **定位**: retained mode TUI (类似 Qt)
- **不选**: 我们的 immediate mode 架构已稳定, cursive 切换成本高
- **保留价值**: 学习其 callback 风格, 我们 8 器官命令可借鉴

### 1.3 tui-rs-tree-widget (EdJoPaTo/tui-rs-tree-widget, 后续 ratatui-tree-widget) — 学习

- 树形显示
- 我们的关系图可视化 (relation 模块) 可直接用

### 1.4 tui-input / tui-textarea / ratatui-image — 推荐加

- 都是 ratatui 生态, MIT, 0 风险
- 总增加: < 200KB 编译产物

---

## 2. 终端 UI 设计模式 (跨语言 SOTA)

### 2.1 Bubble Tea (charmbracelet/bubbletea) — **RECOMMENDED 学习 (Elm 架构)**

- **GitHub**: https://github.com/charmbracelet/bubbletea
- **Stars**: 28K+
- **License**: MIT
- **语言**: Go
- **定位**: The fun, functional, stateful TUI framework
- **核心**: **TEA (The Elm Architecture)**
  - Model = 应用状态
  - Update(msg) -> (Model, Cmd) = 消息处理
  - View(model) -> String = 渲染
  - 单向数据流

**为什么必须学**:
- Elm 架构是 TUI 状态管理的最佳实践
- 我们的 8 器官命令 + 5 page 状态分散, 可以借鉴 Elm 架构统一
- 兄弟库 Bubbles (27K+) 提供 30+ 组件, 可借鉴设计

**借鉴方案 (草案)**:
`
ust
// apeireth-tui/src/elm/mod.rs
pub trait Model: Default {
    type Message: Send;
    fn update(&mut self, msg: Self::Message) -> Option<Cmd<Self::Message>>;
    fn view(&self, frame: &mut Frame);
}

pub struct App<M: Model> {
    model: M,
    cmds: Vec<Cmd<M::Message>>,
}

impl<M: Model> App<M> {
    pub async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.model.view(f))?;
            let event = read_event().await?;
            // route event to Msg
            // update model
            // execute cmds
        }
    }
}
`

### 2.2 Textual (Textualize/textual) — 学习 (Python)

- **Stars**: 28K+
- **License**: MIT
- **语言**: Python
- **核心**:
  - CSS-like 样式
  - Widget 化
  - 异步 + 反应式
- **学习点**: CSS-like 样式系统, 我们 theme.rs 可升级
- **不集成**: Python 依赖

### 2.3 Ink (vadimdemedes/ink) — 学习 (Node)

- React 风格的 TUI
- 我们 command 组件化设计可借鉴

### 2.4 blessed / neo-blessed (Node) — 历史

- 早期 Node TUI, 已被 Ink 超越

### 2.5 charm — Charm 工具链生态

- gum, vhs, soft-serve, skunk, freeze
- 我们 onboarding / config_watcher 可借鉴 gum 交互

---

## 3. 终端渲染增强

### 3.1 sixel / iterm2 / kitty 图形协议

- 终端支持图片 / 像素
- 我们 ratatui-image 可以显示 tool-browser screenshot
- 不是所有终端都支持, 默认 fallback ASCII

### 3.2 ANSI 24-bit color

- 现代终端都支持
- 我们 theme.rs 启用 24-bit

### 3.3 tmux / zellij integration

- 我们的 TUI 在 tmux 内可能有问题
- ratatui 0.28+ 已 fix 大部分 tmux 兼容
- 持续测试

---

## 4. 横向集成 (TUI 周边)

### 4.1 fzf (junegunn/fzf) — 学习

- fuzzy finder, 1K+ language binding
- 我们 command palette 可借鉴

### 4.2 delta (dan-delta) — 学习

- git diff 增强
- 我们 tool-git (未实装) 可借鉴

### 4.3 zellij (zellij-org/zellij) — 学习

- terminal multiplexer (tmux 现代替代)
- 我们 TUI 启动脚本可借鉴 zellij layout 自动化

### 4.4 wezterm (wez/wezterm) — 学习

- 终端模拟器, GPU 加速
- 不直接相关, 仅 wezterm 用户体验参考

### 4.5 helix-editor / lapce — 学习

- Rust 写的现代编辑器
- TUI 编辑体验可借鉴 (我们 command/hand 实际是编辑)

---

## 5. 升级方案 (R220+ TUI 接入阶段)

### 5.1 短期 (接入时, 1-2 days)

1. **runtime_bridge 完善**: TUI 接真后端 (R176 推迟的工作)
2. **tui-input / tui-textarea 加**: 多行编辑
3. **tui-tree-widget 加**: 关系图可视化
4. **ratatui-image 加**: tool-browser screenshot 显示

### 5.2 中期 (1-2 weeks)

5. **Elm 架构重构**: Model/Update/View 三件套, 8 器官命令状态统一
6. **Bubbles 风格组件库**: Chart / Table / List / Tree 标准化
7. **command palette**: fzf-style 模糊搜索命令

### 5.3 长期 (持续)

8. **CSS-like 样式系统**: 借鉴 Textual
9. **Sixel / iterm2 image protocol**: 终端显示图片
10. **VHS 录制集成**: 自动录 TUI 演示

---

## 6. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| ratatui (当前) | ~0 | MIT | 是 |
| tui-input | ~30KB | MIT | 是 |
| tui-textarea | ~50KB | MIT | 是 |
| tui-tree-widget | ~20KB | MIT | 是 |
| ratatui-image | ~200KB | MIT | 是 (screenshot) |

**总增加**: ~300KB 编译产物, 全 MIT, 0 风险

---

## 7. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| runtime_bridge (R155) | TUI 接真后端的桥 (R220+ 完善) |
| cognition_live | 实时显示 cognition 状态 (已 R155 接) |
| nav | 5 nav 路由, Elm 架构后统一 |
| organ | 8 器官命令, 状态统一后更易维护 |
| theme.rs | 升级到 CSS-like 样式系统 |
| council | TUI 拟人化基础 (CouncilMember 显示) |
| relation (R182) | tui-tree-widget 显示关系图 |
| tool-browser (R179) | ratatui-image 显示 screenshot |

---

## 8. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- TUI 公开 API: 0 改 (新架构在子模块内, 通过 trait 抽象)

---

## 9. 参考链接

- ratatui: https://github.com/ratatui/ratatui
- cursive: https://github.com/gyscos/cursive
- tui-rs-tree-widget: https://github.com/EdJoPaTo/tui-rs-tree-widget
- tui-input: https://github.com/sayanarijit/tui-input
- tui-textarea: https://github.com/rhysd/tui-textarea
- ratatui-image: https://github.com/benjajaja/ratatui-image
- Bubble Tea: https://github.com/charmbracelet/bubbletea
- Bubbles: https://github.com/charmacelet/bubbles
- Textual: https://github.com/Textualize/textual
- Ink: https://github.com/vadimdemedes/ink
- charm (gum/vhs): https://github.com/charmbracelet/gum
- fzf: https://github.com/junegunn/fzf
- delta: https://github.com/dandavison/delta
- zellij: https://github.com/zellij-org/zellij
- wezterm: https://github.com/wez/wezterm
- helix: https://github.com/helix-editor/helix
- Elm Architecture: https://guide.elm-lang.org/architecture/