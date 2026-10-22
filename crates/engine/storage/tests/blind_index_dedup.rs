//! B6-BLIND 盲索引端到端测试: 查重/溯源走索引面, 内容面照封不触碰.
//!
//! 钉死面 (任务指定):
//! - **查重往返命中**: 同指纹多行全命中, 异值不命中;
//! - **查询不触原文**: 内容列被篡改后索引查询照常命中 (两面独立);
//! - **库文件无明文原哈希**: `blind_index` 在场, 原哈希字节/hex 均不在;
//! - **篡改盲索引 = 命中失效** (本批如实选定的篡改语义: 漏检, 不吐错值);
//! - **跨分区失效**: 他分区键 (含同 DEK 挪分区) 查不到本分区索引。

use apeireth_storage::crypto::{
    BlindIndexKey, BlindIndexSink, IndexTarget, PartitionId, PartitionKey, SealedTable,
    SqliteFieldSink,
};
use sha2::{Digest, Sha256};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-blind-index-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn dek(partition: &str) -> PartitionKey {
    PartitionKey::generate(PartitionId::new(partition).expect("id"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// 组合 fixture: 内容面 (封缄行) + 索引面 (盲索引条目), 同一真实库文件。
struct Faces {
    _root: std::path::PathBuf,
    db: std::path::PathBuf,
    dek: PartitionKey,
}

fn fixture(tag: &str) -> Faces {
    let root = temp_dir(tag);
    Faces {
        db: root.join("sessions.sqlite3"),
        dek: dek("user-a"),
        _root: root,
    }
}

/// 内容面写一行 (真实 notes 表, SealingLayer 封缄)。
fn write_row(faces: &Faces, row_id: &str, content: &[u8]) {
    let sink =
        SqliteFieldSink::open_file(&faces.db, SealedTable::Notes).expect("open content face");
    let mut layer = sink.seal_with(faces.dek.clone());
    layer.put_sealed(row_id, content).expect("put sealed row");
}

/// 索引面登记一条 (notes.content_hash, 原哈希 → 盲索引)。
fn index_row(faces: &Faces, row_id: &str, fingerprint: &str) {
    let key = BlindIndexKey::derive(&faces.dek);
    let mut sink = BlindIndexSink::open_file(&faces.db, key).expect("open index face");
    let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");
    sink.insert(&target, row_id, fingerprint.as_bytes())
        .expect("insert index");
}

#[test]
fn dedup_round_trip_hits_every_duplicate_without_touching_the_content() {
    let faces = fixture("roundtrip");
    let secret_one = "SECRETMARK-第一条私密内容-a1".as_bytes();
    let secret_two = "SECRETMARK-第二条私密内容-b2".as_bytes();
    let fingerprint_one = sha256_hex(secret_one);
    let fingerprint_two = sha256_hex(secret_two);

    write_row(&faces, "row-1", secret_one);
    write_row(&faces, "row-2", secret_one); // 同内容 = 同指纹 (重复件)
    write_row(&faces, "row-3", secret_two);
    index_row(&faces, "row-1", &fingerprint_one);
    index_row(&faces, "row-2", &fingerprint_one);
    index_row(&faces, "row-3", &fingerprint_two);

    let key = BlindIndexKey::derive(&faces.dek);
    let sink = BlindIndexSink::open_file(&faces.db, key).expect("open index face");
    let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");

    // 查重往返: 给原哈希 → 算盲索引 → 全命中 (不需要解封内容列)。
    assert_eq!(
        sink.find(&target, fingerprint_one.as_bytes())
            .expect("find"),
        vec!["row-1".to_string(), "row-2".to_string()],
        "同指纹必须全命中"
    );
    assert!(
        sink.find(&target, fingerprint_two.as_bytes())
            .expect("find")
            == vec!["row-3".to_string()],
        "异指纹必须精确命中自己"
    );
    assert!(sink
        .find(&target, sha256_hex(b"never-written").as_bytes())
        .expect("find")
        .is_empty());
}

#[test]
fn dedup_queries_do_not_touch_the_sealed_content_face() {
    let faces = fixture("two-faces");
    let secret = "SECRETMARK-私密内容-c3".as_bytes();
    let fingerprint = sha256_hex(secret);
    write_row(&faces, "row-1", secret);
    index_row(&faces, "row-1", &fingerprint);

    // 落盘层篡改内容列 (内容面坏): 索引查询必须照常命中 (查询不触原文)。
    {
        use rusqlite::Connection;
        let conn = Connection::open(&faces.db).expect("raw open");
        let hex_text: String = conn
            .query_row("SELECT data FROM notes WHERE id = 'row-1'", [], |row| {
                row.get(0)
            })
            .expect("raw read");
        let mut bytes = hex::decode(&hex_text).expect("hex");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        conn.execute(
            "UPDATE notes SET data = ?1 WHERE id = 'row-1'",
            rusqlite::params![hex::encode(bytes)],
        )
        .expect("tamper content face");
    }

    let key = BlindIndexKey::derive(&faces.dek);
    let sink = BlindIndexSink::open_file(&faces.db, key).expect("open index face");
    let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");
    assert_eq!(
        sink.find(&target, fingerprint.as_bytes()).expect("find"),
        vec!["row-1".to_string()],
        "内容面坏不影响索引查询 (查询不触原文)"
    );

    // 内容面自身照旧 fail-closed 拒读 (两面独立, 各自语义各自守)。
    let content_sink = SqliteFieldSink::open_file(&faces.db, SealedTable::Notes).expect("open");
    let layer = content_sink.seal_with(faces.dek.clone());
    assert!(layer.get_sealed("row-1").is_err(), "篡改内容列必须拒读");
}

#[test]
fn the_database_holds_the_blind_index_but_never_the_original_hash() {
    let faces = fixture("no-plaintext-hash");
    let secret = "SECRETMARK-私密内容-d4".as_bytes();
    let fingerprint = sha256_hex(secret);
    write_row(&faces, "row-1", secret);
    index_row(&faces, "row-1", &fingerprint);

    let key = BlindIndexKey::derive(&faces.dek);
    let expected_index = key.index(fingerprint.as_bytes()).to_hex();

    let file_bytes = std::fs::read(&faces.db).expect("read db file");
    // 原哈希不在盘上 (hex 形态与原始字节都查)。
    assert!(
        !file_bytes
            .windows(fingerprint.len())
            .any(|window| window == fingerprint.as_bytes()),
        "库文件不得含原哈希 hex (明文哈希不上盘)"
    );
    let raw_digest = hex::decode(&fingerprint).expect("hex");
    assert!(
        !file_bytes
            .windows(raw_digest.len())
            .any(|window| window == raw_digest.as_slice()),
        "库文件不得含原哈希原始字节"
    );
    // 盲索引在场 (数据没丢, 检索键换成了单向键)。
    assert!(
        file_bytes
            .windows(expected_index.len())
            .any(|window| window == expected_index.as_bytes()),
        "库文件应含盲索引 hex"
    );
}

#[test]
fn a_tampered_blind_index_entry_breaks_the_hit_not_the_content_face() {
    let faces = fixture("tamper-index");
    let secret = "SECRETMARK-私密内容-e5".as_bytes();
    let fingerprint = sha256_hex(secret);
    write_row(&faces, "row-1", secret);
    index_row(&faces, "row-1", &fingerprint);

    // 落盘层篡改盲索引条目: 语义如实选定为**命中失效** (漏检, 不吐错值)。
    {
        use rusqlite::Connection;
        let conn = Connection::open(&faces.db).expect("raw open");
        conn.execute(
            "UPDATE blind_index_entries SET blind_index = ?1 WHERE row_id = 'row-1'",
            rusqlite::params![hex::encode([0u8; 32])],
        )
        .expect("tamper index face");
    }

    let key = BlindIndexKey::derive(&faces.dek);
    let sink = BlindIndexSink::open_file(&faces.db, key).expect("open index face");
    let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");
    assert!(
        sink.find(&target, fingerprint.as_bytes())
            .expect("find")
            .is_empty(),
        "篡改盲索引必须命中失效 (不吐错值)"
    );

    // 内容面不受影响: 行照旧读得出原文 (索引坏不毁数据)。
    let content_sink = SqliteFieldSink::open_file(&faces.db, SealedTable::Notes).expect("open");
    let layer = content_sink.seal_with(faces.dek.clone());
    assert_eq!(
        layer.get_sealed("row-1").expect("get"),
        Some(secret.to_vec()),
        "索引面坏不毁内容面"
    );
}

#[test]
fn cross_partition_queries_miss_even_for_the_same_dek() {
    let faces = fixture("cross-partition");
    let secret = "SECRETMARK-私密内容-f6".as_bytes();
    let fingerprint = sha256_hex(secret);
    write_row(&faces, "row-1", secret);
    index_row(&faces, "row-1", &fingerprint);

    let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");

    // 同 DEK 挪分区 id: 派生绑分区, 查不到 (分区墙不破)。
    let twin = PartitionKey::from_bytes(
        PartitionId::new("user-b").expect("id"),
        *faces.dek.expose_dek(),
    );
    let twin_key = BlindIndexKey::derive(&twin);
    let twin_sink = BlindIndexSink::open_file(&faces.db, twin_key).expect("open");
    assert!(
        twin_sink
            .find(&target, fingerprint.as_bytes())
            .expect("find")
            .is_empty(),
        "同 DEK 挪分区必须查不到"
    );

    // 他分区独立 DEK: 同样查不到。
    let other_key = BlindIndexKey::derive(&dek("user-c"));
    let other_sink = BlindIndexSink::open_file(&faces.db, other_key).expect("open");
    assert!(
        other_sink
            .find(&target, fingerprint.as_bytes())
            .expect("find")
            .is_empty(),
        "他分区必须查不到"
    );

    // 对照: 本分区照常命中。
    let own_key = BlindIndexKey::derive(&faces.dek);
    let own_sink = BlindIndexSink::open_file(&faces.db, own_key).expect("open");
    assert_eq!(
        own_sink
            .find(&target, fingerprint.as_bytes())
            .expect("find"),
        vec!["row-1".to_string()],
        "本分区必须照常命中 (对照)"
    );
}
