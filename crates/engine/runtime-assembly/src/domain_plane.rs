//! 用户域数据平面 (W3-① 多户数据隔离·星群数据层): 三面按域分库 + 跨域拒绝即帧。
//!
//! 星群架构 (**星群为骨、共享为翼**) 的数据层落地: 每户一域, 每域自己的
//! **会话库 / 记忆库 / 配额账本** (配额账本随会话库分域, 见
//! [`crate::sqlite_session::SqliteSessionStore`] 的会话 JSON 行):
//!
//! ```text
//! <数据根>/users/<id>/data/sessions.sqlite3   会话 + 三维配额账本 (per-session)
//! <数据根>/users/<id>/data/cognitive.sqlite3  记忆库 (episodes + 六流 + 侧库)
//! ```
//!
//! 默认用户域 = 数据根**原位** (单户兼容零迁移, 路径由组合根注入 ——
//! `APEIRETH_SESSION_DB` / `APEIRETH_COGNITIVE_DB` 的既有语义逐字不变)。
//!
//! ## 隔离原语 (拒绝即帧, 语义钉死)
//!
//! - **本域句柄** ([`DomainDataPlane::handles`]): 一切会话/记忆/配额读写只经
//!   本域句柄; 分库之下他域资源按本域句柄查**读作不存在**
//!   ([`apeireth_core::user_domain::UserDomainError::NotFound`] 语义, 与真
//!   不存在同帧);
//! - **点名他域** ([`DomainDataPlane::handles_for`]): 先过域级授权
//!   ([`apeireth_core::user_domain::UserDomain::authorize`]) —— 跨域一律
//!   [`apeireth_core::user_domain::UserDomainError::Forbidden`] (目标存在与否
//!   同帧, 不泄露他域存在);
//! - **坏域配置拒开**: 域文档坏 = 该域句柄拒绝打开, 其余域照常服务。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_core::user_domain::{
    DomainOp, DomainRegistry, DomainUserId, UserDomain, UserDomainError,
};
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::backend::MemoryBackend;
use apeireth_memory::{run_migrations_on_pool, SqliteMemoryStore};
use apeireth_runtime::canonical::{DomainSessionPlane, SessionManager};
use apeireth_storage::SqliteConnectionPool;

use crate::sqlite_session::SqliteSessionStore;

/// 默认用户域数据面 (原位注入)。
///
/// 会话写闸必须是运行时那一把 (同一 [`SessionManager`] 实例, 不是同一存储的
/// 两把闸) —— W2-③「同会话账本写串行」承诺跨域面不拆。
pub struct DefaultDomainStores {
    /// 默认域会话写闸 (与运行时共用同一实例)。
    pub sessions: Arc<SessionManager>,
    /// 默认域记忆库账本 (计数/流读取面)。
    pub memory: Arc<SqliteMemoryStore>,
    /// 默认域记忆库读写面 (情节读写, 生产 `MemoryBackend`)。
    pub memory_backend: Arc<dyn MemoryBackend>,
    /// 默认域会话库路径 (计数/配额探测口同源)。
    pub sessions_db: PathBuf,
    /// 默认域记忆库路径 (W3-③ 伙伴运行时记忆面接线同源)。
    pub memory_db: PathBuf,
}

/// 一个域的记忆库双面: 账本 (计数) + 后端 (情节读写) + 库路径 (接线同源)。
pub struct DomainMemoryStores {
    /// 记忆库账本 (`ledger_counts` 计数面)。
    pub store: Arc<SqliteMemoryStore>,
    /// 记忆库情节读写面。
    pub backend: Arc<dyn MemoryBackend>,
    /// 记忆库路径 (`users/<id>/data/cognitive.sqlite3`; 默认域 = 原位路径)。
    pub path: PathBuf,
}

/// 一个域的三面句柄 (会话/记忆/配额) —— 只经授权取得, 之后只读写本域。
pub struct DomainHandles<'a> {
    /// 本域会话写闸 (每域恰一把)。
    pub sessions: &'a Arc<SessionManager>,
    /// 本域记忆库。
    pub memory: &'a DomainMemoryStores,
    /// 本域会话库路径 (三维配额账本所在库)。
    pub sessions_db: &'a Path,
}

impl std::fmt::Debug for DomainHandles<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainHandles")
            .field("sessions_db", &self.sessions_db)
            .finish_non_exhaustive()
    }
}

/// 域内计数 (自述域节的真值来源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainDataCounts {
    /// 域内会话数。
    pub sessions: u64,
    /// 域内记忆数。
    pub memories: u64,
    /// 域内受保护记忆数。
    pub protected: u64,
}

/// 会话库行数 (同步读, 计数探测口用): 库文件不存在 = 0 会话 (如实)。
pub fn session_row_count(sessions_db: &Path) -> Result<u64, String> {
    if !sessions_db.exists() {
        return Ok(0);
    }
    let connection = rusqlite::Connection::open_with_flags(
        sessions_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("session store open failed: {error}"))?;
    connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .map_err(|error| format!("session count read failed: {error}"))
}

/// 用户域数据平面: 三面按域分库 + 域级授权 + 计数。
pub struct DomainDataPlane {
    sessions_plane: Arc<DomainSessionPlane>,
    memory: BTreeMap<DomainUserId, DomainMemoryStores>,
    sessions_db: BTreeMap<DomainUserId, PathBuf>,
}

impl std::fmt::Debug for DomainDataPlane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainDataPlane")
            .field("domains", &self.sessions_db.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl DomainDataPlane {
    /// 打开数据平面 (每域三面分库):
    ///
    /// - 默认域 = 注入的原位数据面 (零迁移, 与运行时共用写闸);
    /// - 其余域 = `users/<id>/data/` 下各自独立库文件 (分库隔离);
    /// - 域配置坏 / 库打不开 = **该域拒开** (不入平面, 访问时 `Refused`),
    ///   其余域照常 —— 一户坏配置不砖化整个数据面。
    pub async fn open(
        registry: DomainRegistry,
        defaults: DefaultDomainStores,
    ) -> Result<Self, UserDomainError> {
        let mut sessions: BTreeMap<DomainUserId, Arc<SessionManager>> = BTreeMap::new();
        let mut memory = BTreeMap::new();
        let mut sessions_db = BTreeMap::new();

        let default_domain = registry.load_default()?;
        sessions.insert(default_domain.user_id.clone(), defaults.sessions);
        memory.insert(
            default_domain.user_id.clone(),
            DomainMemoryStores {
                store: defaults.memory,
                backend: defaults.memory_backend,
                path: defaults.memory_db,
            },
        );
        sessions_db.insert(default_domain.user_id.clone(), defaults.sessions_db);

        for user_id in registry.domain_ids()? {
            if user_id.as_str() == apeireth_core::user_domain::DEFAULT_DOMAIN_ID {
                continue;
            }
            // 坏域配置拒开: 该域不入平面 (访问时 Refused), 不影响其余域。
            let Ok(domain) = registry.load(&user_id) else {
                continue;
            };
            let data_dir = registry.layout().data_dir(&user_id);
            if std::fs::create_dir_all(&data_dir).is_err() {
                continue;
            }
            let Ok(domain_memory) = open_domain_memory(&data_dir.join("cognitive.sqlite3")).await
            else {
                continue;
            };
            let domain_sessions_db = data_dir.join("sessions.sqlite3");
            let Ok(store) = SqliteSessionStore::open(&domain_sessions_db).await else {
                continue;
            };
            let manager = Arc::new(SessionManager::new(
                Arc::new(store),
                apeireth_core::kernel::system_clock(),
            ));
            sessions.insert(domain.user_id.clone(), manager);
            memory.insert(domain.user_id.clone(), domain_memory);
            sessions_db.insert(domain.user_id.clone(), domain_sessions_db);
        }

        Ok(Self {
            sessions_plane: Arc::new(DomainSessionPlane::new(registry, sessions)),
            memory,
            sessions_db,
        })
    }

    /// 会话平面 (网关域上下文路由注入面)。
    pub fn sessions_plane(&self) -> &Arc<DomainSessionPlane> {
        &self.sessions_plane
    }

    /// 域解析 (语义见 [`DomainSessionPlane::resolve`])。
    pub fn resolve(&self, bound_user: Option<&str>) -> Result<UserDomain, UserDomainError> {
        self.sessions_plane.resolve(bound_user)
    }

    /// 本域三面句柄 (常规读写入口; 域未开平面 → [`UserDomainError::Refused`])。
    pub fn handles(&self, domain: &UserDomain) -> Result<DomainHandles<'_>, UserDomainError> {
        self.handles_for(domain, &domain.user_id)
    }

    /// 点名取域句柄: 先过域级授权 ([`UserDomain::authorize`], 跨域拒绝即帧),
    /// 再给该域三面句柄。
    pub fn handles_for<'a>(
        &'a self,
        caller: &UserDomain,
        target: &DomainUserId,
    ) -> Result<DomainHandles<'a>, UserDomainError> {
        caller.authorize(target, DomainOp::Data)?;
        let sessions = self.sessions_plane.sessions(caller)?;
        // 授权已钉死 target == caller.user_id; 句柄仍只按 target 查表 (不旁路)。
        let memory = self.memory.get(target).ok_or_else(|| {
            UserDomainError::Refused(format!("user domain `{target}` has no open memory plane"))
        })?;
        let sessions_db = self.sessions_db.get(target).ok_or_else(|| {
            UserDomainError::Refused(format!("user domain `{target}` has no open session plane"))
        })?;
        Ok(DomainHandles {
            sessions,
            memory,
            sessions_db,
        })
    }

    /// 域内计数 (会话/记忆/受保护记忆, 真值)。
    pub fn counts(&self, domain: &UserDomain) -> Result<DomainDataCounts, UserDomainError> {
        let handles = self.handles(domain)?;
        let sessions = session_row_count(handles.sessions_db)
            .map_err(|error| UserDomainError::Refused(error))?;
        let ledger = handles
            .memory
            .store
            .ledger_counts()
            .map_err(|error| UserDomainError::Refused(error.to_string()))?;
        Ok(DomainDataCounts {
            sessions,
            memories: ledger.memories,
            protected: ledger.protected,
        })
    }
}

/// 打开一个域的记忆库 (账本 + 情节后端, 同一 `cognitive.sqlite3`)。
async fn open_domain_memory(path: &Path) -> Result<DomainMemoryStores, UserDomainError> {
    let pool = Arc::new(
        SqliteConnectionPool::open(path)
            .await
            .map_err(|error| UserDomainError::Io(format!("memory store open failed: {error}")))?,
    );
    run_migrations_on_pool(&pool)
        .await
        .map_err(|error| UserDomainError::Io(format!("memory migrations failed: {error}")))?;
    let backend: Arc<dyn MemoryBackend> = Arc::new(SqliteBackend::from_arc(Arc::clone(&pool)));
    let store = Arc::new(
        SqliteMemoryStore::open(path)
            .map_err(|error| UserDomainError::Io(format!("memory ledger open failed: {error}")))?,
    );
    Ok(DomainMemoryStores {
        store,
        backend,
        path: path.to_path_buf(),
    })
}
