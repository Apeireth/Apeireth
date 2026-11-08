[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-b3-production-wiring-2026-11-08.md
Class: EVIDENCE
Status: 🟢 已核验（本地全绿；装配根那一行待主指挥统一加）
Last-Modified: 2026-11-08
Author: lead

# B3 生产接线：影子双跑 + 换算法流程（2026-11-08）

> **四级口径**：本页是**证据页**，不是能力声明页。
> 能力等级以 `docs/status/claims.yaml`（唯一真源）为准。
> 本页**不**宣称 **HARDWARE VALIDATED**（无真机、无真模型 E2E）。
>
> **中性表述**：全文只描述**机制**（影子双跑 / 裁决 / 三道闸 / 归档 / 回退 / 治理链），
> 不引用任何外部产品名、仓库名或文档名。

## 〇、这批补的是哪个洞

`apeireth_memory::shadow_run`（影子双跑 + 裁决）与 `apeireth_memory::swap_flow`
（三道闸 + `Swap cost` + 归档 + 回退）**早已实现**，但**只有 example 在用**
⇒ ⇒ 生产上「换算法」这件事**没有受控流程** ⇒ ⇒ 「零风险上线 / 可回退」这句话**没有落点**。

本批把它接到生产：**默认关 / 零行为变化 / 切换走仓里既有的治理面**。

## 一、接线形态（file:line）

| 面 | 位置 | 作用 |
|---|---|---|
| 接线模块 | `crates/engine/runtime-assembly/src/canonical/shadow_wire.rs`（新建） | 配置面、影子双跑、裁决、三道闸、归档、指针、回退、治理闸 |
| 模块声明 | `crates/engine/runtime-assembly/src/canonical.rs` | `pub mod shadow_wire;`（+ `canonical.rs` **未登记棘轮**） |
| 对外导出 | `crates/engine/runtime-assembly/src/lib.rs` | `pub use canonical::shadow_wire;`（供集成测试与后续面取用） |
| 装配根那一行 | **待主指挥加**（见 §五） | 装配根调用 `install_shadow(...)` |

## 二、**默认关 ⇒ 逐字节零行为变化**（第一条纪律）

* `shadow.json` **不存在** ⇒ `install_shadow` 返回 `Ok(None)` ⇒ **不构造任何东西**：
  不注册模块、不读文件、不写文件、不占内存 ⇒ 装配根那一行是**纯 no-op**。
* **坏配置 ⇒ 拒绝装配**（畸形 / 串档 / 未来版本 / 未知字段 / 越界取值 ⇒ `Err(真因)`），
  **不许**被当成「没配置」⇒ 失败闭合。
* 实测：干净数据根跑完 ⇒ 目录里**只有**测试自己建的库文件（**零新增**）。

## 三、影子双跑：**只观测、不改结果**（第二条纪律）

* **现役侧** = 生产装配出来的 coordinator（**同一个** backend/governance 句柄）；
* **候选侧** = **另建**一个 coordinator（**同一批只读库**、**只差排序旋钮**）
  ⇒ ⇒ 两侧都是**真生产路径**（不是复刻实现 ⇒ 不会「测的和上线的不是一回事」）；
  ⇒ ⇒ 影子侧**物理上**碰不到现役实例的状态与返回。
* **探针可见域固定** = 全局 + 探针会话 ⇒ 两侧同域可比；⚠️ 这**不是**某个用户的逐字可见域
  ⇒ 影子分数是**同域口径**，不是「某用户体感」。
* **token 口径 = 字符数代理**（仓里没有分词器）⇒ 只用于**两侧相对比较**。
* **报告里没有「答案」字段**（只有 id 与指标）⇒ 影子纪律在数据结构上就成立。

### 归档与轮转

* 归档目录：`<数据根>/memory/shadow/reports/`；报告 = `shadow-<时间戳>-g<代次>.json`
  （带时间戳与代次 ⇒ **不覆盖**旧报告，文件名序 = 时序）；
* **轮转**：只保留最近 N 份（默认 20，`shadow.json` 可配）⇒ 目录不会无限长；
* **指针**：`<数据根>/memory/shadow/pointer.json`（当前变体 / 上一版 / 代次 / 切换时刻 / 两版旋钮）
  ⇒ **回退靠它**（上一版的旋钮必须留着，否则回退写不回去）。

## 四、换算法流程：**三道闸 + 治理面**

### ① 治理面（**先过**，失败闭合）

* 切换是**状态变更** ⇒ ⇒ 必须拿到仓里**既有的治理链**放行
  （`GovernanceChain::run` ⇒ `Action::CapabilityDispatch{ capability: "memory.swap_algorithm" }`
  ⇒ 走仓里既有的权限/审批判定，**不自造审批**）；
* **没有治理面 ⇒ 不许换**（`Err`：明确点名「没有治理链 ⇒ 不许换算法」）；
* **治理面拒绝 ⇒ 不许换**（`Err`：带上治理链的原话）。

### ② 库级三道闸（**不许跳**）

| 闸 | 判据 | 不通过时 |
|---|---|---|
| **裁决闸** | `Approved` / `ApprovedWithGain` 才放行 | **不切**、指针不变、不落盘、理由带数字 |
| **回退速度闸** | `rollback_ms < switch_ms`（回退不比切换快 ⇒「可回退」是假的） | 同上 |
| **样本充分闸** | `shadow_runs >= 2` **且** `shadow_cases >= 10`（一轮只是运气；题太少结论没意义） | 同上 |

⇒ **任一不过 ⇒ 什么都不做**（不是「照切」，也不是「静默跳过」）。

### ③ 「切换」到底切了什么（不许含糊）

切的是**生产排序旋钮**（`retrieval.json` 里的 `first_pos_alpha` / `diversity_lambda`）
⇒ ⇒ **在下次进程启动时生效**。

⚠️ **刻意不做运行中热重载**：会话中途静默改排序权重与治理/审计口径冲突；
仓内另有**受治理的运行时调权**（证据驱动、会话维度、有账）⇒ 两套调权面不得并存。
⇒ ⇒ 与仓内既有纪律一致（见 `docs/architecture/contract.toml` 的「配置面的用法纪律」）。

### ④ 回退

* 回退 = **撤销** ⇒ 不需要审批（它是**撤销**方向），但**同样留痕归档**（`kind = "rollback"`）；
* 回退**不重跑影子**（证据只用来支持「换」）⇒ ⇒ 回退恒比切换快；
* 回退会把**生产旋钮写回上一版**（上一版旋钮存在指针里）⇒ ⇒ **回退真的能回退**。

## 五、装配根那一行（**主指挥统一加**）

在 `crates/engine/runtime-assembly/src/canonical/production.rs` 的
「Unified Memory 2.0 coordinator wiring」段里，`memory` / `governance` 两个句柄拿到之后：

```rust
            let _shadow = shadow_wire::install_shadow(
                Arc::clone(&memory),
                Arc::clone(&governance),
                &default_data_root(),
                None,
            )
            .map_err(RuntimeError::misconfigured)?;
```

⚠️ 三个参数说明：
* `memory` / `governance` = **现役那套**只读库句柄（装配手里就有 ⇒ 影子侧能读同一批库）；
* `&default_data_root()` = 数据根（`shadow.json` / 归档 / 指针 / 旋钮都落这里）；
* `None` = **治理链**（**没有 ⇒ 不许换**，失败闭合）。
  ⇒ 若装配根手里**有**治理链句柄 ⇒ 把 `None` 换成 `Some(Arc::clone(&chain))`
  ⇒ 切换才会真正放行（**没有治理面就永远换不了**，这是刻意的）。

## 六、证据（真跑输出）

| 用例 | 断言 | 结果 |
|---|---|---|
| `without_a_config_file_nothing_is_constructed_and_nothing_is_written` | 默认态：不构造服务 + 数据根零新增文件 | ✅ |
| `a_broken_config_refuses_to_assemble` | 坏配置 ⇒ 拒绝装配（带真因） | ✅ |
| `the_shadow_never_changes_the_production_result_and_archives_a_report` | 影子跑完：指针没出现、旋钮没被写、归档目录有 1 份报告、报告无答案字段、10 道探针 ⇒ 10 对、成本与库级口径一致、现役侧**真的召回到了东西**（防空转） | ✅ |
| `a_swap_without_evidence_is_refused_and_changes_nothing` | 没有标准答案 ⇒ 没证据 ⇒ 拒绝切换；指针不变、不落盘、旋钮没写 | ✅ |
| `a_swap_without_a_governance_surface_is_refused` | 没有治理链 ⇒ 拒绝（理由点名治理面） | ✅ |
| `a_swap_denied_by_governance_is_refused` | 治理拒绝 ⇒ 拒绝 | ✅ |
| `a_governed_swap_and_its_rollback_both_land` | 治理放行 + 三道闸过 ⇒ 真的切换（指针 current=候选 / previous=现役 / 代次 1 / 生产旋钮落盘）⇒ 回退后指针与旋钮都回到上一版、代次不变 | ✅ |
| `short_circuiting_the_wiring_would_break_the_enabled_cases` | 装配被短路 ⇒ 依赖服务的用例全红（**反证**） | ✅ |

**⭐ 反证（已实测红过）**：把 `install_shadow` 短路成「永远 `Ok(None)`」⇒
**8 个用例里 6 个变红**（只有「默认关」与「坏配置」两条仍绿 —— 它们**本来就不该依赖服务**）
⇒ ⇒ 证明这些用例**真的在测接线**，不是恒绿断言。

**门禁自查**：`cargo fmt -p apeireth-runtime-assembly -- --check` 干净 ·
`cargo clippy -p apeireth-memory -p apeireth-runtime-assembly --all-targets -- -D warnings` 0 warning ·
`cargo test -p apeireth-memory --lib` **961 passed / 0 failed / 1 ignored** ·
`cargo test -p apeireth-runtime-assembly --lib` **429 passed / 0 failed** ·
`cargo test -p apeireth-runtime-assembly --test shadow_swap_production` **8 passed** ·
`python3 scripts/arch_guard.py --strict` **VIOLATIONS 0**（棘轮文件零增长）·
`python3 scripts/audit_unwired.py --check` ⇒ 本批**未新增孤儿**
（当前报出的 `view_summary` 属**另一条并行线**的 `canonical/memory_contracts.rs`，不是本批）。

## 七、边界 / 没测什么（如实）

1. **工具面 / CLI 出口未接**：本批交付的是**装配根可达的服务 + 进程级槽**
   （`shadow_wire::service()`）；**没有**新工具、没有 CLI 子命令 ⇒ 操作者目前需要**程序内调用**
   ⇒ ⇒ 「换算法」这件事**还没有面向人的入口**（**这是本批最明确的残余缺口**）。
2. **治理链未由装配根提供**：装配根那一行现在传 `None` ⇒ ⇒ **切换会被拒绝**（失败闭合）；
   要真正放行，需要装配根手里**有**治理链句柄 ⇒ 那属于**装配根**的接线范围（本批不改）。
3. **候选侧权重基线 = 默认权重**：生产基线若被运行时调权改过 ⇒ 影子结论按「默认基线」解读。
4. **token 是字符数代理**（仓里没有分词器）⇒ 只用于**两侧相对比较**，不可与外部 token 数对齐。
5. **样本充分闸要求 ≥10 题 / ≥2 轮**：本批的探针题由**运维**在 `shadow.json` 里给
   ⇒ ⇒ 没有探针 ⇒ **裁决必然拒绝**（没证据 ≠ 没变差）⇒ 这是刻意的。
6. **切换在下次启动生效**（不做热重载）⇒ ⇒ 「已切换」与「已生效」是**两个时刻**，中间有一次重启。
7. **未测**：多会话并发下的影子一致性；影子与运行时调权同时生效时的口径；归档目录的**磁盘上限**
   （只做**份数**轮转，不做**字节**上限）；候选侧换**召回算法**（本批只覆盖**重排旋钮**这一类变体）。
8. **本批没改任何既有记忆/检索文件** ⇒ 既有行为**逐字不变**（默认态零行为变化已实测）。

## 八、关联材料

- 库级实现（未改语义）：`crates/engine/memory/src/shadow_run.rs` · `crates/engine/memory/src/swap_flow.rs`
- 接线模块：`crates/engine/runtime-assembly/src/canonical/shadow_wire.rs`
- 集成测试：`crates/engine/runtime-assembly/tests/shadow_swap_production.rs`
- 配置面纪律：`docs/architecture/contract.toml`（`[env]` 段的「配置面的用法纪律」）
- 生产旋钮面：`crates/engine/memory/src/ranking_knobs.rs`（`retrieval.json`）
