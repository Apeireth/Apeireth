[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-default-usable-surface-2026-11-09.md
Class: EVIDENCE
Status: 🟢 活跃
Last-Modified: 2026-11-09
Author: lead

# 默认可用面底账（2026-11-09）——「**接上了但必然失败**」的系统扫描

> **四级口径**：本文是**证据**（枚举 + 真跑 + 可复跑命令），不是能力声明页。
> 本次改动的口径 = **IMPLEMENTED**（装配层已落地 + 有测试钉住）
> ⇒ **PRODUCTION WIRED ✘**（本次**不新增**生产接线：默认组合根本来就是唯一那个）
> · **DEFAULT ENABLED** 不适用 · **HARDWARE VALIDATED ✘**（无真机、无真模型 E2E）。
>
> ⚠️ **中性表述**：本文只描述**机制**（发布面 / 授权前提 / 范围闸 / 装配根），
> **不出现任何外部产品名、仓库名、文档名**；所有数字都来自**本仓自己的实测**。

---

## 〇、一句话结论

**P1（记忆工具面每调用必 `namespace_denied`）不是孤例** —— 系统扫完默认路径后，
本类缺陷**还剩一条**，而且**不是**同一张脸：

| # | 面 | 症状 | 处置 | 状态 |
|---|---|---|---|---|
| D1 | **固定检索 Hook**（`memory-hook.json` 开关） | 开关一开 ⇒ **每回合一条 `scope_denied` 失败帧** | ①不发布 + ③说清楚 | ✅ **本次落地** |
| D2 | **装配层的自我契约**（`memory_assembly.rs` 注释） | 声称「模块侧另有闸 ⇒『开了但兑现不了』的状态不存在」⇒ **假**（那道闸已被删） | ③说清楚 | ✅ **本次落地** |
| D3 | 记忆工具面（7 个） | 每调用必 `namespace_denied` | ①不发布 | ✅ **P1 已修**（本次核验） |

⇒ ⇒ 工具面在默认组合里**已经干净**（5 个发布面全部真跑可兑现，见 §三）。

---

## 一、复现方式

```sh
# ① 底账测试（发布面清单 + 逐个真跑 + 命名空间面外）
cargo test -p apeireth-runtime-assembly --test default_usable_surface_catalog

# ② 固定检索 Hook 的拒开闸（独立二进制：它动进程级数据根）
cargo test -p apeireth-runtime-assembly --test default_usable_surface_hook

# ③ 默认组合的运行时自查（门禁自查）
cargo test -p apeireth-runtime-assembly --lib
python3 scripts/arch_guard.py --strict
python3 scripts/audit_unwired.py --check
```

⚠️ **口径**：`default_usable_surface_catalog` 里那份**允许清单**
（`PUBLISHED_ALLOWLIST`）是**底账本体** ⇒ ⇒ 以后**新增**一个发布面
⇒ 必须**显式**改清单 + 给出**可兑现**的证据 ⇒ ⇒ 「不小心发布」不可能悄悄发生。

---

## 二、**默认组合**是什么（全仓只有一处）

| 组合 | 装配根 | 有托管身份？ | 发布模型可见工具面？ |
|---|---|---|---|
| **桌面 main / CLI main** | `canonical::desktop_host::build_desktop_main_runtime`（`desktop_host.rs:210`） | ❌ **不绑**（`AgentConfig::default()` ⇒ `principal: None`） | ✅ **发布 5 个**（§三） |
| **库默认** | `ProductionModules::register_into` + `build_agent_runtime` | ❌ 无托管身份 | ❌ **不发布**（没有「main」这个身份 ⇒ 没有发布面） |

⇒ ⇒ **桌面产品与 CLI 共用同一个组合根**：桌面应用起 CLI sidecar，
CLI 自己也是这一个根（`crates/adapters/cli/src/lib.rs:1759`）⇒ ⇒ **一处改动 ⇒ 两处生效**。
⇒ ⇒ 全仓**只有这一处**绑定托管身份（`into_main_runtime_with_session_quotas` 的唯一调用点），
⇒ ⇒ **「默认路径」没有第二个入口**。

---

## 三、**发布面 × 默认可用面**对照表

### 3.1 工具面（默认桌面组合，**逐个真跑**）

| 能力面 | 发布？ | 能兑现？ | 前提 / 实测证据 |
|---|---|---|---|
| `tool.self_status`（**自述面**） | ✅ | ✅ | 自述通道在位；真跑返回完整自述 JSON（未接线的节各自带 `reason`） |
| `tool.filesystem` | ✅ | ✅ | 需**工作区根**（CLI = 当前目录）；真跑**读到文件内容** |
| `tool.search` | ✅ | ✅ | 需工作区根；真跑 = 业务级报错（查询为空）⇒ **授权放行** |
| `tool.repo` | ✅ | ✅ | 需工作区根 + git 仓库；真跑 = 业务级报错（非 git 目录）⇒ **授权放行** |
| `tool.repo_map` | ✅ | ✅ | 需工作区根；真跑**成功**返回代码地图 |
| `tool.memory.*`（7 个） | ❌ **不发布** | — | 唯一授权 = 执行上下文的 **Host 身份** ⇒ 默认组合不绑身份 ⇒ 发布会**每调用一次** `namespace_denied` ⇒ ⇒ **①不发布**（`desktop_host.rs` 的过滤器） |
| `tool.behavior.cognitive.*`（4 个） | ❌ 不作为模型工具发布 | ✅ 内部行为路径照旧 | 受治理的记忆路径（召回/写回）不受影响 |

### 3.2 模块面

| 模块 | 默认组合 | 可兑现？ |
|---|---|---|
| 记忆行为模块（召回 / 写回） | 注册 | ✅ |
| 偏好召回 / 自评 | 注册（存储在位） | ✅ |
| 其余（shell / fetch / 文件写 / git 写 / 工作树 / MCP / 内容流 / 器官…） | **opt-in**（`ProductionModulesConfig::default()` 全关） | — **不在默认路径上** |

### 3.3 hook 面

| hook | 默认组合 | 可兑现？ | 处置 |
|---|---|---|---|
| **固定检索 Hook**（`memory-hook.json` 开关） | 默认**关** | ❌ **开了也兑现不了**（见 §四 D1） | ✅ **本次落地**：①不发布 + ③说清楚 |

### 3.4 自述面

| 面 | 默认组合 | 可兑现？ |
|---|---|---|
| `tool.self_status` | ✅ | ✅ 各节带 `reason` 标明「本组合未接线」⇒ **不是**假承诺 |
| `docs/status/claims.yaml`（静态自述） | 逐条带四级口径与边界 | 抽查 `mem.registered_namespace` / `mem.injection_contract` ⇒ 边界写明 |

### 3.5 运维出口

| 出口 | 默认组合 | 可兑现？ |
|---|---|---|
| `memory verify` / `memory export` / `memory handover` | ✅ 已接线 | ✅ 真跑：缺库时给**可执行**提示（「先跑一次会写记忆的功能，或用 `--db` 指定路径」）⇒ **不是**缺陷 |

---

## 四、本类缺陷清单（**逐条带真跑输出**）

### D1 ⛔ **固定检索 Hook：开关一开 ⇒ 每回合一条失败帧**（本类缺陷）

**机制**：Hook 每回合发出**记忆工具面自己**的 `memory_retrieve` 调用
⇒ ⇒ 能不能兑现由**分发器的范围闸**判 ⇒ ⇒ 需要**两个**前提**同时**成立：

1. 该组合**绑了 Host 身份**（授权来自执行上下文的 `AgentPrincipal`）；
2. 该组合**发布了**记忆工具面（否则那个调用**不在会话范围**里 ⇒ 范围闸拒）。

⇒ ⇒ **默认桌面组合两个都不满足**（不绑身份 + 默认组合不发布记忆工具面）
⇒ ⇒ 开关一开 ⇒ **必然**每回合被拒。

**真跑输出**（探针：开关文件 + 默认桌面组合 + 无身份）：

```
=== switch file: /tmp/apeireth-hook-probe-522649/memory-hook.json
=== main principal bound: false
=== published tools: ["filesystem", "repo_map", "repo", "search", "self_status"]
=== tool frames in transcript: 1 ===
  FRAME: {"call_id":"4218c88f-…","hook":"before_model_call","module_id":"module.memory",
          "origin":"module",
          "payload":{"message":"{\"code\":\"scope_denied\",\"message\":\"tool is outside this session's tool scope\"}",
                     "retryable":false,"status":"error"},
          "tool_name":"memory_retrieve","version":1}
=== authorization refusals: 1 ===
```

⇒ ⇒ **模型每回合看到一条失败的工具帧** ⇒ ⇒ 「开了但兑现不了」= 与「没接」同类。

**处置（①不发布 + ③说清楚）**：

* **①不发布**：默认桌面组合在**装配期拒开**这个 Hook（它兑现不了 ⇒ 干脆不出现）；
* **③说清楚**：拒开时给出**可执行**提示（点名开关文件 + 点名失败形态 + 两条出路）。

⭐ **刻意不做**（避免凭空扩权）：**不给**默认桌面分配身份、**不**发布记忆工具面
（那会把 P1 重新打开）⇒ ⇒ 唯一诚实的出路是「换个组合」。

### D2 ⛔ **装配层对自身的契约说错了**（假的安全声明）

**位置**：`crates/engine/runtime-assembly/src/canonical/memory_assembly.rs`

**原文（已撤回）**：

> ⚠️ 开关本身**不保证** Hook 能兑现：… ⇒ ⇒ 模块侧另有闸（`memory_module.rs` 的
> `on_hook`）⇒ ⇒ **「开了但兑现不了」的状态不存在**。

**为什么是假的**：那道「模块侧闸」用的是「**有没有映射**」当判据，
而分发器的范围闸用的是「**身份**」⇒ ⇒ **不是同一把尺子** ⇒ ⇒ 2026-11-09 已删
（删的理由：它把「有身份、无映射」的**合法**组合也一并沉默 ⇒ 打破既有不变量测试）。

⇒ ⇒ 删掉闸之后，「开了但兑现不了」**真的存在**（D1）⇒ ⇒ 而装配层**仍然写着它不存在**
⇒ ⇒ 一个**说错的安全契约**留在装配层 ⇒ ⇒ 下一个人会照着它做判断。

**处置（③说清楚）**：把注释改成**真话**：两个前提 + 默认组合两个都不满足 +
指向本文 + 标注旧结论已撤回。

### D3 ✅ **记忆工具面**（P1，**已修**，本次核验）

**位置**：`crates/engine/runtime-assembly/src/canonical/desktop_host.rs`（过滤器）
+ `crates/engine/runtime-assembly/src/canonical/memory_module.rs`（`is_namespace_scoped`）

**本次核验**：默认桌面组合**不发布**这 7 个面 ⇒ ⇒ 模型拿不到兑现不了的承诺；
受治理的记忆路径（行为模块的召回/写回）**照旧**。

---

## 五、**本次落地**（只做「①不发布」这一类）

| # | 文件 | 改了什么 | 为什么这处是「不发布」而不是「放宽授权」 |
|---|---|---|---|
| 1 | `crates/engine/runtime-assembly/src/canonical/desktop_host.rs` | 默认桌面组合**装配期拒开**固定检索 Hook（开关开着就**拒开**，并给可执行提示） | **不发布**（这个 Hook 在本组合兑现不了 ⇒ 干脆不出现）+ **说清楚**（提示可执行）。**没有**给组合分配身份、**没有**发布记忆工具面、**没有**动任何审批/治理口径 ⇒ ⇒ **零扩权** |
| 2 | `crates/engine/runtime-assembly/src/canonical/memory_assembly.rs` | 改掉**说错的安全契约**（注释），并把开关读取器暴露给装配层（**单一真源**） | 纯**注释/可见性** ⇒ **零行为变化**；让「开关开了吗」只有一处判 ⇒ ⇒ 装配层与装配层不会各判各的 |
| 3 | `crates/engine/runtime-assembly/tests/default_usable_surface_catalog.rs`（**新建**） | **底账**：发布面清单 + 逐个真跑「授权放行」+ 命名空间面外 | 测试 ⇒ **零行为变化** |
| 4 | `crates/engine/runtime-assembly/tests/default_usable_surface_hook.rs`（**新建**） | 固定检索 Hook 的拒开闸（独立二进制：它动进程级数据根） | 测试 ⇒ **零行为变化** |

⭐ **行中性**：两个被改的源文件都**小于** `arch_guard` 的尺寸棘轮下限
（`memory_assembly.rs` ≈ 280 行 / `desktop_host.rs` ≈ 290 行 ⇒ 远小于 1200）
⇒ ⇒ **不受尺寸棘轮约束**；且**没有**动任何棘轮文件
（`production.rs` / `coordinator.rs` / `memory_module.rs` 均未碰）。

### 反证（去掉 ⇒ 测试红）

| 反证 | 去掉什么 | 哪条红 | 实测 |
|---|---|---|---|
| A | `desktop_host.rs` 的**装配期拒开闸** | `the_default_desktop_refuses_an_arm_hook_it_cannot_honor` | ✅ **红**（去掉闸 ⇒ 组合能起来 ⇒ 「开了但兑现不了」回来了） |
| B | 底账清单里的一个条目 | `the_published_surface_is_pinned` | ✅ **红**（`left: ["filesystem","repo","repo_map","search","self_status"]` vs `right: ["repo","repo_map","search","self_status"]`） |

⇒ ⇒ 两条落地**各自**有反证 ⇒ ⇒ 不是「加了断言好看」，是**真的在守**。

---

## 六、**边界 / 没测什么**

* ⚠️ **只扫了默认路径**：其余面（shell / fetch / 文件写 / git 写 / 工作树 / MCP /
  内容流 / 器官…）在 `ProductionModulesConfig::default()` 里**全关**
  ⇒ ⇒ **不在默认路径上** ⇒ ⇒ **未逐个审**它们的「发布了就兑现不了」问题。
* ⚠️ **库默认没有托管身份** ⇒ ⇒ 它**不发布**任何模型可见面 ⇒ ⇒ 本文的对照表
  **不含**「库默认」这一行的工具面清单；**未**测「Host 自己在 `spawn_agent`
  里列出工具」这种**显式选择**的路径（那是 Host 的决定，不是「默认路径」）。
* ⚠️ **桌面应用本体没跑**：它起 CLI sidecar ⇒ ⇒ 走的是同一个组合根
  ⇒ ⇒ **间接**覆盖；**未**做真机 E2E。
* ⚠️ **System One 路径未审**：默认关（env 开关）⇒ ⇒ 不在默认路径上。
* ⚠️ **「可兑现」的判据是「授权层放行」**：用**从 schema 合成的最小合法参数**真跑
  ⇒ ⇒ 只在**真实参数**下才失败的工具**测不出来**（`filesystem` 已额外用真路径验证过）。
* ⚠️ **静态自述（`claims.yaml`）未逐条对表**：只抽查了与本线相关的两条
  ⇒ ⇒ 全量交叉核对**未做**。
* ⚠️ **两个数据根**：开关的**读取处**用**进程级**默认数据根，而组合的数据根
  **可注入**（测试用）⇒ ⇒ 生产入口注入的就是同一个根（`desktop_host.rs:214`）
  ⇒ ⇒ 生产上**同一个文件**；但「注入根 ≠ 全局根」的组合**未被审**。
* ⚠️ **未改动任何生产默认值**：本次只加了一个**装配期拒开**（默认态 = 开关没开
  ⇒ ⇒ 逐字节零行为变化）。

---

## 七、**交接清单**

| # | 事项 | 落点 | 备注 |
|---|---|---|---|
| ① | **其余 opt-in 面**逐个审「发布了就兑现不了」 | 那些面的装配处 | 默认全关 ⇒ 不在默认路径 ⇒ 未审 |
| ② | **「送达率/自述」类比值口径**（分母口径不同 ⇒ 比值不是概率） | 记忆线的 `delivery_metrics` | 另一条线；与本线无关但同属「读数要带口径」纪律 |
| ③ | **claims.yaml 全量对表**（静态自述 vs 默认组合） | `docs/status/claims.yaml` | **主指挥统一更新** ⇒ 本线未动 |
| ④ | **给默认桌面绑 Host 身份** | 组合根 | **产品决策**（本线**刻意不做** ⇒ 那会扩权并重开 P1） |
