# yoyo-evolve / yoyo-gasp 深读笔记（2026-08-16，源码验证）

来源：`Downloads\ref\yoyo-evolve`（自演化编码 agent 真身，210 文件）+ `yoyo-gasp`（GASP 状态仓库）。
结论均来自源码（evolve.sh 3157 行 / skill_evolve.sh / gasp.rs / safety.rs / main.rs 等）。

## 一、核心概念（对 Apeireth 哲学的最强印证）

**executor 可替换 + 状态独立于 executor**（GASP）：
- `yoyo-evolve` = 代码（executor，可被 AI 自己重写，可丢弃）
- `yoyo-gasp` = 自我（identity/skills/memory/state/events.jsonl，append-only 可审计，可移植）
- 「演化循环保持鲁莽，持久的自我保持可审计」——**这正是我们「自升级必须在沙盒/门内做」的工程答案**：把「AI 改自己代码」和「AI 的自我身份」分离

## 二、演化循环真身（✅ 真实现，最可吸收）

```
evolve.yml(3h cron) → evolve.sh → A1 评估 → A2 规划(task_*.md)
→ Phase B 实现(≤2 任务/会话, 1800s 预算)
→ 机械闸(protected files + build/test/clippy + fix loop + innocence check)
→ B-eval LLM 判官(结构化 verdict + 4 维 checklist, FAIL 覆盖 PASS)
→ promote 或 git reset --hard PRE_TASK_SHA 回滚 + revert receipt
→ journal → reflection(learnings.jsonl) → audit.jsonl → push
```

## 三、验证结论表

| 项 | 结论 | 关键证据 |
|---|---|---|
| 演化循环 | ✅ 真 | evolve.sh 全链；AI 用自身工具改 src/*.rs |
| executor/状态分离 | 🟡 半成品 | 双仓库真；但 recorder 是 dead_code、身份文件在 executor 仓、恢复需额外二进制 |
| 安全门 | ✅ 多层 | protected files / dirty-tree guard / no-progress / 预算门 / innocence / revert receipt / 审计；**但 piped mode auto_approve=true，无沙盒，靠事后闸+回滚** |
| 工具系统 | ✅ | 静态 Vec<Box<dyn AgentTool>> + 装饰器 + AuditHook（无动态注册，我们 ToolBridge 更强） |
| 记忆 | ✅ | memory.json + learnings.jsonl 追加 + active_learnings.md 时间加权合成 |
| goal | ✅ 5 个 | self_improvement/product_value/skill_quality/community/dreaming（continuity 代码里不存在）|
| 技能循环 | ✅ 真 | skill-evolve SKILL.md 486 行 spec（生命周期状态/EMA 评分/A-B 子代理 eval/expected: 预测行）|

## 四、吹牛清单（诚实）

- 「200 行 Rust / 零人类代码」→ 实际 130k 行，人类写的 IDENTITY/PERSONALITY/7 core skills/harness **全部受保护 AI 不能改**
- README「8h」→ 实际 3h cron
- 「77 files / 115k 行 / 4300 tests」→ 实际 85 文件 / 130,291 行 / 4,939 test（低估）

## 五、对 Apeireth 可吸收（按价值排序）

1. **验证闸门流水线 + 回滚骨架**（能力演化回路后半段的蓝图）：
   任务≤3 文件/30 分钟、safety commit、innocence check、no-progress 停循环、预算门 fail-open 保留绿态、unverified receipt
2. **双层 eval + 命名 checklist 覆盖合同**：机械闸 + LLM 判官 4 维 checklist，FAIL 覆盖 PASS，降级不 fail-closed——**宪法评审可借鉴**（我们当前评审失败=保守拒绝，这里提供「降级但留痕」的另一面）
3. **revert 即学习信号**：revert receipt → 下一轮 planner 输入（我们能力回滚也应回写学习）
4. **audit.jsonl 单写者 schema**：{ts, tool, args, duration_ms, success}——我们 RecordStore 可对齐
5. **GASP 事件词汇**：Goal/Run/Task/Patch/Eval/Decision 六类事件——我们 SessionLog 可扩展因果链（yoyo-gasp 的 events.jsonl 带 causation_id）
6. **skills 生命周期状态机 + expected: 预测行**——能力演化回路参考（我们 capability 状态机可加 expected 预测+评分）
7. **预算时间盒纪律**（1800s/会话）——多轮循环的预算门
8. **诚实记录**：outcome 标 model/fallback、unverified/unanswered 显式命名——0 装 PASS 的工程化

## 六、一句话

yoyo 是「AI 改自己代码」的**实证先锋**（126 天真实演化），它的闸门+回滚+审计体系是我们能力演化回路后半段的最佳参照；它的「executor 与自我分离」印证了我们「自升级须在门内」的哲学判断；它的安全弱点（无沙盒、auto_approve）恰好是我们执行体隔离+宪法评审要补的。

## 七、补充细节（二次深读）

### 技能循环的「expected: 预测行」范式（能力演化回路直接可抄）
- 每轮 refine/create 必须写 `expected:` = 信号 + 期限 + 回退动作（SKILL.md）
- 生命周期状态机 `dormant→candidate→active→refined→deprecated`；EMA 评分 `new = 0.3*blended + 0.7*old`
- refine 用 snapshot + **A/B 双子代理 eval**（candidate-better≥1 且 0 baseline-better 才提交）；create 要求 validation_case + applied≥1
- harness 强制：**diff-scope allow-list**（只有 skills/_journal.md / learnings / skills_attic / 非 core 的 SKILL.md 可动，越界 git reset 回滚）——「LLM 承诺 + harness 强制」双层

### 安全门真相
- 自演化走 piped 模式：`auto_approve = config.auto_approve || !is_interactive`（main.rs:754-755）——**非交互即全自动批准，无沙盒无逐条批准**，靠事后闸门 + git 回滚兜底（设计选择，但「安全」边界要清楚）
- 交互侧才有 26+ 条 bash 危险启发式 + 权限 glob + 目录限制

### 双层 eval（宪法评审可借鉴）
- 机械层（build/test/clippy + innocence check + fix loop）+ LLM 判官层（只判 diff，**命名 checklist 覆盖合同**：intent_alignment/forgotten_touchpoints/doc_sync/product_surface，任一 FAIL 覆盖 PASS）
- 判官基础设施故障 → **fail-open**（跳过 eval 保留绿态 + 记录），不因判官迟到回滚好代码

### 诚实记录工程化
- outcome.json 标注实际服务的 model + fallback_phases；unverified/unanswered 显式命名落盘
- 轨迹感知注入：extract_trajectory.py 聚合 outcome/revert/CI 错误簇注入规划 prompt

### Apeireth 已超越 / 不适用
- **已超越**：yoyo 的 GASP 进程内记录器 dormant（事件只在 session-end 一次性镜像）——我们有运行中的 SessionLog；yoyo 无宪法评审层（identity 是人类写死的受保护文件）——我们有 E 层判案；yoyo 无执行体隔离（bash 全权限单 runner）——我们有 per-call 子进程
- **不适用**：GitHub issue 社区回路 / sponsor 经济学 / fork 即分身 / Actions 绑定的演化时钟（yoyo 的公共产品化生存结构，Apeireth 不需要）

### 吹牛总清单
- 「200 lines of Rust. Zero human code.」→ 130k 行，人类写的身份/7 core skills/harness 全部受保护 AI 不能改（历史起源包装成现况）
- 数字过时（低估）：77 文件/115k 行/4300 tests → 实测 85 文件/130,291 行/4,939 tests
- 「Every ~8 hours」→ 实际 3h cron；「clone+fold 恢复」→ 需 executor 二进制+身份文件+密钥；「evolve or die / 自己决定一切」→ 演化方向被人类 harness 强约束（这本身是优点）
