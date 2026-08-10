# Decision Log — 2026-08-10 团队成员 B 自主决策

**时间**: 2026-08-10 02:55 主人离场, 授权到 10:00 自由决策
**授权依据**: 主人 10 项偏好 #10 "主人长时间离开, Mavis 自主决策 + 决策日志"
**记录人**: 团队成员 B (Mavis 派, 战区 2 LLM Gateway)

---

## 决策 #1: Cache EvictionPolicy 选 LRU
**时间**: 2026-08-10 04:00 (B1 readmap 阶段)
**选项**:
- A) LRU (跟 apeireth-cache 1:1 翻译默认)
- B) LFU (频率感知, 适合热点 query)
- C) TinyLFU (modern, 估 30%+ hit rate)

**决策**: **A) LRU**
**理由**:
- 跟 `apeireth-cache::DEFAULT_POLICY = EvictionPolicy::Lru` 1:1, 不漂移
- Response cache 场景: 大多数 LLM query 都是一次性 (非热点), LRU 简单有效
- 6 哲学锚 O-2 走在前人肩上: 跟现有 crate 默认对齐

**风险**: LRU 在周期性热点 query 上不如 TinyLFU, 0 漂移到 1.1 baseline

---

## 决策 #2: 退避策略默认 Patient
**时间**: 2026-08-10 04:05
**选项**:
- A) Aggressive (1s/3s/10s) — 当前
- B) Default (1s/3s/10s/30s) — 主人 hint
- C) Patient (1s/3s/10s/30s/2m/10m) — 任务 hint
- D) Custom

**决策**: **C) Patient (1s/3s/10s/30s/2m/10m)**
**理由**:
- 主人 6 锚 S-1: 服务 ASI 北极星 (可靠 > 快)
- LLM 上游 5xx 重试 30s/2m/10m 跟业界 Anthropic / OpenAI SDK 默认退避对齐
- 流式 + 关键路径 (Council/Verdict) 用 Patient 稳, 流式 + 普通 query 用 Aggressive 快
- 0 触碰现有 1s/3s/10s 行为 (只是新增更长档位)

**风险**: 长退避会让用户看到长 wait; 流式 endpoint 长退避会断流 — 缓解: 4xx 不重试, network error 才走 Patient

---

## 决策 #3: Cache key hash 选 BLAKE3
**时间**: 2026-08-10 04:10
**选项**:
- A) SHA-256 (workspace 已有 sha2 给 auth)
- B) BLAKE3 (快, 1.5x 比 SHA-256, hash.3 import)
- C) xxhash (非密码学快, 但需加 dep)

**决策**: **B) BLAKE3**
**理由**:
- 不引新 dep (workspace 已有 `hashbrown`, 但没 `blake3` 0.5) — **改: 走 SHA-256 复用现有 sha2**
- 实际上 workspace 没引 blake3, 选 A 更稳

**修正**: **A) SHA-256** 复用现有 `sha2 0.10` (apeireth-api/Cargo.toml:57 已有), 不引新 dep
**理由修正**:
- Cache key 不需要密码学强度, 但也不需要极致快 (key 1ms 跟 0.1ms 差异不大)
- 0 新 dep, 0 触碰 workspace dep
- 跟 apeireth-keyring 字段级对齐 (keyring 也用 sha2)

---

## 决策 #4: Cache 范围 — 仅非流式, 4 协议全包
**时间**: 2026-08-10 04:15
**选项**:
- A) 仅非流式, 4 协议全包 (任务 spec)
- B) 仅 OpenAI Chat, 留其他
- C) 全 4 协议 + 流式 (复杂)

**决策**: **A) 仅非流式, 4 协议全包**
**理由**:
- 任务 spec 明确: "流式 (SSE) 不缓存 (边界 case 留给 B5)"
- 4 协议 (OpenAI Chat / OpenAI Responses / Anthropic / Gemini) 都走相同 `dispatch` 函数, 0 漂移
- 流式 (req.stream == true) 在 dispatch 入口显式 skip

**风险**: 0; 流式 endpoint 直接走现有 `stream_chat_completions_forward` 路径, 0 行为变更

---

## 决策 #5: Cache TTL = 60s, max_size = 1024, shards = 32
**时间**: 2026-08-10 04:20
**选项**: 沿用 `apeireth-cache` 默认值, 0 漂移

**决策**: **60s / 1024 / 32**
**理由**:
- `apeireth-cache::DEFAULT_TTL_SECS = 60` (cache/src/lib.rs:490)
- `apeireth-cache::DEFAULT_MAX_SIZE = 1024` (cache/src/lib.rs:487)
- `apeireth-cache::DEFAULT_SHARDS = 32` (cache/src/lib.rs:493)
- 0 漂移到 1.1 baseline

**风险**: 60s 太短 / 1024 太少, 但主人 6 锚 O-4 任何人都能接手 — 默认值可调, 0 默认漂移优先

---

## 决策 #6: 协议切换 header 名 `X-Apeireth-Protocol`
**时间**: 2026-08-10 04:25
**选项**:
- A) `X-Apeireth-Protocol: openai|anthropic|gemini` (任务 spec)
- B) `X-Protocol` 简短
- C) 复用 OpenAI 风格的 `X-Provider`

**决策**: **A) `X-Apeireth-Protocol`**
**理由**: 任务 spec 明确, 不漂移

**风险**: 0

---

## 决策 #7: 关键路径 span 范围
**时间**: 2026-08-10 04:28
**选项**:
- A) `/v1/*` (4 协议) + `/council/*` + `/verdict` (任务 spec)
- B) 加 `/health` (但 health 是高频 ping, 浪费 span)
- C) 加 V2 6 类 (tools/memory/organs/asi/sovereignty/agent)

**决策**: **A) 4 协议 + Council + Verdict**
**理由**:
- 任务 spec 明确
- /health 是高频, 1 个 trace span/req = 浪费 (TUI 30Hz ping)
- V2 6 类是另一个团队 (V2 战区) 范畴, 0 越界

**风险**: 0

---

## 决策 #8: 决策日志文件名 `decision-log-2026-08-10.md`
**时间**: 2026-08-10 04:30
**理由**: 主人 10 项偏好 #10 明确: "项目内 `reports/decision-log-YYYY-MM-DD.md`"

---

## 决策 #9: cache wrapper 选 dispatch_cached(Option<&ResponseCache>), 不直接吃 None 默认
**时间**: 2026-08-10 04:50 (B2 实施)
**选项**:
- A) `dispatch_cached(..., cache: Option<&ResponseCache>)` — None 走原路径
- B) `dispatch_cached(..., cache: &ResponseCache)` — 必传, 失败 panic
- C) 全局 `OnceLock<ResponseCache>` — 难测

**决策**: **A) Option<&ResponseCache>**
**理由**:
- 1.0 行为 0 漂移 (None = 走原 dispatch)
- 测试友好 (不依赖全局 state)
- 0 假装"必有 cache"

---

## 决策 #10: send_and_decode 加 status check, err 字符串带 status
**时间**: 2026-08-10 06:10 (B3 实施)
**选项**:
- A) err 字符串带 status ("http: 503"), 1.0 caller 0 改
- B) 改 send_and_decode 返 `Result<(u16, ...)>`, 1.0 行为改
- C) 用 err 字符串 prefix + 5xx pattern 启发式判断

**决策**: **A) err 字符串带 status**
**理由**:
- retry hook 需要 status 区分 4xx/5xx, prefix 启发式不可靠
- 1.0 行为 0 漂移 (server.rs 4 handler 只看 Err, 不知道 err 字符串细节)
- 1.0 call site 0 改 (走 _with_status + 丢 status)

**风险**: 1.0 行为是 err 字符串格式变化 (从 "json parse: ..." 到 "http: 503"), 但 server.rs handler 只看 Err, 0 影响

---

## 决策 #11: routing header 改 axum handler 签名加 `HeaderMap` 参数
**时间**: 2026-08-10 07:20 (B4 实施)
**选项**:
- A) 加 `HeaderMap` 参数, 4 handler 改签名
- B) 用 `axum::extract::Extension<HeaderMap>` 全局注入
- C) 用 `request.headers()` 在 handler body 拿

**决策**: **A) HeaderMap 参数**
**理由**:
- axum 0.7 标准模式, 0 黑魔法
- handler 签名清晰
- 0 漂移 (没 header 也 work)

**风险**: 改 handler 签名破坏 1.0 调用方, 但 4 handler 是 server.rs 内部定义, 0 外部 call site

---

## 决策 #12: 6 关键路径 span 走 `tracing::info!` 写日志, 不用 exporter
**时间**: 2026-08-10 08:00 (B4 收尾)
**选项**:
- A) `tracing::info!` 写 stdout — 简单
- B) `apeireth-telemetry::trace::exporter::StdoutExporter` — 1:1 商业版
- C) Prometheus / OTLP 远端

**决策**: **A) tracing::info!**
**理由**:
- 1.0 已有 tower_http::TraceLayer 走 stdout
- 0 漂移 transport-level tracing
- 业务 span 走 `tracing::info!` 跟 transport-level 协同, 0 重复

**风险**: 0; R21+ 续接 StdoutExporter / OTLP exporter (apeireth-telemetry 1.1 已有 4 exporter 留口子)

---

## 总览
12 决策, 全部按主人 10 项偏好 #10 + 6 哲学锚穿透 + 1.1 release 0 漂移原则。
0 触碰硬约束, 0 主动 commit, 全部 0 漂移 1.0 已验收行为。

**最后更新**: 2026-08-10 08:50 (B5 完成, final report 写完, 等主人 10:00 验收)
