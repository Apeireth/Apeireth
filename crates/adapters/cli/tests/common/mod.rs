//! CLI 进程级 e2e 的共用夹具: **角色面激活** (用户表 + 过门署名名绑定)。
//!
//! # 为什么需要它 (P0-1 · 2026-11-04)
//!
//! 网关**管理面 fail-closed**: 要求 `UserCapability::AdminControl` 的端点
//! (`/v1/admin/*` 与面板族管理面) 在**角色面未激活** (数据目录没有
//! `users.json` ⇒ `UserAccess::is_active()` 为 false) 时一律 403 —— 角色面
//! 未激活 ⇒ 没有人可担 `AdminControl` ⇒ 管理面 (能热改 provider `base_url`)
//! 不得对本机任何进程可达。
//!
//! ⇒ ⇒ 凡是**要打管理面**的 e2e 用例, 必须按**真实产品前置条件**构造:
//! 注入用户表 (激活角色面) + 把有 `AdminControl` 的 owner 绑到调用方的
//! **过门署名名** (令牌→用户→角色链的用户步)。
//!
//! 过门署名名只有三种 (见 `apeireth_gateway::ClientIdentity` 的文档):
//!
//! * `env` —— env 单令牌面 (`APEIRETH_GATEWAY_TOKEN` 命中);
//! * 客户端令牌表条目名 —— 令牌表命中;
//! * `anonymous` —— 未设防口径 (env 未设且令牌表为空) 与**逃生阀**
//!   (`APEIRETH_GATEWAY_TOKEN=none`) 的过门署名。
//!
//! ⇒ owner 绑上它所走的那一面, 管理面即对 owner 令牌可达 (与网关测试面
//! `panel_routes` / `resident_service` / `cross_origin_state_change` 的既有
//! 写法同款)。
#![allow(dead_code)] // D-1 (owner 裁定 2026-10-25): 夹具按用例取用, 各测试二进制只用其中一部分。

use std::path::Path;

use apeireth_governance::{UserRecord, UserTable};

/// 过门署名名: env 单令牌面 (审计署名键; 与
/// `apeireth_gateway::GatewayTokenGate::ENV_FACE_CLIENT_NAME` 同值)。
pub const ENV_CLIENT: &str = "env";

/// 过门署名名: 未设防口径 / 逃生阀 (审计署名键; 客户端令牌表为空时过门
/// 落此名 —— 令牌门的未设防分支插 `ClientIdentity::anonymous()`)。
pub const ANONYMOUS_CLIENT: &str = "anonymous";

/// 一名 owner 用户记录 (绑定给定的过门署名名)。
pub fn owner_record(user_id: &str, client_names: &[&str]) -> UserRecord {
    UserRecord {
        user_id: user_id.to_string(),
        role: "owner".to_string(),
        client_names: client_names
            .iter()
            .map(|name| (*name).to_string())
            .collect(),
        domain: None,
    }
}

/// 写用户表 (`users.json`, 存储文档信封) = 角色面的唯一事实。
///
/// 用户表**缺失** = 角色面未激活 (兼容路径); 坏表 = 装载面拒开 (fail-closed)。
/// 写盘走 [`UserTable::save`] (同一把闸: 校验 → 原子写), 不手搓信封。
pub fn write_users(dir: &Path, records: Vec<UserRecord>) {
    let table = UserTable::new(records);
    table.validate().expect("valid user table");
    table.save(dir).expect("save user table");
}

/// **激活角色面**: 写一份 owner 名册, 绑定给定的过门署名名。
///
/// 激活后, 带对应凭证的调用方可解析出 owner 身份 ⇒ 持 `AdminControl`
/// (管理面) 与 `ManageUsers` (用户管理面) 两档能力。
pub fn activate_role_surface(dir: &Path, user_id: &str, client_names: &[&str]) {
    write_users(dir, vec![owner_record(user_id, client_names)]);
}
