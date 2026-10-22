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
pub const USAGE: &str = "apeireth lessons [--db <COGNITIVE_DB>] [--scope <problem|file|interaction|tool>] [--status <candidate|verified|retracted>]\n\n  只读列出教训对象（四拍状态机：candidate → verified → retracted）。\n  缺省 db = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3。";

/// 命令入口。
pub fn run_lessons(args: &[String]) -> Result<String, String> {
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

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_is_reported_without_a_database() {
        let error = run_lessons(&[]).expect_err("缺 db 必须报用法");
        assert!(error.contains("--db") && error.contains("apeireth lessons"), "{error}");
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
