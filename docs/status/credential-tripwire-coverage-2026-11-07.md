# 结果面凭据绊线 · 覆盖盘点（2026-11-07）

> **用途**：合并收口工作线 §5-9 的**盘点结论**（AGENTS.md §0.2：状态只引文件，不引对话）。
> **性质**：**盘点件 + 方案件**，**未实施任何接线**。⚠️ 各批方案里的断言都是**待做的验收判据**，
> **不是**已验证结论。
> **Owner**：主代理（总工程师线）｜**工作线**：跨线合并收口（进度页见
> [merge-integration-closeout-2026-11-07.md](merge-integration-closeout-2026-11-07.md)）

## 0. 一句话结论

**凭据形状绊线**只覆盖 **2 族**（shell + 外部工具族），**26 族**里 **24 族**未接；
其中 **`filesystem` / `search` / `repo` 三族默认开** ⇒ 暴露面最大、性价比最高。
⇒ **三批落地建议**已给出，但**各批都会让含凭据的输出变形（用户可见）** ⇒ **需 owner 签字**。

## 1. 仓内实有 5 套「凭据相关」机制（盘点前提，**先分清再谈缺口**）

| 代号 | 机制 | 位置 | 默认 | 干什么 |
|---|---|---|---|---|
| **M1** | 凭据绊线内核 `scan_and_sanitize_output` | `crates/capabilities/tools/src/guardrail.rs:415` | 随宿主工具面 | 扫输出里的**凭据形状** ⇒ 单向标签（不可逆） |
| **M2** | 出境清扫 `sweep_output_for_egress` / `EgressSweeper::sweep` | 同文件 `:648` 定义 / `:718` **唯一生产入口** | 随宿主工具面 | 占位族脱敏（**可逆**）+ 调 M1 |
| **M3** | 治理层 `redact_secret_surfaces` | `crates/foundation/governance/src/input_security.rs:258` | — | 正则面：环境式赋值 / `sk-` 族 / 凭据 URL |
| **M4** | 工具层 `redact_secret_field_values` | `crates/capabilities/tools/src/sensitive_path.rs:158` | — | 字段值级（`key=` / `token=` / `password=`） |
| **M5** | 出网闸 | `crates/engine/runtime-assembly/src/egress_gate.rs:246` | **默认关** `APEIRETH_ENABLE_EGRESS_GATE` | 全族占位族（**PII** 侧） |

### 1.1 两处**必须区分**的口径（**经实测 grep 核对**，非推测）
1. **`sweep_output_for_egress` 的生产消费者只有 MCP**（唯一生产入口 `guardrail.rs:718` 的
   `EgressSweeper::sweep`；其余命中全是 `guardrail.rs` 内的 `#[cfg(test)]` 单测 + fuzz 靶子）。
2. **M1 绊线内核另有一个生产消费者 = `shell`**（`crates/capabilities/tools/src/shell.rs:634/637`）。
⇒ ⇒ **准确表述**：**M2 只覆盖外部工具族；M1 覆盖 shell + 外部工具族两族；其余族两者皆无**。
⇒ ⇒ 缺口表述应是「**其它工具族未接凭据形状绊线**」，而**不是**「只有 MCP 用了绊线」。

### 1.2 盘点期新发现（**会绊住后续批次**）
- ⚠️ **MCP 的命中账本只在内存**：`ledger_jsonl()`（`guardrail.rs:728`）的**唯一调用点是测试**
  ⇒ ⇒「脱敏 + 记账」的**记账那半，在 MCP 路径上是 0 装**（运行时 `egress-ledger.jsonl` 记的是**判定**，
  非命中，且默认关）。
- ⚠️ **M2 占位域与 M5 占位域不互通**：MCP 自建 `EgressSweeper::new()`，而 `egress_gate.rs:85/98`
  是**另一份** `RedactionBridge` ⇒ ⇒ MCP 产出的 `__AP_*__` 在 M5 的 `restore_for_local` 里**不可还原**
  ⇒ ⇒ **推广 M2 前必须先定占位域归属**。

## 2. 族清单（26 族，穷尽；判据 = ①走网络/子进程/外部二进制 ②读用户文件或环境变量再回显 ③第三方内容拼进结果）

**默认装配真源更正**：`ProductionModulesConfig::default()` 在
`crates/engine/runtime-assembly/src/canonical/production_config.rs:353-417`（**不在** `production.rs`；
后者 `:78` 只是 `pub use`）。CLI 逐字段覆盖见 `crates/adapters/cli/src/lib.rs:2133+`，
**未被 CLI 覆盖的字段吃 default**。

| # | 族 | 结果面 file:line | 默认 | 已接 | 分类 |
|---|---|---|---|---|---|
| F1 | `shell` | `shell.rs:629`（stdout:649/stderr:650） | 默认关 | ✅ **M1**（:634,:637）+ `credential_tripwire` 标记(:653) | A（已接） |
| F2 | `fetch` | `fetch.rs:481-530`（body/title/text/accessibility） | 默认关 | ❌ 仅不可信包裹(:462) + 输出侧注入扫描(:479) | **A** |
| F3 | MCP `mcp:<srv>:<tool>` | `mcp_bridge/tool.rs:410`/`:440` | 默认关 | ✅ **M2**（:300/:424/:445） | A（已接）**+ 类 B 隐患** |
| F4 | `filesystem` | `filesystem.rs:277` | **默认开** | ❌ 仅 M4 字段值脱敏(:276) + 敏感路径拒读(:176) | **A** |
| F5 | `search` | `search.rs:455-471` | **默认开** | ❌ 仅 M4(:465) | **A** |
| F6 | `repo`（git） | `repo.rs:188-193`/`:208` | **默认开** | ❌ **零脱敏** | **A** |
| F7 | `repo_map` | `repo_map_tool.rs:489` | **默认开**（CLI :2152） | ❌ | A（低） |
| F8 | `content_feed` | 结果只含计数(`content_feed/tool.rs:751-766`)；**记忆摄取面** `content_feed/ingest.rs:151` | 默认关 | ❌ | **A（记忆面）** |
| F9 | `doc_ops` | `doc_ops.rs:4290` | 默认关 | ❌ 仅路径拒读(:448) | **A** |
| F10 | `sheet_ops` | `sheet_ops.rs:3110` | 默认关 | ❌ 仅路径拒读(:482) | **A** |
| F11 | `image_ops` | `image_ops.rs:1548`（仅元数据） | 默认关（双闸） | — 不含凭据 | 不适用 |
| F12 | `ext.*`×10 | `external.rs:127` → `Evidence.details`；**原样回显** = `ext.yara`(:843 raw 8192 字) / `ext.ghidra`(:1543)；含第三方任意串 = exiftool(:538)/tshark(:659)/radare2(:1066) | 默认关 `APEIRETH_ENABLE_EXTERNAL=1` | ❌ | **A**（yara/ghidra 最高危） |
| F13 | `re.*`×9 | `recon_tool.rs:193/338/442/569/658/669/765/860/953/1147`；**`re.pack` 的 `details.strings` = 二进制可打印字符串全量提取**（`recon/pack.rs:42`）⇒ 硬编码密钥原样回显 | 默认关 `APEIRETH_ENABLE_RECON=1` | ❌ | **A** |
| F14 | `git_write` | `git_write.rs:915` | 默认关 | ⚠️ 仅 M3(:765) | A（部分） |
| F15 | `worktree_apply` | `worktree_apply.rs:473-524`（校验命令输出进**错误帧** :511-519） | 默认关（双闸） | ⚠️ 仅 M3(:498) | A（部分） |
| F16-F20 | `worktree_plan`(:379) / `apply_patch`(:331) / `skill_distill`(`skill/distill.rs:417`) / `skill_search`(`skill/search.rs:185-195`) / `proposal_draft`(`proposal.rs:865,885`) | — | 默认关 | F18 ❌ / 其余不适用 | F18 = A（低-中） |
| F21-F25 | `exposure_audit`(`exposure_audit/tool.rs:121`，设计即零明文) / `backup_export`(`backup.rs:1158`) / `voice_speak`(`voice/speak.rs:604` 只报字数) / device×4 / `self_status`·`trait_report`·`task_board`·`care_schedule`·`education` | — | 多默认关；`self_status`/`trait_report` 默认开 | — | 不适用 |
| F26 | `stealth_crawler` | `stealth_crawler.rs:113-142` | **未装配（0 装）**：全仓除自身无调用点 | ❌ | A（仅登记） |

### 2.1 覆盖小结
- **已接**：F1（shell，M1）+ F3（MCP，M2）⇒ **2 族**。
- **默认开却零凭据形状绊线**：**F4 / F5 / F6** ⇒ **暴露面最大 ⇒ 性价比最高**。
- **部分覆盖**：F14 / F15（只有 M3 正则：**不认** PEM / JWT / Bearer / `password:` 赋值形态）。
- **完全无**：F2 / F8（记忆面）/ F9 / F10 / F12 / F13。

## 3. 分批方案（风险 × 改动量排序；**未实施**）

### 批次 1 —— 默认开的本地读三族（F4 `filesystem` / F5 `search` / F6 `repo`）接 M1
- **接 M1，不接占位族**：本地文件内容**不是「出境」** ⇒ 占位族会造出**不可还原**的别名（且无 M5 回程）。
- **风险**：中｜**改动**：3 处｜**默认开 ⇒ 暴露面最大**。
- ⚠️ **可见行为会变形**（仅命中时）⇒ **需 owner 裁决（轻）**；**零命中时逐字不变**。
- **验收判据**：
  ① 文件含 `sk-` + ≥20 字符 ⇒ 结果**无明文**且含 `[REDACTED_OPENAI_KEY]`；
  ② 含 PEM 块 ⇒ 含 `[REDACTED_PEM_PRIVATE_KEY]` 且不含 `PRIVATE KEY-----`；
  ③ search 命中行无明文；④ repo 提交信息里的真 key 无明文；
  ⑤ **干净夹具输出逐字节全等**；⑥ 命中族**可观测**（对齐 shell 的 `credential_tripwire`）；
  ⑦ 接线回归守卫：`grep -rn "scan_and_sanitize_output" crates/capabilities/tools/src --include=*.rs | grep -v guardrail.rs`
  ⇒ 期望**恰为** {shell, filesystem, search, repo}；
  ⑧ `cargo test -p apeireth-tools-canonical --locked` 0 failed + clippy 0 警告 + fmt 0 diff。

### 批次 2 —— 出网 / 子进程 / 远端探针族（F2 `fetch` / F12 / F13 / F14 / F15）
- 接 M1 补齐 M3 **不认**的形状（PEM / JWT / Bearer / `password:` 赋值）。
- **风险**：中高｜**改动**：中（先抽共用 helper）｜⚠️ **可见行为会变形 ⇒ 需 owner 裁决（轻-中）**。
- ⚠️ **三个技术坑**（必须按此落点，否则等于没接）：
  ① `fetch` 正文已被 `UntrustedContentWrapper` 包过(:462) ⇒ **扫描必须在包裹之前**，否则破坏信封；
  ② `external` / `recon` 产物是 `Evidence.details`（JSON）⇒ 必须**逐字符串字段**扫，**不能**整体
  `to_string` 后塞回；
  ③ `worktree_apply` 的输出**只在失败帧**（:511-519）⇒ 选错点等于没接。
- **追加验收判据**：fetch 夹具含 JWT ⇒ 无明文 + 含 `[REDACTED_JWT]` + **信封标记仍在**；
  `ext.yara` ⇒ `details.raw` 无明文且 **Evidence 仍是合法 JSON 信封**；`re.*` details 无明文；
  `git_write` / `worktree_apply` 失败帧无明文；**干净输入逐字节不变**。

### 批次 3 —— 收口三件（**强 owner 裁决**）
- **3a 单一卡口兜底**：把凭据形状扫描器**下沉到 `apeireth-governance`**
  （runtime 已依赖 governance ⇒ `crates/engine/runtime/Cargo.toml:19` ⇒ **不新增依赖边、不产生环**），
  在 transcript 汇聚点 `crates/engine/runtime/src/canonical/execute.rs:3875` 兜一道 ⇒ **一次覆盖所有族**。
  ⇒ ⚠️ **改所有族可见输出** ⇒ **强裁决** ⇒ 建议**默认关 + env 开关**（照 `APEIRETH_ENABLE_EGRESS_GATE` 先例）。
- **3b 记忆摄取面**：`content_feed/ingest.rs:151` 摄取前清扫（或记忆投影面统一扫）。
- **3c 类 B 通道 + 账本落盘**（占位族 + 受控回程；MCP 账本落盘）。
- **验收判据（3a）**：造一个返回真 key 的假能力 ⇒ **进 transcript 的消息**与**会话账本 / 事件日志**
  **都不含明文**（⚠️ 不得只清消息面漏日志面）；**3b**：含真 key 的 feed ⇒ episodic 记录无明文（查库断言）；
  **3c**：账本落盘 ⇒ 重启后仍可查且条目**零真值**。

## 4. 类 B 清单（**后续批次不许接绊线**）

「类 B」= **结果面本身合法地就是凭据** ⇒ 接绊线会**误伤**（把合法返回值脱敏 ⇒ 功能坏）。

| # | 场景 | 现状 | 处置 |
|---|---|---|---|
| B1 | **F3 MCP 远端工具「合法返回密钥」**（远端提供「取我的 API key」工具） | ⚠️ **现状已接 M2 ⇒ 已在误伤**（真 key 被换成 `__AP_*__` / `[REDACTED_*]` ⇒ 该工具不可用） | **需 owner 裁决** |
| B2 | **F2 `fetch` 去取密钥/令牌端点**（URL 响应体**就是**凭据） | 现状未接 ⇒ 不误伤但**明文出境** | **需 owner 裁决**（建议走类 B 通道，不接 M1） |
| B3 | F2 / F12 的**凭据 URL 内嵌形态**（`https://user:pass@host`） | M1 不认；若接入 M3 会误伤 | 同 B2 |

- **现有落点**：占位族 + 受控回程还原（`redaction_bridge.rs:307 restore_for_local` +
  `runtime/src/canonical/egress_gate.rs:48 restore`）= 双闸语义；⚠️ **M1 标签是单向的 ⇒ 类 B 不能靠 M1**。
- `CredentialDisclosureHook`（`governance/src/input_security.rs:742`）是**入参侧**闸，**不是返回通道**；
  `crates/foundation/credentials` 的装配挂接**未做**（其 crate 头自述 0 装）⇒ ⇒ **类 B 通道要新建，不是接线**。
- ⚠️ **反向误伤提醒**：`filesystem` 已用 M4 把 `key=` / `token=` / `password=` 字段值换成 `[redacted]`
  ⇒ 再叠 M1 **不会**二次误伤（M1 的 `password=` 族对已替换值不再命中）；
  ⚠️ **凭据存储件的路径级拒读（`sensitive_path.rs`）不要用绊线替代**（语义不同：一个是拒读、一个是遮形）。

## 5. 需 owner 裁决的点（**实施前必须定**）

1. **MCP 远端工具可否合法返回密钥**（现状**已在误伤**）—— 写进声明？还是建受控通道（**倾向后者**）？
2. **`fetch` 取密钥的取舍** —— 建议**不接 M1**，走类 B 通道 + 声明面写明「本族不做凭据形状脱敏」。
3. **批次 1 / 2 / 3 统一签字** —— 各批都会让含凭据输出变形（`[REDACTED_*]`）= **用户可见结果变形**；
   标签面长什么样？要不要给「原始值另存」口子？
4. **占位域归属** —— 一个会话一个域 vs 每族独立域？（后者则「可逆」承诺**只对 M5 成立**）
5. **命中账本落盘口径** —— 落哪（建议并入 `egress-ledger.jsonl` 同族另立段）？是否落盘？
   ⚠️ 登记簿落盘 = **真值留盘 = 安全红线** ⇒ 建议**只落类别 + 指纹**。
6. **批次 3a 的强裁决** —— 改所有族输出 ⇒ 建议**默认关**。
7. ⚠️ **文档真源订正（随实现一起做）**：`docs/03-reference/capabilities-matrix.md:141` 的「库级守卫」行
   列了 `scan_and_sanitize_output()` / `sweep_output_for_egress()` 两个机制名（**未列消费方**）⇒
   **该行本身不需改**；但**待补**：① M2 的唯一生产入口是 `EgressSweeper::sweep`（`guardrail.rs:718`），
   ② M1 另有 shell 消费者 ⇒ ⇒ 「消费方」口径应以本文件 §1.1 为准。

## 6. 诚实边界

- ⚠️ 本盘点是**静态 grep + 读码级**；**未跑 cargo**（构建/测试/clippy/fmt 一律未跑）⇒
  各批验收断言是**方案**，**不是**已验证结论。
- ⚠️ **F26 `stealth_crawler` = 0 装**（全仓除自身无调用点）⇒ **不为它单开批次**（仅登记）。
- ⚠️ 「MCP 账本不落盘」是**读码结论**（未跑起来核实是否另有落盘路径）。
- ⚠️ `runtime/src/canonical/event_log.rs` 的**事件日志面未穷尽审计** ⇒ 已列为批次 3a 的**必须断言项**，
  **未预判结论**。
