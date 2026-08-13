//! Calculator (meval 0.2) — replaces VCP mathjs 100KB dependency.
//!
//! Supports:
//! - arithmetic (+, -, *, /, %)
//! - functions (sin, cos, tan, log, exp, sqrt, abs, pow, ...)
//! - constants (pi, e)
//! - variables via expr.context("x", 1.0)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalcError {
    #[error("parse error: `{0}`")]
    Parse(String),
    #[error("eval error: `{0}`")]
    Eval(String),
}

pub fn evaluate_expression(expr: &str) -> Result<f64, CalcError> {
    meval::eval_str(expr).map_err(|e| CalcError::Eval(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_arithmetic() {
        assert_eq!(evaluate_expression("1 + 2 * 3").unwrap(), 7.0);
    }

    #[test]
    fn constants() {
        let v = evaluate_expression("pi * 2").unwrap();
        assert!((v - std::f64::consts::TAU).abs() < 1e-10);
    }

    #[test]
    fn function_call() {
        let v = evaluate_expression("sqrt(16)").unwrap();
        assert_eq!(v, 4.0);
    }

    #[test]
    fn invalid_expr_errors() {
        assert!(evaluate_expression("1 +").is_err());
    }
}
