//! Evaluation-only wire controls absent from the canonical request contract.
//! Message/tool conversion and response/status parsing reuse the production
//! OpenAI Chat helper. This module does not change the production provider.

use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::kernel::{CapabilityId, ModelId};
use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError};
use apeireth_protocol::canonical::{ModelDescriptor, NormalizedRequest, NormalizedResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::io::hash;

const PROVIDER: &str = "provider.memory-answer-eval.http";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireOptions {
    /// omit preserves the endpoint default; enabled/disabled are explicit.
    pub thinking: String,
    pub reasoning_effort: Option<String>,
}

pub fn build_body(
    request: &NormalizedRequest,
    wire_model: &str,
    provider: &str,
    options: &WireOptions,
) -> Result<Value, ProviderError> {
    if !["omit", "enabled", "disabled"].contains(&options.thinking.as_str())
        || options
            .reasoning_effort
            .as_deref()
            .is_some_and(|effort| !["none", "low", "high", "max"].contains(&effort))
        || (options.thinking == "enabled" && options.reasoning_effort.as_deref() == Some("none"))
        || (options.thinking == "disabled"
            && options
                .reasoning_effort
                .as_deref()
                .is_some_and(|effort| effort != "none"))
    {
        return Err(ProviderError::BadResponse {
            provider: provider.into(),
            detail: "invalid or contradictory evaluation thinking controls".into(),
        });
    }
    let mut body =
        apeireth_provider::openai_chat::build_request_body(request, wire_model, provider)?;
    if options.thinking != "omit" {
        body["thinking"] = json!({"type": options.thinking});
    }
    if let Some(effort) = &options.reasoning_effort {
        body["reasoning_effort"] = json!(effort);
    }
    Ok(body)
}

pub struct EvalHttpProvider {
    id: CapabilityId,
    endpoint: String,
    model: String,
    timeout_ms: u64,
    credentials: Arc<dyn CredentialResolver>,
    options: WireOptions,
    client: reqwest::Client,
}

impl EvalHttpProvider {
    pub fn new(
        endpoint: String,
        model: String,
        timeout_ms: u64,
        credentials: Arc<dyn CredentialResolver>,
        options: WireOptions,
    ) -> Result<Self, ProviderError> {
        let bad = |detail: &str| ProviderError::BadResponse {
            provider: PROVIDER.into(),
            detail: detail.into(),
        };
        let url =
            reqwest::Url::parse(&endpoint).map_err(|_| bad("invalid evaluation endpoint URL"))?;
        let loopback = matches!(
            url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if url.host_str().is_none()
            || (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || model.trim().is_empty()
            || timeout_ms == 0
        {
            return Err(bad("evaluation endpoint requires HTTPS (or loopback HTTP), no URL credentials/query/fragment, a model and a positive timeout"));
        }
        // The explicit origin binding is checked before a secret is resolved.
        check_binding(credentials.as_ref(), &endpoint)?;
        let endpoint = if url
            .path()
            .trim_end_matches('/')
            .ends_with("/chat/completions")
        {
            endpoint.trim_end_matches('/').to_owned()
        } else {
            apeireth_provider::openai_chat::join_endpoint(&endpoint, "chat/completions")
        };
        let id = CapabilityId::new(PROVIDER)
            .map_err(|_| bad("invalid evaluation capability identity"))?;
        ModelId::new(&model).map_err(|_| bad("invalid evaluation model identity"))?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| bad("evaluation HTTP client could not be constructed"))?;
        // Validate controls during construction as well as per request.
        build_body(
            &NormalizedRequest::new(&model, vec![]),
            &model,
            PROVIDER,
            &options,
        )?;
        Ok(Self {
            id,
            endpoint,
            model,
            timeout_ms,
            credentials,
            options,
            client,
        })
    }

    fn evidence(
        &self,
        request: &Value,
        request_bytes: &[u8],
        response: Option<&[u8]>,
        status: Option<u16>,
        latency_ms: u128,
        truncated: bool,
    ) -> Value {
        let response_text = response.map(String::from_utf8_lossy);
        json!({
            "wire_request": request,
            "wire_request_text": String::from_utf8_lossy(request_bytes),
            "wire_request_sha256": hash(request_bytes),
            "wire_request_bytes": request_bytes.len(),
            "wire_response_text": response_text,
            "wire_response_sha256": response.map(hash),
            "wire_response_bytes": response.map(<[u8]>::len),
            "wire_response_utf8_exact": response.map(|bytes|std::str::from_utf8(bytes).is_ok()),
            "wire_response_truncated": truncated,
            "wire_response_hash_scope": if truncated { "captured_prefix" } else { "complete" },
            "wire_http_status": status,
            "wire_latency_ms": latency_ms,
            "wire_options": self.options,
            "wire_send_attempts": 1,
            "wire_scope": "evaluation_transport_only"
        })
    }
}

fn check_binding(
    credentials: &dyn CredentialResolver,
    endpoint: &str,
) -> Result<(), ProviderError> {
    let verdict = apeireth_provider::endpoint_binding::ensure_credential_bound_to_endpoint(
        credentials,
        apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY,
        endpoint,
        None,
    )?;
    if verdict == apeireth_provider::endpoint_binding::EndpointBindingVerdict::Unmanaged {
        return Err(ProviderError::AuthFailed {
            provider: PROVIDER.into(),
            detail: "evaluation credentials require an explicit endpoint-origin binding".into(),
        });
    }
    Ok(())
}

/// Failed attempts must keep their wire evidence too. The evaluation transport
/// returns one permanent wrapper with the canonical classification recorded,
/// so a runtime retry cannot silently spend another paid model invocation.
fn evidenced_error(error: ProviderError, evidence: Value) -> ProviderError {
    ProviderError::Refused {
        provider: PROVIDER.into(),
        detail: json!({
            "evaluation_wire_error":true,"canonical_error":error.to_string(),
            "canonical_retryable":error.is_retryable(),"wire_evidence":evidence,
        })
        .to_string(),
    }
}

#[async_trait]
impl ProviderCapability for EvalHttpProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor::new(
            ModelId::new(&self.model).expect("validated model"),
            self.id.clone(),
        )]
    }
    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        if request.model != self.model {
            return Err(ProviderError::BadResponse {
                provider: PROVIDER.into(),
                detail: "request model differs from the explicitly configured evaluation model"
                    .into(),
            });
        }
        check_binding(self.credentials.as_ref(), &self.endpoint)?;
        let key = self
            .credentials
            .resolve(apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY)
            .ok_or_else(|| ProviderError::AuthFailed {
                provider: PROVIDER.into(),
                detail: "temporary evaluation credential is unavailable".into(),
            })?;
        let body = build_body(request, &self.model, PROVIDER, &self.options)?;
        let bytes = serde_json::to_vec(&body).map_err(|_| ProviderError::BadResponse {
            provider: PROVIDER.into(),
            detail: "evaluation wire request could not be serialized".into(),
        })?;
        let started = Instant::now();
        // Exactly this serialized body is hashed and sent. Authorization is a
        // header only; no header, key, or resolver is serialized as evidence.
        let response = self
            .client
            .post(&self.endpoint)
            .timeout(Duration::from_millis(self.timeout_ms))
            .bearer_auth(key.expose())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes.clone())
            .send()
            .await;
        let mut response = match response {
            Ok(response) => response,
            Err(error) => {
                let canonical = if error.is_timeout() {
                    ProviderError::Timeout {
                        provider: PROVIDER.into(),
                        timeout_ms: self.timeout_ms,
                    }
                } else {
                    ProviderError::Network {
                        provider: PROVIDER.into(),
                        detail: error.to_string(),
                    }
                };
                return Err(evidenced_error(
                    canonical,
                    self.evidence(
                        &body,
                        &bytes,
                        None,
                        None,
                        started.elapsed().as_millis(),
                        false,
                    ),
                ));
            }
        };
        let status = response.status();
        let mut received = Vec::new();
        loop {
            let chunk = response.chunk().await.map_err(|error| {
                evidenced_error(
                    ProviderError::Network {
                        provider: PROVIDER.into(),
                        detail: error.to_string(),
                    },
                    self.evidence(
                        &body,
                        &bytes,
                        Some(&received),
                        Some(status.as_u16()),
                        started.elapsed().as_millis(),
                        true,
                    ),
                )
            })?;
            let Some(chunk) = chunk else {
                break;
            };
            let room = MAX_RESPONSE_BYTES.saturating_sub(received.len());
            received.extend_from_slice(&chunk[..chunk.len().min(room)]);
            if chunk.len() > room {
                return Err(evidenced_error(
                    ProviderError::BadResponse {
                        provider: PROVIDER.into(),
                        detail: "wire response exceeded the evaluation byte bound".into(),
                    },
                    self.evidence(
                        &body,
                        &bytes,
                        Some(&received),
                        Some(status.as_u16()),
                        started.elapsed().as_millis(),
                        true,
                    ),
                ));
            }
        }
        let mut evidence = self.evidence(
            &body,
            &bytes,
            Some(&received),
            Some(status.as_u16()),
            started.elapsed().as_millis(),
            false,
        );
        if !status.is_success() {
            let error = apeireth_provider::openai_chat::classify_status(
                status,
                String::from_utf8_lossy(&received).into_owned(),
                PROVIDER,
                self.timeout_ms,
            );
            return Err(evidenced_error(error, evidence));
        }
        let raw: Value = serde_json::from_slice(&received).map_err(|_| {
            evidenced_error(
                ProviderError::BadResponse {
                    provider: PROVIDER.into(),
                    detail: "wire response was not valid JSON".into(),
                },
                evidence.clone(),
            )
        })?;
        evidence["wire_response"] = raw.clone();
        evidence["wire_usage"] = raw.get("usage").cloned().unwrap_or(Value::Null);
        let mut normalized =
            apeireth_provider::openai_chat::parse_response(raw, &request.model, PROVIDER)
                .map_err(|error| evidenced_error(error, evidence.clone()))?;
        normalized.raw_metadata.extend(
            evidence
                .as_object()
                .expect("wire evidence is an object")
                .clone(),
        );
        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::Secret;
    use apeireth_protocol::canonical::{NormalizedMessage, ToolCall};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn options() -> WireOptions {
        WireOptions {
            thinking: "disabled".into(),
            reasoning_effort: Some("none".into()),
        }
    }
    fn request() -> NormalizedRequest {
        let mut request = NormalizedRequest::new(
            "deepseek-flash",
            vec![
                NormalizedMessage::system("server instruction\n逐字"),
                NormalizedMessage::user("中文“引号”、换行\nunchanged"),
                NormalizedMessage::assistant_with_tool_calls(
                    "",
                    vec![ToolCall {
                        id: "memory-call".into(),
                        name: "memory_recall".into(),
                        arguments: json!({}),
                    }],
                ),
                NormalizedMessage::tool_result("memory-call", None, "actual memory evidence"),
            ],
        );
        request.temperature = Some(0.0);
        request.max_tokens = Some(512);
        request
    }
    struct Credentials {
        origin: String,
    }
    impl CredentialResolver for Credentials {
        fn resolve(&self, _: &str) -> Option<Secret> {
            Some(Secret::new("fixture-private-header-only"))
        }
        fn bound_endpoints(&self, _: &str) -> Option<Vec<String>> {
            Some(vec![self.origin.clone()])
        }
    }

    #[test]
    fn wire_controls_only_add_body_fields_and_preserve_canonical_message_and_tool_serialization() {
        let request = request();
        let before = request.clone();
        let canonical = apeireth_provider::openai_chat::build_request_body(
            &request,
            "deepseek-flash",
            PROVIDER,
        )
        .unwrap();
        let mut actual = build_body(&request, "deepseek-flash", PROVIDER, &options()).unwrap();
        assert_eq!(actual["messages"], canonical["messages"]);
        assert_eq!(actual["temperature"], 0.0);
        assert_eq!(actual["max_tokens"], 512);
        assert_eq!(actual["thinking"], json!({"type":"disabled"}));
        assert_eq!(actual["reasoning_effort"], "none");
        actual.as_object_mut().unwrap().remove("thinking");
        actual.as_object_mut().unwrap().remove("reasoning_effort");
        assert_eq!(actual, canonical);
        assert_eq!(request, before);
        let omitted = build_body(
            &request,
            "deepseek-flash",
            PROVIDER,
            &WireOptions {
                thinking: "omit".into(),
                reasoning_effort: None,
            },
        )
        .unwrap();
        assert_eq!(omitted, canonical);
    }

    #[test]
    fn invalid_controls_and_unbound_or_credential_bearing_endpoints_fail_before_transport() {
        assert!(build_body(
            &request(),
            "deepseek-flash",
            PROVIDER,
            &WireOptions {
                thinking: "enabled".into(),
                reasoning_effort: Some("none".into())
            }
        )
        .is_err());
        let credentials: Arc<dyn CredentialResolver> = Arc::new(Credentials {
            origin: "https://api.deepseek.com".into(),
        });
        for endpoint in [
            "https://other.invalid",
            "https://user:password@api.deepseek.com",
            "https://api.deepseek.com?key=secret",
            "http://remote.invalid",
        ] {
            assert!(EvalHttpProvider::new(
                endpoint.into(),
                "deepseek-flash".into(),
                1000,
                credentials.clone(),
                options()
            )
            .is_err());
        }
    }

    #[tokio::test]
    async fn loopback_transport_records_actual_body_response_usage_hashes_and_a_single_send() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let raw=json!({"id":"actual-wire-id","model":"actual-vendor-model","choices":[{"message":{"content":"actual answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":1,"total_tokens":3}}).to_string();
        let response_bytes = raw.as_bytes().to_vec();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut buffer = [0; 2048];
            let (headers, body) = loop {
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                received.extend_from_slice(&buffer[..count]);
                if let Some(at) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8(received[..at].to_vec()).unwrap();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if received.len() >= at + 4 + length {
                        break (headers, received[at + 4..at + 4 + length].to_vec());
                    }
                }
            };
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}",raw.len());
            stream.write_all(response.as_bytes()).await.unwrap();
            drop(stream);
            assert!(
                tokio::time::timeout(Duration::from_millis(100), listener.accept())
                    .await
                    .is_err(),
                "there must be no application retry"
            );
            (headers, body)
        });
        let provider = EvalHttpProvider::new(
            endpoint.clone(),
            "deepseek-flash".into(),
            3000,
            Arc::new(Credentials { origin: endpoint }),
            options(),
        )
        .unwrap();
        let response = provider.complete(&request()).await.unwrap();
        let (headers, bytes) = server.await.unwrap();
        assert!(headers
            .to_lowercase()
            .contains("authorization: bearer fixture-private-header-only"));
        assert!(!String::from_utf8_lossy(&bytes).contains("fixture-private-header-only"));
        assert_eq!(response.content, "actual answer");
        assert_eq!(response.usage.total_tokens, 3);
        assert_eq!(response.raw_metadata["wire_request_sha256"], hash(&bytes));
        assert_eq!(
            response.raw_metadata["wire_response_sha256"],
            hash(&response_bytes)
        );
        assert_eq!(
            response.raw_metadata["wire_response"]["model"],
            "actual-vendor-model"
        );
        assert_eq!(response.raw_metadata["wire_send_attempts"], 1);
        assert_eq!(
            response.raw_metadata["wire_response_text"],
            String::from_utf8(response_bytes).unwrap()
        );
        let actual: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(actual, response.raw_metadata["wire_request"]);
        assert_eq!(actual["thinking"], json!({"type":"disabled"}));
        assert_eq!(actual["reasoning_effort"], "none");
    }
}
