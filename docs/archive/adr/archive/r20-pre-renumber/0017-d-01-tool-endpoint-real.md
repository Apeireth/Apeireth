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
