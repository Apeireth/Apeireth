# R179 GitHub 优秀项目调研 — tool-browser 模块 (Rust 原生 + Agent 框架)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R179
> **日期**: 2026-08-13
> **范围**: apeireth-tool-browser 当前实现 + 可升级的 Rust 原生 / Agent 框架
> **状态**: 调研为升级预备. 0 引入 GUI 依赖 (保留 ratatui-only 哲学).

---

## 0. 现状

apeireth-tool-browser 当前:
- Tier 1.3 (R140) 已真接
- 之前用 headless_chrome / 字符串 HTTP 抓取
- 没有 CDP (Chrome DevTools Protocol) 完整封装
- 没有 DOM 解析 + 元素定位
- 没有截图 + 视觉反馈

调研目的是确认: **Rust 原生浏览器自动化 + agent 框架的 SOTA 是什么, 我们的 tool-browser 应该升级到什么架构**.

---

## 1. Rust 原生浏览器自动化库

### 1.1 chromiumoxide (mattsse/chromiumoxide) — **RECOMMENDED**

- **GitHub**: https://github.com/mattsse/chromiumoxide
- **Stars**: ~1.5K, 11 contributors
- **License**: MIT
- **定位**: Rust port of Playwright
- **能力**:
  - 完整 CDP (Chrome DevTools Protocol) 封装
  - async/await 原生 (tokio 兼容)
  - Element handle + selector 引擎
  - Network interception (route + fulfill)
  - Screenshots + PDF
  - File upload / download
  - Tracing support
  - 多 tab / 多 context

**为什么强推荐**:
- 是 Rust 生态最完整的 Playwright 等价物
- 我们 tool-browser 当前缺的能力 (DOM 解析 / 元素定位 / 截图) 它都有
- API 设计与 Playwright 高度一致, 学习成本低
- MIT License, 0 限制

**集成草案**:
`
ust
// apeireth-tool-browser/src/cdp.rs
use chromiumoxide::{Browser, BrowserConfig, Page};

pub struct CdpBrowser {
    browser: Browser,
}

impl CdpBrowser {
    pub async fn launch(headless: bool) -> Result<Self, BrowserError> {
        let config = BrowserConfig::builder()
            .headless_mode(if headless { HeadlessMode::True } else { HeadlessMode::False })
            .build()?;
        let (browser, _handler) = Browser::launch(config).await?;
        Ok(Self { browser })
    }
    
    pub async fn page(&self, url: &str) -> Result<Page, BrowserError> {
        let page = self.browser.new_page(url).await?;
        Ok(page)
    }
}
`

### 1.2 headless_chrome (atroche-rs/headless_chrome) — 备选

- **Stars**: ~593
- **License**: MIT
- **现状**: 早期 Rust 浏览器库, 设计老
- **不选**: 长期不更新, chromiumoxide 完全超越

### 1.3 fantoccini (jnwng/fantoccini) — 备选

- **Stars**: ~528
- **License**: MIT/Apache-2.0
- **定位**: WebDriver (W3C) 客户端, 类似 Selenium
- **不选**: 需要独立 WebDriver server, 比 CDP 重
- **保留价值**: 跨浏览器测试场景 (Firefox / Safari)

### 1.4 spider (spider-rs/spider) — 爬虫而非浏览器

- **Stars**: 11.5K+, Tauri 团队维护
- **定位**: 异步爬虫, 自带并发 / 代理 / cookie
- **价值**: 用于 \"大批量 URL 抓取 + 提取\" 场景
- **集成**: 与 tool-fetch 的关系 — spider 是 fetch 批量版, browser 是交互版

---

## 2. AI Agent 浏览器框架 (Python / 跨语言)

### 2.1 browser-use (browser-use/browser-use) — **学习标杆**

- **GitHub**: https://github.com/browser-use/browser-use
- **Stars**: 76K+ (2026-08)
- **License**: MIT
- **定位**: \"Make AI agents use browsers like humans\"
- **核心能力**:
  - DOM 提取 → LLM-friendly text 转换
  - Multi-step navigation (L1 -> L2 -> L3)
  - 自带 playwright backend
  - Action space: click / type / scroll / wait / extract
  - Screenshot + visual feedback
- **关键设计**:
  - BrowserContext 抽象, 与 LLM agent loop 解耦
  - Action history, 可回放
  - Built-in prompt templates

**为什么必须学习**:
- 76K+ stars = 业内事实标准
- 设计模式 (LLM ↔ Browser 桥) 值得直接借鉴
- 我们可以用 chromiumoxide (Rust 端) + browser-use 的 LLM 交互协议 (跨语言)

**借鉴方案**:
- 不集成 browser-use 本身 (Python 依赖)
- 把 browser-use 的 LLM 交互协议移植到 Rust
- 我们的 peireth-tool-browser 用 chromiumoxide 做底层, 上面加一层 LlmAgentAdapter

### 2.2 BrowserOS (browseros-ai/BrowserOS) — 学习

- **License**: AGPL-3.0
- **定位**: Chromium fork + 内置 AI agent
- **学习点**: agent-first browser UX
- **不集成**: AGPL + 重 (整个 Chromium fork)

### 2.3 Skyvern (Skyvern-AI/Skyvern) — 学习

- **Stars**: 13K+
- **License**: AGPL-3.0
- **定位**: LLM + Computer Vision 自动化浏览器
- **学习点**: computer vision fallback when DOM 不可靠
- **不集成**: AGPL, Python 依赖

### 2.4 Stagehand (browserbase/stagehand) — 学习

- **License**: Apache 2.0 / MIT
- **定位**: AI browser automation framework
- **学习点**: ct / xtract / observe 三 API
- **价值**: 比 browser-use 协议更简洁

---

## 3. 网页内容提取 / 解析

### 3.1 scraper / scraper ChromeExtension — 备选

### 3.2 readability-rs (cfinke/readability-rs) — RECOMMENDED

- **GitHub**: https://github.com/cfinke/readability-rs
- **License**: MIT (port of Mozilla Readability.js)
- **能力**: 提取网页 main content, 去除 nav / ad / footer
- **集成**: tool-fetch 拿到 HTML 后, readability 提取正文

### 3.3 html2text / html5ever — 备选

- html5ever: Servo 的 HTML parser, 低层
- html2text: HTML -> text
- readability-rs 是更高层封装

---

## 4. 升级方案 (最终阶段执行)

### 4.1 短期 (1-2 days)

1. **替换底层**: tool-browser 内部用 chromiumoxide 替代手撸 HTTP
2. **加 screenshot**: page.screenshot() API 暴露
3. **加 element selector**: page.find_element(selector) API

### 4.2 中期 (3-5 days)

4. **加 LLM agent adapter**: 参考 browser-use 的 action space 设计
   `
ust
   pub enum BrowserAction {
       Click { selector: String },
       Type { selector: String, text: String },
       Scroll { direction: ScrollDir, amount: u32 },
       Wait { condition: WaitCondition },
       Extract { selector: String, attribute: String },
       Navigate { url: String },
   }
   `
5. **加 readability 集成**: tool-fetch 抓 HTML 后自动提正文
6. **加 network interception**: 用于 mock / record / replay

### 4.3 长期 (持续)

7. **多 tab 编排**: agent 在多个 tab 间切换
8. **Visual fallback**: screenshot + OCR 当 DOM 不可靠时
9. **跨浏览器**: WebDriver + fantoccini (Firefox / Safari)

---

## 5. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| chromiumoxide | ~500KB 编译产物 | MIT | 是 (CDP 底层) |
| readability-rs | ~50KB | MIT | 是 (内容提取) |
| tokio (已有) | - | MIT | 是 (异步) |

**总增加**: < 1MB 编译产物, 0 新增 License 风险.

---

## 6. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| tool-fetch (R174) | fetch = HTTP only; browser = JS rendered. 互补 |
| tool-codesearch (R181) | 独立 (代码搜索 vs 浏览器) |
| tool-shell (R140) | 独立 |
| council (R180) | browser 可能是 council advisor 之一 (WebResearchAdvisor) |

---

## 7. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 当前 tool-browser 公开 API 0 改 (chromiumoxide 集成在内部, 通过 trait 抽象)

---

## 8. 参考链接

- chromiumoxide: https://github.com/mattsse/chromiumoxide
- headless_chrome: https://github.com/atroche-rs/headless_chrome
- fantoccini: https://github.com/jnwng/fantoccini
- spider: https://github.com/spider-rs/spider
- browser-use: https://github.com/browser-use/browser-use
- BrowserOS: https://github.com/browseros-ai/BrowserOS
- Skyvern: https://github.com/Skyvern-AI/Skyvern
- Stagehand: https://github.com/browserbase/stagehand
- readability-rs: https://github.com/cfinke/readability-rs
- html5ever: https://github.com/servo/html5ever
- Chrome DevTools Protocol: https://chromedevtools.github.io/devtools-protocol/