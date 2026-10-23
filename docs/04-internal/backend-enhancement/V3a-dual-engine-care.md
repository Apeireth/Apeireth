# V3-a 双引擎主动关怀 — 交付说明（架构图 + 触发器词表 + 测试证据）

> `Status: ✅ 本批交付（2026-10-16 立项施工 / 2026-10-17 实测收卷）`
> 机制卡来源：`docs/04-internal/philosophy-study/01-companion-family.md` §4「双引擎关怀」卡
> —— 规则式时间关怀（作息/纪念日/久未联系）+ 模型式情境关怀**并行互兜底**：
> 关怀不能只靠模型"想起来"，**规则兜底保证不漏**。
> 战场：`crates/engine/organ/src/care/`（读码定：与 initiative 语义同 crate，复用
> 公历/安静窗口口径，不新造 workspace 成员）+ `crates/engine/runtime-assembly` 装配 +
> `crates/adapters/cli` 旋钮；`crates/capabilities/tools/src/self_status.rs` 只做关怀节
> 投影类型的最小跨范围补位（报备，见 §5）。

---

## 1. 双引擎架构图

```text
                   care_rules.json (StoredDoc 拒开)
                   ┌──────────────────────────────┐
                   │ rules[]   四类触发器          │
                   │ discipline 打扰纪律三件       │
                   │ fired_at / history 发起留痕   │
                   └──────────────┬───────────────┘
                                  │ 时钟可注入 (冻结时钟可测)
                                  ▼
   引擎一 · 规则引擎 (确定性兜底, 保证不漏)
     inactive ──── N 小时无交互          ┐
     daily_moment 每日 HH:mm             │→ 关怀候选 (素材 + 时机)
     anniversary ─ 每年月日              │   CareCandidate
     interval ──── 每 N 小时            ┘
                                  │
                                  ▼
   打扰纪律 (先立规矩再谈温度, 顺序固定)
     ① 勿扰窗口 ② 专注抑制 ③ 频率上限 ④ 关怀预算 (同账本接缝)
     拦下 = Held (候选不丢, 窗口滑过重试)
                                  │
                                  ▼
   引擎二 · 模型情境引擎 (增色, 显式判定面 —— 沿 initiative 语义)
     候选 + 近期记忆素材 → 结构化判定 { speak, focus, reason }
     ├─ 无模型配置 → RuleFallbackJudge 纯规则照发 (兜底不漏)
     └─ 判定/解析失败 → 回退规则判定 (不漏发不乱发, 真因留痕)
     · 不做隐式生成: 判定只答「该不该说 / 说什么」, 不生成消息正文
                                  │
                                  ▼
   输出面 · 标准回合 (零新消息通道)
     CareInitiative::to_turn_input() → TurnRequest
     → canonical agent loop 生成关怀消息 → 进会话流 (普通转写)
     → IM 推送走既有推送缝
     → 计费落会话配额账本 (三维烧钱闸同一本账, 照常计费; 烧不起即帧)
```

分层：

| 面 | 位置 | 语义 |
|---|---|---|
| 触发器数据面 | `organ/care/rules.rs` | `care_rules.json` StoredDoc 拒开 + 打扰纪律配置 + 发起留痕 |
| 双引擎判定面 | `organ/care/engine.rs` | 规则引擎 + 显式判定面 + 打扰纪律 + 标准回合输入 |
| 工具面 | `organ/care/tool.rs` | `tool.care_schedule`（增删列）+ 治理档钩子（RequireApproval 级） |
| 装配 | `runtime-assembly`（`CareScheduleModule` / `care_status_probe` / 生产配置） | 注册即闸，名册照实 |
| 旋钮 | `cli`（`APEIRETH_ENABLE_CARE`） | 注册 + grant + 审批钩子同源 |
| 自述 | `tools/self_status.rs` `care` 节 | 规则数 / 下次触发 / 最近关怀真值 |

## 2. 触发器词表（闭合，四类）

| 词 (`kind`) | 语义 | 参数 | 到点条件 | 重复抑制 |
|---|---|---|---|---|
| `inactive` | 久未联系 | `hours` (1..=8784) | 距上次交互 N 小时无交互（无交互记录以规则创建时刻为锚；发起后以发起时刻续算） | 发起锚点续算 |
| `daily_moment` | 作息时刻 | `minutes_of_day` (0..=1439, `HH:mm`) | 每日到点；过点 `catchup_minutes` 内补发，超窗当日跳过 | 每日一次（按 UTC 日键去重） |
| `anniversary` | 纪念日 | `month` (1..=12) + `day` (按月历；2-29 只在闰日当天) | 每年月日当天 | 每年一次 |
| `interval` | 间隔循环 | `hours` (1..=8784) | 距上次关怀发起每 N 小时（从未发起以创建时刻为锚） | 发起锚点续算 |

- **词外拒绝**（闭合词表）：`CareTrigger::parse_kind` 未知词拒绝；参数闭合校验
  （非法即拒），装载与写出同口径 —— 坏条目**拒开也拒写**。
- 管理词表（`tool.care_schedule` 动作，闭合）：`care_add` / `care_remove` / `care_list`
  （增删列）。

## 3. 打扰纪律（先立规矩再谈温度，顺序固定）

| # | 纪律 | 面 | 语义 |
|---|---|---|---|
| ① | 勿扰窗口 | `care_rules.json` discipline | `quiet_start_minutes`/`quiet_end_minutes`（跨午夜=起>止，与 initiative 安静窗口同口径）；窗口内拦下，窗口过后补发窗口内仍发 |
| ② | 专注抑制 | 同上 | `focus_gap_minutes`：距上次交互不足该窗口 = 连续交互期，不插话 |
| ③ | 频率上限 | 同上 | 每 `frequency_window_hours` 小时最多 `frequency_max` 次；满了拦下不丢，窗口滑过再发 |
| ④ | 关怀预算 | `CareBudgetGate`（同账本接缝） | 关怀回合**照常计费**：计量落会话配额账本（三维 tokens/tool_steps/cost_micros 同一本账）；账本耗尽即帧（烧钱闸同语义，无免费通道） |

## 4. 治理档（工具面）

- `tool.care_schedule` = **RequireApproval 级**：每次调用停人工审批（与本地审批面板 /
  IM 审批卡同一条审批链），**无自动放行档**；`care_list` 纯读随面同档（双面能力
  宁严勿松，只读自述走 `tool.self_status` 关怀节，零审批）。
- 写面归类：`care_add`/`care_remove` = **改规则 = 写文件（write_intent 同族）**；
  `care_list` = 只读。
- 非法触发器/素材/参数**拒绝即帧**（不进审批），与规则面校验同口径。

## 5. 跨范围补位与他人在途（如实报备）

1. `crates/capabilities/tools/src/self_status.rs`：**最小跨范围补位**（报备）——
   自述快照增加 `care`/`care_reason` 两字段 + `CareStatus` 投影类型 + 渲染节 +
   输出合同 `care` 节 + 固定 fixture 两处字段；同文件的 `backup`/`VoiceStatus`
   面为并行线（语音/备份）在途，本批对其语义 0 改。
2. `crates/engine/organ/src/emergence.rs`：新增 `ymd_from_epoch_ms` 公开推导助手
   （复用既有公历算法给作息/纪念日推算，复用不重复；0 改既有语义）。
3. `crates/engine/organ/Cargo.toml`：新增 3 个 workspace 内部路径依赖
   （`apeireth-protocol`/`apeireth-governance`/`thiserror`）+ 1 个测试依赖
   （`tempfile`，与 capabilities/tools 同版本，0 新外部包进锁文件）；
   `crates/adapters/cli/Cargo.toml` 新增 `apeireth-organ`（治理钩子/时钟接线面）。
4. **他人在途与跨线补位（如实报备）**：本批施工期间语音/备份/任务簿三条线与本批
   并发在途（曾多轮编译红，全部归口原线自检收敛）。本批只做「验收必经」的**机械
   空白补位**并逐处注记，语义 0 改：
   ① `tools/src/backup.rs` 与 `orchestration/tests/task_queue.rs` 各一处
   `manual_let_else` 等价改写（clippy 门禁断点，按 clippy 建议原样改写）；
   ② 任务线停笔未格式化的 6 个文件跑纯 `rustfmt`（0 语义，本批门禁必经）；
   ③ 共享自述结构体（`SelfStatusSnapshot`）的关怀节字段为本批纯叠加新增，
   与备份/语音线各自动土的字段并存互不覆盖。

## 6. 测试证据（2026-10-17 实测，`-j 2` 口径）

- `crates/engine/organ/tests/care.rs` —— **18 例**：四类触发器各一（冻结时钟注入）+
  作息补发窗口 + 双引擎兜底（无模型纯规则照发）+ 模型判定可否决可改向 + 判定失败
  回退真因 + 打扰纪律四件（勿扰/专注/频率/预算）+ StoredDoc 拒开（畸形/串档/
  未来版本/坏条目）+ 自述关怀真值 + 标准回合输入 + 判定合同闭合 + 时钟合同 + 停用规则。
- `crates/engine/organ/src/care/` 单元测试 —— **14 例**：触发器词表闭合/参数校验/
  StoredDoc 往返/重复与表满/纪律配置校验/台账有界/判定解析闭合/模型回退/
  冻结时钟/工具治理档（每动作停审批 + write_intent 同族理由 + 不越权）/工具增删列
  往返/非法参数拒绝即帧/写面归类。
- `crates/engine/runtime-assembly/tests/care_output_face.rs` —— **6 例**：注册即闸 +
  名册照实、自述关怀节真值、拒开显式 null+真因、**关怀消息进会话流**（双引擎判定 →
  标准回合输入 → 真 Runtime 标准回合 → 生成消息进普通转写）、**关怀回合照常计费**
  （会话配额账本计量 + 烧钱闸同语义即帧）、工具面写面归类落盘真值。
- 自述关怀节另有 tools 侧专测 `care_section_reports_rule_count_next_trigger_and_last_care_honestly`
  （真值/缺位显式 null），并入 `self_status` 套件与全库十节合同断言，376 例全绿。
- 既有套件零回归（本批实测值，2026-10-17，`-j 2` 口径）：
  `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` =
  **207 套件 / 5605 passed / 0 failed / 20 ignored，exit 0**（收卷归档跑，
  日志 `artifacts/V3a-dual-engine-care-workspace-test.log`；施工中途快照一轮同为
  202 套件 / 5589 passed / 0 failed / exit 0，套件差为并行线新增测试目标）。
  含本批新增 `organ/tests/care.rs` + `runtime-assembly/tests/care_output_face.rs`
  两套件。测试基线真源见
  [test-baseline.md]../../03-reference/test-baseline.md)，
  本批不代改真源，只报实测值。
- `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` = **0 警告，
  exit 0**；`cargo fmt --all -- --check` = **0 diff，exit 0**（并行线在途期间的
  两处门禁断点按「跨线补位·机械空白」口径补位并注记：`tools/src/backup.rs` 与
  `orchestration/tests/task_queue.rs` 各一处 `manual_let_else` 等价改写，语义 0 改）。
- `scripts/check-neutral-terms.ps1` = 零命中；`scripts/secret-scan.ps1` = 无发现。

## 7. 0 装诚实

> 历史口径（2026-10-17 立）: 本节首条「常驻心跳生产者（定时 poll）未接线」已随
> 后续批交付 —— 见 [care-heartbeat-producer.md](care-heartbeat-producer.md)
> （生产者接缝 + 开关 + 测试证据）; 本节其余条目照旧保留原值。

- **常驻心跳生产者（定时 poll）未接线**：本批交付的是它调用的工具性内核
  （`CareEngine::poll` → 候选 → 判定 → 标准回合输入）+ 工具/自述/治理/旋钮；
  常驻循环归常驻进程线（运行时已有 `canonical::heartbeat` 调度接缝 + gateway
  presence 的 `try_take_initiative` 闸，是 initiative 生产者同一条待接线通道）。
- 交互观察（`observe_interaction`）是进程内值；重启后久未联系类规则以规则创建
  时刻为锚（如实）。自述「下次触发」在交互锚点缺位时同口径退化。
- 模型判定端口（`CareModelPort`）是显式缝：测试以脚本端口钉行为；真 LLM 适配
  可在组装根注入，未注入 = 纯规则档（照发，兜底保证不漏）。
