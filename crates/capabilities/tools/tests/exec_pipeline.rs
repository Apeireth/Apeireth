//! The five-stage tool execution pipeline, exercised end to end.
//!
//! Covers the acceptance set: guard monotonicity (a refusal is never flipped),
//! pre-execute waterfall order semantics, the post-execute three verdicts with
//! the original result archived, output-contract validation, timeout through
//! the deadline timer library, unchanged default behaviour, the fixed stage
//! order, and the closed failure vocabulary.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::deadline::live_deadline_timers;
use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ApprovalPolicyEngine, Decision};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_tools_canonical::{
    AroundPolicy, CommandFamilyGateHook, ExecutionRecord, ExecutionStage, MonotonicGuards,
    NormalizationError, OutputSchema, PipelineFailure, PipelinedCapability, PostExecuteHook,
    PostExecuteRequest, PostVerdict, PreExecuteHook, PreExecuteRequest, PreVerdict, RetryPolicy,
    RiskLevelGateHook, SchemaKind, ToolExecutionPipeline, ToolGuard, ToolGuardRequest,
    ToolGuardrail, ToolOutcome,
};

// ---------------------------------------------------------------- test stubs

enum StubBehavior {
    Success(serde_json::Value),
    Failure(String),
}

struct StubTool {
    id: CapabilityId,
    name: &'static str,
    runs: AtomicU32,
    delay: Duration,
    behavior: StubBehavior,
}

impl StubTool {
    fn new(name: &'static str, behavior: StubBehavior, delay: Duration) -> Self {
        Self {
            id: CapabilityId::new(format!("tool.{name}")).unwrap(),
            name,
            runs: AtomicU32::new(0),
            delay,
            behavior,
        }
    }

    fn runs(&self) -> u32 {
        self.runs.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ToolCapability for StubTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: self.name.into(),
            description: Some("execution pipeline test stub".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.runs.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        match &self.behavior {
            StubBehavior::Success(value) => ToolResult::ok(call.id.clone(), value.clone()),
            StubBehavior::Failure(message) => {
                ToolResult::retryable_error(call.id.clone(), message.clone())
            }
        }
    }
}

fn call() -> ToolCall {
    ToolCall {
        id: "call_7".into(),
        name: "demo".into(),
        arguments: serde_json::json!({}),
    }
}

fn succeeding_tool() -> StubTool {
    StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "done": true })),
        Duration::ZERO,
    )
}

struct FixedPre {
    name: &'static str,
    verdict: PreVerdict,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl FixedPre {
    fn new(
        name: &'static str,
        verdict: PreVerdict,
        log: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            verdict,
            log: Arc::clone(log),
        })
    }
}

impl PreExecuteHook for FixedPre {
    fn name(&self) -> &str {
        self.name
    }

    fn pre_verdict(&self, _request: &PreExecuteRequest<'_>) -> PreVerdict {
        self.log.lock().unwrap().push(self.name);
        self.verdict.clone()
    }
}

struct FixedGuard {
    name: &'static str,
    refusal: Option<String>,
}

impl ToolGuard for FixedGuard {
    fn name(&self) -> &str {
        self.name
    }

    fn deny(&self, _request: &ToolGuardRequest<'_>) -> Option<String> {
        self.refusal.clone()
    }
}

struct FixedPost {
    name: &'static str,
    verdict: PostVerdict,
}

impl PostExecuteHook for FixedPost {
    fn name(&self) -> &str {
        self.name
    }

    fn post_verdict(&self, _request: &PostExecuteRequest<'_>) -> PostVerdict {
        self.verdict.clone()
    }
}

// ---------------------------------------------------------------- acceptance

#[tokio::test]
async fn guard_refusal_is_monotonic_and_never_flipped_back() {
    let tool = succeeding_tool();
    let refusing = || {
        Arc::new(FixedGuard {
            name: "refusing",
            refusal: Some("policy refusal".into()),
        })
    };
    let silent = || {
        Arc::new(FixedGuard {
            name: "silent",
            refusal: None,
        })
    };

    // Whichever order they listen in, and even though the silent guard has no
    // objection, the refusal stands and nothing executes.
    let chains = [
        MonotonicGuards::new().with(refusing()).with(silent()),
        MonotonicGuards::new().with(silent()).with(refusing()),
    ];
    for guards in chains {
        let pipeline = ToolExecutionPipeline::new().with_guards(guards);
        let executed = pipeline.run(&tool, &call()).await;
        assert_eq!(
            executed.failure,
            Some(PipelineFailure::GuardDenied {
                guard: "refusing".into(),
                reason: "policy refusal".into(),
            })
        );
        assert!(!executed.result.is_ok(), "a refusal is a failure frame");
        assert_eq!(tool.runs(), 0, "nothing may execute after a guard refusal");
        assert_eq!(
            executed.record.stages(),
            [ExecutionStage::PreExecute, ExecutionStage::Guard],
            "a refusal closes the run at the guard stage"
        );
    }
}

#[tokio::test]
async fn pre_waterfall_stops_at_the_first_non_allow_verdict_in_order() {
    let tool = succeeding_tool();
    let log = Arc::new(Mutex::new(Vec::new()));
    let pipeline = ToolExecutionPipeline::new()
        .with_pre_hook(FixedPre::new("one", PreVerdict::Allow, &log))
        .with_pre_hook(FixedPre::new(
            "two",
            PreVerdict::Deny {
                reason: "second hook refuses".into(),
            },
            &log,
        ))
        .with_pre_hook(FixedPre::new("three", PreVerdict::Allow, &log));

    let executed = pipeline.run(&tool, &call()).await;
    assert_eq!(
        executed.failure,
        Some(PipelineFailure::PreDenied {
            source: "two".into(),
            reason: "second hook refuses".into(),
        })
    );
    assert_eq!(
        *log.lock().unwrap(),
        ["one", "two"],
        "the walk stops at the first non-allow verdict; later hooks never run"
    );
    assert_eq!(tool.runs(), 0);

    // A refusal in front of an allow is just as final.
    let log = Arc::new(Mutex::new(Vec::new()));
    let pipeline = ToolExecutionPipeline::new()
        .with_pre_hook(FixedPre::new(
            "refusing",
            PreVerdict::Cancel {
                reason: "going away".into(),
            },
            &log,
        ))
        .with_pre_hook(FixedPre::new("late-allow", PreVerdict::Allow, &log));
    let executed = pipeline.run(&tool, &call()).await;
    assert_eq!(
        executed.failure,
        Some(PipelineFailure::PreCancelled {
            source: "refusing".into(),
            reason: "going away".into(),
        })
    );
    assert_eq!(*log.lock().unwrap(), ["refusing"]);
}

#[tokio::test]
async fn folded_in_gates_keep_their_existing_judgment_semantics() {
    let call = call();

    // Risk-level gate: the existing rule engine decides, the waterfall only
    // carries its verdict. Each mapping is checked against the engine's own
    // decision, verbatim.
    let blacklist_engine = {
        let mut engine = ApprovalPolicyEngine::new();
        engine.blacklist.insert("tool.demo".into());
        engine
    };
    let trust_engine = {
        let mut engine = ApprovalPolicyEngine::new();
        engine.trusted.insert("tool.demo".into());
        engine
    };
    for (capability_name, engine) in [
        ("system.exec", ApprovalPolicyEngine::new()),
        ("tool.demo", blacklist_engine),
        ("tool.demo", trust_engine),
    ] {
        let capability = CapabilityId::new(capability_name).unwrap();
        let (decision, _detail) = engine.evaluate(capability_name, &call.arguments, &[], 0);
        let hook = RiskLevelGateHook::new(engine);
        assert_eq!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::from(decision),
            "the risk-level gate's judgment must pass through unchanged"
        );
    }
    assert_eq!(
        PreVerdict::from(Decision::require_approval("needs a human")),
        PreVerdict::Ask {
            reason: "needs a human".into()
        }
    );

    // Command-family determination: the existing pre-call command check
    // decides; its refusal text is carried verbatim.
    let capability = CapabilityId::new("tool.shell").unwrap();
    let gate = CommandFamilyGateHook::new();
    for command in [
        "rm -rf / --no-preserve-root",
        "netsh advfirewall set allprofiles off",
    ] {
        let mut call = call.clone();
        call.arguments = serde_json::json!({ "command": command });
        let expected = ToolGuardrail::verify_shell_command(command)
            .expect_err("the command is refused today")
            .to_string();
        assert_eq!(
            gate.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Deny {
                reason: expected.clone()
            },
            "the command-family refusal must be the very same judgment"
        );
    }
    let mut call = call.clone();
    call.arguments = serde_json::json!({ "command": "cargo test --workspace" });
    assert_eq!(
        gate.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
        PreVerdict::Allow
    );
}

#[tokio::test]
async fn post_accept_keeps_the_executed_result_unchanged() {
    let tool = succeeding_tool();
    let call = call();
    let direct = tool.invoke(&call).await;

    let pipeline = ToolExecutionPipeline::new().with_post_hook(Arc::new(FixedPost {
        name: "reviewer",
        verdict: PostVerdict::Accept,
    }));
    let executed = pipeline.run(&tool, &call).await;

    assert_eq!(executed.result, direct);
    assert_eq!(executed.failure, None);
    assert!(executed.record.superseded().is_empty());
    assert_eq!(executed.outcome, ToolOutcome::freeze(&direct));
}

#[tokio::test]
async fn post_replace_supersedes_the_result_and_archives_the_original() {
    let tool = StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "wrong": 1 })),
        Duration::ZERO,
    );
    let call = call();
    let original = tool.invoke(&call).await;

    let corrected = ToolResult::ok(call.id.clone(), serde_json::json!({ "fixed": true }));
    let pipeline = ToolExecutionPipeline::new().with_post_hook(Arc::new(FixedPost {
        name: "corrector",
        verdict: PostVerdict::Replace {
            corrected: corrected.clone(),
            note: "repaired the shape".into(),
        },
    }));
    let executed = pipeline.run(&tool, &call).await;

    assert_eq!(executed.result, corrected);
    assert_eq!(executed.result.tool_call_id, call.id);
    assert_eq!(executed.failure, None);
    let archived = executed.record.superseded();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].stage, ExecutionStage::PostExecute);
    assert_eq!(archived[0].hook, "corrector");
    assert_eq!(archived[0].original, original, "the original is archived");
    assert_eq!(archived[0].note, "repaired the shape");
}

#[tokio::test]
async fn post_block_judges_the_call_a_failure_and_archives_the_original() {
    let tool = succeeding_tool();
    let call = call();
    let original = tool.invoke(&call).await;

    let pipeline = ToolExecutionPipeline::new().with_post_hook(Arc::new(FixedPost {
        name: "blocker",
        verdict: PostVerdict::Block {
            reason: "output judged unsafe".into(),
        },
    }));
    let executed = pipeline.run(&tool, &call).await;

    assert_eq!(
        executed.failure,
        Some(PipelineFailure::PostBlocked {
            source: "blocker".into(),
            reason: "output judged unsafe".into(),
        })
    );
    assert!(!executed.result.is_ok());
    assert!(executed.result.render().contains("pipeline.post_block"));
    let archived = executed.record.superseded();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].original, original, "the original is archived");
}

#[tokio::test]
async fn output_contract_reports_shape_drift_instead_of_swallowing_it() {
    let schema = OutputSchema::new().require("summary", SchemaKind::String);
    let call = call();

    let conforming = StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "summary": "fine" })),
        Duration::ZERO,
    );
    let executed = ToolExecutionPipeline::new()
        .with_output_schema(schema.clone())
        .run(&conforming, &call)
        .await;
    assert_eq!(executed.failure, None);
    assert!(executed.outcome.ok);

    let drifted = StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "count": 1 })),
        Duration::ZERO,
    );
    let executed = ToolExecutionPipeline::new()
        .with_output_schema(schema)
        .run(&drifted, &call)
        .await;
    assert_eq!(
        executed.failure,
        Some(PipelineFailure::ContractViolation(
            NormalizationError::SchemaMismatch {
                field: "summary".into(),
                expected: "string".into(),
                found: "missing".into(),
            }
        ))
    );
    assert!(
        !executed.result.is_ok(),
        "a contract violation is an error, not a silent pass"
    );
    assert!(executed
        .result
        .render()
        .contains("output_contract.schema_mismatch"));
    let archived = executed.record.superseded();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].stage, ExecutionStage::Normalize);
}

#[tokio::test]
async fn timeout_wraps_execution_through_the_deadline_timer() {
    // Timer counting is process-wide: serialize the deadline assertions.
    static TIMER_GUARD: Mutex<()> = Mutex::new(());
    let _guard = TIMER_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let baseline = live_deadline_timers();

    let slow = StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "done": true })),
        Duration::from_millis(500),
    );
    let pipeline =
        ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(40, 5_000));
    let executed = pipeline.run(&slow, &call()).await;

    assert_eq!(
        executed.failure,
        Some(PipelineFailure::Timeout {
            code: "timeout.deadline_expired",
        }),
        "expiry must surface through the deadline library's own code family"
    );
    assert!(executed
        .result
        .render()
        .contains("timeout.deadline_expired"));
    assert_eq!(slow.runs(), 1, "the attempt started and was cut off");

    // A refused timeout value never arms a timer and never executes.
    let pipeline =
        ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(0, 5_000));
    let executed = pipeline.run(&slow, &call()).await;
    assert_eq!(
        executed.failure,
        Some(PipelineFailure::Timeout {
            code: "timeout.invalid_timeout",
        })
    );
    assert_eq!(slow.runs(), 1, "a refused timeout value must not execute");

    // The armed timer is cleaned up with its owner.
    for _ in 0..400 {
        if live_deadline_timers() == baseline {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        live_deadline_timers(),
        baseline,
        "no timer may outlive its owner"
    );
}

#[tokio::test]
async fn default_pipeline_is_behaviorally_identical_to_direct_execution() {
    let call = call();
    for behavior in [
        StubBehavior::Success(serde_json::json!({ "done": true })),
        StubBehavior::Failure("it broke".into()),
    ] {
        let tool = StubTool::new("demo", behavior, Duration::ZERO);
        let direct = tool.invoke(&call).await;

        let executed = ToolExecutionPipeline::new().run(&tool, &call).await;
        assert_eq!(executed.result, direct, "no hook installed, no change");
        assert_eq!(executed.failure, None);
        assert_eq!(executed.outcome, ToolOutcome::freeze(&direct));
        assert!(executed.record.superseded().is_empty());

        // The capability wrapper emits exactly the wrapped tool's own result.
        let wrapped =
            PipelinedCapability::new(Arc::new(tool), Arc::new(ToolExecutionPipeline::new()));
        assert_eq!(wrapped.invoke(&call).await, direct);
    }
}

#[tokio::test]
async fn stages_run_in_their_fixed_order() {
    let tool = StubTool::new(
        "demo",
        StubBehavior::Success(serde_json::json!({ "summary": "fine" })),
        Duration::ZERO,
    );
    let log = Arc::new(Mutex::new(Vec::new()));
    let pipeline = ToolExecutionPipeline::new()
        .with_pre_hook(FixedPre::new("pre", PreVerdict::Allow, &log))
        .with_guard(Arc::new(FixedGuard {
            name: "guard",
            refusal: None,
        }))
        .with_around(AroundPolicy::new().with_retry(RetryPolicy::new(2)))
        .with_post_hook(Arc::new(FixedPost {
            name: "post",
            verdict: PostVerdict::Accept,
        }))
        .with_output_schema(OutputSchema::new().require("summary", SchemaKind::String));

    let executed = pipeline.run(&tool, &call()).await;
    assert_eq!(executed.failure, None);
    assert_eq!(
        executed.record.stages(),
        ExecutionStage::ALL.to_vec(),
        "the five stages run in exactly one order"
    );
    let visited: Vec<&str> = executed
        .record
        .entries()
        .iter()
        .map(|entry| entry.stage.as_str())
        .collect();
    assert_eq!(
        visited,
        [
            "pre_execute",
            "guard",
            "around",
            "post_execute",
            "normalize"
        ]
    );
}

#[test]
fn every_failure_lands_in_the_closed_error_vocabulary() {
    let failures = vec![
        PipelineFailure::PreDenied {
            source: "hook".into(),
            reason: "no".into(),
        },
        PipelineFailure::PreCancelled {
            source: "hook".into(),
            reason: "stop".into(),
        },
        PipelineFailure::NeedsApproval {
            source: "hook".into(),
            reason: "human".into(),
        },
        PipelineFailure::GuardDenied {
            guard: "guard".into(),
            reason: "no".into(),
        },
        PipelineFailure::Timeout {
            code: "timeout.deadline_expired",
        },
        PipelineFailure::RetriesExhausted {
            attempts: 2,
            last_error: "error: still broken".into(),
        },
        PipelineFailure::PostBlocked {
            source: "corrector".into(),
            reason: "blocked".into(),
        },
        PipelineFailure::ContractViolation(NormalizationError::NotStructured {
            found: "array".into(),
        }),
    ];

    let mut codes = Vec::new();
    for failure in &failures {
        let code = failure.code();
        assert!(!code.is_empty());
        assert!(!codes.contains(&code), "duplicate failure code {code}");
        codes.push(code);
        let emitted = failure.emit("call_7", Some("demo"));
        assert!(!emitted.is_ok(), "a failure closes into an error frame");
        assert_eq!(emitted.tool_call_id, "call_7");
        assert!(emitted.render().contains(code), "{}", emitted.render());
        assert!(!failure.message().is_empty());
    }
    assert_eq!(
        codes.len(),
        8,
        "the vocabulary is closed: eight failure kinds"
    );

    // The verdict vocabularies are closed sets too.
    assert_eq!(PreVerdict::Allow.label(), "allow");
    assert_eq!(
        PreVerdict::Deny {
            reason: String::new()
        }
        .label(),
        "deny"
    );
    assert_eq!(
        PreVerdict::Cancel {
            reason: String::new()
        }
        .label(),
        "cancel"
    );
    assert_eq!(
        PreVerdict::Ask {
            reason: String::new()
        }
        .label(),
        "ask"
    );
    assert_eq!(PostVerdict::Accept.label(), "accept");
    assert_eq!(
        PostVerdict::Replace {
            corrected: ToolResult::ok("c", serde_json::Value::Null),
            note: String::new(),
        }
        .label(),
        "replace"
    );
    assert_eq!(
        PostVerdict::Block {
            reason: String::new()
        }
        .label(),
        "block"
    );
}

#[test]
fn the_stage_record_is_inspectable_and_the_order_is_monotone() {
    let record = ExecutionRecord::new();
    assert!(record.entries().is_empty());
    assert!(record.stages().is_empty());
    assert!(record.superseded().is_empty());
    assert_eq!(
        ExecutionStage::ALL.map(|stage| stage.order()),
        [0, 1, 2, 3, 4],
        "the canonical order is monotone and complete"
    );
}
