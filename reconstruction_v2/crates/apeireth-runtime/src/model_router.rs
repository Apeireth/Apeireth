//! ModelRouter - 根据 model 名路由到对应 ProtocolAdapter
//!
//! 0 装 PASS: 从 UnifiedRuntimeHost (host.rs:74 `protocol_adapter: Arc<dyn ProtocolAdapter>`) 抽取，
//! 把"1 个默认 adapter"升级为"按 model 前缀多 adapter 路由"模式。
//!
//! 设计动机 (per 阶段 1.2 任务说明):
//! - v1.0 + v2.0 重构后: 多个 LLM provider (OpenAI / Anthropic / Gemini / MiniMax) 都已有真实 ProtocolAdapter 实现
//! - 但 host.rs 只持 1 个 MinimaxAdapter, 想用其他 provider 需手动切换
//! - ModelRouter 提供 register/route, 让 UnifiedRuntimeHost 只跟 router 对话, 不直接管 adapter 列表
//!
//! 0-breaking: 注册表为空时 route() 返 default adapter, 行为与旧版一致。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use apeireth_protocol::normalized::{NormalizedRequest, NormalizedResponse};
use apeireth_protocol::ProtocolAdapter;

/// 模型路由策略 (按 model name 前缀匹配)
///
/// 0 装 PASS: 当前实现只用"前最长前缀匹配" (例如 "gpt-4" 命中 "gpt-" 注册的 adapter)。
/// 后续可扩展: 精确匹配 / glob 匹配 / 模型族 (model family) 分组 — 留 #[allow(dead_code)] 接口。
pub enum RouteMatch {
    /// 精确匹配 model name
    Exact(String),
    /// 前缀匹配 (e.g. "gpt-" 匹配 "gpt-4", "gpt-4-turbo")
    Prefix(String),
}

/// ModelRouter 持有 model name pattern → adapter 的映射, 加默认 fallback。
///
/// 0 装 PASS: 内部用 std::sync::RwLock (sync), 因为 HashMap 操作不在 async 上下文里;
/// route() 走读锁, register() 走写锁 — 不会死锁。
pub struct ModelRouter {
    /// 路由表: pattern_string → adapter (pattern 可能是 "exact:gpt-4" 或 "prefix:gpt-")
    routes: RwLock<HashMap<String, Arc<dyn ProtocolAdapter>>>,
    /// 未匹配时的 fallback adapter
    default_adapter: Arc<dyn ProtocolAdapter>,
}

impl ModelRouter {
    /// 0 装 PASS: 与原 host.rs new() 行为 1:1 — 默认路由表为空, 所有请求都走 default。
    pub fn new(default: Arc<dyn ProtocolAdapter>) -> Self {
        Self {
            routes: RwLock::new(HashMap::new()),
            default_adapter: default,
        }
    }

    /// 注册 adapter 到指定 pattern。后续同名 register 会覆盖前者 (last-wins)。
    ///
    /// 0 装 PASS: pattern 字符串约定 "`exact:<name>`" 或 "`prefix:<prefix>`"
    /// (例: "prefix:gpt-" 注册到所有 gpt- 前缀的 model)
    pub fn register(&self, pattern: &str, adapter: Arc<dyn ProtocolAdapter>) {
        let mut routes = self.routes.write().expect("ModelRouter routes poisoned");
        routes.insert(pattern.to_string(), adapter);
    }

    /// 按 model name 解析路由: 先精确匹配, 再前缀匹配, 最后 default。
    ///
    /// 0 装 PASS: 未匹配返回 default 的克隆 (Arc), 调用方拿 Arc 不持锁。
    pub fn route(&self, model_name: &str) -> Arc<dyn ProtocolAdapter> {
        let routes = self.routes.read().expect("ModelRouter routes poisoned");

        // 1) exact:<name> 精确匹配
        let exact_key = format!("exact:{}", model_name);
        if let Some(a) = routes.get(&exact_key) {
            return a.clone();
        }

        // 2) prefix:<p> 前缀匹配 (取最长前缀 wins, 简单 O(n) 遍历)
        let prefix_keys: Vec<&String> = routes.keys().filter(|k| k.starts_with("prefix:")).collect();
        let mut best_match: Option<(&String, &Arc<dyn ProtocolAdapter>)> = None;
        for k in prefix_keys {
            let pfx = k.strip_prefix("prefix:").unwrap();
            if model_name.starts_with(pfx) {
                if best_match.is_none() || pfx.len() > best_match.unwrap().0.strip_prefix("prefix:").unwrap().len() {
                    best_match = Some((k, routes.get(k).unwrap()));
                }
            }
        }
        if let Some((_, a)) = best_match {
            return a.clone();
        }

        // 3) fallback
        self.default_adapter.clone()
    }

    /// 0 装 PASS: 直接一步路由 + execute, 调用方无需先 route().execute()。
    /// 兼容 UnifiedRuntimeHost::handle_chat_turn 当前用法 (self.protocol_adapter.execute(...))。
    pub async fn execute(
        &self,
        api_key: &str,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, apeireth_protocol::ProtocolError> {
        let adapter = self.route(&request.model);
        adapter.execute(api_key, request).await
    }

    /// 当前已注册的 provider 数量 (含 default)
    pub fn provider_count(&self) -> usize {
        let routes = self.routes.read().expect("ModelRouter routes poisoned");
        1 + routes.len() // default + 显式路由
    }
}

impl std::fmt::Debug for ModelRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let routes = self.routes.read().expect("ModelRouter routes poisoned");
        write!(
            f,
            "ModelRouter {{ default: {}, registered: {} }}",
            self.default_adapter.provider_name(),
            routes.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// mock adapter 用于单元测试
    struct MockAdapter {
        name: &'static str,
    }
    #[async_trait::async_trait]
    impl ProtocolAdapter for MockAdapter {
        fn provider_name(&self) -> &'static str { self.name }
        async fn execute(
            &self,
            _api_key: &str,
            _req: &NormalizedRequest,
        ) -> Result<NormalizedResponse, apeireth_protocol::ProtocolError> {
            unimplemented!()
        }
    }

    fn mk(name: &'static str) -> Arc<dyn ProtocolAdapter> {
        Arc::new(MockAdapter { name })
    }

    #[test]
    fn test_default_when_empty() {
        // 0 装 PASS: 路由表空时, 所有 model name 都回退到 default
        let r = ModelRouter::new(mk("default-prov"));
        assert_eq!(r.route("gpt-4").provider_name(), "default-prov");
        assert_eq!(r.route("claude-opus").provider_name(), "default-prov");
    }

    #[test]
    fn test_exact_match_wins_over_prefix() {
        // 0 装 PASS: exact: 优先于 prefix:
        let r = ModelRouter::new(mk("default"));
        r.register("exact:gpt-4-special", mk("special-prov"));
        r.register("prefix:gpt-", mk("openai"));
        assert_eq!(r.route("gpt-4-special").provider_name(), "special-prov"); // exact 命中
        assert_eq!(r.route("gpt-4").provider_name(), "openai");           // prefix 命中
        assert_eq!(r.route("claude").provider_name(), "default");         // fallback
    }

    #[test]
    fn test_longest_prefix_wins() {
        // 0 装 PASS: 多 prefix 注册时, 最长匹配的 prefix 胜出
        let r = ModelRouter::new(mk("default"));
        r.register("prefix:gpt-", mk("openai"));
        r.register("prefix:gpt-4-", mk("openai4"));
        assert_eq!(r.route("gpt-4-turbo").provider_name(), "openai4"); // prefix:gpt-4- 更长
        assert_eq!(r.route("gpt-3.5-turbo").provider_name(), "openai"); // 只命中 prefix:gpt-
    }

    #[test]
    fn test_provider_count() {
        let r = ModelRouter::new(mk("d"));
        assert_eq!(r.provider_count(), 1);
        r.register("prefix:gpt-", mk("o"));
        assert_eq!(r.provider_count(), 2);
        r.register("prefix:claude-", mk("a"));
        assert_eq!(r.provider_count(), 3);
    }
}
