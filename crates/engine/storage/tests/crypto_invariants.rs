//! B6 加密面不变量测试 (安全隐患 #6: 会话/记忆 SQLite 静态未加密).
//!
//! 三条硬不变量 (任务指定, 逐条钉死):
//! 1. **敏感列永不落明文**: `SealedField` 任一序列化形态不含明文子串;
//! 2. **跨分区 AAD 校验失败拒解**: 分区隔离墙 (创世标尺 §5) 的密码学执法面;
//! 3. **篡改密文拒解**: 密文/tag/nonce 任一位翻转或截断 = 拒解。
//!
//! 另钉: nonce 随机、DEK 包裹件不含 KEK 材料、装饰层后端不见明文。

use apeireth_storage::crypto::{
    FieldSink, KdfParams, KeyWrapFile, PartitionId, PartitionKey, RecoveryKey, SealedField,
    SealingLayer, UnlockKey,
};

fn partition(id: &str) -> PartitionId {
    PartitionId::new(id).expect("partition id")
}

fn fast_kdf() -> KdfParams {
    KdfParams::pbkdf2_hmac_sha256(8, b"invariant-test-salt").expect("params")
}

/// 取出一段序列化形态, 断言不含任何明文子串 (逐窗口扫描, 非仅整串比对)。
fn assert_no_plaintext_substring(serialized: &[u8], plaintext: &[u8], face: &str) {
    assert!(!plaintext.is_empty(), "明文样本不得为空");
    assert!(
        !serialized
            .windows(plaintext.len())
            .any(|window| window == plaintext),
        "{face}: 序列化形态含明文子串 (敏感列落明文)"
    );
}

#[test]
fn sealed_serialization_never_contains_the_plaintext_on_any_face() {
    let key = PartitionKey::generate(partition("user-a"));
    let plaintext = "敏感列样本: 用户对伙伴说过的私密内容 #SENSITIVE-7f3a".as_bytes();
    let sealed = key.seal(plaintext).expect("seal");

    assert_no_plaintext_substring(sealed.to_bytes(), plaintext, "bytes 形态");
    assert_no_plaintext_substring(sealed.to_hex().as_bytes(), plaintext, "hex 形态");
    let json = serde_json::to_string(&sealed).expect("json");
    assert_no_plaintext_substring(json.as_bytes(), plaintext, "JSON 形态");

    // 解封后原样回来 —— 「不落明文」不牺牲「分区内透明」。
    assert_eq!(key.open(&sealed).expect("open"), plaintext);
}

#[test]
fn cross_partition_misuse_is_refused_by_the_partition_aad() {
    let key_a = PartitionKey::generate(partition("user-a"));
    let key_b = PartitionKey::generate(partition("user-b"));
    let sealed = key_a.seal("分区 A 的记忆".as_bytes()).expect("seal");

    // ① 他分区的 DEK: 拒解。
    assert!(
        key_b.open(&sealed).is_err(),
        "他分区 DEK 解本分区密文必须拒"
    );
    // ② 同 DEK 换分区 id (AAD 绑分区): 拒解 —— 防跨分区挪用的核心断言。
    let moved = PartitionKey::from_bytes(partition("user-b"), *key_a.expose_dek());
    assert!(
        moved.open(&sealed).is_err(),
        "同钥挪分区必须拒解 (AAD 绑分区 id)"
    );
    // ③ 原地原钥: 可解 (隔离墙只拦跨分区, 不伤分区内透明)。
    assert_eq!(
        key_a.open(&sealed).expect("open"),
        "分区 A 的记忆".as_bytes()
    );
}

#[test]
fn tampered_ciphertext_tag_or_nonce_is_refused() {
    let key = PartitionKey::generate(partition("user-a"));
    let sealed = key.seal("待保护的记忆".as_bytes()).expect("seal");
    let raw = sealed.to_bytes().to_vec();

    // 密文位翻转。
    let mut flip_ct = raw.clone();
    let tag_start = flip_ct.len() - 16;
    flip_ct[tag_start - 1] ^= 0x01;
    assert!(
        key.open(&SealedField::from_bytes(flip_ct).expect("shape"))
            .is_err(),
        "密文翻位必须拒"
    );

    // tag 位翻转。
    let mut flip_tag = raw.clone();
    let last = flip_tag.len() - 1;
    flip_tag[last] ^= 0x01;
    assert!(
        key.open(&SealedField::from_bytes(flip_tag).expect("shape"))
            .is_err(),
        "tag 翻位必须拒"
    );

    // nonce 位翻转 (头部 nonce 在 [5..17))。
    let mut flip_nonce = raw.clone();
    flip_nonce[5] ^= 0x01;
    assert!(
        key.open(&SealedField::from_bytes(flip_nonce).expect("shape"))
            .is_err(),
        "nonce 翻位必须拒"
    );

    // 截断一字节: 形状仍在 (≥ 头+tag), 必须由 AEAD 信封层拒解。
    let mut truncated = raw.clone();
    truncated.truncate(truncated.len() - 1);
    let truncated = SealedField::from_bytes(truncated).expect("shape");
    assert!(key.open(&truncated).is_err(), "截断必须拒解");

    // 截断到头+tag 以下: 形状校验直接拒。
    let mut broken = raw;
    broken.truncate(10);
    assert!(SealedField::from_bytes(broken).is_err(), "过短必须拒");
}

#[test]
fn every_seal_draws_a_fresh_random_nonce() {
    let key = PartitionKey::generate(partition("user-a"));
    let first = key.seal(b"same").expect("seal");
    let second = key.seal(b"same").expect("seal");
    assert_ne!(first.nonce(), second.nonce(), "同明文两封 nonce 必须不同");
    assert_ne!(
        first.to_bytes(),
        second.to_bytes(),
        "同明文两封字节必须不同"
    );
}

#[test]
fn key_wrap_file_carries_no_kek_material() {
    let key = PartitionKey::generate(partition("user-a"));
    let recovery = RecoveryKey::generate();
    let file = KeyWrapFile::seal_new(
        key.partition(),
        &key,
        "用户口令样本".as_bytes(),
        &recovery,
        fast_kdf(),
        1_729_500_000_000,
    )
    .expect("wrap");
    let json = file.to_json().expect("json");

    assert!(
        !json.contains(&recovery.to_hex()),
        "包裹件不得随行恢复件 (KEK 不随备份走)"
    );
    assert!(!json.contains("用户口令样本"), "包裹件不得含口令");
    let kek = apeireth_storage::crypto::derive_kek_from_passphrase(
        "用户口令样本".as_bytes(),
        &fast_kdf(),
    )
    .expect("kek");
    assert!(!json.contains(&hex::encode(*kek)), "包裹件不得含派生 KEK");
}

#[test]
fn key_wrap_unlock_needs_the_right_material_and_refuses_tamper() {
    let key = PartitionKey::generate(partition("user-a"));
    let recovery = RecoveryKey::generate();
    let file = KeyWrapFile::seal_new(
        key.partition(),
        &key,
        b"pw",
        &recovery,
        fast_kdf(),
        1_729_500_000_000,
    )
    .expect("wrap");

    let unlocked = file
        .unlock(&UnlockKey::Passphrase("pw".to_string()))
        .expect("passphrase unlock");
    assert_eq!(unlocked.expose_dek(), key.expose_dek());
    assert_eq!(
        file.unlock(&UnlockKey::Recovery(recovery.clone()))
            .expect("recovery unlock")
            .expose_dek(),
        key.expose_dek()
    );

    assert!(
        file.unlock(&UnlockKey::Passphrase("bad".to_string()))
            .is_err(),
        "错口令必须拒"
    );
    assert!(
        file.unlock(&UnlockKey::Recovery(RecoveryKey::generate()))
            .is_err(),
        "错恢复件必须拒"
    );

    // 篡改槽位密文 → 拒解。
    let mut tampered = file.clone();
    let slot = tampered.slots.last_mut().expect("slot");
    let mut bytes = hex::decode(&slot.sealed_dek_hex).expect("hex");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    slot.sealed_dek_hex = hex::encode(bytes);
    assert!(
        tampered.unlock(&UnlockKey::Recovery(recovery)).is_err(),
        "篡改包裹必须拒"
    );
}

#[test]
fn sealing_layer_decorates_any_backend_without_leaking_plaintext() {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct MemorySink(BTreeMap<String, Vec<u8>>);

    impl FieldSink for MemorySink {
        fn get_field(
            &self,
            field_key: &str,
        ) -> Result<Option<Vec<u8>>, apeireth_storage::crypto::CryptoError> {
            Ok(self.0.get(field_key).cloned())
        }

        fn put_field(
            &mut self,
            field_key: &str,
            value: Vec<u8>,
        ) -> Result<(), apeireth_storage::crypto::CryptoError> {
            self.0.insert(field_key.to_string(), value);
            Ok(())
        }
    }

    let key_a = PartitionKey::generate(partition("user-a"));
    let mut layer = SealingLayer::new(MemorySink::default(), key_a.clone());
    layer
        .put_sealed("episode:1", "装饰层明文样本".as_bytes())
        .expect("put");
    assert_eq!(
        layer.get_sealed("episode:1").expect("get"),
        Some("装饰层明文样本".as_bytes().to_vec())
    );

    let sink = layer.into_inner();
    let raw = sink.get_field("episode:1").expect("get").expect("value");
    assert_no_plaintext_substring(&raw, "装饰层明文样本".as_bytes(), "装饰层后端落盘值");

    let layer_b = SealingLayer::new(sink, PartitionKey::generate(partition("user-b")));
    assert!(
        layer_b.get_sealed("episode:1").is_err(),
        "换分区读装饰层必须拒解"
    );
}
