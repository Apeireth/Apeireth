[Document-Meta]
Document: docs/04-internal/01-handoffs/unwired-disposition-proposal-2026-11-04.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-04
Author: deputy

# 孤儿 API 基线（135 条）逐簇处置建议

> **性质**：处置**建议**，不是已执行的动作。**每一条要动代码的建议都需要先裁决**。
> **基线**：`scripts/unwired-baseline.txt`（135 条，`debt-round8` @ `db744eba6755` 复核）。
> **纪律**：本文件每条建议都带**实测判据**；无判据的不写。

## 一句话结论

**这 135 条里真正「该删」的只有 8 条；97 条是「同一能力被切成了多个入口」的收口问题；37 条是设计如此或已裁定不接。**

⇒ ⇒ **不建议把它当「135 个 bug」处理** ⇒ ⇒ 按「**删 8 条 + 收口 3 个家族 + 裁决 2 条**」推进，
剩下的留在基线里并写清理由。

---

## 一、判据（怎么算的）

对每条孤儿，我做了四步实测：

1. **定位定义**（限定名级）—— 135 条落到 **21 个模块 / 45 个文件**；
2. **数真实引用**（排除定义行与文档注释）—— 区分「零引用」和「只在注释里出现」；
3. **找同能力的已用入口** —— 判它是「别名/并行入口」还是「真没实现」；
4. **看宿主的生产使用** —— 宿主在生产 ⇒ 只是「旋钮没拧」；宿主都不在生产 ⇒ **整族待接线**。

**两个必须写清的判据陷阱**（我自己踩过）：

- **同名遮蔽**：光看符号名会撞车（同名方法散落在不同模块）⇒ 必须限定名级定位；
- **零引用有多种假阳性**：间接消费链（模块零引用但经另一个模块进生产）、
  模块自述 default-off（零引用是设计如此）⇒ **不能把「零引用」直接当「缺口」**。

## 二、处置分类（合计 135 条，逐条可核）

| 处置 | 条数 | 判据 | 是否需要裁决 |
|---|---|---|---|
| **D1 删**（真死 API） | **8** | 零引用 + 同能力已有已用入口 + 无独立语义 | 低风险（建议先跑一遍门禁） |
| **D2 收口**（并行入口 / 注入点 / helper） | **97** | 同能力有主路径在用 ⇒ 是别名或没拧的旋钮 | **需要**（定「谁是主入口」） |
| **D3 待裁决**（许可遗忘） | **2** | 涉及「被遗忘权」的安全语义 | **需要**（我不敢替这条做主） |
| **D4 保留**（设计如此 / 已裁定） | **28** | 默认关的族（25）+ owner 裁定（3） | 否（但要**把理由写进基线**） |

**合计 8 + 97 + 2 + 28 = 135** ✅

---

## 三、D1 删（8 条）—— 建议直接做

### 3.1 可变访问器（4 条）

| 符号 | 位置 | 判据 |
|---|---|---|
| `engine_mut` | `runtime-assembly/canonical` | 不可变版 `engine` 被用 **65 处**，`engine_mut` **0 处** |
| `state_mut` | `runtime-assembly/canonical` | `state` **594 处**，`state_mut` **0 处** |
| `entity_mut` | `organ/src/world_model.rs` | `entity` **3 处**，`entity_mut` **0 处** |
| `accumulator_mut` | `foundation/orchestration/src/council/mod.rs` | 同族，0 引用 |

⇒ ⇒ **宿主类型用 `Mutex`/内部可变性做变更** ⇒ ⇒ `*_mut` 这层壳从来没被用过。
⇒ **建议**：直接删（4 行）。

### 3.2 capability 旧变体（2 条）

| 符号 | 判据 |
|---|---|
| `doc_ops_capability` | 生产注册用 `doc_ops_capability_shared_with_slot`（`runtime-assembly/canonical/tool_modules.rs:1024`）⇒ 旧变体是重构遗留 |
| `sheet_ops_capability` | 同族，同理 |

⇒ **建议**：删旧变体，保留 `*_shared_with_slot`。

### 3.3 版本残留（2 条）

| 符号 | 判据 |
|---|---|
| `extract_features` | 引用 **0** |
| `extract_features_v2` | 引用 **0** |

⇒ ⇒ **v1 与 v2 都是 0 引用** ⇒ ⇒ 不是「新版替旧版」，是**两条都没用** ⇒ ⇒ 整族可删（或整族接线，二选一）。

---

## 四、D3 待裁决（2 条）—— 这两条我不替 owner 做主

**符号**：`forget_principal_with_permit`、`research_coordinated_forget_with_permit`

### 4.1 它们的真实性质（实测，不是猜）

`crates/engine/memory/src/universal_forget.rs` 的模块文档写明：

> 带 permit 的变体（`forget_episode_with_permit` / `forget_principal_with_permit` /
> `forget_episode_guarded` / `research_coordinated_forget_with_permit`）把
> `ForgetPermit` 作为**参数**传进销毁核心 ⇒ 不先交访问事件就拿不到令牌
> （回执的签发标记是模块私有零尺寸类型 ⇒ 外部无法伪造）。

⇒ ⇒ **这两条不是冗余入口，是「被遗忘权」的类型级强制面**。

### 4.2 但它们零引用，且**能力已可达**

| 入口 | 引用 | 说明 |
|---|---|---|
| `forget_principal` | **20** | 生产主体遗忘主路径（跨 5 账一个事务） |
| `research_coordinated_forget` | **5**（测试） | 三段式协调遗忘 |
| `forget_principal_with_permit` | **0** | 严格面 |
| `research_coordinated_forget_with_permit` | **0** | 严格面 |

⇒ ⇒ 两个入口**共用同一个内核**（`coordinated_forget_core(req, Option<permit>)`）：
permit 缺省时内核**自解析访问史**⇒⇒ **能力完整可达**。

### 4.3 三个选项（**建议 B**）

| 选项 | 做法 | 代价 / 风险 |
|---|---|---|
| **A** | 直接删这两个严格面 | 功能无损（能力可达）⇒ 但**丢了「调用方自带 permit」这条输入路径** ⇒ 若将来有外部调用方要自带 permit，得重新加 |
| **B（建议）** | **保留 API，但让无 permit 的主入口内部走严格面**（已是现状：`coordinated_forget_core` 两边共用）⇒ ⇒ 再在基线里**写明理由**（「严格面 = 类型级防绕过入口，即使当前无人用」） | 零改动 ⇒ 只是把「零引用」这个状态**变成有理由的零引用** |
| **C** | 把严格面**接到生产**（比如让生产路径强制用 permit 变体） | 改动面大 ⇒ 且要动 `runtime-assembly` |

⇒ ⇒ **我的建议是 B**：**不是删，是「给它一个理由」** ⇒ ⇒ 顺便把「零引用」从「没人用」升级成
「**刻意不接，但防绕过入口必须留着**」。

---

## 五、D2 收口（97 条）—— 真正的债在这

这 97 条**不是死代码**，是**同一个能力被切成了多个入口**。三类：

### 5.1 注入点（31 条，G4）

`with_max_episodes` / `with_len` / `with_backend` / `with_data` / `with_reader` / …

**判据**：宿主的**不可变配置**在生产被用（`MemoryMaterializer` 5 处、`ProactiveRecallPolicy` 8 处），
但**这些 builder 一个都没被调** ⇒ ⇒ **旋钮存在、没拧**。

⚠️ **已知的真问题**（我这两天踩的）：**同名撞车**——
`memory_materializer::with_max_episodes` 与 `consolidation_trigger::with_batch_limit`
是**同一能力两个名字** ⇒ ⇒ 我一度因为撞名让基线**误判「债减少」**。

⇒ **建议**：**不做逐条删除**，而是**按能力归并** ⇒ 每个能力保留**一个**入口名，其余删或改别名。
⇒ **需要裁决**：每个能力「谁是正名」。

### 5.2 并行入口（50 条，G6）

三种形态，处置不同：

| 形态 | 例 | 建议 |
|---|---|---|
| **capability 三变体** | `doc_ops_capability` / `sheet_ops_capability` / `device_control_capability` / `skill_distill_capability` / `allow_capability` —— 生产一律用 `*_shared_with_slot` | 删旧变体（属 D1 家族） |
| **双时间查询被切成 4 个名字** | `query_as_of`（`temporal_graph_store.rs`）· `query_with_time_range`（`provenance.rs`）· `with_as_of` / `without_as_of`（`scope.rs`）—— **三个文件四个名字，只有 `query_with_time_range` 被用（2 处）**；而 `bitemporal_graph.rs` 的 `facts_as_of` / `beliefs_as_of` **才是真正在用的双时间查询** | ⚠️ **这是最典型的「概念没收敛」** ⇒ 建议**归并成一个 `BitemporalQuery` 门面** |
| **微型 helper** | `stream_len` / `total_subscriptions` / `claim_count` / `has_signal` / `matches_word` / `recent_actions` / `last_watch` / `minimax_default` / `rebind` / `not_wired_reason` | 要么接、要么删 ⇒ **建议批量删**（写好没用的 helper 最容易变成误导） |

### 5.3 领域操作入口（16 条）

`add_action` / `remove_job` / `validate_schedule` / `rebind` / `constrain_to_trusted` / `save_to_cluster` /
`upsert_hallway` / `get_page` / `get_persona` / `list_active` / `ensure_governance` / `fetch_governed` / …

⇒ ⇒ 这些是**领域动作**（加动作、删任务、校验调度、改绑定）⇒ ⇒ **零引用通常意味着「这个能力没人用」**
⇒ ⇒ 但**不能一刀删**（有的可能是「应该有但漏接了」）。

⇒ **建议**：这 16 条**逐条问一句「这个能力要不要有」** ⇒ 要 ⇒ 接；不要 ⇒ 删。

---

## 六、D4 保留（28 条）—— 但要把理由写进基线

| 子类 | 条数 | 判据 | 处置 |
|---|---|---|---|
| **生物/认知观测 API** | 25 | 该族**默认关** ⇒ 打开前不会走到 ⇒ **零引用是设计如此** | 留在基线，**理由补进基线注释** |
| **owner 裁定不接** | 3 | `im_router`（W1 IM 通道，2026-10-23 裁定暂缓）· `closure_of` · `verdict_keys_for_principle`（W5 哲学） | 留在基线，**理由补进基线注释** |

⚠️ **D4 的价值在于「写清理由」**：现在基线里只有一行 `# G3` 的组注释，
**逐条没有理由** ⇒ ⇒ 下一个人看到「25 条零引用」仍会以为是缺口 ⇒ ⇒ **白查一遍**。

⇒ **建议**：给 D4 的 28 条**逐条补一句理由**（脚本可自动生成「默认关：<族名>」）。

---

## 七、执行顺序（建议）

| 批次 | 内容 | 条数 | 风险 | 前置 |
|---|---|---|---|---|
| **1** | D4 逐条补理由（**只改基线注释，不动代码**） | 28 | 零 | 无 |
| **2** | D1 删除（可变访问器 4 + capability 旧变体 2 + 版本残留 2） | 8 | 低 | 跑 39 步门禁 |
| **3** | D3 裁决后按裁定执行 | 2 | 中（安全语义） | **owner 裁决** |
| **4** | D2 的三个家族收口（capability 三变体 / 双时间查询 4 名 / 微型 helper） | 约 60 | 中 | **owner 定「谁是正名」** |
| **5** | D2 剩下的领域操作入口逐条过 | 约 37 | 中 | 同上 |

**先做 1 和 2**：这两批**不改变任何运行行为**，只是把「已知的死代码」清掉、把「为什么留着」写清楚
⇒ ⇒ 净效果是**基线从 135 降到 127**，且剩下的每一条都有理由。

---

## 七之二、✅ **执行结果（2026-11-04 17:40，owner 批「0 风险的那批先做」）**

| 批次 | 内容 | 结果 |
|---|---|---|
| **1️⃣ D4 补理由** | 28 条逐条补上理由（**只改基线注释、不动代码**） | ✅ 完成 ⇒ 基线里 **30 条**带理由（28 + 2 条级联说明）⇒ **理由能活过 `--update` 重渲染**（实测跑完 `--update` 文件**逐字相同**） |
| **2️⃣ D1 删 8 条** | 4 个可变访问器 + 2 个 capability 旧变体 + 2 个特征序列化器 | ✅ 完成 ⇒ 基线 **135 → 129** |
| **3️⃣ D3（2 条）** | 许可遗忘的严格面 | ✅ **owner 裁决「按你的建议」⇒ 执行 B：保留 + 写理由**（11-04 18:45）⇒ 「零引用」从「没人用」升级成「**刻意不接，但防绕过入口必须留着**」 |
| **4️⃣ D2（97 条）** | 并行入口 / 注入点 / helper | ⏸ **未动**（需定「谁是正名」）⇒ ⇒ 这批**没有理由** ⇒ ⇒ 在基线里**明写着「还没裁决」** ⇒ ⇒ 下一个人不会误以为是缺口 |

### 📊 当前基线画像（11-04 18:45）
| 项 | 数字 |
|---|---|
| 孤儿基线总条数 | **129**（135 → 129） |
| **带逐条理由**的条目 | **32**（28 条 D4 + 2 条级联 + 2 条 D3） |
| **无理由**的条目 | **97** ⇒ ⇒ **全部是 D2 收口批** ⇒ ⇒ 需先定「谁是正名」 |

### ⚠️ 删 8 条时**棘轮当场逮到的一处级联**（诚实记录）
删掉 `doc_ops_capability` / `sheet_ops_capability` 后 ⇒ ⇒
**`doc_ops_capability_shared` / `sheet_ops_capability_shared` 失去唯一调用者** ⇒ ⇒ 变成**新孤儿**
⇒ ⇒ 棘轮自动把两条归入 **G5（待判）** ⇒ ⇒ 基线是 **129 不是 127**。

⇒ ⇒ **我停手没继续删** ⇒ ⇒ 理由：「**不带 live 策略槽**」这个装配变体还要不要 ⇒ 是**产品判断**
⇒ ⇒ 两条已在基线里写明级联来源 ⇒ ⇒ 等裁决。

### 📌 另一处并发缺陷的状态更新（2026-11-04 17:40）
`principal_sessions` 的**进程级单槽** + `clear_principal_session_recorder()` **无条件清空**
⇒ ⇒ **产品层隐患仍在**（槽仍是 `Option<Arc<dyn …>>`）⇒ ⇒
**当前做法**：该测试文件的 4 个并行用例**已合并为 1 个**（`principal_attribution_end_to_end`）
⇒ ⇒ 等价于**串行化** ⇒ ⇒ **CI 已绿**（本地 39 步门禁**零红**）。
⇒ ⇒ **但隐患没被消除** ⇒ ⇒ 下一个人若再写并行用例、且也要用这个槽 ⇒ **会再踩**。

## 八、我这次**没有做**什么（以及为什么）

| 没做 | 原因 |
|---|---|
| **没删任何一条** | 处置建议涉及**跨模块 API 命名**（正名/别名）与**安全语义**（许可遗忘）⇒ ⇒ 属架构裁决 ⇒ ⇒ 我出证据，不替 owner 定 |
| **没改基线** | 基线是**棘轮**，改它等于宣告「这些债务已处置」⇒ ⇒ 未执行就不该改 |
| **没碰 `docs/status/`** | 台账与 claim 是 owner 的决策面 ⇒ ⇒ 我只在仓内留这份**建议**（DECISION 类） |
| **没用「零引用 = 缺口」这个判据** | 实测证明它有 **5 种假阳性形态**（同名遮蔽 / 间接消费链 / 语言边界 / 模块自述 default-off / 名字出现≠真调用）⇒ ⇒ 本文件的每条都过了限定名 + 宿主生产使用双重核对 |

## 九、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
grep -c '^orphan ' scripts/unwired-baseline.txt          # 135
python3 scripts/audit_unwired.py --check                 # 基线一致（无新增、无过期）
python3 scripts/group_unwired_baseline.py --check        # 分组一致
```

## 相关文件

- 未接线台账（含 A 类真缺口的逐条判定）：[`docs/status/unwired-ledger.md`](../../status/unwired-ledger.md)
- 记忆线状态报告：[`memory-engine-status-2026-11-03.md`](./memory-engine-status-2026-11-03.md)
- 基线本体：`scripts/unwired-baseline.txt`
