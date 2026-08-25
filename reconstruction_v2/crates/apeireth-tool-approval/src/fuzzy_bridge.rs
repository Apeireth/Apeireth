use apeireth_tool_registry::ToolRegistry;

pub struct FuzzyToolMatcher;

impl FuzzyToolMatcher {
    pub fn match_tool(marker: &str, registry: &ToolRegistry) -> Option<String> {
        Self::match_tool_threshold(marker, registry, 2)
    }
    pub fn match_tool_threshold(marker: &str, registry: &ToolRegistry, max_distance: usize) -> Option<String> {
        if marker.is_empty() { return None; }
        let names = registry.list();
        if names.is_empty() { return None; }
        let m_lower = marker.to_lowercase();
        for name in &names {
            if name.to_lowercase() == m_lower {
                return Some(name.clone());
            }
        }
        let mut best: Option<(usize, String)> = None;
        for name in &names {
            let d = levenshtein(&m_lower, &name.to_lowercase());
            if d <= max_distance {
                if best.as_ref().map(|x| d < x.0).unwrap_or(true) {
                    best = Some((d, name.clone()));
                }
            }
        }
        best.map(|(_, n)| n)
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() { dp[i][0] = i; }
    for j in 0..=b.len() { dp[0][j] = j; }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i-1] == b[j-1] { 0 } else { 1 };
            let m1 = dp[i-1][j] + 1;
            let m2 = dp[i][j-1] + 1;
            let m3 = dp[i-1][j-1] + cost;
            dp[i][j] = m1.min(m2).min(m3);
        }
    }
    dp[a.len()][b.len()]
}

pub fn match_tool_name(marker: &str, registry: &ToolRegistry) -> Option<String> {
    FuzzyToolMatcher::match_tool(marker, registry)
}

pub fn match_tool_name_threshold(marker: &str, registry: &ToolRegistry, max_distance: usize) -> Option<String> {
    FuzzyToolMatcher::match_tool_threshold(marker, registry, max_distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::MockSyncTool;
    use std::sync::Arc;

    fn make_tool(name: &str) -> Arc<dyn apeireth_tool_registry::Tool> {
        Arc::new(MockSyncTool { name: name.to_string() })
    }

    fn registry_with_tools() -> ToolRegistry {
        let r = ToolRegistry::new();
        r.register("FileOperator".to_string(), make_tool("FileOperator"));
        r.register("WeatherQuery".to_string(), make_tool("WeatherQuery"));
        r.register("Greeting".to_string(), make_tool("Greeting"));
        r
    }

    #[test]
    fn fuzzy_exact_match() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("FileOperator", &r), Some("FileOperator".to_string()));
    }

    #[test]
    fn fuzzy_typo_tolerance() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("FileOperater", &r), Some("FileOperator".to_string()));
        assert_eq!(match_tool_name("WeatherQuary", &r), Some("WeatherQuery".to_string()));
    }

    #[test]
    fn fuzzy_no_match_too_far() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("CompletelyDifferent", &r), None);
    }

    #[test]
    fn fuzzy_empty_marker() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("", &r), None);
    }

    #[test]
    fn fuzzy_empty_registry() {
        let r = ToolRegistry::new();
        assert_eq!(match_tool_name("FileOperator", &r), None);
    }

    #[test]
    fn fuzzy_threshold_strict() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name_threshold("FileOperator", &r, 0), Some("FileOperator".to_string()));
        assert_eq!(match_tool_name_threshold("FileOperater", &r, 0), None);
        assert_eq!(match_tool_name_threshold("FileOperater", &r, 1), Some("FileOperator".to_string()));
    }

    #[test]
    fn fuzzy_case_insensitive() {
        let r = registry_with_tools();
        assert_eq!(match_tool_name("fileoperator", &r), Some("FileOperator".to_string()));
    }
}