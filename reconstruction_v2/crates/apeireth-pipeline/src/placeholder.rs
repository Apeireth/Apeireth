//! Recursive placeholder resolver (VCP §6.2.2 #17).

use std::collections::HashMap;
use crate::PipelineError;

pub const MAX_RECURSION_DEPTH: usize = 16;
pub const PLACEHOLDER_REGEX_STR: &str = r"\{\{([a-zA-Z0-9_]+)\}\}";

/// Placeholder context with variable bindings.
#[derive(Debug, Clone, Default)]
pub struct PlaceholderContext {
    pub vars: HashMap<String, String>,
    pub depth: usize,
}

impl PlaceholderContext {
    pub fn new() -> Self { Self::default() }
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.vars.insert(k.into(), v.into());
    }
}

fn regex_lite() -> regex::Regex {
    regex::Regex::new(PLACEHOLDER_REGEX_STR).unwrap()
}

/// Resolve `{{var}}` placeholders. Recursive with MAX_RECURSION_DEPTH cap.
pub fn resolve_placeholders(input: &str, ctx: &mut PlaceholderContext) -> Result<String, PipelineError> {
    let mut out = input.to_string();
    let mut depth = 0;
    let re = regex_lite();

    loop {
        if depth >= MAX_RECURSION_DEPTH {
            return Err(PipelineError::Placeholder(format!("recursion depth {depth}")));
        }
        let mut changed = false;
        let mut new_out = String::with_capacity(out.len());
        let mut last_end = 0;
        for caps in re.captures_iter(&out) {
            let m = caps.get(0).unwrap();
            new_out.push_str(&out[last_end..m.start()]);
            let var = caps.get(1).unwrap().as_str().to_string();
            match ctx.vars.get(&var) {
                Some(v) => {
                    new_out.push_str(v);
                    changed = true;
                }
                None => {
                    new_out.push_str(m.as_str());
                }
            }
            last_end = m.end();
        }
        new_out.push_str(&out[last_end..]);
        if !changed { break; }
        out = new_out;
        depth += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_resolve() {
        let mut ctx = PlaceholderContext::new();
        ctx.set("name", "world");
        let r = resolve_placeholders("hello {{name}}", &mut ctx).unwrap();
        assert_eq!(r, "hello world");
    }

    #[test]
    fn nested_resolve() {
        let mut ctx = PlaceholderContext::new();
        ctx.set("a", "{{b}}");
        ctx.set("b", "final");
        let r = resolve_placeholders("{{a}}", &mut ctx).unwrap();
        assert_eq!(r, "final");
    }

    #[test]
    fn recursion_limit() {
        let mut ctx = PlaceholderContext::new();
        ctx.set("a", "{{a}}");
        let r = resolve_placeholders("{{a}}", &mut ctx);
        assert!(matches!(r, Err(PipelineError::Placeholder(_))));
    }

    #[test]
    fn missing_var_passthrough() {
        let mut ctx = PlaceholderContext::new();
        let r = resolve_placeholders("{{missing}}", &mut ctx).unwrap();
        assert_eq!(r, "{{missing}}");
    }
}
