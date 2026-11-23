use super::*;
use serde_json::{json, Value};

fn fetched(content_type: &str, body: impl Into<Vec<u8>>) -> EgressHttpResponse {
    EgressHttpResponse {
        status: 200,
        final_url: "https://example.org/internal?session=hide-this#hidden".into(),
        content_type: Some(content_type.into()),
        content_length: None,
        body: body.into(),
        redirects: 0,
    }
}

#[test]
fn html_only_projects_bounded_readable_text_and_the_public_origin() {
    let page = project(fetched("Text/HTML; Charset=\"UTF-8\"", concat!(
        "<html><head><title>  A &amp; B  </title><style>.evil{color: red}</style></head>",
        "<body><script>leak_secret()</script><p>你好 &lt;img&gt;</p>",
        "<img src=\"https://example.org/never-load\"><a href=\"file:///never-open\">链接</a></body></html>",
    ))).unwrap();
    let reply = serde_json::to_value(ReadReply {
        schema_version: 1,
        page,
    })
    .unwrap();
    assert_eq!(reply["schema_version"], 1);
    let page = &reply["page"];
    assert_eq!(page.as_object().unwrap().len(), 5);
    assert_eq!(page["source_origin"], "https://example.org");
    assert_eq!(page["title"], "A & B");
    assert_eq!(page["content_kind"], "html");
    assert_eq!(page["truncated"], false);
    let text = page["text"].as_str().unwrap();
    assert!(text.contains("你好 <img>"), "{text}");
    assert!(text.contains("链接"), "{text}");
    for forbidden in [
        "leak_secret",
        "color: red",
        "never-load",
        "never-open",
        "hide-this",
    ] {
        assert!(!reply.to_string().contains(forbidden), "{reply}");
    }
}

#[test]
fn page_projection_enforces_text_media_utf8_and_budgets() {
    for (mime, bytes) in [
        ("application/pdf", b"document".as_slice()),
        ("application/json", b"{ }".as_slice()),
        ("text/plain; charset=gbk", b"content".as_slice()),
        ("text/plain", &[0xfe, 0x2f]),
    ] {
        assert_eq!(
            project(fetched(mime, bytes)).unwrap_err(),
            ReaderError::Unsupported
        );
    }
    assert_eq!(
        project(fetched("text/plain", "\r\n\u{202e}\u{0001}")).unwrap_err(),
        ReaderError::Empty
    );
    let title = "好".repeat(TITLE_LIMIT);
    let text = "世".repeat(TEXT_LIMIT);
    let page = project(fetched(
        "application/xhtml+xml",
        format!("<title>{title}</title><p>{text}</p>"),
    ))
    .unwrap();
    assert!(page.title.unwrap().len() <= TITLE_LIMIT);
    assert!(page.text.len() <= TEXT_LIMIT);
    assert!(page.truncated);
    let page = project(fetched("text/plain", "\u{feff}开\r\n头\u{202e}")).unwrap();
    assert_eq!(page.text, "开\n头");
    assert_eq!(page.content_kind, ContentKind::Text);
    assert_eq!(page.title, None);
}

#[test]
fn minified_html_separates_headings_paragraphs_and_lists_without_splitting_inline_text() {
    let page = project(fetched(
        "text/html",
        concat!(
            "<!doctype html><head><title>Example Domain</title>",
            "<script>hidden()</script></head><body><main>",
            "<H1 class=\"heading\">Example <strong>Domain</strong></H1><p class=\"copy\">",
            "This domain is for <em>documentation</em> examples.",
            "<p>Works with <a href=\"https://example.org\">inline links</a>, too.",
            "<ul><li>One<li>Two</ul><div>Learn<br data-line>more</div></main></body>"
        ),
    ))
    .unwrap();
    assert_eq!(page.title.as_deref(), Some("Example Domain"));
    assert_eq!(
        page.text,
        "Example Domain\nThis domain is for documentation examples.\nWorks with inline links, too.\nOne\nTwo\nLearn\nmore"
    );
}

#[tokio::test]
async fn one_pending_reader_is_bounded_and_returns_a_fixed_busy_error() {
    let reader = Reader::new();
    let permit = reader.slot.try_acquire().unwrap();
    assert_eq!(
        reader.read("https://127.0.0.1/").await.unwrap_err(),
        ReaderError::Busy
    );
    drop(permit);
    assert_eq!(
        reader.read("https://127.0.0.1/").await.unwrap_err(),
        ReaderError::Destination
    );
}

#[tokio::test]
async fn egress_error_details_are_never_exposed() {
    for (error, status, safe_code) in [
        (
            EgressError::InvalidUrl("private-query".into()),
            StatusCode::BAD_REQUEST,
            "invalid_url",
        ),
        (
            EgressError::RedirectDenied {
                reason: "private-query".into(),
            },
            StatusCode::FORBIDDEN,
            "web_destination_denied",
        ),
        (
            EgressError::ResponseTooLarge,
            StatusCode::PAYLOAD_TOO_LARGE,
            "web_page_too_large",
        ),
        (
            EgressError::ConnectionFailed("private-query".into()),
            StatusCode::BAD_GATEWAY,
            "web_upstream_unavailable",
        ),
        (
            EgressError::Timeout,
            StatusCode::GATEWAY_TIMEOUT,
            "web_read_timeout",
        ),
    ] {
        let response = ReaderError::from(error).into_response();
        assert_eq!(response.status(), status);
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], safe_code);
        assert!(!json.to_string().contains("private-query"));
    }
}
