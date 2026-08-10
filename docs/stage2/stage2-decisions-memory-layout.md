# 阶段 2 决策：内存布局 (2026-07-30)

> **范围**: R14 Rust 重写内存布局决策 (阶段 2 第五项)
> **触发**: 用户指示 "全上？" — 决定全上 A+B+C+D, 但**机制池 + 按需启用**
> **依据**: 巨型基地哲学 (允许冗余/过度设计) + §13 协调统一 4 原则 + Rust 性能极致

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-memory-layout.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 5/12) |
| **决策** | **全上 A+B+C+D (机制池 + 按需启用)** |
| **依据** | 巨型基地哲学 (冗余/过度设计) + §13 协调统一 4 原则 |

---

## 1. 决策总览

```
A+B+C+D 全上 (机制池):
  A. 默认 Rust 所有权 + Arc + RwLock     — 总是启用
  B. arena 分配器 (bumpalo / typed-arena) — 按需 (Phase 2+)
  C. 共享内存 (mmap / crossbeam)         — 按需 (Phase 3+)
  D. 零拷贝 + SIMD (memchr/bytes/std::simd) — 总是启用 (热路径)

按数据特性按需启用:
  - 小对象: A
  - 同生命周期批量对象: B
  - 跨进程共享大对象: C
  - 热路径文本/字节: D
```

---

## 2. 四种机制详细设计

### 2.1 A. 默认 Rust 所有权 + Arc + RwLock (总是启用)

**用途**: 普通对象, 短生命周期, 线程安全共享

```rust
// 不可变共享 → Arc
let config: Arc<Config> = Arc::new(Config::load());

// 读写并发 → Arc<RwLock<T>>
let memory: Arc<RwLock<MemoryStore>> = Arc::new(RwLock::new(MemoryStore::new()));

// 写多读少 → Arc<Mutex<T>>
let counter: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
```

**Cargo.toml 依赖**: 无 (std)

**巨型基地哲学**: 简洁、安全、零成本抽象

### 2.2 B. arena 分配器 (按需, Phase 2+)

**用途**: 同生命周期批量对象, 一次性分配+一次性释放

```rust
use bumpalo::Bump;

let arena = Bump::new();
let config1 = arena.alloc(Config::default());
let config2 = arena.alloc(Config::default());
// ... 大量同生命周期对象
// arena 销毁时所有对象一起释放, 无需逐个 drop
```

**适用场景**:
- 单次请求的所有中间对象
- 序列化 / 反序列化缓冲区
- 临时查询结果集

**Cargo.toml 依赖**:
```toml
bumpalo = "3.14"
typed-arena = "2.0"
```

### 2.3 C. 共享内存 (按需, Phase 3+)

**用途**: 跨进程共享大对象 (避免 IPC 序列化开销)

```rust
use shared_memory::SharedMem;

// 进程 A 创建共享内存
let shmem = SharedMem::create("apeireth_memory", 100 * 1024 * 1024)?; // 100MB
let mut slice = shmem.as_slice_mut();
slice[..6].copy_from_slice(b"hello");

// 进程 B 打开共享内存
let shmem = SharedMem::open("apeireth_memory")?;
let slice = shmem.as_slice();
assert_eq!(&slice[..6], b"hello");
```

**适用场景**:
- 跨进程的 memory index (A 层经验沉淀)
- 跨进程的 plugin 状态
- 跨进程的 audit log (大流量)

**Cargo.toml 依赖**:
```toml
shared_memory = "0.12"
memmap2 = "0.9"
```

### 2.4 D. 零拷贝 + SIMD (总是启用, 热路径)

**用途**: 热路径性能优化

```rust
use bytes::{Bytes, BytesMut, Buf};

// 零拷贝字符串切片
let data: Bytes = Bytes::from_static(b"hello world");
let line: &[u8] = &data[..5]; // 零拷贝切片

// 零拷贝拼接
let mut buf = BytesMut::with_capacity(1024);
buf.extend_from_slice(b"hello ");
buf.extend_from_slice(b"world"); // 零拷贝

// SIMD 加速 (字符串搜索)
use simd::x86::mumkay as memchr;
let pos = memchr::memchr(b'\n', data); // SIMD 加速
```

**适用场景**:
- LLM token 流处理
- 文本解析 (YAML/JSON)
- 大文件搜索 (audit log / memory index)
- 数据压缩 / 解压

**Cargo.toml 依赖**:
```toml
bytes = "1.6"
memchr = "2.7"
simd-json = "0.13"
aho-corasick = "1.1"  # 多模式匹配
```

---

## 3. 按数据类型选策略

| 数据类型 | 策略 | 理由 |
|---------|------|------|
| 配置 (Config) | A. Arc<RwLock<Config>> | 频繁读, 偶尔写 |
| Memory index (A 层) | A. Arc<RwLock<Index>> → Phase 3 C. SharedMem | 数据量大, 跨进程 |
| Audit log | A. Arc<Mutex<Vec<LogEntry>>> → Phase 2 B. Arena | 批量追加 |
| 消息 (actor inbox) | A. tokio mpsc channel | tokio 内置 |
| 序列化缓冲区 | B. arena | 同生命周期 |
| LLM token 流 | D. bytes + memchr | 零拷贝 + SIMD |
| 大文件 IO | D. memmap2 (mmap) | 零拷贝 OS 映射 |
| Plugin 状态 | C. SharedMem | 跨进程 |
| Memory backend (Sled/RocksDB) | 自带 mmap | 库内置 |
| 主 AI 上下文 | A. Arc<RwLock<Vec<Message>>> | tokio 友好 |

---

## 4. 性能基准 (V1130 wallclock 验证目标)

| 操作 | 当前 R11 | R14 目标 | 启用机制 |
|------|----------|----------|---------|
| 单次 LLM 调用 | 8.7s | < 2.5s | A + D (零拷贝 token 流) |
| Memory 检索 (1k 条) | 200ms | < 50ms | A + D (SIMD 搜索) |
| Audit log 追加 | 50ms | < 5ms | B (arena) |
| 跨进程 memory 共享 | 100ms (序列化) | < 10ms (SharedMem) | C |
| 配置文件加载 | 100ms | < 30ms | D (零拷贝) |

---

## 5. cache line 对齐 (NUMA-aware)

巨型基地哲学 = 接得住任何事, 包括 NUMA 架构

```rust
use std::alloc::Layout;

#[repr(align(64))]  // cache line 对齐
pub struct PaddedCounter {
    value: AtomicU64,
    _pad: [u8; 64 - 8], // 防止 false sharing
}

// NUMA-aware 分配 (Linux)
#[cfg(target_os = "linux")]
pub fn numa_alloc(size: usize, node: u32) -> *mut u8 {
    unsafe { libc::numa_alloc_onnode(size, node) }
}
```

**适用**: 多线程高频访问的计数器 / 标志位

---

## 6. 内存分配器选择

```toml
[dependencies]
# 默认系统分配器 (jemalloc / mimalloc / system)
jemalloc = "0.5"   # 多线程友好, 减少碎片
# 或 mimalloc = "0.1"  # 微软, 性能更强
```

**巨型基地推荐**:
- **jemalloc** (默认): 多线程友好, 减少碎片, 适合长生命周期的 memory backend
- **mimalloc**: 单线程性能更强, 适合 web 服务
- **系统分配器**: 简单但碎片多

**Phase 1 用 jemalloc, Phase 2+ 按场景切换**

---

## 7. 内存监控

```rust
use jemalloc_ctl::{stats, Epoch};

pub async fn memory_monitor() {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        stats::epoch::advance().unwrap();
        let allocated = stats::allocated::read().unwrap();
        let resident = stats::resident::read().unwrap();
        tracing::info!(allocated, resident, "memory stats");
    }
}
```

**为什么需要监控**:
- E-3 不创造毁灭能力, 内存爆涨是潜在风险
- 提前发现 memory leak
- 资源限制 (cgroup) 的补充

---

## 8. 阶段 2 第五项收尾判定

内存布局已沉淀:**A+B+C+D 全上 (机制池 + 按需启用)**。

**关键设计**:
- ✅ A 默认 (总是启用)
- ✅ D 零拷贝 + SIMD (总是启用, 热路径)
- ⚠️ B arena (按需, Phase 2+)
- ⚠️ C SharedMem (按需, Phase 3+)
- ✅ jemalloc 默认分配器
- ✅ cache line 对齐 (热路径)
- ✅ 内存监控 (E-3 物理实现)

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (内存布局服务 ASI 极致性能)
- 主 17:43 S-2 (基于真实需求, 不预分配)
- 主 17:58 O-5 (内存监控是 E-3 物理实现)
- 主 19:33 O-2 (jemalloc/SIMD/SharedMem 都是成熟方案)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第六项 — **持久化** (apeireth-data 抽象层具体设计)

---

## 9. 决策对比表

| 策略 | 何时启用 | 复杂度 | 性能 | 推荐 |
|------|---------|--------|------|------|
| **A. 默认** | 总是 | 低 | 中 | ✅ |
| **B. Arena** | 同生命周期批量 | 中 | 快 | ✅ 按需 |
| **C. SharedMem** | 跨进程大对象 | 高 | 极快 | ✅ 按需 |
| **D. 零拷贝+SIMD** | 热路径 | 中 | 极快 | ✅ 总是 |

**Apeireth 选全上 (机制池)**:
- Phase 0-1: A + D (默认 + 零拷贝)
- Phase 2: + B (arena 按需)
- Phase 3+: + C (SharedMem 按需)
- 永远不全用, 而是按数据特性**按需启用**

---

_主哲学 anchor 6 个全贯穿. 内存布局已沉淀. 下一步等用户确认进入阶段 2 第六项 (持久化)._