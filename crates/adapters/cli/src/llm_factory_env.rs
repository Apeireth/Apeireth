//! LLM 工厂、council 旋钮与**内部执行强制复核**的 env 解析（从 `lib.rs` 拆出）。
//!
//! 为什么单独成模块：`lib.rs` 有尺寸棘轮（只减不增），而这几段是**独立的
//! 环境解析职责**（「从 env 找工厂」/「从 env 读 council 旋钮」/「从 env 选
//! 监督复核的真实 provider+model」），与命令分派无关。拆分是真拆：搬走一整块
//! 职责，不是为凑行数的位移。

use std::path::Path;
use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::agents::AgentBudget;
use apeireth_plugin::system_one::SystemOneSupervisor;
use apeireth_plugin::CredentialResolver;
use apeireth_protocol::canonical::ModelDescriptor;
use apeireth_runtime::canonical::system_one::{SystemOneGate, SystemOneLimits};
use apeireth_runtime::canonical::{ProviderRouter, Runtime};
use apeireth_runtime_assembly::system_one::{AgentSystemOneSupervisor, FileSystemOneAudit};

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

// ============================================================================
// 内部执行强制复核（System One，`rt.system_one_internal`）的装配面
// ============================================================================
//
// 为什么放这个模块：监督复核唯一的真实外部依赖就是「哪个 provider + 哪个
// model」，而这两个都必须**从 env 解析出的真实凭据/工厂**里取 ⇒ 与本模块
// 「从 env 找工厂 / 找模型」同源 ⇒ 不另起模块（也免得再破 `lib.rs` 尺寸棘轮）。
//
// 0 装诚实（三条）：
// * **没有真实凭据 ⇒ 不装配监督 provider** ⇒ 内部执行一律被拒（fail-closed）
//   ⇒ 绝不塞一个「永远返回 allow」的假 provider 顶替；
// * **复核不授予任何权限**：allow 只表示「模型不反对」，确定性闸（范围 / 审批 /
//   授权纪元 / 取消 / 生命周期）在复核前后各查一次，模型说 allow 也放行不了；
// * **默认关**：旋钮关 ⇒ 组合根**不构造**复核门 ⇒ 与接线前逐字节同行为。

/// 内部执行强制复核的装配开关（`rt.system_one_internal` 生产接线面）。
///
/// **默认关**（fail-closed）：`APEIRETH_ENABLE_SYSTEM_ONE=1` 才装配复核门。
/// 关时组合根**不调** `RuntimeBuilder::with_system_one` ⇒ 内部执行的错误码与
/// 接线前逐字一致（`system_one_unavailable`）。
pub const ENABLE_SYSTEM_ONE_ENV: &str = "APEIRETH_ENABLE_SYSTEM_ONE";

/// 复核审计账本的文件名（落在数据根，与治理链审计账本同根）。
const SYSTEM_ONE_AUDIT_FILE: &str = "system-one-review.jsonl";

/// 复核门开关（默认关）。
pub fn system_one_enabled_from_env() -> bool {
    std::env::var(ENABLE_SYSTEM_ONE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 生效模型 id（与运行时 `with_default_model` 同一解析口径）：
/// `APEIRETH_MODEL` 显式覆盖 > minimax 族首个默认模型 (提供商面规范 id,
/// 小写)。无模型在力回 `None` (自述模型节显式 null + 原因, 不编造)。
pub fn active_model_id() -> Option<String> {
    if let Some(model) = std::env::var("APEIRETH_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty())
    {
        return Some(model);
    }
    apeireth_provider::canonical_minimax::MinimaxProviderPlugin::from_env()
        .ok()
        .and_then(|plugin| plugin.model_ids().first().cloned())
}

/// 监督模型的选定结果：**真实 provider + 真实 model**（不含任何替代品）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemOneSupervisorSelection {
    /// 运行时已注册、且凭据可解析的 provider。
    pub provider: CapabilityId,
    /// 该 provider 的**厂商拼写**（厂商回显的 `model` 字段就是它 ⇒ 精确等式成立）。
    pub model: String,
}

/// 从**运行时已注册的 provider** + **真实凭据**里选出唯一的 (provider, model)。
///
/// 判据（三条都满足才选，缺一即不选）：
/// 1. **provider 已注册** —— 真源是运行时的 provider 注册表，不是 env 猜的；
/// 2. **凭据可解析** —— `{provider}.api_key` 在运行时凭据解析器里能取到 ⇒ 真有 key；
/// 3. **路由可判定** —— 排在它前面的 provider 都不服务这个 model ⇒ 回答者必然是
///    它 ⇒ `served_by` 不会落到别的 provider ⇒ 「精确 provider + 精确 model」成立。
///
/// 选不出 ⇒ `None`（**不装配** ⇒ 内部执行 fail-closed，与接线前同码）。
pub fn select_system_one_supervisor(
    providers: &ProviderRouter,
    resolver: &dyn CredentialResolver,
) -> Option<SystemOneSupervisorSelection> {
    let descriptors = providers.model_descriptors();
    let order = providers.provider_ids();
    for provider in &order {
        // 凭据逻辑名 = provider id + ".api_key"（provider 侧的统一命名约定；
        // 取不到 ⇒ 该 provider 没有真凭据 ⇒ 不用它，也不试别的 provider 顶替）。
        if resolver.resolve(&format!("{provider}.api_key")).is_none() {
            continue;
        }
        for descriptor in descriptors.iter().filter(|item| &item.provider == provider) {
            let model = descriptor
                .display_name
                .clone()
                .unwrap_or_else(|| descriptor.id.as_str().to_string());
            if order
                .iter()
                .take_while(|earlier| *earlier != provider)
                .any(|earlier| serves(&descriptors, earlier, &model))
            {
                continue;
            }
            return Some(SystemOneSupervisorSelection {
                provider: provider.clone(),
                model,
            });
        }
    }
    None
}

/// 该 provider 是否服务这个 model（与 provider 自身的匹配口径同形：
/// 规范 id 小写相等，或厂商拼写相等）。
fn serves(descriptors: &[ModelDescriptor], provider: &CapabilityId, model: &str) -> bool {
    let lowered = model.to_ascii_lowercase();
    descriptors.iter().any(|descriptor| {
        &descriptor.provider == provider
            && (descriptor.id.as_str() == lowered
                || descriptor.display_name.as_deref() == Some(model))
    })
}

/// 构造内部执行强制复核门（持久审计账本 + 默认限额）。
///
/// 旋钮关 ⇒ `None`（组合根不调 `with_system_one` ⇒ **零行为变化**）。
/// 旋钮开但审计账本开不出 ⇒ `None`（**不装配** ⇒ 内部执行 fail-closed）。
pub async fn system_one_gate_from_env(data_root: &Path) -> Option<Arc<SystemOneGate>> {
    if !system_one_enabled_from_env() {
        return None;
    }
    let path = data_root.join(SYSTEM_ONE_AUDIT_FILE);
    if let Some(parent) = path.parent() {
        if let Err(error) = fs_err::create_dir_all(parent) {
            eprintln!("[system-one] 复核审计账本目录建不出: {error} ⇒ 不装配（fail-closed）");
            return None;
        }
    }
    let audit = match FileSystemOneAudit::open(&path).await {
        Ok(audit) => audit,
        Err(error) => {
            eprintln!("[system-one] 复核审计账本开不出: {error} ⇒ 不装配（fail-closed）");
            return None;
        }
    };
    match SystemOneGate::new(Arc::new(audit), SystemOneLimits::default()) {
        Ok(gate) => Some(Arc::new(gate)),
        Err(error) => {
            eprintln!("[system-one] 复核门参数非法: {error} ⇒ 不装配（fail-closed）");
            None
        }
    }
}

/// 绑定**真实**监督 provider（旋钮开 ⇒ 运行时起来后绑一次）。
///
/// 绑不上 ⇒ 门保持**无监督** ⇒ 内部执行一律被拒（fail-closed，与接线前同码）。
pub async fn bind_system_one_supervisor(
    runtime: &Runtime,
    gate: &Arc<SystemOneGate>,
    resolver: &dyn CredentialResolver,
) -> bool {
    let Some(selection) = select_system_one_supervisor(runtime.providers(), resolver) else {
        eprintln!(
            "[system-one] 没有凭据可用的真实 provider ⇒ 监督复核未装配（fail-closed：内部执行将被拒绝）"
        );
        return false;
    };
    let Some(agent_runtime) = runtime.agent_runtime() else {
        eprintln!("[system-one] 运行时没有托管身份 ⇒ 监督复核未装配（fail-closed）");
        return false;
    };
    let supervisor = match AgentSystemOneSupervisor::spawn(
        agent_runtime,
        selection.model.clone(),
        selection.provider.clone(),
        supervisor_budget(),
    )
    .await
    {
        Ok(supervisor) => supervisor,
        Err(error) => {
            eprintln!("[system-one] 监督 provider 装配失败: {error} ⇒ fail-closed");
            return false;
        }
    };
    gate.bind_supervisor(Some(Arc::new(supervisor) as Arc<dyn SystemOneSupervisor>));
    true
}

/// 监督者的预算：请求数与 token **不限**（复核是常驻职责，不是单次任务），
/// 工具调用额度 1（监督者只回一个 JSON 判决，不许调工具）。
fn supervisor_budget() -> AgentBudget {
    AgentBudget {
        max_requests: u64::MAX,
        max_tokens: u64::MAX,
        max_tool_calls: 1,
        max_cost_micros: None,
        turn_timeout_ms: u64::MAX,
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
