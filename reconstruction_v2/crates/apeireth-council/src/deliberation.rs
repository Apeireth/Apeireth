//! Council Deliberation — 多 council 协商 / 投票 / 产出 verdict (v2 自洽)
//!
//! **设计** (对齐 v1 deliberation.rs intent, 不抄 v1 FFI/HTTP/SQL):
//! - `CouncilQuery` / `Opinion` / `Verdict` / `RiskLevel`
//! - `Deliberation` 引擎 + `MultiCouncilDeliberation` 多 council 投票
//! - 加权: `Σ(stance × confidence × weight) / Σ(confidence × weight)`

use crate::persona::BondCharacter;
use crate::sovereign::{CouncilEvent, SovereigntyHook};
use serde::{Deserialize, Serialize};
use std::fmt;


/// Backward-compat alias used by v1 evolution integration.
pub type CouncilVerdict = Verdict;

/// Backward-compat struct used by v1 evolution integration.
pub struct Council {
    pub members: Vec<crate::council_member::CouncilMember>,
}

impl Council {
    pub fn new() -> Self { Self { members: Vec::new() } }
    pub fn advisor_count(&self) -> usize { self.members.len() }
    pub fn advisors_iter(&self) -> impl Iterator<Item = &crate::council_member::CouncilMember> { self.members.iter() }
    pub fn weights_for(&self, _advisor_id: &str) -> f64 { 0.5 }
    pub fn emit_event(&self, _event: DeliberationStreamEvent) { /* noop */ }
    pub fn weights_clone(&self) -> Vec<f64> { self.members.iter().map(|_| 0.5).collect() }

}

impl Default for Council {
    fn default() -> Self { Self::new() }
}

/// Stream event for v1 evolution integration.
pub enum DeliberationStreamEvent {
    Opinion(crate::advisor::AdvisorOpinion),
    Verdict(CouncilVerdict),
    HoldTrigger(crate::hold::HoldTrigger),
}

pub const DEFAULT_DELIBERATION_TIMEOUT_MS: u64 = 60_000;
pub const SEVEN_MANDATORY_DOMAINS: [BondCharacter; 5] = [
    BondCharacter::Sage, BondCharacter::Guardian, BondCharacter::Rebel,
    BondCharacter::Healer, BondCharacter::Explorer,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskLevel { Low, Medium, High, Nuclear }
impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self { Self::Low=>"low",Self::Medium=>"medium",Self::High=>"high",Self::Nuclear=>"nuclear" }
    }
    pub fn weight(&self) -> f64 {
        match self { Self::Low=>1.0,Self::Medium=>1.5,Self::High=>2.5,Self::Nuclear=>4.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilQuery {
    pub query_id: String,
    pub description: String,
    pub risk: RiskLevel,
    pub started_at_ms: i64,
    pub context: QueryContext,
}

/// Query context for v1 evolution integration.
#[derive(Debug, Clone, Default)]
pub struct QueryContext {
    pub session_id: String,
    pub tags: Vec<String>,
    pub risk: RiskLevel,
}

impl CouncilQuery {
    pub fn new(q: impl Into<String>, d: impl Into<String>, r: RiskLevel, t: i64) -> Self {
        Self { query_id: q.into(), description: d.into(), risk: r, started_at_ms: t }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opinion {
    pub opinion_id: String,
    pub author_id: String,
    pub character: BondCharacter,
    pub stance_score: f64,
    pub confidence: f64,
}
impl Opinion {
    pub fn new(o: impl Into<String>, a: impl Into<String>, c: BondCharacter, s: f64, conf: f64) -> Self {
        Self { opinion_id: o.into(), author_id: a.into(), character: c,
               stance_score: s.clamp(-1.0,1.0), confidence: conf.clamp(0.0,1.0) }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Verdict {
    Approved { score: f64, rationale: String },
    Rejected { score: f64, rationale: String },
    Pending { score: f64, rationale: String, review_at_ms: i64 },
}
impl Verdict {
    pub fn is_approved(&self) -> bool { matches!(self, Self::Approved{..}) }
    pub fn is_rejected(&self) -> bool { matches!(self, Self::Rejected{..}) }
    pub fn score(&self) -> f64 {
        match self {
            Self::Approved{score,..}|Self::Rejected{score,..}|Self::Pending{score,..} => *score
        }
    }
}

pub struct Deliberation {
    pub query: CouncilQuery,
    pub opinions: Vec<Opinion>,
    pub threshold: f64,
}
impl Deliberation {
    pub fn new(query: CouncilQuery) -> Self { Self { query, opinions: Vec::new(), threshold: 0.2 } }
    pub fn with_threshold(mut self, t: f64) -> Self { self.threshold = t.clamp(0.0,1.0); self }
    pub fn add_opinion(&mut self, op: Opinion) { self.opinions.push(op); }
    pub fn opinion_count(&self) -> usize { self.opinions.len() }

    pub fn synthesize(&self) -> Verdict {
        if self.opinions.is_empty() {
            return Verdict::Pending { score: 0.0, rationale: "no opinions".into(),
                review_at_ms: self.query.started_at_ms + DEFAULT_DELIBERATION_TIMEOUT_MS as i64 };
        }
        let rw = self.query.risk.weight();
        let mut num = 0.0; let mut den = 0.0;
        for op in &self.opinions { let w = op.confidence * rw; num += op.stance_score * w; den += w; }
        let score = if den > 0.0 { (num/den).clamp(-1.0,1.0) } else { 0.0 };
        if score >= self.threshold { Verdict::Approved { score, rationale: format!("score {} >= threshold {}", score, self.threshold) } }
        else if score <= -self.threshold { Verdict::Rejected { score, rationale: format!("score {} <= -threshold {}", score, self.threshold) } }
        else { Verdict::Pending { score, rationale: format!("score {} within neutral band", score),
            review_at_ms: self.query.started_at_ms + DEFAULT_DELIBERATION_TIMEOUT_MS as i64 } }
    }

    pub fn run_with_hook<H: SovereigntyHook>(&mut self, hook: &mut H) -> Verdict {
        hook.on_council_event(&CouncilEvent::DeliberationStarted {
            session_id: self.query.query_id.clone(), query_id: self.query.query_id.clone(),
            started_at_ms: self.query.started_at_ms });
        for op in &self.opinions {
            hook.on_council_event(&CouncilEvent::OpinionIssued {
                session_id: self.query.query_id.clone(), opinion_id: op.opinion_id.clone(),
                author_id: op.author_id.clone(), author_character: op.character,
                stance_score: op.stance_score, confidence: op.confidence });
        }
        let v = self.synthesize();
        let s = match &v { Verdict::Approved{..}=>"approved",Verdict::Rejected{..}=>"rejected",Verdict::Pending{..}=>"pending" };
        hook.on_council_event(&CouncilEvent::DeliberationCompleted {
            session_id: self.query.query_id.clone(), verdict: s.into(),
            completed_at_ms: self.query.started_at_ms + 1 });
        v
    }
}
impl fmt::Debug for Deliberation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Deliberation").field("query_id",&self.query.query_id)
            .field("opinions",&self.opinions.len()).field("threshold",&self.threshold).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouncilVote {
    All { verdict: Verdict, count: usize },
    Majority { verdict: Verdict, count: usize, total: usize },
    Split { verdicts: Vec<Verdict> },
}

pub struct MultiCouncilDeliberation {
    pub councils: Vec<Deliberation>,
    pub quorum: usize,
}
impl MultiCouncilDeliberation {
    pub fn new(quorum: usize) -> Self { Self { councils: Vec::new(), quorum: quorum.max(1) } }
    pub fn push(&mut self, c: Deliberation) { self.councils.push(c); }
    pub fn len(&self) -> usize { self.councils.len() }
    pub fn is_empty(&self) -> bool { self.councils.is_empty() }
    pub fn vote(&self) -> CouncilVote {
        let verdicts: Vec<Verdict> = self.councils.iter().map(|c| c.synthesize()).collect();
        let total = verdicts.len();
        let mut approve = 0; let mut reject = 0;
        let mut fa = None; let mut fr = None;
        for v in &verdicts {
            if v.is_approved() { approve += 1; if fa.is_none() { fa = Some(v.clone()); } }
            else if v.is_rejected() { reject += 1; if fr.is_none() { fr = Some(v.clone()); } }
        }
        if total > 0 && approve == total {
            CouncilVote::All { verdict: fa.unwrap(), count: approve }
        } else if approve >= self.quorum {
            CouncilVote::Majority { verdict: fa.unwrap(), count: approve, total }
        } else if reject >= self.quorum {
            CouncilVote::Majority { verdict: fr.unwrap_or(Verdict::Pending{score:0.0,rationale:"none".into(),review_at_ms:0}), count: reject, total }
        } else { CouncilVote::Split { verdicts } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sovereign::{BroadcastHook, NoopSovereigntyHook};

    #[test] fn t01_risk_weight_ordering() {
        assert!(RiskLevel::Nuclear.weight() > RiskLevel::High.weight());
        assert!(RiskLevel::High.weight() > RiskLevel::Medium.weight());
        assert!(RiskLevel::Medium.weight() > RiskLevel::Low.weight());
    }
    #[test] fn t02_opinion_clamps_inputs() {
        let op = Opinion::new("o","a",BondCharacter::Sage,5.0,-1.0);
        assert_eq!(op.stance_score,1.0); assert_eq!(op.confidence,0.0);
    }
    #[test] fn t03_synthesize_empty_pending() {
        let d = Deliberation::new(CouncilQuery::new("q","d",RiskLevel::Low,0));
        assert!(matches!(d.synthesize(), Verdict::Pending{..}));
    }
    #[test] fn t04_synthesize_all_approve() {
        let mut d = Deliberation::new(CouncilQuery::new("q","d",RiskLevel::Low,0));
        d.add_opinion(Opinion::new("o1","a1",BondCharacter::Sage,0.8,0.9));
        d.add_opinion(Opinion::new("o2","a2",BondCharacter::Guardian,0.6,0.8));
        assert!(d.synthesize().is_approved());
    }
    #[test] fn t05_synthesize_all_reject() {
        let mut d = Deliberation::new(CouncilQuery::new("q","d",RiskLevel::High,0));
        d.add_opinion(Opinion::new("o1","a1",BondCharacter::Sage,-0.7,0.9));
        d.add_opinion(Opinion::new("o2","a2",BondCharacter::Guardian,-0.9,0.8));
        assert!(d.synthesize().is_rejected());
    }
    #[test] fn t06_synthesize_neutral_pending() {
        let mut d = Deliberation::new(CouncilQuery::new("q","d",RiskLevel::Low,0)).with_threshold(0.5);
        d.add_opinion(Opinion::new("o","a",BondCharacter::Sage,0.1,0.9));
        assert!(matches!(d.synthesize(), Verdict::Pending{..}));
    }
    #[test] fn t07_run_with_broadcast_hook() {
        let mut d = Deliberation::new(CouncilQuery::new("q","d",RiskLevel::Low,1000));
        d.add_opinion(Opinion::new("o1","a1",BondCharacter::Sage,0.5,0.9));
        let mut h = BroadcastHook::new(20);
        d.run_with_hook(&mut h);
        assert_eq!(h.len(), 3);
    }
    #[test] fn t08_multi_council_all() {
        let mut m = MultiCouncilDeliberation::new(2);
        for i in 0..3 {
            let mut d = Deliberation::new(CouncilQuery::new(format!("q{i}"),"x",RiskLevel::Low,0));
            d.add_opinion(Opinion::new("o","a",BondCharacter::Sage,0.5,0.9));
            m.push(d);
        }
        assert!(matches!(m.vote(), CouncilVote::All{..}));
    }
    #[test] fn t09_multi_council_split_or_majority() {
        let mut m = MultiCouncilDeliberation::new(2);
        for i in 0..2 {
            let mut d = Deliberation::new(CouncilQuery::new(format!("q{i}"),"x",RiskLevel::Low,0));
            d.add_opinion(Opinion::new("o","a",BondCharacter::Sage,0.5,0.9));
            m.push(d);
        }
        let mut d3 = Deliberation::new(CouncilQuery::new("q3","x",RiskLevel::Low,0));
        d3.add_opinion(Opinion::new("o","a",BondCharacter::Sage,-0.8,0.9));
        m.push(d3);
        let v = m.vote();
        assert!(matches!(v, CouncilVote::All{..}|CouncilVote::Majority{..}));
    }
    #[test] fn t10_noop_hook_run() {
        let mut d = Deliberation::new(CouncilQuery::new("q","x",RiskLevel::Low,0));
        d.add_opinion(Opinion::new("o","a",BondCharacter::Sage,0.5,0.9));
        let mut h = NoopSovereigntyHook::new();
        let _ = d.run_with_hook(&mut h);
    }
}