//! `file://` URI parsing and directory containment.
//!
//! Donor: `legacy/donor/apeireth-mcp/src/resource_servers.rs`
//! (`extract_path` / `resolve_safe` / `percent_decode` / `guess_mime`).
//!
//! Recovered as **pure functions**. The donor `FileResourceServer` also
//! walked the filesystem and depended on `apeireth-tools` conventions;
//! that host is deferred (it would become a second I/O owner next to
//! `apeireth-tools::filesystem`). A later host can call these helpers
//! without importing the old crate.

use std::path::{Component, Path, PathBuf};

/// Failures while parsing or containing a `file://` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriError {
    /// URI did not start with the expected scheme prefix.
    BadScheme { expected: &'static str, got: String },
    /// Absolute path component after the scheme (rejected).
    AbsolutePath(String),
    /// `..` component (rejected before canonicalize).
    ParentDir(String),
    /// Canonicalize / join failed, or result escaped `base`.
    Escaped { path: String, base: String },
}

impl std::fmt::Display for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadScheme { expected, got } => {
                write!(f, "URI must start with {expected}, got {got}")
            }
            Self::AbsolutePath(p) => write!(f, "absolute path not allowed: {p}"),
            Self::ParentDir(p) => write!(f, "`..` not allowed: {p}"),
            Self::Escaped { path, base } => {
                write!(f, "path {path} escapes base {base}")
            }
        }
    }
}

impl std::error::Error for UriError {}

/// RFC 3986 percent-decode. `+` is **not** treated as space (not form encoding).
/// Invalid `%XX` sequences are copied through as literal bytes.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Strip `file:///` and reject absolute / `..` components.
///
/// Returns a **relative** path. Does not touch the filesystem.
pub fn parse_file_uri(uri: &str) -> Result<PathBuf, UriError> {
    const PREFIX: &str = "file:///";
    let path_str = uri
        .strip_prefix(PREFIX)
        .ok_or_else(|| UriError::BadScheme {
            expected: PREFIX,
            got: uri.to_string(),
        })?;
    let decoded = percent_decode(path_str);
    let p = Path::new(&decoded);
    if p.is_absolute() {
        return Err(UriError::AbsolutePath(decoded));
    }
    reject_escape(p)?;
    Ok(p.to_path_buf())
}

/// Reject any `..` component. Does not canonicalize.
pub fn reject_escape(path: &Path) -> Result<(), UriError> {
    for comp in path.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(UriError::ParentDir(path.display().to_string()));
        }
    }
    Ok(())
}

/// Join `rel` onto `base`, canonicalize both, and require the result still
/// starts with the canonical base. This is the only function here that
/// performs I/O (canonicalize). Callers that want a purely in-memory check
/// should use [`reject_escape`] instead.
pub fn resolve_contained(base: &Path, rel: &Path) -> Result<PathBuf, UriError> {
    reject_escape(rel)?;
    let joined = base.join(rel);
    let canonical = joined.canonicalize().map_err(|_| UriError::Escaped {
        path: joined.display().to_string(),
        base: base.display().to_string(),
    })?;
    let base_canonical = base.canonicalize().map_err(|_| UriError::Escaped {
        path: joined.display().to_string(),
        base: base.display().to_string(),
    })?;
    if !canonical.starts_with(&base_canonical) {
        return Err(UriError::Escaped {
            path: canonical.display().to_string(),
            base: base_canonical.display().to_string(),
        });
    }
    Ok(canonical)
}

/// Extension → MIME guess (donor table, common text types only).
pub fn guess_mime(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => "text/x-rust",
        Some("toml") => "text/x-toml",
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("py") => "text/x-python",
        Some("js") | Some("mjs") => "text/javascript",
        Some("ts") => "text/typescript",
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("yaml") | Some("yml") => "text/yaml",
        Some("sh") | Some("bash") => "text/x-shellscript",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_space_and_passthrough() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("a%2Fb"), "a/b");
        assert_eq!(percent_decode("plain"), "plain");
        // invalid % sequence copied through
        assert_eq!(percent_decode("a%ZZb"), "a%ZZb");
        // '+' is not space
        assert_eq!(percent_decode("a+b"), "a+b");
    }

    #[test]
    fn parse_file_uri_relative() {
        let p = parse_file_uri("file:///src/main.rs").unwrap();
        assert_eq!(p, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn parse_file_uri_percent_decoded() {
        let p = parse_file_uri("file:///hello%20world.rs").unwrap();
        assert_eq!(p, PathBuf::from("hello world.rs"));
    }

    #[test]
    fn parse_file_uri_rejects_parent_dir() {
        let err = parse_file_uri("file:///../etc/passwd").unwrap_err();
        assert!(matches!(err, UriError::ParentDir(_)));
    }

    #[test]
    fn parse_file_uri_rejects_bad_scheme() {
        let err = parse_file_uri("organ://memory").unwrap_err();
        assert!(matches!(err, UriError::BadScheme { .. }));
    }

    #[test]
    fn parse_file_uri_nested_parent_dir() {
        let err = parse_file_uri("file:///src/../../etc/passwd").unwrap_err();
        assert!(matches!(err, UriError::ParentDir(_)));
    }

    #[test]
    fn reject_escape_allows_dot_and_normal() {
        reject_escape(Path::new("src/./main.rs")).unwrap();
        reject_escape(Path::new("src/main.rs")).unwrap();
    }

    #[test]
    fn guess_mime_table() {
        assert_eq!(guess_mime(Path::new("a.rs")), "text/x-rust");
        assert_eq!(guess_mime(Path::new("a.md")), "text/markdown");
        assert_eq!(guess_mime(Path::new("a.json")), "application/json");
        assert_eq!(guess_mime(Path::new("a.bin")), "application/octet-stream");
    }

    #[test]
    fn resolve_contained_keeps_in_base() {
        let tmp = std::env::temp_dir();
        let rel = Path::new(".");
        let out = resolve_contained(&tmp, rel).unwrap();
        let base = tmp.canonicalize().unwrap();
        assert!(out.starts_with(&base));
    }

    #[test]
    fn resolve_contained_rejects_parent_before_io() {
        let err = resolve_contained(Path::new("/tmp"), Path::new("../etc")).unwrap_err();
        assert!(matches!(err, UriError::ParentDir(_)));
    }
}
