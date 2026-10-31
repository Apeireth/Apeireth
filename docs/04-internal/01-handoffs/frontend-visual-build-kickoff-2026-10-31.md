# K3.1 前端重构 · 交接入口（施工前协议）

```
[Document-Meta]
Document:        docs/04-internal/01-handoffs/frontend-visual-build-kickoff-2026-10-31.md
Version:         1.1
Last-Modified:   2026-10-31
Status:          🟡 施工输入（交给施工方的第一份文件）
Author:          总工程师（lead）· 前端视觉总攻线
口径真源:        创作简报 = ./frontend-visual-build-prompt-2026-10-28.md（v3.4）
                 施工规格 = ./frontend-visual-build-spec-2026-10-31.md（v1.4）
                 交付说明 = ./frontend-visual-build-brief-2026-10-28.md（v3.4）
                 设计范式 = ../../design/02-PARADIGM.md（v1.0，已签署）
用法:            本文件放在交接包最前面；与四份真源文档一起交给施工方。
```

> **读法**：本文件是入口协议，不替代设计文档正文。先读本文件 → 再按 §1 顺序读真源。

---

## 0. 施工模式（开工前先声明）

```text
当前阶段：DESIGN_ONLY
切换条件：owner 明确批准 T0 后，才可进入 PRODUCTION_IMPLEMENTATION
```

- **DESIGN_ONLY**：只在 `deliverables/` 产出可运行设计原型、令牌、屏幕、说明、证据与检查点；**不修改生产仓库**。
- **PRODUCTION_IMPLEMENTATION**（后续阶段）：在**自研 UI 引擎前端工程**内实现已批准设计；保留现有运行时契约；每次修改运行相关检查与测试。
  - **切换前提**：引擎产品工程的最小垂直切片已验证（Signal → VNode/RenderPlan → DOM mount/update/unmount → event → focus/IME → dispose，含 wasm/WebView2 验证）；**当前未满足**，所以现在只能是 DESIGN_ONLY。
  - 具体仓库路径与工程基线由引擎线在切换前指定；**本文件不预设路径**。
  - 现有 `frontend/companion-desktop`（旧前端）只是**工程背景**，不是新前端的实现目标。

## 1. 真源与优先级

**读序**（按此顺序读，只把这四份当设计真源）：

1. 本交接入口（协议/模式/契约/证据）；
2. 创作简报 v3.4：主任务、硬底线、交付物、验收门；
3. 施工规格 v1.4：屏幕、字段、交互、状态参考；
4. 交付说明 v3.4：流程、检查点、提交格式；
5. 设计范式 v1.0（已签署）：产品原则、三空间、创意边界。

**冲突优先级**：

```text
本交接包 > 设计范式 v1.0 > 仓库工程纪律 > 仓库历史视觉文档
```

**不作为施工输入**：

- `0-Astra评审-v1.1.md`、`5-旧范式可汲取清单-未采纳-v0.2.md`：历史记录，仅供人类追溯；
- 仓库中的旧开工提示词、旧主题档、旧场景实现（如 `k3-team-kickoff-prompt.md`、`theme-starship-tokens.md`、`tokens.css`、`scene/*`）：只提供工程背景，**不提供产品方向**；与新交接包冲突时一律以新交接包为准；
- 旧范式的"他 / presence 中心 / 旧场景 / 旧金色语义"**不得回流**新范式；不要为了兼容旧视觉把两套范式拼接。

## 2. 开工前必须先交：SOURCE-OF-TRUTH.md

没有这份文件，不开始 T0。它至少包含：

```text
1. 实际读取的文件与版本
2. 发现的冲突（列出每条冲突）
3. 每个冲突采用的规则与理由
4. 当前施工模式（DESIGN_ONLY / PRODUCTION_IMPLEMENTATION）
5. 仍需 owner 决定的事项
```

## 3. 首轮交付冻结（D2 屏幕矩阵）

D2 = 四类屏幕 × 明暗各一张 = **8 张**，不要做成重复截图：

| screen_id | 内容 | 主题 |
|---|---|---|
| `workspace` | 工作空间：工作画布 + AI 协作栏 | light / dark |
| `application` | 应用空间：Starport + 当前应用 + 共享上下文 | light / dark |
| `control-tower` | 控制塔：遥测、Agent、检查器、事件日志 | light / dark |
| `holographic-base` | 全息基地焦点视图：基地/轨迹切换 | light / dark |

每张屏必须声明：`screen_id` / `state_id` / `theme` / `viewport` / `source` / 证据路径。
`state_id` 至少覆盖：`empty / active / selected / loading / awaiting-approval / error / unavailable`。

`application` 主屏必须能切换**文档、图像、浏览器、终端**四个工作流；D2b 是细化屏，不是应用空间的替代品。

## 4. 数据与交互契约

**数据来源标签**（任何遥测、Agent 状态、工具结果、AI 建议、文件版本、审批都必须属于其中一种，禁止无标签的"看起来像真数据"）：

```text
LIVE        当前由真实 API / 事件流提供，并显示来源与时间
MOCK        为演示设计而生成，显示 scenario_id
STALE       最近一次真实值，显示最后更新时间
UNAVAILABLE 能力存在但当前不可取值
NOT_WIRED   设计形状已存在，尚未接入后端
```

**scenarios/**（至少定义）：空工作 / 文档建议等待批准 / 图像建议已应用可回退 / Agent 运行中 / 工具失败 / Provider 未接入 / 文件冲突。

**AI 建议生命周期**（必须可见）：

```text
idle → proposing → preview → awaiting-approval → applying
→ applied | rejected | failed | reverted
```

每条建议至少记录：`suggestion_id / work_id / artifact_id / anchor_type(selection|paragraph|layer|row|code-range) / anchor_id / before_version / after_version / trace_id / permission_scope / source`。
"接受"必须产生新版本或明确失败态；"回退"指向版本节点；局部接受要说明剩余差异如何处理。

**WorkspaceContext**（三空间共享最小上下文）：

```text
WorkspaceContext {
  work_id, artifact_id, artifact_type, selection, version_id,
  active_app, focused_agent_id, permission_scope, trace_id,
  return_route, return_focus
}
```

首轮必须演示一条完整路径：

```text
工作空间选择锚点 → 打开应用空间 → AI 协作栏识别同一锚点 → 查看差异
→ 接受/拒绝 → 返回原工作并恢复焦点
```

**控制塔进入/退出**：

```text
进入：保存 route + work_id + artifact_id + selection + focused_agent + 滚动位置。
退出：恢复进入前的工作画布、协作栏展开状态、滚动位置、键盘焦点。
首次引导：非阻塞 coachmark，不增加第二次确认，不改变"一次交互进入"的规则。
```

控制塔只允许"看 + 选"。基地上的右键菜单只能打开检查器、待签卡或命令面；**不能在基地本体上直接提交写操作**。

## 5. 检查点与证据

每个检查点都更新：

```text
PROGRESS.md    当前进度
MANIFEST.md    文件清单（文件 → D/屏/组件/状态）
EVIDENCE.md    每项门的证据
DECISIONS.md   本轮做过的取舍与理由
checkpoints/vN/  快照（只增不改）
```

`EVIDENCE.md` 每项至少写：`artifact_id / viewport / DPI / theme / state_id / source / 复现步骤 / 截图或录屏路径 / 实际结果 / 未验证项`。

未测内容写"未验证"，不得写"通过"。额度将尽时停在完整检查点，并写明下一步文件、状态和**唯一阻塞原因**。

## 6. 设计自由边界

- **必须保留的是语义槽**：核心、记忆、编排、能力、治理、接入、事件流、选中、告警、可追溯性。
- **可自由替换的是视觉载体**：城市、机械、植物、几何体、纸面、光学结构都可以；具体视觉语言由施工方创造。
- **全息基地**是功能角色与产品命名；**城市是 owner 已选的初始载体**——你可以在城市载体内部自由创造更好的表达；若要换载体，在 T0 提出方案并由 owner 决定。
- **金色是唯一高饱和系统信号色**；成功/警告/危险/信息可以存在，但必须**低饱和、只承担语义**，不得作为装饰或品牌色；工作画布内容色、图片原色、应用内部色不受此限，但不能冒充系统信号。
- **首轮主题冻结为 light / dark 两套**；仓库历史上多主题体系只作兼容参考，不要求首轮同时设计。

## 7. 响应式与命令条行为

窄窗优先级：

```text
1280：左右栏可收纳，中央工作画布保持可用
1440：三栏基准
1920：增加留白，不把面板拉到失去层级
更窄：右栏变抽屉、左栏变 rail、应用 chrome 允许折叠
任何尺寸：不允许用横向滚动承载主工作流
```

底部命令条必须覆盖状态：空闲 / 聚焦 / 输入中 / 有附件 / 发送中 / 可中断 / 错误 / 权限不足；
Enter 发送、Shift+Enter 换行、Esc 取消或收起、Tab 在附件与建议间移动；拖入文件显示文件名/类型/大小/来源/移除；发送后任务出现在工作画布与 AI 协作栏。

## 8. 性能测量协议

```text
目标环境：Windows WebView2，DPI 100/125/150
基准窗口：1440×900（默认窗口）
正常交互目标 60fps；低性能降级不改变状态语义
窗口失焦：暂停环境层与基地渲染
prefers-reduced-motion：停用旋转、视差、无源呼吸
D5 记录测量方式与实际结果；未测写"未验证"
```

## 9. 首次回复格式

```text
1. 已读取文件与版本
2. 冲突及采用规则
3. 当前施工模式
4. T0 计划与预计交付文件
5. 尚待 owner 决定的事项
6. SOURCE-OF-TRUTH.md 路径
```
