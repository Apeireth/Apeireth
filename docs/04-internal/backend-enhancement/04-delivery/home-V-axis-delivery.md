# V 轴交付册（V4-b / V5-a / V5-c，2026-10-17 批）

> **本册合订 3 篇 V 轴交付实录**。读法：先看 §0 判断是否是要找的那件；要看细节，点**归属**进原件。
> **现状以 [`docs/status/claims.yaml`](../../../status/claims.yaml) 为准**；本册只记「当时交付了什么、证据在哪」。
> 原件逐字归档在 `docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/`，
> 每条引文标〔原篇摘录〕，**不改原值**。

## 0. 三篇速览

| 篇 | 一句话 | 当时校验（原篇实测） |
|---|---|---|
| V4-b-memory-freshness | 记忆库运行成本从「随摄入次数增长」压到「随变更量增长」（散列增量门 + 可恢复摄取队列 + 增量折入） | 215 套件 / 5774 passed / 0 failed / 20 ignored |
| V5-a-redaction-bridge | 出境脱敏**双向闸**：占位格式规格 + 双闸时序 + 历史回扫 + 变体绕过防御面 + 脱敏账本 | 本批新增测试 24 项（要求 ≥12） |
| V5-c-patrol-funnel | 六项速扫装进四段漏斗，三件硬机制（成本闸 / 断点续查 / 任务化）把体检升级为**可周期重跑的守望巡检** | 221 套件 / 5839 passed / 0 failed / 20 ignored |

## 1. V4-b 记忆保鲜回路（散列增量门 + 可恢复摄取队列 + 增量折入）

**一句话**〔原篇摘录〕：

> 记忆库的运行成本从「随摄入次数增长」压到「随变更量增长」：**散列增量门**（同源同散列跳过重复摄取/整编，省算力防重复记忆）+ **可恢复摄取队列**（待摄取清单落库、重启续摄、失败留队列带尝试计数）+ **增量折入**（同主题追加不重写，机器钉死 append-only）+ `tool.self_status` 记忆节三真值（待摄取数 / 最近折入 / 去重计数，缺源显式 null）。

**测试证据**〔原篇摘录，节选〕：

> **三件套核心（`crates/engine/memory`，22 个）**——`src/freshness/hash_gate.rs` 8 个（FNV-1a 标准向量锚、散列稳定/随内容变、同内容跳过、变内容重摄、异源不去重、去重计数累计、只查门不污染账、空参拒绝即帧）；`src/freshness/fold.rs` 4 个（**追加不重写**断言旧段字节原样、UPDATE/DELETE 触发器拒绝、按主题独立编号、空参拒绝）；`src/freshness/ingest_queue.rs` 5 个（入队幂等单行、已摄取跳过+去重计数、变内容新行、**失败留队列+尝试计数**、空队 Idle）；`tests/freshness_loop.rs` 5 个（管线端到端、**队列重启续摄**（文件库重开续摄）、**失败留队列跨重启**（尝试计数/原因落库）、**增量折入跨重启追加不重写**、真值口径（未折入 = 显式 null 不编时间））。
> **整编面（`crates/engine/runtime-assembly`，4 个）**——`cognitive.rs`：同源同散列跳过整轮整编（脚本门）、门故障 fail-open 照常整编、成功后恰一笔摄入记账、真散列门端到端（同素材第二轮跳过 + 去重计数 +1 + 不产重复记忆；变素材重摄 + 去重计数不涨）。

**校验真账**（2026-10-17 实测）〔原篇摘录〕：

> | 项 | 命令 | 结果 |
> |---|---|---|
> | 测试 | `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | ✅ **215 套件 / 5774 passed / 0 failed / 20 ignored** |
> | Clippy | `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ✅ 0 警告 |
> | 格式 | `cargo fmt --check` | ✅ 0 diff |

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/V4-b-memory-freshness-delivery.md`

## 2. V5-a 出境脱敏双向闸（占位规格 + 双闸时序 + 历史回扫 + 账本）

**一句话**〔原篇摘录〕：

> Author:          子代理 V5-a 施工线
> > 战场: `crates/capabilities/tools/src/redaction_bridge.rs`(新件) +

**校验真账**（2026-10-17 实测）〔原篇摘录，节选〕：

> > 测试数字单一真源是 [test-baseline.md](../../../03-reference/test-baseline.md), 本页
> > 只记「本批实测值 + 日期」供对账, 不代改真源。口径:
> > `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` /
> > `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` /
> > `cargo fmt --all -- --check` / `scripts/check-neutral-terms.ps1`。

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/V5-a-redaction-bridge-delivery.md`

## 3. V5-c 守望巡检漏斗（筛查→精查→复核→报告）

**一句话**〔原篇摘录〕：

> V5-b 暴露面体检回答「暴露了什么」，本层回答「**巡检一遍要花多少、可信度多高、比上次好了还是坏了**」：
> 六项速扫装进四段漏斗，三件硬机制（成本闸 / 断点续查 / 任务化）把体检升级为**可周期重跑的守望巡检**。

**测试证据**〔原篇摘录，节选〕：

> 集成面 `crates/capabilities/tools/tests/exposure_funnel.rs`（**13 例**）+ 模块单测（**9 例**）：
> | # | 场景（任务硬指标） | 测试 | 钉住的行为 |

**校验真账**（2026-10-17 实测）〔原篇摘录，节选〕：

> - 测试：`cargo test --workspace --all-targets --locked --no-fail-fast -j 2` = **221 套件 / 5839 passed /
>   0 failed / 20 ignored**（2026-10-17 全工作区合流实测，详 §8）；本批新增 27 例（集成 13 + 单测 14）全绿。
> - clippy：`cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` = **0 警告（exit 0）**
>   （2026-10-17 实测；工具 crate 单独复跑亦 0 警告）。
> - fmt：`cargo fmt --check` 全仓 0 diff（2026-10-17 实测；本批文件经 rustfmt 单文件收口）。

**归属**：`docs/archive/merged/2026-10-24/docs/04-internal/backend-enhancement/04-delivery/V5-c-patrol-funnel-delivery.md`

## 4. 现状复核（2026-10-24 文档治理批）

| 项 | 现在 |
|---|---|
| V5-a 双向闸对应 claim | `sec.egress_gate` = **PRODUCTION WIRED** / default **OFF**（四级口径取其 claim，不取本册引文） |
| 全套件数 | 以 [`docs/03-reference/test-baseline.md`](../../../03-reference/test-baseline.md) 为**单一真源**；本册只记各篇当时实测值 |
| 历史值 | 上表校验数字是 **2026-10-17 当时值**，按「历史不追溯改写」保留原样 |

## 5. 读这一册的边界

- 本册**不是**现状真源；`docs/status/claims.yaml` 与 `test-baseline.md` 才是。
- V4-b 的 self_status 记忆节、V5-a 的脱敏账、V5-c 的巡检账**各成一本账**，互不替代。
- 要看设计取舍的点（为什么这么切），回原件；本册只做「按轴收拢 + 速览 + 归属」。

---

**变更历史**：2026-10-24 首版，3 篇合订（原件逐字归档 + 原路径留桩）。
