# ADR 0017: D-01 工具 endpoint 真接 (calendar + message 2 工具)

> **状态**: 🟢 Accepted (主人 2026-08-05 20:53 推翻 A 推荐 stub 501, 选 B 真接)
> **commit 锚**: `r20-stage-2-3-prep-2026-08-05.md` §3.5 + `crates/apeireth-api/src/v1_tools/` 实施
> **最后更新**: 2026-08-05

---

## 1. 背景 (Context)

Apeireth v1 6 工具 (calendar / message / contact / task / search / drive) 实施方式:

### 选项对比

| 选项 | 描述 | 估时 | 阻塞 |
|---|---|---|---|
| **A** (原推荐) | 6 工具全 stub 返 501 | 1 owner × 1 周 | R21 估补 |
| **B** (本 ADR 拍板) | 6 工具全真接 (除写操作) | 1 owner × 4 周 | 1.0 release 阻塞 |

---

## 2. 决策 (Decision)

**6 工具全真接, 写操作 (create / update / delete) 留 R21**

**真接范围**:
- ✅ calendar 读 (list_events / get_event) — iCal/Google Calendar
- ✅ message 收发 (send / list / get) — SMTP + 飞书 webhook
- ✅ contact 读 (lookup / list) — 本地 contacts 表
- ✅ task 读 (list / get) — 本地 tasks 表
- ✅ search 全部 (query / index / delete) — tantivy + sqlite-vec
- ✅ drive 全部 (upload / download / list / delete / get_metadata) — S3/MinIO + 本地

**501 stub 范围** (R21 估补):
- ❌ calendar 写 (create / update / delete) — 需 iCal/Google 双向同步
- ❌ message mark_read — 需 IMAP 同步
- ❌ contact 写 (create / update / delete) — 1.0 商业化才实装
- ❌ task 写 (create / update / complete / delete) — 1.0 商业化才实装

**理由**:
- 1.0 release 演示给主人需要"真东西"
- 读操作 + search + drive 覆盖 80% 用户场景
- 写操作 501 stub 在 R21 商业化版补 (per D-05)

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **1.0 release 演示可行**: 真东西 vs 全 stub
- ✅ **80% 场景覆盖**: 读 + search + drive
- ✅ **客户能跑通 end-to-end**: 上传文件 → 搜索 → 发送
- ✅ **R20 阶段 2 真接是 R17 战役 0-4 收官的延续**: 一致性

### 3.2 负面

- ⚠️ **估时增加 3 周**: 1 owner × 4 周 vs 1 周
- ⚠️ **上游 API 凭据风险**: SMTP/iCal/Google Calendar 需生产凭据
- ⚠️ **写操作返 501 不一致**: 6 工具混合真接 + 501, 用户困惑

### 3.3 风险

- 真接的上游 (SMTP/iCal) 挂掉 → 1.0 release 演示受影响
- 写操作 501 让 R21 商业化压力 (必补)

---

## 4. 备选 (Alternatives Considered)

### A. 6 工具全 stub 返 501 (原推荐)
- 优点: 1 owner × 1 周搞定, R21 再补
- 缺点: 1.0 release 演示不真实, 主人 2026-08-05 拍"不行, 真接"
- 否决: 主人 20:53 拍板 B

### B. 6 工具全真接 (本决策)
- 优点: 1.0 release 真实可用
- 缺点: 估时增加 3 周
- 拍板: 主人 20:53 拍 B

### C. 6 工具全真接 + 写也真接
- 优点: 全功能
- 缺点: 估时增加更多, R21 商业化重复
- 否决: 写操作 1.0 商业化未决, 跟 D-05 quota stub 矛盾

### D. 仅 calendar + message 真接
- 优点: 折中
- 缺点: 跟主人 20:53 拍板矛盾 (主人原话: "全真接")
- 否决: 主人原话已明示

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: 业界产品 1.0 必有真功能
- ✅ **S-2 实事求是**: 1.0 release 不能全 stub
- ✅ **O-2 用户看结果不看哲学**: 用户只看能不能用
- ✅ **O-3 信息密度"高"**: 6 工具真接 + 写操作 501 一表说清
- ✅ **O-4 干净状态 = 没有历史包袱**: 写操作 1.0 不做 = 不留半成品
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 6 工具读真接 + 写 501 全部诚实标注
- ✅ **编译期 hardcode**: 6 工具白名单 + 各工具 action enum 编译期固定
- ✅ **不改 LOCKED**: API 协议层 LOCKED
- ✅ **不改 workspace version**: v1.0.0 严守
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 自建 tool 注册
- ✅ **不重复造轮子**: 沿用 apeireth-tools 既有工具
- ✅ **诚实标缺**: 写操作 501 明确 R21 估补

---

## 7. 引用

- 决策 ID 体系: `docs/stage4/pending-decisions-overview-2026-08-05.md` (D-01)
- 蓝图: `docs/stage4/r20-stage-2-3-prep-2026-08-05.md` §3.5
- 实施:
  - `crates/apeireth-api/src/v1_tools/calendar.rs`
  - `crates/apeireth-api/src/v1_tools/message.rs`
  - `crates/apeireth-api/src/v1_tools/contact.rs`
  - `crates/apeireth-api/src/v1_tools/task.rs`
  - `crates/apeireth-api/src/v1_tools/search.rs`
  - `crates/apeireth-api/src/v1_tools/storage.rs`
- 文档: [`docs/api/v1-tools.md`](../api/v1-tools.md)
- 工具: [`docs/api/v1-tools-{calendar,message,contact,task,search,drive}.md`](../api/)

---

## 8. 附录 (本附录为 ADR 收口追加, 不动兄弟 sub-agent 的 §1-§7)

### 8.1 6 工具 endpoint 详细映射

| # | 工具 | endpoint 路径 | 读真接 | 写真接 | 1.0 release 状态 |
|---|---|---|---|---|---|
| 1 | calendar | `POST /v1/tools/calendar/invoke` | list_events / get_event | create / update / delete (501) | 读真接 + 写 501 |
| 2 | message | `POST /v1/tools/message/invoke` | list / get | send ✅ / mark_read (501) | 收真接 + mark_read 501 |
| 3 | contact | `POST /v1/tools/contact/invoke` | lookup / list | create / update / delete (501) | 读真接 + 写 501 |
| 4 | task | `POST /v1/tools/task/invoke` | list / get | create / update / complete / delete (501) | 读真接 + 写 501 |
| 5 | search | `POST /v1/tools/search/invoke` | query | index / delete ✅ | 全真接 |
| 6 | drive | `POST /v1/tools/drive/invoke` | list / get_metadata / download | upload / delete ✅ | 全真接 |

### 8.2 写操作 501 stub 详细列表 (R21 估补)

| 操作 | 估时 | owner | 依赖 |
|---|---|---|---|
| calendar create / update / delete | 1 owner × 3 周 | backend-engineer | iCal / Google Calendar OAuth 双向同步 |
| message mark_read | 1 owner × 1 周 | backend-engineer | IMAP IDLE 命令 |
| contact create / update / delete | 1 owner × 1 周 | backend-engineer | 1.0 release 无业务实装 |
| task create / update / complete / delete | 1 owner × 2 周 | backend-engineer | 1.0 release 无业务实装 |
| **合计** | **1 owner × 7 周** | | |

### 8.3 1:1 翻译 v0.9.21 商业版 6 tool.js 映射

| 工具 | v0.9.21 商业版 (TS) | 本仓库 (Rust) | 翻译原则 |
|---|---|---|---|
| calendar | `calendarTools.js` (估 800 LOC) | `crates/apeireth-api/src/v1_tools/calendar.rs` (估 500 LOC) | 1:1 行为, Rust idiom |
| message | `messageTools.js` (估 600 LOC) | `crates/apeireth-api/src/v1_tools/message.rs` (估 400 LOC) | 1:1 行为, Rust idiom |
| contact | `contactTools.js` (估 400 LOC) | `crates/apeireth-api/src/v1_tools/contact.rs` (估 300 LOC) | 1:1 行为, Rust idiom |
| task | `taskTools.js` (估 500 LOC) | `crates/apeireth-api/src/v1_tools/task.rs` (估 350 LOC) | 1:1 行为, Rust idiom |
| search | `searchTools.js` (估 700 LOC) | `crates/apeireth-api/src/v1_tools/search.rs` (估 500 LOC) | 1:1 行为, tantivy + sqlite-vec |
| drive | `driveTools.js` (估 900 LOC) | `crates/apeireth-api/src/v1_tools/storage.rs` (估 600 LOC) | 1:1 行为, S3/MinIO |

### 8.4 6 工具 endpoint 调用流程图 (end-to-end)

```
TUI / Tauri / 第三方客户端
  ↓ POST /v1/tools/{name}/invoke + Bearer token
apeireth-api (axum router)
  ↓ 1. 鉴权 (per D-03 链接 token, 5min TTL)
  ↓ 2. 限流 (per D-04 token bucket, 3 档)
  ↓ 3. audit 入口 (per 5 守门 #4)
apeireth-tools dispatcher (per 24 LOCKED crate apeireth-tools)
  ↓ 派发到 6 工具 plugin (per 24 LOCKED crate apeireth-extension)
  ↓ ┌─ calendar →  iCal/Google Calendar Adapter
  ↓ ├─ message  →  SMTP + 飞书 webhook
  ↓ ├─ contact  →  本地 contacts 表 (SQLite 5 表之一)
  ↓ ├─ task     →  本地 tasks 表 (SQLite 5 表之一)
  ↓ ├─ search   →  tantivy + sqlite-vec (RAG)
  ↓ └─ drive    →  S3/MinIO + sha256 去重
  ↓ 4. audit 出口
  ↓ 5. SSE 流式 (per reqwest stream feature, 8 帧)
客户端 (SSE 流式接收)
```

### 8.5 6 工具 1.0 release 测试用例 (per §2.3, 1.0 release #2 test 0 错)

| 工具 | 测试数 | 关键测试 |
|---|---|---|
| calendar | 12 | list_events 7 天窗口, get_event by id, create 501, update 501, delete 501 |
| message | 10 | send SMTP 测试 (mock), list 分页, get by id, mark_read 501 |
| contact | 8 | lookup by email/phone, list 1000 行分页, create 501 |
| task | 8 | list 5 状态过滤, get by id, create 501, complete 501 |
| search | 15 | query 关键词 + filter, index 100 文档, delete by id |
| drive | 17 | upload 1GB 测试, download 限速, list 分页, delete 幂等 |
| **合计** | **70** | (含 5 守门 #4 audit 测试 7 个) |

### 8.6 6 工具 endpoint 性能预算 (per 1.0 release #7 perf)

| 工具 | P50 | P95 | P99 | 备注 |
|---|---|---|---|---|
| calendar list_events (30 天) | 50ms | 200ms | 500ms | iCal 缓存命中 |
| message list (100 条) | 30ms | 100ms | 200ms | SQLite 索引 |
| contact lookup | 10ms | 30ms | 50ms | SQLite 主键 |
| task list (5 状态) | 20ms | 80ms | 150ms | SQLite 索引 |
| search query (10 文档) | 80ms | 300ms | 800ms | tantivy + sqlite-vec |
| drive upload (1GB) | 5s | 15s | 30s | S3/MinIO multipart |
| drive download (100MB) | 1s | 3s | 8s | S3/MinIO signed URL |
