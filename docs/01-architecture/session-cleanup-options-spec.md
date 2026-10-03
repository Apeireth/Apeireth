# 会话清理双档选项 —— 清理语义修复预案（session-cleanup-options-spec）

```text
[Document-Meta]
性质:    修复预案 + 功能拆分设计。**未实施**——§1 裁决位待主人拍板后动工。
基线:    2026 会话清理现状取证（前端 clearLocalData / session-delete 链 / gateway delete_session）。
证据:    frontend/companion-desktop/src/App.svelte:2513（clearLocalData 实际动作）
         frontend/companion-desktop/src/lib/chat-shell/session-delete.ts:1-6（"复活"病灶记录）
         frontend/companion-desktop/src/lib/types.ts:460-486（近期记忆/长期记忆既有术语）
         crates/adapters/gateway/src/session_settings.rs:183-200（DELETE /v1/sessions/{id}）
         docs/01-architecture/forget-three-phase-production-spec.md（记忆遗忘协议）
状态:    C1-C4 裁决位已拍板（按拟定值）；P0 已实施——测试层验收完成
         （pnpm test 41/41 套件含新增 tests/session-cleanup.mjs 七组断言、
         pnpm check 0/0）；场景实测走查与主人签核待办；P1（批量端点）/P2（③ 接线）未动工。
```

---

## §0 病灶（为什么要修）

1. **"清空本地会话数据"名实仅半符**：实际只清 localStorage 会话正文
   （`conversations = []; activeId = null; persist()`，App.svelte:2513），后端完全不碰。
   按钮描述（"长期记忆不会受影响"）是诚实的，但见病灶 2——行为本身有缺陷。
2. **清完"复活"**：会话列表是「本地 ⋈ 后端账本」归并（session-list.ts:6）。只清本地账，
   后端账本行会立刻把会话补回列表。session-delete.ts:1-6 记录的内测病灶（"删除后列表不消失，
   重启又复活"）与本按钮同根因，单会话删除链已修，**批量清空按钮至今带病**。
3. **词汇撞车风险**："近期记忆/长期记忆"已是产品既有术语，指**记忆库条目分类**
   （types.ts:460-486：`近期记忆 = event/feedback` 类，`长期记忆 = fact` 等；MemoryView 按此筛选）。
   若把清会话的按钮改名"清除近期记忆"，用户会预期记忆条目被清——实际不会，制造新的名实不符，
   违反宣称挂标制（founding-design-v2 §0.3：能力描述与台账一致）。

---

## §1 语义决策（三按钮矩阵）

| 按钮 | scope | 动作 | 明确保留 | 守门 |
|---|---|---|---|---|
| **① 清除近期对话记录（保留长期记忆）** | `updated_at` 在近期窗口内的会话 | 本地正文删 + 后端账本行真删（`DELETE /v1/sessions/{id}`，防复活）+ 列表刷新 | 记忆库全量（近期记忆条目与长期记忆都不动） | 二次确认 + 审计留痕 |
| **② 清除全部会话数据（保留长期记忆）** | 全部会话（本地 ∪ 后端账本） | ① 全量版 + 会话级挂起件（挂起审批随 Session 行内嵌删除） | 记忆库全量 | danger 强确认 + 审计留痕 |
| **③ 记忆遗忘**（独立入口，**不属于本组**） | 记忆库 | CoordinatedForget 三段式：明文+派生面+执行态一次性清 | — | R3 硬门 + 人工审批（`approval_id` 引用上游审批） |

### 红线（任何实现不得违反）

- **红线 A**：② **绝不隐式包含记忆清除**。把"清会话"和"忘记忆"捆绑成一键 = 让单次误击可摧毁
  终身记忆，违反 R3 硬门与三段式遗忘协议。② 的确认弹窗只**提供③的入口提示**（可选裁决位），不代执行。
- **红线 B**：命名用"**对话/会话**"字样；"**记忆**"一词留给 ③。心理模型对照写进弹窗描述：
  "对话记录是近期的聊天内容；长期记忆是他从相处中学到的东西，不受影响。"
- **红线 C**：每个按钮描述必须逐句写明清什么、留什么（宣称挂标制在 UI 的落地形式，
  沿用现按钮"后端数据库中的长期记忆不会受影响"句式的传统）。
- **红线 D**：删除真到位——本地与后端账本必须一致，**不允许"复活"**（复用 session-delete 链纪律）。

### 裁决位（已拍板：按拟定值实施）

| # | 裁决位 | 裁决 |
|---|---|---|
| C1 | ① 的"近期"窗口默认值 | ✅ **7 天**（`RECENT_WINDOW_DAYS_DEFAULT`，可配；改窗口必须同步弹窗文案——tests 机器断言两者一致） |
| C2 | ② 是否含前端 call-log（`call-logger.ts` localStorage 工具调用日志） | ✅ **含**（② 档全清；① 档只清已删会话归属条目，无归属条目保留） |
| C3 | ② 强确认形式 | ✅ **二次确认 + 红色危险样式** |
| C4 | ② 弹窗是否放 ③ 入口链接 | ✅ **放**——③ 未接线前灰显「尚未接线」标注 |

---

## §2 行为规格

### 2.1 失败语义（两档共用，复用 session-delete.ts 四段纪律）

1. 乐观移除（确认后立即从列表消失）→ 2. 本地持久化 → 3. 后端真删 → 4. 列表刷新对齐后端账本。
- 任一步失败 = **整体回滚**（列表还原快照 + 持久化尽力而为）+ 亮错误帧（不吞、不粉饰）。
- `404 session_not_found` = **成功**（本机草稿从未进后端账本；沿用 session-delete.ts:28-30 约定）。
- 批量（②）：逐条复用 `deleteSession()`，聚合 outcome；**部分失败 = 保留失败项、亮帧列出失败 id**，
  不谎报"已清空"。成功判据 = 后端账本为空 ∧ 本地列表为空 ∧ 重启后仍空。

### 2.2 悬空引用

记忆条目绑定 session id（MemoryView 会话过滤器）。会话清掉后记忆必须完好且 UI 优雅：
过滤器取不到对应会话时显示"会话已清除"占位，**不得报错或连带删除记忆行**（连带删除 = 越权进③语义）。

### 2.3 审计留痕

删除清单（会话 id 集 + 时间 + 档位 ①/②）进 call-log 同级的本地审计记录。
**0 装诚实**：gateway 侧审计链当前未对 delete_session 接线——批量端点（P1）落审计行，
P0 阶段如实标注"本地留痕，后端审计待接"。

---

## §3 改动清单（文件级）

### P0（前端为主，零后端改动即可消灭复活缺陷）

| 文件 | 改动 |
|---|---|
| `frontend/companion-desktop/src/lib/chat-shell/session-cleanup.ts`（新增） | 纯函数清理链（零 DOM 依赖，Node 直测，仿 session-delete.ts 注入端口模式）：`recentWindowFilter()` + `purgeSessions(scope, ports)` 聚合版 |
| `frontend/companion-desktop/src/lib/settings-live-apply.ts` | `DangerActionKey` 改为 `'clearRecentConversations' \| 'clearAllSessionData' \| ...`；两套弹窗文案（§4 文案定稿） |
| `frontend/companion-desktop/src/lib/views/SettingsView.svelte` | 危险区改两按钮（原 `clearLocalData` 单按钮拆分）+ 描述文案 |
| `frontend/companion-desktop/src/App.svelte` | 原 `onClearLocalData` 拆两个 handler，接入 purgeSessions；成功后 bump `homeReloadKey` |
| `frontend/companion-desktop/src/lib/chat-shell/session-list.ts` | 暴露会话 `updated_at` 供窗口过滤 |
| `frontend/companion-desktop/src/lib/MemoryView.svelte` | 悬空 session 引用占位（§2.2） |
| `frontend/companion-desktop/tests/session-cleanup.mjs`（新增） | 回归四件套：① 复活场景（清后端行回填）② 窗口过滤边界 ③ 部分失败聚合 ④ 404=成功 |

### P1（gateway）

| 文件 | 改动 |
|---|---|
| `crates/adapters/gateway/src/session_settings.rs` | 新增 `DELETE /v1/sessions` 批量端点：逐 id 结果数组 + 审计行（复用 `SessionStore::list/delete`，session.rs:768-786 已具备） |
| `crates/adapters/gateway/tests/` | 批量端点测试：幂等（重复删=不存在算成功）、部分失败如实返回 |

### P2（记忆遗忘入口，另行按协议接线）

- ③ 灰显入口 + CoordinatedForget 接线**另立实施项**（forget-three-phase-production-spec.md 是唯一依据）；
  本预案只保证①②**不越界**碰记忆。

---

## §4 文案定稿（红线 C 的机器可检形式）

**① 清除近期对话记录（保留长期记忆）**
> 将删除最近 {N} 天内的对话记录：本机聊天正文与后端账本记录一并真删，重启不复活。长期记忆（他从相处中学到的东西）不受影响。此操作无法撤销。

**② 清除全部会话数据（保留长期记忆）**
> 将删除**全部**会话：本机聊天正文、后端账本记录与挂起的审批一并真删。长期记忆不受影响——如需连同长期记忆一起遗忘，是另一个动作（记忆遗忘），需要单独审批。此操作无法撤销。

（两段都必须含"长期记忆不受影响"句——tests 里做机器断言，仿 theme-system.mjs 的金色纪律断言模式。）

---

## §5 验收（四层纪律的本项落地）

1. **测试**：✅ 已完成——`tests/session-cleanup.mjs` 七组断言（复活病灶/窗口边界/部分失败聚合/
   空转·回滚·账本缺失/调用日志档位/登记表/文案红线）+ 文案断言绿；`pnpm test` 41/41 套件；
   `pnpm check` 0 errors / 0 warnings。
2. **场景实测走查**：⏳ 待办（人工）——清① → 列表只剩窗口外会话 ∧ 重启不复活；清② → 列表空 ∧
   重启仍空；MemoryView 数据完好、悬空引用显示占位；后端宕机时 → 取消清理 + 错误帧可见。
3. **文案-行为逐句核对**：⏳ 待办（与走查同批）——§4 每句与实际动作对齐后才准挂"已验收"标。
4. **主人签核**：⏳ 待办——结论入 live-verification-ledger 同源台账。

## §6 风险与回滚

| 风险 | 缓解 |
|---|---|
| 误删不可逆 | 强确认 + 逐项文案 + 审计留痕；C3 输入式确认备选 |
| 批量部分失败观感差 | 如实聚合报 N/M + 失败 id 列出（红线：不谎报清空） |
| ③ 未接线造成"找不到彻底删除入口" | C4 弹窗入口提示 + 灰显"未接线"如实标注（0 装） |
| 实现改动波及设置契约 | `reports/settings-live-apply-contract.md` 同步更新 DangerActionKey 行 |

---

> **本文件是活文档**：裁决位拍板后更新 §1 表；验收完成后更新 §5/状态标。历史行为记录不改写，修正以本文表达。
