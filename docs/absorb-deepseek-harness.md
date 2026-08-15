# DeepSeek Harness 吸收笔记（2026-08-16）

> 来源：`deepseek-harness-master`（DSH = 本 GUI 的宿主框架，MIT 许可）。
> 注意：DSH 是 **TypeScript** monorepo（非 Rust），吸收 = 思想迁移 + Rust 重写（MIT 可参考语义，保留版权头；vendored cordis 等第三方许可需另行核对 THIRD_PARTY_NOTICES.md）。

## 已落地（本会话）

- ✅ **Spill（工具结果溢出）** — commit `be33d8a8`：`apeireth-companion::spill::SpillStore` + `ToolBridge::with_spill`，结果 >2000 字符自动溢出到会话私有文件（独占写 wx 防 symlink、文件名 sanitize 防穿越、越界读拒绝），messages 只留 `{spilled, path, hint}`。对应 DSH 清单第 9 项。

## 可吸收清单（按价值排序，子代理源码验证）

1. **事件溯源会话 + 派生 Surface + 崩溃修复**（最高价值，命中 continuity_id 哲学）
   - 单一 append-only 事件日志 = 唯一真相；模型历史 = surface 投影（append/replace）；恢复=重放、fork=seed 前缀、压缩=surface replace、崩溃=合成闭包事件（TOOL_NOT_STARTED / TOOL_OUTCOME_UNKNOWN + turn/end{interrupted}）
   - Apeireth 接法：continuity_id 升格持久会话头（parentSession/seedLength/delegationDepth）；SQLite 存事件行而非两份历史
2. **工具执行三段瀑布 + 并行调度 + abort 合成结果**
   - pre-execute(allow/deny/ask) → execute(around) → post-execute(accept/replace/block) → result(emit)；guard 只能 deny 不能 force-allow；并行结果按模型顺序提交
3. **沙盒分级梯子 + 执行前升级审批**
   - read-only → workspace-write → danger-full-access 严格更宽；sandbox_permissions+justification 成对；未更宽请求零审批打扰；每会话模式 fold 靠 replay 恢复
4. **审批 fail-closed + turn-enclosed 审计对 + 策略投影**
   - never 策略在服务自身路径先判定；审计对必须落会话边界；策略注入 runtime-context
5. **Compaction = surface replace + KV-cache 复用 + 工具配对边界**
   - 摘要调用复用会话前缀保 cache；compaction/start 即持久锁；永不拆 tool-call/result 对
6. **Goal 严格状态机 + 轮驱动**
   - revision+1 + 相位迁移逐条校验；queued/claimed/admitted/stale/cancelled 五标志 + maxGoalRounds block
7. **多 agent 编排：可续子 agent + 深度持久化 + 归因不混淆**
   - delegationDepth 只加深不降低；report/relay/notice 三种 source 区分
8. **Workflow「致命错误 vs 条目失败」纪律**
   - 脚本自身错误炸掉；子 agent 失败映射 per-item null
9. ✅ 已落地（见上）
10. **System prompt 有序 section + 分层覆盖 + 严格变量**
11. **LLM 重试：退避 + providerRetryAfter + 重试事件持久化**（配合 Apeireth 断点续传）
12. **Token 计量启发式（4 字符/token）+ spill/compaction 联动**

## 诚实标注（DSH 自身缺口）

- 会话日志版本迁移：有文档无实现（拒绝+无迁移）
- token-meter：估算非真实分词
- native/landlock-run：本仓库无 Rust 源码（仅预编译平台包）
- 外部 CLI 子 agent 适配器深度未验

## 建议落地顺序（醒来挑）

1. 工具三段瀑布改造 ToolBridge（第 2 项，直接强化现有）
2. 审批 fail-closed 强化（第 4 项，小而契合）
3. Goal 状态机（第 6 项，正对「AI 长出它自己想要什么」哲学）
4. 事件溯源会话底座（第 1 项，最大工程，需整轮做）
