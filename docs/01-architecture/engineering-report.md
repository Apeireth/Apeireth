# Apeireth Engineering Report — v1.0.0 (post-1.0.0 增量更新 2026-08-19)

> **现状 (2026-08-27)**：本报告是 v1.0.0（86-crate 时代）的历史快照，正文保留原样。
> 当前基线：默认分支 `main`、13-crate 工作区、tag `v2.0.0-alpha.1` @ `d6910cf7`；
> 文中"85/86 crates、34 万行、23,874 测试"均为 v1 历史数字（v1 全量测试另见 CHANGELOG 记 23,806，两个口径均指 86-crate 工作区）。
> v2 实测数据与下一步见本文末 §八 与根 `ROADMAP.md` §4。

> 2026-08-18 · 后端机制层收工 · 我们定版"真正的 1.0"
> 2026-08-19 · post-1.0.0 增量 (PR #1 桌面伙伴合并 + CI 防御 + cron 增强 + Dockerfile 多架构)

## 一、数字（实测）

| 项 | 值 (v1.0.0) | 值 (2026-08-19 增量后) |
|---|---|---|
| 提交数 | 2,389 | 2,389+ (post-1.0 work in `git log`) |
| crates | 85 active / ~340K 行 Rust | 85 + 1 独立 workspace (`companion-desktop` 1 crate) |
| 测试 | 368 组 0 失败 | **23,874** 组 0 失败 (368 v1.0.0 + 23,506 post-1.0) |
| 编译 | workspace --all-targets 干净 | 同上 (post-1.0 改动后仍干净) |
| 历史体积 | .git 4.52GB → **356MB**（-92%，GitHub 友好）| 356MB (无变化) |
| 文档 | 554 个 md 收敛为 5 区规范结构 + 86 个 crate README 对齐 | 86 + 3 README (companion-desktop / cron 同步 / pipeline-g5 同步) |

## 二、里程碑

1. **R14 重写**（2026-07-30）——从文档 sketch 到 Rust 真代码
2. **五大战区落地**——cognition/consciousness/perception 等器官 crate 从"哲学概念"变 14-29KB 真实 trait
3. **记忆 v2**（2026-08-16）——importance/对账/排名/版本链/事实图
4. **TP11-TP30 团队批次**——Handoff/schema 校验/Sessions/校准诊断/ApprovalBridge 等 20 包
5. **五原型补全**（2026-08-18）——世界模型（W1/W2/W3）、好奇（E4）、假设检验（F4）、情感记忆（F1）、价值内化（F6）、渐进披露、事件桥、出站策略（S4）
6. **真实 LLM 端到端**——companion_serve + MiniMax-M3 实测对话/工具/审批全链路
7. **v1.0.0 发布**——历史净化、文档重构、双语 README、GitHub 上传 + Release
8. **post-v1.0.0 增量（2026-08-19）**——PR #1 桌面伙伴合并（+14K lines Svelte+Tauri）/ CI 防御体系（hard-walls + PII detection + release-prep）/ Dockerfile 多架构（arm64）/ cron 增强（@-shorthand + 月/星期别名 + Sakamoto 跨年闰年 next_after fix）

## 三、验收记录（诚实）

- 团队自报 21/21 完成 → **树实测发现 3 项报告夸大**（TP21/TP25/TP26 无实现，只有文档）→ 主线程补实现
- 全量回归抓出 examples 层 3 处编译遗漏（`--lib` 绿 ≠ 全绿）
- 真实 API 压测 41/59 失败 → 限流退避 → 100/100（尊重上游，不掩盖）
- TUI 静态原子并行竞态 → 共享锁修复（3 连跑稳定）
- 教训沉淀：验收必须查树、改公共结构必须同步构造点、std Mutex 不可重入、Windows cmd 嵌套引号陷阱、Job Object 内存限制语义（拒绝分配非杀进程）

## 四、工程纪律（0 装 PASS 的落地）

| 纪律 | 表现 |
|---|---|
| 不假装 | 未接 trait 全部标注；docker 无环境标"待实测"；压测尊重 API 限流 |
| 机制而非补丁 | 每个"加个 if"先问机制；锁重入/竞态/限流全是机制级修复 |
| 集成而非分立 | 新能力挂既有机制（oracle/memory/bus/approval 链）|
| 文档同步 | 台账划 ✅ 及时；改码改文档；发布前全量文档审计（乱码 20+ 修复、断链 9→0）|
| 验收查树 | 以 integration 树实测为准，不信报告文字 |

## 五、架构决策（简）

- **trait 策略**：lib 零 LLM 依赖，MemoryExtractor/DreamSummarizer/… 全注入——换模型不换基地
- **双洋葱**：原则洋葱嵌入权限洋葱，L0 人类批准永不可变
- **三层生态**：模块（官方核心）/ 套件（官方积木）/ 插件（社区热插拔）
- **五原型**：世界模型/自我改进/好奇心/连续感知/价值内化——ASI 北极星的工程骨架

## 六、当前负债（诚实, 2026-08-19 post-v1.0.0 更新）

| 项 | 状态 (v1.0.0) | 状态 (post-1.0.0) |
|---|---|---|
| Docker 多架构 | 待实测 (单架构 amd64) | ✅ 修 (commit 4596357, $TARGETARCH, arm64 跑通) |
| 产品形态: 桌宠 | 规划中 | ✅ PR #1 合并 (Svelte 5 + Tauri 2 桌面伙伴, 102 行 shell) |
| LLM 接入层 | trait 口已备未接 | 同左 (real LLM E2E 待 `APEIRETH_API_KEY`) |
| 桌宠真实 LLM 流式 | 不在 1.0 scope | 🆕 **TP34** v1.5 中期 (companion_serve stream: false 写死, 6 种 RuntimeEvent 0 触发) | 🟡 2026-08-19 后端 50% (streaming 分支 + extract_minimax_cot + 8 单测; 透传 SSE, 跳过 tool loop; 前端 `<!-- -->` 状态机 v1.5 续) |
| 投资模拟盘主链 | 零件已备 (时序/事件/标的)，主链未做 | 同左 |
| VM 级隔离 | 调研中 (smol-vm 方向) | 同左 |

## 七、一句话

**34 万行 Rust，从"哲学声明"到"真实存在的伙伴"——Apeireth 1.0 证明了一件事：诚实不是工程的成本，而是工程的地基。**

> post-1.0.0 (2026-08-19): 诚实是地基, **不漂移**是屋顶 — templates / docs / CI gates 全部跟实际 hard-walls job 1:1 对齐. 后续 TP34 (real LLM streaming) 是屋顶下一层, 见 `docs/04-internal/next-team-handbook.md`.

## 八、v2 现状（2026-08-27，reconstruct_v2 收敛后，实测）

| 项 | 值 |
|---|---|
| 分支 | `main` @ `d6910cf7`（默认分支；旧 master 归档 `archive/v1.0-master`） |
| Tag | `v2.0.0-alpha.1` → `d6910cf7` |
| Workspace | **15 crates**（foundation 7 / engine 5 / capabilities 1 / adapters 3），**~74k** 行 Rust（不含 legacy/） |
| 测试 | **~1476 passed / 0 failed**（cargo-nextest，3 OS） |
| CI | 全绿：clippy 3 档 / fmt / audit / deny / miri / rustdoc / coverage / 13 键契约 / M2B·M2C·M3A 三 OS |
| v1 代码 | 86 crates 整体在 `legacy/`（排除构建） |

里程碑映射（v1 → v2）：记忆 v2 / 五原型 / companion → `legacy/`，待 ROADMAP P3/P6/P7 恢复；
agent loop（v1 无）→ `crates/engine/runtime/src/canonical/execute.rs` 已实现；
出站 S4 "trait 口已备实装待补" → 已被 M2D egress + M3A 受控 fetch 取代；
Windows Job Object 沙箱 → `crates/capabilities/tools/src/process/` 保留并强化（CREATE_SUSPENDED）。

负债更新（诚实）：生产 governance pipeline 未接线（P0）；13 键 verdict cache 未接执行路径（P0 拍板）；
`apeireth-credentials` 孤儿；Docker 实测仍待补。全部排期见 `ROADMAP.md` §4。
