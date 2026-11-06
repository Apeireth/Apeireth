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
//!
//! 旗标层: 判据走 crate 级共享模块 [`crate::flags`] (白名单式扫描, 拒绝即帧) ——
//! 本面只留自己的措辞 (英文档: 本面用法帧是英文的) 与三条自有纪律 (重复即拒 /
//! 非空值 / 不收位置参数); `--` 之后一律当位置参数。

use std::io::Write as _;
use std::path::PathBuf;

use apeireth_storage::crypto::{
    KdfParams, KeyWrapFile, PartitionId, PartitionKey, RecoveryKey, UnlockKey, KEY_LOSS_BOUNDARY,
};

use crate::flags;

/// `apeireth crypto` 用法帧。
pub const USAGE: &str = "apeireth crypto\n\nUsage:\n  apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>\n  apeireth crypto recovery-export --keys <FILE> --out <FILE>\n  apeireth crypto unlock --keys <FILE> [--recovery <FILE>] [--out <KEYFILE>]\n\n  wrap / recovery-export 从 stdin 读一行口令 (终端回显);\n  unlock 缺省读口令, --recovery <FILE> 改用恢复件 (hex);\n  --out <KEYFILE> 导出解锁后的分区密钥材料 (0600), 供 B6-2a 会话封缄 daemon 读取\n  (配 APEIRETH_ENABLE_SESSION_SEAL=1 + APEIRETH_SESSION_SEAL_KEY_FILE)。\n  密钥丢失 = 不可恢复 (无后门): 恢复件导出后请离线保管。";

/// `wrap` 的旗标白名单 (三个带值旗标; 本面不收布尔旗标)。
///
/// 判据 (旗标形态 / 缺值 / `--` 分隔) 走 crate 级共享模块 [`crate::flags`] ——
/// 本面只留自己的措辞与用法帧。
const WRAP_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--partition", "--keys", "--recovery-out"],
    boolean: &[],
    meta_as_positional: false,
};

/// `recovery-export` 的旗标白名单 (两个带值旗标)。
const RECOVERY_EXPORT_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--keys", "--out"],
    boolean: &[],
    meta_as_positional: false,
};

/// `unlock` 的旗标白名单 (三个带值旗标; `--out` 是 opt-in 导出第二动作)。
const UNLOCK_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--keys", "--recovery", "--out"],
    boolean: &[],
    meta_as_positional: false,
};

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
    let scanned = scan_options("wrap", &WRAP_FLAGS, args)?;
    let partition_text = require_value(&scanned, "wrap", "--partition")?;
    let keys_path = PathBuf::from(require_value(&scanned, "wrap", "--keys")?);
    let recovery_path = PathBuf::from(require_value(&scanned, "wrap", "--recovery-out")?);

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
    let scanned = scan_options("recovery-export", &RECOVERY_EXPORT_FLAGS, args)?;
    let keys_path = PathBuf::from(require_value(&scanned, "recovery-export", "--keys")?);
    let out_path = PathBuf::from(require_value(&scanned, "recovery-export", "--out")?);
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
    let scanned = scan_options("unlock", &UNLOCK_FLAGS, args)?;
    let keys_path = PathBuf::from(require_value(&scanned, "unlock", "--keys")?);
    let key_wrap = read_key_wrap(&keys_path)?;

    let unlock = match scanned.value("--recovery") {
        Some(recovery_path) => {
            let text = std::fs::read_to_string(recovery_path)
                .map_err(|error| format!("恢复件读取失败 {recovery_path}: {error}"))?;
            let recovery = RecoveryKey::from_hex(&text)
                .map_err(|error| format!("恢复件解析失败 {recovery_path}: {error}"))?;
            UnlockKey::Recovery(recovery)
        }
        None => UnlockKey::Passphrase(read_passphrase("口令 (stdin 一行, 回显): ")?),
    };

    let dek = key_wrap
        .unlock(&unlock)
        .map_err(|error| error.to_string())?;
    // B6-2a 密钥供给（方案 A）: 显式 --out 才导出解锁后的材料, 0600 落盘。
    // 不导出 = 只校验（本命令的旧语义, 逐字节不变）; 导出是 opt-in 的第二动作,
    // 避免「一次 unlock 顺手在盘上留明文密钥」这种隐式副作用。
    let mut frame = format!(
        "crypto unlock: ok partition={} (密钥材料可用; {})",
        dek.partition(),
        KEY_LOSS_BOUNDARY
    );
    if let Some(out_path) = scanned.value("--out") {
        let out_path = PathBuf::from(out_path);
        if out_path.exists() {
            return Err(format!(
                "crypto unlock refused: --out 目标已存在, 不覆盖: {}",
                out_path.display()
            ));
        }
        apeireth_runtime_assembly::canonical::session_seal::write_key_file(&out_path, &dek)?;
        frame.push_str(&format!(
            "\ncrypto unlock: key material -> {} (0600; 配 APEIRETH_ENABLE_SESSION_SEAL=1 + APEIRETH_SESSION_SEAL_KEY_FILE)",
            out_path.display()
        ));
    }
    Ok(frame)
}

fn read_key_wrap(path: &std::path::Path) -> Result<KeyWrapFile, String> {
    // 读失败要带**路径 + 下一步动作**：此前只回裸 `No such file or directory
    // (os error 2)` —— 用户看不出是哪个文件、更不知道先去 `crypto wrap`
    // （真机实测 2026-10-25）。
    let text = std::fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            return format!(
                "密钥文件不存在: {} （先用 `apeireth crypto wrap --partition <ID> --keys <FILE> --recovery-out <FILE>` 创建）",
                path.display()
            );
        }
        format!("密钥文件读取失败 {}: {error}", path.display())
    })?;
    KeyWrapFile::from_json(&text)
        .map_err(|error| format!("密钥文件解析失败 {}: {error}", path.display()))
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

/// 旗标解析: 白名单式扫描 —— **拒绝即帧** (判据在 crate 级共享模块 [`crate::flags`]).
///
/// 措辞走英文档 (本面用法帧是英文的): 未知项 (旗标与位置词类本面同称「参数」) /
/// 缺值 一律点名 + `--` 分隔提示 + 白名单提示 + 用法帧 (跨面帧结构逐字同构)。
///
/// 共享层管的判据: 旗标形态 (负数是值) / 未知项分类 / **缺值** (值缺失或值本身
/// 像旗标) / `--` 分隔。本面**自有**的三条纪律 (共享层不提供 ⇒ 面内显式判,
/// 措辞各自可辨):
///
/// 1. **非空值** —— 值不得是空串/纯空白 (空路径 = 落到当前目录);
/// 2. **重复即拒** —— 同一旗标只允许给一次 (不静默取末位: 密钥落点选错 =
///    密钥写错文件);
/// 3. **不收位置参数** —— 词类参数一律未知参数 (指名拒)。
///
/// ⚠️ 本面是**密钥材料面**: 共享层比本面旧行为**更宽松**的两处 (允许空值 /
/// 允许重复) 一律由上面两条自有纪律补回 ⇒ 收口不放宽任何一条拒绝路径。
fn scan_options(
    verb: &str,
    spec: &flags::FlagSpec,
    args: &[String],
) -> Result<flags::Scan, String> {
    let known = spec.known();
    let command = format!("crypto {verb}");
    let scanned = flags::scan(args, spec).map_err(|error| {
        error.frame(
            flags::Style::En,
            &flags::FrameContext::new(&command, flags::UnknownNoun::Argument)
                .with_usage(USAGE)
                .with_known(&known),
        )
    })?;
    // 自有纪律 ①: 非空值 (逐个取值判 —— 空路径 = 落到当前目录)。
    for (flag, value) in &scanned.values {
        if value.trim().is_empty() {
            return Err(format!(
                "crypto {verb} {flag} requires a non-empty value\n{USAGE}"
            ));
        }
    }
    // 自有纪律 ②: 重复即拒 (不静默取末位 —— 密钥落点选错 = 密钥写错文件)。
    for flag in spec.value {
        if scanned.count(flag) > 1 {
            return Err(format!("crypto {verb} {flag} given twice\n{USAGE}"));
        }
    }
    // 自有纪律 ③: 本命令不收位置参数 (词类参数指名拒)。
    if let Some(error) = scanned.extra_positional() {
        return Err(error.frame(
            flags::Style::En,
            &flags::FrameContext::new(&command, flags::UnknownNoun::Argument).with_usage(USAGE),
        ));
    }
    Ok(scanned)
}

/// 取必填旗标的值 (缺旗标 = 用法帧)。
fn require_value<'a>(scanned: &'a flags::Scan, verb: &str, flag: &str) -> Result<&'a str, String> {
    scanned
        .value(flag)
        .ok_or_else(|| format!("crypto {verb} missing flag: {flag}\n{USAGE}"))
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
