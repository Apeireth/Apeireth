[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-b2-production-wiring-2026-11-08.md
Class: EVIDENCE
Status: 🟢 已核验（本地真跑；远端 CI 待复核）
Last-Modified: 2026-11-08
Author: lead

# B2 批：三接口契约 + `rebuild` 的**生产接线**（2026-11-08）

> **四级口径**：本页记录**接线与证据**；每条能力的等级以 `docs/status/claims.yaml` 为准。
> 本页**不**宣称任何 **HARDWARE VALIDATED**（无真机、无真模型 E2E）。
>
> ⚠️ **中性表述**：只描述**机制**（读契约 / 标注契约 / 注入契约 / 派生索引重建），不引用任何外部产品名。

## 〇、⚠️ 一条**口径纠正**（我此前的说法错了）

批次页把 B2 写成「🟡 库级完成，**生产未接**」⇒ 本批开工前的口径是「三契约生产零调用」。
**核实后：注入契约其实已经在生产路径上**（`context_compiler::compile_injection_with_renderer`
用契约渲染器出注入，且有逐字等价的安全网 `tests/injection_renderer_production.rs`）。

⇒ ⇒ **真正没接的是两条**：**读契约**（`MemoryView`）与**标注契约**（`MemoryAnnotation`）。
⇒ ⇒ 本批做的是：① 把这两条接进生产读路径；② 给 `rebuild` 一个**运维入口**；
③ 给装配根加一道**会咬人**的门（口径对齐自检）。

---

## 一、三条契约各自的语义与生产落点

| 契约 | 它约束什么 | 生产落点（file:line） | 本批状态 |
|---|---|---|---|
| **读契约** `MemoryView` | 算法**只能**通过它读；**不许直接碰库**；可见性判定**在契约里**（不在算法里） | `crates/engine/memory/src/context_compiler.rs` 的 `contract_view()` ⇒ 编译前把选中集包成 `SelectedMemoryView`，编译从视图的**无损投影**取条目 | ⭐ **本批接入** |
| **标注契约** `MemoryAnnotation` | 算法**只能产出标注**；标注是**纯数据**（类型上**没有**任何回写原始层的方法） | 同一步：每条被选中的记忆产出一条分数标注 + 一条出处标注 | ⭐ **本批接入** |
| **注入契约** `InjectionRenderer` | **检索语义不变、输出形态可换**；预算纪律三条（按相关度降序装入 / 装到预算就停 / 截断必须如实标注） | `context_compiler::compile_injection_with_renderer`（既有） | ✅ **已在生产**（本批核实） |

## 二、接线的**核心约束**：**默认行为逐字节不变**

- 契约视图是**无损投影**（条目原样带回）⇒ 编译产物与接线前**逐字节相同**；
- 可见性口径与 canonical `MemoryScope::is_visible_in` **逐条一致** ⇒ 复核**不改变**任何东西
  （两套口径一致 ⇒ 复核是恒等变换，这正是它该有的性质）；
- 契约视图里记录的范围是「**已授权**占位」⇒ 真正的范围判定**仍在上游 canonical**
  ⇒ 契约视图**不放宽**任何东西（它只装已授权的条目）。

**证据**：`crates/engine/memory/tests/memory_contract_production.rs`
用**写死的期望字符串**钉住 overlay 的逐字节形态；`crates/engine/runtime-assembly/tests/memory_contract_production.rs`
从**生产协调器**跑一遍召回→编译，断言形态不变。

## 三、「契约违例被真的挡住」的两道门

| 门 | 触发条件 | 后果 |
|---|---|---|
| **口径对齐自检** | 契约的 `scope_visible` 与 canonical 的 `is_visible_in` **分叉** | **装配失败**（进程起不来）⇒ 不靠人记得 |
| **渲染器预算校验** | 渲染器产出**超预算**的 `Injection` | **报错**（不照用）⇒ 上下文被撑爆时上游预算口径不失效 |

口径自检覆盖 **5 类范围 × 5 类授权范围 = 25 组**，在**装配时**逐组比对
（`memory_contract_wire::install()` ⇒ `assert_scope_alignment()`）。

## 四、`rebuild`：接法与「重建前后一致」的证据

### 接法：**运维入口**（**不进读路径**）
- 重建是**运维动作**（删派生索引 → 重建 → 比对）⇒ ⇒ 放进每请求的读路径会把一次运维变成
  「每次召回都重建一遍」⇒ ⇒ 读路径**只读**，重建走新入口
  `vector_rebuild::rebuild_and_verify_retrieval(...)`。
- 既有 `rebuild_vector_index` 的纪律**逐字保留**：先读旧向量 ⇒ 再 embed（失败 ⇒ **索引一个字节都没动**）
  ⇒ 再清空重写 ⇒ 落模型标识 ⇒ **回读落盘结果**再比对。

### 一致性证据（真跑）
`crates/engine/memory/tests/memory_contract_rebuild_consistency.rs`：
1. 首次建索引 ⇒ `NoBaseline`（没有「重建前」可比）；
2. 重建 + 复核 ⇒ `RebuiltIdentical`，且**重建前后的 top-k id 序列逐项相同**
   ⇒ ⇒ **重建没有改变召回**（比「向量逐条相同」更强：向量相同但排序口径变了，结果也会变）；
3. **模型换了** ⇒ 报 `ModelChanged`（**换代**），**不假装一致**。

### 诚实边界
- 「重建前后一致」只在**同模型同输入**下成立；生产向量若由外部模型提供 ⇒ 模型换版是**换代**，不是重建；
- 生产当前的向量是**按需现算**（episode 元数据里的 `vector` 缓存；内容一变即**主动作废**）
  ⇒ ⇒ **生产没有常驻的派生向量索引文件** ⇒ 运维入口面向的是**文件形态**的派生索引；
  ⚠️ 这一条是**如实口径**，不是「生产已经在用重建」。

## 五、反证（红绿记录）

| 反证 | 做法 | 结果 |
|---|---|---|
| 读契约真在路径上 | 把 `context_compiler::contract_view()` 短路（不计数、不建视图） | `memory_contract_production.rs` **2 条红**（契约计数为 0 / 注入路径断言失败）⇒ 恢复即绿 |
| 装配自检真在跑 | 自检返回的组数断言 = 25 | 组数不为 25 ⇒ 红（证明不是空跑） |
| 重建一致性真在跑 | 重建后 top-k 序列与重建前逐项比对 | 不一致 ⇒ 红 |

## 六、给装配根的**那一行**（由主代理行中性加入）

```rust
let _contracts = memory_contract_wire::install().map_err(RuntimeError::misconfigured)?;
```

- 位置：`crates/engine/runtime-assembly/src/canonical/production.rs`，记忆协调器构造之后（建议紧跟
  `coordinator = coordinator.with_memory_injection_format();` 那一段）；
- 行数：**+1 行**（`production.rs` 现行 1896 行，棘轮只许不增 ⇒ 需在同文件腾出 1 行）；
- 作用：把**口径对齐自检**挂到装配 ⇒ 两套可见性口径一旦分叉 ⇒ **拒绝启动**。

## 七、复现命令（原样可跑）

```sh
cd /home/bai/wt-apeireth-r3
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= bash -c '
  cd /home/bai/wt-apeireth-r3 &&
  cargo test -p apeireth-memory --lib --offline -j 2 && \
  cargo test -p apeireth-memory --test memory_contract_production --offline -j 2 && \
  cargo test -p apeireth-memory --test memory_contract_rebuild_consistency --offline -j 2 && \
  cargo test -p apeireth-runtime-assembly --lib --offline -j 2 && \
  cargo test -p apeireth-runtime-assembly --test memory_contract_production --offline -j 2
'
```

## 八、没证明的（如实）

1. **读契约的「窄面」目前是名义上的**：编译器仍需要**宽条目**（跨日记片段等）
   ⇒ ⇒ 视图同时持有原条目 ⇒ 「算法只能读窄面」这条约束**要等真有算法插进来**才完全兑现；
   现在兑现的是「**选中集经由契约视图**」与「**标注是纯数据**」。
2. **视图里的范围是「已授权」占位**（召回结果不携带每条记录的范围）⇒ 真正的范围判定仍在上游。
3. **口径自检只覆盖 25 组范围组合**，不覆盖「同范围不同 id」这类内容维度（内容维度本来就不在可见性口径里）。
4. **重建一致性的证据用的是测试用确定性 embedder** ⇒ **没有**在真实外部模型上跑过
   ⇒ ⇒ 真实模型下「同模型重建逐位一致」依赖 embed 的确定性，**未验证**。
5. **`rebuild` 未接进生产运维命令**（无 CLI 出口）⇒ 运维入口目前是**库级 API**。
6. **未测**：契约视图与跨日记片段/知识库节同时在场时的交互（本次只改了默认 XML 与注入形态两条路径的主循环）。

## 九、关联材料

- 契约实现：`crates/engine/memory/src/memory_view.rs` · `crates/engine/memory/src/injection_renderer.rs`
  · `crates/engine/memory/src/canonical/memory_contracts.rs`（**本批新增**：生产视图 + 标注产出 + 口径自检 + 渲染校验）
- 装配接线：`crates/engine/runtime-assembly/src/canonical/memory_contract_wire.rs`（**本批新增**）
- 生产读路径：`crates/engine/memory/src/context_compiler.rs`（**本批改**：编译前过契约视图 + 契约计数诊断面）
- 重建运维入口：`crates/engine/memory/src/vector_rebuild.rs`（**本批改**：`rebuild_and_verify_retrieval`）
- 测试：`crates/engine/memory/tests/memory_contract_production.rs` · `crates/engine/memory/tests/memory_contract_rebuild_consistency.rs`
  · `crates/engine/runtime-assembly/tests/memory_contract_production.rs`
