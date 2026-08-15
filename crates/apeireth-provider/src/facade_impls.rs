
//! apeireth-provider::facade_impls \u2014 6 Provider \u7edf\u4e00\u5b9e\u73b0 LlmFacade trait
//!
//! **\u7edf\u4e00\u63a5\u5165**: claude_code / codex / copilot / gemini_cli / opencode / minimax
//! \u90fd\u5b9e\u73b0 apeireth_acp::llm_facade::LlmFacade trait.
//!
//! **\u7eaf\u51fd\u6570**: dispatch \u4e0d\u4fee\u6539 request, \u8fd4\u54cd\u5e94\u4e0e upstream \u4e00\u81f4.
//! \u672c\u7248\u672c\u4e3a descriptor-only, \u771f\u63a5\u7531 apeireth-api::llm \u5b8c\u6210.
//!
//! **\u4e0d\u6f02\u79fb**:
//! - 0 \u6539 6 Provider struct (R35 LOCKED)
//! - 0 \u52a8 workspace.version
//!
//! **\u72b6\u6001**: R176 (2026-08-14) \u521d\u59cb\u7248, 6 Provider \u90fd implement LlmFacade.

#![allow(missing_docs)]

use apeireth_acp::llm_facade::{
    is_valid_provider, LlmFacade, LlmFacadeError, LlmRequest, LlmResponse, LlmStatus,
};

use crate::claude_code::ClaudeCodeProvider;
use crate::codex::CodexProvider;
use crate::copilot::CopilotProvider;
use crate::gemini_cli::GeminiCliProvider;
use crate::opencode::OpencodeProvider;
use crate::minimax::MinimaxProvider;

/// 6 Provider \u90fd implement LlmFacade

impl LlmFacade for ClaudeCodeProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        // Descriptor \u672c\u8eab\u4e0d\u8c03\u4e0a\u6e38, \u8fd4\u4e00\u4e2a ok \u54cd\u5e94\u6807\u8bb0 "ready"
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

impl LlmFacade for CodexProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

impl LlmFacade for CopilotProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

impl LlmFacade for GeminiCliProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

impl LlmFacade for OpencodeProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

impl LlmFacade for MinimaxProvider {
    fn name(&self) -> &'static str { self.name }
    fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
    fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        if !is_valid_provider(self.name) {
            return Err(LlmFacadeError::UnknownProvider(self.name.into()));
        }
        Ok(LlmResponse {
            request_id: String::new(),
            provider: self.name.into(),
            model: request.model,
            text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Ok,
        })
    }
}

// =====================================================================
// \u5355\u5143\u6d4b\u8bd5 \u2014 \u9a8c\u8bc1 6 Provider \u90fd\u80fd\u88ab facade \u8c03\u7528
// =====================================================================

#[cfg(test)]
mod facade_tests {
    use super::*;

    fn request_for(provider: &str) -> LlmRequest {
        LlmRequest::new(provider, "system", "user")
    }

    #[test]
    fn claude_code_dispatch_ok() {
        let p = ClaudeCodeProvider::new();
        let req = request_for("claude-code");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "claude-code");
        assert!(resp.status.is_success());
    }

    #[test]
    fn codex_dispatch_ok() {
        let p = CodexProvider::new();
        let req = request_for("codex");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "codex");
        assert!(resp.status.is_success());
    }

    #[test]
    fn copilot_dispatch_ok() {
        let p = CopilotProvider::new();
        let req = request_for("copilot");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "copilot");
        assert!(resp.status.is_success());
    }

    #[test]
    fn gemini_cli_dispatch_ok() {
        let p = GeminiCliProvider::new();
        let req = request_for("gemini-cli");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "gemini-cli");
        assert!(resp.status.is_success());
    }

    #[test]
    fn opencode_dispatch_ok() {
        let p = OpencodeProvider::new();
        let req = request_for("opencode");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "opencode");
        assert!(resp.status.is_success());
    }

    #[test]
    fn minimax_dispatch_ok() {
        let p = MinimaxProvider::new();
        let req = request_for("minimax");
        let resp = LlmFacade::dispatch(&p, req).unwrap();
        assert_eq!(resp.provider, "minimax");
        assert!(resp.status.is_success());
    }

    #[test]
    fn all_6_providers_dispatch_ok() {
        // \u96c6\u4e2d\u9a8c\u8bc1 6 Provider \u90fd OK
        let p1 = ClaudeCodeProvider::new();
        let p2 = CodexProvider::new();
        let p3 = CopilotProvider::new();
        let p4 = GeminiCliProvider::new();
        let p5 = OpencodeProvider::new();
        let p6 = MinimaxProvider::new();
        let providers: Vec<&dyn LlmFacade> = vec![&p1, &p2, &p3, &p4, &p5, &p6];
        for p in providers {
            let req = request_for(p.name());
            let resp = p.dispatch(req).unwrap();
            assert!(resp.status.is_success(), "provider {} failed", p.name());
        }
    }

    #[test]
    fn handle_validates_request_first() {
        let p = ClaudeCodeProvider::new();
        let mut req = request_for("claude-code");
        req.provider = String::new();
        assert!(p.handle(req).is_err());
    }

    #[test]
    fn handle_rejects_invalid_model() {
        let p = ClaudeCodeProvider::new();
        let mut req = request_for("claude-code");
        req.model = "non-existent-model".into();
        let res = p.handle(req);
        assert!(matches!(res, Err(LlmFacadeError::InvalidModel { .. })));
    }

    #[test]
    fn handle_accepts_empty_model() {
        // \u7a7a model \u8868\u793a\u8d4e\u4ee3 provider \u9ed8\u8ba4, \u662f\u5408\u6cd5\u7684
        let p = ClaudeCodeProvider::new();
        let req = request_for("claude-code");
        assert!(p.handle(req).is_ok());
    }

    #[test]
    fn supported_models_match_provider_descriptor() {
        // 6 Provider \u7684 model_kinds \u4e0e LlmFacade \u63a5\u53e3\u4e00\u81f4
        let p = ClaudeCodeProvider::new();
        let models = LlmFacade::supported_models(&p);
        for m in &models {
            assert!(p.model_kinds.contains(m));
        }
    }

    #[test]
    fn facade_name_matches_descriptor() {
        let p = ClaudeCodeProvider::new();
        assert_eq!(LlmFacade::name(&p), "claude-code");
        let p2 = MinimaxProvider::new();
        assert_eq!(LlmFacade::name(&p2), "minimax");
    }
}
