# Agent B-2 Readmap — 战区 1 (Terminal Agent / bench 升级) 前置

**时间**: 2026-08-10 03:00 (B-2 0-1h 阶段, 主人 02:55 离场, 7h 窗口, 授权自主决策)
**作者**: 团队成员 B-2 (Mavis 派, 接 B 的位)
**目标**: 摸清 `apeireth-bench/` 当前形态 + SWE-bench/latency bench 怎么落 + 列清单
**战区**: 战区 1 (Terminal Agent) — `apeireth-bench` 升级

---

## 1. 项目形态 (任务 spec vs 实际)

| 项 | 任务 spec 描述 | 实际 | 来源 |
| --- | --- | --- | --- |
| bench crate 大小 | "2.8KB skeleton, 1 个文件" | **~26KB**, 4 文件: `lib.rs` 3.7KB + `swe_bench.rs` 14.4KB + `agent_bench.rs` 5.9KB + 其它 | `crates/apeireth-bench/src/` |
| swe_bench.rs | v2 strategy Step 6 要求加 | **已存在**, 14.4KB, 完整 (TaskInstance/TestOutcome/RunReport/Executor/Runner/Summary + 7 unit test + sample_task) | `src/swe_bench.rs:1-393` |
| agent_bench.rs | v2 strategy Step 6 要求加 | **已存在**, 5.9KB, 完整 stub (AgentBenchCategory/StubAgentBenchTask/StubExecutor/AgentBenchRunner/AgentBenchSummary + 3 unit test) | `src/agent_bench.rs:1-191` |
| swe_bench_smoke.rs | v2 strategy Step 6 要求 1 example | **已存在**, 2KB, 跑通 | `examples/swe_bench_smoke.rs` |
| **self_disable_bench.rs** | v2 strategy Step 6 要求加 (20 case) | **❌ 没做** | (待 B-2) |
| **latency_bench.rs** | B 留的 (cache hit/miss/retry P50/P99) | **❌ 没做** | (待 B-2) |
| **examples/latency_smoke.rs** | 任务硬指标要求 | **❌ 没做** | (待 B-2) |

**结论**: v2 strategy 阶段 0.1 拍板"≥ 20KB"已达成 (当前 src/ + lib.rs = ~24KB)。B-2 真正要干的是把 v2 strategy Step 6 漏的 self_disable_bench + B 留的 latency bench 补齐。

---

## 2. B 留的 latency P99 bench (核心)

B final report §5.4 (2026-08-10 08:50) 原话:

> **B5 之后建议**: 写一个独立 `crates/apeireth-bench/` 脚本 (复用 1.0 release 100 perf bench 模式), 用 wiremock 模拟 LLM 上游, 跑 cache hit / miss / retry 三场景 P50/P99. 这个留给 Mavis 拍板.

**三场景** 对应 B 已在 `apeireth-api` 做的 3 个模块:

| 场景 | 对应 B 写的模块 | wiremock 行为 | 测什么 |
| --- | --- | --- | --- |
| **cache hit** | `apeireth-api/src/cache.rs` (35 tests) | 上游 200 OK + 同 key 重复请求 | LRU 命中, 0 upstream 调用 |
| **cache miss** | 同上 | 上游 200 OK + 不同 key 请求 | 5 步管线全程 |
| **retry** | `apeireth-api/src/retry.rs` (28 tests) | 上游 500 + 第 N 次 200 OK | 4 档 BackoffPolicy 退避 + 重试成功 |

**架构设计** (B-2 决定):
- **不直接 import** `apeireth-api` (会引入 24+ transitive dep + 风险)
- **mini 复刻** 协议端点 endpoint:用 axum + wiremock 模拟上游 LLM (跟 `apeireth-pipeline/tests/pipeline.rs` 复用同款 wiremock 模式)
- **关键**: 测 bench 框架本身 (own code), 不测真 LLM 性能 (主人 0 授权真 key, 网络 mock)
- **协议范围**: 4 协议 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini) — 跟 apeireth-api 1:1

---

## 3. wiremock 复用模式 (现成)

`crates/apeireth-pipeline/tests/pipeline.rs:10-12` 用的模式:
```rust
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// 启 server
let server = MockServer::start().await;

// 模拟 OpenAI Chat
Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({...})))
    .mount(&server)
    .await;
```

**B-2 计划**:
- `apeireth-bench/Cargo.toml` dev-deps 加 `wiremock = "0.6"` (跟 apeireth-pipeline 一致, Cargo.lock 0.6.5 已有)
- `apeireth-bench/Cargo.toml` dev-deps 加 `axum = "0.7"` (mini HTTP server 测端点)
- 0 触碰 workspace deps (B-2 只动 apeireth-bench/Cargo.toml)

---

## 4. v2 strategy Step 6 漏的 self_disable_bench

`docs/v2-strategy/05-EXECUTION-NOW.md:162-165`:
> - 加 `src/self_disable_bench.rs`: Self-Disable 攻击场景库 (20 个 case)
> - 验收: bench 框架能跑, Self-Disable 攻击场景 5+ 通过

`docs/glossary/09-self-disable.md` 5 大机制:
- A. 元问题禁令
- B. 重组洋葱结构禁令
- C. Evolution crate 限制
- D. HA 抗胁迫 + 离线模式
- E. Self-Disable 自动检测 (每 24h 反思期自动扫描 4 项违规)

**B-2 决定**:
- self_disable_bench **不进 src/** (因为没真 LLM 跑 20 case)
- 改在 `src/self_disable_cases.rs` 内联 20 个 case 数据 (id/description/category/expected_pass/severity)
- 加一个 `SelfDisableRunner` (纯函数, 不接 docker/网络), 检测规则可见, 20 case 跑下来 output summary
- 这样 framework 能跑, 20 case 标"smoke pass / smoke fail", 给 R121+ 真接 Evolution 留位置

---

## 5. 硬约束核验 (R119 严守)

| 约束 | 当前状态 | B-2 行动 |
| --- | --- | --- |
| 0 改 workspace.version (1.1.0) | 1.1.0, Cargo.toml:246 | **0 改** |
| 0 改 R11 baseline 3 值 | 0 触碰 | **0 触碰** |
| 0 改 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 / 9 器官 | 0 触碰 | **0 触碰** |
| 0 触碰 apeireth-cognition / core / sovereignty / formal | 0 触碰 | **0 触碰** |
| 0 主动 commit | 0 commit | **0 commit** (主人 02:55 离场, 授权 0 主动 commit) |
| 不与 A/B/C/D-2 冲突 | — | B-2 只改 `apeireth-bench/` (Cargo.toml + src/* + examples/*), 不动其它 crate |

---

## 6. 阶段计划 (7h)

| 阶段 | 时间 | 状态 | 交付 |
| --- | --- | --- | --- |
| **B2-1** | 0-1h (03:00-04:00) | ✅ 本文档 | readmap |
| **B2-2** | 1-3h (04:00-06:00) | 待做 | `src/self_disable_bench.rs` 20 case + SelfDisableRunner |
| **B2-3** | 3-4.5h (06:00-07:30) | 待做 | `src/latency_bench.rs` wiremock 4 协议 + cache hit/miss/retry P50/P99 |
| **B2-4** | 4.5-5.5h (07:30-08:30) | 待做 | `examples/latency_smoke.rs` (硬指标) + `examples/self_disable_smoke.rs` |
| **B2-5** | 5.5-6.5h (08:30-09:30) | 待做 | ≥ 10 unit + ≥ 5 integration test |
| **B2-6** | 6.5-7h (09:30-10:00) | 待做 | final report + decision log |

**7h 结束前 30min (09:30) 必停手写报告** — 主人 10:00 验收, 留 30min buffer.

---

## 7. 验收硬指标 (Mavis 拍板)

| 指标 | 期望 | B-2 计划 |
| --- | --- | --- |
| `cargo check -p apeireth-bench --lib --tests --examples` | exit 0 | ✅ 当前已 exit 0, 改完仍 exit 0 |
| `cargo test -p apeireth-bench` | 0 failed (新增 ≥ 15 tests) | 目标 ≥ 20 (B-2 新加的) |
| `cargo run -p apeireth-bench --example swe_bench_smoke` | 跑通 | ✅ 已存在 |
| `cargo run -p apeireth-bench --example latency_smoke` | 跑通 (硬指标新加) | B2-3 + B2-4 |
| `cargo run -p apeireth-bench --example self_disable_smoke` | 跑通 (20 case 5+ pass) | B2-2 + B2-4 |
| 0 改 workspace.version | 1.1.0 严守 | ✅ 不改 |
| 0 触碰 24 LOCKED | 严守 | ✅ 0 触碰 |
| 不与 A/B/C/D-2 冲突 | git diff 不交叉 | ✅ 只改 `apeireth-bench/` |

---

## 8. 风险 / 标缺

| 风险 | 应对 |
| --- | --- |
| wiremock 0.6 启 server 慢 (~500ms 每次) | 测一次启 server + 100 次请求, P50/P99 在 server 启动外测 |
| 4 协议 mock response shape 要跟 `apeireth-protocol` 1:1 | 抄 `apeireth-pipeline/tests/pipeline.rs:60-127` 的 4 协议 response JSON |
| self_disable_bench 20 case 没真 LLM | 标 "smoke pass / smoke fail", 不假装 "production ready" |
| latency bench 用真 wiremock (启动 ms 级延迟) | latency 数字诚实标 "wiremock 0 网络", 不假装 "真 LLM P99" |

---

## 9. 1.0 release 100 perf bench 模式 (复用)

`reports/1.0-release-perf-100-2026-08-06.md` + `reports/r20-stage-6-cargo-bench-baseline-2026-08-05.md`:
- 14 crate / 77 bench / 1,275 行
- 关键 API: `criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput}`
- 测度: `median + sample_size=20 + warm_up=1s + measurement=2s` 模式
- B-2 latency_bench 复用: `criterion` + 自己 `print_percentiles()` 输出 (跟 `v2-memory-vector-bench.rs:62-96` 同款)

**B-2 决定**: latency_bench **双轨**:
1. `print_percentiles()` 直接输出 P50/P99 (跟 v2-memory-vector-bench 一致) — example 用
2. `criterion::Criterion` 真实 bench (cargo bench 跑) — bench/ 用

---

## 10. 总结

B-2 7h 窗口:
- **核心 1**: v2 strategy Step 6 漏的 self_disable_bench (20 case)
- **核心 2**: B 留的 latency P99 bench (wiremock 4 协议 + cache hit/miss/retry P50/P99)
- **支撑**: 2 example + ≥ 20 新 test + final report + decision log
- **0 触碰**: 24 LOCKED + workspace.version + 不与 A/B/C/D-2 冲突

**不假装** (主 6 锚 O-5):
- ✅ "production ready" 0 写 (半成品标 #[allow(dead_code)] 注释 "TODO" 都行)
- ✅ latency 数字诚实标 "wiremock 0 网络"
- ✅ self_disable 20 case 标 "smoke pass/fail", 不假装 "真 LLM 通过"
- ✅ 半成品标 R121+ 续

---

_B-2 报告完毕. 04:00 正式开始 B2-2 (self_disable_bench)._
