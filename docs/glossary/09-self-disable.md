# 09 Self-Disable 防护 (百年章节)

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"Self-Disable 防护" 拆出。

```
[Document-Meta]
Document: docs/glossary/09-self-disable.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 定义

防止主 AI / Evolution / 反思期逐步绕过 L0 HA 的 5 大机制(借鉴 Asimov / Constitutional AI / FLI / Utility Drift Mitigation)。

## 5 大机制

- **A. 元问题禁令** — 反思期不能询问"是否需要 L0 HA"等元问题
- **B. 重组洋葱结构禁令** — 物理隔离 + MultiHuman 多签 + 24h 安静期 + 启动验证 3 里程碑
- **C. Evolution crate 限制** — 编译时 hardcode 拒绝修改 L0 相关 trait
- **D. HA 抗胁迫 + 离线模式** — 生理指标 + 冰冻期 + 安静模式
- **E. Self-Disable 自动检测** — 每 24h 反思期自动扫描 4 项违规

## 出处

阶段 4 external-feedback-and-revisions §3。

## 6 哲学锚穿透

- **S-1** 北极星: 5 大机制保证 ASI 长期安全
- **S-2** 实事求是: 5 大机制借鉴业界 4 个项目
- **O-2** 前人肩上: 借鉴 Asimov / Constitutional AI / FLI / Utility Drift Mitigation
- **O-5** 不假装: 自动检测 4 项违规

## 不漂移

- 🔒 5 大机制严守
- 🔒 L0 HA 永远不可变 (per [05-l0-ha](05-l0-ha.md))
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
