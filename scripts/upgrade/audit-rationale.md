# RUSTSEC audit justification 表

**目的**: 解释 R23-P0 末 cargo-audit 报告 6 个 advisory 为什么接受 (audit-ignore), 不假装 0 风险。

## 评估方法

每件 advisory 走 5 步:

1. **现状**: 谁在锁链, 0 谁直接用
2. **风险**: 风险面在我们代码里 vs 仅 transitive
3. **修复路径**: 上游 bump / 替换 / 自实现的成本
4. **决策**: skip vs 修
5. **8 项承诺守门**: 是否动 LOCKED 24 crate / workspace version / 阶段文档

## 6 件 skip 表

| ID | Crate | Ver | 类 | 风险 | 决策 |
|----|-------|-----|---|------|------|
| RUSTSEC-2024-0384 | instant | 0.1.13 | unmaintained | 0 (transitive only) | skip — 等上游 bump |
| RUSTSEC-2024-0436 | paste | 1.0.15 | unmaintained | 极低 (build macro only) | skip — 等 ratatui/leptos 替换 |
| RUSTSEC-2026-0173 | proc-macro-error2 | 2.0.1 | unmaintained | 0 (build macro only) | skip — 等 leptos 替换 syn 2 |
| RUSTSEC-2026-0174 | http-types | 2.12.0 | notice | 极低 (wiremock dev-dep) | skip — 等 wiremock bump |
| RUSTSEC-2025-0141 | bincode | 2.0.1 | unmaintained | 中 (L1 frame format) | skip — 等下游所有 caller 升级后再切 |
| RUSTSEC-2026-0097 | rand | 0.7.3 | unsound | 中 (rand::rng() attack) | skip — 我们代码 0 custom logger |

## 升 / 跳 的临界点

- 任何 direct workspace member 直接拉这 6 件 → 立刻评估替换 (审计策略失效)
- 上游发布新版本替我们 jump → 自动 unlock (cargo update 周期)
- 1.0 release 后 — 启动 R24 audit 优化, 把 skip 表收尾到 0 件

## cosign.yml audit job 怎么用

```yaml
- name: cargo audit
  run: |
    cargo audit --ignore RUSTSEC-2024-0384 \
                --ignore RUSTSEC-2024-0436 \
                --ignore RUSTSEC-2026-0173 \
                --ignore RUSTSEC-2026-0174 \
                --ignore RUSTSEC-2025-0141 \
                --ignore RUSTSEC-2026-0097 \
                --deny warnings
```

或者读 `scripts/upgrade/audit.toml` (机器可读)。
