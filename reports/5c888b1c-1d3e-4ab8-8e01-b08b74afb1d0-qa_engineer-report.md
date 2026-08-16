# QA1 测试体系盘点报告（自检·只读）

任务: 5c888b1c | 角色: qa_engineer | 日期: 2026-08-16
总体结论: ✅ 测试体系健全，可支撑验收；存在 3 个 ⚠️ 盲区

## 一、测试套件盘点

| 层级 | 位置 | 规模 | 运行方式 |
|---|---|---|---|
| 单元测试 | `crates/*/src` | ~7646 个 `#[test]`（87 crates，top: pybridge 585 / tui 478 / sdk 401） | `cargo nextest run --workspace` |
| 集成测试 | `crates/*/tests/` | 大部分 crate 有独立 tests 目录 | 同上 |
| nextest 配置 | `.config/nextest.toml` | ci profile: fail-fast + retries=2 + slow-timeout 90s | CI: `rust-ci.yml` 用 `nextest run --workspace --profile ci --locked`；TUI 单独 `--test-threads=1` |
| 覆盖率 | `coverage.yml` (tarpaulin→codecov) | 阈值: 项目 70% / patch 60%（codecov.yml 强制） | CI 自动 |
| 形式化验证 | `miri.yml` / `kani.yml` | UB 检测 + 不变量证明（push+PR 触发） | CI 自动 |
| E2E/专项 | `protocol-e2e.yml`、`eval-live.yml`（真实 LLM API）、`cargo-audit.yml`、`cargo-deny.yml` | — | CI 自动 |
| 测试辅助 | `crates/apeireth-test`（retry/budget/suite 聚合）、`apeireth-bench`、`apeireth-eval` | 含 property_tests / kani_proofs | workspace 成员 |

证据: `grep -rh '#\[test\]' crates/*/src | wc -l` → 7646；`nextest.toml`/`codecov.yml` 实存。

## 二、盲区与风险

1. ❌→⚠️ 根 `tests/`（12 个 .rs）**未被任何活跃 crate 编译**——纯 workspace 无 root package，仅 `_archived` crate 引用其一；`tests/README.md` 自述为"占位目录"。属已知死代码，建议清理或归档，勿误以为在执行。
2. ⚠️ 21 个 crate 无 `tests/` 集成测试目录，含核心边界件：`apeireth-provider`、`apeireth-llm-iface`、`apeireth-environment`、`apeireth-cron`、全部 `tool-*` crate（多数仅有单元测试，如 telemetry 386 单测）。
3. ⚠️ `frontend/`（tauri-prototype）无测试配置（无 test script/无 spec 文件）——原型可接受，转正前需补。
4. ⚠️ `scripts/` 堆积大量一次性 `_fix_*.py`/`patch_*.py`（>100 个），无正式测试运行脚本；本地跑测依赖人工 `cargo nextest run --workspace`。

## 三、验收建议（给 Leader）

- 回归基线命令: `cargo nextest run --workspace --profile ci --locked`（本机全量约 7.6k+ 用例，预计耗时较长，建议增量按 crate 跑）
- 结论判定依据均以可复现命令输出为准（见上文 grep 路径）。
