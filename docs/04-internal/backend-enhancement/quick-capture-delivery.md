# 随手记（Quick Capture）交付说明 — C 轴入口面

> `Status: ✅ 已交付（2026-10-13）`
> 一键记一条 → 未整理原料入记忆流（owner 随记习惯的产品化，C-memory-deepening「随手记」条的入口面）。
> 战场声明：主战场 `frontend/companion-desktop/src/features/quick/**`；随行 `tests/quick-capture.mjs` +
> `tests/run-all.mjs`（测试登记）+ `src/lib/design/base.css`（样式随行）。
> 未动：`src/lib/runtime.ts`（`appendMemoryEpisode` 原样复用，签名零改）、`crates/engine/memory`
> （备注级标注随文携带，后端无需改动——理由见 §2.3）。禁区零触碰。

---

## 1. 交互说明

### 1.1 入口（三选一取最简 = 斜杠 `#` 前缀）

| 动作 | 行为 |
| --- | --- |
| 输入 `#` 开头（如 `# 明早买牛奶`） | 快捷窗**即时切到随手记模式**（模式随输入派生，无独立 tab/新窗格） |
| Enter 或发送键（笔形图标） | 一键记入记忆流；成功后输入框清空、反馈「已记下 ✓」 |
| 删除 `#` 前缀 | 即时回到对话模式（placeholder/按钮/回车路由同步切换） |
| 对话模式照旧 | Enter/发送 = 对话回合（`runtime.run`），与本批互不干扰 |

### 1.2 落地反馈与回看

- **落地反馈**：输入框下方绿色「已记下 ✓」（`role="status"`）；拒绝/失败走红色错误帧
  （「内容为空，没记下」/「写入记忆流失败：…」），**不谎报成功**（错误即帧）。
- **回看**：窗内「最近随手记」只读列表（最近 **5** 条，新→旧；行 = `#序号 原文 HH:mm`），
  账本自管（localStorage 读缓存，上限 50 条丢最旧、序号不动）；记忆流是真源。
- **空内容拒**：`#` 后无内容（纯空白算空）显式拒绝——零副作用、零发号、不入账本。

### 1.3 零打扰纪律（纯采集零 token）

| 纪律 | 落法 | 钉法 |
| --- | --- | --- |
| 不走对话回合 | 采集分支与 `submit()`/`runtime.run` 完全分叉 | 镜像断言：`handleCapture` 区无 `runtime.run`/`submit(` |
| 不调模型 | 采集面唯一请求 = 记忆追加 | 行为断言：fetch 记录面唯一 URL = `/v1/memory/append`，零 `/chat/completions` |
| 不发通知 | 快捷窗零通知 API | 镜像断言：两文件零 `notification`/`notify` |
| 不烧 token | 上两条的合取 | 两条同时钉住 |

## 2. 记忆分级标注（capture = 未整理原料级）

### 2.1 备注级标注格式

采集记忆随文携带备注级标注（正文唯一承载面，格式常量同源可逆）：

```
[随手记 #N] <原文>（来源 capture）
```

| 构件 | 语义 |
| --- | --- |
| `随手记 #N` | 分级标签 + 连记序号（与教训「教训 #N」/里程碑同族分级呼应） |
| `<原文>` | 未整理原料本体，不改写不截断 |
| `（来源 capture）` | **来源标注 `capture`**（采集来源一等字段的真实序列化） |

### 2.2 分级语义

- **capture = 未整理原料级**：`protection=off`（不走 protect/unprotect/forget 通道，落流即未保护）、
  **可被后续整编（consolidation）吸收**；
- 账本记录一等字段：`{seq, text, at, source: 'capture', protected: false}`——
  分级与来源是字段不是暗记（测试逐项钉住）；
- **序号规则**：发号在写流之前（正文随号落流）、发号即持久、号只发不收——
  写流失败留空档不复用不改写（如实发号史）；并发连记序号连续（1,2,3…）。

### 2.3 诚实边界（为什么不改后端）

- 契约 §5 记忆追加缝只收 `{session, role, content}`，后端 schema 无 source/category 列
  （rc 诚实边界，见 `panels.rs` EpisodeDto 注释）；来源标注**随文序列化**与深流
  `buildMemoryContent`（`[深流/kind] text（来源 …）`）同一先例口径——真实一等字段的
  随文携带，不是虚构分类；
- 整编吸收缝：`parseCaptureNote(content) → {seq, text} | null` 可逆还原原文
  （原文自带尾注也不吞），后续 consolidation 取原料时用它剥标注；
- 因此 `crates/engine/memory` 的 episode 入口**不需要动**（「只加不改」许可未使用，
  零后端风险、零孤儿代码）。

## 3. 接线图

```
用户输入（快捷助手窗输入框，QuickWindowView.svelte）
  │  detectQuickMode(input)：`#` 前缀 = capture，否则 chat（随输入即时切换）
  ▼
handleCapture()                          ── 不进对话回合（runtime.run 零触达）
  │
  ▼
captureToMemoryStream(config, ledger, input)        [features/quick/quick-capture.ts]
  ├─(1) CaptureLedger.begin(input, at)
  │       ├─ parseCaptureInput：拒帧 empty / not-capture（零副作用零发号）
  │       ├─ 发号 seq（发号即持久，号只发不收）
  │       └─ buildCaptureNote → "[随手记 #N] <原文>（来源 capture）"
  ├─(2) appendMemoryEpisode(config, content)        [src/lib/runtime.ts，原样复用=同源缝]
  │       └─ POST /v1/memory/append {session, role:'user', content}
  │            └─ gateway panels.rs::append_episode
  │                 └─ engine episode 存储（append-only，protected=false=未保护级）
  └─(3) CaptureLedger.commit(record)                [localStorage 读缓存账本]
          └─ recent(5) → 「最近随手记」只读列表（回看）
```

分叉关系（零 token 的结构保证）：

```
query ── `#` 前缀 ──▶ handleCapture ──▶ captureToMemoryStream ──▶ POST /v1/memory/append
     └─ 其他      ──▶ submit        ──▶ runtime.run（对话回合，模型通道，与采集面无交集）
```

## 4. 测试证据

### 4.1 新增套件 `tests/quick-capture.mjs`（10 checks，全绿）

| # | 检查 | 钉住的行为 |
| --- | --- | --- |
| ① | 模式切换 | `#` 前缀=随手记/去前缀=对话，前导空白/裸 `#`/往返切换全表 |
| ② | 入记忆流 | 唯一 POST `/v1/memory/append`（appendMemoryEpisode 同源缝），role=user、session 非空、内容=原文+标注 |
| ③ | 来源标注 | `[随手记 #N] <原文>（来源 capture）`，`parseCaptureNote` 可逆（含原文自带尾注），他族标注不误认 |
| ④ | 未保护级 | `source='capture'`、`protected=false`，零 protect/unprotect/forget 通道 |
| ⑤ | 零模型调用 | 采集唯一请求面=记忆追加，零 `/chat/completions`（纯采集零 token） |
| ⑥ | 回看列表 | 最近 5 条新→旧只读、N 可调、返回副本（改它不动账本） |
| ⑦ | 空内容拒 | `#`/`#␣␣␣`=empty、非 `#` 输入=not-capture，零副作用零发号零入账 |
| ⑧ | 并发连记 | 3 条并发序号连续（1,2,3）且条条入流、正文随号不串位 |
| ⑨ | 账本 | 持久化 round-trip / 超限丢最旧序号不动 / 发号即持久不复用（失败号留空档） |
| ⑩ | 接线镜像 | 模式切换/采集缝/「已记下 ✓」/回看在案；采集分支零对话零通知；采集模块零模型面；关闭缝零回归 |

### 4.2 回归与门禁（实测输出）

| 门禁 | 结果 |
| --- | --- |
| `pnpm test` | **74/74 套件全绿**（73 既有 + 新增 quick-capture 1 套）— 既有 73 套件零回归 |
| `pnpm check` | svelte-check **0 errors, 0 warnings** |
| `cargo test --workspace --locked -j 2` | 见 §4.3 门禁汇总 |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | 见 §4.3 门禁汇总 |
| `cargo fmt --all -- --check` | 见 §4.3 门禁汇总 |
| `scripts/check-neutral-terms.ps1` | `OK 61 changed files: added lines all neutral`（零命中） |
| `scripts/secret-scan.ps1` | `[OK] No secrets found.` |

### 4.3 门禁汇总（2026-10-13 实测，无脚本不发布数字）

| 门禁 | 实测结果 |
| --- | --- |
| `cargo test --workspace --locked -j 2` | ✅ **退出码 0**；结果行 **190 套件 / 5243 例通过 / 0 失败 / 22 忽略**（既有 186 套件零回归；多出的 4 套件为并行线在途新增面）。注：本批取证前一轮同命令出现一次退出码 101 瞬时红，隔离复跑即绿——期间同 build 目录出现并行 `cargo` 抢锁（"Blocking waiting for file lock"），按 flake 口径复跑定性为**并行在途扰动**，非本批引入 |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | ❌ **标红（非本批引入）**：编译错误 2 处全在 **J1/I1 线在途新件**——`crates/capabilities/tools/src/content_feed/tool.rs:532`（E0277）、`crates/capabilities/tools/src/skill/distill.rs:521`（E0308）。该战场本批**禁入**（J1/I1 线战场），零触碰零代改，报备待该线自检收口 |
| `cargo fmt --all -- --check` | ❌ **标红（非本批引入）**：4 文件不净——`gateway/src/domain_context.rs`、`engine/runtime-assembly/tests/user_domain_isolation.rs`（**W3 线在途**，本批禁入）、`gateway/src/session_settings.rs`、`capabilities/tools/src/lib.rs`（并行在途）。本批**零 Rust 文件触碰**（`git status` 佐证），按「他人在途文件不乱动」不代改 |
| `pnpm check` | ✅ svelte-check **0 errors, 0 warnings** |
| `pnpm test` | ✅ **74/74 套件全绿**（终态复跑过两次，含交付前最后一处视图微调之后） |
| `scripts/check-neutral-terms.ps1` | ✅ `OK … added lines all neutral`（零命中） |
| `scripts/secret-scan.ps1` | ✅ `[OK] No secrets found.` |

> 口径说明：工作区级 Rust 门禁（clippy/fmt）当前未全绿——命中面 100% 属两个并行在途批次
> （W3 用户域线、J1/I1 feeds/skill 线）的未收口文件，按纪律如实标红、不代改、不谎报；
> 本批（随手记）自身战场：前端门禁全绿、零 Rust 触碰、测试 10 检查全绿。

## 5. 零债声明

- 不留 TODO/备忘注释；`quick-capture.ts` 每个符号名实相符（发号/入账/回看/拒帧各有其名）；
- 未改 `appendMemoryEpisode` 签名与语义（既有 MemoryView/深流调用方零回归，pnpm 73 套件为证）；
- 未动 `crates/engine/memory` 与一切禁区文件；W3/J1/I1 线战场零触碰；
- 备注级标注是真实一等字段序列化（来源/分级），不是暗分类；可逆解析缝随行交付。
