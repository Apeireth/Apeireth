//! 件一 (渠道配置面) 集成测试: env 加载 / 坏配置拒开 / StoredDoc 拒开 /
//! 启动日志脱敏 / 启动装配 / 断线重连。外部端点一律本地 mock, 0 真实网络。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_gateway::{
    assemble_im_channels, im_channels_path, ImChannelConfig, ImConfigError, IM_CHANNELS_ENV,
};
use apeireth_sdk::im::{
    ImChannelKind, ImChannelTarget, ImHttpSender, ImOutboundText, ImReconnectPolicy, ImSecret,
    ImSender,
};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-im-it-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("create test dir");
    dir
}

fn channel_json(
    id: &str,
    kind: &str,
    endpoint: &str,
    secret: Option<&str>,
    enabled: bool,
) -> String {
    serde_json::json!({
        "id": id,
        "kind": kind,
        "webhook_or_endpoint": endpoint,
        "secret": secret,
        "enabled": enabled,
    })
    .to_string()
}

/// 1. env 渠道列表能装配, 启动日志脱敏且带 kind/端点/开关。
#[test]
fn env_channels_assemble_with_a_redacted_startup_log() {
    let raw = format!(
        "[{}]",
        channel_json(
            "primary",
            "im-feishu",
            "https://open.example.test/hook?key=super-secret",
            Some("shared-super-secret"),
            true
        )
    );
    let config = ImChannelConfig::from_env_value(&raw).unwrap();
    let assembly = assemble_im_channels(&config, ImReconnectPolicy::default()).unwrap();
    assert!(assembly.is_active());
    assert_eq!(assembly.targets.len(), 1);
    assert_eq!(assembly.targets[0].kind, ImChannelKind::ImFeishu);
    assert!(assembly.target("primary").is_some());

    let log = assembly.startup_log.join("\n");
    assert!(log.contains("kind=im-feishu"), "{log}");
    assert!(log.contains("enabled=true"), "{log}");
    assert!(log.contains("secret=configured"), "{log}");
    assert!(!log.contains("super-secret"), "{log}");
}

/// 2. 坏配置 fail-closed: 未知 kind / 相对端点 / 重复 id / 缺字段全部拒开。
#[test]
fn broken_channel_configuration_fails_closed() {
    let unknown_kind = format!(
        "[{}]",
        channel_json("a", "im-other", "https://x.test/h", None, true)
    );
    assert!(matches!(
        ImChannelConfig::from_env_value(&unknown_kind),
        Err(ImConfigError::InvalidJson { .. })
    ));

    let relative_endpoint = format!("[{}]", channel_json("a", "im-qq", "/hook", None, true));
    assert!(matches!(
        ImChannelConfig::from_env_value(&relative_endpoint),
        Err(ImConfigError::InvalidSpec { .. })
    ));

    let duplicate = format!(
        "[{},{}]",
        channel_json("a", "im-qq", "https://x.test/h1", None, true),
        channel_json("a", "im-qq", "https://x.test/h2", None, true)
    );
    assert!(matches!(
        ImChannelConfig::from_env_value(&duplicate),
        Err(ImConfigError::DuplicateChannel { .. })
    ));

    let missing_endpoint = r#"[{"id":"a","kind":"im-wecom","enabled":true}]"#;
    assert!(ImChannelConfig::from_env_value(missing_endpoint).is_err());
    assert!(ImChannelConfig::from_env_value("not json").is_err());
}

/// 3. 数据目录 `im-channels.json`: 好档照读, 坏档拒绝打开 (0 静默默认)。
#[test]
fn stored_channel_file_is_loaded_and_broken_file_refuses_to_open() {
    let dir = temp_dir("stored");
    let path = im_channels_path(&dir);
    let body = ImChannelConfig::from_env_value(&format!(
        "[{}]",
        channel_json(
            "primary",
            "im-wecom",
            "https://open.example.test/hook",
            None,
            true
        )
    ))
    .unwrap();
    apeireth_core::stored_doc::save_single(
        &path,
        &apeireth_gateway::doc_compat(),
        body,
        apeireth_core::stored_doc::DEFAULT_DOC_MODE,
    )
    .unwrap();

    let loaded = ImChannelConfig::load_from_data_dir(&dir)
        .unwrap()
        .expect("file present");
    assert_eq!(loaded.channels[0].id, "primary");
    assert_eq!(loaded.channels[0].kind, ImChannelKind::ImWecom);

    fs_err::write(&path, "{ not json").unwrap();
    assert!(ImChannelConfig::load_from_data_dir(&dir).is_err());
    let _ = fs_err::remove_dir_all(&dir);
}

/// 4. 秘密脱敏: 启动日志与渠道摘要 0 明文秘密 (只报 configured)。
#[test]
fn shared_secret_never_reaches_startup_logs() {
    let raw = format!(
        "[{},{}]",
        channel_json(
            "a",
            "im-feishu",
            "https://open.example.test/hook?token=endpoint-secret",
            Some("channel-shared-secret"),
            true
        ),
        channel_json("b", "im-qq", "https://open.example.test/hook2", None, true)
    );
    let config = ImChannelConfig::from_env_value(&raw).unwrap();
    let log = config.redacted_startup_log().join("\n");
    assert!(!log.contains("channel-shared-secret"), "{log}");
    assert!(!log.contains("endpoint-secret"), "{log}");
    assert!(log.contains("token=[redacted]"), "{log}");
    assert!(log.contains("secret=none"), "{log}");

    let target: ImChannelTarget = config.channels[0].target().unwrap();
    assert!(!format!("{:?}", target).contains("channel-shared-secret"));
    let _ = ImSecret::new("channel-shared-secret").unwrap();
}

/// 5. 断线重连: 瞬时失败按退避预算重试后送达; 预算耗尽报可重试错误。
#[tokio::test]
async fn outbound_delivery_reconnects_after_transient_failures() {
    #[derive(Clone)]
    struct Flaky {
        calls: Arc<AtomicUsize>,
        fail_first: usize,
    }

    impl Respond for Flaky {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call < self.fail_first {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"message_id": "m-1"}))
            }
        }
    }

    // 场景 A: 两次瞬时失败后第三次送达 (重连生效)。
    let server = MockServer::start().await;
    let flaky = Flaky {
        calls: Arc::new(AtomicUsize::new(0)),
        fail_first: 2,
    };
    Mock::given(method("POST"))
        .respond_with(flaky.clone())
        .mount(&server)
        .await;

    let target =
        ImChannelTarget::new("primary", ImChannelKind::ImWecom, server.uri(), None).unwrap();
    let sender = ImHttpSender::new(ImReconnectPolicy {
        max_attempts: 3,
        backoff_base_ms: 1,
        backoff_cap_ms: 2,
    });
    let receipt = sender
        .send_text(
            &target,
            &ImOutboundText {
                conversation_id: "wm_1".to_string(),
                text: "hello".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(receipt.message_ref, "m-1");
    assert_eq!(
        flaky.calls.load(Ordering::SeqCst),
        3,
        "两次瞬时失败后第三次送达"
    );

    // 场景 B: 一直失败 → 重连预算耗尽, 报可重试错误。
    let dead_server = MockServer::start().await;
    let dead = Flaky {
        calls: Arc::new(AtomicUsize::new(0)),
        fail_first: usize::MAX,
    };
    Mock::given(method("POST"))
        .respond_with(dead.clone())
        .mount(&dead_server)
        .await;
    let target =
        ImChannelTarget::new("dead", ImChannelKind::ImQq, dead_server.uri(), None).unwrap();
    let error = sender
        .send_text(
            &target,
            &ImOutboundText {
                conversation_id: "qc_1".to_string(),
                text: "hello".to_string(),
            },
        )
        .await
        .unwrap_err();
    assert!(error.is_retryable(), "{error}");
    assert_eq!(dead.calls.load(Ordering::SeqCst), 3, "重连预算上限生效");
}

/// 6. 未配置渠道: 装配 inert, 本地零回归的前提条件。
#[test]
fn no_configured_channels_assembles_to_an_inert_surface() {
    let dir = temp_dir("empty");
    let config = ImChannelConfig::load(Some(&dir)).unwrap();
    let assembly = assemble_im_channels(&config, ImReconnectPolicy::default()).unwrap();
    assert!(!assembly.is_active());
    assert!(assembly.targets.is_empty());
    assert!(assembly.startup_log.is_empty());
    let _ = fs_err::remove_dir_all(&dir);
}

/// env 主入口优先于数据目录 (装配口径)。
#[test]
fn env_entry_wins_over_the_data_directory() {
    let dir = temp_dir("env-wins");
    let body = ImChannelConfig::from_env_value(&format!(
        "[{}]",
        channel_json("from-file", "im-qq", "https://x.test/h", None, true)
    ))
    .unwrap();
    apeireth_core::stored_doc::save_single(
        &im_channels_path(&dir),
        &apeireth_gateway::doc_compat(),
        body,
        apeireth_core::stored_doc::DEFAULT_DOC_MODE,
    )
    .unwrap();

    std::env::set_var(
        IM_CHANNELS_ENV,
        format!(
            "[{}]",
            channel_json("from-env", "im-qq", "https://x.test/h2", None, true)
        ),
    );
    let config = ImChannelConfig::load(Some(&dir)).unwrap();
    std::env::remove_var(IM_CHANNELS_ENV);
    assert_eq!(config.channels[0].id, "from-env");
    let _ = fs_err::remove_dir_all(&dir);
}
