//! File loader — scan directory for skill JSON files.
use std::path::Path;
use crate::{Registry, Skill, SkillResult};

pub fn load_from_dir(dir: &Path, reg: &mut Registry) -> SkillResult<usize> {
    if !dir.is_dir() { return Ok(0); }
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(skill) = serde_json::from_str::<Skill>(&content) {
                        if reg.register(skill).is_ok() { count += 1; }
                    }
                }
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_nonexistent_dir() {
        let mut reg = Registry::new();
        let c = load_from_dir(std::path::Path::new("/nonexistent"), &mut reg).unwrap();
        assert_eq!(c, 0);
    }
}
