//! 读前观测门禁 (`ObservedGate`): 「未读不得覆盖写」会话期护栏 + 版本 CAS 双钥匙.
//!
//! # 问题
//! 模型对文件盲写 —— 没有读过当前内容就整文件覆盖 —— 会静默毁掉外部
//! (用户 / 其它进程) 的修改。本模块用纯事件门禁拦住这类写入。
//!
//! # 事件模型 (零额外 IO)
//! - **读事件记录观测**: 读取成功时把版本 [`FileVersion`] = (len, mtime) 记入
//!   三态表 (未观测 / 已观测(版本) / 已观测为不存在)。版本来自读取路径本就
//!   取得的元数据 (挂在已开句柄上的 fstat), 不为观测新增 IO。
//! - **写事件过门禁**: 覆盖类写入要求先观测; 写入携带版本 CAS 双钥匙
//!   [`WriteRequest::replace_if_version`] (与观测版本不符 = 外部改动, 拒绝)
//!   与 [`WriteRequest::create_if_absent`] (目标出现, 拒绝)。
//! - **门禁判定是纯函数**: 版本全部以事件负载传入 ([`WriteIntent::current`] 由
//!   写入端探查后填入), 门禁本体从不访问文件系统。写入端的版本探查挂在写入
//!   流程自己身上 ([`VersionProbe`], 写前 / 写后各一次), 不是门禁的 IO。
//!
//! # 会话期护栏, 不是持久锁
//! 观测状态只存在进程内 [`ObservedGate`] 的内存映射里, **永不持久化**:
//! 进程恢复 / 重启即重置为全未观测 (见 [`ObservedGate::reset`])。它拦的是
//! 「没读过就写」与「读到的版本已被外部改动」; 判定与真正落盘之间仍留有
//! 微小竞态窗口, 由 `storage_atomic` 原子替换的完整性档/持久档收窄 ——
//! 这是护栏, 不是跨进程互斥锁。
//!
//! # 排除清单
//! [`ExclusionList`] 可配: 命中路径完全透明 (不记录、不判定), 典型用法是
//! 追加式日志类写入 (如 `*.log` / `logs/*`) —— 追加写不需要先读整个文件。

use std::collections::HashMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::storage_atomic;

/// 文件版本指纹: (len, mtime)。读取时随读事件记录, 写入时作 CAS 比对钥匙。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileVersion {
    /// 文件字节数。
    pub len: u64,
    /// 修改时间 (mtime)。
    pub mtime: SystemTime,
}

impl FileVersion {
    /// 从已经取得的文件元数据构造版本 —— 复用读/写路径手里的元数据, 零额外 IO。
    pub fn from_metadata(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            mtime: meta.modified().unwrap_or(UNIX_EPOCH),
        }
    }
}

impl fmt::Display for FileVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (secs, nanos) = match self.mtime.duration_since(UNIX_EPOCH) {
            Ok(d) => (d.as_secs() as i64, d.subsec_nanos()),
            Err(e) => (
                -(e.duration().as_secs() as i64),
                e.duration().subsec_nanos(),
            ),
        };
        write!(f, "len={} mtime={secs}.{nanos:09}", self.len)
    }
}

/// 观测三态之一 (映射里只存后两态; 查不到键 = 未观测)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationState {
    /// 未观测: 本进程尚未读过该路径。
    Unobserved,
    /// 已观测为存在, 记录读取时版本。
    Present(FileVersion),
    /// 已观测为不存在 (读取时目标缺失 / 删除后回填)。
    Missing,
}

/// 写入分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    /// 覆盖/替换类: 目标已存在并被改写或删除。
    Replace,
    /// 创建类: 目标应当不存在。
    Create,
}

/// 一次写入意图的完整判定输入 —— 纯数据, 不含任何 IO。
#[derive(Debug, Clone, Copy)]
pub struct WriteIntent<'p> {
    /// 目标路径。
    pub path: &'p Path,
    /// 写入分类。
    pub kind: WriteKind,
    /// 版本 CAS 钥匙一: 写入方声明「只替换我观测到的这个版本」。
    pub replace_if_version: Option<FileVersion>,
    /// 版本 CAS 钥匙二: 写入方声明「目标不存在才创建」。
    pub create_if_absent: bool,
    /// 写入端探查到的当前版本 (None = 目标此刻不存在)。
    pub current: Option<FileVersion>,
}

/// 门禁拒绝原因; 错误消息面向模型, 教它正确的下一步。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDenial {
    /// 覆盖类写入前从未读取观测 (盲写)。
    NotObserved(PathBuf),
    /// 目标被观测为不存在, 不是可覆盖对象 (该走创建类写入)。
    ObservedMissing(PathBuf),
    /// 版本 CAS 钥匙不符 —— 外部改动或钥匙陈旧。
    VersionMismatch {
        /// 目标路径。
        path: PathBuf,
        /// 写入方声明 / 观测记录的版本。
        keyed: FileVersion,
        /// 写入端探查到的现状。
        found: Option<FileVersion>,
    },
    /// `createIfAbsent` 钥匙要求目标缺席, 但目标已出现。
    TargetAppeared(PathBuf),
    /// 创建类写入的目标已存在, 且未观测为不存在 (创建会毁掉现有内容)。
    CreateTargetExists(PathBuf),
}

impl GateDenial {
    /// 面向模型的拒绝说明 (教它先读 / 先重观测)。
    pub fn message(&self) -> String {
        match self {
            Self::NotObserved(path) => format!(
                "拒绝覆盖写 {}: 该路径从未被读取观测, 盲写会毁掉外部修改;\
                 先用文件读工具读取当前内容, 再重试写入",
                path.display()
            ),
            Self::ObservedMissing(path) => format!(
                "拒绝覆盖写 {}: 该路径被观测为不存在; 若确要新建, 请改用创建类写入",
                path.display()
            ),
            Self::VersionMismatch { path, keyed, found } => format!(
                "拒绝写入 {}: 版本钥匙与现状不符 (检测到外部改动?), 钥匙 {keyed},\
                 现状 {}; 先重新读取该文件, 再携带新观测版本写入",
                path.display(),
                match found {
                    Some(v) => v.to_string(),
                    None => "不存在".to_string(),
                }
            ),
            Self::TargetAppeared(path) => format!(
                "拒绝创建 {}: createIfAbsent 钥匙要求目标缺席, 但目标已出现;\
                 先读取观测它, 再决定覆盖或换名",
                path.display()
            ),
            Self::CreateTargetExists(path) => format!(
                "拒绝创建 {}: 目标已存在且未观测为不存在, 直接创建会毁掉现有内容;\
                 先读取观测 (或改用覆盖类写入并携带版本钥匙)",
                path.display()
            ),
        }
    }
}

/// 门禁排除清单: 命中路径完全透明 (不记录、不判定)。
///
/// 典型用法: 追加式日志类写入豁免 (如 `*.log` / `logs/*`) —— 追加写不需要
/// 先读整个文件。模式支持最小通配: `*` 匹配任意片段, 其余逐字面。
#[derive(Debug, Clone, Default)]
pub struct ExclusionList {
    patterns: Vec<String>,
}

impl ExclusionList {
    /// 按模式构造排除清单。
    pub fn new<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            patterns: patterns.into_iter().map(Into::into).collect(),
        }
    }

    /// 清单是否为空。
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// 路径是否命中排除清单 (按归一化键比较)。
    pub fn matches(&self, path: &Path) -> bool {
        if self.patterns.is_empty() {
            return false;
        }
        let key = path_key(path);
        self.patterns
            .iter()
            .any(|pattern| pattern_matches(pattern, &key))
    }
}

/// 最小通配匹配: `*` 匹配任意片段, 其余逐字面 (模式与键都按 `/` 归一化)。
fn pattern_matches(pattern: &str, key: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    let segments: Vec<&str> = pattern.split('*').collect();
    if segments.len() == 1 {
        return key == pattern;
    }
    let last = segments.len() - 1;
    let mut pos = 0usize;
    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            continue;
        }
        if i == 0 {
            if !key[pos..].starts_with(segment) {
                return false;
            }
            pos += segment.len();
        } else if i == last {
            if !key[pos..].ends_with(segment) {
                return false;
            }
        } else {
            match key[pos..].find(segment) {
                Some(at) => pos += at + segment.len(),
                None => return false,
            }
        }
    }
    true
}

/// 归一化路径键: 统一 `/` 分隔、去 `.` 组件、折叠 `..`、剥 Windows verbatim
/// 前缀 (canonicalize 的 `\\?\` 拼写与裸拼写落到同一观测)。
///
/// 这是词法归一, 不展开 symlink、不折叠大小写; 同一文件的 symlink 拼写是
/// 两个键 (写入端各自解析后仍各自过门禁, 不会漏拒)。
fn path_key(path: &Path) -> String {
    let mut joined = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                joined.pop();
            }
            other => joined.push(other.as_os_str()),
        }
    }
    let key = joined.to_string_lossy().replace('\\', "/");
    if let Some(rest) = key.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = key.strip_prefix("//?/") {
        rest.to_string()
    } else {
        key
    }
}

/// 读前观测门禁: 进程内三态观测表 + 可配排除清单。
///
/// 全部判定方法是纯函数 (内存映射查询), 不做任何文件系统 IO。
#[derive(Debug, Default)]
pub struct ObservedGate {
    observations: Mutex<HashMap<String, ObservationState>>,
    exclusions: ExclusionList,
}

impl ObservedGate {
    /// 空门禁 (全未观测, 无排除)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 带排除清单的门禁 (命中清单的路径完全透明)。
    pub fn with_exclusions(exclusions: ExclusionList) -> Self {
        Self {
            observations: Mutex::new(HashMap::new()),
            exclusions,
        }
    }

    /// 排除清单引用。
    pub fn exclusions(&self) -> &ExclusionList {
        &self.exclusions
    }

    /// 读事件: 记录「已观测为存在 + 读取时版本」。
    pub fn observe_present(&self, path: &Path, version: FileVersion) {
        if self.exclusions.matches(path) {
            return;
        }
        self.map()
            .insert(path_key(path), ObservationState::Present(version));
    }

    /// 读事件: 记录「已观测为不存在」(读取时目标缺失 / 删除后回填)。
    pub fn observe_missing(&self, path: &Path) {
        if self.exclusions.matches(path) {
            return;
        }
        self.map().insert(path_key(path), ObservationState::Missing);
    }

    /// 查询观测三态。
    pub fn state(&self, path: &Path) -> ObservationState {
        if self.exclusions.matches(path) {
            return ObservationState::Unobserved;
        }
        self.map()
            .get(&path_key(path))
            .copied()
            .unwrap_or(ObservationState::Unobserved)
    }

    /// 观测到的版本 (仅「已观测为存在」有)。
    pub fn observed_version(&self, path: &Path) -> Option<FileVersion> {
        match self.state(path) {
            ObservationState::Present(version) => Some(version),
            _ => None,
        }
    }

    /// 纯判定: 写入意图能否放行。
    ///
    /// 覆盖类 ([`WriteKind::Replace`], 含删除):
    /// 1. 排除清单命中 → 放行;
    /// 2. 未观测 → [`GateDenial::NotObserved`] (未读不得覆盖写);
    /// 3. 观测为不存在 → [`GateDenial::ObservedMissing`] (那是创建场景);
    /// 4. 携带 `replace_if_version` 时必须等于观测版本 (钥匙陈旧/伪造即拒),
    ///    且写入端探查的现状必须等于观测版本 (不等 = 外部改动)。
    ///
    /// 创建类 ([`WriteKind::Create`]):
    /// 1. 排除清单命中 → 放行;
    /// 2. `create_if_absent` 钥匙 + 目标已出现 → [`GateDenial::TargetAppeared`];
    /// 3. 无「已观测为不存在」证据且目标已存在 → [`GateDenial::CreateTargetExists`]。
    pub fn check_write(&self, intent: &WriteIntent) -> Result<(), GateDenial> {
        if self.exclusions.matches(intent.path) {
            return Ok(());
        }
        let state = self.state(intent.path);
        match intent.kind {
            WriteKind::Replace => match state {
                ObservationState::Unobserved => {
                    Err(GateDenial::NotObserved(intent.path.to_path_buf()))
                }
                ObservationState::Missing => {
                    Err(GateDenial::ObservedMissing(intent.path.to_path_buf()))
                }
                ObservationState::Present(observed) => {
                    if let Some(keyed) = intent.replace_if_version {
                        if keyed != observed {
                            return Err(GateDenial::VersionMismatch {
                                path: intent.path.to_path_buf(),
                                keyed,
                                found: intent.current,
                            });
                        }
                    }
                    if intent.current != Some(observed) {
                        return Err(GateDenial::VersionMismatch {
                            path: intent.path.to_path_buf(),
                            keyed: observed,
                            found: intent.current,
                        });
                    }
                    Ok(())
                }
            },
            WriteKind::Create => {
                if intent.create_if_absent && intent.current.is_some() {
                    return Err(GateDenial::TargetAppeared(intent.path.to_path_buf()));
                }
                if state != ObservationState::Missing && intent.current.is_some() {
                    return Err(GateDenial::CreateTargetExists(intent.path.to_path_buf()));
                }
                Ok(())
            }
        }
    }

    /// 观测重置: 清空全部观测 (进程恢复的等价语义 —— 恢复即重置)。
    pub fn reset(&self) {
        self.map().clear();
    }

    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, ObservationState>> {
        self.observations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 版本探查最小接口。
///
/// 门禁本体从不调用它; 只有写入流程持用 (写前取 CAS 输入, 写后回填观测)。
/// 测试以假实现计数, 证明门禁判定零文件系统 IO。
pub trait VersionProbe {
    /// 目标的当前版本 (None = 不存在)。
    fn version(&self, path: &Path) -> Option<FileVersion>;
}

/// 生产版本探查: `std::fs::metadata`。
#[derive(Debug, Clone, Copy, Default)]
pub struct FsVersionProbe;

impl VersionProbe for FsVersionProbe {
    fn version(&self, path: &Path) -> Option<FileVersion> {
        std::fs::metadata(path)
            .ok()
            .map(|meta| FileVersion::from_metadata(&meta))
    }
}

/// 写入请求: 分类 + 版本 CAS 双钥匙 + 目标。
#[derive(Debug, Clone)]
pub struct WriteRequest {
    /// 目标路径。
    pub path: PathBuf,
    /// 写入分类。
    pub kind: WriteKind,
    /// 钥匙一: 只替换该版本 (与观测版本 / 磁盘现状同时比对)。
    pub replace_if_version: Option<FileVersion>,
    /// 钥匙二: 目标出现即拒绝创建。
    pub create_if_absent: bool,
}

/// 门禁写入失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatedWriteError {
    /// 门禁拒绝。
    Denied(GateDenial),
    /// 落盘 IO 失败 (来自 `storage_atomic`)。
    Io(String),
}

impl GatedWriteError {
    /// 对外错误说明。
    pub fn message(&self) -> String {
        match self {
            Self::Denied(denial) => denial.message(),
            Self::Io(detail) => format!("原子写入失败: {detail}"),
        }
    }
}

/// 过门禁的原子写入 (完整性档): 纯判定 + `storage_atomic` 原子替换。
///
/// 流程: 写前版本探查 (CAS 输入) → 门禁纯判定 → [`storage_atomic::write_atomic`]
/// → 写后版本回填观测 (同会话连写无需重读)。排除清单命中时全程透明
/// (零探查、零判定、零回填)。
pub fn gated_write_atomic(
    gate: &ObservedGate,
    probe: &impl VersionProbe,
    request: &WriteRequest,
    bytes: &[u8],
    mode: u32,
) -> Result<(), GatedWriteError> {
    gated_write(gate, probe, request, bytes, mode, false)
}

/// 过门禁的原子写入 (持久档): 同 [`gated_write_atomic`], 落盘走
/// [`storage_atomic::write_atomic_durable`] (替换前 fsync)。
pub fn gated_write_atomic_durable(
    gate: &ObservedGate,
    probe: &impl VersionProbe,
    request: &WriteRequest,
    bytes: &[u8],
    mode: u32,
) -> Result<(), GatedWriteError> {
    gated_write(gate, probe, request, bytes, mode, true)
}

fn gated_write(
    gate: &ObservedGate,
    probe: &impl VersionProbe,
    request: &WriteRequest,
    bytes: &[u8],
    mode: u32,
    durable: bool,
) -> Result<(), GatedWriteError> {
    if gate.exclusions().matches(&request.path) {
        // 排除清单透明: 不探查、不判定、不回填, 直接落盘。
        return write_tier(&request.path, bytes, mode, durable);
    }

    let current = probe.version(&request.path);
    let intent = WriteIntent {
        path: &request.path,
        kind: request.kind,
        replace_if_version: request.replace_if_version,
        create_if_absent: request.create_if_absent,
        current,
    };
    gate.check_write(&intent).map_err(GatedWriteError::Denied)?;
    write_tier(&request.path, bytes, mode, durable)?;

    // 写后回填观测: 同会话后续写凭回填版本直接过 CAS。回填探查失败只让
    // 记录停留旧版本 —— 偏向「下次写被拒、要求重读」的安全方向。
    if let Some(fresh) = probe.version(&request.path) {
        gate.observe_present(&request.path, fresh);
    }
    Ok(())
}

fn write_tier(path: &Path, bytes: &[u8], mode: u32, durable: bool) -> Result<(), GatedWriteError> {
    let result = if durable {
        storage_atomic::write_atomic_durable(path, bytes, mode)
    } else {
        storage_atomic::write_atomic(path, bytes, mode)
    };
    result.map_err(|e| GatedWriteError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn v(len: u64, tag: u64) -> FileVersion {
        FileVersion {
            len,
            mtime: UNIX_EPOCH + std::time::Duration::from_secs(tag),
        }
    }

    // ---- 八项门禁场景 ----

    #[test]
    fn replace_write_on_an_unobserved_path_is_denied() {
        // 未读不许覆盖写: 盲写被拒, 错误消息教它先读。
        let gate = ObservedGate::new();
        let path = Path::new("src/main.rs");
        let intent = WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(v(10, 1)),
            create_if_absent: false,
            current: Some(v(10, 1)),
        };
        let denial = gate.check_write(&intent).unwrap_err();
        assert!(
            matches!(denial, GateDenial::NotObserved(_)),
            "expected NotObserved, got {denial:?}"
        );
        assert!(
            denial.message().contains("读取"),
            "拒绝消息必须教它先读: {}",
            denial.message()
        );
    }

    #[test]
    fn replace_write_after_observation_with_matching_version_is_allowed() {
        // 读后可写 (版本一致): 观测版本 = 钥匙 = 现状 → 放行。
        let gate = ObservedGate::new();
        let path = Path::new("notes.txt");
        let observed = v(12, 7);
        gate.observe_present(path, observed);
        let intent = WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
            current: Some(observed),
        };
        assert!(gate.check_write(&intent).is_ok());
    }

    #[test]
    fn external_change_makes_replace_if_version_stale_and_denies_the_write() {
        // 外部改动后写被拒: 钥匙还是旧观测版本, 写入端探查到新版本。
        let gate = ObservedGate::new();
        let path = Path::new("config.toml");
        let observed = v(100, 1);
        gate.observe_present(path, observed);
        let external = v(140, 2);
        let intent = WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
            current: Some(external),
        };
        match gate.check_write(&intent) {
            Err(GateDenial::VersionMismatch { keyed, found, .. }) => {
                assert_eq!(keyed, observed);
                assert_eq!(found, Some(external));
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }

        // 钥匙陈旧 (观测被后来的重读刷新) 同样拒绝。
        gate.observe_present(path, external);
        let stale = WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
            current: Some(external),
        };
        assert!(matches!(
            gate.check_write(&stale),
            Err(GateDenial::VersionMismatch { .. })
        ));
    }

    #[test]
    fn create_if_absent_is_denied_when_the_target_has_appeared() {
        // createIfAbsent 撞已有文件被拒: 观测时不存在, 但目标已出现。
        let gate = ObservedGate::new();
        let path = Path::new("new.txt");
        gate.observe_missing(path);
        let intent = WriteIntent {
            path,
            kind: WriteKind::Create,
            replace_if_version: None,
            create_if_absent: true,
            current: Some(v(3, 9)),
        };
        assert!(
            matches!(
                gate.check_write(&intent),
                Err(GateDenial::TargetAppeared(_))
            ),
            "createIfAbsent 钥匙必须在目标出现时拒绝"
        );
    }

    #[test]
    fn observed_missing_path_allows_create() {
        // 「已观测为不存在」可创建。
        let gate = ObservedGate::new();
        let path = Path::new("fresh.txt");
        gate.observe_missing(path);
        let intent = WriteIntent {
            path,
            kind: WriteKind::Create,
            replace_if_version: None,
            create_if_absent: true,
            current: None,
        };
        assert!(gate.check_write(&intent).is_ok());
    }

    #[test]
    fn reset_forgets_observations_like_a_fresh_session() {
        // 观测重置语义: 观测状态永不持久化, 重置 (进程恢复等价) 即全未观测。
        let gate = ObservedGate::new();
        let path = Path::new("state.json");
        let observed = v(9, 1);
        gate.observe_present(path, observed);
        assert_eq!(gate.state(path), ObservationState::Present(observed));
        gate.reset();
        assert_eq!(gate.state(path), ObservationState::Unobserved);
        let intent = WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
            current: Some(observed),
        };
        assert!(matches!(
            gate.check_write(&intent),
            Err(GateDenial::NotObserved(_))
        ));
    }

    #[test]
    fn exclusion_list_paths_pass_the_gate_transparently() {
        // 排除清单透明: 追加式日志类写入豁免, 不记录也不判定。
        let gate = ObservedGate::with_exclusions(ExclusionList::new(["*.log", "logs/*"]));
        let log = Path::new("logs/app.log");
        let intent = WriteIntent {
            path: log,
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
            current: None,
        };
        assert!(gate.check_write(&intent).is_ok());
        gate.observe_present(log, v(1, 1));
        assert_eq!(gate.state(log), ObservationState::Unobserved);

        // 非排除路径的规则不受影响。
        let code = Path::new("src/main.rs");
        let blocked = WriteIntent {
            path: code,
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
            current: None,
        };
        assert!(matches!(
            gate.check_write(&blocked),
            Err(GateDenial::NotObserved(_))
        ));
    }

    #[test]
    fn gate_decisions_do_no_filesystem_io_against_a_fake_probe() {
        // 对假 fs 探针: 门禁判定是纯函数, 版本全由事件负载传入。
        let probe = CountingProbe::default();
        let gate = ObservedGate::new();
        let path = Path::new("probe.txt");
        let observed = v(4, 2);

        gate.observe_present(path, observed);
        assert_eq!(gate.state(path), ObservationState::Present(observed));
        gate.observe_missing(Path::new("probe-absent.txt"));
        gate.check_write(&WriteIntent {
            path,
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
            current: Some(observed),
        })
        .unwrap();
        gate.check_write(&WriteIntent {
            path: Path::new("probe-absent.txt"),
            kind: WriteKind::Create,
            replace_if_version: None,
            create_if_absent: true,
            current: None,
        })
        .unwrap();
        gate.reset();
        assert_eq!(
            probe.calls(),
            0,
            "门禁判定不得触碰文件系统 (假探针计数必须为 0)"
        );
    }

    // ---- 写入端 (gated_write_atomic) 集成 ----

    #[test]
    fn gated_write_refuses_blind_overwrite_and_touches_no_probe_on_denial() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.txt");
        std::fs::write(&target, "external content").unwrap();

        let probe = CountingProbe::default();
        let gate = ObservedGate::new();
        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
        };
        let err = gated_write_atomic(&gate, &probe, &request, b"clobber", 0o644).unwrap_err();
        assert!(
            matches!(err, GatedWriteError::Denied(GateDenial::NotObserved(_))),
            "盲写必须被拒, got {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "external content",
            "被拒写入不得落盘"
        );
        assert_eq!(probe.calls(), 1, "被拒时只有写前探查, 无写后回填");
    }

    #[test]
    fn gated_write_detects_external_change_on_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.txt");
        std::fs::write(&target, "v1").unwrap();
        let probe = FsVersionProbe;
        let gate = ObservedGate::new();

        let observed = probe.version(&target).expect("写前探查");
        gate.observe_present(&target, observed);
        // 外部进程改写目标。
        std::fs::write(&target, "v2-external-change").unwrap();

        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
        };
        let err = gated_write_atomic(&gate, &probe, &request, b"v3", 0o644).unwrap_err();
        assert!(
            matches!(
                err,
                GatedWriteError::Denied(GateDenial::VersionMismatch { .. })
            ),
            "外部改动后旧钥匙必须被拒, got {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "v2-external-change"
        );
    }

    #[test]
    fn gated_write_refreshes_observation_so_same_session_followups_work() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("data.txt");
        std::fs::write(&target, "v1").unwrap();
        let probe = FsVersionProbe;
        let gate = ObservedGate::new();

        let observed = probe.version(&target).expect("写前探查");
        gate.observe_present(&target, observed);
        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: Some(observed),
            create_if_absent: false,
        };
        gated_write_atomic(&gate, &probe, &request, b"v2", 0o644).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "v2");

        // 写后回填观测: 同会话再写凭回填版本直接过 CAS, 无需重读。
        let refreshed = gate.observed_version(&target).expect("写后回填观测");
        assert_eq!(refreshed.len, 2);
        let follow_up = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: Some(refreshed),
            create_if_absent: false,
        };
        gated_write_atomic(&gate, &probe, &follow_up, b"v3", 0o644).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "v3");
    }

    #[test]
    fn gated_write_creates_file_when_observed_missing_and_refuses_when_target_appears() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("new.txt");
        let probe = FsVersionProbe;
        let gate = ObservedGate::new();

        gate.observe_missing(&target);
        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Create,
            replace_if_version: None,
            create_if_absent: true,
        };
        gated_write_atomic(&gate, &probe, &request, b"created", 0o644).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "created");

        // 目标已出现: createIfAbsent 钥匙拒绝再走创建。
        let again = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Create,
            replace_if_version: None,
            create_if_absent: true,
        };
        let err = gated_write_atomic(&gate, &probe, &again, b"clobber", 0o644).unwrap_err();
        assert!(
            matches!(err, GatedWriteError::Denied(GateDenial::TargetAppeared(_))),
            "createIfAbsent 撞已有文件必须被拒, got {err:?}"
        );
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "created");
    }

    #[test]
    fn exclusion_list_makes_real_writes_transparent() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("app.log");
        std::fs::write(&target, "line 1\n").unwrap();
        let probe = CountingProbe::default();
        let gate = ObservedGate::with_exclusions(ExclusionList::new(["*.log"]));

        // 未观测的追加式日志写入: 全程透明 (零探查、零判定)。
        let request = WriteRequest {
            path: target.clone(),
            kind: WriteKind::Replace,
            replace_if_version: None,
            create_if_absent: false,
        };
        gated_write_atomic(&gate, &probe, &request, b"line 2\n", 0o644).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "line 2\n");
        assert_eq!(probe.calls(), 0, "排除清单路径必须零探查");
    }

    // ---- 假 fs 探针 ----

    /// 假 fs 探针: 计数版本探查次数 (证明门禁判定零文件系统 IO)。
    #[derive(Default)]
    struct CountingProbe {
        calls: Cell<usize>,
    }

    impl CountingProbe {
        fn calls(&self) -> usize {
            self.calls.get()
        }
    }

    impl VersionProbe for CountingProbe {
        fn version(&self, path: &Path) -> Option<FileVersion> {
            self.calls.set(self.calls.get() + 1);
            // 假文件系统: 只复述真实文件的元数据, 不代表门禁访问了它。
            FsVersionProbe.version(path)
        }
    }
}
