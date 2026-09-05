//! Risk-rank and fail-closed helpers recovered from donor sovereignty.
//!
//! Self-Disable **ownership** stays in `apeireth-core` (`SelfDisableAudit`,
//! compile-time hardcode). This module only ports the comparable algorithms
//! that core does not already own:
//!
//! * monotonic risk ranking (`info/low < medium < high < critical < nuclear`)
//! * no-degrade check (proposed rank must not fall)
//! * three-phase fail-closed template (verify → prepare → apply; later phases
//!   never run after an earlier failure)
//!
//! Not a second governance authority. Not wired into [`crate::GovernancePipeline`].

use serde::{Deserialize, Serialize};

/// Numeric rank used by [`check_no_degrade`]. `info` and `low` share rank 0.
pub fn risk_rank(risk: &str) -> i32 {
    match risk.to_ascii_lowercase().as_str() {
        "low" | "info" => 0,
        "medium" => 1,
        "high" => 2,
        "critical" => 3,
        "nuclear" => 4,
        _ => -1,
    }
}

/// True when `proposed` is strictly weaker than `original`. Empty proposed is
/// treated as "no change" (donor: empty does not fire).
pub fn is_degrade(original: &str, proposed: &str) -> bool {
    !proposed.is_empty() && risk_rank(proposed) < risk_rank(original)
}

/// No-degrade result. `Triggered` carries the from/to labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoDegradeCheck {
    Pass,
    Triggered { from: String, to: String },
}

pub fn check_no_degrade(original: &str, proposed: &str) -> NoDegradeCheck {
    if is_degrade(original, proposed) {
        NoDegradeCheck::Triggered {
            from: original.to_string(),
            to: proposed.to_string(),
        }
    } else {
        NoDegradeCheck::Pass
    }
}

/// Phase that failed inside [`run_fail_closed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailClosedPhase {
    Verify,
    Prepare,
    Apply,
}

impl FailClosedPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verify => "verify",
            Self::Prepare => "prepare",
            Self::Apply => "apply",
        }
    }
}

impl std::fmt::Display for FailClosedPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct FailClosedError<E> {
    pub phase: FailClosedPhase,
    pub source: E,
}

impl<E: std::fmt::Display> std::fmt::Display for FailClosedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fail-closed phase `{}` failed: {}",
            self.phase, self.source
        )
    }
}

impl<E: std::error::Error + 'static> std::error::Error for FailClosedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

pub trait VerifyPhase {
    type Error;
    fn verify(&mut self) -> Result<(), Self::Error>;
}

pub trait PreparePhase {
    type Error;
    fn prepare(&mut self) -> Result<(), Self::Error>;
}

pub trait ApplyPhase {
    type Error;
    fn apply(&mut self) -> Result<(), Self::Error>;
}

/// Run verify → prepare → apply. A failure short-circuits later phases.
pub fn run_fail_closed<PV, PP, PA, E>(
    mut verify: PV,
    mut prepare: PP,
    mut apply: PA,
) -> Result<(), FailClosedError<E>>
where
    PV: VerifyPhase<Error = E>,
    PP: PreparePhase<Error = E>,
    PA: ApplyPhase<Error = E>,
{
    if let Err(source) = verify.verify() {
        return Err(FailClosedError {
            phase: FailClosedPhase::Verify,
            source,
        });
    }
    if let Err(source) = prepare.prepare() {
        return Err(FailClosedError {
            phase: FailClosedPhase::Prepare,
            source,
        });
    }
    if let Err(source) = apply.apply() {
        return Err(FailClosedError {
            phase: FailClosedPhase::Apply,
            source,
        });
    }
    Ok(())
}

/// Four regression-assertion kinds recovered from donor `apeireth-verify`.
/// Global registries / macros / OnceLock traces are discarded.
#[derive(Debug, Clone, PartialEq)]
pub enum RegressionAssertion {
    InRange {
        name: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },
    Monotonic {
        name: &'static str,
        values: Vec<f64>,
        increasing: bool,
    },
    Idempotent {
        name: &'static str,
        first: String,
        second: String,
    },
    Equivalent {
        name: &'static str,
        left: String,
        right: String,
    },
}

impl RegressionAssertion {
    pub fn check(&self) -> Result<(), String> {
        match self {
            Self::InRange {
                name,
                value,
                min,
                max,
            } => {
                if *value < *min || *value > *max {
                    Err(format!(
                        "[InRange:{name}] value={value} not in [{min}, {max}]"
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Monotonic {
                name,
                values,
                increasing,
            } => {
                for window in values.windows(2) {
                    let good = if *increasing {
                        window[0] <= window[1]
                    } else {
                        window[0] >= window[1]
                    };
                    if !good {
                        return Err(format!(
                            "[Monotonic:{name}] values={values:?} not {}monotonic",
                            if *increasing { "" } else { "de" }
                        ));
                    }
                }
                Ok(())
            }
            Self::Idempotent {
                name,
                first,
                second,
            } => {
                if first != second {
                    Err(format!("[Idempotent:{name}] {first:?} != {second:?}"))
                } else {
                    Ok(())
                }
            }
            Self::Equivalent { name, left, right } => {
                if left != right {
                    Err(format!("[Equivalent:{name}] {left:?} != {right:?}"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_match_donor_table() {
        assert_eq!(risk_rank("info"), 0);
        assert_eq!(risk_rank("LOW"), 0);
        assert_eq!(risk_rank("medium"), 1);
        assert_eq!(risk_rank("high"), 2);
        assert_eq!(risk_rank("critical"), 3);
        assert_eq!(risk_rank("nuclear"), 4);
        assert_eq!(risk_rank("mystery"), -1);
    }

    #[test]
    fn no_degrade_rejects_high_to_low() {
        assert!(matches!(
            check_no_degrade("high", "low"),
            NoDegradeCheck::Triggered { .. }
        ));
        assert!(matches!(
            check_no_degrade("high", "critical"),
            NoDegradeCheck::Pass
        ));
        assert!(matches!(check_no_degrade("high", ""), NoDegradeCheck::Pass));
        assert!(matches!(
            check_no_degrade("high", "high"),
            NoDegradeCheck::Pass
        ));
    }

    struct Recording {
        ran: Vec<FailClosedPhase>,
        verify: Result<(), String>,
        prepare: Result<(), String>,
        apply: Result<(), String>,
    }

    impl VerifyPhase for &mut Recording {
        type Error = String;
        fn verify(&mut self) -> Result<(), String> {
            self.ran.push(FailClosedPhase::Verify);
            self.verify.clone()
        }
    }
    impl PreparePhase for &mut Recording {
        type Error = String;
        fn prepare(&mut self) -> Result<(), String> {
            self.ran.push(FailClosedPhase::Prepare);
            self.prepare.clone()
        }
    }
    impl ApplyPhase for &mut Recording {
        type Error = String;
        fn apply(&mut self) -> Result<(), String> {
            self.ran.push(FailClosedPhase::Apply);
            self.apply.clone()
        }
    }

    #[test]
    fn fail_closed_skips_later_phases_on_verify_error() {
        let mut verify = Recording {
            ran: Vec::new(),
            verify: Err("no".into()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        let mut prepare = Recording {
            ran: Vec::new(),
            verify: Ok(()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        let mut apply = Recording {
            ran: Vec::new(),
            verify: Ok(()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        let err = run_fail_closed(&mut verify, &mut prepare, &mut apply).unwrap_err();
        assert_eq!(err.phase, FailClosedPhase::Verify);
        assert_eq!(verify.ran, vec![FailClosedPhase::Verify]);
        assert!(prepare.ran.is_empty());
        assert!(apply.ran.is_empty());
    }

    #[test]
    fn fail_closed_runs_all_on_success() {
        let mut verify = Recording {
            ran: Vec::new(),
            verify: Ok(()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        let mut prepare = Recording {
            ran: Vec::new(),
            verify: Ok(()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        let mut apply = Recording {
            ran: Vec::new(),
            verify: Ok(()),
            prepare: Ok(()),
            apply: Ok(()),
        };
        run_fail_closed(&mut verify, &mut prepare, &mut apply).unwrap();
        assert_eq!(verify.ran, vec![FailClosedPhase::Verify]);
        assert_eq!(prepare.ran, vec![FailClosedPhase::Prepare]);
        assert_eq!(apply.ran, vec![FailClosedPhase::Apply]);
    }

    #[test]
    fn regression_assertions() {
        assert!(RegressionAssertion::InRange {
            name: "x",
            value: 0.5,
            min: 0.0,
            max: 1.0,
        }
        .check()
        .is_ok());
        assert!(RegressionAssertion::InRange {
            name: "x",
            value: 1.5,
            min: 0.0,
            max: 1.0,
        }
        .check()
        .is_err());
        assert!(RegressionAssertion::Monotonic {
            name: "up",
            values: vec![1.0, 2.0, 2.0],
            increasing: true,
        }
        .check()
        .is_ok());
        assert!(RegressionAssertion::Idempotent {
            name: "id",
            first: "a".into(),
            second: "a".into(),
        }
        .check()
        .is_ok());
        assert!(RegressionAssertion::Equivalent {
            name: "eq",
            left: "x".into(),
            right: "y".into(),
        }
        .check()
        .is_err());
    }
}
