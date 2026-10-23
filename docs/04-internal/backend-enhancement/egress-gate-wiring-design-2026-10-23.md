# 模型出网闸（Egress Gate）接线设计 —— 让治理管住「嘴」

> 日期：2026-10-23 · 状态：**设计（待实施）** · 关联：ADR-0013 §2.2（承重墙）、
> 未接线台账 W 系列、`redaction_bridge.rs`（41KB，已存在未接电）
> 上位结论：**先给模型出网装闸 → 才能做「能力面开、通道面关」**（ADR-0013 §2.2 + 顺序约束）

## 1. 问题：治理管手脚，不管嘴

已核实的三个事实：

1. `crates/engine/runtime/src/canonical/provider.rs` 对
   `guard|governance|policy|approval|consent` 的引用数 = **0**；
2. `execute.rs` 把 `NormalizedRequest` 装配完**直接发出**（约 `:1645-1665`）；
3. `governance/chain.rs` 六阶段（Hook 询问 / 审议 / 多签 / 审批闭合 / 审计追加 / 反思自停用）
   **没有任何一段检查"这段内容能不能出境"**。

也就是说：工具调用受治理，**送给模型的内容不受治理**。而 ADR-0013 的红线
（"用户不想外露的数据不能外流"）恰好落在这条无人看管的通道上。

出境脱敏闸本身已写得很完整（`redaction_bridge.rs`：HMAC 占位、同会话恒同、
跨会话换别名、回程可还原、历史回扫、零真值账本），**缺的是把它挂在模型出口上**。

## 2. 设计：一个端口 + 一个卡点 + 两档语义

### 2.1 卡点选在 `ProviderRouter::complete_inner`

所有 provider 调用（普通 / 带工具 / 流式）都汇到 `complete_inner`，
是唯一的出网咽喉。**在这一处挂闸**，等于覆盖全部模型出口，
不需要在十几个 adapter 里各挂一遍。

### 2.2 端口放 runtime，实现放 assembly

```
runtime (canonical/provider.rs)
  trait EgressGate: Send + Sync {
      fn vet(&self, request: &NormalizedRequest) -> EgressVerdict;
  }
  enum EgressVerdict { Pass, Redacted(NormalizedRequest), Deny { reason: String } }

runtime-assembly (新文件 egress_gate.rs)
  struct RedactionEgressGate { bridge: Mutex<RedactionBridge>, policy: EgressPolicy, ledger: ... }
  impl EgressGate for RedactionEgressGate { ... }
```

方向正确：runtime 只定义端口与判定枚举，**不依赖** tools / governance 具体实现；
assembly 层把 `redaction_bridge` + 对象级判定 + 策略接进来；CLI 装配根注入。

### 2.3 两档语义（关键：不是一律脱敏，也不是一律拒绝）

| 判定 | 何时 | 结果 |
|---|---|---|
| **Pass** | 内容不含受保护对象 / 用户已授权该用途 | 原样发出 |
| **Redacted** | 含受保护对象，但允许"以占位形式"出境 | 发占位（`⟨pii-xx…⟩`），**回程还原** |
| **Deny** | 该对象禁止出境 / 未授权用途 / 策略拒绝 | **拒绝即帧**：不发请求，返回可读原因 |

fail-closed：闸未接线 / 判定失败 / 策略读取失败 → **一律 Deny**（宁可不发，
不可猜测放行）。这条与仓内既有纪律一致。

### 2.4 出域账本

每次判定追加一行（append-only，坏行读作不存在，与 `governance-audit.jsonl` 同纪律）：

```json
{"ts":..., "session":"…", "verdict":"redacted", "objects":["pii:person_name"], "usage":"chat", "request_id":"…"}
```

**不回真值**（占位与对象类型即可），账本本身也不得成为泄漏面。
与 `RedactionBridge::ledger_jsonl()` 的零真值账本对齐，不另造一套。

## 2.5 「边界」不止网络一条（外部顾问判据，ADR-0014 §1.2）

顾问指出：「自由在头脑、约束在双手」在 agent 里失效——**把上下文发给模型就是行动**。
因此闸的判定对象不能只是"出网"，而是**五条边界**，每条都要有判定点：

| # | 边界 | 判定点（现状） |
|---|---|---|
| 1 | 原始记忆 → 派生摘要 | 摘要/折入路径 |
| 2 | 私人对象 → 工作对象 | 对象级判定（`object_access`） |
| 3 | 用户自己 → 第三方 | 出网控制（`egress`） |
| 4 | 内部推断 → 外部行动 | 工具执行审批 |
| 5 | **模型上下文 → provider** | **本设计（当前零挂载）** |

本闸先覆盖第 5 条（最缺的一条）；其余四条已有或部分有判定点，按 ADR-0014 落地清单补齐。

## 3. 失败语义与可观测性

- **拒绝原因必须可读**：`"这次被拦是因为：你的住址属于『禁止出境』对象；下一步可在设置里授权该用途，或改问不涉及住址的问题"` ——
  这正是副总指挥说的"缺拦截原因可读性"（`onboarding.rs:15` 的真问题不是默认值太薄）；
- 拒绝要进自述面（`self_status`）与审计链，**不静默**；
- 闸本身的健康状态进自述面（已接线 / 判定次数 / 最近拒绝原因类型），沿用既有 probe 模式。

## 4. 实施顺序（每步可独立合并、可回滚）

1. **端口 + 单测**：runtime 加 `EgressGate` 端口与 `EgressVerdict`；`complete_inner` 调用点；
   默认实现 = `Pass`（保持零行为变化，先不接线）。
2. **实现 + 装配**：assembly 新增 `egress_gate.rs`（用 `RedactionBridge`），
   CLI 装配根按 env（`APEIRETH_EGRESS_GATE`，默认关）注入；默认关时逐字节零变化。
3. **判定策略**：对象级判定（`object_access`）+ 用户授权面（按对象 × 用途、可撤销）。
4. **账本 + 自述面**：出域账本落盘 + 自述面一节（沿用既有 probe 纪律）。
5. **默认开评估**：等 2-4 稳定、有真实使用数据后，再评估是否默认开（**不抢跑**）。

## 5. 测试计划

- 端口层：`Pass` / `Redacted` / `Deny` 三分支各一例；闸返回值不改变 request 时逐字节相等；
- 卡点层：拒绝时**不发请求**（用 mock provider 断言零调用）；
- 脱敏层：占位出境 + 回程还原后与原文一致（同一会话）；跨会话占位不同；
- fail-closed：闸 panic / 策略读失败 → Deny；
- 账本：坏行读作不存在；账本不含真值（grep 断言）。

## 6. 不做（明确边界）

- 不承诺 provider 侧行为（"模型侧残留已抹除"这类承诺不写进能力声明）；
- 不做"一律脱敏"：有些内容脱敏后已无意义，该拒就拒；
- 不在本设计里改 provider adapter（不动十几个 adapter，只动咽喉一处）；
- 不默认开：先接线、默认关、有数据再评估。

## 7. 与其他待办的关系

- **W1（IM 审批卡）**：与本设计无关（那是设备审批送达面），可并行；
- **B2 顺序约束**：本设计第 1-2 步落地前，**不得**把默认工具面开大；
- **claims**：本设计落地后新增 claim 时，`level` 用阶梯（ADR-0013 后的口径），
  `default` 单独标（默认关 = `"OFF"`）。
