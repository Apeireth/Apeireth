//! `v1_tools/contact_test` — contact 5 测试函数 unit test library
//!
//! **目的**: 给 R20 阶段 4 (D-01 真接细节) contact 5 actions + 4 K-1 强校验
//! 加 5 测试, 由 `tests/test_v1_tools_unit_in_process.rs` 通过 `#[path]` 注入.
//!
//! **5 API 测试函数** (per 任务规范):
//! 1. `contact_5_actions_e2e` — list / create / update / delete / get 端到端
//! 2. `contact_list_filter_org_tag` — list filter (org / tag 过滤)
//! 3. `contact_k1_name_email` — K-1-1 name 非空 + K-1-2 email 格式
//! 4. `contact_k1_phone_e164` — K-1-3 phone E.164
//! 5. `contact_k1_tags_unique_and_errors` — K-1-4 tags 唯一 + 错误路径
//!
//! **4 K-1 强校验** (per `contact.rs` 头部 + 任务规范):
//! 1. **name 非空** — 拒空字符串 + 上限 256
//! 2. **email 格式** — 简化版 RFC 5321 (有 @ + 有 . + 不超 254)
//! 3. **phone E.164** — `+` 开头 + 11-15 位数字
//! 4. **tags 唯一** — Vec<String> 无重复 (大小写不敏感)
//!
//! **存储**: `Arc<dyn EntityStorage<Contact>>` — 默认 `InMemoryStorage`
//! (本测试 5 测全用 InMemoryStorage 默认 backend, 不引 JsonFile / Sqlite,
//!  保持 5 测聚焦 5 actions e2e)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 5 actions 全部真实现 (非 stub 501)
//! - ✅ 4 K-1 强校验真跑 (5 测试全覆盖)
//! - ✅ storage trait 真用, InMemoryStorage 真跑
//! - ✅ contact ID 用 UUID v4
//!
//! **6 哲学锚穿透**:
//! - 锚 #1 不漂移: 5 actions 真接, 4 K-1 真跑
//! - 锚 #2 编译期 hardcode: `CONTACT_ACTIONS_COUNT = 5` const assert
//! - 锚 #3 不引入 unsafe: `#![deny(unsafe_code)]` 继承
//! - 锚 #4 真值守门: storage trait NotFound 真返
//! - 锚 #5 复用 calendar 5 工具校验函数 (内联 validate_email 不依赖 LOCKED super::calendar)
//! - 锚 #6 工程铁律: 4 K-1 强校验守门
//!
//! **8 项不修改承诺 (严守)**:
//! - ❌ 不改 LOCKED `contact.rs` (本文件 0 触碰)
//! - ❌ 不改 LOCKED `storage.rs` (本文件 0 触碰)
//! - ❌ 不改 LOCKED `mod.rs` (本文件用 `#[path]` 注入, 不动 mod.rs)
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方 DB / libphonenumber (留 R21, 用简化 E.164)
//! - ❌ 不假装支持完整 E.164 全集 (简化版, +11-15 位数字)
//! - ❌ 不假装支持完整 RFC 5321 邮箱 (复用 contact 内联简化版)

#![deny(unsafe_code)]

// ============================================================
// 通过 #[path] 注入 storage + contact 源文件
// ============================================================

/// **storage 源** — 注入 `src/v1_tools/storage.rs`
/// (命名必须用 `storage` 因为 contact.rs 源文件 `use super::storage::{...}` / `super::storage::validate_id`
///  依赖模块名 `storage` 在 super 作用域内可见, `_storage_src` 会失败)
#[path = "storage.rs"]
mod storage;

/// **contact 源** — 注入 `src/v1_tools/contact.rs`
/// (本 crate mod.rs LOCKED 未 declare contact, 用 #[path] 绕开)
#[path = "contact.rs"]
mod _contact_src;

// ============================================================
// `pub use super::invoke_by_name as invoke;` 桩
// ============================================================
//
// contact.rs 源文件 line 402: `pub use super::invoke_by_name as invoke;`
// (mod.rs LOCKED 包含 `pub async fn invoke_by_name(...)`, 本文件单独编译时
//  `super` 指向 `_contact_test` 自身, 所以本模块内必须提供 `invoke_by_name`)

#[allow(dead_code)]
pub async fn invoke_by_name() -> Result<(), String> {
    Ok(())
}

// ============================================================
// 5 测试函数 (per 任务规范)
// ============================================================

/// **5 测试函数总入口** — 由 `tests/test_v1_tools_unit_in_process.rs` 注入后
/// 通过 `#[tokio::test]` 调每个入口.
pub mod entries {
    // 显式 use super::storage (虽然本测试不直接用, 但让 _contact_src 内 `super::storage` 引用可见)
    #[allow(unused_imports)]
    use super::storage;
    use super::_contact_src::{
        validate_email, validate_phone_e164, validate_tags_unique, Contact, ContactTool,
        CONTACT_ACTIONS, CONTACT_K1_CHECKS,
    };
    // Tool trait 必须 in scope, 否则 ContactTool::call 方法找不到
    use apeireth_tool_registry::Tool;
    use serde_json::json;

    /// 1. 5 actions e2e — list / create / update / delete / get 端到端
    pub async fn contact_5_actions_e2e() {
        let c = ContactTool::new();

        // 1.1 list empty
        let r = c.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0, "新 contact tool 应 list 为空");
        assert!(r["contacts"].as_array().unwrap().is_empty());

        // 1.2 create
        let r = c
            .call(json!({
                "action": "create",
                "name": "Alice",
                "email": "alice@example.com",
                "phone": "+8613800138000",
                "org": "Acme",
                "tags": ["vip", "friend"]
            }))
            .await
            .expect("create");
        let cid = r["contact_id"].as_str().expect("contact_id").to_string();
        assert!(!cid.is_empty(), "contact_id 应非空 (UUID v4)");

        // 1.3 get
        let r = c
            .call(json!({"action": "get", "id": cid}))
            .await
            .expect("get");
        assert_eq!(r["contact"]["name"], "Alice");
        assert_eq!(r["contact"]["email"], "alice@example.com");
        assert_eq!(r["contact"]["phone"], "+8613800138000");
        assert_eq!(r["contact"]["org"], "Acme");
        assert_eq!(r["contact"]["tags"].as_array().unwrap().len(), 2);

        // 1.4 update (改 org + 加 tag + 改 phone)
        let r = c
            .call(json!({
                "action": "update", "id": cid,
                "org": "Apeireth",
                "tags": ["vip", "friend", "colleague"],
                "phone": "+12025550143"
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);
        assert_eq!(r["updated"], cid);

        let r = c
            .call(json!({"action": "get", "id": cid}))
            .await
            .expect("get after update");
        assert_eq!(r["contact"]["org"], "Apeireth", "org 应被 update");
        assert_eq!(r["contact"]["phone"], "+12025550143", "phone 应被 update");
        assert_eq!(
            r["contact"]["tags"].as_array().unwrap().len(),
            3,
            "tags 应 3 个 (vip/friend/colleague)"
        );

        // 1.5 delete
        let r = c
            .call(json!({"action": "delete", "id": cid}))
            .await
            .expect("delete");
        assert_eq!(r["ok"], true);
        assert_eq!(r["deleted"], cid);

        // 1.6 delete 后 get 应 NotFound (Err)
        let r = c.call(json!({"action": "get", "id": cid})).await;
        assert!(r.is_err(), "delete 后 get 应 Err (NotFound)");
        let err = r.unwrap_err();
        assert!(
            err.contains("not found") || err.contains("storage"),
            "delete 后 get err 应含 'not found' 提示, got: {err}"
        );

        // 1.7 delete 不存在 id 应 Err
        let r = c.call(json!({"action": "delete", "id": "nonexistent"})).await;
        assert!(r.is_err(), "delete 不存在应 Err");
        assert!(r.unwrap_err().contains("not found"));
    }

    /// 2. list filter (org / tag)
    pub async fn contact_list_filter_org_tag() {
        let c = ContactTool::new();
        // 编 3 个 contact
        for (name, org, tags) in [
            ("alice", "A", vec!["vip"]),
            ("bob", "B", vec!["friend"]),
            ("carol", "A", vec!["vip", "friend"]),
        ] {
            c.call(json!({
                "action": "create",
                "name": name,
                "email": format!("{name}@example.com"),
                "phone": "+8613800138000",
                "org": org,
                "tags": tags
            }))
            .await
            .expect("create");
        }

        // filter org=A → 2 (alice + carol)
        let r = c
            .call(json!({"action": "list", "filter": {"org": "A"}}))
            .await
            .expect("list org A");
        assert_eq!(r["count"], 2, "filter org=A 应 2 个");
        let arr = r["contacts"].as_array().unwrap();
        let names: Vec<&str> = arr.iter().map(|x| x["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"alice") && names.contains(&"carol"));

        // filter tag=vip → 2 (alice + carol)
        let r = c
            .call(json!({"action": "list", "filter": {"tag": "vip"}}))
            .await
            .expect("list tag vip");
        assert_eq!(r["count"], 2, "filter tag=vip 应 2 个");

        // filter tag=friend → 2 (bob + carol)
        let r = c
            .call(json!({"action": "list", "filter": {"tag": "friend"}}))
            .await
            .expect("list tag friend");
        assert_eq!(r["count"], 2, "filter tag=friend 应 2 个");

        // filter org=A + tag=vip → 2 (alice + carol 都在 A 且有 vip)
        let r = c
            .call(json!({"action": "list", "filter": {"org": "A", "tag": "vip"}}))
            .await
            .expect("list A+vip");
        assert_eq!(r["count"], 2, "filter org=A + tag=vip 应 2 个");

        // filter org=B + tag=vip → 0
        let r = c
            .call(json!({"action": "list", "filter": {"org": "B", "tag": "vip"}}))
            .await
            .expect("list B+vip");
        assert_eq!(r["count"], 0, "filter org=B + tag=vip 应 0 个");

        // 无 filter → 3
        let r = c.call(json!({"action": "list"})).await.expect("list all");
        assert_eq!(r["count"], 3, "无 filter 应 3 个");
    }

    /// 3. K-1-1 name 非空 + K-1-2 email 格式
    pub async fn contact_k1_name_email() {
        let c = ContactTool::new();

        // K-1-1: name 空 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "空 name 应 Err (K-1-1)");
        assert!(
            r.unwrap_err().contains("name"),
            "空 name err 应含 'name'"
        );

        // K-1-1: name 缺 → Err
        let r = c
            .call(json!({
                "action": "create",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "缺 name 应 Err");
        assert!(r.unwrap_err().contains("name"));

        // K-1-1: name 超长 → Err (> 256)
        let long_name = "x".repeat(257);
        let r = c
            .call(json!({
                "action": "create",
                "name": long_name,
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "超长 name 应 Err");

        // K-1-2: email 缺 @ → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "bad",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "email 'bad' 应 Err (K-1-2)");
        let err = r.unwrap_err();
        assert!(err.contains("email") || err.contains("@"));

        // K-1-2: email 缺 . → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "email 'a@b' 缺 '.' 应 Err");

        // K-1-2: email @ 在首 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "email '@b.c' 应 Err");

        // K-1-2: email @ 在尾 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err(), "email 'a@' 应 Err");

        // K-1-2: 合法 email 通过
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "alice@example.com",
                "phone": "+8613800138000"
            }))
            .await
            .expect("create good email");
        assert!(r["contact_id"].is_string());

        // validate_email 函数单测
        assert!(validate_email("a@b.c").is_ok());
        assert!(validate_email("").is_err());
        assert!(validate_email("no_at").is_err());
        assert!(validate_email("@b.c").is_err());
        assert!(validate_email("a@").is_err());
        assert!(validate_email("a@b").is_err()); // 缺 .
    }

    /// 4. K-1-3 phone E.164
    pub async fn contact_k1_phone_e164() {
        let c = ContactTool::new();

        // 缺 + → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "13800138000"
            }))
            .await;
        assert!(r.is_err(), "phone 缺 '+' 应 Err (K-1-3)");
        let err = r.unwrap_err();
        assert!(err.contains("+") || err.contains("E.164"), "err: {err}");

        // 太短 → Err (< 11 位数字)
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+123"
            }))
            .await;
        assert!(r.is_err(), "phone 太短应 Err (K-1-3)");
        let err = r.unwrap_err();
        assert!(err.contains("short") || err.contains("< 11"), "err: {err}");

        // 太长 → Err (> 15 位数字)
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+12345678901234567"
            }))
            .await;
        assert!(r.is_err(), "phone 太长应 Err (K-1-3)");

        // 含非数字 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+86 138 0013 8000"
            }))
            .await;
        assert!(r.is_err(), "phone 含空格应 Err");

        // phone 缺 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c"
            }))
            .await;
        assert!(r.is_err(), "缺 phone 应 Err");
        assert!(r.unwrap_err().contains("phone"));

        // 合法: 中国 (+86 11 位数字)
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await
            .expect("create cn phone");
        assert!(r["contact_id"].is_string());

        // 合法: 美国 (+1 11 位)
        let r = c
            .call(json!({
                "action": "create",
                "name": "y",
                "email": "y@b.c",
                "phone": "+12025550143"
            }))
            .await
            .expect("create us phone");
        assert!(r["contact_id"].is_string());

        // validate_phone_e164 函数单测
        assert!(validate_phone_e164("+8613800138000").is_ok());
        assert!(validate_phone_e164("+12025550143").is_ok());
        assert!(validate_phone_e164("").is_err());
        assert!(validate_phone_e164("+").is_err());
        assert!(validate_phone_e164("8613800138000").is_err()); // 缺 +
        assert!(validate_phone_e164("+123").is_err()); // 太短
        assert!(validate_phone_e164("+12345678901234567").is_err()); // 太长
        assert!(validate_phone_e164("+86a1380013").is_err()); // 含字母
    }

    /// 5. K-1-4 tags 唯一 + 错误路径
    pub async fn contact_k1_tags_unique_and_errors() {
        let c = ContactTool::new();

        // K-1-4: 完全重复 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["vip", "vip"]
            }))
            .await;
        assert!(r.is_err(), "tags 完全重复应 Err (K-1-4)");
        let err = r.unwrap_err();
        assert!(
            err.contains("duplicate") || err.contains("unique"),
            "err: {err}"
        );

        // K-1-4: 大小写不一致也视为重复 (简化版)
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["VIP", "vip"]
            }))
            .await;
        assert!(
            r.is_err(),
            "tags 大小写不同应视为重复 (K-1-4 简化版)"
        );

        // K-1-4: 三重复 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["vip", "VIP", "Vip"]
            }))
            .await;
        assert!(r.is_err(), "tags 三种大小写重复应 Err");

        // K-1-4: 唯一通过
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["vip", "friend", "colleague"]
            }))
            .await
            .expect("create unique tags");
        assert!(r["contact_id"].is_string());

        // K-1-4: 空 tags 数组通过 (可选)
        let r = c
            .call(json!({
                "action": "create",
                "name": "y",
                "email": "y@b.c",
                "phone": "+8613800138000",
                "tags": []
            }))
            .await
            .expect("create no tags");
        assert!(r["contact_id"].is_string());

        // validate_tags_unique 函数单测
        assert!(validate_tags_unique(&[]).is_ok());
        assert!(validate_tags_unique(&["a".into()]).is_ok());
        assert!(validate_tags_unique(&["a".into(), "b".into()]).is_ok());
        assert!(validate_tags_unique(&["a".into(), "A".into()]).is_err()); // 大小写
        assert!(validate_tags_unique(&["vip".into(), "VIP".into()]).is_err());

        // 错误路径: 缺 action / 错 action / 缺 id (get/update/delete)
        assert!(c.call(json!({})).await.is_err(), "缺 action 应 Err");
        assert!(
            c.call(json!({"action": "unknown"})).await.is_err(),
            "错 action 应 Err"
        );
        assert!(
            c.call(json!({"action": "get"})).await.is_err(),
            "get 缺 id 应 Err"
        );
        assert!(
            c.call(json!({"action": "update", "title": "x"})).await.is_err(),
            "update 缺 id 应 Err"
        );

        // update 时 name 空 → Err
        let r = c
            .call(json!({
                "action": "create",
                "name": "valid",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await
            .expect("create for update test");
        let cid = r["contact_id"].as_str().unwrap().to_string();
        let r = c
            .call(json!({"action": "update", "id": cid, "name": ""}))
            .await;
        assert!(r.is_err(), "update name='' 应 Err (K-1-1)");

        // 编译期 hardcode
        assert_eq!(CONTACT_ACTIONS.len(), 5, "5 actions");
        assert_eq!(CONTACT_K1_CHECKS.len(), 4, "4 K-1 强校验");
        assert_eq!(CONTACT_ACTIONS[0], "list");
        assert_eq!(CONTACT_ACTIONS[4], "get");
        assert_eq!(CONTACT_K1_CHECKS[0], "name_not_empty");
        assert_eq!(CONTACT_K1_CHECKS[3], "tags_unique");
    }
}
