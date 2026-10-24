//! `apeireth lessons` —— 教训对象（Lesson）的人类出口（L2 首口）。
//!
//! 设计页：`docs/04-internal/backend-enhancement/lesson-object-design-2026-10-22.md`。
//!
//! 为什么先做 CLI 出口：教训对象 L1 只有库级 API，若无人消费就是「能力齐全但没接电」
//! （见 `docs/status/unwired-ledger.md` W7）。本命令是最小真实消费面 ——
//! 读全部教训并以四拍状态分列，让主人能看见「它学到了什么、哪条被撤回了」。
//!
//! 只读：不做 `verify` / `retract`（那两个动词要写状态机，等自述面/面板一起做，
//! 且需要 owner 确认「谁有权把教训标成已验证」）。

use std::path::PathBuf;

use apeireth_memory::SqliteMemoryStore;
use apeireth_memory::{LessonScope, LessonStatus};

/// 用法帧。
pub const USAGE: &str = "apeireth lessons [--db <COGNITIVE_DB>] [--scope <problem|file|interaction|tool>] [--status <candidate|verified|retracted>]\napeireth lessons add --statement <一句话教训> --scope <scope> --episode <经历ID> [--episode ...] --evidence <证据摘要>\n\n  list  = 只读列出教训对象（四拍状态机：candidate → verified → retracted）。\n  add   = 写入一条**候选**教训（只能写 candidate）。\n\n  **硬约束由存储层把关，CLI 不绕过**：出处经历必须真实存在（防编造）、语句与证据非空。\n  没有出处的教训按幻觉拒绝。\n\n  缺省 db = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3。";

/// 命令入口。
pub fn run_lessons(args: &[String]) -> Result<String, String> {
    // 子命令：`add` 是**写入路径**（此前教训表全仓零写入 ⇒ `apeireth lessons` 永远空表，
    // 是「能力齐全但没接电」的教科书案例）；其余走 list（默认，向后兼容）。
    if args.first().map(String::as_str) == Some("add") {
        return run_add(&args[1..]);
    }
    let db = parse_arg(args, "--db")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("APEIRETH_COGNITIVE_DB")
                .ok()
                .map(PathBuf::from)
        })
        .ok_or_else(|| format!("lessons 需要 --db 或 APEIRETH_COGNITIVE_DB\n{USAGE}"))?;
    let scope = match parse_arg(args, "--scope") {
        Some(raw) => Some(LessonScope::parse(&raw).map_err(|error| error.to_string())?),
        None => None,
    };
    let status = match parse_arg(args, "--status") {
        Some(raw) => Some(LessonStatus::parse(&raw).map_err(|error| error.to_string())?),
        None => None,
    };
    let store = SqliteMemoryStore::open(&db).map_err(|error| format!("认知库打开失败: {error}"))?;
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

/// `apeireth lessons add` —— 写入一条**候选**教训。
///
/// # 为什么只写 `candidate`
///
/// 四拍状态机的后两拍（`verified` / `retracted`）需要**真实消费方**才该动：
/// `verified` 要「新场景验证」（不得用来源经历自证），`retracted` 要「撤回理由 +
/// 影响追踪」（与问责闭环的通知义务耦合）。这两件事没有真实入口前，
/// 只开 `add` 一个动作，**不假装状态机能完整转**。
///
/// # 硬约束不绕过
///
/// 出处经历（`--episode`）必须**真实存在**：存储层会给 `UnknownSource`，
/// 本命令原样报错。这是「没有出处的教训按幻觉拒绝」那条纪律的执行点。
fn run_add(args: &[String]) -> Result<String, String> {
    use apeireth_memory::lesson::NewLesson;

    let db = parse_arg(args, "--db")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("APEIRETH_COGNITIVE_DB")
                .ok()
                .map(PathBuf::from)
        })
        .ok_or_else(|| format!("lessons add 需要 --db 或 APEIRETH_COGNITIVE_DB\n{USAGE}"))?;
    let statement = parse_arg(args, "--statement")
        .ok_or_else(|| format!("lessons add 需要 --statement\n{USAGE}"))?;
    let scope_raw =
        parse_arg(args, "--scope").ok_or_else(|| format!("lessons add 需要 --scope\n{USAGE}"))?;
    let evidence = parse_arg(args, "--evidence")
        .ok_or_else(|| format!("lessons add 需要 --evidence\n{USAGE}"))?;
    let episodes = parse_all(args, "--episode");
    if episodes.is_empty() {
        return Err(format!(
            "lessons add 需要至少一个 --episode（教训必须有出处；\
             没有出处的教训按幻觉拒绝）\n{USAGE}"
        ));
    }
    let scope = LessonScope::parse(&scope_raw).map_err(|error| error.to_string())?;

    let store = SqliteMemoryStore::open(&db).map_err(|error| format!("认知库打开失败: {error}"))?;
    let lesson = store
        .append_lesson_candidate(NewLesson {
            statement,
            scope,
            source_episodes: episodes,
            evidence,
        })
        .map_err(|error| format!("教训写入被拒: {error}"))?;
    Ok(format!(
        "已写入候选教训 #{}（status=candidate）\n\
         它还不是「学到」—— 要经过新场景验证才成为 verified。\n语句: {}\n",
        lesson.id, lesson.statement
    ))
}

/// 收集某个旗标的全部出现值（`--episode` 可重复）。
fn parse_all(args: &[String], name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            if let Some(value) = args.get(index + 1) {
                found.push(value.clone());
            }
            index += 2;
            continue;
        }
        index += 1;
    }
    found
}

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `lessons add`：没有出处经历 ⇒ 拒绝（**没有出处的教训按幻觉拒绝**）。
    #[test]
    fn add_without_episode_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("cognitive.sqlite3");
        let args: Vec<String> = vec![
            "add".into(),
            "--db".into(),
            db.to_string_lossy().to_string(),
            "--statement".into(),
            "试试这条".into(),
            "--scope".into(),
            "tool".into(),
            "--evidence".into(),
            "某次失败".into(),
        ];
        let err = run_lessons(&args).expect_err("没有出处必须拒绝");
        assert!(err.contains("--episode"), "理由应可读: {err}");
        assert!(err.contains("幻觉"), "应说明纪律: {err}");
    }

    /// `lessons add`：出处经历不存在 ⇒ 存储层拒绝（防编造）。
    #[test]
    fn add_with_unknown_episode_is_refused_by_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("cognitive.sqlite3");
        let args: Vec<String> = vec![
            "add".into(),
            "--db".into(),
            db.to_string_lossy().to_string(),
            "--statement".into(),
            "试试这条".into(),
            "--scope".into(),
            "tool".into(),
            "--episode".into(),
            "ep-does-not-exist".into(),
            "--evidence".into(),
            "某次失败".into(),
        ];
        let err = run_lessons(&args).expect_err("编造出处必须被拒");
        assert!(err.contains("教训写入被拒"), "理由应可读: {err}");
    }

    #[test]
    fn usage_is_reported_without_a_database() {
        let error = run_lessons(&[]).expect_err("缺 db 必须报用法");
        assert!(
            error.contains("--db") && error.contains("apeireth lessons"),
            "{error}"
        );
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
