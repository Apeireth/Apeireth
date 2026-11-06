//! **被遗忘权**的数据形状与默认实现。
//!
//! # 这个能力是什么
//!
//! 「删除主体」（删用户）在隐私语义上是**连记忆一起删**，不只是删账号。
//! 此前网关的 `DELETE /v1/admin/users/:id` **只改用户表** —— 用户的记忆还留着。
//! 单条 episode 遗忘（`forget_episode`）也不够：那是用户手动点某一条，
//! 不是「删除主体」。
//!
//! # 为什么单独一个模块
//!
//! 尺寸棘轮（只减不增）：`panels.rs` 已在基线边缘，
//! 新增的 DTO 与 trait 方法必须住在别处。
//!
//! # 语义要点
//!
//! - **一个事务**：要么全清、要么不清。分多次删会出现「删了一半」的中间态，
//!   那种状态比没删更糟（用户以为已删，实际还留着）；
//! - **protected 人格记录 ⇒ 整体拒绝**，不静默跳过（静默跳过会让用户以为
//!   「全删了」而实际留了一部分）；
//! - **未接线时诚实报错**，不静默成功 —— 静默成功会让「删了用户但记忆还在」
//!   这件事不可见。

use async_trait::async_trait;
use serde::Serialize;

use crate::panels::EpisodeMutationDto;

/// 按主体全量遗忘的结果（**被遗忘权**）。
///
/// 逐项列出**实际清了哪些账** —— 因为「全删」这个词会让用户以为
/// 一切都删干净了，而实际可能有账没被覆盖（如 `preference` 在旧库里没有表）。
/// 逐项报出来，调用方才能诚实地告知用户。
#[derive(Debug, Clone, Default, Serialize)]
pub struct PrincipalForgetDto {
    /// 主体 id。
    pub principal_id: String,
    /// episode 记忆（软删：写入治理视图的 forgotten 状态）。
    pub episode: bool,
    /// 偏好。
    pub preference: bool,
    /// 承诺（取消 + 记事件）。
    pub commitment: bool,
    /// 时序事实（写撤回归档，不物理删）。
    pub temporal: bool,
    /// 人格档案（墓碑化）。
    pub persona: bool,
    /// 是否**整体拒绝**（例如有 protected 人格记录 ⇒ 不静默跳过）。
    pub refused: bool,
    /// 拒绝原因（`refused = true` 时有值）。
    pub reason: Option<String>,
}

/// 主体遗忘的默认实现：**未接线**（诚实报错）。
///
/// 与 `update_episode` 同套路：抽象面给出方法，具体实现由 embedder 提供。
/// 默认实现返回明确错误而不是静默成功。
pub fn forget_principal_unwired(
    _principal_id: &str,
    _reason: Option<&str>,
) -> Result<PrincipalForgetDto, String> {
    Err("memory.forget_principal 未接线（本 embedder 未提供主体遗忘面）".to_string())
}

#[async_trait]
pub trait MemoryGovernanceCommand: Send + Sync {
    async fn protect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String>;
    async fn unprotect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String>;
    async fn forget_episode(
        &self,
        id: &str,
        expected_rev: u64,
        reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String>;

    /// 纠正内容（见 [`PanelData::update_episode`] 的语义）。
    async fn update_episode(
        &self,
        _id: &str,
        _content: &str,
        _updated_by: Option<&str>,
        _expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        Err("memory.update 未接线（本 embedder 未提供纠正面）".to_string())
    }

    /// **按主体全量遗忘**（被遗忘权）：跨 5 账一个事务清完。
    ///
    /// 删用户原来**只改用户表** —— 用户的记忆还留着。而「删除主体」在隐私
    /// 语义上是**连记忆一起删**。默认实现诚实报未接线。
    async fn forget_principal(
        &self,
        principal_id: &str,
        reason: Option<&str>,
    ) -> Result<PrincipalForgetDto, String> {
        crate::principal_forget::forget_principal_unwired(principal_id, reason)
    }
}
