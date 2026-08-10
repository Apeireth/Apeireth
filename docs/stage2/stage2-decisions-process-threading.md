# 阶段 2 决策：进程/线程/协程分工 (2026-07-30)

> **范围**: R14 Rust 重写进程/线程/协程分工决策 (阶段 2 第四项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: B+E supervisor 架构 (阶段 2 §2) + 30 crate 划分 (阶段 2 §3) + 阶段 1 灵感 §6 自我升级
> **配套文档**: `stage2-decisions-architecture.md` + `stage2-decisions-crate-split.md`

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-process-threading.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 4/12) |
| **决策** | **B+E 进程级 + tokio 多线程 runtime + spawn_blocking 分离** |

---

## 1. 决策总览

```
进程级 (B+E):
  1 个 supervisor 主进程
    ├── 4 个 supervisor 子进程 (core / council / plugin / upgrade)
    └── N 个 plugin 异构子进程 (PyO3 / WASM / subprocess / HTTP)

线程级 (每个进程内):
  1 个 main thread (主循环 / actor)
  N tokio worker threads (默认 = CPU 数, 可调)
  1 个 signal handler (Unix signal)
  1 个 monitor thread (健康检查 + metric)
  1 个 PyO3 GIL thread (Python 兼容)

协程级 (tokio task):
  spawn() 用于 IO 密集 (HTTP/DB/文件/RPC)
  spawn_blocking() 用于 CPU 密集 + 阻塞调用 (加密/压缩/PyO3)
```

---

## 2. 进程级分工 (supervisor 子树)

### 2.1 主进程: `apeireth-supervisor`

```
PID 1 (主进程)
  - 唯一进程入口
  - 启动 supervisor 树
  - 处理 Unix signal (SIGTERM/SIGINT/SIGHUP)
  - 暴露 health check endpoint (HTTP 9090)
  - 提供 admin RPC (Unix domain socket)
  - 不跑任何业务逻辑 (纯调度)
  - 永不重启 (E 层 supervisor 永存)
```

### 2.2 子进程 1: `core-supervisor` (强耦合)

```
跑: sovereignty + memory + philosophy
策略: rest_for_one (任一挂了, 重启它和之后启动的)
内存: ~500MB-1GB (取决于 memory 缓存大小)
线程: tokio multi-thread, worker = CPU 数
进程间通信: inproc bus (实际是 process pipe + bincode)
```

**为什么独立进程**:
- 主 AI 是**最重要的**, 隔离保护
- memory / philosophy 强耦合, 必须一起重启
- 重启不影响其他子树

### 2.3 子进程 2: `council-supervisor` (互相独立)

```
跑: council (7 persistent + N dynamic) + reflection
策略: one_for_one (任一顾问挂了, 只重启它)
内存: ~200MB-500MB (LLM 调用开销)
线程: tokio multi-thread
特殊: 每个顾问是独立 tokio task, 不需要独立进程
```

**为什么独立进程** (而不仅是 task):
- 智囊团是**审计机构**, 需要独立性
- 主 AI 不能直接访问智囊团进程内存
- 智囊团 crash 不影响主 AI
- 智囊团可以独立升级 (换底层 LLM)

### 2.4 子进程 3-N: `plugin-supervisor` (异构, transient)

```
跑: plugin + pybridge + mcp + environment + acp
策略: transient (异常退出才重启, 正常退出不重启)
内存: 每插件 ~50-200MB
线程: 取决于插件类型
异构: PyO3 (Python) / WASM / subprocess / HTTP
```

**每个插件都是独立进程**:
- 崩溃不影响核心
- 可以热加载 / 热卸载
- 不同语言天然隔离

### 2.5 子进程 N+1: `upgrade-supervisor` (关键)

```
跑: upgrade + sandbox-validator
策略: rest_for_one
内存: ~100MB-300MB
线程: 单线程 (升级是顺序的, 不并发)
特殊: 启动 sandbox 跑测试 (spawn temp subprocess)
```

**为什么独立进程**:
- 升级是关键路径, 隔离保护
- sandbox 跑测试时不影响主基地
- 测试失败不影响核心

### 2.6 进程拓扑

```
                    ┌─────────────────────┐
                    │ apeireth-supervisor │
                    │     (PID 1)         │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌──────────────────┐    ┌────────────────┐
│ core-sup      │    │ council-sup      │    │ upgrade-sup    │
│ (PID 100+)    │    │ (PID 200+)       │    │ (PID 300+)     │
└───────────────┘    └──────────────────┘    └────────────────┘
                                                      │
                                                      ▼
                                            ┌────────────────┐
                                            │ sandbox-valid  │
                                            │ (temp PID)     │
                                            └────────────────┘

plugin-supervisor (PID 400+) 启动多个异构子进程:
  ├── python-llm-plugin (PID 401)
  ├── wasm-sandbox-1 (PID 402)
  ├── http-mcp-1 (PID 403)
  └── ...
```

---

## 3. 进程内线程分工

### 3.1 每个进程的标准线程配置

```rust
fn main() {
    // 1 个 main thread (主循环)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())  // 默认 CPU 数
        .thread_name("apeireth-worker")
        .enable_all()
        .build()
        .unwrap();
    
    runtime.block_on(async {
        // tokio worker threads (N 个, 处理 IO)
        // spawn 各种 actor task
        
        // 1 个 signal handler task
        tokio::spawn(handle_signals());
        
        // 1 个 monitor task
        tokio::spawn(monitor_loop());
        
        // 业务 task
        tokio::spawn(core_actor::run(state.clone()));
        // ...
    });
}
```

### 3.2 各进程的线程配置

| 进程 | tokio worker | 特殊线程 | 总线程数 (8 CPU) |
|------|-------------|----------|-----------------|
| supervisor | 2 | signal + health | 4 |
| core-supervisor | 8 | (无) | 8 |
| council-supervisor | 4 | (无) | 4 |
| plugin-supervisor | 2 | spawn_blocking pool (4) | 6 |
| upgrade-supervisor | 1 | sandbox (spawn_blocking) | 2 |
| python-llm-plugin | 1 | **PyO3 GIL thread** | 2 |

### 3.3 特殊线程说明

#### PyO3 GIL thread (Python 兼容)

```
Python GIL 限制: 同一进程内, 多个 OS 线程不能同时执行 Python 代码
解决: 单独一个 OS 线程持有 GIL, 其他线程通过 channel 提交 Python 调用

代码:
  let gil_thread = std::thread::spawn(move || {
      Python::with_gil(|py| {
          loop {
              let call = gil_rx.recv() blocking;
              py_func.call(...);
          }
      });
  });
```

#### spawn_blocking pool

```
用于阻塞调用 (CPU 密集 / PyO3 / 文件 IO):
  tokio::task::spawn_blocking(move || {
      // 阻塞调用
      expensive_compress(data);
  }).await

pool 大小: 默认 512, 巨型基地可调到 1024
```

---

## 4. 协程 vs 阻塞线程 (tokio task)

### 4.1 决策矩阵

| 操作类型 | 用什么 | 例子 |
|---------|--------|------|
| **IO 密集** | `tokio::spawn` (协程) | HTTP/DB/文件/RPC |
| **CPU 密集** | `spawn_blocking` (阻塞线程) | 加密/压缩/图像处理 |
| **Python 调用** | `spawn_blocking` + GIL thread | PyO3 桥调用 |
| **短延迟任务** | `tokio::spawn` | 消息分发 |
| **长阻塞任务** | `spawn_blocking` | 大文件读写 |
| **同步原语等待** | `tokio::sync::*` | Mutex/RwLock/channel |

### 4.2 actor 模型 in B+E

```rust
// 每个核心组件用 actor 模式
pub struct CoreActor {
    state: Arc<RwLock<CoreState>>,
    inbox: mpsc::Receiver<CoreMessage>,
    ctx: ActorContext,
}

impl CoreActor {
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }
    
    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.inbox.recv() => {
                    self.handle(msg).await;
                }
                _ = self.ctx.shutdown_rx() => break,
                _ = tokio::time::interval(Duration::from_secs(60)) => {
                    self.heartbeat().await;
                }
            }
        }
    }
}
```

### 4.3 tokio runtime 配置 (Cargo.toml)

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1

# 运行时配置 (通过环境变量)
runtime_threads = 8          # 默认 = CPU 数
runtime_blocking_pool = 512  # spawn_blocking 池大小
```

---

## 5. 进程间通信 (回顾 + 具体参数)

| 通信类型 | 工具 | 性能 | 用途 |
|---------|------|------|------|
| 同进程 actor | tokio::sync::mpsc | ⚡ ns | 进程内组件 |
| 父子进程 | Unix domain socket + bincode | 🚀 μs | supervisor ↔ 子进程 |
| 异构子进程 | pipe + JSON/MsgPack | 🚀 μs | Rust ↔ Python |
| 外部 HTTP | reqwest + JSON | 🐢 ms | 远程 API |

### 5.1 supervisor ↔ 子进程 (Unix domain socket)

```rust
// supervisor 启动子进程
let (tx, rx) = std::os::unix::net::UnixStream::pair()?;
let child = Command::new("apeireth-core-supervisor")
    .arg("--socket-fd").arg(rx.as_raw_fd())
    .spawn()?;

// supervisor 通过 tx 发命令
let bincode_msg = bincode::serialize(&SupervisorCommand::Start)?;
tx.write_all(&bincode_msg)?;
```

### 5.2 Rust ↔ Python (pipe + JSON)

```rust
// 启动 Python 插件
let mut child = Command::new("python")
    .arg("plugin.py")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

// 发 JSON
let msg = serde_json::to_string(&PluginCall{...})?;
child.stdin.write_all(msg.as_bytes())?;

// 收 JSON
let mut buf = String::new();
child.stdout.read_to_string(&mut buf)?;
let response: PluginResponse = serde_json::from_str(&buf)?;
```

---

## 6. 资源限制 (巨型基地哲学)

```rust
// systemd-run 或 cgroup 限制
// 每个进程的最大内存
MemoryMax=2G

// 每个进程的最大 CPU
CPUQuota=400%

// supervisor 进程最低资源
MemoryHigh=200M
CPUWeight=100
```

**为什么限制**:
- 巨型基地允许冗余, 但不允许失控
- 单个进程 OOM 不会影响其他
- 资源限制是 E-3 守门的物理实现

---

## 7. 阶段 2 第四项收尾判定

进程/线程/协程分工已沉淀。

**关键设计**:
- ✅ 进程级 4-5 个 supervisor 子树 (B+E)
- ✅ 线程级 tokio multi-thread + 特殊线程 (signal/monitor/PyO3 GIL)
- ✅ 协程级 `tokio::spawn` (IO) vs `spawn_blocking` (CPU/PyO3)
- ✅ actor 模式在 B 内使用
- ✅ 进程间通信分层 (Unix socket / pipe / HTTP)
- ✅ 资源限制 (cgroup) 是 E-3 守门的物理实现

**R14 增量**:
- 新增 `apeireth-runtime` crate 的具体设计
- 新增 `apeireth-bus` 的多 backend 实现
- PyO3 GIL thread 模式
- spawn_blocking pool 配置

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (分工服务 ASI 方向)
- 主 17:43 S-2 (基于 tokio 现状, 不重写)
- 主 17:58 O-5 (资源限制是 E-3 物理实现)
- 主 19:33 O-2 (actor/supervisor 是成熟模式)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者都能查)

**下一步**: 阶段 2 第五项 — **内存布局**

---

## 8. 决策对比表 (4 个备选)

| 范式 | 进程隔离 | 线程并发 | 协程使用 | 推荐 |
|------|---------|---------|---------|------|
| A. 单进程多线程 | ❌ | ✅ | ✅ | ❌ |
| B. 多进程 + 多线程 | ✅ | ✅ | ✅ | ✅ |
| C. 单进程 + actor | ⚠️ | ✅ | ✅ | ⚠️ |
| **B+E 多进程 + 多线程 + actor in process** | ✅ | ✅ | ✅ | ✅✅ |

**Apeireth 选 B+E** (阶段 2 §2 已定):
- 进程级 B (supervisor)
- 进程内 actor + 多线程
- 协程按需 (IO 用 spawn, CPU 用 spawn_blocking)

---

_主哲学 anchor 6 个全贯穿. 进程/线程/协程分工已沉淀. 下一步等用户确认进入阶段 2 第五项 (内存布局)._