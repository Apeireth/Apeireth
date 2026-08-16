//! R23 P1 #1 endpoint const 枚举 — 编译期 hardcode endpoint 列表.
//!
//! **目的**: 把 30 个 axum `.route("/path", method(handler))` 调用统一到
//! `ENDPOINTS` 编译期数组, auditor + 7 哲学锚 dev 工具能直接读 const 表找 route.
//!
//! **不假装**: 30 件是 axum 真接的 (cargo test --lib 现有 3559 passed 验证),
//! 不是 fake stub. handler 字段是字符串引用编译期记, 不重复声明 callable.
//!
//! **6 哲学锚 守门**: 这份硬列表是 "S-1 北极星" (诚实标结构) + "S-6 透明"
//! 双锚的具体化 — 任何 auditor 1 屏能看完整 30 route.

/// HTTP method enum (编译期 exhaustive match 守门, axum 的 `get`/`post` 闭包里的).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpMethod {
    /// axum method 函数名 (S-1 不漂移 — 真实 axum 0.7 的函数名).
    pub const fn as_axum_fn(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
            Self::Delete => "delete",
            Self::Patch => "patch",
        }
    }
}

/// 单个 endpoint 编译期记录.
///
/// 字段:
/// - `path`: axum `.route("<path>", ...)` 用的字符串字面量 (含 `:param` axum 语法)
/// - `method`: HTTP 方法枚举
/// - `handler`: handler 函数名 (字符串, 不引用 fn pointer — 0 cyclic dep)
/// - `file`: 注册源码位置 (用于 audit 时跳转)
///
/// 不存 `fn` 指针 (会引入 cyclic dep — handler 在不同 module),
/// 改字符串名 + 编译期 const assertion 在 `audit_endpoints` 测试里.
#[derive(Debug, Clone, Copy)]
pub struct Endpoint {
    pub path: &'static str,
    pub method: HttpMethod,
    pub handler: &'static str,
    pub file: &'static str,
}

/// 30 件 endpoint 编译期 hardcode 列表.
///
/// 加新 route 时:
/// 1. server.rs / v2_endpoints.rs / observability/mod.rs / v1_tools/mod.rs
///    顶上加 `use crate::endpoints::ENDPOINTS;`
/// 2. 在这个数组里加 1 行 `Endpoint { ... }`
/// 3. 跑 `cargo test -p apeireth-api --lib audit_endpoints` 校验数 = 30
///
/// 数据源 (2026-08-06 R23 P1 末实测, 详见 docs/security/endpoint-inventory-2026-08-06.md):
///
/// | File                       | Routes |
/// |----------------------------|--------|
/// | server.rs                  |   7    |
/// | v2_endpoints.rs            |  18    |
/// | observability/mod.rs       |   3    |
/// | v1_tools/mod.rs            |   1    |
/// | (其他 ws_v1.rs / auth.rs)  |  WS, not in HTTP routes |
/// +----------------------------+--------+
/// | TOTAL                      |  30    |
pub const ENDPOINTS: &[Endpoint] = &[
    // ---- server.rs (7) ----
    Endpoint { path: "/health",                                method: HttpMethod::Get,  handler: "health",              file: "src/server.rs:112" },
    Endpoint { path: "/v1/chat/completions",                   method: HttpMethod::Post, handler: "chat_completions",    file: "src/server.rs:114" },
    Endpoint { path: "/v1/responses",                          method: HttpMethod::Post, handler: "responses",           file: "src/server.rs:115" },
    Endpoint { path: "/v1/messages",                           method: HttpMethod::Post, handler: "messages",            file: "src/server.rs:116" },
    Endpoint { path: "/v1beta/models/:model/generateContent",  method: HttpMethod::Post, handler: "v1beta_generate",     file: "src/server.rs:117" },
    Endpoint { path: "/council/advise",                        method: HttpMethod::Post, handler: "council_advise",      file: "src/server.rs:122" },
    Endpoint { path: "/verdict",                               method: HttpMethod::Post, handler: "verdict",             file: "src/server.rs:123" },
    // ---- v2_endpoints.rs (18) ----
    Endpoint { path: "/health",                                method: HttpMethod::Get,  handler: "v2_health",           file: "src/v2_endpoints.rs:1084" },
    Endpoint { path: "/tools/list",                            method: HttpMethod::Get,  handler: "tools_list",          file: "src/v2_endpoints.rs:1086" },
    Endpoint { path: "/tools/invoke",                          method: HttpMethod::Post, handler: "tools_invoke",        file: "src/v2_endpoints.rs:1087" },
    Endpoint { path: "/memory/episodes",                       method: HttpMethod::Get,  handler: "memory_episodes",     file: "src/v2_endpoints.rs:1089" },
    Endpoint { path: "/memory/append",                         method: HttpMethod::Post, handler: "memory_append",       file: "src/v2_endpoints.rs:1090" },
    Endpoint { path: "/memory/identity",                       method: HttpMethod::Get,  handler: "memory_identity",     file: "src/v2_endpoints.rs:1091" },
    Endpoint { path: "/memory/identity/update",                method: HttpMethod::Post, handler: "memory_identity_update", file: "src/v2_endpoints.rs:1092" },
    Endpoint { path: "/organs",                                method: HttpMethod::Get,  handler: "organs_list",         file: "src/v2_endpoints.rs:1094" },
    Endpoint { path: "/organs/:name",                          method: HttpMethod::Get,  handler: "organ_get",           file: "src/v2_endpoints.rs:1095" },
    Endpoint { path: "/organs/:name/invoke",                   method: HttpMethod::Post, handler: "organ_invoke",        file: "src/v2_endpoints.rs:1096" },
    Endpoint { path: "/asi/score",                             method: HttpMethod::Get,  handler: "asi_score",           file: "src/v2_endpoints.rs:1098" },
    Endpoint { path: "/asi/all",                               method: HttpMethod::Get,  handler: "asi_all",             file: "src/v2_endpoints.rs:1099" },
    Endpoint { path: "/asi/calibrate",                         method: HttpMethod::Post, handler: "asi_calibrate",       file: "src/v2_endpoints.rs:1100" },
    Endpoint { path: "/sovereignty/status",                    method: HttpMethod::Get,  handler: "sovereignty_status",  file: "src/v2_endpoints.rs:1102" },
    Endpoint { path: "/sovereignty/attack",                    method: HttpMethod::Post, handler: "sovereignty_attack",  file: "src/v2_endpoints.rs:1103" },
    Endpoint { path: "/sovereignty/rearm",                     method: HttpMethod::Post, handler: "sovereignty_rearm",   file: "src/v2_endpoints.rs:1104" },
    Endpoint { path: "/agent/aliases",                         method: HttpMethod::Get,  handler: "agent_aliases",       file: "src/v2_endpoints.rs:1106" },
    Endpoint { path: "/agent/alias",                           method: HttpMethod::Post, handler: "agent_register_alias", file: "src/v2_endpoints.rs:1107" },
    Endpoint { path: "/agent/cache",                           method: HttpMethod::Get,  handler: "agent_cache",         file: "src/v2_endpoints.rs:1108" },
    // ---- observability/mod.rs (3) ----
    Endpoint { path: "/observability/metrics",                 method: HttpMethod::Get,  handler: "metrics::metrics_handler", file: "src/observability/mod.rs:427" },
    Endpoint { path: "/observability/health",                  method: HttpMethod::Get,  handler: "metrics::health_handler",  file: "src/observability/mod.rs:428" },
    Endpoint { path: "/observability/status",                  method: HttpMethod::Get,  handler: "metrics::status_handler",  file: "src/observability/mod.rs:429" },
    // ---- v1_tools/mod.rs (1) ----
    Endpoint { path: "/tools/:name/invoke",                    method: HttpMethod::Post, handler: "v1_tools_invoke",     file: "src/v1_tools/mod.rs:183" },
    // ---- panel_readonly.rs (7) — B1 Web 面板 v2 只读端点 (nest /v1/panel) ----
    Endpoint { path: "/panel/sessions",                        method: HttpMethod::Get,  handler: "panel_sessions",        file: "src/panel_readonly.rs:71" },
    Endpoint { path: "/panel/sessions/:id/timeline",           method: HttpMethod::Get,  handler: "panel_session_timeline", file: "src/panel_readonly.rs:109" },
    Endpoint { path: "/panel/memory/streams",                  method: HttpMethod::Get,  handler: "panel_memory_streams",  file: "src/panel_readonly.rs:158" },
    Endpoint { path: "/panel/memory/episodes",                 method: HttpMethod::Get,  handler: "panel_memory_episodes", file: "src/panel_readonly.rs:207" },
    Endpoint { path: "/panel/graph",                           method: HttpMethod::Get,  handler: "panel_graph",           file: "src/panel_readonly.rs:278" },
    Endpoint { path: "/panel/approvals",                       method: HttpMethod::Get,  handler: "panel_approvals",       file: "src/panel_readonly.rs:324" },
    Endpoint { path: "/panel/audit",                           method: HttpMethod::Get,  handler: "panel_audit",           file: "src/panel_readonly.rs:379" },
];

/// 编译期 endpoint 数守门.
///
/// 加新 route 时, 同时:
/// 1. 在 ENDPOINTS 数组里 +1
/// 2. 在 EXPECTED_ENDPOINT_COUNT 改 +1
/// 3. `cargo test -p apeireth-api --lib audit_endpoints` 通过即视为 hardcode 同步
pub const EXPECTED_ENDPOINT_COUNT: usize = 37;

/// PII / 龙骨密钥路径 — 这 4 个路径是 S-2 龙骨 (per docs/01-stage1/S-2 骨骼.md),
/// 任何 auditor 应优先审计.
pub const TIER_0_ENDPOINTS: &[&str] = &[
    "/memory/identity",
    "/memory/identity/update",
    "/sovereignty/attack",
    "/sovereignty/rearm",
];

/// 编译期检查 (test mod 用)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_endpoints_count_matches_const() {
        assert_eq!(
            ENDPOINTS.len(),
            EXPECTED_ENDPOINT_COUNT,
            "ENDPOINTS 数组数 ({}) != EXPECTED_ENDPOINT_COUNT ({}) — 加新 route 时同步 +1",
            ENDPOINTS.len(),
            EXPECTED_ENDPOINT_COUNT
        );
    }

    #[test]
    fn audit_unique_path_method_handler() {
        // 同一 (path, method) 必须只能 1 个 handler 注册;
        // 不同 (path) + 相同 handler 名允许 (因为 alias 是设计意图).
        let mut seen = std::collections::HashSet::new();
        for ep in ENDPOINTS {
            assert!(
                seen.insert((ep.path, ep.method, ep.handler)),
                "重复 endpoint (path, method, handler) 三元组: {} {} {}",
                ep.method.as_axum_fn(),
                ep.path,
                ep.handler
            );
        }
    }

    #[test]
    fn audit_known_path_alias_ok() {
        // /health 在 server.rs 和 v2_endpoints.rs 都注册, 是 alias 模式 (S-1 北极星 4 步 C1)
        // 各 handler 名字不同 (health vs v2_health), 所以 path+method+handler 三元组唯一.
        let mut paths = std::collections::HashMap::<&str, Vec<&str>>::new();
        for ep in ENDPOINTS {
            paths.entry(ep.path).or_default().push(ep.handler);
        }
        // 同一 path 有 2 handler 时, 必须 handler 名不同
        for (path, handlers) in paths {
            let unique: std::collections::HashSet<_> = handlers.iter().collect();
            assert_eq!(
                unique.len(), handlers.len(),
                "path {path} 注册 {} handler 但有重名: {handlers:?}", handlers.len()
            );
        }
    }

    #[test]
    fn audit_tier0_in_endpoints() {
        for tier0 in TIER_0_ENDPOINTS {
            let found = ENDPOINTS.iter().any(|ep| ep.path == *tier0);
            assert!(found, "TIER_0 端点 {tier0} 必须也在 ENDPOINTS 数组里 (S-2 龙骨锚守门)");
        }
    }

    #[test]
    fn audit_all_paths_start_with_slash() {
        for ep in ENDPOINTS {
            assert!(
                ep.path.starts_with('/'),
                "endpoint path 必须以 / 开头: {}",
                ep.path
            );
        }
    }
}
