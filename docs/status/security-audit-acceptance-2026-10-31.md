[Document-Meta]
Document: docs/status/security-audit-acceptance-2026-10-31.md
Class: AUDIT
Status: 🟢 活跃
Last-Modified: 2026-10-31
Author: lead

# 静态安全审计 —— **验收报告**（2026-10-31）

> **被审对象**：owner 于 2026-10-28 安排的 AI 静态安全审计（两轮 scoped run）。
> **本文是验收，不是转述** —— 逐条对**当前代码**复核，并**已动手修了 4 条**。
> **产出目录**：`/root/security-audit-skill/apeireth/`（`INDEX.md` / `run-1/` / `run-2/`）。

---

## 一、审计方报告了什么（其口径）

| | run-1 | run-2（续轮） |
|---|---|---|
| 被审 ref | `f34a38f5` | `d10b70e3` |
| 范围 | 网关 / 工具 / 记忆 / provider / 凭据 / 协议 / 设备 / 治理 | `tools/src/{process,exec_pipeline,device}` |
| confirmed | 11（6 medium / 5 low） | 10（9 low / 1 informational） |
| needs_validation | 0（另 8 条未验证候选留档） | 2 |
| refuted | 1 | 3 |
| 覆盖单元 | 16 | 11 |
| 机器校验 | findings + ledger **PASS** | 同 **PASS** |

去重后 **20 条 confirmed**（run-2 的 10 条里 1 条是 run-1 携带重复）。

---

## 二、**我的验收方法**（不复现审计过程，独立复核）

1. **ref 对齐**：三个被审 ref **在本仓存在**（`7654d5b6` / `f34a38f5` / `d10b70e3`），
   但我的 HEAD 领先它们 **268 个提交** ⇒ ⇒ 审计方那句「`crates/` 与 `docs/` 零变动 ⇒ 不需要重跑」
   **只对他们那份 checkout 成立**，对本仓**不成立** ⇒ ⇒ 必须逐条核。
2. **文件级影响面**：23 条记录涉及的 23 个文件里，**19 个未变**、**4 个已变**
   ⇒ ⇒ 未变的 19 条**原样仍适用**；已变的 4 条要**重新核**。
3. **逐条复核**：两个独立复核代理分别核 medium(6) 与 low/info(17)，我在其之外**亲自深挖 4 条**
   （`universal_forget` / `egress` / `egress_gate` / `persona`）。
4. **发现审计方口径的一处偏差**（重要）：审计产物的 `file:line` 是在**另一棵代码树形态**上测得的
   （`feat/env-lock-crate-level` 分支形态，`git_write/parse.rs` 独立成模块）
   ⇒ ⇒ 当前树里 **9+ 处证据行号整体位移** ⇒ ⇒ **主张成立，但引用已陈旧**（两者分开裁定）。

---

## 三、总裁决：**审计实质可信**（不是「照单全收」，也不是「打回去」）

| 裁定 | 条数 | 说明 |
|---|---|---|
| **仍成立** | 15 | 主张与前提在当前代码都成立 |
| **部分成立** | 5 | 主张成立但行号陈旧 / 前提变化 / 影响被夸大 |
| **重复** | 1 | `#2` 与 `#8` 逐字相同 ⇒ 只计 1 条 |
| **待验已裁定** | 2 | 1 条**提升**为 confirmed/low，1 条**不提升** |
| **驳回** | 1 | `#5 persona 常量注入` —— **审计方的指控不成立**（证据见 §六） |

⇒ ⇒ **去重后 20 条里 19 条成立**（1 条误判）⇒ ⇒ 审计的**质量是高的**
⇒ ⇒ 值得直接进入修复流程。

---

## 四、**已修**（本轮 4 条，全部有 commit + 测试）

| # | 发现 | 修法 | commit |
|---|---|---|---|
| **M1** | `egress.rs:classify_ipv4` —— RFC 6598/6890/5737/2544/1112/1122 特殊段被判成公网 ⇒ **阿里云元数据 `100.100.100.200` 可达**（云 SSRF） | 新增 `Shared`/`Reserved` 分类；**IPv6 解包 NAT64 / 6to4**（堵内嵌绕过） | `37bb6984` |
| **M2** | `fetch.rs:execute_with_egress` —— 远程页面**原文**裸嵌给模型 ⇒ 间接提示词注入 | 复用仓里既有 `UntrustedContentWrapper`（带**边界伪造中和**）；`body` 与 `text` **都**包裹 | `03e6ef5a` |
| **M3** | `egress_gate.rs:egress_gate_from_env_with_all` —— 策略值畸形 ⇒ `return None` ⇒ **闸根本不装** ⇒ fail-open | 抽出纯函数 `plan_authorization`；畸形 ⇒ **fail-closed**（闸照装 + 空授权 ⇒ 一律拒） | `0951393b` |
| **M4** | `universal_forget.rs:forget_principal_on_conn` —— 返回体五个布尔**写死常量** ⇒ 「报告已擦除」与「实际擦除」脱钩 | 改为**实测**（新增 `ForgetCounts` / `AttributionStatus`）；归属不可判定 ⇒ **不谎称已擦除** | `b9a8c99f` |

**每条的验证**：`apeireth-memory` / `apeireth-tools-canonical` / `apeireth-runtime-assembly`
三个 crate **全绿（0 failed）**，`clippy -D warnings` **0 警告**。

---

## 五、**未修 / 暂缓**（含根因与建议，按优先级）

| 优先级 | 发现 | 现状 | 根因 / 建议 |
|---|---|---|---|
| **P1** | `universal_forget` 的**根因**（M4 只修了「不谎称」） | 生产侧**仍无** `memory_principal_sessions` 写入方 ⇒ episode 维度「被遗忘权」**仍未真正生效**（现在只是**如实报告**） | 需决定「生产怎么记录主体→会话归属」⇒ **属产品语义决策**，不由我单方面拍 |
| **P1** | `canonical_anthropic.rs:complete` —— 凭据与端点**无绑定**（默认端点 `api.minimaxi.com`，`x-api-key` 发往该主机） | 仍成立 | 换端点会把旧 key 发给新主机 ⇒ 需「**凭据绑端点**」机制（凭据库维度）⇒ 同样属设计决策；**先记不做** |
| **P2** | `admin_shutdown` —— 跨源 simple POST 直达停机口（CORS 只管响应头） | 仍成立（low） | 需在状态变更路由上**校验 Origin** |
| **P2** | `keyring.rs:EncryptedFileBackend::open` —— check-then-write 无 `create_new`/无跨进程锁 | 仍成立（low） | 对照 `FileCredentialsStore` **已有**文件锁 ⇒ 可照抄 |
| **P2** | `ws_session.rs:admit_auth` —— 只比 `ws_version`，token **从未读** | 仍成立（low），但**仓内无传输接该 guard**、gateway 零 WS 端点 | ⇒ 实际不可达 ⇒ 降级为「接线时必须带 token」的前置条件 |
| **P2** | `appcontainer.rs` —— `FILE_ALL_ACCESS` 落盘不回收 + 固定名⇒确定性 SID | 部分成立（low） | 需回收 ACL + 改随机名 |
| **P3** | 其余 low/info 11 条（device×3 / exec_pipeline×2 / git_write / weather / macos / process×2 / windows） | 仍成立或部分成立（low / informational） | 逐条低危，按成本排序慢慢做 |

---

## 六、**驳回**的一条（验收的价值所在）

### `#5 cognitive.rs:PersonaConstantInjectionModule` —— 「persona 常量注入无 scope」

审计方依据：`persona_layer.rs:1188` 读**无 scope 的单例行** ⇒ 推断「用户人格跨会话泄漏」。

**我的复核结论：不成立（对「泄漏用户人格」的指控）**：

1. 注入内容取自 `build_constant_injection`（`persona_layer.rs:788`）⇒ 它**只读** `memory.self_narrative`
   ⇒ ⇒ 那是**助手自己的身份自述**（注入模板字面就是「【我是谁 · 恒定自述】」）
   ⇒ ⇒ **全局是设计如此**，不是「某个用户的人格」。
2. **用户人格**走 `persona_profile_governance` ⇒ **按 `subject_id` 分域**
   （`persona_store_sqlite.rs:128 / 172 / 216` 三处读取均带 `subject_id=?2`）
   ⇒ ⇒ 不存在跨会话泄漏路径。

⇒ ⇒ **裁定：驳回**（保留在文档里，是因为「驳回」本身要有据可查）。

---

## 七、**已修 / 未修**分界与下一轮动作

* **已修 4 条**（M1–M4）—— 见 §四。
* **未修 19 条**（含 1 条驳回、2 条待验裁定）—— 见 §五。
* **下一轮建议顺序**：
  1. **P1 两条先做「设计决策」**（归属映射怎么记 / 凭据怎么绑端点）⇒ 需要产品语义拍板；
  2. **P2 四条**是纯工程（Origin 校验 / 文件锁 / 接线前置 / ACL 回收）⇒ 可直接做；
  3. **P3 十一条**低危 ⇒ 按成本排序推进。

---

## 八、诚实边界（**沿用并补充审计方的**）

* 本审计是**分片 scoped run**，**不是完整审计**；纯源码审计**未执行目标代码**；
  **Tauri 外壳未审**；`guardrail` 的后置清扫**未挂** `exec_pipeline`；
  run-1 的 **8 条未验证候选未复核**。
* **我补的边界**：审计方的 `file:line` 引用取自**另一分支形态** ⇒ 当前树**已位移** ⇒
  **主张成立 ≠ 引用可用**（下一轮审计报告应改口径或重生成）。
* 复核基线：`8ce0f62b`；§四四条修复之后，medium 6 条的裁定更新为
  **已修 3 条 / 部分成立 1 条（根因未修）/ 驳回 1 条 / 未修 1 条**。

---

## 九、复核命令

```sh
# 两条复核文档（本次验收的逐条依据）
less docs/status/security-audit-verification-medium-2026-10-31.md
less docs/status/security-audit-verification-low-2026-10-31.md

# 已修四条的重跑
cargo test -p apeireth-memory --locked                      # M4
cargo test -p apeireth-tools-canonical --locked             # M1 + M2
cargo test -p apeireth-runtime-assembly --locked            # M3
cargo clippy -p apeireth-memory -p apeireth-tools-canonical -p apeireth-runtime-assembly \
  --all-targets --locked -- -D warnings

# 文档守卫
python3 scripts/check_doc_class.py --check
python3 scripts/docs_links.py --check
```

---

## 十、相关文件

| 文件 | 作用 |
|---|---|
| `docs/status/security-audit-verification-medium-2026-10-31.md` | medium 6 条逐条复核 |
| `docs/status/security-audit-verification-low-2026-10-31.md` | low/info 17 条逐条复核 |
| `crates/capabilities/tools/src/egress.rs` | M1：特殊用途段 + NAT64/6to4 解包 |
| `crates/capabilities/tools/src/fetch.rs` | M2：远程内容不可信包裹 |
| `crates/engine/runtime-assembly/src/egress_gate.rs` | M3：畸形配置 fail-closed |
| `crates/engine/memory/src/universal_forget.rs` | M4：返回体实测 + 归属可见 |
