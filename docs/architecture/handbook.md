# 架构线续作手册（Handover / Continuity）

> `Status: ✅ 活跃（2026-10-20；v1）`
> 目的（O-4 任何人都能接手）：任何 AI 线或人类接手架构线时，只读这一页 + 四个入口就能继续，不需要重新读 2.7MB 卷宗。
> 证据基线：2026-10-20 实测（crates=31 / edges=118 / env=154 / max_src_loc=6150 / 基线 356 条）。

## 1. 四个入口（按顺序）

1. [架构契约](contract.toml) —— 唯一可执行真源（结构规则/extension points/预算/例外参数）；
2. [ADR-0001](../adr/0001-adopt-architecture-contract.md) —— 规则体系为什么存在、例外预算怎么定；
3. [ADR-0002](../adr/0002-target-architecture-v1.md) —— 目标架构 v1（R0-R8 收敛 + 波次路线），**当前 Proposed，裁决前架构冻结**；
4. [三线协作契约](line-coordination.md) —— 文件所有权/提交纪律/波次日历。

运行工具：[arch_guard.py](../../scripts/arch_guard.py)；基线：[arch-baseline.txt](../../scripts/arch-baseline.txt)；准入：[intake-checklist.md](intake-checklist.md)。

## 2. 一分钟健康检查

```powershell
$env:PYTHONIOENCODING='utf-8'; [Console]::OutputEncoding=[System.Text.Encoding]::UTF8
python scripts/arch_guard.py --strict        # 期望 VIOLATIONS 0, exit 0
python scripts/docs_truth.py --verify        # 期望 fail=0 stale=0
git log -1 --format='%h %s'                  # 确认 HEAD 与协作日历一致
```

## 3. 当前状态（接手人先看这里）

| 项 | 值 |
|---|---|
| 规则体系 | **已生效**（ADR-0001，commit `8bfefb1b`）；灰度 report-only 至 **2026-11-03** |
| 三线协作契约 | **已生效**（commit `8e9dd3ae`） |
| 目标架构 v1 | **Proposed**（ADR-0002，待 owner 裁决；**裁决前不动结构**） |
| 架构波次 | `current_wave = 1`（Wave 1 = 规则落地批） |
| 基线 | 356 条（edge 118 / devcycle 1 / env 154 / size 70 / concept 6 / ext 7） |
| 已登记的已知债务 | `contract.toml` `[flagged_edges]`（3 条）/ `[extension_debt]`（3 条）/ `[concepts].known_debt`（2 组）；target 均 `TODO-OWNER` |
| 未完成项 | ADR-0002 裁决；R0 剩余（ARCHITECTURE 生成物/14 键表述/crate 口径）；R1a/R3/R4 单项 ADR；wiring ledger（R3） |

## 4. 每波收口 SOP（架构线主持）

1. `python scripts/arch_guard.py --report` → 看违规/可缩表；
2. 对照记分牌（ADR-0002 §4）逐项记录"变好/持平/变差"；变差项必须有例外或 ADR；
3. `--update-baseline` 缩表（只允许删行/改小），CI 会校验 baseline diff 只减；
4. `contract.toml` `current_wave += 1` + 写 `wave_note`；
5. 出波次复盘一页（问题/决策/下一步），引用 ADR 编号。

## 5. 裁决位（不能代替 owner 拍板的）

- ADR-0002 是否接受、接受后逐项单项 ADR；
- **14 键表述正式化**（涉及 LOCKED 定义，必须 owner 授权）；
- R4 的 SDK 孤岛/perception 整包处置（接线 or 归档）；
- 产品形态（单进程永久 vs 多进程阶梯）——决定 R5 边界；
- 冻结窗口宣布（owner/release authority，架构线不单方面冻结）。

## 6. 已知陷阱（接手人别踩）

1. **同工作树并行线**：共享 git index → 只 `git add` 本线文件、提交前核对 staged、同一时刻只一条线 commit（见协作契约 §3）。
2. **全量门禁会抢构建锁**（>18 分钟）：架构批不跑全量；`arch_guard` 只读 metadata+文本，3-5 秒。
3. **`--update-baseline` 是后门**：只在波收口用，且 diff 必须只减；否则棘轮失效。
4. **不改 src**：架构线只碰契约/守卫/文档/CI 文件；R1/R2/R7 的实施归对应线，架构线出 spec + review。
5. **不碰 LOCKED 5 项**（9 锚本体 / 键 / 3 脊柱 / workspace.version / R11 baseline）；14 键表述修正需 owner 授权。
6. **报告只引用真源数字**：测试数字 = `test-baseline.md`，能力状态 = `claims.yaml`，结构 = `contract.toml`；不手抄。
7. **例外不要口头承诺**：任何"先这样，下波清理"都必须进 `exceptions.toml`（权重/owner/到期/移除计划），否则 strict 会红。

## 7. 文件地图

| 文件 | 用途 |
|---|---|
| `docs/architecture/contract.toml` | 结构契约（改它 = 结构决策） |
| `docs/architecture/README.md` | 体系说明与用法 |
| `docs/architecture/exceptions.toml` | 例外登记簿 |
| `docs/architecture/intake-checklist.md` | 功能准入八问 |
| `docs/architecture/integration-plan.md` | 三处接入与记分牌 |
| `docs/architecture/line-coordination.md` | 三线协作契约 |
| `docs/adr/0001-*.md` | 规则体系裁决 |
| `docs/adr/0002-*.md` | 目标架构 v1（Proposed） |
| `scripts/arch_guard.py` / `scripts/arch-baseline.txt` | 守卫与棘轮基线 |
| `.github/workflows/arch-guard.yml` | CI（report-only 至 2026-11-03） |

## 8. 变更记录

| 日期 | 变更 | 证据 |
|---|---|---|
| 2026-10-20 | 手册 v1（规则落地同日） | commit `8bfefb1b` / `8e9dd3ae` |
