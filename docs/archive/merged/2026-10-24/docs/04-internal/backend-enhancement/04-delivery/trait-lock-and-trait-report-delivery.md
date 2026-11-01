# 性格锁定 + 性格周报（裁定 ④⑤ 落地批 R2）交付记录

> `Status: ✅ 已交付（2026-10-17，裁定落地批 R2）`
> 战场：性格层数据面（「性格养成」自校准账面 `self_tuning` / 成长计既有账）+ `tool.self_status` 调参（性格）节 + 新工具面 `tool.trait_report` + CLI `apeireth trait` 面。
> 禁区 0 触碰：frontend 源码 / CHANGELOG / .github / docs/design / memory persona 面 / voice / care / task / governance（voice 仅一处跨面最小修，见 §4 报备）。
> 中性表述自查：`scripts/check-neutral-terms.ps1` 零命中（本批文与代码同扫）。

---

## 0. 读码勘定（与任务书括号提示的差异，如实报）

- **四根特质滑杆** = `TunableParam::ALL`（遗忘衰减强度 `memory_fade` / 好奇强度
  `curiosity_strength` / 语气情绪饱和度 `tone_saturation` / 整合节奏
  `consolidation_cadence`），即「性格养成」第一铲的四滑杆（设置页「性格与记忆」
  调参面板同名），落位 `crates/foundation/orchestration/src/self_tuning.rs`；
- **成长计既有账** = 自校准沉积记录 `TuningRecord{seq, param, previous, next,
  reason, at_epoch_ms}`（落盘 `tuning-log.jsonl`，逐记录信封）——本批**只读复用**，
  **不新造账**；周报的「移动原因（沉积事件 top）」= 记录的 `reason` 归并计数；
- **冷却** = 既有每日漂移上限（`max_daily_drift`）；cognitive 表族里**没有** traits
  表（`persona_profiles.traits_json` 属 memory persona 面 = 禁区，未触碰）。

## 1. 裁定语义（钉死口径）

| 裁定 | 落地语义 |
|---|---|
| ④ 滑杆锁定 | 四杆各可锁，锁定态**持久**（数据目录 `trait-locks.json`，与成长账同目录）。锁定后成长沉积**跳过该杆且留审计痕**（「跳过沉积 + 审计留痕」口径，非「照记不生效」）：跳过 = 零移动，留痕 = 同账一条 `TuningRecord`（`previous == next`，reason 前缀 `已锁定, 跳过沉积`，带原沉积意图原因），不占当日漂移预算 |
| ④ 解锁 | **显式动作**（`apeireth trait unlock <SLIDER>` / `set_trait_lock(..., false)` / 引擎 `unlock_param`），**无自动解锁路径** |
| ④ 显式人动作不受锁约束 | 锁只挡**自动成长沉积**；撤销（`revert`）/ 调参面板保存仍可改值（锁护"被环境推着走"，不夺主人的手） |
| ④ 锁定态读不出 | **fail-closed 全锁定**（宁可不动任何杆，绝不把坏档读成全解锁放行），失败计入 `SelfTuningWire::lock_failures()`（不静默） |
| ⑤ 性格周报 | 近 **7 天**窗（`[now-7d, now]` 两端含；未来时刻不计）：四杆**移动量**（净移动 `Σ(next-previous)` + 实动次数）/ **移动原因 top 5**（仅实际移动的沉积记录按 reason 归并，计数降序 + 同数字典序）/ **锁定态** + 每杆**最近移动**；零移动留痕（上下限/日漂移/锁定跳过）不计移动，锁定跳过单独计数 |

## 2. 接口说明

### 2.1 数据面（`runtime-assembly::canonical::trait_growth`）

| 符号 | 语义 |
|---|---|
| `TraitGrowthPaths { log_path }` | 落位锚：成长账 = `tuning-log.jsonl`；`locks_path()` = 同目录 `trait-locks.json` |
| `load_trait_locks` / `save_trait_locks` / `set_trait_lock` | 锁定态持久档读写 + 显式锁/解锁动作（读-改-写）；缺档 = 全解锁默认态；坏档 `Err` 即帧 |
| `read_growth_records(log_path)` | 成长计既有账只读读面（逐记录信封 + 旧裸体行只读兼容；缺档 = 空账；坏行跳过并计数） |
| `TraitGrowthFacts::read` | 一次现读：记录 + 坏行计数 + 锁定态 + 四杆当前生效值 |
| `trait_growth_probe` / `trait_status_probe` | 工具面来源 / 自述性格节来源（同一真值源，每次现读） |
| `TRAIT_LOCKS_FILE = "trait-locks.json"` | 锁定档文件名 |

引擎语义（`orchestration::self_tuning`）：`TraitLocks`（四布尔，`is_locked/set/
all_locked/any_locked`）+ `SelfTuningEngine::{locks, set_locks, lock_param,
unlock_param}`；`observe` 在目标杆锁定时产出**跳过留痕**记录（`is_trait_locked_skip`
可识别）。纯确定性、无 IO；持久化由接线层负责（`SelfTuningWire` 观察前现读锁定档）。

### 2.2 工具面 `tool.trait_report`（只读档零审批）

- 能力 id `tool.trait_report`，声明名 `trait_report`，**无参数**（未知字段拒绝即帧）；
- 治理档：**只读**（非写/执行能力，`read_only` 会话预设照常可用）、默认 grant、
  **零审批**（与 `tool.self_status` 同档）；
- 输出帧（形状冻结 [`TRAIT_REPORT_FRAME_FIELDS`]，输出归一合同校验）：

```json
{
  "write_face": "read_only",
  "window_days": 7,
  "window_from_ms": 0, "window_to_ms": 0, "generated_at_ms": 0,
  "sliders": [
    { "param": "memory_fade", "label": "遗忘衰减强度", "value": 1.0,
      "locked": false, "net_delta": 0.25, "move_count": 1, "locked_skipped": 0,
      "last_move": { "param": "memory_fade", "label": "遗忘衰减强度",
                     "at_epoch_ms": 0, "delta": 0.25, "reason": "…" } }
  ],
  "records_in_window": 0, "locked_skipped": 0, "skipped_records": 0,
  "top_reasons": [ { "reason": "…", "count": 1 } ]
}
```

- 错误即帧：成长账/锁定档读不出来 = 失败帧报真因（不编数字）；空账 = 如实空周报。
- 模型描述明示「Read-only … no writes, no execution, no network, no approval」。

### 2.3 `tool.self_status` 调参（性格）节扩（十四节合同不变，节内加字段）

`tuning` 节新增两个恒在场字段（缺源 = 显式 `null` + 同节 `reason`）：

| 字段 | 语义 |
|---|---|
| `locked` | 四杆锁定态 `{memory_fade, curiosity_strength, tone_saturation, consolidation_cadence}` |
| `recent_moves` | 最近移动（新 → 旧，≤4 条）：`{param, label, at_epoch_ms, delta, reason}` |

### 2.4 CLI 面 `apeireth trait`（同内核第二出口；删掉它 agent 侧仍完整可用）

| 命令 | 语义 | 档 |
|---|---|---|
| `apeireth trait report` | 打印近 7 天性格周报（数值 + 原因 + 锁定态 + 账面落位） | 只读 |
| `apeireth trait lock <SLIDER>` | 锁定一根滑杆（持久 + 即时生效） | 显式人动作（写 `trait-locks.json`） |
| `apeireth trait unlock <SLIDER>` | 解锁（无自动解锁路径） | 显式人动作 |

`SLIDER ∈ memory_fade | curiosity_strength | tone_saturation | consolidation_cadence`；
未知动作/滑杆 = 拒绝即帧 + 用法。

## 3. 测试证据（≥8 条逐项对仗）

| # | 证据 | 位置 |
|---|---|---|
| 1 | 锁定后杆值不变 + 沉积跳过 + 审计留痕（reason 带原意图、留痕不占漂移预算） | `orchestration/src/self_tuning.rs::locked_slider_skips_deposit_and_leaves_audit_trace` |
| 2 | 解锁恢复移动 + 只锁一根不误伤别杆 | `orchestration/src/self_tuning.rs::unlock_is_explicit_and_only_the_locked_slider_is_held` |
| 3 | 周报数值正确（7 天窗两端含 / 窗外与未来剔除 / 移动量 / 零移动留痕不计） | `orchestration/src/trait_report.rs::weekly_report_seven_day_window_and_movement_amounts` |
| 4 | 移动原因 top（只数实动沉积、计数降序 + 同数字典序、5 封顶、留痕不混入） | `orchestration/src/trait_report.rs::weekly_report_top_reasons_rank_moved_deposits_only` |
| 5 | 最近移动（新 → 旧 / 限条 / 留痕与未来不计）+ 空账如实空周报 | `orchestration/src/trait_report.rs::{recent_moves_are_newest_first_limited_and_skip_untouched, empty_ledger_reports_zeros_not_invented_numbers}` |
| 6 | 锁定态持久（锁/解锁落盘重读保持）+ 坏档即帧；成长账缺档 = 空账、锁定留痕同账可查；性格节投影 | `runtime-assembly/src/canonical/trait_growth.rs` 单测 ×3 |
| 7 | 周报帧数值从账面真值投影（移动量/原因/锁定态/跳过计数） | `runtime-assembly/src/canonical/trait_report_tool.rs::frame_reports_movement_reasons_and_lock_state_from_the_ledger` |
| 8 | 只读档（无参数拒绝即帧、不落任何文件、声明面 Read-only/no approval） | `runtime-assembly/src/canonical/trait_report_tool.rs::{read_only_face_refuses_parameters_and_never_writes, declaration_is_read_only_zero_approval}` |
| 9 | 装配级：锁定跳过 + 留痕落账 → 显式解锁恢复 → 坏档 fail-closed 不动杆 + 计数不静默 | `runtime-assembly/tests/trait_lock_and_report.rs::locked_slider_skips_the_deposit_leaves_an_audit_row_and_unlock_resumes` |
| 10 | 生产面周报数值 + 注册即闸 + 名册如实（开 true / 关 false） | `runtime-assembly/tests/trait_lock_and_report.rs::trait_report_numbers_are_correct_on_the_production_face` |
| 11 | self_status 性格节锁定态 + 最近移动（同账）；缺源显式 null + 原因 | `runtime-assembly/tests/trait_lock_and_report.rs::self_status_tuning_section_carries_lock_state_and_recent_moves` |
| 12 | 只读能力判定（`read_only` 预设放行面） | `runtime-assembly/tests/trait_lock_and_report.rs::trait_report_is_a_read_only_capability` |
| 13 | CLI 锁定/解锁显式动作持久 + 坏参数拒绝即帧 | `adapters/cli/tests/trait_cli.rs::trait_lock_and_unlock_are_explicit_and_persisted` |
| 14 | CLI 周报文本（7 天窗 / 移动量 / 原因 / 锁定态 / 跳过计数） | `adapters/cli/tests/trait_cli.rs::trait_report_prints_the_seven_day_window_from_the_growth_ledger` |
| 15 | **只读档零审批**（默认 grant + 零审批 + 生产面真调用） | `adapters/cli/tests/trait_cli.rs::trait_report_is_granted_by_default_with_zero_approval` |
| 16 | 文档随行断言（user-manual 写明工具/命令面/锁定档/档位） | `adapters/cli/tests/trait_cli.rs::user_manual_documents_the_trait_lock_and_report_faces` |
| 17 | self_status 节内真值/缺源两态（工具层） | `capabilities/tools/src/self_status.rs::{tuning_section_reports_lock_state_and_recent_moves, missing_trait_status_is_explicit_null_with_a_reason}` |

既有 208 套件零回归：全量 `cargo test --workspace --all-targets --locked
--no-fail-fast -j 2` 见 §5 门禁实录（只增例数，未改既有断言语义）。

## 4. 跨面最小修报备（voice 面，验收必经）

进场发现 **HEAD 现成编译红**（非本批引入）：`crates/capabilities/tools/src/voice/
playback.rs` 单测用 `.expect_err()` 需要 `Debug` 上界，而 `QueueSlot` /
`SpeechTicket` 未实现 `Debug` → `apeireth-tools-canonical` lib 测试目标编译不过
（E0277），全仓门禁无法过。按「验收必经但不在战场内的面 = 最小改动 + 报备 +
理由」补**纯展示 `impl Debug`** 两处（零语义变化）；已报备 voice 归口线自检收口。
若归口线有在途语义改动，可无冲突覆盖这两处。

## 5. 门禁实录（2026-10-17，本批收尾全量实跑）

| 门 | 命令 | 结果 |
|---|---|---|
| 全量测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | **214 套件 / 5747 passed / 0 failed / 20 ignored**（日志 `artifacts/trait-lock-r2-workspace-test.log`；含同树并行在途批增量） |
| clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | **0 警告**（exit 0） |
| fmt | `cargo fmt --all -- --check` | **0 diff**（exit 0） |
| 中性词扫描 | `scripts/check-neutral-terms.ps1` | **OK 零命中**（76 个变更文件 added lines 全中性） |

> 数字口径注记：本表为**本批实测值 + 日期**（工作树含同树并行批的在途增量，套件/例数
> 随之增长属正常）；对外引用以 `docs/03-reference/test-baseline.md` 现行口径为准，
> 本批不代改真源。既有 208 套件**零回归**：全量 0 failed，本批新增 **26 例 / 2 套件**
> （orchestration +7 / tools +2 / runtime-assembly +10 / cli +7），未改既有断言语义。
