# 生产接线收口批 · 交付实录（四缝收口: 整编族两旋钮 + 生产情绪源 + 暴露面体检名册）

```
[Document-Meta]
Document:        docs/04-internal/backend-enhancement/production-wiring-closeout-delivery.md
Version:         1.0 (生产接线收口批四缝交付)
Last-Modified:   2026-10-17
Status:          ✅ 已交付（2026-10-17，生产接线收口批）
Author:          生产接线收口批（子代理）
```

> `Status: ✅ 已交付（2026-10-17，生产接线收口批）` ｜ 战场：`crates/adapters/cli/src/lib.rs`（组合根旋钮 + 情绪源绑定位）+ `crates/adapters/gateway/src/presence.rs`（现值只读口两枚）+ `crates/engine/runtime-assembly`（`production.rs` / `tool_modules.rs` / `self_status_source.rs` / `tests/capability_switch_chains.rs`）+ 三个新 CLI 测试目标 + 文档随行五件（能力矩阵 §0.2 / user-manual 旋钮表 / INSTALL 旋钮表 / api 工具表 / 本实录）。
> 来源纪律：中性表述、扫描零命中、不写 CHANGELOG、不 commit、校验 `-j 2`。
> 口径纪律：测试数字报本批实测值 + 日期，不代改真源 `docs/03-reference/test-baseline.md`。

## 0. 一句话

把近几波「已实现未接线」的四个缝**收成真能用的生产接线**（第一律：能力必须真能用；四级口径：本批收口后四缝均为 PRODUCTION WIRED，旋钮默认关项维持 DEFAULT OFF 不写成现状）：两个整编族能力补上 CLI env 旋钮（注册即闸、随整编跑）、发声微调接上**真情绪源**（读 presence/PAD 合成器现值，情绪→语气微调生效、无情绪数据零变化）、暴露面体检补上**工具面名册接线**（`tool.exposure_audit` 注册即闸、只读档）。每缝配齐：旋钮 fail-closed 测试 + 名册行 + self_status 相应行 + 能力矩阵 §0.2 行 + user-manual 旋钮表行。

## 1. 四缝对照表（接线点 → 旋钮 → 名册/self_status 行 → 生效真值）

| 缝 | 原状态（已实现未接线；四级口径: 收口前 = IMPLEMENTED 未 WIRED） | 接线点（本批落位） | 旋钮 | 名册行（`tool.self_status` capabilities） | self_status 相应行 | 生效真值口径 |
|---|---|---|---|---|---|---|
| 1 **persona_consolidation**（V4-a 人格提取步） | 装配面开关可开，CLI **无 env 旋钮**（V4-b 交付 §8 报备「归口 CLI 接线面」） | `cli/lib.rs` 旋钮解析 → `CognitiveModuleConfig.persona_consolidation`；既有挂点 `MemoryWritebackModule::with_persona_layer`（整编触发点）不动 | `APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1`（默认关 fail-closed） | `persona_consolidation`（既有行，值 = 开关 ∧ 整编 ∧ 记忆写入 ∧ 人格层落库面） | 自述人格节 `persona.last_consolidation_at`（V4-a 既有，最后整编真值） | 开 = 提取步**随整编跑**（`APEIRETH_ENABLE_CONSOLIDATION=1` 同开才真跑）；整编关 = 显式不跑，名册照实 false |
| 2 **consolidation_hash_gate**（V4-b 整编散列门） | 同上（V4-b 交付 §10 报备「CLI env 旋钮归口装配线」） | `cli/lib.rs` 旋钮解析 → `CognitiveModuleConfig.consolidation_hash_gate`；既有挂点 `with_ingest_gate`（整编触发点）不动 | `APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE=1`（默认关 fail-closed） | `consolidation_hash_gate`（既有行，值 = 开关 ∧ 整编 ∧ 记忆写入 ∧ 散列门落库面） | 自述记忆节 `memory_ledger.dedup_skipped`（V4-b 既有，去重计数真值） | 同上（随整编跑）；门故障 fail-open 照常整编（既有语义不变） |
| 3 **生产情绪源**（R3 情绪→语气微调） | 微调机制已实现（四级口径: IMPLEMENTED）且测试钉死，**生产情绪源未接线**（`emotion_tune.rs` 头注「组装根尚未注入 EmotionReader」） | `cli/lib.rs` 新增 `VoiceEmotionSource`（绑定位）+ `voice_speak_config_with_emotion_from_env` 注入 `with_emotion_reader`；`gateway/presence.rs` 新增 `PresenceSynthesizer::current_pad` / `PresenceService::current_pad` 现值只读口；`dispatch_gateway_serve_with` 在网关装配处 `bind` | 非旋钮（随发声配置注入） | `voice_emotion_tune`（新行，值 = 发声配置的读取面实际注入事实） | 自述语音节 `voice`（V3-b 既有，账本/最近说话/嗓名） | 有情绪数据 = 按冻结映射表微调（端到端生效）；**无情绪数据 = 原参数零变化**（未绑定/合成器缺席/锁不可用 = `None` = 零微调，不猜情绪不冒充） |
| 4 **tool.exposure_audit 名册**（V5-b 暴露面体检） | 引擎 + 工具类 + CLI `apeireth exposure report` 已交付，**工具面未进生产装配/名册** | `production.rs` 新增 `exposure_audit: Option<ExposureAuditConfig>` 配置槽 + 注册块；`tool_modules.rs` 新增 `ExposureAuditModule`；`cli/lib.rs` 旋钮 + 配置注入 + 授权面 grant | `APEIRETH_ENABLE_EXPOSURE_AUDIT=1`（默认关 fail-closed） | `exposure_audit`（新行，值 = 体检配置槽实际注入） | 名册行即自述行（体检报告本体是工具产出，不进自述节合同——14 节合同不动） | 开 = 注册 + grant 同源（注册即闸）；**只读档** grant 即用零审批（与 `tool.self_status` 同档）；与 CLI 面同一内核两个出口 |

**旋钮语义总则（fail-closed）**：四个值都只有 `=1` 才生效；默认全部关——默认关的能力不是系统现状，不写成现状。两个整编族旋钮**子依赖主**（整编触发点缺席 = 显式不跑，名册照实 false），与 `skill_distill` 依赖 `file_write` 总闸同一家族语义。

## 2. 落点清单（改动/新增文件）

| 文件 | 性质 | 内容 |
|---|---|---|
| `crates/adapters/cli/src/lib.rs` | 改 | 3 个 env 常量（`ENABLE_PERSONA_CONSOLIDATION_ENV` / `ENABLE_CONSOLIDATION_HASH_GATE_ENV` / `ENABLE_EXPOSURE_AUDIT_ENV`）+ 4 个解析函数（`persona_consolidation_enabled_from_env` / `consolidation_hash_gate_enabled_from_env` / `exposure_audit_enabled_from_env` / `exposure_audit_config_from_env`）+ `VoiceEmotionSource`（绑定位: bind / current / reader）+ `voice_speak_config_with_emotion_from_env`（生产面）+ `CognitiveModuleConfig` 三字段接线 + 治理授权面 `tool.exposure_audit` grant（只读档）+ 运行时组装出口 `build_canonical_runtime_with_sessions_and_voice_emotion_from_env`（旧入口委托不变，签名零破坏）+ 网关装配处 `voice_emotion.bind(&state.presence)` + 域伙伴运行时同一情绪源 |
| `crates/adapters/gateway/src/presence.rs` | 改（最小） | 现值只读口两枚：`PresenceSynthesizer::current_pad`（零副作用现值）+ `PresenceService::current_pad`（锁不可用返 `None` = 无情绪数据，fail-safe 朝零微调）；行为面零改动 |
| `crates/engine/runtime-assembly/src/canonical/production.rs` | 改 | `ProductionModulesConfig.exposure_audit` 配置槽（默认 `None`）+ 注册块（注册即闸） |
| `crates/engine/runtime-assembly/src/canonical/tool_modules.rs` | 改 | `ExposureAuditModule`（`module.tool.exposure_audit`） |
| `crates/engine/runtime-assembly/src/canonical/self_status_source.rs` | 改 | 名册两行：`voice_emotion_tune`（读取面注入事实）+ `exposure_audit`（配置槽注入事实） |
| `crates/engine/runtime-assembly/tests/capability_switch_chains.rs` | 改（加一测） | `matrix_production_wiring_rows_match_the_live_roster`：§0.2 文档行 ↔ 名册实况互钉（含「整编关 = 两行照实 false」反向面） |
| `crates/adapters/cli/tests/consolidation_knobs.rs` | 新 | 缝 1/2 旋钮 fail-closed + 随整编跑 + user-manual 文档随行断言（5 测） |
| `crates/adapters/cli/tests/voice_emotion_source.rs` | 新 | 缝 3 真注入 / 现值读取 / 微调端到端 / 无情绪数据零变化（3 测） |
| `crates/adapters/cli/tests/exposure_audit_knob.rs` | 新 | 缝 4 旋钮 fail-closed + 注册即闸 + 只读零审批 + 工具真能跑（3 测） |
| `docs/03-reference/capabilities-matrix.md` | 改 | §0.2 新增/扩行：整编族两旋钮行 / `exposure_audit` 只读档行 / `voice_emotion_tune` 情绪源行 |
| `docs/02-guides/user-manual.md` | 改 | §4 新增「生产接线旋钮表」（四缝旋钮名 + 档位/纪律 + 名册行名） |
| `INSTALL.md` | 改 | 🎛️ 用户旋钮表 +3 行（两旋钮 + 暴露面体检）+ 「生产情绪源（不是旋钮）」注 |
| `docs/03-reference/api.md` | 改 | 工具（默认状态）表 +1 行：`tool.exposure_audit`（注册即闸、只读档零审批、与 CLI 面同内核两出口） |
| `docs/04-internal/backend-enhancement/production-wiring-closeout-delivery.md` | 新 | 本实录（四缝对照表 + 测试证据） |

## 3. 测试证据（新增 12 个测试函数，全部行为钉死）

**缝 1/2（`crates/adapters/cli/tests/consolidation_knobs.rs`，5 个）**——
1. `persona_consolidation_knob_fails_closed_and_wires_with_the_consolidation_trigger`：默认关 = 名册照实 false（显式未接线）；开 + 整编开 = 名册真值 true（接线生效真值）；
2. `persona_consolidation_stays_inert_without_the_consolidation_trigger`：单独开旋钮而整编关 = 名册照实 false（随整编跑，不产生假接线）；
3. `consolidation_hash_gate_knob_fails_closed_and_wires_with_the_consolidation_trigger`：同 1（散列门）；
4. `consolidation_hash_gate_stays_inert_without_the_consolidation_trigger`：同 2（散列门）；
5. `user_manual_documents_the_production_wiring_knobs`：user-manual 旋钮表含四缝旋钮名与名册行名（文档随行断言）。

**缝 3（`crates/adapters/cli/tests/voice_emotion_source.rs`，3 个）**——
6. `production_voice_config_injects_the_presence_pad_reader_face`：组合根真注入读取面；未绑定 = 显式 `None`（无情绪数据）；绑定后读取面读数 = presence/PAD 合成器**现值**（同一真值，情感残响 2 命中 = 愉悦 0.5 → 愉悦高档判档）；
7. `emotion_tune_reaches_the_speech_request_end_to_end`：**微调生效端到端**（真回合过真实事件口 → 合成器 PAD → 读取面 → 冻结映射表 → 引擎请求）：愉悦高档语速 +1、张力高档语速 -1，两档都逐段断言到引擎请求字段；
8. `no_emotion_data_keeps_the_original_parameters_unchanged`：读取面在场但无情绪数据（纯 CLI 形态）= 原参数零变化；绑定后基线中性读数 = 同样零微调（冻结表中性档 0/0）。

**缝 4（`crates/adapters/cli/tests/exposure_audit_knob.rs`，3 个）**——
9. `exposure_audit_knob_fails_closed_and_grants_with_registration`：默认关 = 不注入配置 + 工具不注册 + 治理拒绝即帧 + 名册照实 false；开 = 注册 + grant + 名册真值 true（注册即闸同源）；
10. `exposure_audit_is_read_only_with_zero_approval`：只读档判定 = Allow 且**非** RequireApproval（与 `tool.self_status` 同档）；
11. `exposure_audit_tool_runs_end_to_end_and_reports_honestly`：**第一律真能用**——工具面真跑出 `apeireth.exposure_report` 报告（items 非空 + 总评在场），外网暴露面项显式 `not_measured`（测不了不冒充干净）。

**口径随行（`crates/engine/runtime-assembly/tests/capability_switch_chains.rs`，1 个）**——
12. `matrix_production_wiring_rows_match_the_live_roster`：能力矩阵 §0.2 四缝行的旋钮名/名册行名 ↔ 名册实况互钉（默认全 false、开关 + 上游槽齐备全 true、整编关时两行照实 false 的反向面）。

**既有套件零回归**：见 §4 校验（`--no-fail-fast` 全绿）；既有断言**零改动**（名册加行是加性演进，`roster.len() >= 25` 类下界断言与 by-name 断言全部照旧通过；14 节自述合同未动）。

## 4. 校验（2026-10-17，`-j 2` 口径）

| 项 | 命令 | 结果 |
|---|---|---|
| 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ **221 套件 / 5839 passed / 0 failed / 20 ignored**（exit 0） |
| Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告（exit 0） |
| 格式 | `cargo fmt --check` | ✅ 0 diff |
| 中性扫描 | `scripts/check-neutral-terms.ps1` | ✅ 零命中（34 变更文件新增行全中性） |
| 密钥扫描 | `scripts/secret-scan.ps1` | ✅ 无发现 |
| 文档口径 | `scripts/check_doc_caliber.py`（本实录 + user-manual / capabilities-matrix / api / INSTALL） | ✅ 新增违规 0 处 |

**套件口径**：接棒口径「215 套件」+ 本批新增 3 个测试目标（`consolidation_knobs` / `voice_emotion_source` / `exposure_audit_knob`）+ 同树他线在途件 3 个测试目标 = **221 套件**（「套件」= 测试二进制目标计数）。通过数 5839 为当前工作树全量实测（树上含他线在途批次，按真源纪律「逐批交付只报自己批的增量」：本批增量 = 新增 12 个测试函数，全绿）；对真源 `test-baseline.md`（2026-10-17 注记 208 套件 / 5619 passed）的差额由批间在途增长解释，不代改真源。

**在途并行注记（如实）**：本批施工期间工作树内另有他线在途（`crates/capabilities/tools` 的内容流生命周期 / 巡检漏斗面）。其在途中间态两度编译红（`funnel/mod.rs` 引用未定义字段 20×E0609；`exposure_funnel` 测试目标 `PatrolCheckpoint`/`PatrolStateStore` 私有 2×E0603），一度卡住全 workspace 编译——按宪章 §六.3「他人在途红标红不代改」报备归口线，未代改；其自检收口后本批终卷门禁实跑全绿。共享文件 `cli/src/lib.rs` 内他线两处机械空白随本批 `rustfmt` 顺平（纯格式、零语义，跨线机械补位单独声明）。

## 5. 接缝诚实声明（0 装双向性）

- **四个旋钮默认关**：默认关的能力不是系统现状；名册行按实际注册条件取值，「配置说开而注册没成」不复述。
- **缝 3 的数据边界**：本批接的是**读取面**（每次发声现读 presence/PAD 合成器现值）。纯 CLI chat 形态无 presence/PAD 合成器 = 无情绪数据 = 零微调（fail-safe 朝原参数，不猜情绪、不用基线冒充读数）；网关形态下发声微调与 `presence_state` 同一个合成器现值。域伙伴运行时（W3-③）与默认域共用同一情绪源绑定位。
- **缝 3 的名册口径**：`voice_emotion_tune` 行报的是**读取面注入事实**（装配面能报的真值）；某次发声是否真吃到微调取决于当时有无情绪数据（行为级证据见测试 6-8）。
- **缝 4 的只读承诺**：体检不写被测面任何字节、报告永不回密钥值、测不了的项显式「未测」（既有三条硬语义不动）；`read_only` 会话预设下 `tool.exposure_audit` 属读能力（不在写/执行名单），照常可用。
- **战场外最小改动**：`INSTALL.md` / `user-manual.md` / `capabilities-matrix.md` / `api.md` 为文档随行（宪章 §四.5）；`gateway/presence.rs` 仅加两个只读口（行为面零改动，属本缝接线点）。其余文件 0 触碰；LOCKED 5 项 0 触碰；0 引新外部 dep；不写 CHANGELOG、不 commit。

## 6. 后续 stage

- 桌面设置面（`frontend/companion-desktop` 的旋钮镜像）与 onboarding 旋钮文案如需暴露本批四个旋钮，归口设置面线（待上屏类）。
- `tool.exposure_audit` 的**周期巡检化**（筛查→精查→复核漏斗、断点续查）归口守望巡检线（他线在途），本批只收工具面名册接线这一缝。
- 情绪源的多源合成（F1 情感环离散点按 PAD 就近映射的第二读数源）留后续裁定；本批只接 presence/PAD 合成器一个真值源（一条推导面，不搞两套判档）。
