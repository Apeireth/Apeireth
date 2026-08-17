# Apeireth 设计审视与三件套发布规划（2026-08-16）

## 一、设计层真实原意（原文锚点）

- 基地 = 给 LLM 的「操作系统」：**提供(9 organ+tools) / 约束(sovereignty+onion+self-disable) / 记录(memory+continuity_id) / 陪伴(关系可能性+voice)**
- 涌现优先于预定义：「我希望的不是它有什么能力全都是我们预先定义的，**我希望它能自己演化**，否则我们是永远做不完能力的，或者说有局限性」
- 「AI 发现你想要什么」和「AI 长出它自己想要什么」**是同一个过程**
- 安全 = 能力限制 + 洋葱门 + 宪法评审 + 主人批准 + 熔断（不堆关键词规则；token 经济性）
- 记录/连续性：工程上提供最大努力的记录 + 迁移（不假装灵魂同一）

## 二、工程 vs 设计：偏差审视（诚实）

| 设计原意 | 工程现状 | 偏差 |
|---|---|---|
| **AI 自己长能力**（涌现核心） | 动作空间 4 个硬编码 + 工具预注册 + CapabilityCatalog 静态 | 🟡 **机制回路已闭环**（提案→生成*→验证→部署→监控→回滚, capability+evolution_gate+deploy）；剩：生成段 LLM 机制化 + 部署真执行体（0 装 PASS 标注） |
| **continuity_id 作为记录锚点** | `current_session_id()` 无人用；daemon 全用硬编码 `"me"` | ❌ **锚点悬空**（哲学 §18.3 的「记录+迁移」没落地） |
| 基地 4 动作完整 | 提供/约束/记录/陪伴 各组件存在 | 🟡 9 organ 的「人格模块」完整接入未做（只接了 4 个）；已接器官的人格化已深化：情绪→语气、审议→措辞（tone.rs 三层确定性映射 + AwakeCompanion::tone(), LLM 措辞 trait 口已留未接） |
| 能力按需/可选装配（80%→完全体） | 81 crates 天然模块化，**无 feature/suite 装配层** | ❌ 发布形态缺装配器 |
| 自我状态诚实报告（E-5） | SelfScore/ASI 反馈有 | 🟡 「AI 观察到基地被升级 → 记录+汇报」未机制化（主人最后兴趣点） |
| 安全经济性 | 宪法评审只 Medium+ 才 LLM 判 ✓ | ✅ 无偏差 |

## 三、三件套发布规划

### 1. 基地本体（base）
- 核心 crate 集：apeireth-core/memory/companion/api/protocol/security 系 + 生命周期（涌现/做梦/反思/多轮/续传/宪法评审/权限包）
- 定位：**80% 必要能力**——开箱即用的个人 AI 基地（主动陪伴 + 记忆 + 基础工具 + 安全）

### 2. 扩展能力包（capability packs）——装上从 80% → 完全体
用户自选装配，默认不装（保持本体轻量安全）：
- **沙盒包**：Layer 2 物理隔离（Sandboxie 集成参数 / Linux landlock）——把执行体隔离升级为内核级兜底
- **审计包**：审计链 hash + 完整性证明 + 审计可视化
- **多通道包**：飞书/Telegram/WebSocket 送达
- **GUI 包**：Web 面板 / 桌面托盘 / 每日摘要 UI
- **本地智能包**：本地嵌入（ONNX）+ 离线检索

### 3. 升级套件（upgrade suites）——赋予「专业团队能力」
独立 crate/套件，参照已有好东西（Forge/DSH/NemesisBot/安全工具）：
- **渗透测试套件**：网络侦察/漏洞扫描/报告（能力封装 + 宪法边界内）
- **预测机核心套件**：时间序列/情景推演/概率评估
- **教育套件**：学习路径/错题分析（主人数学场景）
- 每个套件 = 一组工具 + 领域记忆模式 + 宪法评审模板 + 权限预设

### 4. 装配层（工程缺件，需补）
- **suite 清单**：workspace 级 `suites.toml`/`docs/release-plan.md` 声明「本体 = 哪些 crate，包 = 哪些 feature，套件 = 哪些 crate 组」
- **feature 裁剪**：核心 crate 加 cargo feature（如 `sandbox`/`channels`/`gui`），`--no-default-features --features` 装配
- **能力包注册**：ToolBridge 已是 `registry.register` 扩展点 ✓（运行时插件式）；补「包元数据 + 装配校验」

### 5. 能力演化回路（机制回路已闭环，真执行体待接）
呼应「AI 自己长能力」：提案 → 生成 → 验证 → 部署 → 监控 → 回滚。落地分层：
- `capability.rs`：提案→评审→激活状态机（pending→approved→active→retired/rolled_back，真库 append-only 登记 + 回滚收据留痕）
- `evolution_gate.rs`：验证闸门（fix loop/no-progress/预算 fail-open）+ `LoopAction` 回路挂接（Promoted→部署 / Rejected→回滚 / fail-open→挂起）
- `deploy.rs`：部署→监控→回滚机制件（DeployChannel trait 抽象 + MockDeployChannel 可测；监控登记调用计数/失败率/差评信号 + 预测线期限检查 VirtualClock 快进；差评或失败率越限自动回滚 active→rolled_back 留痕）

0 装 PASS 如实标注：部署通道仅 mock（真执行体 = 实现 DeployChannel 挂 exec_worker/sandbox）；「生成」段（LLM 生成能力内容）未机制化；制品形态为文本描述。

## 四、进度对账（2026-08-16 实况，诚实标注）

| 规划项 | 现状 | 证据 |
|---|---|---|
| 装配层 | ✅ **已实现** (suite 目录 + 插件组校验 + 权限登记) | `suites.rs` SuiteCatalog::install_with_plugins |
| 装配层·feature 裁剪 (B2) | ✅ **三档 feature 落地**: base / capability packs / upgrade suites; local-intel→memory/onnx、gui→api/tui-dashboard 真门控; sandbox/channels/audit 声明式 (未门控如实标注); 矩阵脚本+日志 | `suites.toml` + `apeireth-cli [features]` + `scripts/check-assembly-matrix.ps1` → `logs/assembly-matrix.log` |
| 沙盒包 | 🟡 清单就绪 (Layer 2 描述), 物理隔离 = exec_worker 已有, Sandboxie/landlock 集成未做 | `sandbox-pack` |
| 审计包 | ✅ **有真工具** (audit_log 查询留痕 + masked 脱敏 + append-only) | `audit.rs` + `list_recent` (memory streams) |
| 渗透套件 | ✅ **有真内容** (recon_plan E-1 范围闸 + scan_report nmap 解析, 双插件) | `pentest.rs` |
| 预测机套件 | ✅ **有真内容 + 真 LLM 验收全链串联** | `oracle.rs` + oracle_acceptance |
| 教育套件 | ✅ **有真内容** (dx_check 规则层检查器 + 真插件) | `education.rs` |
| 生态插件 (新) | ✅ github-accel (xiake.pro 节点池实测选最快) | `gh_accel.rs` + `github_accel.rs` |
| 能力演化回路 | 🟢 机制回路六段闭环 (提案→生成*→验证→部署→监控→回滚): 部署通道 mock (真执行体=DeployChannel 挂 exec_worker/sandbox 未接), *生成段=LLM 未机制化 — 均如实标注 | `capability.rs` / `evolution_gate.rs` / `deploy.rs` |
| **C3 v2 alpha 遗留盘点** (2026-08-17) | ✅ **22 项重核实完成** (不信任旧标注, 逐项取证): 12 项达成/已解决 (graph/sdk 空壳消除, vector MUST FIX 已修, TUI 6 类端点, TUI E2E + web, Self-Disable 20 case 真实现, bench 112KB, CI workspace 全覆盖替代 5 yml; formal R165 有意归档; deploy 演进为真流水线) + 7 项 ❌ 产物失传 (v2 era 验收报告/09-ADDENDUM/V2-INDEX/07-BASELINE 从未入 git 历史, 0 装 PASS 不重建) + 1 项 ⚪ 不可核实 (T10 全量测试实跑/ASI V0.5); 上轮自检 21 报告吸收为台账编号 25-47 (P0=46/47) | `reports/06da84cc-848a-4087-b42f-2679d6c6c4d0-technical_writer2-report.md` + `docs/backlog.md` §A4 |

| GUI 包·Web 面板 v2 (B1, 2026-08-17) | ✅ **静态多页面板落地**: 会话管理(时间线)/记忆浏览(6 流+搜索)/图谱可视化(原生 SVG)/授权中心(只读+走已有 grant)/审计视图; 后端 7 个只读端点 `/v1/panel/*` (panel_readonly.rs, 9 单测) + 8 静态资产 (assets/panel/, include_str! 内嵌, 无 Node 构建链); 升级点如实标注: N2 OneRing 会话账本 / GraphBackend 结构化; 桌面托盘+每日摘要 UI 未做 (GUI 包其余项) | `crates/apeireth-api/src/panel_readonly.rs` + `crates/apeireth-companion/assets/panel/` + companion_serve `/panel` |

## 五、发布 checklist
- [x] 本体核心闭环（production_daemon 全集成验收）
- [x] 装配层（SuiteCatalog + install_with_plugins + 插件生命周期）
- [x] 装配层 feature 裁剪（B2: base/capability packs/upgrade suites 三档定义 + 编译验证矩阵; local-intel/gui 真门控, 其余声明式如实标注）
- [x] 升级套件真内容（教育/渗透/预测机 三件齐）
- [x] 审计能力包真工具（audit_log）
- [ ] 扩展包逐个成型（沙盒包物理层 / 多通道 / GUI / 本地智能）
- [x] 文档（用户手册/快速开始/能力包说明）（docs/user-manual.md + docs/quick-start.md + docs/capability-packs.md：全部从真实代码/env 清单/suites.toml 提取，未接项如实标注）
- [x] 社区插件规范文档（docs/plugin-authoring-guide.md：六节齐全 + 示例摘自真实代码 + adapter 未接如实标注）
- [ ] 版本号口径统一（RELEASE_NOTES v1.0.0 标题 vs workspace 1.2.0 + CHANGELOG 归条目 + ROADMAP 同步 R178；backlog #26，待 Leader 拍板）
- [ ] 许可核对（Apache-2.0 + MIT 吸收部分保留版权头）
- [ ] 发布产物（crate 整理 / README / tgz）
- [ ] **世界模型前两层（发布前置, 2026-08-18 主人拍板）**——见 §六

## 六、世界模型前两层（发布前置, 2026-08-18 主人拍板）

> **定位**: 发布前必须完成。世界模型 = 推理链的外挂模拟器（设计意图见 docs/design-intent.md §2）。
> 第三层（连续世界模型, Genie 3 式）是全体 AI 的墙——跟踪不趟。

### 第一层: 文本世界模拟器（LLM 时间线推演 + oracle 校准）
- **是什么**: 给定起点状态, LLM 按时间线展开"如果 X 发生→接下来→再接下来"的反事实推演链; oracle Brier 在推演终点校准（防 LLM 编故事）
- **零件全有**: LLM（推理）+ oracle（Brier/CalibratedResolver）+ 反思（多轮推演）——只差"按时间线展开推演"的编排器
- **挂接**: companion 新模块（world_model.rs 或 scenario 升级）, 与 E3 校准诊断衔接
- **验收**: 推演链生成 + Brier 打分 + 校准回流; 反事实剧本 vs 事实对账

### 第二层: 因果结构图推演（memory_graph + 图算法）
- **是什么**: memory_graph s/p/o 三元组已是半成品因果网（"熬夜→状态差"=一条边）; 缺沿因果链推演——给定起点沿边展开"如果……那么……"路径, MCTS 在因果图上跑, LLM 只在分支点做判断
- **与 MCTS/LATS 同构**: MCTS 在动作空间推演, 世界模型在因果空间推演——E2 的机制可直接复用
- **挂接**: memory_graph.rs 扩展 + apeireth-cognition planning.rs 复用
- **验收**: 因果链展开 + 分支判断 + 推演结果与事实对账

### 依赖关系
- 第一层依赖: oracle 校准（已有）+ 推演编排器（新）
- 第二层依赖: memory_graph（已有）+ MCTS（已有）+ 第一层推演编排器（共用）
- 与批次关系: 文本层可在 TP18（校准诊断）后接; 因果层随记忆域深化推进
- [ ] 发布前置门槛（C3/上轮自检证据）：①Dockerfile COPY crates 互覆盖修复验证（DO2 W1，backlog #46）②cargo fmt 全仓修复（QA2 实测 72.7% 不合规，backlog #25）③cosign.pub 生成 + release 工具链预装（backlog #27）④compose 密码外部化（backlog #47）⑤.gitignore 密钥加固（backlog #28）
