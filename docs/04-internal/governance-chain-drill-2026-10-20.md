# 治理链端到端合练记录（B7 · 2026-10-20）

> 对应隐患 #7「组件已写、整条链未演练」与 `docs/04-internal/backend-enhancement/00-backlog.md`
> D8「治理流程"智囊团审议 + 物理多签"整机未合练」。
> 演练日期：2026-10-20（Asia/Shanghai）。可复跑入口：
> `crates/foundation/governance/tests/governance_chain_drill.rs`（3 个集成测试）。
>
> **口径（0 装，严守）**：本演练对象是**现役 M-of-N 逻辑票链**（字符串票
> `"signer:digest"`，`onion.rs` 自述 0 装占位、非真密码学校验）；**逻辑票 ≠
> 密码学多签**，物理不可伪造的多签留 v2.1（stage123-gap-matrix B7d 显式挂账）。
> 本记录是「链可走通」的执行证据，**不是**「生产已接线」的证据（见 §5 缺口）。

## 1. 链路图（现役零件与本次串联顺序）

```
提案 (gate::Action, L0 类变更)
  │ ① V1+V2+V3 AND 门：core/src/gate.rs ActionGuard::check_action(:130)
  │    V3 = check_ha(:198)：MultiHuman 需登记真人 ≥ multi_sign.required
  ▼
审议意见记名 (governance/src/rubric.rs)
  │ ② Ballot(:117 具名 advisor_id) → synthesize(:166 反对意见记名 dissenting)
  │    → passes_strategy(:332 超多数/加权多数)；hold 形态 evaluate(:233)
  ▼
M-of-N 逻辑票 (core/src/onion.rs)
  │ ③ HumanAuthority::verify_multisig(:94)：2-of-3、去重真实人类、
  │    拒未登记者/畸形票；MultiSignPolicy(:176) MultiSignResult(:185)
  │    三路径：Accepted / Insufficient{required,received} / 超时不成案
  ▼
审批闭合 (governance/src/approval_closure.rs + user_access.rs)
  │ ④ ApprovalAuditPair(:227 asked↔decision) → commit_approval_audit_pair(:390
  │    事务成对) → scan_approval_pairs(:483)；谁可决 = may_resolve_approval(:254)
  ▼
审计落盘 (governance/src/audit.rs)
  │ ⑤ AuditHashChain(:104) append(:116 谁/何时/什么决定入 subject、时间显式注入)
  │    → JSON 落盘 → 读回 verify(:169) → 篡改一字检出 RecordHashMismatch
  ▼
反思期 (core/src/lib.rs)
  ▼ ⑥ SelfDisableAudit(:1598) record_reflection_query(:1652) +
     is_in_reflection_whitelist(:1313) + 违规报 HA；Gate::ReflectionAudit(gate.rs:27)
```

串联顺序与「hold/超时 ⇒ 不批准」的组合规则是**本次演练定义的胶水**：库里没有
单一编排函数把 6 步串成链（如实报：编排未接线）。各环节判定本身用的都是现役
真实实现，无一桩替身；V1 哲学守门用 mock 放行（演练对象是 V3 多签链，不是
V1 词表）。

## 2. 三路径执行记录（2026-10-20 实测）

测试文件：`crates/foundation/governance/tests/governance_chain_drill.rs`。
每步审计断言口径：**谁**（signer/advisor/resolver 具名写入 subject）、**何时**
（显式注入毫秒时间戳，不取墙钟，断言回读相等）、**什么决定**（结果标签写入
subject + 事件词表 event_kind）。

### 2.1 通过路径 `chain_pass_path_ratifies_and_persists_audit`

| 步 | 执行 | 断言结果 |
|---|---|---|
| ① 提案过三门 | `check_action(ReorganizeOnion/Critical, 2-of-3 HA)` | `Allow`；对抗：登记真人空集 → `BlockByHumanAuthority`（门真的在 gate） |
| ② 记名审议 | 7 席具名票（5 赞成/1 反对/1 弃权）→ `synthesize` | 弃权不计票 `opinion_count=6`；反对者 `advisor-history` 具名入 `dissenting`；不触发 hold；超多数 + 加权多数双策略通过 |
| ③ 逻辑票 | `verify_multisig([alice,bob])` | `Accepted`（2-of-3） |
| ④ 闭合 | owner 具名可决 + `ApprovalAuditPair(AllowedOnce)` 成对提交 | `scan_approval_pairs` 零未闭合/零异常 |
| ⑤ 反思期 | 白名单查询 + 元问题查询 | 白名单放行且 `whitelist_hits=1`；元问题拦下、`total_violations=1`、`needs_ha_alert=true` |
| ⑥ 落盘 | 11 条审计记录（1 提案+7 票+1 多签+1 决定+1 反思期）JSON 落盘 | 读回逐记录等价 + `verify()` 过；篡改 subject 一字 → `RecordHashMismatch` 检出 |

### 2.2 拒绝路径 `chain_reject_path_pins_named_dissent_and_insufficient_votes`

| 步 | 执行 | 断言结果 |
|---|---|---|
| ① 记名审议否决 | 形态 A（2 强反/3）；形态 B（3 全反） | A → `StrongDisapprovePercent` hold；B → `UnanimousDisapprove` hold；否决票不得过超多数策略 |
| ② 逻辑票不足 | 1/2 票 | `Insufficient{required:2, received:1}` |
| ②' 防凑票对抗 | 同人×2 + 未登记者 + 畸形票 + 空票 | 收票数仍 1（同人去重、未登记/畸形不计数） |
| ②'' 配置对抗 | offline 恒拒；single 带策略 | `DeniedOffline`；`InvalidHA` |
| ③ 拒绝闭合 | `ApprovalOutcome::Rejected`（fail-closed）成对提交 | 零悬挂 ask；跨用户审批（Member 有 L2 审批权但会话非本人）一律拒 |
| ④ 落盘 | 审计链校验 + 篡改检出 | 通过 |

### 2.3 超时路径 `chain_timeout_path_holds_and_fails_closed`

| 步 | 执行 | 断言结果 |
|---|---|---|
| ① 超时阈值 | `HOLD_DELIBERATION_TIMEOUT_MS=60_000` 边界 | 阈值前 1ms 不触发；到阈值必触发 `DeliberationTimeout` hold |
| ② 票未达法定数 | 超时窗口内 1/2 票 | `Insufficient`；审计记 `multisig.withheld / ratify=no` |
| ③ 超时收束 | `expired → ApprovalOutcome::Unavailable` | fail-closed 不授权；成对闭合无悬挂 ask；越权收尾（Guest）同拒 |
| ④ 落盘 | 3 条审计记录（超时/暂扣/决定）校验 + 篡改检出 | 通过 |

## 3. 实测数字（2026-10-20，可复核）

```
cargo test -p apeireth-governance --locked -j 2
  → lib 194 passed / integration 3 passed / doctest 0 —— 0 failed
cargo clippy -p apeireth-governance --all-targets --locked -j 2 -- -D warnings
  → 0 警告（exit 0）
cargo fmt --check（本文件 rustfmt 单文件 + 全仓）
  → 0 diff
```

## 4. 与挂账行销账对齐（只读对账，不改 00-backlog.md）

`00-backlog.md` **D2**（stage123-gap-matrix 挂账行）——本演练**一项未销**，如实记：

| D2 行 | 本演练影响 | 处置 |
|---|---|---|
| 物理多签 v2.1 | 未触碰：本演练只走逻辑票链，字符串票无不可伪造性 | **未销**（仍缺件，待 v2.1） |
| UpgradeCycle 留白 | 不在本批战场 | **未销**（理由在册） |
| 风险阈值校准 | 只钉阈值语义与 60_000ms 边界，不改阈值 | **未销**（仍需阶段实测校准） |
| Kani 真接线 | 不在本批战场 | **未销**（理由在册） |

`00-backlog.md` **D8**（整机未合练）——部分销账：

| D8 断言 | 本演练影响 | 处置 |
|---|---|---|
| 「审议→多签→反思期串联零执行记录」 | 3 个集成测试可复跑地走通三路径，每步审计具名落盘 | **已销**（执行记录 = 本文件 + 测试） |
| 「`multi_sign` 全库调用点均为 `None`」 | 未触碰生产调用点 | **未销**（卡点：接线点需产品拍板，见 §5-2） |
| 「反思期审计默认不运行」 | 测试内逐路径真跑反思期；生产面仍无周期调度 | **未销**（卡点同上） |

## 5. 发现缺口（0 装：没走通的环节如实记）

1. **编排未接线**：库内无单一编排函数把 §1 六步串成链；串联顺序与「hold/超时
   ⇒ 不批准」组合规则是演练胶水。走通的是零件序，不是生产编排。
2. **逐动作签名收集未接线**：`gate.rs:195` 自述「签名收集由
   `HumanAuthority::verify_multisig` 在调用方接线」，但该调用方在生产面不存在
   （全库 `multi_sign: None`）——`verify_multisig` 今天零生产调用点。
3. **物理（密码学）多签缺位**：签名是 `"signer:digest"` 字符串票，防凑票靠
   去重登记表，无不可伪造性；物理多签留 v2.1（D2/B7d 挂账，措辞严守：本演练
   交付的是**逻辑票链**，不是多签）。
4. **反思期无常驻调度**：`SelfDisableAudit` 需调用方构造并周期执行（D8 原话
   「反思期审计默认不运行」仍成立）；测试内是逐路径手动跑。
5. **审计持久化面缺位**：`AuditHashChain` 是内存链；本次「落盘」是测试胶水
   （fs_err + JSON 往返 + 篡改检出）。`ApprovalAuditSink` trait 是可落盘缝，
   现役只有 `MemoryApprovalAuditSink`——生产级持久 sink 未接线。
6. **审议计时无自动器**：`HOLD_DELIBERATION_TIMEOUT_MS` 只是阈值常量，
   `evaluate_timeout(actual_ms)` 的实际计时由调用方注入；库内无计时器。

## 6. 合规自查

- LOCKED 5 项 0 触碰；战场外文件 0 触碰（00-backlog.md / CHANGELOG.md 只读对账）；
- 0 装：见 §抬头与 §5，未走通环节零粉饰；M-of-N 逻辑票 ≠ 密码学多签措辞全篇一致；
- 中性表述：零第三方产品名；测试/文档零新增外部依赖（fs_err 为既有 dev 依赖）；
- 未 commit，等主代理审查统一提交。
