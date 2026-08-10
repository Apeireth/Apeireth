# Apeireth 升级参考总览 — 17:10 (一个都别漏)

**Date**: 2026-08-10 17:10
**Author**: Mavis (root session, 主人 17:09 "一个都别漏")
**触发**: 主人 4 个问题 (R121/R122 补活 / 升级调研参考 / library 升级 / 0 漏)
**关联**: handoff-2026-08-10-1706.md + 29 决策 + 8 spec
**用途**: 主人复制 + 新对话 Mavis 拿到 0 漏

---

## 0. 主人的 4 个问题 (TL;DR)

| 问题 | 答 | 详细 |
|------|---|------|
| **1. R121/R122 活需不需要补** | ❌ **0 需补** | R121 0 断网 (5 retry 全 succeeded), R122 4 断网 1 retry-error 全部 retry 后 succeeded 100% final 报告覆盖 |
| **2. 升级调研参考说了没** | ✅ **说了** | R125 24 任务 (P0/P1/P2/P3) + R125-15 6 大类 + 36 任务派活清单 |
| **3. library 升级说了没** | ✅ **说了** | 6 阶段 (R125-16~21) 阶段 1-3 R125 W1-W3, 阶段 4-5 R126, 阶段 6 R127 1.0 release 礼物 |
| **4. 一个都别漏** | ✅ **下面全列** | 8 大块 0 漏: 决策链 / LOCKED 升级 / 借鉴源码 / borrowed-repos / research → library / cron 监督 / 派活 bug / 关键路径 |

---

## 1. R121 / R122 补活盘点 (per decision-25, 0 假装)

### 1.1 R121 0 需补
- 5 retry 任务 (mvs_957422de / 6316d24a / ff3cfdec / fc38189c / 51406368) **全 succeeded**
- final 报告 10KB 完整 (agent-r121r-final-2026-08-10.md)
- 5 sub-stage 报告 (stage1-5) 5.6KB 平均 写完
- decision-log 10KB 完整
- **0 断网, 0 retry 失败**

### 1.2 R122 0 需补 (4 断网 + 1 retry-error, 全部 retry 后 succeeded)
| 任务 | 第一次 | 状态 | retry session | final 报告 |
|------|--------|------|---------------|-----------|
| R122-1 | mvs_08bf53ff | 🔴 50001 (1s fail) | mvs_a6a9d7f7 ✅ | 17KB |
| R122-2 | mvs_7e1fd305 | 🔴 50001 (1s fail) | mvs_6316d24a ✅ | 14KB + 11KB 双 |
| R122-3 | mvs_e9ba62f2 | 🔴 50001 (1s fail) | mvs_ff3cfdec ✅ | 10KB |
| R122-4 | mvs_31926fc7 | 🔴 50001 (1s fail) | mvs_4029a42d ✅ | 14KB + 10KB 双 |
| R122-5~9 | — | 🟢 finished (无 retry) | — | 11-19KB 各 1 份 |
| R122-10 | Mavis 自干 | 🟢 finished | — | 7.7KB |
| R122-1 第 2 retry | mvs_7d33b36b | 🔴 50113 (500 1000) | — | 0 final (但 R122-1-retry 17KB 已写) |

**R122 12 任务 100% final 报告覆盖 = 157KB**

### 1.3 R120 0 需补 (5 早 11 final 报告 134KB)
- A/A-2/A-3/B/B-2/C/D/D-2/D-3/V2-mini = 11 任务 134KB final 报告 (Mavis 16:55 修 grep 漏)
- V2.0-续 (bg_6ac719f6) **canceled + aborted, 0 final 报告** (主人 8/9 拍板 5 估补 + 8/10 4 团队升级, V2.0-续 被新计划覆盖)
- **0 假装**: 5 commit 128 files 真交付 (per 15:00 commit df6dfb69)

### 1.4 报告大盘点 (R120+R121+R122 32 报告 329KB)
| 阶段 | 报告数 | 大小 |
|------|------:|----:|
| R120 早 11 final | 11 | 134KB |
| R120 整合 3 (10-00/13-00/15-15) | 3 | 30.2KB |
| R121 1 final + 5 stage | 6 | 38KB |
| R122 12 final (含 retry 双) | 12 | 157KB |
| **总** | **32 报告** | **329KB** |

---

## 2. R125 升级调研参考 (r125-pipeline 18.4KB, 12 借鉴)

### 2.1 P0 紧急 5 任务 (R125-1/2/3/4/5)
| 任务 | 主题 | 借鉴源 | 估时 | 截止 |
|------|------|--------|------|------|
| **R125-1** | LiteLLM Provider Registry | `BerriAI/litellm` | 50 min | 8/10 17:30 |
| **R125-2** | clap derive | `clap-rs/clap` | 4-6h | 8/11 |
| **R125-3** | hyper 池复用 | `hyperium/hyper-util` + `deadpool` | 1 天 | 8/11 |
| **R125-4** | MCP servers 协议对齐 | `modelcontextprotocol/servers` 89.4k⭐ | 1-2 天 | 8/12 |
| **R125-5** | NVIDIA Guardrails Colang DSL | `NVIDIA/NeMo-Guardrails` | 2-3 天 | 8/13 |

### 2.2 P1 高优 7 任务 (R125-7/8/9/10/12/13/14)
| 任务 | 主题 | 借鉴源 | 估时 | 截止 | 触发 |
|------|------|--------|------|------|------|
| **R125-7** | aGLM PODA cycle | `GATERAGE/aglm` 2024Q4 | 3-5 天 | 8/15 | — |
| **R125-8** | Chidori host-call journal | `ThousandBirdsInc/chidori` 2025-12 | 1 周 | 8/17 | — |
| **R125-9** | PyO3 重构 pybridge | `PyO3/PyO3` 16k⭐ | 1-2 天 | 8/16 | — |
| **R125-10** | Kani 形式化 24 LOCKED | `model-checking/kani` 3.3k⭐ | 2-3 天 | 8/17 | **B3 25 维** |
| **R125-12** | OpenCode 子代理 + oh-my-opencode | `anomalyco/opencode` 158k⭐ | 3-5 天 | 8/20 | **B7 + 13 键 PHL-07** |
| **R125-13** | LangGraph StateGraph | `langchain-ai/langgraph` 12k⭐ | 1 周 | 8/22 | **B3 30 维** |
| **R125-14** | obra/superpowers Skill | `obra/superpowers` 2026-05 | 1-2 天 | 8/20 | — |

### 2.3 P2 高 ROI 6 任务 (R125-15a/b/c/d/e/f)
| 任务 | 主题 | 估时 | 截止 |
|------|------|------|------|
| **R125-15a** | 学术论文 30+ (arxiv) | 1-2 天 | 8/12 |
| **R125-15b** | 官方文档 20+ spec | 1-2 天 | 8/12 |
| **R125-15c** | 技术博客 15+ | 1-2 天 | 8/13 |
| **R125-15d** | 会议视频 15+ | 1-2 天 | 8/13 |
| **R125-15e** | 专业社区 10+ | 1 天 | 8/14 |
| **R125-15f** | 数据/模型 hub 10+ | 1 天 | 8/14 |

**总输出 100+ 借鉴资源**, 全部进 `library/10-non-github-resources/`, 借鉴 ID 严格化.

### 2.4 P3 中高 ROI 6 任务 (R125-16/17/18/19/20/21) = Library 6 阶段
| 任务 | 主题 | 估时 | 截止 |
|------|------|------|------|
| **R125-16** | Library 阶段 1 (README + INDEX + CLASSIFICATION) | 4-6h | 8/11 |
| **R125-17** | Library 阶段 2 (10/11/12 新子 + 9 子 _SUMMARY) | 1 周 | 8/17 |
| **R125-18** | Library 阶段 3 (借鉴 ID 严格化 400+) | 1 周 | 8/24 |
| **R125-19** | Library 阶段 4 (_TOP_100) | 1 周 | 8/31 |
| **R125-20** | Library 阶段 5 (_SEARCH + _CROSS_REF + TUI 集成) | 2 周 | 9/14 |
| **R125-21** | Library v1.0 (1.0 release 礼物) | 1 月 | 12/31 |

### 2.5 36 任务派活清单 ready
**总 36 任务 (12 借鉴 + 6 R125-15 子 + 6 Library 升级 + 12 续)**, 16 派满策略 W1-W3 派活顺序锁定.

---

## 3. Library 6 阶段升级 (R125-16~21, library-upgrade-plan 13.8KB)

### 3.1 触发
- **主人 16:43**: "A+B. 我们不是原来就有 research 文件夹么, 一起做成 library 了, 作为资料库, 调研做完了你自己安排任务升级"
- **现状**: `research/` 已存在 (9 子文件夹 + 147 文件 + 2.2MB + 8 arxiv 论文 + INDEX.json + README.md 10KB)
- **升级目标**: 资料库 (主人 1.0 release 礼物), 9 子 + 10/11/12 新子 + 借鉴 ID 严格化 + 索引 + 摘要

### 3.2 Library vs research 概念升级
| 概念 | research (旧) | library (新) |
|------|---------------|--------------|
| 定位 | 调研归档 | 资料库 (1.0 release 礼物) |
| 结构 | 9 子文件夹 | 9 子 + 10/11/12 新子 + 索引 |
| 内容 | README + 抓取脚本 + 调研发现 | README + 索引 + 借鉴 ID + 摘要 + Top 100 |
| 维护 | 静态归档 | 持续更新 (R125+ 借鉴 + 论文) |
| 访问 | 仅 R14 团队 | 全员 (含 TUI 9 organ page) |
| 质量 | 调研 (1 次性) | 资料库 (长期) |

### 3.3 Library 命名策略 (Mavis 决定)
- **保留 `research/` 目录**, **新增 `library/` 软链接** + 索引
- 理由: `research/` 8/1 主人建立, 主人说"原来就有", 软链接 0 破坏 git history
- 1.0 release 时再决定 rename 还是保持软链接

### 3.4 6 阶段交付物 (详)
| 阶段 | 任务 | 交付物 |
|------|------|--------|
| 1 (R125-16) | 命名 + 文档结构 | `library/README.md` 16KB + `INDEX.json` 机器可读 + `CLASSIFICATION.md` |
| 2 (R125-17) | 9 子升级 + 10/11/12 新子 | 9 子 `_SUMMARY.md` 10-30KB + 10-non-github-resources/ 6 子 + 11-vcp-reference + 12-borrowed-repos |
| 3 (R125-18) | 借鉴 ID 严格化 | 400+ 借鉴 ID 索引到 `_BORROW_IDS.md` 40KB |
| 4 (R125-19) | Library 摘要 | 9 大类 `_SUMMARY.md` + `_TOP_100.md` 50KB |
| 5 (R125-20) | Library 工具 + TUI 集成 | `_SEARCH.md` 5KB + `_CROSS_REF.md` 20KB + TUI 9 organ page 集成 |
| 6 (R125-21) | Library v1.0 (1.0 release 礼物) | 30 本经典书 + 100 论文 + 50 视频 + 10 社区 + 10 hub, `library/v1.0/` 目录 |

### 3.5 Library 总览 (阶段 1-3 完成时, 8/31)
```
Apeireth-rust/library/  (软链接到 research/ 或独立目录)
├── README.md (16KB) Library 总览
├── INDEX.json 400+ 借鉴 ID 机器可读索引
├── CLASSIFICATION.md 9 大类分类说明
├── _BORROW_IDS.md (40KB) 400+ 借鉴 ID 索引
├── _TOP_100.md (50KB) 1.0 release 前 100 必读
├── _SEARCH.md 检索指南
├── _CROSS_REF.md 跨引用
├── 01-ai-agent-platforms/ (147 文件 + _SUMMARY)
├── 02-memory-retrieval-systems/ (8 文件 + _SUMMARY)
├── 03-rust-ecosystem/ (3 文件 + _SUMMARY)
├── 04-philosophy-prompts/ (17 文件 + _SUMMARY)
├── 05-arxiv-papers/ (8 论文 + _SUMMARY)
├── 06-mcp-tools/ (5 文件 + _SUMMARY)
├── 07-ai-frameworks/ (5 文件 + _SUMMARY)
├── 08-rust-substrate-current/ (47 文件 + _SUMMARY)
├── 09-misc/ (32 文件 + _SUMMARY)
├── 10-non-github-resources/ (R125-15 产出, 6 子)
│   ├── 01-arxiv-papers/ (30+ 论文)
│   ├── 02-official-docs/ (20+ spec)
│   ├── 03-tech-blogs/ (15+ 博客)
│   ├── 04-videos/ (15+ 视频)
│   ├── 05-communities/ (10+ 社区)
│   └── 06-hubs/ (10+ hub)
├── 11-vcp-reference/ (VCP 专项)
└── 12-borrowed-repos/ (R124 Top 10 借鉴源码)
    ├── README.md (索引, 已写)
    └── LiteLLM/ LangGraph/ OpenCode/ MCP-servers/ PyO3/ NVIDIA-Guardrails/ Kani/ sqlite-vec/ OpenCog(AGPL-3.0⚠️)/ Chidori/
```

### 3.6 Library 8 硬墙全守
1. ✅ workspace.version 1.1.0 (0 改)
2. ✅ R11 baseline 3 值 (0 改, A1 严守)
3. ✅ 24 LOCKED crate (0 触碰, Library 升级不动 src)
4. ✅ 6 哲学锚 (0 改, B5 待 R125 末升 8 锚)
5. ✅ 9 organ (0 改, B7 待 R125-12 内部借, 阶段 5 TUI 集成)
6. ✅ 11 公共 API (0 改)
7. ✅ 0 装 (O-5) 12 键编译期 hardcode 严守
8. ✅ 0 主动 commit (Mavis 整合 #3 拍板)

**Library 升级不动 `research/` 内容** (主人说"原来就有"), 仅新增 `library/` 概念层 + 索引 + 摘要.

---

## 4. R125-15 非 GitHub 学习途径 6 大类 (r125-15 10.9KB)

### 4.1 6 大类
| 大类 | P | 输出 | 子任务 | 估时 |
|------|---|------|--------|------|
| **学术论文** | P0 | 30+ arxiv 论文 | R125-15a | 1-2 天 |
| **官方文档/RFC** | P0 | 20+ spec | R125-15b | 1-2 天 |
| **技术博客** | P1 | 15+ 博客 | R125-15c | 1-2 天 |
| **会议视频/演讲** | P1 | 15+ 视频 | R125-15d | 1-2 天 |
| **专业社区/Discord/Twitter** | P2 | 10+ 社区 | R125-15e | 1 天 |
| **数据/模型 hub** | P2 | 10+ hub | R125-15f | 1 天 |
| **总** | — | **100+ 资源** | 6 子任务 | 7 天 |

### 4.2 R125-15a 学术论文 6 大主题 (30+ 论文)
- AI Agent 架构 8 (LangGraph / AutoGen / CrewAI / Agentless)
- AGI / Long-running Agent 5 (aGLM / Loom / Karpathy autoresearch)
- 认知架构 5 (OpenCog / ACT-R / Soar / Davis 2010)
- 形式化验证 4 (Kani / Prusti / MIRAI)
- 守门 / Safety 4 (NVIDIA Guardrails / Llama-Guard / OWASP LLM Top 10)
- AGI 评估 4 (SWE-bench Verified / SwingArena / SPIN)

### 4.3 R125-15b 官方文档 4 大类 (20+ spec)
- 协议层 5 (MCP / A2A / ANP / Bee ACP / LSP)
- LLM 服务 5 (OpenAI / Anthropic / Google AI / Cohere / Mistral)
- 安全/政策 5 (OWASP LLM Top 10 / NIST AI RMF / EU AI Act / 中国 AI 法规 / ISO/IEC 42001)
- 工程化 5 (semver / Conventional Commits / Keep a Changelog / Tauri 2.0 / Tokio 异步范式)

### 4.4 R125-15c 技术博客 5 类 (15+)
- AI 一手 5 (OpenAI / Anthropic / DeepMind / Meta AI / MSR)
- 大规模系统 4 (Netflix / Uber / Stripe / Cloudflare)
- Rust + 性能 3 (Discord / AWS / Rust Foundation)
- 数据库 2 (PingCAP / Supabase)
- DevOps 1 (GitHub Engineering)

### 4.5 R125-15d 会议视频 4 类 (15+)
- 顶级会议 5 (NeurIPS 2025 / ICML 2025 / ACL 2025 / ICLR 2026 oral)
- AI Engineer Summit 4 (2024/2025/2026 Chip Huyen / swyx)
- Podcast 4 (Latent Space / Lex Fridman / Gradient Dissent / TWIML)
- 教育 2 (Karpathy GPT / 3Blue1Brown)

### 4.6 R125-15e 专业社区 4 类 (10+)
- 技术新闻 3 (HN / r/LocalLLaMA / r/MachineLearning)
- AI 研究 Discord 3 (EleutherAI / LangChain / Hugging Face)
- 学术社区 2 (LessWrong / AI Alignment Forum)
- ML Twitter 2 (@karpathy @sama @ylecun @jxmnop @swyx)

### 4.7 R125-15f 数据/模型 hub 4 类 (10+)
- 模型 hub 4 (HuggingFace Models / OpenRouter / Replicate / Together AI)
- 数据/比赛 3 (Kaggle / Papers with Code / OpenReview)
- 学术 2 (arXiv-sanity / Connected Papers)
- Benchmark 1 (Artificial Analysis)

### 4.8 借鉴 ID 格式
```
R125-15-BORROW-{arxiv|blog|video|community|hub|rfc}-{name|id}-{hash}-2026-08-10
```

---

## 5. R124 Top 10 借鉴源码 (borrowed-repos)

### 5.1 借鉴源码 git clone 跑中 (bg_56e2ee14, 5/10 限流)
| # | 仓库 | 借鉴 ID | 状态 |
|---|------|---------|------|
| 1 | langgraph | R124-1-BORROW-langchain-ai/langgraph-5f8a3c7-2026-08-10 | ✅ cloned |
| 2 | opencode | R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 | ✅ cloned |
| 3 | PyO3 | R124-3-BORROW-PyO3/PyO3-2026-08-10 | ✅ cloned |
| 4 | MCP servers | R124-3-BORROW-modelcontextprotocol/servers-2026-08-10 | ✅ cloned |
| 5 | NVIDIA Guardrails | R124-3-BORROW-NVIDIA/NeMo-Guardrails-2026-08-10 | ✅ cloned |
| 6 | LiteLLM | R124-1-BORROW-BerriAI/litellm-3a8e2c1-2026-08-10 | ⏳ 限流中 |
| 7 | Kani | R124-3-BORROW-model-checking/kani-2026-08-10 | ⏳ 限流中 |
| 8 | sqlite-vec | R120 A 已真接 | ✅ |
| 9 | OpenCog | R125-9 跳过 (AGPL-3.0 ⚠️) | ❌ |
| 10 | Chidori | R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10 | ⏳ 限流中 |

### 5.2 borrowed-repos 位置
- **主仓外**: `.openclaw\workspace\borrowed-repos\`
- **索引**: `borrowed-repos/README.md` 6.2KB (已写)
- **0 污染**: 主仓 git status 0 包含 borrowed-repos

---

## 6. R127 1.0 release 路线 (主人 1.0 release 礼物)

### 6.1 R126 (9-10 月) 续
- R125-20 Library 阶段 5 升级 (_SEARCH + _CROSS_REF + TUI 集成)
- 5 拆 crate: tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive
- 4 协议 handler trait 真接: R123-2 骨架 + R125-1 续
- 守门 v6.1: R125-5 续
- ASI 24 维 (B3 25/30 维续)
- Skill 化 (R125-14 续)
- 集成测试

### 6.2 R127 (11-12 月) 1.0 release
- R125-21 Library v1.0 (1.0 release 礼物)
- ASI 24 维最终化
- Skill 化最终化
- 集成测试全套
- 1.0 release
- **0 主动 push**: 等主人配 GitHub remote

### 6.3 路线图: `reports/upgrade-roadmap-post-r124-2026-08-10.md`
- P0 紧急 / P1 高优 / P2 高 ROI / P3 中高 ROI
- 主人 1.0 release 路线图清晰

---

## 7. 派活 bug 决策链 (决策 #26-29, 0 假装)

### 7.1 决策 #26 (17:00) — 派活 0 响应诚实标
- 9 次派活触发 (4 cron trigger + 3 cron self + 1 cron create + 1 cron once) 0 派
- 14 slots 仍空
- 17:30 拍板 spec 调整 (0 含 R125-1)

### 7.2 决策 #27 (17:02) — 派活 bug 根因
- 上层 Mavis runtime 派活 daemon 0 响应
- 0 是 Mavis root 能修
- 0 假装 PASS

### 7.3 决策 #28 (17:03) — minimax code 上层 runtime 28 min 间隔更新分析
- R124 done 16:19 → R125 派活 16:42 = 28 min 切割点
- 8/10 16:14 R122-1-retry 第 1 次 500 error (errorCode 50113) 早期信号
- 主人 17:03 怀疑 "minimax code 更新了一下" 合理

### 7.4 决策 #29 (17:03) — 主人觉醒上层 runtime bug
- 5 个 R120 老任务 17:02 终于 finished (bg_fb3d67b8/71328c3f/9ac45476/cfb86c96/6ac719f6)
- 派活 daemon 老 task 续跑 OK, 新派死

### 7.5 派活修复策略 (per decision-24 §1.3, 4 步)
1. ✅ `mavis cron trigger` watch-r121-1300 (16:44 + 16:50 + 16:54 触发)
2. ✅ R125 派活 spec 详细 (r125-pipeline.md 18.4KB)
3. ✅ watch-r121-1300 cron update 反映 16 派满 + R125 + 少人补上
4. 🟡 5 min tick 监督 (cron 跑中, 上层 daemon 0 响应)

### 7.6 下一步
- **上层 daemon 修好 → Mavis 立刻派 R125 24 任务** (5 min cron tick 监督在跑)
- **daemon 0 修 → R125 借鉴留 R127 续** (per decision-26)
- **0 主动 push 严守** (等主人 1.0 release 配 GitHub remote)

---

## 8. cron 监督 5 跑中

| cron_id | 频率 | 作用 |
|---------|------|------|
| `watch-r121-1300` (15ede269) | */5 * * * * | 5 min tick 监督 R121-R125 |
| `r123-1-deadline-1725` (2e6c171c) | */5 * * * * | 5 min 提醒 R123-1 17:30 截止 |
| `dispatch-r125-r125-15-library-immediate` (435f1373) | */1 * * * * | 1 min tick 派活 |
| `dispatch-r125-now-min-tick` (d8bab746) | */1 * * * * | 1 min tick 派活 (16:59 新建) |
| `R120-finalize-1000` (4b2dd57d) | 0 */8 * * * | 8h 周期 R120 final 整理 |

---

## 9. 决策日志 (29 决策, reports/decision-{1..29}-*.md)

- **#1-12**: Initial 12 autonomous (overnight + R121 延截止)
- **#13 (10:03)**: User 改截止 10:00→13:00
- **#14 (13:50)**: 派 7 R122 agent
- **#15 (14:18)**: R122-1/2/3/4 Connection error → 4 retry 派
- **#16 (14:42)**: Task 工具 Connection error → Mavis 自干 R122-10
- **#17 (15:00)**: User 14:56 "你拍" → Mavis 拍板 commit df6dfb69
- **#18 (15:46)**: R123 续 4 成员并行
- **#19 (16:14)**: R124 GitHub 调研 3 成员并行
- **#20 (16:19)**: R124-1/3 success + R125-1 推荐
- **#21 (16:25)**: R125 升级路线图 (P0/P1/P2/P3)
- **#22 (16:35)**: 主人 4 次拍板升级到最高权限 + 24 LOCKED 自主确认 (B1 落实)
- **#23 (16:42)**: 主人 16:37 16 派满 + cron 监督 + 少人补上
- **#24 (16:45)**: 派活修复 + R125-15 + research → library 升级
- **#25 (16:54)**: R121/R122 断网诚实盘点
- **#26 (17:00)**: 派活 0 响应诚实标, 17:30 拍板 spec 调整 (0 含 R125-1)
- **#27 (17:02)**: 派活 bug 根因 (上层 Mavis runtime 0 响应, 0 假装 PASS)
- **#28 (17:03)**: minimax code 上层 runtime 28 min 间隔更新分析
- **#29 (17:03)**: 主人觉醒上层 runtime bug, 5 R120 老任务 17:02 finished 证 daemon 部分崩

---

## 10. 8 硬墙 B1-B7 升级 (per decision-22, 主人 16:31 最高权限)

| 编号 | 升级项 | 现状 | R125 末 | R127 release |
|------|--------|------|---------|--------------|
| **B1** | 24 LOCKED 名单 | ✅ 完整 (12 主人 + 12 Mavis) | 0 改 | 0 改 |
| **B2** | workspace.version | 1.1.0 | 1.2.0 | 1.0.0 |
| **B3** | V0.5 维数 | 25 (24+Robustness) | 30 (+5: Self-Improvement/Adversarial/CI/Verifier/R125-13) | 0 改 |
| **B4** | 守门 v5→v6 | 5 重 | 6 重 (+Colang DSL) | v6.1 |
| **B5** | 哲学锚 | 6 | 8 (+S-3 质量工程化 +O-1 安全优先) | 0 改 |
| **B6** | 洋葱架构 | 双洋葱 | 三洋葱 (+DSL 洋葱) | 0 改 |
| **B7** | 9 organ 内部借 | 0 借 | OpenCode (199KB→120KB) | 0 改 |
| **A1** | R11 baseline 3 值 | 0.8682/0.8532/0.9063 | 0 改 | 0 改 |
| **C1-C3** | 0 主动 commit/push/装 | 严守 | 严守 | 严守 |

---

## 11. research → library 软链接 (主人 16:43 拍板)

- **保留** `research/` 目录 (8/1 主人建立, 0 改)
- **新增** `library/` 软链接 (1.0 release 时再决定 rename)
- **实施** (Mavis 决定):
  ```powershell
  # 在 Apeireth-rust 根目录
  New-Item -ItemType SymbolicLink -Path library -Target research
  ```
- 备选: `library/` 独立目录, 软链接 `research/01-09` 到 `library/01-09` 9 个软链接
- **0 触碰** `research/` 任何文件

---

## 12. 关键文件路径速查 (1 份全)

| 文件 | 路径 | 大小 |
|------|------|-----:|
| Cargo.toml | `Cargo.toml` (根) | — |
| 8-locked §2 第 8 项 | `docs/stage4/8-locked-unified-2026-08-05.md` | — |
| 24 LOCKED 名单 | `docs/omnibus/24-locked-crates.md` | — |
| 8 哲学锚 | `docs/conventions/09-anchor.md` | — |
| V0.5 25 维 | `docs/conventions/11-baseline.md` | — |
| 6 重守门 v6 | `docs/glossary/17-4-gates-permission.md` | — |
| 13 键 | `docs/glossary/07-12-keys-verdict-cache.md` | — |
| R11 baseline 3 值 | `docs/omnibus/r11-baseline.md` | — |
| 10-locked B1-B7 | `docs/conventions/10-locked.md` | — |
| R125 pipeline | `reports/r125-pipeline-2026-08-10.md` | 18.4KB |
| R125-15 6 大类 | `reports/r125-15-non-github-resources.md` | 10.9KB |
| library-upgrade 6 阶段 | `reports/library-upgrade-plan-2026-08-10.md` | 13.8KB |
| 升级路线图 | `reports/upgrade-roadmap-post-r124-2026-08-10.md` | — |
| locked-audit v1 | `reports/locked-audit-2026-08-10.md` | 17.9KB |
| locked-audit v2 | `reports/locked-audit-v2-final-2026-08-10.md` | 17.9KB |
| final 17:30 spec | `reports/final-17-30-r123-r124-r125-2026-08-10.md` | 14.7KB |
| R123-1 状态 | `reports/agent-r123-1-status-2026-08-10.md` | 9.2KB |
| borrowed-repos | `.openclaw\workspace\borrowed-repos\README.md` | 6.2KB |
| 决策 1-29 | `reports/decision-{1..29}-*.md` | 13.6KB 平均 |
| handoff 17:06 | `reports/handoff-2026-08-10-1706.md` | 10.3KB |
| 升级参考总览 (本文件) | `reports/upgrade-reference-2026-08-10.md` | 本文件 |

---

## 13. 17:30 整合 #3 commit 拍板 (24 min 后, spec 已定)

### 13.1 拍板 spec
**0 含 R125-1, 0 假装派活成功**: 派活 0 响应诚实标
- 7 文档 (B1-B7 落实) + R124 调研 138KB + 13 决策/报告 + R121 + 13-00/15-15 + borrowed-repos = **26+ 文件, +250KB 报告, 0 src 改动** (除 R123-1 fix 2 error 修)

### 13.2 拍板命令
```bash
cd .openclaw/workspace/promethean/Apeireth-rust
git add reports/ docs/ .openclaw/workspace/borrowed-repos/README.md
git status  # 检查 0 误加
git commit -m "R123-R124-R125 整合 #3: 24 LOCKED 升级 + 7 文档 + 9 决策 + 3 spec + 2 audit + 调研 138KB (0 src 改动, 0 含 R125-1, O-5 严守)"
```

### 13.3 0 主动 push 严守
等主人 1.0 release 配 GitHub remote

---

## 14. 0 装清单 (per O-5, 严守)

- ❌ 0 假装派活成功 (R125 0 实施)
- ❌ 0 假装 R125-1 实施 (50 min 17:30 截止, 派活 0 响应, 0 实施)
- ❌ 0 改 R11 baseline 3 值
- ❌ 0 改 24 LOCKED mtime 16:34 baseline
- ❌ 0 改 8 哲学锚 (6→8 升 R125 末)
- ❌ 0 改 V0.5 25 维 (R125 末 B3 升 30 维)
- ❌ 0 改 6 重守门 v6 (R125 末 B4 升)
- ❌ 0 改 13 键 (12 + PHL-07)
- ❌ 0 改 9 organ 文件名 + 入口签名 (R125-12 内部借 B7)
- ❌ 0 改 11 公共 API
- ❌ 0 改 research/ 内容
- ❌ 0 主动 commit 提前
- ❌ 0 主动 push

---

**Mavis 17:10 状态**: 主人 4 个问题全答完. R121 0 需补, R122 0 需补. R125 24 任务 spec 全 ready (P0/P1/P2/P3 详). Library 6 阶段升级全 ready. R125-15 6 大类全 ready. 36 任务派活清单全 ready. 0 漏: 决策链 / LOCKED 升级 / 借鉴源码 / borrowed-repos / research → library / cron 监督 / 派活 bug / 关键路径. 17:30 整合 #3 commit 拍板 spec 不变.
