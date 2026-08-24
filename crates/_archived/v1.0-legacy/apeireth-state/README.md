# apeireth-state

> Apeireth R21 借鉴 Golutra #6: 9 Tauri state 模式 (OnceLock + Arc + Mutex) 转 TUI 等价物 (ratatui state 共享框架). 3 模式 (OnceLockState / MutexState / RwLockState) + 9 器官 state 共享 (heart/brain/hand/eye/ear/memory/voice/body/mind) + 1 完整状态共享例子 + 87 测试 + 30 集成. 0 真接 tokio/async, 留 R21 续真接. 0 触碰 3 不可变脊柱 (per R148, LOCKED crate 入口签名降级后仅保 Self-Disable / L0 HA / 13 键 verdict cache) + 0 改 workspace version 1.2.0 + 8 哲学 anchor (S-1/S-2/S-3 质量工程化 NEW/O-1 安全优先 NEW/O-2/O-3/O-4/O-5) + 8 项不修改承诺

apeireth-state 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (10 src 文件 / 87 测试 + 2 Kani proof + 30 集成)

- `src/lib.rs` — facade 入口 + re-export 3 模式 / Organ 枚举
- `src/error.rs` — StateError + 5 variant + 6 测试
- `src/mode_once_lock.rs` — OnceLockState 模式 (OnceLock + Arc) + 12 测试
- `src/mode_mutex.rs` — MutexState 模式 (Mutex + Arc) + 10 测试
- `src/mode_rw_lock.rs` — RwLockState 模式 (RwLock + Arc) + 11 测试
- `src/organ.rs` — 9 器官 enum (heart/brain/hand/eye/ear/memory/voice/body/mind) + 11 测试
- `src/registry.rs` — 9 器官 state 注册表 + 13 测试
- `src/shared_state.rs` — shared_state trait/impl 装配 + 6 测试
- `src/statechart.rs` — 状态机表 (per 借鉴 Golutra #6 §3) + 13 测试
- `src/organ_kani_proofs.rs` — state organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/test_state_sharing.rs` (30)
