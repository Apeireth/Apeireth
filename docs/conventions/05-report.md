# 05 报告路径系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §5 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/05-report.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`reports/<type>-<id>-<role>-<name>.md`

## 13 种报告类型(核验后)

| # | 类型 | 格式 | 例子 (核验) |
|---|---|---|---|
| 1 | 单个成就 | `achievement-A<n>-<role>-<name>.md` | `achievement-A1-backend-cli-session.md` |
| 2 | 5 成就回顾 | `retrospective-A<n>-<role>.md` | `retrospective-A5-backend.md` |
| 3 | 最终总结 | `final-A20-<role>-<name>.md` | `final-A20-backend-all-crate-complete.md` |
| 4 | 漂移报告 | `drift-<stage>-<§>-<date>.md` | `drift-stage4-1.5-2026-07-31.md` |
| 5 | R 周期收尾 | `r<N>-finalize-<date>.md` | `r14-finalize-2026-07-31.md` |
| 6 | R 周期 baseline | `r<N>-baseline-verification-<date>.md` | `r14-baseline-verification-2026-07-31.md` |
| 7 | R 周期 SEC | `r<N>-sec-cross-validation-<date>.md` | `r12-sec-cross-validation-2026-07-30.md` |
| 8 | 架构评审 | `<task-id>-<topic>-arch-check.md` | `apeireth-omnibus-appendix-n-r12-handoff-arch-check.md` |
| 9 | R 周报 (R16+) | `r<N>-week<X>-<topic>-<date>.md` | `r16-week1-hello-llm-2026-08-03.md` |
| 10 | R 真接验证 (R16+) | `r<N>-real-key-final-<date>.md` | `r16-real-key-final-2026-08-03.md` |
| 11 | R 后端验收 (R16+) | `r<N>-backend-verification-<date>.md` | `r16-backend-verification-2026-08-03.md` |
| 12 | R batch final (R33+) | `r<N>-r<M>-<topic>-<date>.md` | `r78-r113-batch-final-2026-08-10.md` |
| 13 | R LIVE 真接 (R32+) | `r<N>-live-<topic>-<date>.md` | `r82-live-minimax-8model-results.md` |

## 不规范但实际存在的报告(主人 8/10 01:14 拍板"按你建议来,核验后写",不要求重命名)

- `V<n>-<topic>-<date>.md` (V 系列, 17+ 文件, 验收报告)
- `P<n>-<topic>-<date>.md` (P 系列, 31+ 文件, 工程期决定)
- `borrow-<source>-<n>-<topic>-<date>.md` (借鉴报告, 借 Golutra 等)
- `apeireth-<topic>-<date>.md` (apeireth-* 专题)
- `1.0-release-<topic>-<date>.md` (1.0 release 续补)
- `decision-log-<date>.md` (决策日志)
- `apeireth-r<n>-<topic>-<date>.md` (R 周期专题)
- `round<N>-<NN>-<topic>.md` (round 短格式)

**原则**: 主人 R119 拍板"技术发展史可以不要,思想历史 + 最新",这些不规范报告是 R-Round 阶段的副产物,**保留作为历史记录**,不强制重命名(工程投入不划算)。

## 核验

- ✅ reports/ 实际 25+ 报告, R 周期 R11-R118
- ✅ 13 规范类型 + 8+ 不规范类型
- ✅ R78-R113 batch final (R119-3a-1 重建时最新)
- ✅ R114-R118 batch final (`5c546a84` codex)

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
