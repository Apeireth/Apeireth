# R32 后端升级路线图 (2026-08-08)

**作者**: Mavis
**范围**: 主人 8-04 R25 节奏延续 — "暂告段落, 优先后端"
**前置**: R30 (8 工具 + 5 协议 + 3 层) + R31 (24 个 integration test 修好)

---

## 1. 主拍 (TL;DR)

后端 1.0 有 20+ crate, 0.1.0 还有 20+ 副. 升级候选按 **实战 ROI 排序** 5 个方向, 等主人拍先做哪个. 都不动 LOCKED 边界 (R11 0 触, 8 项不修改承诺 0 触, R19 R25 R26 0 触).

| 序 | 方向 | 痛点 | 估时 | ROI | 触发条件 |
|---|---|---|---|---|---|
| 1 | **apeireth-asi 真实计算** | R19 启发式 token 估算不准, status bar "token LLM X / R19 Y" 不可靠 | 1d | ★★★★★ | 主人问 "R19 数怎么算的" |
| 2 | **apeireth-pipeline 真接 R30 tool loop** | 战役 4-1 接 R17 改瘦 (R25 改瘦 Step 1.5), 但 tool loop 还没接 | 1d | ★★★★ | 任何 R30 改动后 |
| 3 | **apeireth-eval 真实化** | 0.1.0 placeholder, 7 阶段 complete 但都 mock, 没真跑过 1 次 eval | 2d | ★★★ | 想做 release 前 smoke test |
| 4 | **apeireth-graph 真实化** | 0.1.0, 接 cognition 知识图谱 24 节点 + 30 边 | 3d | ★★ | 想看认知图谱可视化 |
| 5 | **apeireth-council 真实多 AI 投票** | 战役 0 保留, 0.1 version, 没用过 | 5d | ★ | 想做 "多 AI 投票决断" |

---

## 2. 详细说明 (前 3 个)

### 方向 1: apeireth-asi 真实计算 (★★★★★ ROI)

**痛点 (实战必踩)**:
- R19 token 估算靠启发式 `char_count / 4` (ASCII) / 1.5 (CJK), 跟 LLM 报数偏差大
- status bar "token LLM X / R19 Y" 双字段对比, 但 Y 数不真
- 主人日常用 TUI 会发现 "R19 数跟 LLM 数对不上, 我不知道哪个准"
- apeireth-asi 已有 24 维 + V1136 9 子测度真计算 API, 但 backend 没接

**方案**:
- `apeireth-tui/src/backend.rs` 替换 `r19_token_compute()` 启发式为 `apeireth_asi::AsiV05Scores::token_count()` 真计算
- 跟 R30 P4 tool loop 配合: tool_calls 也走 ASI token 算 (不重复算 input + tool args)
- 加 unit test 验证 24 维 + 9 子测度覆盖

**估时**: 1d (1 个 fn 替换 + 1 个 test module)
**不动**: R11 LOCKED enum, R19 cycle / verdicts, R25 改瘦路径

### 方向 2: apeireth-pipeline 真接 R30 tool loop (★★★★ ROI)

**痛点 (R30 刚暴露)**:
- apeireth-pipeline 是顶层 orchestrator, 战役 4-1 接 R17 改瘦 (R25 改瘦 Step 1.5)
- R30 加了 P0 function-call 回路 (chat_with_tool_loop_streaming) 在 `apeireth-tui/src/backend.rs`
- apeireth-pipeline 还没接 tool loop, 未来想用 pipeline 跑 TUI / Web / 桌面 App 共享 tool loop
- 现在 TUI 跑 tool loop, 桌面/Web 还得各自接

**方案**:
- `apeireth-pipeline/src/` 加 `tool_loop` 模块, 把 `apeireth-tui::backend::chat_with_tool_loop_streaming` 抽到 pipeline
- TUI / Web / 桌面 App 都 import pipeline tool_loop
- 加 unit test: pipeline.tool_loop(stream sender) 跟 TUI 行为一致

**估时**: 1d (1 个 refactor + 1 个 test)
**不动**: pipeline 现有 24 节点 orchestrator, R25 改瘦路径, TUI 已有 R30 实现

### 方向 3: apeireth-eval 真实化 (★★★ ROI)

**痛点 (release 前必备)**:
- 0.1.0 placeholder, 7 阶段 complete 但都 mock
- release 1.0 前没真跑过 1 次 eval pipeline
- 手工测靠 cargo test 跟手动 TUI 跑, 不可复现

**方案**:
- `apeireth-eval/src/` 真接一个 eval task (e.g. "读 Cargo.toml 第 30 行 → 验证返回 200 OK")
- 跑通 R30 8 工具各 1 个 smoke test
- 输出 7 阶段 metric (PASS/FAIL + 耗时 + token)
- 加 unit test: eval.register_task + eval.run() 端到端

**估时**: 2d (1 个 eval task + 1 个 runner)
**不动**: 7 阶段 enum (R17 LOCKED), eval 现有 placeholder

---

## 3. 不动边界 (R32 0 触)

- ✅ R11 LOCKED enum 0 触
- ✅ R17 战役 0 7 阶段 0 触
- ✅ R19 cycle / verdicts / R19_TOKEN_USED 0 触 (只换 token 计算源)
- ✅ R25 改瘦路径 0 触
- ✅ R26 9 器官 0 触
- ✅ 8 项不修改承诺 0 触

---

## 4. 决策日志

1. **按实战 ROI 排序**: 主人日常用 TUI 必然撞 token 数对不上, 优先 1.
2. **不动 LOCKED 边界**: apeireth-asi 真接不破坏 R19 (cycle / verdicts), R30 不破坏 P0 (tool loop 在 TUI, 不抽走).
3. **估时保守**: 都是 1-3d, 给主人完整 plan 选.

---

## 5. 后续推进

等主人拍. 拍完开干, 不会写 R30 那种长报告, 写小 (决策 + diff + test 数字).
