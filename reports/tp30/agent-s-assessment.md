# Agent-S 评估

## 机制（What it does）

- 核心功能：电脑使用 Agent（computer-use agent），AI 操作 GUI 自动化（鼠标/键盘/截图），类似 OpenAI Operator
- 解决什么问题：传统 RPA（UiPath）贵且僵化 → AI 驱动 GUI 自动化，agent 看截图决定下一步动作
- 关键技术：
  - 视觉理解（截图 → 元素识别）
  - 动作执行（PyAutoGUI / xdotool）
  - 任务分解 + 多步规划
  - 安全边界（白名单 / 危险操作拦截）

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-tool-browser`（浏览器自动化）
  - `apeireth-action`（行动器官：改变环境 + 工具执行）
  - `apeireth-tool-approval`（R173 ApprovalBridge，审批流）
  - `OpenHands`（已 clone 在 `research/source/OpenHands/`，更成熟的电脑使用 agent）
  - `morphic`（已 clone，浏览器自动化 agent）
  - `hermes-agent-rs`（已 clone，Rust agent）
- 差异化优势：
  - Agent-S 是「通用 GUI」，APEIRETH `apeireth-tool-browser` 是「浏览器专属」，场景略错位
  - Agent-S 是 Python，APEIRETH 是 Rust（但已有 Rust 端 browser 工具）
  - Agent-S 涉及安全敏感操作（GUI 全权），APEIRETH `apeireth-guard` (R173 Privacy Guard) 有边界
- 可借鉴：
  - **截图理解 + 动作执行闭环**：可参考到 `apeireth-perception`（已在做感知器官），把 GUI 截图作为感知输入
  - **任务分解 + 多步规划**：参考 Agent-S 的任务分解逻辑到 `apeireth-action` 的 `ActionPlan`
  - **安全边界**：Agent-S 的「白名单 / 危险操作拦截」可参考到 `apeireth-tool-approval` 已有机制

## 吸收建议（Action items）

- P0 立即做：**不动 Agent-S 本体**（Python 不兼容 Rust）。
- P1 评估后做：
  1. 参考 Agent-S / OpenHands 的任务分解模式到 `apeireth-action` 的 `ActionPlan` 抽象
  2. 主人电脑使用场景：若主人想让 APEIRETH 操作 Windows GUI（如自动填表），可考虑：
     - 用 `enigo`（Rust crate）做鼠标键盘
     - 用 `xcap` / `scrap` 做截图
     - 接 `apeireth-perception` 做元素识别（当前 perception 是事件流 + 麦克风，待扩）
- P2 长期调研：观察 OpenHands 进展（OpenHands 更成熟，参考价值更高）。
- 不做（重复 / 价值低）：APEIRETH 当前主路径是「伴侣 + 工具调用」，GUI 自动化是边缘场景。

## 0 装 PASS 标注

- 真用：**否**（未实测 Agent-S 本体）
- 源：**未下载实测**。`research/source/` 无 Agent-S 源码；本评估基于 GitHub README + 同类项目（OpenHands / morphic / hermes-agent-rs 已 clone）对照推理。
- 未调研不写结论：Agent-S 的具体视觉理解算法 / 任务分解模型 / 安全边界实现均为推理判断。如需落地建议，必须先实测 Agent-S + 在 Rust 端做 POC。
- 同类参考优先级：**OpenHands > Agent-S**（OpenHands 已在 source，更成熟），Agent-S 可降级为观察项。