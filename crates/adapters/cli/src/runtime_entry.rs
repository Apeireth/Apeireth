//! Stable convenience entry points over the explicit canonical assembly.
use std::sync::Arc;

use apeireth_governance::PermissionPolicy;
use apeireth_guard::BehaviorChainGuardHook;
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::canonical::{Runtime, SessionStore};
use apeireth_runtime_assembly::canonical::production::BioProbeSlots;
use apeireth_runtime_assembly::canonical::BioApprovalSocialSource;

use crate::managed_artifacts::ArtifactHandle;
use crate::{build_canonical_runtime_assembly_from_env, CareHeartbeatSource, VoiceEmotionSource};

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
/// Provider plugins carry credentials resolution at execution time (never stored).
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    let (runtime, _, _, _, _) = build_canonical_runtime_with_sessions_from_env().await?;
    Ok(runtime)
}

/// Build the runtime and return the shared session-store / memory-backend /
/// permission-policy handles alongside it (same stores and live policy that the
/// gateway introspection surfaces serve). 情绪源绑定位不随本口出口。
pub async fn build_canonical_runtime_with_sessions_from_env() -> Result<
    (
        Runtime,
        Arc<dyn SessionStore>,
        Arc<dyn MemoryBackend>,
        Arc<std::sync::Mutex<PermissionPolicy>>,
        Arc<BehaviorChainGuardHook>,
    ),
    String,
> {
    let (
        runtime,
        session_store,
        memory,
        policy,
        guard_hook,
        _voice_emotion,
        _care_heartbeat,
        _cost_router,
    ) = build_canonical_runtime_with_sessions_and_voice_emotion_from_env().await?;
    Ok((runtime, session_store, memory, policy, guard_hook))
}

/// 生产网关路径的运行时组装 (R3 接线出口): 同一装配 + 情绪源/心跳源绑定位 +
/// 成本路由器 (H-P2); 生物 probe 迟绑定槽不随本口出口。
pub(crate) type CanonicalRuntimeParts = (
    Runtime,
    Arc<dyn SessionStore>,
    Arc<dyn MemoryBackend>,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
    VoiceEmotionSource,
    CareHeartbeatSource,
    Arc<apeireth_core::cost_router::CostRouter>,
);

pub(crate) type BioHandle = Option<Arc<BioApprovalSocialSource>>;
/// W3.3 SG-16/17: 痛觉/再生领域观察者句柄（开关关 = None，内核零接线）。
pub(crate) type NociceptionHandle = Option<Arc<apeireth_organ::nociception::NociceptionObserver>>;
pub(crate) type BioParts = (
    CanonicalRuntimeParts,
    BioProbeSlots,
    BioHandle,
    NociceptionHandle,
    ArtifactHandle,
);

pub async fn build_canonical_runtime_with_sessions_and_voice_emotion_from_env(
) -> Result<CanonicalRuntimeParts, String> {
    let (parts, _slots, _approval, _nociception, _artifacts) =
        build_canonical_runtime_assembly_from_env().await?;
    Ok(parts)
}
