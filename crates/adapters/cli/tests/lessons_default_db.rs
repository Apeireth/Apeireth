//! `apeireth lessons` 认知库落点口径（D3）+ 旗标面严格性测试。
//!
//! 覆盖五件事：
//! ① 无 `--db`、无 `APEIRETH_COGNITIVE_DB` ⇒ 落点等于 CLI 自身默认库口径
//!    （`cli_cognitive_db_path()` 的缺省落点 `.apeireth/cognitive.sqlite3`）——
//!    断言的是**路径**本身，不靠「成功/失败」间接推断；
//! ② `--db <路径>` 覆盖默认落点；
//! ③ 环境变量 `APEIRETH_COGNITIVE_DB` 覆盖默认落点（但仍低于 `--db`）；
//! ④ 未知旗标拒绝即帧 + 退出码非零；
//! ⑤ 悬空旗标（值缺失）拒绝即帧 + 退出码非零，且报「缺值」而非「未知参数」。
//!
//! 口径: 全部在临时目录里跑 —— 仓库 `.apeireth/` 零触碰、零新建库文件；
//! 真进程面用 `CARGO_BIN_EXE_apeireth`，退出码由真进程钉住。
//!
//! 注: 环境变量是**进程级**状态，故涉 env 的断言全部由 [`ENV_LOCK`] 串行化，
//! 免得并行测试互串（同一测试二进制内只有本文件的用例共享进程环境）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use apeireth_cli::lessons_command::{cli_cognitive_db_path, resolve_db, DbSource, USAGE};

/// 涉环境变量的用例串行锁（进程级 env 只能串行改）。
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 环境变量守卫（设/清 + 复原）。
struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

/// 临时目录 (先清后建; 名字带 pid 防并行撞名)。
fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-lessons-db-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// 目录内容清单 (目录带尾斜杠); 空清单 = 零落盘。
fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs_err::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        out.push(if entry.path().is_dir() {
            format!("{name}/")
        } else {
            name
        });
    }
    out.sort();
    out
}

/// ① 无 `--db`、无 env ⇒ 落点 = CLI 默认库口径 (`.apeireth/cognitive.sqlite3`)。
///
/// 断言的是**路径**本身 (不是成功/失败): 缺省落点必须与组合根 `cognitive_db_path()`
/// 同一口径 —— 同一个 CLI 里不再两套默认库。
#[test]
fn default_db_path_follows_the_cli_cognitive_db_path() {
    let _serial = env_lock();
    let _guard = EnvGuard::set("APEIRETH_COGNITIVE_DB", None);
    let resolved = resolve_db(&args(&[])).expect("默认落点必须可解析");
    assert_eq!(
        resolved.source,
        DbSource::Default,
        "无 --db 无 env ⇒ 走默认库档"
    );
    assert_eq!(
        resolved.path,
        cli_cognitive_db_path(),
        "默认落点必须等于 CLI 自身口径"
    );
    assert_eq!(
        resolved.path,
        PathBuf::from(".apeireth/cognitive.sqlite3"),
        "默认落点 = 当前工作目录下的 .apeireth/cognitive.sqlite3"
    );
}

/// ② `--db <路径>` 覆盖默认落点 (旗标 > 默认库)。
#[test]
fn db_flag_overrides_the_default_path() {
    let _serial = env_lock();
    let _guard = EnvGuard::set("APEIRETH_COGNITIVE_DB", None);
    let resolved = resolve_db(&args(&["--db", "/tmp/apeireth-explicit.sqlite3"])).expect("解析");
    assert_eq!(resolved.source, DbSource::Flag, "--db 优先");
    assert_eq!(
        resolved.path,
        PathBuf::from("/tmp/apeireth-explicit.sqlite3"),
        "--db 指定的路径必须原样生效"
    );
}

/// ③ env 覆盖默认落点, 但仍低于 `--db`。
#[test]
fn env_overrides_the_default_path_but_not_the_db_flag() {
    let _serial = env_lock();
    let _guard = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some("/tmp/apeireth-env.sqlite3"));
    let resolved = resolve_db(&args(&[])).expect("解析");
    assert_eq!(resolved.source, DbSource::Env, "env 覆盖默认库");
    assert_eq!(
        resolved.path,
        PathBuf::from("/tmp/apeireth-env.sqlite3"),
        "env 指定的路径必须生效"
    );

    let flagged = resolve_db(&args(&["--db", "/tmp/apeireth-flag.sqlite3"])).expect("解析");
    assert_eq!(flagged.source, DbSource::Flag, "--db 仍高于 env");
    assert_eq!(
        flagged.path,
        PathBuf::from("/tmp/apeireth-flag.sqlite3"),
        "--db 必须压过 env"
    );
}

/// ④ 未知旗标: 拒绝即帧 + 退出码非零 + 当前目录零落盘。
#[test]
fn unknown_flag_is_refused_with_nonzero_exit() {
    let dir = temp_dir("unknown-flag");
    let output = Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["lessons", "--nope"])
        .current_dir(&dir)
        .env_remove("APEIRETH_COGNITIVE_DB")
        .output()
        .expect("run cli");
    assert!(
        !output.status.success(),
        "未知旗标必须非零退出: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("未知参数"), "错误帧点名未知参数: {stderr}");
    assert!(stderr.contains("--nope"), "错误帧点名旗标: {stderr}");
    assert!(stderr.contains(USAGE), "错误帧带用法: {stderr}");
    assert!(
        listing(&dir).is_empty(),
        "拒绝时当前目录零落盘: {:?}",
        listing(&dir)
    );
}

/// ⑤ 悬空旗标 (值缺失): 拒绝即帧 + 退出码非零 + 报「缺值」而非「未知参数」。
#[test]
fn dangling_flag_is_refused_with_nonzero_exit() {
    let dir = temp_dir("dangling-flag");
    let output = Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["lessons", "--db"])
        .current_dir(&dir)
        .env_remove("APEIRETH_COGNITIVE_DB")
        .output()
        .expect("run cli");
    assert!(
        !output.status.success(),
        "悬空旗标必须非零退出: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("缺值"), "错误帧报缺值: {stderr}");
    assert!(
        !stderr.contains("未知参数"),
        "值缺失不得报成未知参数: {stderr}"
    );
    assert!(
        listing(&dir).is_empty(),
        "拒绝时当前目录零落盘: {:?}",
        listing(&dir)
    );
}

/// 附: 默认库不存在时报「默认库不存在 + 实际查了哪个路径」(不笼统说「需要 --db」),
/// 且不静默建空库。
#[test]
fn missing_default_library_is_refused_with_the_queried_path() {
    let dir = temp_dir("missing-default");
    let output = Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["lessons"])
        .current_dir(&dir)
        .env_remove("APEIRETH_COGNITIVE_DB")
        .output()
        .expect("run cli");
    assert!(
        !output.status.success(),
        "默认库不存在必须非零退出: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("默认认知库不存在"),
        "错误帧点名默认库不存在: {stderr}"
    );
    assert!(
        stderr.contains(".apeireth/cognitive.sqlite3"),
        "错误帧报出实际查询路径: {stderr}"
    );
    assert!(
        !stderr.contains("需要 --db"),
        "不得笼统说「需要 --db」: {stderr}"
    );
    assert!(
        listing(&dir).is_empty(),
        "只读面不静默建库: {:?}",
        listing(&dir)
    );
}
