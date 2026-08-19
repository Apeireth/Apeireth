# apeireth-web

> Apeireth Web 前端 — Leptos 0.7 SSR + WASM hydration, 让主人能在浏览器真用 Apeireth Council 7 advisor (R18)

apeireth-web 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (14 src 文件 / 15 测试 + 2 Kani proof + 23 集成)

- `src/lib.rs` — Leptos 0.7 lib 入口 (SSR feature)
- `src/main.rs` — bin 入口 (axum server)
- `src/app.rs` — Leptos App 根组件
- `src/api.rs` / `src/api_endpoints.rs` — 内部 API 桥接 + 2 测试
- `src/council.rs` / `src/council_history.rs` — Council 7 advisor 视图 + 历史
- `src/verdict.rs` — Verdict 视图 (13 键 verdict cache)
- `src/memory.rs` — Memory 视图 (per memory crate 桥接) + 5 测试
- `src/sovereignty.rs` — Sovereignty 视图
- `src/asi.rs` — ASI 北极星视图 (V0.5 24 维)
- `src/templates.rs` — SSR templates (Leptos view! 宏) — 集成测试 13 在 `tests/templates.rs`
- `src/tool_loop_adapter.rs` — Tool loop → Leptos signal 桥接 + 3 测试
- `src/organ_kani_proofs.rs` — web organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 集成测试: `tests/templates.rs` (13) + `tests/templates_ext.rs` (10)
