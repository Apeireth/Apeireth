//! 技能卡模型的落盘面回归 (I1): 落盘 / 拒开 / 重复名 / 版本演进审计。
//!
//! 每卡一 StoredDoc (`<id>.json`, 信封身份 = `skill-card:<id>`), 拒开语义
//! 原样沿用: 坏 JSON / 串档 / 版本不符一律拒绝打开并报结构化错误, 不静默
//! 迁移、不回退默认。

use apeireth_tools_canonical::skill::{
    SkillCard, SkillCardDraft, SkillLibrary, SkillLibraryError, SkillSource, SkillSourceKind,
    SKILL_AUDIT_FILE, SKILL_CARD_DOC_VERSION,
};

fn draft(name: &str, body: &str) -> SkillCardDraft {
    SkillCardDraft {
        name: name.into(),
        summary: "换元后微分元同步替换".into(),
        triggers: vec!["换元".into(), "dx".into()],
        body: body.into(),
        source: SkillSource {
            kind: SkillSourceKind::Book,
            title: "数学分析讲义".into(),
        },
    }
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-skill-card-model-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 落盘: 每卡一 StoredDoc 文件, 信封身份/版本戳正确, 往返逐字段等价。
#[test]
fn one_card_lands_as_one_stored_doc_that_reopens_exactly() {
    let root = temp_root("landing");
    let library = SkillLibrary::new(&root);
    let intake = library.create(&draft("换元法", "第一版"), 1_000).unwrap();

    let path = library.card_path(&intake.card.body.id);
    assert!(path.is_file(), "每卡一 StoredDoc 文件: {}", path.display());
    let envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        envelope["name"],
        format!("skill-card:{}", intake.card.body.id),
        "信封身份带卡 id (串档打开即拒)"
    );
    assert_eq!(envelope["version"], SKILL_CARD_DOC_VERSION);

    let reopened = library.open_card(&intake.card.body.id).unwrap();
    assert_eq!(reopened, intake.card, "往返必须逐字段等价");
    assert_eq!(reopened.body.created_at, 1_000);
    assert_eq!(reopened.body.triggers, vec!["换元", "dx"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// 拒开: 坏 JSON 卡文件拒绝打开 (结构化错误, 不「读坏用默认」), 且不砖化
/// 整个技能库 —— 扫描如实列拒, 好卡照读。
#[test]
fn a_corrupt_card_file_is_refused_not_defaulted_and_the_library_survives() {
    let root = temp_root("corrupt");
    let library = SkillLibrary::new(&root);
    let good = library.create(&draft("好卡", "正文"), 1_000).unwrap();

    let corrupt_path = library.card_path("skill-000000000000bad");
    std::fs::write(&corrupt_path, "{ this is not json").unwrap();

    let err = library.open_card("skill-000000000000bad").unwrap_err();
    assert!(
        err.to_string().contains("拒开") || err.to_string().contains("畸形"),
        "坏 JSON 必须拒开: {err}"
    );

    let scan = library.scan();
    assert_eq!(scan.cards.len(), 1, "好卡照读: {:?}", scan.cards);
    assert_eq!(scan.cards[0].body.id, good.card.body.id);
    assert_eq!(scan.refused.len(), 1, "坏卡必须如实列拒");
    assert!(scan.refused[0].path.ends_with("skill-000000000000bad.json"));
    let _ = std::fs::remove_dir_all(&root);
}

/// 拒开: 版本戳不符 / 串档身份的信封一律拒绝打开。
#[test]
fn version_mismatch_and_cross_card_identity_are_refused() {
    let root = temp_root("tamper");
    let library = SkillLibrary::new(&root);
    let intake = library.create(&draft("换元法", "正文"), 1_000).unwrap();
    let path = library.card_path(&intake.card.body.id);

    // 未来版本戳 (读端只认版本 1)。
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    envelope["version"] = serde_json::json!(99);
    envelope["compatible_versions"] = serde_json::json!([99]);
    std::fs::write(&path, serde_json::to_vec_pretty(&envelope).unwrap()).unwrap();
    let err = library.open_card(&intake.card.body.id).unwrap_err();
    assert!(
        err.to_string().contains("未知未来版本"),
        "版本不符必须拒开: {err}"
    );

    // 串档身份 (文件名与信封 id 不一致)。
    let body: SkillCard = serde_json::from_value(envelope["body"].clone()).unwrap();
    let foreign = serde_json::json!({
        "name": "skill-card:skill-other",
        "version": 1,
        "compatible_versions": [1],
        "body": body,
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&foreign).unwrap()).unwrap();
    let err = library.open_card(&intake.card.body.id).unwrap_err();
    assert!(err.to_string().contains("身份不符"), "串档必须拒开: {err}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 重复名拒绝: 同名 (含大小写/空白变体) 创建即拒, 既有卡一字不改。
#[test]
fn duplicate_names_are_refused_and_never_touch_the_existing_card() {
    let root = temp_root("duplicate");
    let library = SkillLibrary::new(&root);
    let first = library
        .create(&draft("Rust 惯用法", "旧正文"), 1_000)
        .unwrap();

    for variant in ["Rust 惯用法", "  rust 惯用法 ", "RUST 惯用法"] {
        let err = library
            .create(&draft(variant, "新正文"), 2_000)
            .unwrap_err();
        assert!(
            matches!(err, SkillLibraryError::DuplicateName { .. }),
            "{variant} 必须判重复名: {err}"
        );
    }
    let untouched = library.open_card(&first.card.body.id).unwrap();
    assert_eq!(untouched.body.body, "旧正文");
    assert_eq!(untouched, first.card);
    let _ = std::fs::remove_dir_all(&root);
}

/// 版本演进审计: 同名更新 = 旧版整卡留 `audit.jsonl` + 新版原子落盘,
/// id / created_at 保持不变, 版本序号逐次递增。
#[test]
fn version_evolution_archives_the_old_card_and_keeps_identity() {
    let root = temp_root("evolution");
    let library = SkillLibrary::new(&root);
    let first = library.create(&draft("换元法", "第一版"), 1_000).unwrap();
    let second = library.update(&draft("换元法", "第二版"), 2_000).unwrap();
    let third = library.update(&draft("换元法", "第三版"), 3_000).unwrap();

    assert_eq!(second.revision, 2);
    assert_eq!(third.revision, 3);
    assert_eq!(third.card.body.id, first.card.body.id, "id 演进不变");
    assert_eq!(third.card.body.created_at, 1_000, "created_at 演进不变");
    assert_eq!(third.card.body.body, "第三版");

    let audit_path = library.audit_path();
    assert!(audit_path.ends_with(SKILL_AUDIT_FILE));
    let archive = library.revision_archive("换元法").unwrap();
    assert_eq!(archive.len(), 2, "两次演进必须留两条旧版审计");
    assert_eq!(archive[0].revision, 1);
    assert_eq!(archive[0].archived.body, "第一版");
    assert_eq!(archive[0].archived_at, 2_000);
    assert_eq!(archive[1].revision, 2);
    assert_eq!(archive[1].archived.body, "第二版");
    assert_eq!(archive[1].archived_at, 3_000);
    assert_eq!(archive[1].archived.id, first.card.body.id);
    let _ = std::fs::remove_dir_all(&root);
}

/// 版本演进 fail-closed: 没有旧版就不演进 (创建与演进两条路, 不做静默 upsert)。
#[test]
fn version_evolution_requires_an_existing_card() {
    let root = temp_root("evolution-missing");
    let library = SkillLibrary::new(&root);
    let err = library
        .update(&draft("还不存在", "正文"), 1_000)
        .unwrap_err();
    assert!(
        matches!(err, SkillLibraryError::MissingName { .. }),
        "{err}"
    );
    assert!(!library.card_path("skill-x").exists(), "拒绝即帧, 零落盘");
    let _ = std::fs::remove_dir_all(&root);
}
