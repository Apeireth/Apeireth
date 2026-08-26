//! R16-09 apeireth-council LLM 真接入 (LlmBackend adapter)
//!
//! 把 apeireth-api::llm::LlmProvider 适配为 apeireth-council::mock_llm::MockLlmProvider
//! 让 council 7 advisor 真正接入 LLM (不是 mock)
//!
//! MockLlmProvider 是同步 trait, 用 tokio Handle::block_on 转 async

use std::sync::Arc;

use crate::mock_llm::{MockLlmProvider, MockLlmResponse};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};

/// LLM advisor 后端 —— 把 apeireth-api 的 LlmProvider 包成 MockLlmProvider
pub struct LlmAdvisorBackend {
    pub llm: Arc<dyn LlmProvider>,
}

impl LlmAdvisorBackend {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }
}

// LlmAdvisorBackend 是 OFFICIAL 真 LLM 桥 — `MockLlmProvider` deprecation 警告
// 里写的"真 LLM 用 ... + LlmAdvisorBackend"就是这一个 impl. 这是 trait 退役路径,
// 不是误用. 显式 allow + 注释说明.
#[allow(deprecated)]
impl MockLlmProvider for LlmAdvisorBackend {
    fn generate(&self, prompt: &str, system: &str) -> MockLlmResponse {
        // 用 tokio Handle::current().block_on 调 async LLM
        // Council 调 generate() 是同步的, 但 LLM 调用是 async 的
        let result = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(async {
                // 修 Bug: 跟 council_advise 一样, model 字段必须用真 model 名
                let model = if self.llm.name() == "apeireth-api" {
                    "MiniMax-M3".to_string()
                } else {
                    self.llm.name().to_string()
                };
                self.llm
                    .complete(LlmRequest::new(
                        &model,
                        vec![
                            ChatMessage::system(system.to_string()),
                            ChatMessage::user(prompt.to_string()),
                        ],
                    ))
                    .await
            }),
            Err(_) => {
                // 没在 tokio runtime 里, 启动临时 runtime
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return MockLlmResponse::ok("");
                };
                rt.block_on(async {
                    let model = if self.llm.name() == "apeireth-api" {
                        "MiniMax-M3".to_string()
                    } else {
                        self.llm.name().to_string()
                    };
                    self.llm
                        .complete(LlmRequest::new(
                            &model,
                            vec![
                                ChatMessage::system(system.to_string()),
                                ChatMessage::user(prompt.to_string()),
                            ],
                        ))
                        .await
                })
            }
        };

        match result {
            Ok(resp) => {
                // 简化: 返回内容 + 不触发 hold (Council 不通过 LLM 触发 hold, 用 keyword 扫描)
                MockLlmResponse {
                    text: resp.content,
                    triggers_hold: false,
                    confidence: 0.9,
                }
            }
            Err(_) => MockLlmResponse::ok(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_api::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};

    #[test]
    fn test_llm_backend_with_scripted() {
        let llm = Arc::new(
            ScriptedLlmProvider::new("test-llm")
                .with_script("hello", ScriptedResponse::new("hi from LLM")),
        );
        let backend = LlmAdvisorBackend::new(llm);

        // 同步调用 (MockLlmProvider trait)
        let resp = backend.generate("hello world", "you are a test");
        // ScriptedLlmProvider 匹配 "hello" 关键字
        assert_eq!(resp.text, "hi from LLM");
        assert!(!resp.triggers_hold);
        assert!((resp.confidence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_llm_backend_default_response() {
        let llm = Arc::new(ScriptedLlmProvider::new("test-llm"));
        let backend = LlmAdvisorBackend::new(llm);
        let resp = backend.generate("nothing matches", "system");
        // 没匹配任何脚本, 返回默认 (ok 构造)
        assert!(resp.text.contains("默认响应"));
    }
}
