# apeireth-context-fold

**R144** — 上下文折叠 (Context Folding)

## 职责

长上下文智能压缩: 把超出 token 预算的 message history 折叠为摘要 + 关键实体保留.

## 核心能力

- 摘要生成 (LLM 调用)
- 实体提取与保留
- 关键引用保留
- 折叠审计 (折叠了什么, 保留了什么)

## 借鉴

claude-mem / Letta 的两级 memory 设计.

## 0 假装

✅ 摘要结构化输出 | ✅ 折叠前后可比对 | 17 单元测试
