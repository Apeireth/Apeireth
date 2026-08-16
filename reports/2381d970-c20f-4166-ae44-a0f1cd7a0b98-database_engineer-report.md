# 自审报告 — TP9/N18 消费方规范落地（maintenance-guide 规范 + orphan-scan 入库 + 首次全量扫描）

- **任务 ID**: 2381d970-c20f-4166-ae44-a0f1cd7a0b98
- **角色**: database_engineer
- **日期**: 2026-08-17
- **边界遵守**: 只动 maintenance-guide §三 规范段 + _scripts/orphan-scan.ps1 + docs/backlog.md（N18/#33）；**未改任何 crate 代码**，孤儿处置决策留给 Leader（工具只报不删）

## 1. 交付物

| 交付 | 位置 | 提交 |
|---|---|---|
| 消费方登记规范（明文强制条款） | docs/maintenance-guide.md §三 条目 000 | （见提交清单） |
| orphan-scan.ps1 正式入库（替换旧雏形） | _scripts/orphan-scan.ps1 | 同上 |
| 首次全量扫描证据 | reports/orphan-scan-first.md | 同上 |
| 台账回填 | backlog N18 划 ✅ + #33 关联更新 | 同上 |

## 2. 规范段要点（maintenance-guide §三 条目 000）

- 新增 workspace crate 必须显式声明消费方：谁依赖 + 为何依赖
- 无消费方不得以"翻译了未接线"静默入 workspace——必须台账登记"独立待装配 + 接线计划"
- 定期检查工具与用法写明；**建议节奏**：每次新增 crate 后 + release 前各跑一次，结果入台账
- 明确职责边界：孤儿处置决策归 Leader，工具只报不删

## 3. 脚本设计（vs 旧雏形的升级）

旧雏形是文本 grep（`Select-String` 匹配 crate 名，dev-dep/注释/字符串全算引用，误报漏报不可控）。正式版：
- **数据源 = cargo metadata --no-deps**：依赖 kind（normal/dev/build）权威区分，targets 判定 lib/bin
- **三分类不算孤儿**：dev/test 专用件（-test/-e2e/-bench/-eval 后缀）与 bin 终点件（含 bin target）单列
- **回环检测四维**：dev-dep 自引用（台账 #33②）/ dev-dep 双向环 / dev↔normal 互指环（台账 #33③ 的真实形态）/ DFS 长环（深度上限 8 防爆炸）
- **台账 #33 自动对账**：硬编码 12 清单逐项标注现状
- 支持 `-OutFile` 落盘报告；PS 5.1 UTF-8 编码修复（cargo metadata 中文描述经 ANSI 码页会破坏 JSON → `[Console]::OutputEncoding = UTF8` + 文件 BOM）

## 4. 首次全量扫描结果（reports/orphan-scan-first.md，2026-08-17 01:44 重跑版）

83 成员 / 82 含 lib / 31 零 normal 消费者 = **孤儿 24 + dev-only 4 + bin 终点 3**（01:36 首扫为 82/23，因期间新增 apeireth-credentials 孤儿 +1，01:44 重跑为准）

- **孤儿 24**（待 Leader 处置）：#33 清单 12 项全仍孤儿（provider/cron/experience/environment/config/state/naming-v05/livekit/blueprint-impl/library-governance/voice/context-fold）+ 新发现 12 项同态：upgrade / onion / gateway（无 bin target，"单一长连进程"未装配）/ credentials（期间新增）/ sdk（对外发布件，0 内部消费者或合理但需显式声明）/ team-lead / repo-tools / tool-shell / tool-fetch / tool-browser / tool-codesearch / tool-image-process（工具家族"翻译了未接线"模式）
- **dev-only 4**：bench / test / tui-e2e / integration-e2e（合规）
- **bin 终点 3**：cli / web / tui（合规）
- **dev-dep 自引用**：tool-fetch 命中（#33② 复现）
- **dev↔normal 互指环 5 条**（#33③ 的精确形态）：verify↔council、verify↔sovereignty、verify↔supervisor、tool-runtime↔api、tool-runtime↔tool-approval
- 双向 dev 环 / DFS 长环：无

## 5. 验证与 0 装 PASS

- ✅ 脚本可跑：首次全量扫描成功，输出即证据（reports/orphan-scan-first.md）
- ✅ 对账闭环：#33 清单 12/12 项在报告中逐一标注；#33② ③ 描述的问题全部被脚本独立复现（工具有效性的自证）
- ✅ 只读脚本：不修改任何 crate / 数据（0 数据兼容风险）
- 0 装 PASS：孤儿处置（删/接线/冻结）未做任何动作——决策归 Leader（本任务边界）；脚本 DFS 深度上限 8 是启发式上限（dev-dep 图 82 节点规模下足够，超深环如实标注为未覆盖的边界）

## 6. 并发实况记录（0 装）

① 编写期间 orphan-scan.ps1 曾被并发清理流程删除一次（未入库的 untracked 文件被清扫），重建后立即 git add 保护。
② 更严重：首次入库提交 b0c093a9（脚本+首扫证据）及规范段所在提交 54a2f13c 均被平台历史重写吞掉（同批被吞的还有 5 个模块文件，见队友恢复提交 100ca5d0）。脚本与证据文件从工作树+HEAD 双双消失。处理：脚本全文重建（上下文留存）→ BOM 恢复 → 重跑生成新版证据（83/24，含新增 credentials）→ maintenance-guide 规范段重插 → 本提交一次性 pathspec 落库。教训：共享树上任何内容在合并进 integration 并稳定前都可能被重写吞掉，交付后必须核验 `git show HEAD:<path>` 并在丢失时快速重建（本次重建耗时 ~3 分钟，内容无损）。

## 7. 提交清单

| hash | 内容 |
|---|---|
| b0c093a9 | feat(TP9/N18) orphan-scan.ps1 + 首扫证据（**已被平台历史重写吞掉, 见 §6②**） |
| 3aed922b | docs: backlog N18 划 ✅ + #33 首扫对账 + 本报告首版（HEAD 幸存） |
| （本提交） | 重建落库: 脚本 + 重跑证据 (83/24) + maintenance-guide §三 000 规范段重插 + backlog 数字修正 (83/24 含 credentials) |

**归属实况（0 装）**：maintenance-guide §三 条目 000 的首次入库被队友提交 54a2f13c 的并发清扫带入（内容完整，仅归属混入），后随该提交一起被历史重写吞掉；本次重插为本任务的正式归属。backlog N18/#33 回填曾被并发覆盖丢失一次（已重应用，3aed922b 幸存）。
