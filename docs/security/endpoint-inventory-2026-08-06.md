# Endpoint inventory — R23 P1 末 (编译期 hardcode 30 route)

**目的**: 30 件 axum `.route()` 调用映射到 `crates/apeireth-api/src/endpoints.rs`
的 `ENDPOINTS` 编译期 const 数组, 1 屏找全部 HTTP 路由入口.

## 文件分布 (4 文件 ≠ Hermes 估的 6 文件)

Hermes 8/6 21:30 估 "28 route 散落 6 文件". 实测后:

- **30 route 4 文件** (compile-time hardcode 总数)
- 其他 2 文件 (auth.rs + ws_v1.rs) 走 WebSocket / Auth middleware, 不算 HTTP route.
- v2_endpoints.rs 18 route 是大头 (S-1 北极星 4 步真接).

| File                              | Routes |
|-----------------------------------|--------|
| src/server.rs                     |   7    |
| src/v2_endpoints.rs               |  18    |
| src/observability/mod.rs          |   3    |
| src/v1_tools/mod.rs               |   1    |
+-----------------------------------+--------+
| TOTAL                             |  30    |

## TIER 0 (S-2 龙骨锚优先审计)

4 个路径, 任何 auditor 必先扫:

1. `/memory/identity` (GET) — IdentityCard UNIQUE cross-carrier
2. `/memory/identity/update` (POST) — IdentityCard mutation
3. `/sovereignty/attack` (POST) — 入侵触发
4. `/sovereignty/rearm` (POST) — 重新武装

## 测试守门 (5 件 audit-endpoints test)

`cargo test -p apeireth-api --lib endpoints` 跑 5 个断言:

1. `audit_endpoints_count_matches_const` — `ENDPOINTS.len() == EXPECTED_ENDPOINT_COUNT` (30)
2. `audit_unique_path_method_handler` — 同一 (path, method, handler) 三元组唯一
3. `audit_known_path_alias_ok` — 不同 file 注册同 path 但 handler 名不同 (alias 模式 OK)
4. `audit_tier0_in_endpoints` — TIER_0 4 件必须都在 ENDPOINTS 数组
5. `audit_all_paths_start_with_slash` — path 以 / 开头

## 加新 route 时 (HR 提示)

1. server.rs / v2_endpoints.rs / observability/mod.rs / v1_tools/mod.rs 加 `.route(...)` 调用
2. 同 commit 中 `endpoints.rs` 数组里 +1 行 `Endpoint { ... }`
3. `EXPECTED_ENDPOINT_COUNT` 改 +1
4. `cargo test -p apeireth-api --lib endpoints` 通过即视为同步

## 不假装

- 30 route 全是 axum 真接 (cargo test 3559 → 3564 passed 测试基线+5 个 audit 测试)
- 0 件是 fake stub 或 placeholder
- alias 模式 (/health 注册 2 次) 是 S-1 北极星 4 步 C1 设计意图 (server.rs 简洁,
  v2_endpoints.rs 9 器官真接), 1 件 alias handler 不同 = OK
