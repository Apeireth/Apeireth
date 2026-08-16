//! Mock LLM Provider — Rust 内 trait 实现 (禁止 PyO3 / HTTP)
//!
//! **设计**: 7 强制 advisor 可选接入 [`MockLlmProvider`] 后端进行更复杂推理;
//! 本 trait 的实现全部在 Rust 内完成, **不调外部 LLM HTTP, 不调 PyO3**。
//!
//! 提供 [`ScriptedMockLlm`] 默认实现 — 按 prompt 关键词匹配脚本响应。

// R164: 移除 MockLlmProvider trait 的 #[deprecated] attribute (R163 引入时 30 actionable warnings, O-5 不假装原则不允许默认隐藏). 改为结构化文档 + 推荐 LlmAdvisorBackend 生产路径. 不修改 trait shape, 0 触碰 3 不可变脊柱.
#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;
use std::sync::Mutex;

/// Mock LLM 响应。
#[derive(Debug, Clone, PartialEq)]
pub struct MockLlmResponse {
    /// 响应文本
    pub text: String,
    /// 是否触发按住 (True = 强反对)
    pub triggers_hold: bool,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
}

impl MockLlmResponse {
    /// 便利构造 — 不触发按住。
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            triggers_hold: false,
            confidence: 0.8,
        }
    }

    /// 便利构造 — 强反对 (按住触发)。
    pub fn reject(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            triggers_hold: true,
            confidence: 0.95,
        }
    }

    /// 自定义置信度。
    pub fn with_confidence(mut self, c: f64) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }
}

/// Mock / scripted LLM trait (R164): 测试用 mock + 兼容 adapter trait, 不是真 LLM 推理.
///
/// 公开 API 留这个 trait 是为了:
/// 1. Backward compat (57+ 处使用)
/// 2. 测试用 mock / scripted LLM
/// 3. `LlmAdvisorBackend` (llm_backend.rs) 适配实 LLM 为这 trait (生产路径)
///
/// 真 LLM 接入走: `apeireth_api::llm::LlmProvider` (traits.rs),
/// 配合 `apeireth_council::LlmAdvisorBackend` (llm_backend.rs) -> council 7 advisor.
///
/// **不假装**: 别误用 `ScriptedMockLlm` / `HashMapMockLlm` 做生产 — 它们是脚本关键词匹配, 不是推理.
/// 用它当 advisor 后端就是"假装有 LLM".
pub trait MockLlmProvider: Send + Sync {
    /// 生成响应
    fn generate(&self, prompt: &str, system: &str) -> MockLlmResponse;
}

/// 脚本化 Mock LLM — 按 prompt 关键词匹配响应。
///
/// **用法**: 注册 `(关键词 → 响应)` 映射; 调用时按顺序匹配第一个命中的关键词。
/// 若无命中, 返回默认响应 (默认 = 赞成 + 不触发按住)。
pub struct ScriptedMockLlm {
    /// (关键词 → 响应) 列表 (按插入顺序)
    scripts: Vec<(String, MockLlmResponse)>,
    /// 默认响应
    default: MockLlmResponse,
    /// 调用计数 (互斥保护)
    call_count: Mutex<u64>,
}

impl ScriptedMockLlm {
    /// 创建新脚本 Mock LLM (默认响应 = ok)。
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            default: MockLlmResponse::ok("默认响应 — 无关键词命中, 默认赞成"),
            call_count: Mutex::new(0),
        }
    }

    /// 注册脚本。
    pub fn with_script(mut self, keyword: impl Into<String>, response: MockLlmResponse) -> Self {
        self.scripts.push((keyword.into(), response));
        self
    }

    /// 设置默认响应。
    pub fn with_default(mut self, response: MockLlmResponse) -> Self {
        self.default = response;
        self
    }

    /// 已调用次数。
    pub fn call_count(&self) -> u64 {
        *self.call_count.lock().expect("call_count poisoned")
    }
}

impl Default for ScriptedMockLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLlmProvider for ScriptedMockLlm {
    fn generate(&self, prompt: &str, _system: &str) -> MockLlmResponse {
        // Increment counter
        {
            let mut count = self.call_count.lock().expect("call_count poisoned");
            *count += 1;
        }
        // Match first keyword (case-insensitive)
        let lower = prompt.to_lowercase();
        for (keyword, response) in &self.scripts {
            if lower.contains(&keyword.to_lowercase()) {
                return response.clone();
            }
        }
        self.default.clone()
    }
}

/// HashMap 化 Mock LLM — 更直接的关键词查找 (无顺序保证)。
pub struct HashMapMockLlm {
    scripts: HashMap<String, MockLlmResponse>,
    default: MockLlmResponse,
}

impl HashMapMockLlm {
    /// 创建
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            default: MockLlmResponse::ok("默认响应"),
        }
    }

    /// 注册脚本
    pub fn insert(&mut self, keyword: impl Into<String>, response: MockLlmResponse) {
        self.scripts.insert(keyword.into(), response);
    }

    /// 设置默认
    pub fn set_default(&mut self, response: MockLlmResponse) {
        self.default = response;
    }
}

impl Default for HashMapMockLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLlmProvider for HashMapMockLlm {
    fn generate(&self, prompt: &str, _system: &str) -> MockLlmResponse {
        let lower = prompt.to_lowercase();
        for (keyword, response) in &self.scripts {
            if lower.contains(&keyword.to_lowercase()) {
                return response.clone();
            }
        }
        self.default.clone()
    }
}
