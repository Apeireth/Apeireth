# 审计与核对类文档 · 合并册索引

> **本目录定位**：把散落在仓库各处的**审计 / 核对 / 落差清账类**文档合成 4 册结论册，
> 目的是**消灭重复叙述、保留全部结论**：原件每一条结论、每一张表、每一条残余清单与
> owner 决策位都在册内留痕，标注来源日期与「现状复核（今天还成立吗）」。
>
> **合成日期**：2026-10-23 · **复核基线**：当日仓内事实核验（工作树 HEAD `b4aabfc2`）。
> **写入范围**：本目录（`docs/04-internal/audits/`）为唯一写入面；本批**0 git 写操作**，
> **0 触碰**其它任何文件；原件迁档与留桩由整合线负责，本批**不动原件**。
>
> **口径纪律**（沿用仓内现行口径，见 [AGENTS.md](../../../AGENTS.md)）：
> - 四级口径：`IMPLEMENTED`（代码存在）≠ `PRODUCTION WIRED`（接主路径）≠
>   `DEFAULT ENABLED`（默认开）≠ `HARDWARE VALIDATED`（真机验证），定义见
>   [system-capabilities.md](../../01-architecture/system-capabilities.md) 头部。
> - **现行真源只引用不抄写数字**：测试基线 [test-baseline.md](../../03-reference/test-baseline.md)、
>   能力声明 [claims.yaml](../../status/claims.yaml)、架构契约
>   [contract.toml](../../architecture/contract.toml)。
> - **历史不追溯改写**：过期结论不删，标「历史口径（YYYY-MM-DD）」。
> - **中性表述**：本册不写第三方产品名；仓内既有引用与研究引注按施工纪律豁免表处理。

---

## 一、4 册一览

| 册 | 文件 | 定位 | 覆盖的来源类别 |
|---|---|---|---|
| 01 | [01-architecture-and-capability-audit.md](01-architecture-and-capability-audit.md) | **架构与能力审计结论册** | 能力缺口类、SPEC 漂移类、成熟度类、旋钮对账类、施工工序类审计 |
| 02 | [02-security-and-governance-audit.md](02-security-and-governance-audit.md) | **安全与治理审计结论册** | 能力级别抬级取证、行为落差清账、治理链合练、安全硬化交付、沙箱、凭据类审计 |
| 03 | [03-caliber-and-text-hygiene-audit.md](03-caliber-and-text-hygiene-audit.md) | **口径与文本可信度审计结论册** | 口径尾账类（caliber / 守门级数 / 键值口径）、文档卫生类（`.gitignore`）、活跃文档过时扫描类、宣称→证据矩阵类 |

三册均按同一模板书写：**审计对象与日期**（逐份来源各标日期，不改写）→ **审计方法**一句话 →
**结论摘要**（3–6 条）→ 逐份来源分节（**结论** + **现状复核**）。

---

## 二、已合并的原件（完整清单）

> 下表「旧路径」一律为**仓库根相对路径**（以代码片段登记，不做成超链接，避免迁档后断链）。
> 「并入」列给出册号与节号；节号对应各册内的二级标题序号。

### 2.1 并入 01 册（架构与能力）

| 旧路径 | 并入 |
|---|---|
| `docs/04-internal/v1-vs-v2-capability-gap-audit-2026-10-06.md` | 01 §1（v1/v2 能力差距 + 未实现愿景） |
| `docs/04-internal/w3-legacy-gap-archaeology-2026-10-10.md` | 01 §2（W3 九缺口 legacy 考古） |
| `docs/04-internal/r9-r10-spec-drift-audit-2026-08-28.md` | 01 §3（R9/R10 spec drift 真账） |
| `docs/04-internal/round-12-final-gap-audit-2026-08-28.md` | 01 §4（Round 12 终极审计：调研完成度 + 缺口清单） |
| `docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md` | 01 §5（Round 13 1.0 maturity 补查） |
| `docs/04-internal/capability-knob-audit-2026-09-12.md` | 01 §6（功能旋钮四方对账） |
| `docs/04-internal/sub-agent-audit-round-4-2026-08-28.md` | 01 §7（Sub-Agent 审计 Round 4 工序留痕） |

### 2.2 并入 02 册（安全与治理）

| 旧路径 | 并入 |
|---|---|
| `docs/04-internal/backend-enhancement/implemented-to-wired-plan-2026-10-21.md` | 02 §1（12 条 claim 取证：IMPLEMENTED → PRODUCTION WIRED 抬级方案） |
| `docs/04-internal/backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md` | 02 §2（落差清账·行为小修批 E-06/E-07/C-05） |
| `docs/04-internal/governance-chain-drill-2026-10-20.md` | 02 §3（治理链端到端合练记录 B7） |
| `docs/04-internal/backend-enhancement/security-hardening-delivery-2026-10-20.md` | 02 §4（安全加固线交付报告 B1–B8） |
| `docs/04-internal/backend-enhancement/security-restart-closeout-2026-10-22.md` | 02 §5（安全线重启批收口 S1–S3） |
| `docs/04-internal/backend-enhancement/egress-gate-wiring-design-2026-10-23.md` | 02 §6（模型出网闸接线设计：治理管住「嘴」） |
| `docs/04-internal/secret-management-policy.md` | 02 §7（凭证管理规范 R215 防御层） |
| `reports/s1-sandbox-hardening-self-audit.md` | 02 §8（S1 最小权限执行自审，v1 时代） |
| `reports/v2-rc1-backend-security-audit-2026-09-24.md` | 02 §9（v2.0.0-rc.1 后端代码安全审计） |

### 2.3 并入 03 册（口径与文本可信度）

| 旧路径 | 并入 |
|---|---|
| `docs/04-internal/backend-enhancement/audit-gap-caliber-hygiene-2026-10-16.md` | 03 §1（落差清账·口径尾账批 B-04/B-05/D13/F-02，含 2026-10-18 守门口径补测） |
| `docs/04-internal/gitignore-hygiene-audit-2026-08-28.md` | 03 §2（`.gitignore` 卫生审计） |
| `reports/audit-2026-08-19/active_docs_audit.md` | 03 §3（活跃文档过时引用扫描 + README 中英脱节 + 顶层一致性） |
| `docs/04-internal/claims-evidence-matrix.md` | 03 §4（宣称 → 证据对照矩阵，含基准表溯源与 Kani 命题建议） |

---

## 三、保留未合并的原件（含原因）

> 「保留」= **本批不改、不并、不动**，原件继续以原路径在库。

| 原件 | 保留原因 |
|---|---|
| `docs/04-internal/cognitive-module-wiring.md` | **活文档（唯一 slot 接线账本）**：被大量引用（模块注册顺序、hook 位点、WIRED/OFF 状态的真源），内容随代码逐批变动；合并会立刻产生第二份口径。**只引用、不改、不合并**。引用点见 01 §1 / 02 §3。 |
| `docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` | **主账**，承担「v1 功能全集清单」的历史留存用途，且已带 2026-10-06 历史批注；01 §1 的结论以它为被审计对象，被合并会形成自指。 |
| `docs/04-internal/backend-enhancement/system-gap-ledger-2026-10-18.md` | **活台账**（未接线项的当前台账，被多处引用），逐批变动；不在本次候选清单内。 |
| `docs/04-internal/stage123-gap-matrix.md` | 缺口矩阵**历史卷宗**，已被上游审计反复引用作对照面；保留原样。 |
| `docs/04-internal/recon-suite-2026-10.md` | 核对套件**执行记录**（不是审计结论册），与本目录定位不同。 |
| `docs/03-reference/test-baseline.md` | 测试数字**单一真源**，只引用不抄写（硬性要求）。 |
| `docs/status/claims.yaml` / `docs/status/wiring.yaml` | 能力声明**单一真源**（生成物禁手改），只引用不抄写。 |
| `docs/architecture/contract.toml` | 架构契约**唯一可执行真源**，只引用不抄写。 |
| `docs/04-internal/ENGINEER-MANIFESTO.md`、`docs/04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md` | 宪章本体（非审计结论），02/03 册的纪律判据出处。 |
| `docs/adr/**`、`docs/archive/**`、`docs/architecture/**`、`docs/status/**` | 本任务禁改区，只读引用。 |

---

## 四、合并时的取舍规则（供整合线与后续合卷参考）

1. **结论不丢**：每份来源的「结论要点 + 关键表 + 残余清单 + owner 决策位」逐条留痕；
   原文中「唯一性细节」（如某项残余的具体口径、某条扫描命令、某个文件名）宁可多留。
2. **过期不删**：今天的仓内事实与原结论冲突时，写「**已过时：<原因>**」并指出**现在的口径出处**
   （真源文件路径），原文数值保留在「历史口径（YYYY-MM-DD）」语境里。
3. **核不出来写「未核实」**：复核时没有查到证据的项，一律标「未核实」，不臆造。
4. **不搬写数字**：测试数字、crate 数、开关默认值等现行真源数字，一律以引用代替抄写；
   册内出现的数字均带来源日期与口径说明。
5. **中立表述**：第三方产品名不写入本册；研究引注与仓内既有引用按豁免表处理。

---

## 五、本目录的读者路径

- 要判断「某能力今天算不算已接线」→ 01 §1/§6 + 02 §1（抬级取证）+ [claims.yaml](../../status/claims.yaml)。
- 要判断「某个安全/治理结论是否还成立」→ 02 §1–§9。
- 要判断「文档里哪些数字/口径是陈的」→ 03 §1–§4 + [test-baseline.md](../../03-reference/test-baseline.md)。
- 要查「某模块当前接线在哪」→ [cognitive-module-wiring.md](../cognitive-module-wiring.md)（活账本，不在本目录）。

---

## 六、链接自查

本目录内所有相对链接均以 `docs/04-internal/audits/` 为起点，逐条核对目标文件存在（2026-10-23）：

| 链接目标 | 存在 |
|---|---|
| `../../03-reference/test-baseline.md` | ✅ |
| `../../status/claims.yaml` | ✅ |
| `../../architecture/contract.toml` | ✅ |
| `../../01-architecture/system-capabilities.md` | ✅ |
| `../cognitive-module-wiring.md` | ✅ |
| `../backend-enhancement/implemented-to-wired-plan-2026-10-21.md` | ✅ |
| `../backend-enhancement/audit-gap-behavior-fixes-2026-10-16.md` | ✅ |
| `../backend-enhancement/audit-gap-caliber-hygiene-2026-10-16.md` | ✅ |
| `../backend-enhancement/security-hardening-delivery-2026-10-20.md` | ✅ |
| `../backend-enhancement/security-restart-closeout-2026-10-22.md` | ✅ |
| `../backend-enhancement/egress-gate-wiring-design-2026-10-23.md` | ✅ |
| `../backend-enhancement/system-gap-ledger-2026-10-18.md` | ✅ |
| `../governance-chain-drill-2026-10-20.md` | ✅ |
| `../secret-management-policy.md` | ✅ |
| `../../../reports/audit-2026-08-19/active_docs_audit.md` | ✅ |
| `../../../reports/s1-sandbox-hardening-self-audit.md` | ✅ |
| `../../../reports/v2-rc1-backend-security-audit-2026-09-24.md` | ✅ |
| `../../../AGENTS.md` | ✅ |

> 合并原件一律以代码片段登记（见 §二），**不建超链接**：原件迁档后路径会变，链接会断。
