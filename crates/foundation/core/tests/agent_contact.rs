//! 联系人 / Agent 身份 + 人设（后端合同 B1 批）集成测试。
//!
//! 覆盖验收口径：
//! - 同名不同 `contact_id` 不碰撞；
//! - `contact_id` / `agent_id` 由后端铸造（客户端无法指定）；
//! - 人设字节上限（边界值恰好等于上限接受，超一字节拒绝）与控制字符校验；
//! - `persona_revision` / `revision` 的单调语义；
//! - 乐观并发冲突时状态零变化（含 `updated_at_ms`）；
//! - 归档是软删除且不可复活；
//! - 落盘后重启读回逐字段一致（墓碑保留）；
//! - 线格式：`ContactOut` 无内部字段，`ContactIn` 不采信客户端状态/能力/版本。

use std::path::{Path, PathBuf};

use apeireth_core::agent_contact::{
    Contact, ContactCapabilities, ContactCreateRequest, ContactError, ContactIn, ContactOut,
    ContactPatch, ContactRegistry, ContactStatus, ContactWorkspaceBinding, ContractMemoryScope,
    FileContactRegistry, InMemoryContactRegistry, PersonaText, MAX_PERSONA_BYTES,
};
use apeireth_core::kernel::ids::{ContactAgentId, ContactId, WorkspaceId};
use uuid::Uuid;

/// 固定时间戳（毫秒），保证断言可复核。
const NOW_MS: u64 = 1_700_000_000_000;

/// 稍晚的时间戳。
const LATER_MS: u64 = 1_700_000_060_000;

/// 临时根目录（`apeireth-core` 无 `tempfile` dev-dependency ⇒ 自造临时目录）。
fn temp_root(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("apeireth-b1-{tag}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&path).expect("创建临时目录");
    path
}

/// 清理临时根目录。
fn cleanup(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

/// 建一个联系人（草稿态，无人设）。
fn create_contact(registry: &InMemoryContactRegistry, display_name: &str) -> Contact {
    registry
        .create(ContactCreateRequest {
            display_name: display_name.to_string(),
            subtitle: None,
            avatar_ref: None,
            persona: None,
            memory_scope: ContractMemoryScope::None,
            now_ms: NOW_MS,
        })
        .expect("create")
}

// ---------------------------------------------------------------------------
// 身份
// ---------------------------------------------------------------------------

#[test]
fn 同名不同_contact_id_不碰撞() {
    let registry = InMemoryContactRegistry::new();
    let first = create_contact(&registry, "阿岚");
    let second = create_contact(&registry, "阿岚");

    assert_ne!(
        first.contact_id(),
        second.contact_id(),
        "同名不同 id 必须落在两条独立记录上，而不是被当成同一人"
    );
    assert_ne!(
        first.agent_id(),
        second.agent_id(),
        "同名联系人各自的 agent 身份也必须独立"
    );
    assert_eq!(first.display_name(), second.display_name());

    let read_back_first = registry.get(&first.contact_id()).expect("第一条可读回");
    let read_back_second = registry.get(&second.contact_id()).expect("第二条可读回");
    assert_eq!(read_back_first.display_name(), "阿岚");
    assert_eq!(read_back_second.display_name(), "阿岚");
    assert_eq!(
        registry.list().len(),
        2,
        "同名不构成同一身份：列表里必须是两条记录"
    );
}

#[test]
fn 身份由后端生成_客户端无法指定() {
    let registry = InMemoryContactRegistry::new();
    let client_contact_id = Uuid::new_v4();
    let client_agent_id = Uuid::new_v4();
    let raw = format!(
        r#"{{"display_name":"客户端想指定身份","contact_id":"{client_contact_id}","agent_id":"{client_agent_id}","status":"ready","capabilities":{{"read":true,"write":true,"approval_required":false}},"revision":99,"persona_revision":42}}"#
    );

    let input: ContactIn = serde_json::from_str(&raw).expect("未知字段必须被忽略而不是报错");
    let contact = registry
        .create(input.into_request(NOW_MS).expect("入面转请求"))
        .expect("create");

    assert_ne!(
        contact.contact_id().as_uuid(),
        client_contact_id,
        "contact_id 必须由后端铸造，客户端指定的要被丢掉"
    );
    assert_ne!(
        contact.agent_id().as_uuid(),
        client_agent_id,
        "agent_id 必须由后端铸造，客户端指定的要被丢掉"
    );
    assert_eq!(
        contact.status(),
        ContactStatus::Draft,
        "status 由后端派生，客户端塞的 ready 不得被采信"
    );
    assert_eq!(
        contact.capabilities(),
        ContactStatus::Draft.default_capabilities(),
        "capabilities 由后端派生，客户端塞的能力不得被采信"
    );
    assert_eq!(contact.revision(), 0, "revision 由后端维护");
    assert_eq!(contact.persona_revision(), 0, "persona_revision 由后端维护");

    // 类型层面也钉死：入面根本没有这些字段。
    let contact_id: ContactId = contact.contact_id();
    let agent_id: ContactAgentId = contact.agent_id();
    assert_ne!(contact_id.as_uuid(), client_contact_id);
    assert_ne!(agent_id.as_uuid(), client_agent_id);
}

// ---------------------------------------------------------------------------
// 人设校验
// ---------------------------------------------------------------------------

#[test]
fn 人设字节上限_边界值接受_超一字节拒绝() {
    let exactly = "a".repeat(MAX_PERSONA_BYTES);
    assert_eq!(
        exactly.len(),
        MAX_PERSONA_BYTES,
        "边界样本必须恰好等于字节上限"
    );
    let persona = PersonaText::new(exactly.clone()).expect("恰好等于上限的人设必须被接受");
    assert_eq!(persona.byte_len(), MAX_PERSONA_BYTES);

    let over = "a".repeat(MAX_PERSONA_BYTES + 1);
    let error = PersonaText::new(over).expect_err("超一字节必须拒绝");
    match error {
        ContactError::InvalidPersona { reason } => {
            assert!(
                reason.contains(&MAX_PERSONA_BYTES.to_string()),
                "错误原因必须写明上限: {reason}"
            );
        }
        other => panic!("超限必须报 InvalidPersona, 实际: {other:?}"),
    }
}

#[test]
fn 人设上限计的是字节不是字符() {
    // 多字节语言: 字符数远未到上限, 字节数已超 ⇒ 必须拒。
    let text = "漢".repeat(MAX_PERSONA_BYTES / 3 + 1);
    assert!(
        text.chars().count() < MAX_PERSONA_BYTES,
        "样本字符数必须小于上限 (否则测不到字节口径)"
    );
    assert!(text.len() > MAX_PERSONA_BYTES, "样本字节数必须超上限");
    let error = PersonaText::new(text).expect_err("多字节文本按字节口径必须拒绝");
    assert!(
        matches!(error, ContactError::InvalidPersona { .. }),
        "必须报 InvalidPersona, 实际: {error:?}"
    );
}

#[test]
fn 人设控制字符被拒_普通换行放行() {
    // 被拒: BEL、ESC、U+2028 行分隔符、U+2029 段分隔符、DEL。
    for (label, text) in [
        ("BEL", "人设\u{7}"),
        ("ESC", "人设\u{1b}"),
        ("LS", "人设\u{2028}"),
        ("PS", "人设\u{2029}"),
        ("DEL", "人设\u{7f}"),
    ] {
        let error = PersonaText::new(text).expect_err("控制字符必须被拒");
        assert!(
            matches!(error, ContactError::InvalidPersona { .. }),
            "{label} 必须报 InvalidPersona, 实际: {error:?}"
        );
    }

    // 放行: 普通换行 (本批裁决: 人设是散文长文本, 注入槽是文本块)。
    let with_newline = PersonaText::new("第一行\n第二行").expect("普通换行必须放行");
    assert_eq!(with_newline.as_str(), "第一行\n第二行");
}

#[test]
fn 人设空文本被拒() {
    let error = PersonaText::new("   ").expect_err("纯空白人设必须被拒");
    assert!(
        matches!(error, ContactError::InvalidPersona { .. }),
        "空人设必须报 InvalidPersona, 实际: {error:?}"
    );
}

#[test]
fn 人设写入注册表时同样受校验() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    let over = "a".repeat(MAX_PERSONA_BYTES + 1);
    let request = ContactCreateRequest {
        display_name: "阿岚".to_string(),
        subtitle: None,
        avatar_ref: None,
        persona: None,
        memory_scope: ContractMemoryScope::None,
        now_ms: NOW_MS,
    };
    assert!(request.persona.is_none());
    // 直接把超限文本塞进入面 ⇒ 必须在入面校验阶段就被拒。
    let raw = format!(r#"{{"display_name":"阿岚","persona":"{over}"}}"#);
    let input: ContactIn = serde_json::from_str(&raw).expect("入面反序列化");
    let error = input
        .into_request(NOW_MS)
        .expect_err("超限人设必须在入面被拒");
    assert!(
        matches!(error, ContactError::InvalidPersona { .. }),
        "必须报 InvalidPersona, 实际: {error:?}"
    );
    assert_eq!(contact.persona(), None);
}

// ---------------------------------------------------------------------------
// 版本语义
// ---------------------------------------------------------------------------

#[test]
fn 人设变更时_persona_revision_恰好加一_记录版本同步加一() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    assert_eq!(contact.persona_revision(), 0);
    assert_eq!(contact.revision(), 0);

    let updated = registry
        .update_persona(
            &contact.contact_id(),
            contact.revision(),
            PersonaText::new("你是一名严谨的助理。").expect("人设合法"),
            LATER_MS,
        )
        .expect("第一次写入人设");
    assert_eq!(updated.persona_revision(), 1, "人设版本必须恰好 +1");
    assert_eq!(updated.revision(), 1, "记录版本必须同步 +1");
    assert_eq!(updated.updated_at_ms(), LATER_MS);

    let updated = registry
        .update_persona(
            &contact.contact_id(),
            updated.revision(),
            PersonaText::new("你是一名严谨且简洁的助理。").expect("人设合法"),
            LATER_MS + 1,
        )
        .expect("第二次写入人设");
    assert_eq!(updated.persona_revision(), 2, "人设版本必须再 +1");
    assert_eq!(updated.revision(), 2, "记录版本必须再 +1");
}

#[test]
fn 能力投影更新只动记录版本_不动人设版本() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    let updated = registry
        .update_capabilities(
            &contact.contact_id(),
            contact.revision(),
            ContactCapabilities {
                read: true,
                write: false,
                approval_required: true,
            },
            LATER_MS,
        )
        .expect("能力投影可写");
    assert_eq!(updated.revision(), 1, "能力投影写入也要 +1 记录版本");
    assert_eq!(
        updated.persona_revision(),
        0,
        "能力投影不是人设变更, 人设版本必须不动"
    );
}

// ---------------------------------------------------------------------------
// 乐观并发
// ---------------------------------------------------------------------------

#[test]
fn 过期版本号冲突_状态逐字段不变() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    let updated = registry
        .update_persona(
            &contact.contact_id(),
            contact.revision(),
            PersonaText::new("第一版人设").expect("人设合法"),
            LATER_MS,
        )
        .expect("第一次写入");
    let before: ContactOut = updated.to_out();

    let error = registry
        .update_persona(
            &contact.contact_id(),
            0,
            PersonaText::new("用过期版本号写入").expect("人设合法"),
            LATER_MS + 5_000,
        )
        .expect_err("过期版本号必须冲突");
    assert_eq!(
        error,
        ContactError::RevisionConflict {
            expected: 0,
            actual: 1
        },
        "冲突必须精确报出期望值与实际值"
    );

    let after: ContactOut = registry
        .get(&contact.contact_id())
        .expect("冲突后仍可读回")
        .to_out();
    assert_eq!(after, before, "冲突后记录必须逐字段不变 (含 updated_at_ms)");
}

#[test]
fn 未知联系人报_not_found() {
    let registry = InMemoryContactRegistry::new();
    let missing = ContactId::new();
    let error = registry
        .update_persona(
            &missing,
            0,
            PersonaText::new("人设").expect("人设合法"),
            NOW_MS,
        )
        .expect_err("未知联系人必须报错");
    assert_eq!(error, ContactError::NotFound(missing));
}

// ---------------------------------------------------------------------------
// 归档
// ---------------------------------------------------------------------------

#[test]
fn 归档是软删除_墓碑保留_不可复活() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    let archived = registry
        .archive(&contact.contact_id(), contact.revision(), LATER_MS)
        .expect("归档成功");

    assert_eq!(archived.status(), ContactStatus::Archived);
    assert_eq!(
        archived.revision(),
        contact.revision() + 1,
        "归档必须 +1 记录版本"
    );
    assert_eq!(archived.updated_at_ms(), LATER_MS);

    assert!(registry.list().is_empty(), "归档后默认列表必须不含它");
    assert_eq!(registry.list_all().len(), 1, "墓碑仍在全量列表里");

    let tombstone = registry.get(&contact.contact_id()).expect("归档后仍可取到");
    assert_eq!(tombstone.status(), ContactStatus::Archived);

    // 不可复活: 用**正确**的版本号再写, 也必须被拒 (归档优先于版本核对)。
    let error = registry
        .update_persona(
            &contact.contact_id(),
            archived.revision(),
            PersonaText::new("想复活").expect("人设合法"),
            LATER_MS + 1,
        )
        .expect_err("归档后不可复活");
    assert_eq!(
        error,
        ContactError::Archived,
        "归档后任何写入都必须报 Archived"
    );

    let error = registry
        .update_capabilities(
            &contact.contact_id(),
            archived.revision(),
            ContactCapabilities::default(),
            LATER_MS + 1,
        )
        .expect_err("归档后不可写能力投影");
    assert_eq!(error, ContactError::Archived);

    let error = registry
        .archive(&contact.contact_id(), archived.revision(), LATER_MS + 1)
        .expect_err("归档后不可重复归档");
    assert_eq!(error, ContactError::Archived);

    let tombstone = registry.get(&contact.contact_id()).expect("仍可取到墓碑");
    assert_eq!(
        tombstone.revision(),
        archived.revision(),
        "被拒的写入不得改动墓碑"
    );
}

// ---------------------------------------------------------------------------
// 持久化
// ---------------------------------------------------------------------------

#[test]
fn 落盘后重启读回逐字段一致() {
    let root = temp_root("persist");
    let (contact_id, expected) = {
        let registry = FileContactRegistry::open(&root).expect("打开落盘注册表");
        let contact = registry
            .create(ContactCreateRequest {
                display_name: "阿岚".to_string(),
                subtitle: Some("值班调度".to_string()),
                avatar_ref: Some("avatar://local/alan".to_string()),
                persona: None,
                memory_scope: ContractMemoryScope::Contact,
                now_ms: NOW_MS,
            })
            .expect("create");
        let updated = registry
            .update_persona(
                &contact.contact_id(),
                contact.revision(),
                PersonaText::new("你是一名严谨的助理。").expect("人设合法"),
                LATER_MS,
            )
            .expect("写入人设");
        (contact.contact_id(), updated.to_out())
    };

    // 重启: 换一个注册表实例读回。
    let reopened = FileContactRegistry::open(&root).expect("重启后打开");
    let restored = reopened.get(&contact_id).expect("重启后读回");
    assert_eq!(restored.to_out(), expected, "重启后必须逐字段一致");
    assert_eq!(
        restored.persona().map(PersonaText::as_str),
        Some("你是一名严谨的助理。")
    );
    assert_eq!(restored.memory_scope(), ContractMemoryScope::Contact);

    cleanup(&root);
}

#[test]
fn concurrent_contact_patches_compare_revision_inside_the_file_lock() {
    let root = temp_root("atomic-binding");
    let first = FileContactRegistry::open(&root).expect("first registry");
    let contact = first
        .create(ContactCreateRequest {
            display_name: "阿岚".into(),
            subtitle: None,
            avatar_ref: None,
            persona: None,
            memory_scope: ContractMemoryScope::None,
            now_ms: NOW_MS,
        })
        .expect("create");
    let stale = FileContactRegistry::open(&root).expect("second registry");
    let binding = ContactWorkspaceBinding {
        workspace_id: WorkspaceId::new(),
        revision: 3,
    };
    let updated = first
        .update(
            &contact.contact_id(),
            0,
            ContactPatch {
                persona: Some(PersonaText::new("新版人设").unwrap()),
                workspace_binding: Some(Some(binding)),
                ..ContactPatch::default()
            },
            LATER_MS,
        )
        .expect("combined edit");
    assert_eq!(updated.revision(), 1);
    assert_eq!(updated.persona_revision(), 1);
    assert_eq!(updated.workspace_binding(), Some(Some(binding)));
    assert_eq!(updated.workspace_binding_revision(), Some(1));
    assert!(matches!(
        stale.update(
            &contact.contact_id(),
            0,
            ContactPatch {
                workspace_binding: Some(None),
                ..ContactPatch::default()
            },
            LATER_MS
        ),
        Err(ContactError::RevisionConflict { actual: 1, .. })
    ));
    let reopened = FileContactRegistry::open(&root).expect("reopen");
    assert_eq!(reopened.get(&contact.contact_id()), Some(updated));
    cleanup(&root);
}

#[test]
fn 归档记录重启后仍是墓碑() {
    let root = temp_root("tombstone");
    {
        let registry = FileContactRegistry::open(&root).expect("打开落盘注册表");
        let contact = registry
            .create(ContactCreateRequest {
                display_name: "阿岚".to_string(),
                subtitle: None,
                avatar_ref: None,
                persona: None,
                memory_scope: ContractMemoryScope::None,
                now_ms: NOW_MS,
            })
            .expect("create");
        registry
            .archive(&contact.contact_id(), contact.revision(), LATER_MS)
            .expect("归档");
    }

    let reopened = FileContactRegistry::open(&root).expect("重启后打开");
    let tombstone = reopened.get(&ContactId::new());
    assert!(tombstone.is_none(), "随机 id 不该命中");
    let all = reopened.list_all();
    assert_eq!(all.len(), 1, "重启后墓碑必须仍在");
    assert_eq!(
        all[0].status(),
        ContactStatus::Archived,
        "重启后墓碑状态不变"
    );
    assert!(reopened.list().is_empty(), "重启后默认列表仍不含墓碑");

    cleanup(&root);
}

#[test]
fn 落盘文件落在约定路径上() {
    let root = temp_root("layout");
    {
        let registry = FileContactRegistry::open(&root).expect("打开落盘注册表");
        let contact = registry
            .create(ContactCreateRequest {
                display_name: "阿岚".to_string(),
                subtitle: None,
                avatar_ref: None,
                persona: None,
                memory_scope: ContractMemoryScope::None,
                now_ms: NOW_MS,
            })
            .expect("create");
        let file = root
            .join("agents")
            .join("contacts")
            .join(contact.contact_id().to_string())
            .join("contact.json");
        assert!(
            file.is_file(),
            "落盘路径必须是 agents/contacts/<id>/contact.json"
        );
    }
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// 线格式
// ---------------------------------------------------------------------------

#[test]
fn 线格式_读面无内部字段_入面不采信客户端状态() {
    let registry = InMemoryContactRegistry::new();
    let contact = create_contact(&registry, "阿岚");
    let out = contact.to_out();

    let value = serde_json::to_value(&out).expect("序列化");
    let object = value.as_object().expect("读面是对象");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "agent_id",
            "avatar_ref",
            "capabilities",
            "contact_id",
            "display_name",
            "memory_scope",
            "persona",
            "persona_revision",
            "revision",
            "status",
            "subtitle",
            "updated_at_ms",
        ],
        "读面字段集必须固定, 不得夹带内部字段"
    );

    // 直接序列化领域类型 (走 impl Serialize for Contact) 也必须是同一份读面。
    let serialized = serde_json::to_value(&contact).expect("序列化领域类型");
    assert_eq!(serialized, value, "领域类型序列化必须落在读面上");

    // 入面塞 status / capabilities / revision ⇒ 不采信。
    let raw = r#"{"display_name":"阿岚","status":"archived","capabilities":{"read":true,"write":true,"approval_required":false},"revision":77,"persona_revision":88}"#;
    let input: ContactIn = serde_json::from_str(raw).expect("未知字段必须被忽略");
    let request = input.into_request(NOW_MS).expect("入面转请求");
    let created = registry.create(request).expect("create");
    assert_eq!(
        created.status(),
        ContactStatus::Draft,
        "客户端 status 不采信"
    );
    assert_eq!(created.revision(), 0, "客户端 revision 不采信");
    assert_eq!(
        created.persona_revision(),
        0,
        "客户端 persona_revision 不采信"
    );
    assert_eq!(
        created.capabilities(),
        ContactStatus::Draft.default_capabilities(),
        "客户端 capabilities 不采信"
    );
}
