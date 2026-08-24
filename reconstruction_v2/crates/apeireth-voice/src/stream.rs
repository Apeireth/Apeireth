use crate::lipsync::{LipSyncCalculator, VisemeFrame};
// Reserved for real audio stream API integration (VAD/TTS pipeline).
// Current synthetic stub doesn't consume Bytes yet; flag suppress until next iteration.
#[allow(unused_imports)]
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFrame {
    pub sequence_id: u64,
    pub timestamp_ms: u64,
    pub viseme: VisemeFrame,
    pub is_final: bool,
}

pub struct AudioChunkStreamer {
    calculator: LipSyncCalculator,
    sequence: u64,
}

impl Default for AudioChunkStreamer {
    fn default() -> Self {
        Self::new(24000)
    }
}

impl AudioChunkStreamer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            calculator: LipSyncCalculator::new(sample_rate),
            sequence: 0,
        }
    }

    pub fn process_raw_audio(&mut self, timestamp_ms: u64, pcm_bytes: &[u8], is_final: bool) -> StreamFrame {
        self.sequence += 1;

        // Convert byte slice to i16 samples
        let samples: Vec<i16> = pcm_bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let viseme = self.calculator.process_chunk(timestamp_ms, &samples);

        StreamFrame {
            sequence_id: self.sequence,
            timestamp_ms,
            viseme,
            is_final,
        }
    }
}
