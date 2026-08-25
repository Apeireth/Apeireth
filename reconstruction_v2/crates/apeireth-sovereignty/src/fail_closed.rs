//! Fail-closed three-phase template

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailClosedPhase {
    Verify, Prepare, Apply,
}

impl FailClosedPhase {
    pub fn as_str(&self) -> &'static str {
        match self { Self::Verify => "verify", Self::Prepare => "prepare", Self::Apply => "apply" }
    }
}

impl fmt::Display for FailClosedPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

#[derive(Debug)]
pub struct FailClosedError<E> {
    pub phase: FailClosedPhase,
    pub source: E,
}

impl<E: fmt::Display> fmt::Display for FailClosedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fail-closed phase `{}` failed: {}", self.phase, self.source)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for FailClosedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { Some(&self.source) }
}

pub trait VerifyPhase {
    type Error: std::fmt::Debug;
    fn verify(&mut self) -> Result<(), Self::Error>;
}
pub trait PreparePhase {
    type Error: std::fmt::Debug;
    fn prepare(&mut self) -> Result<(), Self::Error>;
}
pub trait ApplyPhase {
    type Error: std::fmt::Debug;
    fn apply(&mut self) -> Result<(), Self::Error>;
}

pub fn run_fail_closed<PV, PP, PA, E>(mut verify: PV, mut prepare: PP, mut apply: PA) -> Result<(), FailClosedError<E>>
where
    PV: VerifyPhase<Error = E>, PP: PreparePhase<Error = E>, PA: ApplyPhase<Error = E>, E: std::fmt::Debug,
{
    if let Err(source) = verify.verify() { return Err(FailClosedError { phase: FailClosedPhase::Verify, source }); }
    if let Err(source) = prepare.prepare() { return Err(FailClosedError { phase: FailClosedPhase::Prepare, source }); }
    if let Err(source) = apply.apply() { return Err(FailClosedError { phase: FailClosedPhase::Apply, source }); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone)]
    struct RecordingOp { ran: Vec<FailClosedPhase>, verify_result: Result<(), String>, prepare_result: Result<(), String>, apply_result: Result<(), String> }
    impl Default for RecordingOp {
        fn default() -> Self { Self { ran: Vec::new(), verify_result: Ok(()), prepare_result: Ok(()), apply_result: Ok(()) } }
    }
    impl VerifyPhase for RecordingOp { type Error = String; fn verify(&mut self) -> Result<(), String> { self.ran.push(FailClosedPhase::Verify); self.verify_result.clone() } }
    impl PreparePhase for RecordingOp { type Error = String; fn prepare(&mut self) -> Result<(), String> { self.ran.push(FailClosedPhase::Prepare); self.prepare_result.clone() } }
    impl ApplyPhase for RecordingOp { type Error = String; fn apply(&mut self) -> Result<(), String> { self.ran.push(FailClosedPhase::Apply); self.apply_result.clone() } }

    #[test] fn all_phases_pass() { assert!(run_fail_closed(RecordingOp::default(), RecordingOp::default(), RecordingOp::default()).is_ok()); }
    #[test] fn verify_failure_aborts() {
        let mut v = RecordingOp::default(); v.verify_result = Err("boom".into());
        let err = run_fail_closed(v, RecordingOp::default(), RecordingOp::default()).unwrap_err();
        assert_eq!(err.phase, FailClosedPhase::Verify);
    }
    #[test] fn prepare_failure_aborts() {
        let mut p = RecordingOp::default(); p.prepare_result = Err("boom".into());
        let err = run_fail_closed(RecordingOp::default(), p, RecordingOp::default()).unwrap_err();
        assert_eq!(err.phase, FailClosedPhase::Prepare);
    }
    #[test] fn apply_failure() {
        let mut a = RecordingOp::default(); a.apply_result = Err("boom".into());
        let err = run_fail_closed(RecordingOp::default(), RecordingOp::default(), a).unwrap_err();
        assert_eq!(err.phase, FailClosedPhase::Apply);
    }
    #[test] fn phase_labels() {
        assert_eq!(FailClosedPhase::Verify.as_str(), "verify");
        assert_eq!(FailClosedPhase::Prepare.as_str(), "prepare");
        assert_eq!(FailClosedPhase::Apply.as_str(), "apply");
    }
    #[test] fn error_display_includes_phase() {
        let err = FailClosedError { phase: FailClosedPhase::Verify, source: "x".to_string() };
        assert!(format!("{}", err).contains("verify"));
    }
}
