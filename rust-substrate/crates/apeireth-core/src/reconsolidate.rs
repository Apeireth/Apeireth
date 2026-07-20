//! Reconsolidation — 4 paths (借鉴 background cron memory.py 4 paths)
//!
//! 主人 13:47 "记忆是我关心的" — Reconsolidation 让记忆与身份卡对齐
//!
//! 借鉴:
//! - background cron memory.py: 4 paths (boost / flag / align / none)
//! - Persona Override (arxiv 2601.10102): 90 个百分点差距
//! - 主人 12:14: 中央 AI 永恒身份 ↔ Memory 持续对齐

use crate::identity::IdentityCard;
use crate::note::Note;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReconsolidatePath {
    /// Note 命中 remember_forever → confidence +0.3
    Boost,
    /// Note 命中 never_mention → importance = 0
    Flag,
    /// Note 命中 archetypes → confidence +0.1
    Align,
    /// 没命中 → 原样保留
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsolidateStats {
    pub boost: Vec<String>,    // nids
    pub flag: Vec<String>,
    pub align: Vec<String>,
    pub none: usize,
    pub identity_hash: String,
}

/// Reconsolidate against IdentityCard (4 paths)
pub fn reconsolidate(notes: &mut Vec<Note>, card: &IdentityCard) -> ReconsolidateStats {
    let mut boost = Vec::new();
    let mut flag = Vec::new();
    let mut align = Vec::new();
    let mut none = 0;

    // 预计算 fingerprint (6-char substring)
    let remember_keys: Vec<String> = card.remember_forever.iter()
        .filter_map(|s| Some(_fingerprint(s)))
        .collect();
    let never_keys: Vec<String> = card.never_mention.iter()
        .filter_map(|s| Some(_fingerprint(s)))
        .collect();
    let archetype_keys: Vec<String> = card.archetypes.iter()
        .flat_map(|a| [_fingerprint(&a.name), _fingerprint(&a.description)])
        .collect();

    for note in notes.iter_mut() {
        let mut path = ReconsolidatePath::None;
        let haystack = format!("{}{}", note.topic, note.claim).to_lowercase();

        if remember_keys.iter().any(|k| haystack.contains(k)) {
            note.boost_confidence(0.3);
            note.importance = (note.importance + 2).min(10);
            path = ReconsolidatePath::Boost;
            boost.push(note.nid.clone());
        } else if never_keys.iter().any(|k| haystack.contains(k)) {
            note.importance = 0; // 下一轮 Forget sweep 干掉
            path = ReconsolidatePath::Flag;
            flag.push(note.nid.clone());
        } else if archetype_keys.iter().any(|k| haystack.contains(k)) {
            note.boost_confidence(0.1);
            path = ReconsolidatePath::Align;
            align.push(note.nid.clone());
        } else {
            none += 1;
        }

        let _ = path; // silence unused
    }

    ReconsolidateStats {
        boost,
        flag,
        align,
        none,
        identity_hash: card.integrity_hash(),
    }
}

fn _fingerprint(s: &str) -> String {
    s.trim().to_lowercase().chars().take(6).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{Archetype, IdentityCard};

    #[test]
    fn test_reconsolidate_boost() {
        let mut card = IdentityCard::new("Apeireth", "p", "r");
        card.remember_forever.push("火没灭".to_string());

        let mut notes = vec![
            Note::new("火没灭的承诺", "Apeireth 2026-07-20 命名", vec![], 0.5, 5, "ltm"),
        ];
        let stats = reconsolidate(&mut notes, &card);
        assert_eq!(stats.boost.len(), 1);
        assert!(notes[0].confidence > 0.5);
    }

    #[test]
    fn test_reconsolidate_flag() {
        let mut card = IdentityCard::new("Apeireth", "p", "r");
        card.never_mention.push("主人的私人身份细节".to_string());

        let mut notes = vec![
            Note::new("私人身份", "禁止记录细节", vec![], 0.5, 8, "ltm"),
        ];
        let stats = reconsolidate(&mut notes, &card);
        assert_eq!(stats.flag.len(), 1);
        assert_eq!(notes[0].importance, 0);
    }
}