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
| 目标架构 v1 | **Accepted**（ADR-0002，2026-10-20 owner 拍板）；Wave 2 单项 ADR 已出：0003 R1a / 0004 R3 / 0005 R4a |
| 架构波次 | `current_wave = 2`（Wave 1 规则落地已收口；Wave 2 进行中：R0 剩余 + R1a 合并生物线 W3.5 + R3a + R4a-1） |
| Wave 2 已落 | R4a-1：`scripts/gate_full.ps1`（契约驱动 exclude）+ `.github/workflows/image-family.yml`；`arch_guard` 补例外抑制/登记表双通道/`--print-vendored` |
| 待 owner 裁决 | **无**（R4b 已裁决：SDK 四孤岛 + perception 归档，ADR-0005 §3；14 键已正式化，ADR-0006） |
| R4b 执行 | 实施线待派：按 ADR-0005 §3.1 迁移清单（SDK 四孤岛 23.6k 行 + perception 6.3k 行 → legacy/archived；members 31→30）；架构线验收 |
| 在途违规（灰度期不阻断） | 生物线施工期 2 条 W1：`production.rs` +37 行 / `self_status_source.rs` +414 行；Wave 2 收口随基线更新。env 新开关已由效率线 R3a 登记转正（EXC-0001 已退休） |
| 效率线 R3a 现状（2026-10-20 同步） | `env-registry.toml` 已播种 **155 条**（level=UNCLASSIFIED 占位，owner=TODO-OWNER）；`wiring.yaml` gaps 已按 R3c 口径生成；**R3b foundation 批已验收**（15 条填实、foundation TODO=0、抽查 5/5 evidence 真实，ADR-0004 §7.6）；R3c workflow review 通过（report-only） |
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
| `scripts/gate_full.ps1` | 全量门禁（主 workspace 排除 vendored 图像家族 + 图像独立门禁，清单读契约） |
| `scripts/arch_doc.py` / `ARCHITECTURE.generated.md` | R0 架构依赖图生成物（契约+metadata，CI `--check` 防漂移） |
| `docs/architecture/env-registry.toml` | 开关登记表（R3a，格式由架构线定、效率线播种） |
| `.github/workflows/arch-guard.yml` | 架构守卫 CI（report-only 至 2026-11-03） |
| `.github/workflows/image-family.yml` | 图像家族独立 CI（R4a-1） |

## 8. 波次日志

| 波 | 收口 | 结果 |
|---|---|---|
| **W1 规则落地** | 2026-10-20 | 契约/守卫/基线 356 条/ADR-0001/协作契约落地；`arch_guard --strict` 0 违规；进入灰度（report-only 至 2026-11-03） |
| **W2 架构手术** | **2026-10-20 收口** | ADR-0002 Accepted；ADR-0003 **R1a 交付**（生物线 `151e9aea`/`77b1c1fb`：nervous_snapshot 647 行 + probes 578 行 + 测试 1,205 行，默认关；`self_status_source.rs` −173 行）；R4a-1 门禁隔离；R0 完成（依赖表生成物 + CI）；R3b 主体完成（env 登记 155 条，TODO-OWNER 0、fragments 0，验收见 ADR-0004 §7.7）；缩表 2 条、守卫 0 违规 |
| **W3 架构手术** | **进行中（不提前收口）** | R2/R7 ADR 草案已出（Proposed，待 owner 裁决）；R4b 四岛收口（23,649 行归档 + lark 解耦）；R3b 主体完成（155 条 TODO-OWNER=0）；R3c 待 2026-11-03 后 review；③ c 头环境阻塞；R1b 待 R2 |

## 9. 变更记录

| 日期 | 变更 | 证据 |
|---|---|---|
| 2026-10-20 | 手册 v1（规则落地同日） | commit `8bfefb1b` / `8e9dd3ae` |
| 2026-10-20 | 手册 v1.1：Wave 2 单项 ADR + R4a-1 门禁隔离（gate_full/image CI）+ arch_guard 例外抑制/登记表双通道/`--print-vendored` | commit `5e68f644` |
| 2026-10-20 | Wave 2 收口（R1a 交付 + R3b 主体完成 + 缩基线 2 条）；R4b 执行：B 归档干净、A 内容全落地但遭**共享 index 抢占**拆成 `df78542f`(混) + `6dccb1d2`(净)，裁决 **R2 接受现状不重写历史**（并行线 5 秒级连提，重写会二次事故）；事故登记入 ADR-0005 §3.2 | `22771a50` / `e8e7db33` / `6dccb1d2` |
| 2026-10-20 | **R4b 四岛收口**：lark 解耦（im 自有 wire 原语，行为保持）+ 归档，SDK 四孤岛 42 文件/23,649 行全部迁 `legacy/archived/sdk-islands/`；mutation 验证 = lark 代码引用 0、guard crates=30/edges=112/env=154/VIOLATIONS 0；遗留 = 5 依赖清理批 + c 头重生成 + 文档对齐 + STALE 11 缩表（Wave 3 收口） | `29387d78` / `23cae7cc` |
| 2026-10-20 | **Wave 3 中段复盘**：①文档对齐 `545d0d7f` ②依赖清理 `9f4baa3d` ④R7 ADR `4ea2729c` + R2 ADR `6bd8c9c9`（均 Proposed 待 owner）⑤14 键批 3 `981022e9` ⑥精准缩表 2 条 `d56f172b`（STALE 0）；阻塞：③ c 头（环境无 cbindgen，不伪造）、R2/R7 待裁决、R3c 待 11-03、Wave 3 不提前收口；剩余脏账 = 生物线在途 cli/lib.rs 尺寸增长（非本线） | `d56f172b` |
