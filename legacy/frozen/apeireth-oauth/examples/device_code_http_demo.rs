//! R44: device_code HTTP polling demo
//!
//! **用法**: cargo run -p apeireth-oauth --features real-http --example device_code_http_demo
//!
//! **预期输出**: 4 步 polling -> 第 1, 2 次 authorization_pending -> 第 3 次 access_token.

#![cfg(feature = "real-http")]

use apeireth_oauth::device_code::{DeviceCodeSession, DeviceCodeStep};
use apeireth_oauth::transport::HttpTransport;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mock = MockServer::start().await;
    Mock::given(method("POST")).and(path("/device_authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "device_code": "GmRhmhcxhwAzkoEqiMEg_DnyEysNkuNhszIySk9eS",
                "user_code": "WDJB-MJHT",
                "verification_uri": "https://example.com/device",
                "verification_uri_complete": "https://example.com/device?user_code=WDJB-MJHT",
                "expires_in": 1800,
                "interval": 5
            }"#,
        ))
        .mount(&mock).await;
    Mock::given(method("POST")).and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"error":"authorization_pending","error_description":"pending"}"#,
        ))
        .up_to_n_times(2)
        .mount(&mock).await;
    Mock::given(method("POST")).and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "access_token": "demo_access_token_xyz",
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": "demo_refresh_abc",
                "scope": "read write"
            }"#,
        ))
        .mount(&mock).await;

    let transport = HttpTransport::new()?;

    let mut session = DeviceCodeSession::new("demo_client", vec!["read".into(), "write".into()])?;
    let resp = transport
        .post_device_authorization(
            &format!("{}/device_authorization", mock.uri()),
            &session.client_id,
            &session.scope.join(" "),
        )
        .await?;
    println!("step 0: HTTP response: device_code={}, user_code={}, interval={}", resp.device_code, resp.user_code, resp.interval);

    // 用 HTTP response 填充 session state
    session.issue_code_from_http(
        resp.device_code.clone(),
        resp.user_code.clone(),
        resp.verification_uri.clone(),
        resp.verification_uri_complete.clone(),
        resp.expires_in,
        resp.interval,
    )?;

    session.user_submitted()?;

    for i in 1..=3 {
        let poll_resp = transport
            .post_token_poll(
                &format!("{}/token", mock.uri()),
                &session.client_id,
                &resp.device_code,
            )
            .await?;
        if let Some(err) = poll_resp.error {
            println!("poll #{}: error={} ({})", i, err, poll_resp.error_description.unwrap_or_default());
        } else if let Some(token) = poll_resp.access_token {
            println!("poll #{}: ACCESS_TOKEN={}, type={:?}", i, token, poll_resp.token_type);
            session.complete()?;
            break;
        }
    }

    println!("final step: {:?} (Complete = 4)", session.current_step);
    assert_eq!(session.current_step, DeviceCodeStep::Complete);
    println!("R44 device_code HTTP polling demo OK");
    Ok(())
}
