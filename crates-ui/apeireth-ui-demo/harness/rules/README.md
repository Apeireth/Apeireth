# 规则层差分装置（TS ↔ Rust 逐字比对）

> **目的**：生产前端（TypeScript）与引擎侧（Rust）对**同一条业务规则**各有一份实现
> ⇒ 两份实现会**漂移** ⇒ 既有对照装置（`harness/governance/`）只比**画面**、
> **不比规则** ⇒ 漂移无人知晓。
> ⇒ 本装置把「**同一组输入 ⇒ 两侧各跑一遍 ⇒ 输出逐字相同**」变成 **CI 门禁**。

## 三个文件（各司其职）

| 文件 | 角色 | 谁维护 |
|---|---|---|
| `cases.json` | **唯一真源**：每条规则一组 case（`args` = 两侧共用输入，`expected` = TS 侧真值） | 人手写 `args`；`expected` 由机器写 |
| `gen_expected.mjs` | **TS 侧执行器**：直接 `import` 生产 `.ts` ⇒ 逐条算 ⇒ 把结果写进 `expected` | 只读生产前端 |
| `README.md`（本文件） | 口径 / 归一化 / 加规则流程 | 人 |
| `red_green.mjs` | **红-绿自证器**：临时篡改 `expected` ⇒ 证明装置**真的在比** ⇒ 跑完 md5 自检 | 只读生产前端 |

Rust 侧的比对测试在 **`crates-ui/apeireth-ui-kit/tests/rules_parity.rs`**（跨 crate 读取本目录的
`cases.json` ⇒ 两侧共用同一份真源）。

## 跑法

```bash
# ① 生成 / 刷新 expected（**必须**带 --experimental-strip-types；Node 22.x 才能直接 import 生产 .ts）
/tmp/n22/node_modules/node-linux-x64/bin/node --experimental-strip-types \
  crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs

# ② 只校验不写回（CI 可用：与生产 TS 实算不一致 ⇒ 退出码 1）
/tmp/n22/node_modules/node-linux-x64/bin/node --experimental-strip-types \
  crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs --check

# ③ Rust 侧比对（native ⇒ 0 网络、0 浏览器、0 外部依赖）
cd crates-ui
CARGO_TARGET_DIR=/home/bai/Projects/apeireth-ui-target RUSTC_WRAPPER= \
  cargo test -p apeireth-ui-kit --test rules_parity --locked
```

> ⚠️ **Node 版本**：生产 `.ts` 直跑需要 **Node 22.x + `--experimental-strip-types`**
> （本机实测 `v22.23.1`；系统默认的 24 未实测，勿混用）。
> ⚠️ **`crates-ui/` 是独立 cargo workspace** ⇒ `cargo test -p apeireth-ui-kit` 必须在
> `crates-ui/` 目录下跑（仓库根的 workspace **不含**该 crate）。
> ⚠️ **时区**：`gen_expected.mjs` 自己会设 `TZ=Asia/Shanghai`（见「时区口径」）⇒
> **不需要**调用方手动设 `TZ`；但**不要**把它改成别的时区（Rust 侧夹具是固定 +08:00）。

## 口径（**两侧必须逐字相同**）

| 项 | 口径 |
|---|---|
| 真值方向 | `expected` **一律**来自**生产 TypeScript 实算** ⇒ 人手改 `expected` 必被 Rust 侧判红 |
| 归一化 | 两侧类型不同 ⇒ 各转成**可比的 JSON** 再比；**不比**额外字段（两侧只构造归一化字段） |
| 键序 | 两侧的归一化对象按**同一顺序**固定构造 ⇒ 逐字比较时键序**也相同**（⇒ 键序错位即红） |
| 布尔 / 字符串 | 原样（字符串逐码点相等） |
| 计数 | 摊平成 `{all, open, expired}` |
| 条目数组 | `{length, items:[…]}`；每项取 `approvalId / session / sessionTitle / createdAtMs / expiresAtMs`；**缺失与 `null` 一律归一为 `null`**；顺序敏感 |
| 已知漂移 | 用例标 `"parity": "known_drift"` ⇒ 钉住**当前**漂移形态（见下） |

> **当前规模（2026-10-26 实测）**：**29 条规则 / 240 个用例**
> （= 28 条**同名规则全部收录** + 1 条小数面补充规则；5 条 `known_drift`）。

### 逐条规则的归一化（一句话）

| 规则 | 归一化 |
|---|---|
| `is_approval_expired` | 布尔原样（TS `boolean` ↔ Rust `bool`，零变换）。 |
| `approval_filter_counts` | 计数三元组摊平成 `{all, open, expired}`（TS 的 `Record<ApprovalFilter,number>` ↔ Rust 的 `(usize,usize,usize)`）。 |
| `filter_approval_ledger` | 数组 ⇒ `{length, items:[归一化条目]}`（保序筛选 + 稳定排序 ⇒ 顺序敏感）。 |
| `merge_session_inbox` | 数组 ⇒ `{length, items:[归一化条目]}`（同会话整组替换 + 其余不动 + 按 `createdAtMs` 倒序稳定排序 ⇒ 顺序敏感）。 |
| `format_relative_ms` | 字符串原样（逐码点相等，含中文与空格）。 |
| `format_duration_ms` | 字符串原样（逐码点相等，含 `ms` / `s` 后缀与小数点）。 |
| `format_clock_ms` | 字符串原样（逐码点相等，**按 +08:00 渲染**；非法日期两侧同为「—」）。 |
| `format_date_time_ms` | 字符串原样（逐码点相等，**同日按本地日历日**判 ⇒ 同日 `HH:MM`、跨日 `MM/DD HH:MM`）。 |
| `find_capability` | 命中 ⇒ `{found, id, supported, available, reason, aliasOf}`；未命中 ⇒ `{found:false, 其余全 null}`；只看**命中的那一条**声明。 |
| `capability_available` | 布尔原样。 |
| `capability_unavailable_reason` | 字符串或 `null`（逐码点相等）。 |
| `format_duration` | 字符串原样（含「—」占位与 `m` / `s` 后缀）。 |
| `format_session_totals` | 五字段显示串对象 `{tokens, cache, turns, duration, toolCalls}`（逐码点相等）。 |
| `resolve_budget_limit_input` | 四字段对象 `{value, effective, status, feedback}`（解析结果摊平；反馈文案逐码点相等）。 |
| `budget_knob_badge` | 四字段对象 `{effective, source, note, label}`（徽标摊平；注记与文案逐码点相等）。 |
| `effective_budget` | 三键徽标对象（`maxTurnRounds` / `maxToolCalls` / `contextBudgetChars`），徽标 = 上行四字段。 |
| `budget_remaining_rows` | 数组 ⇒ `{length, items:[{key,label,scope,cap,used,remaining,ratio}]}`；`ratio` 数字或 `null`；固定七行、顺序敏感。 |
| `short_id` | 字符串原样（UTF-16 码元计数与截断；截断位补 `…`；`head` 缺省 ⇒ 两侧同取 8）。 |
| `span_depth` | 整数原样（父链深度；上限 6；环由 visited 集合截断）。 |
| `span_duration_ms` | 数值或 `null`（未收口 ⇒ `null`；负时长钳到 0）。 |
| `span_status_badge` | 两字段 `{variant, label}`（Rust 的 `BadgeVariant` 取 `class_name` ⇒ 与 TS 字面量同形）。 |
| `audit_event_group` | 字符串原样（取首段并去空白；空 ⇒ `other`）。 |
| `audit_group_chips` | 数组 ⇒ `{length, items:[{group,count}]}`；按 count 倒序（两侧皆稳定排序）。 |
| `filter_audit_items` | 数组 ⇒ `{length, items:[{id,timestamp,title,summary,source,detail}]}`；`detail` 缺失与 `null` 归一为 `null`；保序筛选。 |
| `guard_decision_badge` | 两字段 `{variant, label}`（同上取 `class_name`）。 |
| `guard_stage_label` | 字符串原样（三段管线中文名；未知名原样返回）。 |
| `risk_score_percent` | 整数原样（0..1 钳制后 ×100 取整，半数向 +∞）。 |
| `guard_event_row` | 四字段 `{id, badge:{variant,label}, stageLabel, riskPct}`（键序固定）。 |
| `format_duration_f64` | 字符串原样（**小数毫秒面**；与 `format_duration` 同实现、不同入口）。 |

> **补充规则**：`format_duration_f64` **不是**同名规则 ⇒ 它是 `format_duration`
> 的**小数面补充**（见「`format_duration` 的小数 API 面差异」一节）。

## 同名规则清点表（**覆盖面口径**）

### 判据

> **收录所有「Rust 侧有 `pub fn`、生产侧有同名 `export function`」的规则。**

「同名」按 **snake_case ↔ camelCase** 归一（Rust `pub fn foo_bar` ↔ TS `export function fooBar`）。

### 清点方法（可复跑）

```bash
# ① Rust 侧：全部 pub fn（去重）
grep -rn 'pub fn ' crates-ui/apeireth-ui-kit/src/ \
  | sed -E 's/.*pub fn ([a-zA-Z0-9_]+).*/\1/' | sort -u

# ② 生产侧：全部 export function（去重）
grep -rn 'export function ' frontend/companion-desktop/src/lib/ \
  | sed -E 's/.*export function ([a-zA-Z0-9_]+).*/\1/' | sort -u

# ③ 取交集（归一化：去下划线 + 小写 ⇒ 命中）
```

### 实测结论（2026-10-26）

| 项 | 数 |
|---|---|
| Rust 侧 `pub fn`（**去重**后） | **135** |
| 生产侧 `export function`（**去重**后） | **374** |
| **同名规则（去重）** | **28** |
| ⇒ 本装置已收录 | **29 条规则 / 240 个用例**（= 28 条同名 **全部** + 1 条小数面补充规则） |

> ⚠️ **与上一批口径的差**：上一批报「29 条同名」⇒ 本批**实测 28 条**。
> 差的 1 条来自 **`format_duration` 在 Rust 侧有两处 `pub fn`**：
> `settings_budget::format_duration`（`Option<i64>`）与 `tool_card::format_duration`（`Option<f64>`）
> ⇒ 按**函数名去重**是 1 条，按**出现次数**是 2 条。
> ⇒ 本表按**函数名去重**（与生产侧「一个导出名 = 一条规则」对齐）。
> ⇒ `tool_card::format_duration` 的生产对应物**不在** `export function` 名单里
> （它在 Svelte 组件 `ToolCallCard.svelte` 的**局部函数**内）⇒ 不属同名规则。

### 全量清点表（28 条同名规则 ⇒ 收录状态）

| # | Rust `pub fn`（模块） | 生产 `export function`（文件） | 收录 |
|---|---|---|---|
| 1 | `approval_filter_counts`（governance_tab） | `approvalFilterCounts`（governance/ledger.ts） | ✅ 上一批 |
| 2 | `audit_event_group`（governance_tab） | `auditEventGroup`（governance/trace.ts） | ✅ **本批** |
| 3 | `audit_group_chips`（governance_tab） | `auditGroupChips`（governance/trace.ts） | ✅ **本批** |
| 4 | `budget_knob_badge`（settings_budget） | `budgetKnobBadge`（budget.ts） | ✅ 上一批 |
| 5 | `budget_remaining_rows`（settings_budget） | `budgetRemainingRows`（budget.ts） | ✅ 上一批 |
| 6 | `capability_available`（governance_tab） | `capabilityAvailable`（runtime.ts） | ✅ 上一批 |
| 7 | `capability_unavailable_reason`（governance_tab） | `capabilityUnavailableReason`（runtime.ts） | ✅ 上一批 |
| 8 | `effective_budget`（settings_budget） | `effectiveBudget`（budget.ts） | ✅ 上一批 |
| 9 | `filter_approval_ledger`（governance_tab） | `filterApprovalLedger`（governance/ledger.ts） | ✅ 上一批 |
| 10 | `filter_audit_items`（governance_tab） | `filterAuditItems`（governance/trace.ts） | ✅ **本批** |
| 11 | `find_capability`（governance_tab） | `findCapability`（runtime.ts） | ✅ 上一批 |
| 12 | `format_clock_ms`（governance_tab） | `formatClockMs`（governance/format.ts） | ✅ 上一批 |
| 13 | `format_date_time_ms`（governance_tab） | `formatDateTimeMs`（governance/format.ts） | ✅ 上一批 |
| 14 | `format_duration`（settings_budget） | `formatDuration`（chat-shell/turn-telemetry.ts） | ✅ 上一批 |
| 15 | `format_duration_ms`（governance_tab） | `formatDurationMs`（governance/format.ts） | ✅ 上一批 |
| 16 | `format_relative_ms`（governance_tab） | `formatRelativeMs`（governance/format.ts） | ✅ 上一批 |
| 17 | `format_session_totals`（settings_budget） | `formatSessionTotals`（chat-shell/turn-telemetry.ts） | ✅ 上一批 |
| 18 | `guard_decision_badge`（governance_tab） | `guardDecisionBadge`（governance/guard.ts） | ✅ **本批** |
| 19 | `guard_event_row`（governance_tab） | `guardEventRow`（governance/guard.ts） | ✅ **本批** |
| 20 | `guard_stage_label`（governance_tab） | `guardStageLabel`（governance/guard.ts） | ✅ **本批** |
| 21 | `is_approval_expired`（governance_tab） | `isApprovalExpired`（governance/ledger.ts） | ✅ 上一批 |
| 22 | `merge_session_inbox`（governance_tab） | `mergeSessionInbox`（governance/ledger.ts） | ✅ 上一批 |
| 23 | `resolve_budget_limit_input`（settings_budget） | `resolveBudgetLimitInput`（budget.ts） | ✅ 上一批 |
| 24 | `risk_score_percent`（governance_tab） | `riskScorePercent`（governance/guard.ts） | ✅ **本批** |
| 25 | `short_id`（governance_tab） | `shortId`（governance/format.ts） | ✅ **本批** |
| 26 | `span_depth`（governance_tab） | `spanDepth`（governance/trace.ts） | ✅ **本批**（含 1 条 known_drift） |
| 27 | `span_duration_ms`（governance_tab） | `spanDurationMs`（governance/trace.ts） | ✅ **本批** |
| 28 | `span_status_badge`（governance_tab） | `spanStatusBadge`（governance/trace.ts） | ✅ **本批** |

⇒ **28 条同名规则全部收录**（本批新增 11 条 + 上一批 17 条）⇒ **同名规则覆盖率 100%**。

**补充规则（1 条，非同名）**

| Rust `pub fn`（模块） | 生产 `export function`（文件） | 收录 | 理由 |
|---|---|---|---|
| `format_duration_f64`（settings_budget） | `formatDuration`（chat-shell/turn-telemetry.ts） | ✅ **本批** | 生产 `formatDuration` 收 `number` ⇒ **可小数**；Rust 侧原入口 `format_duration` 收 `Option<i64>` ⇒ **小数表达不了** ⇒ 补一个**同实现的 f64 入口**（见下）。 |

### 为什么这 11 条**都能收录**（判据逐条）

| 规则 | 判据 | 结论 |
|---|---|---|
| `short_id` | 纯函数（`string` ⇒ `string`） | ✅ 收录 |
| `span_depth` | 纯函数（结构化输入 ⇒ 整数） | ✅ 收录（**1 条 known_drift**：空串父 id） |
| `span_duration_ms` | 纯函数（结构化输入 ⇒ 数值 \| `null`） | ✅ 收录 |
| `span_status_badge` | 纯函数（`string` ⇒ `{variant,label}`） | ✅ 收录 |
| `audit_event_group` | 纯函数（`string` ⇒ `string`） | ✅ 收录 |
| `audit_group_chips` | 纯函数（数组 ⇒ 数组） | ✅ 收录 |
| `filter_audit_items` | 纯函数（数组 ⇒ 数组） | ✅ 收录 |
| `guard_decision_badge` | 纯函数（`string` ⇒ `{variant,label}`） | ✅ 收录 |
| `guard_stage_label` | 纯函数（`string` ⇒ `string`） | ✅ 收录 |
| `risk_score_percent` | 纯函数（`number` ⇒ 整数） | ✅ 收录 |
| `guard_event_row` | 纯函数（结构化输入 ⇒ 结构化输出） | ✅ 收录 |

⇒ **11 条全部是 DOM-free 的纯函数** ⇒ 全部可入装置；**无一依赖 DOM / 宿主 / 网络**。
⇒ **零** `ui-kit/src/**` 改动即可收录这 11 条（只有 `format_duration_f64` 需要一个**新入口**）。

> 同一张表也写在 `cases.json` 的 `normalization` 字段里 ⇒ Rust 侧用
> `NORMALIZATIONS` 常量表**双向校验**（改了文档不改适配器 ⇒ 测试变红）。

## ⏰ 时区口径（**两侧怎么对齐**）

生产 `formatClockMs` / `formatDateTimeMs` 走
`new Date(ts).toLocaleTimeString('zh-CN', {hour:'2-digit', …})`
⇒ 产出同时依赖**本地时区**与 **ICU 版本**
⇒ ⇒ 两侧**必须落在同一个时区**才有可比性，否则是**口径未统一**（假红），不是规则漂移。

| 侧 | 做法 |
|---|---|
| **Node（TS 真值）** | `gen_expected.mjs` 在**进程最早期**（任何 `Date` / `Intl` 调用之前）设 `process.env.TZ = 'Asia/Shanghai'`。Node ≥ 16 支持**运行中**改 `process.env.TZ`（会重置 ICU 的时区缓存）⇒ 实测生效（见下方证据）。 |
| **Rust（引擎侧）** | `rules_parity.rs` 里自实现一个**固定 +08:00** 的 `TimeTextFormatter`（`PlusEightTimeText`）⇒ 在每条时刻规则的适配器里装进**进程级槽**（`set_time_text_formatter`）。⚠️ **不动**引擎默认的 `UtcTimeText`（native 默认仍是固定 UTC ⇒ 生产口径仍由 wasm 宿主的 `BrowserTimeText` 承担）。 |

⇒ ⇒ 两侧同在 `Asia/Shanghai` ⇒ 本装置比的是「**非 UTC 时区下也逐字一致**」
⇒ **比 UTC 比对更有价值**（它能抓住「按本地日判同日」这类只在非 UTC 下才暴露的偏差）。

### 实测证据（Node 侧真的切到 +08:00 了吗）

生成器每次运行都跑一道**自检**（不通过 ⇒ 退出码 1 ⇒ 不产出可能错的期望值）：

```
[rules] 时区自检通过：TZ=Asia/Shanghai（2026-10-24T09:30:00Z ⇒ 17:30:00）
```

判据两条：① `Intl.DateTimeFormat().resolvedOptions().timeZone === 'Asia/Shanghai'`；
② 已知时刻 `2026-10-24T09:30:00Z` 的 `toLocaleTimeString` 必须给 **`17:30:00`**
（= UTC **+8 小时**）⇒ 证明输出真的按 +08:00 渲染（若仍是 UTC 会给 `09:30:00`）。

用例里也能直接看出来：`format_clock_ms / epoch-zero`（`tsMs=0`）的期望值是
**`08:00:00`**（UTC 则是 `00:00:00`）⇒ 时刻确实按 +08:00 渲染。

### 「同日」判定与「现在」的取法

- 生产 `formatDateTimeMs(ts)` 内部用 `new Date()`（= **真实当前时刻**）判「同日」
  （`d.toDateString() === today.toDateString()` ⇒ **按本地时区**）。
- ⇒ 用例里的 `now_ms` 是**钉住的「现在」**：
  - **TS 侧**：调用生产函数**期间**把全局 `Date` 换成「无参构造 ⇒ 返回固定 `nowMs`」的
    子类（`withPinnedNow`）⇒ **生产文件逐字未改**，只是「现在」被钉住；
  - **Rust 侧**：直接用用例给的 `now_ms`（`format_date_time_ms(ts, now)`）。
- ⇒ 「同日」在两侧都按 **+08:00 日历日**判 ⇒ 可复现（不随运行时刻漂移）。
- 覆盖到的分支：同日 / 跨日 / 跨月 / 跨年 / 负 epoch / 极大值 / `00:00:00` 与 `23:59:59`
  边界 / **非法日期**（超出 JS 有效范围 ⇒ 两侧同为「—」）。
- ⚠️ 有一条用例专门证时区：`format_date_time_ms / same-day-utc-differs`
  （`tsMs=-1000, nowMs=0`）⇒ **UTC 日不同（12-31 vs 01-01）但本地日相同** ⇒ 期望 `07:59`
  ⇒ 若哪一侧偷偷按 UTC 判同日，这条会红。

## 已知漂移怎么登记（`parity: "known_drift"`）

用例可带三个字段：

```json
{
  "id": "expires-null",
  "args": { "…": "…" },
  "expected": true,          // TS 侧真值（机器写）
  "parity": "known_drift",   // 已知漂移
  "rust_expected": false,    // Rust 侧**当前**行为（人手钉住）
  "why": "漂移说明（必填）"
}
```

⇒ Rust 侧断言：**两侧仍不相等**且 **Rust 侧仍等于 `rust_expected`**。
⇒ 漂移若被任何一侧修好 ⇒ 测试**变红**并提示「请更新 cases.json」⇒
**漂移不会被静默吞掉**。

**当前登记的漂移（5 条）**：

1. `is_approval_expired` / `expires-null` ——
   生产 `isApprovalExpired` 写的是 `entry.expiresAtMs !== undefined && entry.expiresAtMs <= nowMs`
   ⇒ 字段为 **`null`** 时 JS 把 `null` 强制为 `0` ⇒ `0 <= nowMs` 为真 ⇒ **TS 判过期**；
   Rust 的 `Option<i64>` 判 `None` ⇒ **不判过期** ⇒ **两侧不一致**。
   （当前后端契约用 `skip_serializing_if` ⇒ 字段是**省略**而非 `null` ⇒ 属**契约边界**；
   本装置把它钉住，等口径改造后按 `known_drift` 的规则更新。）
2. `resolve_budget_limit_input` / `known-drift-unsafe-int` ——
   生产 `parseIntegerField` 用 `Number.isSafeInteger` 判定 ⇒ **2^53 及以上**被当成非法值
   ⇒ 回默认；Rust 侧是 `i64`（上限 2^63-1）⇒ 同一串字面量被当成合法值直通。
   ⇒ 属**整数通道宽度不同**（不是文案漂移），装置把它钉住。
3. `merge_session_inbox` / `merge-created-at-no-offset-known-drift` ——
   `created_at` **不带时区偏移**时，生产 `Date.parse` 按 ES 规范把「无偏移日期时间」
   当作**本地时间** ⇒ 装置时区（Asia/Shanghai）下折算成 UTC 要**减 8 小时**；
   Rust 侧 `parse_rfc3339_ms` 把「缺偏移」当 **UTC**（偏移量 0）⇒ 两侧**差 8 小时**。
   ⇒ 属「本地时区 vs UTC」的**解析口径差**（Rust 侧 0 依赖 ⇒ 无本地时区概念 ⇒ **不可统一**）。
4. `merge_session_inbox` / `merge-created-at-date-only-known-drift` ——
   `created_at` 是**纯日期**（`2026-01-02`）时，生产 `Date.parse` 接受（⇒ UTC 零点），
   Rust 侧 `parse_rfc3339_ms` 要求完整 `YYYY-MM-DDTHH:MM:SS` ⇒ **解析失败 ⇒ 字段为 `null`**。
   ⇒ 属「生产 `Date.parse` 的可接受形态**更宽**」的解析口径差。
5. `span_depth` / `span-depth-empty-parent` ——
   父 span id 是**空串**时，生产 `spanDepth` 的循环条件是 `cur && …` ⇒ 空串是 JS **假值**
   ⇒ 直接停 ⇒ 深度 **0**；Rust 侧是 `Option<String>` ⇒ `Some("")` **仍进循环** ⇒ 深度 **1**。
   ⇒ 属「JS 假值语义 vs Rust `Option` 语义」的口径差。

> ⇒ **5 条漂移都被「假装修好 ⇒ 必须变红」钉住**（见「红-绿自证」一节）。
> ⇒ **不统一**的理由逐条写在各用例的 `why` 字段里（**统一 = 改行为** ⇒ 本装置只钉，不改）。

## 加一条新规则（**逐步**）

1. **`cases.json`**：加一个 rule 对象：

   ```json
   {
     "rule": "<规则名（= Rust 侧函数名）>",
     "ts":   {"file": "<生产 .ts 路径>", "export": "<生产导出名>"},
     "rust": {"crate": "apeireth-ui-kit", "module": "<模块名>", "fn": "<函数名>"},
     "normalization": "<归一化一句话>（**必须与 Rust 侧 NORMALIZATIONS 逐字相同**）",
     "args_schema": "<args 的形状说明>",
     "cases": [
       {"id": "<用例 id>", "args": { }, "parity": "same"}
     ]
   }
   ```

   - 只写 `id` + `args` + `"parity": "same"`；**`expected` 留给机器写**。
   - **至少 4 个用例**（自检会卡），必须覆盖：**负数 / 恰好等于边界 / 空集合 / 缺省字段 /
     `null` 字段**（按规则语义取舍）。
   - 若规则要吃**结构化输入**（如能力清单、旋钮行），在 rule 级加 `knob_rows` 之类的夹具字段
     （Rust 侧 `Rule` 结构体要同步加对应字段）。
   - **结构化输入的夹具形状**：装置的夹具**只列规则会读的字段**（其余字段置空串 / `None`）
     ⇒ 两侧都只读那些字段 ⇒ 可比；代价是「生产规则将来开始读别的字段」**不会被装置发现**
     ⇒ 已登记在「已知未覆盖清单」。
   - **Rust 侧入口与生产入口**不同型时（如 `Option<i64>` vs `number`）：
     要么在 Rust 侧补**同实现的另一入口**（如 `format_duration_f64`），
     要么写进「已知未覆盖清单」（**不许**假装覆盖）。

2. **`gen_expected.mjs`**：在 `buildImpls()` 里加一条 `(args) => 归一化输出` 的 TS 实现
   （**直接调生产函数**，不要重写业务逻辑）。若生产函数不接受外部规格表（如
   `effectiveBudget` 内部写死 `BUDGET_KNOB_SPECS`），就把规格做成**用例夹具**并在
   生成器里**核对它与生产常量表一致**（不一致 ⇒ 退出码 1）。
   ⇒ 生产模块若是**只读**（`import type`）⇒ Node 的 `--experimental-strip-types`
   会把类型导入**擦除** ⇒ 不会连带加载运行时副作用 ⇒ 可安全直跑。

3. **`rules_parity.rs`**：
   - 在 `eval()` 里加同名分支（JSON ⇒ Rust 类型 ⇒ 调 Rust 函数 ⇒ 归一化 JSON）；
   - 在 `NORMALIZATIONS` 表里加**同一句**归一化口径（否则自检测试会红）；
   - 加一个 `#[test] fn <rule>_matches_production_typescript()`（体是 `assert_rule("<rule>")`）。
   - **徽标语义**（`{variant,label}`）两侧形状不同 ⇒ Rust 侧取 `BadgeVariant::class_name()`
     ⇒ 与 TS 的字面量（`'green'|'amber'|'danger'|'blue'|'dim'`）**同形** ⇒ 可逐字比。

4. 跑 ①②③ ⇒ 全绿才收工。

5. **红-绿自证**：把某个 `expected` 临时改错 ⇒ 对应 Rust 测试**必须变红** ⇒ 恢复后回绿。
   （这是「装置真的在比」的证据，不是形式。）
   ⇒ 脚本化跑法见「红-绿自证（脚本化）」一节。

> ⚠️ 加了规则却忘了写 Rust 适配器 ⇒ 自检测试会红（`规则「x」没有 Rust 侧适配器`）。
> ⚠️ `expected` 只能由生成器写 ⇒ 手改必被 `--check` 与 Rust 侧判红。

## 红-绿自证（脚本化）

装置的「真的在比」不是形式 ⇒ 每条新规则都要**实测**一次红-绿。

脚本：**`harness/rules/red_green.mjs`**（**只读**生产前端与 Rust 侧；不改任何生产代码）。

```bash
# 普通规则（篡改 expected ⇒ 必须变红）
node crates-ui/apeireth-ui-demo/harness/rules/red_green.mjs \
  short_id span_depth guard_event_row

# 已知漂移用例（假装「漂移被修好」⇒ 必须变红）
node crates-ui/apeireth-ui-demo/harness/rules/red_green.mjs --drift
```

### 普通规则（篡改 `expected` ⇒ 必须变红）

实测输出形态（`guard_event_row`）：

```
thread 'guard_event_row_matches_production_typescript' panicked at
  apeireth-ui-kit/tests/rules_parity.rs:1697:13:
规则「guard_event_row」用例「guard-row-allow」两侧不一致：
  TS 侧（expected）= {"id": "tampered", …}
  Rust 侧（实际）= {"id": "trace-1-3-1792922400000", "badge": {"variant": "green", "label": "放行"}, …}
  输入 args = {"event": {…}}
test result: FAILED. 0 passed; 1 failed; …
⇒ 退出码 101
```

### 已知漂移用例（假装「漂移被修好」⇒ 必须变红）

把漂移用例的 `expected` **改成 `rust_expected`**（= 模拟两侧一致）⇒ 装置必须变红并提示
**「已知漂移已被修好 ⇒ 请更新 cases.json」** ⇒ 漂移**不会被静默吞掉**。

> ⇒ 脚本跑完会**自检** `cases.json` 的 **md5 回到原值**（篡改只是临时的）。
> ⇒ 本批 **12 条新规则** + **3 条新漂移用例** **全部**跑过红-绿（实测输出见交付报告）。

## 两个已知缺口的口径决策（**为什么这么判**）

### `merge_session_inbox` 的时刻解析口径差

| 判据 | 结论 |
|---|---|
| 生产 `approvalTimeMs` 用 `Date.parse` ⇒ 接受**多种形态**（无偏移日期时间按**本地时间**、纯日期按 UTC 零点、非 ISO 串如 `Jan 2, 2026` 也收） | 口径**更宽** |
| Rust `parse_rfc3339_ms` 只收 **RFC 3339**（`YYYY-MM-DDTHH:MM:SS[.fff][Z\|±HH:MM]`）⇒ 「缺偏移」当 **UTC** | 口径**更窄** |
| **统一**（改 Rust 侧解析器）？ | ❌ **不改**：① Rust 侧是 **0 外部依赖** crate ⇒ **无本地时区概念** ⇒ 「无偏移 = 本地」**无法实现**；② 改 `parse_rfc3339_ms` = **改行为**（它被多处调用）⇒ 违反本批纪律。 |
| **钉住漂移**？ | ✅ **本批采用**：两条 `known_drift` 用例分别钉住「无偏移 ⇒ 差 8 小时」与「纯日期 ⇒ Rust 解析失败」⇒ 漂移**被修好**时装置立刻红。 |

⇒ **可观测后果**：生产把「无偏移」的 `created_at` 按 **Asia/Shanghai** 解释，
Rust 按 **UTC** ⇒ 两者**差 8 小时** ⇒ 会影响「按 `createdAtMs` 倒序」的**排序**与**过期判定**。

### `format_duration` 的小数 API 面差异

| 判据 | 结论 |
|---|---|
| 生产 `formatDuration(ms: number \| null \| undefined)` ⇒ 收 **`number`** ⇒ **可小数** | 入口更宽 |
| Rust `format_duration(ms: Option<i64>)` ⇒ **小数表达不了** | 入口更窄 |
| 是**漂移**吗？ | ❌ **不是** ⇒ 是**两套 API 面**（同一规则、同一口径实现、入口类型不同）。 |
| **可观测吗**？ | ✅ **可观测**（实测穷举 `ms ∈ [0, 400000]` 的**小数**形态）：`ms ∈ [999.5, 1000)` ⇒ 生产给 **`1000ms`**（`ms < 1000` 分支里的 `Math.round(999.5)`），而**任何整数都给不出** `1000ms`（`999` ⇒ `999ms`，`1000` ⇒ `1.0s`）⇒ **不是名义差异**。 |
| 怎么处理？ | ✅ **给 Rust 侧加一个 `f64` 变体**（**新 API**，不改旧的）：`settings_budget::format_duration_f64(ms: Option<f64>)`；旧的 `format_duration(Option<i64>)` 改为**薄封装**（`i64 ⇒ f64`）⇒ **同一实现、同一口径** ⇒ 装置用 `format_duration_f64` 这条**补充规则**覆盖小数面（13 个用例）。 |

⇒ ⚠️ `budget_knob_badge` 的**小数配置值**是**另一条**规则的小数面（生产的 `configured`
是 `number` ⇒ `1.5` 会被 `Number.isSafeInteger` 判为非法 ⇒ 回默认；Rust 侧 `Option<i64>`
表达不了 `1.5`）⇒ 本批**未**改该规则的公开面 ⇒ 仍登记在「已知未覆盖清单」。


## 装置自身的健壮性

- **幂等**：`gen_expected.mjs` 跑两次输出**逐字节相同**（`--check` 可当 CI 门禁）。
  ⇒ 前提：期望值不含「当前时刻」「随机数」⇒ 时刻类用例一律**固定 `now_ms`**。
- **`generator` 标记**带 **Node 版本**与**时区**（便于复现）：

  ```json
  "generator": {
    "script": "crates-ui/apeireth-ui-demo/harness/rules/gen_expected.mjs",
    "node": "v22.23.1",
    "mode": "--experimental-strip-types",
    "tz": "Asia/Shanghai",
    "production": "frontend/companion-desktop/src/lib/{…}"
  }
  ```

  Rust 侧自检**断言标记存在**，且 `generator.tz` 必须等于 Rust 夹具的时区名
  （`Asia/Shanghai`）⇒ 谁改了任一侧的时区 ⇒ 另一侧立刻红。

## 已知未覆盖清单（**诚实边界**）

- **`format_clock_ms` / `format_date_time_ms` 的 ICU 差异**：装置比的是
  **Node 的 ICU**（`zh-CN`）与 Rust 侧**手写**的 +08:00 渲染 ⇒ **ICU 版本差异不覆盖**
  （例如某些 ICU 会给 `下午03:30` 或 `03:30 PM`）。宿主侧（wasm）走
  `BrowserTimeText`（直接调浏览器 `Intl`）⇒ 那才是与生产**真正同源**的一条路。
- **1901 年前的本地时刻**：Asia/Shanghai 历史上是 **LMT +08:05:43**（1901 年前）⇒
  固定 +08:00 的夹具**不覆盖** ⇒ 用例一律用 1901 年之后（实测用例最早到 1969）。
- **`format_date_time_ms` 的「现在」是钉住的**：生产真实运行用的是**真实当前时刻**
  ⇒ 装置比的是「同一时区 + 同一 now」的判定 ⇒ 「now 漂移导致的同日翻转」不覆盖。
- **`budget` 四条的旋钮规格是**夹具**：生产 `effectiveBudget` / `budgetRemainingRows`
  **不接受**外部规格表 ⇒ 它们的规格只能来自生产常量表 `BUDGET_KNOB_SPECS`
  ⇒ 装置用 `knob_rows` 夹具承载，并让生成器**每次核对**它与生产常量表一致
  ⇒ 不一致 ⇒ 退出码 1（不是静默产出错期望值）。
- **`KnobRow` 的非规则字段**：`label` / `env` / `desc` / `hint` / `icon` 是宿主喂的
  **文案与图标** ⇒ 这四条规则**不读**它们 ⇒ 夹具置空 + 用 crate 内既有公开图标常量占位
  ⇒ **零改动 `src/**`** 即可让 `KnobRow` 从 JSON 构造。
- **`budget_knob_badge` 的小数配置值**：生产的 `configured` 是 `number` ⇒ 可为 `1.5`
  （⇒ `Number.isSafeInteger` 判非法 ⇒ 回默认 ⇒ 徽标报 `constant`）；
  Rust 侧是 `Option<i64>` ⇒ **小数无法表达** ⇒ 该形态**未覆盖**（不是漂移，是 API 面不同）。
  ⇒ 本批**只**给 `format_duration` 补了 `f64` 入口（任务点名的缺口）⇒ **未**改本规则的公开面。
- **`format_session_totals` 的 `lastTurnToolCalls`**：已进用例，但该字段**不参与**输出
  （两侧的显示视图都不含它）⇒ 只钉住「不崩」。
- **`merge_session_inbox` 的时刻解析口径**：TS 用 `Date.parse`（接受多种形态），
  Rust 用 `parse_rfc3339_ms`（只收 RFC 3339）⇒ 本批**已把更宽的形态做成用例并按
  `known_drift` 钉住**（「无偏移 ⇒ 差 8 小时」「纯日期 ⇒ Rust 解析失败」两条）。
  ⇒ 仍未覆盖的是**更极端的非 ISO 串**（如 `Jan 2, 2026`）与**数字型 epoch**
  （生产的 `approvalTimeMs` 收 `number`；Rust 的 `created_at` 是 `Option<String>`
  ⇒ **数字 epoch 在 Rust 侧不可表达** ⇒ 属 API 面差，不是漂移）。
- **`short_id` 的代理对切断边界**：生产 `shortId` 用 `String.slice`（按 **UTF-16 码元**切）
  ⇒ 切点落在**非 BMP 字符**中间时留下**孤立代理**（半个 `😀`）；
  Rust 的 `String` **不可能含孤立代理**（且 `num::string_slice` 会**丢弃整个代理对**）
  ⇒ 该形态**在 Rust 侧不可表达** ⇒ **未覆盖**。
  ⇒ 用例只放**完整落在切点内**的非 BMP 字符（`😀abc` + `head=2` ⇒ 两侧同为 `😀…`）⇒ 可比。
- **非有限数（NaN / ±Infinity）**：JSON **无**这些字面量 ⇒ `risk_score_percent`
  （生产 `Number.isFinite` 分支）、`span_duration_ms`（`Math.max(0, NaN)`）的
  **非有限分支未覆盖**；Rust 侧的 `f64` 入口**有**这些分支（`format_duration_f64` 的
  `!is_finite()` ⇒ 「—」）⇒ 生产与 Rust 在该分支**同口径**（但装置表达不出来）。
- **结构化夹具只列「规则会读的字段」**：`span`（`span_id`/`parent_span_id`/`started_at`/
  `ended_at`）、留痕条目（`title`）、守卫事件（`trace_id`/`round`/`timestamp_ms`/
  `decision`/`stage`/`risk_score`）⇒ 其余字段在 Rust 侧置空串 / `None`。
  ⇒ **代价**：若生产规则**将来开始读**别的字段，装置**不会**自动发现 ⇒ 该风险**未覆盖**。
- **`guard_event_row` 的 `stage` 夹具**：生产的 `GuardEvent.stage` 是**三段联合类型**
  ⇒ 夹具只放**合法值** ⇒ `guardStageLabel` 的 **default 分支**由
  `guard_stage_label` 规则**单独**覆盖（那里 `stage` 是自由字符串）。
- 本装置**只比规则输出**，不比对画面（画面由 `harness/governance/` 负责）；
  两侧类型不同 ⇒ 比的是**归一化后的可比 JSON**，不是各自的内部结构。
