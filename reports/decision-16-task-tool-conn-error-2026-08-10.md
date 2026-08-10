# 决策 #16 — Task 工具 Connection error 挂了, Mavis 改自干补位

**时间**: 2026-08-10 14:42
**来源**: 14:42 试图派 R122-10/11 时 task 工具返回 "Tool task not found" (后台 Connection error 模式复现)
**决策**: 放弃 task 工具, Mavis 自己干 refactor scan (R122-10), 跟 5 active agent 并行
**理由**:
- 跟 R121 第一波 10:14 / R122-1/2/3/4 14:15 失败同源, 后端分发瞬时故障
- R122-10 refactor scan 是 read-only 任务, Mavis 自干无需工具
- 5 active agent (R122-1/3/4-retry + R122-8/9) 跑中, 不受 task 工具故障影响 (他们的子 session 独立)
- 截止 15:15 剩 33 min, 不再尝试派新 task 工具调用, 集中 Mavis 自干 + monitor
**Mavis 行动**:
- 自干 R122-10 refactor scan (7.7KB 报告, 14:50 完)
- 继续 monitor 5 active agent
- 15:00 写 15:15 final report
**影响**:
- 5 done (R122-5/6/7/2/10) / 5 active / 0 派新 = 10 total
- 6 缺口未派 (P1-9 SDK retry 替代, P2-12 浏览器, P2-13 多模态, Kani retry 替代, 等等)
- 截止时间不延, 严守 15:15
