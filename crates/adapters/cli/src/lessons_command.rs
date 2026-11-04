//! `apeireth lessons` —— 教训对象（Lesson）的人类出口（L2 首口）。
//!
//! 设计页：`docs/04-internal/backend-enhancement/lesson-object-design-2026-10-22.md`。
//!
//! 为什么先做 CLI 出口：教训对象 L1 只有库级 API，若无人消费就是「能力齐全但没接电」
//! （见 `docs/status/unwired-ledger.md` W7）。本命令是最小真实消费面 ——
//! 读全部教训并以四拍状态分列，让主人能看见「它学到了什么、哪条被撤回了」。
//!
//! 只读：不做 `verify` / `retract`（那两个动词要写状态机，等自述面/面板一起做，
//! 且需要 owner 确认「谁有权把教训标成已验证」）。只读也意味着**不静默建库** ——
//! 库文件不存在时拒绝即帧，并报出实际查了哪个路径。
//!
//! 认知库口径（2026-11-04 修 D3）：`--db` > 环境变量 `APEIRETH_COGNITIVE_DB` > CLI 默认库。
//! 落点唯一来源 = 组合根 `cognitive_db_path()`（env 覆写优先，否则默认库
//! `.apeireth/cognitive.sqlite3`）—— 本面**不另立一套默认库口径**，旧口径是
//! 「只认 `--db` 或 env，缺了即报错」，同一个 CLI 里两套默认库。
//!
//! 旗标面严格：未知旗标 / 缺值 / 重复旗标 一律拒绝即帧（旧口径 `parse_arg` 静默忽略
//! 未知旗标 —— 敲错 `--dbu` 会得到一个看似正常的空帧，而非被拒）。

use std::path::PathBuf;

use apeireth_memory::SqliteMemoryStore;
use apeireth_memory::{LessonScope, LessonStatus};

use crate::flags;
use crate::COGNITIVE_DB_ENV;

/// 用法帧。
pub const USAGE: &str = "apeireth lessons [--db <COGNITIVE_DB>] [--scope <problem|file|interaction|tool>] [--status <candidate|verified|retracted>]\n\n  只读列出教训对象（四拍状态机：candidate → verified → retracted）。\n  认知库优先级: --db > 环境变量 APEIRETH_COGNITIVE_DB > CLI 默认库（当前工作目录下的 .apeireth/cognitive.sqlite3）。\n  默认库不存在时拒绝即帧并报出实际查询路径（不静默建空库）。";

/// 认知库落点的来源（决定错误帧措辞）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbSource {
    /// `--db` 显式指定。
    Flag,
    /// 环境变量 `APEIRETH_COGNITIVE_DB` 覆写。
    Env,
    /// CLI 默认库（组合根 `cognitive_db_path()` 的缺省落点）。
    Default,
}

/// 认知库解析结果：实际查询路径 + 来源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbResolution {
    /// 实际查询的库路径。
    pub path: PathBuf,
    /// 落点来源。
    pub source: DbSource,
}

/// 命令入口。
pub fn run_lessons(args: &[String]) -> Result<String, String> {
    let flags = parse_flags(args)?;
    let scope = match flags.scope.as_deref() {
        Some(raw) => Some(LessonScope::parse(raw).map_err(|error| error.to_string())?),
        None => None,
    };
    let status = match flags.status.as_deref() {
        Some(raw) => Some(LessonStatus::parse(raw).map_err(|error| error.to_string())?),
        None => None,
    };
    let db = resolve_db_from(&flags);
    ensure_db_present(&db)?;
    let store =
        SqliteMemoryStore::open(&db.path).map_err(|error| format!("认知库打开失败: {error}"))?;
    let counts = store
        .count_lessons_by_status()
        .map_err(|error| error.to_string())?;
    let lessons = store
        .list_lessons(scope, status, 100)
        .map_err(|error| error.to_string())?;

    let mut frame = format!(
        "lessons: verified={} candidate={} retracted={} (共 {} 条在册)\n",
        counts.verified,
        counts.candidate,
        counts.retracted,
        counts.verified + counts.candidate + counts.retracted
    );
    if lessons.is_empty() {
        frame.push_str("（暂无教训 —— 没有记录不等于没学到，是还没有人被提炼成可验证的教训）\n");
        return Ok(frame);
    }
    for lesson in lessons {
        frame.push_str(&format!(
            "- [{status:?}] ({scope:?}) {statement}\n    来源经历: {sources}\n    证据: {evidence}\n",
            status = lesson.status,
            scope = lesson.scope,
            statement = lesson.statement,
            sources = lesson.source_episodes.join(", "),
            evidence = lesson.evidence,
        ));
        if let Some(witness) = &lesson.verified_by {
            frame.push_str(&format!("    验证场景: {witness}\n"));
        }
        if let Some(reason) = &lesson.retract_reason {
            frame.push_str(&format!("    撤回理由: {reason}\n"));
        }
    }
    Ok(frame)
}

/// 解析本命令将查询的认知库（口径：`--db` > 环境变量 > CLI 默认库）。
///
/// 与命令入口同一条解析链：落点唯一来源 = 组合根 `cognitive_db_path()`。
pub fn resolve_db(args: &[String]) -> Result<DbResolution, String> {
    Ok(resolve_db_from(&parse_flags(args)?))
}

/// CLI 自身的认知库落点口径（env 覆写优先，否则默认库）—— 供其它面/测试面比对。
pub fn cli_cognitive_db_path() -> PathBuf {
    crate::cognitive_db_path()
}

/// 已解析的旗标面（闭集：`--db` / `--scope` / `--status`）。
#[derive(Debug, Default, Clone)]
struct Flags {
    db: Option<String>,
    scope: Option<String>,
    status: Option<String>,
}

/// 本命令的旗标白名单（三个带值旗标；本命令不收布尔旗标）。
const LESSONS_FLAGS: flags::FlagSpec = flags::FlagSpec {
    value: &["--db", "--scope", "--status"],
    boolean: &[],
    meta_as_positional: false,
};

/// 旗标解析：未知旗标 / 缺值 / 重复旗标 一律拒绝即帧。
///
/// 判据（旗标形态 / 缺值 / `--` 分隔）走 crate 级共享模块
/// [`crate::flags`]；本命令只留自己的措辞（「未知参数」—— 旗标与位置词类
/// 同称）与用法帧。
fn parse_flags(args: &[String]) -> Result<Flags, String> {
    let known = LESSONS_FLAGS.known();
    let scanned = flags::scan(args, &LESSONS_FLAGS).map_err(|error| {
        error.frame(
            flags::Style::Zh,
            &flags::FrameContext::new("lessons", flags::UnknownNoun::Argument)
                .with_usage(USAGE)
                .with_known(&known),
        )
    })?;
    // 同一旗标只允许给一次（重复即拒，不静默取最后一个）。
    for name in ["--db", "--scope", "--status"] {
        if scanned.count(name) > 1 {
            return Err(format!("lessons 重复参数: {name}\n{USAGE}"));
        }
    }
    Ok(Flags {
        db: scanned.value("--db").map(str::to_string),
        scope: scanned.value("--scope").map(str::to_string),
        status: scanned.value("--status").map(str::to_string),
    })
}

/// 认知库解析（`--db` > 环境变量 > CLI 默认库）。
///
/// 落点唯一来源 = 组合根 [`crate::cognitive_db_path`]（env 覆写优先，否则默认库
/// `.apeireth/cognitive.sqlite3`）—— 本面不另立一套默认库常量，复制即口径漂移。
/// `DbSource` 只是**来源标签**（供错误帧说清「谁定的这个落点」），不参与定路径。
fn resolve_db_from(flags: &Flags) -> DbResolution {
    if let Some(explicit) = flags.db.as_deref() {
        return DbResolution {
            path: PathBuf::from(explicit),
            source: DbSource::Flag,
        };
    }
    DbResolution {
        path: crate::cognitive_db_path(),
        source: if env_db_overridden() {
            DbSource::Env
        } else {
            DbSource::Default
        },
    }
}

/// 环境变量是否真的覆写了落点。
///
/// 判据与组合根 `cognitive_db_path()` 的 env 分支同款（空值/纯空白按未设处理）——
/// 两处规则若漂移，「来源标签」就会与实际落点对不上。
fn env_db_overridden() -> bool {
    std::env::var(COGNITIVE_DB_ENV)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

/// 库必须已存在：不存在即拒绝即帧，并报出**实际查询了哪个路径**。
///
/// 口径（各来源同款，无例外）: 教训面是**只读**面 —— 库不存在时报「不存在 + 已查
/// 路径」，绝不静默建一个空库再报「暂无教训」（那是拿新建的空账冒充「没学到」，
/// 也把一个写动作塞进只读面）。
fn ensure_db_present(db: &DbResolution) -> Result<(), String> {
    if db.path.is_file() {
        return Ok(());
    }
    let path = db.path.display();
    let frame = if db.path.exists() {
        format!("lessons 认知库路径不是文件\n  已查路径: {path}\n{USAGE}")
    } else {
        match db.source {
            DbSource::Flag => {
                format!("lessons --db 指定的认知库不存在\n  已查路径: {path}\n{USAGE}")
            }
            DbSource::Env => format!(
                "lessons 环境变量 {COGNITIVE_DB_ENV} 指定的认知库不存在\n  已查路径: {path}\n{USAGE}"
            ),
            DbSource::Default => format!(
                "lessons 默认认知库不存在\n  已查路径: {path}（相对当前工作目录）\n  换库方式: --db <路径> 或环境变量 {COGNITIVE_DB_ENV}\n{USAGE}"
            ),
        }
    };
    Err(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-lessons-cli-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        dir.join("cognitive.sqlite3")
    }

    #[test]
    fn missing_library_is_refused_with_the_queried_path() {
        let db = temp_db("missing").join("absent.sqlite3");
        let args = vec!["--db".to_string(), db.to_string_lossy().to_string()];
        let error = run_lessons(&args).expect_err("库不存在必须拒绝");
        assert!(error.contains("不存在"), "{error}");
        assert!(error.contains(&db.to_string_lossy().to_string()), "{error}");
        assert!(error.contains(USAGE), "{error}");
    }

    #[test]
    fn unknown_flag_is_refused() {
        let error = run_lessons(&["--nope".to_string()]).expect_err("未知旗标必须拒绝");
        assert!(error.contains("未知参数"), "{error}");
        assert!(error.contains("--nope"), "{error}");
        assert!(error.contains(USAGE), "{error}");
    }

    #[test]
    fn dangling_flag_is_refused_as_missing_value() {
        let error = run_lessons(&["--db".to_string()]).expect_err("悬空旗标必须拒绝");
        assert!(error.contains("缺值"), "{error}");
        assert!(!error.contains("未知参数"), "{error}");
        assert!(error.contains(USAGE), "{error}");
    }

    #[test]
    fn flag_shaped_value_is_refused_as_missing_value() {
        let error =
            run_lessons(&["--db".to_string(), "--scope".to_string()]).expect_err("缺值必须拒绝");
        assert!(error.contains("缺值"), "{error}");
        assert!(!error.contains("未知参数"), "{error}");
    }

    #[test]
    fn duplicate_flag_is_refused() {
        let error = run_lessons(&[
            "--db".to_string(),
            "/tmp/a.sqlite3".to_string(),
            "--db".to_string(),
            "/tmp/b.sqlite3".to_string(),
        ])
        .expect_err("重复旗标必须拒绝");
        assert!(error.contains("重复参数"), "{error}");
    }

    #[test]
    fn unknown_scope_is_refused_by_the_closed_vocabulary() {
        let dir = std::env::temp_dir().join(format!("apeireth-lessons-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let db = dir.join("cognitive.sqlite3");
        // 先建库（open 会跑迁移）。
        SqliteMemoryStore::open(&db).expect("open store");
        let args = vec![
            "--db".to_string(),
            db.to_string_lossy().to_string(),
            "--scope".to_string(),
            "自由文本".to_string(),
        ];
        let error = run_lessons(&args).expect_err("词表外必须拒绝");
        assert!(error.contains("场景词表外"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_ledger_is_reported_honestly() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-lessons-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let db = dir.join("cognitive.sqlite3");
        SqliteMemoryStore::open(&db).expect("open store");
        let args = vec!["--db".to_string(), db.to_string_lossy().to_string()];
        let frame = run_lessons(&args).expect("frame");
        assert!(frame.contains("verified=0"), "{frame}");
        assert!(frame.contains("暂无教训"), "{frame}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
