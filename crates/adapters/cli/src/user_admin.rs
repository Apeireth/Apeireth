//! CLI 用户管理面 (W3-②): `apeireth user add|list|role|remove`。
//!
//! **双入口同源**: 本面只做 CLI 外壳 (参数解析 / owner 凭证门 / 文本输出), 管理
//! 动作全部走网关面同一份 UserTable 服务
//! ([`apeireth_gateway::UserAdminService`]), 与 HTTP 面
//! (`/v1/panel/users`、`/v1/admin/users*`) 改**同一份** `users.json` —— 引导
//! 口径 / 末 owner 保护 / 发令牌序一处实现, 两个入口行为逐字一致。
//!
//! 用户表持久化 = 数据目录 `users.json` (存储文档拒开语义, 与 W3-① 域目录的
//! 接缝 = 用户表独立文件 + `domain` 域引用宽接口)。管理动作过 **owner 角色门**:
//! 出示 `--token` (令牌→用户→角色链), 只有 `ManageUsers` 能力 (owner 档) 可管理;
//! 非 owner / 未绑定 / 伪令牌一律拒绝。
//!
//! 引导口径 (fail-closed, 不锁死): 用户表为空时 `user add` 是引导入口 ——
//! 免令牌, 但**第一个用户必须是 owner** (由共享服务强制, 否则用户表永久失主,
//! 管理面失守); 表非空后一切管理动作都要 owner 令牌。破坏性不变量: 最后一个
//! owner 不可删除/降级 ([`apeireth_governance::UserTable`] 拒)。
//!
//! 发令牌序 (问责链不得断, 落盘前过 `validate_client_user_bindings`): 令牌条目
//! 声明 `user_id` 的, **先建用户再写绑定**; 删用户前**先摘令牌绑定** —— 悬空/
//! 名实不符的绑定会让管理动作拒开 (网关侧同一把闸: 装载期拒开)。
//!
//! 本面是本地文件管理 (与 `onboard` 同族): 无网关审计面, 事实走 stdout/stderr
//! 与退出码, 不伪造 `gateway.user_action` 审计行 (那是网关角色门的面)。

use std::collections::BTreeMap;
use std::path::Path;

use apeireth_gateway::{
    resolve_user_identity, ClientTokenTable, GateVerdict, GatewayTokenGate, UserAdminService,
};
use apeireth_governance::{UserCapability, UserIdentity, UserTable};

/// `apeireth user` 的用法行 (错误帧随行, 帮助面同源)。
const USAGE: &str = "Usage:\n  apeireth user add --user-id ID --role owner|member|guest [--clients NAME[,NAME...]] [--domain DOMAIN] [--token TOKEN]\n  apeireth user list [--token TOKEN]\n  apeireth user role --user-id ID --role owner|member|guest [--token TOKEN]\n  apeireth user remove --user-id ID [--token TOKEN]";

/// 在默认数据目录 (`APEIRETH_DATA_DIR` / `~/.apeireth`) 执行管理命令。
pub fn run_user_admin(args: &[String]) -> Result<String, String> {
    run_user_admin_at(&crate::default_panel_data_dir(), args)
}

/// 在指定数据目录执行管理命令 (测试与运维同入口)。
pub fn run_user_admin_at(data_dir: &Path, args: &[String]) -> Result<String, String> {
    let (action, options) = parse_user_admin(args)?;
    let tokens = ClientTokenTable::load(Some(data_dir))
        .map_err(|error| format!("client token table refused to open: {error}"))?;
    // 管理面与网关令牌门同一令牌族口径 (env 单令牌 + 令牌表, 恒定时间比较)。
    let gate = GatewayTokenGate::from_env();
    // 共享 UserTable 服务 (HTTP 面同一份): 用户表缺失 = 空表, 坏表拒开。
    let service = UserAdminService::new(data_dir);
    let users = service.load().map_err(|error| error.to_string())?;
    let bootstrap = users.users.is_empty();
    let bearer = options.get("token").map(String::as_str);

    match action.as_str() {
        "add" => {
            let user_id = required(&options, "user-id")?;
            let role_raw = required(&options, "role")?;
            let clients = split_clients(options.get("clients"));
            if !bootstrap {
                require_manager(&tokens, &users, bearer, &gate)?;
            }
            // 引导口径由共享服务强制: 表空时首个用户必须是 owner。
            let created = service
                .add(
                    user_id,
                    role_raw,
                    clients,
                    options.get("domain").map(String::as_str),
                )
                .map_err(|error| format!("refusing to add user {user_id:?}: {error}"))?;
            Ok(format!("added user {} ({})", created.user_id, created.role))
        }
        "list" => {
            if !bootstrap {
                require_manager(&tokens, &users, bearer, &gate)?;
            }
            let rows = service.list().map_err(|error| error.to_string())?;
            if rows.is_empty() {
                return Ok("(user table is empty)".to_string());
            }
            let mut lines = Vec::with_capacity(rows.len());
            for record in &rows {
                lines.push(format!(
                    "{}\t{}\t{}\t{}",
                    record.user_id,
                    record.role,
                    record.domain.as_deref().unwrap_or("-"),
                    if record.client_names.is_empty() {
                        "-".to_string()
                    } else {
                        record.client_names.join(",")
                    }
                ));
            }
            Ok(lines.join("\n"))
        }
        "role" => {
            let user_id = required(&options, "user-id")?;
            let role_raw = required(&options, "role")?;
            require_manager(&tokens, &users, bearer, &gate)?;
            let updated = service
                .set_role(user_id, role_raw)
                .map_err(|error| format!("refusing to change role: {error}"))?;
            Ok(format!("user {} is now {}", updated.user_id, updated.role))
        }
        "remove" => {
            let user_id = required(&options, "user-id")?;
            require_manager(&tokens, &users, bearer, &gate)?;
            let removed = service
                .remove(user_id)
                .map_err(|error| format!("refusing to remove user: {error}"))?;
            Ok(format!("removed user {}", removed.user_id))
        }
        other => Err(format!(
            "unknown user command: {other} (expected add, list, role, or remove)\n{USAGE}"
        )),
    }
}

/// 管理动作的角色门: 令牌→用户→角色链解析, 只有 [`UserCapability::ManageUsers`]
/// (owner 档) 放行。fail-closed: 伪令牌/未绑定/非 owner 全拒。出示的令牌过
/// **整个令牌族**校验 (与网关令牌门口径一致: env 单令牌 + 客户端令牌表,
/// 恒定时间比较; env 面单令牌的客户端名固定 `env`)。
fn require_manager(
    tokens: &ClientTokenTable,
    users: &UserTable,
    bearer: Option<&str>,
    gate: &GatewayTokenGate,
) -> Result<UserIdentity, String> {
    let Some(bearer) = bearer else {
        return Err(
            "user management requires an owner credential: pass --token <owner token> \
             (only the owner role may manage users)"
                .to_string(),
        );
    };
    let GateVerdict::Pass { client } = gate.verify(Some(bearer), tokens) else {
        return Err(
            "invalid management token: it matches no credential of the token family".to_string(),
        );
    };
    let identity = resolve_user_identity(&client, tokens, users).ok_or_else(|| {
        format!("refusing to manage users: client {client:?} is not bound to any user")
    })?;
    if !identity.allows(UserCapability::ManageUsers) {
        return Err(format!(
            "refusing to manage users: role {} may not manage users (owner only)",
            identity.role
        ));
    }
    Ok(identity)
}

/// 解析 `apeireth user <action> [--flag value ...]` (逐动作白名单, 未知旗标拒)。
fn parse_user_admin(args: &[String]) -> Result<(String, BTreeMap<String, String>), String> {
    let Some((action, rest)) = args.split_first() else {
        return Err(format!("user requires a subcommand\n{USAGE}"));
    };
    let allowed: &[&str] = match action.as_str() {
        "add" => &["user-id", "role", "clients", "domain", "token"],
        "list" => &["token"],
        "role" => &["user-id", "role", "token"],
        "remove" => &["user-id", "token"],
        other => {
            return Err(format!(
                "unknown user command: {other} (expected add, list, role, or remove)\n{USAGE}"
            ))
        }
    };
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let mut index = 0;
    while index < rest.len() {
        let flag = rest[index].as_str();
        let Some(name) = flag.strip_prefix("--") else {
            return Err(format!("unknown user argument: {flag}\n{USAGE}"));
        };
        if !allowed.contains(&name) {
            return Err(format!("unknown user argument: {flag}\n{USAGE}"));
        }
        let value = rest
            .get(index + 1)
            .ok_or_else(|| format!("user {flag} requires a value\n{USAGE}"))?;
        if options.insert(name.to_string(), value.clone()).is_some() {
            return Err(format!("user {flag} was given twice\n{USAGE}"));
        }
        index += 2;
    }
    Ok((action.clone(), options))
}

fn required<'a>(options: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, String> {
    options
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("user command requires --{name}\n{USAGE}"))
}

fn split_clients(raw: Option<&String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}
