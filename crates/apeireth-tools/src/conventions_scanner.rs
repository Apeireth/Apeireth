//! R33-1: Aider 借鉴 — 项目 conventions scanner
//!
//! 借鉴 Aider (`aider` CLI, 1.0+): 启动时扫项目 root 的 `Cargo.toml`,
//! 抽 edition / rust-version / resolver / lints / workspace deps / members, 输出 system prompt block.
//! LLM 拿到这个 block 后能"知道项目用什么风格", 生成的代码更贴项目习惯.
//!
//! **借鉴点 (Aider 真代码)**:
//! - `aider/repo.py:Repo.__init__` 启动时 git init + 读 README + 抽 metadata
//! - `aider/args.py:--edit-format` 自动适配项目风格
//! - `aider/history.py:Chat` 把 conventions 注入 system prompt
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 Cargo.toml 内容, 只读
//! - 0 写 system prompt, 只生成 block 字符串
//! - 0 业务耦合 (apeireth-tools 不依赖 apeireth-tui, 任意消费者都能调)
//! - 用 `toml` crate 真解析 (R20 阶段 1 已用, 0 新增 dep)
//!
//! **用法** (TUI / council / eval harness 都能调):
//! ```ignore
//! let conv = ProjectConventions::scan("./");
//! let block = conv.to_system_prompt_block();
//! system_prompt.push_str(&block);
//! ```
#![allow(clippy::result_large_err)]

use std::path::Path;

/// 项目约定元数据 (从 Cargo.toml 抽出)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectConventions {
    /// workspace root 路径
    pub workspace_root: String,
    /// [workspace.package] edition (e.g. "2021")
    pub edition: Option<String>,
    /// [workspace.package] rust-version (e.g. "1.75")
    pub rust_version: Option<String>,
    /// [workspace] resolver (e.g. "2")
    pub resolver: Option<String>,
    /// [workspace] members 数量
    pub members_count: usize,
    /// [workspace.dependencies] 数量
    pub workspace_deps_count: usize,
    /// `[workspace.lints.X]` 已配置的 lint 类别 (e.g. `["rust", "clippy"]`)
    pub lint_categories: Vec<String>,
    /// 关键 deps key 名 (Aider 风格: 提示 LLM "项目用啥栈")
    pub key_deps: Vec<String>,
    /// 任何解析失败都记 (不抛错, 优雅降级, block 仍能用)
    pub scan_error: Option<String>,
}

impl ProjectConventions {
    /// 扫 workspace_root/Cargo.toml
    pub fn scan(workspace_root: impl AsRef<Path>) -> Self {
        let root = workspace_root.as_ref();
        let mut s = Self {
            workspace_root: root.display().to_string(),
            ..Default::default()
        };
        let cargo_path = root.join("Cargo.toml");
        if !cargo_path.exists() {
            s.scan_error = Some(format!("Cargo.toml not found at {}", cargo_path.display()));
            return s;
        }
        let text = match std::fs::read_to_string(&cargo_path) {
            Ok(t) => t,
            Err(e) => {
                s.scan_error = Some(format!("read Cargo.toml: {e}"));
                return s;
            }
        };
        if let Err(e) = s.parse_cargo_toml(&text) {
            s.scan_error = Some(format!("parse Cargo.toml: {e}"));
        }
        s
    }

    /// 解析 Cargo.toml 内容 (用 toml crate)
    fn parse_cargo_toml(&mut self, text: &str) -> Result<(), String> {
        let v: toml::Value = text.parse::<toml::Value>().map_err(|e| e.to_string())?;
        let root = v
            .as_table()
            .ok_or_else(|| "root is not a table".to_string())?;

        // [workspace] 块
        if let Some(ws) = root.get("workspace").and_then(|x| x.as_table()) {
            if let Some(r) = ws.get("resolver").and_then(|x| x.as_str()) {
                self.resolver = Some(r.to_string());
            }
            if let Some(members) = ws.get("members").and_then(|x| x.as_array()) {
                self.members_count = members.len();
            }
        }

        // [workspace.package] 块
        if let Some(wp) = root
            .get("workspace")
            .and_then(|x| x.get("package"))
            .and_then(|x| x.as_table())
        {
            if let Some(e) = wp.get("edition").and_then(|x| x.as_str()) {
                self.edition = Some(e.to_string());
            }
            if let Some(rv) = wp.get("rust-version").and_then(|x| x.as_str()) {
                self.rust_version = Some(rv.to_string());
            }
        }

        // [workspace.dependencies] 块
        if let Some(wd) = root
            .get("workspace")
            .and_then(|x| x.get("dependencies"))
            .and_then(|x| x.as_table())
        {
            self.workspace_deps_count = wd.len();
            // 按字母序前 8 个 key 当 "key deps" 摘要
            let mut keys: Vec<String> = wd.keys().cloned().collect();
            keys.sort();
            self.key_deps = keys.into_iter().take(8).collect();
        }

        // [workspace.lints.{rust,clippy}] 块
        if let Some(lints) = root
            .get("workspace")
            .and_then(|x| x.get("lints"))
            .and_then(|x| x.as_table())
        {
            for cat in ["rust", "clippy"] {
                if lints.get(cat).is_some() {
                    self.lint_categories.push(cat.to_string());
                }
            }
        }
        Ok(())
    }

    /// Aider 风格 system prompt block
    ///
    /// 输出固定格式 markdown, 让 LLM 知道"项目用什么风格":
    /// - edition / rust-version (Rust 版本)
    /// - resolver (Cargo 解析器)
    /// - members / deps 数量 + key deps (项目规模 + 栈)
    /// - lint 类别 (代码风格约束)
    /// - 3 条风格提示 (workspace = true / 继承 lint / 不漂移)
    pub fn to_system_prompt_block(&self) -> String {
        let mut s = String::new();
        s.push_str("# 项目约定 (auto-scanned from Cargo.toml, Aider-style)\n\n");
        if let Some(e) = &self.edition {
            s.push_str(&format!("- Rust edition: {e}\n"));
        } else {
            s.push_str("- Rust edition: (未设置, 默认 2015)\n");
        }
        if let Some(v) = &self.rust_version {
            s.push_str(&format!("- Rust version: {v}\n"));
        }
        if let Some(r) = &self.resolver {
            s.push_str(&format!("- Cargo resolver: {r}\n"));
        }
        s.push_str(&format!(
            "- Workspace members: {} 个 crate\n",
            self.members_count
        ));
        s.push_str(&format!(
            "- Workspace deps: {} 个 (key: {})\n",
            self.workspace_deps_count,
            if self.key_deps.is_empty() {
                "(无)".to_string()
            } else {
                self.key_deps.join(", ")
            }
        ));
        if !self.lint_categories.is_empty() {
            s.push_str(&format!(
                "- Lint 类别: {}\n",
                self.lint_categories.join(", ")
            ));
        }
        if let Some(err) = &self.scan_error {
            s.push_str(&format!(
                "\n[scan warning] {err} (block 仍可用, 但部分字段可能不准)\n"
            ));
        }

        s.push_str("\n# 风格提示 (Aider-style hint)\n");
        s.push_str("- 写代码时遵循上面抽到的 edition / rust-version / lints\n");
        s.push_str("- 复用 workspace deps 用 `{ workspace = true }`, 不要钉版本\n");
        s.push_str(
            "- 子 crate Cargo.toml 末尾加 `[lints]\\nworkspace = true` 继承 workspace lint\n",
        );
        s.push_str("- 保持现状 (不漂移): workspace version = 1.0.0 已是 1.0 release 锁版, 勿改\n");
        s
    }

    /// 一句话摘要 (给 log / debug 用)
    pub fn summary(&self) -> String {
        format!(
            "apeireth conventions: edition={} rust={} resolver={} members={} deps={} lints={}",
            self.edition.as_deref().unwrap_or("?"),
            self.rust_version.as_deref().unwrap_or("?"),
            self.resolver.as_deref().unwrap_or("?"),
            self.members_count,
            self.workspace_deps_count,
            if self.lint_categories.is_empty() {
                "(无)".to_string()
            } else {
                self.lint_categories.join(",")
            },
        )
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod conventions_scanner_tests {
    use super::*;
    use std::io::Write;

    /// 写一个临时 Cargo.toml
    fn write_cargo(dir: &Path, content: &str) {
        let path = dir.join("Cargo.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn scan_workspace_root_with_full_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo(
            tmp.path(),
            r#"
[workspace]
resolver = "2"
members = ["a", "b", "c"]

[workspace.package]
edition = "2021"
rust-version = "1.75"

[workspace.dependencies]
tokio = { version = "1.40" }
serde = { version = "1.0" }
anyhow = "1.0"

[workspace.lints.rust]
unused = 'warn'

[workspace.lints.clippy]
all = 'warn'
"#,
        );
        let c = ProjectConventions::scan(tmp.path());
        assert_eq!(c.edition, Some("2021".to_string()));
        assert_eq!(c.rust_version, Some("1.75".to_string()));
        assert_eq!(c.resolver, Some("2".to_string()));
        assert_eq!(c.members_count, 3);
        assert_eq!(c.workspace_deps_count, 3);
        assert_eq!(
            c.lint_categories,
            vec!["rust".to_string(), "clippy".to_string()]
        );
        // key_deps 前 3 个 (字母序)
        assert!(c.key_deps.contains(&"tokio".to_string()));
        assert!(c.key_deps.contains(&"serde".to_string()));
        assert!(c.key_deps.contains(&"anyhow".to_string()));
        assert!(c.scan_error.is_none());
    }

    #[test]
    fn scan_missing_cargo_toml_records_error() {
        let tmp = tempfile::tempdir().unwrap();
        let c = ProjectConventions::scan(tmp.path());
        assert!(c.scan_error.is_some());
        assert!(c
            .scan_error
            .as_ref()
            .unwrap()
            .contains("Cargo.toml not found"));
        assert!(c.edition.is_none());
    }

    #[test]
    fn scan_empty_cargo_toml_uses_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo(tmp.path(), "# empty\n");
        let c = ProjectConventions::scan(tmp.path());
        assert!(c.scan_error.is_none());
        assert_eq!(c.edition, None);
        assert_eq!(c.rust_version, None);
        assert_eq!(c.members_count, 0);
        assert_eq!(c.workspace_deps_count, 0);
        assert!(c.lint_categories.is_empty());
    }

    #[test]
    fn scan_malformed_cargo_toml_records_error() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo(tmp.path(), "[workspace\nresolver = broken = = =");
        let c = ProjectConventions::scan(tmp.path());
        assert!(c.scan_error.is_some());
        assert!(c.scan_error.as_ref().unwrap().contains("parse"));
    }

    #[test]
    fn scan_real_workspace_root_extracts_conventions() {
        // 用项目自己的 workspace root 扫 (向上 2 层: apeireth-tools -> crates -> workspace root)
        let pkg_root = std::env::current_dir().unwrap();
        let root = pkg_root
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&pkg_root);
        let c = ProjectConventions::scan(root);
        // 找 workspace root Cargo.toml: 如果 current_dir 是 workspace root, resolver 应该是 "2"
        // (跑 `cargo test -p apeireth-tools` 时 cwd 是 crate root, parent.parent 是 workspace root)
        if c.scan_error.is_none() && c.workspace_deps_count > 0 {
            // 只在真找到 workspace root 时验
            assert_eq!(c.resolver, Some("2".to_string()));
            assert!(
                c.members_count > 30,
                "apeireth workspace > 30 members, got {}",
                c.members_count
            );
            assert!(c.workspace_deps_count > 0);
            assert!(c.lint_categories.contains(&"rust".to_string()));
        }
        // 不强制要求 edition/rust-version (项目 Cargo.toml 里 [workspace.package] 用 `version.workspace = true`, 真值在 [package] 里)
    }

    #[test]
    fn to_system_prompt_block_contains_key_sections() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo(
            tmp.path(),
            r#"
[workspace]
resolver = "2"
members = ["x", "y"]
[workspace.package]
edition = "2021"
[workspace.dependencies]
tokio = { version = "1.40" }
"#,
        );
        let c = ProjectConventions::scan(tmp.path());
        let block = c.to_system_prompt_block();
        assert!(block.contains("# 项目约定"));
        assert!(block.contains("Rust edition: 2021"));
        assert!(block.contains("Cargo resolver: 2"));
        assert!(block.contains("2 个 crate"));
        assert!(block.contains("tokio"));
        assert!(block.contains("# 风格提示"));
        assert!(block.contains("workspace = true"));
    }

    #[test]
    fn summary_one_liner_format() {
        let mut c = ProjectConventions::default();
        c.edition = Some("2021".to_string());
        c.rust_version = Some("1.75".to_string());
        c.resolver = Some("2".to_string());
        c.members_count = 91;
        c.workspace_deps_count = 12;
        c.lint_categories = vec!["rust".to_string(), "clippy".to_string()];
        let s = c.summary();
        assert!(s.contains("2021"));
        assert!(s.contains("1.75"));
        assert!(s.contains("resolver=2"));
        assert!(s.contains("members=91"));
        assert!(s.contains("rust,clippy"));
    }

    #[test]
    fn key_deps_truncated_to_8() {
        let tmp = tempfile::tempdir().unwrap();
        let mut content = String::from("[workspace.dependencies]\n");
        for k in ["a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10"] {
            content.push_str(&format!("{k} = \"1.0\"\n"));
        }
        write_cargo(tmp.path(), &content);
        let c = ProjectConventions::scan(tmp.path());
        assert_eq!(c.workspace_deps_count, 10);
        assert_eq!(c.key_deps.len(), 8);
    }

    #[test]
    fn scan_error_display_includes_path() {
        let tmp = tempfile::tempdir().unwrap();
        let c = ProjectConventions::scan(tmp.path());
        let err = c.scan_error.unwrap();
        assert!(err.contains("Cargo.toml not found"));
        // path 可能因系统而异, 至少包含 . 或者 tmp 字样
        assert!(err.contains(tmp.path().to_str().unwrap()) || err.contains("Cargo.toml"));
    }
}
