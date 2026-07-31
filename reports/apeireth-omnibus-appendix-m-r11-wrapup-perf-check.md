# M2.5-PERF — R11 收尾性能数字只读校验

> 校验对象：`reports/apeireth-omnibus-appendix-m-r11-wrapup-draft.md`；主证据：`reports/r11-performance.md`、`apeireth/v1130_continuity_tracker_dashboard.py`、`apeireth/v1136_dashboard_render.py`；必要时追溯各 R11 交付报告。未修改代码或 Omnibus。
>
> 判定：✓ = 数字与对应证据一致；❌ = 不满足 1:1（数字冲突、快照混用或范围错位）。代码只证明实现口径/阈值，运行数字必须由报告或产物证明。

## 13 项核对表

| # | 草稿/核对项 | 判定 | 1:1 证据 | 误差源 / 接手说明 |
|---:|---|:---:|---|---|
| 1 | V1136 render：cold/warm/combined median p95 = 81.5/40.8/72.4µs；5×100；34 tests | ✓ | `r11-performance.md:98-114,201-213`；实现 `v1136_dashboard_render.py:407-423,429-436` | 实现证明 percentile/bench 口径，具体微秒数仅是 2026-07-30 本机样本；草稿 §1.2:49 一致。标题说 p50/p95/p99，但列出的三个数均是 **p95 median**，不是三个 percentile。 |
| 2 | V1075 `/health` 200 latency=1150.4ms；起停 1.17s | ✓ | `r11-devops-deployment-report.md:88-92`；`r11-v1075-process/deployment-report.md:48` | 不在三份主性能证据中；是进程 fallback 单次链路，不等于后端稳态 P95。草稿 §1.2:52 一致。 |
| 3 | V1130 wallclock ≈7–11s vs 2.5s；`IC_V1130_UNREACHABLE` | ✓ | `r11-architect-integration-contract.md:22,214,240,332`；实现阈值 `v1130_continuity_tracker_dashboard.py:283-320` | 报告实点 8695ms，7–11s 是多次运行范围；代码只锁定 2.5s target。该项位于草稿范围声明/§3/§5，不在 §1.2。 |
| 4 | V1138 哲学守门 44 passed in 0.31s | ✓ | `r11-v1138-delivery-summary.md:30-49` | 非性能基准，是 pytest 验收耗时；草稿 §1.1:39 一致，不在 §1.2。 |
| 5 | V1136 `v05_total=0.9063` vs V1131 `0.8532` | ❌ **P0** | QA 终态：`r11-qa-acceptance.json:10-16,30-46`；旧 perf 快照：`r11-performance.md:141-151` 写 V1136 **0.8595**、V1125 placeholder 0.8532 | **跨快照硬冲突**：0.9063 与指定 `r11-performance.md` 不 1:1，差 +0.0468；且 perf 报告的 0.8532 标签是 V1125 占位，不是 V1131 dashboard。草稿必须注明“QA 终态 0.9063”并禁止引用旧 perf 报告为同次运行。该项在 §0，不在 §1.2。 |
| 6 | V1138 集成验收 4/4，30.59s | ✓ | `r11-qa-acceptance.json:5-9,102-109` | 单次离线验收快照；草稿 §0:22/§1.1:38 一致，不在 §1.2。 |
| 7 | p0_workflow 14/14；level_score=0.8964；regress=187/187 | ✓ | `r11-workflow.md:6,68,119,159` | 187 是 V1136 选定回归子集，不是仓库 6394 全量；草稿 §1.1:40 一致。 |
| 8 | requirements gate 5/5；21/21；107 passed in 37.93s | ✓ | `r11-requirements-gate.machine.md:11,111,136-141`（107/37.93）；草稿/门报告给出 5/5、21/21 | 三个计数属不同层级（gate、gate 单测、pytest 子集），不可相加；草稿 §1.1:42 一致。 |
| 9 | P0 regression guard 57/57 in 16.26s | ✓ | `r11-ate-p0-regression-guard-report.md:22` | 同报告后续双轨复跑为 14.15s/14.18s（:198-202）；16.26s 是本地首轮有效快照，不是唯一终态耗时。 |
| 10 | orchestration 15/15 in 19.6s | ✓ | `r11-orchestration.md:322` | 报告写“15 用例，pytest 19.6s 全过”；草稿 §1.1:41 一致。 |
| 11 | automation 197 passed, 2 skipped in 55.53s（47.1s） | ❌ **P0** | 历史：`r11-automation.md:31-53`（表计 47.1s；命令 55.53s）；终态：`:146-180` 为 **200 passed, 2 skipped in 49.20s**；草稿 §1.4:70 采用终态 | **快照/范围错位**：197/2 是初次交付历史，不是当前终态；47.1s 与 55.53s 是同一 197 套件的两次计时，不应写成一个无标签耗时。若附录写终态，应保留 200/2/49.20s；若保留 197，必须显式标“历史初跑”。 |
| 12 | MCP 39/39 契约 + 119/119 回归无破坏 | ✓ | `r11-mcp-integration.md:348` | 两个测试集合，不是 158 个单一套件；草稿 §1.3:61 一致。 |
| 13 | V1141 IC 57/57（51 fast 12.96s + 6 slow ≈80s） | ✓ | `r11-architect-integration-contract.md:6,332` | ≈80s 是 6 slow 的近似总耗时，不能与 12.96s 合称单次 92.96s；草稿 §1.3:62 一致。 |

## P0 硬错与范围错位

1. **P0-DATA-01（第 5 项）**：QA 终态 0.9063 与指定性能报告旧快照 0.8595 不一致；必须加快照/来源标签，不能声称 1:1。
2. **P0-DATA-02（第 11 项）**：197/2/55.53s（或表计 47.1s）是历史初跑；当前终态为 200/2/49.20s。附录现用终态是正确方向，但核对清单把历史数当现值不成立。
3. **P0-SCOPE-01**：所谓“§1.2 的 13 项”范围错误。§1.2 仅直接包含 #1、#2；#4/#6-#10 在 §0/§1.1，#12/#13 在 §1.3，#11 在 §1.4，#3 在范围声明/§3/§5，#5 在 §0。
4. `v1136_dashboard_render.py` 实现可复现 p50/p95/p99，但不固化 81.5/40.8/72.4µs；`v1130_continuity_tracker_dashboard.py` 固化 2.5s 阈值，但不证明 7–11s 样本。接手者复跑时应生成新快照，不覆盖历史。

## 结论（交接版）

- 汇总：**11/13 数字可追溯；2/13 不满足 1:1，另有 1 个章节范围 P0。**
- 第 5 项必须区分旧 perf 快照 0.8595 与 QA 终态 0.9063。
- 第 11 项必须区分 automation 历史 197/2 与终态 200/2。
- §1.2 只覆盖 dashboard render 与 V1075 fallback，不应承载全部 13 项。
- V1130 仍为明确未达标项：实点 8.695s、经验范围约 7–11s，对 2.5s target 失败。
- 微秒 render 指标达标不代表 V1130 build 或 HTTP 进程链路达标。
- 建议附录最终追加前修正来源/快照标签；不需要改代码。
