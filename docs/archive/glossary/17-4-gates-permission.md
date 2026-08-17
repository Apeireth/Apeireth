# 17 6 重守门 (v6 修正, R125 B4 升)

> **R119-3a-2 Mavis 重建 (2026-08-10)**: 从 GLOSSARY.md §"4 重守门(v5 修正)" 拆出。
> **R125 B4 升 v6 (2026-08-10 16:55, Mavis 自主, 主人 16:31 最高权限授权)**: 5 重 → 6 重, 加 Colang DSL 守门 (R125-5 NVIDIA Guardrails 借鉴触发). v5 (4 重 + 权限发放) 实质保留.

```
[Document-Meta]
Document: docs/glossary/17-4-gates-permission.md
Version: Manual-Rev-L + Fix-17 + R125-B4
R-Cycle: R125-B4
Last-Modified: 2026-08-10 (R125 B4 16:55 升 v6)
Status: 🟢 活跃 (v6 6 重, R125-5 实施时落地)
```

## 定义 (v6 修正, R125 B4 升 6 重)

主人 8/10 16:31 最高权限授权 + 8/10 16:55 R125-5 NVIDIA Guardrails 借鉴, **6 重守门嵌套结构** (从 v5 4 重 + 权限发放 → v6 5 重 + 权限发放 + Colang DSL):

## 6 重守门 (嵌套结构, 从内到外)

1. **编译时 hardcode** (内层, **原则洋葱整体**编译时拒绝, 不只是 12 键)
2. **运行时拦截** (中间层, 所有决策前 async trait check)
3. **物理隔离** (外层, 重大修改需物理访问 + 物理多签)
4. **反思期审计** (外层, **守护越权检查**, 不与生命力反思混淆)
5. **多 AI 一致** (外层, 3 个不同 LLM 独立检查, R125-14 superpowers Skill 触发)
6. **Colang DSL 守门** (新加, R125-5 NVIDIA Guardrails 借鉴, DSL 表达"什么操作允许/禁止")

## 权限发放 (独立机制, 不是守门)

- **多 AI 一致** = apeireth-council 智囊团审议(7 强制 + 动态专家, 按风险触发)
- **公式** = V0.5 v2 24 维权重公式 (v4.1 §13 提议), R125 末 B3 升 25 维
- **人类决策** = L0 HA 真实人类批准
- **风险分级输出** = critical 7 / high 5 / medium 3 / low 1 / info 0
- **守门 1-6 联合** = 守住"没有相应权限而运行的代码"

## v4 → v5 → v6 变化

- v4 说"5 重守门融入每层"
- v5 改"4 重守门嵌套 + 权限发放(独立机制)" (2026-07-31)
- **v6 改"6 重守门嵌套 + 权限发放(独立机制) + Colang DSL 守门(新加第 6 重)"** (2026-08-10 R125 B4, 触发 R125-5 NVIDIA Guardrails)

- 守门 1 范围扩大到**原则洋葱整体**(不只是 12 键)
- 多 AI 一致**不再算守门**——是权限发放机制 (v5)
- **v6 多 AI 一致**算守门 (5 重) (R125-14 superpowers Skill 触发)
- **Colang DSL 守门** (6 重) (R125-5 NVIDIA Guardrails 借鉴)

## 6 项不假装 = O 层(与 12 键同层, v6 修正)

## 出处

阶段 2 §6.1 + 阶段 1 §18.6 + 阶段 4 correction-v5 + R125 B4 (R125-5 NVIDIA Guardrails).

详见 [`docs/stage4/stage4-correction-v5-gates-refined.md`](../../stage4/stage4-correction-v5-gates-refined.md)

## 5 个内容 (升级为 6 个, v6 修正)

1. 编译时 hardcode — Rust 6 大编译时约束
2. 运行时拦截 — `async RuntimeInterceptor` trait
3. 物理隔离 HA — 修改需重新编译 + 物理多签(AI×3 + 人×2 + 密钥×3)
4. 反思期审计 — Cognitive-Dream 24h 自动审计
5. **多 AI 一致** (v6 升) — 3 个不同 LLM 独立检查
6. **Colang DSL 守门** (v6 新加) — DSL 表达"什么操作允许/禁止"

## 每层都自带(不是独立层)

- E 层修改流程 = 五重治理(MEWG + 多人 + 多 AI + 物理多签 + 反思期)
- S 层修改流程 = 智囊团 + 双签
- A 层修改流程 = A → M promotion
- M 层修改流程 = 经验沉淀包
- O 层修改流程 = 权限矩阵

## 出处(6 个内容, v6 修正)

阶段 2 §6.1 + 阶段 1 §18.6 + 阶段 4 correction-v4 + R125 B4 (v6 升 6 重).

详见 [`docs/stage4/stage4-correction-v4-onion-dedupe.md`](../../stage4/stage4-correction-v4-onion-dedupe.md)

## 6 哲学锚穿透 (R125 B5 升 8 锚)

- **S-1** 北极星: 6 重守门 + 权限发放 = ASI 完整性工程化
- **S-2** 实事求是: v5 → v6 修正 (4 → 5 → 6 重 + Colang DSL)
- **S-3** 质量工程化: Colang DSL 编译期 hardcode (跟 R123-1 clippy+doc 清关联)
- **O-1** 安全优先: 6 重守门 (新加) = 最高安全标准
- **O-2** 走在前人经验上: 借鉴 NVIDIA Guardrails Colang DSL (R125-5)
- **O-5** 不假装: 编译期 hardcode 原则洋葱整体

## 不漂移 (R125 B4 升 v6)

- 🔒 6 重守门嵌套结构严守 (v6 升 6 重, R125-5 实施时落地)
- 🔒 权限发放独立机制严守
- 🔒 Colang DSL 守门 (v6 新加, R125-5 实施时落地)
- 0 改 workspace.version (R125 末 B2 升 1.2.0)
- 0 改 R11 baseline 3 值
- 0 改 12 键原 12 (R125-12 后新增 PHL-07 = 13 键)
