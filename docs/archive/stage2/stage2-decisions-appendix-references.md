# 阶段 2 决策 — 真实细节附录 (References Appendix)

> **范围**: 阶段 2 各项决策的"真实代码/源码细节"附录
> **触发**: 用户 2026-07-30 指示 "联网搜索, 然后把设计写的清清楚楚, 找到最好的, 适合我们项目, 符合我们哲学的设计就可以了"
> **配套**: 14 个 stage2-decisions-*.md 决策文件

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-appendix-references.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **目的** | 阶段 3 看图纸时, 配合阶段 2 决策 + 此附录, 看到"最好的/最适合我们哲学"的真实代码细节 |
| **检索方式** | curl docs.rs / raw.githubusercontent.com / crates.io API |

---

## 1. 关键 crate 真实代码 (已抓取)

### 1.1 sled (KV, Phase 1)

**crate**: `sled = "0.34"` (阶段 2 §6 已选)
**真实 README**: https://raw.githubusercontent.com/spacejam/sled/master/README.md

**核心 API (真实摘录)**:

```rust
let tree = sled::open("/tmp/welcome-to-sled")?;

// insert and get, similar to std's BTreeMap
let old_value = tree.insert("key", "value")?;

assert_eq!(
    tree.get(&"key")?,
    Some(sled::IVec::from("value")),
);

// range queries
for kv_result in tree.range("key_1".."key_9") {}

// deletion
let old_value = tree.remove(&"key")?;

// atomic compare and swap
tree.compare_and_swap(
    "key",
    Some("current_value"),
    Some("new_value"),
)?;

// block until all operations are stable on disk
tree.flush()?;
```

**Features (来自 README)**:
- API 类似 BTreeMap<[u8], [u8]>
- ACID 事务 (跨 keyspace 原子读写)
- 完全原子单 key 操作 (compare_and_swap)
- 零拷贝读取 (IVec = Arc slice)
- write batches (apply_batch)
- watch_prefix (订阅前缀变化)
- 多 keyspace (Db::open_tree)
- merge operators (自定义 merge 逻辑)
- forward/reverse iterator
- 75-125M unique IDs/sec (generate_id)
- zstd 压缩 (compression feature)
- **自动 fsync every 500ms** (可调)

**为什么是最佳 (符合我们哲学)**:
- ✅ 纯 Rust (RocksDB 是 C++, 编译复杂)
- ✅ 嵌入式 (无外部依赖)
- ✅ ACID 强事务 (满足阶段 2 §6 单 backend 强事务)
- ✅ watch_prefix 适合"事件流" 模式 (阶段 2 §9 通信总线)
- ✅ generate_id 75M/s 满足性能目标 (V1130 < 2.5s)
- ⚠️ **README 警告**: "This README is out of sync with the main branch which contains a large in-progress rewrite" — **需要关注 main 分支重写状态**

### 1.2 tokio (异步 runtime)

**crate**: `tokio = "1.40"` (已有)
**真实 README**: https://raw.githubusercontent.com/tokio-rs/tokio/master/README.md

**核心定位 (来自 README)**:
- Fast (zero-cost abstractions, bare-metal performance)
- Reliable (Rust ownership + type system)
- Scalable (minimal footprint, backpressure + cancellation)

**核心组件**:
- 多线程 work-stealing task scheduler
- async I/O (TCP/UDP/Unix/Files)
- timer / interval / sleep
- sync primitives (Mutex/RwLock/mpsc/broadcast/oneshot/watch/Semaphore/Notify)
- signal handling (SIGTERM/SIGINT/SIGHUP)

**为什么是最佳**:
- ✅ Rust 异步标准 (95% crate 兼容)
- ✅ 工业级 (Discord / AWS / Cloudflare 使用)
- ✅ multi-thread runtime 适合 B+E 架构 (阶段 2 §2)
- ✅ backpressure + cancellation 适合巨型基地反背压 (阶段 2 §9)

### 1.3 tantivy (全文检索, Phase 2)

**crate**: `tantivy = "0.22"` (阶段 2 §6)
**真实 README**: https://raw.githubusercontent.com/quickwit-oss/tantivy/main/README.md

**核心定位 (来自 README)**:
- "Fast full-text search engine library written in Rust"
- 灵感来自 Apache Lucene
- 不是 out-of-the-shelf server, 而是 crate (用于构建搜索引擎)

**Features (来自 README)**:
- 全文检索
- 可配置 tokenizer (17 种拉丁语 stemming, 中文 tantivy-jieba/cang-jie, 日文 lindera, 韩文 lindera-ko-dic)
- BM25 scoring (与 Lucene 相同)
- 自然查询语言 `(michael AND jackson) OR "king of pop"`
- Phrase queries `"michael jackson"`
- 增量索引
- 多线程索引 (英文 Wikipedia < 3 分钟)
- Mmap directory
- 启动时间 < 10ms (适合 CLI 工具)

**为什么是最佳**:
- ✅ 纯 Rust (与 sled / VCP 浪潮一致)
- ✅ Lucene 等价功能 (成熟)
- ✅ 多语言 tokenizer (适合 A 层经验沉淀 + 中文检索)
- ✅ 启动 < 10ms (适合 CLI 模式)
- ✅ Mmap directory (零拷贝 IO, 阶段 2 §5 D 机制)

### 1.4 OpenClaw (借鉴项目)

**项目**: https://github.com/openclaw/openclaw
**真实 README**: https://raw.githubusercontent.com/openclaw/openclaw/main/README.md

**核心定位**:
- Personal AI Assistant
- 多平台 (macOS / iOS / Android node)
- 多渠道 (DM access)
- 沙箱 (groups + multi-user surfaces)
- Skills + Workspace

**借鉴点 (阶段 2 §9 通信总线)**:
- ✅ Gateway 模式 (单长生命周期进程拥有所有消息界面)
- ✅ DM access security (默认 sandbox)
- ✅ 多 Node 接入 (Telegram/Discord/iOS/Android)
- ✅ Operator quick refs

**不借鉴**:
- ❌ 不是 Agent 平台, 是 AI Assistant (不一样)
- ❌ 单 Gateway (我们要 B+E 分布式)

---

## 2. 待抓取 (阶段 3 前补)

### 2.1 async-openai (LLM, 阶段 2 §7)

**URL**: https://github.com/64bit/async-openai (master 分支可能空, 需要 main 或 release)
**需要**: 真实 completion / streaming / function calling 代码示例
**获取**: `curl -sL https://raw.githubusercontent.com/64bit/async-openai/master/README.md` (master 可能空, 试 main/release)

### 2.2 qdrant-client (向量, 阶段 2 §6)

**URL**: https://github.com/qdrant/rust-client
**需要**: 真实 upsert / search 代码
**获取**: `curl -sL https://raw.githubusercontent.com/qdrant/rust-client/main/README.md`

### 2.3 Hermes Agent (借鉴, 阶段 1 §16)

**URL**: https://github.com/hermes-agent/hermes-agent
**需要**: 6 trait 实际代码 / supervisor 树实现 / 1,428 tests 经验
**获取**: `curl -sL https://raw.githubusercontent.com/hermes-agent/hermes-agent/main/README.md`

### 2.4 Erlang/OTP 双实例流量切换 (阶段 2 §11)

**真实来源**: RabbitMQ / riak / cowboy 等 Erlang 项目
**借鉴模式**: "Cutover" 模式 (从 v1 100% 切到 v2 100%, 渐进)
**参考**: https://www.rabbitmq.com/docs/blue-green

### 2.5 VCP 浪潮 (联想网络, 阶段 2 §6 自研)

**真实来源**: VCP ToolBox https://github.com/visioncortex/compound_eye
**需要**: 联想网络 + 河道能量 + 神经网络信号传播源码
**状态**: 自研, 需借鉴 (不在 crates.io)

### 2.6 OpenClaw Gateway 完整源码

**URL**: https://github.com/openclaw/openclaw
**需要**: gateway 进程结构 / WebSocket 模式 / Node 接入协议
**状态**: 已抓 README, 需要看 src/

### 2.7 WebAssembly 沙箱 (阶段 2 §8)

**真实来源**: wasmtime / wasmer
**URL**: https://github.com/bytecodealliance/wasmtime
**需要**: 沙箱隔离 API / 资源限制 / capability-based security

### 2.8 libp2p (多端通信, 阶段 2 §9 备选)

**URL**: https://github.com/libp2p/rust-libp2p
**需要**: 是否可用于 Apeireth 多节点通信 (替代或补充 gRPC)

---

## 3. 决策最佳性论证模板

每项阶段 2 决策, 在附录中需要论证:

```
[决策名]: [选了什么]

✅ 为什么这是"最佳"
  - 对比备选 1 (XXX crate): XXX 优点, 我们不选是因为 XXX
  - 对比备选 2 (YYY crate): YYY 优点, 我们不选是因为 XXX
  - 对比"自研": 我们不自研是因为 XXX
  - 唯一缺点: XXX (但在我们场景下可接受)

✅ 为什么"适合我们项目"
  - 与 R11 LOCKED (Cargo.toml 已配) 一致性: XXX
  - 与 30 crate 划分 (阶段 2 §3) 一致性: XXX
  - 与 B+E 架构 (阶段 2 §2) 一致性: XXX
  - 与巨型基地哲学 (允许冗余/过度设计) 一致性: XXX

✅ 为什么"符合我们哲学"
  - 主 22:33 S-1 (北极星导向): XXX
  - 主 17:43 S-2 (实事求是, 基于真实需求): XXX
  - 主 17:58 O-5 (不假装): XXX
  - 主 19:33 O-2 (走在前人经验上): XXX
  - 主 23:44 O-3 (干到底): XXX
  - 主 00:56 O-4 (任何人都能接手): XXX

⚠️ 限制条件
  - XXX 已知问题 / 待解决
  - XXX 升级路径 (Phase 2+)
```

---

## 4. 附录维护原则

- ❌ **不重复**阶段 2 决策文件中的内容 (用引用而非复制)
- ✅ **聚焦"真实代码 + 真实 URL + 论证"** 三个维度
- ✅ **每次阶段 2 决策变更** 同步更新此附录
- ✅ **新决策加新章节** (避免破坏既有结构)

---

## 5. 进度

```
[✓] sled - 已抓 (1.1)
[✓] tokio - 已抓 (1.2)
[✓] tantivy - 已抓 (1.3)
[✓] OpenClaw README - 已抓 (1.4)
[ ] async-openai - 待抓
[ ] qdrant-client - 待抓
[ ] Hermes Agent - 待抓
[ ] Erlang/OTP Cutover - 待抓
[ ] VCP 浪潮 - 待抓 (自研, 看 VCP ToolBox)
[ ] OpenClaw Gateway 源码 - 待抓
[ ] wasmtime - 待抓
[ ] libp2p - 待抓
```

---

_主哲学 anchor 6 个全贯穿. 真实细节附录已沉淀. 下一步: 补 §2 待抓取项, 让附录完整覆盖 14 个决策._