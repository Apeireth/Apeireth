//! 诱饵实验 → 集成测试 (真热更验收): `/v1/admin/config` 热更 api_key 后,
//! **下一次**模型请求的出站 `Authorization` 必须是新 key。
//!
//! 复现序列 (与真机诱饵实验 1:1):
//! 1. env 注入启动值 (`OPENAI_API_KEY=sk-real-old-key...`), 未设
//!    `APEIRETH_KEYRING_BACKEND` (即 "KeyringSelector 退化到 EnvCredentialResolver"
//!    的退化环境), 网关 serve, `/v1/chat/completions` 走 mock provider 一次成功;
//! 2. `POST /v1/admin/config {"provider":"openai","api_key":"sk-decoy-..."}`;
//! 3. 再次 `/v1/chat/completions` —— mock provider 收到的
//!    `Authorization: Bearer ...` 必须是 decoy, 不得再是旧 key。
//!
//! 接线走生产接缝 (`apeireth_cli::keyring_bootstrap` 的 resolver/writer 构造 +
//! 真 HTTP 网关), mock provider 只替换服务商侧网络终点, 不替换任何凭据链路。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{Clock, Timestamp, VirtualClock};
use apeireth_governance::AllowAll;
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime};

const OLD_KEY: &str = "sk-real-old-key-1234567890";
const DECOY_KEY: &str = "sk-decoy-00000000000";

const SUCCESS_BODY: &str = r#"{
    "id": "chatcmpl-decoy",
    "model": "model-a",
    "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello"}, "finish_reason": "stop"}],
    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
}"#;

/// Serializes env-mutating tests (`std::env` is process-global).
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard(&'static str, Option<String>);

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self(key, prev)
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        Self(key, prev)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.1 {
            Some(value) => std::env::set_var(self.0, value),
            None => std::env::remove_var(self.0),
        }
    }
}

/// A mock provider endpoint that records every raw HTTP request it receives.
struct RecordingServer {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl RecordingServer {
    async fn start(body: &'static str) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_clone = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let requests = Arc::clone(&requests_clone);
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 8192];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    requests
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&buf[..n]).into_owned());
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.flush().await;
                });
            }
        });
        Self {
            base_url: format!("http://{addr}"),
            requests,
        }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

#[tokio::test]
async fn decoy_hot_key_reaches_the_next_provider_request() {
    let _lock = ENV_LOCK.lock().unwrap();
    // 诱饵实验环境: 启动值在 env, 无 keyring backend (resolver 退化到 env 读取)。
    let _g_key = EnvGuard::set("OPENAI_API_KEY", OLD_KEY);
    let _g_backend = EnvGuard::remove("APEIRETH_KEYRING_BACKEND");
    let _g_token = EnvGuard::remove("APEIRETH_GATEWAY_TOKEN");
    let _g_model = EnvGuard::remove("APEIRETH_MODEL");

    let server = RecordingServer::start(SUCCESS_BODY).await;

    // 生产接线: runtime 凭据解析链来自 keyring_bootstrap (运行时凭据库 >
    // env 启动值), admin 写入端口同源。
    let resolver = apeireth_cli::keyring_bootstrap::build_keyring_resolver();
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(
            server.base_url.clone(),
            vec!["model-a".into()],
            http,
            5_000,
        )
        .unwrap(),
    );
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_session_store(Arc::new(InMemorySessionStore::new()))
            .with_governance(Arc::new(AllowAll))
            .with_credentials(resolver)
            .with_plugin(plugin)
            .with_default_model("model-a")
            .build()
            .await
            .unwrap(),
    );

    let mut services = apeireth_gateway::GatewayServices::default();
    services.credentials = apeireth_cli::keyring_bootstrap::build_keyring_credential_writer();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(apeireth_gateway::serve_canonical_with_services(
        listener, runtime, services,
    ));
    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    let chat = serde_json::json!({
        "model": "model-a",
        "messages": [{"role": "user", "content": "hi"}],
    });

    // 步骤 1: 启动值请求一次, 出站 Authorization = env 里的旧 key。
    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&chat)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["apeireth"]["served_by"], "provider.openai-compatible");
    let first = server.requests().into_iter().next().unwrap();
    assert!(
        first.contains(&format!("Bearer {OLD_KEY}")),
        "baseline must carry the startup key: {first}"
    );

    // 步骤 2: 诱饵序列原样热更 (provider "openai" 是 admin/UI 侧拼写)。
    let response = client
        .post(format!("{base}/v1/admin/config"))
        .json(&serde_json::json!({ "provider": "openai", "api_key": DECOY_KEY }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["ok"], true, "{body}");

    // 步骤 3: 下一次请求必须现解析出 decoy, 不得再用旧 key。
    let response = client
        .post(format!("{base}/v1/chat/completions"))
        .json(&chat)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let last = server.requests().into_iter().last().unwrap();
    assert!(
        last.contains(&format!("Bearer {DECOY_KEY}")),
        "the decoy key must reach the next provider request: {last}"
    );
    assert!(
        !last.contains(OLD_KEY),
        "the old key must not survive the hot update: {last}"
    );

    // 清理进程级运行时凭据库, 不污染同二进制其它测试。
    apeireth_cli::keyring_bootstrap::hot_credential_store()
        .remove("provider.openai-compatible.api_key");
}
