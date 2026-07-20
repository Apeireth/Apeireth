//! Forget Engine — 主动遗忘 (借鉴 PersistBench 97% sycophancy 警示 + DeltaMemory exp decay)
//!
//! 主人 13:47 "记忆模块" — 必须可被主动遗忘, 否则会变 sycophancy
//!
//! 借鉴:
//! - PersistBench (arxiv 2602.01146): 97% LLM 记忆诱导谄媚失败 → 必须主动遗忘
//! - DeltaMemory: salience decay formula
//! - MemoryOS-Rust: heat score → auto-promotion

use crate::note::Note;

#[derive(Debug, Clone)]
pub struct ForgetStats {
    pub scanned: usize,
    pub forgotten: usize,
    pub kept: usize,
    pub avg_salience_kept: f64,
}

/// Forget sweep — 主动遗忘 (从 background cron 借鉴)
pub fn forget_sweep(notes: &mut Vec<Note>, threshold: f64) -> ForgetStats {
    let initial = notes.len();
    let mut kept_salience_sum = 0.0;
    let mut kept_count = 0usize;

    notes.retain(|n| {
        let keep = !n.should_forget(threshold);
        if keep {
            kept_salience_sum += n.salience;
            kept_count += 1;
        }
        keep
    });

    ForgetStats {
        scanned: initial,
        forgotten: initial - kept_count,
        kept: kept_count,
        avg_salience_kept: if kept_count > 0 {
            kept_salience_sum / kept_count as f64
        } else {
            0.0
        },
    }
}

/// Salience decay pass — 定期跑 (借鉴 DeltaMemory)
pub fn decay_pass(notes: &mut [Note], decay_rate: f64) {
    for n in notes.iter_mut() {
        n.apply_decay(decay_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forget_sweep_basic() {
        let mut notes = vec![
            Note::new("t1", "c1", vec![], 0.5, 5, "stm"),  // 0.5*0.5 = 0.25 < 0.30
            Note::new("t2", "c2", vec![], 0.9, 8, "ltm"),  // 0.9*0.8 = 0.72 keep
            Note::new("t3", "c3", vec![], 0.1, 1, "stm"),  // 0.01 forget
        ];
        let stats = forget_sweep(&mut notes, 0.30);
        assert_eq!(stats.scanned, 3);
        assert_eq!(stats.forgotten, 2);
        assert_eq!(stats.kept, 1);
        assert_eq!(notes.len(), 1);
    }
}