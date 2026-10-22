//! 装配期环境开关读取面。
//!
//! 架构契约 `[env]` 的方向是「env 读取收敛到 config 模块，现状散在 148 文件（只减
//! 不增）」。本模块是 Recon 接线批的落地：**开关名 = 开关权** —— 装配侧注册与治理
//! 侧授权读同一份真源（`apeireth_tools_canonical::recon::wiring`），本处不重复解析
//! env，也不手抄能力 id。
//!
//! 收敛进本模块的既有旋钮：[`onion_layer_enabled_from_env`]（从 `lib.rs` 平移，
//! 行为逐字节不变；平移同时抵消本批新增行，保持 `lib.rs` 尺寸棘轮只减不增）。

use std::sync::{Arc, Mutex};

use apeireth_governance::PermissionPolicy;

pub use apeireth_tools_canonical::recon::wiring::{
    external_config_from_env, external_enabled_from_env, recon_config_from_env,
    recon_enabled_from_env, ENABLE_EXTERNAL_ENV, ENABLE_RECON_ENV, EXTERNAL_TOOL_IDS,
    RECON_LOCAL_TOOL_IDS, RECON_NETWORK_TOOL_IDS, TOOL_SCOPE_ENV,
};

/// **W3 洋葱层旋钮** (2026-10-10, 默认关): `APEIRETH_ENABLE_ONION_LAYER=1` ——
/// 生产治理管线末层追加双洋葱权威判定 (HA 离线物拒 / L5 E 层兜底)。
pub fn onion_layer_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_ONION_LAYER")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 把 Recon / 外部适配器的授权写进生产策略面。
///
/// **开关开 = 注册 + 授权同源**：同一开关值既决定装配侧注册（`production.rs`），
/// 也决定本处 grant，杜绝「注册了但权力没到手」。
///
/// 治理档位按**爆炸半径**分两档，不按便利：
///
/// * 只读本地分析 6 件（pack / binfmt / yara / dotnet / js / pcap）—— grant 即用
///   零审批，与 `tool.repo` / `tool.education` 同档（零出网、零执行、workspace 限域）；
/// * 出网 3 件（net / web / tls）+ 外部适配器 10 件 —— grant + `require_approval`，
///   每次调用停人工审批：前者对外发报文，后者执行第三方二进制。
pub fn apply_recon_external_grants(policy: &Arc<Mutex<PermissionPolicy>>) {
    if recon_enabled_from_env() {
        let mut guard = policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)");
        for id in RECON_LOCAL_TOOL_IDS {
            guard.grant(apeireth_governance::Permission::ExecuteTool(
                (*id).to_string(),
            ));
        }
        for id in RECON_NETWORK_TOOL_IDS {
            guard.grant(apeireth_governance::Permission::ExecuteTool(
                (*id).to_string(),
            ));
            guard.require_approval_for(*id);
        }
    }
    if external_enabled_from_env() {
        let mut guard = policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)");
        for id in EXTERNAL_TOOL_IDS {
            guard.grant(apeireth_governance::Permission::ExecuteTool(
                (*id).to_string(),
            ));
            guard.require_approval_for(*id);
        }
    }
}
