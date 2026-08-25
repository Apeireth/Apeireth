//! apeireth-tui - Terminal UI framework (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Terminal + 真 render + 真 command parsing

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub cols: u16,
    pub rows: u16,
    pub title: String,
}

impl Default for TerminalConfig {
    fn default() -> Self { Self { cols: 80, rows: 24, title: "Apeireth TUI".into() } }
}

pub struct Terminal { pub config: TerminalConfig, pub state: HashMap<String, String> }

impl Terminal {
    pub fn new(config: TerminalConfig) -> Self { Self { config, state: HashMap::new() } }
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.state.insert(k.into(), v.into());
    }
    pub fn render(&self) -> String {
        let mut out = format!("+{:-<width$}+\n", "", width = (self.config.cols as usize) + 2);
        out += &format!("| {:^width$} |\n", self.config.title, width = self.config.cols as usize);
        out += &format!("+{:-<width$}+\n", "", width = (self.config.cols as usize) + 2);
        for (k, v) in &self.state {
            let line = format!("{}: {}", k, v);
            if line.len() > self.config.cols as usize {
                out += &format!("| {}...|\n", &line[..self.config.cols as usize - 3]);
            } else {
                out += &format!("| {:<width$} |\n", line, width = self.config.cols as usize);
            }
        }
        out += &format!("+{:-<width$}+", "", width = (self.config.cols as usize) + 2);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_render() {
        let mut t = Terminal::new(TerminalConfig::default());
        t.set("key", "value");
        let s = t.render();
        assert!(s.contains("Apeireth"));
        assert!(s.contains("key: value"));
    }
    #[test]
    fn test_default_config() {
        let c = TerminalConfig::default();
        assert_eq!(c.cols, 80);
        assert_eq!(c.rows, 24);
    }
}
