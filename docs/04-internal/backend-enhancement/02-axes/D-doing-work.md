# D. 干活能力

> `Status: ♻️ 历史+实施记录（隶属 00-backlog #D；2026-10-20 第三线源码对账：P3 任务编排已交付 V6-a，见文末对账注）`

## 现状
- 已有：apply_patch 受控写（写总闸+观测门禁+审批链+IM 卡）、shell（沙箱+写意图扫描）、repo 只读、MCP 外部工具（审批级）、**git 写分级 P1 已上架**（`tool.git_write`：L1 可逆默认人批/可开自动放行档、L2 出网永远人批、L3 毁灭无工具面，交付记录 [git-write-tiering-delivery.md](../git-write-tiering-delivery.md)）、**worktree_apply 受控写 P2 已上架**（`tool.worktree_plan` 只读改单零审批 + `tool.worktree_apply` 四拍受控写：计划→改→验证→落，暂存面执行/观测门禁/写总闸语义/落拍全有或全无，交付记录 [worktree-apply-delivery.md](../worktree-apply-delivery.md)）；
- 缺：真·并发工作树隔离（P3 任务编排已交付 V6-a：`orchestration::task_queue` + `tool.task_board`，默认关，交付记录 [V6-a-task-queue-delivery.md](../V6-a-task-queue-delivery.md)；工作树级并发隔离仍待）。

## 想法
从"能看"到"能干活"：
1. **worktree_apply**：工作树式改动（多文件大改）走"计划→改→验证→落"四拍，每拍可审 **✅ 已交付**（四拍时序图/治理语义/测试证据见 [worktree-apply-delivery.md](../worktree-apply-delivery.md)）；
2. **git 写分级**：commit 级（add/commit/建分支）走审批（可设自动放行档）、push 永远要人批、毁灭级（reset --hard/强推）**不给工具**；
3. **任务编排**：长任务拆步执行（计划模式+任务目标的生产化——折子戏/目标续行已有语义）；
4. **MCP 深化**：多服务器管理界面、工具发现治理（新工具默认审批级）。

## 缺口
- 工作树级并发隔离（并发任务互不踩）；P3 任务编排（持久队列+幂等唤醒+崩溃恢复+人判失败）已交付 V6-a；验证回路已随 P2 验证拍落地（暂存面结构校验 + 受控通道校验命令）。

## 分期
P1 git 写分级（小件大价值）**✅ 已交付**（分级词表/接线图/测试证据见 [git-write-tiering-delivery.md](../git-write-tiering-delivery.md)）→ P2 worktree_apply **✅ 已交付**（四拍时序图/测试证据见 [worktree-apply-delivery.md](../worktree-apply-delivery.md)）→ P3 任务编排 **✅ 已交付（V6-a）**（[V6-a-task-queue-delivery.md](../V6-a-task-queue-delivery.md)）→ P4 MCP 深化 **🔧**（工具桥/catalog/治理映射 2026-09 已交付；多服务器管理界面待）。

## 验收
每级写权力过审批链+审计配对；毁灭级永远无工具；e2e 钉死。
