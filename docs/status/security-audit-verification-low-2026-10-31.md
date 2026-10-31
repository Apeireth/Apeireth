# 安全审计复核（低危 / 信息级 + 2 条待验）—— 2026-10-31

[Document-Meta]
Document: docs/status/security-audit-verification-low-2026-10-31.md
Class: AUDIT
Status: 🟢 活跃
Last-Modified: 2026-10-31
Author: verify-bot

> **复核对象**：`/tmp/audit20.json` 中「低危 / 信息级」15 条 + 「待验」2 条，共 **17 条**。
> **复核基线**：分支 `debt-round8`，HEAD = `8ce0f62b`（工作区另有一处与本复核无关的未提交改动：`crates/engine/memory/src/universal_forget.rs`，不涉及本 17 条）。
> **方法**：只读 —— `read` / `grep` / `git log` / `git diff`；**未运行** `cargo build` / `nextest` / `clippy` / 任何测试套件。
> **裁定词表**（每条唯一）：`仍成立` / `已修` / `部分成立` / `不成立` / `无法核实`。

---

## 〇、复核口径与本轮修复状态

### 0.1 裁定口径

| 口径 | 含义 |
|---|---|
| `仍成立` | claim 与前置条件均准确，且**每一条** `file:line` 证据在当前代码仍指向所述内容 |
| `已修` | 问题在当前代码已不存在（本文给出修复点 `file:line`） |
| `部分成立` | 核心 claim 仍成立，但**前置条件 / 影响面 / 证据引用**之一已变（本文逐条指出是哪一条） |
| `不成立` | claim 本身错误 |
| `无法核实` | 代码不足以判定（本文说明卡在哪） |

### 0.2 本轮修复状态（守卫要求的「已修 / 未修」并列口径）

| 项 | 状态 |
|---|---|
| **代码缺陷本轮被修复的条数** | **已修**：**0 条** —— 17 条在当前代码里没有一条被「修掉」，判定全部落在 `仍成立` / `部分成立` |
| **审计记录层面被归并的重复项** | **已修**：**1 条** —— `#8` 与 `#2` 同一指纹（同一缺陷），本轮按「重复项」去重，只保留 `#2` 作为缺陷本体 |
| **仍需修复的条数** | **未修**：**16 条**（`#1`–`#7`、`#9`–`#17`），其中 `#16` 建议由「待验」**提升为 confirmed（low）**，`#17` 建议**不提升**（最多 informational） |

### 0.3 审计基线与当前代码的差异（必须先说清，否则「引用陈旧」会被误读成「问题已修」）

审计产物的 `file:line` 是在**另一棵代码树**上测得的：该树的 `git_write.rs` 把 `split_command_segments` / `tokenize_segment` 放在子模块 `git_write/parse.rs`（本仓 `feat/env-lock-crate-level` 分支即此形态），而**当前树 `debt-round8` 已把这两段内联回 `git_write.rs`**；`device_control.rs` / `approval.rs` / `guard_watch.rs` 的 JSONL 追加原语也在当前树换过写法。⇒ **凡引用行号发生位移的条目，本文一律给出当前树的正确行号**，并按 §0.1 口径降一级为 `部分成立`。

### 0.4 复核命令

```bash
# 文档分类守卫（AUDIT 类强制项：已修 + 未修/暂缓 + 复核命令）
python3 scripts/check_doc_class.py --check

# 文档链接完整性守卫
python3 scripts/docs_links.py --check

# #14 的文件变更核对（该文件在 7654d5b6..HEAD 之间有 1 个 commit）
git log --oneline 7654d5b6..HEAD -- crates/capabilities/tools/src/process/mod.rs
git diff 7654d5b6..HEAD -- crates/capabilities/tools/src/process/mod.rs
```

---

## 一、`apeireth:crates/adapters/gateway/src/canonical_entry.rs:admin_shutdown`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| `admin_shutdown` 只解构 `State`，**没有 body extractor** ⇒ 裸 POST 是合法 CORS simple request | `crates/adapters/gateway/src/canonical_entry.rs:1505` |
| 路由注册为 `post(admin_shutdown)`，挂在整面三道门（域门 / 角色门 / 令牌门）之内 | `crates/adapters/gateway/src/canonical_entry.rs:1247` |
| CORS 层在最外层（`.layer()` 最后叠加 ⇒ 最外），只做**响应头**形状 | `crates/adapters/gateway/src/canonical_entry.rs:1294-1301` |
| 令牌门三态：`UnprotectedDefault` + 令牌表为空 ⇒ `PassUnprotected` | `crates/adapters/gateway/src/canonical_entry.rs:824-828` |
| 角色门未注入用户表时直接 `UserContext::unbound()` 放行 | `crates/adapters/gateway/src/canonical_entry.rs:1063-1066` |
| `/v1/admin/*` 在角色面激活时要求 `AdminControl`（未绑定用户 fail-closed 403） | `crates/adapters/gateway/src/canonical_entry.rs:1033-1036` |
| 域面闸只在**非默认域**才拦；未绑定客户端解析为默认域 ⇒ 本路径不被拦 | `crates/adapters/gateway/src/domain_context.rs:366` |
| 跨源白名单谓词（Tauri 源 + 回环）只决定**回哪些头**，不决定**收不收** | `crates/adapters/gateway/src/canonical_entry.rs:675-701` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| CORS 层不拒非预检请求 | 成立。`CorsLayer` 只对 `allow_origin` 谓词命中的源加 `Access-Control-Allow-Origin`；谓词不命中时**请求照样进路由**（`canonical_entry.rs:1294-1301`） |
| `POST /v1/admin/shutdown` 无 body ⇒ simple request 成立 | 成立。`admin_shutdown(State(state): State<GatewayState>)`（`:1505`）无 `Json`/`Bytes` extractor，`Content-Type` 无需特殊值 ⇒ 浏览器不发预检 |
| 前置条件「env 未设 + 令牌表空 + 无 users.json」 | 仍成立。`UnprotectedDefault` 分支（`:824-828`）在 `clients.is_empty()` 时 `PassUnprotected`；角色门 `is_active()` 为假时直接放行（`:1063-1066`）；`users.json` 缺失即未激活（`crates/adapters/gateway/src/client_tokens.rs:579`） |
| 前置条件「域面闸不拦」 | 仍成立。`context.is_non_default()` 为假时闸门不生效（`domain_context.rs:366`） |
| 证据行号 | 6/6 全部准确，无陈旧引用 |

---

## 二、`apeireth:crates/capabilities/tools/src/device/sense.rs:DeviceSenseTool::sense`

**结论：`仍成立`（严重度 low）** —— 与 #8 是**同一条**（见 §三）

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| 授权人字符串直接从**模型工具参数**读出（trim + 非空过滤） | `crates/capabilities/tools/src/device/sense.rs:287-292` |
| 授权判定 = `sensitive_read_authorized == Some(true)` **且** `authorizer.is_some()`（只看存在性） | `crates/capabilities/tools/src/device/sense.rs:293` |
| 审计行 `authorized_by` 存的就是模型给的那串 | `crates/capabilities/tools/src/device/sense.rs:313` |
| schema：授权旗的描述把「旗 + 授权人」讲成敏感读面的通行证 | `crates/capabilities/tools/src/device/sense.rs:459-462` |
| schema：`authorized_by` 承诺「recorded truthfully in the audit ledger」（值完全由模型控制） | `crates/capabilities/tools/src/device/sense.rs:463-466` |
| 工具描述把自授权配方直接写给模型 | `crates/capabilities/tools/src/device/sense.rs:485` |
| 边界 Guard 只在「旗=true 且 authorized_by 为空」时拒；**不要求旗、不查主体** | `crates/capabilities/tools/src/device/sense.rs:550-564` |
| 开关 `APEIRETH_ENABLE_DEVICE_SENSE` 只认 `"1"`，默认关 fail-closed | `crates/capabilities/tools/src/device/sense.rs:646-650` |
| 注册即授权：`tool.device_sense` **零审批** grant（即用即走） | `crates/adapters/cli/src/lib.rs:1338-1344` |
| 真实协议适配器（`local_bridge` / `cloud_bridge` / `direct`）**留插槽不接** ⇒ 摄像头读只出 mock fixture | `crates/capabilities/tools/src/device/adapter.rs:8-18` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「授权来自模型参数而非宿主审批件」 | 成立（`sense.rs:287-293`）。仓内**不存在**任何 `tool.device_sense` 的宿主审批门 |
| 「Guard 只看非空性」 | 成立（`sense.rs:550-564`）：无主体校验、无审批账查询 |
| 「描述教模型自授权」 | 成立（`sense.rs:461`、`:485`） |
| 「模型给的授权人原样进审计账」 | 成立（`sense.rs:313`） |
| 前置条件「开关开 + 有 enabled 的摄像头设备」 | 仍成立（`sense.rs:646-650`；设备表默认零设备） |
| 前置条件「零审批」 | 仍成立（`cli/src/lib.rs:1338-1344`） |
| 影响面微调 | 摄像头类**只经 mock 适配器出固定 fixture**（`adapter.rs:8-18`）⇒ 泄漏的是「谁授权」的**伪造留痕**与 fixture 数据，不是真实摄像画面。审计未夸大，但值得写明 |
| 证据行号 | 6/6 全部准确（run-1 用 `:461`、run-2 用 `:465`，两者都在同一 schema 块内） |

---

## 三、`apeireth:crates/capabilities/tools/src/device/sense.rs:DeviceSenseTool::sense`（run-2 携带的重复记录）

**结论：`仍成立`（严重度 low）—— 与 #2 是同一条缺陷，本轮按重复项归并**

### #2 与 #8 是不是同一条？—— **是**

| 判据 | #2（run-1） | #8（run-2） |
|---|---|---|
| 指纹 | `apeireth:crates/capabilities/tools/src/device/sense.rs:DeviceSenseTool::sense` | **完全相同** |
| 标题 | 「Camera-class sensitive read is authorized by the model's own tool argument」 | **逐字相同** |
| `description` / `root_cause` / `intended` / `remediation` | — | **逐字相同**（仅多一句「No host-side approval gate exists for tool.device_sense…」与「recorded verbatim as the granting principal」的补充措辞） |
| `conditions` | 3 条 | **逐字相同**（run-2 多写了一句「the default device table is empty」） |
| `evidence` | `:293` `:287` `:313` `:461` `:485` `:551` | `:293` `:287` `:313` `:465` `:485` `:551` —— **仅 `:461` 换成 `:465`**，两条都落在 `sense.rs:459-466` 这一段 schema 里 |
| `confidence` | high | high |

⇒ **判定：#8 是 run-2 对 run-1 已记录缺陷的携带（carry-over），不是第二条独立缺陷。** 二者应合并计一条；本轮对 #2 与 #8 给出同一裁定（`仍成立`），统计口径上只计 **1** 条缺陷。

### 逐条核对

所有事实核对与 §二 相同（同一文件、同一符号、同一段代码）。#8 独有的一句「No host-side approval gate exists for tool.device_sense」也已核实为真：`crates/adapters/cli/src/lib.rs:1338-1344` 的授权是**直接 grant、零审批**；全仓 `device_sense` 的 grant 只有这一处。

---

## 四、`apeireth:crates/capabilities/tools/src/git_write.rs:grade_commit`

**结论：`部分成立`（严重度 low）—— claim 与前置条件成立，但 6 条证据行号全部位移**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| 短选项值参表含 `F`（`git commit -F <path>`） | `crates/capabilities/tools/src/git_write.rs:441` | `:355`（陈旧） |
| `--message/--file/--reuse-message/--fixup/--squash` 只置标志位，**操作数路径不校验** | `crates/capabilities/tools/src/git_write.rs:470-478` | `:419`（陈旧） |
| 短选项束遇 `F` 也只置标志位后 `break`（操作数 unchecked） | `crates/capabilities/tools/src/git_write.rs:491-496` | `:438`（陈旧） |
| `git tag --message/--file` 操作数同样不校验 | `crates/capabilities/tools/src/git_write.rs:672-676` | `:617`（陈旧） |
| `git tag -F` 短形被接受为附注来源 | `crates/capabilities/tools/src/git_write.rs:694` | — |
| git 子进程用**默认（空）isolation 要求** ⇒ 无文件系统墙 | `crates/capabilities/tools/src/git_write.rs:946-949` + `crates/capabilities/tools/src/process/mod.rs:458` | `:891`（陈旧） |
| stdout 只过 `redact_secret_surfaces`（只认凭据形状串） | `crates/capabilities/tools/src/git_write.rs:963` + `crates/foundation/governance/src/input_security.rs:258-266` | `:908`（陈旧） |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「白名单允许 `commit -F/--file`、`tag -F/--file`」 | 成立（`git_write.rs:441`、`:470-478`、`:491-496`、`:672-676`、`:694`） |
| 「消息文件操作数从未做工作区包含性 / 凭据面校验」 | 成立。`grade_commit` / `grade_tag` 只判**选项形状**；`execute_plan` 把 argv 原样交给 git（`git_write.rs:941-949`），**路径不被检查** |
| 「git 子进程无文件系统隔离要求」 | 成立。`ProcessRequest::new("git")` 未调 `with_isolation`，默认 `IsolationRequirement::default()` 为空（`process/mod.rs:458`） |
| 「stdout 脱敏只覆盖凭据形状串」 | 成立。`redact_secret_surfaces` 只处理凭据 URL / `KEY=…` / `sk-…` 族（`input_security.rs:258-266`），不处理任意文件内容 |
| 前置条件「`APEIRETH_ENABLE_GIT_WRITE=1`（默认关）」 | 仍成立（`crates/adapters/cli/src/lib.rs:181-188`） |
| 前置条件「`APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1` 去掉人批」 | 仍成立（`cli/src/lib.rs:188`、`:676-680`）；`commit` 属 L1 可逆档（`git_write.rs:508`） |
| **为什么降级为「部分成立」** | **不是** claim 或前置条件变了，而是**证据行号陈旧**：当前树把 `git_write/parse.rs` 内联回 `git_write.rs`，`grade_commit` 之前多出 86 行；6 条引用全部落空（位移 +55 ~ +86） |

---

## 五、`apeireth:crates/foundation/credentials/src/keyring.rs:EncryptedFileBackend::open`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| `open()` 是 check-then-write：`exists()` 则读，否则生成 + 写；**无 `create_new`、无跨进程锁** | `crates/foundation/credentials/src/keyring.rs:791-797` |
| 模块文档自认「跨进程并发仍依赖 OS 文件语义」 | `crates/foundation/credentials/src/keyring.rs:731` |
| `write_master_key` 委托给原子持久写（临时文件 rename 覆盖目标） | `crates/foundation/credentials/src/keyring.rs:1060-1067` |
| `replace_target` = `fs::rename(tmp, dest)` ⇒ 覆盖既有目标（毁掉先前主密钥） | `crates/foundation/core/src/storage_atomic.rs:170-176` |
| 密钥不匹配 ⇒ AEAD 解密失败 | `crates/foundation/credentials/src/keyring.rs:854-866` |
| `set()` 只持进程内 `Mutex`；`load_all` 无跨进程锁 | `crates/foundation/credentials/src/keyring.rs:946-958` |
| 对照：`FileCredentialsStore` 的 load-modify-save **确有**跨进程文件锁 | `crates/foundation/credentials/src/store.rs:141-147`（`with_file_lock`） |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「两个进程各自生成不同主密钥并互相覆盖」 | 成立。`exists()` 与「生成 + rename」之间无原子创建、无锁（`keyring.rs:791-797` + `storage_atomic.rs:175`） |
| 「失败方读对方密文时 AEAD 解密失败」 | 成立（`keyring.rs:854-866`） |
| 「`intended` 称 `FileCredentialsStore` 已跨进程串行化」 | 成立（`store.rs:141-147`）⇒ 对照面确实存在差距，不是审计编造 |
| 前置条件「同 `APEIRETH_KEYRING_DIR` + 同一 encrypted-file 后端 + 首用/并发写窗口」 | 仍成立；`encrypted-file` 后端可由 `APEIRETH_KEYRING_BACKEND=encrypted-file` 选中（`crates/adapters/cli/src/keyring_bootstrap.rs:240`），**非默认**但生产可选 |
| 证据行号 | 6/6 全部准确 |

---

## 六、`apeireth:crates/foundation/protocol/src/ws_session.rs:admit_auth`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| `admit_auth` **只**比较 `auth.ws_version`；`auth.token` 从未被读 | `crates/foundation/protocol/src/ws_session.rs:126-137` |
| 版本命中即无条件 `state = Open` 并 `Deliver` | `crates/foundation/protocol/src/ws_session.rs:135-136` |
| 线协议合同要求「server 校验 token → 接受 or close 1008」 | `crates/foundation/protocol/src/ws_v1.rs:234` |
| `AuthFrame.token` 只被用作 1 MiB 长度上限；`validate()` 无生产调用点 | `crates/foundation/protocol/src/ws_v1.rs:340` |
| `WsSessionGuard` / `WsFrameDecision` 仅在本 crate re-export | `crates/foundation/protocol/src/lib.rs:131` |
| 遗留 donor 处理器**确实**校验了 Auth token | `legacy/donor/apeireth-api/src/ws_v1.rs:260-268` |
| 工作区排除 `legacy` ⇒ donor 不在编译面 | `Cargo.toml:3` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「握手门只校验 ws_version，token 从不校验」 | 成立（`ws_session.rs:126-137`：函数体里没有 `auth.token` 的任何引用） |
| 「guard 不暴露期望 token 入参 ⇒ 任何传输都无从校验」 | 成立：`WsSessionGuard::new()` 无 token 参数（`ws_session.rs:62-70`），结构上无处校验 |
| 「相对遗留 handler 是控制回退」 | 成立（`legacy/donor/apeireth-api/src/ws_v1.rs:261-268` 走 `auth.check_bearer(...)`） |
| 前置条件「仓内无传输接 `WsSessionGuard`」 | 仍成立：全仓 grep `WsSessionGuard` 只命中 `protocol` crate 自身与 re-export（`lib.rs:131`） |
| 前置条件「gateway 未注册任何 WS 端点」 | 仍成立：`crates/adapters/gateway/src/` 内 `WebSocketUpgrade` / `websocket` 零命中 |
| 证据行号 | 6/6 全部准确 |

---

## 七、`apeireth:crates/capabilities/tools/src/device/device_control.rs:DeviceControlRunner::approval_evidence`

**结论：`部分成立`（严重度 low）—— claim 与前置条件成立，但 6 条证据行号全部位移**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| 模型工具参数**原样**变成 `ApprovalEvidence` | `crates/capabilities/tools/src/device/device_control.rs:2127-2130` | `:2112`（陈旧，+15） |
| 不可逆档只靠「批准人非空」满足 | `crates/capabilities/tools/src/device/device_control.rs:1558`（`non_empty(evidence.approved_by…)`）+ `:1569-1571` | `:1542`（陈旧，+16） |
| 动钱档只额外要求 `im_card_id` 是**非空字符串** | `crates/capabilities/tools/src/device/device_control.rs:1576-1578` | `:1560`（陈旧，+16） |
| 模型选定的批准人字符串被写进 `DeviceActionRecord.approved_by` | `crates/capabilities/tools/src/device/device_control.rs:1535`（落账 `device-actions.jsonl`，见 `:1110`） | `:1519`（陈旧，+16） |
| `DeviceApprovalDecisionLedger::decision_for` 存在，但 **`device_control.rs` 不调用** | `crates/capabilities/tools/src/device/approval.rs:347` | `:333`（陈旧，+14） |
| `DeviceApprovalPushLedger::delivered_record_for` 存在，但 **`device_control.rs` 不调用** | `crates/capabilities/tools/src/device/approval.rs:181` | `:174`（陈旧，+7） |
| （对照）IM 路径**确实**查决定账与推送账 | `crates/adapters/gateway/src/im_device.rs:222`（`decision_for`）、`:238`（`delivered_record_for`） | — |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「execute 拍用模型参数构造审批证据」 | 成立（`device_control.rs:2127-2130`） |
| 「不可逆档/动钱档只验字符串存在性」 | 成立（`:1558`、`:1576`） |
| 「设备动作账把模型字符串记成『谁批准』」 | 成立（`:1535` + `:1110`） |
| 「从不查 `decision_for` / `delivered_record_for`」 | 成立：**全仓 grep 这两个方法名，生产调用点只有 `crates/adapters/gateway/src/im_device.rs:222`、`:238`**，均在网关 IM 面，工具执行拍完全够不着 |
| 前置条件「本 ref 下不可逆/动钱档不可达」 | 仍成立。`actions_for_kind` 只给 `appliance` 挂可逆动作，其余类别零执行动作（`device_control.rs:462-467`）⇒ 两档只能靠显式逐设备登记（进程内 mock/测试面）挂接 |
| 前置条件「`APEIRETH_ENABLE_DEVICE_AUTO_PASS_REVERSIBLE=1` 时可逆档自动放行（默认关）」 | 仍成立（子开关读取 `device_control.rs:137`；默认 `auto_pass_reversible: false` 在 `:158`） |
| **为什么降级为「部分成立」** | 证据行号陈旧：当前树把 `approval.rs` / `device_control.rs` 的 JSONL 追加原语从 `crate::proposal::append_jsonl_row` 改为手写 `OpenOptions::append` + `write_all` + `sync_all`，净增行数不同（+7 / +14 / +15 / +16） |

---

## 八、`apeireth:crates/capabilities/tools/src/device/device_control.rs:DeviceControlRunner::execute`

**结论：`部分成立`（严重度 low）—— claim 与前置条件成立，但 4 条证据行号位移**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| **真实执行**先发生（`adapter.apply`），此刻尚无任何账行 | `crates/capabilities/tools/src/device/device_control.rs:1522` | `:1506`（陈旧，+16） |
| 唯一审计记录在动作**跑完之后**才 append，错误经 `?` 直接上抛 | `crates/capabilities/tools/src/device/device_control.rs:1538` | `:1522`（陈旧，+16） |
| mock 适配器 `apply` 直接改进程内状态 map（唯一世界变化） | `crates/capabilities/tools/src/device/device_control.rs:749` | `:749`（准确） |
| 账失败以 `DeviceControlError::Ledger` 上抛 | `crates/capabilities/tools/src/device/device_control.rs:263` | `:263`（准确） |
| 上抛后由 `refuse` 包成**永久错误帧**（模型看到失败、设备状态已变） | `crates/capabilities/tools/src/device/device_control.rs:1973-1974` | — |
| 对照：兄弟账 `guard_watch` 写不进 ⇒ **整帧拒绝**（fail-closed） | `crates/capabilities/tools/src/device/guard_watch.rs:904` | `:897`（陈旧，+7） |
| 对照：兄弟账 `device_sense` 审计写不进 ⇒ **不放行**（fail-closed） | `crates/capabilities/tools/src/device/sense.rs:318-323` | `:319`（准确） |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「先执行、后写账 ⇒ 账写失败时动作已生效但无留痕」 | 成立（`:1522` 早于 `:1538`） |
| 「返回错误帧但设备状态已变」 | 成立（`:1538` 的 `?` → `:1973-1974` `permanent_error`） |
| 「兄弟账 fail-closed，设备动作账不是」 | 成立（`guard_watch.rs:904` / `sense.rs:318-323` 对照 `device_control.rs:1538`） |
| 「无执行前意图行 / 无 post-execution 失败处理」 | 成立：`execute` 拍内 `:1501-1538` 只有一次 `ledger.append` |
| 前置条件「设备控制面需双闸齐开」 | 仍成立（双闸判定 `device_control.rs:124-125`：感知闸 + 控制闸同时为 `"1"`；子开关常量 `:109`） |
| **为什么降级为「部分成立」** | 证据行号陈旧（同 §七 的追加原语改写导致净位移） |

---

## 九、`apeireth:crates/capabilities/tools/src/device/weather.rs:WeatherTool::now`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| 入口：`place` 原样取出，**无长度 / 字符集 / 控制符校验** | `crates/capabilities/tools/src/device/weather.rs:533` |
| mock 源（**唯一接线的源**）把 `place` 原样抄进报告 | `crates/capabilities/tools/src/device/weather.rs:153` |
| live 档是留插槽，解析即报「未接线」 | `crates/capabilities/tools/src/device/weather.rs:122`、`:202` |
| `ReminderSignal.evidence` 拼进 `report.place`（即后续的触发语义） | `crates/capabilities/tools/src/device/weather.rs:327-330` |
| 字段文档自称「有界摘要」，但 `place` 无界 | `crates/capabilities/tools/src/device/weather.rs:278` |
| 工具 schema 把 `place` 声明为自由文本，无 `maxLength` / `pattern` | `crates/capabilities/tools/src/device/weather.rs:612` |
| 信号落 `weather-signals.jsonl`（append-only） | `crates/capabilities/tools/src/device/weather.rs:54`、`:383` |
| 信号 → `ReminderCandidate`（`trigger` 取闭合词表，`evidence` 带回原始 place） | `crates/engine/runtime-assembly/src/canonical/care_reminder.rs:117-128` |
| `ReminderCandidate` → `CareCandidate.trigger`（**evidence 被格式化进 trigger**） | `crates/engine/organ/src/care/reminder.rs:130-144` |
| `CareCandidate.trigger` → 关怀回合输入串 | `crates/engine/organ/src/care/engine.rs:487-508`（`:497` 插 `trigger`） |
| 平台既有的自由文本纪律（非空 / 有界 / 无控制符）—— **只作用于 `material` 与告警语**，不作用于 `trigger` | `crates/engine/organ/src/care/rules.rs:638-652`；`crates/capabilities/tools/src/device/guard_watch.rs:407-420` |
| `material` 槽确实走了 `validate_care_material` | `crates/engine/organ/src/care/tool.rs:178` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「`place` 无校验直接进报告」 | 成立（`weather.rs:533`） |
| 「进 `ReminderSignal.evidence`」 | 成立（`weather.rs:327-330`） |
| 「落 `weather-signals.jsonl`，后续被关怀引擎重放」 | 成立（`weather.rs:54`、`:383` → `care_reminder.rs:130-144`） |
| 「重放进 `CareCandidate.trigger`」 | 成立 —— 路径是 `evidence` → `ReminderCandidate.evidence`（`care_reminder.rs:124`）→ 被 `format!` 进 `CareCandidate.trigger`（`care/reminder.rs:133-139`）。审计写作「trigger」指的是**最终承载它的那个字段**，中间经了一次 `evidence` 中转 |
| 「再进关怀回合输入，由 runtime 以 user 角色追加」 | 成立（`care/engine.rs:487-508`；回合输入经 `crates/engine/runtime/src/canonical/heartbeat.rs:418-425` 派发为标准回合） |
| 「该槽缺了平台既有的自由文本纪律」 | 成立：`validate_care_material` 只校验 `material`（`care/tool.rs:178`），`validate_guard_message` 只校验告警语（`guard_watch.rs:407`）；`trigger` 无对应校验 |
| 前置条件「模型可被外部内容引导给出带控制符的 place」 | 仍成立（schema 无约束，`weather.rs:612`） |
| 证据行号 | 6/6 全部准确（`care/rules.rs:638` 亦准确） |

---

## 十、`apeireth:crates/capabilities/tools/src/exec_pipeline/mod.rs:PipelineFailure::emit`

**结论：`部分成立`（严重度 low）—— claim 与前置条件成立，1 条证据行号位移**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| 截止分支赢得 `select!`，在途 invoke future 被**丢弃**（无 `CancellationToken` / `AbortHandle` / `Drop` 守卫） | `crates/capabilities/tools/src/exec_pipeline/around.rs:149-155` | `:151`（准确） |
| 缺失结果 ⇒ `PipelineFailure::Timeout`，**只有 code，无「已开始/未开始」判别位** | `crates/capabilities/tools/src/exec_pipeline/around.rs:156-160` | `:157`（准确） |
| `Timeout` 与 `RetriesExhausted` **一律**映射为 `retryable_error` | `crates/capabilities/tools/src/exec_pipeline/mod.rs:271-273` | `:272`（准确） |
| 平台自己的正确区分：`NotStarted → retryable` / `WindDownExpired → permanent` | `crates/engine/runtime/src/canonical/execute.rs:415-427` | `:423`（准确） |
| 效果可在超时**之后**落地：git 计划跑在 `spawn_blocking` 里，tokio 无法中止已启动任务 | `crates/capabilities/tools/src/git_write.rs:1110` | `:1055`（**陈旧**，+55） |
| MCP 汇点：JSON-RPC `tools/call` 已投递给第三方服务器，丢弃 future 只是丢响应 | `crates/capabilities/tools/src/mcp_bridge/tool.rs:107-113` | `:111`（准确） |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「超时把已开始的调用判成可重试」 | 成立（`around.rs:149-160` + `exec_pipeline/mod.rs:271-273`） |
| 「一个失败族同时覆盖『未执行』与『已执行但未确认』」 | 成立：`PipelineFailure::Timeout` 无判别位（`around.rs:157-159`），与 `execute.rs:415-427` 的正确区分形成对照 |
| 「效果可能在超时后落地」 | 成立（`git_write.rs:1110`；`mcp_bridge/tool.rs:107-113`） |
| 前置条件「工具超过截止时刻且副作用已交付」 | 仍成立（`around.rs:149-155` 的 `select!` 结构未变） |
| **为什么降级为「部分成立」** | 仅 `git_write.rs:1055` 一条引用陈旧（当前 `:1110`），其余 5 条准确 |

---

## 十一、`apeireth:crates/capabilities/tools/src/exec_pipeline/mod.rs:ToolExecutionPipeline::run_in_session`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| `run_in_session` 的 stage-1 走 `self.pre.evaluate(...)`（瀑布钩子） | `crates/capabilities/tools/src/exec_pipeline/mod.rs:411-427` |
| preapproved 分支入口；**该分支不评估 `self.governance`** | `crates/engine/runtime/src/canonical/execute.rs:3105` |
| preapproved 分支唯一的复核 = 冻结审批的 身份/指纹/动作 绑定（**无策略检查**） | `crates/engine/runtime/src/canonical/execute.rs:3124-3125` |
| preapproved 调用**直接 `Ready` 返回**（无任何治理评估） | `crates/engine/runtime/src/canonical/execute.rs:3175-3181` |
| 主回合路径里唯一的 capability-dispatch 治理评估（**在 preapproved 之外**） | `crates/engine/runtime/src/canonical/execute.rs:3189-3196` |
| `ApprovalDecision::Approve` 分支：认领审批 → 持久化 → 恢复回合，**不回查共享策略** | `crates/engine/runtime/src/canonical/execute.rs:1360-1394` |
| stage-1 钩子拿的是**工具自建策略**：`authorized_file_write_policy()` 新建一份 `PermissionPolicy` 并 `grant(ExecuteTool(apply_patch))`，无 `require_approval` 标记 | `crates/capabilities/tools/src/apply_patch_tool.rs:492-501`（`:497` 为 grant 那一行） |
| 该策略被挂进流水线 | `crates/capabilities/tools/src/apply_patch_tool.rs:514` |
| 钩子判定 = 「策略里有没有显式授权」，有就 `Allow` | `crates/capabilities/tools/src/apply_patch_tool.rs:380-397` |
| 组装根实际注入的就是那份**自建**策略 | `crates/engine/runtime-assembly/src/canonical/tool_modules.rs:588` |
| 治理层确有 `revoke` 能力（热撤存在） | `crates/foundation/governance/src/permission.rs:76`、`:122` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「stage-1 钩子用的是永远放行的工具本地策略」 | 成立。`authorized_file_write_policy()` 每次新建策略并只 grant 一条（`apply_patch_tool.rs:492-501`），`ApplyPatchRiskMappingHook` 只查这份策略（`:380-397`），组装根注入的也正是它（`tool_modules.rs:588`） |
| 「审批恢复分支跳过治理」 | 成立（`execute.rs:1360-1394`：认领 → 落盘 → 恢复，无共享策略复核） |
| 「preapproved 分支不咨询 `self.governance` 直接 Ready」 | 成立（`execute.rs:3105-3182`；治理评估在 `:3189` 之后才发生，属另一分支） |
| 「热撤存在但对已批准/已预批的调用不生效」 | 成立 —— `PermissionPolicy::revoke` 存在（`permission.rs:76`、`:122`），但 preapproved 分支（`execute.rs:3175`）与 approve 恢复分支（`execute.rs:1360`）都不读它 |
| 前置条件「写面开启（`APEIRETH_ENABLE_FILE_WRITE=1`）且调用走审批预批路径」 | 仍成立（`apply_patch` 由写总闸注册；`APEIRETH_ENABLE_FILE_WRITE` 见 `crates/capabilities/tools/src/sheet_ops.rs:68` 的总闸语义） |
| 证据行号 | 6/6 全部准确（`execute.rs:3105`、`:3124`、`:3175`、`:3190`、`:1360`、`apply_patch_tool.rs:497`） |

---

## 十二、`apeireth:crates/capabilities/tools/src/process/appcontainer.rs:grant_directory`

**结论：`部分成立`（严重度 low）—— claim 与前置条件成立，1 条证据行号位移**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| `grant_directory` 用 `FILE_ALL_ACCESS` + 容器/对象继承 `0x3` 授权 | `crates/capabilities/tools/src/process/appcontainer.rs:139-141` | `:140`（准确） |
| 该全权掩码被抄进 `EXPLICIT_ACCESS_W.grfAccessPermissions` | `crates/capabilities/tools/src/process/appcontainer.rs:384-390` | `:385`（准确） |
| 合并后的 DACL 经 `SetNamedSecurityInfoW` **落盘**；全仓唯一写点，**无回收路径** | `crates/capabilities/tools/src/process/appcontainer.rs:392-408`（`:400`） | `:400`（准确） |
| 授权施加在「请求 cwd」上，且在 `CreateProcessW` 之前 | `crates/capabilities/tools/src/process/windows.rs:1154-1159` | `:1158`（准确） |
| 模型撰写的脚本逐字进命令行尾 `cmd /D /S /C "<script>"` | `crates/capabilities/tools/src/shell.rs:485` | `:485`（准确） |
| 预调用高危命令黑名单**不含** `icacls` / `chmod` / `chown` / `takeown` | `crates/capabilities/tools/src/guardrail.rs:264-289` | `:301`（**陈旧**：当前 `:301` 是 netsh 注释） |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「`FILE_ALL_ACCESS` ⇒ 含 `WRITE_DAC`(0x40000) / `WRITE_OWNER`(0x80000) / `DELETE`」 | 成立。掩码常量直接来自 `FILE_ALL_ACCESS`（`appcontainer.rs:154` 导入，`:140` 传入，`:385` 落入 ACE） |
| 「ACE 永不回收」 | 成立。全仓 `SetNamedSecurityInfoW` 仅此一处写点（`appcontainer.rs:400`），无任何删除/回收 ACE 的代码 |
| 「模型撰写的子进程可改工作区 DACL 并持久化」 | 成立 —— 命令行尾逐字传脚本（`shell.rs:485`），黑名单不拦 ACL 操作（`guardrail.rs:264-289`） |
| 「设计只授予工作区读写，实现却用了一个确定性安装级全权档」 | 成立。设计文档 `docs/01-architecture/shell-sandbox-lite-design-2026-10-06.md`（模块头 `appcontainer.rs:3` 引 §2.1）；实现是固定 profile 名（`:49`）+ 全权掩码（`:140`） |
| 前置条件「Windows 主机 + 可用 AppContainer profile」 | 仍成立。沙箱默认开（`crates/adapters/cli/src/lib.rs:2986-2992`：`APEIRETH_SHELL_SANDBOX` 只认 `"0"` 为关） |
| **为什么降级为「部分成立」** | `guardrail.rs:301` 一条引用陈旧（当前黑名单数组在 `:264-289`）；其余 5 条准确 |

---

## 十三、`apeireth:crates/capabilities/tools/src/process/macos.rs:spawn_and_supervise`

**结论：`仍成立`（严重度 low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| macOS `pre_exec` **只**装 `setpgid` + rlimits；无父死信号、无 `getppid` 复查 | `crates/capabilities/tools/src/process/macos.rs:89-100`（`:90` `pre_exec`、`:93` `setpgid`、`:97` `set_rlimits`） |
| Linux 兄弟后端**有** `PR_SET_PDEATHSIG` | `crates/capabilities/tools/src/process/linux.rs:106-108` |
| Linux 另用 `getppid()` 复查关掉 fork/prctl 竞态 | `crates/capabilities/tools/src/process/linux.rs:109-114` |
| Windows 兄弟后端用 job object kill-on-close 达成等价控制 | `crates/capabilities/tools/src/process/windows.rs:104-108` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「macOS 后端无父死 kill，子进程可活过执行器」 | 成立（`macos.rs:89-100` 全函数无 `PR_SET_PDEATHSIG` / `kqueue` / `procctl`） |
| 「Linux 有对应控制，构成对照」 | 成立（`linux.rs:106-114`） |
| 「Windows 有 job-object kill-on-close」 | 成立（`windows.rs:104-108`） |
| 前置条件「部署在 macOS」 | 仍成立；Linux / Windows 后端保留该控制 |
| 前置条件「执行器在子进程运行期间死亡」 | 仍成立（超时/上限杀的是进程组，执行器自身崩溃则无守护） |
| 证据行号 | 6/6 全部准确 |

---

## 十四、`apeireth:crates/capabilities/tools/src/process/mod.rs:capabilities_for_execution`

**结论：`仍成立`（严重度 low）—— **本次变更没有触及该 finding**

### 变更核对（必做项）

```bash
git log --oneline 7654d5b6..HEAD -- crates/capabilities/tools/src/process/mod.rs
# → 718ccb7a  feat(guard): D-1 裁定落地 —— dead_code 全局 allow → warn + 豁免棘轮 + CI 门禁

git diff 7654d5b6..HEAD -- crates/capabilities/tools/src/process/mod.rs
# → 仅两处 #[allow(dead_code)] 标注：
#     + #[allow(dead_code)]  pub(crate) fn apply_request_to_command(...)
#     + #[allow(dead_code)]  fn wait(&mut self) -> Result<ChildStatus, ProcessError>;
```

**结论：该变更只加了 `#[allow(dead_code)]` 属性（一处在 `capabilities_for_execution` 之后、一处在 trait 里），完全未触碰 `capabilities_for_execution` 的逻辑，也未新增 `environment_isolation` 分支 ⇒ 本次变更没有解决本 finding。**

### 当前代码证据（file:line）

| 事实 | 位置 | 审计原引用 |
|---|---|---|
| `capabilities_for_execution` 收窄「可选限额」与「降权」，**没有** `environment_isolation` 分支 | `crates/capabilities/tools/src/process/mod.rs:860-891` | `:860`（准确） |
| 记录合同：「本次运行**实际生效**的隔离属性」 | `crates/capabilities/tools/src/process/mod.rs:589` | `:589`（准确） |
| `EnvironmentSpec::Inherit` 分支**不做** `env_clear` ⇒ 子进程继承父环境（含 provider 密钥） | `crates/capabilities/tools/src/process/mod.rs:905-906` | `:904`（轻微偏移，落在 `#[allow(dead_code)]` 上） |
| `Inherit` 是每个 `ProcessRequest` 的默认环境策略 | `crates/capabilities/tools/src/process/mod.rs:456` | `:456`（准确） |
| Linux 也无条件宣称 `EnvironmentIsolation=Enforced` | `crates/capabilities/tools/src/process/linux.rs:39-42` | `:39`（准确） |
| macOS 同样无条件宣称 | `crates/capabilities/tools/src/process/macos.rs:38-41` | `:38`（准确） |
| Windows 同样无条件宣称 | `crates/capabilities/tools/src/process/windows.rs:201-204` | — |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「执行记录宣称 `EnvironmentIsolation=Enforced`，而默认请求其实是 `Inherit`」 | 成立。三个平台后端都无条件 `Enforced`（`linux.rs:39-42`、`macos.rs:38-41`、`windows.rs:201-204`），而 `capabilities_for_execution`（`process/mod.rs:860-891`）对 `environment_isolation` **只字不提** |
| 「`Inherit` ⇒ 子进程拿到父环境（含 provider 密钥）」 | 成立（`process/mod.rs:905-906` 无 `env_clear`；默认 `Inherit` 在 `:456`） |
| 「记录合同要求报『实际生效』」 | 成立（`process/mod.rs:589`） |
| 前置条件「模型侧工具构造的 `ProcessRequest` 不设环境」 | 仍成立 —— 审计点名的 `external.rs` / `repo.rs` / `git_write.rs` / `worktree_apply.rs` 均未调 `with_clear_env` / `with_explicit_env`；本仓可核的 `git_write.rs:946-949` 即为默认 `Inherit` |
| **本次变更是否解决** | **否**（见上） |

---

## 十五、`apeireth:crates/capabilities/tools/src/process/windows.rs:capabilities`

**结论：`仍成立`（严重度 informational）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| 模块文档宣称「从不接受 shell 命令串、从不调用 `cmd /c` 或 `sh -c`」（**未限定 Unix**，Windows 后端是子模块 ⇒ 陈旧） | `crates/capabilities/tools/src/process/mod.rs:17-18` |
| `StructuredSpawn` 的定义：可执行文件与 argv 以**结构化数组**传递，**永不**是 shell 命令串 | `crates/capabilities/tools/src/process/mod.rs:87-89` |
| `capabilities_for_execution` 不收窄 `structured_spawn`，全仓无 `raw_arg` 分支 | `crates/capabilities/tools/src/process/mod.rs:860-891` |
| Windows 后端**无条件**宣称 `StructuredSpawn=Enforced` | `crates/capabilities/tools/src/process/windows.rs:191-196`（`:193`） |
| AppContainer 路径把 `raw_arg` 尾**逐字**追加到命令行；plain-raw / std 路径同 | `crates/capabilities/tools/src/process/windows.rs:1163-1169` |
| 既有 fail-closed 先例：受限 token 路径**拒**带 `raw_arg` 尾的请求 | `crates/capabilities/tools/src/process/windows.rs:1025-1031` |
| Windows shell 路径正是逐字尾 `cmd /D /S /C "<script>"` | `crates/capabilities/tools/src/shell.rs:485` |
| shell 工具自己**要求** `StructuredSpawn=Enforced` ⇒ 需求闸为一个自身违反的属性背书 | `crates/capabilities/tools/src/shell.rs:385-390` |

### 逐条核对

| 审计 claim | 核对 |
|---|---|
| 「Windows 路径确实执行 `cmd /D /S /C "<script>"`」 | 成立（`shell.rs:485` → `windows.rs:1164-1169` 逐字追加） |
| 「模块文档的『从不 cmd /c』对 Windows 已陈旧」 | 成立（`process/mod.rs:17-18` 是 crate 级文档，Windows 后端在同 crate 内） |
| 「`StructuredSpawn` 的语义排除 shell 命令串」 | 成立（`process/mod.rs:87-89`） |
| 「shell 工具要求该属性为 Enforced」 | 成立（`shell.rs:385-390`） |
| 前置条件「Windows only」 | 仍成立（Unix 路径走 `with_args` 保持结构化，`shell.rs:487-489`） |
| 前置条件「`tool.shell` opt-in（`APEIRETH_ENABLE_SHELL=1`）」 | 仍成立 |
| 前置条件「`APEIRETH_SHELL_SANDBOX=0` 走 plain-raw，开则走 AppContainer，两条都追加尾」 | 仍成立（`windows.rs:1163-1169` 为共享代码路径；受限 token 路径不在可选面内） |
| 证据行号 | 6/6 全部准确 |

---

## 十六、`apeireth:crates/capabilities/tools/src/process/appcontainer.rs:SANDBOX_APPCONTAINER_NAME`（待验）

**结论：`仍成立`（严重度 low）—— 建议由「待验」提升为 confirmed（low）**

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| 容器名是**硬编码常量** `Apeireth.Shell.Sandbox`（无用户维度、无每次运行维度） | `crates/capabilities/tools/src/process/appcontainer.rs:49` |
| SID 由**名字**派生（`DeriveAppContainerSidFromAppContainerName`）⇒ 对所有账号、所有主机进程**完全相同** | `crates/capabilities/tools/src/process/appcontainer.rs:105-115`（`:108` 为派生调用） |
| profile 条目生命周期 = **安装级**，不随进程删除 | `crates/capabilities/tools/src/process/appcontainer.rs:77`（模块/方法注释） |
| 该**名字派生**的 SID 被授工作区 `FILE_ALL_ACCESS` | `crates/capabilities/tools/src/process/appcontainer.rs:139-141`（`:140`） |
| ACE 落盘且永不回收 | `crates/capabilities/tools/src/process/appcontainer.rs:392-408`（`:400`） |
| 授权对象就是这个包 SID（`TRUSTEE_IS_SID` + `self.sid`） | `crates/capabilities/tools/src/process/appcontainer.rs:388-390` |
| 沙箱默认开（`APEIRETH_SHELL_SANDBOX` 只认 `"0"` 为关） | `crates/adapters/cli/src/lib.rs:2986-2992` |

### 逐条核对（候选是否成立）

| 候选 claim | 核对 |
|---|---|
| 「沙箱身份是常量 ⇒ SID 对每个账号与每个主机进程都一样」 | **成立**。名字固定（`:49`），SID 由名字确定性派生（`:108`），profile 又是安装级常驻（`:77`）⇒ 这是一个**机器级共享身份** |
| 「工作区上的持久 `FILE_ALL_ACCESS` ACE 因此可被任何能进该容器的本地主体使用」 | **成立**。ACE 主体是该包 SID（`:388-390`），授权 `FILE_ALL_ACCESS`（`:140`），落盘不回收（`:400`）；而**进入已注册 AppContainer 不需要特权**（SID 由公开名字推导），⇒ 同机任一本地主体都可创建该容器中的进程并继承工作区全权 |
| 「这使 #12 的永久授权从『本沙箱可用』升级为『机器上任何本地主体可用』」 | **成立**，且是 #12 影响面的**放大器**而非重复计数 |
| 需要什么前提 | Windows 主机 + 多用户（存在**另一个**本地主体）+ shell 沙箱开启（默认开）+ 该 profile 已被注册过（首次运行后常驻） |
| 影响量级 | 跨本地主体的**工作区读/写/篡改**（含 `WRITE_DAC` ⇒ 还能继续扩权）；不含远程攻击面，不含凭据外泄的直接路径 |
| 证据行号 | 3/3 全部准确（`:49`、`:108`、`:140`） |

### 提升建议：**提升为 confirmed，严重度 low**

理由：claim 全部可在代码内核实（固定名 → 确定性 SID → 持久全权 ACE），不是推测；但影响需「多用户 Windows 主机 + 沙箱开启」的前置，属**本地、需前置条件**的横向越权，不构成远程或默认开启的高危 ⇒ 定 **low**，与 #12（永久全权 ACE）同级但不同源（#12 = 权限过宽且不回收；#16 = 身份可被机器内他人复用）。
建议修复方向：profile 名掺入用户 SID 或每次运行的随机后缀（并回收 ACE），或把 grant 收到数据面掩码。

---

## 十七、`apeireth:crates/capabilities/tools/src/process/windows.rs:spawn_appcontainered_child`（待验）

**结论：`部分成立`（严重度 informational）—— 代码事实成立，但影响链无法在仓内闭环 ⇒ **不建议提升为 confirmed***

### 当前代码证据（file:line）

| 事实 | 位置 |
|---|---|
| 执行前**确实**把 frozen cwd 重新 canonicalize 并做包含性复核 | `crates/capabilities/tools/src/shell.rs:644-677`（`:645` canonicalize、`:668` 复核并拒绝） |
| 复核通过后**丢弃**已验证的 canonical 路径，改用**原始** frozen 路径构造请求 | `crates/capabilities/tools/src/shell.rs:492`（`.with_working_directory(PathBuf::from(&frozen.cwd))`） |
| Windows 授权侧只剥 `\\?\` 前缀，路径本体就是那个原始 frozen 路径 | `crates/capabilities/tools/src/process/windows.rs:1154-1158` |
| ACL 写入是**按路径**的 `SetNamedSecurityInfoW` | `crates/capabilities/tools/src/process/appcontainer.rs:400` |
| `cwd` 允许是工作区下的**相对子目录**（更容易被替换） | `crates/capabilities/tools/src/shell.rs:242-263`（`resolve_cwd_for`） |

### 逐条核对（候选是否成立）

| 候选 claim | 核对 |
|---|---|
| 「已验证的 canonical 路径被丢弃，授权走原始路径」 | **成立**（`shell.rs:492` + `windows.rs:1154-1158`）—— 这是一个**真实的实现缺口**（授权路径与校验路径不是同一条） |
| 「reparse point 会被路径型 ACL API 穿越」 | **无法核实**。这是 Win32 语义问题，仓内无对应实现或断言可证；审计原文自己就写了「unverified on Windows」 |
| 「替换必须发生在『复核通过』与『授权写入』之间的窗口内」 | **无法核实**。窗口客观存在（`shell.rs:677` → `windows.rs:1158`，其间还含 profile 获取与探针，`appcontainer.rs:78-127`、`:161-173`），但**仓内没有可并发完成该替换的动作**；最现实的替换者是同一服务账号 ⇒ 其本身已拥有这些路径的权限，**不构成提权** |
| 「⇒ 授权可被移到工作区之外」 | **不成立于可证范围**：跨主体提权需要另一个本地主体能替换工作区内目录，而该主体本来就没有工作区写权限 |
| 证据行号 | 4/4 全部准确（`shell.rs:645`、`:492`、`windows.rs:1154`、`appcontainer.rs:400`） |

### 提升建议：**不提升为 confirmed**（拒绝按安全缺陷计数），最多记为 **informational 的加固项**

理由：代码事实（canonical 路径被丢弃）成立且值得修 —— 但候选的**危害结论**依赖两个仓外/仓内均无法证实的前提（Win32 路径型 ACL API 是否穿越 reparse point；谁能在一个调用窗口内替换工作区目录）。把「无法核实的影响」计成 confirmed 会污染审计基线。建议动作：把授权改用已验证的 `cwd_canonical`（一行改动），按加固项登记，不进缺陷台账。

---

## 十八、汇总

| 指纹 | 严重度 | 裁定 | 修复状态 | 一句话依据 |
|---|---|---|---|---|
| `apeireth:crates/adapters/gateway/src/canonical_entry.rs:admin_shutdown` | low | **仍成立** | 未修 | 裸 POST 无 body ⇒ 跨源 simple request 直达停机口；CORS 只管响应头（`canonical_entry.rs:1505`、`:1247`、`:1294-1301`） |
| `apeireth:crates/capabilities/tools/src/device/sense.rs:DeviceSenseTool::sense` | low | **仍成立** | 未修 | 授权人来自模型参数且只验非空（`sense.rs:287-293`、`:313`），零审批放行（`cli/src/lib.rs:1338-1344`） |
| `apeireth:crates/capabilities/tools/src/device/sense.rs:DeviceSenseTool::sense`（run-2 携带） | low | **仍成立**（= #2，重复项） | 已修（重复归并） | 指纹/标题/描述/根因/条件/修复建议逐字相同 ⇒ **是同一条**，只 #2 的 `:461` 换成 `:465` |
| `apeireth:crates/capabilities/tools/src/git_write.rs:grade_commit` | low | **部分成立** | 未修 | `-F/--file` 操作数不校验（`git_write.rs:441`、`:470-478`、`:491-496`、`:672-676`）；**6 条证据行号全部陈旧**（当前树内联了 `parse.rs`，位移 +55~+86） |
| `apeireth:crates/foundation/credentials/src/keyring.rs:EncryptedFileBackend::open` | low | **仍成立** | 未修 | check-then-write 无原子创建/无跨进程锁（`keyring.rs:791-797` + `storage_atomic.rs:175`），对照 `FileCredentialsStore` 有文件锁（`store.rs:141-147`） |
| `apeireth:crates/foundation/protocol/src/ws_session.rs:admit_auth` | low | **仍成立** | 未修 | 只比 `ws_version`，`auth.token` 从未读（`ws_session.rs:126-137`）；仓内无传输接该 guard，legacy 被工作区排除 |
| `apeireth:crates/capabilities/tools/src/device/device_control.rs:DeviceControlRunner::approval_evidence` | low | **部分成立** | 未修 | 批准人取自模型参数且只验非空（`device_control.rs:1558`、`:1576`、`:1535`、`:2127-2130`），从不查决定账/推送账；**6 条证据行号位移** |
| `apeireth:crates/capabilities/tools/src/device/device_control.rs:DeviceControlRunner::execute` | low | **部分成立** | 未修 | 先 `apply`（`:1522`）后 append（`:1538`），账失败仍报永久错误帧（`:1973-1974`），与兄弟账 fail-closed 不一致；**4 条证据行号位移** |
| `apeireth:crates/capabilities/tools/src/device/weather.rs:WeatherTool::now` | low | **仍成立** | 未修 | `place` 无校验（`weather.rs:533`）→ `evidence`（`:327-330`）→ `CareCandidate.trigger`（`care/reminder.rs:133-139`）→ 关怀回合输入（`care/engine.rs:497`） |
| `apeireth:crates/capabilities/tools/src/exec_pipeline/mod.rs:PipelineFailure::emit` | low | **部分成立** | 未修 | 超时无「已开始」判别位（`around.rs:156-160`）且一律 `retryable`（`exec_pipeline/mod.rs:271-273`）；`git_write.rs:1055` 一条引用陈旧（当前 `:1110`） |
| `apeireth:crates/capabilities/tools/src/exec_pipeline/mod.rs:ToolExecutionPipeline::run_in_session` | low | **仍成立** | 未修 | stage-1 用自建永远放行策略（`apply_patch_tool.rs:492-501`、`:514`），preapproved 直接 Ready 不评治理（`execute.rs:3105`、`:3175`），approve 恢复不回查共享策略（`execute.rs:1360`） |
| `apeireth:crates/capabilities/tools/src/process/appcontainer.rs:grant_directory` | low | **部分成立** | 未修 | `FILE_ALL_ACCESS` 落盘不回收（`appcontainer.rs:140`、`:385`、`:400`）；`guardrail.rs:301` 一条引用陈旧（黑名单在 `:264-289`） |
| `apeireth:crates/capabilities/tools/src/process/macos.rs:spawn_and_supervise` | low | **仍成立** | 未修 | macOS `pre_exec` 只有 `setpgid` + rlimits（`macos.rs:89-100`），Linux 有 `PR_SET_PDEATHSIG`（`linux.rs:106`）而 macOS 无 |
| `apeireth:crates/capabilities/tools/src/process/mod.rs:capabilities_for_execution` | low | **仍成立** | 未修 | 无 `environment_isolation` 分支（`process/mod.rs:860-891`），三平台均无条件宣称 `Enforced`（`linux.rs:39`、`macos.rs:38`、`windows.rs:201`）；**本次 commit `718ccb7a` 只加 `#[allow(dead_code)]`，未触及该 finding** |
| `apeireth:crates/capabilities/tools/src/process/windows.rs:capabilities` | informational | **仍成立** | 未修 | Windows 宣称 `StructuredSpawn=Enforced`（`windows.rs:193`），但 shell 走 `cmd /D /S /C`（`shell.rs:485` → `windows.rs:1164-1169`），而 shell 自己要求该属性 Enforced（`shell.rs:388`） |
| `apeireth:crates/capabilities/tools/src/process/appcontainer.rs:SANDBOX_APPCONTAINER_NAME` | low | **仍成立**（待验→**提升 confirmed / low**） | 未修 | 固定名 ⇒ 确定性 SID（`appcontainer.rs:49`、`:108`）⇒ 机器级共享身份持有工作区永久全权 ACE（`:140`、`:400`） |
| `apeireth:crates/capabilities/tools/src/process/windows.rs:spawn_appcontainered_child` | informational | **部分成立**（待验→**不提升**） | 未修 | canonical 路径被丢弃属实（`shell.rs:492` vs `:645`；`windows.rs:1154`），但「reparse 穿越 + 可并发替换」两个前提仓内均无法证实 ⇒ 只作加固项 |

### 计数

| 口径 | 数量 | 条目 |
|---|---|---|
| `仍成立` | **11** | #1、#2、#4、#5、#8、#9、#11、#13、#14、#15、#16 |
| `部分成立` | **6** | #3、#6、#7、#10、#12、#17 |
| `已修`（代码缺陷） | **0** | — |
| `不成立` | **0** | — |
| `无法核实`（整条） | **0** | #17 的**影响链**属「无法核实」，但代码事实成立，故整条裁定落在 `部分成立` |
| 合计 | **17** | 与输入条数一致 |
| 去重后的独立缺陷 | **16** | #2 与 #8 合并计 1 条 |
| 其中「证据行号陈旧导致降级」 | **5** | #3、#6、#7、#10、#12 |
| 其中「影响链无法在仓内闭环」 | **1** | #17 |
