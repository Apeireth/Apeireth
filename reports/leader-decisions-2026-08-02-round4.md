# 用户裁决记录（2026-08-02 第四轮）

## 1. 兼容组件 vs PyBridge 兼容层 — 结论：不重复

**LOCKED 文档的真实设计**：
- 阶段 1 §X"可扩展边界—通过兼容组件接入其他语言模块（Python/Go/JS 等）"
- 阶段 1 §X"兼容性完美—兼容现有 1100+ 模块"
- 阶段 2 §6"平台中立：任意关系可由任意语言插件表达"
- 阶段 2 §11"一人/多人部署兼容"（HA + 部署兼容）
- 阶段 2 crate-split §2 兼容组件层（4 个 crate）：
  - apeireth-pybridge（PyO3 兼容桥 — 现有 1100+ Python 模块）
  - apeireth-mcp（MCP 客户端 — 外部 MCP 服务）
  - apeireth-extension（VCP 6 类插件协议）
  - 其他适配器

**用户决策正解**：
- "兼容层"指的正是 Apeireth 设计的兼容组件层（4 crate），不是别的模块
- PyBridge（apeireth-pybridge）是兼容组件层的**一个**，不是独立模块
- 不能"偷懒说不用 Rust 实现所有" = 不能砍 PyBridge，也不能砍兼容组件层
- 兼容层是必要路径 — R11 1100+ Python 模块必须能被桥接，否则 Apeireth 不能"兼容完美"

**Leader 推断与执行**：
- 兼容组件层（pybridge + mcp + extension + 其他）必须保留并深化
- PyBridge 默认 feature-gated 关闭，需要时才打开
- 不与 Apeireth 设计冲突，是设计的一部分

## 2. 按实际工程来
- 实现上文档不可能全部预料到 → 4C 分层
- LOCKED 不动，代码按工程现实推进；漂移用报告登记
- 已体现在 3C（双轨）+ 4C（分层）中

## 3-5. 用户选项记录
3. backlog 策略 = C（双轨：最新 integration 之上做增量，不重做已 merged）
4. LOCKED vs 实现 = C（分层：LOCKED 不动，drift 登记差距）
5. 评测接口异常 = A（接受 leader 兜底评审，落盘 reports/leader-evaluation-*.md）

## 下一步执行
- 派活继续遵循 3C 双轨：在最新 integration HEAD (6dc3c574) 之上做增量
- 缺口矩阵已分配架构师执行
- apeireth-cognition 深度任务已分配数据库工程师执行
- 不重复已 merged_to_integration 的工作
