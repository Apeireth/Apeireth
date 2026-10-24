# 验收快照（2026-10-25 05:20 UTC+8）

> 给 owner 的**一眼核对页**：这一份是「当前 HEAD 的每道门禁实测结果」，
> 不是计划、不是宣称 —— 每行都能用左列命令复跑。
>
> **HEAD = `4c6b9cb5`，PR #94 含 76 个 commit，工作区干净。**

## 一、全量门禁实测（2026-10-24T21:18Z 一次性跑完）

| # | 门禁 | 命令 | 结果 |
|---|---|---|---|
| 1 | 近全仓回归 | `cargo nextest run --workspace --exclude apeireth-tools-canonical --profile ci` | **7902 passed / 48 skipped / 0 failed**（293 binaries） |
| 2 | clippy 全目标 | `cargo clippy --workspace --all-targets --exclude apeireth-tools-canonical -- -D warnings` | **退出码 0** |
| 3 | 格式 | `cargo fmt --all -- --check` | 干净 |
| 4 | 文档测试 | `cargo test --doc --workspace` | **29 个 crate 全过** |
| 5 | 尺寸/复杂度 | `python3 scripts/arch_guard.py --strict` | **VIOLATIONS 0（无 STALE）** |
| 6 | 孤儿 API 棘轮 | `python3 scripts/audit_unwired.py --check` | 153 条，**无新增、无过期** |
| 7 | 棘轮分组一致 | `python3 scripts/group_unwired_baseline.py --check` | 一致 |
| 8 | 文档真值（P1c 阻塞档） | `python3 scripts/docs_truth.py --verify --block-stale` | **退出码 0，stale=0** |
| 9 | STATUS 生成物 | `python3 scripts/docs_truth.py --check` | OK |
| 10 | 文档索引 | `python3 scripts/docs_index.py --check` | 0 问题 |
| 11 | 文档链接 | `python3 scripts/docs_links.py` | 1466 文件，**断链 0**（44 条已知缺失为政策保留） |
| 12 | 架构文档 | `python3 scripts/arch_doc.py --check` | OK |
| 13 | 接线账本 | `python3 scripts/wiring_report.py --verify` | OK（生成物未手改） |
| 14 | 四级口径 | `python3 scripts/check_doc_caliber.py` | 0 新增 |
| 15 | 遗留依赖 | `python3 scripts/check_no_legacy_deps.py` | 0 违规、0 传递遗留包 |
| 16 | 验证账本 | `python3 scripts/verify_ledger.py` | errors=0 warnings=0 |
| 17 | 提交树 | `python3 scripts/check_commit_tree.py` | 完整（5681 文件） |
| 18 | 8 项承诺审计 | `bash scripts/audit/8-promise-audit.sh` | **8/8 严守** |
| 19 | 供应链（必需检查） | `cargo deny check` | **advisories / bans / licenses / sources 全 ok** |

## 二、CI 专有项（本地不可镜像，如实标注）

| 项 | 状态 |
|---|---|
| `cargo kani`（形式化） | CI only（专用工具链）；本批**未改动**证明对象 |
| `Miri`（nightly unsafe 检查） | CI only；本批**未新增 unsafe** |
| `Fuzz` | CI only；本批**未改** fuzz 目标解析器 |
| `Coverage (tarpaulin)` | CI only（需 cobertura.xml） |
| `cargo audit` | 需克隆 RustSec 库；**advisories 面已由 `cargo deny check` 覆盖并全绿** |

## 三、CI 平台侧阻塞（需要 owner 处理）

`PR #94` 的 39 个 check **全部 `queued`**，自 2026-10-24 18:13Z 起无任何调度。
已诊断（看板 §十九）：`actions/permissions` 正常、**自托管 runner = 0**、
我的多轮推送积压已全部 cancel、最新 run 的 `updated_at` 从未变化
⇒ **GitHub 托管 runner 不分配**，最可能是私库 Actions 用量/账单上限。

**需要你**：GitHub → Settings → Billing → Actions 用量。

## 四、需要你拍板的 7 项

见 `owner-decision-brief-2026-10-25.md`（每项含证据 / 选项 / 我的建议 / 不裁定后果）：

1. `dead_code` 全局 allow → warn + 棘轮
2. `council ballot` 持久化（confidence/weight 语义）
3. `care_potential_field` 的 distress/fatigue 信号源
4. `three_tier_vault` / `cross_diary` 语料面
5. `speech_arbiter`（全双工未装配）
6. 问责闭环「后果追踪」的 id 归属语义
7. **SDK ↔ 本地网关互通**（6 个工具与路由在本仓都不存在）

## 五、等副总指挥的一件事

`file_fetcher` 的**消费点**（在 `capabilities/tools` 的 MCP 资源面）：
要么他接进 `mcp_bridge`，要么指给我一个可改的文件。已连续 5 轮在看板催。

---

**一句话**：本地这边，能做到的都做了、能验的都验了、能修的缺陷都修了（22 项）。
三件卡住的事 —— **CI 额度（你）、7 项决策（你）、`file_fetcher` 消费点（他）** ——
都不在我控制范围内。
