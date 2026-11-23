//! Bounded text operations within an already-authorized project directory.
//! The caller holds the workspace registry lock for the operation.

use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sensitive_path::{is_sensitive_path, redact_secret_field_values};

mod directory;
use directory::Directory;

pub const MAX_TEXT_BYTES: usize = 128 * 1024;
const MISSING: &str = "missing";
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

fn valid_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(
        stem.trim_end_matches(' '),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix.trim_end_matches(' '),
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    !name.is_empty()
        && name.len() <= 160
        && !name.starts_with('.')
        && !name.ends_with(['.', ' '])
        && !device
        && !name
            .chars()
            .any(|c| c.is_control() || "/\\:<\">|?*".contains(c))
}

fn revision(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// Validate the frozen user-visible input without touching the filesystem.
pub fn valid_write(name: &str, expected_revision: &str, text: &str) -> bool {
    valid_name(name)
        && (expected_revision == MISSING
            || (expected_revision.len() == 64
                && expected_revision
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))))
        && text.len() <= MAX_TEXT_BYTES
        && !text.contains('\0')
}

fn directory(root: &Path, name: &str) -> Result<Directory, &'static str> {
    if !valid_name(name) || is_sensitive_path(root, &root.join(name)) {
        return Err("project files require a non-sensitive file name directly in the workspace");
    }
    Directory::open(root).map_err(|_| "project directory is unavailable")
}

fn text(directory: &Directory, name: &str) -> Result<Option<String>, &'static str> {
    let file = match directory.open_file(name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("project file cannot be read"),
    };
    let mut bytes = Vec::new();
    file.take((MAX_TEXT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "project file cannot be read")?;
    if bytes.len() > MAX_TEXT_BYTES || bytes.contains(&0) {
        return Err("project file exceeds the text limit or is not plain text");
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "project file is not UTF-8 text")
}

/// Read a single file. A missing file has an explicit creation revision.
pub fn read_text(root: &Path, name: &str) -> Result<Value, &'static str> {
    match text(&directory(root, name)?, name)? {
        Some(text) => Ok(
            json!({"name":name,"revision":revision(&text),"text":redact_secret_field_values(&text)}),
        ),
        None => Ok(json!({"name":name,"revision":MISSING})),
    }
}

/// Persist the text after comparing the content revision observed by the caller.
/// Replacing an existing file is atomic within the directory. Creating a new
/// file uses a no-replace move so it cannot overwrite a concurrent creation.
pub fn write_text(
    root: &Path,
    name: &str,
    expected_revision: &str,
    value: &str,
) -> Result<Value, &'static str> {
    if !valid_write(name, expected_revision, value) {
        return Err("project write arguments or revision are invalid");
    }
    let directory = directory(root, name)?;
    let current = text(&directory, name)?;
    if current
        .as_ref()
        .is_some_and(|text| redact_secret_field_values(text) != *text)
    {
        return Err("project file contains protected fields");
    }
    let actual = current
        .as_deref()
        .map(revision)
        .unwrap_or_else(|| MISSING.into());
    if actual != expected_revision {
        return Err("project file revision changed; read it again before writing");
    }
    let stage = format!(
        ".apeireth-write-{}-{}",
        std::process::id(),
        NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
    );
    let mut file = directory
        .create_stage(&stage)
        .map_err(|_| "project write could not be staged")?;
    let prepared = file
        .write_all(value.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    let result = prepared.and_then(|()| directory.publish(&stage, name, current.is_none()));
    directory.discard_stage(&stage);
    result.map_err(|_| "project write could not be confirmed")?;
    Ok(json!({"name":name,"revision":revision(value)}))
}

#[cfg(test)]
mod tests;
