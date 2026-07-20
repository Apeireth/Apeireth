**Source**: https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/crates/memoryos-gateway/src/main.rs

---

use axum::{
    routing::{get, post},
    Router,
};
use memoryos_core::{AppError, ConfigManager};
use std::{path::PathBuf, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod handlers;
mod middleware;
mod routes;
mod state;
mod worker_monitor;

use handlers::chat_completions;
use routes::defense::create_defense_routes;
use routes::faq::{create_faq_routes, FaqState};
use routes::graph::{create_graph_routes, GraphState};
use routes::memory_manage::{create_memory_manage_routes, MemoryManageState};
use routes::multimodal::{create_multimodal_routes, MultiModalState};
use routes::security::{create_security_routes, SecurityState};
use routes::wiki::{create_wiki_routes, WikiState};
use state::AppState;
use worker_monitor::spawn_worker_monitor;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 1. Init Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load Config
    let config_manager = ConfigManager::new()?;
    let config = config_manager.get();

    // Validate configuration
    config.validate()?;
    tracing::info!("Configuration validated successfully");

    let async_memory_enabled = std::env::var("MEMORYOS_ASYNC_MEMORY_PIPELINE")
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);

    // 3. Init App State (Connect to DBs)
    let state = AppState::new((*config).clone()).await?;

    if async_memory_enabled {
        tracing::info!(
            "Gateway async memory pipeline is enabled; deploy worker as optional consumer for queued memory tasks."
        );
        spawn_worker_monitor(
            config.storage.redis.url.clone(),
            state.worker_monitor.clone(),
        );
    } else {
        tracing::info!(
            "Gateway running in standalone sync-memory mode; worker deployment is optional and not required."
        );
    }

    // 4. Spawn config hot-reload task
    // Config hot-reload is architecturally limited: AppState holds an Arc<AppConfig>
    // snapshot taken at startup. The ConfigManager's ArcSwap is not referenced by
    // any runtime component. Most config changes require a restart.
    // See docs/CONFIG_HOT_RELOAD_LIMITATION.md for details.
    tracing::info!(
        "Config changes require a restart (hot-reload not effective with current architecture)"
    );

    // 5. Setup Router
    let state_arc = Arc::new(state.clone());

    // FAQ 管理路由
    let heat_tracker = Arc::new(memoryos_core::HeatTracker::new(
        memoryos_core::HeatConfig::default(),
    ));
    let faq_state = FaqState {
        heat_tracker: heat_tracker.clone(),
        auto_promoter: Arc::new(memoryos_core::AutoPromoter::new(
            memoryos_core::AutoPromotionConfig::default(),
            heat_tracker,
        )),
        vector_store: state.vector_store.clone(),
        tenant_manager: state.tenant_manager.clone(),
        faq_matcher: state.faq_matcher.clone(),
    };

    // Admin 路由（需要认证 + admin 权限）
    let admin_routes = Router::new()
        .route("/v1/admin/keys", post(routes::admin::create_api_key))
        .route(
            "/v1/admin/keys/:key",
            axum::routing::delete(routes::admin::delete_api_key),
        )
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::admin_only,
        ))
        .with_state(state_arc.clone());

    // FAQ 管理路由（独立 state，嵌套到 admin 路径下，需要 admin 权限）
    let faq_routes = create_faq_routes(faq_state).layer(axum::middleware::from_fn_with_state(
        state_arc.clone(),
        middleware::admin_only,
    ));

    // Graph 路由
    let graph_state = GraphState {
        graph_manager: std::sync::Arc::new(tokio::sync::RwLock::new(
            memoryos_core::GraphManager::new(),
        )),
    };
    let graph_routes = create_graph_routes(graph_state);

    // Memory management 路由
    let memory_manage_state = MemoryManageState {
        vector_store: state.vector_store.clone(),
        tenant_manager: state.tenant_manager.clone(),
    };
    let memory_manage_routes = create_memory_manage_routes(memory_manage_state);

    // Multimodal 路由
    let multimodal_storage = std::sync::Arc::new(
        memoryos_adapters::multimodal::QdrantMultiModalStorage::new(&config.storage.vector.url)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to init QdrantMultiModalStorage: {}", e))
            })?,
    );
    let multimodal_state = MultiModalState {
        storage: multimodal_storage,
    };
    let multimodal_routes = create_multimodal_routes(multimodal_state);

    // Security 路由
    let data_dir = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".memoryos");
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        tracing::warn!("Failed to create data dir {:?}: {}", data_dir, e);
    }

    let audit_path = data_dir.join("audit.jsonl");
    let gdpr_path = data_dir.join("gdpr.json");

    let audit_logger = std::sync::Arc::new(memoryos_core::AuditLogger::new(
        memoryos_core::AuditConfig {
            persist_path: Some(audit_path.to_string_lossy().to_string()),
            ..memoryos_core::AuditConfig::default()
        },
    ));

    let gdpr_path_str = gdpr_path.to_string_lossy().to_string();
    let gdpr_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
        memoryos_core::GdprManager::with_persistence(&gdpr_path_str),
    ));
    let security_state = SecurityState {
        audit_logger,
        gdpr_manager,
        qdrant_storage: state.qdrant_storage.clone(),
        redis_storage: state.redis_storage.clone(),
    };
    let security_routes = create_security_routes(security_state);

    // Wiki generation routes
    let default_llm_adapter = state.providers.get(&config.llm.default_provider).cloned();
    let wiki_state = WikiState {
        llm_adapter: default_llm_adapter,
        jobs: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        connector_sessions: std::sync::Arc::new(tokio::sync::RwLock::new(
            std::collections::HashMap::new(),
        )),
    };
    let wiki_routes = create_wiki_routes(wiki_state);

    // Defense routes (v1.1 — IP ban/whitelist management, admin-only)
    let defense_routes = state.defense.as_ref().map(|d| {
        create_defense_routes(d.clone()).layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::admin_only,
        ))
    });

    // All routes that require authentication (including nested sub-routes)
    let authed_routes = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/memory/add", post(routes::memory::add_message))
        .route(
            "/v1/memory/retrieve",
            post(routes::memory::retrieve_context),
        )
        .route(
            "/v1/memory/:memory_id/history",
            get(routes::history::get_memory_history),
        )
        .with_state(state.clone())
        .nest("/v1/graph", graph_routes)
        .nest("/v1/memory/manage", memory_manage_routes)
        .nest("/v1/multimodal", multimodal_routes)
        .nest("/v1/security", security_routes)
        .nest("/v1/wiki", wiki_routes)
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::auth_middleware,
        ));

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/health/live", get(routes::health::liveness))
        .route("/health/ready", get(routes::health::readiness))
        .route("/health/status", get(routes::health::status))
        .route("/metrics", get(routes::metrics::metrics_handler))
        .with_state(state)
        .merge(authed_routes)
        .merge(admin_routes)
        .nest("/v1/admin/faq", faq_routes);

    // Mount defense routes if IP defense system is available
    let app = if let Some(dr) = defense_routes {
        app.nest("/v1/admin/defense", dr)
    } else {
        app
    };

    // Add CORS middleware — use allowed_origins from config; fall back to Any for dev
    let cors = {
        let origins = &config.server.allowed_origins;
        let layer = tower_http::cors::CorsLayer::new().allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ]);
        if origins.is_empty() {
            tracing::warn!(
                "CORS: allow_origin(Any) — set server.allowed_origins in config for production"
            );
            layer
                .allow_origin(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
                .allow_credentials(false)
        } else {
            let parsed: Vec<axum::http::HeaderValue> =
                origins.iter().filter_map(|o| o.parse().ok()).collect();
            tracing::info!("CORS: {} allowed origins configured", parsed.len());
            layer
                .allow_origin(parsed)
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::HeaderName::from_static("x-user-id"),
                    axum::http::HeaderName::from_static("x-tenant-id"),
                    axum::http::HeaderName::from_static("x-request-id"),
                ])
                .allow_credentials(true)
        }
    };

    let app = app
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB limit
        .layer(tower_http::timeout::TimeoutLayer::new(
            std::time::Duration::from_secs(config.server.timeout_seconds),
        ))
        .layer(tower_http::request_id::SetRequestIdLayer::x_request_id(
            tower_http::request_id::MakeRequestUuid,
        ))
        .layer(tower_http::request_id::PropagateRequestIdLayer::x_request_id())
        .layer(axum::middleware::from_fn_with_state(
            state_arc.clone(),
            middleware::rbac_middleware,
        ))
        .layer(axum::middleware::from_fn(middleware::metrics_middleware))
        .layer(axum::middleware::from_fn(middleware::rate_limit_middleware));

    // Add IP defense middleware if enabled
    let app = if let Some(defense) = &state_arc.defense {
        tracing::info!("IP defense middleware enabled");
        app.layer(axum::middleware::from_fn_with_state(
            defense.clone(),
            middleware::ip_defense_middleware,
        ))
    } else {
        app
    };

    if config.auth.enabled {
        tracing::info!("API Key authentication enabled");
    } else {
        tracing::warn!("========================================================");
        tracing::warn!("  WARNING: API Key authentication is DISABLED!");
        tracing::warn!("  The service is publicly accessible without any auth.");
        tracing::warn!("  Set `auth.enabled = true` in config.toml for production.");
        tracing::warn!("========================================================");
    }

    // 6. Start Server
    let addr: std::net::SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| AppError::Config(format!("Invalid host/port: {}", e)))?;

    tracing::info!("MemoryOS Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to bind port: {}", e)))?;

    // Enable IP defense if available
    if state_arc.defense.is_some() {
        tracing::info!("IP defense system enabled with ConnectInfo");
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::Internal(format!("Server error: {}", e)))?;
    } else {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| AppError::Internal(format!("Server error: {}", e)))?;
    }

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}