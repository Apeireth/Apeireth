# B1 Web 面板 v2 — 全栈工程师自审报告

**Task ID**: bd6049a4-33c1-4929-98f6-67db7bfcd492
**角色**: fullstack_engineer
**日期**: 2026-08-17
**任务**: Web 面板 v2（会话管理/记忆浏览/图谱可视化/授权中心/审计视图）

---

## 0. 交付摘要

| 项 | 状态 | 证据 |
|---|---|---|
| 后端 7 个只读面板端点 | ✅ 已提交 + 9/9 单测 | `crates/apeireth-api/src/panel_readonly.rs`（提交 0913550，合并后复验通过） |
| 静态多页面板（6 页 + css + js） | ✅ 资产就绪 | `crates/apeireth-companion/assets/panel/`（8 文件，原生 JS，无 Node 构建链） |
| companion_serve 接线（/panel + /v1/panel nest） | ✅ 代码就绪（include_str! 零新依赖） | `examples/companion_serve.rs` 工作区改动 |
| curl 端点验证 | ⏳ 见 §4 | 被并行在途改动阻塞（详见 §5） |
| 文档同步（模块地图 + release-plan） | ✅ | `docs/maintenance-guide.md` + `docs/release-plan.md` |

## 1. 边界遵守（只加不改）

- **api crate**：仅新增 `panel_readonly.rs` 模块 + `lib.rs` 一行 `pub mod` + `endpoints.rs` 审计表登记（30→37，审计测试同步通过）。未改任何已有端点语义。
- **companion**：仅新增 `assets/panel/` 目录 + `companion_serve.rs` 追加 `/panel` 静态路由与 `/v1/panel` nest（hunks 独立，不触碰已有逻辑）+ `chat.html` 头部加一个面板入口链接。
- **tool 系内部 0 触碰**；授权批准复用已有 `POST /v1/apeireth/grant`（master token），**未新建任何安全口**。

## 2. 后端设计（数据真接口）

7 个只读 GET 端点，`panel_router(Arc<SqliteMemoryStore>)` 构造，nest 于 `/v1/panel`：

| 端点 | 数据源（真接口） | 面板用途 |
|---|---|---|
| `GET /sessions` | `SessionStore::list_all_sessions` + `count_by_session` | 会话列表 |
| `GET /sessions/:id/timeline` | `EpisodeStore::query(for_session)` | 时间线浏览 |
| `GET /memory/streams` | `history_streams::StreamDepth::query_by_name`（6 流） | 记忆流浏览 |
| `GET /memory/episodes` | `EpisodeStore::query` + 子串过滤 | 记忆列表+搜索 |
| `GET /graph` | factg-*/link-* episodes（companion `SqliteGraphBackend` 同一持久形态） | 图谱可视化 |
| `GET /approvals` | apreq-* episodes，chain 取最新 rev（对齐 `companion::approval_requests::list` 语义） | 授权中心 |
| `GET /audit` | `RecordStore::list_for_tool` / `ActionStream::list_recent`（与 `audit_log` 工具同源） | 审计视图 |

依赖合规：`cargo tree` 实测 apeireth-memory / apeireth-tool-runtime 两树均不含 apeireth-api（R179 P0-3 已拆循环边）→ api 可安全直依赖，无循环。

升级点（如实标注）：
- 会话管理现挂 SessionStore；backlog **N2 OneRing 统一账本**就绪后前端换源（JSON 形状保持 list+count）。
- 图现直读 episodes 形态；图后端换结构化 `GraphQuery`/Kùzu 后换 `/panel/graph` 实现即可。
- 授权"拒绝"无现有机制 → 面板如实标注未接线（不予批准即视为拒绝），0 装 PASS。

## 3. 前端设计（静态资产优先，R19+ 砍前端决策）

- 6 页：index（总览+健康）/ sessions / memory / graph / approvals / audit，共享 `panel.css` + `panel.js`。
- 纯原生 JS + fetch，**无任何构建链/框架**；所有数据渲染经 `esc()` HTML 转义。
- 图谱：确定性环形布局 SVG，节点可点击过滤，predicate 边标签 + importance 线宽。
- 授权中心：只读列表 + 批准表单调用**已有** `/v1/apeireth/grant`（master token 由主人输入，同 chat.html grant 面板模式）。
- 分发：`include_str!` 编译期内嵌（与 chat.html 同形态，单二进制零运行时文件依赖），白名单匹配防路径穿越。

## 4. 测试与验证

### 4.1 单测（已通过）

`cargo test -p apeireth-api --lib panel_readonly` → **9/9 PASS**（tower oneshot + in-memory store）：
- sessions 计数 / timeline / streams（含非法 kind→400）/ episodes 搜索 / graph 过滤 / approvals chain 去重 / audit 列表+过滤+masked 脱敏。

端点审计表：`cargo test -p apeireth-api --lib audit_` → 12/12 PASS（37 端点计数+唯一性+TIER_0 守门）。

### 4.2 curl 端点验证

（待 companion 编译恢复后补录——阻塞原因见 §5）

## 5. 并行协作阻塞记录（如实）

验证期间工作区存在其他成员的在途改动（非本任务文件，未触碰）：
1. `crates/apeireth-companion/src/job_object.rs` 在途改动曾致 E0277（B3 沙盒方向）→ 阻塞 companion lib 编译；
2. companion_serve.rs 有 **N2 OneRing 账本**并行实现（正是本任务会话管理的升级点来源），其中 chat_completions 改动一度 Handler 约束不满足；
3. 本任务面板接线与上述改动 hunks 不重叠；已通报 Leader，等待对方提交/修复后完成最终 curl 验证并提交面板资产。

## 6. 提交清单

| 提交 | 内容 |
|---|---|
| 0913550 | feat(api): B1 面板只读端点 7 件（panel_readonly）+ ENDPOINTS 30→37 + 9 单测 |
| （待） | feat(web): B1 静态面板 8 资产 + companion_serve /panel 接线 + chat.html 入口 |
| （待） | docs: B1 文档同步（模块地图 + release-plan GUI 包进度） |

## 7. 0 装 PASS 自查

- ✅ 未接视图如实标注：授权"拒绝"机制未接线；桌面托盘/每日摘要 UI 不在本任务（GUI 包其余项）。
- ✅ 端点全部真读 SQLite 持久层，无 mock/桩数据；测试种子是内存 store 真写入。
- ✅ 不假装批准闭环：批准后请求状态随对话流转更新（面板只读），文案如实展示。
