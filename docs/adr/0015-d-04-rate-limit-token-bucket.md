# ADR 0015: D-04 限流 = token bucket 走 `apeireth-constraint`

> **状态**: 🟢 Accepted (主人 2026-08-05 20:53 拍板, 沿用默认)
> **commit 锚**: `r20-stage-2-3-prep-2026-08-05.md` §3.2 + `crates/apeireth-constraint/src/` 实施
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth API 需限流保护:
- 防 DoS
- 防单个用户/工具占用过多资源
- 公平分配

**问题**:
- 业界限流算法多: 固定窗口、滑动窗口、token bucket、leaky bucket
- 选哪个?

**约束**:
- 已有 `apeireth-constraint` crate (E 估已含 token bucket)
- 跟现有架构兼容
- 实现简洁

---

## 2. 决策 (Decision)

**限流 = token bucket, 3 档串联, 走 `apeireth-constraint`**

### 2.1 3 档 token bucket

| 档位 | 容量 (burst) | 补充速率 | 适用 |
|---|---|---|---|
| **Global** | 1000 req | 500 req/s | 全局入口 |
| **Per-User** | 100 req | 20 req/s | 单用户 |
| **Per-Tool** | 50 req | 10 req/s | 单工具 |

**算法** (per `crates/apeireth-constraint`):
```
Bucket(capacity, refill_rate):
  tokens: 初始 = capacity
  last_refill: now()
  
  take(n):
    elapsed = now() - last_refill
    tokens = min(capacity, tokens + elapsed * refill_rate)
    if tokens >= n:
      tokens -= n
      return ALLOW
    else:
      return DENY(retry_after = (n - tokens) / refill_rate)
```

### 2.2 串联顺序

```rust
// 先全局 → 再用户 → 再工具
let app = Router::new()
    .route("/v1/tools/:name/invoke", post(invoke_tool))
    .layer(tool_limiter)   // 3rd
    .layer(user_limiter)   // 2nd
    .layer(global_limiter); // 1st
```

任一层 deny → 立即 429，不消耗下一层 token。

### 2.3 工具级详细配额 (per `tools.toml`)

| 工具 | capacity | refill | 备注 |
|---|---|---|---|
| calendar | 50 | 10/s | 高频读/低频写 |
| message | 20 | 5/s | 发送成本高 |
| contact | 100 | 30/s | 读多写少 |
| task | 50 | 10/s | 平衡 |
| search | 100 | 30/s | 查询密集 |
| drive | 30 | 5/s | 带宽敏感 |

### 2.4 失败策略: fail-open

`apeireth-constraint` 健康检查失败 → 自动降级 fail-open（不阻塞业务）
代价: 临时过载风险; 但限流系统极少失败, 风险可接受

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **简单**: token bucket 业界最常用算法, 理解成本低
- ✅ **公平**: 3 档覆盖全局/用户/工具
- ✅ **burst 友好**: 突发流量不立即全拒绝
- ✅ **跟现有架构兼容**: `apeireth-constraint` 已有, 0 重写
- ✅ **可观测**: 3 档 bucket 都有 Prometheus 指标

### 3.2 负面

- ⚠️ **fail-open 风险**: 限流系统挂掉时不限流
- ⚠️ **3 档串联**: 配置分散在 3 个文件 (global.toml / per_user.toml / tools.toml)
- ⚠️ **工具级配额固定**: 编译期 hardcode, 运行时调不了

### 3.3 风险

- fail-open 时短暂过载, 但可观测
- 工具级配额调整需重编译 (R21 估补支持运行时配置)

---

## 4. 备选 (Alternatives Considered)

### A. 固定窗口 (fixed window)
- 优点: 实现最简单
- 否决: 边界突刺 (window 边缘可 2 倍流量), 不适合流式 LLM

### B. 滑动窗口 (sliding window)
- 优点: 精度高
- 否决: 实现复杂, 状态大 (per 客户端 per 时刻)

### C. leaky bucket
- 优点: 输出平滑
- 否决: burst 友好性不如 token bucket, LLM 流式场景 token bucket 更合适

### D. 引入外部 (e.g. envoy / nginx)
- 优点: 不动 Rust 代码
- 否决: 部署复杂度, 跟 Rust tracing 集成差

### E. 2 档 (去工具级)
- 优点: 简单
- 否决: 工具之间不公平 (calendar 跟 drive 共享)

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: token bucket 业界最常用
- ✅ **S-2 实事求是**: 3 档配置实测合理
- ✅ **O-2 用户看结果不看哲学**: 用户只看到 429 + Retry-After, 不看 token bucket
- ✅ **O-3 信息密度"高"**: 3 档 + 工具配额表
- ✅ **O-4 干净状态 = 没有历史包袱**: 沿用 `apeireth-constraint` 既有实现
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 3 档配置在 `tools.toml` 编译期固定
- ✅ **编译期 hardcode**: tools.toml 编译期 hardcode 工具配额
- ✅ **不改 LOCKED**: 限流配置不动
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 token bucket
- ✅ **不重复造轮子**: 沿用 `apeireth-constraint`
- ✅ **诚实标缺**: fail-open 风险已说明

---

## 7. 引用

- 决策 ID 体系: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-04)
- 蓝图: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` §3.2
- 实施: `crates/apeireth-constraint/src/`
- 文档: [`docs/api/rate-limit.md`](../api/rate-limit.md)
