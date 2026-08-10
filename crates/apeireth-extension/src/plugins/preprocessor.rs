//! MessagePreprocessorPlugin — 消息中间件
//!
//! 在消息路由前 transform, 通常修改 text / context.
//! `process` 返回修改后的 ExtensionInput, 下一棒继续.

use crate::error::Result;
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;

/// 消息预处理器
pub struct MessagePreprocessorPlugin {
    manifest: Manifest,
    /// 转换函数: 输入 → 修改后输入
    transform: Box<dyn Fn(ExtensionInput) -> ExtensionInput + Send + Sync>,
}

impl MessagePreprocessorPlugin {
    /// 构造
    pub fn new<F>(manifest: Manifest, transform: F) -> Self
    where
        F: Fn(ExtensionInput) -> ExtensionInput + Send + Sync + 'static,
    {
        Self {
            manifest,
            transform: Box::new(transform),
        }
    }

    /// 转换入口
    pub fn process(&self, input: ExtensionInput) -> ExtensionInput {
        (self.transform)(input)
    }

    /// 示例: 大写化 args.text
    pub fn example_uppercase(name: impl Into<String>) -> Self {
        Self::new(
            Manifest {
                name: name.into(),
                version: "0.1.0".into(),
                kind: PluginKind::MessagePreprocessor,
                description: "Example preprocessor (uppercase text)".into(),
                entry: "upper.rs".into(),
                permissions: vec!["read".into()],
                max_input_bytes: 1024 * 1024,
                max_output_bytes: 1024 * 1024,
                timeout_ms: 1000,
            },
            |mut input| {
                if let Some(text) = input
                    .args
                    .get_mut("text")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                {
                    input.args["text"] = serde_json::Value::String(text.to_uppercase());
                }
                input
            },
        )
    }
}

#[async_trait]
impl AsyncExtension for MessagePreprocessorPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::MessagePreprocessor
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        let transformed = self.process(input);
        Ok(ExtensionOutput::ok(json!({
            "transformed_args": transformed.args,
            "transformed_context": transformed.context,
        })))
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn preprocessor_uppercase() {
        let p = MessagePreprocessorPlugin::example_uppercase("upper-1");
        let out = p
            .call(ExtensionInput::new(json!({"text": "hello"})))
            .await
            .unwrap();
        assert_eq!(out.result["transformed_args"]["text"], "HELLO");
    }

    #[tokio::test]
    async fn preprocessor_no_text_field() {
        let p = MessagePreprocessorPlugin::example_uppercase("upper-2");
        let out = p
            .call(ExtensionInput::new(json!({"other": 1})))
            .await
            .unwrap();
        // unchanged
        assert_eq!(out.result["transformed_args"], json!({"other": 1}));
    }

    #[tokio::test]
    async fn preprocessor_kind() {
        let p = MessagePreprocessorPlugin::example_uppercase("upper-3");
        assert_eq!(p.kind(), PluginKind::MessagePreprocessor);
    }

    #[tokio::test]
    async fn preprocessor_chain_two() {
        // chain uppercase + append
        let upper = MessagePreprocessorPlugin::new(
            Manifest {
                name: "u".into(),
                version: "0.1.0".into(),
                kind: PluginKind::MessagePreprocessor,
                description: "upper".into(),
                entry: "u.rs".into(),
                permissions: vec!["read".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
            },
            |mut i| {
                if let Some(s) = i
                    .args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                {
                    i.args["text"] = serde_json::Value::String(s.to_uppercase());
                }
                i
            },
        );
        let append = MessagePreprocessorPlugin::new(
            Manifest {
                name: "a".into(),
                version: "0.1.0".into(),
                kind: PluginKind::MessagePreprocessor,
                description: "append".into(),
                entry: "a.rs".into(),
                permissions: vec!["read".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
            },
            |mut i| {
                if let Some(s) = i
                    .args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                {
                    i.args["text"] = serde_json::Value::String(format!("{s}!"));
                }
                i
            },
        );
        let i = ExtensionInput::new(json!({"text": "hi"}));
        let i2 = upper.process(i);
        let i3 = append.process(i2);
        assert_eq!(i3.args["text"], "HI!");
    }
}
