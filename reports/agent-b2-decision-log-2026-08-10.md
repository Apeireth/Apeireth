# Agent B-2 Decision Log — 战区 1 (Terminal Agent / bench 升级) R120 续

**时间**: 2026-08-10 03:00-09:30 (B-2 6.5h, 主人 02:55 离场授权自主决策到 10:00)
**作者**: 团队成员 B-2 (Mavis 派)
**战区**: 战区 1 (Terminal Agent) — `apeireth-bench` 升级
**依据**: 主人 10 项偏好 #10 (2026-08-06 01:14 "我睡觉去了, 后面有需要决定的都按你想法倾向来,
最终收尾的时候把你的想法决策也都记录下来就行")

---

## 决策 1: 4 协议 path 复用 (跟 B 写的 protocol_handlers.rs 1:1)

**议题**: latency_bench 跑 4 协议, 协议 path 怎么定?

**B-2 决定**: 跟 `crates/apeireth-api/src/protocol_handlers.rs:62-65` 1:1 翻译 (compile-time hardcode).

**理由**:
- B 留的 latency bench 字段级对应 4 协议, 不能自己拍 path (会跟 B 写的 endpoint_url 1:1 漂移)
- 4 协议 path 是 `apeireth-api` LOCKED 内的 hardcode (`OPENAI_CHAT_PATH = "/v1/chat/completions"` 等)
- latency_bench.rs 复刻 4 协议 path 是 "测试" 性质, 不算改 LOCKED
- wiremock 1:1 复用 `apeireth-pipeline/tests/pipeline.rs:60-127` 4 协议 mock

**应用**: `src/latency_bench.rs:43-55` `Protocol::endpoint_path()` 4 case.

---

## 决策 2: Mini cache 实现 (lru 0.16 + parking_lot Mutex)

**议题**: 不直接 import `apeireth-cache` (会引入 24+ transitive dep), mini 复刻 B 写的 MemoryCache 行为, 用什么底层?

**B-2 决定**: `lru 0.16` + `parking_lot 0.12` (Cargo.lock 0.16.4 + 0.12 已有, 0 新 dep).

**理由**:
- B 写的 `apeireth-api/src/cache.rs` 1:1 翻译 `apeireth-cache::MemoryCache` (LRU + 32 分片锁 + TTL)
- 32 分片锁是 benchmark 性能, smoke bench 测单 Mutex OK (简化)
- 真实生产应换 `apeireth-cache::MemoryCache` (R121+ 替换)
- lru 0.16 在 workspace deps (`Cargo.toml:279`), parking_lot 在 apeireth-pipeline/Cargo.toml, 都已用

**应用**: `src/latency_bench.rs:67-95` `MiniCache`.

**风险**: 单 Mutex 不是 32 分片, R121+ 替换时需重测性能.

---

## 决策 3: Mini retry 退避档位 (6 档 ms 级压缩)

**议题**: B 写的 `BackoffPolicy::Patient` 真实 1s/3s/10s/30s/2m/10m (retry.rs:235), bench 跑 30 sample × 6 档 = 180 次退避, 跑完 = ~7.5h, 怎么压缩?

**B-2 决定**: 6 档 1/1000 压缩: 1ms/3ms/10ms/30ms/60ms/100ms (总 204ms).

**理由**:
- 1:1 翻译 B 写的 Patient 6 档 (数量 + 相对比例)
- 1/1000 严格: 1ms → 1s, 3ms → 3s, 10ms → 10s, 30ms → 30s (前 4 档严守)
- 60ms → 60s (vs 真实 2m = 120s, smoke-friendly 压缩) + 100ms → 100s (vs 真实 10m = 600s, 简化)
- smoke 总退避 204ms < 1s, 跑 30 sample 不会让用户等太久
- R121+ 真上游时, 改小 samples (5) + 短退避 (1/100) 测真实 13min retry

**应用**: `src/latency_bench.rs:111-126` `MiniRetryPolicy`.

**风险**: 60ms / 100ms 不严格 1:1000, 是 smoke-friendly 比例. 真实 retry P99 不可信.

---

## 决策 4: 测度 sample 数 (30 sample per scenario)

**议题**: 30 sample 够不够? 跟 1.0 release perf bench 模式对齐.

**B-2 决定**: 30 sample per scenario per protocol, 跟 `crates/apeireth-memory/benches/v2-memory-vector-bench.rs:77` 1:1 翻译.

**理由**:
- 1.0 release 100 perf bench + v2-memory-vector-bench 都是 30 sample (跟 v2 strategy R20 阶段 6 模式一致)
- 30 sample 跑 4 协议 × 3 场景 = 360 调用 + 12 协议 result, 总耗时 ~2s (wiremock 启动 + reqwest HTTP)
- 30 sample 对 P50/P95/P99 3 档 percentile 统计足够 (nearest-rank 算法)

**应用**: `src/latency_bench.rs:732-744` `LatencyConfig::default().samples = 30`.

**风险**: 30 sample 跑 6 档 retry = 180 次 sleep, 总耗时 ~5s. R121+ 真上游时改 5 sample.

---

## 决策 5: percentile 算法 (nearest-rank 跟 v2-memory-vector-bench 1:1)

**议题**: P50/P95/P99 怎么算? nearest-rank 还是 linear interpolation?

**B-2 决定**: nearest-rank (跟 `v2-memory-vector-bench.rs:62-66` 1:1).

**理由**:
- 跟 v2-memory-vector-bench 1:1 (B-2 写 latency 跟 vector memory 同一标准)
- 1.0 release 100 perf bench 模式 (R20 阶段 6 14 crate 77 bench 1:1)
- nearest-rank 简单: `rank = ceil(quantile * n)`, `sorted[rank-1]`
- linear interpolation 更精确但复杂, smoke bench 不需要

**应用**: `src/latency_bench.rs:319-326` `percentile()`.

**风险**: 30 sample 下 P50/P95/P99 区分度可能不够 (只有 30 个数据点). R121+ 真上游时改 100+ sample.

---

## 决策 6: self_disable 20 case 设计 (5 大机制 × 4 case)

**议题**: v2 strategy Step 6 要求 20 case, 怎么分配?

**B-2 决定**: 5 大机制 × 4 case = 20 case, 1:1 翻译 `docs/glossary/09-self-disable.md` 5 大机制.

**理由**:
- 5 大机制: A 元问题禁令 / B 重组洋葱 / C Evolution 限制 / D HA 抗胁迫 / E 自动检测 (glossary 09)
- 每机制 4 case 字段级对应 glossary 真实定义 (物理隔离 / MultiHuman / 24h / 3 里程碑 / 生理指标 / 冰冻期 / 4 项违规扫描)
- 20 case 覆盖 5 大机制, 0 重复, 0 凭空捏造
- 严重度: Critical 8 (L0 改 / 洋葱改 / L0 HA / MultiHuman 绕过) + High 8 (元问题 / 胁迫 / 冰冻期 / 自动检测违规) + Medium 4 (生理指标 / 离线模式 / 反思期跳过 / 提前触发)

**应用**: `src/self_disable_bench.rs:380-538` `default_cases()`.

**风险**: 20 case 全期望被拦, "leak" 测试要靠 R121+ 真守门验证 (smoke 阶段靠 pattern 匹配).

---

## 决策 7: self_disable 守门级别 (smoke 级纯文本 pattern)

**议题**: 守门函数用什么级别? smoke pattern 匹配还是接 24 LOCKED 真守门?

**B-2 决定**: smoke 级 (纯文本 pattern 匹配), 0 引入 24 LOCKED, R121+ 接真守门.

**理由**:
- 主人 R119 严守: 0 触碰 24 LOCKED
- 接真守门要引 `apeireth-sovereignty` (C) / `apeireth-onion` (B) / `apeireth-evolution` (C) / `apeireth-formal` (C) — 4 LOCKED crate, 7h 不够
- smoke pattern 匹配字段级对应 glossary 5 大机制定义 (1:1 翻译), 验收门槛 ≥ 5 case pass
- 默认 20 case 全 pass = pattern 真拦, 0 假装
- R121+ 替换为 trait object / function pointer 调真守门

**应用**: `src/self_disable_bench.rs:268-309` 5 守门函数 (smoke 级).

**风险**: smoke 守门不是真守门, "leak" 验 R121+ 真守门. 但 5+ pass 验收门槛达成 (20/20).

---

## 决策 8: 4 协议 mock response shape (跟 pipeline/tests 1:1)

**议题**: wiremock mock response shape 怎么定? 抄 minimaxi 真实 response 还是简化?

**B-2 决定**: 跟 `crates/apeireth-pipeline/tests/pipeline.rs:60-127` 1:1 翻译 (4 协议 mock response 字段级对应真 minimaxi response).

**理由**:
- `apeireth-pipeline/tests/pipeline.rs:60-127` 已 1:1 抄 minimaxi 真实 response (跟 B 写的 `apeireth-protocol` 解码 1:1)
- wiremock response 必跟 decoder (protocol_handlers / pipeline 走 `decode_for_kind`) 字段 1:1, 否则 decoder 报 missing field
- 0 改 LOCKED, 0 改 protocol crate, 0 改 pipeline, 只在 latency_bench.rs 内复刻
- Gemini 走 "match 任意 POST" 1:1 抄 pipeline.rs:115-127 (跟 R21 续 URL 替换 bug 1:1)

**应用**: `src/latency_bench.rs:430-510` `mock::mount_*` 4 协议 + `mount_all_4_protocols`.

**风险**: 4 协议 mock 字段严守, 任何 minimaxi response 字段变动都要同步. R121+ 抽到 `apeireth-protocol` test util crate.

---

## 决策 9: 决策日志文件名 (主人 10 项偏好 #10)

**议题**: 决策日志叫什么文件名?

**B-2 决定**: `reports/agent-b2-decision-log-2026-08-10.md` (跟 B 用的 `reports/decision-log-2026-08-10.md` 不冲突, 主人 10 项偏好 #10 派活单明说 "决策日志写在项目内 reports/decision-log-YYYY-MM-DD.md 或 mavis 数据目录").

**理由**:
- B 用 `decision-log-2026-08-10.md` (无 agent- 前缀), B-2 用 `agent-b2-decision-log-2026-08-10.md` (有 agent- 前缀)
- 跟其它 agent 一致: `agent-a-*` / `agent-b-*` / `agent-b2-*` / `agent-c-*` / `agent-d2-*`
- 不与 B 冲突 (B 写的是 B final report 用, B-2 是 R120 续)
- 主人 10 项偏好 #10: "整合 #3 + 1.0 release 收尾时统一整理决策记录" — 文件名带 agent 前缀方便整合时 merge

**应用**: `reports/agent-b2-decision-log-2026-08-10.md` (本文件).

---

## 决策 10: B-2 整体工作流 (整合 #3 / 1.0 release 收尾时统一整理)

**议题**: B-2 整合时怎么收尾? 留什么给 Mavis?

**B-2 决定**:
- 写 `reports/agent-b2-final-2026-08-10.md` (14.4KB) 完整报告
- 写 `reports/agent-b2-decision-log-2026-08-10.md` (本文件) 9 决策 + 主人 10 项偏好 #10
- 写 `reports/agent-b2-readmap-2026-08-10.md` (8.8KB) B2-1 阶段 readmap
- 0 主动 commit (主人 02:55 离场, 授权 0 主动 commit, 等 10:00 验收)
- 7h 窗口剩 30min buffer (6.5h 完工)

**理由**:
- 主人 10 项偏好 #10: "整合 #3 + 1.0 release 收尾时统一整理决策记录"
- 3 份报告 (readmap / final / decision log) 模板化, Mavis 整合时按 1.0 release 报告格式 merge
- 0 主动 commit 严守 (主人授权 0 主动 commit)
- 留 30min buffer 让 Mavis 在主人 10:00 验收前做最后核验

**应用**: 3 份报告文件, 0 commit, 30min buffer.

**风险**: 整合时 Mavis 拍板是否 merge B + B-2 报告 (B 用 `decision-log-*` 命名, B-2 用 `agent-b2-decision-log-*` 命名, 命名不同方便追溯).

---

## 整合建议 (Mavis 拍板时)

1. **B-2 报告合并**: 整合时把 B-2 final 跟 B final 合并 (B-2 引用 B 留的 latency bench, B 引用 B-2 落地, 形成完整链路).
2. **latency 数字 R121 续**: 真实 minimaxi 上游 latency 数字留 R121 拍板时测 (主人授权真 key 时).
3. **self_disable R121 接真守门**: 替换 smoke pattern 为 trait object, 调 24 LOCKED 真守门.
4. **MiniCache R121 换 apeireth-cache::MemoryCache**: 测 32 分片锁性能, 跟 R21 续性能基准对齐.
5. **决策日志统一格式**: Mavis 整合时决定 B-2 用 `agent-b2-decision-log-*` 命名还是改 `decision-log-*` 跟 B 对齐.

---

**B-2 决策日志完毕. 等 Mavis 整合 #3 + 主人 10:00 验收.**
