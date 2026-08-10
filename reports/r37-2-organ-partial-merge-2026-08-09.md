# R37-2: 9 organ 部分合并 — 透明 facade 模式

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成 (logical merge 模式, 0 breaking)
**ROI**: ★★★★ (R34 #2 候选, 透明 re-export 让 logical -3 crate, 0 物理瘦身但业务侧 0 改)

---

## 1. 目标

R34 架构调研 #2: "9 organ 部分合并: memory+life_force / perception+consciousness / motivation+value, 留 page UI 名字".

3 组合并, 业务侧 1 entry point (e.g. `apeireth_memory::*` 拿到 life_force + memory 内容), 但实际 3 老 crate 保留 workspace members (TUI 引用了 4 个).

---

## 2. 模式选择: 透明 facade (R23 P3 借鉴)

| 模式 | 物理瘦身 | 业务侧 0 改 | 风险 |
|------|----------|-------------|------|
| 真合并 (挪 src/) | -3 crate | 需改 import path | 改 src/ 边界, 主人 R23 P3 锁的"0 触碰 9 LOCKED 文件"难保 |
| 透明 facade (re-export) | 0 (但 logical -3) | 0 改 | 0 |
| 真删 (R36 模式) | -3 crate | 0 改 (需要 0 引用) | TUI 引用 4 个, 0 删 |

**选透明 facade**: TUI 引用 `apeireth_consciousness::*` 等 4 个, 不能像 R36 5 老 provider 0 引用真删. 透明 re-export 让:
- 业务侧 `use apeireth_memory::X` 拿到 memory 全部 (含 re-export 的 life_force)
- 业务侧 `use apeireth_life_force::X` 仍能 (workspace member 保留)
- Logical "memory 内含 life_force", 物理 0 改

---

## 3. 改动

### 3.1 `crates/apeireth-memory/src/lib.rs`

加 +1 行 re-export (R23 P3 transparent 模式, 跟 extensions 模式 1:1):
```rust
pub use apeireth_life_force::*;  // R37-2: 9 organ 部分合并
```

### 3.2 `crates/apeireth-memory/Cargo.toml`

加 +1 行 dep (workspace path 不变, 0 新成员):
```toml
apeireth-life-force = { path = "../apeireth-life-force" }
```

### 3.3 `crates/apeireth-perception/src/lib.rs` + Cargo.toml

```rust
// lib.rs
pub use apeireth_consciousness::*;  // R37-2

// Cargo.toml
apeireth-consciousness = { path = "../apeireth-consciousness" }
```

### 3.4 `crates/apeireth-motivation/src/lib.rs` + Cargo.toml

```rust
// lib.rs (在 use 段后)
pub use apeireth_value::*;  // R37-2

// Cargo.toml
apeireth-value = { path = "../apeireth-value" }
```

### 3.5 `Cargo.toml` workspace members

3 老 crate 仍在 members (TUI 引用 4 个, 0 删), 但加注释:
```toml
"crates/apeireth-life-force",  # R37-2: transparent re-export 到 memory
"crates/apeireth-value",       # R37-2: transparent re-export 到 motivation
"crates/apeireth-consciousness", # R37-2: transparent re-export 到 perception
```

---

## 4. 测试

### 4.1 3 老 crate 单独 test (0 退化)

```
cargo test -p apeireth-life-force --lib    → 39 passed
cargo test -p apeireth-consciousness --lib → 19 passed
cargo test -p apeireth-value --lib         → 46 passed
```

### 4.2 3 target crate 单独 test (0 退化)

```
cargo test -p apeireth-memory --lib    → 64 passed (含 R33-2 promote_with_summarize 6)
cargo test -p apeireth-perception --lib → 29 passed
cargo test -p apeireth-motivation --lib → 10 passed
```

### 4.3 全 workspace build + test

```
cargo build --workspace
# Finished `dev` profile in 14.30s
# 0 error

cargo test --workspace --lib
# 40+ crate test result ok, 0 fail, 0 退化
```

---

## 5. 不漂移 (主哲学锚 #1)

- 0 改 9 LOCKED 文件 (memory: append_only / identity / migrations / episode / session_note / streams / history_streams / continuity_link / llm_analysis 0 触碰)
- 0 改 24 LOCKED crate (workspace 1.0.0 0 触碰)
- 0 改 TUI 9 organ page UI (R26 LOCKED 0 触碰, organ/ module 内部 0 改)
- 0 改 TUI 调用方 (TUI 用 `apeireth_consciousness::*` 等, transparent 0 触碰)
- 0 改 R23 extensions re-export 模式
- 物理 members 0 变, logical -3 (memory = memory + life_force, perception = perception + consciousness, motivation = motivation + value)

---

## 6. 后续路线

- ✅ R37-2 完成 (logical merge)
- ⏭ R37-2-1 (0.5d): 真合并 (挪 src/ 到 target crate 子目录), 物理 -3. 可选, R23 P3 锁的"0 触碰"风险.
- ⏭ R32-3 (eval smoke test, 已做)
- ⏭ R33-3 (MCP resources, 2d)
- ⏭ R33-4 (AutoGen council, 2d)
- ⏭ R33-5 (LangGraph conditional 实战 — 跟 R32-2 后续)

---

**Total LOC**: 3 改 src/lib.rs (+1 行 re-export / 3) + 3 改 Cargo.toml (+1 行 dep / 3) + 1 改 workspace Cargo.toml (注释 3 行).
**build/test**: 全 workspace pass, 0 退化, 0 breaking.
