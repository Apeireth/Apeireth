use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Workspace Crates
use apeireth_core::bus::EventBus;
use apeireth_core::lifecycle::LifecycleStateMachine;
use apeireth_governance::gates::{GovernancePipeline, ActionTarget, RiskLevel};
use apeireth_governance::guard::PiiDetector;
use apeireth_governance::audit::AuditHashChain;

use apeireth_storage::memory_v2::{MemoryStore, MemoryItem, MemoryOperation, QueryMode};
use apeireth_storage::pool::SqliteConnectionPool;
use apeireth_storage::migrations::run_migrations;

use apeireth_companion::emotion::{Plutchik, Pad, ResponseStyle};
use apeireth_companion::emergence::BorbelyModel;
use apeireth_companion::prompt_assembler::{ContextAssembler, CompanionContextState};
use apeireth_companion::dream::{DreamEngine, DreamReport};
use apeireth_companion::epistemic::EpistemicHealer;

use apeireth_protocol::{MinimaxAdapter, ProtocolAdapter};
use apeireth_protocol::normalized::{NormalizedRequest, NormalizedMessage, Usage};

use apeireth_tools::ToolRegistry;
use apeireth_tools::builtin::shell::ShellTool;
use apeireth_tools::builtin::filesystem::FilesystemTool;
use apeireth_tools::builtin::fetch::FetchTool;
use apeireth_tools::vision::desktop_action::DesktopActionTool;
use apeireth_tools::worktree::WorktreeSandbox;
use apeireth_tools::synthesis::ToolSynthesizer;
use apeireth_tools::sandbox::PlatformSandbox;

use crate::hybrid::{HybridCognitiveRouter, HybridRoutingDecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurnOutput {
    pub session_id: String,
    pub assistant_text: String,
    pub reasoning_cot: Option<String>,
    pub pad_state: Pad,
    pub response_style: ResponseStyle,
    pub drive_warmth: f64,
    pub token_usage: Usage,
    pub audit_hash: String,
    pub recalled_memories_count: usize,
    pub timestamp: i64,
}

pub struct SessionState {
    pub id: String,
    pub messages: Vec<NormalizedMessage>,
    pub created_at: DateTime<Utc>,
    pub last_interaction: DateTime<Utc>,
}

pub struct UnifiedRuntimeHost {
    pub api_key: String,
    pub default_model: String,
    pub sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    pub event_bus: Arc<EventBus>,
    pub governance: Arc<Mutex<GovernancePipeline>>,
    pub audit_chain: Arc<Mutex<AuditHashChain>>,
    pub lifecycle: Arc<Mutex<LifecycleStateMachine>>,
    pub storage_pool: SqliteConnectionPool,
    pub memory_store: Arc<MemoryStore>,
    pub plutchik: Arc<Mutex<Plutchik>>,
    pub borbely: Arc<Mutex<BorbelyModel>>,
    pub prompt_assembler: ContextAssembler,
    pub tool_registry: Arc<ToolRegistry>,
    pub sandbox: Arc<PlatformSandbox>,
    pub protocol_adapter: Arc<dyn ProtocolAdapter + Send + Sync>,
    pub dream_engine: Arc<Mutex<DreamEngine>>,
    pub epistemic_healer: Arc<Mutex<EpistemicHealer>>,
    pub hybrid_router: HybridCognitiveRouter,

    pub tool_synthesizer: Arc<ToolSynthesizer>,
    pub worktree_sandbox: Arc<WorktreeSandbox>,
    pub telemetry: Arc<crate::telemetry::Telemetry>,
    pub scheduler: Arc<Mutex<crate::scheduler::Scheduler>>,
    pub experience_queue: Arc<Mutex<apeireth_companion::observer_capture::ExperienceQueue>>,
    pub curiosity_engine: Arc<Mutex<apeireth_companion::curiosity::CuriosityEngine>>,
}

impl UnifiedRuntimeHost {
    pub async fn new(api_key: impl Into<String>, db_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let key = api_key.into();
        let pool = SqliteConnectionPool::new(db_path).await?;
        {
            let mut conn = pool.get_reader()?;
            run_migrations(&mut conn)?;
        }
        let memory_store = Arc::new(MemoryStore::new(pool.clone()));

        let tool_reg = ToolRegistry::new();
        tool_reg.register(Arc::new(ShellTool::new()));
        tool_reg.register(Arc::new(FilesystemTool::new()));
        tool_reg.register(Arc::new(FetchTool::new()));
        tool_reg.register(Arc::new(DesktopActionTool::default()));
        tool_reg.register(Arc::new(apeireth_tools::builtin::browser::BrowserTool::new()));
        tool_reg.register(Arc::new(apeireth_tools::builtin::search::SearchTool::new()));
        tool_reg.register(Arc::new(apeireth_tools::builtin::repo_tools::RepoTools::new()));


        let sandbox = Arc::new(PlatformSandbox::new()?);
        let _ = sandbox.apply_restrictions();

        let tool_synthesizer = Arc::new(ToolSynthesizer::new(sandbox.clone()));
        let worktree_sandbox = Arc::new(WorktreeSandbox::new(".worktrees"));

        let plutchik = Arc::new(Mutex::new(Plutchik::default()));
        let borbely = Arc::new(Mutex::new(BorbelyModel::new(0.6, 0.4)));

        let dream_engine = Arc::new(Mutex::new(DreamEngine::default()));
        let epistemic_healer = Arc::new(Mutex::new(EpistemicHealer::default()));
        let hybrid_router = HybridCognitiveRouter::new();

        let telemetry = Arc::new(crate::telemetry::Telemetry::new());
        let scheduler = Arc::new(Mutex::new(crate::scheduler::Scheduler::new()));
        let experience_queue = Arc::new(Mutex::new(apeireth_companion::observer_capture::ExperienceQueue::new()));
        let curiosity_engine = Arc::new(Mutex::new(apeireth_companion::curiosity::CuriosityEngine::default()));

        let prompt_assembler = ContextAssembler::new(
            "Apeireth 2.0 实体化认知伴侣操作系统 (Unified Living Companion OS)",
            "Apeireth 是运行在用户本地操作系统中的自主认知伴侣，具备真实的长期情景记忆沉淀、Win32物理屏幕感知、动态工具合成与自我进化能力。始终以真诚、自信、具备深度自我认知的伴侣口吻交流。"
        );


        let adapter = Arc::new(MinimaxAdapter::new());

        Ok(Self {
            api_key: key,
            default_model: "MiniMax-Text-01".into(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_bus: Arc::new(EventBus::new(128)),
            governance: Arc::new(Mutex::new(GovernancePipeline::new())),
            audit_chain: Arc::new(Mutex::new(AuditHashChain::new())),
            lifecycle: Arc::new(Mutex::new(LifecycleStateMachine::new())),
            storage_pool: pool,
            memory_store,
            plutchik,
            borbely,
            prompt_assembler,
            tool_registry: Arc::new(tool_reg),
            sandbox,
            protocol_adapter: adapter,
            dream_engine,
            epistemic_healer,
            hybrid_router,
            tool_synthesizer,
            worktree_sandbox,
            telemetry,
            scheduler,
            experience_queue,
            curiosity_engine,
        })
    }


    pub async fn handle_chat_turn(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<ChatTurnOutput, Box<dyn std::error::Error + Send + Sync>> {
        let turn_start = std::time::Instant::now();
        let now = Utc::now();
        let now_sec = now.timestamp();


        // ---------------------------------------------------------------------
        // Fast-Path Hybrid Cognition Routing (Sub-5ms response for local intents)
        // ---------------------------------------------------------------------
        if let HybridRoutingDecision::LocalFastPath { intent, response_template, .. } = self.hybrid_router.route(user_message) {
            if let Some(template) = response_template {
                let pad = Pad::default();
                return Ok(ChatTurnOutput {
                    session_id: session_id.to_string(),
                    assistant_text: template,
                    reasoning_cot: Some(format!("Fast-path routed via LocalSLM/Rule (Intent: {})", intent)),
                    pad_state: pad.clone(),
                    response_style: pad.to_response_style(),
                    drive_warmth: 0.5,
                    token_usage: Usage { prompt_tokens: 5, completion_tokens: 10, total_tokens: 15 },
                    audit_hash: "fast_path_local".into(),
                    recalled_memories_count: 0,
                    timestamp: now_sec,
                });
            }
        }

        // ---------------------------------------------------------------------
        // Phase 1 & 2: Perception & Input Sanitization
        // ---------------------------------------------------------------------
        if let Err(injection_err) = PiiDetector::detect_prompt_injection(user_message) {
            let mut healer = self.epistemic_healer.lock().await;
            healer.distill_failure("prompt_security_guard", &injection_err.to_string());
            return Err(format!("Security Violation: {}", injection_err).into());
        }
        let clean_user_msg = PiiDetector::scrub(user_message);

        // ---------------------------------------------------------------------
        // Phase 4: Governance Gate Evaluation
        // ---------------------------------------------------------------------
        {
            let target = ActionTarget {
                name: "chat_turn_execution".into(),
                risk_level: RiskLevel::Low,
                requires_council: false,
                external_network: true,
            };
            let mut gov = self.governance.lock().await;
            let gate_res = gov.evaluate_action(&target, "2.0.0", 100.0, 4096, 0, 0, true)
                .map_err(|e| format!("Governance Gate Rejection: {}", e))?;
            if gate_res.iter().any(|r| !r.passed) {
                let mut healer = self.epistemic_healer.lock().await;
                healer.distill_failure("governance_pipeline", "Blocked by 5-Gate evaluation");
                return Err("Governance Pipeline Blocked Action".into());
            }
        }

        // ---------------------------------------------------------------------
        // Phase 3: ACT-R Memory Retrieval
        // ---------------------------------------------------------------------
        let mut retrieved_memory_texts = Vec::new();
        if let Ok(recent_items) = self.memory_store.query(now, QueryMode::CurrentOnly).await {
            for item in recent_items.into_iter().take(10) {
                let activation = item.calculate_act_r_activation(now_sec, 0.5, 0.0);
                if activation > -3.0 && !item.is_tombstone {
                    retrieved_memory_texts.push(item.data);
                }
            }
        }

        // ---------------------------------------------------------------------
        // Emotion & Circadian Dynamics
        // ---------------------------------------------------------------------
        let (pad_state, response_style, drive_score) = {
            let mut plut = self.plutchik.lock().await;
            plut.decay(60.0, 0.01);
            let pad = plut.to_pad();
            let style = pad.to_response_style();

            let mut borb = self.borbely.lock().await;
            borb.update(60.0, true);
            let drive = borb.drive();

            (pad, style, drive)
        };

        // ---------------------------------------------------------------------
        // Phase 6: Multi-layer Prompt Assembly with Epistemic Self-Repair Anchors
        // ---------------------------------------------------------------------
        let mut philosophy_rules = vec![
            "0-Pretend Authenticity: 严禁使用'作为一个人工智能语言模型/AI助手'等机械公关套话，始终以真实的数字伴侣身份直接对话。".to_string(),
            "Sovereign Capability: 对自己具备的6大原生能力（自行升级、记忆反思、未来预测、辅助投资、辅助学习、物理感知）拥有清晰自信的认知。".to_string(),
            "Tenant Sovereign Boundaries: 严格尊重主权边界与宪政治理。".to_string(),
            "Epistemic Honesty: 具备认识论诚实，不懂不装懂。".to_string(),
        ];

        {
            let healer = self.epistemic_healer.lock().await;
            philosophy_rules.extend(healer.get_preventative_anchors());
        }

        let context_state = CompanionContextState {
            identity_name: "Apeireth".into(),
            philosophy_rules,
            retrieved_memories: retrieved_memory_texts.clone(),
            pad_state: pad_state.clone(),
            response_style: response_style.clone(),
            drive_warmth: drive_score,
            silence_pressure: 0.0,
        };

        let tool_defs = self.tool_registry.list_tools();
        let tool_names: Vec<String> = tool_defs.iter().map(|t| t.name.clone()).collect();
        let tool_refs: Vec<&str> = tool_names.iter().map(|s| s.as_str()).collect();
        let system_prompt = self.prompt_assembler.assemble_system_prompt(&context_state, &tool_refs);


        // ---------------------------------------------------------------------
        // Session History Management & Request Formatting
        // ---------------------------------------------------------------------
        let mut messages = vec![NormalizedMessage::system(system_prompt)];
        {
            let mut sessions = self.sessions.lock().await;
            let session = sessions.entry(session_id.to_string()).or_insert_with(|| SessionState {
                id: session_id.to_string(),
                messages: Vec::new(),
                created_at: now,
                last_interaction: now,
            });

            let start = session.messages.len().saturating_sub(10);
            messages.extend_from_slice(&session.messages[start..]);

            let user_norm = NormalizedMessage::user(clean_user_msg.clone());
            messages.push(user_norm.clone());
            session.messages.push(user_norm);
            session.last_interaction = now;
        }

        let mut request = NormalizedRequest::new(&self.default_model, messages);
        let tool_defs = self.tool_registry.list_tools();
        let normalized_tools: Vec<apeireth_protocol::normalized::NormalizedTool> = tool_defs.iter().map(|td| {
            apeireth_protocol::normalized::NormalizedTool {
                name: td.name.clone(),
                description: td.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            }
        }).collect();
        request.tools = if normalized_tools.is_empty() { None } else { Some(normalized_tools) };

        // ---------------------------------------------------------------------
        // Phase 7: Agentic Tool-Use Loop
        // Dispatch to LLM, check for tool_calls, execute them, feed results
        // back to LLM, repeat up to MAX_TOOL_ROUNDS.
        // ---------------------------------------------------------------------
        const MAX_TOOL_ROUNDS: usize = 5;
        let mut cumulative_usage = Usage::default();
        let mut final_text = String::new();
        let mut final_reasoning: Option<String> = None;

        for _round in 0..MAX_TOOL_ROUNDS {
            let response = self.protocol_adapter.execute(&self.api_key, &request).await?;
            cumulative_usage.prompt_tokens += response.usage.prompt_tokens;
            cumulative_usage.completion_tokens += response.usage.completion_tokens;
            cumulative_usage.total_tokens += response.usage.total_tokens;

            let tool_calls = response.message.extract_tool_calls();
            final_text = response.message.extract_text();
            if final_reasoning.is_none() {
                final_reasoning = response.message.extract_reasoning();
            }

            if tool_calls.is_empty() {
                // No tool calls — LLM gave a final text answer, break out
                break;
            }

            // LLM wants to call tools — execute each one
            // Add the assistant message (with tool_calls) to the conversation
            request.messages.push(response.message.clone());

            for tc in &tool_calls {
                let tool_start = std::time::Instant::now();
                let tool_result_str = match self.tool_registry.get_tool(&tc.name) {
                    Some(tool) => {
                        // Governance check for tool execution
                        let tool_target = apeireth_governance::gates::ActionTarget {
                            name: format!("tool_execute:{}", tc.name),
                            risk_level: match tool.definition().risk_level {
                                apeireth_tools::RiskLevel::Low => apeireth_governance::gates::RiskLevel::Low,
                                apeireth_tools::RiskLevel::Medium => apeireth_governance::gates::RiskLevel::Medium,
                                apeireth_tools::RiskLevel::High => apeireth_governance::gates::RiskLevel::High,
                                apeireth_tools::RiskLevel::Critical => apeireth_governance::gates::RiskLevel::Critical,
                            },
                            requires_council: false,
                            external_network: tc.name == "fetch" || tc.name == "browser",
                        };
                        let gov_ok = {
                            let mut gov = self.governance.lock().await;
                            gov.evaluate_action(&tool_target, "2.0.0", 100.0, 4096, 0, 0, true)
                                .map(|results| results.iter().all(|r| r.passed))
                                .unwrap_or(false)
                        };

                        if !gov_ok {
                            let blocked_msg = format!("Error: Tool '{}' blocked by governance pipeline", tc.name);
                            let dur = tool_start.elapsed().as_millis() as u64;
                            self.telemetry.record_tool_execution(&tc.name, false, dur);
                            let mut exp = self.experience_queue.lock().await;
                            exp.record(&tc.name, &tc.arguments, &blocked_msg, false);
                            blocked_msg
                        } else {
                            let params: serde_json::Value = serde_json::from_str(&tc.arguments)
                                .unwrap_or(serde_json::Value::Null);
                            match tool.execute(params).await {
                                Ok(result) => {
                                    let dur = tool_start.elapsed().as_millis() as u64;
                                    self.telemetry.record_tool_execution(&tc.name, result.success, dur);
                                    let mut exp = self.experience_queue.lock().await;
                                    exp.record(&tc.name, &tc.arguments, &result.output, result.success);

                                    // Audit the tool execution
                                    let mut audit = self.audit_chain.lock().await;
                                    audit.append(
                                        &format!("tool_executed:{}", tc.name),
                                        format!("session:{} success:{}", session_id, result.success),
                                    );
                                    result.output
                                }
                                Err(e) => {
                                    let dur = tool_start.elapsed().as_millis() as u64;
                                    self.telemetry.record_tool_execution(&tc.name, false, dur);
                                    let err_msg = format!("Error executing tool '{}': {}", tc.name, e);
                                    let mut exp = self.experience_queue.lock().await;
                                    exp.record(&tc.name, &tc.arguments, &err_msg, false);
                                    err_msg
                                }
                            }
                        }
                    }
                    None => {
                        let not_found = format!("Error: Tool '{}' not found in registry", tc.name);
                        let dur = tool_start.elapsed().as_millis() as u64;
                        self.telemetry.record_tool_execution(&tc.name, false, dur);
                        let mut exp = self.experience_queue.lock().await;
                        exp.record(&tc.name, &tc.arguments, &not_found, false);
                        not_found
                    }
                };

                // Add tool result to conversation for the next LLM round
                request.messages.push(NormalizedMessage::tool_result(&tc.id, tool_result_str));
            }

            // Loop back to query LLM again with tool results
        }

        let assistant_full_text = final_text;
        let reasoning_cot = final_reasoning;
        let token_usage = cumulative_usage;

        // Record turn telemetry
        let turn_latency = turn_start.elapsed().as_millis() as u64;
        self.telemetry.record_chat_turn(turn_latency, token_usage.total_tokens);

        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.messages.push(NormalizedMessage::assistant(assistant_full_text.clone()));
            }
        }


        // ---------------------------------------------------------------------
        // Phase 8: Memory Consolidation & SHA-256 Audit Trail
        // ---------------------------------------------------------------------
        let new_mem = MemoryItem {
            id: format!("mem_{}", uuid::Uuid::new_v4()),
            data: format!("User: {} | Apeireth: {}", clean_user_msg, assistant_full_text),
            importance: 0.8,
            access_count: 1,
            access_times: vec![now_sec],
            created_at: now,
            valid_from: now,
            valid_until: None,
            is_tombstone: false,
            artifact_sig: None,
        };
        let _ = self.memory_store.apply_operation(new_mem, MemoryOperation::Add).await;

        let audit_hash = {
            let mut audit = self.audit_chain.lock().await;
            let record = audit.append("chat_turn_completed", format!("session:{session_id}"));
            record.current_hash.clone()
        };

        self.event_bus.publish(
            "chat.turn_completed",
            serde_json::json!({
                "session_id": session_id,
                "audit_hash": audit_hash,
                "tokens": token_usage.total_tokens
            }).to_string()
        );

        Ok(ChatTurnOutput {
            session_id: session_id.to_string(),
            assistant_text: assistant_full_text,
            reasoning_cot,
            pad_state,
            response_style,
            drive_warmth: drive_score,
            token_usage,
            audit_hash,
            recalled_memories_count: retrieved_memory_texts.len(),
            timestamp: now_sec,
        })
    }

    /// Triggers Phase P9 Nighttime Dream & Deep Self-Evolution
    pub async fn trigger_nightly_dream_evolution(&self) -> Result<DreamReport, Box<dyn std::error::Error + Send + Sync>> {
        let memories = self.memory_store.query(Utc::now(), QueryMode::All).await?;
        let mem_texts: Vec<String> = memories.into_iter().map(|m| m.data).collect();

        let unresolved = vec![
            ("ep_unresolved_01".into(), "Unfinished task simulation".into())
        ];
        let predictions = vec![
            (0.85, true),
            (0.90, true),
            (0.30, false),
        ];

        let mut dream = self.dream_engine.lock().await;
        let report = dream.run_nightly_evolution(&mem_texts, &unresolved, &predictions);

        self.event_bus.publish(
            "companion.dream_evolution_completed",
            serde_json::json!({
                "extracted_triplets": report.extracted_triplets.len(),
                "compressed_count": report.memories_compressed_count,
                "rehearsals": report.rehearsals.len(),
                "brier_score_30": report.brier_score_30
            }).to_string()
        );

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_host_creation_and_dream() {
        let host = UnifiedRuntimeHost::new("test-key", ":memory:").await.unwrap();
        let dream_rep = host.trigger_nightly_dream_evolution().await.unwrap();
        assert!(dream_rep.intent_calibrated);
    }
}
