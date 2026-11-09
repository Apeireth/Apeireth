[Document-Meta]
Class: EVIDENCE
Title: 默认桌面路径上的记忆工具面授权 —— 「接上了但永远报错」的 P1 修掉
Date: 2026-11-09
Owner: memory
Scope: crates/engine/runtime-assembly/src/canonical/{desktop_host.rs,memory_module.rs,memory_assembly.rs}
Caliber: PRODUCTION WIRED

# 一句话

默认桌面组合**不给模型任何必然失败的能力**：记忆工具面是**命名空间受限**的，
它唯一的授权是执行上下文里带的 **Host 身份**；默认桌面**不绑身份**
⇒ ⇒ 那就**不把这张脸发布出去** ⇒ ⇒ 「接上了但每次都 `namespace_denied`」的状态被消除；
固定检索 Hook 另配**显式开关**（默认关、Host 可开），且**开了也只能在能兑现时开**。

# 一、缺陷（先复现，再改）

默认桌面装配（`canonical/desktop_host.rs::build_desktop_main_runtime_at`）：

```rust
host.into_main_runtime_with_session_quotas(AgentConfig {
    tools,
    budget,
    ..Default::default()   // ⇒ principal: None（**没有 Host 身份**）
})
```

而记忆工具面（`canonical/memory_module.rs`）的每一次调用都要过
「工具内 principal + 映射租约」校验 ⇒ 没有身份 ⇒ 一律：

```
{"code":"namespace_denied","message":"Host principal is required"}
```

**实测（临时探针，跑完已并入永久回归文件）**：

```
PROBE exposed_memory_tools=["memory_retrieve", "memory_write", "memory_update",
                            "memory_delete", "memory_search", "memory_compact",
                            "memory_organize"]
PROBE managed_main_principal=Some(None)
PROBE tool_results=["error: {\"code\":\"namespace_denied\",\"message\":\"Host principal is required\"}"]
```

⇒ ⇒ **7 个 `memory_*` 工具被发布给模型，但每次调用都是 `permanent_error`**。

⇒ ⭐ 这与「没接」是**同一类缺陷**：模型拿到的是一张**兑现不了的承诺**
⇒ ⇒ 既浪费回合，也让用户以为「记忆功能坏了」。

# 二、根因（分清三层，别混）

| 层 | 事实 | 判定 |
|---|---|---|
| **身份**（principal） | 默认桌面 `AgentConfig::default()` ⇒ `principal: None`；`AgentPrincipal` 的**类型**定义在 `crates/foundation/plugin/src/agents.rs:23`（**不是** `profiles.rs`：那个文件在 **runtime** crate 里，是 `ProfileConfig` ⇒ 身份的**来源面**）；`activate_profile` 在 CLI/gateway/UI **无调用** ⇒ ⇒ 生产代码里**没有给「主身份」（桌面/CLI main）赋值**的点，仅有的赋值点都是**派生语义**（`crates/engine/runtime/src/canonical/profiles.rs:39` 的 profile 激活、`crates/engine/runtime-assembly/src/canonical/memory_module/organizer.rs:73` 的子代理继承会话身份） | ⭐ **主因** |
| **授权**（grant） | `desktop_agent_policy()` 只为 `self.modules()`（行为模块）建 grant；`MemoryModule` 在 `tool_modules` 里 ⇒ 非测试代码**没有 `module.memory` grant** ⇒ ⇒ `memory_organize` 的子代理创建在 `crates/engine/runtime/src/canonical/instances.rs:178-183` 被拒（实测帧：`organizer_denied / ScopeDenied: host did not grant module agent creation`） | 次因（只影响 `memory_organize`） |
| **固定 Hook** | `memory_assembly.rs` 硬编码 `with_automatic_retrieval(false)`，且 `production.rs` **没有开关** ⇒ ⇒ 生产里 `hooks()` 恒空 ⇒ 「默认关」=「**没有打开路径**」 | ⭐ 口径缺口（本笔**已补开关**，见 §三·d） |

**逐层验证（三个变体各跑一次，结论互不混淆）**：

| 变体 | 身份 | `module.memory` grant | 实测结果 |
|---|---|---|---|
| **A** 默认桌面 | ❌ 无 | ❌ 无 | 7 个工具**全部** `namespace_denied / Host principal is required` |
| **B** 只给身份 | ✅ 有 | ❌ 无 | 6 个数据工具**真成功**（回执带 `"revision":0`）；`memory_organize` ⇒ `organizer_denied / host did not grant module agent creation` |
| **C** 身份 + grant | ✅ 有 | ✅ 有 | 7 个工具**全通**（`memory_organize` 回执 `{"agent_id":…,"status":"scheduled"}`） |

⇒ ⇒ **根因 = 两个都缺**：身份缺 ⇒ 7 个全哑；grant 缺 ⇒ `memory_organize` 再哑一层。

# 三、改法（**最小权限**：不放宽任何治理口径）

## 3.0 候选对照（先评估，再拍板）

| 候选 | 改动面 | 权限面影响 | 默认行为变化 | 反证/证据 | 裁定 |
|---|---|---|---|---|---|
| **(a)** 给桌面 main 分配 Profile/Namespace 身份 | `desktop_host.rs` + 身份持久化 | ⭐ **扩权**（凭空给主身份） | 记忆工具面**能用** | ⛔ **三条硬证据否决**：① `crates/engine/runtime/src/canonical/execute.rs:2470-2475`：会话有身份但**没有** `PrincipalAuthorizations` 授权 ⇒ **所有行为模块 Hook 被跳过**（桌面会当场失去受治理的召回/写回注入）；② 授权 epoch 由 `crates/engine/runtime/src/canonical/principal_authority.rs:74` 随机生成且**随转录持久化** ⇒ Host 每次重启换 epoch ⇒ 旧会话被 `denied()`；③ `ProfileId::new()`/`NamespaceId::new()` 是随机 id ⇒ 每次启动换一个命名空间 ⇒ 记忆跨重启不可见 | ❌ **不做**（需改 runtime crate，本笔不动） |
| **(b)** 只在 Host 绑定映射时才注册记忆工具面 | `memory_module.rs` / `memory_assembly.rs` | 不扩权 | 默认桌面：**工具不出现** | ⛔ 会连带废掉「有身份、无映射 ⇒ 私有命名空间仍可写」这条**合法**语义（现有 `memory_namespace_production.rs` 两例即钉它）⇒ 粒度过粗 | ❌ 不取（改用 **b′**） |
| **(b′)** ⭐ **落地**：在**组合根**上不发布命名空间受限的能力 | `desktop_host.rs`（+6 行） | **不扩权**（只收窄可见面） | 默认桌面：**这张脸根本不出现** ⇒ 模型不会去调必然失败的工具 | ✅ 反证实测：去掉 `filter` ⇒ 首条红 | ✅ **本笔落地** |
| **(c)** 给 `memory_organize` 补 `module.memory` grant | `desktop_host.rs` | ⭐ **扩权**（模块可起子 Agent） | 默认桌面：`memory_organize` 仍会被身份闸拒（⇒ **无收益**） | 变体 B 实测：只有身份+grant 同时到位才通 | ❌ **不做**（桌面路径上它根本不会被发布 ⇒ 纯扩权、零收益） |
| **(d)** 固定 Hook 的显式开关 | `memory_assembly.rs` | 不扩权（默认关） | 默认：**逐字节零变化**；Host 写开关文件才开 | ✅ 落地，但**必须带模块侧闸**（见 §三·d） | ✅ **本笔落地**（带闸） |

## 3.a 落地一：不发布兑现不了的能力（对应 b′）

- `memory_module.rs`：新增 `pub fn is_namespace_scoped(capability: &CapabilityId) -> bool`
  （按能力前缀 `tool.memory.` 判定 ⇒ **单一真源**，与 `capability()` 用同一个常量）；
- `desktop_host.rs`：桌面 main 发布工具时**过滤掉命名空间受限的能力**
  ⇒ ⇒ 没有身份 ⇒ 这张脸**根本不出现** ⇒ ⇒ 模型不会去调一个必然失败的工具。

⇒ **受治理的记忆路径完全不受影响**：行为模块（召回 / 写回进对话）照旧。

## 3.b 落地二：固定 Hook 的显式开关（对应 d）

**开关**：数据根下的 `memory-hook.json`（`StoredDoc` 信封，主体
`{"automatic_retrieval": true}`）⇒ **默认关**（文件缺失 ⇒ 关）；
**坏文件 ⇒ 按关处理**（失败闭合到默认态）。

⚠️ **为什么还有一道模块侧的闸**：实测（把 `with_automatic_retrieval(false)` 临时改成
`true` 后跑默认桌面路径）⇒ 固定 Hook 发出的 `memory_retrieve` 被**分发器范围闸**拒：

```
{"hook":"before_model_call","module_id":"module.memory","origin":"module",
 "payload":{"message":"{\"code\":\"scope_denied\",\"message\":\"tool is outside this session's tool scope\"}",
            "retryable":false,"status":"error"},"tool_name":"memory_retrieve"}
```

⇒ ⇒ **光加开关会把同一个 P1 换个面再犯一次**（每回合一条失败帧）
⇒ ⇒ **前提**：开这个开关的组合里**要有身份**（授权来自执行上下文的 `AgentPrincipal`）。
⇒ ⇒ ⚠️ **2026-11-09 纠错**：曾用「**有无命名空间映射**」当闸 ⇒ ⇒ **错**（实测把「有身份、无映射」的合法组合也一并沉默 ⇒ 既有不变量测试 `convergence_memory` 被打断）⇒ ⇒ **该闸已撤回**；现在**授权判据 = 身份**（与分发器范围闸同一判据）。
（同一把尺子：不做兑现不了的事）⇒ ⇒ 「开了但兑现不了」的状态不存在。

⇒ ⚠️ **诚实边界**：坏开关文件只「按关处理」而**不报错** —— 装配面没有错误通道
（`wire()` 不返回 `Result`）⇒ ⇒ 要改成「坏配置拒开」需要动棘轮文件 `production.rs`（本笔不动）。

# 四、证据（真跑）

## 4.1 缺陷与修复（`tests/memory_tool_authority_desktop.rs`，**4 例全绿**）

| 用例 | 钉住什么 |
|---|---|
| `desktop_main_offers_no_namespace_scoped_tool` | 默认桌面 main **不发布**任何命名空间受限能力 |
| `an_out_of_scope_memory_call_never_reaches_the_module` | 越界调用在**分发器的范围闸**就被拒，**到不了**记忆模块 |
| `the_desktop_memory_behavior_path_still_works` | **受治理**的记忆路径（召回 / 写回）**照旧可用** |
| `a_host_bound_identity_still_operates_the_memory_tools` | 给了 Host 身份 ⇒ 记忆工具**照常能用**（这张脸没被改坏） |

**反证（实测红过）**：
- 去掉 `desktop_host.rs` 里的 `filter` ⇒ `desktop_main_offers_no_namespace_scoped_tool` **红**
  （实测输出：`published = ["memory_retrieve", …, "memory_organize", "self_status"]`）；
- 去掉该用例里的 **Host 身份** ⇒ `a_host_bound_identity_still_operates_the_memory_tools` **红**
  （实测帧：`namespace_denied / Host principal is required`）。
  ⚠️ **更正**：去掉 `module.memory` grant **不会**让该例变红 ——
  grant 只管**模块自有子代理**的创建（`instances.rs:190-198`），普通写不查它；
  子代理那一条由 `memory_namespace_production.rs::production_memory_module_owns_a_child_agent_that_writes_with_its_own_grant` 钉着。

## 4.2 固定 Hook 的开关（`tests/memory_tool_authority_hook.rs`，**2 例全绿**）

| 用例 | 钉住什么 |
|---|---|
| `the_fixed_hook_switch_opens_a_working_path` | 开关打开 + Host 身份 + 命名空间映射 ⇒ Hook **真的跑**，召回进转录 |
| `the_fixed_hook_switch_works_with_identity_even_without_a_mapping` | 开关打开 + **有身份** + **无映射** ⇒ Hook **照常工作** ⇒ 钉住「授权来自身份，不是映射」 |

**反证（实测红过）**：
- 把开关读成常量 `false` ⇒ 第 1 例 **红**（Hook 永远不跑）；
- 把开关读成常量 `false` ⇒ 第 1 例 **红**（Hook 永不触发 ⇒ 「默认关 = 没有打开路径」回来了）。

## 4.3 单元测试（`memory_assembly.rs` 内，**3 例**）

`the_fixed_hook_is_off_without_its_switch_file` · `the_fixed_hook_switch_file_opens_the_path` ·
`a_broken_hook_switch_file_leaves_the_hook_off`（畸形 / 字段拼错 / 类型错 / 串档 ⇒ 全部按关处理）。

## 4.4 门禁（复跑）

`cargo test -p apeireth-runtime-assembly --lib` **432 passed**（429 + 本笔 3 例）·
`cargo test -p apeireth-runtime-assembly --test memory_tool_authority_desktop` **4 passed** ·
`cargo test -p apeireth-runtime-assembly --test memory_tool_authority_hook` **2 passed** ·
`cargo clippy -p apeireth-runtime-assembly --all-targets --offline -- -D warnings` **0 warning** ·
`cargo fmt -p apeireth-runtime-assembly -- --check` 干净 ·
`python3 scripts/arch_guard.py --strict` **VIOLATIONS 0** ·
`python3 scripts/audit_unwired.py --check` 孤儿基线 **86 不变** ·
`python3 scripts/check_doc_class.py --check` OK。

# 五、⚠️ 没证明 / 仍然欠着（如实列）

1. ⭐ **默认桌面路径上，记忆工具面现在「不可用」**（不是「可用」）
   ⇒ ⇒ 想让默认桌面真的能用记忆工具，必须**给桌面 main 绑一个 Host 身份**
   ⇒ ⇒ 那是**产品决策**（身份从哪来、scope 是什么、跨重启怎么保持），
   **本笔刻意不做**（凭空扩权比缺功能更危险；技术阻塞见 §三·a 的三条证据）。
2. ⭐ **`memory_organize` 在默认桌面策略下仍起不了子 Agent**
   （缺 `module.memory` grant）⇒ 本笔**未动**（补 grant = 扩授权面，且默认桌面上
   这张脸**根本不会被发布** ⇒ 零收益）。
3. ⭐ **固定 Hook 的开关目前只对「Host 绑了命名空间映射」的部署有效**
   （模块侧闸）⇒ ⇒ 而**生产代码里没有任何一处绑定映射**（`register_with_profile_mappings`
   只有测试在用）⇒ ⇒ 该开关在今天的生产组合里**打不开任何东西** ——
   **这是「缺口仍在」的如实登记，不是「开关已可用」的宣称**。
4. ⭐ **坏开关文件只「按关处理」不报错**（装配面没有错误通道）⇒ ⇒ 「坏配置拒开」需要动
   棘轮文件 `production.rs` ⇒ 未做。
5. ⭐ **行数变化**（如实登记）：
   `memory_assembly.rs` 172 → 268（**+96**：开关读取 + 文档 + 3 例单元测试）·
   `memory_module.rs` 436 → 447（**+11**：能力前缀常量 + 单一真源判定；**映射闸已撤回**）·
   `desktop_host.rs` 264（**+0**，本轮未再动）。
   ⇒ ⚠️ **更正**：这三个文件**都不受尺寸棘轮约束** ——
   `scripts/arch_guard.py` 的 `MIN_SIZE_TRACK = 1200`（只追踪 ≥1200 行的 `src` 文件），
   且 `EXC-0002` 的指纹是 `size-grow …/canonical/cognitive.rs`、`EXC-0003` 是 `tool_modules.rs`
   ⇒ ⇒ **`desktop_host.rs` 没有 size-grow 抑制**（原文如此写是错的，本笔更正）。
   ⇒ 若主指挥要求**零增长**，精确替代方案：把开关读取 + 其 3 例单元测试整体搬进
   新文件 `crates/engine/runtime-assembly/src/canonical/memory_hook.rs`，
   `canonical.rs` 只需加 **1 行**（`pub mod memory_hook;`）—— 该行需主指挥代加（本笔不在写入范围）。
6. 未测：真机 / 真模型 E2E；多主体并发；跨重启的身份保持（**默认桌面现在没有身份**）；
   固定 Hook 在「有映射但会话无身份」的错配部署下的表现（未构造该部署）。
7. 未跑：`cargo nextest` / 全仓套件 / `cargo build`（只跑了本 crate 的 lib + 两个集成测试二进制）。

# 六、复现

```sh
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= \
  bash -c 'cd /home/bai/wt-apeireth-r3 && \
    cargo test -p apeireth-runtime-assembly --test memory_tool_authority_desktop --offline && \
    cargo test -p apeireth-runtime-assembly --test memory_tool_authority_hook --offline && \
    cargo test -p apeireth-runtime-assembly --lib --offline'
```

固定 Hook 的开关文件（数据根下，默认**不存在** ⇒ 默认关）：

```json
{"name":"apeireth.memory.hook","version":1,"compatible_versions":[1],
 "body":{"automatic_retrieval":true}}
```
