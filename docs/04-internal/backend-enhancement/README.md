# docs/04-internal/backend-enhancement — 后端增强施工台账

> **性质**：L4 在飞 —— 后端增强各**施工轴**的台账、缺口账与交付实录。
> **纪律**：全仓第一引用目标是本目录的 [CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（进场第一课）；**债只记一处**：[00-backlog.md](00-backlog.md) 的 **D 编号是唯一真源**。
> **不要**从本目录推断"系统现在有什么"——那是 [`../../status/claims.yaml`](../../status/claims.yaml) 与 [`../../03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md) 的职责。

---

## 0. 我要什么 → 去哪

| 我要做的事 | 去哪 |
|---|---|
| **进场第一课** | [CONSTRUCTION-DISCIPLINE.md](CONSTRUCTION-DISCIPLINE.md)（多 AI 并发施工宪章） |
| **查待办/债** | [00-backlog.md](00-backlog.md)（想法级索引 + D 编号债栏） |
| **查系统级缺口** | [system-gap-ledger-2026-10-18.md](system-gap-ledger-2026-10-18.md)（SG/NT/NF 条目） |
| **查抬级方案** | [implemented-to-wired-plan-2026-10-21.md](implemented-to-wired-plan-2026-10-21.md)（IMPLEMENTED → PRODUCTION WIRED） |
| **查施工序** | [V-forward-direction.md](00-governance/V-forward-direction.md)（V1–V6 波次） |
| **查门禁/口径策略** | [size-ratchet-policy-2026-10-23.md](00-governance/size-ratchet-policy-2026-10-23.md)｜[audit-gap-caliber-hygiene-2026-10-16.md](audit-gap-caliber-hygiene-2026-10-16.md)｜[audit-gap-behavior-fixes-2026-10-16.md](audit-gap-behavior-fixes-2026-10-16.md) |
| **查某轴的现状与想法** | 下表「施工轴」 |

---

## 1. 施工轴

| 轴 | 主文件 | 附属 / 交付 |
|---|---|---|
| **A 配额** | [A-quota-real-interface.md](02-axes/A-quota-real-interface.md) | — |
| **B 性能** | [B-performance.md](02-axes/B-performance.md) | — |
| **C 记忆** | [C-memory-deepening.md](02-axes/C-memory-deepening.md) | [V4-b-memory-freshness-delivery.md](04-delivery/V4-b-memory-freshness-delivery.md)（保鲜回路交付）｜[quick-capture-delivery.md](quick-capture-delivery.md)（随手记入口，**活契约**） |
| **D 干活** | [D-doing-work.md](02-axes/D-doing-work.md) | [git-write-tiering-delivery.md](git-write-tiering-delivery.md)（**分级词表冻结**）｜[worktree-apply-delivery.md](worktree-apply-delivery.md)｜[V6-a-task-queue-delivery.md](V6-a-task-queue-delivery.md)（六态队列） |
| **E 安全** | [E-security-depth.md](02-axes/E-security-depth.md) | [E-threat-model.md](E-threat-model.md)（**G1–G10 活销账依据**）｜[E-approval-family-contract.md](E-approval-family-contract.md)（**族词表冻结**）｜[E-object-level-contract.md](02-axes/E-object-level-contract.md) |
| **F 服务端形态** | [F-server-form.md](02-axes/F-server-form.md) | [W2-2-remote-access-auth-delivery.md](W2-2-remote-access-auth-delivery.md)｜[W2-3-multi-client-session-reuse.md](04-delivery/W2-3-multi-client-session-reuse.md)｜[W3-1-multi-user-data-isolation.md](04-delivery/W3-1-multi-user-data-isolation.md)｜[W3-2-user-identity-roles.md](04-delivery/W3-2-user-identity-roles.md)｜[W3-3-domain-partner-instances.md](04-delivery/W3-3-domain-partner-instances.md)｜[V2-c-domain-faces-progressive-open.md](02-axes/V2-c-domain-faces-progressive-open.md) |
| **G 观测与可靠** | [G-observability.md](G-observability.md) | [G5-backup-restore-delivery.md](G5-backup-restore-delivery.md)（**备份格式单一真源**） |
| **H 模型面** | [H-model-plane.md](02-axes/H-model-plane.md) | — |
| **I 星港** | [I-starport-builtins.md](02-axes/I-starport-builtins.md) | — |
| **J 生活接入** | [J-life-integration.md](02-axes/J-life-integration.md) | [J-tradewind.md](02-axes/J-tradewind.md) |
| **K 在场·星槎** | [K-life-presence.md](K-life-presence.md) | [K-device-control-three-beat.md](02-axes/K-device-control-three-beat.md)｜[K-guard-watch-flow.md](02-axes/K-guard-watch-flow.md)｜[K-device-approval-im-push.md](K-device-approval-im-push.md)｜[K-starraft.md](02-axes/K-starraft.md) |
| **L 经验沉淀** | [L-experience-distillation.md](L-experience-distillation.md) | 本轴已封账；提炼产物在 [`../engineering-experience/`](../engineering-experience/) |
| **P 吸收自动化** | [I-absorption-automation.md](02-axes/I-absorption-automation.md) | [I1-skill-distill-delivery.md](I1-skill-distill-delivery.md)（技能卡产物形态） |

> **命名说明**：`I-absorption-automation.md` 属 **P 轴**（文件名沿用旧 `I-` 是防断链妥协，**不要改文件名**）。

---

## 1.5 目录结构（2026-10-23 文档治理批重组）

```
backend-enhancement/
├── README.md        本文件（台账地图）
├── 00-governance/   规则与序：施工纪律 / 债栏 / 波次 / 棘轮与口径策略
├── 01-ledgers/     跨轴台账（系统级缺口账 / 抬级取证）
├── 02-axes/        每轴唯一主文件（A/B/C/D/E/F/H/I/J/K …）+ 轴内契约
├── 03-designs/     尚未施工的设计稿
├── 04-delivery/    已做完的交付实录（证据册，不是规格）
└── （根目录）      宪章与真源 + 被源码/门禁写死路径的文件（见 §4 纪律第 5 条）
```

**重组原则**：**被源码、门禁或机器解析文件（`claims.yaml` / `wiring.yaml` / `capabilities-matrix.md`）按全仓路径写死的文件一律留原位**——移走会让机器引用指向一个桩。已移位 33 篇，原路径留 4 行重定向桩（登记于 [`../../_redirects.tsv`](../../_redirects.tsv)），既有链接不断。

---

## 2. 设计稿（尚未施工）

| 文件 | 主题 |
|---|---|
| [egress-gate-wiring-design-2026-10-23.md](egress-gate-wiring-design-2026-10-23.md) | 模型出网闸接线设计 |
| [accountability-loop-design-2026-10-23.md](03-designs/accountability-loop-design-2026-10-23.md) | 问责闭环（影响声明 / 决策回执 / 后果追踪 / 通知义务） |
| [lesson-object-design-2026-10-22.md](lesson-object-design-2026-10-22.md) | 教训对象（Lesson）设计 |
| [b6-2a-sealed-session-design-2026-10-22.md](b6-2a-sealed-session-design-2026-10-22.md) | `sessions.data` 列级封缄接入 |
| [SG-25-tool-capability-taxonomy-proposal-2026-10-20.md](SG-25-tool-capability-taxonomy-proposal-2026-10-20.md) | 工具能力 taxonomy 提案 |
| [im-channel-module-2026-10-23.md](03-designs/im-channel-module-2026-10-23.md) | IM 社交通道模块（未来专项） |

## 3. 交付实录（做完的证据，不是规格）

| 文件 | 主题 |
|---|---|
| [security-hardening-delivery-2026-10-20.md](security-hardening-delivery-2026-10-20.md) | 安全加固线（B1–B8 外部审计） |
| [security-restart-closeout-2026-10-22.md](security-restart-closeout-2026-10-22.md) | 安全线重启批收口 |
| [B8-release-signing-2026-10-19.md](B8-release-signing-2026-10-19.md) | 发布签名与来源证明（被 `claims.yaml` 引用） |
| [V5-a-redaction-bridge-delivery.md](04-delivery/V5-a-redaction-bridge-delivery.md) | 出境脱敏桥交付 |
| [V5-c-patrol-funnel-delivery.md](04-delivery/V5-c-patrol-funnel-delivery.md) | 守望巡检漏斗 |
| [V3a-dual-engine-care.md](V3a-dual-engine-care.md) | 双引擎主动关怀 |
| [care-heartbeat-producer.md](04-delivery/care-heartbeat-producer.md) | 常驻心跳关怀生产者（被 `env-registry.toml` 引用） |
| [trait-lock-and-trait-report-delivery.md](04-delivery/trait-lock-and-trait-report-delivery.md) | 性格锁定 + 性格周报 |
| [production-wiring-closeout-delivery.md](04-delivery/production-wiring-closeout-delivery.md) | 生产接线收口批 |
| [image-ops-core-swap-delivery.md](image-ops-core-swap-delivery.md) | 图像操作换创作内核 |
| [zero-debt-opening-clearance-2026-10-12.md](04-delivery/zero-debt-opening-clearance-2026-10-12.md) | 零债开场清偿批 |
| [f-02-asset-outbound-execution-2026-10-18.md](04-delivery/f-02-asset-outbound-execution-2026-10-18.md) | 大资产出库执行批 |
| `f-02-migration-manifest-2026-10-18.tsv` | 迁移清单附件（被 `scripts/guard_ml/README.md` 引用） |

> **交付记录的读法**：里面的数字是**当批实测**（历史口径），**现行数字只认**
> [`../../03-reference/test-baseline.md`](../../03-reference/test-baseline.md) 与
> [`../../03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md)。

---

## 4. 纪律

1. **债只记一处**：本目录所有"待办"最终汇到 [00-backlog.md](00-backlog.md) 的 **D 编号**；别处不得另起编号体系。
2. **口径不复制**：能力状态查 `../../status/claims.yaml`，默认开关查 `../../03-reference/capabilities-matrix.md`。
3. **0 装诚实**：轴文件里的"已交付"必须标四级口径；`unwired-ledger` 未销账的项不得写成生产现状。
4. **交付记录不改写**：只追加，不回改历史批次。
5. **文件被源码写死的不许改路径**：`I-starport-builtins.md`（26 个前端文件引用）、`quick-capture-delivery.md`（前端源码引用）、`G-observability.md`（Rust 源码注释引用）、`care-heartbeat-producer.md`（`env-registry.toml` 引用）、`system-gap-ledger`（4 个 Rust 文件引用）、`B8-release-signing`（`claims.yaml` 引用）——**要动它们先改引用方**。
