# 视觉施工提示词 · 外部方法论研究（2026-10-30）

```
[Document-Meta]
Document:        docs/04-internal/02-research/ui-motion-prompt-research-2026-10-30.md
Version:         1.1
Last-Modified:   2026-10-30
Status:          🟡 研究稿（外部方法论取材；只提炼方法与结构，不逐字复制）
Author:          总工程师（lead）· 前端视觉总攻线
口径真源:        用途 = 优化 01-handoffs 施工提示词（v1.2 / v1.3）；许可=各自仓库为准
变更:            v1.0：三源（设计系统方法论 / 连续动效 / 视频 treatment 写作法）
                 v1.1（2026-10-30）：按 owner 方向新增来源 C（pdoom-video）与 treatment 写作法映射
```

> **用途**：owner 2026-10-30 指定研究外部 agent-skill 与作品项目的提示词/文档，看能否优化我们的施工提示词（"抄话术"）。
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

### 1.3 来源 C：`mexicat/pdoom-video`（视觉 treatment / style bible 写作法）

- 仓库：<https://github.com/mexicat/pdoom-video>
- 许可：**MIT**（仓库 `LICENSE`：Copyright (c) 2026 Giacomo Magnanini）
- 抓取的文档（只读）：`README.md`、`docs/TREATMENT.md`、`docs/ENGINE.md`
- 价值：`TREATMENT.md` 是"**读得出画面的处理稿**"范本——一句话构想 / tone 规则 / 调色板与发光纪律 /
  字体家族与角色 / 贯穿母题 / **plate-by-plate 逐段处理**（每段写清元素、动作、材质、数字、镜头与节奏）/
  修订记录（Revision 2/3/4/5 记录被否项与原因）。

### 1.4 许可合规

- **MIT**（来源 A、C）：允许改编与再分发，需保留版权与许可声明 ⇒ 本稿记录来源即可；提示词为中文改写，不搬原文。
- **PolyForm Noncommercial**（来源 B）：仅取**概念与术语**（连贯性 / 节奏 / 静止段 / 因果可见），**不逐字复制**其文本。
- 若未来要复用其**代码/长文本**，需重新核许可并单独记录。

---

## §2 借用映射（来源 → 我们提示词 v1.3 的落点）

| # | 来源方法 | 我们的落点 | 改写方式 |
|---|---|---|---|
| 1 | 设计前判断（Brief Inference / Taste preflight：领域/观众/情绪/动效深度/布局家族/参考锚点） | 提示词 §九 + 简报 §3.5 | 中文化为"设计前判断"块，要求**写出来再动手** |
| 2 | 三个概念方向（中心思想不同）+ 禁止默认模板 | 提示词 §九 | 保留"三案/排除什么/禁止默认指挥中心模板" |
| 3 | 验证协议：跑门不口称 / 未测写"未验证" / 渲染后再看 / 门通过 ≠ 审美 | 提示词 §十三 + 简报 §8 | 适配成 HTML 原型的可执行检查清单（对比度/键盘/reduced-motion/窗口宽度/状态/mock） |
| 4 | 八状态（default/hover/focus/active/disabled/loading/error/selected） | 提示词硬约束 13 + D3 状态矩阵 | 与 D3 现有清单合并，补 selected |
| 5 | 反 slop 清单（banned defaults + 评审 lens） | 提示词硬约束 15 + 简报 §4 反范式 | 去掉移动端专属项，保留通用项（等重卡片/居中/假标签/emoji/单侧彩边…） |
| 6 | 评审 pass 格式（判定 / 三条打回理由 / 证据 / 最多两条真好 / 未能判断项） | 提示词 §十四 | 中文模板化，要求"审过再交" |
| 7 | token by intent / 一个主题一个真源 / 输出完整性 | 提示词硬约束 11–12 + 交付物 | 与"金色=系统信号"纪律合并 |
| 8 | 决策层级（用户需求 > 无障碍 > 一致性 > 美学 > DX） | 提示词硬约束 17 | 原样中文表述 |
| 9 | onetake 动效纪律（连续性/节奏/静止段/因果可见/动效 token 化） | 提示词 §八 + 简报 §4.5 | 从"视频一镜到底"转译为"界面状态过渡连续、不幻灯片" |
| 10 | onetake "每条硬规则都来自一次被驳回"的写法 | 提示词 §八 标题、§〇 硬规则 | 保留"每条都来自一次被打回"的权威感 |
| 11 | **pdoom treatment 写法**（一句话构想 / tone / 调色板与发光纪律 / 字体家族与角色 / 贯穿母题 / plate-by-plate / 修订记录） | 提示词 §三 目标画面速写、§五 风格圣经、§十 T0 处理稿；简报 §3.5、§6、§7.18–23 | 提炼为"可画出来的描述"标准 + style bible 七节 + T0 模板；不复刻其音乐视频机制 |
| 12 | **"修订记录"是资产**（Revision 2/3/4/5 写明被否项与原因） | 提示词 §5.7 + D5 修订记录 | 中文化为"owner 驳回的项要记录，后续不再犯" |
| 13 | **"每段处理稿要能被画出来"**（元素/动作/材质/数字/镜头） | 提示词 §十 T0 通过标准 | 我们的标准：另一个人照着这段能画出来 |

---

## §3 明确**未**采用 / 已适配的项

| 外部项 | 处理 | 理由 |
|---|---|---|
| 移动优先（mobile-first） | 未采用；改为"最小窗口宽度无横向溢出" | 产品是桌面窗口，不是移动 App |
| 视频渲染 oracle / 视频流水线 | 未采用；只借"可测自检"思路 | 我们交付 HTML 设计原型，不是视频 |
| 音乐视频机制（karaoke 同步 / BPM 网格 / 逐帧确定性渲染） | 未采用 | 与产品界面无关；只借 treatment 写作法 |
| 具体图标库 / 具体品牌 look 预设 | 未采用 | 我们有自己的图标族与明暗双模式设计系统 |
| 138/2144 套设计系统库、DTCG token 文件 | 未采用 | 我们是单一产品设计系统，不是通用库 |
| 原生 `<select>` 禁令 / 单侧彩边等具体规则 | 部分采用（单侧彩边进反 slop） | 与我们的视觉语言不冲突；但不改变"控件族由 L2 决定" |
| 外部原文文本 | **未复制** | 许可与单一真源纪律；提示词为中文改写 |

---

## §4 本次改动（提示词 v1.2 → v1.3）

1. **新增 §〇「怎么用」**：先读目标画面速写 → 先交 T0 处理稿 → owner 说"行"才生产；
2. **新增 §三「目标画面速写」**：三屏各一段"读得出画面"的描述 + 五母题（系统信号线/点云节点/等宽微标签/发丝网格/事件行高亮）；
3. **新增 §五「风格圣经」**：色彩与发光纪律、字体三系、布局与材质、母题、语气、原创性与反 AI 感、修订记录；
4. **新增 §十「T0 处理稿」模板**：一句话构想 + 3 概念方向 + style bible + 三屏 plate 处理稿 + 映射表 + 不确定项；通过标准 = owner 读得出画面；
5. **新增 §九「设计前判断」**（从旧 §〇 移正）；
6. **D5 加「修订记录」**；交付优先级加 T0；
7. 简报同步：§3.5 T0 前置门、§6 T0 说明、§7 硬约束 18–23（发光纪律/字体三系/母题/语气/原创性/修订记录）、§8 验收加 T0 与画面可读性、§10 方法论引用扩展。

---

## §5 诚实边界

- 本稿为**方法提炼**，未逐行记录外部原文；引用链接见 §6。
- 外部仓库可能更新；许可以各自仓库当前 `LICENSE` 为准（本次核对日期 2026-10-30）。
- 未执行任何外部脚本/代码；未把外部文件导入仓库。
- "话术"借鉴只影响提示词表达与检查项，不改变 Apeireth 的范式/架构决策（见 [02-PARADIGM.md](../../design/02-PARADIGM.md)）。
- 目标画面速写是**方向锚点**，不是最终设计；owner 已明确"提示词要先让人能想象出画面"。

---

## §6 出处

| 事实 | 出处 |
|---|---|
| 来源 A（设计系统 + 客观门方法论） | <https://github.com/plugin87/ux-ui-agent-skills>（MIT） |
| 来源 B（连续动效方法论） | <https://github.com/feitangyuan/onetake>（PolyForm Noncommercial 1.0.0） |
| 来源 C（treatment / style bible 写作法） | <https://github.com/mexicat/pdoom-video>（MIT，© 2026 Giacomo Magnanini） |
| 本次更新的提示词 | [frontend-visual-build-prompt-2026-10-28.md](../01-handoffs/frontend-visual-build-prompt-2026-10-28.md)（v1.3） |
| 本次更新的简报 | [frontend-visual-build-brief-2026-10-28.md](../01-handoffs/frontend-visual-build-brief-2026-10-28.md)（v1.3） |
