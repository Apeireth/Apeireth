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

use apeireth_protocol::{MinimaxAdapter, ProtocolAdapter};
use apeireth_protocol::normalized::{NormalizedRequest, NormalizedMessage, Usage};

use apeireth_tools::ToolRegistry;
use apeireth_tools::builtin::shell::ShellTool;
use apeireth_tools::builtin::filesystem::FilesystemTool;
use apeireth_tools::builtin::fetch::FetchTool;
use apeireth_tools::sandbox::PlatformSandbox;

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

        let mut tool_reg = ToolRegistry::new();
        tool_reg.register(Arc::new(ShellTool::new()));
        tool_reg.register(Arc::new(FilesystemTool::new()));
        tool_reg.register(Arc::new(FetchTool::new()));

        let sandbox = Arc::new(PlatformSandbox::new()?);
        let _ = sandbox.apply_restrictions();

        let plutchik = Arc::new(Mutex::new(Plutchik::default()));
        let borbely = Arc::new(Mutex::new(BorbelyModel::new(0.6, 0.4)));

        let prompt_assembler = ContextAssembler::new(
            "Apeireth 2.0 Living Companion",
            "Apeireth is a sovereign, authentic, highly capable AI cognitive companion that communicates honestly and directly."
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
        })
    }

    pub async fn handle_chat_turn(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<ChatTurnOutput, Box<dyn std::error::Error + Send + Sync>> {
        let now = Utc::now();
        let now_sec = now.timestamp();

        // ---------------------------------------------------------------------
        // Phase 1 & 2: Perception & Input Sanitization
        // ---------------------------------------------------------------------
        if let Err(injection_err) = PiiDetector::detect_prompt_injection(user_message) {
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
        // Phase 6: Multi-layer Prompt Assembly
        // ---------------------------------------------------------------------
        let context_state = CompanionContextState {
            identity_name: "Apeireth".into(),
            philosophy_rules: vec![
                "0-Pretend Authenticity".into(),
                "Tenant Sovereign Boundaries".into(),
                "Epistemic Honesty".into(),
            ],
            retrieved_memories: retrieved_memory_texts.clone(),
            pad_state: pad_state.clone(),
            response_style: response_style.clone(),
            drive_warmth: drive_score,

            silence_pressure: 0.0,
        };

        let tools = vec!["shell", "filesystem", "fetch"];
        let system_prompt = self.prompt_assembler.assemble_system_prompt(&context_state, &tools);

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

            // Append prior session context (up to last 10 messages)
            let start = session.messages.len().saturating_sub(10);
            messages.extend_from_slice(&session.messages[start..]);

            // Add current user message
            let user_norm = NormalizedMessage::user(clean_user_msg.clone());
            messages.push(user_norm.clone());
            session.messages.push(user_norm);
            session.last_interaction = now;
        }

        let request = NormalizedRequest::new(&self.default_model, messages);

        // ---------------------------------------------------------------------
        // Phase 7: Live Protocol Dispatch & CoT Decomposition
        // ---------------------------------------------------------------------
        let response = self.protocol_adapter.execute(&self.api_key, &request).await?;
        let assistant_full_text = response.message.extract_text();
        let reasoning_cot = response.message.extract_reasoning();
        let token_usage = response.usage;

        // Append assistant response to session
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

        // Publish to EventBus
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_host_creation() {
        let host = UnifiedRuntimeHost::new("test-key", ":memory:").await;
        assert!(host.is_ok());
    }
}
