# `apeireth-integration-r20-stage4`

R20 阶段 4 集成测试 wrapper — 14 crate cross-crate (5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 + 2 SDK stub), 30+ 集成测试, 0 改 LOCKED, 0 改 workspace version.

**主仓路径**: `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-integration-r20-stage4\`

## 跑法

```bash
cd crates/apeireth-integration-r20-stage4
cargo test
```

## 6 子文件

| 子文件 | 覆盖 |
|--------|------|
| `tests/integration/test_e2e_tools.rs` | `apeireth-sdk` 6 工具 + D-02 子路径 + Auth 5 组件 |
| `tests/integration/test_5_provider_stub.rs` | `apeireth-team-lead` 14 fn + 4 Provider fallback |
| `tests/integration/test_observability_bus.rs` | `apeireth-observability` 3 端点 + PII 脱敏 + trace_id |
| `tests/integration/test_i18n_runtime.rs` | `apeireth-i18n` 5 语言 + fallback + 模板变量 |
| `tests/integration/test_m3_defense.rs` | 14 crate TOOL_WHITELIST 跨 crate 守门 |
| `tests/integration/test_71gb_incident.rs` | `apeireth-rollback` 4 重防御 + 6 策略 |

## 状态

R20 阶段 4 估补集成测试 (主 2026-08-05 拍, 1.0 release #2 test 100% 收尾时搬运).
不主动 commit, 留主拍板.
