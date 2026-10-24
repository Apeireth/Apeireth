# 装配收口与裁定落地册（生产接线收口 + 性格锁定/周报）

> **本册合订 2 篇交付实录**。读法：先看 §0 速览判断是不是你要找的那件；要看细节，点**归属**进原件。
> **现状以 [`docs/status/claims.yaml`](../../../status/claims.yaml) 与 [`test-baseline.md`](../../../03-reference/test-baseline.md) 为准**；
> 本册只记「当时交付了什么、证据在哪」。原件逐字归档在 `docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery`，每条引文标〔原篇摘录〕，**不改原值**。

## 0. 一句话

两条「把库级能力接进生产装配」的收口批：四缝收口（旋钮/情绪源/暴露面）+ 裁定 ④⑤ 落地（性格锁定 + 性格周报）

| 篇 | 归属 | 当时状态 |
|---|---|---|
| 生产接线收口批（四缝收口） | `production-wiring-closeout-delivery.md` | 已交付（原篇自述，见归属） |
| 性格锁定 + 性格周报（裁定 ④⑤ 落地批 R2） | `trait-lock-and-trait-report-delivery.md` | 已交付（原篇自述，见归属） |

## 1. 生产接线收口批（四缝收口）

**一句话**〔原篇摘录〕：

> 把近几波「已实现未接线」的四个缝**收成真能用的生产接线**（第一律：能力必须真能用；四级口径：本批收口后四缝均为 PRODUCTION WIRED，旋钮默认关项维持 DEFAULT OFF 不写成现状）：两个整编族能力补上 CLI env 旋钮（注册即闸、随整编跑）、发声微调接上**真情绪源**（读 presence/PAD 合成器现值，情绪→语气微调生效、无情绪数据零变化）、暴露面体检补上**工具面名册接线**（`tool.exposure_audit` 注册即闸、只读档）。每缝配齐：旋钮 fail-closed 测试 + 名册行 + self_status 相应行 + 能力矩阵 §0.2 行 + user-manual 旋钮表行。


**测试证据**〔原篇摘录，节选〕：

> **缝 1/2（`crates/adapters/cli/tests/consolidation_knobs.rs`，5 个）**——
> 1. `persona_consolidation_knob_fails_closed_and_wires_with_the_consolidation_trigger`：默认关 = 名册照实 false（显式未接线）；开 + 整编开 = 名册真值 true（接线生效真值）；

**校验真账**（原篇实测）〔原篇摘录，节选〕：

> | 项 | 命令 | 结果 |
> |---|---|---|
> | 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ **221 套件 / 5839 passed / 0 failed / 20 ignored**（exit 0） |
> | Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告（exit 0） |

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/production-wiring-closeout-delivery.md`


## 2. 性格锁定 + 性格周报（裁定 ④⑤ 落地批 R2）

**一句话**〔原篇摘录〕：

> ---


**测试证据**〔原篇摘录，节选〕：

> | # | 证据 | 位置 |
> |---|---|---|

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/trait-lock-and-trait-report-delivery.md`

## 现状复核（2026-10-24 文档治理批）

| 项 | 现在 |
|---|---|
| 全套件数 | 以 [`docs/03-reference/test-baseline.md`](../../../03-reference/test-baseline.md) 为**单一真源**；本册只记各篇当时实测值 |
| 历史值 | 上文校验数字是各篇**当时实测值**，按「历史不追溯改写」保留原样 |

## 读这一册的边界

- 本册**不是**现状真源；要看某能力现在到哪一级，查 `claims.yaml` 与能力矩阵。
- 各篇的账（会话账 / 域账 / 角色矩阵 / 实例账）**各成一本**，互不替代。
- 要看设计取舍（为什么这么切），回原件；本册只做「按轴收拢 + 速览 + 归属」。

---

**变更历史**：2026-10-24 首版，2 篇合订（原件逐字归档 + 原路径留桩）。
