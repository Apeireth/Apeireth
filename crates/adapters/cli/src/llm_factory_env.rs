//! LLM 工厂与 council 旋钮的 env 解析（从 `lib.rs` 拆出）。
//!
//! 为什么单独成模块：`lib.rs` 有尺寸棘轮（只减不增），而这两段是**独立的
//! 环境解析职责**（「从 env 找工厂」/「从 env 读 council 旋钮」），与命令分派
//! 无关。拆分是真拆：搬走一整块职责，不是为凑行数的位移。

use std::sync::Arc;

/// 从 env 找真 LLM 工厂 (openai-compatible 优先, MiniMax 回退) + 首个模型 id。
/// 都未配 → Err (0 装: 不造"调用时才失败"的假可用)。
/// 未配置任何 LLM 工厂时的提示（**必须可行动**：说清缺什么、怎么配）。
///
/// 此前这里只回一行英文 `no llm factory configured` —— 用户不知道要设哪个 env
/// （真机实测 2026-10-25：`apeireth council <主题>` 直接吐那一行）。
/// 与 `subagent` 同一口径。
pub const LLM_FACTORY_MISSING_HINT: &str = "council 需要 LLM 工厂: 请设 \
     APEIRETH_OPENAI_MODELS (+URL/KEY) 或 APEIRETH_API_KEY (MiniMax); \
     也可设 APEIRETH_COUNCIL_ADVISORS 调整顾问数";

pub(crate) fn llm_factory_from_env(
) -> Result<(Arc<dyn apeireth_plugin::llm_factory::LlmFactory>, String), String> {
    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) = apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env_with_resolver(
            crate::llm_credential_resolver(),
        ) {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "deepseek-v4-flash".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env_with_resolver(
                crate::llm_credential_resolver(),
            )
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "MiniMax-M3".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    Err(LLM_FACTORY_MISSING_HINT.to_string())
}

/// **council 旋钮** (2026-10-10 拍板, 默认 3): `APEIRETH_COUNCIL_ADVISORS=N` (1-7) =
/// 裁决顾问数 (规范序取前 N: Safety/Performance/Philosophy/History/Strategy/
/// Ethics/Legal, Safety 恒首位); `APEIRETH_COUNCIL_TIMEOUT_MS` = 单顾问超时
/// (默认 30000, 台账 #46 实测: 思考型模型 reasoning 余量下 10s 不够)。
pub(crate) fn council_config_from_env() -> apeireth_orchestration::CouncilConfig {
    use apeireth_orchestration::CouncilConfig;
    use std::time::Duration;
    let max_advisors = std::env::var("APEIRETH_COUNCIL_ADVISORS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map(|n| n.clamp(1, 7))
        .unwrap_or(3);
    let per_advisor_timeout = std::env::var("APEIRETH_COUNCIL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(30_000));
    CouncilConfig {
        max_advisors,
        per_advisor_timeout,
        ..CouncilConfig::default()
    }
}

#[cfg(test)]
mod hint_tests {
    use super::LLM_FACTORY_MISSING_HINT;

    /// 「缺工厂」的提示必须**可行动**：点名要配的 env。
    ///
    /// 守卫理由：这里曾经只有一行英文（`no llm factory configured`），
    /// 用户照它无法自救。改成纯文本常量后可以脱 env 直接钉住。
    #[test]
    fn missing_factory_hint_names_the_envs_to_set() {
        for env_name in ["APEIRETH_OPENAI_MODELS", "APEIRETH_API_KEY"] {
            assert!(
                LLM_FACTORY_MISSING_HINT.contains(env_name),
                "提示必须点名 {env_name}（可行动）: {LLM_FACTORY_MISSING_HINT}"
            );
        }
    }
}
