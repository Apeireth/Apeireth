# 自审报告 — §5.6 社区插件规范文档（technical_writer）

- **任务 ID**: fcfb2abe-d2a3-4e19-a514-7f209de48747
- **角色**: 技术文档（technical_writer）
- **日期**: 2026-08-16

## 1. 改动文件（只新增文档 + 文档同步，0 产品代码改动）

| 文件 | 动作 | 说明 |
|---|---|---|
| `docs/plugin-authoring-guide.md` | 新增 | 主交付：社区插件开发规范，六节齐全 |
| `docs/maintenance-guide.md` | 编辑（+1 行） | 头部互链 plugin-authoring-guide |
| `docs/release-plan.md` | 编辑（+1 行） | 发布 checklist 勾选「社区插件规范文档」 |

提交记录（小步提交，中文 message）：
- `9095cd92` docs(§5.6): 新增社区插件开发规范 plugin-authoring-guide.md
- `1443c2d8` docs(§5.6): 文档同步 — maintenance-guide 头部互链 + release-plan checklist 勾项

## 2. 六节覆盖对照（任务要求 → 文档章节）

| 任务要求 | 文档位置 | 证据 |
|---|---|---|
| ① Plugin trait 用法 + ToolBridge 注册示例 | §1.1-§1.5 | Plugin trait（plugin.rs:18-26 摘录）、Tool trait（trait_def.rs:27-45 摘录）、EducationDxPlugin 完整装配（education.rs:152-206 摘录）、education_suite_demo.rs 运行入口摘录、依赖/注册清单 |
| ② 白名单/日常包规则 | §2.1-§2.3 | execute_if_allowed 八闸执行链（tool_bridge.rs:552-699 逐段核实）、ApprovalManager 三层规则（黑名单/白名单 16 工具/RiskRule 7 类）、日常包 9 工具（packs.rs:149-164）、插件自授权标准动作 + revoke_by_name 原因 |
| ③ 测试模板（含 0 装 PASS 写法） | §3.1-§3.4 | 全链路测试模板（education.rs:284-316 摘录）、BadPlugin 失败路径 + install_all 回滚（plugin.rs:114-162 摘录）、0 装 PASS 四条禁令表 + 模块头标注示例（education.rs:6-9 摘录）、测试命令（-j 4，禁全量） |
| ④ 卸载真清理要求（不留注册残留） | §4.1-§4.3 | registry.unregister + revoke_by_name 标准组合、幂等要求、验收断言（education.rs:310-315 摘录） |
| ⑤ 数据源 adapter 模板（填一个文件 = 新插件，对接预测机 §5.2） | §5.1-§5.4 | 单文件模板（天气源骨架，标注「模板代码，基于已核实真实 API 编写」）+ mock 先行纪律 + forecast 对接路径 + §5.2 规划 8 个社区数据源清单 + 升级路径 |
| ⑥ 发布检查单 | §6 | 9 项检查单（对齐 team-work-doc §7 验收总纲）+ 合入流程（集成守门员） |

## 3. 验证结果（0 装 PASS：文档任务无代码测试，如实说明验证方式）

- **未运行 cargo test**（本任务 0 产品代码改动，不产生代码变化；工作区有其他成员并行改动，不越界跑全量）。
- **示例真实性验证**：所有代码摘录在写文档时逐段对照源文件读取记录（read_file 原文），非凭记忆重写。关键 API 二次核实：
  - Plugin trait 5 方法签名（plugin.rs）
  - Tool trait 4 方法 + ToolKind 6 类 + ToolAxes 5 轴及默认值（trait_def.rs / types.rs）
  - PermissionPack::permanent/timed、revoke_by_name、check_and_consume、default_daily_pack 9 工具清单（packs.rs）
  - ApprovalManager 白名单 16 工具清单、RiskRule 类别、tool_risk 关键词映射（tool_bridge.rs）
  - ConstitutionGate 15 条前缀规则（constitution_gate.rs）
  - Forecast::new / ForecastRegistry register/resolve/calibration（oracle.rs）
  - 执行链顺序初稿有误（漏动态原则层、宪法硬门 desc 含参数、LLM 评审位置），核实 tool_bridge.rs:565-705 后已修正 §2.1。
- **文档结构验证**：六节齐全 + 附录来源索引（每条 API 标注 file:line）。

## 4. 0 假装标注（本交付如实说明「没做什么」）

1. **§5.2 规划的数据源 adapter trait 未接**：全仓库 grep 核实（2026-08-16）无 DataSource/ForecastSource 类 trait；oracle.rs 只有 `UncertaintyResolver` trait 口（oracle.rs:52-54）。文档 §5.1 已如实标注，并给出现实路径（数据源 = 取数 Tool 插件 + AI 用内置 forecast 工具登记预测）。**建议 leader/相关队长**：adapter trait + adapter registry 热插拔若需立项，应登记 docs/backlog.md（本任务边界为文档，未代登台账）。
2. **动态插件加载未接**：当前插件 = Rust 编译期单元（PR 进 apeireth-companion）；tool-registry 的 watch_plugin_dir 只记录文件事件不是装载器——文档 §0 已标注。
3. **ForecastRegistry 不向插件暴露注入口**（ToolBridge 内部自建，session 固定 "me"）——文档 §5.1 已标注，模板不让插件碰 ForecastRegistry。
4. **release-plan 大文档项「文档（用户手册/快速开始/能力包说明）」保持未勾**——本交付只覆盖社区插件规范一项，不假装整个文档项完成。
5. 文档中的单文件模板（§5.2 天气源骨架）是**新编写**的模板代码，基于已核实的真实 API，已在文中显式标注「模板代码，非现有文件摘录」，未冒充现有代码。

## 5. 集成点说明（给相邻机制/后续任务）

- **W2 预测机套件**：若官方 adapter trait 落地，需更新本指南 §5（迁移对照表）与 §5.4 升级路径；模板纪律（mock 先行/真 API 可选）可直接复用为 adapter 验收标准。
- **集成守门员**：两个文档同步提交（1443c2d8）与并行提交（e8b1f5c8 B2 装配层、259c228e N11）无冲突区域，已各自干净提交；maintenance-guide.md 头部块与 release-plan checklist 尾部各 +1 行。
- **后续社区候选插件**（翻译器/科学计算器/体育预测数据源/Emoji/塔罗/生图，team-work-doc §5.6）：可直接按本指南 §1.3 示例 + §3.1 测试模板 + §6 检查单开工。

## 6. 给守门员的合并提示

- 两个提交均为纯文档（markdown），无编译影响，无需跑 cargo。
- 若其他成员也在改 maintenance-guide 头部块，我的改动是引用块最后一行（`> **社区插件开发规范见 ...`），注意保留。
- docs/plugin-authoring-guide.md 为新文件，无冲突可能。
