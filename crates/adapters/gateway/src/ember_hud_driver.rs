//! Ember HUD 4.0s Physiological Breath & Peripheral Vignette Glow Driver.
//!
//! # Visual & Cognitive Foundations
//!
//! Replaces heavy, screen-occluding Live2D avatars with an ambient, minimalist **Ember Core**:
//! - **4.0s Physiological Breath Rhythm**: Modulates glow intensity via non-linear cubic sine wave:
//!   $$I(t) = I_{\text{base}} + A(s) \cdot \left[ \sin\left(\frac{2\pi t}{4.0} + \phi\right) \right]^3$$
//! - **Peripheral Vignette Glow**: Casts subtle ambient color temperatures at screen corners utilizing
//!   human peripheral vision sensitivity without breaking productivity flow.
//! - Emits structured GPU WGSL uniform buffers for front-end WebGL / WebGPU / Svelte 5 rendering.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};

/// Front-end Shader Uniform Buffer for Ember HUD & Vignette rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmberShaderUniforms {
    pub time_secs: f32,
    pub base_intensity: f32,
    pub breath_amplitude: f32,
    pub current_glow_intensity: f32,
    pub color_temperature_k: u32,
    pub rgb_tint: [f32; 3],
    pub vignette_radius: f32,
    pub vignette_softness: f32,
    pub stance_label: String,
}

/// Dynamic cognitive stance driving Ember HUD rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmberCognitiveStance {
    /// In deep focus; glow dims to 1% to protect flow.
    DeepCodingFocus,
    /// Actively observing & listening in full duplex.
    AttentivePresence,
    /// Consolidating / dreaming / researching in background.
    DreamingConsolidation,
    /// Warm, compassionate empathetic support.
    EmpatheticCare,
}

/// Ember HUD Driver & Shader State Synthesizer.
#[derive(Debug, Clone)]
pub struct EmberHudDriver {
    pub breath_period_secs: f32,
    pub base_vignette_radius: f32,
    pub base_vignette_softness: f32,
}

impl Default for EmberHudDriver {
    fn default() -> Self {
        Self {
            breath_period_secs: 4.0,
            base_vignette_radius: 0.85,
            base_vignette_softness: 0.45,
        }
    }
}

impl EmberHudDriver {
    pub fn new(breath_period_secs: f32) -> Self {
        Self {
            breath_period_secs: breath_period_secs.max(1.0),
            ..Default::default()
        }
    }

    /// Evaluates dynamic 4.0s cubic sine breathing intensity.
    pub fn compute_breath_intensity(
        &self,
        time_secs: f32,
        base_intensity: f32,
        amplitude: f32,
    ) -> f32 {
        let phase = (2.0 * std::f32::consts::PI * time_secs) / self.breath_period_secs;
        let sine_cube = phase.sin().powi(3);
        (base_intensity + amplitude * sine_cube).clamp(0.0, 1.0)
    }

    /// Converts color temperature (Kelvin) to RGB normalized float vector.
    pub fn kelvin_to_rgb(kelvin: u32) -> [f32; 3] {
        let temp = (kelvin as f32) / 100.0;

        let r = if temp <= 66.0 {
            1.0
        } else {
            let x = temp - 60.0;
            (329.6987 * x.powf(-0.13320476) / 255.0).clamp(0.0, 1.0)
        };

        let g = if temp <= 66.0 {
            let x = temp;
            (99.4708 * x.ln() - 161.11957 / 255.0).clamp(0.0, 1.0)
        } else {
            let x = temp - 60.0;
            (288.12217 * x.powf(-0.07551485) / 255.0).clamp(0.0, 1.0)
        };

        let b = if temp >= 66.0 {
            1.0
        } else if temp <= 19.0 {
            0.0
        } else {
            let x = temp - 10.0;
            (138.51773 * x.ln() - 305.0448 / 255.0).clamp(0.0, 1.0)
        };

        [r, g, b]
    }

    /// Synthesizes complete shader uniforms for current timestamp and stance.
    pub fn synthesize_uniforms(
        &self,
        time_secs: f32,
        stance: EmberCognitiveStance,
    ) -> EmberShaderUniforms {
        let (base_i, amp, kelvin, stance_str) = match stance {
            EmberCognitiveStance::DeepCodingFocus => (0.02, 0.03, 3200, "deep_focus"),
            EmberCognitiveStance::AttentivePresence => (0.15, 0.20, 4500, "attentive"),
            EmberCognitiveStance::DreamingConsolidation => (0.08, 0.12, 6000, "dreaming"),
            EmberCognitiveStance::EmpatheticCare => (0.25, 0.35, 2700, "empathetic_care"),
        };

        let glow = self.compute_breath_intensity(time_secs, base_i, amp);
        let rgb = Self::kelvin_to_rgb(kelvin);

        EmberShaderUniforms {
            time_secs,
            base_intensity: base_i,
            breath_amplitude: amp,
            current_glow_intensity: glow,
            color_temperature_k: kelvin,
            rgb_tint: rgb,
            vignette_radius: self.base_vignette_radius,
            vignette_softness: self.base_vignette_softness,
            stance_label: stance_str.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ember_hud_breathing_cycle() {
        let driver = EmberHudDriver::default();

        // At t = 0, sin(0)^3 = 0 -> intensity = base
        let i0 = driver.compute_breath_intensity(0.0, 0.1, 0.2);
        assert!((i0 - 0.1).abs() < 1e-5);

        // At t = 1.0 (quarter cycle, peak), sin(pi/2)^3 = 1 -> intensity = base + amp
        let i1 = driver.compute_breath_intensity(1.0, 0.1, 0.2);
        assert!((i1 - 0.3).abs() < 1e-5);

        // At t = 3.0 (three-quarter cycle, trough), sin(3pi/2)^3 = -1 -> intensity = base - amp
        let i3 = driver.compute_breath_intensity(3.0, 0.3, 0.2);
        assert!((i3 - 0.1).abs() < 1e-5);
    }

    #[test]
    fn test_ember_hud_shader_uniforms_synthesis() {
        let driver = EmberHudDriver::new(4.0);
        let uniforms = driver.synthesize_uniforms(2.0, EmberCognitiveStance::EmpatheticCare);

        assert_eq!(uniforms.color_temperature_k, 2700);
        assert_eq!(uniforms.stance_label, "empathetic_care");
        assert!(uniforms.rgb_tint[0] > 0.8); // High red for warm amber
        assert!(uniforms.current_glow_intensity >= 0.0);
    }
}
