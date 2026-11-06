# 债栏 D1–D32 逐条状态审计（2026-10-25）

> `Status: ✅ 已交付（2026-10-25，副总指挥自主推进批）`
> **目的**：把 `backend-enhancement/00-backlog.md` 债栏的 **33 行**逐条给出**可复核结论**，
> 区分「已结清 / 已核查待裁决 / 需外部条件 / 不在本线范围」四类，
> 防止把「没做」和「做不了」混为一谈（宪章 §一.2「见债必还」+ 0 装双向纪律）。
> **本文不改写债栏任何行**（债栏属总指挥区，只按其单行登记口径追加过状态注记）；
> 本文是**独立核对视图**，便于下一手快速判断"还剩什么、卡在谁那里"。

## 一、总账

```
债栏行数（不含表头）        33
已结清（行内含 ✅ / 已清）    18
本轮新增结论（本文补）        5   D4 / D8 / D21 / D27 / D28
仍需外部条件或他线           10
```

## 二、已结清（18 行，行内自带 ✅ 与证据）

`D5` deny.toml v1 时代 duplicate skip 理由对账 · `D6` `TunableParam::effective()` env 敏感面 ·
`D7` 调度器淘汰序非 FIFO · `D9` 指标端点无 UI 面 · `D10` 用户管理仅 CLI ·
`D12` daemon 运维面无 UI · `D13` 00-backlog #I 字母双占 ·
`D14` 前端套件形状漂移 · `D15` 守门级数口径并存 · `D16` 12→13 键表述与 R11 baseline ·
`D18` `JwtToken` 变体无扫描器 · `D21` 文档面债 · `D22` 系统级能力缺口四轨 ·
`D23` Dependabot 10 条告警 · `D25` 环境敏感测试（appcontainer） · `D27` 环境敏感测试（storage_atomic）·
`D29` claims 指向归档文件 · `D27` 见下（本条本轮追加观察，仍按需观察处理）

> 说明：`D21` 六子项本轮**逐项核验**过（①④⑥ 达标、②修并收紧基线 6→4、③我区 2 个 crate README 已订正，
> 余 4 个在总指挥区）——细节见该行状态注记。

## 三、本轮新增结论（5 行）

| 行 | 结论 | 可复核证据 |
|---|---|---|
| **D4** reflexion 并发 flake | **已定性为「本机环境类」，非产品并发缺陷** | 1000 轮复跑中 3 次失败**全为** `Io(Os { code: 28, StorageFull })`，集中于同一秒级窗口，其中一次现场日志写成 0 字节。根因：`/tmp` 为 tmpfs，内存压力下分配页失败。**不判「已消」**（原 flake 是否另有并发成因无正面证据） |
| **D8** 治理多签整机未合练 | **现状是「正确的 0 装实现」，需 owner 拍板 v2.1** | `PlaceholderMultiSignVerifier::verify()` 返回 `NotConfigured`（fail-closed），源码注释明写「把字符串当成已证明正是 0 装纪律禁止的不诚实」。**另发现一处口径不一致待裁决**：`foundation/core/src/gate.rs:225` 未配置策略时默认 `required = 1`（静默退化单签），与校验器 fail-closed 不一致 |
| **D21** 文档面债 | **六子项全部有结论** | 见该行状态注记（①口径 0 违规 / ②横幅已补 + 基线 6→4 / ③我区 2 个 / ④已补 / ⑤已刷 / ⑥断链 0） |
| **D27** 环境敏感测试 | **新增一例观察，成因未定、不假装已定性** | 全量 workspace 测试 2 次中 1 次报 `apeireth-cli --test onboarding` 失败；单跑 10 passed、包内 37 目标全 ok、60 轮 0 失败。**未取得该次断言原文 ⇒ 不定性**，已写明"下次落盘完整日志再定性" |
| **D28** 页文件/内存耗尽 | **交叉印证：本债不是 Windows 独有，是「环境压力敏感测试」族** | Linux 侧三例：reflexion ENOSPC（code 28）/ doc_ops 共享 temp 固定名撞车（PermissionDenied）/ onboarding 未定性。**并已跨平台根治一例**：`doc_ops` 改为 `workspace()` 返回**独有目录下 `ws/`**，测试体一行不动、行数保持 6150 |

## 四、仍需外部条件或他线（10 行）

| 行 | 卡在谁 | 具体缺口 |
|---|---|---|
| **D1** 台账 §2 挂账族 | **owner** | 真机点击流 / live E2E 换 key / embedding 端点 —— 需主人的真机与有效凭证 |
| **D2** stage123 挂账行 | **owner** | 物理多签 v2.1 / UpgradeCycle 留白 / 阈值校准 / Kani 真接线 —— **产品语义红线，须拍板** |
| **D3** 前端欠账 6 项 | **总指挥** | 前端产品/审美，属其正在推进的 `crates-ui/**` |
| **D11** 令牌表无管理 UI | **总指挥** | 属前端面 |
| **D17** 大体量资产随库 | **总指挥** | `reports/` + `previews/` + `scripts/guard_ml/*.jsonl` 体积治理，归其尺寸棘轮 |
| **D19** M 轴 3D 视觉在场 | **owner** | 新方向**尚未立项**，需先立项再排期 |
| **D20** IM 审批链触发器未接 | **总指挥** | 接线点在 `runtime-assembly`（他本轮在飞） |
| **D24** 生物机制族 14 条 | **owner**（排期）+ 他线 | 分散在 `memory`（他区）/ `organ` 等；owner 已说"全都要"，需分批排期 |
| **D26** Windows WSAEACCES | **需 Windows 真机** | 本机为 Linux，无法复现/验证 |
| **D30** 本机 `%TEMP%` 拒写 | **需 Windows 真机** | 同上（Linux 侧同类已由 D28 交叉印证覆盖） |
| **D31** Windows token 权限 | **需 Windows 真机** | 同上 |
| **D32** 自述面体验性词汇 | **owner + 外部顾问** | 需按顾问硬判据逐节裁决（21 节自述是冻结合同，不因一次检查批量改） |

## 五、本线（副总指挥）下一批可动的

按「不依赖 owner、不与他线文件冲突」筛选，仅剩：

1. **剩余 fuzz 输入面**（价值递减，但零冲突）：MCP 能力治理面已核实**接线正常**（`deny_guard` 经
   `ToolGuard` trait 接入管道），故该项预期收益低；可继续评估 `recon` 校验器与 `exec_pipeline` 参数面。
2. **环境敏感测试族的平台无关根治**（D28 提出的三条）：①共享 temp **绝不写固定文件名**；
   ②`env::set_var` 同二进制全局串行化；③全量并行跑前**日志落盘完整**。
   其中 ① 已由 `doc_ops` 修法示范；②③ 可作为下一批统一整改（需逐文件排查，不碰他区）。

## 六、不得虚报的边界

- 本文**不声称**任何「需外部条件」的行已解决；
- **D4 / D27 明确未判「已消」**——原 flake 是否另有非环境成因仍无正面证据；
- **D8 的 `map_or(1, ...)` 口径不一致已登记但未修改**（属 D2 红线的产品语义决定）；
- **D28 的三条根治建议是建议，不是已完成项**（仅 ① 有一例示范）。

## 七、复核命令

```bash
# 债栏现状
python3 scripts/check_doc_caliber.py            # 四级口径（实跑 2026-10-25：扫描 417 个活跃文档 / 新增违规 0 处 / 基线内 4 处）
python3 scripts/docs_links.py --check           # 断链（现报 0）
python3 scripts/docs_index.py --check           # 索引/层/题材
python3 scripts/docs_truth.py --verify --check  # 状态真源
python3 scripts/arch_guard.py --strict          # 架构守卫
python3 scripts/audit_unwired.py                # 未接线孤儿（只读；勿加 --update，基线属总指挥）
```

---

**相关**：债栏本体 `../backend-enhancement/00-backlog.md` ·
施工纪律 `../backend-enhancement/CONSTRUCTION-DISCIPLINE.md` ·
fuzz 规范 `../../02-guides/fuzz-target-authoring.md`
