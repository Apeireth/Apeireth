# RFC 流程 (Request for Comments)

> 重大技术决策的提案机制：**先对齐，再动手**。RFC 让设计讨论有处落笔、有史可查——不散落在聊天记录里（与项目「调研入台账」纪律一致）。

## 什么时候需要 RFC

**需要**（技术决策级，见 [GOVERNANCE.md](../../GOVERNANCE.md) 决策机制第 2 级）：

- 公共结构/接口变更（struct/enum/签名/协议）影响面超出单一 crate；
- crate 边界调整、新 crate 引入、依赖拓扑变化；
- 新能力域（如新的 memory 后端、新的 adapter 协议）；
- 性能口径/基准方法变更；
- 其他"做错了要推倒重来"的改动。

**不需要**：

- Bug 修复、文档、小重构、局部优化 → 直接 PR（走懒共识）；
- 拿不准算不算 → 在 [Discussions](https://github.com/Apeireth/Apeireth/discussions) 问一句，维护者会告诉你。

## 流程总览

```
想法 → 草案 (Draft RFC) → 公开讨论 (≥ 7 天) → 裁决 → 实施 → 归档
```

### 1. 草案

在 `docs/development/rfc/` 新建 `rfc-NNNN-短横线标题.md`（NNNN 递增，从 0001 开始），模板：

```markdown
# RFC-NNNN: 标题

- 状态: 草案 | 讨论中 | 已接受 | 已拒绝 | 已替代 (被 RFC-XXXX 替代)
- 提案人: @你的 GitHub
- 日期: YYYY-MM-DD
- 关联: Issue #xx / 相关文档链接

## 动机 (Motivation)
为什么现在要做这件事？解决什么问题？不做的后果是什么？

## 方案 (Proposal)
具体设计：接口草案、数据流、边界划分。允许贴代码骨架，但标注哪些是示意。

## 取舍 (Drawbacks / Alternatives)
为什么不用别的方案？本方案的代价是什么？

## 兼容性与迁移 (Compatibility)
对现有 API/数据/性能的影响；迁移路径。

## 未决问题 (Open Questions)
还没想清楚的部分——如实列出，不装。
```

同时开一个对应的 Discussion/Issue 用于讨论（PR 也可以，见下）。

### 2. 公开讨论（≥ 7 天）

- 用 PR 提交 RFC 文件本身是推荐做法——讨论、修改、裁决都沉淀在 PR 的 review 历史里；
- 讨论期 **≥ 7 天**，重大者延长；提案人须回应每一条实质性质疑；
- 遵守 [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md)：对事不对人，批评带理由。

### 3. 裁决

- 目标是**共识**；出现僵持时 Maintainer 投票（一人一票，简单多数，Project Lead 有决定票）；
- 裁决结果写回 RFC 头部的「状态」字段，附一句话结论；
- **拒绝也是好结果**——写清楚为什么拒绝，未来有人再提时站在你的结论上继续。

### 4. 实施与归档

- 已接受的 RFC 拆分为实施任务（Issue），实施 PR 引用 `RFC-NNNN`；
- 实施中若发现设计要变，**更新 RFC 并记录偏离原因**，不允许无声偏离；
- RFC 文件是历史文件：状态定格后**只增不改**（除了勘误），被替代时标注替代关系。

## 与既有决策记录的关系

- 单点拍板（一次性决策、无后续讨论价值）→ 记入 [`docs/04-internal/design-intent.md`](../04-internal/design-intent.md) 拍板历史，不走 RFC；
- 多轮讨论、影响面大的设计 → RFC；
- 调研未落地 → 登记 [`docs/04-internal/backlog.md`](../04-internal/backlog.md) 台账，不散落聊天记录。

## 承诺

- 每个进入讨论期的 RFC，维护者**必给裁决**，不允许无声搁置（超 14 天未裁决可 @chuling 升级）；
- 草案写得糙没关系，**不写才是损失**。
