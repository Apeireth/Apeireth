# examples（顶层占位）

```
[Document-Meta]
Document: examples/README.md
Version: Manual-Rev-G + Fix-13
R-Cycle: R14
Last-Modified: 2026-07-31
Status: 🟡 占位（真实示例在 crate-level）
```

> **状态**: 🟡 **占位** — 顶层 `examples/hello_world.rs` 是早期占位（已删除），**真实示例在 crate-level**。

## 📂 真实示例位置

| 示例 | 真实位置 |
|---|---|
| `hello_world.rs` | `crates/apeireth-core/examples/hello_world.rs`（8,913 bytes）|

## 🚀 怎么跑

```bash
cd redacted/.openclaw/workspace/promethean/Apeireth-rust/
cargo run -p apeireth-core --example hello_world
```

## 📜 历史

- **2026-07-30**：顶层 `examples/hello_world.rs` 创建（R11 baseline 保留）
- **2026-07-31 Fix-13**：顶层 `examples/hello_world.rs` **删除**（避免重复）+ 顶层 `examples/README.md` 创建（指向 crate-level 真实位置）

**详见**：
- `APEIRETH-FINAL-CHECK-2026-07-31.md`
- `crates/apeireth-core/examples/hello_world.rs`

---

_本目录为占位（owner: 施工团队，A9-A17 阶段创建更多示例）._