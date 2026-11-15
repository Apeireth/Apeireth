//! Sealing, freezing and revealing.
//!
//! The evaluation is split into three stages so that the oracle does not live
//! in the same process as the run:
//!
//! 1. **seal** — the input bundle and the oracle bundle are hashed and the
//!    seal material is written to a custody directory with write-once
//!    semantics. The seal records that the shard was **not** authored
//!    independently of the implementation (`authored_independently: false`).
//! 2. **run** — the corpus is written, the questions are asked and the
//!    answers are frozen. This stage never opens the oracle file: it only
//!    carries the digests of the seal.
//! 3. **reveal** — the oracle is read once, the answers are graded, and the
//!    reveal is appended to an append-only ledger. A second reveal of the same
//!    seal is refused.
//!
//! ⭐ 口径（本轮实测边界，不是设计目标）：
//! * 独立保管（由另一方持有密封材料）**未实现**：密封与揭封由同一操作者在
//!   同一台机器上完成，只有文件级分离 ⇒ 只能称「同账号隔离」，不构成强保密。
//! * 题集是**回归集**，不是独立盲测：题与答案由同一实现者编写，
//!   `prior_implementation_exposure` 为真。
//! * 可核验性是**已实现**的部分：第三方可用 `verify` 子命令重算整条绑定链
//!   （输入 → 密封 → 冻结 → 构建回执 → 可执行文件 → 评估源 → 配置）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::artifacts::{file_hash, hash, hash_bytes, read, write_new, Result};
use super::schema::{InputBundle, OracleBundle, FREEZE_PROTOCOL, SEAL_PROTOCOL};

/// Protocol of the build receipt.
pub const BUILD_PROTOCOL: &str = "apeireth.memory-longterm-build.v1";

/// Protocol of the reveal ledger.
pub const LEDGER_PROTOCOL: &str = "apeireth.memory-longterm-reveal.v1";

/// The seal of one shard.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Seal {
    pub protocol: String,
    /// The digest of the seal body: the verifiable identity of this shard.
    pub seal_sha256: String,
    pub corpus_id: String,
    pub input_sha256: String,
    pub oracle_sha256: String,
    pub expected_cases: usize,
    /// Whether the shard was authored without exposure to the implementation.
    pub authored_independently: bool,
    /// Whether the implementation was already exposed to the shard.
    pub prior_implementation_exposure: bool,
    /// Where the sealed material lives (same account, same host).
    pub custody_dir: String,
    pub sealed_at_utc: String,
}

/// The receipt that binds the executable to the sources that produced it.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildReceipt {
    pub protocol: String,
    pub product_head: String,
    pub product_tree: String,
    pub cargo_lock_sha256: String,
    pub executable_sha256: String,
    pub evaluator_sources: BTreeMap<String, String>,
    pub build_command: String,
    pub toolchain: String,
    pub target: String,
    pub features: String,
    pub source_binding_receipt_sha256: String,
}

/// The freeze of one shard: what the run is bound to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Freeze {
    pub protocol: String,
    pub seal_sha256: String,
    pub build_receipt_sha256: String,
    pub config_sha256: String,
    pub scoring_protocol: String,
    pub reader_system_sha256: String,
    pub repetitions: usize,
    pub expected_cases: usize,
    pub first_exposure_utc: String,
    pub tuning_on_this_shard_forbidden: bool,
    /// The custody mode that is actually in force.
    pub custody_mode: String,
}

/// One entry of the append-only reveal ledger.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevealEntry {
    pub protocol: String,
    pub seal_sha256: String,
    pub revealed_at_utc: String,
    pub run_dir: String,
    pub oracle_sha256: String,
}

/// The evaluator sources, bound at compile time.
fn evaluator_sources() -> BTreeMap<String, &'static [u8]> {
    let mut sources = BTreeMap::new();
    sources.insert(
        "examples/memory_longterm_eval.rs".to_string(),
        include_bytes!("../memory_longterm_eval.rs") as &[u8],
    );
    sources.insert(
        "examples/memory_longterm_eval/mod.rs".to_string(),
        include_bytes!("mod.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/artifacts.rs".to_string(),
        include_bytes!("artifacts.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/config.rs".to_string(),
        include_bytes!("config.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/corpus.rs".to_string(),
        include_bytes!("corpus.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/evidence.rs".to_string(),
        include_bytes!("evidence.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/grading.rs".to_string(),
        include_bytes!("grading.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/live.rs".to_string(),
        include_bytes!("live.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/tests.rs".to_string(),
        include_bytes!("tests.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/harness.rs".to_string(),
        include_bytes!("harness.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/pressure.rs".to_string(),
        include_bytes!("pressure.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/reader.rs".to_string(),
        include_bytes!("reader.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/schema.rs".to_string(),
        include_bytes!("schema.rs"),
    );
    sources.insert(
        "examples/memory_longterm_eval/seal.rs".to_string(),
        include_bytes!("seal.rs"),
    );
    sources
}

fn now_utc() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// The canonical digest of a JSON artifact: a third party can recompute it
/// from the file with the same serializer.
fn digest_of<T: Serialize>(value: &T) -> String {
    hash(&serde_json::to_string(value).unwrap_or_default())
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The workspace root, found by walking up from the current directory.
pub fn workspace_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if text.contains("[workspace]") {
                    return Some(current);
                }
            }
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Seal one shard into the custody directory.
pub fn create_seal(input: &InputBundle, oracle: &OracleBundle, custody: &Path) -> Result<Seal> {
    std::fs::create_dir_all(custody)?;
    let sealed_at_utc = now_utc();
    let custody_dir = custody.to_string_lossy().to_string();
    let input_sha256 = super::corpus::bundle_digest(input);
    let oracle_sha256 = digest_of(oracle);
    let expected_cases = oracle.cases.len();
    let body = serde_json::json!({
        "protocol": SEAL_PROTOCOL,
        "corpus_id": input.corpus_id,
        "input_sha256": input_sha256,
        "oracle_sha256": oracle_sha256,
        "expected_cases": expected_cases,
        "authored_independently": false,
        "prior_implementation_exposure": true,
        "custody_dir": custody_dir,
        "sealed_at_utc": sealed_at_utc,
    });
    let seal = Seal {
        protocol: SEAL_PROTOCOL.to_string(),
        seal_sha256: digest_of(&body),
        corpus_id: input.corpus_id.clone(),
        input_sha256,
        oracle_sha256,
        expected_cases,
        authored_independently: false,
        prior_implementation_exposure: true,
        custody_dir,
        sealed_at_utc,
    };
    write_new(&custody.join("seal.json"), &seal)?;
    Ok(seal)
}

/// Build the receipt that binds the running executable to its sources.
pub fn create_build_receipt(
    build_command: &str,
    features: &str,
    executable: &[u8],
) -> Result<BuildReceipt> {
    let sources: BTreeMap<String, String> = evaluator_sources()
        .into_iter()
        .map(|(name, bytes)| (name, hash_bytes(bytes)))
        .collect();
    let binding = serde_json::to_string(&sources)?;
    let cargo_lock = workspace_root()
        .map(|root| root.join("Cargo.lock"))
        .filter(|path| path.is_file())
        .and_then(|path| file_hash(&path).ok())
        .unwrap_or_else(|| "unavailable".to_string());
    let toolchain = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    Ok(BuildReceipt {
        protocol: BUILD_PROTOCOL.to_string(),
        product_head: git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unavailable".into()),
        product_tree: git_output(&["rev-parse", "HEAD^{tree}"])
            .unwrap_or_else(|| "unavailable".into()),
        cargo_lock_sha256: cargo_lock,
        executable_sha256: hash_bytes(executable),
        evaluator_sources: sources,
        build_command: build_command.to_string(),
        toolchain,
        target: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        features: features.to_string(),
        source_binding_receipt_sha256: hash(&binding),
    })
}

/// Freeze the shard: bind the seal, the build receipt and the configuration.
pub fn create_freeze(
    seal: &Seal,
    receipt: &BuildReceipt,
    config_sha256: &str,
    repetitions: usize,
    reader_system_sha256: &str,
) -> Freeze {
    Freeze {
        protocol: FREEZE_PROTOCOL.to_string(),
        seal_sha256: seal.seal_sha256.clone(),
        build_receipt_sha256: digest_of(receipt),
        config_sha256: config_sha256.to_string(),
        scoring_protocol: super::schema::SCORING_PROTOCOL.to_string(),
        reader_system_sha256: reader_system_sha256.to_string(),
        repetitions,
        expected_cases: seal.expected_cases,
        first_exposure_utc: now_utc(),
        tuning_on_this_shard_forbidden: true,
        custody_mode: "same_account_file_separation".to_string(),
    }
}

/// Verify the whole binding chain. A failure means no model request may be
/// sent.
pub fn verify_chain(
    input: &InputBundle,
    seal: &Seal,
    freeze: &Freeze,
    receipt: &BuildReceipt,
    config_sha256: &str,
    executable: &[u8],
) -> Result<()> {
    let problems: Vec<String> = vec![
        if seal.protocol != SEAL_PROTOCOL {
            format!("seal protocol is {}", seal.protocol)
        } else {
            String::new()
        },
        if seal.corpus_id != input.corpus_id {
            format!(
                "seal corpus is {} but the input is {}",
                seal.corpus_id, input.corpus_id
            )
        } else {
            String::new()
        },
        if seal.input_sha256 != super::corpus::bundle_digest(input) {
            "the seal does not bind this input bundle".to_string()
        } else {
            String::new()
        },
        if freeze.protocol != FREEZE_PROTOCOL {
            format!("freeze protocol is {}", freeze.protocol)
        } else {
            String::new()
        },
        if freeze.seal_sha256 != seal.seal_sha256 {
            // The freeze binds the seal's own identity (the digest of its
            // body), not a digest of the seal file: the body digest is what
            // `create_seal` published and what the ledger keys on.
            "the freeze does not bind this seal".to_string()
        } else {
            String::new()
        },
        if freeze.build_receipt_sha256 != digest_of(receipt) {
            "the freeze does not bind this build receipt".to_string()
        } else {
            String::new()
        },
        if freeze.config_sha256 != config_sha256 {
            "the freeze does not bind this configuration".to_string()
        } else {
            String::new()
        },
        if receipt.executable_sha256 != hash_bytes(executable) {
            "the build receipt does not bind this executable".to_string()
        } else {
            String::new()
        },
        if receipt.source_binding_receipt_sha256 != digest_of(&receipt.evaluator_sources) {
            "the build receipt does not bind its own source list".to_string()
        } else {
            String::new()
        },
        if receipt.evaluator_sources != evaluator_source_digests() {
            "the evaluator sources changed after the build".to_string()
        } else {
            String::new()
        },
    ]
    .into_iter()
    .filter(|problem| !problem.is_empty())
    .collect();
    if problems.is_empty() {
        return Ok(());
    }
    Err(format!(
        "seal/freeze/build/config binding failed; no model request sent: {}",
        problems.join("; ")
    )
    .into())
}

/// The digest of every evaluator source, as compiled in.
pub fn evaluator_source_digests() -> BTreeMap<String, String> {
    evaluator_sources()
        .into_iter()
        .map(|(name, bytes)| (name, hash_bytes(bytes)))
        .collect()
}

/// The append-only reveal ledger of one custody directory.
pub fn ledger_path(custody: &Path) -> PathBuf {
    custody.join("reveal-ledger.jsonl")
}

/// Claim the one-time reveal of a shard.
///
/// ⭐ One-time use: the second reveal of the same seal is refused. The ledger
/// is append-only and the first write is write-once.
pub fn claim_reveal(
    custody: &Path,
    seal_sha256: &str,
    run_dir: &str,
    oracle_sha256: &str,
) -> Result<RevealEntry> {
    std::fs::create_dir_all(custody)?;
    let path = ledger_path(custody);
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    for line in existing.lines().filter(|line| !line.trim().is_empty()) {
        if let Ok(entry) = serde_json::from_str::<RevealEntry>(line) {
            if entry.seal_sha256 == seal_sha256 {
                return Err(format!(
                    "one-time use: the shard {seal_sha256} was already revealed at {} (run {})",
                    entry.revealed_at_utc, entry.run_dir
                )
                .into());
            }
        }
    }
    let entry = RevealEntry {
        protocol: LEDGER_PROTOCOL.to_string(),
        seal_sha256: seal_sha256.to_string(),
        revealed_at_utc: now_utc(),
        run_dir: run_dir.to_string(),
        oracle_sha256: oracle_sha256.to_string(),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    file.sync_all()?;
    Ok(entry)
}

/// The reveal entries of one custody directory.
pub fn reveal_entries(custody: &Path) -> Vec<RevealEntry> {
    let path = ledger_path(custody);
    std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<RevealEntry>(line).ok())
        .collect()
}

/// Read the seal of a custody directory.
pub fn read_seal(custody: &Path) -> Result<Seal> {
    read(&custody.join("seal.json"))
}

/// Read the build receipt of a custody directory.
pub fn read_build_receipt(custody: &Path) -> Result<BuildReceipt> {
    read(&custody.join("build-receipt.json"))
}

/// Read the freeze of a custody directory.
pub fn read_freeze(custody: &Path) -> Result<Freeze> {
    read(&custody.join("freeze.json"))
}

/// The digest of the seal body, recomputed from the seal itself.
///
/// ⭐ The seal publishes the digest of its body, so anybody holding the seal
/// file can recompute it and check that the file was not edited.
pub fn seal_body_digest(seal: &Seal) -> String {
    let body = serde_json::json!({
        "protocol": seal.protocol,
        "corpus_id": seal.corpus_id,
        "input_sha256": seal.input_sha256,
        "oracle_sha256": seal.oracle_sha256,
        "expected_cases": seal.expected_cases,
        "authored_independently": seal.authored_independently,
        "prior_implementation_exposure": seal.prior_implementation_exposure,
        "custody_dir": seal.custody_dir,
        "sealed_at_utc": seal.sealed_at_utc,
    });
    digest_of(&body)
}

/// Verify a reveal **without any internal state of the sealing side**.
///
/// ⭐ Only the files in the custody directory are needed: the seal file (its
/// body digest is recomputed here) and the append-only ledger. The check is:
/// the seal file is self-consistent, the ledger holds exactly one entry for
/// that seal (one-time use), the entry carries the ledger protocol, and the
/// entry names the run directory the shard was released into.
pub fn verify_reveal(custody: &Path, expected_run_dir: Option<&str>) -> Result<RevealEntry> {
    let seal = read_seal(custody)?;
    let recomputed = seal_body_digest(&seal);
    if recomputed != seal.seal_sha256 {
        return Err(format!(
            "the seal file does not match its own body digest ({} vs {})",
            seal.seal_sha256, recomputed
        )
        .into());
    }
    let entries = reveal_entries(custody);
    let matching: Vec<&RevealEntry> = entries
        .iter()
        .filter(|entry| entry.seal_sha256 == seal.seal_sha256)
        .collect();
    if matching.len() != 1 {
        return Err(format!(
            "the ledger holds {} entries for the shard {} (one-time use expects exactly one)",
            matching.len(),
            seal.seal_sha256
        )
        .into());
    }
    let entry = matching[0];
    if entry.protocol != LEDGER_PROTOCOL {
        return Err(format!("the ledger entry protocol is {}", entry.protocol).into());
    }
    if let Some(expected) = expected_run_dir {
        if entry.run_dir != expected {
            return Err(format!(
                "the ledger entry names the run {} but this run is {}",
                entry.run_dir, expected
            )
            .into());
        }
    }
    Ok(entry.clone())
}
