# A. 配额真接口

> `Status: 📋 想法细节（隶属 00-backlog #A）`

## 现状
- 四维配额（token/步数/花费/深度）只存在于 `foundation/orchestration` 的配额类型（`CognitiveQuota`），仅单测构造，**无 env/config 接线、生产无调用点**；
- 设置页「预算与配额」如实标注"暂无接口"（零假旋钮纪律）；
- 回合预算（轮数/单轮工具/上下文字符）已可配且即效——**同族先例**。

## 想法
把四维配额做成**真执行的烧钱闸**：
1. 配额配置面（env + 设置页旋钮，沿回合预算同一条即效缝）；
2. 调度器消费配额：会话累计 token/步数/花费，超限 → 预算耗尽行为（熔断/询问二选一，接生产 `BudgetExhausted`）；
3. self_status 预算节扩四维（真值/上限/余量）；驾驶舱 CACHE/BUDGET 面板与消耗卡同源点亮。

## 缺口
- 花费计量：单价表（每模型）→ token×单价；无单价表则"花费"维暂无接口（诚实纪律）；
- 深度维：调用深度跟踪未实施（旧字段已删），需重新设计（工具调用树深度）。

## 分期
P1 token/步数两维（计量已有）→ P2 花费（单价表）→ P3 深度（调用树）。

## 验收
四维各有：配置旋钮+真实计量+超限行为+自报真值+测试钉死；未实现维度继续如实标注。

## P1 实施记录（token/步数两维·后端执行面）

> 本节为叠加记录（原有各节不动）。前端旋钮另批。

**新符号**
- `apeireth-orchestration`：`CognitiveQuota` 增量生产接口 `Default`/`unlimited`/`consume_tokens`/`consume_tool_step`（saturating 记账，生产账本即此类型，随会话持久化）；
- `apeireth-runtime::canonical::quota`：`QuotaDimension`（`tokens`/`tool_steps`）、`SessionQuotaLimits`（`None`=不设限，opt-in）、`QuotaExceeded`（维度名/上限/已耗）、`QuotaExhaustedAction`（`CircuitBreak`=已接线；`AskForBudgetExtension`=询问档接口桩，显式未接线）、`CostRateMicros`+`CostRateTable`（花费单价表骨架，显式未接线）、`parse_quota_max_tokens`/`parse_quota_max_tool_steps`（正整数直通、越界钳制 `MAX_QUOTA_TOKENS`/`MAX_QUOTA_TOOL_STEPS`、0/非法回 `None`=不设限）；
- `RuntimeError::BudgetExhausted { dimension, limit, consumed }`（预算耗尽错误帧）；
- `apeireth-tools-canonical`：`QuotaStatus`/`QuotaDimensionStatus`/`QuotaMeterReading`/`SessionQuotaProbe` + `QUOTA_COST_NOT_WIRED`/`QUOTA_DEPTH_NOT_WIRED`/`QUOTA_METER_SCOPE_REQUIRED`；
- `apeireth-runtime-assembly::session_quota_probe`（会话账本只读探测口）；
- `ToolCapability::invoke_in_session` / `SelfStatusSource::snapshot_in_session`（会话作用域透传，默认实现零行为变化）。

**配置→执行→自述三面接线**
1. 配置面：env `APEIRETH_QUOTA_MAX_TOKENS`/`APEIRETH_QUOTA_MAX_TOOL_STEPS` → CLI `quota_max_*_from_env`（与回合预算同一取值纪律，但默认=None 不设限）→ `RuntimeConfig.quota: SessionQuotaLimits`（`with_session_quota`）；
2. 执行面：会话账本 `Session.quota: CognitiveQuota`（token=回合计量 usage 累加、步数=派发工具调用计数；计量点即落库点）；检查点=回合起手/工具调用点/回合收束（并行窗口准入同口径），`consumed >= limit` 即 `BudgetExhausted` 帧收束（熔断档；未答调用补合成结果，前文保留）；
3. 自述面：`budget.quota` 四行——token/步数两维真值（上限/已耗/余量，不设限显示 "—"），花费/深度两维如实"暂无接口"；已耗按会话作用域现探测（无作用域显式 null+原因）。

**测试证据**：`crates/engine/runtime/tests/session_quota.rs`（累计/超限即帧/维度名/回合中止语义/烧钱闸不起烧）、`canonical/quota.rs` 单测（env 解析四态/不设限默认/口径唯一/询问档与单价表骨架显式未接线）、`self_status.rs` 与 `self_status_source.rs` 单测（两维真值与"—"/暂无接口标注/作用域缺失显式 null）、`production_knobs.rs`（env→RuntimeConfig 同源）。

## H-P1 接头记录（单价表来源就绪）

> 叠加记录（原有各节不动）。H 线 P1（模型画像表）交出**单价表来源**：
> `impl CostRateTable for ModelProfileTable`（`apeireth-runtime::canonical::quota`，画像行有输入+输出双价才给价，缺一半/无价/未知回 `None`——绝不编价）
> + `CostRateMicros::cost_micros`（每侧向上取整、饱和——计量点接线时的冻结换算口径）。
> 本文件 P1 记录「`CostRateMicros`+`CostRateTable`（花费单价表骨架）」一句自该批起读作"单价表来源已实现（画像表）"。
> **花费维计量与超限行为仍属 P2**（显式未接线钉死：上限恒不可达、花费列无计量点、自述沿用"暂无接口"）；P2 接线路径 = 计量点按 `usage × rate` 记 `consumed_cost_micros`，`check` 增 `cost_micros` 维走超限即帧同款。详见 [H-model-plane.md](H-model-plane.md) P1 实施记录。
