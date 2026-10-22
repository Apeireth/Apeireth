//! 统一原子文件写入 + 文件锁 (存储基础件).
//!
//! 本模块是仓内所有「临时文件 + 替换」写路径的唯一实现点, 语义分两档:
//!
//! - [`write_atomic`] (完整性档): 父目录创建 → 同目录以 `create_new(true)` 独占
//!   建临时文件 → 写入 → 替换。**不含 fsync**。语义 = 读者要么读到旧的完整内容,
//!   要么读到新的完整内容; **这不是崩溃持久** —— 断电/内核崩溃后新内容可能
//!   丢失或回退, 但任何时刻都不会出现半写内容。
//! - [`write_atomic_durable`] (持久档): 同上, 但写入后先 `sync_all` 落盘再替换,
//!   unix 追加父目录 fsync 保证目录项落盘。语义 = 崩溃后不回退到旧内容。
//!   平台差异 (如实标注): Windows 无父目录 fsync 的等价语义, 该步跳过,
//!   只保证文件本体落盘。
//!
//! 另提供跨线程/跨进程互斥的文件锁 [`with_file_lock`]: 锁文件记录持有者 PID,
//! 等待方通过 PID 存活探测判断「持有者进程不存在即视为可接管」, 接管动作由
//! 认领文件串行化竞争者并做二次探测 (防止 PID 复用或并发换主造成误判)。
//! **接管只认确证事实** (持有者进程不存在 / 记录损坏), 瞬时读取失败一律保守
//! 等待 —— 误删活跃锁会破坏互斥 (并发写丢更新), 等待只是慢一点。
//!
//! 权限语义: `mode` 为 unix 权限位 (如 [`OWNER_ONLY_MODE`]); unix 下临时文件
//! 以该 mode 创建并收敛到精确值, 替换后目标文件即为该 mode; 非 unix 平台无
//! unix mode 语义, 权限由 OS 默认 ACL 承载 (0 假装边界)。

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// 默认文件权限 (普通数据文件): 属主读写, 组/其他只读。
pub const DEFAULT_FILE_MODE: u32 = 0o644;

/// 仅属主读写 (凭据/密钥类): 属主读写, 组/其他无任何权限。
pub const OWNER_ONLY_MODE: u32 = 0o600;

/// [`with_file_lock`] 的默认获取超时。
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// 锁获取的重试间隔 (有界退避, 避免热自旋)。
const LOCK_POLL: Duration = Duration::from_millis(2);

/// 临时文件名序号 (进程内单调)。临时名 = `{文件名}.tmp-{pid}-{seq}`:
/// pid 保证跨进程不撞名, seq 保证进程内并发调用不撞名。
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// 两档之一 (完整性档): 原子替换, **不含 fsync**。
///
/// 语义: 读者要么读到旧的完整内容, 要么读到新的完整内容。注释明示: **这不是
/// 崩溃持久** —— 不承诺断电后新内容仍在, 只承诺任何时刻不出现半写内容。
/// 适合临时态/可再生数据 (会话快照、工作状态)。
///
/// `mode` 为 unix 权限位, 最终目标文件恰为该 mode; 非 unix 平台忽略。
pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    write_via_tmp(path, bytes, mode, false)
}

/// 两档之二 (持久档): 原子替换 + `sync_all` 落盘, 崩溃后不回退。
///
/// 与 [`write_atomic`] 的差异: 内容先 `sync_all` 再替换 (unix 追加父目录 fsync;
/// Windows 无父目录 fsync 等价语义, 如实跳过并在此标注平台差异)。
/// 适合配置/凭据类数据: 落盘后进程或机器崩溃, 不会回退到旧版本。
///
/// `mode` 为 unix 权限位, 最终目标文件恰为该 mode; 非 unix 平台忽略。
pub fn write_atomic_durable(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    write_via_tmp(path, bytes, mode, true)
}

fn write_via_tmp(path: &Path, bytes: &[u8], mode: u32, durable: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| contextualize(e, "创建父目录", parent))?;
        }
    }
    let tmp = tmp_sibling(path)?;
    write_at_tmp(path, &tmp, bytes, mode, durable)
}

/// 在**已定名**的临时路径上完成「独占创建 → 写入 → 替换 → 失败清理」。
///
/// 与 [`write_via_tmp`] 拆开是给测试留缝: 验证「临时名已被占用 → 拒绝写入」需要
/// 确定性地预置**那一格**临时名, 而进程内序号 [`TMP_SEQ`] 会被同二进制的并行
/// 测试推进, 无法在测试里预测下一个序号 (D27: 靠预占序号窗口猜必中在并行负载下
/// 失手, 本函数让测试钉定临时名而不猜)。
fn write_at_tmp(path: &Path, tmp: &Path, bytes: &[u8], mode: u32, durable: bool) -> io::Result<()> {
    let result = (|| -> io::Result<()> {
        // create_new(true) 独占创建: 同名文件 (含预置 symlink) 已存在时直接失败,
        // 拒绝覆盖/写穿, 而不是静默接管别人的临时文件。
        let mut file =
            create_exclusive(tmp, mode).map_err(|e| contextualize(e, "独占创建临时文件", tmp))?;
        file.write_all(bytes)
            .map_err(|e| contextualize(e, "写入临时文件", tmp))?;
        if durable {
            file.sync_all()
                .map_err(|e| contextualize(e, "临时文件落盘", tmp))?;
            note_sync_call();
        }
        // 关闭句柄后再替换 (部分平台不允许替换仍被打开的目标)。
        drop(file);
        replace_target(tmp, path).map_err(|e| contextualize(e, "替换目标文件", path))?;
        if durable {
            // 持久档收尾: unix 保证目录项也落盘; Windows 无父目录 fsync
            // 等价语义, 如实跳过 (平台差异)。
            sync_parent_dir(path).map_err(|e| contextualize(e, "父目录落盘", path))?;
        }
        Ok(())
    })();
    if result.is_err() {
        // 失败清理是尽力而为: 半成品临时文件绝不能被读为目标内容。
        let _ = fs::remove_file(tmp);
    }
    result
}

/// 给底层错误补操作与路径上下文 (保留 `ErrorKind`, 便于调用方按类别处理)。
fn contextualize(err: io::Error, operation: &str, path: &Path) -> io::Error {
    io::Error::new(
        err.kind(),
        format!("{operation}失败 {}: {err}", path.display()),
    )
}

/// 同目录临时文件路径: `{文件名}.tmp-{pid}-{seq}`。
fn tmp_sibling(path: &Path) -> io::Result<PathBuf> {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("写入目标缺少文件名: {}", path.display()),
        )
    })?;
    let mut name = OsString::from(file_name);
    name.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        TMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(parent.join(name))
}

/// 以 `create_new(true)` 独占创建临时文件; unix 下以 `mode` 创建并收敛到精确值。
fn create_exclusive(tmp: &Path, mode: u32) -> io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    let file = options.open(tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 创建即定权 + 显式收敛一次: 创建受 umask 影响, 此处保证最终恰为 `mode`。
        let _ = file.set_permissions(fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = mode; // 非 unix: 权限由 OS 默认 ACL 承载, `mode` 无对应语义。
    Ok(file)
}

/// 用已写完的临时文件替换目标。
///
/// 直接 `rename`; 少数平台在目标被短暂打开/被安全软件持有时会瞬时报错,
/// 先做有界重试 (不无限重试: 真权限问题要尽快浮出)。若目标存在时替换仍失败,
/// 退化为「先把目标移到旁侧备份 → 替换 → 清理/失败放回」的顺序, 全程尽力保完整。
fn replace_target(tmp: &Path, dest: &Path) -> io::Result<()> {
    const MAX_ATTEMPTS: u32 = 4;
    const RETRY_DELAY: Duration = Duration::from_millis(5);
    let mut attempts = 0u32;
    loop {
        match fs::rename(tmp, dest) {
            Ok(()) => return Ok(()),
            Err(_) if attempts + 1 < MAX_ATTEMPTS => {
                attempts += 1;
                std::thread::sleep(RETRY_DELAY);
            }
            Err(_) if dest.exists() => {
                let mut bak_name = OsString::from(tmp.as_os_str());
                bak_name.push(".bak");
                let bak = PathBuf::from(bak_name);
                if let Err(err) = fs::rename(dest, &bak) {
                    return Err(err);
                }
                return match fs::rename(tmp, dest) {
                    Ok(()) => {
                        let _ = fs::remove_file(&bak);
                        Ok(())
                    }
                    Err(err) => {
                        // 替换失败: 把目标放回, 尽力恢复到替换前状态。
                        let _ = fs::rename(&bak, dest);
                        Err(err)
                    }
                };
            }
            Err(err) => return Err(err),
        }
    }
}

/// 持久档的收尾: unix 追加父目录 fsync (保证目录项落盘);
/// Windows 无父目录 fsync 等价语义, 如实跳过 (平台差异)。
/// miri 不支持目录 fsync (仅 file-backed fd): 以 no-op 如实跳过 (该步是崩溃
/// 一致性语义, miri 不做崩溃注入); 真实行为由平台测试覆盖 (2026-10-15
/// 釜底抽薪垫片 —— 免去逐测试 ignore, 治 stored_doc 等全部持久档测试)。
#[cfg(unix)]
fn sync_parent_dir(dest: &Path) -> io::Result<()> {
    #[cfg(miri)]
    {
        let _ = dest;
        Ok(())
    }
    #[cfg(not(miri))]
    {
        let parent = match dest.parent() {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => Path::new("."),
        };
        fs::File::open(parent)?.sync_all()
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_dest: &Path) -> io::Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// 文件锁: 锁文件 + PID 存活探测 + 认领文件串行化接管
// ---------------------------------------------------------------------------

/// 目标文件的约定锁文件路径: `<目标路径>.lock`。
///
/// [`with_file_lock`] 直接以给定路径作锁文件; 本助手统一「锁文件长在目标旁边」
/// 的命名约定, 避免各调用点各起名字。
pub fn lock_path_for(target: &Path) -> PathBuf {
    suffix_path(target, ".lock")
}

/// 持锁守卫: 持有期间锁文件存在, 释放 (含 panic 展开) 时删除。
struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 认领文件守卫: 接管竞争者临界区的串行化标记, 离开临界区即删除。
struct ClaimGuard {
    path: PathBuf,
}

impl Drop for ClaimGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 在 `lock_path` 锁文件保护下执行 `f`, 默认超时 [`DEFAULT_LOCK_TIMEOUT`]。
///
/// 语义:
/// - **互斥**: 锁文件以原子发布方式创建, 任一时刻至多一个持有者 (跨线程且跨进程)。
/// - **死锁接管**: 锁文件内容为持有者 PID; 等待方探测到 PID 对应进程不存在
///   即视为可接管 (持有者崩溃不永久砖化锁)。接管动作由认领文件串行化竞争者,
///   并在认领后做二次探测 —— 若锁内容已变化 (被并发换主) 则放弃本轮,
///   防止 PID 复用/并发接管导致误删活跃锁。
/// - **接管只认确证**: 接管输入限「持有者进程不存在」与「记录损坏 (内容可读
///   但非 PID)」两类确证事实; 瞬时读取失败一律保守等待, 绝不推断陈旧
///   (误删活跃锁 = 互斥破坏 = 并发写丢更新, 等待只是慢一点)。
/// - 获取失败 (超时) 返回 [`io::ErrorKind::TimedOut`]; `f` panic 时锁照常释放。
pub fn with_file_lock<T>(lock_path: &Path, f: impl FnOnce() -> T) -> io::Result<T> {
    with_file_lock_timeout(lock_path, DEFAULT_LOCK_TIMEOUT, f)
}

/// [`with_file_lock`] 的显式超时版本。
pub fn with_file_lock_timeout<T>(
    lock_path: &Path,
    timeout: Duration,
    f: impl FnOnce() -> T,
) -> io::Result<T> {
    let guard = acquire_file_lock(lock_path, timeout)?;
    let out = f();
    drop(guard);
    Ok(out)
}

fn acquire_file_lock(lock_path: &Path, timeout: Duration) -> io::Result<LockGuard> {
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let claim_path = suffix_path(lock_path, ".claim");
    let deadline = Instant::now() + timeout;
    let me = std::process::id().to_string();

    loop {
        if publish_marker(lock_path, &me)? {
            return Ok(LockGuard {
                path: lock_path.to_path_buf(),
            });
        }

        let observed = read_marker(lock_path);
        // 只有「确证无合法活跃持有者」才走接管路径:
        // - `Parsed(pid)` 且该进程不存在 (持有者崩溃遗留);
        // - `Corrupt` (内容完整但不是 PID, 记录损坏 —— 不存在合法活跃持有者)。
        // 持有者活着、名字刚空出 (Absent)、瞬时不可读 (Unreadable) 一律等待重试:
        // 不推断、不删除, 防止误删活跃锁。
        let takeoverable = match observed {
            MarkerState::Parsed(pid) => !pid_alive(pid),
            MarkerState::Corrupt => true,
            MarkerState::Absent | MarkerState::Unreadable => false,
        };
        if !takeoverable {
            wait_for_retry(deadline, lock_path)?;
            continue;
        }

        // 持有者进程不存在 (或记录损坏) → 走接管路径。
        if publish_marker(&claim_path, &me)? {
            let claim = ClaimGuard {
                path: claim_path.clone(),
            };
            let second = read_marker(lock_path);
            // 二次探测: 只有「两次读到同一个已死 PID」或「两次都是损坏记录」
            // 才接管; 任何变化 (已释放/换主/转为不可读) 都放弃本轮,
            // 防止 PID 复用/并发接管导致误删活跃锁。
            let stale = match (observed, second) {
                (MarkerState::Parsed(first), MarkerState::Parsed(next)) => {
                    first == next && !pid_alive(next)
                }
                (MarkerState::Corrupt, MarkerState::Corrupt) => true,
                _ => false,
            };
            if stale {
                match fs::remove_file(lock_path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err),
                }
            }
            drop(claim);
        } else {
            // 认领文件已存在: 其记录的持有者若已死 (或记录损坏), 移除以免接管
            // 路径被残留认领卡死; 持有者活着、瞬时不可读、名字已空则不动 (保守)。
            match read_marker(&claim_path) {
                MarkerState::Parsed(pid) if pid_alive(pid) => {}
                MarkerState::Unreadable | MarkerState::Absent => {}
                _ => {
                    let _ = fs::remove_file(&claim_path);
                }
            }
        }
        wait_for_retry(deadline, lock_path)?;
    }
}

/// 原子发布一个「内容完整可见」的标记文件 (锁/认领共用)。
///
/// 先把内容写入私有临时文件, 再以硬链接发布到目标路径 —— 链接创建在目标
/// 已存在时失败, 因此目标路径一旦可见, 其内容必定完整可读, 不存在
/// 「文件已建但 PID 尚未写入」的空内容窗口。
fn publish_marker(path: &Path, content: &str) -> io::Result<bool> {
    let tmp = tmp_sibling(path)?;
    let result = (|| -> io::Result<bool> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(content.as_bytes())?;
        drop(file);
        match fs::hard_link(&tmp, path) {
            Ok(()) => Ok(true),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(false),
            Err(err) => Err(err),
        }
    })();
    let _ = fs::remove_file(&tmp);
    result
}

/// 锁标记内容的读取裁决。
///
/// **只有确证事实才允许触发接管** (见 [`with_file_lock`] 语义): 瞬时读取失败
/// (如刚创建的文件被安全软件短暂独占扫描、删除动议中的共享冲突) 一律推断为
/// 「保守等待」。历史教训 (2026-10-12 定根): 把「读不到」当作「持有者已死」
/// 会误删活跃锁 → 两个写者同时进入临界区 → 并发写丢更新 (负载 flake 的根)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerState {
    /// 内容完整且解析为持有者 PID。
    Parsed(u32),
    /// 文件在、内容可读但不是合法 PID (记录损坏: 不存在合法活跃持有者)。
    Corrupt,
    /// 名字不存在 (锁空闲或刚被释放)。
    Absent,
    /// 读取失败 (瞬时 IO 竞争) —— 不作任何推断。
    Unreadable,
}

fn read_marker(path: &Path) -> MarkerState {
    match fs::read_to_string(path) {
        Ok(text) => match text.trim().parse::<u32>() {
            Ok(pid) => MarkerState::Parsed(pid),
            Err(_) => MarkerState::Corrupt,
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => MarkerState::Absent,
        Err(_) => MarkerState::Unreadable,
    }
}

fn suffix_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

fn wait_for_retry(deadline: Instant, lock_path: &Path) -> io::Result<()> {
    let now = Instant::now();
    if now >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("等待文件锁超时: {}", lock_path.display()),
        ));
    }
    std::thread::sleep(LOCK_POLL.min(deadline - now));
    Ok(())
}

/// PID 存活探测: 进程不存在 → false, 视为可接管。
///
/// - unix: `kill(pid, 0)` 存活性探测 (仅探测, 不发信号)。`ESRCH` = 进程不存在;
///   `EPERM` (存在但无权限) 保守视为存活。
/// - Windows: 打开进程查询句柄; 查询参数错误 = PID 不存在, 其余失败
///   (如访问被拒) 保守视为存活。
/// - 其它平台: 保守视为存活 (不猜测, 不误接管)。
pub fn pid_alive(pid: u32) -> bool {
    // miri 不支持 libc::kill (FFI 边界): 保守返回"存活" —— 锁接管逻辑永不
    // 触发, 走安全方向 (宁等不抢); 真实语义由平台测试覆盖 (2026-10-15 垫片)。
    #[cfg(miri)]
    {
        let _ = pid;
        return true;
    }
    #[cfg(unix)]
    {
        use std::ffi::c_int;
        if pid == 0 || pid > c_int::MAX as u32 {
            return false;
        }
        // 最小化平台绑定: 只声明本模块用到的 POSIX 入口, 不引入额外依赖。
        unsafe extern "C" {
            fn kill(pid: c_int, sig: c_int) -> c_int;
        }
        let rc = unsafe { kill(pid as c_int, 0) };
        if rc == 0 {
            return true;
        }
        // POSIX 固定 ESRCH = 3: 目标进程不存在。其它错误 (EPERM 等) → 存活。
        std::io::Error::last_os_error().raw_os_error() != Some(3)
    }
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        if pid == 0 {
            return false;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        const ERROR_INVALID_PARAMETER: i32 = 87;
        unsafe extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if !handle.is_null() {
            let _ = unsafe { CloseHandle(handle) };
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() != Some(ERROR_INVALID_PARAMETER)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        true
    }
}

// ---------------------------------------------------------------------------
// 测试观察缝: 持久档的 sync 调用在行为层面可观测
// ---------------------------------------------------------------------------
// 崩溃/断电无法在单元测试内复现; 两档的差异行为 (持久档在替换前 sync) 通过
// 线程本地计数器暴露给测试。仅测试构建编译, 生产零开销。

#[cfg(test)]
thread_local! {
    static SYNC_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn note_sync_call() {
    SYNC_CALLS.with(|c| c.set(c.get() + 1));
}

#[cfg(not(test))]
fn note_sync_call() {}

#[cfg(test)]
pub(crate) fn sync_call_count() -> u64 {
    SYNC_CALLS.with(|c| c.get())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{mpsc, Arc, Barrier};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_dir(tag: &str) -> PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-storage-atomic-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建测试目录");
        dir
    }

    /// 从不分配给真实进程的 PID: 大于任何平台的 PID 上限, 探测恒为「不存在」。
    const NEVER_ALIVE_PID: u32 = u32::MAX - 7;

    #[test]
    fn write_atomic_replaces_content_without_residue() {
        let dir = test_dir("replace");
        let target = dir.join("data.json");
        write_atomic(&target, b"{\"v\":1}", DEFAULT_FILE_MODE).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"{\"v\":1}");
        write_atomic(&target, b"{\"v\":2}", DEFAULT_FILE_MODE).unwrap();
        // 读者永远看到完整的新版本, 且无临时/备份残留。
        assert_eq!(fs::read(&target).unwrap(), b"{\"v\":2}");
        let leftovers: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp-") || n.ends_with(".bak"))
            .collect();
        assert!(leftovers.is_empty(), "不应残留临时文件: {leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn readers_observe_only_complete_versions() {
        // 并发读者在写者反复替换期间只允许读到两个完整版本之一, 永不半写。
        let dir = test_dir("torn");
        let target = dir.join("blob.bin");
        let full_a = vec![b'A'; 96 * 1024];
        let full_b = vec![b'B'; 96 * 1024];
        write_atomic(&target, &full_a, DEFAULT_FILE_MODE).unwrap();

        let writer = {
            let target = target.clone();
            let (full_a, full_b) = (full_a.clone(), full_b.clone());
            std::thread::spawn(move || {
                for i in 0..64u32 {
                    let bytes = if i % 2 == 0 { &full_b } else { &full_a };
                    write_atomic(&target, bytes, DEFAULT_FILE_MODE).expect("并发写");
                }
            })
        };
        let mut readers = Vec::new();
        for _ in 0..2 {
            let target = target.clone();
            let (full_a, full_b) = (full_a.clone(), full_b.clone());
            readers.push(std::thread::spawn(move || {
                for _ in 0..256 {
                    match fs::read(&target) {
                        Ok(bytes) => {
                            assert!(
                                bytes == full_a || bytes == full_b,
                                "读到非完整内容: {} 字节",
                                bytes.len()
                            );
                        }
                        // 部分平台的替换退化路径有极短的目标缺位窗口, 重试即可;
                        // 任何成功读到的内容必须是完整版本。
                        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                        Err(err) => panic!("读取失败: {err}"),
                    }
                }
            }));
        }
        writer.join().expect("写者");
        for r in readers {
            r.join().expect("读者");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_refuses_preexisting_tmp_name() {
        // create_new 语义: 预置同名临时文件必须让写入失败, 而不是被静默覆盖
        // (防同路径预置抢占/写穿)。临时名由测试钉定 —— 进程内序号 TMP_SEQ 会被
        // 同二进制并行测试推进, 「预占 0..512 序号窗口必中」在并行负载下失手
        // (D27 失败帧: 实际序号落在窗口外 → 写入成功 → 测试假 Red)。
        // 预置名带进程+线程唯一后缀, 与任何真实调用不撞名; 被测拒绝语义不变:
        // 临时名已存在 → 独占创建失败 → 目标不得被改动。
        let dir = test_dir("exclusive");
        let target = dir.join("t.txt");
        fs::write(&target, "old").unwrap();
        let planted = dir.join(format!(
            "t.txt.tmp-{}-{}",
            std::process::id(),
            TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&planted, "occupied").unwrap();
        let err = write_at_tmp(&target, &planted, b"new", DEFAULT_FILE_MODE, false).unwrap_err();
        assert!(
            matches!(err.kind(), io::ErrorKind::AlreadyExists),
            "预置临时名必须拒绝写入: {err:?}"
        );
        assert_eq!(fs::read(&target).unwrap(), b"old", "目标不得被改动");
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_never_writes_through_planted_symlink_tmp() {
        // 同名 symlink 预置: create_new 视其为已存在而失败, 绝不跟随写穿。
        // 临时名同样由测试钉定 (理由见 write_atomic_refuses_preexisting_tmp_name)。
        let dir = test_dir("symlink");
        let target = dir.join("t.txt");
        let victim = dir.join("victim.txt");
        fs::write(&victim, "untouched").unwrap();
        let planted = dir.join(format!(
            "t.txt.tmp-{}-{}",
            std::process::id(),
            TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::os::unix::fs::symlink(&victim, &planted).unwrap();
        let err = write_at_tmp(&target, &planted, b"new", DEFAULT_FILE_MODE, false).unwrap_err();
        assert!(
            matches!(err.kind(), io::ErrorKind::AlreadyExists),
            "预置 symlink 必须拒绝写入: {err:?}"
        );
        assert_eq!(
            fs::read(&victim).unwrap(),
            b"untouched",
            "symlink 目标不得被写穿"
        );
        assert!(!target.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    // miri 不支持父目录 fsync (仅 file-backed fd): 持久档 unix 收尾会 fsync 目录,
    // miri 下必炸 (unsupported operation)。行为由普通平台测试覆盖, miri 侧显式跳过。
    #[cfg_attr(miri, ignore = "miri 不支持目录 fsync (file-backed fd only)")]
    fn durable_tier_syncs_before_replace_atomic_tier_does_not() {
        // sync 存在性 (行为层面可观测缝): 持久档在替换前做 sync, 完整性档不做。
        let dir = test_dir("sync-tier");
        let target = dir.join("x.bin");
        let before = sync_call_count();
        write_atomic(&target, b"a", DEFAULT_FILE_MODE).unwrap();
        assert_eq!(
            sync_call_count(),
            before,
            "完整性档不得触发 sync (非崩溃持久)"
        );
        write_atomic_durable(&target, b"b", DEFAULT_FILE_MODE).unwrap();
        assert_eq!(sync_call_count(), before + 1, "持久档必须在替换前 sync");
        assert_eq!(fs::read(&target).unwrap(), b"b");
        // 平台差异路径可调用且不报错 (unix 落目录项, 非 unix 如实跳过)。
        sync_parent_dir(&target).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持目录 fsync (file-backed fd only)")]
    fn durable_tier_applies_exact_unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("mode");
        let target = dir.join("secret.json");
        write_atomic_durable(&target, b"x", OWNER_ONLY_MODE).unwrap();
        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "凭据类应恰为 600, 实际 {mode:o}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_excludes_concurrent_holders() {
        let dir = test_dir("lock-mutex");
        let lock = dir.join("unit.lock");
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let holder = {
            let lock = lock.clone();
            std::thread::spawn(move || {
                with_file_lock(&lock, || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
                .expect("持锁");
            })
        };
        started_rx.recv().unwrap();
        let err = with_file_lock_timeout(&lock, Duration::from_millis(200), || ()).unwrap_err();
        assert!(
            matches!(err.kind(), io::ErrorKind::TimedOut),
            "活跃持有者不得被抢锁: {err:?}"
        );
        release_tx.send(()).unwrap();
        holder.join().expect("持锁线程");
        assert!(!lock.exists(), "释放后锁文件必须删除");
        with_file_lock_timeout(&lock, Duration::from_secs(5), || ()).expect("释放后可再获取");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_takes_over_marker_of_dead_holder() {
        // 死锁接管: 锁文件记录的 PID 对应进程不存在 → 视为可接管, 不永久砖化。
        let dir = test_dir("lock-takeover");
        let lock = dir.join("unit.lock");
        assert!(!pid_alive(NEVER_ALIVE_PID), "测试用 PID 必须恒为不存在");
        assert!(pid_alive(std::process::id()), "自身 PID 必须存活");
        fs::write(&lock, format!("{NEVER_ALIVE_PID}\n")).unwrap();
        let out =
            with_file_lock_timeout(&lock, Duration::from_secs(5), || "ok").expect("死锁必须可接管");
        assert_eq!(out, "ok");
        assert!(!lock.exists(), "接管执行完后锁文件必须删除");
        // 认领文件不得残留 (只在接管竞争期间存在)。
        let residue: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".claim"))
            .collect();
        assert!(residue.is_empty(), "认领文件不得残留: {residue:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_takes_over_corrupt_marker_record() {
        // 记录损坏 (内容可读但非 PID) 是确证「无合法活跃持有者」→ 可接管,
        // 与「读取失败」严格区分 (后者保守等待, 见不可读用例)。
        let dir = test_dir("lock-corrupt");
        let lock = dir.join("unit.lock");
        fs::write(&lock, b"this is not a pid").unwrap();
        let out = with_file_lock_timeout(&lock, Duration::from_secs(5), || "ok")
            .expect("损坏记录必须可接管");
        assert_eq!(out, "ok");
        assert!(!lock.exists(), "接管执行完后锁文件必须删除");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_never_takes_over_unreadable_marker() {
        // 瞬时不可读 ≠ 持有者已死 (2026-10-12 定根的并发写丢更新 flake 病根):
        // 读不到标记时必须保守等待到超时, 既不接管也不删除。
        // 不可读的确定性模拟: 在锁文件名上放一个目录 —— 读取必失败且非 NotFound。
        let dir = test_dir("lock-unreadable");
        let lock = dir.join("unit.lock");
        fs::create_dir(&lock).unwrap();
        let err = with_file_lock_timeout(&lock, Duration::from_millis(200), || ()).unwrap_err();
        assert!(
            matches!(err.kind(), io::ErrorKind::TimedOut),
            "不可读标记必须保守等待到超时, 不得推断陈旧: {err:?}"
        );
        assert!(lock.is_dir(), "不可读标记不得被删除");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_serializes_concurrent_writers() {
        // 并发写序列化: 锁内读-改-写共享计数文件, 不得丢更新。
        let dir = test_dir("lock-serialize");
        let lock = dir.join("unit.lock");
        let counter = dir.join("counter.txt");
        write_atomic(&counter, b"0", DEFAULT_FILE_MODE).unwrap();
        const THREADS: u64 = 4;
        const PER_THREAD: u64 = 25;
        let barrier = Arc::new(Barrier::new(THREADS as usize));
        let mut handles = Vec::new();
        for _ in 0..THREADS {
            let (lock, counter) = (lock.clone(), counter.clone());
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..PER_THREAD {
                    with_file_lock(&lock, || {
                        let text = fs::read_to_string(&counter).expect("读计数");
                        let n: u64 = text.trim().parse().expect("计数可解析");
                        write_atomic(&counter, (n + 1).to_string().as_bytes(), DEFAULT_FILE_MODE)
                            .expect("写计数");
                    })
                    .expect("持锁");
                }
            }));
        }
        for h in handles {
            h.join().expect("写者线程");
        }
        let final_value: u64 = fs::read_to_string(&counter)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(final_value, THREADS * PER_THREAD, "并发写不得丢更新");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg_attr(miri, ignore = "miri 不支持 libc::kill (pid_alive 进程存活探测)")]
    fn file_lock_released_when_closure_panics() {
        let dir = test_dir("lock-panic");
        let lock = dir.join("unit.lock");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = with_file_lock(&lock, || -> () {
                panic!("闭包内故障");
            });
        }));
        assert!(result.is_err(), "panic 必须继续向上传播");
        assert!(!lock.exists(), "panic 展开时锁必须释放, 否则死锁");
        with_file_lock_timeout(&lock, Duration::from_secs(5), || ()).expect("panic 后可再获取");
        let _ = fs::remove_dir_all(&dir);
    }
}
