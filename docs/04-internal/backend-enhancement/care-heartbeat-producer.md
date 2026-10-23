# V3-a 收尾 · 常驻心跳关怀生产者 — 交付说明（生产者时序图 + 测试证据）

> `Status: ✅ 本批交付（2026-10-17 施工收卷）`
> 承接 [V3a-dual-engine-care.md](V3a-dual-engine-care.md) §7 0 装诚实点名的最后一针:
> 关怀内核已备、缺定时生产者 —— 本批把常驻心跳生产者接到
> `canonical::heartbeat` 调度缝 + runtime-assembly 装配上, 让「它会想起你」的
> 生产者面真的自动发生 (开关 `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸)。
> 战场: `crates/engine/runtime`（heartbeat 调度缝）+ `crates/engine/runtime-assembly`
> 装配 + gateway presence 闸（只读消费, 本批 0 改 presence 本体, 见 §6）。

---

## 1. 生产者时序图

```text
常驻进程 (开关双闸齐开: APEIRETH_ENABLE_CARE=1 且 APEIRETH_ENABLE_CARE_HEARTBEAT=1)
│
│  CareHeartbeatProducer::start_care(config, CareEngine, now_ms, dispatch)
│    └─ HeartbeatProducer::spawn ── 起常驻线程 apeireth-heartbeat-poll
│       (间隔可配 APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS, 缺省 300s = 5 分钟)
▼
┌─────────────────────────── 心跳线程主循环 ───────────────────────────┐
│ wait(Condvar, poll_interval) ── stop ──> 退出 (join 收线程, 不僵尸)  │
│        │ 到点 (首拍即刻, 之后每 interval 一拍)                      │
│        ▼                                                            │
│ canonical::heartbeat 调度缝                                         │
│   HeartbeatScheduler::schedule_task(Timer, priority=20 闲散事件级)  │
│   HeartbeatScheduler::poll_next_task(now)                           │
│     ├─ None = 心流锁拦下 (深度工作不插话) → 短拍重试, 候选不丢      │
│     └─ Some(task) → 进入 poll                                       │
│        ▼                                                            │
│ CareEngine::poll  ← 纯规则轮询 (缺省判定档零模型调用)               │
│   ① 规则引擎: 四类触发器到点 → CareCandidate (素材 + 时机)          │
│      ├─ 无到点候选 → Idle: 零动作零计费                             │
│      ▼                                                              │
│   ② 打扰纪律四件 (顺序固定):                                       │
│      勿扰窗口 → 专注抑制 → 频率上限 → 关怀预算 (同账本接缝)        │
│      拦下 → Held: 候选不丢 (不写台账, 不消耗发起锚点)               │
│      ▼                                                              │
│   ③ 发起闸 (try_take_initiative 同源, 防唠叨预算与其他 initiative   │
│      生产者同一闸源): 拒绝 → Held(initiative_gate): 候选不丢        │
│      ▼                                                              │
│   ④ 发起意愿判定 (显式判定面): RuleFallback 纯规则 / ModelContext   │
│      模型情境判定 (判定失败回退规则, 不漏发不乱发)                  │
│      ├─ 说不 → Declined: 当次消费, 台账留痕, 零回合                 │
│      └─ 该说 → 发起留痕 + 台账 → CareInitiative                     │
│        ▼                                                            │
│ CareInitiative::to_turn_input() → HeartbeatPollOutcome::Initiate     │
│        ▼                                                            │
│ HeartbeatDispatch::dispatch (常驻宿主接 Runtime::execute)           │
│   → 标准回合 (零新消息通道) → 关怀消息进会话流 (普通转写)           │
│   → 计费落会话配额账本 (三维烧钱闸同一本账, 照常计费)               │
└─────────────────────────────────────────────────────────────────────┘
自述关怀节 (tool.self_status `care`):
  heartbeat_next_poll_at_ms  = 下次 poll 时刻 (时钟 + 间隔, 现值如实)
  heartbeat_last_hit_at_ms   = 最近心跳命中 (关怀发起) 时刻; 尚无 = 显式 null
```

停机 (优雅): `CareHeartbeatProducer::shutdown()` → 置停 + Condvar 唤醒 +
`JoinHandle::join` 收线程 (幂等) —— 停机后零 poll 零发起 (不僵尸), 线程句柄
回收不外泄 (`thread_reaped()` 是无泄漏断言口)。

## 2. 开关与配置（注册即闸, 默认关 fail-closed）

| 旋钮 | 语义 | 口径 |
|---|---|---|
| `APEIRETH_ENABLE_CARE_HEARTBEAT` | 心跳关怀总闸 | **注册即闸, 默认关 (fail-closed)**: 只认 `1`; 垃圾值/缺省 = 关 |
| `APEIRETH_ENABLE_CARE` | 上游关怀总闸 (V3-a) | 心跳闸**依赖**它: 两闸同时 `1` 才起线程 (依赖即闸) |
| `APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS` | poll 间隔 | 可配正整数秒, 缺省 300 (5 分钟); **非法即拒** (0/非整数 = 拒绝即帧) |

- 开关关 = `CareHeartbeatProducer::start` 返回 `None`, 不建线程不 poll
  (显式不注册, 不是假运行); 自述关怀节心跳两列显式 null (不冒充在跑)。
- 生效值解析走纯函数 `CareHeartbeatConfig::from_env_value` (测试可注入,
  不靠全局 env 竞态)。

## 3. 计费诚实

- **关怀回合照常计费** (V3-a 已钉, 本批在心跳路径复证): 心跳发起的标准回合
  计量落会话配额账本 (usage 10+5 = 15 tokens, 与普通回合同一本账, 无免费通道)。
- **心跳 poll 本身零模型调用 (纯规则轮询)**: 无到点候选/纪律拦下/发起闸拦下
  都不触模型; 缺省判定档 (`RuleFallbackJudge`) 连命中判定也是纯规则 ——
  模型调用只发生在标准回合里 (测试钉: provider 调用数 == 标准回合数)。
- 注入模型情境判定档 (`ModelContextJudge`) 时, 模型调用发生在「候选到点」的
  判定面 (引擎二), 不在轮询 —— 如实注明, 不含糊成「poll 恒零模型」。

## 4. 停机清理与自述心跳态

- 心跳线程随优雅停机收 (join 回收, 不僵尸): 停机后 poll/发起计数冻结
  (无幽灵 tick), `shutdown()` 幂等, `thread_reaped()` 供线程句柄无泄漏断言。
- `tool.self_status` 关怀节扩心跳态两列 (自述投影的唯一实现面):
  - `heartbeat_next_poll_at_ms` — 下次 poll 时刻 (真值 = 时钟 + 间隔);
  - `heartbeat_last_hit_at_ms` — 最近心跳命中 (关怀发起) 时刻。
  两列恒在场; 未接线/未运行 = 显式 null (0 装诚实, 不冒充在跑)。

## 5. 测试证据（2026-10-17 实测, `-j 2` 口径, rustc/cargo 1.97.1）

本批新增 **17 个测试函数** (≥10 要求):

| # | 用例 | 钉的行为 | 位置 |
|---|---|---|---|
| 1 | `a_heartbeat_hit_enters_the_conversation_flow_via_a_standard_turn` | 心跳命中 → 标准回合进会话流 (真 Runtime: 回合输入 + 生成消息都进普通转写) | runtime-assembly `tests/care_heartbeat_producer.rs` |
| 2 | `the_disturbance_discipline_holds_on_the_heartbeat_path` | 打扰纪律在心跳路径生效 (勿扰窗口拦下 = 零回合零模型调用, 候选不丢不写台账) | 同上 |
| 3 | `no_due_rule_means_zero_action_and_zero_billing` | 无规则命中零动作零计费 (零回合零模型调用零会话零账本) | 同上 |
| 4 | `the_switch_is_fail_closed_and_depends_on_the_care_switch` | 开关 fail-closed (默认关/垃圾值关/依赖 `APEIRETH_ENABLE_CARE`/关 = 不注册不 poll) + 间隔可配 + 非法即拒 | 同上 |
| 5 | `shutdown_reaps_the_heartbeat_thread_without_leaking` | 停机清理: join 收线程、线程句柄无泄漏、停机后零 poll (不僵尸)、幂等 | 同上 |
| 6 | `the_poll_interval_is_configurable_and_reported_honestly` | poll 间隔可配 (200ms 档) + 自述「下次 poll 时刻」= 时钟 + 间隔 | 同上 |
| 7 | `the_initiative_gate_gates_the_initiation_without_losing_the_candidate` | 发起闸 (try_take_initiative 同源) 在心跳路径生效: 拒绝 = 拦下不丢候选零计费, 放行才发起并消耗发起锚点 | 同上 |
| 8 | `a_heartbeat_care_turn_is_billed_on_the_shared_session_ledger` | 关怀回合照常计费 (会话配额账本 15 tokens) + poll 零模型调用 | 同上 |
| 9 | `self_status_care_section_reports_the_heartbeat_state_honestly` | 自述关怀节心跳两列真值 / 尚无命中 = null / 未接线 = 两列 null | 同上 |
| 10 | `the_poll_interval_defaults_to_five_minutes_and_is_configurable` | 生产者缺省 5 分钟 + 可配 + 下次 poll 时刻真值 | runtime `canonical::heartbeat` 单测 |
| 11 | `a_hit_dispatches_the_standard_turn_input_and_records_the_hit` | 命中 → 发起标准回合输入 + 最近心跳命中留痕 | 同上 |
| 12 | `zero_action_polls_never_dispatch` | 零动作 poll = 零发起零计费 | 同上 |
| 13 | `shutdown_reaps_the_thread_handle_without_leaking_and_stops_ghost_ticks` | 8 线程批量停机: 句柄回收无泄漏、不僵尸、幂等 | 同上 |
| 14 | `poll_failures_are_reported_honestly_and_do_not_kill_the_loop` | poll 失败即帧式留痕 (真因不吞) + 循环不倒 | 同上 |
| 15 | `the_flow_lock_at_the_scheduling_seam_holds_the_poll_until_released` | 调度缝心流锁拦下闲散 poll (深度工作不插话), 放锁短拍重试恢复 | 同上 |
| 16 | `the_initiative_gate_holds_without_losing_the_candidate_or_recording_it` | 发起闸拦下 = Held: 零判定零记账 (候选不丢); 放行才发起 | organ `care::engine` 单测 |
| 17 | `the_initiative_gate_is_only_consulted_after_the_discipline_passes` | 发起闸只在纪律放行后被问询 (空转 poll 零消耗防唠叨预算) | 同上 |

- 既有套件回归 (本批实测值, 2026-10-17, `-j 2` 口径):
  - `cargo test -p apeireth-organ` = 全绿 (含 `tests/care.rs` 23 例零回归);
  - `cargo test -p apeireth-tools-canonical` 自述套件 34 例全绿 (关怀节心跳两列
    演进断言随行注明理由);
  - `cargo test -p apeireth-runtime-assembly --test care_output_face --test
    capability_switch_chains` = 16 例全绿 (V3-a 原套件 + 开关链零回归);
  - `cargo test -p apeireth-runtime --lib canonical::heartbeat` = 7 例全绿
    (含既有调度器 1 例)。
- 全仓口径 (2026-10-17 实测, `-j 2` 口径):
  `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` =
  **223 套件 / 5890 passed / 0 failed / 20 ignored, exit 0**（收卷归档跑,
  日志 `artifacts/care-heartbeat-producer-workspace-test.log`; 含本批新增
  `runtime-assembly/tests/care_heartbeat_producer.rs` 套件与 runtime/organ
  新增单测)。既有 221 套件零回归 (实测 223 = 221 既有 + 本批 1 + 并行线 1)。
- `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` 与
  `cargo fmt --all -- --check` 的收卷状态见 §6 (施工中途两线在途红已由归口线
  自检收敛, 归属与快照如实登记)。
- `scripts/check-neutral-terms.ps1` = OK 零命中 (34 个变更文件全中性)。
- 测试基线真源见 [../../03-reference/test-baseline.md](../../03-reference/test-baseline.md),
  本批不代改真源, 只报实测值。

## 6. 跨范围补位与他人在途（如实报备）

1. `crates/engine/organ/src/care/engine.rs` (最小跨范围补位, 报备): 发起闸
   同源接缝 —— `CareInitiativeGate` 端口 (`try_take_initiative` 合同) +
   `FnInitiativeGate` (组装根接 presence 闸的闭包桥) + `AlwaysTakeInitiative`
   (内核单用缺省档, 文档注明非生产档) + `CareHold::InitiativeGate` 拦下原因 +
   `CareEngine::with_initiative_gate`, poll 流程插入点 = 纪律四件后、判定前
   (拒绝 = Held 候选不丢, 且零判定零记账 —— 保住「规则兜底保证不漏」与
   「poll 零模型调用」两个承诺)。理由: 发起闸若放 poll 之后, 台账/发起锚点
   已写, 闸拒绝即「说发了却没发」(自述说谎 + 频率账超记 + 候选丢失);
   内核加显式缝是零债解。既有 18+5 例 care 套件零回归 (缺省档行为不变)。
2. `crates/capabilities/tools/src/self_status.rs` (最小跨范围补位, 报备,
   V3-a §5.1 同先例): `CareStatus` + 心跳两字段 + 渲染两键 (Some/None 两臂)
   + `unavailable` 同步 + 固定 fixture 两处随行。同文件的
   `memory_ledger`/`persona` 面为并行线在途, 本批对其语义 0 改。
3. **本批 0 改**: gateway `presence.rs` (presence 闸只读消费, 本体未动)、
   cli、frontend、CHANGELOG、docs/design、memory、governance、tui、task。
4. **他人在途红 (标红不代改, 归口线自检收口)**: 施工期间 V4-c 检索调权线
   (`retrieval_tuning*`) 与用户域线 (gateway `domain_*`) 并发在途。收卷快照
   (2026-10-17, `-j 2`):
   - `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`
     = exit 101, **红点恰 1 处**在 `runtime-assembly/src/canonical/
     retrieval_tuning_wire.rs:227` (`manual_let_else`, V4-c 在途文件, 按
     「他人在途红标红不代改」不代改); 本批自有面 (`heartbeat.rs` /
     `care_heartbeat.rs` / `care/engine.rs` / `care_heartbeat_producer.rs` /
     关怀节投影) **0 警告** (同编译单元实测);
   - `cargo fmt --all -- --check` = exit 1, 红点全在他人在途文件区
     (`retrieval_tuning*` / `memory/context_compiler.rs` / `memory/coordinator.rs` /
     `capability_switch_chains.rs` / `tools/self_status.rs` 的 memory_ledger
     新测两处 / gateway `canonical_entry.rs`+`domain_context.rs`+
     `tests/domain_faces_open.rs` / `runtime-assembly/cognitive.rs` 两处);
     本批自有面 **0 diff** (纯 rustfmt 收过);
   - 测试目标编译红 (施工中途 `roster_from_config` 加参后部分既有调用点未随齐)
     已由 V4-c 线自检收敛 —— 收卷测试 run **全目标编译过、全绿 exit 0** (§5)。

## 7. 0 装诚实（接线点清单）

- **已接线 (本批交付)**: 常驻心跳生产者内核 (`HeartbeatProducer`, 调度缝 +
  线程生命周期) + 装配面 (`CareHeartbeatProducer` / `CareHeartbeatConfig` /
  `CareHeartbeatPoll`) + 开关 (注册即闸 fail-closed) + 计费诚实 + 停机清理 +
  自述关怀节心跳两列 + 发起闸同源端口 (organ) + 17 例测试证据。
- **未接线 → 已接线 (2026-10-17 常驻宿主接线微批随行, 归口常驻宿主线)**: 常驻
  宿主的**启动注入点**已按本清单接齐 —— (a) gateway `presence.rs` 给
  `PresenceService::try_take_initiative` 装配级读取口 (presence 本体只加不改,
  同一份防唠叨预算同源闸); (b) 宿主 (cli 组合根 `care_heartbeat` 模块 +
  `dispatch_gateway_serve_with`) 把该闸经 `FnInitiativeGate` 注入 `CareEngine`、
  把 `dispatch` 接到 `Runtime::execute`、并把心跳态探测口
  (`care_status_probe_with_heartbeat`) 注进自述关怀节 —— **双闸齐开
  (CARE + CARE_HEARTBEAT) 才起**, 自述心跳两列已从显式 null 转真值 (未起/
  停机仍显式 null, 不冒充在跑)。原「未接线」清单为 2026-10-17 施工收卷时点
  的 0 装快照 (历史口径保留), 其 (a)(b) 两项现已接齐, 现状以本随行注记为准。
