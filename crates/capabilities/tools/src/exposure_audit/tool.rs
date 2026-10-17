//! `tool.exposure_audit` —— 暴露面体检的模型可调出口 (只读档)。
//!
//! 同一内核 [`crate::exposure_audit::run_exposure_audit`] 的两个出口之一 (AI 先
//! 用, 见设计第一律): 模型可随时给自己做一次本地暴露面体检; CLI
//! (`apeireth exposure report`) 是同一引擎的人类出口。治理档 = **只读** (体检
//! 只读被测面、报告只回话不落盘), 无需审批; 报告零明文 (凭据值/密钥值不进
//! 输出)。

use std::path::PathBuf;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::exposure_audit::{run_exposure_audit, ExposureAuditConfig};

/// 暴露面体检工具能力 id (只读档)。
pub const EXPOSURE_AUDIT_CAPABILITY_ID: &str = "tool.exposure_audit";

/// 暴露面体检工具的声明名。
pub const EXPOSURE_AUDIT_TOOL_NAME: &str = "exposure_audit";

/// 空参数校验: 只收可选 `bundle` (未知字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExposureAuditParams {
    bundle: Option<String>,
}

/// 暴露面体检工具 (`tool.exposure_audit`, 声明名 `exposure_audit`)。
pub struct ExposureAuditTool {
    id: CapabilityId,
    config: ExposureAuditConfig,
}

impl ExposureAuditTool {
    /// 绑定体检配置 (数据根 + 探测口)。
    pub fn new(config: ExposureAuditConfig) -> Self {
        Self {
            id: CapabilityId::new(EXPOSURE_AUDIT_CAPABILITY_ID).unwrap(),
            config,
        }
    }

    /// 输出归一合同: `items` (体检项数组) + `overall` (总评对象)。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("items", SchemaKind::Array)
            .require("overall", SchemaKind::Object)
    }
}

#[async_trait]
impl ToolCapability for ExposureAuditTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "bundle": {
                        "type": "string",
                        "description": "Optional backup bundle directory to check exclusion compliance. Defaults to the most recent recorded backup."
                    }
                },
                "required": [],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(EXPOSURE_AUDIT_TOOL_NAME)
            .with_description(
                "Run a read-only local exposure audit (defensive self-check of your own attack \
                 surface): credential-file distribution and plaintext secret-field risk points by \
                 the shared sensitive-path vocabulary, locally listening sockets from a controlled \
                 system-table snapshot, backup-bundle exclusion compliance, and client-token/user \
                 binding integrity. Each item reports finding count, risk level and one-line \
                 remediation advice. The audit never writes to the audited surface and never \
                 reports secret values. External exposure (public reachability) is explicitly \
                 marked as not measured.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => serde_json::json!({}),
            other => other.clone(),
        };
        let params = match serde_json::from_value::<ExposureAuditParams>(arguments) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid exposure_audit parameters: {error}"),
                );
            }
        };
        let mut config = self.config.clone();
        if let Some(bundle) = params.bundle {
            if bundle.trim().is_empty() {
                return ToolResult::permanent_error(
                    &call.id,
                    "invalid exposure_audit parameters: bundle must not be empty".to_string(),
                );
            }
            config.bundle = Some(PathBuf::from(bundle));
        }
        // 体检跑在阻塞池上: 遍历大目录卡住时执行链的到期计时器仍能触发。
        match tokio::task::spawn_blocking(move || run_exposure_audit(&config)).await {
            Ok(report) => ToolResult::ok(&call.id, report.to_json_value()),
            Err(error) => ToolResult::permanent_error(
                &call.id,
                format!("exposure_audit task failed: {error}"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_id_and_output_schema_are_pinned() {
        let tool = ExposureAuditTool::new(ExposureAuditConfig::new("data"));
        assert_eq!(tool.id().as_str(), EXPOSURE_AUDIT_CAPABILITY_ID);
        let declaration = tool.declaration();
        assert_eq!(declaration.name, EXPOSURE_AUDIT_TOOL_NAME);
        let schema = ExposureAuditTool::output_schema();
        assert_eq!(schema.fields().len(), 2);
        assert!(schema
            .validate(&serde_json::json!({
                "items": [],
                "overall": {},
            }))
            .is_ok());
    }
}
