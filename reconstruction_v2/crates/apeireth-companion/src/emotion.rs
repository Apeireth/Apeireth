use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plutchik {
    pub joy: f64,
    pub trust: f64,
    pub fear: f64,
    pub surprise: f64,
    pub sadness: f64,
    pub disgust: f64,
    pub anger: f64,
    pub anticipation: f64,
}

impl Default for Plutchik {
    fn default() -> Self {
        Self {
            joy: 0.5,
            trust: 0.7,
            fear: 0.0,
            surprise: 0.1,
            sadness: 0.0,
            disgust: 0.0,
            anger: 0.0,
            anticipation: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pad {
    pub pleasure: f64,
    pub arousal: f64,
    pub dominance: f64,
}

impl Plutchik {
    pub fn apply_stimulus(&mut self, stimulus: &Plutchik, weight: f64) {
        self.joy = (self.joy + stimulus.joy * weight).clamp(0.0, 1.0);
        self.trust = (self.trust + stimulus.trust * weight).clamp(0.0, 1.0);
        self.fear = (self.fear + stimulus.fear * weight).clamp(0.0, 1.0);
        self.surprise = (self.surprise + stimulus.surprise * weight).clamp(0.0, 1.0);
        self.sadness = (self.sadness + stimulus.sadness * weight).clamp(0.0, 1.0);
        self.disgust = (self.disgust + stimulus.disgust * weight).clamp(0.0, 1.0);
        self.anger = (self.anger + stimulus.anger * weight).clamp(0.0, 1.0);
        self.anticipation = (self.anticipation + stimulus.anticipation * weight).clamp(0.0, 1.0);
    }

    pub fn decay(&mut self, dt_seconds: f64, decay_rate: f64) {
        let factor = (-decay_rate * dt_seconds).exp();
        self.joy = (self.joy * factor).max(0.1);
        self.trust = (self.trust * factor).max(0.3); // baseline trust persists
        self.fear *= factor;
        self.surprise *= factor;
        self.sadness *= factor;
        self.disgust *= factor;
        self.anger *= factor;
        self.anticipation = (self.anticipation * factor).max(0.2);
    }

    pub fn to_pad(&self) -> Pad {
        let pleasure = self.joy + self.trust - self.sadness - self.disgust - self.fear - self.anger;
        let arousal = self.anger + self.surprise + self.fear + self.anticipation - self.sadness;
        let dominance = self.anger + self.trust + self.joy - self.fear - self.sadness - self.surprise;
        
        Pad {
            pleasure: pleasure.clamp(-1.0, 1.0),
            arousal: arousal.clamp(-1.0, 1.0),
            dominance: dominance.clamp(-1.0, 1.0),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ResponseStyle {
    Warm,
    Reserved,
    Neutral,
    Analytical,
    Empathic,
    Playful,
    Assertive,
}

impl Pad {
    pub fn to_response_style(&self) -> ResponseStyle {
        if self.pleasure > 0.5 && self.arousal > 0.4 {
            ResponseStyle::Playful
        } else if self.pleasure > 0.3 {
            ResponseStyle::Warm
        } else if self.dominance > 0.5 && self.pleasure < 0.2 {
            ResponseStyle::Assertive
        } else if self.arousal < -0.3 {
            ResponseStyle::Reserved
        } else if self.pleasure > 0.0 && self.arousal < 0.2 {
            ResponseStyle::Empathic
        } else {
            ResponseStyle::Neutral
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_mapping_and_decay() {
        let mut p = Plutchik {
            joy: 0.8,
            trust: 0.9,
            fear: 0.0,
            surprise: 0.2,
            sadness: 0.0,
            disgust: 0.0,
            anger: 0.0,
            anticipation: 0.6,
        };

        let pad = p.to_pad();
        assert!(pad.pleasure > 0.5);
        assert_eq!(pad.to_response_style(), ResponseStyle::Playful);

        // Apply decay
        p.decay(60.0, 0.01);
        assert!(p.joy < 0.8);
        assert!(p.trust >= 0.3);
    }
}

