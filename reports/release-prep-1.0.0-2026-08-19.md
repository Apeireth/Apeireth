# Apeireth release-prep — v1.0.0

**Date**: 2026-08-19
**HEAD**: `71984e03`
**Mode**: DRY-RUN (1 fail 不阻塞)
**Skip**: none

## 3 维度结果

| # | 类别 | 状态 | 备注 |
|---|------|------|------|
| A.1 | workspace.version = 1.2.0 | ✅ PASS | — |
| A.2 | 0 触碰 24 LOCKED crate (master HEAD) | ✅ PASS | — |
| A.3 | R11 baseline (0.8682/0.8532/0.9063) 在 crates/apeireth-asi/tests/integration_r_measure.rs | ✅ PASS | — |
| A.4 | companion-desktop 是独立 workspace | ✅ PASS | — |
| B | PII 0 命中 (8 关键词全过) | ✅ PASS | — |
| C | 12 项 checklist dry-run OK | ✅ PASS | — |

## 汇总

- PASS:    6
- FAIL:    0
- SKIPPED: 0

## 切 tag 前清单 (per CONTRIBUTING.md)

- [ ] 3 维度全 PASS (本脚本)
- [ ] release-1.0-checklist.sh 12 项 PASS (含 docs/test/security/perf/observability/license)
- [ ] release-1.0.0.yml pipeline dispatch + 5/5 gate green
- [ ] companion-desktop-ci.yml (Tauri shell + pnpm svelte-check) green
- [ ] pii-leak-detection.yml daily cron PASS (next 24h 内会跑)
- [ ] git tag v1.0.0 -m 'release 1.0.0'
- [ ] GitHub release page 写 release notes

