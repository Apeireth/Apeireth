//! 冷启动/待机基准共用的生产组装路径: 从零组装到 Runtime ready。
//!
//! 组装内容 (与 `cold-start` / `idle-footprint` 两个基准严格同口径):
//! 1. SQLite 记忆存储 (rusqlite bundled, 含迁移) 打开;
//! 2. SQLite 会话存储 (连接池 + 迁移) 打开;
//! 3. `ProductionModules::build` 组装生产模块集 (记忆召回 + 记忆写回 +
//!    文件/搜索/仓库工具模块, 注入记忆/治理/作用域记忆后端与工作区根);
//! 4. `register_into(Runtime::builder())` 注册模块与能力;
//! 5. `build().await` 到 Runtime ready (快照可产出 = 就绪校验)。
//!
//! 不含 (如实标注): Judge/Council/Organ/Shell/Fetch/MCP 等显式 opt-in 模块
//! (默认关闭)、偏好/自评后端 (未注入则对应模块不注册)、异步运行时创建
//! (tokio 运行时由基准提前建好, 不计入冷启动区间)。

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use apeireth_core::kernel::system_clock;
use apeireth_memory::{MemoryGovernanceStore, ScopedMemoryBackend, SqliteMemoryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::{Runtime, SessionStore};
use apeireth_runtime_assembly::{
    ProductionBackends, ProductionModules, ProductionModulesConfig, SqliteSessionStore,
};

/// 启动期分段计时 (剖析报告数据源; 固定 4 个计时点, 计时器开销 < 1 µs)。
#[derive(Debug, Clone, Copy, Default)]
pub struct StartupSegments {
    /// SQLite 记忆存储打开 (含记忆 schema 迁移), 纳秒。
    pub memory_open_ns: f64,
    /// SQLite 会话存储打开 (连接池建立 + 存储迁移), 纳秒。
    pub session_open_ns: f64,
    /// `ProductionModules::build` 模块集构造, 纳秒。
    pub modules_build_ns: f64,
    /// `register_into` + `build().await` 到 ready, 纳秒。
    pub runtime_build_ns: f64,
}

impl StartupSegments {
    /// 全程合计 (纳秒)。
    pub fn total_ns(&self) -> f64 {
        self.memory_open_ns + self.session_open_ns + self.modules_build_ns + self.runtime_build_ns
    }
}

/// 一次性完整组装: 存储打开 → 模块组装 → Runtime build 到 ready。
///
/// 返回的 [`Runtime`] 由调用方决定销毁时机 (销毁不计入冷启动计时)。
pub async fn assemble_production(data_dir: &Path) -> Runtime {
    assemble_production_profiled(data_dir).await.0
}

/// 与 [`assemble_production`] 完全同口径 (同一实现), 额外返回分段计时。
///
/// 分段口径: `memory_open` / `session_open` / `modules_build` / `runtime_build`
/// 四段首尾相接, 合计 ≈ 冷启动全程 (段间仅剩纳秒级计时器调用)。
pub async fn assemble_production_profiled(data_dir: &Path) -> (Runtime, StartupSegments) {
    let mut seg = StartupSegments::default();

    let tick = Instant::now();
    let memory =
        Arc::new(SqliteMemoryStore::open(data_dir.join("memory.sqlite3")).expect("打开记忆存储"));
    seg.memory_open_ns = tick.elapsed().as_nanos() as f64;

    let backend = memory.clone() as Arc<dyn MemoryBackend>;
    let governance = memory.clone() as Arc<dyn MemoryGovernanceStore>;
    let scoped = memory.clone() as Arc<dyn ScopedMemoryBackend>;

    let tick = Instant::now();
    let sessions = SqliteSessionStore::open(data_dir.join("session.sqlite3"))
        .await
        .expect("打开会话存储");
    seg.session_open_ns = tick.elapsed().as_nanos() as f64;

    let config = ProductionModulesConfig {
        memory_recall: true,
        memory_writeback: true,
        preference_recall: false,
        self_assessment: false,
        filesystem: true,
        search: true,
        repo: true,
        ..ProductionModulesConfig::default()
    };
    let backends = ProductionBackends {
        memory: Some(backend),
        memory_governance: Some(governance),
        scoped_memory: Some(scoped),
        workspace_root: Some(data_dir.to_path_buf()),
        ..ProductionBackends::default()
    };

    let tick = Instant::now();
    let modules = ProductionModules::build(config, backends, system_clock()).expect("组装模块集");
    seg.modules_build_ns = tick.elapsed().as_nanos() as f64;

    let tick = Instant::now();
    let builder = modules.register_into(
        Runtime::builder().with_session_store(Arc::new(sessions) as Arc<dyn SessionStore>),
    );
    let runtime = builder.build().await.expect("Runtime build 到 ready");
    seg.runtime_build_ns = tick.elapsed().as_nanos() as f64;

    // 就绪校验: 快照可产出且组装非空 (能力 + 模块)。
    let snapshot = runtime.snapshot();
    assert!(
        !snapshot.capabilities.is_empty() && !snapshot.behavior_modules.is_empty(),
        "组装结果必须非空"
    );
    (runtime, seg)
}

/// 建一个多线程 tokio 运行时 (不计入冷启动区间; 冷启动只测组装)。
pub fn tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("构建 tokio 运行时")
}
