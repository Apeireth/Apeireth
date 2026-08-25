use std::time::Duration;

#[tokio::test]
#[ignore = "network e2e: real HTTP requests to bilibili.com / api.bilibili.com"]
async fn test_proxy_and_direct_fetch() {
    let urls = ["https://www.bilibili.com", "https://api.bilibili.com/x/web-interface/search/all/v2?keyword=live2d"];

    for proxy_opt in [Some("http://127.0.0.1:7897"), None] {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0");

        if let Some(p) = proxy_opt {
            if let Ok(proxy) = reqwest::Proxy::all(p) {
                builder = builder.proxy(proxy);
            }
        }

        if let Ok(client) = builder.build() {
            println!("Testing with proxy: {:?}", proxy_opt);
            for url in &urls {
                match client.get(*url).send().await {
                    Ok(resp) => {
                        println!("SUCCESS [{}] Status={}", url, resp.status());
                    }
                    Err(e) => {
                        println!("FAILED [{}] Error={}", url, e);
                    }
                }
            }
        }
    }
}
