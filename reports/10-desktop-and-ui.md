# 册 10 — 桌面与界面（3 篇合册）

> **一句话定位**：桌面端（桌面壳 / 形象 / 设置生效契约）与界面侧的合成与落地记录——其中 **设置实时生效契约**是界面行为的现行约束。
>
> **来源篇数**：3｜**合册生成日**：2026-10-23
> **机械合订说明**：本册由脚本从原篇**机械摘取**（标题 / 元信息块 / 各节要点 / 归属路径），**不生成、不改写、不编造**；
> **本册未做逐条「现状复核」**——即不判断每条结论今天是否仍成立。要现行口径请查下方真源，原件全文见归档路径。
>
> **复核入口（现行真源）**：[claims.yaml](../docs/status/claims.yaml) ／ [test-baseline](../docs/03-reference/test-baseline.md) ／ [capabilities-matrix](../docs/03-reference/capabilities-matrix.md) ／ [00-backlog](../docs/04-internal/backend-enhancement/00-backlog.md)

## 目录

| # | 原标题 | 原日期 | 行数 |
|---|---|---|---|
| 1 | 基地内置功能面研判——参考项目普查综合 + 既有调研对齐 | 2026-10-11 | 165 |
| 2 | 设置面三级即效语义 · 控件→生效时机对照表 + 测试证据 | 原篇未标日期 | 116 |
| 3 | R20 阶段 5 — Tauri 2.0 desktop scaffold skeleton 报告 | 2026-08-05 | 215 |

---

### 基地内置功能面研判——参考项目普查综合 + 既有调研对齐 (2026-10-11)

**来源**：`reports/builtin-feature-synthesis-2026-10-11.md`（165 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/builtin-feature-synthesis-2026-10-11.md`（原件逐字留存）

**元信息**

# 基地内置功能面研判——参考项目普查综合 + 既有调研对齐
> `Status: ✅ 研判完成（2026-10-11；196 链接全量核验闭环，15 份明细落盘）`
> 口径纪律：全文中性措辞；项目名仅作引注数据出现（豁免通道）；实现层零代码复制。
> 上游源码类参考另有专篇：[embedded-app-suite-research-2026-10-11.md](embedded-app-suite-research-2026-10-11.md)（内置应用面架构拆解）。
## §0 结论速览——我们该内置什么
按「基地强度增益 ÷ 施工成本 ÷ 权限风险」三维研判，推荐内置面如下（P0 立项、P1 跟波、P2 观望）：

**各节要点（机械摘取，非全文）**

- **§0 结论速览——我们该内置什么**
  - 按「基地强度增益 ÷ 施工成本 ÷ 权限风险」三维研判，推荐内置面如下（P0 立项、P1 跟波、P2 观望）：
  - | 级 | 内置功能面 | 一句话 | 直接增益 |
  - | **P0** | 应用宿主 + 内置应用面（多标签壳/贡献点注册表） | 一切内置件的底座（承接 I-1/I-2） | 基地从「聊天壳」变「工作台」 |
  - | **P0** | 文档面 + 文本面（唯一真源文档管线、代码编辑组件） | 类办公文档与源码直编 | 创作生产力 |
- **§1 输入清单与方法**
  - 1. **链接普查**：`重要参考项目，产品方向前辈同行，想法随记.txt` 全文抽取 196 条唯一链接（189 GitHub + 7 站外），分 15 批并行核验，逐项提炼内置功能启示；明细 13 份落盘 `_research_mem/link-survey/`（本报告引注数据）。
  - 2. **源码级调研**：参考中枢服务 + 参考桌面壳浅克隆（`_research_mem/upstream-*-ref/`），内置应用面架构拆解见专篇。
  - 3. **既有调研对齐**（防重复造轮子、防推翻前案）：
  - - `reports/r33-excellent-projects-survey-2026-08-09.md`：优秀项目调研范式（能借鉴/不借鉴/触发条件），「借鉴一个做到位，不 cargo cult」；
- **§2 普查硬事实（诚实口径）**
  - | 口径 | 数值 |
  - | 链接总数 | 196 抽出（1 条尾注误粘已修正；1 对大小写重复合并）→ 195 实核（GitHub 188 + 站外 7） |
  - | 已核验 | 195/195（15 批明细：`_research_mem/link-survey/part-01..14.md` + `part-official-sites.md`） |
  - | 存活 | **181**（GitHub 181 / 188） |
- **§3 内置功能面清单（逐面：吸收什么 / 不吸收什么 / 与既有账目关系）**
- **A. 应用宿主与内置应用面（P0，承接 I 批次）**
  - - **吸收**：贡献点注册表（纯数据注册 + 字段白名单 + 禁注入 HTML/URL/通道）、mount/unmount 协议、多标签壳 + 拖出独立窗、浮层 acquire/release 会话；文档面「唯一真源 + 编译索引 + 渲染态直改 + 区间事务写回」；浏览面「托管运行时 + 语义树操控」。
  - - **不吸收**：对方的文件格式自称、模块命名、任何代码（上游 CC BY-NC-SA 4.0 红线）；85 插件各自一进程的过度进程化（R136 前案结论）。
  - - **与既有账目**：00-backlog 拟立想法 I；D（受控写）的审批链与文档面「提案-合入」同构复用。
- **B. 任务状态形象面（P0——本次普查最大的新增洞察）**
  - 参考对象：像素任务状态宠物（干活/等批准/待审阅/完成/出错五态换装）、面板代审批（允许/以后允许/拒绝，与确认框双向同步、先答为准、超时交还）、灵动岛式任务栏（状态环 + 当前步骤 + 用时）、2D 形象桌宠（透明窗、点击穿透、心跳日程主动关怀、表情状态跟踪）、局域网多宠互联。
  - - **吸收**：① 智能体状态 → 形象动作映射（我们的执行事件流已有，只差表现层）；② **审批前置到桌面面板**（与既有 ApprovalInbox 双向同步）；③ 本地心跳日程零 token 主动提醒；④ 隐私模式一键隐藏任务内容。
  - - **不吸收**：游戏引擎级交互策略（成本高）；多宠互联（隐私硬边界——星群架构下需走匿名化经验共享，暂缓）。
  - - **与既有账目**：形象面已有（`features/pet`）；接 E 审批族 + G 观测（状态即数据源）。**这是「基地更强大」感知最强、权限面最小的一件，建议首批。**
- **C. 语音面（P1）**
  - - **吸收**：本地低开销转写引擎（同精度提速 + 量化 + 批推理）→ 流式合成 → 5 秒零样本音色克隆 → 打断式实时对话（VAD + 聊天记录文件观察者两条宿主适配路径）。
  - - **不吸收**：直播向双人同台仲裁（垂直场景）；未组装即诚实降级——`voice.duplex` 契约已在桌面能力差距矩阵立好，**装一件亮一件，绝不假波形**。
  - - **与既有账目**：H 模型面（本地/云端双推理）、G（语音事件观测）。
- **D. 记忆与知识面（P1）**
  - - **吸收**：① 工具观察自动沉淀 + 语义摘要 + 跨会话召回（与 Memory 2.0 协调器对接，走 `<governed_memory>` 关闭世界注入）；② 针状观察分类（决策/修复/安全警示/敏感）直接对应我们的治理留痕；③ 上下文知识库「知识/记忆/技能文件系统化 + 分层摘要按需读」；④ 代码片段库（Git 存储 + 修订历史）。
  - - **不吸收**：托管记忆云服务（离线优先战略）；加密货币相关的社区运营件。
  - - **与既有账目**：C 记忆深化主战场；yanshu 供体映射的 ADAPT/DEFER 纪律照旧；向量检索无 embedding 时可解释降级（既有不变量）。
- **E. 联网与网页面（P1）**
  - - **吸收**：语义化网页操控（无障碍快照 + 稳定元素引用 + 语义句柄操作）、抓取转 Markdown 直供知识库、统一搜索接口封装、**外链存档与失效检测**（本次百度分享失效是实证）。
  - - **不吸收**：反爬隐身/对抗采集（法律与道德风险，默认拒绝）；「克隆网站生成应用」类（超出基地定位）。
  - - **与既有账目**：I-4/I-5 批次；E 对象级判定先到位再放行。
- **F. 生产力工具面（P2，按需立项）**
  - 数据库工作台（多库连接 + AI 助手 + 工具协议接入）、邮件（本地离线收发 + 智能体只读辅助拟稿 + 人工确认发出）、图表（数据驱动可视化）、翻译、笔记、终端面（持久会话）。
  - - **不吸收**：金融交易闭环（垂直 + 高风险）；渗透测试类（默认拒绝域）。
  - - **与既有账目**：F 服务端形态（多端复用）、E（邮件/终端高危面审批）。
- **G. 安全与治理面（横切，随波验收）**
  - - **吸收**：凭据保险库（本地加密 + 系统凭据管理器）、出站敏感信息脱敏（占位替换、执行前还原）、应用沙箱隔离（快照恢复 + 资源配额）、AI 代码安全审计三层检测。
  - - **与既有账目**：E 安全纵深主战场；audit/deny 工具链既有；R136「安全伪工程」教训——隔离必须是真的。
- **H. 生态与分发面（P1）**
  - - **吸收**：插件市场（精选清单 + 一键安装升级）、技能包打包分发、主题/动效系统（设计系统文档驱动界面）、操作序列录制（三档校验强度）。
  - - **与既有账目**：`awesome-dsh-plugin` 生态清单是「内置什么」的直接需求侧信号（用户装什么 = 我们该内置什么）；主题面与前端动效参考同批。

**归属链接**：reports/r33-excellent-projects-survey-2026-08-09.md、reports/vcp-plugin-gap-analysis-2026-08-12.md、desktop-backend-capability-gap.md、yanshu-memory-donor-mapping.md、reports/r34-architecture-survey-2026-08-09.md

### 设置面三级即效语义 · 控件→生效时机对照表 + 测试证据 (原篇未标日期)

**来源**：`reports/settings-live-apply-contract.md`（116 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/settings-live-apply-contract.md`（原件逐字留存）

**元信息**

# 设置面三级即效语义 · 控件→生效时机对照表 + 测试证据
范围：`frontend/companion-desktop` 设置面（SettingsView.svelte / App.svelte /
ThemeSettingsPanel.svelte + 纯函数件 `src/lib/settings-live-apply.ts`）。
每类控件只有一条生效时机；页面「保存」按钮（页头按钮 + 底部保存栏）已删除。
## 三级即效语义
1. **开关 / 滑杆 / 预设**：点（或松手）即生效——立即走 apply 路径（await 推送），

**各节要点（机械摘取，非全文）**

- **三级即效语义**
  - 1. **开关 / 滑杆 / 预设**：点（或松手）即生效——立即走 apply 路径（await 推送），
  - 行内 pending（"正在应用到运行时…"），失败回填该项旧值 + 错误横幅。
  - 2. **文本输入**：失焦或回车提交（服务商组整组提交，避免半截配置）；字段旁 ✓
  - 瞬间确认，未提交显「未保存」微提示；失焦提交失败整组回填旧值 + 横幅。
- **控件→生效时机对照表**
  - | 页面 / 区块 | 控件 | 生效时机 | apply 路径 | 失败行为 |
  - | --- | --- | --- | --- | --- |
  - | 外观与主题 | 主题卡 | 1 点击即效 | onSave（App 侧回退 config + 文档主题） | 回退 + 横幅 |
  - | 外观与主题 | UI 配色卡 | 1 点击即效 | 同上 | 回退 + 横幅 |
- **测试证据**
  - `pnpm test` = 22/22 套件通过（既有 20 套件零回归 + 新增 2 套件）：
  - - 新增 `tests/settings-live-apply.mjs`（7 组断言，import 真实模块 + 源码镜像）：
  - 1. 开关即效接线（能力开关群 → handleCapabilityToggle → applyCapabilityNow → onSave；pending / 失败回填 / 横幅字面量在案）；
  - 2. 滑杆即效（六根滑杆 onchange → applyKnobNow；滑杆值直映注入字段 tune_memory_fade 等）；
- **校验状态**
  - | 校验 | 结果 |
  - | --- | --- |
  - | `pnpm check`（frontend/companion-desktop） | 0 errors / 0 warnings |
  - | `pnpm test` | 22/22 suites passed |
- **修订记录**
  - - **会话清理双档**（按 `docs/01-architecture/session-cleanup-options-spec.md` P0）：
  - 「数据与存储」的 `清空本地会话数据 / onClearLocalData`（只清本地正文、后端账本行
  - 经归并复活）改为双档真删链 `onClearSessionData('recent'|'all') → purgeSessions`
  - （`src/lib/chat-shell/session-cleanup.ts`）；危险登记表 4 件 → 5 件；两档文案均含

**归属链接**：scripts/check-neutral-terms.ps1、../docs/01-architecture/session-cleanup-options-spec.md

### R20 阶段 5 — Tauri 2.0 desktop scaffold skeleton 报告 (2026-08-05)

**来源**：`reports/tauri-2.0-scaffold-r20-stage-5.md`（215 行）｜**归档正文**：`docs/archive/merged/2026-10-23/reports/tauri-2.0-scaffold-r20-stage-5.md`（原件逐字留存）

**元信息**

```
// src-tauri/src/lib.rs
pub const TAURI_SCHEMA_VERSION: &str = "2";           // #1
pub const PLATFORM_NAME: &str = "apeireth";            // #2
pub const TAURI_MAIN_WINDOW_WIDTH: u32 = 1200;         // #3
pub const TAURI_MAIN_WINDOW_HEIGHT: u32 = 800;         // #4
pub const TAURI_MIN_WINDOW_WIDTH: u32 = 800;           // #5
pub const TAURI_MIN_WINDOW_HEIGHT: u32 = 600;          // #6
pub const TAURI_DEV_URL: &str = "http://localhost:1420"; // #7
pub const TAURI_BUNDLE_TARGETS: &[&str] = &["deb", "rpm", "appimage", "msi", "dmg"]; // #8
```

**各节要点（机械摘取，非全文）**

- **§1 必出文件 (10 个, 全部到位)**
- **1.1 src-tauri/ (Tauri 2.0 scaffold)**
  - | 文件 | 字节 | 行数 | 估范围 | 状态 |
  - | `Cargo.toml` | 3,348 | 74 | 1.5-1.8 KB / ~80 行 | ✅ |
  - | `tauri.conf.json` | 2,505 | 77 | 50-80 行 | ✅ |
  - | `build.rs` | 39 | 3 | (Tauri 必需) | ✅ |
- **1.2 docs/desktop/ (团队可见迁移计划)**
  - | 文件 | 字节 | 行数 | 状态 |
  - | `tauri-2.0-migration-plan-2026-08-05.md` | 10,173 | 158 | ✅ 100 行目标, 多 58 行 (含 §0-§8 详尽章节) |
- **1.3 dist/ (Tauri frontendDist 占位)**
  - | 文件 | 字节 | 说明 |
  - | `dist/index.html` | 366 | Tauri `generate_context!` macro 必需, W2 接管时换真前端 (TUI 共享) |
- **§2 编译期 hardcode 8 项 (K-1 强校验 #1 #2)**
- **§3 m3 防御 (TOOL_WHITELIST 8 工具, K-1 强校验 #3)**
  - 8 个 `#[tauri::command]`:
  - - 3 业务: `invoke_tool` / `invoke_stream` / `invoke_state`
  - - 5 工具: `invoke_window_create` / `invoke_window_close` / `invoke_menu_setup` / `invoke_tray_create` / `invoke_dialog_open` / `invoke_shell_open`
  - (注: 5 工具 IPC 我列了 6 个, TOOL_WHITELIST 是 8 名字包含 2 个 ipc 业务 = 总 8)
- **§4 K-1 强校验 4 条全过 (4 fixture 验证)**
  - | K-1 | 校验项 | 验证方式 | 结果 |
  - | **K-1.1** | "apeireth" 平台名 | `assert_eq!(PLATFORM_NAME, "apeireth")` | ✅ `k1_platform_name_is_apeireth` |
  - | **K-1.2** | 5 bundle targets | `assert_eq!(TAURI_BUNDLE_TARGETS.len(), 5)` + 5 contains | ✅ `k1_bundle_targets_count_5` |
  - | **K-1.3** | TOOL_WHITELIST 8 名字 | `assert_eq!(TOOL_WHITELIST.len(), 8)` + 8 fixture | ✅ `k1_tool_whitelist_has_8` + `tool_whitelist_8_names_match_fixture` |
- **§5 cargo check 验证 (0 error 0 warning)**
  - **最后 5 行 (0 error 0 warning)**: ✅
  - `cargo test --lib`: 18 passed; 0 failed; 0 ignored
  - `cargo run --example tauri_demo`: 7 节演示全过 (scaffold 跑通)
- **§6 0 触碰约束实查 (4 LOCKED 路径 mtime 对比)**
- **6.1 24 LOCKED crate mtime 0 drift**
  - (覆盖 24 LOCKED + 9 skeleton + 1 team-lead; 全部 mtime 16:34:11 baseline)
- **6.2 workspace root Cargo.toml (0 触碰)**
  - - 我未修改 `Apeireth-rust/Cargo.toml` (git status 显示 M 来自其他 R20 阶段 6 sub-agent)
  - - `src-tauri/Cargo.toml` 加空 `[workspace]` table 阻止被父 workspace 收编, **独立 workspace**
- **6.3 7 LOCKED 文档 (0 触碰)**
  - - `docs/v2-strategy/00-VISION.md` → `07-VCP-GAP-UPGRADE-PLAN.md` + README
  - - 全部 mtime 早于 12:00 (我开始工作 21:00+), 0 触碰

**归属链接**：../docs/archive/stage1/00-VISION.md、../docs/archive/desktop/tauri-2.0-migration-plan-2026-08-05.md

