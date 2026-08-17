# taipy 评估

## 机制（What it does）

- 核心功能：Python 低代码 dashboard / 数据应用构建（data-driven web apps）
- 解决什么问题：业务人员想用 Python 快速搭 dashboard → Streamlit/Gradio 受限 → taipy 提供更结构化的方案
- 关键技术：
  - 声明式 UI（Markdown-like 语法）
  - 数据流编排（scenario management）
  - 内置图表 + 表单组件
  - 与 Pandas/NumPy 深度集成

## 对照（How it relates to APEIRETH）

- 相似能力：
  - `apeireth-tui`（TUI 套件）
  - 缺失：APEIRETH 无 Web dashboard 套件
  - `apeireth-action`（含 StructuredOutput，但非 dashboard）
- 差异化优势：
  - taipy 是 Python Web dashboard，APEIRETH 当前主战场是终端 + 命令行
  - taipy 面向业务人员，APEIRETH 当前面向开发者
- 可借鉴：
  - **数据流编排思想**：taipy 的 scenario management（参数 + 数据双向绑定）可参考到 `apeireth-context-fold`（上下文折叠）
  - **声明式 UI**：若 APEIRETH 后期要做 Web 端点（`apeireth-companion` 已有 OpenAI 兼容），可参考 taipy 的声明式范式

## 吸收建议（Action items）

- P0 立即做：**不动**。Web dashboard 不是 APEIRETH 主战场。
- P1 评估后做：若主人想做 Web 端「主人日记 / 记忆浏览」，可考虑：
  - 用 `axum` + 类似 taipy 声明式 UI 范式在 `apeireth-companion` 上加 Web 端
  - 但当前 `apeireth-tui` 已满足主路径，Web 是次要
- P2 长期调研：列入观察项。
- 不做（重复 / 价值低）：taipy 是 Python，与 APEIRETH Rust 栈不兼容；直接 fork 价值低。

## 0 装 PASS 标注

- 真用：**否**（未实测）
- 源：**未下载实测**。`research/source/` 无 taipy 源码；本评估基于 GitHub README 公开信息 + APEIRETH 现状推理。
- 未调研不写结论：taipy 的具体 API 设计 / 与 axum 兼容性均为推理判断。如需落地建议，必须先实测 + 在 `apeireth-companion` 上做 POC。