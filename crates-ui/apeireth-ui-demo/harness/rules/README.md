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
| `find_capability` | 命中 ⇒ `{found, id, supported, available, reason, aliasOf}`；未命中 ⇒ `{found:false, 其余全 null}`；只看**命中的那一条**声明。 |
| `capability_available` | 布尔原样。 |
| `capability_unavailable_reason` | 字符串或 `null`（逐码点相等）。 |
| `format_duration` | 字符串原样（含「—」占位与 `m` / `s` 后缀）。 |
| `format_session_totals` | 五字段显示串对象 `{tokens, cache, turns, duration, toolCalls}`（逐码点相等）。 |

> 同一张表也写在 `cases.json` 的 `normalization` 字段里 ⇒ Rust 侧用
> `NORMALIZATIONS` 常量表**双向校验**（改了文档不改适配器 ⇒ 测试变红）。

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

**当前登记的漂移（1 条）**：`is_approval_expired` / `expires-null` ——
生产 `isApprovalExpired` 写的是 `entry.expiresAtMs !== undefined && entry.expiresAtMs <= nowMs`
⇒ 字段为 **`null`** 时 JS 把 `null` 强制为 `0` ⇒ `0 <= nowMs` 为真 ⇒ **TS 判过期**；
Rust 的 `Option<i64>` 判 `None` ⇒ **不判过期** ⇒ **两侧不一致**。
（当前后端契约用 `skip_serializing_if` ⇒ 字段是**省略**而非 `null` ⇒ 属**契约边界**；
本装置把它钉住，等口径改造后按 `known_drift` 的规则更新。）

## 加一条新规则（流程）

1. **`cases.json`**：加一个 rule 对象（`rule` / `ts` / `rust` / `normalization` /
   `args_schema` / `cases`）；每个 case 只写 `id` + `args` + `"parity": "same"`
   （`expected` 留给机器写）。
2. **`gen_expected.mjs`**：在 `buildImpls()` 里加一条 `(args) => 归一化输出` 的 TS 实现
   （**直接调生产函数**，不要重写业务逻辑）。
3. **`rules_parity.rs`**：
   - 在 `eval()` 里加同名分支（JSON ⇒ Rust 类型 ⇒ 调 Rust 函数 ⇒ 归一化 JSON）；
   - 在 `NORMALIZATIONS` 表里加**同一句**归一化口径（否则自检测试会红）；
   - 加一个 `#[test] fn <rule>_matches_production_typescript()`。
4. 跑 ①②③ ⇒ 全绿才收工。
5. **至少 4 个用例**，必须覆盖：**负数 / 恰好等于边界 / 空集合 / 缺省字段 / `null` 字段**。

> ⚠️ 加了规则却忘了写 Rust 适配器 ⇒ 自检测试会红（`规则「x」没有 Rust 侧适配器`）。

## 诚实边界（本版**未**覆盖）

- **`format_clock_ms` / `format_date_time_ms` 暂不做**：生产走
  `toLocaleTimeString('zh-CN')` ⇒ **依赖浏览器本地时区与 ICU 版本**
  ⇒ 引擎侧当前是**固定 UTC** ⇒ 两侧在非 UTC 时区**必然不等** ⇒ 这不是规则漂移，
  是**口径未统一** ⇒ 等时区口径改造完成后再入装置（现由并行施工线处理）。
- **`KnobRow` 系列的 4 条**（`resolve_budget_limit_input` / `effective_budget` /
  `budget_knob_badge` / `budget_remaining_rows`）**未做**：Rust 侧的 `KnobRow` 带
  `icon: &'static IconData` 字段 ⇒ JSON 无法直接构造 ⇒ 要做就得改 `src/**` 的公开面
  （超出本批「最小改动」边界）⇒ **宁少勿假**。
- **`format_session_totals` 的 `lastTurnToolCalls`**：已进用例，但该字段**不参与**输出
  （两侧的显示视图都不含它）⇒ 只钉住「不崩」。
- **`merge_session_inbox` 的时刻解析口径**：TS 用 `Date.parse`（接受多种形态），
  Rust 用 `parse_rfc3339_ms`（只收 RFC 3339）⇒ 本装置的用例**只用 RFC 3339**
  ⇒ 更宽的输入形态属**已知口径差**，未钉。
- 本装置**只比规则输出**，不比对画面（画面由 `harness/governance/` 负责）；
  两侧类型不同 ⇒ 比的是**归一化后的可比 JSON**，不是各自的内部结构。
