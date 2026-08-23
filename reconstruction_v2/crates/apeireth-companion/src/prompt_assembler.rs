use crate::emotion::{Pad, ResponseStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionContextState {
    pub identity_name: String,
    pub philosophy_rules: Vec<String>,
    pub retrieved_memories: Vec<String>,
    pub pad_state: Pad,
    pub response_style: ResponseStyle,
    pub drive_warmth: f64,
    pub silence_pressure: f64,
}

pub struct ContextAssembler {
    l0_identity: String,
    l1_story: String,
}

impl ContextAssembler {
    pub fn new(identity: &str, story: &str) -> Self {
        Self {
            l0_identity: identity.into(),
            l1_story: story.into(),
        }
    }

    pub fn assemble_system_prompt(&self, state: &CompanionContextState, tools: &[&str]) -> String {
        let mut prompt = String::new();

        // L0: Identity & Philosophy
        prompt.push_str(&format!("# SYSTEM IDENTITY: {}\n", state.identity_name));
        prompt.push_str(&format!("{}\n\n", self.l0_identity));
        prompt.push_str("## Core Philosophy Anchors (Non-Negotiable):\n");
        for rule in &state.philosophy_rules {
            prompt.push_str(&format!("- {}\n", rule));
        }

        // L1: Narrative & Persona
        prompt.push_str(&format!("\n## Narrative & Persona Context:\n{}\n", self.l1_story));

        // L2: Active Working Memory (ACT-R Top facts)
        if !state.retrieved_memories.is_empty() {
            prompt.push_str("\n## Retrieved Episodic & Semantic Memories:\n");
            for (idx, mem) in state.retrieved_memories.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", idx + 1, mem));
            }
        }

        // L3: Emotional State (PAD) & Response Style
        prompt.push_str(&format!(
            "\n## Dynamic Cognitive & Emotional State:\n- PAD Space: Pleasure={:.2}, Arousal={:.2}, Dominance={:.2}\n- Tone Modulation: {:?}\n- Drive: Warmth={:.2}, SilencePressure={:.2}\n",
            state.pad_state.pleasure, state.pad_state.arousal, state.pad_state.dominance,
            state.response_style, state.drive_warmth, state.silence_pressure
        ));

        // L4: Available Tools
        if !tools.is_empty() {
            prompt.push_str("\n## Available Tools:\n");
            for tool in tools {
                prompt.push_str(&format!("- {}\n", tool));
            }
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_assembler_full_layers() {
        let assembler = ContextAssembler::new("Apeireth 2.0 Autonomous Companion", "You are exploring codebase architectures.");
        let state = CompanionContextState {
            identity_name: "Apeireth".into(),
            philosophy_rules: vec!["0 Pretending".into(), "Sovereign containment".into()],
            retrieved_memories: vec!["User works on Rust concurrency".into()],
            pad_state: Pad { pleasure: 0.8, arousal: 0.5, dominance: 0.9 },
            response_style: ResponseStyle::Playful,
            drive_warmth: 0.65,
            silence_pressure: 0.1,
        };

        let prompt = assembler.assemble_system_prompt(&state, &["shell", "filesystem"]);
        assert!(prompt.contains("0 Pretending"));
        assert!(prompt.contains("User works on Rust concurrency"));
        assert!(prompt.contains("Pleasure=0.80"));
        assert!(prompt.contains("shell"));
    }
}

