//! `apeireth crypto` 装配面 (B6 安全加固批 · 隐患 #6): 分区密钥包裹 + 恢复件导出 + 解锁校验。
//!
//! **最小装配接线** (报备口径): 本文件是 engine 层 `apeireth_storage::crypto`
//! 的人类出口 (设计第一律: 同内核包装出口, 不另起炉灶, 不两套逻辑) ——
//! 密钥层级语义全在 engine 面 (分区 DEK / AES-256-GCM 密封字段 / KEK 包裹 /
//! 备份恢复 / 密钥丢失边界), 本文件只做: 参数解析 + 口令/恢复件读入 + 文件落盘。
//!
//! 三动词:
//!
//! | 动词 | 语义 |
//! |---|---|
//! | `wrap` | 生成分区 DEK, 双槽包裹 (用户口令 KDF + 恢复件), 导出恢复件 |
//! | `recovery-export` | 用口令解包后换新恢复件 (丢了恢复件可补出) |
//! | `unlock` | 校验口令/恢复件可解包 (恢复前自检; 不落任何数据) |
//!
//! **0 装诚实**: 口令从 stdin 读一行 (终端**回显**, 未接无回显输入 —— 待办);
//! 平台凭据库/桌面侧车 keychain 锚定实现未接线 (engine 只有 `KekAnchor` trait);
//! 模型面工具 (`tool.*`) 未注册 (归口装配面治理接线); 本动词不写审计行
//! (审计归口治理面); live E2E 未实测。

use std::io::Write as _;
use std::path::PathBuf;

use apeireth_storage::crypto::{
    KdfParams, KeyWrapFile, PartitionId, PartitionKey, RecoveryKey, UnlockKey, KEY_LOSS_BOUNDARY,
};

/// `apeireth crypto` 用法帧。
pub const USAGE: &str = "apeireth crypto\n\nUsage:\n  apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>\n  apeireth crypto recovery-export --keys <FILE> --out <FILE>\n  apeireth crypto unlock --keys <FILE> [--recovery <FILE>]\n\n  wrap / recovery-export 从 stdin 读一行口令 (终端回显);\n  unlock 缺省读口令, --recovery <FILE> 改用恢复件 (hex)。\n  密钥丢失 = 不可恢复 (无后门): 恢复件导出后请离线保管。";

/// `apeireth crypto` 入口。
pub fn run_crypto(args: &[String]) -> Result<String, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Err(USAGE.to_string()),
    };
    match verb {
        "wrap" => run_wrap(rest),
        "recovery-export" => run_recovery_export(rest),
        "unlock" => run_unlock(rest),
        _ => Err(USAGE.to_string()),
    }
}

/// 生成分区 DEK + 双槽包裹 (口令 + 恢复件) + 恢复件导出。
fn run_wrap(args: &[String]) -> Result<String, String> {
    let options = parse_options(args, &["--partition", "--keys", "--recovery-out"])?;
    let partition_text = require_value(&options, "--partition")?;
    let keys_path = PathBuf::from(require_value(&options, "--keys")?);
    let recovery_path = PathBuf::from(require_value(&options, "--recovery-out")?);

    if keys_path.exists() {
        return Err(format!(
            "crypto wrap refused: 密钥文件已存在, 不覆盖 (换文件或先删): {}",
            keys_path.display()
        ));
    }
    if recovery_path.exists() {
        return Err(format!(
            "crypto wrap refused: 恢复件文件已存在, 不覆盖: {}",
            recovery_path.display()
        ));
    }

    let partition = PartitionId::new(partition_text).map_err(|error| error.to_string())?;
    let passphrase = read_passphrase("口令 (stdin 一行, 回显): ")?;
    if passphrase.trim().is_empty() {
        return Err("crypto wrap refused: 口令不得为空".to_string());
    }
    let kdf = KdfParams::with_random_salt_default().map_err(|error| error.to_string())?;
    let key = PartitionKey::generate(partition.clone());
    let recovery = RecoveryKey::generate();
    let key_wrap = KeyWrapFile::seal_new(
        &partition,
        &key,
        passphrase.as_bytes(),
        &recovery,
        kdf,
        now_millis(),
    )
    .map_err(|error| error.to_string())?;

    std::fs::write(
        &keys_path,
        key_wrap.to_json().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(&recovery_path, format!("{}\n", recovery.to_hex()))
        .map_err(|error| error.to_string())?;

    Ok(format!(
        "crypto wrap: ok partition={} keys={} recovery={} (恢复件只此一次导出, 离线保管; {})",
        partition,
        keys_path.display(),
        recovery_path.display(),
        KEY_LOSS_BOUNDARY
    ))
}

/// 用口令解包后换新恢复件 (丢失恢复件时的补出出口)。
fn run_recovery_export(args: &[String]) -> Result<String, String> {
    let options = parse_options(args, &["--keys", "--out"])?;
    let keys_path = PathBuf::from(require_value(&options, "--keys")?);
    let out_path = PathBuf::from(require_value(&options, "--out")?);
    if out_path.exists() {
        return Err(format!(
            "crypto recovery-export refused: 恢复件文件已存在, 不覆盖: {}",
            out_path.display()
        ));
    }

    let key_wrap = read_key_wrap(&keys_path)?;
    let passphrase = read_passphrase("口令 (stdin 一行, 回显): ")?;
    let dek = key_wrap
        .unlock(&UnlockKey::Passphrase(passphrase))
        .map_err(|error| error.to_string())?;

    let recovery = RecoveryKey::generate();
    let mut updated = key_wrap;
    updated
        .add_recovery_slot(&dek, &recovery)
        .map_err(|error| error.to_string())?;
    std::fs::write(
        &keys_path,
        updated.to_json().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(&out_path, format!("{}\n", recovery.to_hex()))
        .map_err(|error| error.to_string())?;

    Ok(format!(
        "crypto recovery-export: ok partition={} recovery={} (旧恢复件已作废; {})",
        updated.partition_id,
        out_path.display(),
        KEY_LOSS_BOUNDARY
    ))
}

/// 解锁自检 (口令或恢复件可解包; 不落任何数据)。
fn run_unlock(args: &[String]) -> Result<String, String> {
    let options = parse_options(args, &["--keys", "--recovery"])?;
    let keys_path = PathBuf::from(require_value(&options, "--keys")?);
    let key_wrap = read_key_wrap(&keys_path)?;

    let unlock = match options.get("--recovery") {
        Some(recovery_path) => {
            let text = std::fs::read_to_string(recovery_path).map_err(|error| error.to_string())?;
            let recovery = RecoveryKey::from_hex(&text).map_err(|error| error.to_string())?;
            UnlockKey::Recovery(recovery)
        }
        None => UnlockKey::Passphrase(read_passphrase("口令 (stdin 一行, 回显): ")?),
    };

    let dek = key_wrap
        .unlock(&unlock)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "crypto unlock: ok partition={} (密钥材料可用; {})",
        dek.partition(),
        KEY_LOSS_BOUNDARY
    ))
}

fn read_key_wrap(path: &std::path::Path) -> Result<KeyWrapFile, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    KeyWrapFile::from_json(&text).map_err(|error| error.to_string())
}

/// 从 stdin 读一行口令 (终端回显 —— 未接无回显输入, 见文件头 0 装诚实)。
fn read_passphrase(prompt: &str) -> Result<String, String> {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|error| format!("读口令失败: {error}"))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// 旗标解析: 只认白名单旗标 + 每旗标恰一值 (未知/重复/悬空 = 用法帧)。
fn parse_options(
    args: &[String],
    allowed: &[&str],
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut options = std::collections::BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if !allowed.contains(&flag) {
            return Err(format!("unknown crypto argument: {flag}\n{USAGE}"));
        }
        index += 1;
        let Some(value) = args.get(index) else {
            return Err(format!("crypto {flag} requires a value\n{USAGE}"));
        };
        if value.trim().is_empty() {
            return Err(format!("crypto {flag} requires a non-empty value\n{USAGE}"));
        }
        if options.insert(flag.to_string(), value.clone()).is_some() {
            return Err(format!("crypto {flag} given twice\n{USAGE}"));
        }
        index += 1;
    }
    Ok(options)
}

fn require_value<'a>(
    options: &'a std::collections::BTreeMap<String, String>,
    flag: &str,
) -> Result<&'a String, String> {
    options
        .get(flag)
        .ok_or_else(|| format!("crypto 缺旗标 {flag}\n{USAGE}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("apeireth-crypto-keys-{tag}-"))
            .tempdir()
            .expect("temp dir")
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn usage_frames_for_empty_unknown_and_dangling_flags() {
        assert!(run_crypto(&[]).unwrap_err().contains("Usage:"));
        assert!(run_crypto(&args(&["nope"])).unwrap_err().contains("Usage:"));
        assert!(run_crypto(&args(&["wrap", "--partition"]))
            .unwrap_err()
            .contains("requires a value"));
        assert!(run_crypto(&args(&["wrap", "--unknown", "x"]))
            .unwrap_err()
            .contains("unknown"));
        assert!(run_crypto(&args(&["wrap", "--keys", "a", "--keys", "b"]))
            .unwrap_err()
            .contains("twice"));
    }

    #[test]
    fn wrap_refuses_to_overwrite_existing_key_files() {
        let dir = temp_dir("overwrite");
        let keys = dir.path().join("keys.json");
        std::fs::write(&keys, "{}").expect("seed");
        let error = run_crypto(&args(&[
            "wrap",
            "--partition",
            "user-a",
            "--keys",
            keys.to_str().expect("utf8"),
            "--recovery-out",
            dir.path().join("recovery.txt").to_str().expect("utf8"),
        ]))
        .unwrap_err();
        assert!(error.contains("refused"), "覆盖必须拒: {error}");
    }

    /// 全链路 (wrap 文件形态 → engine 解包 → recovery-export 换新恢复件):
    /// 不经 stdin (口令读入面在 CLI 侧薄壳), 直接钉 engine 面的三文件语义。
    #[test]
    fn key_files_round_trip_through_the_engine_crypto_face() {
        let dir = temp_dir("roundtrip");
        let keys = dir.path().join("keys.json");
        let recovery_path = dir.path().join("recovery.txt");

        let partition = PartitionId::new("user-a").expect("id");
        let key = PartitionKey::generate(partition.clone());
        let recovery = RecoveryKey::generate();
        let kdf = KdfParams::pbkdf2_hmac_sha256(8, b"cli-test-salt").expect("params");
        let key_wrap =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, kdf, 0).expect("wrap");
        std::fs::write(&keys, key_wrap.to_json().expect("json")).expect("write keys");
        std::fs::write(&recovery_path, format!("{}\n", recovery.to_hex())).expect("write recovery");

        let read_back = read_key_wrap(&keys).expect("read");
        let unlocked = read_back
            .unlock(&UnlockKey::Passphrase("pw".to_string()))
            .expect("unlock");
        assert_eq!(unlocked.expose_dek(), key.expose_dek());

        let recovery_text = std::fs::read_to_string(&recovery_path).expect("read");
        let recovery_back = RecoveryKey::from_hex(&recovery_text).expect("hex");
        assert_eq!(recovery_back.expose(), recovery.expose());
    }
}
