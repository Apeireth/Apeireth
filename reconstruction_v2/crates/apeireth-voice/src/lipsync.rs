use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VisemeType {
    Sil, // Silence
    Aa,  // 'a' sound (wide open)
    Ih,  // 'i' sound (wide stretch)
    Ou,  // 'u' sound (pursed round)
    Ee,  // 'e' sound (medium open)
    Oh,  // 'o' sound (large round)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisemeFrame {
    pub timestamp_ms: u64,
    pub mouth_open_y: f32, // Live2D ParamMouthOpenY: 0.0 .. 1.0
    pub mouth_form: f32,   // Live2D ParamMouthForm: -1.0 (narrow) .. 1.0 (smile)
    pub viseme_type: VisemeType,
    pub rms_energy: f32,
}

pub struct LipSyncCalculator {
    /// Audio sample rate (Hz). Reserved for actual phoneme-frame conversion
    /// when real audio frames are fed in (current stub uses synthetic timing).
    #[allow(dead_code)]
    sample_rate: u32,
    silence_threshold: f32,
}

impl Default for LipSyncCalculator {
    fn default() -> Self {
        Self {
            sample_rate: 24000,
            silence_threshold: 0.02,
        }
    }
}

impl LipSyncCalculator {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            silence_threshold: 0.02,
        }
    }

    /// Computes RMS energy from a slice of 16-bit PCM audio samples
    pub fn calculate_rms_i16(&self, pcm_samples: &[i16]) -> f32 {
        if pcm_samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = pcm_samples.iter().map(|&s| {
            let normalized = s as f64 / 32768.0;
            normalized * normalized
        }).sum();
        ((sum_sq / pcm_samples.len() as f64).sqrt() as f32).clamp(0.0, 1.0)
    }

    /// Computes zero-crossing rate to estimate phoneme sharpness
    pub fn calculate_zcr_i16(&self, pcm_samples: &[i16]) -> f32 {
        if pcm_samples.len() < 2 {
            return 0.0;
        }
        let mut crossings = 0;
        for i in 1..pcm_samples.len() {
            if (pcm_samples[i] >= 0 && pcm_samples[i - 1] < 0) || (pcm_samples[i] < 0 && pcm_samples[i - 1] >= 0) {
                crossings += 1;
            }
        }
        crossings as f32 / pcm_samples.len() as f32
    }

    /// Converts raw audio chunk into a real-time VisemeFrame
    pub fn process_chunk(&self, timestamp_ms: u64, pcm_samples: &[i16]) -> VisemeFrame {
        let rms = self.calculate_rms_i16(pcm_samples);
        let zcr = self.calculate_zcr_i16(pcm_samples);

        if rms < self.silence_threshold {
            return VisemeFrame {
                timestamp_ms,
                mouth_open_y: 0.0,
                mouth_form: 0.0,
                viseme_type: VisemeType::Sil,
                rms_energy: rms,
            };
        }

        // Non-linear mouth opening amplification
        let mouth_open_y = ((rms - self.silence_threshold) * 3.5).powf(0.85).clamp(0.0, 1.0);

        // Classify phoneme by high-frequency zero-crossing rate
        let (mouth_form, viseme) = if zcr > 0.35 {
            (0.8, VisemeType::Ee) // Sibilants / 'e' / 'i'
        } else if zcr > 0.20 {
            (0.5, VisemeType::Ih)
        } else if rms > 0.30 {
            (0.2, VisemeType::Aa) // Strong open vowel 'a'
        } else if zcr < 0.10 {
            (-0.6, VisemeType::Ou) // Rounded vowel 'u' / 'o'
        } else {
            (0.0, VisemeType::Oh)
        };

        VisemeFrame {
            timestamp_ms,
            mouth_open_y,
            mouth_form,
            viseme_type: viseme,
            rms_energy: rms,
        }
    }
}
