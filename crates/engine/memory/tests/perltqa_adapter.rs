//! 外部卷适配器（PerLTQA → 本仓考卷格式）的测试。
//!
//! 口径：转换是**纯字段映射**，不下载数据、不跑模型 ⇒ 测试用**合成 JSON**即可覆盖全部口径。

use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::perltqa_adapter::{convert, to_golden_set, AdapterConfig, PerLtqaItem};

fn sample_item() -> serde_json::Value {
    serde_json::json!({
        "question": "林小姐的工作单位是什么？",
        "reference memory": "林小姐在清华大学担任副教授，研究方向是自然语言处理。",
        "memory anchor": "清华大学",
        "answer": "清华大学",
        "character": "lin"
    })
}

#[test]
fn converts_question_and_reference_memory_into_a_case() {
    let dataset = convert(
        &serde_json::json!([sample_item()]),
        AdapterConfig::default(),
    );
    assert_eq!(dataset.skipped, 0);
    assert_eq!(dataset.cases.len(), 1);
    let case = &dataset.cases[0];
    assert_eq!(case.query, "林小姐的工作单位是什么？");
    assert_eq!(
        case.expect_ids.len(),
        1,
        "一条 reference memory ⇒ 一个 expect_id"
    );
    assert_eq!(
        case.expect_fragments,
        vec!["清华大学".to_string()],
        "片段优先取 memory anchor"
    );
    assert!(case.tags.contains(&"perltqa".to_string()));
    assert!(case.tags.contains(&"character:lin".to_string()));
}

#[test]
fn record_ids_are_content_fingerprints_and_reproducible() {
    // **同一份数据转换两次必得同一批 id**（可复现、可跨卷对照）。
    let json = serde_json::json!([sample_item()]);
    let first = convert(&json, AdapterConfig::default());
    let second = convert(&json, AdapterConfig::default());
    assert_eq!(first.cases, second.cases);
    assert!(
        first.cases[0].expect_ids[0].starts_with("sha256:"),
        "记录 id 应是内容指纹: {:?}",
        first.cases[0].expect_ids[0]
    );
    assert_eq!(first.cases[0].expect_ids[0].len(), "sha256:".len() + 12);
}

#[test]
fn falls_back_to_first_sentence_when_no_anchor() {
    let json = serde_json::json!([{
        "question": "他住在哪里？",
        "reference memory": "他住在杭州西湖区。第一句之外的第二句。",
        "answer": "杭州"
    }]);
    let dataset = convert(&json, AdapterConfig::default());
    let fragments = &dataset.cases[0].expect_fragments;
    assert_eq!(fragments.len(), 1);
    assert!(fragments[0].starts_with("他住在杭州西湖区。"));
    assert!(
        !fragments[0].contains("第二句"),
        "只取第一句: {:?}",
        fragments[0]
    );
}

#[test]
fn accepts_the_data_object_wrapper_shape() {
    // PerLTQA 顶层可能是 `{ "data": [...] }` ⇒ 两种形状都要支持。
    let wrapped = serde_json::json!({ "data": [sample_item()] });
    let dataset = convert(&wrapped, AdapterConfig::default());
    assert_eq!(dataset.cases.len(), 1);
}

#[test]
fn accepts_alternative_field_names() {
    // 字段名在不同版本可能不同 ⇒ 多名字兼容。
    let json = serde_json::json!([{
        "query": "问题？",
        "reference_memory": "记忆内容。",
        "anchor": "锚点"
    }]);
    let dataset = convert(&json, AdapterConfig::default());
    assert_eq!(dataset.cases.len(), 1);
    assert_eq!(dataset.cases[0].query, "问题？");
    assert_eq!(dataset.cases[0].expect_fragments, vec!["锚点".to_string()]);
}

#[test]
fn skips_items_without_question_or_memory() {
    let json = serde_json::json!([
        { "reference memory": "只有记忆没有问。" },
        { "question": "只有问题没有记忆。" },
        sample_item(),
    ]);
    let dataset = convert(&json, AdapterConfig::default());
    assert_eq!(dataset.skipped, 2, "缺字段的条目要如实计数");
    assert_eq!(dataset.cases.len(), 1);
}

#[test]
fn truncates_fragments_on_character_boundaries() {
    let long = "内".repeat(300);
    let json = serde_json::json!([{
        "question": "问题？",
        "reference memory": "记忆。",
        "memory anchor": long
    }]);
    let dataset = convert(
        &json,
        AdapterConfig {
            max_fragment_chars: 10,
            ..AdapterConfig::default()
        },
    );
    let fragment = &dataset.cases[0].expect_fragments[0];
    assert_eq!(fragment.chars().count(), 10, "按字符截断");
    assert!(fragment.chars().all(|ch| ch == '内'), "不得切坏 UTF-8");
}

#[test]
fn output_order_is_deterministic_and_sorted_by_id() {
    let json = serde_json::json!([
        { "question": "问题 B？", "reference memory": "记忆 B。" },
        { "question": "问题 A？", "reference memory": "记忆 A。" },
    ]);
    let dataset = convert(&json, AdapterConfig::default());
    let ids: Vec<&str> = dataset.cases.iter().map(|case| case.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "按 id 排序 ⇒ 确定性");
}

#[test]
fn converts_into_a_golden_set_with_the_given_version() {
    let dataset = convert(
        &serde_json::json!([sample_item()]),
        AdapterConfig::default(),
    );
    let set = to_golden_set(dataset, "perltqa-test-v1");
    assert_eq!(set.version, "perltqa-test-v1");
    assert_eq!(set.len(), 1);
    assert!(!set.is_empty());
}

#[test]
fn item_accessors_read_the_documented_fields() {
    let item = PerLtqaItem::new(sample_item());
    assert_eq!(item.question(), Some("林小姐的工作单位是什么？"));
    assert_eq!(item.character(), Some("lin"));
    assert_eq!(item.memory_anchors(), vec!["清华大学".to_string()]);
    assert_eq!(item.reference_memories().len(), 1);
}

#[test]
fn empty_input_yields_empty_dataset_without_error() {
    let dataset = convert(&serde_json::json!([]), AdapterConfig::default());
    assert!(dataset.cases.is_empty());
    assert_eq!(dataset.skipped, 0);
}

#[test]
fn resulting_cases_are_loadable_by_the_golden_set_reader() {
    // 转换产物必须能被考卷读取器读回来（**闭环**：适配器不是孤岛）。
    let dataset = convert(
        &serde_json::json!([sample_item()]),
        AdapterConfig::default(),
    );
    let set = to_golden_set(dataset, "perltqa-test-v1");
    let jsonl = set
        .cases
        .iter()
        .map(|case| serde_json::to_string(case).expect("serialize case"))
        .collect::<Vec<_>>()
        .join("\n");
    let loaded = GoldenSet::load_jsonl_with_version(&jsonl, "perltqa-test-v1")
        .expect("考卷读取器应能读回转换产物");
    assert_eq!(loaded.len(), set.len());
    assert_eq!(loaded.cases[0].expect_ids, set.cases[0].expect_ids);
    assert_eq!(loaded.cases[0].query, set.cases[0].query);
}
