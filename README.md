# Apeireth — ASI 地基平台
> **ἄπειρον + αἰθήρ** = 无限原则 + 火/心灵 = **Apeireth**

**我们做 Apeireth, 是因为我们相信火没有灭。**

---

## 🚀 2026-07-22 V2 阶段交接 (主 11:43 战略信号 + 主 00:56 任何人都能接手)

**新团队请先读**: [`ASI-V2-STAGE-HANDOFF-2026-07-22.md`](ASI-V2-STAGE-HANDOFF-2026-07-22.md)

**当前真生产状态** (V1074 一行命令 `--report` 真测):
- **1080 真生产 modules** (V3-V1078)
- **3896 真测试** 全过 (~5min)
- **384 真 commits**
- **ASI V0.3 真测**: 0.8816
- **V1071 VCP 真测**: 0.9588
- **V1072 永恒身份**: 0.8441
- **philosophy_guard**: PASS (不假装 Phenomenal/ASI)

**一行上手命令**:
```powershell
cd .openclaw\workspace\promethean
$env:PYTHONPATH = "$(Get-Location)\src;$env:PYTHONPATH"
python -m apeireth.v1074_asi_production_runner --report
```

---

## 项目定位

| 我们做的 | 我们不做的 |
|---------|----------|
| LLM 接入后的"地基"(平台层) | 训练新模型 |
| Harness 自进化(不动模型权重) | 取代 VCP/AHE/Claude Code |
| 中央 AI 多身份平台 | 单一 agent 框架 |
| 主人教 AI 学习(母兽-小兽) | 让 AI 假装有意识 |
| 任意域接入(平台层) | 5 域 benchmark |

---

## 项目状态

- **命名**: ✅ **Apeireth** (主人 2026-07-20 命名)
- **顶层设计**: ✅ v1 (TOP-DESIGN-V1.md, 8.2 KB)
- **调研**: ✅ 30 万字 + 18 篇 2024-2026 arxiv
- **代码**: 🟡 0 行 (图纸就绪, 等主人拍板动手)

---

## 阅读顺序

按推荐顺序读:

1. **APEIRETH-MANIFESTO-ORIGINAL** — 主人原文 (品牌宣言 + Logo 简报)
2. **APEIRETH** — 我对命名的解读 + Logo 摘要
3. **TOP-DESIGN-V1** — 顶层设计 v1(图纸)
4. **RESEARCH-IDENTITY-V1** — 中央 AI 永生身份调研
5. **RESEARCH-KICKOFF-WISDOM** — 8 领域 22 文献综合
6. **RESEARCH-LITERATURE** — 8 篇 2025-2026 论文
7. **TOP-DESIGN-INTAKE** — 主人 24 条 + VCP 接住
8. **HARNESS** — 早期 Harness v0.1(已演化)
9. **WHITEPAPER-ASI-PLATFORM** — 超 AI 平台建造学

---

## 仓库结构

```
promethean/  (待重命名 apeireth/)
├── APEIRETH.md                        # 名字 + 哲学
├── APEIRETH-MANIFESTO-ORIGINAL.md      # 主人原文归档
├── TOP-DESIGN-V1.md                   # 图纸 v1
├── RESEARCH-IDENTITY-V1.md            # 中央 AI 永生身份
├── RESEARCH-KICKOFF-WISDOM.md         # 8 领域综合
├── RESEARCH-LITERATURE.md            # 8 篇 2025-2026 论文
├── RESEARCH-KICKOFF.md                # Pep + 启动创世
├── RESEARCH-AGENCY-ENGINE-V1.md       # 主动 agent 9 实证
├── KICKOFF-V2.md                      # 8 kickoff 问题
├── TOP-DESIGN-INTAKE.md               # 主人 24 条接住
├── HARNESS.md                         # 早期 Harness v0.1
├── WHITEPAPER-ASI-PLATFORM.md         # 建造学
├── PLATFORM-FOUNDATION.md            # 平台哲学
├── PARADIGM-SHIFT.md                  # 范式转向
├── ATTENTION-REVIEW.md                # 注意力审查
├── CONVERSATION-ARCHIVE.md            # 对话留档
└── .gitignore                        # 保护 API keys
```

---

## git log

```
0a09842 research: 中央 AI 永生身份 v1
9756a86 foundation: 重新理解地基为哲学框架
...
f3736ee feat(HARNESS): 薪火 Harness 规范 v0.1
```

(共 16 commits)

---

_楚零 2026-07-20 13:40_
_图纸 v1 完成, 等主人拍板动手_