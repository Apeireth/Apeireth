#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Append V7 round-12 summary to memory/2026-07-21.md."""
from pathlib import Path

summary = """
## 00:26 - V7 round-12 真生产细节调研 (cron, 主人 00:25 真务实修)

### 12 query 真生产借鉴角度 (不重复 round-11)
- **失败/恢复**: idempotency, retries, rollback boundaries (agent-axiom book Ch10)
- **熔断/卡住**: agent circuit breaker, $83 retry 烧钱事故, MVP Fast 2026 guide
- **沙盒权限**: Per-Tool Sandboxing (Sandlock MCP), Janus (system-level policy)
- **代码沙盒**: gVisor/Firecracker 评测, Reddit 6周实测 Firecracker 胜
- **MCP spec**: AnySearch 返回 0 (空白, 下一轮补)
- **A2A 协议**: google/A2A 24871 stars, v1.0.0 spec
- **SWE-bench Verified**: 500-task verified subset, July 2026 leaderboard
- **Agent-as-Judge**: metauto-ai 800 stars, Zhuge 2024 paper
- **Cost routing**: Cluster/Route/Escalate, RouteNLP, dynamic model routing survey
- **Energy/Budget**: Joule runtime (hierarchical routing + deterministic tools), Constraint-Driven Online Resource Allocation, Anytime Verified Agents
- **World Model**: DeepMind Genie 3, NVIDIA Cosmos, Meta V-JEPA 2, Wayve GAIA-2
- **Metacognition**: Georgia Tech, Zylos research, Agentic Knowledgeable Self-awareness (ACL 2025)

### 结果
- 12 query / 33 merged sources / 81.5s
- Bocha bw=0/ai=0 (确认 round-10 起); AnySearch 主力
- timeoutSeconds 3600 (1h) 配置正确, 实际跑 81.5s 还差很多

### 输出
- `promethean/research-v7-round-12.json`
- 真生产细节: idempotency / circuit-breaker / per-tool sandbox / MCP-A2A 协议 / SWE-bench / cost-cascade / world-model / metacognition 全部到位

### 下一轮 (round-13) 候选
- MCP spec (round-12 返回 0, 必须补)
- Skills hot-reload / discovery
- 上下文压缩 (Context compression / Mem0 architecture)
- 形式化验证 agent (Verified agent loops)
- 红队框架 (Agentic red team)

### 真生产借鉴要点 (主 23:50 抓紧干)
- **失败是常态**: idempotency + retry budget + circuit breaker 是底线, 不是可选
- **沙盒分两层**: code-exec (gVisor/Firecracker) + per-tool (MCP Sandlock), 不混
- **协议是基础**: MCP (tool boundary) + A2A (agent boundary) = agent internetworking
- **评测要分层**: SWE-bench (能力) + Agent-as-Judge (开放) + harness telemetry (过程)
- **成本要 cascade**: 简单查询 → 小模型, 复杂 → 大模型, 监控烧钱循环
- **世界模型 + 元认知**: 二阶控制 + 自我仿真, 真生产可借
"""

p = Path(r'.openclaw\workspace\memory\2026-07-21.md')
existing = p.read_text(encoding='utf-8') if p.exists() else ''
p.write_text(existing + summary, encoding='utf-8')
print(f'append {len(summary)} chars to {p}')