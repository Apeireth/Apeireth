//! N17 工具装配 (TP2): repo-tools 统一注册件.
//!
//! **装配三件套** (§10 铁边界): `Tool` trait 适配 + `ToolRegistry.register` + 卸载真清理.
//! 执行真走本 crate `QualityAnalyzer` (复杂度/技术债/依赖/安全/函数统计), 不自写调用方式.
//!
//! **JSON 约定**:
//! `{"op": "analyze", "repo": <path>}` → 全仓 AnalysisResult
//! `{"op": "complexity"|"tech_debt"|"deps"|"security"|"functions", "repo": <path>, "file": <path>}`
//! **0 装 PASS**: 分析结果条目 serde 直出 (各条目字段以 analyzer.rs 定义为准).

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use apeireth_tool_registry::{
    AwaitingAxis, OutputAxis, ResidentAxis, Tool, ToolAxes, ToolKind, ToolRegistry, TransportAxis,
    TriggerAxis,
};

use crate::analyzer::{AnalyzerConfig, QualityAnalyzer};

/// 注册名 (全局唯一)
pub const TOOL_NAME: &str = "RepoQualityAnalyzer";

fn make_analyzer(repo: &str) -> Result<QualityAnalyzer, String> {
    let mut config = AnalyzerConfig::default();
    config.repo_path = PathBuf::from(repo);
    QualityAnalyzer::new(config).map_err(|e| e.to_string())
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing `{key}`"))
}

/// Tool trait 适配器: 无状态 (每次 call 按 repo 构造 QualityAnalyzer).
pub struct RepoQualityAnalyzerTool;

#[async_trait]
impl Tool for RepoQualityAnalyzerTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }

    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Local,
            output: OutputAxis::Value,
        }
    }

    async fn call(&self, args: Value) -> Result<Value, String> {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("");
        let repo = required(&args, "repo")?;
        let analyzer = make_analyzer(repo)?;
        match op {
            "analyze" => {
                let result = analyzer.analyze().await.map_err(|e| e.to_string())?;
                let mut out = serde_json::to_value(&result).map_err(|e| e.to_string())?;
                if let Some(o) = out.as_object_mut() {
                    o.insert("op".to_string(), json!("analyze"));
                }
                Ok(out)
            }
            "complexity" | "tech_debt" | "deps" | "security" | "functions" => {
                let file = std::path::Path::new(required(&args, "file")?);
                let out = match op {
                    "complexity" => serde_json::to_value(
                        &analyzer
                            .analyze_complexity(file)
                            .await
                            .map_err(|e| e.to_string())?,
                    ),
                    "tech_debt" => serde_json::to_value(
                        &analyzer
                            .analyze_tech_debt(file)
                            .await
                            .map_err(|e| e.to_string())?,
                    ),
                    "deps" => serde_json::to_value(
                        &analyzer
                            .analyze_deps(file)
                            .await
                            .map_err(|e| e.to_string())?,
                    ),
                    "security" => serde_json::to_value(
                        &analyzer
                            .analyze_security(file)
                            .await
                            .map_err(|e| e.to_string())?,
                    ),
                    _ => serde_json::to_value(
                        &analyzer
                            .analyze_functions(file)
                            .await
                            .map_err(|e| e.to_string())?,
                    ),
                }
                .map_err(|e| e.to_string())?;
                Ok(json!({ "op": op, "repo": repo, "file": file.to_string_lossy(), "result": out }))
            }
            _ => Err(format!(
                "unknown op `{op}` (expected analyze|complexity|tech_debt|deps|security|functions)"
            )),
        }
    }
}

/// 统一注册进 registry (§10 铁边界③). 无状态工具, 注册即可用.
pub fn register(registry: &ToolRegistry) -> Result<(), String> {
    registry.register(TOOL_NAME.to_string(), Arc::new(RepoQualityAnalyzerTool));
    Ok(())
}

/// 卸载真清理 (§5.6 插件规范, 0 残留).
pub fn unregister(registry: &ToolRegistry) -> bool {
    registry.unregister(TOOL_NAME).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_adds_and_unregister_cleans() {
        let registry = ToolRegistry::new();
        register(&registry).expect("register");
        assert!(registry.get(TOOL_NAME).is_some());
        let before = registry.len();
        assert!(unregister(&registry));
        assert!(registry.get(TOOL_NAME).is_none(), "卸载后 0 残留");
        assert_eq!(registry.len(), before - 1);
    }

    #[tokio::test]
    async fn analyze_on_temp_repo_works() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}\n").unwrap();
        let tool = RepoQualityAnalyzerTool;
        let r = tool
            .call(json!({"op": "analyze", "repo": tmp.path().to_string_lossy()}))
            .await
            .expect("analyze");
        assert_eq!(r["op"], "analyze");
    }

    #[tokio::test]
    async fn missing_repo_rejected() {
        let tool = RepoQualityAnalyzerTool;
        let e = tool.call(json!({"op": "analyze"})).await.unwrap_err();
        assert!(e.contains("repo"));
    }

    #[tokio::test]
    async fn nonexistent_repo_rejected() {
        let tool = RepoQualityAnalyzerTool;
        let e = tool
            .call(json!({"op": "analyze", "repo": "C:/definitely-not-exists-n17"}))
            .await
            .unwrap_err();
        assert!(!e.is_empty(), "不存在的 repo 应报错");
    }
}
