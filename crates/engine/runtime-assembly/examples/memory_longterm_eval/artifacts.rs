//! Artifact plumbing: hashing, write-once files and run directories.
//!
//! Every run artifact is written once and never overwritten. A second attempt
//! to write the same path fails, so a re-run cannot silently replace evidence.

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// The result type of this evaluation: a plain error box, so that every
/// failure carries a readable reason.
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const MAX_READ_BYTES: u64 = 256 * 1024 * 1024;

pub fn hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let out = digest.finalize();
    let mut text = String::with_capacity(out.len() * 2);
    for byte in out {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

pub fn hash(text: &str) -> String {
    hex(text.as_bytes())
}

pub fn file_hash(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(hash_bytes(&bytes))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex(bytes)
}

/// Read and deserialize a JSON artifact, with an upper bound on the file size.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_READ_BYTES {
        return Err(format!("artifact is larger than the read bound: {path:?}").into());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// Write a JSON artifact exactly once.
///
/// ⭐ Every artifact goes through the redaction guard: a string that looks
/// like a provider key is replaced before it reaches the disk, so an
/// accidental echo of a credential cannot end up in a run directory.
pub fn write_new<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|err| format!("artifact is not write-once ({path:?}): {err}"))?;
    let mut text = serde_json::to_string_pretty(value)?;
    text = redact_secrets(&text);
    text.push('\n');
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Replace anything that looks like a provider key with a fixed marker.
///
/// The marker keeps the shape of the artifact (a string is still a string)
/// while removing the credential itself. The rule is deliberately narrow: a
/// token that starts with `sk-` and runs to the next delimiter.
pub fn redact_secrets(text: &str) -> String {
    const MARKER: &str = "[REDACTED]";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"sk-") {
            let mut end = index + 3;
            while end < bytes.len() {
                let byte = bytes[end];
                if byte.is_ascii_whitespace()
                    || byte == b'"'
                    || byte == b'\''
                    || byte == b','
                    || byte == b')'
                    || byte == b'('
                    || byte == b']'
                    || byte == b'}'
                {
                    break;
                }
                end += 1;
            }
            out.push_str(MARKER);
            index = end;
            continue;
        }
        let ch = text[index..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// Create a run directory and its per-call subdirectory.
pub fn new_run(path: &Path) -> Result<PathBuf> {
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join("calls"))?;
    Ok(path.to_path_buf())
}

/// Per-stage latency distribution over the spans recorded during a run.
pub use super::schema::Distribution;

pub fn distributions(spans: &[super::schema::Span]) -> Vec<Distribution> {
    let mut stages: Vec<String> = Vec::new();
    for span in spans {
        if !stages.contains(&span.stage) {
            stages.push(span.stage.clone());
        }
    }
    stages.sort();
    stages
        .into_iter()
        .map(|stage| {
            let mut values: Vec<u64> = spans
                .iter()
                .filter(|span| span.stage == stage)
                .map(|span| span.elapsed_us)
                .collect();
            values.sort_unstable();
            let count = values.len();
            let failed = spans
                .iter()
                .filter(|span| span.stage == stage && span.outcome != "ok")
                .count();
            let p50 = percentile(&values, 50);
            let p95 = percentile(&values, 95);
            let max = values.last().copied().unwrap_or(0);
            Distribution {
                stage,
                count,
                failed,
                p50_us: p50,
                p95_us: p95,
                max_us: max,
            }
        })
        .collect()
}

/// Nearest-rank percentile over a sorted, non-empty sample.
pub fn percentile(sorted: &[u64], pct: u64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (pct * sorted.len() as u64).div_ceil(100);
    let index = (rank.max(1) - 1) as usize;
    sorted[index.min(sorted.len() - 1)]
}

/// Monotonic stopwatch that reports elapsed microseconds.
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_us(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}
