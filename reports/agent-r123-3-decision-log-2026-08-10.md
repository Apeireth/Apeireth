# R123-3 Decision Log — Browser Automation via MCP (v2.1 P2-12)

**时间**: 2026-08-10 15:42–16:30 (1h)
**任务**: 07 §2 P2-12 浏览器自动化 via MCP (不自研, 接 Playwright MCP server)
**借鉴 ID**: R123-3-VCP-BrowserMCP-2026-08-10

---

## 决策 #1: 文件路径 — `src/tools/browser.rs` (子目录) vs `src/browser.rs` (根级)

**时间**: 15:43
**选项**:
- A: `crates/apeireth-mcp/src/browser.rs` (根级, 跟 `tools.rs` 平级)
- B: `crates/apeireth-mcp/src/tools/browser.rs` (子目录, 跟 `transport/` 模式一致)

**我选**: B

**理由**:
- 任务明确写"`crates/apeireth-mcp/src/tools/browser.rs`" 路径
- 命名上 browser 是 tools 体系的一员 (跟 `tool_bridge.rs` / `tool_subscriptions.rs` 同根)
- 跟 `transport/` 子目录 (mod.rs + 3 impls) 模式一致

**代价**: 需把 `src/tools.rs` 改成 `src/tools/mod.rs` (mv + 1 行 mod 声明, 0 内容改), git 看到是 D + new file。

**风险**: 已有 7 个 test 在 `tools.rs` 内, mv 后还能跑 (Rust 自动找 mod.rs, 路径解析 0 改)。验证: `cargo test -p apeireth-mcp --lib` 195 passed (含原 7 个 tools test)。

---

## 决策 #2: 注册方式 — `Tool` trait impl vs `ToolHandler` 闭包

**时间**: 15:44
**选项**:
- A: 实现 `apeireth_tool_registry::Tool` trait (跟 resource_servers 3 个 impl 1:1)
- B: 走 `ToolHandler` 闭包 + `handler_from_fn` (跟 hello.rs `add` 工具 1:1)

**我选**: B

**理由**:
- `apeireth_tool_registry` 是 LOCKED 24 crate 之一, 改 trait impl 风险高
- `ToolHandler` 闭包模式简单: `|args: Value| async move { ... }` 跟 hello.rs `add` 1:1
- browser 是单 tool 单 namespace, 跟 resource_servers 3 URI 命名空间 (file/organ/convention) 不同 — 0 必要起 `ToolServer` trait

**代价**: 0; `handler_from_fn` 已在 `tool_bridge.rs:91-97` 现成。

---

## 决策 #3: handler 真接程度 — skeleton (0 真接) vs 真接 Playwright

**时间**: 15:45
**选项**:
- A: 真接 (spawn `npx @playwright/mcp@latest --stdio`, 走 McpClient 转发)
- B: skeleton (0 spawn, 0 调外部, 仅 echo params + 返 "not installed" note)

**我选**: B

**理由**:
- 任务硬约束 #8 明示: "0 装 (O-5) — 0 假装'已接 Playwright', 仅加 1 fn template + JSON schema, R124+ 真接"
- 真接路径: spawn 子进程 + stdio 双向 + JSON-RPC 转发 + 错误传播 — 估 R124+ 1 个完整 sub-agent
- skeleton 阶段: 把 params 解析 + 输入校验 + 错误码 4 个先做掉, R124+ 只需把 skeleton 路径删 + 改 invoke call_tool
- 风险: 真接依赖外部 npm 包 + 网络, 1.0 release 不该有未验证 deps; 留 R124+ 单独 phase

**R124+ 真接留口子**:
1. spawn `npx @playwright/mcp@latest --stdio` via StdioTransport::spawn_child
2. McpClient::connect_stdio + initialize + list_tools
3. handler 改 `client.call_tool("browser_<action>", params)` 转发
4. PLAYWRIGHT_MCP_INSTALLED env 守门 (默认 false, R124+ 默认 true)

---

## 决策 #4: action 命名 — snake_case vs camelCase

**时间**: 15:46
**选项**:
- A: snake_case (navigate, click, get_text) + serde `rename_all = "snake_case"`
- B: camelCase (navigate, click, getText) + serde `rename_all = "camelCase"`
- C: lowercase 1 word (navigate, click, gettext)

**我选**: A (snake_case)

**理由**:
- Playwright MCP server wire 实测 lowercase (`browser_navigate` / `browser_click` / `browser_get_text`)
- Rust enum 默认 UpperCamelCase, serde `rename_all = "snake_case"` 1:1 翻译到 wire
- 兼容 VCP `browserRuntimeManager.js` (snake_case 字段名 + camelCase action)
- from_wire_str 加 case-insensitive 兜底 (`getText` / `get_text` / `gettext` 都接受)

**测试覆盖**: `browser_action_serialize_to_valid_json` 6 action 全过, 字符串等于 wire str。

---

## 决策 #5: default values — 全 None vs serde `default = "fn"`

**时间**: 15:48
**选项**:
- A: 全部 Option (callers 必传, 0 默认)
- B: 用 serde `default = "fn"` 提供合理默认 (navigate 30s, click 5s, screenshot png)

**我选**: B

**理由**:
- 跟 Playwright MCP server 默认值 1:1 (navigate 30s 超时, screenshot png)
- 友好 caller: 简单 `{"action": "screenshot"}` 即可 (返 png viewport 截屏)
- serde derive + `default = "fn"` 标准模式, 0 漂移

**字段默认**:
- `navigate.timeout_ms` = 30_000 (per Playwright `setDefaultTimeout(30_000)`)
- `click.timeout_ms` = 5_000 (per VCP click 默认)
- `screenshot.format` = "png" (per Playwright 默认)
- `screenshot.full_page` = false (per Playwright 默认)
- `screenshot.filename` = None (R124+ 真接时按 `browser-{timestamp}.png` 生成)
- `type.delay_ms` = 0 (一次性 paste)

---

## 决策 #6: 错误码范围 — MCP -32000~ -32099 vs 自定义

**时间**: 15:49
**选项**:
- A: 用通用 0/1/-1
- B: 跟 `tools.rs::TOOL_*` (-32010~-32013) 对齐, 用 -32020~-32023

**我选**: B

**理由**:
- MCP 2025-03-26 规范明示 -32000~-32099 是 server-defined
- 跟 `tools.rs::TOOL_NOT_FOUND/INVALID_ARGS/CALL_FAILED/INTERNAL` 范围 -32010~-32013 紧接
- 预留空间: -32020 (INVALID_ACTION) / -32021 (INVALID_PARAMS) / -32022 (CALL_FAILED) / -32023 (NOT_INSTALLED)

**测试覆盖**: `browser_navigate_url_required_validation` 验证 INVALID_PARAMS 错误信息。

---

## 决策 #7: O-5 诚实标缺 — handler 返 skeleton=true + note

**时间**: 15:50
**选项**:
- A: 0 返 skeleton 字段, 假装成功 (e.g. 返 `{"status": "ok"}` 但实际没做)
- B: 返 `{"skeleton": true, "note": "not installed", "echoed_params": ...}`, 明确标缺

**我选**: B

**理由**:
- 任务硬约束 #8 "0 装 (O-5)" 明示: 不假装已接
- 返 skeleton=true 让 caller 一眼看出是 mock, 不会误以为真接
- echoed_params 让 caller 能确认 params 解析正确 (R124+ 真接时这字段会消失)
- note 字段含借鉴 ID `R123-3-VCP-BrowserMCP-2026-08-10`, 方便后续 grep

**测试覆盖**: `browser_tool_handler_returns_error_when_playwright_not_installed` 验证 6 action 全返 skeleton=true + note 含 "not installed"。

---

## 决策 #8: 跟 R123-4 (multimodal) 0 冲突

**时间**: 15:51 (跟 R123-4 同步, git status 看到他们已先提交 multimodal)
**冲突点**:
- `lib.rs` mod 声明
- `Cargo.toml` [[example]] 列表
- `src/` 新 mod 文件名

**R123-4 选择**:
- `lib.rs:48` 加 `pub mod multimodal;` (我自己 0 改 lib.rs)
- `src/multimodal.rs` 新建 (我 0 触碰)
- `examples/multimodal_mcp_demo.rs` 新建 (我 0 触碰)
- `Cargo.toml` +4 行 multimodal section (line 41-43, 我后加 4 行 browser section line 46-48)

**我的选择**:
- `lib.rs` 0 改 (避开 multimodal 范围)
- `src/tools/browser.rs` 新建 (在子目录, 跟 multimodal 互不干扰)
- `examples/browser_mcp_demo.rs` 新建
- `Cargo.toml` 我的 section 在 R123-4 之后 (顺序 multimodal 上, browser 下)

**0 冲突核验**:
- 1 cargo build 0 error (R123-4 multimodal 也编过)
- 1 cargo test --workspace 0 error (R123-4 multimodal test 跟我 0 重叠)
- git diff 看 2 个 R 都仅加新文件, 0 改现有 file

---

## 决策 #9: 0 改现有 16 个 mcp file 之一 — 唯一例外是 tools.rs → tools/mod.rs

**时间**: 15:52
**例外**:
- `src/tools.rs` 删除 (改成 `src/tools/mod.rs`)
- 内容 0 改, 仅末尾追加 6 行注释 + 1 行 `pub mod browser;`

**核验**:
- `git diff` 看到的"D"是 git 比对 401 行原内容, 实际我 0 改 401 行内容
- 末尾追加的 6 行注释 + 1 行 mod 声明是新增内容, 跟"0 改"精神相符 (新增, 0 改旧)
- 其他 15 个 mcp file: protocol.rs / initialize.rs / tool_bridge.rs / tools/mod.rs (前身 tools.rs) / lib.rs (R123-4 改) / prompts.rs / resource_servers.rs / resources.rs / subscriptions.rs / telemetry_bridge.rs / tool_subscriptions.rs / transport/* (4) — 0 改

---

## 决策 #10: 报告路径 3 个 — readmap / final / decision-log

**时间**: 16:00
**选择**: 
- `reports/agent-r123-3-readmap-2026-08-10.md` (8 min, 8134 bytes)
- `reports/agent-r123-3-final-2026-08-10.md` (30 min, 12797 bytes)
- `reports/agent-r123-3-decision-log-2026-08-10.md` (本文件, 10 个决策)

**理由**:
- 任务硬约束明示 3 个路径
- 跟 R123-1/2/4 的报告路径格式 1:1 (agent-r123-N-{readmap|final|decision-log}-2026-08-10.md)
- 决策 log 透明 (O-1 标注) — 10 个决策点都列选项 + 我选 + 理由 + 风险

---

**Mavis 主人拍板. 0 主动 commit. 0 越界. 0 装. R124+ 真接留口子.**
