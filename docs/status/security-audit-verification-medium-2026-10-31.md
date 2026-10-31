# 安全审计复核 · 6 条 medium 发现（2026-10-31）—— 逐条对照当前代码

[Document-Meta]
Document: docs/status/security-audit-verification-medium-2026-10-31.md
Class: AUDIT
Status: 🟢 活跃
Last-Modified: 2026-10-31
Author: verify-bot

> **复核对象**：`/tmp/audit20.json` 中 6 条 `medium` 发现的指纹（run-1 批次）。
> **被复核的代码**：`/home/bai/wt-apeireth-r3`，分支 `debt-round8`，HEAD `8ce0f62b`。
> **对照基准（审计时的 ref）**：`7654d5b6`。
> **口径**：**只读复核** —— 不改代码、不提交；每条裁定都以**当前代码的实际行号**为准，
> 审计条目里引用的旧行号若已漂移，本页给出**新行号**并标明漂移原因。
> **⚠️ 复核命令**（本页结论可复算）见文末 §7。
>
> **⚠️ 并发变更提示（复核期间发生，2026-10-31 14:42 前后）**：本页 6 条裁定**一律以被指定的
> HEAD `8ce0f62b` 为准**（= 本任务指定的复核基准）。复核期间同一工作树上出现了两处**并发改动**，
> 它们**不改变本页裁定**，但会让「仍成立」这个字样在**新的 HEAD / 工作树**上过期：
>
> | 并发改动 | 影响 | 在新基线上的裁定 |
> |---|---|---|
> | 新提交 `b9a8c99f`「主体级遗忘的返回体不再说谎」（改 `crates/engine/memory/src/universal_forget.rs` + 其测试） | 命中 **发现 #4** | #4 ⇒ **部分成立**（见 §4 末「并发变更后」段） |
> | 新提交 `37bb6984`「出网地址分类补上特殊用途段 ⇒ 堵住云元数据 / CGNAT SSRF 通道」（改 `crates/capabilities/tools/src/egress.rs`，+165 行） | 命中 **发现 #1** | #1 ⇒ **已修**（见 §1 末「并发变更后」段） |
>
> 发现 #2 / #3 / #5 / #6 所在文件**未被这两处改动触及** ⇒ 裁定不变。

---

## 0. 本轮的范围与前置说明

| 项 | 内容 |
|---|---|
| 复核条数 | 6 条 `medium` |
| 其中**被审文件已变**（需重跑对照） | ① `universal_forget.rs`（2 次提交）② `canonical/cognitive.rs`（5 次提交）③ `egress_gate.rs`（1 次提交） |
| 其中**被审文件未变** | ① `capabilities/tools/src/egress.rs` ② `capabilities/tools/src/fetch.rs` ③ `engine/provider/src/canonical_anthropic.rs` |
| 引用行号漂移的根因 | `crates/engine/memory/src/migrations.rs` 在 `7654d5b6..HEAD` 之间有 **1 次提交** ⇒ 该文件被引用的行号整体位移；其余被引用的源文件（`persona_layer.rs` / `tool_result.rs` / `context_window.rs` / `execute.rs` / `provider.rs`（runtime）/ `egress_gate.rs`（runtime）/ `user_admin.rs` / `credentials.rs`）**行号零漂移**。 |
| 复核期间的并发改动 | HEAD 由 `8ce0f62b` 前进到 `b9a8c99f`（命中 #4）、再到 `37bb6984`（命中 #1）⇒ 详见页首「并发变更提示」与 §1 / §4 的「并发变更后」段；**本页 6 条裁定一律以被指定的 `8ce0f62b` 为准**。 |

---

## 1. `apeireth:crates/capabilities/tools/src/egress.rs:classify_ipv4`

**结论：仍成立**（medium 维持；claim 与 precondition 均未变；6 条证据行号**全部精确命中**）

### 当前代码证据（file:line）

* `crates/capabilities/tools/src/egress.rs:90-106` —— `classify_ipv4` 只做六个 std 判定
  （`:91` loopback、`:93` private、`:95` link-local、`:97` multicast、`:99` unspecified、`:101` broadcast），
  兜底 `:104` ⇒ `EgressIpClass::Public`。
* `crates/capabilities/tools/src/egress.rs:86-88` —— `is_public_ip` ≡ `classify_ip(ip) == Public`。
* `crates/capabilities/tools/src/egress.rs:212-215` —— `validate_resolved` 自述「if any resolved address is not public, the whole destination is denied」。
* `crates/capabilities/tools/src/egress.rs:233-251` —— `PublicInternetOnly` 唯一闸；`:241` `if !is_public_ip(addr.ip()) { … deny }`。
* `crates/capabilities/tools/src/egress.rs:195` —— `validate_destination` 对 `PublicInternetOnly` 直接 `Ok(())`（**主机名层面无任何网段预筛**）。
* `crates/capabilities/tools/src/egress.rs:529-531` —— IP 字面量目的地直接成为自己的解析结果（无 DNS）。
* `crates/capabilities/tools/src/egress.rs:550-552` —— 域名目的地 `resolve_to_addrs` 钉扎到**已分类**的地址 ⇒ 不构成第二道分类。
* `crates/capabilities/tools/src/fetch.rs:78-82` —— `FetchConfig::public_internet_only()` 默认策略 = `PublicInternetOnly`。
* `crates/adapters/cli/src/lib.rs:2129` —— 生产接线 `fetch_enabled.then(FetchConfig::public_internet_only)`。
* `crates/adapters/cli/src/lib.rs:1243-1245` —— `tool.fetch` 注册即闸：grant + `require_approval_for("tool.fetch")`（逐次人工审批）。
* `crates/capabilities/tools/src/fetch.rs:196-213` —— 工具入参**只有 `url`** ⇒ 审批卡上永远看不到解析后的 IP。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：`is_private` 只覆盖 RFC 1918 ⇒ `100.64.0.0/10`、`192.0.0.0/24`、`198.18.0.0/15`、文档网段被判为 Public | **仍成立**：`:90-106` 无这些网段判定，兜底 `:104` 判 Public。仓内亦无第二套分类器（全仓 grep `100\.64\|is_private\|special.?purpose\|IANA` 只命中 `egress.rs:93` 与两处测试常量 `crates/capabilities/tools/tests/architecture_invariants.rs:154`、`crates/capabilities/tools/src/recon_tool.rs:1575`） |
| root_cause：只排除 loopback / RFC1918 / link-local / multicast / unspecified / broadcast | **仍成立**：与 `:91`–`:101` 六个判定一一对应，无多无少 |
| intended：`PublicInternetOnly` 必须拒掉一切非公网地址 | **仍成立**：`:212-215` 自述与 `:241` 实现一致（**实现符合自述**，问题在于自述的「公网」定义本身偏宽） |
| evidence `:93`（`is_private` 只到 RFC1918） | ✅ 精确命中 `egress.rs:93` |
| evidence `:104`（兜底 Public） | ✅ 精确命中 `egress.rs:104` |
| evidence `:241`（唯一 PublicInternetOnly 闸） | ✅ 精确命中 `egress.rs:241` |
| evidence `:530`（IP 字面量无 DNS） | ✅ 精确命中 `egress.rs:530` |
| evidence `:95`（169.254.169.254 仍被 link-local 挡） | ✅ 精确命中 `egress.rs:95` |
| evidence `:551`（DNS 钉扎不构成第二道分类） | ✅ 精确命中 `egress.rs:551` |
| condition「`tool.fetch` 必须开（`APEIRETH_ENABLE_FETCH=1`）；默认生产策略 `PublicInternetOnly`」 | ✅ 成立：`crates/adapters/cli/src/lib.rs:2019-2021`（开关默认关）、`:2129`（默认策略）；`crates/capabilities/tools/src/fetch.rs:78-82` |
| condition「每次 fetch 走人工审批，审批只显示 URL、从不显示解析 IP」 | ✅ 成立：`crates/adapters/cli/src/lib.rs:1245` + `crates/capabilities/tools/src/fetch.rs:196-213`（入参仅 `url`） |
| condition「169.254.169.254 等主流云元数据仍被挡」 | ✅ 成立：`egress.rs:95` |
| severity `medium` / confidence `high` | ✅ 维持：前置条件与影响面均未变化 |

**审计条目中的过期引用（非缺陷，仅提示）**：remediation 建议「mirroring the platform's provider-proxy gate」——
仓内**不存在**这样一个带 IANA 特殊用途网段清单的 provider-proxy 闸（`crates/engine/provider/src/` 全目录 grep `proxy|Proxy` 只命中 `canonical_openai_compatible.rs:7` 的一句注释）⇒ 照抄该建议前需自行设计清单。

**是否需重跑**：**不需重跑**（被审文件零变更，证据行号零漂移）。若采纳修复，需新增「`100.64.0.0/10`、`192.0.0.0/24`、`198.18.0.0/15`、`192.0.2.0/24` 等在 `PublicInternetOnly` 下被拒」的回归测试。

### 并发变更后（新提交 `37bb6984`，2026-10-31 14:42 前后落地）

针对本条的**在途修复**已落地为提交 `37bb6984`（`crates/capabilities/tools/src/egress.rs`，+165 行）：
新增 `EgressIpClass::{Shared, Reserved}` 两个分类，并加判
`is_shared_address_space`（RFC 6598 `100.64.0.0/10`，**含阿里云元数据 `100.100.100.200`**）与
`is_reserved_ipv4`（`0.0.0.0/8`、`192.0.0.0/24`、`192.0.2.0/24`、`198.18.0.0/15`、`198.51.100.0/24`、`203.0.113.0/24`、`240.0.0.0/4`），
另加 IPv6 侧 NAT64（`64:ff9b::/96`）/ 6to4（`2002::/16`）内嵌 IPv4 判定与 `2001:db8::/32` 文档段，
并补了边界与端到端回归测试（含「`PublicInternetOnly` 必拒 `100.100.100.200`」）。

⇒ **在 `37bb6984` 之后的基线上本条已是「已修」**；但本页裁定仍以被指定的 HEAD `8ce0f62b` 为准 ⇒ **仍成立**。
（复核者只读，未参与该改动。）

---

## 2. `apeireth:crates/capabilities/tools/src/fetch.rs:execute_with_egress`

**结论：仍成立**（medium 维持；claim 与 precondition 均未变；6 条证据行号**全部精确命中**）

### 当前代码证据（file:line）

* `crates/capabilities/tools/src/fetch.rs:361-367` —— `execute_with_egress` 入口。
* `crates/capabilities/tools/src/fetch.rs:431-439` —— 工具结果 JSON 构造；`:436` `"body": decoded`（远程正文**逐字**嵌入）。
* `crates/capabilities/tools/src/fetch.rs:444-459` —— HTML 派生视图同样裸写进结果：`:450` `title`、`:453` `text`、`:457` `accessibility`。
* `crates/capabilities/tools/src/fetch.rs:469` —— `ToolResult::ok(&call.id, value).with_name("fetch")` ⇒ **全文件无任何封套**（`crates/capabilities/tools/src/` 全目录 grep `UntrustedContentWrapper|UntrustedEnvelope|untrusted` 在 `fetch.rs` **零命中**）。
* 对照（同仓其它外文内容面）：`crates/capabilities/tools/src/content_feed/tool.rs:824-834`（`UntrustedEnvelope::new(...).disclose(...)`）、`crates/capabilities/tools/src/stealth_crawler.rs:92-97`（`wrap_untrusted_content`）、`crates/capabilities/tools/src/skill/invocation.rs:92`。
* `crates/foundation/protocol/src/canonical/tool_result.rs:117-125` —— `render()` 只是值的 `to_string()`（字符串值解包），**无通用封套**。
* `crates/foundation/protocol/src/canonical/tool_result.rs:128-131` —— `into_message()` ⇒ 角色 `Tool` 的 transcript 消息。
* `crates/engine/runtime/src/canonical/execute.rs:681-699` —— 唯一的通用披露闸 `disclose_cross_source_overlay`（`:689` `UntrustedEnvelope::new`）**只作用于 `PromptOverlay` 块**；工具结果不走此路。
* `crates/engine/memory/src/context_window.rs:118` —— 工具结果文本被抄进连续性摘要（`:195-202` `first_line` 取首行、**240 字上限**）。
* `crates/engine/memory/src/context_window.rs:143-147` —— 摘要以 **`NormalizedMessage::system`** 回灌 ⇒ 工具角色 → 系统角色。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：`fetch` 把远程 body / text / accessibility 快照裸放进工具结果，而 `content_feed`、`stealth_crawler` 会隔离远程内容 | **仍成立**：`fetch.rs:436/:450/:453/:457` vs `content_feed/tool.rs:824` vs `stealth_crawler.rs:93` |
| root_cause：`execute_with_egress` 用原始远程内容构造结果值，不套 `UntrustedContentWrapper` / `UntrustedEnvelope` | **仍成立**：`fetch.rs:469` 直出；该文件零封套引用 |
| intended：本仓其它外文内容面都隔离远程文本 | **仍成立**（对照同上） |
| evidence `fetch.rs:436` | ✅ 精确命中（`"body": decoded`） |
| evidence `tool_result.rs:117` | ✅ 精确命中（`pub fn render(&self) -> String`） |
| evidence `tool_result.rs:130` | ✅ 精确命中（`NormalizedMessage::tool_result(...)`） |
| evidence `execute.rs:689` | ✅ 精确命中（`UntrustedEnvelope::new(...)`，位于唯一通用披露闸内） |
| evidence `context_window.rs:118` | ✅ 精确命中（`let line = format!("{role}: {}\n", first_line(&text));`） |
| evidence `context_window.rs:143` | ✅ 精确命中（`messages.push(NormalizedMessage::system(format!(...)))`） |
| condition「`tool.fetch` 必须开；`content_feed`/`stealth_crawler` 是 opt-in ⇒ 不一致默认潜伏」 | ✅ 成立：`crates/adapters/cli/src/lib.rs:2019-2021`（fetch 默认关）、`:2030`（content_feed 默认关） |
| condition「运维审批该次 fetch，审批只看到 URL、看不到页面内容」 | ✅ 成立：`crates/capabilities/tools/src/fetch.rs:196-213`（入参仅 `url`） |
| condition「升级到系统消息需 transcript 超过模型上下文窗 80%」 | ✅ 成立：`crates/engine/memory/src/context_window.rs:23`（`compact_threshold: 0.80`）、`:77-83`（`should_compact`） |
| condition「有副作用的工具仍受审批闸约束 ⇒ 注入文本本身改不了状态」 | ✅ 成立（审批面未变） |
| severity `medium` / confidence `high` | ✅ 维持 |

**是否需重跑**：**不需重跑**（被审文件零变更，证据行号零漂移）。若采纳修复，需新增「fetch 结果含封套标记 / 工具结果不再被原样回灌为系统消息」的回归测试。

---

## 3. `apeireth:crates/engine/provider/src/canonical_anthropic.rs:AnthropicProviderCapability::complete`

**结论：仍成立**（medium 维持；claim 与 precondition 均未变；6 条证据行号**全部精确命中**）

### 当前代码证据（file:line）

* `crates/engine/provider/src/canonical_anthropic.rs:62` —— `pub const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/anthropic";`
  （`:59-61` 的注释自述「Follows the repository's existing default (minimaxi's Anthropic-compatible gateway)」）。
* `crates/engine/provider/src/canonical_anthropic.rs:64-76` —— `DEFAULT_MODELS`；`:65` 首项即 `"MiniMax-M3"`。
* `crates/engine/provider/src/canonical_anthropic.rs:493-496` —— `from_env`：`APEIRETH_ANTHROPIC_URL` 非空则用之，否则 `:496` 回落 `DEFAULT_BASE_URL` ⇒ **无 scheme / host 校验、无「凭据-端点绑定」**。
* `crates/engine/provider/src/canonical_anthropic.rs:383` —— 请求 URL 由 `self.base_url` 拼出（`{base}/v1/messages`）。
* `crates/engine/provider/src/canonical_anthropic.rs:390` —— `.header("x-api-key", key.expose())` ⇒ 解析出的 key 发往该主机。
* `crates/engine/provider/src/canonical_anthropic.rs:381` —— `let key = self.resolve_key()?;`（每轮解析 `APEIRETH_ANTHROPIC_KEY`）。
* `crates/engine/provider/src/credentials.rs:46` —— 逻辑名 `provider.anthropic.api_key`；`:53` `pub const ANTHROPIC_API_KEY_ENV: &str = "APEIRETH_ANTHROPIC_KEY";`。
* 生产接线：`crates/adapters/cli/src/lib.rs:1700-1703` —— `AnthropicProviderPlugin::from_env()` **无条件注册**；`:1702` 把 `provider.anthropic` 放进 `fallback_order`（`:1695-1698` `provider.minimax` 在前）。
* onboarding 标签：`docs/02-guides/cli-onboarding.md:91`（`$env:APEIRETH_ANTHROPIC_KEY = "sk-…"  # Anthropic 通道`）、`docs/02-guides/quick-start.md:42`。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：`provider.anthropic` 解析 `APEIRETH_ANTHROPIC_KEY`，默认 `base_url` 却是 `api.minimaxi.com`；除非运维改 URL，否则 key 以 `x-api-key` 发到 MiniMax | **仍成立**：`:62` + `:493-496` + `:383` + `:390` 四段闭环 |
| root_cause：无凭据-端点绑定；凭据名宣称 Anthropic，落地默认主机是 MiniMax 网关；onboarding 标为 Anthropic 并配 claude 模型 | **仍成立**：`:64-76`（默认模型表含 `claude-*`）、`docs/02-guides/cli-onboarding.md:91` |
| intended：厂商凭据只应发往其签发方端点 | **仍成立**（未实现） |
| evidence `:62` | ✅ 精确命中 |
| evidence `:493` | ✅ 精确命中 |
| evidence `:383` | ✅ 精确命中 |
| evidence `:390` | ✅ 精确命中 |
| evidence `credentials.rs:53` | ✅ 精确命中 |
| evidence `:65`（能力宣称 MiniMax-M3 ⇒ 路由回退能命中它） | ✅ 精确命中；且 `:1702` 把它放进 fallback 顺序（`:1697` minimax 在前 ⇒ minimax 缺 key/失败时即回退到 anthropic 能力 ⇒ key 直达 `api.minimaxi.com`） |
| condition「`APEIRETH_ANTHROPIC_URL` 保持未设（默认）」 | ✅ 成立：`:493-496` |
| condition「运维照 onboarding/文档 env 列表粘贴 Anthropic key」 | ✅ 成立：`docs/02-guides/cli-onboarding.md:91`、`docs/02-guides/quick-start.md:42` |
| condition「`api.minimaxi.com` 是无关第三方厂商」 | ✅ 成立：`:59-62` 自述该主机归属 |
| severity `medium` / confidence `high` | ✅ 维持 |

**是否需重跑**：**不需重跑**（被审文件零变更，证据行号零漂移）。若采纳修复，需新增「`provider.anthropic` 默认端点 = `api.anthropic.com`，或改用独立凭据名」的回归测试。

---

## 4. `apeireth:crates/engine/memory/src/universal_forget.rs:forget_principal_on_conn`

**结论：仍成立**（medium 维持）。**被审文件变了 2 次提交，但变更没有触及本条的任何一处断言。**

### 本文件的变更（`7654d5b6..HEAD`）

```
8c5491d5 fix: **H4 / H5 / H6 / H7 全部修完** ⇒ 抗漂移 8 个漏洞**全部清零**
c7a39254 feat(memory): 「先记访问史，再忘记」守卫**生产接线**（G1 收口）
```

变更内容：新增「**先记访问史，再忘记**」守卫（`resolve_access_permit` / `verify_permit_against_history` /
`audit_access_guard_decision`）、新增 `EpisodeGuard::{Permitted, Compat}` 枚举、`UniversalForgetOutcome`
新增 `access_guard: AccessGuardStatus` 字段（`crates/engine/memory/src/universal_forget.rs:139-148`）、
新增严格面 `forget_principal_with_permit{,_on_pool}`（`:273-300`、`:344-369`）。

### 变更是否解决了本条？—— **没有**（三处断言逐一不成立为「已修」）

1. **「映射表没人写」未变**：新代码反而**再加一处**依赖该空表的查询 ——
   `crates/engine/memory/src/universal_forget.rs:477-493` `principal_episode_targets` 仍是
   `FROM episodes e JOIN memory_principal_sessions ps ON ps.session_id=e.session_id`（`:484`）。
2. **「outcome 标志无条件」未变**：`:465-472` 仍硬编码 `episode: true`（不看任何行数）。
3. **「API 如实回报」未变**：网关侧仍只看 `Ok` 就置 `memory_forgotten = true`。

### 当前代码证据（file:line）

* `crates/engine/memory/src/universal_forget.rs:446` —— 墓碑语句仍是 JOIN 映射表：
  `INSERT OR IGNORE INTO episode_governance … SELECT e.id,'forgotten',… FROM episodes e JOIN memory_principal_sessions ps ON ps.session_id=e.session_id WHERE ps.principal_id=?3`
  （审计条目引用的 `:122` ⇒ **行号漂移到 `:446`**）。
* `crates/engine/memory/src/universal_forget.rs:465-472` —— outcome 硬编码 `episode: true` / `commitment: true` / `temporal: true` / `persona: true`
  （审计条目引用的 `:142` ⇒ **漂移到 `:466`**）。
* `crates/engine/memory/src/universal_forget.rs:477-493` —— 新增的目标枚举，同样 JOIN 映射表（`:484`）。
* `crates/engine/memory/src/migrations.rs:137-144` —— V14 建 `memory_principal_sessions` + 索引
  （审计条目引用的 `:393` ⇒ **行号漂移**）。
* `crates/engine/memory/src/migrations/list.rs:261-266` —— 「Legacy rows are intentionally not backfilled.」
  （审计条目引用写作 `migrations.rs:262` ⇒ **文件与行号均需更正**；当前 `migrations.rs:262` 是 `commitment_events.payload_json` 的列定义）。
* **生产代码零写入**：全仓 grep `memory_principal_sessions` 仅命中
  `crates/engine/memory/src/universal_forget.rs:444/:446/:484`（读）、`crates/engine/memory/src/migrations.rs:138/:143/:144`（建表/索引）、
  以及**两个测试夹具**：`crates/engine/memory/tests/universal_forget.rs:13/:50/:79`、
  `crates/engine/memory/tests/forget_guard_wiring.rs:187/:391` ⇒ 生产环境该表**恒空**。
  （审计条目写作「the only INSERTs are in `crates/engine/memory/tests/universal_forget.rs`」⇒ **漏列第二个测试文件**。）
* `crates/engine/memory/src/migrations.rs:537-540` —— `episodes_no_delete` 触发器 `RAISE(ABORT, 'episodes is append-only: …')`
  ⇒ 任何级联删除都不可能清掉 episode 正文（审计条目引用的 `:793` ⇒ **行号漂移**）。
* `crates/adapters/gateway/src/user_admin.rs:466` —— `memory_forgotten = true;`（**行号精确命中**）；`:467-474` 的 note 只列
  `episode/preference/commitment/temporal/persona` 五个布尔值 ⇒ **新增的 `access_guard` 状态没有透出到 API 响应**。
* 接线：`crates/engine/memory/src/memory_governance.rs:642-655` —— `SqliteMemoryStore::forget_principal` → `UniversalForgetFacade::forget_principal`。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：删用户 ⇒ `forget_principal` ⇒ episode 墓碑依赖没人写的 `memory_principal_sessions` ⇒ 零行被墓碑，outcome 仍报 `episode=true`，API 仍答 `memory_forgotten=true` | **仍成立**（全链未变） |
| root_cause：`memory_principal_sessions` 由 V14 建表却从未被填；episode-forget 语句 JOIN 它；outcome 标志无条件 | **仍成立** |
| intended：删主体应墓碑其 episode，失败必须如实上报 | **仍成立**（未实现） |
| evidence `universal_forget.rs:122` | ⚠️ **行号漂移** ⇒ 同一断言现在在 `:446` |
| evidence `universal_forget.rs:142` | ⚠️ **行号漂移** ⇒ 同一断言现在在 `:466` |
| evidence `migrations.rs:262` | ⚠️ **行号漂移 + 文件引用有误** ⇒ 「legacy rows intentionally not backfilled」在 `crates/engine/memory/src/migrations/list.rs:262` |
| evidence `migrations.rs:393` | ⚠️ **行号漂移** ⇒ 建表现在在 `crates/engine/memory/src/migrations.rs:137-144` |
| evidence `migrations.rs:793` | ⚠️ **行号漂移** ⇒ `episodes_no_delete` 触发器现在在 `crates/engine/memory/src/migrations.rs:537-540` |
| evidence `user_admin.rs:466` | ✅ 精确命中 |
| evidence「只有测试夹具 INSERT 映射表」 | ✅ 仍成立，但**漏列** `crates/engine/memory/tests/forget_guard_wiring.rs:187/:391` |
| condition「认知库存有该主体的 episode（默认安装）」 | ✅ 成立 |
| condition「owner/admin 调删用户端点」 | ✅ 成立（`crates/adapters/gateway/src/user_admin.rs:434-438`） |
| severity `medium` / confidence `high` | ✅ 维持：影响面（GDPR 式被遗忘权失真）与前置条件均未变 |

**是否需重跑**：**不需重跑**（结论由静态代码即可判定）。若采纳修复，需新增「生产路径上 `memory_principal_sessions` 有行 ⇒ 墓碑行数 > 0 ⇒ `outcome.episode` 由行数派生」以及「网关响应体必须含墓碑行数」两条回归测试。

### 并发变更后（新提交 `b9a8c99f`，2026-10-31 14:42 落地）

复核期间新提交 **`b9a8c99f`**「主体级遗忘的返回体不再说谎」改了本文件
（`crates/engine/memory/src/universal_forget.rs` + `crates/engine/memory/tests/universal_forget.rs`）：

* `crates/engine/memory/src/universal_forget.rs:588-589` —— `let mappable = sessions_mapped > 0;`
  / `let episode_cleared = mappable && episodes_outstanding == 0;`
  ⇒ **「outcome 标志无条件」这一条已被修掉**（不再是写死常量）。
* `crates/engine/memory/src/universal_forget.rs:504-513` —— 实测 `sessions_mapped` 与 `episodes_attributable`；
  `:548-552` —— 实测后置残留 `episodes_outstanding`；
  `:604-610` —— 新增 `ForgetCounts`（本次**实际动了多少行**：`tx.changes()`）；
  `:591-595` —— 新增 `AttributionStatus { sessions_mapped, episodes_attributable, episodes_outstanding }`。
* **根因未修**（该提交自己的说明也这么写）：生产侧**仍无** `memory_principal_sessions` 的写入方
  ⇒ 生产里 `sessions_mapped == 0` ⇒ `mappable == false` ⇒ `episode_cleared == false`
  ⇒ **episode 维度仍不会被真正清除**，只是**不再谎称已清除**。
* 网关侧**未改**（`crates/adapters/gateway/src/user_admin.rs:466` 仍是 `Ok` 即 `memory_forgotten = true`）
  ⇒ 但 note 现在会显示 `episode=false` ⇒ **顶层标志仍在说谎，明细已不撒谎**。

⇒ **在 `b9a8c99f` 之后的基线上，本条裁定为「部分成立」**：claim 的「硬编码标志」半边已修，
「episode 归属不可判定 ⇒ 被遗忘权在生产失效」与「API 顶层 `memory_forgotten=true`」两半仍在。
本页裁定仍以被指定的 HEAD `8ce0f62b` 为准 ⇒ **仍成立**。

---

## 5. `apeireth:crates/engine/runtime-assembly/src/canonical/cognitive.rs:PersonaConstantInjectionModule`

**结论：仍成立**（medium 维持）。**被审文件变了 5 次提交，但注入路径一行未动。**

### 本文件的变更（`7654d5b6..HEAD`）

```
32ee6bcc feat(organ): semantic_axis 并入 absorption_insight 补第 5 段（owner 裁定）
1cfc4655 feat(d2): council ballot 接线 —— 评价⇒票⇒落盘⇒守夜人复盘段不再空转
6dd25824 feat(accountability): D-6 裁定落地 —— 显式绑定账 + failed 自动回填
1fc6beef feat(memory): 关系里程碑接电 —— 初见与阶段跃迁真的会被记下来（且跨重启）
af412c9c feat(memory): partner 羁绊**跨重启持久化**
```

变更内容：`PartnerBondModule` 加里程碑记录与 `SqlitePartnerStore`（`crates/engine/runtime-assembly/src/canonical/cognitive.rs:1663-1673`）、
`absorption_insight_text` 加第 5 段 `semantic_axis`（`:907-1025`）、`CouncilModule` 加 ballot 落盘（`:2631-2760`）、
模块表期望值随问责开关变化（`:4668-4689`）。

### 变更是否解决了本条？—— **没有**

`PersonaConstantInjectionModule` 的实现体（`:573-598`）与 `on_hook`（`:606-630`）**逐字未变**：
仍是 `self.persona.load()` → `build_constant_injection(&memory)` → `PromptOverlay::system(text)`（`:621`）。

### 当前代码证据（file:line）

* `crates/engine/runtime-assembly/src/canonical/cognitive.rs:606-622` —— `TurnStart` 分支：
  `:614` `self.persona.load()` → `:617` `build_constant_injection(&memory)` → `:621` `PromptOverlay::system(text)`。
* 对照（召回面走的是**跨源**变体）：`crates/engine/runtime-assembly/src/canonical/cognitive.rs:1467`、
  `:1559` —— `PromptOverlay::system_cross_source("memory_recall", overlay)`。
  ⇒ 人格注入**没有**跨源标记 ⇒ 在 provider 漏斗里**不触发** `crates/engine/runtime/src/canonical/execute.rs:681-699` 的封套披露。
* `crates/engine/memory/src/persona_layer.rs:330` —— `NARRATIVE_MARKERS` 首项即 `"我是"`（**行号精确命中**）。
* `crates/engine/memory/src/persona_layer.rs:522-527` —— 提炼器只读 `episode.role != "assistant"` 就跳过（`:525` **行号精确命中**）。
* `crates/engine/memory/src/persona_layer.rs:756-762` —— 自述句追加进唯一的 `self_narrative` 字符串（`:760` **行号精确命中**）。
* `crates/engine/memory/src/persona_layer.rs:1185-1198` —— 读取 `FROM persona_memory_state WHERE singleton = 1`（`:1188` **行号精确命中**）⇒ **无会话/主体过滤**。
* `crates/engine/memory/src/migrations.rs:129-134` —— `persona_memory_state` 是单例行（`singleton INTEGER PRIMARY KEY CHECK (singleton = 1)`），无 owner/session/tenant 列
  （审计条目引用的 `:384` ⇒ **行号漂移**）。
* `crates/engine/memory/src/migrations.rs:105-114` —— `persona_memory_entries` 列集合
  （`id/kind/text/sources_json/polarity/created_at/updated_at/protection`）**无任何 scope 列**
  （审计条目引用的 `:360` ⇒ **行号漂移**）。
* 装配点：`crates/engine/runtime-assembly/src/canonical/production.rs:1676-1686` —— 人格层落库面在场即注册注入模块；
  落库面默认接线：`crates/adapters/cli/src/lib.rs:2291`（`persona_layer: Some(persona_store.clone())`）。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：`persona_memory_state` / `persona_memory_entries` 无 owner / session / tenant 列 | **仍成立**：`migrations.rs:129-134`、`:105-114` |
| 描述：含「我是」的 assistant 句进入**唯一的全局** `self_narrative` 行 | **仍成立**：`persona_layer.rs:330` + `:525` + `:760` + `:1188` |
| 描述：每次 `TurnStart` 以 `PromptOverlay::system` 注入，绕过召回路的跨源封套 | **仍成立**：`cognitive.rs:621` vs `:1467`/`:1559` |
| root_cause：人格记忆层无 scope 列 + 注入走 `system` 而非跨源变体 | **仍成立** |
| intended：到达模型的持久记忆应当按会话/主体限定，并走未可信封套披露 | **仍成立**（未实现） |
| evidence `migrations.rs:384` | ⚠️ **行号漂移** ⇒ 现在 `:129-134` |
| evidence `migrations.rs:360` | ⚠️ **行号漂移** ⇒ 现在 `:105-114` |
| evidence `persona_layer.rs:330` / `:525` / `:760` / `:1188` | ✅ **四条全部精确命中**（该文件零变更） |
| condition「人格整合开启且人格层接线（默认安装）」 | ⚠️ **需要精化**：人格层**落库面**默认接线（`cli/src/lib.rs:2291` ⇒ 注入模块默认注册，`production.rs:1680-1686`），但**写出自我叙事**需要两个默认关的 env：`APEIRETH_ENABLE_CONSOLIDATION=1`（`crates/adapters/cli/src/lib.rs:372-376`）**且** `APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1`（`:382-386`）⇒ 默认安装下 `self_narrative` 为空、注入为空（`production.rs:1678-1679`「未沉淀就是未沉淀」）。审计条目的 condition 主体「Persona consolidation must be enabled」是对的；「(default install)」括注只对「人格层接线」成立，对「整合开启」**不成立**。 |
| condition「助手须说出第一人称叙述句」 | ✅ 成立 |
| severity `medium` / confidence `high` | ✅ 维持：条件更严（需两个 env），但缺陷形态与影响（跨会话无标记注入）不变 |

**是否需重跑**：**不需重跑**（结论由静态代码即可判定）。若采纳修复，需新增「人格层按 session/owner 分区」与「人格注入块带跨源标记 ⇒ provider 请求里出现封套警告头」两条回归测试。

---

## 6. `apeireth:crates/engine/runtime-assembly/src/egress_gate.rs:egress_gate_from_env_with_all`

**结论：仍成立**（medium 维持）。**被审文件变了 1 次提交，但变更没有触及解析失败分支。**

### 本文件的变更（`7654d5b6..HEAD`）

```
6dd25824 feat(accountability): D-6 裁定落地 —— 显式绑定账 + failed 自动回填
```

变更内容：① 出网动作 `action_id` 加进程内单调序号，避免同毫秒撞 id（`crates/engine/runtime-assembly/src/egress_gate.rs:49-56`、`:190-195`）；
② 抽出共享的 env 生效判据 `egress_gate_enabled_from_value` / `egress_gate_enabled_from_env`（`:313-333`），装配根与闸工厂共用。

### 变更是否解决了本条？—— **没有**

授权策略**解析失败**的分支（`:399-403`）逐字未变：仍是 `eprintln!` + `return None`。
新增的 `egress_gate_enabled_from_value`（`:319-322`）只判「闸开不开」，**不判策略解析成败**。

### 当前代码证据（file:line）

* `crates/engine/runtime-assembly/src/egress_gate.rs:399-403` —— 策略解析失败：
  `:400` `eprintln!("[egress] 授权策略解析失败（{error}）—— 不装配出网闸");`、`:401` `return None;`
  ⇒ **闸整体不装**（无脱敏、无授权、无账本）（审计条目引用的 `:359` ⇒ **行号漂移到 `:400-402`**）。
* `crates/engine/runtime-assembly/src/egress_gate.rs:344` —— 函数文档自称「授权写坏了 = 不猜：**fail-closed**（不装配闸）+ stderr 如实报错」
  ⇒ 与 `:401` 的实际效果**自相矛盾**（审计条目引用的 `:303` ⇒ **行号漂移到 `:344`**）。
* `crates/engine/runtime-assembly/src/egress_gate.rs:12` —— 模块文档：「**Deny**：闸自身出错（读不到策略、桥内部失败）→ **拒绝即帧**（fail-closed）」
  （**行号精确命中**）。
* `crates/engine/runtime/src/canonical/egress_gate.rs:19` —— 端口契约：「fail-closed 由实现方保证：判定失败 / 策略读不到 / 闸内部错误，一律返回 `Deny`」
  （**行号精确命中**）。
* `crates/engine/runtime/src/canonical/provider.rs:115` —— 路由器默认 `egress: Arc::new(AllowAllEgress)`（**行号精确命中**）。
* `crates/engine/runtime/src/canonical/provider.rs:267` —— 判定点 `match self.egress.vet(request)`（**行号精确命中**）。
* `crates/engine/runtime/src/canonical/egress_gate.rs:60-64` —— `AllowAllEgress::vet` 恒返回 `Pass` ⇒ 原始请求照发。
* 装配根：`crates/adapters/cli/src/lib.rs:1659-1665` —— `if let Some(gate) = …egress_gate_from_env_with_all(…) { builder = builder.with_egress_gate(gate); }`
  ⇒ 返回 `None` ⇒ 路由器保持 `AllowAllEgress`。

### 逐条核对

| 审计条目中的断言 | 复核结论 |
|---|---|
| 描述：策略 JSON 写坏 ⇒ 闸不装 ⇒ 路由器默认 `AllowAllEgress` ⇒ 未脱敏的 prompt（含 PII/凭据）发给外部模型厂商 | **仍成立**（全链未变） |
| root_cause：解析失败被处理成「不装闸」，等于**移除**脱敏与授权，而不是拒绝出网 | **仍成立** |
| intended：代码自称 fail-closed ⇒ 策略写坏应当拦出网（或至少保住脱敏桥） | **仍成立**（未实现，且与 `:12`、`:344`、runtime 端口契约 `:19` 三处文字冲突） |
| evidence `egress_gate.rs:359` | ⚠️ **行号漂移** ⇒ 同一断言现在 `:400-402` |
| evidence `egress_gate.rs:303` | ⚠️ **行号漂移** ⇒ 同一断言现在 `:344` |
| evidence `egress_gate.rs:12` | ✅ 精确命中 |
| evidence `runtime/src/canonical/egress_gate.rs:19` | ✅ 精确命中 |
| evidence `runtime/src/canonical/provider.rs:115` | ✅ 精确命中 |
| evidence `runtime/src/canonical/provider.rs:267` | ✅ 精确命中 |
| condition「触发面是部署 env `APEIRETH_ENABLE_EGRESS_GATE`；错误只落在 stderr」 | ✅ 成立：`:400` 的 `eprintln!`（无回执、无 API 面） |
| condition「影响需要 prompt 里有密钥或 PII」 | ✅ 成立 |
| severity `medium` / confidence `high` | ✅ 维持 |

**是否需重跑**：**不需重跑**（结论由静态代码即可判定）。若采纳修复，需新增「`APEIRETH_ENABLE_EGRESS_GATE` 设为非法 JSON ⇒ 出网判定为 `Deny`（而不是 `Pass`）」的回归测试 —— 这条测试当前**必然红**，是修复的验收闸。

---

## 7. 复核命令（本页结论可复算）

```bash
# 1) 文档分类守卫（AUDIT 类强制项：同时含「已修」与「未修/暂缓」+ 复核命令）
python3 scripts/check_doc_class.py --check

# 2) 文档断链守卫
python3 scripts/docs_links.py --check

# 3) 六条指纹的当前落点（行号以本仓 HEAD 为准）
grep -n "fn classify_ipv4" crates/capabilities/tools/src/egress.rs
grep -n "fn execute_with_egress" crates/capabilities/tools/src/fetch.rs
grep -n "DEFAULT_BASE_URL\|fn from_env" crates/engine/provider/src/canonical_anthropic.rs
grep -n "fn forget_principal_on_conn\|fn principal_episode_targets" crates/engine/memory/src/universal_forget.rs
grep -n "PersonaConstantInjectionModule" crates/engine/runtime-assembly/src/canonical/cognitive.rs
grep -n "fn egress_gate_from_env_with_all" crates/engine/runtime-assembly/src/egress_gate.rs

# 4) 被审文件在审计 ref 之后的变更（4 / 5 / 6 的对照依据）
git log --oneline 7654d5b6..HEAD -- crates/engine/memory/src/universal_forget.rs
git log --oneline 7654d5b6..HEAD -- crates/engine/runtime-assembly/src/canonical/cognitive.rs
git log --oneline 7654d5b6..HEAD -- crates/engine/runtime-assembly/src/egress_gate.rs
git diff 7654d5b6..HEAD -- crates/engine/memory/src/universal_forget.rs
git diff 7654d5b6..HEAD -- crates/engine/runtime-assembly/src/canonical/cognitive.rs
git diff 7654d5b6..HEAD -- crates/engine/runtime-assembly/src/egress_gate.rs

# 5) 「映射表生产零写入」这条关键前提
grep -rn "memory_principal_sessions" crates/
```

---

## 汇总

| finding | 严重度 | 裁定 | 一句话依据 |
|---|---|---|---|
| `apeireth:crates/capabilities/tools/src/egress.rs:classify_ipv4` | medium | **仍成立** | `egress.rs:90-106` 仍只按六个 std 判定分类、`:104` 兜底 Public ⇒ RFC6598/基准/文档网段仍算公网，6 条证据行号全中 |
| `apeireth:crates/capabilities/tools/src/fetch.rs:execute_with_egress` | medium | **仍成立** | `fetch.rs:436/:450/:453/:457` 仍裸嵌远程内容（该文件零封套引用），而 `content_feed/tool.rs:824`、`stealth_crawler.rs:93` 已隔离 ⇒ 不一致仍在 |
| `apeireth:crates/engine/provider/src/canonical_anthropic.rs:AnthropicProviderCapability::complete` | medium | **仍成立** | `canonical_anthropic.rs:62` 默认端点仍是 `api.minimaxi.com`，`:493-496` 无凭据-端点绑定，`:390` 把 `APEIRETH_ANTHROPIC_KEY` 以 `x-api-key` 发往该主机 |
| `apeireth:crates/engine/memory/src/universal_forget.rs:forget_principal_on_conn` | medium | **仍成立** | 文件虽变 2 次提交，但 `:446` 仍 JOIN 无人写的 `memory_principal_sessions`、`:466` 仍硬编码 `episode: true`、`user_admin.rs:466` 仍回 `memory_forgotten=true` ⇒ 变更未触及 |
| `apeireth:crates/engine/runtime-assembly/src/canonical/cognitive.rs:PersonaConstantInjectionModule` | medium | **仍成立** | 文件虽变 5 次提交，但注入仍是 `cognitive.rs:621` 的 `PromptOverlay::system`（召回面 `:1467`/`:1559` 走跨源变体），`persona_layer.rs:1188` 仍读无 scope 的单例行 ⇒ 变更未触及 |
| `apeireth:crates/engine/runtime-assembly/src/egress_gate.rs:egress_gate_from_env_with_all` | medium | **仍成立** | 文件虽变 1 次提交，但 `:400-402` 仍是「解析失败 ⇒ 不装闸」，`cli/src/lib.rs:1659-1665` 保持 `AllowAllEgress`（`provider.rs:115`）⇒ 变更未触及 |
| **合计（6 条 medium）** | — | **已修 0 条 / 未修 6 条** | 本轮 6 条 medium 发现**无一条**被**被指定的 HEAD `8ce0f62b`** 修复；3 条被审文件零变更、3 条被审文件已变更但变更均未触及对应断言 |
| **合计（并发变更后）** | — | **已修 1 条 / 部分成立 1 条 / 未修 4 条** | 复核期间的并发改动：`37bb6984` 已修 #1（特殊用途段分类）、`b9a8c99f` 已修 #4 的「硬编码标志」半边（根因未修）；#2/#3/#5/#6 不受该两处改动影响 |

### 遗留的「证据行号漂移」清单（供下一轮审计报告改口径）

| finding | 审计条目引用 | 当前真实位置 | 漂移根因 |
|---|---|---|---|
| #4 | `universal_forget.rs:122` | `universal_forget.rs:446` | 被审文件变更（2 次提交） |
| #4 | `universal_forget.rs:142` | `universal_forget.rs:466` | 同上 |
| #4 | `migrations.rs:262` | `migrations/list.rs:262` | `migrations.rs` 变更（1 次提交）**且原引用文件写错** |
| #4 | `migrations.rs:393` | `migrations.rs:137-144` | 同上 |
| #4 | `migrations.rs:793` | `migrations.rs:537-540` | 同上 |
| #5 | `migrations.rs:384` | `migrations.rs:129-134` | 同上 |
| #5 | `migrations.rs:360` | `migrations.rs:105-114` | 同上 |
| #6 | `egress_gate.rs:359` | `egress_gate.rs:400-402` | 被审文件变更（1 次提交） |
| #6 | `egress_gate.rs:303` | `egress_gate.rs:344` | 同上 |

> **口径提醒**：行号漂移只说明「引用过时」，**不说明事实失效** —— 上表 9 条漂移的**断言本身在当前代码里全部仍然成立**（已逐条在新位置复核）。
