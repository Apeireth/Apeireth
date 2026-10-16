# 项目治理 (Governance)

> Apeireth 社区治理章程。定义角色、决策机制、晋升退出与争议解决——让协作有章可循，让贡献者知道"找谁、怎么定、不服怎么办"。
>
> 本文件本身接受 PR 修订，修订按本文的「重大变更」流程走（见 [决策机制](#决策机制)）。

## 独立性原则

Apeireth 是**完全独立的开源项目**，不隶属于任何外部项目、组织或生态联盟。本项目：

- 以事实与质量立身，**禁止买星、刷榜、刷评论、伪造活动数据**等任何形式的社区数据造假（见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) 社区诚信条款）；
- 对外评价其他项目时对事论事、基于实测，不拉踩、不虚假背书；
- 合作（如有）仅限具体技术协作，每个合作须公开声明利益关系，且不影响本项目的技术独立判断。

## 角色阶梯

| 角色 | 定义 | 获得方式 | 权责 |
|---|---|---|---|
| **参与者** | 任何人 | — | 提 Issue、参与讨论、提交 PR |
| **贡献者 (Contributor)** | 合并过至少 1 个 PR | PR 被合并即自动获得 | 同上；可认领 `good-first-issue` 之外的常规任务 |
| **审阅者 (Reviewer)** | 能对某领域做技术把关 | Maintainer 提名 + 贡献记录 | 对本领域 PR 给出绑定性 review 意见（per [CODEOWNERS](CODEOWNERS) 分组） |
| **提交者 (Committer)** | 长期高质量贡献 | Maintainer 提名 + 社区公示 ≥ 7 天无异议 | 合并本领域非重大 PR |
| **维护者 (Maintainer)** | 跨领域把关 + 社区运营 | 现任 Maintainer 一致同意（见[晋升](#晋升与退出)） | 合并 PR、定优先级、执行行为准则、参与 RFC 裁决 |
| **项目负责人 (Project Lead)** | 最终裁决人 | 创始角色 | 重大事项最终裁决、发布签发、治理修订否决权 |

当前角色登记（诚实口径：以实际履行为准，空缺不装满）：

| GitHub | 角色 | 覆盖领域 |
|---|---|---|
| [@chuling](https://github.com/chuling) | 项目负责人 / Maintainer | 全域（per [CODEOWNERS](CODEOWNERS)） |
| _Reviewer / Committer 名单_ | **待增补** | 按 `CODEOWNERS` 分组逐域开放 |

> 随贡献者成长，本表随 PR 更新。任何人达到标准都可提名进入本表——没有封闭的小圈子。名单增补的排期跟踪于 [社区运营手册](docs/development/community-ops.md) §八 挂账事项。

## 决策机制

按影响面分三级，**能懒共识的不投票，能小范围定的不惊动全社区**：

1. **日常决策**（bug 修复、文档、小重构）：[懒共识](https://www.apache.org/foundation/glossary.html#LazyConsensus)——提出方案后公示，**72 小时内无实质反对即视为通过**。Reviewer 的 binding review 意见等同决策。
2. **技术决策**（公共结构/接口变更、crate 边界调整、重大依赖引入、性能口径变更）：
   - 走 [RFC 流程](docs/development/rfc-process.md)：提案 → 讨论期（≥ 7 天）→ Maintainer 裁决（共识优先，僵持时投票，一人一票，简单多数，Lead 有决定票）；
   - 公共结构改动仍受工程纪律约束：`grep` 所有构造点 + `cargo check --workspace --all-targets`（见 [CONTRIBUTING.md](CONTRIBUTING.md)）。
3. **重大变更**（治理修订、协议变更、方向调整、发布签发、社区驱逐）：Maintainer 全体讨论 + Lead 终审，公示 ≥ 7 天，决议记入 [`docs/04-internal/design-intent.md`](docs/04-internal/design-intent.md) 拍板历史。

**记录纪律**：一切非琐碎决策进文档或 RFC 记录，不散落聊天记录（与项目「调研入台账」纪律一致）。

## 晋升与退出

**晋升**：

- 贡献者 → Reviewer/Committer：由现任 Maintainer 提名，依据合并记录与领域判断力，公示 ≥ 7 天，社区无实质异议即生效；
- Reviewer/Committer → Maintainer：由 Maintainer 全体一致同意；**主动邀请制 + 自荐制并行**——贡献突出者可自荐，Maintainer 必须在 14 天内给出接受或具体差距说明，不允许无理由拖延。

**退出**：

- 自愿退出：声明即生效，感谢其贡献；历史权限按最小化原则回收；
- **不活跃退出**：连续 6 个月无活动（无 commit/review/讨论）且无说明者，转为 Emeritus（荣誉退休）身份，权限回收；回归可申请恢复，无需从头再来；
- **强制移除**：严重违反行为准则、或长期损害项目利益，经 Maintainer 讨论（当事人回避）+ Lead 终审执行。

## 争议解决

按以下顺序升级，**每一级都有时限，不允许无限期搁置**：

1. **直接沟通**：技术分歧先在 RFC/PR 里就事论事对齐（72 小时）；
2. **第三方介入**：请一名未参与争议的 Reviewer/Maintainer 调解（7 天内给出意见）；
3. **Maintainer 裁决**：调解无效则 Maintainer 集体讨论裁决（7 天内）；
4. **Lead 终审**：对裁决不服可上诉至 Project Lead，Lead 的决定为最终决定。

涉及行为准则的事件按 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) 的报告渠道处理，与技术争议分线。

## 与仓库机制的对应关系

| 治理概念 | 仓库落地 |
|---|---|
| 领域 owner 与绑定性 review | [CODEOWNERS](CODEOWNERS) |
| 提交者身份声明 | `Signed-off-by`（等效 DCO，见 [CONTRIBUTING.md](CONTRIBUTING.md)） |
| 技术决策记录 | [RFC 流程](docs/development/rfc-process.md) + [`docs/04-internal/design-intent.md`](docs/04-internal/design-intent.md) |
| 安全事件分线 | [SECURITY.md](SECURITY.md) / `.well-known/security.txt` |
| 新人动线 | [`docs/02-guides/first-contribution.md`](docs/02-guides/first-contribution.md) |
| 社区运营与数据真实性纪律 | [`docs/development/community-ops.md`](docs/development/community-ops.md) |

## 承诺

- 治理规则**先于权力**：规则对 Maintainer 同等生效；
- 一切治理修订公开进行，有记录、可追溯、可回滚；
- 没有永远正确的决定，只有永远诚实的记录。
