# `apeireth-integration-e2e` — 三层端到端集成测试 (R20 阶段 5)

**Apeireth 集成测试 e2e** — 主仓 + API + TUI 三层端到端集成, 60+ 测试, **不碰 24 LOCKED**, **不**改 parent workspace Cargo.toml.

## 目的

`apeireth-integration-e2e` 是 R20 阶段 5 集成测试估补的最后一块, 跨三层验证:

| 层 | 测什么 | 工具 |
|---|------|------|
| **Workspace** | 主仓 `cargo check` / `cargo test` 状态, 0 LOCKED 触碰, 0 sandbox 错路径, 0 改 workspace version | `cargo metadata` + 文件系统审计 |
| **API** | 6 端点 e2e (tools / memory / organs / asi / sovereignty / agent + 401/404/500/200 + 8 帧 WebSocket) | `wiremock` mock + `reqwest` 真发 |
| **TUI** | 5 nav + 9 器官 1 屏 4 panel 端到端, 跟 `apeireth-tui-e2e` 互补 | `ratatui` `TestBackend` |

## 跟现有 e2e crate 的关系

| crate | 范围 | 关系 |
|------|------|------|
| `apeireth-tui-e2e` | TUI 5 nav + 9 器官 (20+ 测试) | **本 crate 互补**, 本 crate 镜像 tui 公开 API surface, 加 14 个 nav/organ + 颜色 + 哲学锚 |
| `apeireth-integration-e2e` (本 crate) | 主仓 + API + TUI 三层 (60+ 测试) | **三层 e2e**, 跑全栈 + workspace 审计 + 报告 |

## 8 项不修改承诺 (per `docs/stage4/8-locked-unified-2026-08-05.md`)

1. **0 改 24 LOCKED crate** — 仅新建本 crate, 0 触碰任何已有 `crates/*/src/`
2. **0 改 parent workspace Cargo.toml** — 采用 sub-workspace 模式 (跟 `apeireth-rate-limiter` 同款), 不进 parent members 列表
3. **0 改 workspace version** — 本 crate 自己 `version = "1.0.0"` 硬编码 (跟 parent 一致, 但**不**修改 parent)
4. **6 哲学锚穿透** — `S-1` (北极星) / `S-2` (实事求是) / `O-2` (走在前人肩上) / `O-3` (干到底) / `O-4` (任何人都能接手) / `O-5` (不假装)
5. **0 依赖 NewAPI** — wiremock + ratatui 全是工业标准, 0 外部代理
6. **0 重复造轮子** — 用 `wiremock::MockServer` 走 0.6, `ratatui::TestBackend` 走 0.29, 不另起
7. **0 假装实缺** — 60+ 测试全跑真代码路径, 跳过的用 `#[ignore]` + 原因
8. **0 主动 commit** — 落到主仓路径, **不** `git commit`

## 模块布局

```
crates/apeireth-integration-e2e/
├── Cargo.toml                         # sub-workspace, 5 path-dep 全部进 dev
├── README.md                          # 本文件
├── src/
│   ├── lib.rs                         # 主入口, 500+ 行三层 e2e 文档
│   ├── error.rs                       # E2EError 9 变体 hardcode
│   ├── harness.rs                     # IntegrationHarness (workspace + API + TUI)
│   ├── api_e2e.rs                     # 19 API 端点 e2e
│   ├── tui_e2e.rs                     # 14 TUI nav + organ e2e
│   ├── workspace_e2e.rs               # 5 workspace 状态 e2e
│   └── report.rs                      # E2eReport + 4 格式化函数
├── tests/
│   └── test_integration_e2e_in_process.rs  # 60+ 集成测试
└── examples/
    └── integration_e2e_demo.rs        # 跑全部测试 + 报告
```

## 跑法 (sub-workspace 模式)

```bash
cd .openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-integration-e2e

# 编译
cargo check

# 跑全部 60+ 集成测试
cargo test

# 跑单个测试
cargo test test_api_metrics_endpoint_returns_prometheus

# 跑带 output
cargo test -- --nocapture

# 跑 demo example
cargo run --example integration_e2e_demo
```

## 验收 (per 派活单)

- [x] 8 文件齐全 (1 lib + 5 src + 1 test + 1 example + Cargo.toml + README)
- [x] 60+ 测试 (`cargo test` 全过)
- [x] `cargo check` 0 error
- [x] 0 触碰 24 LOCKED crate (`git diff crates/*/src/` 0 行)
- [x] 0 改 parent workspace Cargo.toml (sub-workspace)
- [x] 6 哲学锚 / 8 项承诺全在 `lib.rs` 头部
- [x] **不**主动 commit

## 子模块 API 速查

- [`IntegrationHarness`] — 三层 harness, `start()` / `shutdown()` / `workspace_*` / `api_*` / `tui_*`
- [`E2EError`] — 9 变体统一错误
- [`E2eReport`] / [`E2eLayer`] / [`E2eLayerReport`] — 报告三件套
- [`api_e2e::test_api_*`] — 19 个 API 端点 e2e
- [`tui_e2e::test_tui_*`] — 14 个 TUI e2e (5 nav + 9 organ)
- [`workspace_e2e::test_workspace_*`] — 5 个 workspace 状态 e2e
- [`report::generate_report`] / [`report::format_human_readable`] / [`report::format_json`] / [`report::assert_all_passed`]

## 状态: R20 阶段 5 估补集成测试 (主 2026-08-05 派)
