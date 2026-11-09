# C-B4 浏览器面 · 重复审计与范围裁定

[Document-Meta]
Document: docs/01-architecture/capability-creation-line-web-browser-scope.md
Version: 1.0
Last-Modified: 2026-11-09
Class: DECISION
Status: 🟡 裁定待 owner 确认（范围裁定已给建议）
Author: W3 实施代理（能力创造线 C-B4 单元）

> **本稿是「范围裁定 + 目标形态」，不是「做到哪了」**。做到哪了 ⇒
> [进度页](../status/capability-creation-line-progress.md)（唯一权威）。
> **授权依据**：[ADR-0025](../adr/0025-capability-creation-line-charter.md)。
> **族规格原文**：[能力创造线设计稿](./capability-creation-line-spec.md) §3.1 / §4.0 / §4.1（C-B4 行）。
>
> ⚠️ **本波只做审计与裁定，不写实现**。落地由后续施工批按本稿目标形态执行。

---

## 0. 一句话结论

> **推荐选项 ①（收窄版：无 JS 引擎的会话化交互面）**，但**必须改名**
> ⇒ 能力 id 取 **`tool.web_session`**（**不叫** `tool.web_browser`），
> 且**「多步会话」在当前受控出网上不可实现**（响应头未暴露 ⇒ 读不到 `Set-Cookie`
> ⇒ 无 cookie jar）⇒ 建议**分两档**：
> **①a（0 改动 egress，立刻可施工）** 与 **①b（需 egress 加响应头读取面，作为后续小批）**。
> **选项 ②（引浏览器引擎）移交 owner 逐项重拍**；**选项 ③（移交）不成立**（仓内无等价面）。

---

## 1. 审计范围与方法

### 1.1 判定口径

沿本线 §1.1b 三档（[设计稿](./capability-creation-line-spec.md) §1.1b）：

| 档 | 定义 | 处置 |
|---|---|---|
| **真新面** | 仓内**无任何**实现该族 job 的面 | 本线开工 |
| **部分重复（需补强）** | 已有面实现了该族 job 的**可识别子集** ⇒ 必须写清「**已有 X（file:line）· 缺 Y**」 | 开工，**只补 Y** |
| **纯重复（应移交）** | 已有面与该族**等价**（同 job + 同目标域 + 同治理档） | 移交存量线 |

### 1.2 审计过的面（只读，全仓扫描）

`crates/capabilities/tools/src/`：`stealth_crawler.rs` · `fetch.rs` · `fetch/accessibility.rs` ·
`fetch/html_text.rs` · `egress.rs` · `http_write.rs`（C-B2 在途） · `content_feed/` ·
`mcp_bridge/` · `external.rs` · `recon/`；
`crates/adapters/sdk/src/client.rs` · `crates/engine/runtime/src/canonical/`（深度维面，见 §6.4）。

---

## 2. 逐个先例的重复判定

| # | 先例 | 判定 | 已有 X（file:line） | 缺 Y | 处置 |
|---|---|---|---|---|---|
| P1 | `stealth_crawler.rs` | **部分重复（薄）** | UA 池选取 `:83-92` · 外部内容封套（**自造第三套**）`:113-127` · script 块剥离 `:159-198` · 结构化抽取 `:201-226` | **零 IO**：无导航 / 无会话 / 无表单 / 无写 | **不并入新面**（见 §2.1） |
| P2 | `fetch.rs`（`tool.fetch`） | **部分重复** | 受控 GET `:436` · 文本解码 `pub(crate)` `:674-735` · 派生视图（title/text/accessibility）`:523-556` · 不可信封套 `:485` | **多步 / 会话状态 / 表单提交 / 按索引动作** | **复用不重写**（§5.5） |
| P3 | `fetch/accessibility.rs` | **部分重复** | `extract_tree` `:205` · `NodeRole` 闭集 `:31-60` · 树深上限 `:20,:26` | **把快照变成可交互元素**（可激活类型 + 索引定位） | **复用为元素模型**（§5.3） |
| P4 | `fetch/html_text.rs` | **部分重复** | `extract_links` `:137`（**当前无生产调用点**，仅单测 `:250,:269`）· `extract_title` `:177` · `extract_text` `:30` | 表单字段抽取（`<form>/<input>`） | **复用 + 补表单抽取** |
| P5 | `egress.rs`（`ControlledEgress`） | **部分重复（且是硬依赖）** | GET `:601,:609` · **写方法 `write` `:708`** · `EgressMethod` 闭合词表 `:421-452` · 白名单 `:223-249` · SSRF/DNS 判定 `:272-357` | **响应头未暴露** ⇒ 读不到 `Set-Cookie` | **必前置依赖**（§6.1） |
| P6 | `http_write.rs`（C-B2 同波在途） | **部分重复** | 写面 + 凭据链 + 闭合动作词表（文件头 `:1-40`） | **会话上下文**（表单提交要带会话态） | **复用 `ControlledEgress::write`**，不新造写通道 |
| P7 | `mcp_bridge/` | **部分重复（治理面）** | 动态族 `tool.mcp.<server>.<tool>`（`mcp_bridge/family.rs:20`）· 远端工具暴露 + 审批 + 治理 | **本机**会话驱动 | **不重复建设**，边界写清（§5.5） |
| P8 | `content_feed/` | **部分重复（薄 · job 不同）** | 订阅源 + 受控拉取 + 摘要 + 入记忆流（模块头 `:1-40`） | 即时交互 + 结果进上下文 | 边界写清（§5.5） |
| P9 | `adapters/sdk/src/client.rs` | **非本族** | SDK **远程调用**契约白名单 `:76` · 子路径 `:226` · 客户端方法 `:656`（`web_search`） | —— | **不是本族**；本族不撞名（§5.6） |
| P10 | `external.rs` | **非本族（但可借形）** | 外部二进制适配 + `Evidence` 证据形状（文件头 `:1-13`） | —— | 借**证据字段纪律**，不搬进程证据形状 |

> ⇒ **逐档汇总**：**真新面 = 0**；**部分重复 = 7**（P1–P8）；**纯重复 = 0**
> ⇒ ⇒ **不适用移交（选项 ③ 不成立）**；本族按 §1.1b 只补「缺 Y」。

### 2.1 P1 的处置理由（**先定性再复用**的结论）

* `stealth_crawler.rs` 的能力集中在**反检测伪装**（UA 池 / canvas 噪声 / WebGL 厂商改写
  / `navigator.webdriver` 抑制，`:22-46`）⇒ **与「受控会话交互」无关**，且
  **规避站点对自动化的检测 = 伪装**，与本线「硬门 + 逐次人批」的治理取向相反
  ⇒ **不并入新面**。
* 它 **零调用点**（全仓仅 `lib.rs:337` 再导出）⇒ 不在生产里 ⇒ 不并入**不破坏**既有行为；
  它是**死代码**，**清理不在本波写域**（只报备，不动）。
* ⚠️ 它**自造了第三套第三方内容封套**（`<<<[UNTRUSTED_CONTENT source="…"]>>>`，`:113-127`），
  而仓内另有 `UntrustedEnvelope`（orchestration）与 `UntrustedContentWrapper`（governance）
  ⇒ ⇒ **新面不得再造第四套**（§6.3）。

---

## 3. 范围裁定（三个选项）

### 3.1 选项 ①：**收窄版**（无 JS 引擎）—— **推荐**

**能做什么**

| 能力 | 落点（复用既有件） |
|---|---|
| 导航（取一页 + 推进） | `ControlledEgress::get_with_headers`（`egress.rs:609`） |
| 按索引**点击**（**仅链接类**） | `extract_tree`（`accessibility.rs:205`）→ 链接 href → 再 `navigate` |
| 表单提交 | `ControlledEgress::write`（`egress.rs:708`，C-B2 已落） |
| 读页（标题 / 文本 / 链接 / 无障碍树） | `extract_title` / `extract_text` / `extract_links` / `extract_tree` |
| 多步（回合内串行） | 动作词表顺序执行 |

**不能做什么（必须写进模型面描述与自述面）**

* **无 JS 引擎 ⇒ 取不到渲染后 DOM** ⇒ 单页应用 / 前端渲染的列表 / 折叠面板 /
  懒加载内容 **一律取不到**（只拿到 JS 壳）。
* **按钮类元素不可激活**（其动作由 JS 绑定）⇒ `click` 只对**链接**有效，
  其余一律**拒绝即帧**并说明。
* **无截图 / 无像素 / 无视频 / 无音频**。
* **无反检测伪装**（§2.1）。
* **无 cookie jar** ⇒ 站点侧**会话不连续**（见 §6.1；这是 ①a 的硬边界）。

### 3.2 选项 ②：**引外部浏览器引擎**（需 owner 逐项重拍）

**⚠️ 先纠正一个前提**：任务书写「引浏览器引擎 ⇒ 与本线 0 新外部包纪律正面冲突」。
**证据表明此前提不成立**：

* 「0 新外部包」是 **W1 的档位属性**，不是全线纪律 ——
  [ADR-0025](../adr/0025-capability-creation-line-charter.md) §4.1 波次表：
  W1 =「**0 新外部 dep 档**」；**W3/W4（平台面）预算明写 `dep +1~2`**
  （[设计稿](./capability-creation-line-spec.md) §5 W3 行同口径）。
* ⇒ **选项 ② 不是「破纪律」，而是「吃已授权的 dep 额度」**。

**真实代价（为什么仍建议移交给 owner）**

1. **运行时二进制依赖**：浏览器引擎通常要一台**真浏览器二进制**
   ⇒ 那是**部署依赖**（分发、体积、版本对齐、升级），不是 crate 依赖
   ⇒ 仓内**零先例**，且与「桌面代理」的离线交付口径冲突。
2. **沙箱 / 进程边界**：浏览器 = 子进程 ⇒ 要走既有 `ProcessExecutor` 边界 +
   每站点独立 profile 目录 + 资源上限 ⇒ 与 **C-A1（原生 GUI 面）** 强耦合
   ⇒ 两面必须**同波**设计，否则会出现两套「外部进程」边界。
3. **供应链**：新引擎 crate 与 `deny.toml` 的 `multiple-versions = "deny"` 冲突风险
   （ADR 风险台账 R2）。
4. **治理取向**：引擎自带反检测能力 ⇒ 与「硬门 + 逐次人批」取向相反
   ⇒ 需要**单独**的治理裁定（是否允许伪装？）。
5. **不可测**：真实浏览器路径无法在无头 CI 里稳定复现 ⇒ 对抗测试成本高。

### 3.3 选项 ③：**移交** —— **不成立**

仓内**无任何**等价面：P2 只做单 URL 单次 GET（无会话 / 无写 / 无按索引动作），
P1 零 IO，P7 是**远端**工具桥 ⇒ **job 不同 + 目标域不同 + 治理档不同**
⇒ 按 §1.1b 不构成「纯重复」⇒ **不适用移交**。

### 3.4 裁定结论

| 选项 | 结论 | 理由（一句） |
|---|---|---|
| ① 收窄版 | ✅ **采纳（建议）**，且**必须改名** | 0 新外部包即可交付真价值；但**无 JS ⇒ 不能叫 browser** |
| ② 引引擎 | ⏸️ **移交 owner 逐项重拍** | 真实代价是**运行时二进制 + 沙箱 + 供应链**，不是预算纪律 |
| ③ 移交 | ❌ 不成立 | 仓内无等价面（真新面 = 0，但部分重复 ≠ 纯重复） |

> **改名是裁定的一部分，不是细节**：本单元的立项目的正是「避免造一个
> **看着是浏览器、其实不是**的面」⇒ **裁 ① 却仍叫 `tool.web_browser`
> = 直接违约** ⇒ **推荐 `tool.web_session`**。
> 若 lead 坚持保留 `tool.web_browser`，则**必须**在模型面描述里明写
> 「**无 JS 引擎 ⇒ 动态渲染页面取不到**」，并把这笔「名字过度承诺」的诚实债
> 记进进度页。

---

## 4. 可施工目标形态（裁 ①）

### 4.1 身份

| 项 | 取值 | 理由 |
|---|---|---|
| 能力 id | **`tool.web_session`** | 裁 ① 无 JS ⇒ `browser` 过度承诺（§3.4） |
| 模型面工具名 | `web_session` | 与 id 同族短名（沿 `tool.fetch`/`fetch` 的既有惯例） |
| Rust 类型名 | `WebSessionTool` | **不得**出现 `BrowserTool` 字样（守卫禁名单，见 §6.2） |
| 落点 | **`crates/capabilities/tools/src/web_session.rs`**（新文件，< 800 行） | **不新 crate**（§7） |

### 4.2 动作词表（**闭合 5 个**，表外拒绝即帧）

| 动作 | 语义 | 治理档 |
|---|---|---|
| `navigate` | 取一个 URL 并推进当前页（GET） | **R2** |
| `click` | 按**当前页无障碍树**的元素序号激活（**仅链接类**；其余拒绝即帧） | **R2** |
| `submit_form` | 提交一个表单（POST；字段来自当前页 `<form>` 抽取或显式给定） | **R3** |
| `read_page` | 读当前页（标题 / 文本 / 链接表 / 无障碍树快照） | **R0**（不新出网） |
| `close_session` | 结束当前会话（清会话态） | **R0** |

> 治理档依据 [ADR-0025](../adr/0025-capability-creation-line-charter.md) §2.4：
> R2 = 白名单 + 逐次人批（可逆外部动作）；R3 = 硬门 + 逐次人批（不可逆/破坏），
> 且 ADR **明文把「浏览器提交」列为 R3** ⇒ `submit_form` 归 R3。

### 4.3 结果形状与**证据帧**（动作必带证据帧）

每个动作结果**必须**带一组可复核事实（不得出现「成功」而无证据）：

| 证据字段 | 内容 |
|---|---|
| `action` / `request` | 动作名 · URL · 方法（GET/POST）· 是否跟随重定向 |
| `response` | 状态码 · 字节数 · 内容类型 |
| `extraction` | 标题 · 链接数 · 无障碍树节点数 · 文本长度 |
| `evidence` | 每步一条**带序号**的证据行（可复核事实 + 失败真因） |

* 失败**不假装成功**：真因进证据帧（沿 `external.rs` / `recon::Evidence` 的字段纪律）。
* **第三方内容一律进仓内既有不可信封套**（§6.3）⇒ 页面正文 / 标题 / 链接文本
  都在信封内；**信封来源标签 = 该页 URL**。

### 4.4 域名白名单语义（**白名单外必拒**）

* 出网策略**只认** `EgressPolicy::ExplicitAllowList`（`egress.rs:256-260`）
  ⇒ **不得**用 `PublicInternetOnly` / `Unrestricted` 构造本面。
* 白名单未配置 ⇒ **空表 ⇒ 全拒**（fail-closed）⇒ 绝不「默认全网可达」。
* ⚠️ **匹配口径必须写清**：`EgressAllowList::contains`（`egress.rs:241-248`）是
  **精确 host[:port]** 匹配（**大小写不敏感、无通配、子域不隐含**）
  ⇒ 配置里**每个子域要逐条登记**。
* 拒绝发生在**接触目标之前**（`egress.rs:531` `validate_destination` 在 DNS 之前）
  ⇒ 对抗测试必须断言「**零 TCP 连接**」。

### 4.5 env（**2 个**）

| env | 用途 | 默认 |
|---|---|---|
| `APEIRETH_ENABLE_WEB_SESSION` | 族开关（注册即闸） | **off**（关时逐字节零变化测试要跟） |
| `APEIRETH_WEB_SESSION_ALLOW_HOSTS` | 精确主机白名单（逗号分隔） | **未设 = 空 = 全拒** |

> ⚠️ **规格内部不一致**：[设计稿](./capability-creation-line-spec.md) §3.1 写 C-B4
> 「env+2」，§4.0 写「**新名 1**：`APEIRETH_ENABLE_WEB_BROWSER`」
> ⇒ **本裁定取 2**：白名单必须可配，否则「白名单外必拒」无法落地。
> 两个 env 都要登记 `docs/architecture/env-registry.toml`
> （default=off + purpose + evidence + 关时零变化测试）。

### 4.6 不做的（防「看着像浏览器」）

JS 执行 · 渲染后 DOM · 截图/像素/音视频 · 反检测伪装 · 多浏览器 profile ·
代理配置 · 跨站 cookie 携带（只按 host 存、只回送同 host）· 任何写本地文件的动作。

---

## 5. 边界（防重复）

### 5.1 与 `tool.fetch`（P2）

| | `tool.fetch` | `tool.web_session`（新面） |
|---|---|---|
| 请求次数 | **一次** | **多次**（顺序动作） |
| 会话态 | **无**（明确不帶 cookie / 凭据 / 代理） | ①a 无 · ①b 有（同 host cookie） |
| 写动作 | **无**（GET only） | `submit_form`（POST，走 `ControlledEgress::write`） |
| 按索引动作 | **无** | `click` |
| 出网 | 受控 GET | 受控 GET + 受控写（**同一传输层**） |

⇒ **同一页只走一条路**：新面**不得**自建取页/解码/抽取逻辑
⇒ 必须复用 `decode_text_body`（`fetch.rs:674`，`pub(crate)`）与
`extract_title` / `extract_text` / `extract_links` / `extract_tree`。

### 5.2 与 `tool.web_search`（C-H4）

* `tool.web_search` = 对**检索端点**的一次查询 ⇒ 返回**带来源标注**的结果集（只读面）。
* 新面 = 对**具体页面**的多步交互。
* ⇒ `web_search` 的结果 URL 可以作为 `navigate` 的输入；**两者不共享会话态**。

### 5.3 与 C-A1（原生 GUI 面）

* C-A1 = **原生 OS 控件树**（平台无关的 `native_control_tree` 语汇）。
* 新面 = **网页内的**元素树 ⇒ 复用既有 `AccessibilityTree`（`fetch/accessibility.rs`）
  ⇒ **不新造第二棵 DOM 树**，也**不**把 C-A1 的原生定位器语汇搬过来。

### 5.4 与 `mcp_bridge`（P7）

* MCP 桥 = **远端服务自带**的工具（动态族 `tool.mcp.<server>.<tool>`），走 MCP 既有治理链。
* 新面 = **本机** + 受控出网 + 逐次人批。
* ⇒ 两者可**同时存在**（远端有远端浏览器服务时，MCP 桥已能覆盖）⇒
  **不重复建设**；新面的存在理由是「本机、无外部服务依赖、可审计」。

### 5.5 与 `content_feed`（P8）/ `external.rs`（P10）

* `content_feed` = **订阅源 + 入记忆流**（内容不进提示词，走 L2 封套披露）。
* 新面 = **即时交互 + 结果进上下文（带封套）**。
* `external.rs` 的 `Evidence` 形状**只借字段纪律**，不搬进程证据形状。

### 5.6 与 SDK 契约（P9）

* `adapters/sdk/src/client.rs:76` 的 `web_search` 是**远程调用面**的契约名，
  与本仓 `tool.*` 能力 id **不是同一命名空间**（同名是 C-H4「天然对齐」的来源）。
* ⇒ 本族**不必动 SDK**（SDK 白名单里没有浏览器类工具；本面是**本机**工具面）。

---

## 6. 架构不变量核对（实现前必读）

### 6.1 出网：必须走 `ControlledEgress`（**不得自带第二个 HTTP 客户端**）

* 既有守卫：`crates/capabilities/tools/tests/architecture_invariants.rs:145`
  （`fetch_tool_uses_controlled_egress_and_no_second_http_client`）。
* ⇒ 新面文件内 **`reqwest::` = 0 命中**；取页走 `get_with_headers`，表单走 `write`
  （`egress.rs:601,:609,:708`）。
* ⚠️ **前置依赖（新发现，影响「多步会话」）**：
  `EgressHttpResponse`（`egress.rs:397-405`）**不暴露响应头**
  （`get_inner` 只取 `Content-Type` / `Content-Length`，`egress.rs:581-590`）
  ⇒ ⇒ **读不到 `Set-Cookie` ⇒ 做不出 cookie jar ⇒「站点侧会话」在当前传输层上不可实现**。
  ⇒ ⇒ **①a 的诚实边界 = 「回合内多步、站点侧会话不连续」**；
  **①b**（真会话）需在 `egress.rs` **只加传输语义**地暴露响应头读取面
  （照 C-B2 加 `EgressMethod` / `write` 的既有形状），且 `Set-Cookie` 属**凭据形状**
  ⇒ **不回显、不入日志、过形状层扫描**。

### 6.2 公共契约面：不得出现平台机制名 / 不得出现 `BrowserTool`

* 既有守卫：`architecture_invariants.rs:113`（平台机制名名单）
  ⇒ 新面公共契约只说「导航 / 点击 / 表单 / 读页」这类**平台无关动作词**；
  引擎/协议机制名（若将来有）只落 backend。
* `architecture_invariants.rs:101` 把 `BrowserTool` 列入 **egress 反向依赖禁名单**
  ⇒ ⇒ 新面类型名取 `WebSessionTool`；且 **`egress.rs` 内不得出现新面类型名**
  （依赖方向恒为 tools → egress，单向）。

### 6.3 第三方内容：走既有封套，**不得造第四套**

* 仓内已有：`UntrustedEnvelope`（orchestration，固定警告头 + 显式边界 + 边界伪造转义
  + 每源预算 + 分级省略）· `UntrustedContentWrapper`（governance）·
  `stealth_crawler.rs:113-127` 的**自造第三套**。
* ⇒ 新面**复用** `UntrustedEnvelope`（带每源预算与分级省略，更适合多页多源）
  ⇒ **不自造第四套**；页面正文 / 标题 / 链接文本**全部**在信封内。

### 6.4 与 C-H2（深度维）的接口

* C-H2 的 `DepthQuota` 计量的是**调用树深度**（`quota.rs:308-434`），
  与「网页里的嵌套」无关 ⇒ **两者不共享语义**。
* ⚠️ 但**表单提交会引入一条新的嵌套面**：新面是**工具内部**的出网序列，
  **不是** runtime 的调用树 ⇒ ⇒ 新面**不得**自行改写深度计数，
  也**不得**把「页面内深度」塞进深度维。

### 6.5 要新增的守卫（施工批必须同批落）

1. `web_session.rs` 内 `reqwest::` = 0，且必须出现 `ControlledEgress`
   （照 `architecture_invariants.rs:145` 的形状）。
2. `egress.rs` 内不得出现 `WebSessionTool` / `ToolRegistry` 等
   （照 `architecture_invariants.rs:92` 的形状扩名单）。
3. 公共契约面不得出现 `BrowserTool` 字样 + 平台机制名名单。

---

## 7. 额度影响（对照 ADR-0025 §2.2 授权）

| 棘轮 | 现值 | 授权上限 | **裁 ①（①a）净增** | 规格原假设 | 差异 |
|---|---:|---:|---:|---|---|
| `max_workspace_crates` | 31 | 35 | **+0** | +1（新 crate） | **省 1** |
| `max_production_edges` | 118 | 128 | **+0** | +2 | **省 2** |
| `max_distinct_names`（env） | 160 | 176 | **+2** | +2（§3.1）/ +1（§4.0） | 取 §3.1 口径 |
| `max_file_loc` | 6300 | 6300 | ±0 | ±0 | 新文件 < 800 行 |
| **新外部 dep** | 0 | —— | **+0** | 「选型待定」 | **W3 的 `dep +1~2` 额度省下** |

* **不新 crate 的理由**：ADR §2.2 把 crate 额度限定给「**新依赖闭包必须隔离**的
  4 个新 crate（web / automation / trust / local-inference）」⇒ 本族 0 新外部依赖
  ⇒ **无隔离需求 ⇒ 不吃 crate 额度**（也不产生新的 workspace 成员边）。
* ⚠️ **env 基线已越线**：`scripts/arch_guard.py --strict` 实测 `env=161`（>160），
  违规来自 C-H5 的 `APEIRETH_ENABLE_CALENDAR` 未登记 ⇒ 本族 +2 后为 163
  （≤176 授权上限，但**当前合同基线已越**）⇒ 收口时须复评。
* ⚠️ `edge` 口径以 `scripts/arch_guard.py` 的**成员边**计数为准（成员间依赖），
  本族落在既有 crate 内 ⇒ 0 条新成员边；**具体数字以收口时 guard 实测为准**。

---

## 8. 不做清单 / 诚实口径

* **本波不做实现**：本稿只出审计 + 裁定 + 目标形态；**代码零改动**。
* **未实测**：本族**零实现** ⇒ 无测试数字；选项 ① 的「无 JS ⇒ 取不到渲染后 DOM」
  是**由目标形态推导**的边界，**不是**实测结论（施工批须用本地假端点补齐）。
* **未接线**：无 env 登记、无生产注册、无策展表条目 ⇒ 落地由后续施工批按 §4 接线。
* **待 owner 拍板**：① 改名（`tool.web_session` vs 保留 `tool.web_browser`）·
  ② 选项 ②（引引擎）是否单独立项 · ③ 白名单 env 名是否改名以贴合最终能力 id。

---

## 9. 收口待办（**不在本波写域**，由 lead 执行）

1. `python3 scripts/docs_index.py --gen` —— **新建文档必须进 `docs/INDEX.csv`**
   （`scripts/docs_index.py:317-323` 断言「每份 .md 都要被 INDEX 覆盖」）
   ⇒ 本波**不得**改 `docs/**`（除本稿）⇒ **由 lead 收口时生成**。
2. `python3 scripts/check_doc_class.py --check` —— 本稿 `Class: DECISION`
   （含「选项」与「建议」⇒ 合规）。
3. `python3 scripts/docs_links.py --check` —— 本稿内链均指向既有文档。
4. 若接受改名 ⇒ 同步更新 [设计稿](./capability-creation-line-spec.md) §3.1 / §4.0 的
   C-B4 行（能力 id + env 名 + 额度），并回填进度页。

---

## 10. 复核命令（可复算）

```bash
# ① 重复审计：先例证据可复核
rg -n "select_user_agent|wrap_untrusted_content|parse_scraped_document" crates/capabilities/tools/src/stealth_crawler.rs
rg -n "get_with_headers|decode_text_body|extract_tree" crates/capabilities/tools/src/fetch.rs
rg -n "pub fn extract_tree|pub fn extract_links|pub fn extract_title|pub fn extract_text" crates/capabilities/tools/src/fetch/
rg -n "pub struct EgressHttpResponse|pub async fn write|pub async fn get_with_headers" crates/capabilities/tools/src/egress.rs
rg -n "MCP_FAMILY_ROOT" crates/capabilities/tools/src/mcp_bridge/family.rs
rg -n "web_search" crates/adapters/sdk/src/client.rs | head -5
# ② 额度与守门
python3 scripts/arch_guard.py --strict
python3 scripts/check_doc_class.py --check
python3 scripts/docs_links.py --check
python3 scripts/docs_index.py --check   # 新文档未进 INDEX ⇒ 预期失败（待 lead --gen）
```

---

## 11. 引用

* [能力创造线设计稿](./capability-creation-line-spec.md) —— §1.1b 判定口径 · §3.1 C-B4 行 · §4.0 C-B4 行 · §5 波次 · §8.0 既有架构不变量
* [ADR-0025](../adr/0025-capability-creation-line-charter.md) —— §2.2 授权额度 · §2.4 治理档 R0–R4 · §4.1 波次表
* [进度页](../status/capability-creation-line-progress.md) —— 做到哪了（唯一权威）
* [工作线登记表](../status/workline-registry.md)
