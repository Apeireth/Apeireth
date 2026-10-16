//! G-P1 指标端点: `GET /v1/panel/metrics` —— 遥测面板指标格的 HTTP 数据源。
//!
//! 网关只做 HTTP 形状与传输;数据来自 canonical runtime 的指标注册表
//! (`apeireth_runtime::canonical::metrics`, 回合收束埋点写入的进程内滚动
//! 窗口),不自建第二份存储。本端点在 W2-② 全 API 面令牌门之内 (整面中间件,
//! 新路由默认过门)。
//!
//! # 空态与 "—" 语义 (与遥测条同款)
//!
//! 无数据时端点**显式报空**而不是缺字段或 `501`:
//! `sessions: []`、计数为 0、分布 `count: 0` 且 `mean/p50/p95/sum` 为
//! `null`、缓存/计价列为 `null`。消费端把 `null` 渲染成 "—",与聊天头遥测
//! 条对 `cache_hit_rate: None` 的处理同一语义 —— **未知不用 0 冒充**。
//!
//! 合同全文见 `docs/04-internal/backend-enhancement/G-observability.md`。

use apeireth_runtime::canonical::{
    DistributionSummary, MetricsSnapshot, SessionAggregate, WindowAggregate,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::panels::GatewayState;

/// 端点合同版本 (形状变更必须递增并同步契约文档)。
pub const METRICS_SCHEMA_VERSION: u32 = 1;

/// 留存策略的如实陈述: 进程内滚动环形, 不落盘。
pub const METRICS_RETENTION: &str = "in_process_rolling";

/// `GET /v1/panel/metrics` 的响应信封 (稳定合同, 字段改名须同步契约文档)。
#[derive(Debug, Clone, Serialize)]
pub struct MetricsEnvelopeDto {
    /// 合同版本。
    pub schema_version: u32,
    /// 生效滚动窗口 (拍/层)。
    pub window_turns: usize,
    /// 留存策略标识 (恒为 `in_process_rolling`: 进程内环形, 不落盘)。
    pub retention: &'static str,
    /// 全局层聚合。
    pub global: AggregateDto,
    /// 会话层聚合 (窗口内有拍的会话;无则显式空数组)。
    pub sessions: Vec<SessionAggregateDto>,
}

/// 一层 (会话或全局) 的窗口聚合 DTO。
#[derive(Debug, Clone, Serialize)]
pub struct AggregateDto {
    /// 窗口内收束拍计数。
    pub turns: u64,
    /// 成功收束计数。
    pub succeeded: u64,
    /// 失败收束计数。
    pub failed: u64,
    /// 待批收束计数。
    pub pending_approval: u64,
    /// 回合延迟分布 (毫秒)。
    pub latency_ms: DistributionDto,
    /// 输入 token 分布 (仅带 usage 计量的拍)。
    pub input_tokens: DistributionDto,
    /// 输出 token 分布 (口径同上)。
    pub output_tokens: DistributionDto,
    /// 轮数分布。
    pub rounds: DistributionDto,
    /// 工具派发数分布。
    pub tool_calls: DistributionDto,
    /// 缓存命中累计;`null` = 计量源无缓存列 (未知 → "—")。
    pub cache_hit_tokens: Option<u64>,
    /// 缓存命中率 (0.0–1.0);`null` = 命中未知或输入为 0。
    pub cache_hit_rate: Option<f64>,
    /// 成本累计 (微单位, 可选列);`null` = 未计价。
    pub cost_micros: Option<u64>,
}

/// 均值/P50/P95/计数分布 DTO。空窗口 (`count: 0`) 时四个数值列显式
/// `null` ("—" 语义),不用 0 冒充"有数据的 0"。
#[derive(Debug, Clone, Serialize)]
pub struct DistributionDto {
    /// 参与统计的样本数。
    pub count: u64,
    /// 算术均值;空窗口 `null`。
    pub mean: Option<f64>,
    /// 最近秩 P50;空窗口 `null`。
    pub p50: Option<u64>,
    /// 最近秩 P95;空窗口 `null`。
    pub p95: Option<u64>,
    /// 窗口累计和;空窗口 `null`。
    pub sum: Option<u64>,
}

/// 会话层聚合条目。
#[derive(Debug, Clone, Serialize)]
pub struct SessionAggregateDto {
    /// 会话 id。
    pub session_id: String,
    /// 该会话的窗口聚合。
    pub aggregate: AggregateDto,
}

impl From<&DistributionSummary> for DistributionDto {
    fn from(value: &DistributionSummary) -> Self {
        let has_samples = value.count > 0;
        Self {
            count: value.count,
            mean: has_samples.then_some(value.mean),
            p50: has_samples.then_some(value.p50),
            p95: has_samples.then_some(value.p95),
            sum: has_samples.then_some(value.sum),
        }
    }
}

impl From<&WindowAggregate> for AggregateDto {
    fn from(value: &WindowAggregate) -> Self {
        Self {
            turns: value.turns,
            succeeded: value.succeeded,
            failed: value.failed,
            pending_approval: value.pending_approval,
            latency_ms: (&value.latency_ms).into(),
            input_tokens: (&value.input_tokens).into(),
            output_tokens: (&value.output_tokens).into(),
            rounds: (&value.rounds).into(),
            tool_calls: (&value.tool_calls).into(),
            cache_hit_tokens: value.cache_hit_tokens,
            cache_hit_rate: value.cache_hit_rate,
            cost_micros: value.cost_micros,
        }
    }
}

impl From<&SessionAggregate> for SessionAggregateDto {
    fn from(value: &SessionAggregate) -> Self {
        Self {
            session_id: value.session.to_string(),
            aggregate: (&value.aggregate).into(),
        }
    }
}

impl MetricsEnvelopeDto {
    /// 端点信封 (一次注册表快照的完整投影)。
    pub fn from_snapshot(snapshot: &MetricsSnapshot) -> Self {
        Self {
            schema_version: METRICS_SCHEMA_VERSION,
            window_turns: snapshot.window_turns,
            retention: METRICS_RETENTION,
            global: (&snapshot.global).into(),
            sessions: snapshot
                .sessions
                .iter()
                .map(SessionAggregateDto::from)
                .collect(),
        }
    }
}

/// 指标端点路由 (挂在令牌门内;见 `canonical_entry` 的整面中间件)。
pub fn metrics_routes() -> Router<GatewayState> {
    Router::new().route("/v1/panel/metrics", get(metrics_snapshot))
}

/// `GET /v1/panel/metrics` —— 会话 + 全局两层聚合的原子快照。
async fn metrics_snapshot(State(state): State<GatewayState>) -> Response {
    let snapshot = state.runtime.metrics().snapshot();
    (
        StatusCode::OK,
        Json(MetricsEnvelopeDto::from_snapshot(&snapshot)),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use apeireth_core::kernel::SessionId;
    use apeireth_runtime::canonical::{Runtime, TurnMetrics, TurnOutcomeKind};
    use tower::ServiceExt;

    use super::*;

    /// 测试装配: 最小运行时 + 空服务面 (端点照样出显式空)。
    async fn test_state() -> GatewayState {
        let runtime =
            std::sync::Arc::new(Runtime::builder().build().await.expect("minimal runtime"));
        crate::canonical_entry::build_gateway_state_with_services(
            runtime,
            crate::panels::GatewayServices::default(),
        )
    }

    async fn body_json(router: Router, uri: &str) -> serde_json::Value {
        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).expect("json")
    }

    /// 空态合同: 显式空 + "—" 语义 (null), 不缺字段、不报 501。
    #[tokio::test]
    async fn empty_state_is_explicit_and_dash_honest() {
        let body = body_json(
            metrics_routes().with_state(test_state().await),
            "/v1/panel/metrics",
        )
        .await;
        assert_eq!(body["schema_version"], 1);
        assert_eq!(body["retention"], "in_process_rolling");
        assert_eq!(body["window_turns"], 50, "N 配置默认 50");
        assert_eq!(body["global"]["turns"], 0);
        assert_eq!(body["global"]["succeeded"], 0);
        assert_eq!(body["global"]["latency_ms"]["count"], 0);
        assert_eq!(
            body["global"]["latency_ms"]["mean"],
            serde_json::Value::Null
        );
        assert_eq!(body["global"]["latency_ms"]["p50"], serde_json::Value::Null);
        assert_eq!(
            body["global"]["input_tokens"]["sum"],
            serde_json::Value::Null
        );
        assert_eq!(body["global"]["cache_hit_tokens"], serde_json::Value::Null);
        assert_eq!(body["global"]["cache_hit_rate"], serde_json::Value::Null);
        assert_eq!(body["global"]["cost_micros"], serde_json::Value::Null);
        assert_eq!(body["sessions"], serde_json::json!([]));
    }

    /// 有数据: 会话 + 全局两层聚合都出现在合同形状里。
    #[tokio::test]
    async fn recorded_beats_surface_in_both_layers() {
        let state = test_state().await;
        let session = SessionId::new();
        state.runtime.metrics().record(
            session,
            TurnMetrics {
                outcome: TurnOutcomeKind::Succeeded,
                latency_ms: 12,
                input_tokens: Some(100),
                output_tokens: Some(20),
                cache_hit_tokens: None,
                cache_hit_rate: None,
                tool_calls: Some(1),
                rounds: Some(2),
                cost_micros: None,
            },
        );
        let body = body_json(metrics_routes().with_state(state), "/v1/panel/metrics").await;
        assert_eq!(body["global"]["turns"], 1);
        assert_eq!(body["global"]["succeeded"], 1);
        assert_eq!(body["global"]["latency_ms"]["p50"], 12);
        assert_eq!(body["global"]["input_tokens"]["sum"], 100);
        let sessions = body["sessions"].as_array().expect("sessions array");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], session.to_string());
        assert_eq!(sessions[0]["aggregate"]["turns"], 1);
    }
}
