# R212 Council deliberation checkpoint (LangGraph style)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R212
> **日期**: 2026-08-13
> **来源**: R180 council 调研 (LangGraph Checkpoint) + 主人"全做全做全补弱"
> **状态**: 实施完成, 12/12 单测全过

---

## 0. 动机

现状 (R25/R33-4-1): `Council::deliberate()` / `deliberate_persona()` 一次跑完. 7 advisor × 3 persona round = 21 步, 中途 crash / timeout / L0 HA 触发会丢失全部进度.

VCP / AutoGen / LangGraph 等 multi-agent 框架都有 checkpoint / state persistence 机制, 我们的 Council 没有 — 是个能力空缺.

R212 给 Council 加 Checkpoint 机制 (LangGraph 借鉴, 不模仿):
- `Checkpoint` struct: 序列化快照 (session_id, query snapshot, opinions_so_far, current_step)
- `CheckpointStore` trait: 抽象持久化
- `MemoryCheckpointStore`: 内存 (HashMap<RwLock<Vec>>)
- `FileCheckpointStore`: 文件 (JSONL 一行一 cp)
- 自实现 `CheckpointQuery` (CouncilQuery Serde 镜像, 0 触碰 deliberation.rs)

---

## 1. 设计

### 1.1 公共 API

```rust
pub const CHECKPOINT_VERSION: u32 = 1;

pub struct Checkpoint {
    pub version: u32,
    pub checkpoint_id: String,
    pub session_id: String,
    pub query: CheckpointQuery,  // 镜像
    pub opinions_so_far: Vec<AdvisorOpinion>,
    pub current_step: usize,
    pub total_steps: usize,
    pub elapsed_ms_so_far: u64,
    pub started_at_ms: i64,
    pub written_at_ms: i64,
}
impl Checkpoint {
    pub fn progress(&self) -> f64;          // 0.0..1.0
    pub fn is_complete(&self) -> bool;
    pub fn next_step(&self) -> usize;
}

pub struct CheckpointQuery { /* 6 字段镜像 CouncilQuery */ }
impl CheckpointQuery {
    pub fn from_council_query(q: &CouncilQuery) -> Self;
}

pub trait CheckpointStore: Send + Sync {
    fn put(&self, cp: &Checkpoint) -> CheckpointResult<()>;
    fn get(&self, session_id: &str) -> CheckpointResult<Checkpoint>;
    fn list(&self, session_id: &str) -> CheckpointResult<Vec<Checkpoint>>;
    fn delete(&self, session_id: &str) -> CheckpointResult<()>;
}

pub struct MemoryCheckpointStore { /* HashMap<String, Vec<Checkpoint>> + RwLock */ }
impl MemoryCheckpointStore { pub fn new(); pub fn session_count(); pub fn total_checkpoints(); }

pub struct FileCheckpointStore { base_dir: PathBuf }
impl FileCheckpointStore { pub fn new<P: AsRef<Path>>(base_dir: P) -> CheckpointResult<Self>; }
```

### 1.2 自实现镜像 CheckpointQuery

`deliberation.rs` 里的 `CouncilQuery` / `QueryContext` 没有 derive Serialize/Deserialize. 0 触碰原则下, 我们在 checkpoint.rs 自实现 `CheckpointQuery` (6 字段) + `from_council_query()` 转换.

### 1.3 CheckpointStore trait 设计

- `put`: 写入, version 校验
- `get`: 取 latest (最后一个)
- `list`: 取全部 (时间顺序)
- `delete`: 删整个 session

### 1.4 内存实现

`HashMap<session_id, Vec<Checkpoint>>` + `RwLock`. 多 writer 安全.

### 1.5 文件实现

每个 session 一个 JSONL 文件 (append-only), 一行一 checkpoint. session_id 做安全化 (`[a-zA-Z0-9_-]` 保留, 其它换 `_`) 防路径穿越.

### 1.6 与 Council 集成 (R213 后续)

R212 只做 CheckpointStore + 数据结构. 真正集成 `Council::deliberate_with_checkpoints()` / `Council::resume_from_checkpoint()` 留给 R213+ (需要 mutate deliberation.rs 流程).

---

## 2. 测试覆盖 (12 cases)

| ID | 用例 | 覆盖点 |
|---|---|---|
| t01 | checkpoint_progress | progress() 计算 |
| t02 | checkpoint_complete | is_complete() 边界 |
| t03 | next_step | 当前 step getter |
| t04 | memory_store_put_get | 基础 put+get |
| t05 | memory_store_list | 列出全部 |
| t06 | memory_store_get_latest | get 取最后 |
| t07 | memory_store_delete | 删 session |
| t08 | memory_store_session_count | 计数 |
| t09 | memory_store_version_mismatch | version 校验 |
| t10 | file_store_put_get | 文件 put+get |
| t11 | file_store_list_order | 文件读取顺序 |
| t12 | file_store_sanitize_session_id | 路径穿越防护 |

---

## 3. 0 触碰守门

- `deliberation.rs` 0 改 (未碰 CouncilQuery / QueryContext)
- `advisor.rs` 0 改 (复用 AdvisorOpinion 已有 Serialize/Deserialize)
- 7 强制 advisor 0 改
- 3 不可变脊柱 0 触碰
- workspace.version 1.2.0 0 改
- 0 新增 Cargo.toml 依赖 (serde / serde_json / thiserror 已存在)

---

## 4. 路线意义

R212 完成后, Council 战区有:
- 5 状态机 + 7 advisor + 5 synthesis 权重
- 21+ 源文件 (lib + 7 advisor 子模块 + ...)
- Checkpoint 持久化 + 2 实现 (memory + file)
- LangGraph 风格 resume 能力的数据基础

R213: 集成到 Council::deliberate() (加 save_checkpoint 调用), 实现 resume API.

---

## 5. 下一步

- **R213** Council::deliberate_with_checkpoints() + resume API + tool-codesearch 真 LRU
- **R217** Kani 1 proof 演示 (2-3 hours)
- **R149 followup** tool-fetch 加 streaming / 增量更新
