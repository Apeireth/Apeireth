[Document-Meta]
Document: docs/04-internal/01-handoffs/unwired-canonical-name-candidates-2026-11-04.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-04
Author: deputy

# D2（94 条）正名候选表 ⇒ 每条给出**建议 + 实测判据**

> **性质**：处置**建议**，不是已执行的动作。**每一条要动代码的都需先裁决**。
> **上游**：[`unwired-disposition-proposal-2026-11-04.md`](./unwired-disposition-proposal-2026-11-04.md)（D2 的定义与判据方法）。
> **口径**：`debt-round8` @ `6d195bcd6143`，孤儿基线 **129 条**，其中 **D2 待裁决 94 条**。

## 一句话结论

**94 条不是一个问题，是三个**：
- **26+16 = 42 条**是「**注入点/便利包装**」⇒ ⇒ 处置是「**接线或删**」，**没有改名这回事**；
- **4 条**是「**双时间查询散在 3 个文件、4 个名字**」⇒ ⇒ 处置是「**归并门面**」，**必须改名**；
- **其余 48 条**是「**领域操作入口**」⇒ ⇒ 处置是「**这个能力要不要有**」，**逐条问业务**。

⇒ ⇒ **真正需要"定正名"的只有双时间那 4 条**；其余 90 条要么删、要么接，**不涉及命名裁决**。

---

## 二、家族一：注入点 / 便利包装（42 条）——**处置是「接线或删」，不是改名**

### 2.1 记忆 crate 的 26 条（我的区）

| 符号 | 位置 | 同文件已用入口 | 建议 |
|---|---|---|---|
| `with_max_episodes` | `memory_materializer.rs` | `bounded` | ⚠️ **同名撞名**：与 `consolidation_trigger::with_batch_limit` 同义 ⇒ ⇒ **删**（我这侧） |
| `with_ranking_config` | `coordinator.rs` | `with_scoped_backend` / `with_audit_events` | **接线**：coordinator 已有 `ranking` 字段 ⇒ 这个 builder 是它的唯一入口 ⇒ ⇒ 生产该走它 |
| `with_min_candidate_score` | `proactive_recall.rs` | `with_budget` / `enabled` | **接线**：与 `with_budget` 同族（预算旋钮）⇒ ⇒ 合并成一个预算构造 |
| `with_proactive_budget` | `scope.rs` | `session` / `kind` / `is_visible_in` | **接线或删**：与上条**跨文件同义**（预算）⇒ ⇒ 归并 |
| `recall_with_policy` | `proactive_recall.rs` | 同上 | **接线**：主动召回的策略入口，生产没接 ⇒ ⇒ 属「该有但漏了」 |
| `retrieve_with_reranker` | `retrieval_pipeline.rs` | `new` / `push` | ⚠️ **先答「要不要重排」**：重排器没接 ⇒ ⇒ 若要 ⇒ 接线；若不要 ⇒ 删 |
| `materialize_to_sink` | `memory_materializer.rs` | `bounded` | **接线**：materializer 的写面（与 `sink` 概念配对）⇒ ⇒ 该有 |
| `is_episode_forgotten` | `universal_forget.rs` | `forget_episode`（43 处） | **接线**：治理状态的只读查询面 ⇒ ⇒ 遗忘权面板要用 |
| `ensure_governance` | `memory_governance.rs` | — | **接线**：治理库自举 ⇒ ⇒ 生产该有 |
| `fetch_governed` | `memory_governance.rs` | — | **接线**：治理视图读 ⇒ ⇒ 与 `governed_recent_episodes` 配对 |
| `governance_cas_failure` | `memory_governance.rs` | — | **删**（CAS 失败态在治理面已由 `status` 表达） |
| `get_for_subject` | `commitments.rs` | `open` / `pool` | **接线或删**：承诺按主体查 ⇒ ⇒ 承诺族未接生产 ⇒ ⇒ **随承诺族一起裁** |
| `list_active` | `commitments.rs` | 同上 | 同上 |
| `get_persona` | `five_dimensional.rs` | `push_working` | 同上（人格族未接生产 ⇒ 随族裁） |
| `get_page` | `wiki_fs.rs` | `compile_page` / `run_lint` | **接线**：知识库分页读 ⇒ ⇒ 与 `compile_page` 配对 |
| `links_for_diary` | `cross_diary.rs` | `links_for_fact` | **接线**：跨日记链接 ⇒ ⇒ 与 `links_for_fact` 同族 |
| `save_to_cluster` | `meta_thinking.rs` | `to_markdown` | **接线或删**：`ClusterStore` 已在生产（间接链）⇒ ⇒ 这条是写面 ⇒ ⇒ 该有 |
| `with_reader` | `meta_thinking.rs` | 同上 | **接线**：读面注入 ⇒ ⇒ 与 `save_to_cluster` 配对 |
| `with_date` | `dailynote/note.rs` | `new` | **接线**：日记日期 ⇒ ⇒ 11-04 已修日记族 ⇒ ⇒ 该有 |
| `upsert_hallway` | `hallways.rs` | `label` / `update` | **接线**：走廊写面 |
| `default_min_count` | `hallways.rs` | 同上 | **删**（默认值构造，与 `upsert_hallway` 配套但无独立价值） |
| `set_quiet_threshold` | `sleep_cycle.rs` | `with_config` / `with_clock` | **接线**：睡眠周期静默阈值 ⇒ ⇒ 该有 |
| `with_len` | `online_calibration.rs` | `apply` / `identity` | **删**（便利构造，α 参数已够） |
| `scalar` | `online_calibration.rs` | 同上 | **删**（1 行便利构造）⇒ ⚠️ 全仓 22 次提及**全是**"Unicode scalar value"这个普通词（注释）⇒ ⇒ 与 API 无关 |
| `decompose_default` | `calibration.rs` | `calibration_gap` / `reliability_contribution` | **接线或删**：校准分解默认值 ⇒ ⇒ 待答「校准分解要不要接」 |

### 2.2 能力层（tools / storage / plugin 等）的 16 条

**处置原则相同**（接线或删），但**归属别人的区** ⇒ ⇒ 我只出清单，不动手。

| 符号 | 位置 | 备注 |
|---|---|---|
| `doc_ops_capability_shared` / `sheet_ops_capability_shared` | `tools/src` | ⭐ **11-04 级联产物** ⇒ 理由已写进基线 ⇒ ⇒ **待你裁决「不带 live 策略槽的装配变体要不要」** |
| `explicit_allow_list` | `tools/src/fetch.rs` | fetch 工具的显式白名单 ⇒ 与 `CapabilitySwitch` 族一起裁 |
| `not_wired_reason` | `tools/src/accountability_view.rs` | ⭐ **自曝「未挂载原因」** ⇒ ⇒ 它是**给未接线功能用的** ⇒ ⇒ 随那个功能的接线/删除一起处置 |
| `with_rate_limiting` / `with_shell_executable` / `with_user_agent` | `tools/src` | 工具面旋钮 ⇒ 同 G4 族 |
| `save_to_cluster`（`tools/src` 另有）· `serve_canonical` · `sheet_ops_capability` | — | 同族 |
| `library_root` / `trait_locks_path` / `set_quiet_threshold` | `tools/src` · `runtime-assembly` | 路径/锁 ⇒ 属「宿主装配面」 |

---

## 三、家族二：双时间查询（4 条）——⭐ **唯一真正需要「定正名」的**

### 3.1 现状：同一能力，4 个名字，散在 3 个文件

| 符号 | 文件 | 宿主 | 引用 | 判据 |
|---|---|---|---|---|
| `facts_as_of` | `bitemporal_graph.rs` | 事实图 | **7** | ✅ **真在用** |
| `beliefs_as_of` | `bitemporal_graph.rs` | 信念图 | **7** | ✅ **真在用** |
| `query_with_time_range` | `provenance.rs` | 溯源 | **2** | ✅ 在用（但是**另一套时间轴**：溯源区间 ≠ 事实 as-of） |
| `query_as_of` | `temporal_graph_store.rs` | 临时图 | **0** | ❌ 零调用 |
| `with_as_of` / `without_as_of` | `scope.rs` | `RecallPolicy` | **0** | ❌ 零调用 |

### 3.2 ⚠️ 撞名警告（**这一族必须改名时才不会踩坑**）

`with_as_of_ms` **在两个文件都有**：
- `temporal_graph_store.rs` 的 `TemporalGraphQuery::with_as_of_ms` ⇒ **调用 10 处**
- `scope.rs` 的 `RecallPolicy::with_as_of_ms` ⇒ **调用 10 处**

⇒ ⇒ **两个同名 API 各自都在用** ⇒ ⇒ **谁都不能删** ⇒ ⇒ 只能**改名区分**。
⇒ ⇒ 这也解释了为什么 `with_as_of` / `without_as_of`（`scope.rs`）零调用：
**它们的兄弟 `with_as_of_ms` 被用着**，但**语义不同**（一个收 `Option<i64>`，一个收 `i64`）⇒ ⇒ **看着像、其实不是**。

### 3.3 建议（B 方案：**不改名、不删、只补理由**）

| 符号 | 建议 | 理由 |
|---|---|---|
| `query_as_of` | **接线或删**（二选一） | 它是 `TemporalGraphQuery` 的 as-of 查询 ⇒ ⇒ 与 `facts_as_of` **能力重叠** ⇒ ⇒ 若 `TemporalGraphStore` 未来要接生产，就接它；若这条线不接，就随族裁 |
| `with_as_of` / `without_as_of` | **接线**（补上 `with_as_of_ms` 缺的「取消 as-of」语义） | `RecallPolicy` 已有 `with_as_of_ms`（10 处在用）⇒ ⇒ 但**没有「不带 as-of」的构造** ⇒ ⇒ 这两条是它的补集 ⇒ ⇒ **该有** ⇒ ⇒ 建议**保留 + 接线** |

⇒ ⇒ **本族不需要改名**（除了将来若要统一命名，再单独一轮）⇒ ⇒ **风险低**。

---

## 四、家族三：领域操作入口（48 条）——**处置是「这个能力要不要有」**

这 48 条**不是冗余，是「没人用的领域动作」**。逐条问业务即可，**不需要命名裁决**。

### 4.1 微型 helper（16 条）——**建议批量删或接线**

判据：**体长 ≤ 17 行 + 全仓提及 1～2 次**（除 `scalar` 是撞名噪声）。

| 符号 | 位置 | 体长 | 建议 |
|---|---|---|---|
| `add_action` | `guard/src/chain.rs` | 1 行 | **接线**（行为链加动作 ⇒ 与 `actions`（已用）配对） |
| `claim_count` | `governance/src/evidence.rs` | 1 行 | **删**（计数字段可由 `claims.len()` 得） |
| `has_signal` | `governance/src/input_security.rs` | 1 行 | **接线或删**（输入安全信号查询） |
| `stream_len` | `orchestration/src/runtime_invariants.rs` | 1 行 | **删**（`len()` 够用） |
| `total_subscriptions` | `plugin/src/mcp/subscribe.rs` | 1 行 | **删** |
| `matches_word` | `governance/src/approval_family.rs` | 2 行 | **接线或删**（审批族关键词匹配） |
| `last_watch` | `tools/src/device/guard_watch.rs` | 4 行 | **接线**（设备守护最近一次 ⇒ 面板要用） |
| `minimax_default` | `plugin/src/perception_backend.rs` | 6 行 | **删**（感知后端默认 ⇒ 无独立价值） |
| `recent_actions` | `guard/src/chain.rs` | 6 行 | **接线**（行为链最近动作 ⇒ 与 `actions` 配对） |
| `constrain_to_trusted` | `guard/src/intent.rs` | 7 行 | ⚠️ **安全面** ⇒ ⇒ **先答「意图要否约束到可信源」** ⇒ ⇒ **不删** |
| `rebind` | `plugin/src/alias.rs` | 7 行 | **接线或删**（别名重绑） |
| `remove_job` | `orchestration/src/job_board.rs` | 7 行 | **接线**（任务板删任务 ⇒ 与 `add` 配对） |
| `default_min_count` | `memory/hallways.rs` | 10 行 | **删** |
| `validate_schedule` | `orchestration/src/cron.rs` | 11 行 | ⚠️ **安全面** ⇒ ⇒ **先答「调度要不要校验」** ⇒ ⇒ **不删** |
| `decompose_default` | `memory/calibration.rs` | 17 行 | **接线或删**（校准分解） |
| `with_used` | ⚠️ **定义未定位** | — | ⇒ 需先定位（`grep` 未命中 `pub fn with_used`）⇒ ⇒ 可能已不存在 ⇒ **待查** |

### 4.2 其余领域入口（32 条）

分布在 `engine/storage`（8）· `engine/runtime-assembly`（5）· `adapters/sdk`（5）· `engine/guard`（4）·
`foundation/governance`（4）· `engine/runtime`（3）· `adapters/gateway`（3）· `adapters/cli`（3）·
`foundation/plugin`（6）· `foundation/orchestration`（5）· `foundation/core`（1）

⇒ ⇒ 处置原则：**每条问一句「这个能力这个产品要不要有」** ⇒ ⇒ 要 ⇒ 接线；不要 ⇒ 删。
⇒ ⇒ **归属多数是别人的区** ⇒ ⇒ 我只出清单，不动手。

---

## 四之二、📌 **给 owner 的选项（拍板用）**

### 选项 A：**按能力逐族裁**（**我建议**）
⇒ ⇒ 对每一族给出「**留哪几个 / 删哪几个 / 接到哪**」的明确清单 ⇒ ⇒ **owner 只做「同意 / 不同意」的判断**
⇒ ⇒ 一次能收掉 **30～46 条**（双时间 4 + 记忆 crate 注入点 26 + 微型 helper 中的 8 条死代码）
⇒ ⇒ 代价：涉及**别人的区**（tools / storage / plugin / guard 等 48 条）⇒ ⇒ 需**跨区协作**。

### 选项 B：**只做记忆 crate 内的**（**最省事，我今天就能干完**）
⇒ ⇒ 只处理**双时间 4 条 + 记忆 crate 注入点 26 条**（都在我的区 ⇒ ⇒ 零跨区）
⇒ ⇒ 基线 **129 → 99**；剩下的 48 条**留在基线里**（并写明「已识别、待跨区裁」）
⇒ ⇒ 代价：治标不治本 ⇒ ⇒ 「概念没收敛」的核心问题**只解决了一半**。

### 选项 C：**先只做「安全的删除」**（**最保守**）
⇒ ⇒ 只删**体长 ≤ 7 行、零引用、且不是安全面**的微型 helper（约 6～8 条）
⇒ ⇒ 基线 **129 → 121**；其余全部保留并**逐条补理由**
⇒ ⇒ 代价：动得最少 ⇒ ⇒ 但**注入点族一个没动** ⇒ ⇒ 「旋钮没拧」的问题原样留着。

### 选项 D：**先不删，只把 94 条的理由补齐**（**最保守 + 信息量最大**）
⇒ ⇒ 94 条**逐条写理由**（像 11-04 那 28 条一样）⇒ ⇒ 基线从「94 条没说明」变成「94 条条条有说明」
⇒ ⇒ 代价：零代码改动 ⇒ ⇒ 但**债没还**，只是**把债照亮了**。

### 我的建议
**B + C 合并做**：先做**记忆 crate 内的双时间 4 条 + 注入点 26 条**（我的区、零跨区、今天能完），
同时把**跨区的 48 条**按本文件的清单**逐条写出建议**（出清单不碰代码）⇒ ⇒
⇒ ⇒ 基线 **129 → 99**，且剩下的都有理由 ⇒ ⇒ **跨区那批等你一句话就能执行**。

## 五、执行顺序（建议）

| 批次 | 内容 | 条数 | 归属 | 风险 |
|---|---|---|---|---|
| **1** | **双时间家族**：接线 `with_as_of`/`without_as_of`（补 `RecallPolicy` 的「取消 as-of」）· `query_as_of` 二选一 | 4 | 记忆 crate（我） | 低 |
| **2** | **记忆 crate 的注入点**：按 2.1 表逐条「接线 or 删」 | 26 | 记忆 crate（我） | 中 |
| **3** | **微型 helper**：批量删 8 条 + 接线 8 条 + 3 条安全面**先答后定** | 16 | 跨 crate | 中 |
| **4** | **capability 三变体**：定「不带 live 槽的装配变体要不要」 | 2 | tools（他） | 低 |
| **5** | **其余领域入口**：逐条问业务 | 46 | 跨 crate | 中 |

⇒ ⇒ **先做 1 和 2**（都在记忆 crate 内 ⇒ ⇒ **不跨区、不需要任何人点头**）⇒ ⇒ 净效果 **129 → 99**。

---

## 六、这次**没有做**什么

| 没做 | 原因 |
|---|---|
| **没删任何一条** | 涉及跨 crate API 与**产品契约**（能力清单）⇒ ⇒ 属裁决 ⇒ ⇒ 我出证据 |
| **没改名** | 「定正名」是**架构裁决**；且双时间那族实测**不需要改名**（撞名的是 `with_as_of_ms`，两个都在用）⇒ ⇒ 改名反而会制造新风险 |
| **没碰 `docs/status/`**（除台账那一处 stale） | 台账与 claim 是 owner 决策面 ⇒ ⇒ 只改**已过期的那一条** |
| **没用「同文件有已用入口」当判据** | 实测它太弱（134/135 命中）⇒ ⇒ 本文件用的是「**限定名 + 引用次数 + 宿主生产使用**」三重核对 |

## 七、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
grep -c '^orphan ' scripts/unwired-baseline.txt          # 129
python3 scripts/audit_unwired.py --check                 # 一致（无新增、无过期）
python3 scripts/group_unwired_baseline.py --check        # 一致
```

## 相关文件

- 上游建议书：[`unwired-disposition-proposal-2026-11-04.md`](./unwired-disposition-proposal-2026-11-04.md)
- 未接线台账：[`docs/status/unwired-ledger.md`](../../status/unwired-ledger.md)
- 记忆线状态：[`memory-engine-status-2026-11-03.md`](./memory-engine-status-2026-11-03.md)
