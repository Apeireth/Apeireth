use std::sync::Arc;
use apeireth_runtime::UnifiedRuntimeHost;
use apeireth_gateway::server::create_router_with_host;
use axum::http::StatusCode;
use tower::ServiceExt;
use axum::body::Body;
use axum::http::Request;


#[tokio::test]
async fn test_gateway_http_with_living_runtime_host() {
    let key_path = r"C:\Users\31683\apikey-ultra.txt";
    let api_key = match std::fs::read_to_string(key_path) {
        Ok(k) => k.trim().to_string(),
        Err(_) => return,
    };

    let db_path = format!("test_gw_host_{}.db", uuid::Uuid::new_v4());
    let host = Arc::new(UnifiedRuntimeHost::new(&api_key, &db_path).await.unwrap());
    let app = create_router_with_host(host);

    let req_body = serde_json::json!({
        "model": "MiniMax-Text-01",
        "messages": [
            {"role": "user", "content": "Respond with the word 'APEIRETH_ACTIVE' only."}
        ]
    });

    let request = Request::builder()
        .uri("/v1/chat/completions")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let res_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    println!("Gateway Response JSON:\n{}", serde_json::to_string_pretty(&res_json).unwrap());

    assert_eq!(res_json["object"], "chat.completion");
    assert!(res_json["choices"][0]["message"]["content"].as_str().unwrap().contains("APEIRETH_ACTIVE"));
    assert!(res_json["usage"]["total_tokens"].as_u64().unwrap() > 0);
    assert!(res_json["apeireth_meta"]["audit_hash"].as_str().is_some());

    let _ = std::fs::remove_file(db_path);
}
