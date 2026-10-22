//! `CliPanelData` 的记忆治理端口实现（从 `gateway_panels.rs` 平移，尺寸棘轮）。
//!
//! 全部委托给 [`PanelData`] 的同名方法：真正的治理语义在
//! `apeireth_memory::MemoryGovernanceStore`（rev CAS + 状态机），本模块只做
//! 「窄口 → 全口」的转发，不复制任何逻辑。

use apeireth_gateway::{EpisodeMutationDto, MemoryGovernanceCommand, PanelData};
use async_trait::async_trait;

use crate::gateway_panels::CliPanelData;

#[async_trait]
impl MemoryGovernanceCommand for CliPanelData {
    async fn protect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        PanelData::protect_episode(self, id, expected_rev).await
    }

    async fn unprotect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        PanelData::unprotect_episode(self, id, expected_rev).await
    }

    async fn forget_episode(
        &self,
        id: &str,
        expected_rev: u64,
        reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String> {
        PanelData::forget_episode(self, id, expected_rev, reason).await
    }

    async fn update_episode(
        &self,
        id: &str,
        content: &str,
        updated_by: Option<&str>,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        PanelData::update_episode(self, id, content, updated_by, expected_rev).await
    }
}
