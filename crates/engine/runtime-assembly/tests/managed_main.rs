#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::SessionId;
use apeireth_plugin::agents::{AgentConfig, AgentOwner, AgentStatus};
use apeireth_plugin::{Plugin, PluginContext, PluginManifest};
use apeireth_runtime::{AgentHostPolicy, TurnRequest};
use std::{sync::Arc, time::Duration};
use support::*;

struct Probe(PluginManifest);
#[async_trait::async_trait]
impl Plugin for Probe {
    fn manifest(&self) -> &PluginManifest {
        &self.0
    }
    async fn initialize(&self, _: &PluginContext) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn managed_main_releases_control_lock_and_keeps_sessions_concurrent_until_shutdown() {
    let provider = Provider::new(vec![]);
    let _blocked = provider.hold_next();
    let host = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let runtime = Arc::new(
        host.into_main_runtime(AgentConfig::default())
            .await
            .unwrap(),
    );
    let identity = runtime.managed_main().unwrap();
    let first_session = SessionId::new();
    let first = {
        let runtime = runtime.clone();
        tokio::spawn(async move {
            runtime
                .execute_outcome(TurnRequest::new(first_session, "first private history"))
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(2), async {
        while provider.requests().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(runtime.managed_main().unwrap().status, AgentStatus::Running);
    let host = runtime.agent_runtime().unwrap();
    let id = pid("test.hot_probe");
    tokio::time::timeout(
        Duration::from_secs(1),
        host.register_module(
            Arc::new(Probe(PluginManifest::new(
                id.clone(),
                "1.0.0",
                "control lock probe",
            ))),
            PluginContext::new(
                Arc::new(apeireth_core::kernel::SystemClock),
                Arc::new(apeireth_plugin::NoCredentials),
                apeireth_core::kernel::TraceId::new(),
            ),
        ),
    )
    .await
    .expect("Provider wait must not hold the control lock")
    .unwrap();
    tokio::time::timeout(Duration::from_secs(1), host.stop_module(&id))
        .await
        .unwrap()
        .unwrap();
    let second_session = SessionId::new();
    tokio::time::timeout(
        Duration::from_secs(1),
        runtime.execute_outcome(TurnRequest::new(second_session, "second private history")),
    )
    .await
    .expect("different sessions must progress concurrently")
    .unwrap();
    for session in [first_session, second_session] {
        assert_eq!(runtime.tool_dispatcher().agent(session), identity.id);
    }
    assert!(!provider.requests()[1].messages.iter().any(|m| {
        apeireth_protocol::canonical::ContentPart::join_text(&m.content)
            .contains("first private history")
    }));
    host.shutdown().await.unwrap();
    assert!(first.await.unwrap().is_err());
    assert_eq!(runtime.managed_main().unwrap().status, AgentStatus::Stopped);
    let usage = runtime.managed_main().unwrap().usage;
    assert_eq!(usage.requests, 2);
    assert_eq!(usage.cancellations, 1);
    assert_eq!(host.usage(Some(&AgentOwner::Host("main".into()))), usage);
    assert!(runtime
        .execute_outcome(TurnRequest::new(second_session, "closed"))
        .await
        .is_err());
}

#[tokio::test]
async fn managed_main_cannot_adopt_another_instance_session_or_reset_its_budget() {
    let provider = Provider::new(vec![]);
    let host = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let other = host
        .spawn_agent("other", AgentConfig::default())
        .await
        .unwrap();
    let runtime = host
        .into_main_runtime(AgentConfig {
            budget: apeireth_plugin::agents::AgentBudget {
                max_requests: 1,
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(runtime
        .execute_outcome(TurnRequest::new(other.snapshot().session, "steal session"))
        .await
        .is_err());
    assert!(provider.requests().is_empty());
    runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "allowed"))
        .await
        .unwrap();
    assert!(runtime
        .execute_outcome(TurnRequest::new(
            SessionId::new(),
            "budget cannot reset per session"
        ))
        .await
        .is_err());
    assert_eq!(provider.requests().len(), 1);
    assert_eq!(runtime.managed_main().unwrap().usage.requests, 1);
    runtime.agent_runtime().unwrap().shutdown().await.unwrap();
}

#[tokio::test]
async fn session_quota_main_preserves_unknown_usage_while_finite_and_shared_budgets_fail_closed() {
    for (session_quotas, shared_budget) in [(false, false), (true, false), (true, true)] {
        let provider = Provider::new(vec![]);
        provider.set_usage(Default::default());
        let host = builder(provider.clone())
            .build_agent_runtime(AgentHostPolicy {
                runtime_budget: shared_budget.then_some(
                    apeireth_runtime::canonical::accounting::SharedUsageBudget {
                        max_requests: 1,
                        max_tokens: 100,
                        max_tool_calls: 10,
                        max_cost_micros: None,
                    },
                ),
                ..Default::default()
            })
            .await
            .unwrap();
        let config = AgentConfig::default();
        let runtime = if session_quotas {
            host.into_main_runtime_with_session_quotas(config)
                .await
                .unwrap()
        } else {
            host.into_main_runtime(config).await.unwrap()
        };
        let session = SessionId::new();
        runtime
            .execute_outcome(TurnRequest::new(session, "first unknown usage"))
            .await
            .unwrap();
        let second = runtime
            .execute_outcome(TurnRequest::new(session, "second unknown usage"))
            .await;
        assert_eq!(second.is_ok(), session_quotas && !shared_budget);
        let expected = if second.is_ok() { 2 } else { 1 };
        let usage = runtime.managed_main().unwrap().usage;
        assert_eq!(usage.unreported_usage_requests, expected);
        assert_eq!(usage.requests, expected);
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        runtime.agent_runtime().unwrap().shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn rich_main_honors_configured_model_and_deadline_and_preserves_cancelled_facts() {
    let provider = Provider::new(vec![]);
    let _blocked = provider.hold_next();
    let host = builder(provider.clone())
        .build_agent_runtime(AgentHostPolicy::default())
        .await
        .unwrap();
    let runtime = host
        .into_main_runtime(AgentConfig {
            model: Some("fixture".into()),
            budget: apeireth_plugin::agents::AgentBudget {
                turn_timeout_ms: 20,
                ..Default::default()
            },
            ..Default::default()
        })
        .await
        .unwrap();
    let result = runtime
        .execute_outcome(TurnRequest::new(SessionId::new(), "bounded"))
        .await;
    assert!(
        matches!(result, Err(apeireth_runtime::RuntimeError::AgentTask(error)) if error.code == apeireth_plugin::agents::AgentErrorCode::Timeout)
    );
    let main = runtime.managed_main().unwrap();
    assert_eq!(main.usage.requests, 1);
    assert_eq!(main.usage.cancellations, 1);
    assert_eq!(provider.requests()[0].model, "fixture");
    runtime.agent_runtime().unwrap().shutdown().await.unwrap();
}
