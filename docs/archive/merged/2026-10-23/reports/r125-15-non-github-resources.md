# R125-15 — 非 GitHub 学习途径调研 (学术 + 官方 + 博客 + 视频 + 社区 + hub)

**Date**: 2026-08-10 16:46
**Author**: Mavis (root session, 主人 16:43 拍板 A 选项)
**关联决策**: `decision-24-r125-15-library-2026-08-10.md`
**周期**: 7 天 (8/11-8/17, R125 主线 W1)
**派活**: 6 个 sub-agent, 1 类 1 个 (R125-15a / 15b / 15c / 15d / 15e / 15f)
**输出**: 30+ 论文 / 20+ 文档 / 15+ 视频 / 10+ 社区 / 10+ hub = **85+ 借鉴资源**
**状态**: ✅ **6 子任务 spec ready, 借鉴 ID 格式定义, Mavis 自主**

---

## 0. 触发事件

**主人 16:41 问**: "除了 GitHub, 你觉得我们的项目还有什么学习途径?"
**Mavis 16:42 答**: 6 大类 (学术论文 / 官方文档 / 博客 / 视频 / 社区 / hub) + Mavis 给 R125-15 + Apeireth Library 建议
**主人 16:43 拍板**: "A+B. 我们不是原来就有 research 文件夹么, 一起做成 library 了, 作为资料库, 调研做完了你自己安排任务升级"

---

## 1. R125-15 6 大类调研总览

| 大类 | P 级别 | 输出 | 子任务 | 估时 |
|---|---|---|---|---|
| **学术论文** | P0 | 30+ arxiv 论文 | R125-15a | 1-2 天 |
| **官方文档 / RFC** | P0 | 20+ spec | R125-15b | 1-2 天 |
| **技术博客** | P1 | 15+ 博客 | R125-15c | 1-2 天 |
| **会议视频 / 演讲** | P1 | 15+ 视频 | R125-15d | 1-2 天 |
| **专业社区 / Discord / Twitter** | P2 | 10+ 社区 | R125-15e | 1 天 |
| **数据/模型 hub** | P2 | 10+ hub | R125-15f | 1 天 |
| **总计** | — | **100+ 资源** | 6 子任务 | 7 天 |

---

## 2. R125-15a 学术论文 (P0, 30+ 论文)

### 2.1 调研方向

按 Apeireth 1.0 release 路线图, 30+ 论文分 6 大主题:

| 主题 | 数量 | 重点论文 |
|---|---:|---|
| **AI Agent 架构** | 8 | LangGraph (R124-2 B-001), AutoGen (B-002), CrewAI (B-004), Agentless (B-046) |
| **AGI / Long-running Agent** | 5 | aGLM (B-016, 2024Q4), Loom (B-017, 2025), Karpathy autoresearch (B-018) |
| **认知架构** | 5 | OpenCog (B-028, 30+ 年), ACT-R (B-026), Soar (B-030), Davis 2010 Cognitive Arch (B-033) |
| **形式化验证** | 4 | Kani (B-029), Prusti (B-030), MIRAI (B-031) |
| **守门 / Safety** | 4 | NVIDIA Guardrails (B-024), Llama-Guard (B-035), OWASP LLM Top 10 |
| **AGI 评估** | 4 | SWE-bench Verified (B-043, OpenAI 2024-08), SwingArena (B-044, ICLR 2026 Oral), SPIN (B-045) |

### 2.2 调研任务

- **位置**: `library/10-non-github-resources/01-arxiv-papers/`
- **格式**: 每篇 1 个 .md 文件 (5-10KB), 含 arxiv ID + 摘要 + 核心 idea + 借鉴方向 + 借鉴 ID
- **下载**: PDF 到 `references/borrowed-papers/` (主仓外 0 污染)
- **0 主动 commit**: Mavis 整合 #3 拍板

### 2.3 借鉴 ID 格式

```
R125-15-BORROW-arxiv-{arxiv_id}-{hash}-2026-08-10
```

例: `R125-15-BORROW-arxiv-2607.00151-7a3b2c1-2026-08-10`

---

## 3. R125-15b 官方文档 / RFC (P0, 20+ spec)

### 3.1 调研方向

按 Apeireth 协议层 + 安全层 + 工程化, 20+ spec 分 4 大类:

| 类别 | 数量 | 重点 spec |
|---|---:|---|
| **协议层** | 5 | MCP (modelcontextprotocol.io), A2A (a2a-protocol), ANP (Agent Network Protocol), Bee ACP, LSP (Language Server Protocol) |
| **LLM 服务** | 5 | OpenAI API reference, Anthropic API reference, Google AI Studio, Cohere API, Mistral API |
| **安全 / 政策** | 5 | OWASP LLM Top 10, NIST AI RMF, EU AI Act, 中国 AI 法规, ISO/IEC 42001 |
| **工程化** | 5 | semver, Conventional Commits, Keep a Changelog, Tauri 2.0 spec, Tokio 异步范式 |

### 3.2 调研任务

- **位置**: `library/10-non-github-resources/02-official-docs/`
- **格式**: 每份 1 个 .md 文件 (3-8KB), 含 URL + 摘要 + 核心 API/字段 + 借鉴方向
- **下载**: PDF 到 `references/borrowed-docs/` (主仓外 0 污染)

---

## 4. R125-15c 技术博客 (P1, 15+ 博客)

### 4.1 调研方向

按 Apeireth 工程化 + 性能 + 可靠性, 15+ 博客分 5 类:

| 类别 | 数量 | 重点博客 |
|---|---:|---|
| **AI 一手** | 5 | OpenAI Engineering, Anthropic Research, Google DeepMind Blog, Meta AI Blog, Microsoft Research |
| **大规模系统** | 4 | Netflix Tech Blog, Uber Engineering, Stripe Engineering, Cloudflare Blog |
| **Rust + 性能** | 3 | Discord Engineering (Rust + Elixir), AWS Rust 案例, Rust Foundation Blog |
| **数据库** | 2 | PingCAP Blog (TiDB), Supabase Blog |
| **DevOps** | 1 | GitHub Engineering Blog |

### 4.2 调研任务

- **位置**: `library/10-non-github-resources/03-tech-blogs/`
- **格式**: 每博客 1 个 .md (3-5KB), 含 URL + 3-5 篇核心文章 + 借鉴方向
- **0 下载 PDF**: 博客文章 URL 已存档, Mavis 0 必下载, 主人需要时按 URL 读

---

## 5. R125-15d 会议视频 / 演讲 (P1, 15+ 视频)

### 5.1 调研方向

按 AI 前沿 + 工程化实战, 15+ 视频分 4 类:

| 类别 | 数量 | 重点视频 |
|---|---:|---|
| **顶级会议 main track** | 5 | NeurIPS 2025 main track, ICML 2025 main track, ACL 2025 best papers, ICLR 2026 oral |
| **AI Engineer Summit** | 4 | 2024/2025/2026 AI Engineer Summit talks (Chip Huyen, swyx, etc) |
| **Podcast** | 4 | Latent Space (swyx 主理, AI 业界金标准), Lex Fridman, Gradient Dissent, TWIML |
| **教育** | 2 | Andrej Karpathy GPT 系列视频, 3Blue1Brown 深度学习 |

### 5.2 调研任务

- **位置**: `library/10-non-github-resources/04-videos/`
- **格式**: 每视频 1 个 .md (3-5KB), 含 URL + 字幕 (如有) + 核心 idea + 借鉴方向
- **下载**: 字幕到 `references/borrowed-videos/` (主仓外 0 污染)
- **0 下载视频本体**: 视频大 (GB 级), 主人按 URL 看

---

## 6. R125-15e 专业社区 / Discord / Twitter (P2, 10+ 社区)

### 6.1 调研方向

按 AI 业界实时脉搏, 10+ 社区分 4 类:

| 类别 | 数量 | 重点社区 |
|---|---:|---|
| **技术新闻** | 3 | Hacker News (news.ycombinator.com), Reddit r/LocalLLaMA, Reddit r/MachineLearning |
| **AI 研究 Discord** | 3 | EleutherAI Discord, LangChain Discord, Hugging Face Discord |
| **学术社区** | 2 | LessWrong, AI Alignment Forum |
| **ML Twitter** | 2 | @karpathy @sama @ylecun @jxmnop @swyx 顶级 AI 研究者, ML Twitter 列表 |

### 6.2 调研任务

- **位置**: `library/10-non-github-resources/05-communities/`
- **格式**: 每社区 1 个 .md (2-3KB), 含 URL + 加入方式 + 核心价值 + 推荐关注列表

---

## 7. R125-15f 数据/模型 hub (P2, 10+ hub)

### 7.1 调研方向

按实战级资源, 10+ hub 分 4 类:

| 类别 | 数量 | 重点 hub |
|---|---:|---|
| **模型 hub** | 4 | Hugging Face Models, OpenRouter (200+ LLM 路由), Replicate, Together AI |
| **数据/比赛** | 3 | Kaggle Competitions (SWE-bench, GAIA, AgentBench), Papers with Code, OpenReview |
| **学术** | 2 | arXiv-sanity (Karpathy), Connected Papers (图谱) |
| **Benchmark** | 1 | Artificial Analysis (LLM 性能对比) |

### 7.2 调研任务

- **位置**: `library/10-non-github-resources/06-hubs/`
- **格式**: 每 hub 1 个 .md (2-3KB), 含 URL + 核心价值 + 借鉴方式

---

## 8. R125-15 派活清单 (6 sub-agent)

| 子任务 | 主题 | 估时 | 截止 | 派活 spec |
|---|---|---|---|---|
| **R125-15a** | 学术论文 (P0, 30+ 论文) | 1-2 天 | 8/12 17:30 | 派 1 sub-agent (general agent) |
| **R125-15b** | 官方文档 (P0, 20+ spec) | 1-2 天 | 8/12 17:30 | 派 1 sub-agent |
| **R125-15c** | 技术博客 (P1, 15+ 博客) | 1-2 天 | 8/13 17:30 | 派 1 sub-agent |
| **R125-15d** | 会议视频 (P1, 15+ 视频) | 1-2 天 | 8/13 17:30 | 派 1 sub-agent |
| **R125-15e** | 社区 (P2, 10+) | 1 天 | 8/14 17:30 | 派 1 sub-agent |
| **R125-15f** | Hub (P2, 10+) | 1 天 | 8/14 17:30 | 派 1 sub-agent |

**总输出**: 100+ 借鉴资源, 全部进 `library/10-non-github-resources/`, 借鉴 ID 严格化.

---

## 9. R125-15 借鉴 ID 汇总

| 类型 | ID 格式 | 例 |
|---|---|---|
| arxiv | `R125-15-BORROW-arxiv-{arxiv_id}-{hash}-2026-08-10` | `R125-15-BORROW-arxiv-2607.00151-7a3b2c1-2026-08-10` |
| RFC | `R125-15-BORROW-rfc-{rfc_num}-{hash}-2026-08-10` | `R125-15-BORROW-rfc-MCP-2025-08-07-3e2f1a4-2026-08-10` |
| 博客 | `R125-15-BORROW-blog-{name}-{hash}-2026-08-10` | `R125-15-BORROW-blog-OpenAI-Engineering-8a1b2c3-2026-08-10` |
| 视频 | `R125-15-BORROW-video-{title}-{hash}-2026-08-10` | `R125-15-BORROW-video-LatentSpace-001-9c8a7b6-2026-08-10` |
| 社区 | `R125-15-BORROW-community-{name}-{hash}-2026-08-10` | `R125-15-BORROW-community-HackerNews-2a3b4c5-2026-08-10` |
| Hub | `R125-15-BORROW-hub-{name}-{hash}-2026-08-10` | `R125-15-BORROW-hub-HuggingFace-d6e5f4a-2026-08-10` |

---

## 10. R125-15 8 硬墙全守

1. ✅ **workspace.version 1.1.0** (0 改, R125 末 B2 升 1.2)
2. ✅ **R11 baseline 3 值** (0 改, A1 严守)
3. ✅ **24 LOCKED crate** (0 触碰, 调研不动 src)
4. ✅ **6 哲学锚** (0 改, B5 待 R125 末升 8 锚)
5. ✅ **9 organ** (0 改, B7 待 R125-12 内部借)
6. ✅ **11 公共 API** (0 改, R125-15 调研不动 API)
7. ✅ **0 装 (O-5)** 12 键编译期 hardcode 严守
8. ✅ **0 主动 commit** (Mavis 整合 #3 拍板)

---

## 11. R125-15 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| **arXiv 限流 PDF 下载** | R125-15a 调研延期 | 失败 1 次重试, 用 arxiv-sanity mirror |
| **官方 spec URL 变化** | R125-15b URL 失效 | 记录 archive.org snapshot |
| **会议视频无字幕** | R125-15d 借鉴困难 | 仅记录 URL + 摘要, 主人按 URL 看 |
| **社区 Discord 邀请失效** | R125-15e 加入困难 | 记录永久邀请 + web 版 |
| **100+ 借鉴 ID 严格化** 工作量大 | 1 周延 | 派 2-3 sub-agent 并行 (R125-18 借鉴 ID 严格化) |
| **6 sub-agent 并行跑** | 16 cap 16 | 6 任务 + 3 续 + 7 借鉴 = 16 满 |
| **主人 GitHub remote 未配** | R125-15 commit 难 | 0 主动 push, 留主人 1.0 release 前配 remote |

---

## 12. 0 拍板执行

### 12.1 16:46 立即执行

- [x] 写本 R125-15 spec
- [ ] Mavis runtime 下个 tick 派 6 sub-agent (R125-15a/b/c/d/e/f)
- [ ] 8/14 17:30 R125-15 全部 done, 输出 100+ 借鉴资源
- [ ] R125 末整合 #3 commit 拍板

### 12.2 R125 末 (8/31) 节点

- [ ] R125-15 6 子任务 done, 100+ 借鉴资源
- [ ] R125-16 ~ R125-21 Library 升级 6 阶段 (阶段 1-3 完成)
- [ ] Library v1.0-alpha (W3 末) — 9 大类索引 + 借鉴 ID 严格化 + _TOP_100

### 12.3 R126 / R127 续

- [ ] R125-20 Library 阶段 5 升级
- [ ] R125-21 Library v1.0 (1.0 release 礼物)
- [ ] 5 拆 crate + 4 协议 handler trait 真接
- [ ] ASI 24 维 + Skill 化 + 集成测试

---

**Mavis 16:46 状态**: 主人 6 次拍板累积 (01:14 + 01:49 + 16:27 + 16:31 + 16:37 + 16:43). R125-15 6 大类非 GitHub 学习途径 spec ready. 6 sub-agent 派活 7 天周期. 100+ 借鉴资源输出. Library 升级 6 阶段 spec ready. R125 末 36 任务派活清单 (12 借鉴 + 6 R125-15 子 + 6 Library 升级 + 12 续). 17:30 整合 #3 commit 拍板. 0 主动 commit, 0 越界 8 硬墙, 主人 1.0 release 路线图清晰.
