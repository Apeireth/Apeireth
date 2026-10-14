# G. 观测与可靠

> `Status: 📋 想法细节（隶属 00-backlog #G）`
> `P1 数据面: ✅ 已交付（2026-10-12, G-P1 后端批, 见文末交付记录）`

## 现状
- 已有：审计事件流、guard/trace 落盘、self_status 自述、cache_trace 钩子、驾驶舱/对话遥测（部分"—"待数据源）、错误即帧；
- 缺：统一指标面（延迟分布/缓存效率/工具成功率）、崩溃恢复语义、数据备份。

## 想法
1. **指标数据源**：回合延迟分布、token 流向、缓存命中率的持久指标（喂驾驶舱 CACHE/BUDGET 面板与设置仪表——把"—"点亮）；
2. **tracing 面**：请求级追踪（trace_id 已有贯通面）落成可查时间线（内部过程频道的数据源）；
3. **崩溃恢复**：会话/记忆的恢复语义测试（掉电模拟）、WAL/检查点策略审阅；
4. **备份**：数据目录快照/导出（导出面已有雏形）、恢复演练。

## 缺口
- 指标留存策略（滚动窗口/磁盘预算）——别让观测自己变成负担。（P1 已定案：进程内滚动环形，见文末「留存策略」）
- 指标缓存命中列暂无数据源（usage 计量链缺缓存列）——见文末「缺口登记」。

## 分期
P1 指标数据源（点亮现有 UI 槽位）→ P2 tracing 时间线 → P3 崩溃恢复 → P4 备份。

## 验收
面板"—"归零（每格有真数据源）、掉电模拟测试过、备份恢复演练记录在案。

---

# P1 指标数据源 · 后端数据面交付（2026-10-12, G-P1 后端批）

> 本批只供数据：指标模型 + 回合收束埋点 + `GET /v1/panel/metrics` 端点。
> 驾驶舱 CACHE/BUDGET 面板与聊天头遥测的**前端消费是后续批**；本批不触碰前端。
> 代码面：`crates/engine/runtime/src/canonical/metrics.rs`（新, 模型+聚合+注册表）、
> `crates/engine/runtime/src/canonical/execute.rs`（仅两处收束埋点, 每处一行调用）、
> `crates/adapters/gateway/src/panel_metrics.rs`（新, 指标端点路由文件, 过 W2-② 令牌门）。

## 指标字段表

**一收束一拍 [`TurnMetrics`]**（`metrics.rs`）：

| 字段 | 类型 | 取数口径 | 空态（—） |
|---|---|---|---|
| `outcome` | `succeeded \| failed \| pending_approval` | 收束分类, 与收束事件 (`RuntimeEvent`) 同口径 | 恒有 |
| `latency_ms` | `u64` | 收束段执行毫秒（计时点在会话锁内、不含会话排队与人工审批等待） | 恒有 |
| `input_tokens` | `Option<u64>` | 收束处 usage 计量 `prompt` 列（**turn-telemetry 同源**, 不另立计量） | 失败/待批收束无计量 → `null` |
| `output_tokens` | `Option<u64>` | usage `completion` 列（同源） | 同上 |
| `cache_hit_tokens` | `Option<u64>` | usage 计量的缓存列（`NormalizedUsage::prompt_cache_hit_tokens`，缺口①已接） | provider 未报缓存列 → `null`（未知不冒充 0） |
| `cache_hit_rate` | `Option<f64>` | Σ命中/Σ输入（0.0–1.0; 上屏百分比由消费端换算） | 命中未知或输入为 0 → `null` |
| `tool_calls` | `Option<u32>` | 收束记录轨迹 (`ExecutionTrace`) 内的工具派发计数 | 失败/待批收束 → `null`；审批续跑拍只带续跑段轨迹（缺口登记④） |
| `rounds` | `Option<u32>` | 收束轮数（成功 = `TurnResponse.rounds`; 待批 = `rounds_used`） | 失败收束轮数不可知 → `null` |
| `cost_micros` | `Option<u64>`（可选列） | 计价换算点唯一归花费线回合计量（`quota`), 指标面不二次换算 | 本批恒 `null`（缺口登记②） |

**窗口聚合 [`WindowAggregate`]**（会话 / 全局两层同形）：

| 字段 | 类型 | 口径 |
|---|---|---|
| `turns` / `succeeded` / `failed` / `pending_approval` | `u64` | 窗口内收束拍计数（`turns` = 三类之和） |
| `latency_ms` / `input_tokens` / `output_tokens` / `rounds` / `tool_calls` | `{count, mean, p50, p95, sum}` | 均值 / 最近秩 P50 / 最近秩 P95 / 计数 / 累计和；**只统计有值拍**（`count` = 有值拍数） |
| `cache_hit_tokens` / `cache_hit_rate` / `cost_micros` | `Option` | 同拍字段口径; 窗口内无计量 = `null` |

分位口径：**最近秩**（排序升序取 `ceil(p × n)` 位, 1-based）。

## 端点合同 `GET /v1/panel/metrics`

- 门：W2-② 全 API 面令牌门内（整面中间件; 缺 Bearer → 401 `auth_missing_key`, 伪造 → 401 `auth_invalid_key`）。
- 响应（`schema_version: 1`, 稳定合同）：

```json
{
  "schema_version": 1,
  "window_turns": 50,
  "retention": "in_process_rolling",
  "global": {
    "turns": 3, "succeeded": 2, "failed": 1, "pending_approval": 0,
    "latency_ms":  { "count": 3, "mean": 12.0, "p50": 10, "p95": 30, "sum": 36 },
    "input_tokens":{ "count": 2, "mean": 100.0, "p50": 100, "p95": 120, "sum": 200 },
    "output_tokens": { "count": 2, "mean": 20.0, "p50": 20, "p95": 24, "sum": 40 },
    "rounds":      { "count": 3, "mean": 1.0, "p50": 1, "p95": 2, "sum": 3 },
    "tool_calls":  { "count": 2, "mean": 1.0, "p50": 1, "p95": 2, "sum": 2 },
    "cache_hit_tokens": null, "cache_hit_rate": null, "cost_micros": null
  },
  "sessions": [ { "session_id": "…", "aggregate": { …同 global 形… } } ]
}
```

- **空态 = 显式空 + "—" 语义（与遥测条同款）**：无数据时 `sessions: []`、计数 `0`、
  分布 `count: 0` 且 `mean/p50/p95/sum` 为 `null`、缓存/计价列为 `null`。
  消费端把 `null` 渲染 "—"; **未知不用 0 冒充**。

## 留存策略（如实：进程内滚动）

- **内存环形, 不落盘**：每层一个环形缓冲, 窗口 = 最近 N 拍;
  N = `RuntimeConfig::metrics_window_turns`（**默认 50**, builder `with_metrics_window`, 钳制 `1..=4096`）。
- **会话环驻留上限** `MAX_TRACKED_SESSIONS = 256`, 超限驱逐最久未记录的会话环;
  总内存上界 = `窗口 × (1 + 256)` 拍, 观测面不会自己变成负担。
- 进程退出即清空; 端点 `retention` 恒报 `"in_process_rolling"`（不假装持久化）。

## 采集埋点（execute.rs 最小面）

- **一收束一拍**：`execute_outcome_with_sink`（回合收束）与 `resolve_approval`
  （审批续跑再收束）各记一拍, 成功/失败/待批分别计数 —— 与 `RuntimeEvent`
  收束事件同口径（同一回合可有多个收束拍: 待批拍 + 续跑成功拍）。
- 埋点 = 计时起点 + 一行 `record_*` 调用, **不动控制流**; `include_str` 守卫
  钉住"两个收束点各一行"的最小面（防观测线在共享面越铺越大）。

## 测试证据（21 项新测试; 复跑命令如下）

```powershell
cargo test -p apeireth-runtime --locked -j 2 --lib metrics          # 12: 分位/均值·三分计·两层·窗口·空态·未知列·并发原子·留存驱逐·口径同源
cargo test -p apeireth-runtime --locked -j 2 --test canonical_metrics # 4: 真回合埋点采集（成功/失败/待批+续跑）·埋点最小面守卫
cargo test -p apeireth-gateway  --locked -j 2 --lib panel_metrics     # 2: 端点空态"—"·两层聚合上端点
cargo test -p apeireth-gateway  --locked -j 2 --test panel_metrics_contract # 3: JSON 合同（含空态）·令牌门 401 同面
cargo test --workspace --locked -j 2   # 既有套件零回归（含 execute 既有测试）
```

覆盖对照（任务要求 ≥8）：埋点采集三分计 ✓ / 窗口聚合 P50·P95·均值 ✓ / 会话+全局两层 ✓ /
端点 JSON 合同含空态"—" ✓ / 令牌门无令牌 401 ✓ / 并发读安全多线程聚合原子性 ✓ /
埋点零侵入（execute 最小面守卫 + 既有测试零回归）✓ / 既有套件零回归 ✓（见复跑输出）。

## 缺口登记（不假装）

1. **CACHE 格（缓存命中）暂无数据源** → **已点亮（2026-10-13 总装批）**：`NormalizedUsage`
   增缓存列 `prompt_cache_hit_tokens`（`#[serde(default)]` 落库兼容），三家 provider 归一化处
   按原始 usage 真实字段填充（openai 兼容面 `prompt_tokens_details.cached_tokens` /
   `prompt_cache_hit_tokens`, anthropic 面 `cache_read_input_tokens`; 找不到缓存列的家
   留 `None` 不编造）；`metrics.rs` 的 `TurnMetrics::succeeded` 取数口随之点亮（真值,
   未报 = "—"）。口径注（如实）：
   各家缓存列与 `prompt_tokens` 的包含关系不同（兼容面 cached ⊆ prompt；`cache_read_input_tokens`
   记在 `input_tokens` 之外），命中率沿用冻结公式 Σ命中/Σ输入 不换口径不裁剪。
2. **cost 列（可选列）恒空**：usage×单价的换算点唯一归花费线回合计量（`quota`,
   A-P2 冻结口径）, 指标面不做第二次换算以免口径分裂; 由花费线把 per-turn 成本
   喂进 `record` 入口点亮（接线缝已留）。
3. **能力清单条目**：`/v1/apeireth/capabilities` 的 `metrics.read` 条目在
   `panels.rs`（共享面, 本批不动）, 随前端消费批接入; 端点本身已过令牌门且合同在案。
   → **已收口（2026-10-13 总装批）**：`panels.rs` 能力清单新增 `metrics` 组
   `metrics.read` 条目（只读面, 恒可用, 端点已过令牌门）。
4. **tool_calls 续跑段口径**：审批续跑成功拍只带续跑段轨迹（冻结 continuation 不带
   轨迹, 前段派发不回填）——如实记在字段表, 不做估算。
