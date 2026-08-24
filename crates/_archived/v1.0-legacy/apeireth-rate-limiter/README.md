# apeireth-rate-limiter

> Apeireth 专用 rate limiter (R20 阶段 6 估补, token/leaky/fixed/sliding window 4 算法 + 5 storage stub, 0 真接 R20 阶段 6 skeleton; **2026-08-19 借鉴 3 限流重试 (LiteLLM full-jitter + opencode agent retry + Guardrails action policy) 加 retry 模块 — Backoff trait + ConstantBackoff + ExponentialBackoff + RetryAfter + decide 4 步决策**; 测试数 (单测标注): **70** (retry 14 + 算法/边界/统计/storage 56, 0 装严守 HTTP-date 0 解走 fallback).

apeireth-rate-limiter 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
