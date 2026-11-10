# L3 骨架声明审计（7 条）· 2026-11-06

<!-- [Document-Meta]
Class: AUDIT
Title: L3 骨架声明（7 条）离 PRODUCTION WIRED 还差什么
Owner: runtime / governance
Date: 2026-11-06
Scope: 只读取证 + 出计划（不改生产代码、不改 docs/status/claims.yaml）
-->

> **口径（四级）**：`NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED`
> （阶梯定义见 [docs/01-architecture/system-capabilities.md](../../01-architecture/system-capabilities.md) 头部；
> `DEFAULT ENABLED` **不是一级**，是横切属性，由 `default: "ON"|"OFF"|null` 表达 ——
> **可以 PRODUCTION WIRED 且默认关**）。
> 本文档全篇按四级口径书写；每条结论都带 **文件:行号**。
>
> **纪律**：本次是**只读取证 + 出计划**。
> ⇒ ⇒ **没有**改 `docs/status/claims.yaml`（声明面是主代理的决策面）。
> ⇒ ⇒ **没有**改任何生产代码。
> ⇒ ⇒ **没有**跑 `cargo build/nextest/clippy`（磁盘 85%，只允许 grep/read + python 守卫）。
> ⇒ ⇒ 复核命令见 §6。

---

## 0. 结论摘要（一句话版）

⭐ **7 条的 `level: IMPLEMENTED` 全部如实** —— 逐条用「生产调用点 grep」核过，
**没有一条**是「其实已在生产链路上、只是口径没更新」。
⇒ ⇒ ⇒ **「只差口径升级」的条数 = 0 条**（严格口径，见 §3 的「宽口径声明」附加说明）。

但 **成本差一个数量级**：

| 档 | 条 | 差什么 | 预计改动面 |
|---|---|---|---|
| **A（最便宜）** | `gov.reflection_queue` | 装配「延迟反射调度器」+ 旋钮 | 1 生产文件 + 1 旋钮测试 |
| **B（宿主面已在，只缺一个生产消费者）** | `rt.owned_agents`、`mem.registered_namespace` | 在生产里注册一个**会创建子 Agent 的模块** | 1 生产装配 + 1 依赖（System One） |
| **C（依赖链末端）** | `rt.system_one_internal` → `rt.module_execution_contract` → `rt.module_messages` → `rt.mesh_local_worker` | 4 条互相有硬依赖 | 见 §5 依赖图 |

---

## 1. 方法（可复算）

**取证口径**：本仓「已接线」的判据写在 [docs/status/unwired-ledger.md](../../status/unwired-ledger.md) 头部 ——
「**PRODUCTION WIRED = 生产装配有构造/调用点且实测跑过**」。
⇒ ⇒ 本审计**不**采信 `docs/status/wiring.yaml` 的 `production_refs` 字段：
它由 `scripts/wiring_report.py:438 is_production_source()` 生成，判据是**路径前缀 + 不在 `/tests/` 下**
⇒ ⇒ 会把 `crates/engine/runtime/src/canonical/instances.rs` 这类**库文件**也算成「生产引用」
⇒ ⇒ **它不是接线证据**（本审计全程不引用它作结论）。

**核心复核命令**（全部只读）：

```bash
cd /home/bai/wt-apeireth-r3

# ① 生产装配根在哪（CLI 是组合根，网关复用它，SDK 只是 HTTP 客户端）
grep -rn "fn build_canonical_runtime_with_parts" crates/adapters/cli/src/lib.rs
grep -rn "with_governance\|desktop_agent_policy\|register_with_policy\|build_desktop_main_runtime" \
     crates/adapters/cli/src/lib.rs

# ② 每条骨架的「生产装配开关」在生产里到底有没有被调
grep -rn "with_reflection_scheduler" crates/ --include=*.rs        # gov.reflection_queue
grep -rn "with_system_one"       crates/ --include=*.rs            # rt.system_one_internal
grep -rn "message_policy"        crates/ --include=*.rs            # rt.module_messages
grep -rn "tool_module_with_contract" crates/ --include=*.rs        # rt.module_execution_contract
grep -rn "runtime_assembly::mesh\|MeshModule::new" crates/ --include=*.rs  # rt.mesh_local_worker
grep -rn "MemoryModule::new\|register_with_profile_mappings" crates/ --include=*.rs  # mem.registered_namespace
grep -rn "spawn_agent" crates/ --include=*.rs                     # rt.owned_agents 的唯一生产消费者
```

---

## 2. 七条逐条审计表（总表）

| # | claim id | 现口径 | `default` | 现 `verify_cmd` 是否「窄」 | 生产调用点（实测） | 口径是否如实 | 离 PRODUCTION WIRED 还差什么 |
|---|---|---|---|---|---|---|---|
| 1 | `gov.reflection_queue` | IMPLEMENTED | `null` | ❌ **宽**（`cargo test -p apeireth-governance --lib` = 全 crate 241 个单测） | ❌ 无（生产用默认 `ImmediateReflectionScheduler`） | ✅ 如实 | 装配 `DelayedReflectionScheduler` + 旋钮 |
| 2 | `rt.owned_agents` | IMPLEMENTED | `null` | ✅ 窄（3 个 test target） | ⚠️ **宿主授权已在生产面**，但「模块自建 Agent」无生产消费者 | ✅ 如实 | 生产里注册一个会调 `spawn_agent` 的模块 |
| 3 | `mem.registered_namespace` | IMPLEMENTED | `null` | ✅ 窄（2 个 test target） | ❌ 无（`MemoryModule::new` 只在测试） | ✅ 如实 | 生产装配注册 `MemoryModule` + Profile 映射 |
| 4 | `rt.module_execution_contract` | IMPLEMENTED | `OFF` | ✅ 窄（2 个 test target） | ❌ 无（`tool_module_with_contract` 只在测试） | ✅ 如实 | 生产侧改用 `tool_module_with_contract(mode)` |
| 5 | `rt.system_one_internal` | IMPLEMENTED | `OFF` | ✅ 窄（2 个 test target） | ❌ 无（`with_system_one` 只在测试） | ✅ 如实 | CLI 组合根装配 `SystemOneGate` + supervisor |
| 6 | `rt.module_messages` | IMPLEMENTED | `OFF` | ✅ 窄（1 个 test target） | ❌ 无（`message_policy` 生产恒 `None`） | ✅ 如实 | 宿主策略填 `message_policy` + grants |
| 7 | `rt.mesh_local_worker` | IMPLEMENTED | `OFF` | ✅ 窄（2 个 test target） | ❌ 无（`mesh` 只在测试被引用） | ✅ 如实 | 生产装配构造 `MeshModule`（loopback peer） |

**全 7 条的 `verify_cmd` 都能跑通吗？**
⇒ ⇒ **未跑**（磁盘 85%，禁止 cargo）。
⇒ ⇒ 改用**读代码**判断：**7 条的 `verify_cmd` 引用的 test target 全部存在**（逐个 `ls` 过，见 §4 表最后一列），
且 `cargo test --test <name>` 形态自洽 ⇒ **不会因「目标不存在」而红**。
⇒ ⇒ ⚠️ **但**：`gov.reflection_queue` 那条是 **crate 级宽命令**（见 §4.1），
它今天**可能**因 governance crate 里 244 个与本声明无关的单测中的任意一条红而红
⇒ ⇒ ⇒ 这正是「捎带别人的红」。

---

## 3. ⭐ 「只差口径升级」的判定（重点）

### 3.1 判定结论：**0 条**

逐条把「生产装配开关」grep 一遍，结果如下（**只列生产面**）：

| claim | 装配开关 | 生产面调用点 | 判定 |
|---|---|---|---|
| `gov.reflection_queue` | `GovernanceChain::with_reflection_scheduler`（`crates/foundation/governance/src/chain.rs:566-571`） | **零** —— 全仓只有 `chain.rs:1273` / `chain.rs:1309` 两个**同文件单测**在用 | 未接线 |
| `rt.system_one_internal` | `RuntimeBuilder::with_system_one`（`crates/engine/runtime/src/canonical/runtime.rs:715-721`） | **零** —— 只有 `crates/engine/runtime-assembly/tests/**` 在用 | 未接线 |
| `rt.module_messages` | `AgentHostPolicy.message_policy`（`crates/engine/runtime/src/canonical/instances.rs:55`） | **零** —— 生产恒 `None`（`instances.rs:74`） | 未接线 |
| `rt.module_execution_contract` | `CapabilityProvider::tool_module_with_contract`（`crates/engine/runtime/src/canonical/capability.rs:43-51`） | **零** —— 只有 `crates/engine/runtime-assembly/tests/modular_ale.rs:46` | 未接线 |
| `rt.mesh_local_worker` | `MeshModule::new`（`crates/engine/runtime-assembly/src/mesh.rs:141`） | **零** —— 只有 `crates/engine/runtime-assembly/tests/mesh_execution.rs:15` | 未接线 |
| `mem.registered_namespace` | `MemoryModule::new`（`crates/engine/runtime-assembly/src/canonical/memory_module.rs:89`） | **零** —— 只有 `crates/engine/runtime-assembly/tests/**` | 未接线 |
| `rt.owned_agents` | `ModuleAgents::spawn_agent`（`crates/foundation/plugin/src/agents.rs:212`） | **零** —— 唯一的库内生产消费者是 `memory_module/organizer.rs:57-95`，而 `MemoryModule` 自己没进生产 | 未接线 |

⇒ ⇒ ⇒ **7/7 的 IMPLEMENTED 都如实**（0 装纪律的「双向性」守住了：不把「实现了没接线」说成「接了」）。

### 3.2 ⚠️ 但有**半条**接近「只差口径升级」：`rt.owned_agents`

`rt.owned_agents` 的标题是「**同一 Runtime 的模块拥有独立 Agent 与分层核算**」——
这句话里其实有**三个**子命题，其中**两个已经被 PRODUCTION WIRED 的兄弟声明覆盖**：

| 子命题 | 现在的口径 | 证据 |
|---|---|---|
| 同一 Runtime（受管 Host） | ✅ **PRODUCTION WIRED** | `rt.desktop_managed_main`（claims.yaml:934）——它的 `sources` 与本 claim **几乎逐个相同**（`instances.rs` / `accounting.rs` / `instances/main.rs` / `desktop_host.rs` / `cli/src/lib.rs` / `care_heartbeat.rs` / `execute.rs`） |
| 分层核算（按 owner 归集用量） | ✅ **PRODUCTION WIRED** | `crates/engine/runtime/src/canonical/accounting.rs:531 aggregate(owner)`；由 `rt.desktop_managed_main` 覆盖 |
| **模块拥有独立 Agent**（模块自主创建子 Agent） | ❌ **IMPLEMENTED** | 生产里无 `spawn_agent` 调用点（§3.1 最后一表） |

⇒ ⇒ **`rt.owned_agents` 的 notes 自己就承认了这一点**：
> 「2026-10-29 S1：SubLoop 路径为 PRODUCTION WIRED；…**桌面 managed 默认及全部 Legacy 迁移仍未完成，维持 IMPLEMENTED 口径**。」
> 「2026-10-29 S5：…**该默认路径另有独立 PRODUCTION WIRED claim，宽泛本项维持 IMPLEMENTED**，整体仍 Partial。」

⇒ ⇒ ⇒ ⭐ **最便宜的口径动作（建议，主代理决策）**：
**把 `rt.owned_agents` 收窄成只声明「模块拥有独立 Agent」**（把「同一 Runtime / 分层核算」让给
`rt.desktop_managed_main`），并把「全部 Legacy 迁移」另立一条。
⇒ ⇒ 收窄后它只剩**一个**缺口：**生产里有一个模块真的创建子 Agent**。
⇒ ⇒ ⚠️ 这条改的是 `claims.yaml` 的 `title`/`sources`/`notes` ⇒ **属主代理决策面，本次只报告不动**。

### 3.3 另一条「口径便宜」的动作：`gov.reflection_queue` 的 `verify_cmd`

这条**不改口径**也能立刻降本：把 crate 级宽命令收窄到只跑自己那 6 个测试（§4.1），
**零代码改动**，当天可做。

---

## 4. 逐条详细审计

### 4.1 `gov.reflection_queue` — 「R8-C 进程内延迟反思队列——默认未装配、零行为变化」

**口径**：`IMPLEMENTED`（`docs/status/claims.yaml:356-370`）；`default: null`；`switch_env: null`
**sources / tests**：同一文件 `crates/foundation/governance/src/chain.rs`
**现 `verify_cmd`**：`cargo test -p apeireth-governance --lib` ⇒ ❌ **宽命令**

**⚠️ 宽命令实证**：`cargo test -p apeireth-governance --lib` 跑的是 **governance crate 全 lib 单测**
= 24 个源文件、**241** 个 `#[test]`/`#[tokio::test]`；而与本声明相关的只有 `chain.rs` 里的 **6** 个：

| chain.rs 行号 | 测试名 |
|---|---|
| 1007 | `allow_traverses_gate_audit_and_reflection` |
| 1235 | `delayed_scheduler_releases_each_due_reflection_once` |
| 1258 | `delayed_scheduler_take_due_now_uses_its_clock` |
| 1268 | `chain_audits_due_reflections_before_deciding` |
| 1289 | `default_scheduler_never_reports_due_reflections` |
| 1303 | `chain_fails_closed_when_due_reflection_audit_fails` |

⇒ ⇒ 其余 **235** 个测试任一红，本声明的 `verify_cmd` 就红 ⇒ **典型「捎带别人的红」**。

**「零行为变化」的实证（口径如实的依据）**：
* 生产链的构造点：`crates/adapters/cli/src/governance_chain.rs:16-22`
  `GovernanceChain::new(gate).with_audit_sink(audit)` —— **没有** `.with_reflection_scheduler(...)`；
* 生产链的挂载点：`crates/adapters/cli/src/lib.rs:1653`
  `builder = builder.with_governance(governance_chain::production(containment));`
  （`production()` 见 `crates/adapters/cli/src/governance_chain.rs:31-34`，全仓只被这一处用）；
* ⇒ ⇒ 生产用的是默认 `ImmediateReflectionScheduler`（`chain.rs:534`）：
  `schedule()` 只发一个 token（`chain.rs:350-361`），`take_due()` 走 trait 默认实现**恒返回空**
  （`chain.rs:315-317`）⇒ ⇒ `sweep_due_reflections`（`chain.rs:806-819`）是 **no-op**；
* ⇒ ⇒ **「默认未装配、零行为变化」与代码事实一致** ⇒ `IMPLEMENTED` 如实。

**⭐ 接线清单（要真装配，需要改这几处）**：

| # | 位置（文件:行号） | 改什么 |
|---|---|---|
| 1 | `crates/adapters/cli/src/governance_chain.rs:16-22` | `adapt_gate_with` 里给 `GovernanceChain` 注入 `DelayedReflectionScheduler`（`chain.rs:370-468`）。⚠️ `adapt_gate_with` 全仓只被同文件 `production()` 调用（`:33`）⇒ ⇒ **改这一处即生产面** |
| 2 | `crates/adapters/cli/src/governance_chain.rs:31-34` | `production()` 需要拿到「延迟毫秒」⇒ 改成 `production_with_reflection(gate, delay_ms)` 或加参数；**旋钮关时必须传 `None`**（⇒ 继续用默认 `Immediate` ⇒ 零行为变化） |
| 3 | `crates/adapters/cli/src/lib.rs` | 加 env 旋钮常量与读取函数（照抄 `crates/adapters/cli/src/lib.rs:143` `ENABLE_TASK_BOARD_ENV` + `:774-777` `task_board_enabled_from_env()` 的形态）。⚠️ **默认关** ⇒ 零行为变化；开关名建议 `APEIRETH_ENABLE_GOV_REFLECTION`（建议值，属主代理定） |
| 4 | 新测试 `crates/adapters/cli/tests/governance_reflection_knob.rs` | 旋钮关 = 到期审计零条；旋钮开 = 到期反思**恰好一次**落审计（`governance.reflection_due`，`chain.rs:815`） |

**⚠️ 三个诚实边界（写口径时必须带上，否则会被口径守卫打回）**：
1. **title 与口径会打架**：本 claim 的 title 明写「默认未装配、零行为变化」。
   ⇒ 真装配后（哪怕默认关）title 必须同步改写，例如
   「R8-C 进程内延迟反思队列——**默认未装配；旋钮开启后按到期审计留痕**」。
2. **装配后能做的只是「到期审计留痕」，不是「跑反思」**：
   `chain.rs:336-338` 明写「An actual reflection runner is R8-D and is **NOT IMPLEMENTED**」。
3. **到期是惰性的**：`sweep_due_reflections` 在**下一次 `GovernanceChain::run`** 开头触发（`chain.rs:612-619`）
   ⇒ ⇒ 没有后台线程/守护 ⇒ 「延迟队列」= 下一次治理评估时扫一遍。

**建议 `verify_cmd`**：
* 收窄版（今天就能改，零代码改动）：
  `cargo test -p apeireth-governance --lib -- reflection && cargo test -p apeireth-governance --lib -- scheduler`
  ⇒ 命中恰好上面那 **6** 个（`-- reflection` 命中 5 个、`-- scheduler` 补上 `:1258` 那个名字里没有 "reflection" 的）。
* 升级版（接线后）：
  `cargo test -p apeireth-governance --lib -- reflection && cargo test -p apeireth-governance --lib -- scheduler && cargo test -p apeireth-cli --test governance_reflection_knob --locked -j 2`

---

### 4.2 `rt.owned_agents` — 「同一 Runtime 的模块拥有独立 Agent 与分层核算」

**口径**：`IMPLEMENTED`（`claims.yaml:844-878`）；`default: null`
**现 `verify_cmd`**：`cargo test -p apeireth-runtime --test cognitive_module_abi --locked -j 2 && cargo test -p apeireth-runtime --test canonical_subloop --locked -j 2 && cargo test -p apeireth-runtime-assembly --test convergence_agents --locked -j 2`
⇒ ✅ **窄**（3 个具名 test target，不是全 workspace）。
⚠️ 但它**不含生产面证据**（`crates/adapters/cli/tests/canonical_cli_bootstrap.rs` 没进 verify_cmd），
而本 claim 的 `tests` 列表里有它 ⇒ ⇒ 建议补。

**生产装配现状（宿主面已在，缺消费者）**：

| 环节 | 位置 | 状态 |
|---|---|---|
| 宿主给每个模块发子 Agent 授权 | `crates/engine/runtime-assembly/src/canonical/desktop_host.rs:22-37`（`policy.modules.insert(... ModuleAgentGrant ...)`） | ✅ **在生产面**（被 `crates/adapters/cli/src/lib.rs:1710` `cognitive.desktop_agent_policy()?` 调） |
| 生产 Runtime 用这份 policy 建实例宿主 | `crates/adapters/cli/src/lib.rs:1765-1770` → `desktop_host.rs:55-82` `build_desktop_main_runtime` → `builder.build_agent_runtime(policy)` | ✅ 在生产面 |
| 运行时给模块发 `ModuleAgents` 端口 | `crates/engine/runtime/src/canonical/instances.rs:106-124`（`Factory::bind` → `BoundModule`）；`instances.rs:274-278` 装 `agent_factory`；`crates/engine/runtime/src/canonical/runtime.rs:1071-1072` 传给插件上下文；`crates/engine/runtime/src/canonical/instances/hot.rs:24`；`crates/foundation/plugin/src/plugin.rs:63-66`（`ctx.agents`） | ✅ 在生产面 |
| 运行时自己给模块发 SubLoop spawner | `crates/engine/runtime/src/canonical/execute.rs:2476-2493`（`RuntimeSubLoopSpawner::new`） | ✅ 在生产面（hooks 路径） |
| **模块自己真的去建子 Agent** | `ModuleAgents::spawn_agent`（`crates/foundation/plugin/src/agents.rs:212`） | ❌ **生产零调用点** |

⇒ ⇒ 唯一的库内生产消费者是
`crates/engine/runtime-assembly/src/canonical/memory_module/organizer.rs:57-95`
（`ctx.agents.spawn_agent(...)`，失败文案「Host did not enable module-owned agents」），
而 `MemoryModule` 自己**从未在生产里被构造**（见 §4.3）。
⇒ ⇒ ⇒ **「模块拥有独立 Agent」这个可观察行为，生产里不会发生** ⇒ `IMPLEMENTED` 如实。

**⚠️ 口径重叠（重要）**：`rt.owned_agents` 的 `sources` 与 `rt.desktop_managed_main`（PRODUCTION WIRED）
**几乎逐个相同** ⇒ ⇒ 这条是个**宽口径声明**，把已升级的子项与未接线的子项捆在一起。
⇒ ⇒ 建议（主代理决策）：**收窄 title**（§3.2）。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/engine/runtime-assembly/src/canonical/production.rs`（`ProductionModules::build` / `build_inner`，`:771` / `:781` 起）或 `crates/adapters/cli/src/lib.rs` 的组合根（`:1603-1770`） | **注册一个会创建子 Agent 的模块**。最现成的候选是 `MemoryModule`（`crates/engine/runtime-assembly/src/canonical/memory_module.rs`），其 organizer 已实现 `spawn_agent` ⇒ ⇒ **与 §4.3 是同一笔改动** |
| 2 | `crates/engine/runtime/src/canonical/instances.rs:170-199` | **不用改**：授权闸读 `policy.modules`，已由 `desktop_host.rs:27-36` 满足 |
| 3 | `crates/adapters/cli/src/lib.rs:1710` | **不用改**：`desktop_agent_policy()` 已在生产面 |
| 4 | 新测试 `crates/adapters/cli/tests/owned_module_agent_knob.rs` | 生产 bootstrap 里能查到一个 `AgentOwner::Module{..}` 的实例（现成的断言范例：`crates/engine/runtime-assembly/tests/convergence_agents.rs:252` / `:688`） |

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2 && cargo test -p apeireth-runtime-assembly --test convergence_agents --locked -j 2`
（⇒ 用**生产 bootstrap** 的断言当主证据；`convergence_agents` 留作库面定向。）

---

### 4.3 `mem.registered_namespace` — 「注册式 Memory 工具、固定 Hook 与通用 Profile 隔离」

**口径**：`IMPLEMENTED`（`claims.yaml:887-912`）；`default: null`
**现 `verify_cmd`**：`cargo test -p apeireth-runtime-assembly --test profile_memory_authorization --locked -j 2 && cargo test -p apeireth-runtime-assembly --test convergence_memory --locked -j 2`
⇒ ✅ **窄**。

**生产装配现状**：
* 生产注册路径是 `crates/adapters/cli/src/lib.rs:1712` `cognitive.register_with_policy(builder, &policy)?`
  → `crates/engine/runtime-assembly/src/canonical/cognitive_registry.rs:6-35` → `production.rs:1827-1838` `register_into`
  ⇒ ⇒ **走的是旧 `ProductionCognitiveModules` 的 Memory/Persona/Organ**，不是新 `MemoryModule`；
* `MemoryModule::new`（`crates/engine/runtime-assembly/src/canonical/memory_module.rs:89`）
  ⇒ **生产零调用点**（只有 `crates/engine/runtime-assembly/tests/{convergence_memory,profile_memory_authorization,portable_host,system_one_execution,modular_ale}.rs`）；
* `register_with_profile_mappings`（`crates/engine/runtime-assembly/src/canonical/production.rs:1842-1852`）
  ⇒ **生产零调用点**（唯一调用者是 `crates/engine/runtime-assembly/tests/profile_memory_authorization.rs:195`）；
* 「固定 Hook」在 `crates/engine/runtime-assembly/src/canonical/memory_module.rs:192-198`（`hooks()` → `ModuleHookV1::BeforeModelCall`）
  + `:244-268`（`on_hook`）⇒ ⇒ **随模块一起进生产**，模块不注册则 Hook 也不存在；
* ⇒ ⇒ **七个注册式 Memory 工具、固定 Hook、Profile 隔离，生产里一个都没有** ⇒ `IMPLEMENTED` 如实。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/engine/runtime-assembly/src/canonical/production.rs`（`ProductionModulesConfig` / `ProductionBackends` / `build_inner`） | 加「注册式 Memory 模块」槽：构造 `MemoryModule::new(coordinator, clock)`（`memory_module.rs:89`）+ 走 `register_into`（`production.rs:1827`）或 `register_with_profile_mappings`（`production.rs:1842`） |
| 2 | `crates/adapters/cli/src/lib.rs`（组合根 `:1603-1770`） | 注入 `MemoryCoordinator` + `ProfileMemoryMappings`（`crates/engine/runtime-assembly/src/profile_memory.rs:59`）+ env 旋钮（**默认关** ⇒ 旧 Memory/Persona/Organ 不被替换 ⇒ 逐字节不变） |
| 3 | ⚠️ **硬依赖**：`crates/engine/runtime-assembly/src/canonical/memory_module.rs:59-69` | 七个工具里 5 个是 `EffectClass::Protected`（只有 `organize` 是 `Orchestration`、`organizer_status` 是 `Pure`）⇒ ⇒ **Protected ⇒ `requires_system_one` 为真**（`crates/foundation/plugin/src/system_one.rs:241-244`）⇒ ⇒ **必须先接 §4.5 的 System One**，否则这些工具**每次调用都 `permanent_error`**（`crates/foundation/plugin/src/module_contract.rs:132-138` / `:146-154` / `:161-169`） |
| 4 | 新测试 `crates/adapters/cli/tests/registered_memory_knob.rs` | 旋钮关 = 旧路径逐字节不变；开 = 七个 `tool.memory.*` 注册 + namespace 隔离 + 固定 Hook 生效 |

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-cli --test registered_memory_knob --locked -j 2 && cargo test -p apeireth-runtime-assembly --test profile_memory_authorization --locked -j 2`

---

### 4.4 `rt.module_execution_contract` — 「Module public/private contract and task receipts」

**口径**：`IMPLEMENTED`（`claims.yaml:943-959`）；`default: "OFF"`；`switch_env: null`
**现 `verify_cmd`**：`cargo test -p apeireth-plugin --test module_contract --locked -j 2 && cargo test -p apeireth-runtime-assembly --test modular_ale --locked -j 2`
⇒ ✅ **窄**。

**生产装配现状**：
* 契约面本体在 `crates/foundation/plugin/src/module_contract.rs`（`ModuleToolContract` / `ContractTool` / `ModuleTaskHandle`）；
* 生产拿工具模块走的是 `crates/engine/runtime/src/canonical/capability.rs:37-41` `tool_module()`
  ⇒ ⇒ 所有生产工具的 `module_contract()` 仍是 `Default`（`module_contract.rs:49-60`：`Compatibility` surface + `module_visible: false`）；
* 要换面必须走 `capability.rs:43-51` `tool_module_with_contract(mode)`
  ⇒ ⇒ **该函数的生产零调用点**（唯一调用者 `crates/engine/runtime-assembly/tests/modular_ale.rs:46`）；
* 生产侧的落点是 `crates/engine/runtime-assembly/src/canonical/production.rs:634-647` `collect_tool_module`（调 `provider.tool_module()`）
  —— ⚠️ **这里就是要改的一行**；
* 连带后果（实测）：`EffectClass::Protected` / `ToolSurface::Internal` 在**非测试**代码里只出现在 3 处
  —— `crates/foundation/plugin/src/tool_module.rs:54`、`crates/engine/runtime-assembly/src/canonical/memory_module.rs:64`、
  `crates/engine/runtime-assembly/src/mesh.rs:347-349`
  ⇒ ⇒ **生产里没有任何工具带 Logical/Internal surface**；
* ⇒ ⇒ ⇒ 连带 §4.6 的模块消息（要求目标面是 `Logical`+`module_visible`）与 §4.7 的 Mesh（`Internal`）在生产里**都不可达** ⇒ `IMPLEMENTED` 如实。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/engine/runtime-assembly/src/canonical/production.rs:634-647` `collect_tool_module` | 改调 `provider.tool_module_with_contract(mode)`；`mode` 来自新增的 `ProductionModulesConfig` 字段（Host 显式选面） |
| 2 | `crates/engine/runtime/src/canonical/capability.rs:37-51` | 已有该方法 ⇒ **可能不用改**（只需确认 `ToolModule::with_public_contract` 的 `manifest` 声明路径兼容） |
| 3 | ⚠️⚠️ **硬依赖 + 语义陷阱**：`crates/foundation/plugin/src/tool_module.rs:46-70` | `with_public_contract` 对**非 Agentic** 模式一律给 `EffectClass::Protected`（`:51-55`）⇒ ⇒ **一旦接上，工具立刻变成「需要 System One 审查」** ⇒ ⇒ **必须先接 §4.5**，否则**每一次调用都 `permanent_error`**；**或者**只允许 `ExecutionMode::Agentic`（→ `Orchestration`，不触发 System One）——⚠️ **这是本条最容易踩的坑，建议主代理先裁定策略** |
| 4 | `crates/foundation/plugin/src/module_contract.rs:190-195` `ModuleTaskHandle` | 「任务回执」目前只有两个库内构造点：`memory_module/organizer.rs:100` 与 `mesh.rs:192` ⇒ ⇒ 通用工具面的回执产出面**还没有**（`ContractTool` 不产它）⇒ 若 claim 要含「task receipts」，需在 `ContractTool`/工具侧回填 |
| 5 | 新测试 `crates/adapters/cli/tests/module_contract_knob.rs` | 旋钮关 = 工具面仍是 `Compatibility`（逐字节不变）；开 = 声明面切到 `Logical` 且 `module_visible` 生效 |

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-plugin --test module_contract --locked -j 2 && cargo test -p apeireth-cli --test module_contract_knob --locked -j 2`

---

### 4.5 `rt.system_one_internal` — 「Mandatory System One internal execution review」

**口径**：`IMPLEMENTED`（`claims.yaml:967-985`）；`default: "OFF"`
**现 `verify_cmd`**：`cargo test -p apeireth-runtime-assembly --test system_one_execution --test modular_ale --locked -j 2`
⇒ ✅ **窄**。

**生产装配现状**：
* 运行时开关 `RuntimeBuilder::with_system_one`（`crates/engine/runtime/src/canonical/runtime.rs:715-721`）
  ⇒ **生产零调用点**（只有 `crates/engine/runtime-assembly/tests/{system_one_execution,module_message_pipeline,modular_ale}.rs` 与 `tests/support/system_one.rs:90`）；
* 生产 Runtime 在 `crates/adapters/cli/src/lib.rs:1603-1770` 构造 ⇒ ⇒ `self.system_one = None`（`runtime.rs:709`）；
* ⇒ ⇒ `crates/engine/runtime/src/canonical/dispatcher/system_one.rs:23-29` 的 `gate` 取不到 ⇒ 任何内部执行请求都得到
  `DispatchErrorCode::SystemOneUnavailable`；
* ⇒ ⇒ 但**当前生产没有任何工具需要它**（§4.4 的面表：生产里 `Protected`/`Internal` 零出现）⇒ ⇒ **零行为变化** ⇒ `IMPLEMENTED` 如实；
* 适配器已经齐了：`crates/engine/runtime-assembly/src/system_one.rs:17-30` `FileSystemOneAudit`
  + `:53-94` `AgentSystemOneSupervisor`（Host 显式选 provider/model，走既有 tool-free `SingleShot` 路径）。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/adapters/cli/src/lib.rs`（组合根 `:1603-1770`） | env 旋钮（**默认关**）+ 组装 `SystemOneGate::new(audit, SystemOneLimits::default())` ⇒ `builder = builder.with_system_one(gate)`（`runtime.rs:718`） |
| 2 | `crates/adapters/cli/src/lib.rs` | supervisor 要**真实可用的 provider + model**：CLI 已有 `crates/adapters/cli/src/llm_factory_env.rs` 与 `active_model_id()`（`lib.rs:501`）⇒ ⚠️ **无凭据/无模型时必须 fail-closed 或直接不装配**，**不许**用假 provider 冒充接上 |
| 3 | `crates/adapters/cli/src/lib.rs` | 审计落盘路径：`FileSystemOneAudit::open(path)` 需要一个路径 ⇒ 用 CLI 现成的 `crate::default_panel_data_dir()`（同 `governance_chain.rs:32` 的用法） |
| 4 | 新测试 `crates/adapters/cli/tests/system_one_knob.rs` | 旋钮关 = 零审查、行为逐字节不变；开 = 内部执行被审查、审计落盘、`deny/escalate/超时/审计失败` 全部 fail-closed |

**⚠️ 诚实边界（口径必须带）**：装配后得到的证据是**软件级**（脚本化 provider + 真文件/真存储副作用），
**不是** live model、**不是** HARDWARE VALIDATED（`claims.yaml:981` 的 notes 已写明）。

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-runtime-assembly --test system_one_execution --locked -j 2 && cargo test -p apeireth-cli --test system_one_knob --locked -j 2`

---

### 4.6 `rt.module_messages` — 「Bounded owner-authenticated module message pipeline」

**口径**：`IMPLEMENTED`（`claims.yaml:993-1009`）；`default: "OFF"`
**现 `verify_cmd`**：`cargo test -p apeireth-runtime-assembly --test module_message_pipeline --locked -j 2`
⇒ ✅ **窄**。

**生产装配现状**：
* 策略槽 `AgentHostPolicy.message_policy`（`crates/engine/runtime/src/canonical/instances.rs:55`）默认 `None`（`instances.rs:74`）；
* ⇒ ⇒ `ModuleMessagePipeline` **不会被构造**（`instances.rs:259-268`：只有 `Some(policy)` 才建）；
* ⇒ ⇒ `BoundModule::messages()`（`instances.rs:126-133`）虽然**返回**端口，但 pipeline 是 `None` ⇒ 发送必失败；
* 生产策略由 `crates/engine/runtime-assembly/src/canonical/desktop_host.rs:8-42` `desktop_agent_policy()` 构造
  ⇒ ⇒ 它**不填** `message_policy` ⇒ **生产零调用点**（`Some(ModuleMessagePolicy::default())` 只出现在
  `crates/engine/runtime-assembly/tests/module_message_pipeline.rs:155` 与 `tests/mesh_execution.rs:57`）；
* ⇒ ⇒ **有界队列 / 背压 / 去重 / 期限 / 幂等窗口，生产里全不存在** ⇒ `IMPLEMENTED` 如实。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/engine/runtime-assembly/src/canonical/desktop_host.rs:21-41` | 在 `desktop_agent_policy()` 里填 `policy.message_policy = Some(ModuleMessagePolicy { limits, grants })`；⚠️ `grants` 里每条都**必须有非空 `authorization_ref`**（`crates/engine/runtime/src/canonical/module_messages.rs:73-79` 的 `validate()`）⇒ 必须是**宿主显式授权串** |
| 2 | `crates/adapters/cli/src/lib.rs` | env 旋钮（**默认关**）+ 组装 grants（目标模块/能力白名单、`allow_background` / `allow_events` / `allow_public_status`） |
| 3 | ⚠️ **硬依赖**：`crates/engine/runtime/src/canonical/instances/messages.rs:407-416` | 外部消息要求目标工具面是 `Logical` **且** `module_visible`；内部消息要求 `Internal` 面 ⇒ ⇒ **必须先接 §4.4**（否则没有可投递的目标面） |
| 4 | 新测试 `crates/adapters/cli/tests/module_message_knob.rs` | 旋钮关 = 无消息面；开 = 目标/授权交集生效、越权拒绝、零 Main Provider 转发 |

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-runtime-assembly --test module_message_pipeline --locked -j 2 && cargo test -p apeireth-cli --test module_message_knob --locked -j 2`

---

### 4.7 `rt.mesh_local_worker` — 「Governed module-owned local Mesh worker and public status」

**口径**：`IMPLEMENTED`（`claims.yaml:1016-1029`）；`default: "OFF"`
**现 `verify_cmd`**：`cargo test -p apeireth-runtime-assembly --test mesh_execution --test module_message_pipeline --locked -j 2`
⇒ ✅ **窄**。

**生产装配现状**：
* `crates/engine/runtime-assembly/src/mesh.rs` 整个模块只在**测试**里被引用
  （`crates/engine/runtime-assembly/tests/mesh_execution.rs:15` 的 `use apeireth_runtime_assembly::mesh::{...}`）；
  `crates/engine/runtime-assembly/src/lib.rs:8` 的 `pub mod mesh;` **只是导出**，不是装配；
* `MeshModule::new`（`mesh.rs:141`）**生产零调用点**；
* ⇒ ⇒ **本地 Mesh worker 生产里根本不存在** ⇒ `IMPLEMENTED` 如实。

**⭐ 接线清单**：

| # | 位置 | 改什么 |
|---|---|---|
| 1 | `crates/adapters/cli/src/lib.rs`（组合根）或 `crates/engine/runtime-assembly/src/canonical/production.rs` | 构造 `MeshModule::new(MeshConfig::loopback(peer, endpoint, payload))`（`mesh.rs:45-58`）并 `with_plugin` 注册；⚠️ `MeshConfig::validate()`（`mesh.rs:68-80`）要求 **loopback IP + 非 0 端口 + peer 名 ≤ 64 字节** ⇒ ⇒ **只能接固定配置的回环对端**，不许做任意目的地 |
| 2 | ⚠️ **硬依赖 1**：`crates/engine/runtime-assembly/src/mesh.rs:202-205` | `MeshModule::start` 需要 `ctx.agents` ⇒ ⇒ **依赖 §4.6 的消息面** |
| 3 | ⚠️ **硬依赖 2**：`crates/engine/runtime-assembly/src/mesh.rs:344-353` | mesh 工具是 `ToolSurface::Internal` + `EffectClass::Protected` ⇒ ⇒ **依赖 §4.5 的 System One**（否则调用必被 `permanent_error` 拒） |
| 4 | ⚠️ **硬依赖 3**：`crates/engine/runtime-assembly/src/mesh.rs:64-66` / `:116` | `status_target` 指向**另一个模块的 Internal 工具** ⇒ ⇒ **依赖 §4.4** |
| 5 | 新测试 `crates/adapters/cli/tests/mesh_knob.rs` | 旋钮关 = 无 mesh 工具面；开 = worker 起、状态可见、私有包隔离、拒绝路径 |

**⚠️ 诚实边界（口径必须带）**：明文 UDP 回环 ⇒ **不是**加密、**不是**密码学认证；
无 live model / 无外网 / 无 NAT / 无发现 / 无持久任务恢复（`claims.yaml:1028` 的 notes 已写明）。

**建议 `verify_cmd`（升级后）**：
`cargo test -p apeireth-runtime-assembly --test mesh_execution --locked -j 2 && cargo test -p apeireth-cli --test mesh_knob --locked -j 2`

---

## 5. ⭐ 依赖图与建议升级顺序

### 5.1 硬依赖图（实测出来的，不是拍的）

```
        rt.system_one_internal            （§4.5，依赖链的根）
                 │  provides: SystemOneGate（runtime.rs:718）+ 审计 sink + supervisor
                 │  needed by: 任何 EffectClass::Protected / ToolSurface::Internal 的工具
                 │             （crates/foundation/plugin/src/system_one.rs:241-244）
                 ├──────────────┬────────────────────┐
                 ▼              ▼                    ▼
   rt.module_execution_   mem.registered_      rt.mesh_local_worker
     contract (§4.4)        namespace (§4.3)       (§4.7)
   provides: Logical /          │                     ▲
   Internal surface              │                     │
   (module_contract.rs:62-70)   │                     │
                 │               │                     │
                 ▼               │                     │
   rt.module_messages (§4.6) ────┤                     │
   needs: 目标面 Logical+visible  │                     │
   (instances/messages.rs:407-416)                     │
                 │               │                     │
                 └───────────────┴─────────────────────┘
                                 ▼
                    rt.owned_agents (§4.2)
                    needs: 生产里有一个模块真的 spawn_agent
                    （唯一现成消费者 = memory_module/organizer.rs:57-95
                      ⇒ ⇒ 与 mem.registered_namespace 同一笔改动）

  （独立、零依赖）
   gov.reflection_queue (§4.1)  ← 不同 crate，与上面 6 条无耦合
```

⚠️ **实测出来的三个「语义陷阱」**（接线时必看）：
1. `with_public_contract` 对非 Agentic 模式给 `Protected`（`tool_module.rs:51-55`）⇒ ⇒ **接 §4.4 前必须先接 §4.5**，否则工具一上线就被永久拒绝。
2. 七个注册式 Memory 工具里 5 个是 `Protected`（`memory_module.rs:59-69`）⇒ ⇒ 同上。
3. mesh 工具是 `Internal` + `Protected`（`mesh.rs:344-353`）⇒ ⇒ 同上。

### 5.2 建议升级顺序（最便宜先做）

| 序 | 动作 | 成本 | 解锁 |
|---|---|---|---|
| **0** | **`gov.reflection_queue` 的 `verify_cmd` 收窄**（零代码改动） | 1 行 | 立刻消掉「235 个无关单测捎带」的风险 |
| **1** | **`rt.owned_agents` 收窄 title**（主代理决策面；把「同一 Runtime / 分层核算」让给 `rt.desktop_managed_main`） | 0 代码 | 把「宽口径声明」变成「一个缺口」，后面只需一笔 |
| **2** | **`gov.reflection_queue` 真装配**（旋钮默认关） | 1 生产文件 + 1 旋钮测试 | 该条自身升 PRODUCTION WIRED（title 须同步改写） |
| **3** | **`rt.system_one_internal` 装配** | 组合根 + 旋钮测试 | **解锁 3 条**（§4.3 / §4.4 / §4.7） |
| **4** | **`rt.module_execution_contract` 装配**（先裁定 §5.1 陷阱 1 的策略） | `production.rs:634` + 旋钮测试 | 解锁 §4.6 / §4.7 |
| **5** | **`mem.registered_namespace` 装配**（依赖 3） | 生产槽 + Profile 映射 + 旋钮测试 | 同时点亮 §4.2 的唯一消费者 |
| **6** | **`rt.module_messages` 装配**（依赖 4） | `desktop_host.rs` 策略 + grants + 旋钮测试 | 解锁 §4.7 |
| **7** | **`rt.mesh_local_worker` 装配**（依赖 4+6+3） | 组合根构造 `MeshModule` + 旋钮测试 | 末位（依赖最重） |
| **8** | **`rt.owned_agents` 收口**（与 5 同一笔） | 补生产面断言 | 升 PRODUCTION WIRED |

⇒ ⇒ **最便宜的两笔都在最前面**：序 0（零代码）与序 1（零代码）。
⇒ ⇒ **最贵的在最后**：`rt.mesh_local_worker`（三条依赖 + 回环 socket + 明文边界说明）。

---

## 6. 复核命令（本审计的可复算入口）

```bash
cd /home/bai/wt-apeireth-r3

# A. 声明面现状（只读）
python3 scripts/docs_truth.py --verify            # 51 条：本次实测 stale=0 / fail=0
grep -n "rt.owned_agents\|mem.registered_namespace\|rt.module_execution_contract\|rt.system_one_internal\|rt.module_messages\|rt.mesh_local_worker\|gov.reflection_queue" docs/status/claims.yaml

# B. 生产装配开关逐条 grep（§3.1 那张表的原始命令）
grep -rn "with_reflection_scheduler\|with_system_one\|message_policy\|tool_module_with_contract\|MeshModule::new\|MemoryModule::new\|spawn_agent" crates/ --include=*.rs

# C. 文档守卫（本次新增文档后必跑）
python3 scripts/check_doc_class.py --check
python3 scripts/check_doc_caliber.py
python3 scripts/check_doc_paths.py --check
```

⚠️ **本次没跑 cargo**（磁盘 85%，任务纪律禁止）。
⇒ ⇒ 「命令今天还绿吗」**未实测**；本文只核到「引用的 test target 全部存在、`cargo test --test <name>` 形态自洽」。

---

## 7. 审计边界（没测什么 / 不在本次范围）

- **没跑 cargo**：没有 build / nextest / clippy / fmt ⇒ §4 各条「现在真跑能过吗」**未实测**；
- **没改 `docs/status/claims.yaml`**：§3.2 的收窄建议、§5.2 序 1 的 title 改写、§4.1 的 title 改写
  **全部只作为建议** ⇒ 声明面是主代理的决策面；
- **没改生产代码**：§4 各条「接线清单」里的每一处都只是**计划**；
- **没有 live model / 真机证据**：System One 与 Mesh 的装配即使做完，最高只能到
  `PRODUCTION WIRED`（软件证据），**到不了** `HARDWARE VALIDATED`；
- **`docs/status/wiring.yaml` 的 `production_refs` 不作证据**（§1 已说明其判据是路径分类器）；
- **未登记 workline**：按 `AGENTS.md` §0.3，新工作线须在 `docs/status/workline-registry.md` 登记 ——
  ⇒ ⇒ 本次**未登记**（只允许改台账与新 handoff）⇒ ⇒ **请主代理决定是否补登记**。

---

## 8. 本批改动清单（0 装诚实真账）

| 状态 | 项 | 改法 | 四级口径 |
|---|---|---|---|
| **已修** | 台账缺这 7 条骨架声明的接线缺口记录（可查不到「差什么」） | `docs/status/unwired-ledger.md` **追加一节**（不删改既有行） | 台账新增节标 `IMPLEMENTED` / `PRODUCTION WIRED` 四级口径 |
| **已修** | 这 7 条的接线缺口没有逐条**可复算**的证据面（只有 `wiring.yaml` 的路径分类器） | **新增** `docs/04-internal/01-handoffs/l3-skeleton-claims-2026-11-06.md`（本文） | 本文档是 `AUDIT`（审计面）⇒ 7 条结论一律带 `IMPLEMENTED` / `PRODUCTION WIRED` 四级标注 |
| **已修** | `gov.reflection_queue` 的 `verify_cmd` 是 crate 级宽命令（**捎带别人的红**） | 本文**已定位并给出窄口径**（§4.1）⇒ ⇒ 落码属声明面（主代理），本次**只报告不落码** | 现口径仍为 `IMPLEMENTED`（未升级） |
| **未修** | 7 条声明的**生产接线**（本次只读，一律未动） | 见 §4 各条「接线清单」与 §5.2 升级顺序 | 7 条现口径**仍为** `IMPLEMENTED` ⇒ 未升级为 `PRODUCTION WIRED` |
| **未修** | `docs/status/claims.yaml` 的口径调整（声明面属主代理决策面） | §3.2 / §4.1 / §5.2 序 1 的建议**只报告不落码** | 现口径不变（`IMPLEMENTED`） |
| **暂缓** | 本次**未跑 cargo**（磁盘 85%，任务纪律禁止） | 复核入口见 §6 | §7 已写明「未实测」⇒ 不产生任何对外测试数字 |

- **测试**：本次**未跑**（磁盘约束）⇒ 不产生任何对外测试数字；
- **0 触碰 LOCKED 5 项**：未动 9 哲学锚 / 键集 / 3 不可变脊柱 / `workspace.version` / R11 baseline；
- **0 引新外部 dep**：纯文档批；
- **0 改生产代码**；
- **0 改 `docs/status/claims.yaml`**（声明面是主代理的决策面）。
