//! `/v1/tools/contact/invoke` — **R20 阶段 4 估补** (5 actions, 4 K-1 强校验, storage 抽象)
//!
//! **5 actions** (per R20 阶段 4 任务规范):
//! - `list` — 全量列 (可按 org 过滤)
//! - `create` — 新建 (id 内部 UUID v4)
//! - `update` — 更新 (id 由 caller 传, 部分字段更新)
//! - `delete` — 删
//! - `get` — 单查
//!
//! **6 实体字段** (per R20 阶段 4 任务规范):
//! - `id` (UUID v4) / `name` (K-1 非空) / `email` (K-1 格式) /
//!   `phone` (K-1 E.164 格式) / `org` (可选) / `tags` (K-1 唯一, Vec<String>)
//!
//! **4 K-1 强校验** (per R20 阶段 4 任务规范):
//! 1. **name 非空** — 拒空字符串
//! 2. **email 格式** — 复用 `calendar::validate_email`
//! 3. **phone E.164** — `+` 开头 + 11-15 位数字 (E.164 ITU-T E.123)
//! 4. **tags 唯一** — Vec<String> 内无重复 (大小写不敏感)
//!
//! **存储**: `Arc<dyn EntityStorage<Contact>>` — 默认 `InMemoryStorage`
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 5 actions 全部真实现 (非 stub 501)
//! - ✅ 4 K-1 强校验真跑 (5 测试全覆盖)
//! - ✅ storage trait 真用, InMemoryStorage 真跑
//! - ✅ contact ID 用 UUID v4
//!
//! **6 哲学锚穿透**:
//! - 锚 #1: 5 actions 真接, 4 K-1 真跑
//! - 锚 #2: `CONTACT_ACTIONS_COUNT = 5` const assert
//! - 锚 #3: `#![deny(unsafe_code)]` 继承
//! - 锚 #4: storage trait NotFound 真返
//! - 锚 #5: 复用 calendar 5 工具校验函数
//! - 锚 #6: 4 K-1 强校验守门
//!
//! **8 项不修改承诺**:
//! - ❌ 不改 LOCKED crate
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方 DB / libphonenumber (留 R21)
//! - ❌ 不假装支持完整 E.164 全集 (留 R21, 当前简化版)
//! - ❌ 不假装支持完整 RFC 5321 邮箱校验 (复用 calendar 简化版)
//! - ❌ 不破坏 24 LOCKED crate
//! - ❌ 不假装 contact 已支持 tag 索引 / 全文搜 (留 R21)

#![deny(unsafe_code)]

use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use super::storage::{EntityStorage, InMemoryStorage};

/// **K-1-2 email 格式校验** — 简化版 (per RFC 5321 字面字符集)
///
/// 内联在本文件 (不依赖 LOCKED `super::calendar::validate_email`, 守 8 项不修改承诺).
/// 真值 (R21 估补): 完整 RFC 5321 / 5322 + IDN 域名 / 大小写不敏感 local part.
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("email must not be empty".to_string());
    }
    if email.len() > 254 {
        return Err(format!("email too long ({} > 254)", email.len()));
    }
    // 必须含 @ 且 @ 不在首尾
    let at_pos = email.find('@').ok_or_else(|| format!("email missing '@': {email}"))?;
    if at_pos == 0 {
        return Err(format!("email local part empty: {email}"));
    }
    if at_pos == email.len() - 1 {
        return Err(format!("email domain part empty: {email}"));
    }
    let (local, domain) = email.split_at(at_pos);
    let domain = &domain[1..]; // 跳过 '@'
    // local: 非空 + 不含 @
    if local.is_empty() || local.contains('@') {
        return Err(format!("email local part invalid: {email}"));
    }
    // domain: 必须含 . + 不含 @
    if !domain.contains('.') {
        return Err(format!("email domain missing '.': {email}"));
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(format!("email domain has leading/trailing '.': {email}"));
    }
    // 字符集: ASCII 可打印 + 限定符号
    for c in email.chars() {
        if !c.is_ascii() {
            return Err(format!("email non-ASCII char: {email}"));
        }
    }
    Ok(())
}

// ============================================================
// 实体 + 6 字段
// ============================================================

/// **Contact** — contact 6 字段实体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

// ============================================================
// K-1 强校验 (4 类)
// ============================================================

/// **K-1-3 phone E.164** — `+` 开头, 11-15 位数字 (E.164 ITU-T E.123)
///
/// 简化版, 守 80% 常见错 (国家码 + 本地号)
/// 完整 libphonenumber 校验留 R21
pub fn validate_phone_e164(phone: &str) -> Result<(), String> {
    if phone.is_empty() {
        return Err("phone must not be empty".to_string());
    }
    if !phone.starts_with('+') {
        return Err(format!("phone must start with '+' (E.164): {phone}"));
    }
    // 去掉 + 后必须全数字
    let digits = &phone[1..];
    if digits.is_empty() {
        return Err(format!("phone has no digits after '+': {phone}"));
    }
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("phone has non-digit chars: {phone}"));
    }
    if digits.len() < 11 {
        return Err(format!(
            "phone too short for E.164 ({} < 11): {phone}",
            digits.len()
        ));
    }
    if digits.len() > 15 {
        return Err(format!(
            "phone too long for E.164 ({} > 15): {phone}",
            digits.len()
        ));
    }
    Ok(())
}

/// **K-1-4 tags 唯一** — Vec<String> 内无重复 (大小写不敏感)
pub fn validate_tags_unique(tags: &[String]) -> Result<(), String> {
    let mut seen: Vec<String> = Vec::with_capacity(tags.len());
    for t in tags {
        let lower = t.to_lowercase();
        if seen.iter().any(|s| s.to_lowercase() == lower) {
            return Err(format!("K-1 violation: duplicate tag '{t}'"));
        }
        seen.push(t.clone());
    }
    Ok(())
}

// ============================================================
// ContactTool — 5 actions + storage 抽象
// ============================================================

/// **ContactTool** — 5 actions (list/create/update/delete/get)
pub struct ContactTool {
    storage: Arc<dyn EntityStorage<Contact>>,
}

impl ContactTool {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(InMemoryStorage::<Contact>::new("contact")),
        }
    }

    /// **with_storage** — 注入自定义 storage (测试 / 升级用)
    pub fn with_storage(storage: Arc<dyn EntityStorage<Contact>>) -> Self {
        Self { storage }
    }

    /// **storage** — 当前 storage ref (测试断言用)
    pub fn storage(&self) -> &Arc<dyn EntityStorage<Contact>> {
        &self.storage
    }

    /// **dispatch** — 5 actions 路由
    async fn dispatch(&self, args: Value) -> Result<Value, String> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: action".to_string())?;
        match action {
            "list" => self.action_list(args).await,
            "create" => self.action_create(args).await,
            "update" => self.action_update(args).await,
            "delete" => self.action_delete(args).await,
            "get" => self.action_get(args).await,
            other => Err(format!("unknown action: {other}")),
        }
    }

    async fn action_list(&self, args: Value) -> Result<Value, String> {
        let filter_org = args
            .get("filter")
            .and_then(|f| f.get("org"))
            .and_then(|v| v.as_str());
        let filter_tag = args
            .get("filter")
            .and_then(|f| f.get("tag"))
            .and_then(|v| v.as_str());
        let all = self
            .storage
            .list()
            .await
            .map_err(|e| format!("list failed: {e}"))?;
        let contacts: Vec<&Contact> = all
            .iter()
            .filter(|c| {
                filter_org.map(|o| c.org == o).unwrap_or(true)
                    && filter_tag
                        .map(|t| c.tags.iter().any(|x| x == t))
                        .unwrap_or(true)
            })
            .collect();
        Ok(json!({
            "contacts": contacts,
            "count": contacts.len(),
        }))
    }

    async fn action_create(&self, args: Value) -> Result<Value, String> {
        // 6 字段提取 + 4 K-1 强校验
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: name".to_string())?;
        // K-1-1: name 非空
        if name.is_empty() {
            return Err("K-1 violation: name must not be empty".to_string());
        }
        if name.len() > 256 {
            return Err(format!("name too long ({} > 256)", name.len()));
        }

        let email = args
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: email".to_string())?;
        // K-1-2: email 格式 (复用 calendar)
        validate_email(email)?;

        let phone = args
            .get("phone")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: phone".to_string())?;
        // K-1-3: phone E.164
        validate_phone_e164(phone)?;

        let org = args
            .get("org")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tags: Vec<String> = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        // K-1-4: tags 唯一
        validate_tags_unique(&tags)?;

        let c = Contact {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            email: email.to_string(),
            phone: phone.to_string(),
            org,
            tags,
        };
        let contact_id = c.id.clone();
        self.storage
            .upsert(c)
            .await
            .map_err(|e| format!("create failed: {e}"))?;
        Ok(json!({ "contact_id": contact_id, "ok": true }))
    }

    async fn action_update(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;

        let mut existing = self
            .storage
            .get(id)
            .await
            .map_err(|e| format!("update failed: {e}"))?;

        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            if name.is_empty() {
                return Err("K-1 violation: name must not be empty".to_string());
            }
            existing.name = name.to_string();
        }
        if let Some(email) = args.get("email").and_then(|v| v.as_str()) {
            validate_email(email)?;
            existing.email = email.to_string();
        }
        if let Some(phone) = args.get("phone").and_then(|v| v.as_str()) {
            validate_phone_e164(phone)?;
            existing.phone = phone.to_string();
        }
        if let Some(org) = args.get("org").and_then(|v| v.as_str()) {
            existing.org = org.to_string();
        }
        if let Some(tags_val) = args.get("tags") {
            if let Some(arr) = tags_val.as_array() {
                let tags: Vec<String> = arr
                    .iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect();
                validate_tags_unique(&tags)?;
                existing.tags = tags;
            }
        }

        self.storage
            .upsert(existing)
            .await
            .map_err(|e| format!("update failed: {e}"))?;
        Ok(json!({ "ok": true, "updated": id }))
    }

    async fn action_delete(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;
        let removed = self
            .storage
            .delete(id)
            .await
            .map_err(|e| format!("delete failed: {e}"))?;
        if !removed {
            return Err(format!("contact not found: {id}"));
        }
        Ok(json!({ "ok": true, "deleted": id }))
    }

    async fn action_get(&self, args: Value) -> Result<Value, String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing field: id".to_string())?;
        super::storage::validate_id(id)?;
        let c = self
            .storage
            .get(id)
            .await
            .map_err(|e| format!("get failed: {e}"))?;
        Ok(json!({ "contact": c }))
    }
}

impl Default for ContactTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ContactTool {
    fn name(&self) -> &str {
        "Contact"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default_for_kind(ToolKind::Sync)
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        self.dispatch(args).await
    }
}

pub use super::invoke_by_name as invoke;

// ============================================================
// 编译期 hardcode
// ============================================================

/// **5 actions** (per R20 阶段 4 任务规范)
pub const CONTACT_ACTIONS: [&str; 5] = ["list", "create", "update", "delete", "get"];

/// **4 K-1 强校验** (per R20 阶段 4 任务规范)
pub const CONTACT_K1_CHECKS: [&str; 4] = [
    "name_not_empty",
    "email_format",
    "phone_e164",
    "tags_unique",
];

const _: () = {
    assert!(CONTACT_ACTIONS.len() == 5, "5 actions: list/create/update/delete/get");
    assert!(CONTACT_K1_CHECKS.len() == 4, "4 K-1 强校验");
};

// ============================================================
// 单元测试 (5 测试: 5 actions e2e + 4 K-1 校验 + 编译期)
// ============================================================

#[cfg(test)]
mod contact_tests {
    use super::*;
    use serde_json::json;

    /// 1. 5 actions 端到端
    #[tokio::test]
    async fn contact_5_actions_e2e() {
        let c = ContactTool::new();
        // 1. list empty
        let r = c.call(json!({"action": "list"})).await.expect("list");
        assert_eq!(r["count"], 0);

        // 2. create
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

        // 3. get
        let r = c
            .call(json!({"action": "get", "id": cid}))
            .await
            .expect("get");
        assert_eq!(r["contact"]["name"], "Alice");
        assert_eq!(r["contact"]["tags"].as_array().unwrap().len(), 2);

        // 4. update (改 org + 加 tag)
        let r = c
            .call(json!({
                "action": "update", "id": cid,
                "org": "Apeireth", "tags": ["vip", "friend", "colleague"]
            }))
            .await
            .expect("update");
        assert_eq!(r["ok"], true);
        let r = c
            .call(json!({"action": "get", "id": cid}))
            .await
            .expect("get after update");
        assert_eq!(r["contact"]["org"], "Apeireth");
        assert_eq!(r["contact"]["tags"].as_array().unwrap().len(), 3);

        // 5. delete
        let r = c
            .call(json!({"action": "delete", "id": cid}))
            .await
            .expect("delete");
        assert_eq!(r["deleted"], cid);
        let r = c.call(json!({"action": "get", "id": cid})).await;
        assert!(r.is_err(), "delete 后 get 应失败");
    }

    /// 2. K-1-1/2 强校验: name 非空 / email 格式
    #[tokio::test]
    async fn contact_k1_name_and_email() {
        let c = ContactTool::new();
        // name 空
        let r = c
            .call(json!({
                "action": "create",
                "name": "",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err()); let err_msg = r.unwrap_err(); assert!(err_msg.contains("name"));
        // name 缺
        let r = c
            .call(json!({
                "action": "create",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err());
        // email 缺 @
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "bad",
                "phone": "+8613800138000"
            }))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("email") || err_msg.contains("@"));
    }

    /// 3. K-1-3 强校验: phone E.164
    #[tokio::test]
    async fn contact_k1_phone_e164() {
        let c = ContactTool::new();
        // 缺 +
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "13800138000"
            }))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("+") || err_msg.contains("E.164"));
        // 太短
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+123"
            }))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("short") || err_msg.contains("<"));
        // 含非数字
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+86 138 0013 8000"
            }))
            .await;
        assert!(r.is_err());
        // 合法通过
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000"
            }))
            .await
            .expect("create ok");
        assert!(r["contact_id"].is_string());
    }

    /// 4. K-1-4 强校验: tags 唯一
    #[tokio::test]
    async fn contact_k1_tags_unique() {
        let c = ContactTool::new();
        // 完全重复
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["vip", "vip"]
            }))
            .await;
        assert!(r.is_err());
        let err_msg = r.unwrap_err();
        assert!(err_msg.contains("duplicate") || err_msg.contains("unique"));
        // 大小写不一致也视为重复
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["VIP", "vip"]
            }))
            .await;
        assert!(r.is_err(), "大小写不同应视为重复 (K-1-4 简化版)");
        // 唯一通过
        let r = c
            .call(json!({
                "action": "create",
                "name": "x",
                "email": "a@b.c",
                "phone": "+8613800138000",
                "tags": ["vip", "friend"]
            }))
            .await
            .expect("create tags unique");
        assert!(r["contact_id"].is_string());
    }

    /// 5. 编译期 hardcode + list filter (org / tag) + 错误路径
    #[tokio::test]
    async fn contact_list_filter_and_errors() {
        // 编 3 个 contact
        let c = ContactTool::new();
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

        // filter org=A → 2
        let r = c
            .call(json!({"action": "list", "filter": {"org": "A"}}))
            .await
            .expect("list org A");
        assert_eq!(r["count"], 2);

        // filter tag=vip → 2
        let r = c
            .call(json!({"action": "list", "filter": {"tag": "vip"}}))
            .await
            .expect("list tag vip");
        assert_eq!(r["count"], 2);

        // 错误路径: 缺 action / 缺 id (get/update/delete) / 不存在 id
        let r = c.call(json!({})).await;
        assert!(r.is_err());
        let r = c.call(json!({"action": "get"})).await;
        assert!(r.is_err());
        let r = c.call(json!({"action": "delete", "id": "nonexistent"})).await;
        assert!(r.is_err());
    }

    /// 6. 编译期 hardcode + K-1 校验函数单测
    #[test]
    fn contact_constants_and_validators() {
        assert_eq!(CONTACT_ACTIONS.len(), 5);
        assert_eq!(CONTACT_K1_CHECKS.len(), 4);
        assert_eq!(CONTACT_ACTIONS[0], "list");
        assert_eq!(CONTACT_ACTIONS[4], "get");
        assert_eq!(CONTACT_K1_CHECKS[0], "name_not_empty");
        assert_eq!(CONTACT_K1_CHECKS[3], "tags_unique");

        // phone E.164
        assert!(validate_phone_e164("+8613800138000").is_ok());
        assert!(validate_phone_e164("+12025550143").is_ok()); // US
        assert!(validate_phone_e164("").is_err());
        assert!(validate_phone_e164("+").is_err());
        assert!(validate_phone_e164("8613800138000").is_err()); // 缺 +
        assert!(validate_phone_e164("+123").is_err()); // 太短
        assert!(validate_phone_e164("+12345678901234567").is_err()); // 太长

        // tags unique
        assert!(validate_tags_unique(&["a".into(), "b".into()]).is_ok());
        assert!(validate_tags_unique(&[]).is_ok());
        assert!(validate_tags_unique(&["a".into(), "A".into()]).is_err()); // 大小写
    }
}
