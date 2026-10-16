//! H-P1 口径唯一: 内置基线表的「已知模型」集合必须与提供商默认模型表声明的
//! 集合逐个一致 (大小写不敏感) —— 基线不另立名单, 也不漏行。
//!
//! 提供商默认模型表是接口事实 (id 与族能力声明), 画像基线镜像它们; 两边任何
//! 一侧漂移都由本测试钉死 (改模型表必须同步改基线, 或反之)。

use apeireth_core::model_profile::{
    normalize_model_id, ModelProfileTable, BASELINE_ANTHROPIC_MODELS, BASELINE_MINIMAX_MODELS,
    BASELINE_OPENAI_COMPATIBLE_MODELS,
};

#[test]
fn the_baseline_covers_exactly_the_provider_declared_default_models() {
    let baseline = ModelProfileTable::baseline();
    let mut baseline_ids: Vec<String> = baseline
        .profiles()
        .iter()
        .map(|profile| normalize_model_id(&profile.model_id))
        .collect();
    baseline_ids.sort();
    baseline_ids.dedup();

    let mut declared: Vec<String> = [
        apeireth_provider::canonical_openai_compatible::DEFAULT_MODELS,
        apeireth_provider::canonical_minimax::DEFAULT_MODELS,
        apeireth_provider::canonical_anthropic::DEFAULT_MODELS,
    ]
    .concat()
    .iter()
    .map(|id| normalize_model_id(id))
    .collect();
    declared.sort();
    declared.dedup();

    // 族常量镜像自提供商默认模型表: 缺一即口径漂移。
    let mut mirrored: Vec<String> = [
        BASELINE_OPENAI_COMPATIBLE_MODELS,
        BASELINE_MINIMAX_MODELS,
        BASELINE_ANTHROPIC_MODELS,
    ]
    .concat()
    .iter()
    .map(|id| normalize_model_id(id))
    .collect();
    mirrored.sort();
    mirrored.dedup();

    assert_eq!(
        mirrored, declared,
        "画像族常量必须与提供商默认模型表逐 id 一致 (口径唯一)"
    );
    assert_eq!(
        baseline_ids, declared,
        "内置基线必须命中且只命中提供商声明的已知模型 (口径唯一)"
    );
}

#[test]
fn every_baseline_row_carries_conservative_facts_and_no_prices() {
    // 基线行 = 保守下界事实 + 单价未提供; 未知模型不在表里 (显式 unknown 另测)。
    for profile in ModelProfileTable::baseline().profiles() {
        assert!(
            profile.context_window.is_some() && profile.max_output.is_some(),
            "{}: 基线行必须有保守下界窗口/输出",
            profile.model_id
        );
        assert!(
            profile.supports_tools.is_some() && profile.supports_streaming.is_some(),
            "{}: 基线行必须有族能力布尔",
            profile.model_id
        );
        assert!(
            profile.input_price_micros.is_none()
                && profile.output_price_micros.is_none()
                && profile.cache_read_price_micros.is_none(),
            "{}: 基线单价一律未提供 (绝不编价)",
            profile.model_id
        );
    }
}
