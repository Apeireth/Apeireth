# 全库中性化清污 —— 统一风格指南（所有执行者必须逐条遵守）

你正在对 Windows 仓库 `C:\Users\31683\Apeireth-rust` 做**历史存量清污**：把源码与文档的
注释/文档字符串/测试名/文案里的【第三方产品名】与【来源措辞】改成中性工程表述。
**最高纪律：防清污变破坏。每处改动只动注释/字符串文案/测试名，代码语义零触碰。**

## 0. 绝对禁区（碰了就是破坏）
- **保留类目录/文件不许动**（连一个字符都不改）：
  `docs/archive/**`、`docs/_archived/**`、`reports/**`、`research/**`、`artifacts/**`、
  `legacy/**`、`CHANGELOG.md`、任意 `README*`（任何层级）、`OSS_NOTICE.md`、
  `THIRD-PARTY-NOTICES.md`、`NOTICE`、`LICENSE*`、`scripts/check-neutral-terms.ps1`。
- **不许动代码语义**：标识符（变量/函数/类型/模块名——唯一例外见 §4 测试名）、字符串字面量的
  运行时值、env 变量名、端点 URL、文件路径、命令文本、配置值、JSON payload 值、正则语义。
- **不许动真实路径字符串**：注释/文档里出现的 `legacy/…`、`docs/…`、`research/…`、`reports/…`、
  `crates/…`、`frontend/…`、`scripts/…` 等路径引用必须**逐字节保持原样**（包括
  `legacy/donor/…` 这类目录名里带 donor 的路径）。路径里的词不是本战役要清的措辞。
- **不许动**：日期、数字、账目值、版本号、批次号（R32-1/R129-5/决策 #N/台账 M2/O-2/S-2）、
  自家沿革词（v1/v2/RC/v2.0.0-rc.1）、产品专名（Apeireth/Ash/Ember/HUD）。
- **不许改行数结构**（md 表格的 `|` 列数、缩进、代码块围栏必须原样）；逐行就地替换措辞，不重排段落。
- 不写 CHANGELOG，不 git commit，不新建业务文件。

## 1. 禁词类一：来源措辞（必须清零）
| 禁词 | 中性替换（按语境选） |
|---|---|
| 移植自 / 移植来源 / 移植版 / 真移植 / 移植 | 「实现于」「语义对齐」「工程实现」「真实现」「v2 实现」；语义冗余则删词/删句 |
| 借鉴自 / 借鉴 / 参照实现 | 「对齐」「语义对齐」「同类工程做法」「设计惯例」「公开设计」；语义冗余则删词/删句 |
| 原实现 | 「修复前形态」「修复前」「既有实现」（审计修复注释场景优先「修复前」） |
| 1:1 翻译 / 1:1 直接翻译 / 1:1 对齐(第三方) | 「语义对齐」「实现于」「工程实现」 |
| donor / Donor（作来源限定词时） | 直接删限定词（Donor default → Default）；必要时「既有实现」「基线 baseline」 |
| 前任代码 / 上游(指来源代码时) | 删词或「既有实现」 |

**例**：
- `//! E4 Curiosity 器官真实现 (v2 移植版, per \`legacy/donor/apeireth-companion/src/curiosity.rs\`).`
  → 该括号是纯出处说明且含路径：按 §3(c) 处理（删出处括号 → `//! E4 Curiosity 器官真实现（v2）。`）
- `/// **Auth 5 组件容器** (1:1 翻译 \`apeireth-api::auth::AuthPipeline\`).`
  → `/// **Auth 5 组件容器**（语义对齐 \`apeireth-api::auth::AuthPipeline\`）。`（自有 v1 crate 名+路径保留）
- `/// Donor description length cap.` → `/// Description length cap.`
- `/// donor frozen-task 指数退避 1s→2s→4s` → `/// frozen-task 指数退避 1s→2s→4s`
- `// H2 修复 (2026-09-24 审计): 原实现是 \`permission.l4.requires_ha || true\``
  → `// H2 修复 (2026-09-24 审计): 修复前是 \`permission.l4.requires_ha || true\``
- `Recovered from \`legacy/donor/apeireth-cognition/src/calibration.rs\`.`（纯出处行）→ **整行删除**
- 「1:1 映射」「1:1 对应」「[OK 1:1]」是技术比率表述 → **保留不动**。
- 「上游」在"上游服务/上游 HTTP 错误/上游审批/traceparent 上游/依赖上游版本"等架构与供应链语境
  → **保留不动**；仅当指"代码来自上游项目"时才清。

## 2. 禁词类二：第三方产品/项目/公司名（注释/文档文案里出现即清）
- **必清清单（不限于）**：DSH、DeepSeek Harness、DeepSeekHarness、dsh-web-app、dsh-app://、
  dsh-client-locale、Harness-R1、gitleaks、LiteLLM、opencode、OpenCog、gemini-cli、claude-code、
  LangGraph、CrewAI、Cua、Serena、MetaGPT、OpenClaw、Hermes、LoopX、PenguinHarness、Kimi Code、
  CopilotKit、NEKO、open-llm-vtuber、airi、firefly、mio、reverse-skill、jimmyxiao2009、
  微信、QQ、钉钉、飞书、ChatGPT、Claude(作产品名时)、Gemini(作产品名时)、Cursor(IDE 时)、
  Notion、Obsidian、Slack、Discord、Telegram、Copilot、VCP / vcp / VCPToolbox、Roam、Logseq 等。
- **替换**：语义需要 → 「同类工程做法」「设计惯例」「同类即时通讯工具」「同类 Agent 框架」
  「同类检索工具」「业界通用做法」；不需要 → **删句/删括号**。
  **禁止**用「某开源实现」「某某产品」这类含糊指代替换（那等于没清）。
- 品牌清单（一串项目名）→ 聚合为「同类工程 N 项」等中性汇总，保留数量/结论/表格结构。
- **保留类技术名（不许清）**：
  - 协议/标准名：HTTP/HTTPS/JSON/WebSocket/SSE/MCP/RPC/TLS/DNS/WASM/SSML/multipart/RFC xxxx/
    GFM/KaTeX/Shiki/SPDX/MIT/Apache-2.0/RUSTSEC-xxxx。
  - **线协议/API 方言名**（本产品真实实现的接口形状）：OpenAI-compatible、OpenAI Chat Completions、
    OpenAI Realtime、Anthropic Messages API、Anthropic voice 规范、LiveKit 协议、MiniMax 接口。
  - 代码事实引用：env 变量名（OPENAI_API_KEY、APEIRETH_*）、能力/供应商 ID
    （`provider.openai-compatible`）、端点 URL（api.openai.com、api.anthropic.com）、模型 ID
    （deepseek-v4-flash、anthropic/claude-3-5）、密钥格式样例（sk-ant-voice-…、xoxb-…）、
    真实文件名（.gitleaks.toml、Cargo.toml）、依赖/crate/npm 包名（tokio、serde、reqwest、
    sqlite-vec、`@anthropic-ai/voice`、vcpkg、pyo3）、工具链名（cargo、kani、clippy、pnpm、Tauri、
    Svelte、Electron、Node.js、PowerShell、Playwright、cargo-about）、平台名（Windows/macOS/Linux）、
    学术文献引用（ACT-R、Gerstner & Kistler、Golub & Van Loan）。
  - 凭据泄漏签名类型名（SlackToken、xoxb- 前缀等安全扫描器词汇）→ 保留（技术事实）。
  - 供应商能力事实（如"DeepSeek 没有 embeddings 端点"这类运维指引里对已接入 provider 的客观描述）
    → 保留；纯比较/夸赞/范式引用（"微信范式""像 Cursor 一样"）→ 清。
- CSS 的 `cursor:`、代码标识符 `LogCursor`/`let cursor`、`.cloned()` 方法等假阳性 → 一律不动。

## 3. 含路径行的处理（关键！决定验收成败）
- (a) **纯路径引用行**（行的内容就是指向 `legacy/…`/`docs/…` 的指针，无其他禁词）→ **整行不动**。
- (b) **纯出处信用句**（如 `//! Recovered from \`legacy/donor/x/y.rs\`.`、`**移植来源**: v1 \`legacy/donor/…\``）
  且删掉后不损失工程信息 → **整行删除**（规则明示允许"整句删除"）。
- (c) **混合行**（有工程信息 + 出处措辞 + 路径）：删掉出处括号/从句（连同其中的路径），保留工程信息；
  若路径是**承载信息**（测试基线位置、夹具位置、规格指针、"基线在 X"）→ **整行不动**，并写进豁免报告。
- 判据：改写后的行里绝不能再出现 `donor`/`移植`/`借鉴`/`原实现`/`1:1 翻译`/DSH 等任何禁词
  ——否则验收扫描器会对该行命中。宁可少改、不可让禁词出现在你新增/改写的行里。

## 4. 测试名（唯一允许动的标识符）
以下测试函数名含 donor，**逐一改名**（只改名字，不动测试体；注释里引用旧名的同步改）：
- `activation_matches_donor_formula_exact_values` → `activation_matches_baseline_formula_exact_values`
- `defaults_match_donor_71gb_constants` → `defaults_match_baseline_71gb_constants`
- `machine_id_parsers_roundtrip_donor_fixtures` → `machine_id_parsers_roundtrip_baseline_fixtures`
- `ranks_match_donor_table` → `ranks_match_baseline_table`
- `default_domain_weights_match_donor` → `default_domain_weights_match_baseline`
- `defaults_match_donor_constants` → `defaults_match_baseline_constants`
- `normalize_mcp_result_from_donor_snake_case` → `normalize_mcp_result_from_baseline_snake_case`
其他测试名若含禁词（如移植/借鉴/Dsh），同理改成中性名（保持 snake_case，语义不变）。
局部变量 `donor`、函数 `Get-GitleaksAllowlist` 等**非测试**标识符不动。

## 5. 各文件类型的改动面
- `.rs`：仅 `//`、`///`、`//!`、`/* */` 注释与文档字符串、测试函数名。字符串字面量**不改**
  （除非断言两侧同一文案同步改且确属文案——拿不准就不改，写进豁免报告）。
- `.ts/.svelte/.mjs/.cjs`：仅注释（`//`、`/* */`、`<!-- -->`）与 UI 文案字符串
  （UI 文案指用户可见的中性描述，不指协议字段值）。
- `.ps1/.sh`：仅 `#` 注释与 Write-Output 文案；命令语义/正则表不动。
- `.md`：正文措辞；**代码块里的命令、路径、配置、标识符不动**（代码块里的注释可以清）；
  表格行列结构不动；链接 URL 不动（URL 里含品牌名的，整条引用按 §2 删句或保留 URL 但去掉周边赞语）。
- `.toml/.json/.yml/.yaml`：只动注释；键值不动。

## 6. 自检（完成后必须跑，把结果写进报告）
```
git grep -n -E '移植|借鉴|原实现|参照实现|1:1\s*翻译|donor|前任代码' -- <你的目录>
git grep -n -i -E '\bDSH\b|DeepSeek\s*Harness|gitleaks|LangGraph|CrewAI|LiteLLM|opencode|OpenCog|gemini-cli|claude-code|Harness-R1|VCP|微信|QQ' -- <你的目录>
```
要求：命中数为 0，或每一处残留都在你的报告豁免清单里（说明原因：真实路径字符串 /
代码标识符 / 供应商协议事实 / 保留类文件）。同时 `git diff --stat` 核对自己没碰禁区文件。

## 7. 报告格式（返回给调度者）
1. 改动文件数与清污句数（约数即可）；2. 主要替换范式 3-5 例（原文→新文）；
3. 豁免残留清单（文件:行 + 原因）；4. 你发现的任何拿不准、已保守处理的点。
