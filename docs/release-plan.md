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
| **AI 自己长能力**（涌现核心） | 动作空间 4 个硬编码 + 工具预注册 + CapabilityCatalog 静态 | ❌ **最大一叶障目**：能力生成/演化回路缺失（提案→生成→验证→部署→监控→回滚） |
| **continuity_id 作为记录锚点** | `current_session_id()` 无人用；daemon 全用硬编码 `"me"` | ❌ **锚点悬空**（哲学 §18.3 的「记录+迁移」没落地） |
| 基地 4 动作完整 | 提供/约束/记录/陪伴 各组件存在 | 🟡 9 organ 的「人格模块」完整接入未做（只接了 4 个） |
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

### 5. 能力演化回路（蓝图，发布后第一优先级）
呼应「AI 自己长能力」：AI 提案新能力 → 登记能力库 → 宪法评审 → 主人批准 → 激活 → 监控 → 差评回滚。本会话已落第一块：`capability` 提案机制件（pending→approved→active 状态机 + 真库登记）。

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
| 能力演化回路 | 🟡 第一块落地 (提案→评审→批准→激活), 生成/验证/部署/监控/回滚未全 | `capability.rs` / `evolution_gate.rs` |

## 五、发布 checklist
- [x] 本体核心闭环（production_daemon 全集成验收）
- [x] 装配层（SuiteCatalog + install_with_plugins + 插件生命周期）
- [x] 装配层 feature 裁剪（B2: base/capability packs/upgrade suites 三档定义 + 编译验证矩阵; local-intel/gui 真门控, 其余声明式如实标注）
- [x] 升级套件真内容（教育/渗透/预测机 三件齐）
- [x] 审计能力包真工具（audit_log）
- [ ] 扩展包逐个成型（沙盒包物理层 / 多通道 / GUI / 本地智能）
- [ ] 文档（用户手册/快速开始/能力包说明）
- [x] 社区插件规范文档（docs/plugin-authoring-guide.md：六节齐全 + 示例摘自真实代码 + adapter 未接如实标注）
- [ ] 许可核对（Apache-2.0 + MIT 吸收部分保留版权头）
- [ ] 发布产物（crate 整理 / README / tgz）
