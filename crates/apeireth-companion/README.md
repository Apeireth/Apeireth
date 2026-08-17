# apeireth-companion

> Apeireth 伙伴器官（1.0 核心）—— 长期跨 session 用户关系器官，承载"用户是 AI 伙伴"语义。约 25,000 行，644 测试。

## 能力（对齐实际代码）

| 领域 | 模块 | 说明 |
|---|---|---|
| **记忆 v2** | `memory_extractor` / `memory_graph` | 重要性打分、Mem0 式对账（ADD/UPDATE/DELETE+tomb）、排名注入、双时态事实图、版本链 |
| **世界模型** | `world_model` (W1) / `causal_world_model` (W2+W3) | LLM 反事实时间线推演 + Brier 终点校准；MCTS 因果图推演 + 记忆时间线挖因果边 |
| **她本身** | `curiosity` (E4) / `hypothesis` (F4) / `emotion_memory` (F1) / `value_cases` (F6) / `emergence` (E7) | 好奇引擎（回声偏置+浅尝辄止+疑问路由）、假设检验闭环、主人情绪时间线、价值案例库、开口策略涌现循环 |
| **注入管线** | `context` / `assemble` / `progressive` / `proactive_memory` | ContextAssembler（L0/L1 常驻+预算截断）、渐进式披露目录、主动预载 |
| **工具桥** | `tool_bridge` / `observer_capture` | 工具执行 + 审批桥 + 结果即时沉淀（W5） |
| **安全** | `job_object` / `sandbox` / `restricted_token` | Windows Job Object 沙箱、限额留痕、受限 token |
| **daemon** | `daemon` / `dream` / `reflection` | 做梦整合、反思、涌现说话（LLM 节流+退避） |

## 运行

```bash
cargo run -p apeireth-companion --example companion_serve   # :8090 OpenAI 兼容伙伴端点
cargo test -p apeireth-companion --lib                        # 644 测试
cargo run -p apeireth-companion --example tp_acceptance_sim   # TP 验收模拟 4/4
```

环境变量：`APEIRETH_API_KEY`（真 LLM 必需）、`APEIRETH_SEED_MEMORY`、`APEIRETH_GRANT`、`APEIRETH_DREAM_QUIET_SECONDS`。

## 文档

- 架构：[docs/01-architecture/architecture.md](../../docs/01-architecture/architecture.md)
- 快速开始：[docs/02-guides/quick-start.md](../../docs/02-guides/quick-start.md)
