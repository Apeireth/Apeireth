//! 沉淀管线 (`tool.skill_distill` 的管线核) 回归: 内容输入 → 结构化技能卡
//! (触发词归一) → 入技能库; 坏卡拒 / 来源二选一 / 只读通道 / 创建与版本演进。

use apeireth_tools_canonical::skill::{
    distill_into, distill_request_refusal, skill_distill_summary, SkillCardDraft,
    SkillDistillParams, SkillLibrary, SkillLibraryError, SkillSource, SkillSourceKind,
    SKILL_SOURCE_INLINE_REF, SKILL_SOURCE_MAX_BYTES,
};

fn draft(name: &str, triggers: &[&str]) -> SkillCardDraft {
    SkillCardDraft {
        name: name.into(),
        summary: "定积分换元后积分限与微分元同步替换".into(),
        triggers: triggers.iter().map(|t| (*t).to_string()).collect(),
        body: "步骤: ① 令 x=φ(t); ② dx=φ'(t)dt; ③ 换积分限。".into(),
        source: SkillSource {
            kind: SkillSourceKind::Document,
            title: "换元法讲义".into(),
        },
    }
}

fn params(card: SkillCardDraft, text: Option<&str>, path: Option<&str>) -> SkillDistillParams {
    SkillDistillParams {
        card,
        text: text.map(str::to_string),
        path: path.map(str::to_string),
        update: false,
    }
}

fn temp_workspace(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-skill-distill-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建测试目录");
    dir
}

/// 沉淀管线主路: 内容输入 (文本) + 模型侧草稿 → 结构化技能卡 (触发词归一)
/// 落技能库, 报告如实回显来源与触发词。
#[test]
fn text_input_distills_a_structured_card_with_normalized_triggers() {
    let root = temp_workspace("text");
    let library = SkillLibrary::under_data_dir(&root);
    let call = params(
        draft("换元法", &[" 换元 ", "dx", "换元", ""]),
        Some("换元法是换元后要同步替换微分元与积分限……"),
        None,
    );

    let report = distill_into(&library, &root, &call, 1_000).unwrap();
    assert_eq!(report.status, "distilled");
    assert_eq!(report.name, "换元法");
    assert_eq!(report.revision, 1);
    assert_eq!(report.triggers, vec!["换元", "dx"], "触发词归一后入卡");
    assert_eq!(report.source_ref, SKILL_SOURCE_INLINE_REF);
    assert!(report.source_chars > 0);
    assert_eq!(report.archived_count, 0);

    let card = library
        .find_by_name("换元法")
        .expect("结构化技能卡必须入库")
        .body;
    assert_eq!(card.triggers, vec!["换元", "dx"]);
    assert!(card.body.contains("dx=φ'(t)dt"));
    let _ = std::fs::remove_dir_all(&root);
}

/// 文件路径走只读通道: 只读采集来源 (来源文件字节不变), 报告引用真实路径。
#[test]
fn file_path_input_is_read_through_the_read_only_channel() {
    let root = temp_workspace("path");
    let library = SkillLibrary::under_data_dir(&root);
    let source_rel = "notes/book-chapter-3.md";
    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::write(root.join(source_rel), "书摘: 分部积分的选元口诀……").unwrap();
    let before = std::fs::read(root.join(source_rel)).unwrap();

    let call = params(draft("分部积分", &["分部积分"]), None, Some(source_rel));
    let report = distill_into(&library, &root, &call, 1_000).unwrap();
    assert_eq!(report.source_ref, source_rel);
    assert_eq!(
        report.source_chars,
        "书摘: 分部积分的选元口诀……".chars().count()
    );

    let after = std::fs::read(root.join(source_rel)).unwrap();
    assert_eq!(before, after, "只读通道不得改写来源文件");
    let _ = std::fs::remove_dir_all(&root);
}

/// 只读通道限域: 绝对路径 / `..` 逃逸拒绝即帧; 敏感面 (凭据面) 走共享的
/// 凭据面拒帧 (与读类工具同一安全契约帧)。
#[test]
fn source_path_escapes_and_sensitive_surfaces_are_refused() {
    let root = temp_workspace("escape");
    let library = SkillLibrary::under_data_dir(&root);

    for (label, path) in [
        ("绝对路径", "C:\\Windows\\notes.md"),
        ("父级逃逸", "../outside.md"),
    ] {
        let call = params(draft("越界卡", &["越界"]), None, Some(path));
        let err = distill_into(&library, &root, &call, 1_000).unwrap_err();
        assert!(err.starts_with("拒绝沉淀"), "{label} 必须拒绝即帧: {err}");
    }

    let call = params(draft("越界卡", &["越界"]), None, Some(".env"));
    let err = distill_into(&library, &root, &call, 1_000).unwrap_err();
    assert!(
        err.contains("凭据面") && err.contains("protected"),
        "敏感面必须走共享凭据面拒帧: {err}"
    );

    assert!(
        library.scan().cards.is_empty(),
        "越界请求零落盘: {:?}",
        library.scan()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 内容输入二选一: 两个都给或两个都不给都拒绝即帧。
#[test]
fn exactly_one_content_input_is_required() {
    let root = temp_workspace("source-choice");
    let library = SkillLibrary::under_data_dir(&root);

    let both = params(draft("双源卡", &["双源"]), Some("正文"), Some("a.md"));
    let err = distill_into(&library, &root, &both, 1_000).unwrap_err();
    assert!(err.contains("只能给一个"), "{err}");

    let neither = params(draft("无源卡", &["无源"]), None, None);
    let err = distill_into(&library, &root, &neither, 1_000).unwrap_err();
    assert!(err.contains("必须恰给一个"), "{err}");

    let oversized = params(
        draft("超限卡", &["超限"]),
        Some(&"字".repeat(SKILL_SOURCE_MAX_BYTES + 1)),
        None,
    );
    let err = distill_into(&library, &root, &oversized, 1_000).unwrap_err();
    assert!(err.contains("超限"), "{err}");
    assert!(library.scan().cards.is_empty(), "拒绝即帧, 零落盘");
    let _ = std::fs::remove_dir_all(&root);
}

/// 坏卡拒: 空触发词 / 空正文 / 空卡名的草稿在管线里拒绝即帧, 技能库零痕迹。
#[test]
fn bad_card_drafts_are_refused_before_any_landing() {
    let root = temp_workspace("bad-card");
    let library = SkillLibrary::under_data_dir(&root);

    let mut no_triggers = draft("空触发词卡", &["  "]);
    no_triggers.triggers = vec!["   ".into()];
    let err = distill_into(
        &library,
        &root,
        &params(no_triggers, Some("正文"), None),
        1_000,
    )
    .unwrap_err();
    assert!(err.contains("触发词"), "{err}");

    let mut empty_body = draft("空正文卡", &["空正文"]);
    empty_body.body = String::new();
    let err = distill_into(
        &library,
        &root,
        &params(empty_body, Some("正文"), None),
        1_000,
    )
    .unwrap_err();
    assert!(err.contains("正文"), "{err}");

    assert!(library.scan().cards.is_empty(), "坏卡零落盘");
    assert!(!root.join("skills/audit.jsonl").exists(), "坏卡零审计写");
    let _ = std::fs::remove_dir_all(&root);
}

/// 创建与版本演进两条路: 创建重复名拒绝; `update: true` 同名演进留旧版审计。
#[test]
fn create_refuses_duplicates_and_update_mode_evolves_with_audit() {
    let root = temp_workspace("intake");
    let library = SkillLibrary::under_data_dir(&root);

    distill_into(
        &library,
        &root,
        &params(draft("换元法", &["换元"]), Some("第一版来源"), None),
        1_000,
    )
    .unwrap();

    let duplicate = params(draft("换元法", &["换元"]), Some("再来一版"), None);
    let err = distill_into(&library, &root, &duplicate, 2_000).unwrap_err();
    assert!(err.contains("重复名"), "{err}");

    let mut evolve = params(
        draft("换元法", &["换元", "积分限"]),
        Some("第二版来源"),
        None,
    );
    evolve.update = true;
    let report = distill_into(&library, &root, &evolve, 2_000).unwrap();
    assert_eq!(report.status, "evolved");
    assert_eq!(report.revision, 2);
    assert_eq!(report.archived_count, 1);

    let archive = library.revision_archive("换元法").unwrap();
    assert_eq!(archive.len(), 1, "旧版必须留审计");
    assert_eq!(archive[0].revision, 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// 审批卡载荷 = 技能卡摘要 (创建/演进模式 + 来源 + 触发词数); 坏请求拒绝即帧
/// (错误即帧, 不进审批)。
#[test]
fn approval_payload_is_the_card_summary_and_bad_requests_stay_out() {
    let good = params(draft("换元法", &["换元", "dx"]), Some("来源文本"), None);
    let summary = skill_distill_summary(&good).unwrap();
    assert!(summary.contains("换元法"), "{summary}");
    assert!(summary.contains("document·换元法讲义"), "{summary}");
    assert!(summary.contains("触发词 2 个"), "{summary}");
    assert!(summary.contains("创建 (重复名拒绝)"), "{summary}");

    let mut evolve = good.clone();
    evolve.update = true;
    let summary = skill_distill_summary(&evolve).unwrap();
    assert!(summary.contains("版本演进"), "{summary}");

    let bad = params(draft("坏卡", &[]), Some("来源"), None);
    assert!(skill_distill_summary(&bad).is_err(), "坏请求不得进审批");
}

/// 请求校验器与管线同口径 (Guard 与工具共用一份判定)。
#[test]
fn request_refusal_is_shared_by_guard_and_pipeline() {
    let call = params(draft("换元法", &["换元"]), None, None);
    let refusal = distill_request_refusal(&call).expect("缺来源必须拒");
    assert!(refusal.contains("内容输入"));

    let ok = params(draft("换元法", &["换元"]), Some("正文"), None);
    assert!(distill_request_refusal(&ok).is_none());

    // 版本演进找不到旧版 = 库层拒绝 (fail-closed)。
    let root = temp_workspace("missing");
    let library = SkillLibrary::under_data_dir(&root);
    let mut evolve = ok;
    evolve.update = true;
    let err = distill_into(&library, &root, &evolve, 1_000).unwrap_err();
    assert!(err.contains("没有同名卡"), "{err}");
    let _ = std::fs::remove_dir_all(&root);
}

/// 库层错误的显示文本是结构化拒绝 (错误即帧, 不静默降级)。
#[test]
fn library_errors_render_as_structured_refusals() {
    let error = SkillLibraryError::DuplicateName {
        name: "换元法".into(),
        existing_id: "skill-1".into(),
    };
    let text = error.to_string();
    assert!(text.contains("重复名拒绝"), "{text}");
    assert!(text.contains("update"), "{text}");
}
