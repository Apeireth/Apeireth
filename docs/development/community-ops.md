# 社区生态运营手册 (Community Ops)

> 生态建设的运维手册：标签体系、分诊 runbook、运营节奏、仓库元数据，以及**数据真实性纪律**。
>
> 核心教训（行业复盘）：**社区文件齐 ≠ 社区活**。本手册的重心不在"有没有文件"，而在"有没有回音、有没有新人、有没有可信的数据"。

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
| `1.0-blocker` | 1.0 发布前必修 | 高优先分诊（≤ 1 天） |
| `docs` | 文档任务 | 文档 owner 处理 |
| `rfc` | 走 RFC 流程的提案 | 链接 `docs/development/rfc/` |
| `stale` | 14 天无活动 | 自动/人工关闭前提醒一次 |

### 状态标签（可选）

`blocked`（等外部依赖）、`wontfix`（明确不做，留理由）、`duplicate`（链向原 issue）。

**规则**：每个 open issue 至少有 1 个分诊标签；关闭必须有理由（关闭理由就是给下一个搜索者看的文档）。

## 三、Issue 分诊 Runbook

```
新 issue 进入
  ↓ ≤ 1 天（1.0-blocker）/ ≤ 3 天（bug）/ ≤ 7 天（enhancement）
分诊（打标签 + 给结论）
  ├─ 确认 → 进队列，标注优先级
  ├─ 需复现/信息 → 提问 + needs-repro/needs-info（14 天倒计时）
  ├─ 非问题/不做 → 关闭 + 说清为什么
  └─ 讨论型 → 引导至 Discussion 或 RFC
```

**不允许的状态**：无人回应、只打标签不给结论、issue 黑洞。SLA 超时由 [CONTRIBUTING.md](../../CONTRIBUTING.md) 的升级机制兜底（@chuling）。

## 四、运营节奏（活性机制——生态的真正区别）

| 事项 | 频率 | 目标 |
|---|---|---|
| `good-first-issue` 存量巡检 | 每两周 | 保持 ≥ 5 个 open 的新人任务（被认领了就补） |
| Issue/PR 积压回顾 | 每月 | 无超 SLA 项；stale 清理 |
| 新贡献者跟进 | 每个新 PR | 首个 PR 合并后欢迎 + 引导下一步（进 GOVERNANCE 角色阶梯） |
| Discussion 话题运营 | 持续 | 设计讨论、路线图预告、版本沟通 |
| 数据真实性自查 | 每季度 | 引用的指标口径与日期是否仍准确 |

> **为什么写进手册**：同类项目复盘显示，最常见的失败模式是"发布即静止"——文件齐全、模板漂亮，但三个月无提交、无人应答、星标与活动严重失真。**有节奏的运营**是防线，不是装饰。

## 五、仓库元数据 Runbook

GitHub 仓库设置（无文件可管，故入册于此，防止"仓库描述为空"这类可发现性失误）。**2026-10-11 已全部配置完成**（经 GitHub API 配置并回读验证）：

- [x] 仓库描述（英文一句话定位，✅ 已有）
- [x] Topics 标签（✅ 10 个：`agi` `ai-agents` `autonomous-agents` `cognitive-architecture` `microkernel` `operating-system` `pure-safe-rust` `rust` `topological-memory` `zero-trust`）
- [x] README 双语入口（✅ 已有）
- [x] 社区健康文件：CONTRIBUTING ✅ / CODE_OF_CONDUCT ✅ / SECURITY ✅ / GOVERNANCE ✅ / Issue 模板 ✅ / PR 模板 ✅
- [x] Discussions 已开启 + welcome 帖（[Discussion #26](https://github.com/Apeireth/Apeireth/discussions/26)）
- [x] 标签体系已建（`good-first-issue` / `help-wanted` / `docs` 改名对齐手册 + `needs-repro` / `needs-info` / `stale` / `rfc` / `1.0-blocker` / `blocked` 新建）
- [x] 分支保护：main 需 **≥ 1 approval**（stale review 自动失效 + 会话必须 resolve + 禁 force-push）+ 必需检查 5 门禁：`cargo fmt --check` / `cargo deny check (...)` / `8 硬墙守门 (LOCKED ...)` / `Secret scan (...)` / `Guard 3.3 ...`（选取口径：**每个 PR 必跑（无 paths 过滤）且当前稳定绿**）

> **分支保护口径说明（诚实记录）**：main 最新提交尚有一批 CI 红检查（nextest / miri / kani / doc-test 矩阵等），若全部设必需将卡死一切合并。故现阶段只强制上述 5 个稳定门禁；**红检查修复后应逐步纳入 required set**，最终目标是"CI 全绿才能合并"（登记为持续改进项）。

## 六、SLA 汇总（对社区的承诺一览）

| 承诺 | 时限 | 出处 |
|---|---|---|
| Issue 首次分诊 | 1 / 3 / 7 天（按类型） | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| PR 首次响应 | ≤ 3 个工作日 | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| PR 超时升级 | 5 个工作日可 @chuling | [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| RFC 裁决 | 讨论期 ≥ 7 天，裁决 ≤ 14 天 | [rfc-process.md](rfc-process.md) |
| 行为准则事件 | 尽快，私密处理 | [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) |
| 不活跃维护者转 Emeritus | 连续 6 个月无活动 | [GOVERNANCE.md](../../GOVERNANCE.md) |

**工作日定义**：周一至周五（UTC+8），法定节假日顺延；社区承诺"必有回音"，时限是纪律不是机器——做不到时先回一句"已看到，排队中"。

## 七、反面教训清单（复盘存档）

来自同类开源项目的公开复盘，逐条设防：

| 教训 | 风险 | 我们的防线 |
|---|---|---|
| 缺 Issue 模板 | Bug 报告无复现、分诊瘫痪 | 模板全覆盖 + blank issue 关闭 |
| 星标与提交活跃度严重失真 | 信誉崩塌 | [数据真实性纪律](#一数据真实性纪律最重要先立规矩) |
| 文件齐全但无人应答 | 新人流失、"发布即静止" | SLA + 升级机制 + [运营节奏](#四运营节奏活性机制生态的真正区别) |
| 无 topics/描述 | 可发现性差 | 元数据 Runbook |
| 协作承诺无治理支撑 | 争议无解、维护者倦怠 | [GOVERNANCE.md](../../GOVERNANCE.md) + RFC |
| 单向发布、无外部贡献回流 | 伪生态 | 新人动线 + good-first-issue 存量 + 认领制 |

## 八、挂账事项（待办台账，防遗忘）

> 按「调研入台账」纪律：生态线未竟事项**集中在本节登记**，不散落聊天记录。完成即勾销并注明日期；新增事项先入本表再排期。

| # | 事项 | 触发时机 | 落地动作 | 状态 |
|---|---|---|---|---|
| 1 | **行为准则专用邮箱**（`conduct@`） | 社区规模扩大：出现定期外部贡献者 / 线下活动 / 多 Maintainer | 注册转发别名（**Lead + 另一名 Maintainer 双收件**，落实回避原则——举报对象是其一时另一人接手）→ 更新 [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) §报告与申诉渠道 → 勾销本行 | ⏳ 待办（现阶段兜底渠道真实有效：`apeireth-security@apeireth.org` / DM @chuling；**响应承诺先于地址**：收件 48h 内人工回应） |
| 2 | **`stale` 标签自动化** | 社区活跃、issue/PR 流量上来后（人工巡检开始吃力时） | GitHub Actions scheduled job（如 `actions/stale` 或自写 workflow）：按 §二 规则 **14 天无活动打 `stale`**、关闭前提醒一次、豁免 `1.0-blocker`/`blocked`；参数与豁免规则写进 workflow 注释 | ⏳ 待办（现阶段人工巡检，见 §四 运营节奏） |
| 3 | **分支保护 required set 扩容** | main 红 CI 修复一批后 | 将修复后的稳定门禁逐个加入分支保护必需检查，目标"CI 全绿才能合并"（见 §五 口径说明） | ⏳ 持续改进 |
| 4 | **稳定级清单 RFC** | 1.0 发布前 | 逐 crate 评审 🟡/🟢 升级并登记 [api-stability.md](api-stability.md)，消除"整体未稳定"模糊地带 | ⏳ 待办 |
| 5 | **Reviewer / Committer 名单增补** | 首批外部贡献者成长起来 | 按 [GOVERNANCE.md](../../GOVERNANCE.md) §晋升流程提名公示，登记角色表（自荐制并行，Maintainer 14 天内必答） | ⏳ 待增补 |
| 6 | **Discussions 板块细分** | welcome 帖活跃后按需 | 依讨论量开设 Q&A / Ideas / Show and tell 分类，运营 §四 频率表同步 | ⏳ 按需 |

> 维护约定：每次生态线例会/季度自查（§四）过一遍本表；挂账超过触发时机未动 = 运营欠账，如实记录不装没看见。
