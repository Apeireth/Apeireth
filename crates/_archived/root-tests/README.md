# root-tests（归档：原 workspace 根 tests/）

> **🗄️ 归档说明（2026-08-17, 台账#37, 任务 70282ede）**:
> 本目录为原 workspace 根 `tests/` 的整体归档（R165 formal 归档同模式: 归档不删除）。
> 其中 12 个 `.rs`（6 顶层 + integration/ 6 个）不被任何活跃 crate 编译——
> 纯 virtual workspace 无 root package, 根级 `tests/` 不进任何构建目标。
> 活跃集成测试在 `crates/<name>/tests/`。归档保留历史（早期 harness/E2E 脚本参考）。
> 以下为原 README 内容（占位目录自述）。

---

# tests（计划目录）

```
[Document-Meta]
Document: tests/README.md
Version: Manual-Rev-G + R14 (占位目录)
R-Cycle: R14
Last-Modified: 2026-07-31
Status: 🟡 占位（workspace 集成测试在 crates/<name>/tests/）
```

> **状态**: 🟡 **占位** — workspace 顶层 `tests/` 暂未创建。
>
> **为什么现在不存在内容**：
> - 集成测试实际在 `crates/<name>/tests/`（如 `crates/apeireth-core/tests/integration_session_lifecycle.rs`）
> - workspace 顶层 `tests/` 是 Rust 标准约定，但**本工作区采用 crate-level 集成测试**（更模块化）
> - 真正创建顶层 `tests/` 时由施工团队按需决定（A6-A7 阶段考虑）
>
> **当前真实测试**（精确数字）：
> - `crates/apeireth-core/src/lib.rs`（17,516 bytes / **6 个 `#[test]` 内嵌单元测试**）：
>   - `test_philosophy_key_descriptions`
>   - `test_v1_v2_v3_and_gate`
>   - `test_meta_question_forbidden`
>   - `test_cognitive_dream_state_machine`
>   - `test_verdict_cache`
>   - `test_5_gates`
> - `crates/apeireth-core/tests/integration_session_lifecycle.rs`（5,978 bytes / **2 个 `#[test]` 集成测试**）
> - `crates/apeireth-core/examples/hello_world.rs`（8,913 bytes / 真示例）
> - **总计** = **6 单元 + 2 集成 = 8 测试**（外部 AI 之前审计说 16 个是数错，实际是 6 + 2 = 8）
>
> **未来顶层 tests/**（设计层）：
> - 跨 crate 集成测试（如 17 crate 联合场景）
> - workspace-level smoke tests
> - E2E 测试（CLI → Session → Episode → verdict）
>
> **详见**：
> - `docs/stage5/stage5-construction-document.md §3 §4 §5`
> - `APEIRETH-FINAL-CHECK-2026-07-31.md`

---

_本目录为占位（owner: 施工团队，A6-A7 阶段考虑创建 workspace 顶层集成测试）._