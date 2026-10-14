//! W3-② 用户管理命令 (`apeireth user add|list|role|remove`) 的面契约。
//!
//! 这里钉四件事:
//! 1. 引导口径: 用户表为空时首用户免令牌但**必须是 owner** (表不得失主);
//! 2. owner 角色门: 表非空后一切管理动作都要 owner 令牌 (令牌→用户→角色链),
//!    member / 未绑定 / 伪令牌 / 缺令牌一律拒;
//! 3. 管理操作与失主不变量: add/role/remove 往返 + 最后一个 owner 不可删/降;
//! 4. 拒开语义: 坏用户表 / 未知角色 / 令牌↔用户绑定不一致一律拒绝操作。

use std::path::{Path, PathBuf};

use apeireth_cli::user_admin::run_user_admin_at;
use apeireth_core::stored_doc::{self, StoredDoc};
use apeireth_gateway::{
    ClientTokenConfig, ClientTokenSpec, CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION,
    CLIENT_TOKENS_FILE,
};

const OWNER_TOKEN: &str = "tok-owner-0123456789abcdef";
const MEMBER_TOKEN: &str = "tok-member-0123456789abcdef";
const ORPHAN_TOKEN: &str = "tok-orphan-0123456789abcdef";

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-user-admin-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 写一份客户端令牌表 (`client-tokens.json`, StoredDoc 信封)。
fn write_tokens(dir: &Path, entries: &[(&str, &str, Option<&str>)]) {
    let clients = entries
        .iter()
        .map(|(name, token, user_id)| ClientTokenSpec {
            name: (*name).to_string(),
            token: (*token).to_string(),
            user_id: user_id.map(str::to_string),
        })
        .collect();
    let doc = StoredDoc::new(
        CLIENT_TOKENS_DOC_NAME,
        CLIENT_TOKENS_DOC_VERSION,
        vec![CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(clients),
    );
    fs_err::write(
        dir.join(CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

/// 引导 + 事后管理的全链: 首用户免令牌但必须 owner; 之后 owner 令牌可
/// add / list / role / remove, 全部落盘可回读。
#[test]
fn bootstrap_the_first_owner_then_manage_with_the_owner_token() {
    let dir = temp_dir("bootstrap");
    // 发令牌序 (fail-closed): 令牌条目声明 `user_id` 的, 先建用户再写绑定 ——
    // 悬空绑定会让一切管理动作拒开 (问责链不得断)。
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);

    // 引导: 用户表为空 → 免令牌, 首用户入表。
    let summary = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap add");
    assert!(summary.contains("owner-1"), "{summary}");

    // member-1 建档后才可发它的令牌绑定。
    write_tokens(
        &dir,
        &[
            ("tui", OWNER_TOKEN, Some("owner-1")),
            ("phone-web", MEMBER_TOKEN, Some("member-1")),
        ],
    );

    // 之后的管理动作要 owner 令牌: 加 member。
    let summary = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "phone-web",
            "--domain",
            "home",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("owner adds member");
    assert!(summary.contains("member-1"), "{summary}");

    // list: owner 令牌可读全表 (user/role/domain/clients)。
    let listing = run_user_admin_at(&dir, &args(&["list", "--token", OWNER_TOKEN])).expect("list");
    assert!(listing.contains("owner-1\towner"), "{listing}");
    assert!(
        listing.contains("member-1\tmember\thome\tphone-web"),
        "{listing}"
    );

    // role: member-1 降为 guest。
    let summary = run_user_admin_at(
        &dir,
        &args(&[
            "role",
            "--user-id",
            "member-1",
            "--role",
            "guest",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("role change");
    assert!(summary.contains("guest"), "{summary}");

    // remove: 令牌还引用 member-1 时删不得 (问责链不得断 → 拒)。
    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("令牌仍引用的用户删不得");
    assert!(
        error.contains("not in the user table") || error.contains("inconsistent"),
        "{error}"
    );

    // 摘掉令牌绑定后才可删 (先收令牌, 后删用户)。
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    let summary = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect("remove");
    assert!(summary.contains("member-1"), "{summary}");
}

/// owner 角色门: member 令牌 / 伪令牌 / 未绑定令牌 / 缺令牌全拒 (fail-closed)。
#[test]
fn management_is_owner_only_across_every_refusal_shape() {
    let dir = temp_dir("owner-gate");
    write_tokens(
        &dir,
        &[
            ("tui", OWNER_TOKEN, Some("owner-1")),
            ("stranger", ORPHAN_TOKEN, None),
        ],
    );
    // 非空表的最小底座: owner-1 (免令牌引导)。
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    // member-1 建档后再发它的令牌绑定 (悬空绑定拒开, 见引导测试注)。
    write_tokens(
        &dir,
        &[
            ("tui", OWNER_TOKEN, Some("owner-1")),
            ("phone-web", MEMBER_TOKEN, Some("member-1")),
            ("stranger", ORPHAN_TOKEN, None),
        ],
    );
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "phone-web",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("owner adds member");

    let attempt = |token: Option<&str>, label: &str| {
        let mut command = args(&[
            "add",
            "--user-id",
            "x-1",
            "--role",
            "guest",
            "--clients",
            "y",
        ]);
        if let Some(token) = token {
            command.extend(args(&["--token", token]));
        }
        let error = run_user_admin_at(&dir, &command).expect_err(&format!("{label} 必须被拒"));
        error
    };

    // member 自称可管理 (拿自己的令牌闯管理面) → 拒。
    let error = attempt(Some(MEMBER_TOKEN), "member 令牌");
    assert!(error.contains("member"), "{error}");
    assert!(error.contains("may not manage users"), "{error}");

    // 伪令牌 → 拒。
    let error = attempt(Some("tok-forged-0123456789abcdef"), "伪令牌");
    assert!(error.contains("invalid management token"), "{error}");

    // 表内令牌但未绑定用户 → 拒 (fail-closed)。
    let error = attempt(Some(ORPHAN_TOKEN), "未绑定令牌");
    assert!(error.contains("not bound"), "{error}");

    // 缺令牌 → 拒 (owner 凭证刚需)。
    let error = attempt(None, "缺令牌");
    assert!(error.contains("owner credential"), "{error}");

    // owner 令牌过闸 (同一动作真能成)。
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "x-1",
            "--role",
            "guest",
            "--clients",
            "y",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("owner 过闸");
}

/// 引导不变量: 首用户必须 owner (表不得以非 owner 开局失主)。
#[test]
fn the_first_user_must_be_owner() {
    let dir = temp_dir("bootstrap-owner");
    let error = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "tui",
        ]),
    )
    .expect_err("首用户非 owner 必须拒");
    assert!(error.contains("first user must be owner"), "{error}");
}

/// 失主不变量: 最后一个 owner 不可删除、不可降级 (管理面不得永久失守)。
#[test]
fn the_last_owner_can_never_be_removed_or_demoted() {
    let dir = temp_dir("last-owner");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");

    let error = run_user_admin_at(
        &dir,
        &args(&["remove", "--user-id", "owner-1", "--token", OWNER_TOKEN]),
    )
    .expect_err("删最后一个 owner 必须拒");
    assert!(error.contains("at least one owner"), "{error}");

    let error = run_user_admin_at(
        &dir,
        &args(&[
            "role",
            "--user-id",
            "owner-1",
            "--role",
            "member",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect_err("降最后一个 owner 必须拒");
    assert!(error.contains("at least one owner"), "{error}");
}

/// 拒开语义: 未知角色 / 坏用户表 / 令牌↔用户绑定不一致一律拒绝操作。
#[test]
fn broken_faces_refuse_to_open_or_to_save() {
    let dir = temp_dir("fail-closed");
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");

    // 未知角色: 拒 (词表冻结)。
    let error = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "u-2",
            "--role",
            "admin",
            "--clients",
            "z",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect_err("未知角色必须拒");
    assert!(error.contains("unknown user role"), "{error}");

    // 坏用户表 (非法 JSON): 拒开, 不回退空表 (否则会走引导口绕过 owner 门)。
    fs_err::write(dir.join("users.json"), b"{ not json").unwrap();
    let error = run_user_admin_at(&dir, &args(&["list", "--token", OWNER_TOKEN]))
        .expect_err("坏用户表必须拒开");
    assert!(error.contains("user table refused to open"), "{error}");

    // 令牌↔用户绑定不一致 (令牌声明的 user_id 存在但没回绑这个客户端名):
    // 拒操作, 不带病落盘。
    write_tokens(&dir, &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    let table = stored_doc::StoredDoc::new(
        "users",
        1,
        vec![1],
        apeireth_governance::UserTable::new(vec![apeireth_governance::UserRecord {
            user_id: "owner-1".to_string(),
            role: "owner".to_string(),
            client_names: vec!["desk".to_string()],
            domain: None,
        }]),
    );
    fs_err::write(
        dir.join("users.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();
    let error = run_user_admin_at(
        &dir,
        &args(&[
            "add",
            "--user-id",
            "u-2",
            "--role",
            "guest",
            "--clients",
            "z",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect_err("绑定不一致必须拒");
    assert!(
        error.contains("inconsistent") || error.contains("does not"),
        "{error}"
    );
}
