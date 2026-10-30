# 视觉施工提示词 · 外部方法论研究（2026-10-30）

```
[Document-Meta]
Document:        docs/04-internal/02-research/ui-motion-prompt-research-2026-10-30.md
Version:         1.0
Last-Modified:   2026-10-30
Status:          🟡 研究稿（外部方法论取材；只提炼方法与结构，不逐字复制）
Author:          总工程师（lead）· 前端视觉总攻线
口径真源:        用途 = 优化 01-handoffs 施工提示词 v1.2；许可=各自仓库为准
```

> **用途**：owner 2026-10-30 指定研究两个外部 agent-skill 的提示词，看能否优化我们的施工提示词（"抄话术"）。
> 本稿记录：来源与许可、借用了什么（映射到我们的提示词章节）、明确没借什么、以及诚实边界。
> **纪律**：只提炼**方法与结构**，不逐字复制他人文本；外部原文不导入仓库。

---

## §0 取材方式（2026-10-30 实测）

| 通道 | 实测 | 说明 |
|---|---|---|
| GitHub raw 直连（`curl`） | ✅ 200 | 2026-10-30 直连 `raw.githubusercontent.com` 成功；未走节点池 |
| GitHub API（仓库树） | ✅ 200 | 用于列文件清单（匿名限速，调用次数已省着用） |
| 仓库存档 | ❌ 未入库 | 外部文件不导入仓库（许可 + 体积 + 单一真源纪律） |

---

## §1 来源与许可

### 1.1 来源 A：`plugin87/ux-ui-agent-skills`（设计系统 + 客观门方法论）

- 仓库：<https://github.com/plugin87/ux-ui-agent-skills>
- 许可：**MIT**（仓库 README 徽章声明；以该仓库 `LICENSE` 原件为准）
- 抓取的提示词文件（只读）：
  - `.claude/skills/design-doctrine/SKILL.md`（验证协议 / 五条不可谈判 / 反 slop / 决策框架）
  - `.claude/skills/design-screen/SKILL.md`（设计前判断 Brief Inference / 管线 / 交付审计）
  - `.claude/skills/design-component/SKILL.md`（八状态 / token 映射 / 状态矩阵 / 动效 token 化）
  - `.claude/skills/critique/SKILL.md`、`.claude/agents/design-critic.md`（对抗性评审 pass 格式）
  - `.claude/rules/{accessibility,components,typography-and-spacing,tokens-and-color}.md`（门禁细则）

### 1.2 来源 B：`feitangyuan/onetake`（连续动效方法论）

- 仓库：<https://github.com/feitangyuan/onetake>
- 许可：**PolyForm Noncommercial 1.0.0**（仓库 README/`LICENSE` 声明）
- 抓取的提示词文件（只读）：`SKILL.md`（一镜到底 / 节奏 / 连续性 / 动效硬规则 / 验证 oracle）

### 1.3 许可合规

- **MIT**：允许改编与再分发，需保留版权与许可声明 ⇒ 本稿记录来源即可；提示词为中文改写，不搬原文。
- **PolyForm Noncommercial**：仅取**概念与术语**（连贯性 / 节奏 / 静止段 / 因果可见），**不逐字复制**其文本；Apeireth 为开源免费个人工具（ADR-0013），但为稳妥仍按"只取方法"处理。
- 若未来要复用其**代码/长文本**，需重新核许可并单独记录。

---

## §2 借用映射（来源 → 我们提示词 v1.2 的落点）

| # | 来源方法 | 我们的落点 | 改写方式 |
|---|---|---|---|
| 1 | 设计前判断（Brief Inference / Taste preflight：领域/观众/情绪/动效深度/布局家族/参考锚点） | 提示词 §〇 + 简报 §9 末 | 中文化为"设计前判断"块，要求**写出来再动手** |
| 2 | 三个概念方向（中心思想不同）+ 禁止默认模板 | 提示词 §〇 | 保留"三案/排除什么/禁止默认指挥中心模板" |
| 3 | 验证协议：跑门不口称 / 未测写"未验证" / 渲染后再看 / 门通过 ≠ 审美 | 提示词 §九 + 简报 §8 | 适配成 HTML 原型的可执行检查清单（对比度/键盘/reduced-motion/窗口宽度/状态/mock） |
| 4 | 八状态（default/hover/focus/active/disabled/loading/error/selected） | 提示词硬约束 13 + D3 状态矩阵 | 与 D3 现有清单合并，补 selected |
| 5 | 反 slop 清单（banned defaults + 评审 lens） | 提示词硬约束 15 + 简报 §4 反范式 | 去掉移动端专属项，保留通用项（等重卡片/居中/假标签/emoji/单侧彩边…） |
| 6 | 评审 pass 格式（判定 / 三条打回理由 / 证据 / 最多两条真好 / 未能判断项） | 提示词 §十 | 中文模板化，要求"审过再交" |
| 7 | token by intent / 一个主题一个真源 / 输出完整性 | 提示词硬约束 11–12 + 交付物 | 与"金色=系统信号"纪律合并 |
| 8 | 决策层级（用户需求 > 无障碍 > 一致性 > 美学 > DX） | 提示词硬约束 17 | 原样中文表述 |
| 9 | onetake 动效纪律（连续性/节奏/静止段/因果可见/动效 token 化） | 提示词 §六 + 简报 §4.5 | 从"视频一镜到底"转译为"界面状态过渡连续、不幻灯片" |
| 10 | onetake "每条硬规则都来自一次被驳回"的写法 | 提示词 §六 标题、§〇 硬规则 | 保留"每条都来自一次被打回"的权威感（我们的版本引用我们自己的验收语） |

---

## §3 明确**未**采用 / 已适配的项

| 外部项 | 处理 | 理由 |
|---|---|---|
| 移动优先（mobile-first） | 未采用；改为"最小窗口宽度无横向溢出" | 产品是桌面窗口，不是移动 App |
| 视频渲染 oracle / 视频流水线 | 未采用；只借"可测自检"思路 | 我们交付 HTML 设计原型，不是视频 |
| 具体图标库 / 具体品牌 look 预设 | 未采用 | 我们有自己的图标族与明暗双模式设计系统 |
| 138/2144 套设计系统库、DTCG token 文件 | 未采用 | 我们是单一产品设计系统，不是通用库 |
| 原生 `<select>` 禁令 / 单侧彩边等具体规则 | 部分采用（单侧彩边进反 slop） | 与我们的视觉语言不冲突；但不改变"控件族由 L2 决定" |
| 外部原文文本 | **未复制** | 许可与单一真源纪律；提示词为中文改写 |

---

## §4 本次改动（提示词 v1.1 → v1.2）

1. **新增 §〇 设计前判断 + 三概念方向**（写出来再动手；禁止默认模板）；
2. **新增 §六 动效纪律**（六条；连续/节奏/静止/因果/无源装饰禁/动效 token 化）；
3. **新增 §九 交付前验证门**（客观可测；未测写"未验证"；门通过 ≠ 审美）；
4. **新增 §十 交付前评审 pass**（自我否定模板）；
5. **扩充 §八 硬约束**：语义 token / 单一真源 / 八状态 / 唯一信号色 / 反 slop 清单 / 内容压力 / 决策层级 / 输出完整性；
6. **试金石补"一眼一个落点"**；
7. 简报同步：§4.5 动效纪律、§7 硬约束 11–17、§8 验证门与评审 pass、§10 方法论引用。

---

## §5 诚实边界

- 本稿为**方法提炼**，未逐行记录外部原文；引用链接见 §6。
- 外部仓库可能更新；许可以各自仓库当前 `LICENSE` 为准（本次核对日期 2026-10-30）。
- 未执行任何外部脚本/代码；未把外部文件导入仓库。
- "话术"借鉴只影响提示词表达与检查项，不改变 Apeireth 的范式/架构决策（见 [02-PARADIGM.md](../../design/02-PARADIGM.md)）。

---

## §6 出处

| 事实 | 出处 |
|---|---|
| 来源 A（设计系统 + 客观门方法论） | <https://github.com/plugin87/ux-ui-agent-skills>（MIT） |
| 来源 B（连续动效方法论） | <https://github.com/feitangyuan/onetake>（PolyForm Noncommercial 1.0.0） |
| 本次更新的提示词 | [frontend-visual-build-prompt-2026-10-28.md](../01-handoffs/frontend-visual-build-prompt-2026-10-28.md)（v1.2） |
| 本次更新的简报 | [frontend-visual-build-brief-2026-10-28.md](../01-handoffs/frontend-visual-build-brief-2026-10-28.md)（v1.2） |
