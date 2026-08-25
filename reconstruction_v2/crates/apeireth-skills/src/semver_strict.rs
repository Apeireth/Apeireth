//! Strict semver 2.0.0 (3-segment + pre-release + build metadata).

pub fn is_strict_semver(v: &str) -> bool {
    let main = v.split('-').next().unwrap_or(v);
    let parts: Vec<&str> = main.split('.').collect();
    if parts.len() != 3 { return false; }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid() { assert!(is_strict_semver("1.0.0")); }
    #[test]
    fn invalid_two_segments() { assert!(!is_strict_semver("1.0")); }
    #[test]
    fn invalid_alpha() { assert!(!is_strict_semver("1.0.0a")); }
}
