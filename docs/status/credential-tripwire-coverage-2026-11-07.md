# 结果面凭据绊线 · 覆盖盘点（2026-11-07）

> **用途**：合并收口工作线 §5-9 的**盘点结论**（AGENTS.md §0.2：状态只引文件，不引对话）。
> **性质**：**盘点件 + 方案件**，**未实施任何接线**。⚠️ 各批方案里的断言都是**待做的验收判据**，
> **不是**已验证结论。
> **Owner**：主代理（总工程师线）｜**工作线**：跨线合并收口（进度页见
> [merge-integration-closeout-2026-11-07.md](merge-integration-closeout-2026-11-07.md)）

## 0. 一句话结论

**凭据形状绊线（M1）** 覆盖 **4 族**（shell + filesystem + search + repo —— 后三族为**批次 1 新增**，2026-11-07 落地）；**出境清扫（M2）** 仍只覆盖 **外部工具族 1 族**；⇒ **26 族**里 **22 族**未接凭据形状绊线。
⇒ **owner 7 条裁决已定案**（§5）；⇒ **批次 1 已落地**（默认开的三族）；⇒ 批次 2（出网/探针族）、批次 3（单一卡口 / 记忆面 / 类 B 通道）待做。

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
| F4 | `filesystem` | `filesystem.rs:277` | **默认开** | ✅ **M1**（批次 1，`filesystem.rs:283`（read 面）/`:350`+`:353`（list 面）/`:393`（stat 面））| **A** |
| F5 | `search` | `search.rs:455-471` | **默认开** | ✅ **M1**（批次 1，`search.rs:499-505`（结果面：查询回显 + 路径 + 命中行正文））| **A** |
| F6 | `repo`（git） | `repo.rs:188-193`/`:208` | **默认开** | ✅ **M1**（批次 1，`repo.rs:248`（成功面）/`:273`（错误面））| **A** |
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
- **已接**：F1（shell，M1）+ **F4 / F5 / F6（filesystem / search / repo，M1 —— 批次 1 新增）** ⇒ **M1 覆盖 4 族**；F3（MCP，M2）⇒ **M2 仍只 1 族**。
- **默认开却零凭据形状绊线**：~~F4 / F5 / F6~~ ⇒ ✅ **批次 1 已补上**；⚠️ **剩下的默认开族**：**F7 `repo_map`**（CLI `:2152`）**未接**（批次 1 故意只做三族 ⇒ 接线守卫的期望集恰为 {shell, filesystem, search, repo}）⇒ ⇒ **它是下一个默认开且未接的族** ⇒ 批次 2 或单开一条时接，届时**守卫期望集须同步扩为五**。
- **部分覆盖**：F14 / F15（只有 M3 正则：**不认** PEM / JWT / Bearer / `password:` 赋值形态）。
- **完全无**：F2 / F8（记忆面）/ F9 / F10 / F12 / F13。

## 2.5 批次 1 落地后的**实现事实**（2026-11-07，含两处「如实登记」）

### 2.5.1 层序 = **M1（形状层）→ M4（字段层）** + 一道**桥接**
- 初版接线是「M4 → M1」⇒ 实测 **`password=` 一族会再遮一次**（M1 只豁免**自己那套大写** `[REDACTED_…]`，
  对 M4 的**小写** `[redacted]` 不豁免）⇒ ⇒ 可见标签漂成 `[REDACTED_PASSWORD]` ⇒ ⇒ **裁决改为「M1 → M4」**。
- ⚠️ **纯对调达不到「与接线前逐字节相同」**：字段层实现会跳过 `[` 开头的值 ⇒ M1 遮出的标签落在字段值位时
  字段层不接管 ⇒ ⇒ 故共享助手加**桥接**（`guardrail.rs:772-812`）：形状层标签先换成**字段层认得的普通词占位**
  （`__TRIPWIRE_MASK_n__`，**刻意不含** `:` `=` `,` `}` `]` `#` 这些字段层分隔符；**且刻意避开 `APEIRETH_[A-Z0-9_]+`** ⇒ 那是本仓 **env 开关命名空间** ⇒ 占位若进入该形态，`arch_guard` 会把它当**新增 env 开关**计数 ⇒ env 预算 161 > 160 ⇒ **棘轮会红**）→ 字段层照常接管 →
  字段层**没接管**的占位再还原成形状层标签 ⇒ ⇒ **字段值归字段层、裸形状归形状层**。
  ⇒ ⚠️ **内核一字未改**（`scan_and_sanitize_output` 及其豁免逻辑未动）⇒ ⇒ 桥接只是**两层之间的标签交接**。
- 实测四种形态：`password = <明文>` ⇒ `password = [redacted]` + 报 `PlaintextPassword`（真命中）；
  裸 `sk-…` ⇒ `[REDACTED_OPENAI_KEY]`；`api_key = "sk-…"` ⇒ `api_key = "[redacted]"` **且仍报 `OpenAiKey`**
  （旧层序下该命中是**静默**的 ⇒ ⇒ 层序换向的净收益）。

### 2.5.2 两处**如实登记**（不藏）
1. **repo 族不接字段层（M4）** —— repo 接线前**没有**字段层 ⇒ 接了会引入**新的可见变形** ⇒ ⇒
   保持零可见变化（形状层仍遮 ⇒ **明文不泄漏**）⇒ ⇒ 与 filesystem/search 的处理**不一致**，**如实记录**。
2. **形状层的行尾截断**（内核既有语义，非批次 1 引入）：形状层把口令值截到行尾 ⇒
   `password = x # 注释` 的**尾部注释**在接线后不可见（接线前字段层保留）⇒ ⇒
   **「逐字节相同」在带尾注释的行上不成立** ⇒ ⇒ **已知边界**（不改内核语义）。

### 2.5.3 共享助手已收口 + 接线守卫的**弱化**（如实记录）
- 三族各自的重复实现**全部删除**，统一到 `crate::guardrail`（`scan_result_face` / `with_field_layer` /
  `attach_credential_tripwire` / `append_credential_tripwire`，均 `pub(crate)`）⇒ ⇒ **不复制到十几族**。
- ⚠️ **接线守卫弱化**：`grep scan_and_sanitize_output` 的集合仍**恰为** {shell, filesystem, search, repo}
  ⇒ 守卫测试与期望集**均未动**；但收口后三族的命中来源变成**注释里对内核 API 的指名**（shell 仍是真调用点）
  ⇒ ⇒ **文本代理变弱** ⇒ ⇒ **裁决：批次 2 扩族时把 grep 模式扩为
  `scan_result_face|scan_and_sanitize_output`**，并把 `repo_map` 加进期望集 ⇒ **期望集扩为五**。

## 2.6 批次 2（+2c）落地后的覆盖实况（2026-11-07）

**批次 2 = fetch / git_write / worktree_apply / re.*（9 工具）+ 批次 2c = ext.*（10 工具）**
⇒ ⇒ **M1（凭据形状绊线）覆盖 9 族**：shell / filesystem / search / repo / fetch / git_write /
worktree_apply / recon / external ⇒ ⇒ **26 族里 17 族未接**（原 24 族未接）。

### 2.6.1 各族的**接线出口**（都是「一次接线覆盖全族」）
| 族 | 唯一结果面出口 | 覆盖 |
|---|---|---|
| `filesystem` / `search` / `repo` | 各自族文件内的结果面构造 | 批次 1 |
| `fetch` | `fetch.rs:129-133` 一带 | 正文 + **最终 URL** + `title`/`text`/`accessibility` **逐项** |
| `git_write` | 成功面 + 失败面 | `output` + **`commands` 回显面** + 失败面 |
| `worktree_apply` | **仅三条失败帧** | 成功路径**一字未改** |
| `re.*`（9 工具） | **`evidence_result()`**（唯一出口；`re.pack` 已并入）⇒ 一次覆盖全族 | 四面 |
| `ext.*`（10 工具） | **`ok_evidence()`**（唯一出口）⇒ 一次覆盖全族 | 四面 + **stderr 回显面**（接线点：`external.rs:106` 截断面 7 个适配器 / `:118` 规则扫描族整段面 / `:545` 元数据族整段面 / `:825` 规则扫描族整段面 / `:1731` 证书链族整段面；**失败面只过形状层**，与 `git_write` / `worktree_apply` 失败面同口径） |

### 2.6.2 **三个坑**（都按坑定位，不是笼统「接一下」）
1. **`fetch` 的正文已被 `UntrustedContentWrapper` 包裹** ⇒ ⇒ **扫描必须在包裹之前**
   （扫包裹后的文本 ⇒ 形状层替换落进信封**内部** ⇒ 信封不再「原样包住原文」；且包裹会做
   边界伪造中和 ⇒ 文本形态已变）⇒ ⇒ 固定顺序：**先扫正文，再包裹**。
2. **`worktree_apply` 的外部原文只在失败帧** ⇒ ⇒ 只接失败帧（无法启动 / 超时 / 非零退出三条）；
   **成功路径是本族自己的结构化信息** ⇒ 不含外部原文 ⇒ 不接（避免无谓的可见变形）。
3. **`Evidence.details` 是任意 JSON** ⇒ ⇒ **递归逐字符串叶子**扫（**不能**整体 `to_string` 后塞回 ⇒
   会破坏结构）⇒ ⇒ 键名/数字/布尔/null/数组长度/嵌套层级**全不动**（实测逐字对比钉死）。

### 2.6.3 共享能力与**泛化收口已完成**（2026-11-07）
- 共享助手（`guardrail.rs`）：`scan_result_face`（纯文本面）/ `with_field_layer`（字段层后过）/
  `scan_json_string_leaves`（**JSON 递归叶子**）/ `attach_credential_tripwire`（对象面侧车）/
  `append_credential_tripwire`（裸文本面末尾一行 JSON 注解）。
- ✅ **泛化收口已完成**（上一条「两份相似实现」遗留**已消**）：「证据信封四面遮形」上收到
  **共享模块** `crates/capabilities/tools/src/evidence_faces.rs`（**新建 146 行**，
  由 `lib.rs` 以 `pub(crate) mod evidence_faces;` 挂载，**只加 1 行**）。
  - **职责切分**：**共享层 = 扫描**（信封形状 / 四面 / 面序 / 层序 / 去重）；
    **族层 = 挂侧车 + 族特有面**（`external` 的 stderr 第五面原样留在 `external/tripwire.rs`）。
  - **重复消掉**：两份逐段对应的四面编排（各 36 行）⇒ 只剩 1 份；
    `recon/tripwire.rs` 121 → **68**（−53）｜`external/tripwire.rs` 186 → **126**（−60）｜
    **两族合计 −113 行**；`external.rs` / `recon/mod.rs` **零改动**（公开函数名与再导出原样有效）。
  - ⚠️ **一条隐式契约 → 已变成显式守卫**：接线守卫按**文件所在族**枚举 ⇒ 若共享模块**自己**绑定
    那三个内核助手字面量，族枚举会多出第 10 族 `evidence_faces` ⇒ 守卫红、且报错**看不出真因**
    ⇒ ⇒ 扫描入口**必须由族文件注入**（`recon/tripwire.rs:49` / `external/tripwire.rs:83`
    各注入一次**真实函数项**）⇒ ⇒ **守卫里已加显式断言**：共享模块**不得**含那三个字面量
    ⇒ ⇒ **变异检验已做**（注入违规 ⇒ 该断言红；还原 ⇒ 绿）。

### 2.6.4 两处**已知残余**（批次 2d 实测如实登记，不粉饰）
1. **截断面跨边界残余**：`ext.*` 的截断面是**既有 400 字合同**（先于本族接线）⇒ 秘密串
   **跨过** 400 字边界时，帧内可能残留**前缀** ⇒ ⇒ 本批**未扩大也未缩小**该残余
   ⇒ ⇒ **裁决：不改成「先扫后截」** —— 理由：那样会让**更多**原文进帧（危险方向相反）
   ⇒ ⇒ **登记为已知残余**。
2. **失败面只过形状层**：stderr 里「键=值但值不是已知形状」的明文（如 `token=<随机串>`）**不遮**
   ⇒ ⇒ **裁决：维持** —— 若要带字段层，就得**放弃**「零命中 ⇒ 逐字节不变」的严格口径
   （干净 stderr 含 `token=` 行时帧就会变）⇒ ⇒ 与 `git_write` / `worktree_apply` 失败面**同口径**。

### 2.6.5 接线守卫：从「少报型」修回「枚举型」
- 原守卫只认内核入口字面量 `scan_and_sanitize_output` ⇒ 走**共享助手**的族**不被枚举**
  ⇒ 批次 2 的族**全漏** ⇒ ⇒ **模式扩为** `scan_and_sanitize_output|scan_result_face|scan_json_string_leaves`
  且**族级判定**（某族可能把接线放在**子模块**里 ⇒ 只取 `file_stem` 会算成 `tripwire` ⇒ 改为「首段目录名优先」）
  ⇒ ⇒ **期望集 = 9 族**（批次 1 四族 + 批次 2 四族 + 批次 2c 一族）⇒ ⇒ 守卫重新覆盖**全部已接线族**。
- ⚠️ **三处刻意留白（如实登记）**：① JSON **键名**里的凭据形状**不遮**（判据「键名不动」是验收项；
  凭据通常在**值**里而非**键**里）② JSON 叶子**只过形状层**（孤立叶子无键上下文 ⇒ 字段层对它本就是空操作）
  ③ `external::tripwire` 的**公开路径未在族层再导出**（族文件行数压在棘轮基线上 ⇒ 无外部调用方需要它）。

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

#### 3a 的**env 开关裁决**（2026-11-07 实测后追加，**替代**裁决 #6 里的「新增 env 开关」）
- ⚠️ **实测发现**：契约 `[env].max_distinct_names = 160`，而**登记表 `env-registry.toml` 恰好 160 条**
  ⇒ ⇒ **env 命名空间已被用满**；`arch_guard` 的 src 口径实测也是 **160/160** ⇒ ⇒
  ⇒ **任何新增一个 distinct `APEIRETH_*` 名（哪怕只写死一次、哪怕只在注释里）⇒ 计数 161 ⇒ 破预算 ⇒ 棘轮红**。
- ⇒ ⇒ 三个选项的取舍：**(a) 前缀拼名**（src 口径计数不变，但**审计面失真**：登记表有全名、src 里没有
  ⇒ ⇒ 等于**绕过守卫的测量**）；**(b) 抬预算到 161**（**违棘轮「只减不增」**，且需 ADR）；
  **(c) 净零腾预算**（要动别线写域）。
- ⇒ ⇒ **曾考虑「复用既有 `APEIRETH_ENABLE_EGRESS_GATE` 加第三形态」⇒ 技术上不通 ⇒ 已放弃**
  （理由：该闸的**实现**在 assembly 层（`runtime-assembly/src/egress_gate.rs`）而 3a 的落点在
  **runtime 层** ⇒ 而 **runtime 不能依赖 runtime-assembly**（方向纪律）⇒ ⇒ 复用它的 env 名会
  **语义与层次双错**（「出网闸」之名管「进 transcript 前的兜底」）⇒ ⇒ 放弃）。
- ⇒ ⇒ **裁决 (e)：采用「已登记前缀 + 后缀」拼名，登记表记全名** ⇒ ⇒
  **src 口径计数不变（160）** ⇒ ⇒ **预算不动**（不抬预算破棘轮「只减不增」）、**不腾预算**
  （需动别线写域）、**不用别的任何绕过** ⇒ ⇒
  ⚠️ **代价必须如实说**：这一手**弱化**了 `arch_guard` 的「新增未登记开关」检测（它扫 src 字面量 ⇒
  看不全名）⇒ ⇒ **审计面因此以「登记表」为准** ⇒ ⇒ **登记表必须记全名**（已记）**且必须写明这个手法**
  （已写进 purpose）⇒ ⇒ **下一个读登记表的人看得见**，而**只扫 src 的人看不见** ⇒ ⇒
  ⇒ 这是**有意识的取舍**，**不是静默绕过**。
- ⇒ ⇒ **扫描器落点**：`crates/foundation/governance/src/credential_sweep.rs`（**新建**）⇒ ⇒
  与 tools 侧那份**行为已验证一致**（38 条语料行为差分 `mismatched=0` + 逐族标签对照）⇒
  ⇒ **两份实现并存**（tools 侧那份在别线写域 ⇒ 跨 crate 收口**另批**）⇒ ⇒ **如实登记**。
- ⇒ ⇒ **接线点**：`crates/engine/runtime/src/canonical/session.rs` 的两处 `messages.push`
  （用户消息 + 工具结果消息**都**经那里）⇒ ⇒ 在 push **之前**构造遮形副本（原 message 不被改）
  ⇒ ⇒ **事件日志面也写遮形副本**（否则 `provider_view()` 从 log 折出来会**绕过**卡口）⇒ ⇒ 两面一致。

#### 3b 已落地（记忆摄取面）+ 两处**如实登记**
- **接线**：`crates/capabilities/tools/src/content_feed/ingest.rs` 的 `stable_ingest_content`
  ⇒ ⇒ 摄取内容入库前过形状层（M1）。
- ⚠️ **读码结论与预期相反（如实登记）**：**散列输入与入库内容是同一份字符串**
  （`crates/engine/memory/src/freshness/ingest_queue.rs:146` 算散列 / `:149` 查门 / `:166` 落库
  是同一个 `content`）⇒ ⇒ 「只遮入库不改散列」在 tools 写域内**不可实现**（要拆开必须改
  `crates/engine/memory/**`）⇒ ⇒ 采取**同点遮形** ⇒ ⇒ **零命中时两者都不变**（散列逐字节不变 ⇒
  散列门语义完好）⇒ ⇒ **命中时散列随之变，但遮形是确定性的** ⇒ 同条目重复拉取仍产生**同一散列**
  ⇒ ⇒ **去重语义不破**；且工具侧更前面的**生命周期条目账**走**原始**字段 ⇒ **无迁移重复风险**。
- ⚠️ **J1 面未闭**：`content_feed/tool.rs:806-823` 的 `episode_for_item` 把同一份标题 + 正文
  **直写** episodic 记忆（不经摄取队列）⇒ ⇒ **J1 仍是明文进存储的面** ⇒ ⇒ 需 **3b-2**（另批）。
- ⚠️ **裁决 #2 与已落地实现相矛盾** ⇒ ⇒ **重新拍板（裁决 (f)）**：**维持批次 2 的实现**
  （`fetch` 接 M1）⇒ ⇒ 理由：`fetch` 取令牌端点的用法**因此不可用**，但这是**有意的取舍** ——
  **宁可该用法不可用，也不让凭据明文出境** ⇒ ⇒ **改写裁决 #2 的口径**为「`fetch` **不接占位族**
  （不造不可还原别名）⇒ 走**单向**标签 M1 ⇒ ⇒ **后果登记为已知限制**（不是缺陷）。
- ⇒ ⇒ **接线守卫已扩为十族**（批次 3b 的 `content_feed` 加入；**追认**该越界改动 —— 它是
  「让 0 failed 成立」的**机械必要**改动）。

#### 3c-1 已落地（占位域归属，**机制面**）+ 主代理复核确认的两条缺口
- ✅ **跨族互认已通**：新增**会话域持有者** `SessionRedactionDomain`（`mcp_bridge/mod.rs`）
  + MCP 侧注入面（`bridge.rs` 的 `with_session_domain` / `tool.rs` 的 `new_with_domain`）
  + M5 闸持同一域（`egress_gate.rs` 的 `with_domain`）⇒ ⇒ **实测**：工具面产的占位符 ⇒ 闸能还原。
- ⚠️ **口径 = IMPLEMENTED（机制面），不是 PRODUCTION WIRED**：装配根与 CLI 适配器**未注入同一域**
  ⇒ ⇒ **生产上仍是两个域**。
- ⚠️ **主代理复核确认的粒度缺陷（如实登记，3c-1b 的输入）**：`McpToolBridge` 持**单个**域
  （`mcp_bridge/bridge.rs:171` 的 `session_domain: Option<Arc<Mutex<SessionRedactionDomain>>>`）⇒ ⇒
  而**桥是进程级** ⇒ ⇒ **一进程一域** ⇒ ⇒ **多会话并发时，跨会话的同一个值会得到同一个占位符**
  ⇒ ⇒ **跨会话可关联** ⇒ ⇒ **违裁决 #4「一个会话一个域」** ⇒ ⇒ 需把域**下沉到逐调用**
  （按 `SessionId` 选域 ⇒ 域注册表）。
- ⚠️ **一处公共 API 改名**：`McpDynamicTool::new_with_sweeper` → `new_with_domain`（仓内唯一调用点是自身；
  既有测试未用它）⇒ ⇒ 仓外消费者需同步 ⇒ ⇒ 若要同名兼容壳 ⇒ 需给 `guardrail.rs` 开取桥的口子
  ⇒ ⇒ **本工作线明确不碰该文件** ⇒ ⇒ 留待裁决。

#### 3c-2 已落地（类 B 通道最小闭环，**默认关**）—— 主代理复核确认
- ✅ **类 B 与占位族**分成**两本登记簿**（这是本批的核心设计）：占位族登记簿的值会被
  **受控回程面**（模型回信 → 本地）还原 ⇒ 真值**可能**进 transcript；**类 B 登记簿**里的值
  **只**认**显式的本地消费面** ⇒ ⇒ **类 B 真值永不进 transcript / 事件日志 / 账本**。
- **声明面**：类 B 声明走**一个可选声明文件**（`CLASS_B_DECLARATION_FILE`）⇒ **可序列化、可审计、
  默认不含任何声明** ⇒ ⇒ 文件缺失或损坏 ⇒ **fail-closed**（按空声明处理）⇒ ⇒ **默认关**。
- **域的注入**：出网闸未开 ⇒ `shared_session_domain()` 返回 `None` ⇒ 域**只能事后注入**
  ⇒ ⇒ **默认关时行为逐字节不变**（有测试钉死）。
- **单向兜底**：类 B 清扫只认「可逆的那几种形状」⇒ **检测器漏网的形状由既有单向绊线贴单向标签**
  ⇒ ⇒ **宁可丢值，不可漏值** ⇒ ⇒ 这条纪律写进了代码注释。
- ⚠️ **口径 = IMPLEMENTED（机制面，默认关），不是 PRODUCTION WIRED，也不是 DEFAULT ENABLED**：
  装配根 `runtime-assembly/src/canonical/production.rs` **未改**（本批只做了 CLI 适配器侧的声明面
  与域注入面）⇒ ⇒ **通道存在但需显式配置才生效**。
- ⚠️ **B1 的误伤现状仍在**：**未声明为类 B 的族**（含 MCP 全族与 `fetch`）**仍按单向销毁**
  ⇒ ⇒ **本批只提供通道，不默认给任何族开声明** ⇒ ⇒ **「通道有了」≠「误伤已修」**。

#### 3c-1b 已落地（域按会话选 + 生产接线）—— **类 B 通道至此 PRODUCTION WIRED（默认关）**
- ✅ **粒度缺陷已修**：`McpToolBridge` 不再持**单只**进程级域，改为持**域注册表**
  `SessionDomainRegistry`（`session_id → 域`，`mcp_bridge/mod.rs`）⇒ ⇒ MCP 工具在
  `invoke_in_session` 里**逐调用按会话取域** ⇒ ⇒ **跨会话同值 ⇒ 不同占位符且互不可还原**；
  ⇒ 同会话跨两工具 ⇒ 仍是**同一占位符**。
- ✅ **会话标识用既有字段（未臆造）**：复用 `ToolCapability::invoke_in_session(…,
  session: Option<SessionId>)`（`crates/foundation/plugin/src/tool.rs:136`）⇒ ⇒ **改动前
  `McpDynamicTool` 未覆写它 ⇒ 会话标识没被消费** ⇒ ⇒ 本批覆写并让它真正生效。
- ✅ **生产接线（同一实例）**：装配根造出**唯一一份**注册表 ⇒ 装进装配层共享槽 ⇒ ⇒
  **M5 闸与 MCP 桥取同一实例** ⇒ ⇒ 跨族同域 ⇒ ⇒ 类 B 通道的**生产前提成立**。
- **默认关**：判据 = **既有登记名** `APEIRETH_ENABLE_EGRESS_GATE` ⇒ **0 新增 `APEIRETH_*` 名**
  ⇒ ⇒ 闸没开 ⇒ **不装注册表** ⇒ ⇒ 行为**逐字节不变**。
- ⚠️ **跨进程 / 跨重启续接不成立（如实登记）**：登记簿**只在内存**；且会话密钥掺了**进程级秘密**
  ⇒ ⇒ 新进程里同一会话 id 派生出的密钥**不同** ⇒ ⇒ 别名不同、真值也没有 ⇒ ⇒ **不假装支持续接**。
- ⚠️ **遗留边界（如实登记）**：**无会话标识**的调用（宿主侧直调）落回**固定的进程级共享域**
  ⇒ ⇒ 该类调用的别名彼此相关，但**与任何会话的域不相关** ⇒ ⇒ **不构成跨会话关联**。
- ⚠️ **本工作线越写域两处（已先报告、主代理裁决追认）**：`runtime/src/canonical/egress_gate.rs`
  （端口加 3 个**带默认实现**的方法 ⇒ **旧实现逐字节不变**）与 `runtime/src/canonical/provider.rs`
  （2 个调用点传会话）⇒ ⇒ **不改则闸拿不到会话 ⇒ 跨族自动互认在生产断链**（3c-1 的性质回退）。
- ⚠️ **口径**：类 B 通道 = **PRODUCTION WIRED（默认关）**，**但声明清单为空** ⇒ ⇒ **实践中仍关**
  ⇒ ⇒ **误伤未修**（未声明族的行为一字未改）⇒ ⇒ **逐族名单待 owner 裁决**。

#### 3c 的调研结论（**只调研未实施**，要点）
- **类 B 通道**：占位族须用**可逆** `__AP_*__` 族（M1 单向标签**必须让位**）⇒ ⇒ 域标识 = 会话 id ⇒
  ⇒ **必须先落「一个会话一个域」**（现在 MCP 是**每工具一域**（`mcp_bridge/tool.rs:341` ←
  `bridge.rs:668`）、M5 又自建一份（`egress_gate.rs:85/98`）⇒ **两侧不互通**）⇒ ⇒ 回程只在
  **本地消费面**（**不进 transcript**）⇒ ⇒ **模型侧永不见真值**。
- **账本**：并入 `egress-ledger.jsonl` 另立段（现有 `EgressLedgerEntry` 已是「类别+指纹」零真值形状）
  ⇒ ⇒ ⚠️ **登记簿只在内存** ⇒ ⇒ **跨重启回程不成立** ⇒ 需单列一批（接 `foundation/credentials`
  的加密后端；该 crate 的装配挂接**未做** = 0 装）。
- **批次拆分**：3c-1 域归属 / 3c-2 类 B 通道 / 3c-3 账本落盘 / 3c-4 B1·B2 止血 / 3c-5（可选）登记簿持久化。
- **B1 现状**：MCP 侧**无任何 restore** ⇒ 真 key 被换成占位符后**就地销毁**（不是藏起来，是**丢了**）
  ⇒ ⇒ 该用法**完全不可用** ⇒ ⇒ **3c-4 止血**必须排在类 B 通道之前或同时。
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
- ⚠️ **反向误伤提醒（2026-11-07 批次 1 实测订正 ⇒ 原句不成立）**：原文称「M4 已把 `key=`/`token=`/`password=`
  换成 `[redacted]` ⇒ 再叠 M1 不会二次误伤」⇒ ⇒ **实测只对一半成立**：M1 只豁免**自己那套大写**
  `[REDACTED_…]` 标签（内核的 `starts_with("[REDACTED_")` 对**小写** `[redacted]` 不成立）⇒ ⇒
  **`password=` 一族会再遮一次**（标签 `[redacted]` → `[REDACTED_PASSWORD]`，且绊线报命中）；
  而 `key=` / `token=` 等**非口令字段不误伤**（已对照实测）⇒ ⇒ **明文没有泄漏**，受影响的是
  「可见标签」与「绊线报不报」⇒ ⇒ **裁决（2026-11-07）：层序改为「M1 → M4」** ⇒ ⇒ 该类内容
  **最终可见输出与接线前逐字节相同**（零可见变化），**绊线仍报口令族**（真实命中）⇒ ⇒
  ⇒ **不扩写**到内核豁免逻辑（那是内核语义层的另一批）。
  ⚠️ **凭据存储件的路径级拒读（`sensitive_path.rs`）不要用绊线替代**（语义不同：一个是拒读、一个是遮形）。

## 5. 裁决记录（owner 2026-11-07 裁决「按你的建议来」⇒ 7 条全部定案）

| # | 原待裁决点 | **裁决** | 理由与后果 |
|---|---|---|---|
| 1 | MCP 远端工具可否合法返回密钥（**现状已在误伤**） | **建受控通道**（类 B 通道），**不放开**扫描 | 现状：远端返回的密钥被换成 `__AP_*__`/`[REDACTED_*]` ⇒ **该工具不可用**。⇒ 「工具合法返回凭据」与「输出面遮形」是**两种语义** ⇒ ⇒ **M1 标签单向不可逆** ⇒ ⇒ 只能走**占位族 + 受控回程** ⇒ ⇒ **新建通道**（不是接线）⇒ 属批次 3c |
| 2 | `fetch` 取密钥的取舍 | **不接 M1**；走类 B 通道 + 声明面写明「本族不做凭据形状脱敏」 | 响应体**就是**凭据 ⇒ 接了必然误伤 ⇒ ⇒ 与 #1 同解 |
| 3 | 批次 1/2/3 统一签字（输出变形 = 用户可见结果变形） | **批准**（批次 1 先做；批次 2/3 各自再过） | 命中即遮形是**预期行为**（零命中时**逐字节不变**）⇒ ⇒ 标签面 = 既有 `[REDACTED_*]` 口径 ⇒ ⇒ **不给「原始值另存」口子**（那会把遮形变成搬运） |
| 4 | 占位域归属 | **一个会话一个域** | 跨族共享 ⇒ 「可逆」承诺才成立（M5 的 `restore_for_local` 才能还原）⇒ ⇒ 每族独立域会让「可逆」名存实亡 |
| 5 | 命中账本落盘口径 | **只落类别 + 指纹**，**不落真值** | 落真值 = **把凭据写进磁盘** = 安全红线 ⇒ ⇒ 只落「命中类别 + 位置指纹」⇒ ⇒ 与 MCP 侧「账本只在内存」的现状**不冲突**（待 3c 统一） |
| 6 | 批次 3a（扫描器下沉 governance + 单一卡口）**强裁决** | **默认关 + env 开关**（照 `APEIRETH_ENABLE_EGRESS_GATE` 先例） | 它改**所有**族可见输出 ⇒ ⇒ 默认开风险过大 ⇒ ⇒ 默认关 + 显式开关 ⇒ ⇒ **0 新依赖边**（runtime 已依赖 governance） |
| 7 | 文档真源订正 | **随实现一起做**（不单开一批） | `capabilities-matrix.md:141` 那行**只列机制名、未列消费方** ⇒ ⇒ 该行**本身不需改**；⇒ ⇒ 待补的是「消费方口径以本文件 §1.1 为准」⇒ ⇒ 与批次 1 落地同批改 |

### 5.1 裁决带来的**执行顺序**
1. **批次 1**（默认开三族 `filesystem`/`search`/`repo` 接 M1）⇒ **批准，先做**（暴露面最大）。
2. **批次 2**（出网/探针族）⇒ 在批次 1 验收后做（技术坑：fetch **包裹前**扫、`Evidence.details` **逐字段**扫、
   `worktree_apply` 只在**失败帧**扫）。
3. **批次 3**（3a 单一卡口默认关 / 3b 记忆摄取面 / 3c 类 B 通道 + 账本落盘）⇒ 最后做，
   其中 **3c 同时落地 #1 与 #2 的裁决**。

### 5.2 诚实边界（裁决**不改变**的既有事实）
- 「其它族未接**凭据形状**绊线」这个缺口**依然存在**直到三批做完 ⇒ ⇒ 现阶段**不得**声称
  「所有工具输出已脱敏」。
- 类 B 通道是**新建**（`crates/foundation/credentials` 的装配挂接**未做**）⇒ ⇒ #1/#2 的裁决
  **在批次 3c 落地前不会生效** ⇒ ⇒ **MCP 误伤现状持续存在** ⇒ **如实登记**。

## 5.3 需 owner 裁决的点（**已全部定案**，保留此节仅为历史留痕）

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
