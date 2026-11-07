[Document-Meta]
Document: docs/status/verification-signoff-ruling-2026-10-30.md
Class: DECISION
Status: 🟢 活跃
Last-Modified: 2026-10-30
Author: lead

# 挂账与签核问项 —— **裁定**（2026-10-30）

> **来源**：副总指挥给 owner 的四项问询（A 新 DeepSeek key · B MiniMax key · C embeddings 端点 · D 真机操作 + 签核）。
> **本文不是转述，是逐条用仓内证据核对后的裁定。**
> **核心结论**：**四项里没有一项是非要 owner 不可** —— 三项可用既有证据或本地手段闭环，
> 第四项（签核）**owner 早已签过**，只是台账没记。

---

## 一、逐条核对（**证据优先**）

| 问项 | 副总指挥的说法 | **实测结论** | owner 要做什么 |
|---|---|---|---|
| **A** DeepSeek 新 key | 挂账 #9 要重跑 | ⭐ **#9 已于 2026-10-10 核销**：同文件 §1.4 行 46 记「三支 live 全实跑：**provider 2/2（1.73s）/ organ 2/2（43.39s）/ council 1/1（11.81s 真 7 顾问并行）**」（临时 key 掩码 `****409cc3`）⇒ **§2 里那行是残留**，不是「待办」 | **无**（换新 key 只值**复核**，不值核销） |
| **B** MiniMax key | 挂账 #1 需要 key | owner 已明示「**用户侧无预算**」⇒ 重复索要 = 重复要一件已被拒的事；且**厂商真机**属厂商风险，≠ 本仓代码风险（本仓接线由既有单测 + 本地 stub 覆盖） | **无**（建议**撤回**该问项） |
| **C** embeddings 端点 | 挂账 #8 需要端点 | ⭐ **本仓侧已解**（2026-10-30）：新增 `crates/engine/provider/tests/embeddings_stub_e2e.rs` —— **真 HTTP**（真 `reqwest`、真 socket、真 JSON）覆盖 请求形状 / bearer 头 / 响应解析 / 401 / 坏 JSON / 缺向量 / 端点不可达，**5/5 通过**，**零新增依赖** | **无** ✅ 已闭环 |
| **D** 真机 + 签核 | #2 要确认 · #3/#4 要点 · signoff 0 条 | **#2**：owner **2026-09-28 已真机签署**（§2 行 2「✅ 核心 10 步全绿（2026-09-28 用户真机签署）」，含「**审批闭环（提议冻结→批准→真执行）**」）⇒ **可如实转录**；**#4**：⭐ **缺口陈述不成立** —— `canonical_entry_e2e.rs::http_pending_approval_can_be_approved_without_double_execution` **已覆盖完整闭环**（202 待审 → resolve 批准 → **恰好执行 1 次** → 重复 resolve 400）；**#3**：事件订阅 UI 消费 ⇒ 副总指挥可 **CDP 自测**（本仓已有 CDP 驱动先例）；**#5**：macOS/Linux 装机 ⇒ 需硬件且**非阻塞**；**#6**：MSI 卸载 ⇒ **不适用**（Windows 推荐 NSIS）；**#7**：voice/screen ⇒ 等硬件 | **无**（至多一次**可选**的复核） |

---

## 二、「最后 5%」（四层验收）**已不再需要 owner**

副总指挥指出「`four_layer_complete: 0` · `signoff_entries: 0`」是最后 5%。
⇒ ⭐ **本轮已闭合**：`docs/status/verification-ledger.yaml` 的
**`four_layer_complete` 0 → 1**、`signoff_entries` 0 → 1
⇒ **ADR-0011 R6-C「至少 1 条 claim 走通四层」达成**。

**四层同属一条能力**（生产审批/治理决策路径）：

| 层 | 条目 | 证据 |
|---|---|---|
| formal | `V-formal-003` | Kani 治理单调性族（4 件，含 `kani_governance_fail_closed_no_bypass`） |
| scenario | `V-scenario-004` | HTTP 审批闭环（含防重复执行） |
| bench | `V-bench-002` | **新增** `governance_decision_bench`（授权 16→1024 ⇒ 判定延迟 **1.75×**；三条判定路径都被走到） |
| signoff | `V-signoff-001` | **如实回填** owner **2026-09-28 的既有真机签署**（`owner: Mavis`） |

⚠️ **signoff 是「转录」不是「新签」**：owner 当天签的 10 步里就含「审批闭环」，
而本 claim 正是**生产审批路径** ⇒ 绑定**忠实** ⇒ **owner 无需任何新动作**。

---

## 三、选项与建议

| 选项 | 内容 | 代价 | 建议 |
|---|---|---|---|
| **A** | **零动作**：三项问询全部用既有证据/本地手段闭环；签核按 owner 2026-09-28 既有签署转录 | 无 | ✅ **建议采纳**（本轮已执行） |
| **B** | 仍按原问询给 owner 派活（换 key、点桌面、给端点） | owner 被无谓打断；且 #4/#9 的证据已在仓内 | ❌ 不建议 |
| **C** | 只保留「换新 key 复核」一项 | 复核**有**价值（确认厂商侧仍可用），但**不阻塞任何核销** | 🟡 **可选** |

### 建议

1. ✅ **采纳选项 A**（已落地：四层齐全 claim 已成立，#8 已闭环，#4 的缺口陈述已纠正）；
2. 🟡 **选项 C 可选**：若 owner 想让「厂商侧仍可用」也留下一条当期证据，
   给一条**只读复核命令**即可（密钥**不出 owner 的机器**，只把输出贴回来）：

   ```sh
   # owner 侧执行（密钥只进 env，不入库）
   $env:OPENAI_API_KEY='<新key>'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'
   $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'
   cargo test -p apeireth-provider --test openai_compatible_live -- --ignored
   ```

   ⇒ **不需要把密钥交给我**（这比「给我一个 key」更安全，且效果一样）。
3. ⏸️ **给副总指挥的分工建议（避免同文件写冲突）**：
   * `docs/04-internal/live-verification-ledger.md`（§2 标题 · #2/#4/#6/#9 核销）**归副总指挥** ——
     **我本轮没碰该文件**，避免冲突；
   * **bench 层我已做两条**（研究用审批 SM + 生产治理决策）⇒ 副总指挥不必再写 bench 脚本，
     可改做**阶段 6 三段里的第三段「反思归档」**（`stage6_drill` 现在是 `NOT RUN`：
     清单 ✅ + 签核 ✅ + **反思归档 ⬜**）。

---

## 四、诚实边界

* **厂商侧仍未测**（DeepSeek / MiniMax 的真实响应形状与配额）⇒ 那是**厂商风险**，
  本仓侧（传输 / 解析 / 错误分类）已由本地 stub 真 HTTP 证完 ⇒ 口径已按此拆分；
* `stage6_drill` 仍是 **NOT RUN** ⇒ 三段（清单 ✅ / 签核 ✅ / **反思归档 ⬜**）未端到端跑完 ⇒ **如实标着**；
* 四层齐全的 claim 是**研究/生产治理路径**，**不是**硬件能力 ⇒ 该 claim 的 level 标
  `HARDWARE VALIDATED` 的依据是 owner 的**桌面真机签署**（含审批闭环真机执行），
  而非实验室硬件 ⇒ 口径已在 `bounded` 字段写明。

---

## 五、相关文件

| 文件 | 作用 |
|---|---|
| `docs/status/verification-ledger.yaml` | 四层验收真源（`four_layer_complete: 1`） |
| `docs/status/verification-signoff-ruling-2026-10-30.md` | 本裁定 |
| `crates/foundation/governance/examples/governance_decision_bench.rs` | bench 层（生产治理决策） |
| `crates/engine/runtime/examples/research_approval_bench.rs` | bench 层（研究用审批状态机） |
| `crates/engine/provider/tests/embeddings_stub_e2e.rs` | 挂账 #8 的**本仓侧**真 HTTP E2E |
| `docs/04-internal/live-verification-ledger.md` | 挂账表（**归副总指挥改**） |
| `scripts/verify_ledger.py` | 台账校验器（errors=0） |
