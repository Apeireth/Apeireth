//! Skill descriptor → eval scenario bridge.
pub fn skill_to_eval_scenario(skill_id: &str) -> String {
    format!("eval:{skill_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constructs() { assert_eq!(skill_to_eval_scenario("x"), "eval:x"); }
}
