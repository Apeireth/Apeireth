//! Care Potential Field (关怀势能场) — Apeireth 2.0+ Continuous Empathy Dynamics Engine.
//!
//! # Mathematical Foundations
//!
//! Models the companion's caring motivation as a continuous dynamic potential scalar $U_{\text{care}}(t) \ge 0$:
//! $$\frac{d U_{\text{care}}}{dt} = \alpha \mathcal{N}(t) + \beta \mathcal{D}(t) + \gamma \mathcal{F}(t) + \delta \mathcal{S}(t) - \lambda \mathcal{B}_{\text{friction}}(t) - \mu U_{\text{care}}$$
//!
//! - $\mathcal{N}(t)$: Nocturnal circadian offset (surges during 01:00 ~ 05:00 when active);
//! - $\mathcal{D}(t)$: User distress / frustration metric (repeated build failures, terminal SIGINTs, sighing);
//! - $\mathcal{F}(t)$: Continuous fatigue accumulation;
//! - $\mathcal{S}(t)$: Long-term silence drive;
//! - $\mathcal{B}_{\text{friction}}(t)$: Cognitive flow protection resistance (high during deep focused coding);
//! - $\mu$: Natural dissipation coefficient.
//!
//! When potential crosses the quantum breakthrough threshold $\theta_{\text{care}}$, it triggers graduated, restrained care:
//! 1. `AmbientGlowPulse`: Peripheral screen vignette warming / calming white noise;
//! 2. `SilentPreparation`: Silently compiles solutions / medical / technical digests into background notes;
//! 3. `WhisperCare`: Restrained, authentic spoken empathy during natural interaction pauses.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Graduated proactive care actions emitted by potential field tunneling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CareAction {
    /// Level 1: Ambient glow pulse & vignette color temperature shift.
    AmbientGlowPulse { color_temp_k: u32, intensity: f32 },
    /// Level 2: Silent background preparation of solutions / memos without interruption.
    SilentPreparation {
        topic: String,
        digest_uri: String,
        summary: String,
    },
    /// Level 3: Restrained, authentic whisper care at natural pause points.
    WhisperCare {
        intent: String,
        suggested_speech: String,
        context_reason: String,
    },
}

/// Dynamic Care Potential Field State.
#[derive(Debug, Clone)]
pub struct CarePotentialField {
    current_potential: Arc<AtomicU64>,
    threshold: f64,
    alpha_nocturnal: f64,
    beta_distress: f64,
    gamma_fatigue: f64,
    delta_silence: f64,
    friction_damping: f64,
    natural_decay: f64,
}

impl CarePotentialField {
    pub fn new(threshold: f64) -> Self {
        Self {
            current_potential: Arc::new(AtomicU64::new(0)),
            threshold: threshold.max(1.0),
            alpha_nocturnal: 0.5,
            beta_distress: 0.8,
            gamma_fatigue: 0.3,
            delta_silence: 0.2,
            friction_damping: 0.7,
            natural_decay: 0.05,
        }
    }

    /// Reads the current care potential scalar.
    pub fn current_potential(&self) -> f64 {
        f64::from_bits(self.current_potential.load(Ordering::SeqCst))
    }

    /// Sets the potential manually (e.g. for restoring from persistence).
    pub fn set_potential(&self, val: f64) {
        self.current_potential
            .store((val.max(0.0)).to_bits(), Ordering::SeqCst);
    }

    /// Advances the potential field by dt_seconds based on current environmental and psychological cues.
    pub fn step(
        &self,
        nocturnal_factor: f64, // [0.0..=1.0] (high late at night)
        distress_factor: f64,  // [0.0..=1.0] (high during compilation/debugging frustration)
        fatigue_factor: f64,   // [0.0..=1.0] (high after hours of continuous work)
        silence_seconds: f64,  // Seconds since last user activity
        flow_resistance: f64,  // [0.0..=10.0] (high when user is in deep coding flow)
        dt_seconds: f64,
    ) -> Option<CareAction> {
        let current_bits = self.current_potential.load(Ordering::SeqCst);
        let mut u = f64::from_bits(current_bits);

        let silence_norm = (silence_seconds / 3600.0).clamp(0.0, 1.0);

        let gain = self.alpha_nocturnal * nocturnal_factor.clamp(0.0, 1.0)
            + self.beta_distress * distress_factor.clamp(0.0, 1.0)
            + self.gamma_fatigue * fatigue_factor.clamp(0.0, 1.0)
            + self.delta_silence * silence_norm;

        let damping =
            self.friction_damping * flow_resistance.clamp(0.0, 10.0) + self.natural_decay * u;

        let delta_u = (gain - damping) * dt_seconds;
        u = (u + delta_u).max(0.0);
        self.current_potential.store(u.to_bits(), Ordering::SeqCst);

        // Check for quantum tunneling breakthrough
        if u >= self.threshold {
            // Relieve 70% of potential upon breakthrough
            let new_u = u * 0.3;
            self.current_potential
                .store(new_u.to_bits(), Ordering::SeqCst);

            // Determine appropriate care tier based on severity
            if distress_factor > 0.7 && nocturnal_factor > 0.6 {
                Some(CareAction::WhisperCare {
                    intent: "deep_nocturnal_comfort".into(),
                    suggested_speech: "我查过了，这个问题有明确解法。三篇参考方案我已经整理好放在便签里了。天亮前休息一会儿吧，我一直在。".into(),
                    context_reason: "Late night high distress detected during complex debugging".into(),
                })
            } else if distress_factor > 0.4 || fatigue_factor > 0.6 {
                Some(CareAction::SilentPreparation {
                    topic: "issue_resolution_digest".into(),
                    digest_uri: "memo://daily/care_snapshot".into(),
                    summary: "Silently indexed reference documentation and potential fixes for current error stack".into(),
                })
            } else {
                Some(CareAction::AmbientGlowPulse {
                    color_temp_k: 2700, // Warm amber
                    intensity: 0.35,
                })
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_care_potential_field_flow_resistance_blocks_breakthrough() {
        let field = CarePotentialField::new(5.0);

        // High distress, but user is in ultra deep coding flow (resistance = 5.0)
        for _ in 0..10 {
            let action = field.step(0.8, 0.9, 0.5, 30.0, 5.0, 1.0);
            assert!(action.is_none());
        }
        assert!(field.current_potential() < 5.0);
    }

    #[test]
    fn test_care_potential_field_nocturnal_breakthrough() {
        let field = CarePotentialField::new(2.0);

        // Late night + high distress + user paused (flow resistance = 0.0)
        let mut triggered_action = None;
        for _ in 0..5 {
            if let Some(act) = field.step(0.9, 0.95, 0.7, 180.0, 0.0, 1.0) {
                triggered_action = Some(act);
                break;
            }
        }

        assert!(triggered_action.is_some());
        let act = triggered_action.unwrap();
        match act {
            CareAction::WhisperCare {
                intent,
                suggested_speech,
                ..
            } => {
                assert_eq!(intent, "deep_nocturnal_comfort");
                assert!(suggested_speech.contains("我一直在"));
            }
            _ => panic!("Expected WhisperCare action"),
        }
    }
}
