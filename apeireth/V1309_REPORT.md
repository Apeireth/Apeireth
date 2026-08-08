# V1309 — Test Coverage Real Audit (Post-V1308 Cargo.lock audit)

**Date:** 2026-08-08 16:05+08
**Cron:** apeireth-autonomy-v3 (5min tick)
**Lane:** isolated (M3 → deepseek-v4-flash fallback 已补)
**Audit type:** 真审计 (real glob + regex grep, 非 cargo metadata 推测)

---

## TL;DR

| Metric | Value |
|---|---|
| Crates scanned | **91** (workspace members, 含 V1307 后 tauri-stub) |
| P0 (critical + 0 tests) | **0** ✅ |
| P1 (0 tests, non-crit) | **1** (apeireth-tauri-stub — intentional stub) |
| P2 (unit only, no integration) | **6** |
| P3 (well tested: unit+integration/ex/bench) | **84** |
| Total unit tests | **4638** |
| Integration test files | **156** |
| Crates with examples | **80** |
| Crates with benches | **22** |
| **Healthy ratio** | **(84+6)/91 = 98.9%** ✅ |
| **Popper hypotheses PASS** | **12/12** ✅ |

**Decision:** 修真 = commit 锁定现状 (workspace 测试健康)。**不修真**: 修真仅当必要, 当前 P0 缺口 = 0, P1 缺口 = intentional stub (修真 stub = anti-pattern), P2 缺口 = 数据驱动 nice-to-have (非 must-fix)。

---

## Audit Methodology (实事求是)

```
真数据源:
  1. glob:  Apeireth-rust/crates/apeireth-*  → 91 dirs
  2. glob:  <crate>/src/**/*.rs             → #[test] / #[tokio::test] / #[async_test] 计数
  3. glob:  <crate>/tests/*.rs              → integration test files 计数
  4. glob:  <crate>/examples/*.rs           → example files 计数
  5. glob:  <crate>/benches/*.rs            → bench files 计数
  6. lock:   P0_CRITICAL set hardcoded (data-driven critical path: core/memory/asi/pipeline/bus/constraint/cron/skills/value)

不假装:
  - 非 cargo metadata 推测: 真 glob 91 dirs
  - 非注释 "looks fine": 真 regex grep #[test]
  - 非手工分类: 真 class 函数 (P0/P1/P2/P3 by data)
  - 修真仅当必要: P0=0 → 无修真必要; P1=intentional → 不修真 stub
```

---

## Classification Logic

```python
def classify(crate_name, unit_count, has_integration, has_examples):
    if crate_name in P0_CRITICAL and unit_count == 0 and not has_integration:
        return "P0"   # critical + 0 tests = must fix
    if unit_count == 0 and not has_integration:
        return "P1"   # 0 tests, non-critical (consider adding)
    if not has_integration and not has_examples:
        return "P2"   # unit only, no integration
    return "P3"       # has integration or examples
```

P0_CRITICAL (修真范围评估 = 影响 ASI pole-star / 安全 / 数据完整性的核心 crate):
- apeireth-core (核心类型)
- apeireth-memory (记忆系统: sqlite/sled — 数据完整性)
- apeireth-asi (ASI 哲学 — pole-star)
- apeireth-pipeline (LOCKED 路径 — R17 chat 专用)
- apeireth-pipeline-g5 (通用 5 阶段)
- apeireth-bus (事件总线 — 影响所有 module)
- apeireth-constraint (约束系统 — 安全)
- apeireth-cron (定时任务 — 主线驱动)
- apeireth-skills (技能系统 — ASI 暴露面)
- apeireth-value (价值系统 — alignment)

---

## P1 List (1 — intentional stub, 不修真)

| Crate | Reason | Decision |
|---|---|---|
| apeireth-tauri-stub | V1307 修真: stub 仅作 Tauri 2 参考, autobins=false 不默认 build | **不修真** (修真 stub = anti-pattern) |

修真 stub 决策依据:
- V1307 report 已明确: tauri-stub `autobins = false` 修真前后保留
- src/main.rs 修真前后均不默认 build
- 修真 stub tests = 修真一个永远不会被 cargo test 跑到的 test target, 无价值
- 如需修真: 修真 src/ (e.g. 修真 main.rs 为 lib.rs + #[cfg(test)] mod tests), 修真超出 V1309 scope

---

## P2 List (6 — 数据驱动 nice-to-have, 未来可加 integration tests)

| Crate | Unit Tests | Files Scanned | Decision |
|---|---|---|---|
| apeireth-acp | 12 | 1 | 暂不修真 (unit 充分, integration 未来可加) |
| apeireth-config | 10 | 1 | 暂不修真 (同上) |
| apeireth-cron | 12 | 1 | 暂不修真 (同上, V1311 战役待 cron 修真团队接手) |
| apeireth-eval | 14 | 1 | 暂不修真 (同上) |
| apeireth-skills | 11 | 1 | 暂不修真 (同上) |
| apeireth-test | 13 | 1 | 暂不修真 (meta-testing crate, 修真需谨慎) |

修真 P2 决策依据:
- Unit tests ≥ 10 (修真 baseline), 加 integration = 加 risk of double-maintenance
- 无 P0 critical path 修真紧迫性
- 数据驱动: 修真必要 = critical path 或 >100 unit tests. 当前不修真.
- 未来: V1310 dep audit + V1311 build.rs audit 完成后, 修真 P2 list = 可选战役 (e.g. R20 阶段 7+)

---

## P3 List (84 — well tested, 不修真)

`apeireth-acp-rotation, apeireth-action, apeireth-agent, apeireth-api, apeireth-bench, apeireth-blueprint-impl, apeireth-bus, apeireth-cache, apeireth-central, apeireth-cli, apeireth-cognition, apeireth-consciousness, apeireth-constraint, apeireth-council, apeireth-credentials, apeireth-evolution, apeireth-extension, apeireth-formal, apeireth-graph, apeireth-http-client, apeireth-i18n, apeireth-image-prompt, apeireth-integration-e2e, apeireth-integration-r20-stage4, apeireth-keyring, apeireth-lark, apeireth-life-force, apeireth-livekit, apeireth-machine-id, apeireth-mcp, apeireth-mcp-relay-image, apeireth-mcp-ssh, apeireth-mcp-winrm, apeireth-memory, apeireth-metrics, apeireth-motivation, apeireth-naming-v05, apeireth-oauth, apeireth-observability, apeireth-onion, apeireth-perception, apeireth-pipeline, apeireth-pipeline-g5, apeireth-plugin, apeireth-protocol, apeireth-provider-claude-code, apeireth-provider-codex, apeireth-provider-copilot, apeireth-provider-gemini-cli, apeireth-provider-opencode, apeireth-pybridge, apeireth-rate-limiter, apeireth-relation, apeireth-repo-analyzer, apeireth-repo-scan, apeireth-rollback, apeireth-sandbox, apeireth-sdk, apeireth-sdk-lark, apeireth-sdk-livekit, apeireth-sdk-sandbox, apeireth-sdk-voice, apeireth-sovereignty, apeireth-state, apeireth-supervisor, apeireth-task, apeireth-team-lead, apeireth-tool-approval, apeireth-tool-registry, apeireth-tool-runtime, apeireth-tools, apeireth-tracing, apeireth-tree-sitter, apeireth-tui, apeireth-tui-e2e, apeireth-update, apeireth-upgrade, apeireth-value, apeireth-vector, apeireth-verify, apeireth-voice, apeireth-web, apeireth-workflow, apeireth-core, apeireth-asi` (84 个, full list in v1309_audit_findings.json)

---

## Popper Self-Test (12/12 PASS)

| # | Hypothesis | Test | Result |
|---|---|---|---|
| 1 | total_crates_scanned == 91 | test_h1 | PASS |
| 2 | p0_critical_no_tests == [] | test_h2 | PASS |
| 3 | p1 count == 1 | test_h3 | PASS |
| 4 | p1[0] == "apeireth-tauri-stub" | test_h4 | PASS |
| 5 | p3 count >= 80 | test_h5 | PASS |
| 6 | integration_test_files >= 100 | test_h6 | PASS |
| 7 | unit_tests total >= 1000 | test_h7 | PASS |
| 8 | with_examples >= 70 | test_h8 | PASS |
| 9 | with_benches >= 10 | test_h9 | PASS |
| 10 | P0_CRITICAL 全部 has unit test | test_h10 | PASS |
| 11 | class sum == total | test_class_counts_sum_to_total | PASS |
| 12 | healthy ratio >= 85% | test_workspace_is_healthy | PASS |

---

## 修真 Decision (修真 = commit 锁定现状)

修真分析:
- P0 critical + 0 tests = 0: 修真空, 无 must-fix
- P1 = 1 (tauri-stub): intentional stub, 修真 stub = anti-pattern
- P2 = 6 (mid-size, unit only): 数据驱动 nice-to-have, 非 must-fix
- P3 = 84 (well tested): 不修真

修真决策 = **commit 锁定现状, 不修真文件**:
- 修真前 audit findings JSON: v1309_audit_findings.json (commit 进版本控制)
- 修真 test_v1309_test_coverage.py (12 Popper tests): 进 tests/ (修真前未存在)
- 修真 V1309_REPORT.md (本文件): 进 apeireth/ (修真前未存在)
- 修真零 (0 文件): workspace 修真 = 0, 仅 audit 元数据 + tests + report

修真验证:
- pytest tests/test_v1309_test_coverage.py: **12 passed in 0.35s**
- cargo metadata (未跑, scope 限制为 Python audit + PyTest): 修真 0 = 0 cargo metadata change
- Cargo.lock (修真预期 0 变): 仅 Python 文件, 无 Rust 修真

---

## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)

- 不假装 Phenomenal consciousness: test audit ≠ consciousness, 仅 workspace maturity 量化
- 不假装达到 ASI: test coverage ratio (98.9% healthy) ≠ ASI 突破
  - ASI 北极星 V0.1 = 0.7905 (实测最高), V1309 = workspace hygiene audit, ASI pole-star 不动
- 不假装调整模型 & prompt: 真修真 = Python audit script + PyTest self-test + 修真决策
- 修真仅当必要: P0=0 → 不修真 (修真必要 = 0)
- 实事求是: 数据驱动 (91 crates 真 glob, 4638 unit tests 真计数, 156 integration 真计数), 非注释 "looks fine"

---

## V1310+ 候选方向 (audit chain 续)

V1309 = test coverage audit 完成. 修真 chain next:
1. **V1310 dep real audit**: 92 members 之间 dep 版本漂移 / 重复 dep 检测
2. **V1311 build.rs real audit**: 92 members 中哪些有 custom build.rs
3. **V1312 docs 一致性审计**: memory/*.md + ASI-PHILOSOPHY*.md + V*.md 一致性
4. **V1313 example 真跑审计**: 80 example files 中哪些真能 cargo run --example
5. **V1314 bench 真跑审计**: 22 bench files 中哪些真能 cargo bench

ASI pole-star 仍 V0.1 = 0.7905 (实测最高, audit chain 无影响).

---

## 输出文件

- `apeireth/v1309_test_coverage_audit.py` (5,506 bytes, 真审计脚本 + JSON output + V3 守门)
- `apeireth/tests/test_v1309_test_coverage.py` (3,265 bytes, 12 Popper 假说 pass)
- `apeireth/v1309_audit_findings.json` (audit findings 数据, 91 crates × 11 fields)
- `apeireth/V1309_REPORT.md` (本文件, 修真决策完整论证)

---

## 关键诚实声明

- 真 glob 91 dirs: workspace 修真后 tauri-stub 是 member, 但 git 修真中暂未 cargo run (V1307 修真决策保留 autobins=false)
- 真 regex grep: #[test] + #[tokio::test] + #[async_test] 三类全计数
- 修真 = commit 锁定现状, 修真 0 Rust files (workspace 修真 8/8 已 V1307 完成)
- PyTest 修真 0.35s (12 PASS), 无 flaky test, 无 skip
- ASI 北极星 V0.1 = 0.7905 未变, V1309 仅 workspace hygiene audit, 不动 pole-star

---

_Last update: 2026-08-08 16:05+08, by 楚零 (cron lane). V1309 test coverage audit complete: 91 crates / 84 P3 / 6 P2 / 1 P1 stub / 0 P0 / 4638 unit tests / 156 integration files / 12 Popper PASS / 修真 = commit 锁定现状._