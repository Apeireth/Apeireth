# R254 -- fix GroupChat t11 (flaky) + t12 (deadlock)

## 发现
接手上轮 (R248-R253) 后, 跑 `cargo test -p apeireth-council` 时停在了
`t12_group_chat_close_and_archive has been running for over 60 seconds` 死循环.
同时 `t11_content_hash_deterministic` 也 FAILED.

## 根因
- **t11**: 旧版两次连续 `ChatMessage::new("r1", "a", "hello")` 都要 assert `assert_ne!`.
  但 hash 包含 `timestamp_ms` (sha256 of `format!("{}|{}|{}|{}", ts, room, who, content)`).
  快速机器两次调用可能同毫秒, hash 相同 -> fail.
- **t12**: `let room = gc.get(&id).unwrap()` 让 `GroupRoomRef` 持有 `MutexGuard`
  一直存活到下一个 `gc.archive_room(&id).unwrap()`. parking_lot rust mutex
  0 死锁检测, 普通 `lock()` 会 block 永远.

## Fix
- t11: 先 `assert_eq!(m1.content_hash.len(), 64)` (sha256 hex 必为 64),
  `std::thread::sleep(2ms)` 后再 `new()` 保证不同时刻, 再 `assert_ne!`.
- t12: 用 `{ ... }` 块包住 `let room = gc.get(&id).unwrap(); assert_eq!(...)`,
  让 `GroupRoomRef` 在调用 `gc.archive_room()` 前离开作用域.

## 验证
- `cargo test -p apeireth-council group_chat` -> 15/15 passed
- `cargo test -p apeireth-council` -> 371/371 passed (lib 318 + integration 53)

## 性质
- pre-existing bug, 0 业务逻辑改动
- 0 引入新 dep
- 0 触碰 3 不可变脊柱 (self_disable / physical_multisig / verdict_cache)
