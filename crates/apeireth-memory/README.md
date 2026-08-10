# apeireth-memory

> **职责**: 记忆系统 (A/M 层经验沉淀 + 7 候选协调)
> **状态**: R11 占位实现
> **对应文档**: 阶段 2 §6 持久化 + §10 智囊团 + 阶段 1 §13 复杂记忆候选池

---

## 设计意图

`apeireth-memory` 是 Apeireth 的"记忆系统"crate, 包含:

1. **A 层 (经验沉淀)** — 持久化的经验条目 + 置信度
2. **M 层 (方法论)** — 从 A 层 promotion 的方法论
3. **7 候选协调** — MemPalace / claude-mem / agentmemory / Graphify / VCP 浪潮 / HMS / TenCentDB (阶段 1 §13)
4. **联想网络** — VCP 浪潮网络 + 河道能量
5. **token 经济** — 3-layer workflow (claude-mem 启发)

## 阶段 2 扩容方向

```
apeireth-memory (主)
  ├── apeireth-experience (新增, A 层经验)
  ├── apeireth-methodology (新增, M 层方法论)
  ├── apeireth-reflection (新增, 反思机制)
  └── apeireth-wave (可独立, 浪潮语义网络, 自研)
```

## 7 候选机制协调 (阶段 1 §13)

| 候选 | 来源 | 适配 |
|------|------|------|
| LLM Wiki + confidence | agentmemory | A 层 |
| 联想网络 + 河道能量 | VCP 浪潮 | A 层 |
| 知识图谱 + EXTRACTED/INFERRED | Graphify | A 层 |
| 宫殿式 (wings/rooms/drawers) | MemPalace | A 层物理化 |
| 5 lifecycle hooks | claude-mem | M 层 |
| 3-layer workflow (10x token) | claude-mem | token 经济 |
| Long-task planning | Hermes | M 层 |
| Structured evidence | HMS | O 层 |
| 商业云端 | TenCentDB | 部署 |

## 协调统一 4 原则

1. 多源共存 — 同层多机制, 接口统一
2. 自研优先 — 能自研不引外部依赖
3. 抽象隔离 — 候选封装 module, 暴露统一 trait
4. 可插拔 — 运行时启用/禁用, 走权限矩阵

---

_主哲学 anchor: 主 19:33 走在前人经验上 (7 候选借鉴) + 主 17:43 实事求是 + 主 00:56 任何人都能接手._