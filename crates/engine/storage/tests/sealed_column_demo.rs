//! B6-DEEP 列级接入示范端到端测试: SealingLayer 真接 storage 真实表.
//!
//! 真实表 = `run_migrations` 建出的 `episodes`/`notes` (`id, data` 形态,
//! schema v1); 示范接缝 = [`apeireth_storage::crypto::SqliteFieldSink`]。
//! 钉死面 (任务指定):
//! - **写→读→原文一致** (往返);
//! - **重启后读得出** (关连接重开文件库, 会话/记忆重启语义);
//! - **篡改拒读** (列值被翻位 = fail-closed 拒读, 不吐坏明文);
//! - **库文件零明文** (敏感列永不落明文, 真实表层面复证);
//! - **跨分区拒读** (分区隔离墙在真实表上的执法面)。
//!
//! 接缝边界: memory/episode.rs 等真读写路径归口各存储 owner, 本文件只钉
//! storage 侧接缝语义 (接入登记见批报告)。

use apeireth_storage::crypto::{PartitionId, PartitionKey, SealedTable, SqliteFieldSink};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-sealed-column-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn key(partition: &str) -> PartitionKey {
    PartitionKey::generate(PartitionId::new(partition).expect("id"))
}

/// 示例明文 (含敏感标记, 供库文件零明文扫描)。
const SECRET: &[u8] = "SECRETMARK-私密会话内容-7f3a".as_bytes();

#[test]
fn sealed_column_round_trips_through_the_real_notes_table() {
    let root = temp_dir("roundtrip");
    let db = root.join("sessions.sqlite3");

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("open real table");
    let mut layer = sink.seal_with(key("user-a"));
    layer.put_sealed("note-1", SECRET).expect("put");
    layer
        .put_sealed("note-2", "另一条随手记".as_bytes())
        .expect("put");

    assert_eq!(
        layer.get_sealed("note-1").expect("get"),
        Some(SECRET.to_vec())
    );
    assert_eq!(
        layer.get_sealed("note-2").expect("get"),
        Some("另一条随手记".as_bytes().to_vec())
    );
    assert_eq!(layer.get_sealed("note-missing").expect("get"), None);
}

#[test]
fn sealed_column_survives_a_reopen_of_the_real_database() {
    let root = temp_dir("reopen");
    let db = root.join("sessions.sqlite3");
    let key = key("user-a");

    {
        let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("open");
        let mut layer = sink.seal_with(key.clone());
        layer.put_sealed("note-1", SECRET).expect("put");
    } // 连接关闭 = 进程/会话重启边界

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("reopen");
    let layer = sink.seal_with(key);
    assert_eq!(
        layer.get_sealed("note-1").expect("get"),
        Some(SECRET.to_vec()),
        "重启后必须读得出原文"
    );
}

#[test]
fn a_tampered_column_value_is_refused_on_read() {
    let root = temp_dir("tamper");
    let db = root.join("sessions.sqlite3");
    let key = key("user-a");

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("open");
    let mut layer = sink.seal_with(key.clone());
    layer.put_sealed("note-1", SECRET).expect("put");

    // 直连翻位 (绕过接缝模拟落盘层篡改): 读路径必须 fail-closed 拒读。
    {
        use rusqlite::Connection;
        let conn = Connection::open(&db).expect("raw open");
        let hex_text: String = conn
            .query_row("SELECT data FROM notes WHERE id = 'note-1'", [], |row| {
                row.get(0)
            })
            .expect("raw read");
        let mut bytes = hex::decode(&hex_text).expect("hex");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        conn.execute(
            "UPDATE notes SET data = ?1 WHERE id = 'note-1'",
            rusqlite::params![hex::encode(bytes)],
        )
        .expect("tamper");
    }

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("reopen");
    let layer = sink.seal_with(key);
    assert!(layer.get_sealed("note-1").is_err(), "篡改列值必须拒读");
}

#[test]
fn the_real_database_file_holds_no_plaintext() {
    let root = temp_dir("no-plaintext");
    let db = root.join("sessions.sqlite3");

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Episodes).expect("open");
    let mut layer = sink.seal_with(key("user-a"));
    layer.put_sealed("ep-1", SECRET).expect("put");
    drop(layer);

    let file_bytes = std::fs::read(&db).expect("read db file");
    assert!(
        !file_bytes
            .windows(SECRET.len())
            .any(|window| window == SECRET),
        "真实库文件不得含明文子串 (敏感列永不落明文)"
    );
    // 密文单元魔数 hex 形态确实在场 (数据没丢, 只是封了)。
    assert!(
        file_bytes
            .windows(b"41505331".len())
            .any(|window| window == b"41505331"),
        "库文件应含密文单元魔数 hex (APS1)"
    );
}

#[test]
fn cross_partition_reads_on_the_real_table_are_refused() {
    let root = temp_dir("cross-partition");
    let db = root.join("sessions.sqlite3");
    let key_a = key("user-a");
    let key_b = key("user-b");

    {
        let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("open");
        let mut layer = sink.seal_with(key_a.clone());
        layer.put_sealed("note-1", SECRET).expect("put");
    }

    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("reopen");
    let layer_b = sink.seal_with(key_b);
    assert!(
        layer_b.get_sealed("note-1").is_err(),
        "他分区 DEK 读本分区真实表必须拒"
    );

    // 对照: 同分区同钥可读 (证明上面的拒是分区墙, 不是数据坏)。
    let sink = SqliteFieldSink::open_file(&db, SealedTable::Notes).expect("reopen");
    let layer_a = sink.seal_with(key_a);
    assert_eq!(
        layer_a.get_sealed("note-1").expect("get"),
        Some(SECRET.to_vec()),
        "同分区同钥必须照常读出 (分区墙不伤分区内透明)"
    );
}
