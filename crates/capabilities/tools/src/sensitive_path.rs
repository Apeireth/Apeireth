//! Shared workspace path policy for local read tools.
//!
//! This is intentionally narrower than a blanket dotfile ban. Ordinary
//! project metadata such as `.gitignore` and `.cargo/config.toml` remains
//! readable, while common environment files, key material, credential stores,
//! and private-key directories are kept out of filesystem and search results.

use std::path::Path;

/// Whether `path` contains a known sensitive workspace path.
///
/// `root` and `path` should be canonical paths when available. The helper also
/// works with a lexical child path, which lets callers protect a sensitive
/// symlink name before following its target.
pub(crate) fn is_sensitive_path(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let components: Vec<String> = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(|component| component.to_ascii_lowercase())
        .collect();

    for (index, component) in components.iter().enumerate() {
        if is_sensitive_directory(component)
            || is_sensitive_file_name(component)
            || (component == ".config"
                && components
                    .get(index + 1)
                    .is_some_and(|next| next == "gcloud"))
        {
            return true;
        }
    }

    false
}

fn is_sensitive_directory(name: &str) -> bool {
    matches!(name, ".ssh" | ".aws" | ".gnupg" | ".secret" | ".secrets")
}

fn is_sensitive_file_name(name: &str) -> bool {
    if name == ".env" || name.starts_with(".env.") {
        return true;
    }

    if name == "id_rsa"
        || name.starts_with("id_rsa.")
        || name == "id_ed25519"
        || name.starts_with("id_ed25519.")
    {
        return true;
    }

    if name == "credentials"
        || name.starts_with("credentials.")
        || name == "secret"
        || name == "secrets"
        || name.starts_with("secret.")
        || name.starts_with("secrets.")
    {
        return true;
    }

    if ["pem", "key", "p12", "pfx", "jks", "kdbx"]
        .iter()
        .any(|extension| name.ends_with(&format!(".{extension}")))
    {
        return true;
    }

    ["private-key", "private_key", "privatekey"]
        .iter()
        .any(|marker| name.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn protects_known_sensitive_files_and_directories() {
        let root = Path::new("workspace");
        for path in [
            ".env",
            ".env.local",
            "foo.pem",
            "foo.key",
            "foo.p12",
            "foo.pfx",
            "id_rsa",
            "id_rsa.backup",
            "id_ed25519",
            "id_ed25519.pub",
            "credentials",
            "credentials.json",
            "secret",
            "secrets.production",
            ".ssh/config",
            ".aws/credentials",
            ".config/gcloud/application_default_credentials.json",
        ] {
            assert!(
                is_sensitive_path(root, &root.join(path)),
                "expected protected path: {path}"
            );
        }
    }

    #[test]
    fn does_not_block_normal_project_dotfiles() {
        let root = Path::new("workspace");
        for path in [
            ".gitignore",
            ".cargo/config.toml",
            "README.md",
            "Cargo.toml",
            "src/lib.rs",
        ] {
            assert!(
                !is_sensitive_path(root, &PathBuf::from(root).join(path)),
                "unexpected protected path: {path}"
            );
        }
    }
}
