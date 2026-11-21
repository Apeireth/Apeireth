//! B6-2a 试点：`sessions.data` 列级封缄的装配面（默认关，fail-closed）。
//!
//! 设计见 `docs/04-internal/backend-enhancement/b6-2a-sealed-session-design-2026-10-22.md`。
//! 本模块只做**开关与密钥供给**，加密语义全部来自 `apeireth_storage::crypto`
//! （分区 DEK + AES-256-GCM + AAD 绑分区），这里不另起炉灶、不复制密码学代码。
//!
//! # 两个环境变量
//!
//! * [`SESSION_SEAL_ENV`] —— `"1"` = 开（只认字面 `"1"`，与既有 bio 开关同纪律）；
//! * [`SESSION_SEAL_KEY_FILE_ENV`] —— 解锁后的密钥材料文件路径（0600，
//!   由 `apeireth crypto unlock --out` 产出）。
//!
//! 开关关 = 不读密钥文件、不构造封缄层，落盘逐字节同旧路径。
//! 开关开 + 密钥文件缺失/坏 = **启动失败**（绝不降级明文）。
//!
//! # 密钥丢失 = 会话永久不可解
//!
//! [`KEY_LOSS_BOUNDARY`]：没有后门、没有重置。`keys.json` 与恢复件必须离线保管，
//! 备份集必须同时备份密钥材料 —— 只备份库文件等于制造一堆永远解不开的密文。

use std::path::{Path, PathBuf};

use apeireth_storage::crypto::{PartitionId, PartitionKey, SealedField};

/// 会话封缄开关（默认关）。
pub const SESSION_SEAL_ENV: &str = "APEIRETH_ENABLE_SESSION_SEAL";
/// 解锁后的密钥材料文件（0600）。
pub const SESSION_SEAL_KEY_FILE_ENV: &str = "APEIRETH_SESSION_SEAL_KEY_FILE";

/// 密钥丢失边界（对外文档与 CLI 帧共用同一句）。
pub const KEY_LOSS_BOUNDARY: &str = "密钥丢失 = 不可恢复（无后门）";

/// 密钥文件格式标识。
const KEY_FILE_FORMAT: &str = "apeireth-session-seal-key/v1";
/// 行绑定密文信封；其后的 hex 仍是 storage `SealedField`。
const ROW_SEAL_PREFIX: &str = "APSR2:";
const ROW_PARTITION_DOMAIN: &str = "apeireth.session.row.v2";

fn switch_on(key: &str) -> bool {
    enabled_value(std::env::var(key).ok().as_deref())
}

/// 开关取值判定（纯函数，可脱离 env 测）：只认字面 `"1"`（前后空白容忍）。
fn enabled_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// `APEIRETH_ENABLE_SESSION_SEAL` 是否为显式开启。
pub fn seal_enabled_from_env() -> bool {
    switch_on(SESSION_SEAL_ENV)
}

/// 按环境构造封缄器：开关关 = `None`（零变化）；开关开 = 读密钥文件，
/// 缺失或坏 = `Err`（调用方必须让启动失败，不许降级明文）。
pub fn sealer_from_env() -> Result<Option<SessionSealer>, String> {
    let path = std::env::var(SESSION_SEAL_KEY_FILE_ENV).ok();
    resolve_sealer(seal_enabled_from_env(), path.as_deref())
}

/// [`sealer_from_env`] 的纯函数内核（开关值与路径由调用方给，便于脱 env 测试）。
fn resolve_sealer(enabled: bool, key_path: Option<&str>) -> Result<Option<SessionSealer>, String> {
    if !enabled {
        return Ok(None);
    }
    let path = key_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!("{SESSION_SEAL_ENV}=1 但未设 {SESSION_SEAL_KEY_FILE_ENV}（无密钥不得降级明文）")
        })?;
    Ok(Some(SessionSealer::new(load_key_file(Path::new(path))?)))
}

/// 读密钥文件（JSON，0600）。格式坏/被截断 = 显式错误。
pub fn load_key_file(path: &Path) -> Result<PartitionKey, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("会话封缄密钥文件读取失败 {}: {error}", path.display()))?;
    let material: KeyFile = serde_json::from_str(&text)
        .map_err(|error| format!("会话封缄密钥文件解析失败 {}: {error}", path.display()))?;
    if material.format != KEY_FILE_FORMAT {
        return Err(format!(
            "会话封缄密钥文件格式不符 {}: 期望 {KEY_FILE_FORMAT}, 实际 {}",
            path.display(),
            material.format
        ));
    }
    let dek = decode_hex32(&material.dek_hex)
        .map_err(|error| format!("会话封缄密钥文件 dek_hex 非法 {}: {error}", path.display()))?;
    let partition = PartitionId::new(material.partition.clone())
        .map_err(|error| format!("会话封缄密钥文件 partition 非法: {error}"))?;
    Ok(PartitionKey::from_bytes(partition, dek))
}

/// 独占写密钥文件（0600，仅 Unix 设权限位；其他平台由调用方保证）。
///
/// 目标已存在时拒绝，避免调用方的先检查后写竞态覆盖另一把可用密钥。
pub fn write_key_file(path: &Path, key: &PartitionKey) -> Result<(), String> {
    let material = KeyFile {
        format: KEY_FILE_FORMAT.to_string(),
        partition: key.partition().as_str().to_string(),
        dek_hex: encode_hex(key.expose_dek()),
    };
    let json = serde_json::to_string_pretty(&material)
        .map_err(|error| format!("会话封缄密钥文件序列化失败: {error}"))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("会话封缄密钥文件写入失败 {}: {error}", path.display()))?;
    use std::io::Write as _;
    if let Err(error) = file
        .write_all(json.as_bytes())
        .and_then(|()| file.sync_all())
    {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(format!(
            "会话封缄密钥文件写入失败 {}: {error}",
            path.display()
        ));
    }
    drop(file);
    if let Err(error) = sync_key_parent(path) {
        let _ = std::fs::remove_file(path);
        return Err(format!(
            "会话封缄密钥文件目录项落盘失败 {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn sync_key_parent(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(
        path.parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()
}

#[cfg(not(unix))]
fn sync_key_parent(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KeyFile {
    format: String,
    partition: String,
    dek_hex: String,
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn decode_hex32(text: &str) -> Result<[u8; 32], String> {
    let text = text.trim();
    if text.len() != 64 {
        return Err(format!("长度应为 64 个 hex 字符, 实际 {}", text.len()));
    }
    let mut out = [0u8; 32];
    for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| "含非 ASCII 字节".to_string())?;
        out[index] = u8::from_str_radix(pair, 16).map_err(|_| format!("非法 hex: {pair}"))?;
    }
    Ok(out)
}

/// `sessions.data` 的封缄器（绑定一个分区 DEK）。
///
/// 只做两件事：写入时封缄成 hex 密文、读取时按形态判明文/密文。
/// 形态判定是**过渡期兼容**（B6-2a 之前落盘的明文行必须照读），
/// 不是"猜编码"：密文行只有一种合法形态（hex 的 `SealedField`），
/// 解不开就是硬错误，绝不退回明文。
#[derive(Clone)]
pub struct SessionSealer {
    key: PartitionKey,
}

impl SessionSealer {
    /// 绑定分区 DEK。
    pub fn new(key: PartitionKey) -> Self {
        Self { key }
    }

    /// 绑定分区名。
    pub fn partition(&self) -> &str {
        self.key.partition().as_str()
    }

    fn row_key(&self, row_ref: &str) -> Result<PartitionKey, String> {
        if row_ref.is_empty() {
            return Err("会话封缄行引用不得为空".to_string());
        }
        let partition = PartitionId::new(format!(
            "{}|{ROW_PARTITION_DOMAIN}|{}:{row_ref}",
            self.partition(),
            row_ref.len()
        ))
        .map_err(|error| format!("会话封缄行引用非法: {error}"))?;
        Ok(PartitionKey::from_bytes(partition, *self.key.expose_dek()))
    }

    /// 明文 → 落盘形态（hex 编码的 `SealedField`，与 `SqliteFieldSink` 产物同格式）。
    pub fn seal_to_text(&self, plaintext: &[u8]) -> Result<String, String> {
        self.key
            .seal(plaintext)
            .map(|sealed| sealed.to_hex())
            .map_err(|error| format!("会话封缄失败: {error}"))
    }

    /// 落盘形态 → 明文。明文旧行（非密文形态）= 原样返回（过渡兼容）；
    /// 是密文形态但解不开 = 硬错误（篡改/换钥/换分区）。
    pub fn open_text(&self, stored: &str) -> Result<Vec<u8>, String> {
        if stored.starts_with(ROW_SEAL_PREFIX) {
            return Err("会话解封失败：行绑定密文缺少行引用".to_string());
        }
        match SealedField::from_hex(stored) {
            Ok(sealed) => self
                .key
                .open(&sealed)
                .map_err(|error| format!("会话解封失败（密文被篡改或密钥不符）: {error}")),
            Err(_) => Ok(stored.as_bytes().to_vec()),
        }
    }

    /// 该落盘值是否已是密文形态（迁移工具与测试共用，避免各自字节判定）。
    pub fn is_sealed(stored: &str) -> bool {
        Self::is_row_sealed(stored) || SealedField::from_hex(stored).is_ok()
    }

    /// 明文 → 与稳定行引用绑定的 v2 落盘形态。
    pub fn seal_row_to_text(&self, row_ref: &str, plaintext: &[u8]) -> Result<String, String> {
        self.row_key(row_ref)?
            .seal(plaintext)
            .map(|sealed| format!("{ROW_SEAL_PREFIX}{}", sealed.to_hex()))
            .map_err(|error| format!("会话封缄失败: {error}"))
    }

    /// 按行引用开封 v2；旧的分区级 v1 密文和明文仍兼容读取。
    pub fn open_row_text(&self, row_ref: &str, stored: &str) -> Result<Vec<u8>, String> {
        let Some(hex) = stored.strip_prefix(ROW_SEAL_PREFIX) else {
            return self.open_text(stored);
        };
        let sealed = SealedField::from_hex(hex)
            .map_err(|error| format!("会话解封失败（行绑定密文形态非法）: {error}"))?;
        self.row_key(row_ref)?
            .open(&sealed)
            .map_err(|error| format!("会话解封失败（行引用、密文或密钥不符）: {error}"))
    }

    /// 已完成 v2 迁移的在线读路径：明文/v1 一律视为降级，不再走迁移兼容。
    pub fn open_current_row_text(&self, row_ref: &str, stored: &str) -> Result<Vec<u8>, String> {
        if !Self::is_row_sealed(stored) {
            return Err("会话解封失败：受保护在线读遇到非 v2 行".to_string());
        }
        self.open_row_text(row_ref, stored)
    }

    /// 是否带行绑定 v2 信封；即使内部 hex 损坏也判作密文并在开封时硬失败。
    pub fn is_row_sealed(stored: &str) -> bool {
        stored.starts_with(ROW_SEAL_PREFIX)
    }
}

impl std::fmt::Debug for SessionSealer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionSealer")
            .field("partition", &self.partition())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-session-seal-{name}-{}",
            std::process::id()
        ))
    }

    fn key() -> PartitionKey {
        PartitionKey::from_bytes(PartitionId::new("sessions").expect("partition"), [7u8; 32])
    }

    /// 开关只认字面 `"1"`；其余取值一律不算开（fail-closed 的第一道）。
    #[test]
    fn switch_accepts_only_an_explicit_one() {
        for value in [Some("1"), Some(" 1 "), Some("1\n")] {
            assert!(enabled_value(value), "{value:?} 应判开启");
        }
        for value in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("on"),
            Some("11"),
        ] {
            assert!(!enabled_value(value), "{value:?} 不得判开启");
        }
    }

    /// 开关开但没有/坏密钥文件 = 硬错误（绝不降级明文）。
    #[test]
    fn resolve_sealer_fails_closed_without_usable_key() {
        assert!(resolve_sealer(false, None).expect("关 = 零变化").is_none());

        let error = resolve_sealer(true, None).expect_err("开但没路径必须报错");
        assert!(error.contains(SESSION_SEAL_KEY_FILE_ENV), "{error}");
        assert!(
            resolve_sealer(true, Some("   ")).is_err(),
            "空白路径等同未设"
        );

        let missing = scratch("resolve-missing.json");
        assert!(
            resolve_sealer(true, Some(missing.to_str().unwrap())).is_err(),
            "缺密钥文件必须让启动失败"
        );

        let path = scratch("resolve-ok.json");
        write_key_file(&path, &key()).expect("write");
        let sealer = resolve_sealer(true, Some(path.to_str().unwrap()))
            .expect("合法密钥文件")
            .expect("Some");
        assert_eq!(sealer.partition(), "sessions");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn key_file_round_trips_at_0600() {
        let path = scratch("roundtrip.json");
        write_key_file(&path, &key()).expect("write key file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "密钥文件必须是 0600");
        }
        let loaded = load_key_file(&path).expect("load key file");
        assert_eq!(loaded.partition().as_str(), "sessions");
        assert_eq!(loaded.expose_dek(), key().expose_dek());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn key_file_write_never_overwrites_an_existing_key() {
        let path = scratch("no-overwrite.json");
        write_key_file(&path, &key()).expect("first write");
        let original = std::fs::read(&path).expect("original");
        let replacement =
            PartitionKey::from_bytes(PartitionId::new("sessions").expect("partition"), [8u8; 32]);
        assert!(
            write_key_file(&path, &replacement).is_err(),
            "已存在的有效密钥不得被覆盖"
        );
        assert_eq!(std::fs::read(&path).expect("after"), original);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn key_file_rejects_wrong_format_and_bad_dek() {
        let path = scratch("bad.json");
        std::fs::write(
            &path,
            r#"{"format":"nope","partition":"sessions","dek_hex":"00"}"#,
        )
        .expect("write");
        let error = load_key_file(&path).expect_err("格式标识不符必须拒绝");
        assert!(error.contains("格式不符"), "{error}");

        std::fs::write(
            &path,
            r#"{"format":"apeireth-session-seal-key/v1","partition":"sessions","dek_hex":"zz"}"#,
        )
        .expect("write");
        let error = load_key_file(&path).expect_err("坏 dek 必须拒绝");
        assert!(error.contains("dek_hex 非法"), "{error}");

        let missing = scratch("missing.json");
        assert!(load_key_file(&missing).is_err(), "缺文件必须报错");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn seal_round_trip_and_plaintext_compatibility() {
        let sealer = SessionSealer::new(key());
        let plaintext = br#"{"id":"s1","messages":[]}"#;
        let sealed = sealer.seal_to_text(plaintext).expect("seal");
        assert!(SessionSealer::is_sealed(&sealed), "落盘形态应为密文");
        assert!(!sealed.contains("messages"), "密文不得含明文子串");
        assert_eq!(sealer.open_text(&sealed).expect("open"), plaintext.to_vec());

        // 过渡兼容: 明文旧行原样读出。
        let legacy = r#"{"id":"s0"}"#;
        assert_eq!(
            sealer.open_text(legacy).expect("legacy"),
            legacy.as_bytes().to_vec()
        );
        assert!(!SessionSealer::is_sealed(legacy));
    }

    #[test]
    fn row_bound_ciphertext_round_trips_and_keeps_v1_compatibility() {
        let sealer = SessionSealer::new(key());
        let plaintext = br#"{"id":"s1"}"#;
        let v2 = sealer
            .seal_row_to_text("sessions:s1", plaintext)
            .expect("seal row");
        assert!(SessionSealer::is_row_sealed(&v2));
        assert!(SessionSealer::is_sealed(&v2));
        assert_eq!(
            sealer.open_row_text("sessions:s1", &v2).expect("open v2"),
            plaintext
        );
        assert!(sealer.open_text(&v2).is_err(), "v2 不得脱离行引用解封");
        assert!(sealer
            .open_current_row_text("sessions:s1", "plaintext")
            .is_err());

        let v1 = sealer.seal_to_text(plaintext).expect("seal legacy v1");
        assert_eq!(
            sealer
                .open_row_text("sessions:s1", &v1)
                .expect("open legacy v1"),
            plaintext
        );
    }

    #[test]
    fn row_bound_ciphertext_cannot_move_between_rows() {
        let sealer = SessionSealer::new(key());
        let sealed = sealer
            .seal_row_to_text("sessions:s1", b"payload")
            .expect("seal");
        assert!(
            sealer.open_row_text("sessions:s2", &sealed).is_err(),
            "同钥分区内换行也必须拒解"
        );
    }

    #[test]
    fn tampered_ciphertext_is_refused() {
        let sealer = SessionSealer::new(key());
        let mut sealed = sealer.seal_to_text(b"payload").expect("seal");
        // 翻最后一位 (密文尾), 形态仍合法但 AEAD 校验必失败。
        let last = sealed.pop().expect("non-empty");
        let flipped = if last == '0' { '1' } else { '0' };
        sealed.push(flipped);
        let error = sealer.open_text(&sealed).expect_err("篡改必须拒读");
        assert!(error.contains("解封失败"), "{error}");
    }

    #[test]
    fn foreign_partition_key_cannot_open() {
        let sealer = SessionSealer::new(key());
        let sealed = sealer.seal_to_text(b"payload").expect("seal");
        let other = SessionSealer::new(PartitionKey::from_bytes(
            PartitionId::new("memory").expect("partition"),
            [9u8; 32],
        ));
        assert!(other.open_text(&sealed).is_err(), "换分区/换钥必须拒解");
    }
}
