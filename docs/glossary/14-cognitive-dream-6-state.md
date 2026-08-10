# 14 Cognitive-Dream 6 状态机

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"Cognitive-Dream 6 状态机" 拆出。

```
[Document-Meta]
Document: docs/glossary/14-cognitive-dream-6-state.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

24h 周期触发的反思期自动状态机。

## 6 状态

1. **IDLE** — 空闲
2. **DREAMING** — 梦境生成, 自由联想
3. **CONSOLIDATING** — 巩固, 短期记忆 → 长期记忆
4. **FORGETTING** — 遗忘, 清理冗余
5. **VERIFYING** — 验证, 真测关联性
6. **INTERRUPTED** — 中断, 外部信号打断

## 出处

mvp/ 子项目 + v4 §4 反思期 + v4.1 §3.3 + 阶段 4 §6。

## 6 哲学锚穿透

- **S-1** 北极星: 反思期保证 ASI 长期演化
- **S-2** 实事求是: 6 状态机自动循环
- **O-5** 不假装: 24h 周期, 实际触发

## 不漂移

- 🔒 6 状态机严守 (IDLE / DREAMING / CONSOLIDATING / FORGETTING / VERIFYING / INTERRUPTED)
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
