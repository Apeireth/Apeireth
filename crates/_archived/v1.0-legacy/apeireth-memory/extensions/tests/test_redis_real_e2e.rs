//! # R46 B6 - Mini-Redis RESP mock + RedisProvider 真接 e2e
//!
//! per task description in reports/r46-r53-...md.
//! Supports RESP2 subset: PING/SET/GET/DEL/EXISTS/FLUSHDB/DBSIZE + handshake no-ops.
//! Binary-safe bulk strings; no TTL (RedisProvider tests do not exercise TTL).

#![cfg(feature = "real-http")]

use apeireth_memory_extensions::{
    MemoryProvider, ProviderConfig, ProviderKind, ProviderScope, RedisProvider,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Default)]
struct MiniRedisState {
    kv: HashMap<Vec<u8>, Vec<u8>>,
}
type SharedState = Arc<AsyncMutex<MiniRedisState>>;

fn find_crlf(buf: &[u8]) -> Option<usize> {
    (0..buf.len().saturating_sub(1)).find(|&i| buf[i] == b'\r' && buf[i + 1] == b'\n')
}

async fn fill_buf(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Result<usize, String> {
    let mut tmp = [0u8; 2048];
    let n: usize = AsyncReadExt::read(stream, &mut tmp)
        .await
        .map_err(|e: std::io::Error| e.to_string())?;
    buf.extend_from_slice(&tmp[..n]);
    Ok(n)
}

async fn read_resp_command(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
) -> Result<Vec<Vec<u8>>, String> {
    // Fill until we have enough to parse a complete command.
    while find_crlf(buf).is_none() {
        let n = fill_buf(stream, buf).await?;
        if n == 0 {
            return Err("eof".into());
        }
    }
    let header_end = find_crlf(buf).ok_or("no header crlf")?;
    if buf[0] != b'*' {
        return Err(format!("expected '*' got {:?}'", buf[0]));
    }
    let n: usize = std::str::from_utf8(&buf[1..header_end])
        .map_err(|e: std::str::Utf8Error| e.to_string())?
        .trim()
        .parse::<usize>()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let mut consumed = header_end + 2;
    let mut args: Vec<Vec<u8>> = Vec::with_capacity(n);
    for _ in 0..n {
        // Wait for $header CRLF.
        while find_crlf(&buf[consumed..]).is_none() {
            let rn = fill_buf(stream, buf).await?;
            if rn == 0 {
                return Err("eof mid-arg".into());
            }
        }
        let header_end2 = find_crlf(&buf[consumed..]).unwrap();
        if buf[consumed] != b'$' {
            return Err("expected '$'".to_string());
        }
        let len: i64 = std::str::from_utf8(&buf[consumed + 1..consumed + header_end2])
            .map_err(|e: std::str::Utf8Error| e.to_string())?
            .trim()
            .parse::<i64>()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let hdr_len = consumed + header_end2 + 2;
        if len < 0 {
            args.push(Vec::new());
            consumed = hdr_len;
        } else {
            let need = hdr_len + (len as usize) + 2;
            while buf.len() < need {
                let rn = fill_buf(stream, buf).await?;
                if rn == 0 {
                    return Err("eof mid-data".into());
                }
            }
            let data = buf[hdr_len..hdr_len + len as usize].to_vec();
            args.push(data);
            consumed = need;
        }
    }
    buf.drain(..consumed);
    Ok(args)
}

async fn handle_conn(mut stream: TcpStream, state: SharedState) {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    loop {
        let cmd = match read_resp_command(&mut stream, &mut buf).await {
            Ok(c) => c,
            Err(_) => return,
        };
        if cmd.is_empty() {
            return;
        }
        let cmd_name_upper: Vec<u8> = cmd[0].iter().map(|b| b.to_ascii_uppercase()).collect();
        let _nm = std::str::from_utf8(&cmd_name_upper).unwrap_or("?");
        let reply = dispatch(state.clone(), &cmd_name_upper, &cmd[1..]).await;
        if stream.write_all(&reply).await.is_err() {
            return;
        }
        if cmd_name_upper == b"QUIT" {
            return;
        }
    }
}

async fn dispatch(state: SharedState, cmd_name: &[u8], args: &[Vec<u8>]) -> Vec<u8> {
    match cmd_name {
        b"PING" => b"+PONG\r\n".to_vec(),
        b"SET" => cmd_set(state, args).await,
        b"GET" => cmd_get(state, args).await,
        b"DEL" => cmd_del(state, args).await,
        b"EXISTS" => cmd_exists(state, args).await,
        b"FLUSHDB" => cmd_flushdb(state).await,
        b"DBSIZE" => cmd_dbsize(state).await,
        b"COMMAND" | b"CLIENT" | b"HELLO" | b"AUTH" | b"SELECT" => b"+OK\r\n".to_vec(),
        b"QUIT" => b"+OK\r\n".to_vec(),
        _ => b"+OK\r\n".to_vec(),
    }
}

async fn cmd_set(state: SharedState, args: &[Vec<u8>]) -> Vec<u8> {
    if args.len() < 2 {
        return b"-ERR wrong\r\n".to_vec();
    }
    let key = args[0].clone();
    let value = args[1].clone();
    state.lock().await.kv.insert(key, value);
    b"+OK\r\n".to_vec()
}

async fn cmd_get(state: SharedState, args: &[Vec<u8>]) -> Vec<u8> {
    if args.len() != 1 {
        return b"-ERR wrong\r\n".to_vec();
    }
    let st = state.lock().await;
    match st.kv.get(&args[0]) {
        Some(v) => {
            let mut out = format!("${}\r\n", v.len()).into_bytes();
            out.extend_from_slice(v);
            out.extend_from_slice(b"\r\n");
            out
        }
        None => b"$-1\r\n".to_vec(),
    }
}

async fn cmd_del(state: SharedState, args: &[Vec<u8>]) -> Vec<u8> {
    let mut st = state.lock().await;
    let mut n = 0i64;
    for k in args {
        if st.kv.remove(k).is_some() {
            n += 1;
        }
    }
    format!(":{}\r\n", n).into_bytes()
}

async fn cmd_exists(state: SharedState, args: &[Vec<u8>]) -> Vec<u8> {
    let st = state.lock().await;
    let mut n = 0i64;
    for k in args {
        if st.kv.contains_key(k) {
            n += 1;
        }
    }
    format!(":{}\r\n", n).into_bytes()
}

async fn cmd_flushdb(state: SharedState) -> Vec<u8> {
    state.lock().await.kv.clear();
    b"+OK\r\n".to_vec()
}

async fn cmd_dbsize(state: SharedState) -> Vec<u8> {
    let n = state.lock().await.kv.len();
    format!(":{}\r\n", n).into_bytes()
}

pub async fn spawn_mini_redis() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let _ = fs_err::File::create("/tmp/miniredis.log");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr");
    let state: SharedState = Arc::new(AsyncMutex::new(MiniRedisState::default()));
    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let st = state.clone();
                    tokio::spawn(handle_conn(stream, st));
                }
                Err(_) => return,
            }
        }
    });
    (addr, handle)
}

fn make_cfg(url: String) -> ProviderConfig {
    ProviderConfig::new(
        url,
        Duration::from_secs(5),
        1024 * 1024,
        true,
        Duration::from_secs(60),
        ProviderScope::Global,
    )
}

// === 裸 TCP 摸底 ===
#[tokio::test]
async fn e2e_00_raw_socket_ping() {
    let (addr, _h) = spawn_mini_redis().await;
    let mut s = tokio::net::TcpStream::connect(addr).await.expect("connect");
    s.write_all(b"*1\r\n$4\r\nPING\r\n").await.expect("write");
    let mut buf = [0u8; 64];
    let n = s.read(&mut buf).await.expect("read");
    let reply = std::str::from_utf8(&buf[..n]).unwrap_or("<bin>");
    eprintln!("[test] got: {reply:?}");
    assert!(reply.starts_with("+PONG"), "want +PONG, got {reply:?}");
}

// === 6 个 e2e 测试 (RedisProvider + miniredis) ===
#[tokio::test]
async fn e2e_01_redis_provider_kind() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    assert_eq!(p.kind(), ProviderKind::Redis);
}

#[tokio::test]
async fn e2e_02_redis_set_get_roundtrip() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    p.set("hello", b"world").await.expect("set");
    p.set("k2", &[0u8, 1, 2, b'\r', b'\n', 3])
        .await
        .expect("set binary");
    assert_eq!(p.get("hello").await.unwrap(), Some(b"world".to_vec()));
    assert_eq!(
        p.get("k2").await.unwrap(),
        Some(vec![0u8, 1, 2, b'\r', b'\n', 3])
    );
}

#[tokio::test]
async fn e2e_03_redis_get_missing_returns_none() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    assert!(p.get("nope").await.unwrap().is_none());
}

#[tokio::test]
async fn e2e_04_redis_delete_exists_size() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    p.set("a", b"1").await.unwrap();
    p.set("b", b"2").await.unwrap();
    assert!(p.exists("a").await.unwrap());
    assert!(!p.exists("c").await.unwrap());
    assert_eq!(p.size().await.unwrap(), 2);
    p.delete("a").await.unwrap();
    assert!(!p.exists("a").await.unwrap());
    assert_eq!(p.size().await.unwrap(), 1);
}

#[tokio::test]
async fn e2e_05_redis_clear() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    for i in 0..10 {
        p.set(&format!("k{}", i), b"v").await.unwrap();
    }
    assert_eq!(p.size().await.unwrap(), 10);
    p.clear().await.unwrap();
    assert_eq!(p.size().await.unwrap(), 0);
    assert!(p.get("k0").await.unwrap().is_none());
}

#[tokio::test]
async fn e2e_06_redis_full_cycle_no_off_by_one() {
    let (addr, _h) = spawn_mini_redis().await;
    let url = format!("redis://127.0.0.1:{}/0", addr.port());
    let p = RedisProvider::new(make_cfg(url)).unwrap();
    for i in 0..100 {
        let k = format!("key_{i:04}", i = i);
        let v = format!("val_{i:04}_with_padding_xxxxxxxxxxxxxxxxx", i = i);
        p.set(&k, v.as_bytes()).await.unwrap();
    }
    assert_eq!(p.size().await.unwrap(), 100);
    for i in 0..100 {
        let k = format!("key_{i:04}", i = i);
        let expected = format!("val_{i:04}_with_padding_xxxxxxxxxxxxxxxxx", i = i);
        assert_eq!(p.get(&k).await.unwrap(), Some(expected.into_bytes()));
    }
    p.clear().await.unwrap();
    assert_eq!(p.size().await.unwrap(), 0);
}
