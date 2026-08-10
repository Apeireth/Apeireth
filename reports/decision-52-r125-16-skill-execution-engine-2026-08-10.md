# ⚠️ MARKER — 旧 Decision-52 (写错方向, 0 装)

**Date**: 2026-08-10 21:11 (R125-16 sub-agent 撤销)
**触发**: R125-16 sub-agent 8/10 20:39 写错了升级方向, 严重违反 0 重复造轮子严守 (per 主人 10 项偏好 #6)

**错误概要**: R125-16 sub-agent 写这个决策时, 误以为 R125-15e 是唯一写过 apeireth-central/src/ skill_*.rs 的 sub-agent, 没注意到整合 #4 commit (abf12243) 之后 R125-18 (P3-1, bg_bfeb840c) 已经写了 5 个 mod (skill_execution + skill_prompt + skill_validation + skill_companion + skill_frontmatter). R125-16 覆盖了 R125-18 的 `skill_execution.rs` (450 行 SkillExecutor + StepExecution + 9 unit test).

**处理**: 8/10 21:11 立即撤销 + 修复:
1. 撤销 lib.rs 改动 (移除 R125-16 段 + 2 行 pub mod skill_outcome / skill_runner)
2. 撤销 Cargo.toml 改动 (skill_runner_demo 段 → skill_recommender_demo 段)
3. 临时维护 R125-18 `skill_execution.rs` (1:1 R125-18 readmap 简化 5 unit test, 标明 "临时维护" + R125-18 借鉴 ID)
4. 覆盖 4 文件为 marker (skill_outcome.rs / skill_runner.rs / tests/skill_runner_test.rs / examples/skill_runner_demo.rs)
5. 写新方向 `skill_recommender.rs` (NEW, 0 跟 R125-15e / R125-18 / R125-19 冲突)
6. 重写 decision-52 (新路径 `decision-52-r125-16-skill-recommender-2026-08-10.md`)
7. 重写 final 报告 (新路径 `agent-r125-16-final-v2-2026-08-10.md`)
8. 报告 Mavis 父 session (per 报告 back 严守)

**0 装 PASS 严守**: 0 假装"未发生", 0 装"R125-18 跟 R125-16 0 冲突". 诚实标.

**整合 #5 commit 处理**: 0 必重跑决策, 整合 #5 commit 时 Mavis 删除本 marker 文件 + 4 marker 源文件 (skill_outcome.rs / skill_runner.rs / tests/skill_runner_test.rs / examples/skill_runner_demo.rs).

**新决策**: 决策 #52 (新方向) 在 `reports/decision-52-r125-16-skill-recommender-2026-08-10.md` (本 marker 之后写的).

**新 final 报告**: `reports/agent-r125-16-final-v2-2026-08-10.md`.

---

# (下面是旧内容, 写错方向的, 0 必参考, 仅保留以备 audit)

(原内容已删除, 详见 reports/agent-r125-16-final-2026-08-10.md.archived-2026-08-10-21-11 marker)
