# Golutra 借鉴 #6 — TUI State 共享模式 (报告)

**作者**: 楚零 (Mavis 派 1 of 4 worker, 4 小时硬限内完成)
**日期**: 2026-08-06 02:00
**任务**: 借鉴 Golutra 9 Tauri state 模式 (OnceLock + Arc + Mutex) 转 TUI 等价物 (ratatui state 共享框架)
**状态**: ✅ 完成, 不主动 commit (留 Mavis 整合 #3 拍板)

---

## 1. 新文件清单 (11 文件, 2709 行新代码)

### `crates/apeireth-state/` 独立新 crate (11 文件, 2709 行)

| 文件 | 行数 | 描述 |
|------|-----:|------|
| `Cargo.toml` | 35 | `[lints] workspace = true` (借 workspace.lints), 0 引 tokio, 0 改 workspace version |
| `src/lib.rs` | 186 | 顶层 + 6 哲学锚穿透 + 8 项承诺 + re-exports + 5 编译期 hardcode 守门 |
| `src/error.rs` | 219 | `StateError` (5 变体 thiserror) + `StateErrorKind` 序列化摘要 |
| `src/shared_state.rs` | 195 | `SharedState<T>` trait + `SharedStateMode` (3 变体) + `StateReadGuard` / `StateWriteGuard` enum dispatch |
| `src/mode_once_lock.rs` | 240 | **模式 1**: `OnceLockState<T>` 进程全局 lazy init (借鉴 Golutra `OnceLock<Arc<T>>`) |
| `src/mode_mutex.rs` | 236 | **模式 2**: `MutexState<T>` 跨线程互斥 (借鉴 Golutra `tauri::State<Mutex<T>>`) |
| `src/mode_rw_lock.rs` | 256 | **模式 3**: `RwLockState<T>` 跨线程读写锁 (借鉴 Golutra `tauri::State<RwLock<T>>`) |
| `src/organ.rs` | 242 | `Organ` 9 器官 enum + `ORGAN_COUNT` / `ORGAN_NAMES_ZH` / `ORGAN_ASCII_CHARS` 编译期 hardcode + 9 `OrganStub` 类型族 (宏生成) |
| `src/registry.rs` | 330 | `OrganStateRegistry` 9 字段聚合 + `OrganStateRegistryBuilder` (per-organ 模式覆盖) |
| `examples/state_sharing_demo.rs` | 218 | **1 完整例子**: 3 模式演示 + 9 器官并发访问 (3 线程) + 7 段 (OnceLock / Mutex / RwLock / Registry / 并发 / Trait dispatch / 9 Stub) |
| `tests/test_state_sharing.rs` | 552 | **30 集成测试**: 3 模式 mock + 9 器官并发访问 + 8 项承诺 + 6 哲学锚穿透 + 5 variant 错误传播 |

**总: 2709 行, 11 文件** (skeleton 阶段, R21+ 续做 async/真接)

---

## 2. workspace Cargo.toml 改动 (0 改 version, +1 member)

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -170,6 +170,13 @@ members = [
     "crates/apeireth-update",
+    # R21 借鉴 Golutra #6: 9 Tauri state 模式 (OnceLock + Arc + Mutex) 转 TUI 等价物 (ratatui state 共享框架).
+    # per analysis/golututra/BORROW_FROM_GOLUTRA.md §8 P1 第 9/10 项, 主 2026-08-06 01:55 派活.
+    # 3 模式 (OnceLockState / MutexState / RwLockState) + 9 器官 state 共享 (heart/brain/hand/eye/ear/memory/voice/body/mind) +
+    # 1 完整 state sharing 例子 + 25+ 集成测试. 0 真接 tokio/async, 留 R21 续真接. 0 触碰 24 LOCKED crate +
+    # 0 改 workspace version (1.0.0) + 6 哲学 anchor + 8 项不修改承诺.
+    "crates/apeireth-state",
 ]
```

**0 改 `[workspace.package] version = "1.0.0"`** ✅
**0 改 `[workspace.lints]`** ✅
**0 改 `[workspace.dependencies]`** ✅
**+1 member 路径**: `crates/apeireth-state` (新增独立 crate)

> 注: git diff 看到的 `pyo3 = "0.22" → "0.29"` 是 R20 阶段 6 sister report 改动, **非本任务**.

---

## 3. 0 LOCKED 触碰验证 (含 apeireth-tui 0 改)

### 3.1 apeireth-tui LOCKED boundary mtime 守门

**baseline mtime 16:34:11 (sister report §2 LOCKED baseline)**:

| 文件 | 当前 mtime | 状态 |
|---|---|---|
| `src/app.rs` | 16:34:11 | ✅ baseline 守门 |
| `src/theme.rs` | 16:34:11 | ✅ baseline 守门 |
| `src/http_llm.rs` | 16:34:11 | ✅ baseline 守门 |
| `src/main.rs` | 0:26:40 | ✅ sister report (R20 阶段 6 改) |
| `src/organ/mod.rs` | 1:27:49 | ✅ sister report (借鉴 #1 加 1 行 `pub mod command;`) |
| `Cargo.toml` | 16:02:45 | ✅ baseline 守门 |
| `src/lib.rs` | **不存在** | ✅ (apeireth-tui 是 binary-only crate, 任务 spec 提到的 "lib.rs" 不存在, 0 触碰仍成立) |

### 3.2 24 LOCKED crate 0 触碰

- 24 LOCKED crate 的 `src/` 0 改 (mtime 验证)
- 新文件全部在 `crates/apeireth-state/` 独立目录
- workspace Cargo.toml 仅 +1 行 member 路径, 0 改其他
- 0 引 24 LOCKED crate 进 apeireth-state 的 Cargo.toml (dev / runtime 0 依赖)

### 3.3 workspace version 0 改验证

`[workspace.package] version = "1.0.0"` (line 180) 0 改 ✅
(per git diff `--unified=0 Cargo.toml` 仅 +1 行 `"crates/apeireth-state",`)

---

## 4. 6 哲学锚穿透 + 8 项承诺守门表

| 锚 | 守门 | 文件位置 |
|---|---|---|
| **S-1** 北极星导向 | 9 器官 state 服务 ASI 北极星 (heart 60Hz / brain LLM / mind 6 哲学锚 1:1 镜像) | `lib.rs::BORROWED_GOLUTRA_STATE_COUNT` + `registry.rs` 9 字段 |
| **S-2** 实事求是 | 3 模式全部 stub impl (OnceLock 真接 set, Mutex/RwLock 真接 lock), OrganStub 是 0 业务占位 (标 "_marker: 0") | `mode_once_lock.rs::init` 走 `OnceLock::set` 真接, `organ.rs::OrganStub._marker` 标占位 |
| **O-2** 走在前人肩上 | 借 Golutra 9 Tauri state 模式 + `std::sync::{Mutex, RwLock, OnceLock, Arc}` 业界标准 + `serde` 序列化 | `lib.rs::` 借鉴 Golutra 字段 + 行为模式, `Cargo.toml` 0 引 parking_lot |
| **O-3** 干到底 | 9 器官 × 3 模式 = 27 hardcode + 5 hardcode 常量 + 9 Stub + 30 集成测试 + 218 行例子 | `lib.rs` 5 const 守门 + `organ.rs` ORGAN_COUNT=9 + `tests/` 30 集成测试 |
| **O-4** 任何人都能接手 | 7 src 模块都有 module-level doc, 1 example 完整 7 段, 30 集成测试覆盖 | 全部 11 文件顶部 §0-§10 完整 |
| **O-5** 不假装 | OrganStub._marker: u8 (0 业务字段) 标占位, builder.with_mode skeleton 阶段 0 行为, mode_once_lock.init 真接 OnceLock::set | `organ.rs::define_organ_stub!` macro 显式 `pub _marker: u8` |
| 8 项 1 不假装已实现 | OrganStub 占位标 `_marker: 0`, Builder.with_mode skeleton 占位, Mutex/RwLock 都真接 stdlib (0 tokio/async) | inline test 验 |
| 8 项 2 编译期 hardcode | 5 const 守门 (PLATFORM_NAME / APEIRETH_STATE_SCHEMA_VERSION / BORROWED_GOLUTRA_STATE_COUNT=9 / STATE_MODE_COUNT=3 / STATE_ERROR_COUNT=5) + 9 Organ 变体 + 3 Mode 变体 + 5 StateError 变体 + 9 OrganStub 类型 | `lib.rs` const assert + 多个 inline test 验 |
| 8 项 3 不改 LOCKED | 0 触碰 (24 LOCKED crate + workspace version 0 改) | mtime + git diff 验证 |
| 8 项 4 不改 workspace version | Cargo.toml 仅 +1 member 路径, version 0 改 | git diff --unified=0 验证 |
| 8 项 5 6 哲学锚穿透 | 见上 S-1 / S-2 / O-2 / O-3 / O-4 / O-5 | 表格 + 文件注释 |
| 8 项 6 不依赖 NewAPI | 纯 std + serde + thiserror, 0 引 tokio / reqwest / hyper / 任何 HTTP client | Cargo.toml 验证 (0 HTTP deps) |
| 8 项 7 不重复造轮子 | 借 stdlib `std::sync::{Mutex, RwLock, OnceLock, Arc}` 业界标准, 借 workspace.lints, 借 thiserror 派生 | Cargo.toml [lints] workspace = true + src/error.rs thiserror |
| 8 项 8 诚实标缺 | OrganStub._marker 占位, builder.with_mode skeleton 0 行为, mode_once_lock.init 失败返 StateError::Other (Not 真 "已集成, 0 假装") | inline test 验 |

---

## 5. 0 commit 声明

**`git status` 验证 (本任务期间)**:
```
?? crates/apeireth-state/                                       (新 crate, 11 文件全 untracked)
 M Cargo.toml                                                   (仅 +1 行 member 路径)
```

**`git log --oneline -5`** (per 当前 HEAD, 0 主动 commit):
```
0da4af03 feat(provider): R20 阶段 4 估补 — claude-code Provider client skeleton (强效果)
915f28ef test(bench): R20 阶段 6 — cargo bench 性能 baseline (1.0 release #7 perf)
7685b128 chore(V1300): apeireth-image-prompt [lints] workspace = true (修 V1298 audit 1/16 缺)
17dcf9ef memory: cron tick 22:01 V1299 self-stance log (rust-toolchain 6/6 hyp PASS + 52 tests + 2 parser bug fix + d08e0c0f)
d08e0c0f feat(V1299) + tests(52): Rust Toolchain Audit (VCP 真源代码深读 #20) 真生产
```

**0 主动 commit**: 本任务期间未运行 `git commit` / `git push`. 新文件 `??` untracked, 留 Mavis 整合 #3 拍板.

---

## 6. 路径合规

| 项目 | 路径 | 状态 |
|---|---|---|
| 唯一目标主仓 | `.openclaw\workspace\promethean\Apeireth-rust\` | ✅ |
| 严禁 sandbox 错路径 | `.minimax-agent-cn\projects\apeireth-debug\Apeireth-rust\` | ❌ 未触碰 |
| 新 crate 位置 | `crates\apeireth-state\` | ✅ 独立新 crate, 跟借鉴 #5 pipeline-g5 / 借鉴 #4 oauth 同模式 |
| 集成测试位置 | `crates\apeireth-state\tests\test_state_sharing.rs` | ✅ 独立 tests/ 目录 |
| 例子位置 | `crates\apeireth-state\examples\state_sharing_demo.rs` | ✅ 独立 examples/ 目录 |
| 借鉴文档 | `analysis\golutra\BORROW_FROM_GOLUTRA.md` | ✅ 已读 §8 P1 第 9/10 项 |

---

## 7. 编译 + 测试结果

**`cargo check -p apeireth-state`**: ✅ Finished, 0 error
**`cargo test -p apeireth-state`**: ✅
```
running 69 tests   (lib unit tests)
test result: ok. 69 passed; 0 failed; 0 ignored

running 30 tests   (integration tests)
test result: ok. 30 passed; 0 failed; 0 ignored

running 0 tests    (example - cargo test 不跑 example, 见下)
test result: ok. 0 passed; 0 failed
```

**`cargo run -p apeireth-state --example state_sharing_demo`**: ✅ 7 段输出, 9 器官并发访问, 0 panic
```
--- Demo 1: OnceLockState<Config> (模式 1: 进程全局 lazy init) ---
  Before init: is_initialized=false
  After init: is_initialized=true
  Config: platform=apeireth, version=0.1.0

--- Demo 2: MutexState<Counter> (模式 2: 跨线程互斥) ---
  5 threads +1, final count=5, last_provider=provider_4

--- Demo 3: RwLockState<History> (模式 3: 跨线程读写锁) ---
  3 writers × 2 entries = 6, 5 readers total_read=6, actual entries=6

--- Demo 4: OrganStateRegistry 9 器官聚合 ---
  [0] [♥] (心) = Mutex
  [1] [BRAIN] (脑) = Mutex
  [2] [HAND] (手) = Mutex
  [3] [EYE] (眼) = RwLock
  [4] [EAR] (耳) = RwLock
  [5] [MEM] (记忆) = RwLock
  [6] [VOICE] (声) = Mutex
  [7] [BODY] (体) = Mutex
  [8] [MIND] (意) = Mutex

--- Demo 5: 9 器官并发访问 (3 线程) ---
  Thread: heart 5 ticks OK
  Thread: memory 3 appends OK
  Thread: brain + mind reads OK

--- Demo 6: SharedState<T> trait dispatch (3 模式 enum match) ---
  mode=OnceLock -> value=123
  mode=Mutex -> value=456
  mode=RwLock -> value=789

--- Demo 7: 9 OrganStub 编译期 hardcode ---
  HeartStub/BrainStub/HandStub/EyeStub/EarStub/MemoryStub/VoiceStub/BodyStub/MindStub (9 stub, _marker=0)
```

**总计 99 测试通过** (69 lib + 30 integration), 0 失败.

---

## 8. 关键诚实标缺 (per 8 项之 8)

| 项 | Readiness | 标缺内容 | 真实化时间 |
|---|---|---|---|
| **OnceLock 真接** | Partial | `init()` 走 `OnceLock::set` 真接, `get()` 走 `OnceLock::get` 真接; 0 异步, 0 并发守门 (skeleton 阶段) | R21+ 加 parking_lot 或 std::sync::Mutex 守并发 init |
| **MutexState 真接** | Ok | 真接 `std::sync::Mutex`, `try_lock` / `try_write` 全部走 stdlib; Poisoned 错误传播真接 | — (无续做项) |
| **RwLockState 真接** | Ok | 真接 `std::sync::RwLock`, `try_read` / `try_write` 全部走 stdlib; Poisoned 错误传播真接 | — (无续做项) |
| **SharedState trait dispatch** | Ok | 3 模式 enum match 编译期守门, 0 dyn overhead | — (无续做项) |
| **OrganStub 9 类型** | Stub | `_marker: u8` 占位, 0 业务数据; 真实集成时换为 sister 报告 9 organ State 类型 | R21+ 替换为 `apeireth_tui::organ::command::heart::State` 等 |
| **OrganStateRegistryBuilder.with_mode** | Stub | skeleton 阶段 0 业务, 仅保留 builder API 形状 | R21+ 续做 per-organ 模式覆盖 |
| **async / tokio 集成** | N/A | 0 引 tokio, 0 引 async-trait (per 借用 #5 pipeline-g5 / 借用 #4 oauth 同模式) | R21+ 真接 tokio::sync::Mutex / async-trait 续做 |
| **9 organ State 集成** | 未来 | OrganStateRegistry 9 字段类型是 `MutexState<OrganStub>` 或 `RwLockState<OrganStub>`, 真实集成时换为 sister 报告 9 organ State | R21+ 真实集成 (LOCKED 边界外, 在 `apeireth-tui/app.rs` 加 1 行 `let shared = OrganStateRegistry::new();`) |

**LOCKED 边界** (per R20 1.0 release): 
- 一旦 LOCKED 9 organ State 类型可用 (sister 报告 9 organ 已在), 真实集成由 R21+ 续做
- 真实集成点: `apeireth-tui/src/app.rs` LOCKED 边界, 加 1 行 `let shared = OrganStateRegistry::new();` 是允许的 (sister 报告 §2 已示范 1 行 mod 声明边界)

---

## 9. 借鉴 Golutra 9 Tauri state 模式 (P1 第 9/10 项) — 总结

| Golutra (Tauri 2) | 本 crate (TUI / ratatui) | 1:1 |
|---|---|---|
| `static STATE: OnceLock<Arc<T>> = OnceLock::new();` | `OnceLockState<T>` 模式 (lazy init 进程全局) | ✅ |
| `state: tauri::State<Mutex<T>>` 注入 | `MutexState<T>` 模式 (`Arc<Mutex<T>>` 跨线程互斥) | ✅ |
| `state: tauri::State<RwLock<T>>` 注入 | `RwLockState<T>` 模式 (`Arc<RwLock<T>>` 跨线程读写锁) | ✅ |
| 9 个 Tauri state 在 main.rs 启动时 `state.manage(...)` 装配 | `OrganStateRegistry::new()` 9 字段一次性装配 | ✅ |
| `tauri::Error` 错误模式 | `StateError` 5 变体 (Poisoned / NotInitialized / TypeMismatch / Unsupported / Other) + `StateErrorKind` 序列化摘要 | ✅ |
| 9 state 在 ui_gateway 各 module 内 `pub(crate) fn export_commands` | `SharedState<T>` trait + 3 模式 enum match 跨模式统一读 | ✅ |
| 70 command 按域拆分 | 不借鉴 (Apeireth 走借鉴 #1 sister 报告 9 organ × 6 command 模式) | 借鉴 #1 走 |
| sidecar / 命名管道 IPC | 不借鉴 (Apeireth 走 in-process / HTTP) | 不实现 |

**借鉴核心**: 编译期 enum 守门 + 3 模式 + 9 器官聚合 + Result 强类型 — Golutra 的 9-state 共享模式完美适配 TUI state 共享框架.

**整合路径** (per 借鉴 #0.3 中央 AI 主体性):
- `apeireth-tui` 的 `organ::command::heart::State` 等 9 类型 **保留为内部细节** (LOCKED 边界)
- 本 crate `OrganStateRegistry` 9 字段是 **新**的共享框架入口
- 真实集成由 R21+ 续做 (在 LOCKED 边界外做, 加 1 行 `OrganStateRegistry::new()`)

---

## 10. 已知后续 (R21+ 续做)

1. **真接 9 organ State 类型** — 当前 OrganStateRegistry 用 OrganStub 占位, 真实集成时换为 sister 报告 9 organ State (`apeireth_tui::organ::command::{heart,brain,...}::State`)
2. **真接 tokio async** — 当前是 sync 框架 (std::sync::Mutex), 0 引 tokio; R21+ 续做 async-trait / tokio::sync::Mutex
3. **OrganStateRegistryBuilder.with_mode 真接** — 当前 skeleton 阶段 0 行为, R21+ 续做 per-organ 模式覆盖
4. **OnceLock 并发 init 守门** — 当前 skeleton 阶段直接 `OnceLock::set` 0 并发守门, R21+ 加 parking_lot::Mutex 或 std::sync::Mutex 守门

---

## 11. 验证清单 (per 任务 spec)

- [x] **新 crate 文件清单 + 行数** — §1 (11 文件, 2709 行)
- [x] **workspace Cargo.toml 改动** — §2 (+1 member 路径, version 0 改)
- [x] **0 LOCKED 触碰验证 (含 apeireth-tui 0 改)** — §3 (mtime + git diff 验证)
- [x] **6 哲学锚 + 8 项承诺守门表** — §4
- [x] **0 commit 声明** — §5
- [x] **路径合规** — §6
- [x] **关键诚实标缺 (3 模式哪些 stub, 哪些真接, R21 续)** — §8
- [x] **不主动 commit (留 Mavis 整合 #3)** — §5
- [x] **0 改 workspace version** — §2 + §3
- [x] **0 触碰 24 LOCKED crate** — §3
- [x] **0 干 Tauri 2.0 (主 22:13 拍 "只干 TUI")** — 仅借鉴字段 + 行为模式, 不实现 Tauri

---

**报告完.** 0 commit 主动 (留 Mavis 整合 #3 拍板). 0 LOCKED 触碰. 6 哲学锚 + 8 项承诺全守门. 99 测试通过.
