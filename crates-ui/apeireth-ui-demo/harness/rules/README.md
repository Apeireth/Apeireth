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
| 归一化 | 两侧类型不同 ⇒ 各转成**可比的 JSON** 再比；**不比**额外字段、**不比**字段顺序 |
| 布尔 / 字符串 | 原样（字符串逐码点相等） |
| 计数 | 摊平成 `{all, open, expired}` |
| 条目数组 | `{length, items:[…]}`；每项取 `approvalId / session / sessionTitle / createdAtMs / expiresAtMs`；**缺失与 `null` 一律归一为 `null`**；顺序敏感 |
| 已知漂移 | 用例标 `"parity": "known_drift"` ⇒ 钉住**当前**漂移形态（见下） |

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

**当前登记的漂移（2 条）**：

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

2. **`gen_expected.mjs`**：在 `buildImpls()` 里加一条 `(args) => 归一化输出` 的 TS 实现
   （**直接调生产函数**，不要重写业务逻辑）。若生产函数不接受外部规格表（如
   `effectiveBudget` 内部写死 `BUDGET_KNOB_SPECS`），就把规格做成**用例夹具**并在
   生成器里**核对它与生产常量表一致**（不一致 ⇒ 退出码 1）。

3. **`rules_parity.rs`**：
   - 在 `eval()` 里加同名分支（JSON ⇒ Rust 类型 ⇒ 调 Rust 函数 ⇒ 归一化 JSON）；
   - 在 `NORMALIZATIONS` 表里加**同一句**归一化口径（否则自检测试会红）；
   - 加一个 `#[test] fn <rule>_matches_production_typescript()`（体是 `assert_rule("<rule>")`）。

4. 跑 ①②③ ⇒ 全绿才收工。

5. **红-绿自证**：把某个 `expected` 临时改错 ⇒ 对应 Rust 测试**必须变红** ⇒ 恢复后回绿。
   （这是「装置真的在比」的证据，不是形式。）

> ⚠️ 加了规则却忘了写 Rust 适配器 ⇒ 自检测试会红（`规则「x」没有 Rust 侧适配器`）。
> ⚠️ `expected` 只能由生成器写 ⇒ 手改必被 `--check` 与 Rust 侧判红。

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
- **`budget_knob_badge` 的小数配置值**：生产的 `configured` 是 `number` ⇒ 可为 `1.5`；
  Rust 侧是 `Option<i64>` ⇒ **小数无法表达** ⇒ 该形态**未覆盖**（不是漂移，是 API 面不同）。
- **`format_session_totals` 的 `lastTurnToolCalls`**：已进用例，但该字段**不参与**输出
  （两侧的显示视图都不含它）⇒ 只钉住「不崩」。
- **`merge_session_inbox` 的时刻解析口径**：TS 用 `Date.parse`（接受多种形态），
  Rust 用 `parse_rfc3339_ms`（只收 RFC 3339）⇒ 本装置的用例**只用 RFC 3339**
  ⇒ 更宽的输入形态属**已知口径差**，未钉。
- 本装置**只比规则输出**，不比对画面（画面由 `harness/governance/` 负责）；
  两侧类型不同 ⇒ 比的是**归一化后的可比 JSON**，不是各自的内部结构。
