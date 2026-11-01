# backend-enhancement —— 后端增强工作区

> **这一层是 L4 在飞 + L5 卷宗**：按轴组织的设计、交付与审计材料。读者是施工的人。
> 能力现状以 [`docs/status/claims.yaml`](../../status/claims.yaml) 为准；开关现值见
> [`docs/03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md) §0.2。

## 我要什么 → 去哪

| 我想知道 | 去哪 |
|---|---|
| 规矩、纪律、接线计划 | [`00-governance/`](00-governance/README.md) |
| 当时哪里有缺口、欠了什么 | [`01-audits/`](01-audits/README.md) |
| 某个轴怎么设计的 | [`02-axes/`](02-axes/README.md) |
| 专题设计（会话封存 / 出网闸 / 教训对象…） | [`03-designs/`](03-designs/README.md) |
| 某批交了什么、证据在哪 | [`04-delivery/`](04-delivery/README.md) |

## 台账地图

| 目录 | 是什么 | 正文篇数 |
|---|---|---|
| [`00-governance/`](00-governance/README.md) | 轴内治理：宪章、纪律、「库级实现 → 接生产装配」计划、尺寸棘轮 | 3 |
| [`01-audits/`](01-audits/README.md) | 审计缺口与系统缺口台账 —— 当时哪里不合口径、欠了什么。 | 2 |
| [`02-axes/`](02-axes/README.md) | 按轴组织的设计稿：每轴一份，讲该轴的接口、边界与交付面。 | 20 |
| [`03-designs/`](03-designs/README.md) | 专题设计稿 —— 打算怎么做、边界在哪。 | 2 |
| [`04-delivery/`](04-delivery/README.md) | 交付册与交付实录 —— 交了什么、证据在哪。 | 24 |

### 顶层留位（11 篇正文 + 49 个桩）

> **移位不减少文件数**：低层共 {stub} 个 4 行重定向桩；顶层 {len(real)} 篇正文因**被> `crates/` 源码注释 / `claims.yaml` / workflow 直接引用**而留原位（移走会让脚本读到桩，静默改行为）。

| 篇 | 为什么留原位 |
|---|---|
| [`00-backlog.md`](00-backlog.md) | **持仓台账**：`crates/` 与治理演练测试引用，多处交叉 |
| [`B8-release-signing-2026-10-19.md`](B8-release-signing-2026-10-19.md) | `claims.yaml` / `wiring.yaml` 证据引用 |
| [`CONSTRUCTION-DISCIPLINE.md`](CONSTRUCTION-DISCIPLINE.md) | **施工纪律宪章**：`AGENTS.md` 指向，多处引用 |
| [`G-observability.md`](G-observability.md) | `crates/adapters/gateway/src/panel_metrics.rs` 等源码注释引用 |
| [`I1-skill-distill-delivery.md`](I1-skill-distill-delivery.md) | `crates/capabilities/tools/src/skill/mod.rs` 引用 |
| [`K-life-presence.md`](K-life-presence.md) | `crates/capabilities/tools/src/device/mod.rs` 引用 |
| [`SG-25-tool-capability-taxonomy-proposal-2026-10-20.md`](SG-25-tool-capability-taxonomy-proposal-2026-10-20.md) | `crates/.../tool_taxonomy.rs` 引用 |
| [`b6-2a-sealed-session-design-2026-10-22.md`](b6-2a-sealed-session-design-2026-10-22.md) | `crates/.../session_seal.rs` 引用 |
| [`egress-gate-wiring-design-2026-10-23.md`](egress-gate-wiring-design-2026-10-23.md) | `crates/.../egress_gate.rs` 引用 |
| [`lesson-object-design-2026-10-22.md`](lesson-object-design-2026-10-22.md) | `crates/.../lesson.rs` / `lessons_command.rs` 引用 |
| [`system-gap-ledger-2026-10-18.md`](system-gap-ledger-2026-10-18.md) | `crates/foundation/orchestration/src/` 引用 |

## 纪律

- **只增不改**：带日期的历史记录保留原值，需要时加「历史口径（日期）」标注。
- **受保护资产**：`docs/adr/` 正文、契约与真源不由本目录流程改写。
- **四级口径**：声称「已实现 / 已接线」必须标级别（IMPLEMENTED / PRODUCTION WIRED / HARDWARE VALIDATED），
  `scripts/check_doc_caliber.py` 会拦。

---

**变更历史**：
- 2026-10-23 首版：60 篇平铺 → 5 类子目录 + 27 篇留原位。
- 2026-10-24 二轮收拢：顶层真文件 **27 → 13**，新增 `01-audits/`；轴稿/设计稿/交付册各归其位，
  留原位 13 篇全部因**机器引用**（逐条注明出处）。
