//! 操作幂等键的 HTTP 头解码与载荷摘要（C2-1）。
//!
//! HTTP 面只认一个头: x-apeireth-operation —— 头名大小写不敏感、单值、去首尾空白。
//! 缺头 = 今日路径, 不做幂等; 有头 = 语法必须合法（与核心稳定标识同规则）,
//! 否则 400 invalid_request, 文案带 operation key。
//!
//! 载荷摘要固定编码三段 (input, model, system)，指定联系人时追加第四段。每段 = 1 字节存在位 +
//! u32 小端长度 + UTF-8 字节; 缺段只有存在位 0。model 与 system 取入口
//! 解析后的实际值, 所以同一请求在模型热切换前后会落到不同摘要（按不同载荷处理）。

use apeireth_runtime::canonical::{OperationRequest, ProjectWorkspaceBinding};
use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

use crate::canonical_entry::{CanonicalChatRequest, CanonicalEntryError};

/// 幂等操作键请求头（HTTP 头名大小写不敏感）。
pub(crate) const OPERATION_HEADER: &str = "x-apeireth-operation";

/// Decode the optional operation header into a runtime operation request.
pub(crate) fn operation_request(
    headers: &HeaderMap,
    request: &CanonicalChatRequest,
    workspace: Option<ProjectWorkspaceBinding>,
) -> Result<Option<OperationRequest>, CanonicalEntryError> {
    let mut values = headers.get_all(OPERATION_HEADER).iter();
    let Some(raw) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(CanonicalEntryError::InvalidRequest(
            "operation key must be sent in exactly one x-apeireth-operation header".to_string(),
        ));
    }
    let key = raw
        .to_str()
        .map_err(|_| {
            CanonicalEntryError::InvalidRequest(
                "operation key must be visible ASCII text".to_string(),
            )
        })?
        .trim();
    let digest = payload_digest(
        &request.input,
        request.model.as_deref(),
        request.system.as_deref(),
        request
            .expected_contact_id
            .map(|id| id.to_string())
            .as_deref(),
    );
    let digest = workspace.map_or(digest, |binding| workspace_digest(digest, binding));
    let operation = OperationRequest::new(key, digest)
        .map_err(|error| CanonicalEntryError::InvalidRequest(error.to_string()))?;
    Ok(Some(operation))
}

fn workspace_digest(digest: [u8; 32], binding: ProjectWorkspaceBinding) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"apeireth.project-workspace/v1\0");
    hasher.update(digest);
    hasher.update(binding.workspace_id.to_string().as_bytes());
    hasher.update(binding.revision.to_le_bytes());
    hasher.finalize().into()
}

/// 固定编码的载荷摘要: 段边界由长度前缀钉死, 不做分隔符拼接。
fn payload_digest(
    input: &str,
    model: Option<&str>,
    system: Option<&str>,
    contact: Option<&str>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for field in [Some(input), model, system]
        .into_iter()
        .chain(contact.map(Some))
    {
        match field {
            Some(value) => {
                hasher.update([1u8]);
                hasher.update((value.len() as u32).to_le_bytes());
                hasher.update(value.as_bytes());
            }
            None => hasher.update([0u8]),
        }
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn chat(input: &str, model: Option<&str>, system: Option<&str>) -> CanonicalChatRequest {
        let mut request = CanonicalChatRequest::new(input);
        request.model = model.map(str::to_string);
        request.system = system.map(str::to_string);
        request
    }

    fn headers_with(name: &'static str, value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn a_missing_header_means_no_operation() {
        let headers = HeaderMap::new();
        let operation = operation_request(&headers, &chat("hello", None, None), None).unwrap();
        assert!(operation.is_none());
    }

    #[test]
    fn the_header_name_is_case_insensitive_and_the_key_is_trimmed() {
        let headers = headers_with("X-Apeireth-Operation", "  op-1  ");
        let operation = operation_request(&headers, &chat("hello", None, None), None)
            .unwrap()
            .unwrap();
        assert_eq!(operation.id(), "op-1");
    }

    #[test]
    fn an_invalid_key_is_rejected_with_operation_key_wording() {
        let headers = headers_with(OPERATION_HEADER, "Bad Key");
        let error = operation_request(&headers, &chat("hello", None, None), None).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("operation key"), "{text}");
    }

    #[test]
    fn a_duplicate_header_is_rejected() {
        let mut headers = headers_with(OPERATION_HEADER, "op-1");
        headers.append(OPERATION_HEADER, HeaderValue::from_static("op-2"));
        let error = operation_request(&headers, &chat("hello", None, None), None).unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
    }

    #[test]
    fn field_boundaries_do_not_collide() {
        let joined = payload_digest("ab", None, None, None);
        let model_split = payload_digest("a", Some("b"), None, None);
        let system_split = payload_digest("a", None, Some("b"), None);
        assert_ne!(joined, model_split);
        assert_ne!(joined, system_split);
        assert_ne!(model_split, system_split);
        assert_ne!(joined, payload_digest("ab", None, None, Some("contact-a")));
        assert_ne!(
            payload_digest("ab", None, None, Some("contact-a")),
            payload_digest("ab", None, None, Some("contact-b"))
        );
        assert_eq!(payload_digest("ab", None, None, None), joined);
    }

    #[test]
    fn scope_and_revision_change_the_operation_digest() {
        let digest = payload_digest("hello", None, None, None);
        let binding = ProjectWorkspaceBinding {
            workspace_id: apeireth_core::kernel::ids::WorkspaceId::new(),
            revision: 3,
        };
        let scoped = workspace_digest(digest, binding);
        assert_ne!(digest, scoped);
        assert_ne!(
            workspace_digest(
                digest,
                ProjectWorkspaceBinding {
                    revision: 4,
                    ..binding
                }
            ),
            scoped
        );
        assert_ne!(
            workspace_digest(
                digest,
                ProjectWorkspaceBinding {
                    workspace_id: apeireth_core::kernel::ids::WorkspaceId::new(),
                    ..binding
                }
            ),
            scoped
        );
    }
}
