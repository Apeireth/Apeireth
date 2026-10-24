# 设计裁定落地批 R1 —— 23 项拍板中「参数与默认值七处」落地对照表 + 测试证据

> **日期**: 2026-10-17
> **来源**: `docs/design/human-side-design-review.md` §10 拍板清单（owner 裁定 = **全按建议**）。
> **范围**: 参数与默认值七处，全部小改 + 测试钉（其余裁定项归口各自施工线）。
> **纪律**: 中性表述；扫描零命中；不写 CHANGELOG；不 commit；校验 `-j 2` 口径；
> 0 触碰 LOCKED 5 项；0 引新外部 dep；0 主动 commit。
> **口径**: 本报告数字均为 2026-10-17 本批实测（命令在文末），不代改
> `docs/03-reference/test-baseline.md` 真源。

---

## 1. 裁定 → 落点对照表

| # | 裁定项 | 裁定值 | 落点（符号级） | 改动 | 旧行为兼容口 |
|---|---|---|---|---|---|
| 1 | ③ 情感残响强度（旧事回涌系数） | **0.2** | `crates/adapters/gateway/src/presence.rs` `EMOTION_RESIDUAL_STRENGTH`（`turn_completed` 的召回命中→pleasure 施加点） | 常量化并由 0.15 改 0.2：每次召回命中 pleasure 回涌 0.2 | 无配置面（常量）；命中量 = 常量 × 召回次数，测试钉 0.2/0.4 两档 |
| 2 | ⑩ 遗忘半衰期（未保护记忆；保护豁免不变） | **90 天** | `crates/engine/memory/src/retention.rs` `BASELINE_HALF_LIFE_HOURS`；`crates/engine/memory/src/persona_layer.rs` `PERSONA_SLOW_HALF_LIFE_HOURS` | 基线 24h → 90 天（2160h）；persona 未保护层 30 天 → 90 天（口径统一，两常量同值钉）；protected 豁免语义 0 改 | 显式 `RetentionPolicy::with_min_strength(s, h)` 仍可覆盖（如旧 24h）；`APEIRETH_TUNE_MEMORY_FADE` 倍率推导不变 |
| 3 | ⑭ 勿扰窗口默认（两处同源） | **23:00-08:00** | `crates/capabilities/tools/src/voice/quiet.rs` `VOICE_QUIET_HOURS_DEFAULT`；`crates/engine/organ/src/care/rules.rs` `CareDiscipline` 缺省窗（`default_quiet_start/end_minutes`） | 两处默认同改 23:00-8:00（原发声面 22:00-06:00、关怀纪律无默认窗）；接缝 `runtime-assembly::voice_quiet_hours_from_care_rules` 语义不变（care_rules 配窗优先、未配回落 env） | 显式 `care_rules.json` 窗口 / `APEIRETH_VOICE_QUIET_HOURS` 照常覆盖（含显式 `off` 关窗、显式 null 无窗）；旧 22-6 窗可显式表达 |
| 4 | ⑬ 关怀频率上限默认 | **4h≤1 次、日≤5 次** | `care/rules.rs` `CareDiscipline{frequency_max=1, frequency_window_hours=4, daily_max=5}`（`daily_max` 为新增字段）；`care/engine.rs` `discipline_hold` ③ 双档闸（4h 滚动窗 + UTC 日档） | 滚动窗口档 2/12h → 1/4h；新增日档 ≤5（同 `CareHold::FrequencyCap` 词表，闭合词表不扩） | 显式配置照常覆盖（2/12h、日 10 均可表达）；旧 `care_rules.json` 缺 `daily_max` 字段 = 缺省 5 |
| 5 | ⑮ 专注抑制默认 | **3 分钟** | `care/rules.rs` `default_focus_gap_minutes` | 15 → 3（连续交互 3 分钟内不打扰） | 显式 `focus_gap_minutes` 照常覆盖（旧 15 可表达） |
| 6 | ⑳ 环路熔断深度默认 | **5 层** | `crates/foundation/orchestration/src/task_queue.rs` `DEFAULT_MAX_CHAIN_DEPTH` | 4 → 5（spawn 链 5 层内放行、第 6 层拒绝即帧+审计） | 显式 `TaskQueueConfig{max_chain_depth}` 照常覆盖（旧 4 可表达） |
| 7a | ⑯ 语音默认排队串行 | **APEIRETH_VOICE_INTERRUPT 缺省 0** | `crates/capabilities/tools/src/voice/playback.rs` `PlaybackPolicy::default` / `from_env` | 缺省档 InterruptPrevious → QueueSerial；`1` 仍 = 打断 | 显式 `APEIRETH_VOICE_INTERRUPT=1` 回到打断档 |
| 7b | ⑯ 队列上限 3 条（满则拒新） | **3 条 + 拒新即帧** | `playback.rs` `SPEECH_QUEUE_MAX` / `TryBeginError::QueueFull` / `PlaybackController::try_reserve_queue`（RAII 队位）/ `begin_speech`；`speak.rs` 错误帧映射 | 排队话帧（等待开播）上限 3 条；第 4 条 = **拒新 + 错误帧**（钉「拒新」不「丢最旧」：在播旧话照旧、账本不造发声） | 打断档无队（不占队位）；队位随取消/收尾归还 |
| 7c | ⑰ 发声范围默认 | **仅关怀/提醒** | `speak.rs` `SpeakScope`（`care`/`chat` 申报）+ `SpeakGate`（`CareOnly` 缺省 / `All`）+ `speak_scope_refusal` + `VoiceSpeakConfig::speak_gate` + `VOICE_SPEAK_SCOPE_ENV` | voice_speak 增 scope 判定：care 提醒帧出声；常规划算对话回复（含未申报，fail-closed 朝安静）拒绝即帧；判定面 = 调用参数（读码定的最简判定面，无其它来源档可用） | `APEIRETH_VOICE_SPEAK_SCOPE=all`（或 `SpeakGate::All`）显式扩全量 = 旧行为 |

同源接缝（第 3 项）：`runtime-assembly/src/canonical/voice_quiet.rs::voice_quiet_hours_from_care_rules`
仍是唯一入口（care_rules 配窗 = 引用之；未落/未配 = 发声面最小 env；坏配置拒开不静默回退），语义 0 改。

## 2. 测试证据（新增钉 ≥10；全部 2026-10-17 实测通过）

| 钉 | 测试（文件 :: 名） | 钉住什么 |
|---|---|---|
| ③ | `crates/adapters/gateway/src/presence.rs::emotional_residual_strength_is_point_two_per_recall_hit` | 残响常量 = 0.2；1 次召回 pleasure +0.2、2 次 +0.4（点值） |
| ⑩ | `crates/engine/memory/src/retention.rs::forget_half_life_defaults_to_ninety_days_and_stays_overridable` | 基线 2160h=90 天；默认策略 90 天；一个半衰期强度 0.5；显式 24h 覆盖（兼容） |
| ⑩ | `crates/engine/memory/src/persona_layer.rs::persona_decay_strength_is_exempt_or_slow`（扩钉） | persona 未保护层 = 90 天；**与 retention 基线同值（口径统一）**；protected 恒 1.0（保护豁免不变） |
| ⑩ | `crates/engine/memory/src/layered_memo/decay.rs::half_life_defaults_to_90_days`（新）；`half_life_24h`（改钉显式覆盖）、`virtual_clock_half_life_without_waiting`、`should_forget_threshold`（随默认口径改写） | 衰减引擎默认 90 天；显式 24h 兼容；虚拟时钟 90 天→0.5、180 天→遗忘 |
| ⑭ | `crates/capabilities/tools/src/voice/quiet.rs::default_window_is_2300_to_0800_per_the_ruling` | 发声面默认窗 = 23:00-08:00（23:00/03:00 抑制、08:00 出窗、12:00 不拦） |
| ⑭ | `crates/engine/runtime-assembly/src/canonical/voice_quiet.rs::care_discipline_default_and_voice_quiet_default_share_one_window` | **两处同源一致性**：care 默认窗与发声面默认窗同值同跨午夜语义，逐分钟判定一致 |
| ⑬⑭⑮ | `crates/engine/organ/tests/care.rs::care_discipline_defaults_pin_the_ruling_caliber` | 缺省打扰纪律全口径：23:00-8:00 / 4h≤1 / 日≤5 / 3 分钟 |
| ⑬ | `care.rs::default_frequency_cap_holds_the_second_care_within_four_hours` | 4h 滚动窗档行为：4 小时内第二发被拦（拦下不丢） |
| ⑬ | `care.rs::default_daily_cap_holds_the_sixth_care_of_a_day` | 日档行为：当日第 6 发被拦（4h 窗空也拦）；UTC 日界重置后照发 |
| ⑮ | `care.rs::default_focus_suppression_is_three_minutes` | 交互后 2 分钟被拦（Focus）、4 分钟恢复 |
| 兼容 | `care.rs::explicit_discipline_values_override_the_ruling_defaults` | 显式 22-6 窗 / 2 次 12h / 日 10 / 15 分钟全可覆盖默认（旧行为兼容） |
| ⑳ | `crates/foundation/orchestration/tests/task_queue.rs::default_chain_depth_fuse_is_five_layers_and_explicit_config_still_wins` | 默认熔断 5 层：1..=5 入队、第 6 层拒绝即帧；显式 1 层照常熔断（兼容） |
| ⑯ | `crates/capabilities/tools/src/voice/playback.rs::default_policy_is_queue_serial_and_explicit_knob_still_picks_interrupt` | 缺省 QueueSerial；parse("0"/"1") 语义；坏值拒解析；SPEECH_QUEUE_MAX=3 |
| ⑯ | `playback.rs::queue_cap_refuses_the_overflow_frame_and_keeps_the_oldest` | 队满拒新（第 4 条拿不到队位）；在播旧话照旧（不丢最旧）；队位归还恢复 |
| ⑯ | `crates/capabilities/tools/src/voice/speak.rs::full_speech_queue_refuses_the_new_frame_as_an_error_frame` | 队满在工具面 = 错误帧「话帧队列已满」；溢出话帧不开播 |
| ⑰ | `speak.rs::speak_scope_gate_admits_care_frames_only_by_default` | 缺省仅 care 帧出声；chat 帧/未申报拒绝即帧且引擎零调用 |
| ⑰ | `speak.rs::explicit_all_scope_restores_full_speaking` | 显式 All 档全量出声（旧行为兼容） |
| ⑰ | `speak.rs::speak_scope_knob_parses_care_or_all_and_refuses_garbage` | 旋钮解析 fail-closed；纯判定面 |
| ⑰ | `crates/capabilities/tools/tests/voice_speak.rs::scope_field_gates_the_frame_kind_on_the_tool_face` | 工具面 JSON `scope` 字段端到端：care 出声 / chat 拒 / 未申报拒；显式 all 兼容 |

## 3. 既有测试的必要改写（默认值变更波及；逐条登记）

| 测试/夹具 | 改写 | 理由 |
|---|---|---|
| `care.rs` 9 处夜间时点夹具 `CareDiscipline::default()` → `unwindowed()`（显式关窗） | 夹具隔离 | 新默认勿扰窗 23:00-8:00 会先拦下夜间到点；这些测试盯触发器/频率/专注/预算，不是勿扰窗 |
| `runtime-assembly/tests/care_output_face.rs` 竖切夹具补显式关窗 | 夹具隔离 | 同上（到点 01:00） |
| `runtime-assembly/src/canonical/voice_quiet.rs::missing_or_unwindowed_care_rules_fall_back_to_the_minimal_env` | 兜底断言字面值 → 常量；unwindowed 夹具显式 null 窗 | 默认窗改 23:00-8:00；「未配窗 → env 兜底」支路前提显式化 |
| `layered_memo/decay.rs` 三处（`half_life_24h`/`virtual_clock_half_life_without_waiting`/`should_forget_threshold`） | 断言改钉 90 天口径；`half_life_24h` 改钉显式 24h 覆盖 | 原钉默认 24h |
| `tools/tests/voice_speak.rs`、`speak.rs` 内测试参数申报 `scope: "care"`（`args()`/`params()` 夹具） | 夹具数据 | 发声范围缺省档生效；机械面用例以关怀帧申报（范围自身另有专钉） |
| `tests/voice_speak.rs::interrupt_policy_stops_the_previous_utterance` 注释 | 「缺省档」→「显式打断档」 | 缺省档已改排队串行（注释随实况） |

## 4. 校验记录（2026-10-17，工具链 rustc 1.97.1 x86_64-pc-windows-msvc；口径 `-j 2`）

| 命令 | 结果 |
|---|---|
| `cargo test -p apeireth-organ --test care --locked -j 2` | 23 passed / 0 failed |
| `cargo test -p apeireth-memory --all-targets --locked -j 2 --no-fail-fast` | 全绿（lib 822 passed + 各集成目标 0 failed） |
| `cargo test -p apeireth-orchestration --all-targets -p apeireth-organ --locked -j 2 --no-fail-fast` | 全绿（orchestration lib 354 / task_queue 16 / organ lib 206 + 各目标 0 failed） |
| `cargo test -p apeireth-tools-canonical --lib --test voice_speak --locked -j 2` | 416 passed + 16 passed（2 ignored 真机位）/ 0 failed |
| `cargo test -p apeireth-gateway --lib --locked -j 2` | 101 passed / 0 failed |
| `cargo test -p apeireth-runtime-assembly --lib --test care_output_face --locked -j 2 --no-fail-fast` | 132 + 6 passed / 0 failed（含两处同源接缝钉） |
| `cargo test -p apeireth-cli --all-targets --locked -j 2 --no-fail-fast` | 全绿 / 0 failed |
| `cargo test --workspace --all-targets --locked --no-fail-fast -j 2 --exclude apeireth-cli` | 全绿 / 0 failed（负载高峰窗口期的全量差一包实测） |
| `cargo test --workspace --all-targets --locked --no-fail-fast -j 2`（208 套件口径全量） | **214 套件 / 5746 passed / 0 failed**（零回归；套件数随在途线增长 ≥ 208 基线；逐套件行见 `reports/design-ruling-r1-fulltest-2026-10-17.log`） |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | **0 警告**（Finished） |
| `cargo clippy -p apeireth-orchestration -p apeireth-memory -p apeireth-organ -p apeireth-tools-canonical -p apeireth-gateway --all-targets --locked -j 2 -- -D warnings` | 0 警告（分包先行实测） |
| `scripts/check-neutral-terms.ps1` | OK（74 changed files: added lines all neutral，含本报告） |
| `cargo fmt --check` | **全工作区 0 diff** |

负载 flake 隔离复跑定性 1 例：`cli/tests/self_status_e2e::memory_ledger_counts_track_the_cognitive_store`
在全量负载 + 并发线写盘窗口下失败 1 次，隔离复跑 7/7 过 —— 定性负载 flake，非本批语义（本批未触碰其断言面）。

## 5. 并发在途红（他人在途，标红不代改；本批收尾时全部已收口）

1. `crates/capabilities/tools/tests/exposure_audit.rs` + `crates/adapters/cli/src/exposure.rs`（脱敏审计线在途）：期间编译断点（literal 越界 / `dir` 未定义 / `fs_err` 未接），由归口线自修收口；非本批文件 0 触碰。
2. `apeireth-runtime-assembly` / `apeireth-cli`（性格锁定批在途）：`trait_growth` / `TuningStatus{locked,recent_moves}` / `canonical/trait_report_tool.rs` / `trait_command` 期间编译断点，归口性格锁定批收口；收口后本批全部校验已补齐（§4）。
3. 跨线补位记录：`playback.rs` 的 `Debug for QueueSlot` 由性格锁定批按该文件报备注释补（`expect_err` 编译断点）；本批对 `SpeechTicket` 同型断点改用 `let...else` 不扩生产面。

## 6. 登记不实施（超范围，归口文档线）

1. `docs/04-internal/claims-evidence-matrix.md`（审计产出，历史不追溯改写）内「layered_memo/decay.rs，默认 24h」为审计时点口径，现值已 90 天 —— 建议归口文档线加「历史口径（日期）」标注。
2. `docs/design/human-side-design-review.md` §10 拍板表「你的裁定」列未填（owner 拍板 = 全按建议）—— 归口设计稿线回填。
3. `docs/03-reference/test-baseline.md` 数字不代改（真源归口）；本批实测值如 §4。
