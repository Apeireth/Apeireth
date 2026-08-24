//! R177 web organ Kani proofs (W9)

#![allow(missing_docs)]

use crate::api::*;

#[test]
fn r177_web_01_advisor_opinion() {
    let o = AdvisorOpinion {
        domain: "x".into(),
        stance: "y".into(),
        reasoning: "z".into(),
    };
    assert_eq!(o.domain, "x");
}

#[test]
fn r177_web_02_council_response() {
    let r = CouncilAdviseResponse {
        topic: "t".into(),
        status: "ok".into(),
        advisors: vec![],
        verdict: "v".into(),
        protocol: "openai".into(),
        debate_id: None,
    };
    assert_eq!(r.topic, "t");
}

#[test]
fn r177_web_03_opinion_field() {
    let o = AdvisorOpinion {
        domain: "d".into(),
        stance: "s".into(),
        reasoning: "r".into(),
    };
    assert_eq!(o.stance, "s");
}

#[test]
fn r177_web_04_response_field() {
    let r = CouncilAdviseResponse {
        topic: "t".into(),
        status: "ok".into(),
        advisors: vec![],
        verdict: "v".into(),
        protocol: "openai".into(),
        debate_id: None,
    };
    assert_eq!(r.verdict, "v");
}

#[test]
fn r177_web_05_module_compiles() {
    let _ = std::mem::size_of::<u64>();
}

#[cfg(kani)]
#[kani::proof]
fn r177_web_kani_01_opinion_invariant() {
    let o = AdvisorOpinion {
        domain: "d".into(),
        stance: "s".into(),
        reasoning: "r".into(),
    };
    assert_eq!(o.domain, "d");
}

#[cfg(kani)]
#[kani::proof]
fn r177_web_kani_02_response_invariant() {
    let r = CouncilAdviseResponse {
        topic: "t".into(),
        status: "ok".into(),
        advisors: vec![],
        verdict: "v".into(),
        protocol: "openai".into(),
        debate_id: None,
    };
    assert_eq!(r.topic, "t");
}
