# 后端 Gap 审计 R178 (2026-08-15)

> **作者**: Codex 后端工程师
> **基线**: R178 后端完工（`docs/r178/r178-backend-completion-2026-08-15.md`）+ mempalace 对比（`docs/research/mempalace-vs-apeireth-memory.md`）
> **触发**: 主人终极授权 + 后端"还要做"清单 8 项复核
> **日期**: 2026-08-15

---

## 1. TL;DR — 状态总表

| 优先级 | 项 | 状态 | 落地时间 | 工作量剩 | 阻塞 |
|---|---|---|---|---|---|
| **P0-1** | 5 Provider SDK 真接 (claude-code/codex/copilot/gemini-cli/opencode) | ❌ 未做（R35 设计态：1:1 翻译 shadow） | — | 5 周（可并行） | 需主人拍板 |
| **P0-2** | `/health/deps` 真探测（curl 真实可达性） | 🟡 **半成**（r178 加 8 个 config-level test，"real" 字段标识） | r178（2026-08-15） | 1 天 | 无 |
| **P0-3** | apeireth-memory ↔ apeireth-api 拆循环 | ❌ 未做（双向依赖已审计存在） | — | 1 周（含 SQLite 重构） | 需主人拍板 |
| **P0-4** | 4 memory provider 真接（S3/Redis/Postgres/Disk-LRU） | ❌ 未做（InMemory 真接，其余 stub） | — | 4 周（1 周/个） | 需主人拍板 |
| **P1-5** | ADR-0028 organ 双轨命名 | ✅ **完成**（`docs/adr/0028-organ-naming-bridge.md`, 5068B） | r178 前 | — | — |
| **P1-6** | ADR-0029 observability 命名 | ✅ **完成**（`docs/adr/0029-observability-naming.md`, 4465B） | r178 前 | — | — |
| **P1-7** | ADR-0030 version policy | ✅ **完成**（`docs/adr/0030-version-policy.md`, 4706B；含 0030-workspace-version-policy.md 4128B） | r178 前 | — | — |
| **P1-8** | README banner 增 R170-R178 | ❌ 未做（README 顶部 banner 止于 R169） | — | 30 min | 无 |

**总结**:
- ✅ **3 项完成**（P1 文档治理 3 个 ADR）
- 🟡 **1 项半成**（P0-2 health/deps 配置级完成，真探测待 1 天）
- ❌ **4 项未做**（P0-1/3/4 + P1-8，合计 ~10 周工作量）

---

## 2. P0 详细

### 2.1 P0-1: 5 Provider SDK 真接（5 周，可并行）

**现状**（r178 §6.1 复核）:
- R35 设计态：5 Provider 是 `1:1` 翻译上游 SDK 结构（**字段级影子**），**非真调 SDK**
- 当前 `provider_backend` 仅 `scripted` mock + env 探测

**真接路径**:

| Provider | 真 SDK crate | 工作量 | 备注 |
|---|---|---|---|
| claude-code | `@anthropic-ai/claude-agent-sdk` (Node → napi-rs) | 1 周 | 需要 Node sidecar 或 wasm runtime |
| codex | `@openai/codex` (已有 TS SDK) | 1 周 | 本机已装 `codex-cli 0.147.0`，可 napi-rs 调 |
| copilot | `@github/copilot-sdk` (Node) | 1 周 | GitHub 官方 |
| gemini-cli | `@google/gemini-cli` (Node) | 1 周 | |
| opencode | `opencode-sdk` (Node) | 1 周 | |

**架构问题**: 全是 Node SDK，Rust 端要么 napi-rs / wasmtime sidecar，要么 RPC over stdio。

**推荐**: **wasmtime + Node wasm**（apeireth-sovereignty 已有 WASM sandbox 模板）+ 各 SDK 包成 wasm module。**5 个并行 = 5 周**。

**阻塞**: 需主人拍板"是否值得为 SDK 真接引入 Node runtime 依赖"。

---

### 2.2 P0-2: `/health/deps` 真探测（剩 1 天）

**r178 已做** (`crates/apeireth-api/src/server.rs` + 8 新 test):
- ✅ 5 dep 配置级检查（provider_backend / memory_store / sovereignty_guard / replay_cache / rate_limiter）
- ✅ `"real": true/false` 字段标识（**不假装原则**）
- ✅ `unknown` 状态兜底

**剩 1 天**:
```rust
// provider_backend: 真实可达性 = HTTP HEAD {base_url}/v1/models (Bearer token)
// memory_store: SQLite .open() + SELECT 1
// sovereignty_guard: self_disable 状态查询（已暴露，R268 验证过）
// replay_cache: Redis PING / fs stat()
// rate_limiter: 实际 token-bucket 状态读
```

**不阻塞**，可在下一个 session 直接做。

---

### 2.3 P0-3: apeireth-memory ↔ apeireth-api 拆循环（1 周）

**循环现状**（审计确认）:
```
apeireth-memory → apeireth-api  (memory 用 api 暴露 endpoint)
apeireth-api   → apeireth-memory (api 暴露 memory 给前端)
```
**双向依赖** = 设计环。

**V2 方案**（per 主人 P0-3 spec）:
- 当前: 双 crate 直接互相 import
- V2: 用 **本地 SQLite 模拟**（每个 crate 自己开 SQLite db，绕过 import）

**拆法**:
1. `apeireth-memory` 暴露 trait `MemoryReadPort` / `MemoryWritePort`
2. `apeireth-api` 用 trait object + 自己持有 SQLite handle（**不 import apeireth-memory**）
3. 同样反向: `apeireth-memory` 用 trait object 调 API（**不 import apeireth-api**）
4. 测试: 拆完后 `cargo tree -p apeireth-memory | grep apeireth-api` 应为 0 hit

**工作量**: 1 周（含 12 个 cargo test + 2 Kani proof）

**阻塞**: 需主人拍板"是否值得失去直接类型检查换解耦"。

---

### 2.4 P0-4: 4 memory provider 真接（4 周）

**现状**（per `crates/apeireth-telemetry/src/cache/memory_provider/`）:
```
s3.rs         // TODO R21
redis.rs      // TODO R21
postgres.rs   // TODO R21
disk_lru.rs   // TODO R21
memory_provider.rs  // InMemory 真接 (real)
```

**真接路径**:

| Provider | crate | 工作量 | 阻塞 |
|---|---|---|---|
| S3 | `aws-sdk-s3` 或 `rust-s3` | 1 周 | 需主人提供 S3 凭据 |
| Redis | `redis-rs` | 1 周 | 需 Redis 实例 |
| Postgres | `sqlx` 或 `tokio-postgres` | 1 周 | 需 Postgres 实例 |
| Disk-LRU | 自研 + `lru` crate | 1 周 | 无（纯本地） |

**推荐顺序**: Disk-LRU → Redis → Postgres → S3（按"本地优先 → 云"）

**Disk-LRU 无阻塞**，可第一个做（1 周）。

---

## 3. P1 详细

### 3.1 P1-5/6/7: 3 个 ADR ✅ 已完成

| ADR | 文件 | 大小 | 状态 |
|---|---|---|---|
| 0028 | `docs/adr/0028-organ-naming-bridge.md` | 5068 B | ✅ |
| 0029 | `docs/adr/0029-observability-naming.md` | 4465 B | ✅ |
| 0030 | `docs/adr/0030-version-policy.md` | 4706 B | ✅ |
| 0030-workspace | `docs/adr/0030-workspace-version-policy.md` | 4128 B | ✅ |
| 0031 | `docs/adr/0031-reexport-organ-concept-unity.md` | 4180 B | ✅ (bonus) |

**P1-5/6/7 三项 = 0 剩余工作量**。

---

### 3.2 P1-8: README banner 增 R170-R178（30 min）

**现状**: `README.md` 顶部 banner 止于 R169，本 session 已经到 R178（+9 轮）。

**做法**（30 min）:
```bash
# 1. 读 README.md 当前 banner
# 2. 加 R170-R178 一行 summary (per r178 完工报告 §3)
# 3. 更新 "Latest" 链接
# 4. cargo check workspace 验证无 markdown lint 错误
```

**不阻塞**，下一个 session 直接做。

---

## 4. mempalace 借鉴交叉（叠加 P0/P1）

来源: `docs/research/mempalace-vs-apeireth-memory.md` §5

| 借鉴项 | 落入 P 桶 | 工作量 | 备注 |
|---|---|---|---|
| 5.1 Embedder 身份校验 | **新 P0-5** | 2 天 | 修当前 `HashEmbedder` 静默降级 bug |
| 5.2 Normalize 版本 | **新 P0-6** | 1 天 | `semantic.rs` 改 chunk 策略后旧 chunk 不会 stale |
| 5.3 Dedup 模块 | **新 P1-9** | 2 天 | 同一 session 多次写入近重复 |
| 5.4 走廊结构 | **新 P1-10** | 3 天 | `continuity_link` 升级 weight + co_occurrence |
| 5.5 时序知识图谱 | **新 P2** | 1 周 | Zep-killer 卖点 |
| 5.6 4 层渐进加载 | **新 P1-11** | 3 天 | wake-up ~700 tokens |

**新增 6 项**（P0: 2 / P1: 3 / P2: 1），合计 **~3 周**。

---

## 5. r178 完工基础（不可回退）

per `docs/r178/r178-backend-completion-2026-08-15.md`:

- ✅ 22404 cargo tests PASS / 0 failed / 70 ignored / 342 suites
- ✅ 80+ active crates, 24 LOCKED crate 0 触碰
- ✅ 6 哲学锚穿透 100% (S-1/S-2/O-2/O-3/O-4/O-5)
- ✅ 8 项不修改承诺 0 违反
- ✅ workspace version 1.2.0 严守
- ✅ `/health/deps` 端点 + 8 新测试（**本 session 唯一非 LOCKED 修改**）

**本审计 0 触碰承诺**: 不写新代码（**仅文档**），不触碰 LOCKED crate。

---

## 6. 推荐执行顺序

### 立即（下一个 session, 1 周内）

1. **P0-2 真探测补完** (1 天) — 阻塞 0
2. **P1-8 README banner** (30 min) — 阻塞 0
3. **新 P0-5 Embedder 身份校验** (2 天) — 阻塞 0
4. **新 P0-6 Normalize 版本** (1 天) — 阻塞 0
5. **Disk-LRU 真接** (1 周) — 阻塞 0

### 短期（2-4 周）

6. **新 P1-9 Dedup 模块** (2 天)
7. **新 P1-10 走廊结构** (3 天)
8. **新 P1-11 4 层渐进加载** (3 天)
9. **P0-3 拆循环** (1 周) — 需主人拍板

### 中期（1-2 月）

10. **P0-4 剩余 3 provider** (Redis + Postgres + S3, 各 1 周)
11. **P0-1 5 Provider SDK 真接** (5 周并行) — 需主人拍板 Node runtime 依赖
12. **新 P2 时序知识图谱** (1 周) — 需主人拍板是否上 graph-primitive

---

## 7. 决策点（等主人拍板）

| # | 决策 | 影响 |
|---|---|---|
| 1 | P0-1 是否引入 Node runtime 调真 SDK？ | 影响 sovereignty / wasm 边界 |
| 2 | P0-3 是否值得失去直接类型检查换解耦？ | 影响 memory-api 协同开发体验 |
| 3 | P0-4 cloud provider (S3/Postgres/Redis) 是否都做？ | 影响部署形态 |
| 4 | mempalace 借鉴 6 项是否全部采纳？ | 影响 R179-R182 路线图 |
| 5 | README banner 增量范围（R170-R178 还是 R170-R183）？ | 影响文档一致性 |

---

## 8. 验证命令

```powershell
cd Apeireth-rust
cargo check --workspace --tests           # 0 errors / 68 warnings (本审计不动代码)
cargo test --workspace --no-fail-fast     # 22404 PASS / 0 fail (r178 baseline)
ls docs/adr/ | grep -E "002[89]|0030"     # 3 ADR 已存
cat docs/audit/backend-gap-audit-r178.md  # 本文档
```

---

_作者: Codex 后端工程师_
_日期: 2026-08-15_
_基线: R178 完工 + mempalace HEAD 对比_
_本文档 0 commit / 0 push / 0 代码修改_
