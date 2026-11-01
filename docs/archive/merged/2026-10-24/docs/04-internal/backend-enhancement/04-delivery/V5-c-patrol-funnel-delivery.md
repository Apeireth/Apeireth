# V5-c 守望巡检漏斗 · 交付实录（筛查→精查→复核→报告 + 成本闸/断点续查/任务接缝）

```
[Document-Meta]
Document:        V5-c-patrol-funnel-delivery.md
Version:         1.0（V5-c 批交付实录）
Last-Modified:   2026-10-17
Status:          🟢 活跃
Author:          子代理（V5-c 守望巡检漏斗批）
```

> `Status: ✅ 已交付（2026-10-17，V5-c 批）` ｜ 战场：`crates/capabilities/tools/src/exposure_audit/funnel/`
> （**叠加**新层）+ `crates/capabilities/tools/tests/exposure_funnel.rs`（新证据面）
> + `exposure_audit/mod.rs` 注册两行（`pub mod funnel;` + 一句注释，V5-b 既有代码与测试 **0 改动**）。
> 机制卡：`docs/04-internal/philosophy-study/04-security-guard-family.md` §7②「分层漏斗评估流水线」
> （廉价筛查→精查→复核→报告 + 成本/工期闸 + 增量 diff + 断点续查）与 §5 暴露面清单化/两次报告可 diff。

## 1. 一句话

V5-b 暴露面体检回答「暴露了什么」，本层回答「**巡检一遍要花多少、可信度多高、比上次好了还是坏了**」：
六项速扫装进四段漏斗，三件硬机制（成本闸 / 断点续查 / 任务化）把体检升级为**可周期重跑的守望巡检**。

## 2. 漏斗管线图

```text
┌────────────────── 守望巡检漏斗 (筛查 → 精查 → 复核 → 报告) ──────────────────┐
│                                                                              │
│ ①筛查段 (快而宽)     ②精查段 (慢而准)     ③复核段 (降误报)    ④报告段        │
│ 六项速扫 ──候选──►  高风险逐项深查  ──►   二次测量一致才报 ──► 漏斗报告        │
│ (V5-b 现有面)        · 明文点→字段级      瞬时态→剔除(降噪)    · 各段计数/耗时 │
│ run_exposure_audit     取证(脱敏后呈现)   风险漂移→剔除         · 降噪率        │
│ → 风险候选            · 备份排除→逐件核对                      · 增量 diff      │
│                      · 凭据面→路径元数据                        (新增/消失/未变)│
│                                                                              │
│ 成本闸: 每段 max_duration_ms / max_candidates —— 超限截断如实 (truncated +    │
│         held_back 留账, 不静默丢件; 截断的余量在续跑会话里继续)              │
│ 断点续查: 巡检状态落库 (每件一落) —— 中途猝死重启续跑, 已做不重做            │
│ 任务接缝: 巡检 = 持久任务 (V6-a 队列语义同款, 对队列只读消费) —— 失败留账     │
│           只能人判, 失败绝不自动重试                                         │
└──────────────────────────────────────────────────────────────────────────────┘
```

数据流：`run_exposure_audit_at`（V5-b 六项速扫，只读消费）→ `Candidate`（键 = `<体检项 id>|<暴露面词面>`）
→ 精查取证 `DeepEvidence`（脱敏后）→ 二次测量复验 → `FunnelReport`（JSON 合同 + 文本）+
`PatrolCheckpoint`（`patrol_state.json`，巡检自己的工作区，**与被测面分开**）。

## 3. 四段语义（逐段硬口径）

| 段 | 输入 → 输出 | 硬口径 |
|---|---|---|
| ①筛查（快而宽） | 六项速扫 6 项 → 风险候选（发现逐条，同键取最高风险） | 只读消费 V5-b 现有面，0 改体检口径；候选整份落库（含未准入） |
| ②精查（慢而准） | 风险级 ≥ 精查线（默认中风险）的候选 → 脱敏取证 | 明文点→**字段级取证脱敏后呈现**（`[redacted]`，值零采集）；备份排除→**逐件核对**（清单行逐条对词表 + 显式排除行逐条核对「真的没进包」）；凭据面→**路径元数据**（内容零读取）；无取证对象→显式「无取证对象」不编造 |
| ③复核（降误报） | 全部候选 × 二次测量快照 → 确认 / 剔除 | **二次测量一致才报**；瞬时态（二次不见）与风险漂移（二次不一致）双双剔除入降噪账；未复核的如实「留存未复核」不作结论；二次测量只采一次（跨续跑同一把尺子） |
| ④报告 | 确认候选 + 各段账 → 漏斗报告 + 增量 | 各段计数/耗时/截断如实；降噪率 =（瞬时态 + 风险漂移）/ 筛查候选；**完成巡检**才与上次 diff（新增/消失/未变），未完成如实 `diff: null` |

**成本闸**：每段 `max_candidates`（件数）+ `max_duration_ms`（耗时），**本会话口径**计数；超限即截断
并如实记 `truncated` + `held_back`（余量留账），截断的余量在续跑会话继续处理。报告段是纯组装段，
**不设逐项闸**（零假旋钮：不设用不上的闸，耗时仍如实入账）——设计裁定。

**断点续查**：每件处理完立即落库 `patrol_state.json`（`JsonPatrolStore`，或 `MemoryPatrolStore` /
实现 `PatrolStateStore` 端口）；重启 `run_patrol` 自动续跑（同巡检 id、`attempts` 按执行段计数、
崩溃恢复 running→pending 与队列 `open` 同款），已筛查/已精查/已复核全不重做。落库档损坏**拒绝即帧**
（`PatrolError::Checkpoint`），不静默重建。

**任务化（task_queue 接缝）**：巡检 = 持久任务。`PatrolTaskAccount` 的状态词面/转移规则/人判边界
与 V6-a 持久任务队列**同款**（六态 `TaskState`、自报必须执行者本人、**失败只能人判**、失败绝不自动
重试、重试是显式人判动作）。对队列本体**只读消费**（`load_from_queue` 只调 `TaskQueue::get`）：
入队/指派/人判写面全部沿用 V6-a 既有出口，漏斗层 0 新增队列写面；`to_task_spec()` 产出队列既有入口
吃的 `TaskSpec`（带幂等键 + 指派），两侧同一本账（`sync_from_record` 对齐）。

## 4. 文件清单（战场内叠加）

| 文件 | 行数（约） | 角色 |
|---|---|---|
| `exposure_audit/funnel/mod.rs` | 400 | 漏斗编排（四段流转 + 成本闸 + 断点续查 + `run_patrol`）+ 管线图 |
| `exposure_audit/funnel/evidence.rs` | 400 | 候选模型 + 精查取证（字段级脱敏 / 逐件核对 / 路径元数据） |
| `exposure_audit/funnel/checkpoint.rs` | 460 | 巡检状态落库（线上合同 + `JsonPatrolStore` / `MemoryPatrolStore`） |
| `exposure_audit/funnel/task_seam.rs` | 290 | 巡检任务账（V6-a 队列语义同款 + 只读消费） |
| `exposure_audit/funnel/report.rs` | 320 | 漏斗报告（四段账 / 降噪账 / 增量 / JSON 合同 + 文本） |
| `exposure_audit/funnel/diff.rs` | 80 | 增量 diff（新增/消失/未变） |
| `exposure_audit/mod.rs` | +3 | 注册 `pub mod funnel;`（V5-b 既有代码 0 改） |
| `tools/tests/exposure_funnel.rs` | 520 | 集成证据 13 例 |

0 引新外部 dep（serde/serde_json/apeireth-orchestration 全为既有依赖；`Cargo.lock` 0 行 diff）。

## 5. 测试证据（≥12 硬指标：8 类场景全覆盖）

集成面 `crates/capabilities/tools/tests/exposure_funnel.rs`（**13 例**）+ 模块单测（**9 例**）：

| # | 场景（任务硬指标） | 测试 | 钉住的行为 |
|---|---|---|---|
| 1 | 漏斗四段流转 | `funnel_flows_screen_deep_recheck_report_in_order` | 四段固定顺序、逐段账首尾相接、完成即任务 Done、逐条上报带取证 |
| 2 | 降噪（复核段剔除瞬时态） | `recheck_drops_transient_probe_findings_as_noise` | 瞬时监听面二次不见→剔除入降噪账、稳定候选照报、降噪率 0.5 |
| 2b | 降噪（风险漂移） | `recheck_drops_risk_shift_findings_as_noise` | 同词面二次测量风险漂移→不一致不报 |
| 3 | 增量 diff（新增/消失/未变） | `diff_marks_new_gone_and_unchanged_between_patrols` | 三次巡检：首跑全新增→加件见新增+未变→减件见消失+未变 |
| 4 | 成本闸截断如实 | `cost_gate_truncates_deep_stage_and_stays_honest` | 件数闸：处理 1 留 2、显式 truncated、不冒充完成、未完成不 diff、口径回显 |
| 4b | 成本闸（耗时闸） | `time_budget_truncates_screen_stage_and_stays_honest` | 可控时钟 1s/跳：耗时闸截断筛查段、余量留账、不冒充完成 |
| 5 | 断点续查（重启续巡） | `checkpoint_resumes_after_mid_run_death_without_rework` | 落库 6 次后**中途猝死**（错误即帧）→重启续巡：同巡检 id、attempts=2、已精查 2 件不重做只做余下、跑完产出增量 |
| 6 | 巡检任务化（队列同账） | `patrol_task_shares_queue_account_and_keeps_human_judge_only_failure` | 入队规格同款、侧读同一本账（id/幂等键/状态/执行段）、agent 碰不到失败面（`HumanOnly`）、失败留账只能人判、失败不自动重试 |
| 7 | 脱敏（精查取证零明文） | `deep_evidence_and_checkpoint_carry_zero_plaintext` | 取证行/报告/落库档三面零明文，`[redacted]` 后呈现 |
| 8 | 被测面只读零写 | `audited_face_stays_read_only_while_state_lands_outside` | 盘面快照前后逐字节一致、状态落在巡检自己的工作区 |
| 8b | 备份排除项逐件核对 | `backup_exclusion_deep_check_audits_bundle_item_by_item` | 清单行逐条对词表 + 显式排除项泄漏进包逐件揪出 |
| 8c | 诚实未测零编造 | `clean_surface_fabricates_no_candidates` | 干净面零候选零上报、零除零记 0、空巡检也算完成 |
| 8d | 报告 JSON 合同 | `funnel_report_json_contract_is_stable` | schema/四段顺序/任务节/增量/成本闸回显形状钉死 |

模块单测 14 例：候选去重取最高风险、明文取证脱敏零明文、路径元数据零读取、风险词面往返、
diff 三分与首跑无基线、落库档往返、坏档拒绝即帧、无档=空、任务账转移（认领授权/人判边界/崩溃恢复同款）、
报告 JSON 合同形状、降噪率零除零口径。

**既有 215 套件零回归**：V5-b 既有测试（`tools/tests/exposure_audit.rs` 14 例 + `exposure_audit/*` 单测）
**零改动**（`git diff` 钉死：该文件 0 行 diff）；全工作区实测见 §7。

## 6. 设计裁定（O-6 三阶口径，择要）

- **总体最优**：漏斗作为 exposure_audit 的**叠加层**而不是重写体检——筛查段直接复用
  `run_exposure_audit_at`（同一内核多出口的既有原则），拒：把六项速扫复制进漏斗（两套口径必漂移）。
- **系统最优**：编排放 capabilities/tools（与体检同居），任务接缝**只读消费** foundation/orchestration
  的队列语义（依赖方向 tools→orchestration 既有）；拒：把漏斗下沉 orchestration（foundation 不该知道
  体检域）、拒：给队列加新写面（巡检入队/人判沿用 V6-a 既有出口，队列核 0 改）。
- **架构最优**：成本闸按**本会话**计数 + 余量留账续跑，截断与完成解耦；拒：截断即终态（丢巡检连续性）、
  拒：全局预算一把抓（每段成本结构不同：精查是大头）。落库走 `PatrolStateStore` 端口（文件/内存两实现），
  拒：直接散写文件（无法测试猝死/坏档语义）。

## 7. 0 装诚实真账（校验实测）

- 测试：`cargo test --workspace --all-targets --locked --no-fail-fast -j 2` = **221 套件 / 5839 passed /
  0 failed / 20 ignored**（2026-10-17 全工作区合流实测，详 §8）；本批新增 27 例（集成 13 + 单测 14）全绿。
- clippy：`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` = **0 警告（exit 0）**
  （2026-10-17 实测；工具 crate 单独复跑亦 0 警告）。
- fmt：`cargo fmt --check` 全仓 0 diff（2026-10-17 实测；本批文件经 rustfmt 单文件收口）。
- 中性表述：`scripts/check-neutral-terms.ps1` = OK 零命中（2026-10-17 实测）。
- 0 触碰 LOCKED 5 项（9 哲学锚本体 / 13 键 / 3 不可变脊柱 / workspace.version / R11 baseline）：`git diff` 钉 0 行。
- 0 引新外部 dep：`Cargo.lock` 0 行 diff。未写 CHANGELOG、未 commit。

**未接线/未实测（如实标注）**：
1. 漏斗层暂无独立工具/CLI 出口（本批交付漏斗内核 + 证据面；`tool.exposure_audit` 与 `apeireth exposure report`
   仍是 V5-b 两出口）。巡检入队走 V6-a 既有 `tool.task_board` 写面的接线归口后续批。
2. 周期调度（定时巡检）未建——本批是「跑一次巡检」的执行层，触发器接线归口后续批。
3. live E2E（真 netstat 探针 + 真备份包）未实测；本批证据全部为受控 fixture 面。
4. 测试数字引用口径：测试基线单一真源 `docs/03-reference/test-baseline.md` 未代改；本批增量以文末实测追记如实报。

## 8. 实测追记（数字随批，如实登记）

- **全工作区实测**（`cargo test --workspace --all-targets --locked --no-fail-fast -j 2`，2026-10-17，
  全工作区合流树含多条在途线）= **221 套件 / 5839 passed / 0 failed / 20 ignored**（exit 0）。
  本批增量 = 1 套件（`tools/tests/exposure_funnel.rs`）+ 27 例（集成 13 + 模块单测 14），
  另 `exposure_audit/funnel` 各模块单测并入既有 `tools` lib 套件计数。
  测试基线单一真源 `docs/03-reference/test-baseline.md` **不代改**（维护规则归口基线卷）。
- **负载 flake 定性（纪律 §四.1 口径）**：首轮全量跑出现 1 例失败
  `apeireth-cli self_status_e2e::memory_ledger_counts_track_the_cognitive_store`；
  **隔离复跑 = 通过**（1 passed / 0 failed），**整轮复跑 = 221 套件全绿（exit 0）**，
  且该测试所在的工具 crate 正被并发在途线编辑（`self_status.rs` / `cli/src/lib.rs` 在跑测期间有
  在途改动）——定性为负载/在途竞态 flake，非本批回归（本批改动为纯新增：funnel/ 新层 + 新测试 +
  `exposure_audit/mod.rs` 注册 3 行，不触碰 memory/self_status 语义）。在途红线标红不代改，
  归口线自检收口。
