# Scrapling 评估

## 机制（What it does）

- 核心功能：反爬网页爬取（adaptive web scraping），自动绕过 Cloudflare/F5/distil 等反爬
- 解决什么问题：requests/BeautifulSoup 在现代反爬站点（CF challenge、JS 渲染）直接 403 → Scrapling 自动选择爬取策略
- 关键技术：
  - 启发式指纹（自动探测反爬栈）
  - 多策略回退（HTTP fetch → headless browser → Spider-like fetch）
  - 自动 retry + 退避
  - 与 Scrapy 类似的 selector（CSS/XPath）

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-tool-fetch`（R149 Tier 1.5 unified fetch engine，吸收 VCP 7 plugins）
  - `apeireth-tool-browser`（浏览器自动化工具）
  - `apeireth-tool-web` / `apeireth-web`（Web 套件）
  - `composio-next`（已 clone 在 `research/source/composio-next/`，参考第三方 API 整合）
  - `playwright-mcp`（已 clone，参考浏览器 MCP 集成）
- 差异化优势：
  - Scrapling 解决「反爬」，APEIRETH `apeireth-tool-fetch` 是「统一 fetch 引擎」但未必处理反爬
  - Scrapling Python 实现，APEIRETH Rust 实现需要重写或 Python sidecar
- 可借鉴：
  - **启发式反爬探测**：在 `apeireth-tool-fetch` 加 `AntiScrapDetector` 模块，自动探测 CF challenge / rate-limit / JS-required
  - **多策略回退**：fetch → headless browser → raw socket 的三档回退（apeireth-tool-fetch 当前缺）
  - **自动 retry + 退避**：Scrapling 的退避策略可参考

## 吸收建议（Action items）

- P0 立即做：在 `apeireth-tool-fetch` 加 `AntiScrapDetector` trait + 启发式反爬栈探测（CF challenge 特征 = `Server: cloudflare` + `<title>Just a moment...</title>` 等）。
- P1 评估后做：策略回退链（HTTP → browser），与 `apeireth-tool-browser` 集成。
- P2 长期调研：JS 渲染场景下的 fingerprint rotation。
- 不做（重复 / 价值低）：Scrapling 完整 fork（Python 不兼容 Rust）。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 Scrapling 源码；本评估基于 GitHub README 公开信息 + 同类参考（`composio-next` / `playwright-mcp` 已 clone）推理。
- 未调研不写结论：Scrapling 的反爬特征库 / 退避算法具体参数 / 与 CF challenge 交互细节均为推理判断。如需落地建议，必须先实测 Scrapling 源码 + 在 `apeireth-tool-fetch` 上做 P0 POC 验证。
- 注：与 `apeireth-tool-fetch` (R149) 实际能力冲突时，以 `apeireth-tool-fetch` 现状为准（避免越权修改其它 agent 工作）。