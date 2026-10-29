use super::*;

/// **推理字段归一化的端到端证明**：经真实 provider 的 `complete()` 路径。
///
/// mock 厂商在 `message` 里同时给 `content` 与 `reasoning_content`；
/// provider 打开推理适配后，客户端可见正文应是「`<think>` 块 + 结论」。
/// 适配**关**时正文只有结论（零行为变化）—— 两态都断言。
#[tokio::test]
async fn reasoning_adapter_is_wired_through_the_real_complete_path() {
    async fn one_shot_response_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let body = r#"{"id":"chatcmpl-1","choices":[{"message":{"role":"assistant","content":"结论","reasoning_content":"先想一步"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}"#;
            let out = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(out.as_bytes()).await;
            let _ = socket.flush().await;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });
        format!("http://{addr}")
    }

    let base_url = one_shot_response_server().await;
    let resolver: ResolverSlot = Arc::new(Mutex::new(Some(Arc::new(
        apeireth_plugin::StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, "sk-test"),
    )
        as Arc<dyn apeireth_plugin::CredentialResolver>)));
    let capability = OpenAiCompatibleProviderCapability::new(
        base_url,
        vec!["deepseek-v4".into()],
        http(),
        DEFAULT_TIMEOUT_MS,
        resolver,
    )
    .expect("capability builds");

    let mut request = request();
    request.model = "deepseek-v4".to_string();

    // ① 适配关：只有结论
    let plain = capability.complete(&request).await.expect("plain complete");
    let plain_text = plain.content;
    assert_eq!(plain_text, "结论", "适配关时推理文本不得进入正文");
    assert!(!plain_text.contains("<think>"));

    // ② 适配开：think 块前置
    let enabled =
        capability.with_reasoning_config(crate::reasoning_adapter::ReasoningAdapterConfig {
            enabled: true,
            model_filters: vec!["deepseek".to_string()],
            tag: "think".to_string(),
        });
    enabled.set_base_url(one_shot_response_server().await);
    let reasoned = enabled.complete(&request).await.expect("reasoned complete");
    let reasoned_text = reasoned.content;
    assert!(
        reasoned_text.starts_with("<think>"),
        "think 块应前置: {reasoned_text}"
    );
    assert!(reasoned_text.contains("先想一步"), "{reasoned_text}");
    assert!(reasoned_text.ends_with("结论"), "{reasoned_text}");
}
