//! Plain-text reading of one user-requested, public HTTPS page.

use std::sync::Arc;
use std::time::Duration;

use apeireth_tools_canonical::egress::{
    ControlledEgress, EgressDestination, EgressError, EgressHttpResponse,
    PUBLIC_HTTPS_URL_MAX_BYTES,
};
use apeireth_tools_canonical::fetch::html_text::extract_reading_text;
use apeireth_tools_canonical::fetch::{extract_text, extract_title};
use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, OriginalUri, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::domain_context::DomainContext;
use crate::model_connection::is_authenticated_managed_operator;
use crate::panels::GatewayState;
use crate::ClientIdentity;

const PATH: &str = "/v1/admin/desktop-web-reader";
const BODY_LIMIT: usize = 32 * 1024;
const DOWNLOAD_LIMIT: usize = 512 * 1024;
const TEXT_LIMIT: usize = 64 * 1024;
const TITLE_LIMIT: usize = 512;
const MAX_REDIRECTS: usize = 4;
const READ_TIMEOUT: Duration = Duration::from_secs(8);
const ACCEPT: &str = "text/html, application/xhtml+xml, text/plain";

struct Reader {
    egress: ControlledEgress,
    slot: Semaphore,
}

impl Reader {
    fn new() -> Self {
        Self {
            egress: ControlledEgress::public_https_only()
                .with_timeout(READ_TIMEOUT)
                .with_max_response_bytes(DOWNLOAD_LIMIT)
                // The transport budget counts the initial request.
                .with_max_redirects(MAX_REDIRECTS + 1),
            slot: Semaphore::new(1),
        }
    }

    async fn read(&self, url: &str) -> Result<Page, ReaderError> {
        let _permit = self.slot.try_acquire().map_err(|_| ReaderError::Busy)?;
        let response = self
            .egress
            .get_with_headers(url, &[("accept", ACCEPT)])
            .await
            .map_err(ReaderError::from)?;
        project(response)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadPage {
    url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContentKind {
    Html,
    Text,
}

#[derive(Debug, Serialize)]
struct Page {
    source_origin: String,
    title: Option<String>,
    text: String,
    content_kind: ContentKind,
    truncated: bool,
}

#[derive(Serialize)]
struct ReadReply {
    schema_version: u8,
    page: Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderError {
    InvalidRequest,
    InvalidUrl,
    Forbidden,
    Destination,
    RequestSize,
    PageSize,
    Unsupported,
    Empty,
    Busy,
    Upstream,
    Timeout,
}

impl From<EgressError> for ReaderError {
    fn from(error: EgressError) -> Self {
        match error {
            EgressError::InvalidUrl(_) | EgressError::UnsupportedScheme(_) => Self::InvalidUrl,
            EgressError::DestinationDenied { .. } | EgressError::RedirectDenied { .. } => {
                Self::Destination
            }
            EgressError::ResponseTooLarge => Self::PageSize,
            EgressError::Timeout => Self::Timeout,
            _ => Self::Upstream,
        }
    }
}

impl IntoResponse for ReaderError {
    fn into_response(self) -> Response {
        use StatusCode as Status;
        let (status, code, message) = match self {
            Self::InvalidRequest => (Status::BAD_REQUEST, "invalid_request", "阅读请求字段无效。"),
            Self::InvalidUrl => (
                Status::BAD_REQUEST,
                "invalid_url",
                "请输入有效的 HTTPS 网页地址。",
            ),
            Self::Forbidden => (
                Status::FORBIDDEN,
                "single_operator_required",
                "网页阅读仅用于认证的受管桌面。",
            ),
            Self::Destination => (
                Status::FORBIDDEN,
                "web_destination_denied",
                "网页地址或跳转超出公网 HTTPS 范围。",
            ),
            Self::RequestSize => (
                Status::PAYLOAD_TOO_LARGE,
                "reader_request_too_large",
                "阅读请求过大。",
            ),
            Self::PageSize => (
                Status::PAYLOAD_TOO_LARGE,
                "web_page_too_large",
                "网页超出读取大小上限。",
            ),
            Self::Unsupported => (
                Status::UNSUPPORTED_MEDIA_TYPE,
                "web_page_unsupported",
                "此网页格式暂时无法读取。",
            ),
            Self::Empty => (
                Status::UNPROCESSABLE_ENTITY,
                "web_page_empty",
                "网页没有可读取的正文。",
            ),
            Self::Busy => (
                Status::TOO_MANY_REQUESTS,
                "web_reader_busy",
                "正在读取另一个网页。",
            ),
            Self::Upstream => (
                Status::BAD_GATEWAY,
                "web_upstream_unavailable",
                "无法读取此网页。",
            ),
            Self::Timeout => (
                Status::GATEWAY_TIMEOUT,
                "web_read_timeout",
                "读取网页超时。",
            ),
        };
        (
            status,
            Json(serde_json::json!({"error":{"code":code,"message":message}})),
        )
            .into_response()
    }
}

fn content_kind(value: Option<&str>) -> Result<ContentKind, ReaderError> {
    let mut parts = value.ok_or(ReaderError::Unsupported)?.split(';');
    let kind = match parts
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "text/html" | "application/xhtml+xml" => ContentKind::Html,
        "text/plain" => ContentKind::Text,
        _ => return Err(ReaderError::Unsupported),
    };
    for (key, charset) in parts.filter_map(|part| part.split_once('=')) {
        if key.trim().eq_ignore_ascii_case("charset")
            && !matches!(
                charset
                    .trim()
                    .trim_matches('"')
                    .to_ascii_lowercase()
                    .as_str(),
                "utf-8" | "utf8" | "us-ascii" | "ascii"
            )
        {
            return Err(ReaderError::Unsupported);
        }
    }
    Ok(kind)
}

fn readable_text(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n").chars().filter(|c| {
        matches!(c, '\n' | '\t') || (!c.is_control()
            && !matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    }).collect()
}

fn bounded_text(value: &str, limit: usize) -> (&str, bool) {
    let value = value.trim();
    if value.len() <= limit {
        return (value, false);
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (&value[..end], true)
}

fn project(response: EgressHttpResponse) -> Result<Page, ReaderError> {
    if response.status == 204 || response.status == 205 {
        return Err(ReaderError::Empty);
    }
    if response.status != 200 {
        return Err(ReaderError::Upstream);
    }
    if response.body.is_empty() {
        return Err(ReaderError::Empty);
    }
    let content_kind = content_kind(response.content_type.as_deref())?;
    let raw = std::str::from_utf8(&response.body).map_err(|_| ReaderError::Unsupported)?;
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let (title, text) = match content_kind {
        ContentKind::Html => {
            let title = extract_title(raw)
                .and_then(|value| extract_text(&value).ok())
                .and_then(|value| {
                    let title = readable_text(&value)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    let (title, _) = bounded_text(&title, TITLE_LIMIT);
                    (!title.is_empty()).then(|| title.to_owned())
                });
            let text = extract_reading_text(raw).map_err(|_| ReaderError::Empty)?;
            (title, readable_text(&text))
        }
        ContentKind::Text => (None, readable_text(raw)),
    };
    let (text, truncated) = bounded_text(&text, TEXT_LIMIT);
    if text.is_empty() {
        return Err(ReaderError::Empty);
    }
    let url = response
        .final_url
        .parse()
        .map_err(|_| ReaderError::Upstream)?;
    let destination = EgressDestination::parse(&url).map_err(|_| ReaderError::Upstream)?;
    if destination.scheme != "https" || destination.port != 443 {
        return Err(ReaderError::Upstream);
    }
    Ok(Page {
        source_origin: format!("https://{}", destination.host),
        title,
        text: text.to_owned(),
        content_kind,
        truncated,
    })
}

async fn read(
    State(state): State<GatewayState>,
    Extension(reader): Extension<Arc<Reader>>,
    client: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    OriginalUri(uri): OriginalUri,
    body: Result<Json<ReadPage>, JsonRejection>,
) -> Result<Json<ReadReply>, ReaderError> {
    if !is_authenticated_managed_operator(
        &state,
        client.as_ref().map(|extension| &extension.0),
        domain.as_ref().map(|extension| &extension.0),
    ) {
        return Err(ReaderError::Forbidden);
    }
    if uri.query().is_some() {
        return Err(ReaderError::InvalidRequest);
    }
    let Json(request) = body.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ReaderError::RequestSize
        } else {
            ReaderError::InvalidRequest
        }
    })?;
    if request.url.is_empty() || request.url.len() > PUBLIC_HTTPS_URL_MAX_BYTES {
        return Err(ReaderError::InvalidUrl);
    }
    let page = reader.read(&request.url).await?;
    Ok(Json(ReadReply {
        schema_version: 1,
        page,
    }))
}

pub(crate) fn routes() -> Router<GatewayState> {
    Router::new()
        .route(PATH, post(read))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(Extension(Arc::new(Reader::new())))
}

pub(crate) async fn protect_response(request: Request, next: Next) -> Response {
    let scoped = request.uri().path() == PATH;
    let mut response = next.run(request).await;
    if scoped {
        let headers = response.headers_mut();
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        );
    }
    response
}

#[cfg(test)]
mod tests;
