# 社区生态运营手册 (Community Ops)

> 生态建设的运维手册：标签体系、分诊 runbook、运营节奏、仓库元数据，以及**数据真实性纪律**。
>
> 核心教训（行业复盘）：**社区文件齐 ≠ 社区活**。本手册的重心不在"有没有文件"，而在"有没有回音、有没有新人、有没有可信的数据"。

> **2026-11-11 更新**：现行社区入口与发布前缺口见 §九；邮箱尚未启用，行为举报与申诉私密渠道待确认。普通用户从 [SUPPORT.md](../../SUPPORT.md) 开始，不要求先了解工程架构。

## 一、数据真实性纪律（最重要，先立规矩）

社区数据是生态的信用货币。我们承诺：

1. **禁止任何形式的数据造假**：不买星、不刷榜、不刷 issue/评论、不伪造贡献者活动、不人为制造 commit 热度（见 [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) 社区诚信条款，违反即违反行为准则）；
2. **指标如实呈现**：对外引用星标/下载/测试数/性能数时注明口径与日期，未达标处照实写差距（与「0 装 PASS」同源）；
3. **不虚假背书**：不为任何未实测的能力/合作/生态繁荣做宣传；
4. **星标与活跃度失真是红旗**：若出现外部数据异常（如星标激增但无对应社区活动），主动在 Discussion 说明情况，不消费虚假繁荣。

> 一次造假要用十次诚实来偿还。我们的生态宁可长得慢，长得是真的。

## 二、标签体系 (Labels)

### 分诊标签（必配）

| 标签 | 含义 | 动作 |
|---|---|---|
| `bug` | 缺陷确认 | 进修复队列 |
| `enhancement` | 功能请求 | 接受/暂缓/拒绝 + 理由 |
| `good-first-issue` | 新人任务：范围清晰、有指引 | 保持存量（见运营节奏） |
| `help-wanted` | 求助：非新人向、需要领域知识 | 认领制 |
| `needs-repro` | 缺复现步骤 | 提问后 14 天无回应则 `stale` |
| `needs-info` | 信息不足 | 同上 |
| `1.0-blocker` | 历史 1.0 发布阻塞标签 | 保留旧记录；不再作为当前新建问题入口 |
| `docs` | 文档任务 | 文档 owner 处理 |
| `rfc` | 走 RFC 流程的提案 | 链接 `docs/development/rfc/` |
| `stale` | 14 天无活动 | 自动/人工关闭前提醒一次 |

### 状态标签（可选）

`blocked`（等外部依赖）、`wontfix`（明确不做，留理由）、`duplicate`（链向原 issue）。

**规则**：每个 open issue 至少有 1 个分诊标签；关闭必须有理由（关闭理由就是给下一个搜索者看的文档）。

## 三、Issue 分诊 Runbook

```
新 issue 进入
  ↓ 目标：≤ 3 个工作日（bug）/ ≤ 7 个工作日（enhancement）
分诊（打标签 + 给结论）
  ├─ 确认 → 进队列，标注优先级
  ├─ 需复现/信息 → 提问 + needs-repro/needs-info（14 天倒计时）
  ├─ 非问题/不做 → 关闭 + 说清为什么
  └─ 讨论型 → 引导至 Discussion 或 RFC
```

**应避免的状态**：长期无人回应、只打标签不给结论。时限为维护目标，不是固定服务保证；未能及时处理时应在原帖说明。PR 提醒方式见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

## 四、运营节奏（活性机制——生态的真正区别）

| 事项 | 频率 | 目标 |
|---|---|---|
| `good-first-issue` 存量巡检 | 每两周 | 保持 ≥ 5 个 open 的新人任务（被认领了就补） |
| Issue/PR 积压回顾 | 每月 | 回顾超过响应目标的事项；stale 清理 |
| 新贡献者跟进 | 每个新 PR | 首个 PR 合并后欢迎 + 引导下一步（进 GOVERNANCE 角色阶梯） |
| Discussion 话题运营 | 持续 | 设计讨论、路线图预告、版本沟通 |
| 数据真实性自查 | 每季度 | 引用的指标口径与日期是否仍准确 |

> **为什么写进手册**：同类项目复盘显示，最常见的失败模式是"发布即静止"——文件齐全、模板漂亮，但三个月无提交、无人应答、星标与活动严重失真。**有节奏的运营**是防线，不是装饰。

## 五、仓库元数据 Runbook

GitHub 仓库设置（无文件可管，故入册于此，防止"仓库描述为空"这类可发现性失误）。以下保留 **2026-10-11 配置记录**，不视为对当前全部设置的重新核验；2026-11-11 实查见 §九：

- [x] 仓库描述（英文一句话定位，✅ 已有）
- [x] Topics 标签（✅ 10 个：`agi` `ai-agents` `autonomous-agents` `cognitive-architecture` `microkernel` `operating-system` `pure-safe-rust` `rust` `topological-memory` `zero-trust`）
- [x] README 双语入口（✅ 已有）
- [x] 社区健康文件：CONTRIBUTING ✅ / CODE_OF_CONDUCT ✅ / SECURITY ✅ / GOVERNANCE ✅ / Issue 模板 ✅ / PR 模板 ✅
- [x] Discussions 已开启 + welcome 帖（[Discussion #26](https://github.com/Apeireth/Apeireth/discussions/26)）
- [x] 标签体系已建（`good-first-issue` / `help-wanted` / `docs` 改名对齐手册 + `needs-repro` / `needs-info` / `stale` / `rfc` / `1.0-blocker` / `blocked` 新建）
- [x] 分支保护：main 需 **≥ 1 approval**（stale review 自动失效 + 会话必须 resolve + 禁 force-push）+ 必需检查 5 门禁：`cargo fmt --check` / `cargo deny check (...)` / `8 硬墙守门 (LOCKED ...)` / `Secret scan (...)` / `Guard 3.3 ...`（选取口径：**每个 PR 必跑（无 paths 过滤）且当前稳定绿**）

> **分支保护口径说明（诚实记录）**：main 最新提交尚有一批 CI 红检查（nextest / miri / kani / doc-test 矩阵等），若全部设必需将卡死一切合并。故现阶段只强制上述 5 个稳定门禁；**红检查修复后应逐步纳入 required set**，最终目标是"CI 全绿才能合并"（登记为持续改进项）。

## 六、响应目标与治理期限

| 承诺 | 时限 | 出处 |
|---|---|---|
| Issue 首次分诊目标 | 3 / 7 个工作日（按类型） | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| PR 首次响应目标 | ≤ 3 个工作日 | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| PR 超时升级 | 5 个工作日可 @chuling | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| RFC 裁决 | 讨论期 ≥ 7 天，裁决 ≤ 14 天 | [rfc-process.md](rfc-process.md) |
| 行为准则事件 | 私密渠道待确认，暂不承诺收件与响应时限 | [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) |
| 不活跃维护者转 Emeritus | 连续 6 个月无活动 | [GOVERNANCE.md](../../GOVERNANCE.md) |

**工作日定义**：周一至周五（UTC+8），法定节假日顺延。Issue/PR 时限是维护目标，不构成全天候服务保证；做不到时应尽量先说明排队情况。RFC 与角色治理期限仍以所链治理文件为准。

## 七、反面教训清单（复盘存档）

来自同类开源项目的公开复盘，逐条设防：

| 教训 | 风险 | 我们的防线 |
|---|---|---|
| 缺 Issue 模板 | Bug 报告无复现、分诊瘫痪 | 模板全覆盖 + blank issue 关闭 |
| 星标与提交活跃度严重失真 | 信誉崩塌 | [数据真实性纪律](#一数据真实性纪律最重要先立规矩) |
| 文件齐全但无人应答 | 新人流失、"发布即静止" | 响应目标 + 提醒机制 + [运营节奏](#四运营节奏活性机制生态的真正区别) |
| 无 topics/描述 | 可发现性差 | 元数据 Runbook |
| 协作承诺无治理支撑 | 争议无解、维护者倦怠 | [GOVERNANCE.md](../../GOVERNANCE.md) + RFC |
| 单向发布、无外部贡献回流 | 伪生态 | 新人动线 + good-first-issue 存量 + 认领制 |

## 八、挂账事项（待办台账，防遗忘）

> 按「调研入台账」纪律：生态线未竟事项**集中在本节登记**，不散落聊天记录。完成即勾销并注明日期；新增事项先入本表再排期。

| # | 事项 | 触发时机 | 落地动作 | 状态 |
|---|---|---|---|---|
| 1 | **社区行为举报与申诉私密入口** | 正式开放社区前确认 | 项目负责人确认实际收件入口与接收人 → 经授权发送无敏感信息的测试并回读 → 明确举报涉及接收人时的回避与复核安排 → 同步行为准则、支持页与官网 | ⏳ 待确认：2026-11-11 用户明确 `apeireth-security@apeireth.org` 尚未启用；无可用 GitHub 私信兜底，不承诺 48h 响应 |
| 2 | **`stale` 标签自动化** | 社区活跃、issue/PR 流量上来后（人工巡检开始吃力时） | GitHub Actions scheduled job（如 `actions/stale` 或自写 workflow）：按 §二 规则 **14 天无活动打 `stale`**、关闭前提醒一次、豁免 `1.0-blocker`/`blocked`；参数与豁免规则写进 workflow 注释 | ⏳ 待办（现阶段人工巡检，见 §四 运营节奏） |
| 3 | **分支保护 required set 扩容** | main 红 CI 修复一批后 | 将修复后的稳定门禁逐个加入分支保护必需检查，目标"CI 全绿才能合并"（见 §五 口径说明） | ⏳ 持续改进 |
| 4 | **稳定级清单 RFC** | 1.0 发布前 | 逐 crate 评审 🟡/🟢 升级并登记 [api-stability.md](api-stability.md)，消除"整体未稳定"模糊地带 | ⏳ 待办 |
| 5 | **Reviewer / Committer 名单增补** | 首批外部贡献者成长起来 | 按 [GOVERNANCE.md](../../GOVERNANCE.md) §晋升流程提名公示，登记角色表（自荐制并行，Maintainer 14 天内必答） | ⏳ 待增补 |
| 6 | **Discussions 板块细分** | 已具备 | Q&A / Ideas / Show and tell 已存在；支持页与 Issue chooser 直接分流 | ✅ 2026-11-11 GitHub GraphQL 已核验分类与 slug |

> 维护约定：每次生态线例会/季度自查（§四）过一遍本表；挂账超过触发时机未动 = 运营欠账，如实记录不装没看见。

## 九、官网与社区入口收口（2026-11-11）

### 目标与边界

面向普通用户提供清楚的问答、建议、缺陷、分享与安全入口，开发者继续使用贡献指南。规则必须反映实际可用的联系方式与处理能力。此工作线只改社区文档和表单，不修改产品运行时、安全边界、LOCKED 项或依赖。官网在独立 Sites 源码仓库维护，本站改动不代表 GitHub 文档已合入默认分支。

### 当前核验与实现

| 项目 | 证据与状态 |
|---|---|
| 仓库 | `Apeireth/Apeireth`，默认分支 `main`；2026-11-11 审核基线 `72f826634cf09730cb72b82b5880533f546ccd54` |
| Issues / Discussions | GitHub REST 回读均已开启；GraphQL 返回 Announcements、General、Ideas、Polls、Q&A、Show and tell，welcome 帖为 [#26](https://github.com/Apeireth/Apeireth/discussions/26) |
| 社区文件 | GitHub community profile 为 100%；仍发现联系方式不实、重复旧模板与普通用户入口不清楚，文件齐全不能替代内容审核 |
| 私密漏洞报告 | REST `private-vulnerability-reporting` 返回 `enabled: true`；统一指向 [Report a vulnerability](https://github.com/Apeireth/Apeireth/security/advisories/new)。未发送测试漏洞报告 |
| 行为举报与申诉 | 用户确认邮箱未启用；文档明确待确认，移除 GitHub 私信指引；平台违规可按 GitHub 平台渠道报告，不宣称项目申诉入口已可用 |
| 支持页与贡献指南 | 新增根目录 `SUPPORT.md`，区分问答与工程贡献；工程要求不再作为普通提问前置条件 |
| 问题表单 | 保留两份中文 YAML 表单；移除重复旧 Markdown 与 1.0 blocker 模板；默认标签仅用实存的 `bug` / `enhancement`，不使用不存在的 `triage` |
| 安全政策与 security.txt | 主入口改为 GitHub 私密报告；移除 `.local` 邮箱、非报告加密用途的签名公钥与未经确认的 DNS Encryption；取消固定响应/修复时限和所有更新自动被 cargo audit 提醒的表述 |

### 验收与后续

- 本分支须通过严格 YAML 解析、表单字段检查、文档与本地链接检查；命令结果记入 PR，不将未运行的 Rust 全量测试记为通过。
- 新表单与文档合入 `main` 后才在默认入口生效；合入后回读 Issue chooser 与社区文件。未合入前不宣称线上已更新。
- 私密联系入口仍由项目负责人确认，验收动作见 §八第 1 项；没有实际收件与回避安排前，不勾销此缺口。
- 官网同步使用已存在的 GitHub 分类和私密漏洞入口，保留“社区行为举报与申诉渠道待确认”的说明，不链接尚未合入的新支持页。
