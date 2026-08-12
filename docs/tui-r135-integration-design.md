# TUI R135 接入设计 (A 路 #4: 后端已就绪, TUI 接入方案)

**状态**: 设计文档 (0 触碰 TUI 代码, 等主人 R135 拍板后执行)
**日期**: 2026-08-12
**作者**: 楚零 (Apeireth AI agent, 优优的 0 触碰助手)
**对应 R 周期**: R135 (A 路 #4 总验收 + TUI 准备)

## 1. 背景

R131 + R132 + R133 + R134 完成后, 主人冻结的 **A 路** 4 阶段 (A1 骨架清账 / A2 弱项补强 / A3 孤岛消解 / A4 总验收 + TUI 准备) 全部完成:

- **A1 骨架清账** (R132): 5 战区 × 1 真 LLM e2e / pybridge 跨语言 / integration-e2e 跑全 5 战区 / 0 stub 留尾巴
- **A2 弱项补强** (R133): 6 步全过 — Self-Disable 形式化 / ApprovalBridge / retry+backoff / rate-limiter / telemetry 4 umbrella / SDK 估补
- **A3 孤岛消解** (R134): 5 真孤岛 3 标 2 填实 (acp/cron/team-lead 标 archived, repo-tools/governance 写 example)
- **A4 总验收** (R135): 全 workspace 20599/20599 PASS, 0 失败, 5/5 战区 e2e, + TUI 接入 design doc (本文件)

后端已经 100% 就绪, 主人 R131 期间显式拍板"后端完全做好"再接 TUI. 现 R135 是冻结定义中的 **TUI 准备** 阶段, 写 1 份 design doc 阐明 TUI 如何接入已就绪后端.

## 2. 当前 TUI 状态 (per 0 触碰原则)

`apeireth-tui` 是 binary + lib 双输出 crate, 12 mod 真实实现:
- `app` (14KB) / `backend` (188KB) / `cognition_live` (11KB) / `config_watcher` (5KB) / `error` (9KB) / `http_llm` (26KB) / `http` (18KB) / `llm_config` (10KB) / `main` (47KB) / `observability` (36KB) / `onboarding` (7KB) / `persistence` (16KB) / `theme` (12KB)
- 5 nav 页面: bridge / dialogue / growth / history / settings (per `pages` mod)
- 0 改动 (R135 阶段 0 触碰, 等拍板后执行)

TUI 已有 backend.rs (188KB) 跟后端 crate (api/agent/council/memory) 走 HTTP 通信. R135 design 重点: **TUI 现有 backend 接口已能调后端, R133 后端升级需在 TUI 侧验证或加新入口**.

## 3. TUI 接入清单 (R135 拍板后执行, 当前 0 触碰)

### 3.1 必接 (R133 后端已就绪, TUI 接入缺这些)

| 后端能力 (R133) | 当前 TUI 是否用 | 设计接入点 |
|---|---|---|
| `ToolCallPipeline::new_with_policy_and_rate_limit` (5 阶段 + ApprovalBridge + rate-limiter) | ❌ 0 用 | TUI `bridge` 页面加 "Tool pipeline inspector" — 调 ApprovalBridge 真接 tool_call, 演示 policy 拒绝 + rate limit |
| `ToolCallPipeline` 5 阶段 telemetry counter | ❌ 0 用 | TUI `settings` 页面加 "Pipeline metrics" — 调 `init_tool_metrics` + 读 5 counter 显示 |
| `Apeireth-formal` 5 Kani proof 形式化验证 | ❌ 0 用 | TUI `growth` 页面加 "Formal proofs" — 调 Kani proof runner 显示 5 proof pass 状态 |
| `apeireth-repo-tools` 4 API (scan/stats/key_files/git_state) | ❌ 0 用 | TUI `history` 页面加 "Repo scan" 入口 — 调 `RepoScanner` 显示当前 repo 状态 |

### 3.2 可选 (R135 拍板后, 主人定优先级)

| 后端能力 | 设计接入点 |
|---|---|
| `apeireth-rate-limiter` 4 算法 | TUI `settings` 加 "Rate limiter demo" — 调 token_bucket/leaky/fixed/sliding 4 算法, 演示 1/s 限流 |
| `apeireth-library-governance` evaluate | TUI `settings` 加 "Governance check" — 调 GovernanceEngine.evaluate 显示 5 策略派发 |
| `apeireth-tool-approval` 5 规则 | TUI `bridge` 页面加 "Approval 规则" — 调 ApprovalManager 5 规则编辑 (Blacklist/Trust/Risk/Frequency/Whitelist) |

### 3.3 不接 (R135 拍板后, 主人排除)

- `apeireth-acp` / `apeireth-cron` / `apeireth-team-lead` (R134 已标 ARCHIVED, 0 真接价值)
- 商业版 SDK 真接 (lark/voice/livekit) — 需付费凭证, 留 R135+ 续

## 4. 接入实现方案

### 4.1 调用链 (TUI → 后端)

```
apeireth-tui::backend::pipeline_inspector (新增)
    ↓ HTTP /v1/guard (R131.2 API)
apeireth-api::protocol_handlers::guard_handler
    ↓ 调
apeireth-tool-runtime::ToolCallPipeline::new_with_policy_and_rate_limit
    ↓ 调
apeireth-tool-approval::ApprovalBridge
apeireth-rate-limiter::RateLimiterImpl
```

### 4.2 TUI UI 改动 (8 处, R135+ 拍板后)

| 页面 | 改动 | 估计行数 |
|---|---|---|
| `bridge` | 加 "Tool pipeline inspector" 入口 + 渲染 | ~150 行 |
| `settings` | 加 "Pipeline metrics" + "Rate limiter demo" + "Governance check" 3 入口 | ~200 行 |
| `growth` | 加 "Formal proofs" 入口 | ~80 行 |
| `history` | 加 "Repo scan" 入口 | ~80 行 |
| `pages` 共 5 文件 | 1 入口文件 + 4 渲染函数 | ~510 行 |

总改动 ~1020 行 TUI 代码, 0 触碰现有逻辑 (R135 原则: 增量添加, 0 改既有).

### 4.3 后端 API 改动 (R131.2 已就绪, R135 复用)

R131.2 已建 `/v1/guard` 端点 (R131.2 设计 doc 1 个, 主人拍板后实装), R135 接入时直接调:
- `POST /v1/guard` body: `{ tool_name, args, user_id }` → 返 `{ allow | deny | require_approval | throttle }`
- TUI 用这个做 policy inspector demo
- R135 不动 API, 仅 TUI 侧加 UI

## 5. 验收定义 (R135+ 拍板后)

1. **TUI 编译**: `cargo build -p apeireth-tui` 0 errors
2. **TUI 单元测试**: `cargo test -p apeireth-tui --lib` 全过 (现有 0 fail 维持)
3. **TUI e2e 冒烟**: `cargo run -p apeireth-tui` 起, 5 页面 + 新增入口不闪退
4. **5 战区 e2e** (A1): 维持 5/5 PASS
5. **A 路 #1-#4 全过**: 4 阶段冻结定义 100% 闭环

## 6. 时间估算 + 风险

- **总工时**: R135+ 拍板后, 1 个 R 周期 (5-7 天) 足够
- **风险 1**: TUI backend.rs 188KB 依赖 api/agent/council/memory HTTP 协议, R133 后端升级需要 TUI HTTP 客户端适配 (R131.2 `/v1/guard` 路径已就绪)
- **风险 2**: 8 处 UI 改动可能引入 regression, R135 阶段严格 0 触碰既有 (仅增量)
- **风险 3**: `apeireth-formal` Kani proof 在 TUI 子进程调可能受 Kani 编译器依赖, R135 阶段不直接调, 走 `run_all()` API (sanity check 替代)

## 7. 不假装

- 本 design doc **不** 触碰 TUI 代码, 仅写规划
- 8 处 UI 改动是估算, 实际 R135+ 拍板后实现时可能有 ±20% 偏差
- 风险 1 (HTTP 适配) 是潜在 blocker, R135 拍板后第一时间验证
- 接入清单 3.1 必接 / 3.2 可选 / 3.3 不接 三段分类基于 **当前已知 R133 能力**, R135+ 后端可能再升级, 接入清单会随之更新

## 8. R135 总验收清单 (A 路 #4)

- [x] R131 + R132 + R133 + R134 4 阶段冻结定义 100% 闭环
- [x] cargo test --workspace 20599/20599 PASS, 0 失败
- [x] 5 战区 e2e 5/5 (R131.7 + R132.6)
- [x] 11 R133 e2e + 30+ unit + 441 telemetry + 40 SDK + 5 R134 = **527+ 测试 PASS**
- [x] TUI 接入 design doc (本文件)
- [ ] TUI 接入 R135+ (等主人拍板)
- [ ] R135+ commit + tag (等主人执行)

**R135 待办**: 主人拍板 TUI 接入方案 (本文件 3.1 必接 / 3.2 可选 / 3.3 不接) → 我执行 8 处 UI 改动 → 主人 commit.
