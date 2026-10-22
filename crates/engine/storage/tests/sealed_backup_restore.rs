//! B6 备份/恢复/密钥丢失流程测试 (T1 备份三件套: 备份 = 数据 + DEK 包裹件).
//!
//! 钉死面:
//! - **备份恢复往返**: 导出 → 校验 → 恢复, 逐件逐字节一致 (口令与恢复件两条恢复路径);
//! - **KEK 不随备份走**: bundle 里零 KEK 材料 (口令/恢复件/派生 KEK 全不见);
//! - **校验信封完整性再落盘**: 篡改/版本不符/路径穿越 = 拒恢复且**零字节落盘**;
//! - **密钥丢失 = 显式不可恢复**: 无可用解锁材料 = `KeyMaterialLost` 边界帧, 不假装能找回。

use std::path::{Path, PathBuf};

use apeireth_storage::crypto::{
    export_sealed_backup, restore_sealed_backup, verify_sealed_backup, BackupEntry, CryptoError,
    KdfParams, KeyWrapFile, PartitionId, PartitionKey, RecoveryKey, UnlockKey, KEY_LOSS_BOUNDARY,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-sealed-backup-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn fast_kdf() -> KdfParams {
    KdfParams::pbkdf2_hmac_sha256(8, b"backup-round-trip-salt").expect("params")
}

struct Fixture {
    _root: PathBuf,
    bundle: PathBuf,
    target: PathBuf,
    key: PartitionKey,
    recovery: RecoveryKey,
    key_wrap: KeyWrapFile,
}

fn fixture(tag: &str) -> Fixture {
    let root = temp_dir(tag);
    let partition = PartitionId::new("user-a").expect("id");
    let key = PartitionKey::generate(partition.clone());
    let recovery = RecoveryKey::generate();
    let key_wrap = KeyWrapFile::seal_new(
        &partition,
        &key,
        "用户口令样本".as_bytes(),
        &recovery,
        fast_kdf(),
        1_729_500_000_000,
    )
    .expect("wrap");
    Fixture {
        bundle: root.join("bundle"),
        target: root.join("restore-target"),
        key,
        recovery,
        key_wrap,
        _root: root,
    }
}

fn entries() -> Vec<BackupEntry> {
    vec![
        BackupEntry {
            name: "sessions/session-1.json".to_string(),
            data: "{\"turn\":\"敏感对话样本 SESSION-CONTENT\"}"
                .as_bytes()
                .to_vec(),
        },
        BackupEntry {
            name: "memory/episode-1.json".to_string(),
            data: "{\"content\":\"敏感记忆样本 EPISODE-CONTENT\"}"
                .as_bytes()
                .to_vec(),
        },
    ]
}

fn list_files(root: &Path) -> Vec<String> {
    fn walk(prefix: &str, dir: &Path, out: &mut Vec<String>) {
        let Ok(listing) = std::fs::read_dir(dir) else {
            return;
        };
        for item in listing.flatten() {
            let path = item.path();
            let name = if prefix.is_empty() {
                item.file_name().to_string_lossy().into_owned()
            } else {
                format!("{prefix}/{}", item.file_name().to_string_lossy())
            };
            if path.is_dir() {
                walk(&name, &path, out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk("", root, &mut out);
    out.sort();
    out
}

#[test]
fn sealed_backup_round_trip_restores_every_member_byte_for_byte() {
    let fixture = fixture("roundtrip");
    let manifest = export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");
    assert_eq!(manifest.entry_count(), 2);
    assert_eq!(manifest.partition_id, "user-a");

    let verified = verify_sealed_backup(&fixture.bundle).expect("verify");
    assert_eq!(verified, manifest);

    let report = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect("restore with passphrase");
    assert_eq!(report.entries, 2);
    assert_eq!(report.partition_id, "user-a");

    for entry in entries() {
        let restored = std::fs::read(fixture.target.join(&entry.name)).expect("read restored");
        assert_eq!(restored, entry.data, "件 `{}` 必须逐字节一致", entry.name);
    }
}

#[test]
fn restore_also_works_from_the_recovery_piece() {
    let fixture = fixture("recovery-path");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    let report = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Recovery(fixture.recovery.clone()),
    )
    .expect("restore with recovery piece");
    assert_eq!(report.entries, 2);
    assert_eq!(
        std::fs::read(fixture.target.join("memory/episode-1.json")).expect("read"),
        "{\"content\":\"敏感记忆样本 EPISODE-CONTENT\"}".as_bytes()
    );
}

#[test]
fn the_bundle_carries_the_wrapped_dek_but_never_the_kek() {
    let fixture = fixture("no-kek");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    // KEK 不随备份走: bundle 全树逐字节扫, 零 KEK 材料。
    let mut haystack = Vec::new();
    for name in list_files(&fixture.bundle) {
        haystack.extend_from_slice(&std::fs::read(fixture.bundle.join(&name)).expect("read"));
    }
    let kek = apeireth_storage::crypto::derive_kek_from_passphrase(
        "用户口令样本".as_bytes(),
        &fast_kdf(),
    )
    .expect("kek");
    assert!(
        !haystack.windows(32).any(|window| window == &kek[..]),
        "bundle 不得含派生 KEK"
    );
    assert!(
        !haystack
            .windows(fixture.recovery.to_hex().len())
            .any(|w| w == fixture.recovery.to_hex().as_bytes()),
        "bundle 不得含恢复件"
    );
    assert!(
        !haystack
            .windows("用户口令样本".len())
            .any(|w| w == "用户口令样本".as_bytes()),
        "bundle 不得含口令"
    );

    // 但 DEK 包裹件确实随行 (keys/key-wrap.json 在场且可解析)。
    let wrap_text =
        std::fs::read_to_string(fixture.bundle.join("keys/key-wrap.json")).expect("wrap file");
    let wrap = KeyWrapFile::from_json(&wrap_text).expect("wrap parses");
    assert_eq!(wrap.partition_id, "user-a");
}

#[test]
fn a_single_flipped_byte_refuses_the_whole_restore_with_zero_disk_writes() {
    let fixture = fixture("tamper");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    let target_path = fixture.bundle.join("files/memory/episode-1.json.sealed");
    let mut bytes = std::fs::read(&target_path).expect("read");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&target_path, bytes).expect("tamper");

    assert!(
        verify_sealed_backup(&fixture.bundle).is_err(),
        "翻位后校验必须报坏"
    );
    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect_err("翻位必须拒恢复");
    assert!(matches!(error, CryptoError::RestoreRefused(_)));

    // 宁拒不损: 零字节落盘 (目标树保持不存在/空)。
    let written = list_files(&fixture.target);
    assert!(written.is_empty(), "拒恢复时不得落任何字节: {written:?}");
}

#[test]
fn envelope_integrity_is_checked_before_any_disk_write() {
    let fixture = fixture("integrity");
    let manifest = export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    // 同长度密文替换 (摘要照旧会抓, 此处改清单摘要骗过摘要层 → AEAD 信封层必须兜住)。
    let target_path = fixture.bundle.join("files/memory/episode-1.json.sealed");
    let original = std::fs::read(&target_path).expect("read");
    let mut swapped = original.clone();
    let last = swapped.len() - 1;
    swapped[last] ^= 0x01;
    std::fs::write(&target_path, &swapped).expect("swap");
    let tampered_digest = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&swapped);
        hex::encode(hasher.finalize())
    };
    let mut patched = manifest.clone();
    patched
        .entries
        .iter_mut()
        .find(|entry| entry.name == "memory/episode-1.json")
        .expect("entry")
        .sealed_sha256 = tampered_digest;
    std::fs::write(
        fixture.bundle.join("manifest.json"),
        serde_json::to_string_pretty(&patched).expect("json"),
    )
    .expect("rewrite manifest");

    // 摘要层被骗过 → verify 绿; 但 AEAD 信封完整性必须在落盘前拒。
    verify_sealed_backup(&fixture.bundle).expect("digest layer passes after forgery");
    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect_err("信封完整性失败必须拒恢复");
    assert!(matches!(error, CryptoError::RestoreRefused(_)));
    assert!(
        list_files(&fixture.target).is_empty(),
        "拒恢复时不得落任何字节"
    );
}

#[test]
fn wrong_passphrase_is_refused_without_touching_the_disk_and_key_loss_is_explicit() {
    let fixture = fixture("key-loss");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("错口令".to_string()),
    )
    .expect_err("错口令必须拒");
    assert!(matches!(error, CryptoError::RestoreRefused(_)));
    assert!(
        list_files(&fixture.target).is_empty(),
        "拒恢复时不得落任何字节"
    );

    // 口令遗忘且恢复件缺失 = 显式「不可恢复」边界 (无槽可解 = KeyMaterialLost)。
    let mut stripped = fixture.key_wrap.clone();
    stripped.slots.clear();
    std::fs::write(
        fixture.bundle.join("keys/key-wrap.json"),
        stripped.to_json().expect("json"),
    )
    .expect("strip slots");
    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect_err("无槽必须报不可恢复");
    assert!(
        matches!(error, CryptoError::KeyMaterialLost(_)),
        "密钥丢失必须是显式边界帧"
    );
    assert!(KEY_LOSS_BOUNDARY.contains("不可恢复"), "边界文案必须显式");
}

#[test]
fn version_mismatch_is_explicitly_refused_for_new_formats() {
    let fixture = fixture("version");
    let manifest = export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    let mut future = manifest;
    future.format_version = 2;
    std::fs::write(
        fixture.bundle.join("manifest.json"),
        serde_json::to_string_pretty(&future).expect("json"),
    )
    .expect("rewrite");

    let error = verify_sealed_backup(&fixture.bundle).expect_err("未来版本必须拒");
    assert!(
        matches!(error, CryptoError::BackupFormat(_)),
        "版本必须显式拒不猜"
    );
}

#[test]
fn hostile_manifest_paths_are_refused() {
    let fixture = fixture("traversal");
    let manifest = export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    let mut hostile = manifest;
    hostile.entries[0].name = "../escape.txt".to_string();
    std::fs::write(
        fixture.bundle.join("manifest.json"),
        serde_json::to_string_pretty(&hostile).expect("json"),
    )
    .expect("rewrite");

    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect_err("穿越件名必须拒");
    assert!(
        matches!(error, CryptoError::PathSafety(_))
            || matches!(error, CryptoError::RestoreRefused(_))
    );
    assert!(
        list_files(&fixture.target).is_empty(),
        "穿越拒恢复时不得落任何字节"
    );
}

#[test]
fn export_refuses_to_overwrite_an_existing_bundle() {
    let fixture = fixture("overwrite");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");
    let error = export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_001,
    )
    .expect_err("覆盖导出必须拒");
    assert!(matches!(error, CryptoError::BackupFormat(_)));
}

#[test]
fn cross_partition_bundle_cannot_be_restored_under_another_partition() {
    let fixture = fixture("cross-partition");
    export_sealed_backup(
        &fixture.bundle,
        &fixture.key_wrap,
        &fixture.key,
        &entries(),
        1_729_500_000_000,
    )
    .expect("export");

    // 换分区的口令槽物料 (同口令不同分区包裹件) 解不动 A 分区 bundle。
    let other_partition = PartitionId::new("user-b").expect("id");
    let other_key = PartitionKey::generate(other_partition.clone());
    let other_recovery = RecoveryKey::generate();
    let other_wrap = KeyWrapFile::seal_new(
        &other_partition,
        &other_key,
        "用户口令样本".as_bytes(),
        &other_recovery,
        fast_kdf(),
        1_729_500_000_000,
    )
    .expect("wrap");
    std::fs::write(
        fixture.bundle.join("keys/key-wrap.json"),
        other_wrap.to_json().expect("json"),
    )
    .expect("swap wrap");

    let error = restore_sealed_backup(
        &fixture.bundle,
        &fixture.target,
        &UnlockKey::Passphrase("用户口令样本".to_string()),
    )
    .expect_err("跨分区包裹必须拒恢复");
    assert!(matches!(error, CryptoError::RestoreRefused(_)));
    assert!(
        list_files(&fixture.target).is_empty(),
        "跨分区拒恢复时不得落任何字节"
    );
}
