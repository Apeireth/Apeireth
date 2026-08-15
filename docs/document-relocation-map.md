[Document-Meta]
Document:        document-relocation-map.md
Version:         0.1-DRAFT
Layer:           文档治理 (归位映射)
Last-Modified:   2026-08-15
Status:          🟢 已执行 (2026-08-15)
Author:          主人 + AI 协作者
Source-of-Truth: 主人 2026-08-15 拍板 ("顶层/产品设计 → stage1, 具体想法 → stage2")
0 主动 commit:   严守 (仅出映射表, 不 commit)
0 装 PASS 严守:  严守 (本表是规划, 尚未执行移动)

# 文档归位映射表

> **归位原则 (主人拍板)**:
> - **stage1 = 顶层设计 + 产品设计** —— 回答「Apeireth 是什么 / 为什么 / 长成什么样」: 哲学、愿景、洋葱、守门、器官、产品闭环、前端形态。
> - **stage2 = 具体想法** —— 回答「怎么做」: 工程决策、模块设计、技术栈、调研、竞品、路线。
> - **stage3-6 保持** —— 施工层 (图纸/落实/施工/验证), 本质是「具体」的更深展开, 不动。
>
> **LOCKED 的初心是不偏移, 不是不整理。** 归位只动「文件位置 + README 索引」, 不改 stage1/stage2 已 LOCKED 的**原文内容**。新文档 (产品闭环 / 愿景) 是**新增**, 不触碰既有 LOCKED 正文。

---

## 0. TL;DR

当前「顶层/产品设计」类文档散落在 6+ 个目录 (docs/ 根、v2-strategy/、omnibus/、spirit/、r14-design/、youyou 桌面), 造成「心的方向」和「工程的方向」混在一起。本表把它们收敛成两桶:

- **归位到 stage1** (顶层+产品): 产品闭环、愿景、哲学、洋葱、器官、前端形态、架构 v3/v4。
- **归位到 stage2** (具体想法): 调研、竞品、技术审查、模块设计、集成蓝图、路线。
- **保持不动**: 规范 (conventions)、术语 (glossary)、交接 (handover)、施工层 (stage3-6)、历史基线。

---

## 1. 表 A —— 归位到 stage1 (顶层设计 + 产品设计)

| # | 文档 | 现位置 | 归类 | 动作 |
|---|---|---|---|---|
| 1 | product-loop-design.md | docs/ | 产品设计 (产品闭环) | 移 → docs/stage1/ |
| 2 | 阿佩瑞斯-未来愿景小说.txt | Desktop/youyou/ | 愿景 (产品) | 移 → docs/stage1/ |
| 3 | Apeireth的未来哲学杂谈1.txt | Desktop/youyou/ | 哲学/愿景 (伙伴) | 移 → docs/stage1/ |
| 4 | 00-VISION.md | docs/v2-strategy/ | 顶层愿景 | 移 → docs/stage1/ |
| 5 | 08-MULTIPARADIGM-FRONTEND-AND-SECURITY-VISION.md | docs/v2-strategy/ | 产品设计 (前端形态) | 移 → docs/stage1/ |
| 6 | design-v2-v4-v4.1-v6.md | docs/omnibus/ | 顶层设计汇总 | 移 → docs/stage1/ |
| 7 | philosophy-core.md | docs/omnibus/ | 哲学核心 | 移 → docs/stage1/ |
| 8 | 9-organs.md | docs/omnibus/ | 9 organ 顶层 | 移 → docs/stage1/ |
| 9 | architecture-v3-aircraft-carrier.md | docs/ | 顶层架构 (航母) | 移 → docs/stage1/ |
| 10 | architecture-v4-living-intelligence.md | docs/ | 顶层架构 (生命) | 移 → docs/stage1/ |
| 11 | architecture-v4-1-living-intelligence-update.md | docs/ | 顶层架构 (生命更新) | 移 → docs/stage1/ |
| 12 | onion-wall-architecture-2026-07-31.md | docs/ (与 r14-design/ 重复) | 顶层 (洋葱墙) | 移 → docs/stage1/ (去重) |
| 13 | r14-design-philosophy-2026-07-30.md | docs/r14-design/ | 哲学 (8 原则) | 移 → docs/stage1/ |
| 14 | philosophy-traits-2026-07-30.md | docs/r14-design/ | 哲学 (trait 框架) | 移 → docs/stage1/ |
| 15 | rust-traits-spec-2026-07-30.md | docs/r14-design/ | 哲学/规格 | 移 → docs/stage1/ |

---

## 2. 表 B —— 归位到 stage2 (具体想法)

| # | 文档 | 现位置 | 归类 | 动作 |
|---|---|---|---|---|
| 1 | 01-INDUSTRY-LANDSCAPE.md | docs/v2-strategy/ | 行业调研 | 移 → docs/stage2/ |
| 2 | 02-VCP-DEEP-COMPARISON.md | docs/v2-strategy/ | 竞品对比 | 移 → docs/stage2/ |
| 3 | 03-EXTREME-PLAN.md | docs/v2-strategy/ | 路线计划 | 移 → docs/stage2/ |
| 4 | 04-CRATE-CONSOLIDATION.md | docs/v2-strategy/ | crate 重组 | 移 → docs/stage2/ |
| 5 | 05-EXECUTION-NOW.md | docs/v2-strategy/ | 执行步骤 | 移 → docs/stage2/ |
| 6 | 06-TUI-UPGRADE-ROADMAP.md | docs/v2-strategy/ | TUI 路线 | 移 → docs/stage2/ |
| 7 | 07-VCP-GAP-UPGRADE-PLAN.md | docs/v2-strategy/ | 差距补弱 | 移 → docs/stage2/ |
| 8 | 17-APEIRETH-VS-VCP-CONSUMER-PLAN.md | docs/ | 竞品/消费计划 | 移 → docs/stage2/ |
| 9 | 18-VCP-BORROW-RETROSPECTIVE.md | docs/ | 借鉴回顾 | 移 → docs/stage2/ |
| 10 | Apeireth-v2.1-Industry-Top-Backend-Roadmap.md | docs/ | 行业后端路线 | 移 → docs/stage2/ |
| 11 | competitive-analysis-2026-08-05.md | docs/ | 竞品分析 | 移 → docs/stage2/ |
| 12 | backend-capabilities.md | docs/ | 能力清单 | 移 → docs/stage2/ |
| 13 | tech-review-2026-08-05.md | docs/ | 技术审查 | 移 → docs/stage2/ |
| 14 | tui-r135-integration-design.md | docs/ | 集成设计 | 移 → docs/stage2/ |
| 15 | 01-07-*.md (7 模块) | docs/architecture-v4-2-r145-modules/ | 模块具体设计 | 移 → docs/stage2/ |
| 16 | 9-organ-integration-blueprint.md | docs/spirit/ | 集成蓝图 | 移 → docs/stage2/ |

---

## 3. 表 C —— 保持不动

| 文档/目录 | 原因 |
|---|---|
| docs/conventions/ (16 规范) | 规范层, 独立存在, 不并入 stage |
| docs/glossary/ (21 术语) | 术语层, 独立存在; 可在 stage1 README 加索引 |
| docs/stage1/inspiration-stage1-2026-07-30.md | 已在 stage1, 原文 LOCKED |
| docs/stage2/ (19 份 decisions) | 已在 stage2, 原文 LOCKED |
| docs/stage3-blueprints/ stage4/ stage5/ stage6/ | 施工层, 保持 6 阶段顺序 |
| docs/omnibus/24-locked-crates.md, r11-baseline.md, stage1-5.md | 历史基线, 不挪 |
| docs/00-R14-START-HERE.md, STRUCTURE-R14.md, CONTEXT-HANDOVER.md, team-onboarding.md, README.md | 交接/索引/入口 |
| docs/sqlite-best-practices-v2.md, observability-naming-cheatsheet.md | 技术规范 (可并入 conventions, 低优先) |
| docs/r14-design/ 里 roadmap/prep/readiness/workspace-prep/review | 周期产物, 可归档, 不急 |

---

## 4. LOCKED 处理 (关键)

1. **stage1/stage2 的 LOCKED 原文一个字不改** —— 只做文件归位 + README 索引更新。
2. **表 A/B 的「移」= 移动文件位置**, 不改正文; git 会保留历史。
3. **新文档 (product-loop-design / 愿景小说 / 哲学杂谈) 是新增**, 归位后作为 stage1 的「产品设计层」, 与既有 LOCKED 灵感并列, 不覆盖。
4. 归位完成后, 更新 stage1/README.md 与 stage2/README.md 的索引表, 标出新归位文档。

---

## 5. 行动清单 (拍板后执行)

1. 执行表 A: 15 项移入 stage1 (含去重 onion-wall)。
2. 执行表 B: 16 项移入 stage2 (含合并 architecture-v4-2-r145-modules 的 7 模块)。
3. 表 C 保持; 清理 v2-strategy/ 空目录, 留一个 README 指向新位置。
4. 更新 stage1/README.md + stage2/README.md 索引。
5. (可选) 表 A #2/#3 从 youyou 桌面移入 docs/stage1/ 时, 桌面留一个链接文件指向新位置。

---

## 6. 待主人拍板项

| # | 拍板项 | 建议 |
|---|---|---|
| 1 | 归位原则 (表 A/B 的分桶) 是否认可 | 认可后即可执行 |
| 2 | 是否现在执行移动, 还是等 codex 当前工作收尾 | 建议等 codex 工作区稳定后再动 (避免与未提交改动冲突) |
| 3 | youyou 桌面两个 .txt 是「移入」还是「复制入 + 桌面保留」 | 建议移入 docs/stage1, 桌面留链接 |
| 4 | v2-strategy/ 目录是「清空留 README」还是「整体改名」 | 建议清空留 README (git 历史可溯) |

---

_End of document._

---

## 附: 执行结果 (2026-08-15)

**状态: 已执行** (主人拍板 "codex 干完了, 你挪吧")。

- ✅ 表 A (15 项) → stage1 完成。
- ✅ 表 B (16 项) → stage2 完成 (含 architecture-v4-2-r145-modules 整目录)。
- ✅ v2-strategy/ 清空留 README + 跳转提示。
- ✅ stage1/README + stage2/README 加「归位新增」表。
- ✅ stage4 整理: 5 核心留根 + 4 顶层/产品 → stage1 + 47 历史 → `_history/` (6 桶)。
- ✅ onion-wall 去重: 两份 MD5 不同, 保留 (stage1 + r14-design 各一份)。
- ✅ spirit/ 残留已归档 → `_archived/spirit/` (2 草稿 + gen_bridge1.py, 后者未被 git 跟踪故保留而非硬删)。
- ✅ r14-design/ 剩余 R14 周期产物已归档 → `_archived/r14-design/` (6 文件)。

**其余 docs 目录 (api / adr / glossary / license / licenses-3rdparty / installation / security / ci / sdk / roadmap / research / desktop / release / versioning / construction / final-check / pages-source / session / audit) 已按主题组织良好, 无需挪动。**
