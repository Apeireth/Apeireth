[Document-Meta]
Class: EVIDENCE
Title: 默认桌面路径上的记忆工具面授权 —— 「接上了但永远报错」的 P1 修掉
Date: 2026-11-09
Owner: memory
Scope: crates/engine/runtime-assembly/src/canonical/{desktop_host.rs,memory_module.rs}
Caliber: PRODUCTION WIRED

# 一句话

默认桌面组合**不给模型任何必然失败的能力**：记忆工具面是**命名空间受限**的，
它唯一的授权是执行上下文里带的 **Host 身份**；默认桌面**不绑身份**
⇒ ⇒ 那就**不把这张脸发布出去** ⇒ ⇒ 「接上了但每次都 `namespace_denied`」的状态被消除。

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

⇒ ⇒ **7 个 `memory_*` 工具被发布给模型，但每次调用都是 `permanent_error`**
（`memory_retrieve` / `memory_write` / `memory_update` / `memory_delete` /
`memory_search` / `memory_compact` / `memory_organize`）。

⇒ ⭐ 这与「没接」是**同一类缺陷**：模型拿到的是一张**兑现不了的承诺**
⇒ ⇒ 既浪费回合，也让用户以为「记忆功能坏了」。

# 二、根因（分清三层，别混）

| 层 | 事实 | 判定 |
|---|---|---|
| **身份**（principal） | 默认桌面 `AgentConfig::default()` ⇒ `principal: None`；全仓**生产代码没有 `AgentPrincipal` 赋值点**（`AgentPrincipal` 定义在 `foundation/plugin/src/profiles.rs`，`activate_profile` 在 CLI/gateway/UI **无调用**） | ⭐ **主因** |
| **授权**（grant） | `desktop_agent_policy()` 只为 `self.modules()`（行为模块）建 grant；`MemoryModule` 在 `tool_modules` 里 ⇒ 非测试代码**没有 `module.memory` grant** | 次因（`memory_organize` 起不了子 Agent） |
| **固定 Hook** | `memory_assembly.rs` 硬编码 `with_automatic_retrieval(false)`，且 `production.rs` **没有开关** ⇒ 生产里 `hooks()` 恒空 | ⭐ **「默认关」其实等于「没有打开路径」**（口径问题，非本笔修） |

# 三、改法（**最小权限**：不放宽任何治理口径）

**不发布「没有身份就不可能兑现」的能力**，而不是给默认桌面发一个身份。

- `memory_module.rs`：新增 `pub fn is_namespace_scoped(capability: &CapabilityId) -> bool`
  （按能力前缀 `tool.memory.` 判定 ⇒ **单一真源**，与 `capability()` 用同一个常量）；
- `desktop_host.rs`：桌面 main 发布工具时**过滤掉命名空间受限的能力**
  ⇒ ⇒ 没有身份 ⇒ 这张脸**根本不出现** ⇒ ⇒ 模型不会去调一个必然失败的工具。

⭐ **刻意不做**（避免放宽权限面）：
- ❌ 不给默认桌面 main 分配 Profile/Namespace 身份（那等于**凭空扩权**）；
- ❌ 不给 `MemoryModule` 补 `module.memory` grant（那会**扩大授权面**）；
- ❌ 不动审批/治理口径。

⇒ **受治理的记忆路径完全不受影响**：行为模块（召回 / 写回进对话）照旧。

# 四、证据（真跑）

`crates/engine/runtime-assembly/tests/memory_tool_authority_desktop.rs`（**4 例全绿**）：

| 用例 | 钉住什么 |
|---|---|
| `desktop_main_offers_no_namespace_scoped_tool` | 默认桌面 main **不发布**任何命名空间受限能力 |
| `an_out_of_scope_memory_call_never_reaches_the_module` | 越界调用在**分发器的范围闸**就被拒，**到不了**记忆模块 |
| `the_desktop_memory_behavior_path_still_works` | **受治理**的记忆路径（召回 / 写回）**照旧可用** |
| `a_host_bound_identity_still_operates_the_memory_tools` | 给了 Host 身份 + 模块 grant ⇒ 记忆工具**照常能用**（这张脸没被改坏） |

**反证**（实测红过）：
- 去掉 `desktop_host.rs` 里的 `filter` ⇒ `desktop_main_offers_no_namespace_scoped_tool` **红**
  （记忆工具又被发布出去）；
- 去掉 `module.memory` grant ⇒ `a_host_bound_identity_still_operates_the_memory_tools` **红**。

**门禁**：`cargo test -p apeireth-runtime-assembly --lib` **429 passed** ·
`cargo test -p apeireth-runtime-assembly --test memory_tool_authority_desktop` **4 passed** ·
`cargo test -p apeireth-cli --test canonical_cli_bootstrap --test care_heartbeat_host` **全绿** ·
`cargo clippy -p apeireth-runtime-assembly --all-targets -- -D warnings` **0 warning** ·
`cargo fmt --check` 干净 · `arch_guard --strict` **VIOLATIONS 0** · 孤儿基线 **86 不变**。

# 五、⚠️ 没证明 / 仍然欠着（如实列）

1. ⭐ **默认桌面路径上，记忆工具面现在「不可用」**（不是「可用」）
   ⇒ ⇒ 想让默认桌面真的能用记忆工具，必须**给桌面 main 绑一个 Host 身份**
   ⇒ ⇒ 那是**产品决策**（身份从哪来、scope 是什么、跨重启怎么保持），
   **本笔刻意不做**（凭空扩权比缺功能更危险）。
2. ⭐ **`memory_organize` 在默认桌面策略下仍起不了子 Agent**
   （缺 `module.memory` grant）⇒ 本笔**未动**（补 grant = 扩授权面）。
3. ⭐ **「固定 Hook」在生产仍不可达**（硬编码关 + 没有开关）
   ⇒ 「默认关」与「没有打开路径」**不是一回事** ⇒ 需要单独一笔把它变成
   **默认关但可开**（本笔未做）。
4. 未测：真机 / 真模型 E2E；多主体并发；跨重启的身份保持（**默认桌面现在没有身份**）。
5. `desktop_host.rs` 有 `EXC-0002`（size-grow 抑制）⇒ 本笔**仍按行中性自律**处理
   （只加 6 行 ⇒ 同文件未腾行 ⇒ **实际增长了 6 行**，已在提交说明里写清）。

# 六、复现

```sh
runuser -u bai -- env HOME=/home/bai LC_ALL=C.UTF-8 CARGO_INCREMENTAL=0 RUSTC_WRAPPER= \
  bash -c 'cd /home/bai/wt-apeireth-r3 && \
    cargo test -p apeireth-runtime-assembly --test memory_tool_authority_desktop --offline'
```
