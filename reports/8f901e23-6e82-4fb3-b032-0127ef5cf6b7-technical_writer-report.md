# 自审报告 — 发布文档三件：用户手册/快速开始/能力包说明（返工轮次 1）

- **任务 ID**: 8f901e23-6e82-4fb3-b032-0127ef5cf6b7
- **角色**: 技术文档（technical_writer）
- **日期**: 2026-08-16
- **返工说明**: Round 1 评审不通过（deliverable_missing）——上下文重置后把上一任务（§5.6）的提交误当本任务交付。本轮按任务描述从头执行，三文档全部新建并逐项对照真实代码提取。

## 1. 交付物清单（本轮新增）

| 文件 | 内容 | 任务要求对应 |
|---|---|---|
| `docs/quick-start.md` | 前置条件（Rust 1.80/MiniMax key/磁盘路径）→ 构建 → companion_serve（主入口 + 12 项 env 表）→ companion_daemon（8 项 env 表 + stdin 交互）→ production_daemon 全机制验收 → 其他示例 → 常见坑 → 下一步 | ① 快速开始：安装→最小运行，从真实命令与 env 清单提取 |
| `docs/user-manual.md` | 基地定位 / 记忆与连续性（生命周期 5 机制表 + 用户可控项）/ 工具与审批（真实工具清单 + 审批 5 步 + 动态原则）/ 主动陪伴三层门控 / 5 种 Sink + 出站脱敏 / 安全机制 / §8 未接清单 | ② 用户手册：9 organ/记忆/工具/审批/多 sink 用户视角，机制描述如实，不虚构交互 |
| `docs/capability-packs.md` | 三档装配模型 + 统一装配入口命令 / 档 1 base crate 组与不可裁项 / 档 2 五包 gated vs declarative 真门控表 / 档 3 三套件组成+运行时装配序列 / 矩阵验证脚本 | ③ 能力包说明：基于 suites.toml 三档 + B2 feature 矩阵，装配命令与差异 |
| `docs/release-plan.md` | checklist「文档（用户手册/快速开始/能力包说明）」勾选 + 证据标注 | release-plan checklist 对应项勾选 |
| 本报告 | reports/8f901e23-...-technical_writer-report.md | 自审报告 |

## 2. 准确性验证（每条可抽查）

所有命令/env/机制均摘自真实源，抽查对照：

| 文档内容 | 真实来源 |
|---|---|
| quick-start §1 Rust 1.80 / edition 2021 | `Cargo.toml:224-226` |
| quick-start §3 companion_serve 12 项 env | `docs/maintenance-guide.md` §四「companion_serve 环境变量」（逐条照录） |
| quick-start §4 companion_daemon 8 项 env + stdin 交互 | `examples/companion_daemon.rs` 模块头 env 注释（原文） |
| quick-start §5 production_daemon 需 apikey-ultra.txt | `examples/production_daemon.rs:1-9` 模块头 |
| quick-start §3 MiniMax 端点/模型 | `examples/production_daemon.rs:35-36`（BASE_URL/MODEL const） |
| user-manual §2.2 生命周期 5 机制 | maintenance-guide 模块地图（memory_extractor/dream/reflection/memory_injection/onering 行） |
| user-manual §3.1 工具清单 | `tool_bridge.rs:380-431`（register 调用）+ `with_goals`（tool_bridge.rs:487-510） |
| user-manual §3.2 白名单 16 工具/日常包 9 工具/授权请求 GET 端点 | `tool_bridge.rs:438-455` / `packs.rs:149-164` / maintenance-guide approval_requests 行 |
| user-manual §4 三层门控 + tone 三层合成 | `organs.rs` 模块头（诚实标注原文） |
| user-manual §5 Sink 表 + 出站脱敏 | `daemon.rs:169-446`（5 Sink）+ `daemon.rs:476`（redact_text Mask） |
| user-manual §8 未接清单 | organs.rs 头 / packs.rs 头 / suites.toml packs.sandbox note / team-work-doc §4 A1 / plugin-authoring-guide §5.1 |
| capability-packs 全文 | workspace 根 `suites.toml`（base/packs.*/suites.* 全部小节）+ `crates/apeireth-cli/Cargo.toml [features]`（default=base, local-intel/gui 真转发, sandbox/channels/audit=空声明）+ `suites.rs:164-203` + `examples/education_suite_demo.rs` |

**0 装 PASS**：未实现项一律标注「未接/declarative/未门控」，不虚构交互（user-manual §3.2 明确写「没有弹窗，就是前端页面上的批准操作」；§8 集中列未接项）。

## 3. 边界遵守

- ✅ 只新增文档（3 个新 .md）+ release-plan 一行勾选 + 本报告；**0 产品代码改动**。
- ✅ 未引入新规范；与 maintenance-guide §四 env 清单逐条一致（未新增 env）。
- ✅ 提交纪律：小步提交，中文 message，只含自己文件。

## 4. 0 假装标注（没做什么）

1. 三文档未做真实环境端到端运行验证（本任务边界为文档，且跑真 LLM 需 key；命令均可按 quick-start 原文复现抽查）。
2. 「9 organ」完整器官枚举在工作文档中无权威清单，user-manual §4 只描述 organs.rs 真实接线的器官（情绪/审议/演化）+ 安全器官（洋葱/主权），未虚构其余器官名称。
3. suites.toml 的 case 编号细节（case 5/7）引自文件内 note 原文，未重跑矩阵脚本验证（脚本输出 logs/assembly-matrix.log 由 B2 任务维护）。

## 5. 给守门员的合并提示

- release-plan.md 只改一行（文档 checklist 项 → [x]）；该文件有其他成员并行改动，本提交只含本行 hunk。
- 三个新文档无冲突可能。
