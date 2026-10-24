//! 记忆「纠正」的 HTTP 面（`PUT /v1/apeireth/memory/episodes/:id/content`）。
//!
//! 从 `panels.rs` 拆出（尺寸棘轮：既有文件只减不增）。语义真源在
//! [`apeireth_memory::MemoryGovernanceStore::update_episode_content`]：
//! 重写治理视图内容、原始 append-only episode 行不动、乐观 rev CAS、
//! 检索走 override ⇒ 后续召回不再出现旧结论。
//!
//! 本模块只做三件事：请求体解析、端口分派、错误帧映射——不复制任何存储语义。

use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::panels::PanelDataAdapter;
use super::panels::{memory_governance_unavailable, panel_error};
use super::panels::{EpisodeMutationDto, GatewayState};
use super::principal_forget::{MemoryGovernanceCommand, PrincipalForgetDto};

/// 纠正请求体：新内容 + 期望修订号 + 谁改的（缺省 `"user"` = 主人自己改）。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct EpisodeContentRequest {
    pub(crate) expected_rev: u64,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) updated_by: Option<String>,
}

/// 纠正 handler：`PUT /v1/apeireth/memory/episodes/:id/content`。
pub(crate) async fn update_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeContentRequest>,
) -> Response {
    if let Some(response) = memory_governance_unavailable(&state) {
        return response;
    }
    let memory = state
        .services
        .memory_governance
        .as_ref()
        .expect("checked above");
    match memory
        .update_episode(
            &id,
            &request.content,
            request.updated_by.as_deref().or(Some("user")),
            request.expected_rev,
        )
        .await
    {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

// `PanelDataAdapter` 的治理端口实现（从 panels.rs 平移，尺寸棘轮）。
#[async_trait]
impl MemoryGovernanceCommand for PanelDataAdapter {
    async fn protect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        self.inner.protect_episode(id, expected_rev).await
    }

    async fn unprotect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        self.inner.unprotect_episode(id, expected_rev).await
    }

    async fn forget_episode(
        &self,
        id: &str,
        expected_rev: u64,
        reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String> {
        self.inner.forget_episode(id, expected_rev, reason).await
    }

    async fn update_episode(
        &self,
        id: &str,
        content: &str,
        updated_by: Option<&str>,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        self.inner
            .update_episode(id, content, updated_by, expected_rev)
            .await
    }

    async fn forget_principal(
        &self,
        principal_id: &str,
        reason: Option<&str>,
    ) -> Result<PrincipalForgetDto, String> {
        self.inner.forget_principal(principal_id, reason).await
    }
}
