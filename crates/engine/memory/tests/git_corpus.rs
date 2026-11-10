//! `git_corpus` 的契约测试（git 提交历史 → 挖题器的接线面）。
//!
//! 钉住几件事：
//! 1. **读得对**：提交的 hash / 主题 / 时间 / 正文都解析出来，且**倒序 newest-first**；
//! 2. **正文多行不破解析**（这正是 `\x1f`/`\x1e` 分隔符设计的意义）；
//! 3. **limit 生效**（取最新的 N 条）；`limit = 0` ⇒ 不限制；
//! 4. **如实报错**：不是 git 仓库 / 路径不存在 / `git` 不在 PATH ⇒ 各自报各自的错；
//! 5. **接线口径**：`content` = 主题 + 正文、`id` = 短 hash、`timestamp_ms` = 秒 × 1000；
//! 6. **挖得出决策题**（主题含「裁定 / 决定 / 因为」⇒ 命中 `decision` 型）；
//! 7. **太短的提交被过滤**（主题 < 8 字且无正文 ⇒ 不出题）；
//! 8. **完全确定**：同一仓库读两次 ⇒ 逐字相等。
//!
//! ⚠️ **不依赖当前工作区的 git 状态**（那是别人的仓库，状态会变 ⇒ 测试会 flaky）：
//! 每条测试**自己造一个临时 git 仓库**（`TempRepo`），退出时清理。
//!
//! ⚠️ 其中一条测试会临时改 `PATH`（模拟「机器上没装 git」）⇒ `PATH` 是**进程级**全局变量
//! ⇒ 本文件里所有会跑 git 的测试都持有同一把锁（`git_lock()`），避免并发时互相踩。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use apeireth_memory::git_corpus::{
    load_commits, mine_commits, to_mine_records, CommitDoc, GitCorpusError,
};
use apeireth_memory::golden_mining::MiningConfig;

/// 本文件里所有会跑 git 的测试共用的锁（串行化 ⇒ `PATH` 改动不会影响别的测试）。
fn git_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 临时 git 仓库（**自己造**，不碰工作区；退出时删掉）。
struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "apeireth-git-corpus-{}-{tag}-{seq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("创建临时仓库目录");
        // `init.defaultBranch=main` 只是让分支名稳定；**不依赖**分支名的地方照样能跑。
        git(&path, &["-c", "init.defaultBranch=main", "init", "--quiet"]);
        Self { path }
    }

    /// 做一次提交（**显式给日期** ⇒ 时间戳确定、可排序）。
    ///
    /// 日期用 git 的内部格式 `@<epoch> <tz>`（不依赖机器时区 ⇒ 跨机器逐字一致）。
    fn commit(&self, subject: &str, body: Option<&str>, date: &str) -> String {
        let mut args: Vec<String> = vec![
            "-c".to_string(),
            "user.email=t@example.com".to_string(),
            "-c".to_string(),
            "user.name=测试".to_string(),
            // 防止全局 gitconfig 开了签名 / 钩子 ⇒ 测试环境不受个人配置影响。
            "-c".to_string(),
            "commit.gpgsign=false".to_string(),
            "commit".to_string(),
            "-m".to_string(),
            subject.to_string(),
        ];
        if let Some(body) = body {
            args.push("-m".to_string());
            args.push(body.to_string());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        git_with_env(
            &self.path,
            &refs,
            &[("GIT_AUTHOR_DATE", date), ("GIT_COMMITTER_DATE", date)],
        )
    }

    /// 写一个文件并提交（提交之间要有内容变化，否则 git 拒绝空提交）。
    fn commit_file(
        &self,
        name: &str,
        content: &str,
        subject: &str,
        body: Option<&str>,
        date: &str,
    ) -> String {
        std::fs::write(self.path.join(name), content).expect("写入临时文件");
        git(&self.path, &["add", "."]);
        self.commit(subject, body, date)
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// 跑一条 git 命令 ⇒ 返回 stdout；失败直接 panic（测试失败要看得见原因）。
fn git(dir: &Path, args: &[&str]) -> String {
    git_with_env(dir, args, &[])
}

/// 跑一条 git 命令（带环境变量）⇒ 返回 stdout。
fn git_with_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let mut command = Command::new("git");
    command.args(args).current_dir(dir);
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("启动 git 失败");
    assert!(
        output.status.success(),
        "git {args:?} 失败：{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// 造一个 3 条提交的仓库（主题含决策词 ⇒ 能挖出 decision 型题）。
fn three_commit_repo(tag: &str) -> TempRepo {
    let repo = TempRepo::new(tag);
    repo.commit_file(
        "a.txt",
        "x",
        "主题：为什么改成这样",
        Some("原因：压测不通过，所以改了实现。"),
        "@1700000000 +0000",
    );
    repo.commit_file(
        "b.txt",
        "y",
        "裁定：记忆索引改用 sqlite 全文检索",
        None,
        "@1700000100 +0000",
    );
    repo.commit_file(
        "c.txt",
        "z",
        "决定：限流阈值定在每秒两千次并写进文档",
        None,
        "@1700000200 +0000",
    );
    repo
}

// ============================================================
// 一、读提交历史
// ============================================================

#[test]
fn loads_three_commits_with_correct_fields() {
    let _lock = git_lock();
    let repo = three_commit_repo("fields");

    let commits = load_commits(repo.path(), 0).expect("读提交历史");

    assert_eq!(commits.len(), 3, "三条提交 ⇒ 三条记录");
    // 倒序 newest-first（与 `git log` 一致）。
    assert_eq!(commits[0].subject, "决定：限流阈值定在每秒两千次并写进文档");
    assert_eq!(commits[1].subject, "裁定：记忆索引改用 sqlite 全文检索");
    assert_eq!(commits[2].subject, "主题：为什么改成这样");
    // 时间（epoch 秒，提交者时间）。
    assert_eq!(commits[0].timestamp, 1_700_000_200);
    assert_eq!(commits[1].timestamp, 1_700_000_100);
    assert_eq!(commits[2].timestamp, 1_700_000_000);
    // hash：短、非空、互不相同。
    for commit in &commits {
        assert!(!commit.short_hash.is_empty(), "hash 不能为空");
        assert!(
            commit.short_hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash 必须是十六进制：{}",
            commit.short_hash
        );
    }
    assert_ne!(commits[0].short_hash, commits[1].short_hash);
    assert_ne!(commits[1].short_hash, commits[2].short_hash);
}

#[test]
fn limit_takes_the_newest_n_commits() {
    let _lock = git_lock();
    let repo = three_commit_repo("limit");

    let commits = load_commits(repo.path(), 2).expect("读提交历史");

    assert_eq!(commits.len(), 2, "limit = 2 ⇒ 两条");
    assert_eq!(
        commits[0].subject, "决定：限流阈值定在每秒两千次并写进文档",
        "取的是**最新**两条"
    );
    assert_eq!(commits[1].subject, "裁定：记忆索引改用 sqlite 全文检索");

    // limit = 0 ⇒ 不限制（契约：0 或负数 ⇒ 不限制；usize 没有负数）。
    let all = load_commits(repo.path(), 0).expect("读提交历史");
    assert_eq!(all.len(), 3, "limit = 0 ⇒ 全部");
}

#[test]
fn parses_multiline_body_without_breaking_records() {
    let _lock = git_lock();
    let repo = TempRepo::new("body");
    let body = "原因：压测不通过。\n第二行：所以改了实现。\n第三行：限流阈值定在每秒两千次。";
    repo.commit_file(
        "a.txt",
        "x",
        "主题：为什么改成这样",
        None,
        "@1700000000 +0000",
    );
    repo.commit_file(
        "b.txt",
        "y",
        "主题：正文多行",
        Some(body),
        "@1700000100 +0000",
    );

    let commits = load_commits(repo.path(), 0).expect("读提交历史");

    assert_eq!(commits.len(), 2, "多行正文不该把一条提交切成两条");
    let with_body = &commits[0];
    assert_eq!(with_body.subject, "主题：正文多行");
    assert_eq!(
        with_body.body, body,
        "正文里的换行必须原样保留（分隔符设计的目的）"
    );
    // 没有正文的提交 ⇒ body 为空。
    assert_eq!(commits[1].body, "", "没有正文 ⇒ body 为空串");
}

#[test]
fn reads_only_the_current_branch() {
    let _lock = git_lock();
    let repo = TempRepo::new("branch");
    repo.commit_file("a.txt", "x", "主线提交", None, "@1700000000 +0000");
    let base = git(repo.path(), &["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string();
    git(repo.path(), &["checkout", "-b", "feature"]);
    repo.commit_file("b.txt", "y", "特性分支提交", None, "@1700000100 +0000");
    git(repo.path(), &["checkout", &base]);

    let commits = load_commits(repo.path(), 0).expect("读提交历史");

    assert_eq!(commits.len(), 1, "只读当前分支 ⇒ 别的分支的提交不能混进来");
    assert_eq!(commits[0].subject, "主线提交");
}

#[test]
fn non_repository_path_returns_error() {
    let _lock = git_lock();
    // ⚠️ 这个普通目录必须落在**任何 git 仓库之外**（不能建在临时仓库里面：
    // 仓库的子目录按 git 的标准行为仍算「在仓库里」⇒ 会读到整个仓库的历史）。
    let plain =
        std::env::temp_dir().join(format!("apeireth-git-corpus-plain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&plain);
    std::fs::create_dir_all(&plain).expect("创建普通目录");

    let error = load_commits(&plain, 0).expect_err("不是 git 仓库 ⇒ 必须报 Err");
    let _ = std::fs::remove_dir_all(&plain);

    assert!(
        matches!(error, GitCorpusError::NotARepository { .. }),
        "实际错误：{error:?}"
    );
    let text = error.to_string();
    assert!(
        text.contains("git 仓库"),
        "错误文案要说清是仓库问题：{text}"
    );
    assert!(
        text.contains(&plain.display().to_string()),
        "错误文案要带上路径：{text}"
    );
}

#[test]
fn missing_path_returns_error() {
    let _lock = git_lock();
    let missing = std::env::temp_dir().join("apeireth-git-corpus-根本不存在的路径");

    let error = load_commits(&missing, 0).expect_err("路径不存在 ⇒ 必须报 Err");

    assert!(
        matches!(error, GitCorpusError::NotARepository { .. }),
        "实际错误：{error:?}"
    );
}

#[test]
fn git_unavailable_returns_error() {
    let _lock = git_lock();
    let repo = three_commit_repo("nogit");
    // 把 PATH 指向一个**存在但没有 git** 的空目录 ⇒ 找不到 git 可执行文件。
    let empty_path_dir = repo.path().join("empty-path");
    std::fs::create_dir_all(&empty_path_dir).expect("创建空目录");
    let old_path = std::env::var_os("PATH");
    std::env::set_var("PATH", &empty_path_dir);
    let result = load_commits(repo.path(), 0);
    // 立刻还原 PATH（否则后面的测试会找不到 git）。
    match old_path {
        Some(value) => std::env::set_var("PATH", value),
        None => std::env::remove_var("PATH"),
    }

    let error = result.expect_err("git 不存在 ⇒ 必须报 Err");

    assert!(
        matches!(error, GitCorpusError::GitUnavailable(_)),
        "实际错误：{error:?}"
    );
    assert!(
        error.to_string().contains("git"),
        "错误文案要说清是 git 的问题：{error}"
    );
}

#[test]
fn empty_repository_returns_empty_list_not_error() {
    let _lock = git_lock();
    let repo = TempRepo::new("empty");

    let commits = load_commits(repo.path(), 0).expect("空仓库不是错误");

    assert!(commits.is_empty(), "空仓库 ⇒ 没有提交，也没有历史");
}

// ============================================================
// 二、接线（提交 → 挖题器记录）
// ============================================================

#[test]
fn to_mine_records_joins_subject_and_body() {
    let commits = vec![CommitDoc {
        short_hash: "abc1234".to_string(),
        subject: "主题：为什么这样改".to_string(),
        body: "原因：压测不通过。".to_string(),
        timestamp: 1_700_000_000,
    }];

    let records = to_mine_records(&commits);

    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.id, "abc1234", "记录 id = 短 hash");
    // ⚠️ **可检索文本 = 正文，不含主题**：
    // 题目由主题生成 ⇒ 若主题又出现在可检索文本里 ⇒ 检索**字面命中**
    // ⇒ recall@10 恒满分 ⇒ **卷子失效**。
    assert_eq!(
        record.content, "原因：压测不通过。",
        "可检索文本 = 正文（不含主题）"
    );
    assert!(
        !record.content.contains("主题：为什么这样改"),
        "主题不得进可检索文本：{}",
        record.content
    );
    assert_eq!(record.title.as_deref(), Some("主题：为什么这样改"));
    assert_eq!(record.timestamp_ms, 1_700_000_000_000, "时间戳 = 秒 × 1000");
    assert_eq!(
        record.thread_id, None,
        "提交之间没有会话概念 ⇒ thread_id = None"
    );
}

#[test]
fn commits_without_body_have_no_searchable_content() {
    // ⚠️ **口径变更（2026-10-26，防泄漏）**：可检索文本 = **正文**。
    // ⇒ body 为空的提交**没有可检索文本**（原先会退化成「拿主题当正文」）。
    //
    // **为什么不能退回主题**：主题是**题面来源** ⇒ 它若同时是可检索文本
    // ⇒ 检索**字面命中** ⇒ recall 恒满分 ⇒ **卷子失效**。
    // ⇒ 这类提交**由 `mine_commits` 过滤掉**（不进语料），但 `to_mine_records`
    // 仍如实反映「正文为空 ⇒ 可检索文本为空」。
    let commits = vec![CommitDoc {
        short_hash: "deadbee".to_string(),
        subject: "只有主题".to_string(),
        body: String::new(),
        timestamp: 42,
    }];

    let records = to_mine_records(&commits);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "", "正文为空 ⇒ 可检索文本为空");
    assert_eq!(
        records[0].title.as_deref(),
        Some("只有主题"),
        "主题仍在（它是提问的把手）"
    );
    assert_eq!(records[0].timestamp_ms, 42_000);
}

#[test]
fn to_mine_records_skips_commits_without_subject() {
    let commits = vec![
        CommitDoc {
            short_hash: "aaa1111".to_string(),
            subject: "   ".to_string(),
            body: "有正文也没用".to_string(),
            timestamp: 1,
        },
        CommitDoc {
            short_hash: "bbb2222".to_string(),
            subject: "正常主题".to_string(),
            body: String::new(),
            timestamp: 2,
        },
    ];

    let records = to_mine_records(&commits);

    assert_eq!(records.len(), 1, "主题为空 ⇒ 整体跳过");
    assert_eq!(records[0].id, "bbb2222");
}

// ============================================================
// 三、挖题
// ============================================================

#[test]
fn mine_commits_produces_decision_cases() {
    let _lock = git_lock();
    let repo = three_commit_repo("decision");
    let commits = load_commits(repo.path(), 0).expect("读提交历史");

    let cases = mine_commits(&commits, MiningConfig::default());

    assert!(!cases.is_empty(), "决策类提交应当挖得出题");
    let decision_cases: Vec<_> = cases
        .iter()
        .filter(|case| case.kind == "decision")
        .collect();
    assert!(
        !decision_cases.is_empty(),
        "主题含「裁定 / 决定 / 因为」⇒ 必须命中 decision 型；实际题型：{:?}",
        cases
            .iter()
            .map(|case| case.kind.as_str())
            .collect::<Vec<_>>()
    );
    // 每题的答案必须指回真实提交（id = 短 hash）。
    for case in &cases {
        assert!(!case.expect_ids.is_empty(), "每题至少一条标准答案");
        for id in &case.expect_ids {
            assert!(
                commits.iter().any(|commit| &commit.short_hash == id),
                "标准答案 id 必须能在提交里找到：{id}"
            );
        }
    }
}

#[test]
fn mine_commits_filters_out_too_short_commits() {
    let _lock = git_lock();
    let repo = TempRepo::new("short");
    repo.commit_file("a.txt", "x", "fix", None, "@1700000000 +0000");
    repo.commit_file(
        "b.txt",
        "y",
        "裁定：限流阈值定在每秒两千次并写进文档",
        Some("正文：限流阈值定为每秒两千次，理由是压测通过。"),
        "@1700000100 +0000",
    );

    let commits = load_commits(repo.path(), 0).expect("读提交历史");
    let short_hash = commits
        .iter()
        .find(|commit| commit.subject == "fix")
        .expect("找到那条短提交")
        .short_hash
        .clone();
    let cases = mine_commits(&commits, MiningConfig::default());

    assert!(
        !cases
            .iter()
            .any(|case| case.expect_ids.contains(&short_hash)),
        "**没有正文** ⇒ 没有可检索文本 ⇒ 不该出题（防泄漏口径）"
    );
    assert!(
        !cases.is_empty(),
        "同一批里的正常提交仍然要挖得出题（过滤不能连坐）"
    );
}

#[test]
fn mine_commits_keeps_short_subject_when_body_exists() {
    // 过滤规则是「正文非空 **或** 主题 ≥ 8 字」⇒ 短主题 + 有正文 ⇒ 保留。
    let commits = vec![CommitDoc {
        short_hash: "cccc3333".to_string(),
        subject: "改配置".to_string(),
        body: "原因：压测不通过，所以把限流阈值定在每秒两千次。".to_string(),
        timestamp: 1_700_000_000,
    }];

    let cases = mine_commits(&commits, MiningConfig::default());

    assert!(
        cases
            .iter()
            .any(|case| case.expect_ids.contains(&"cccc3333".to_string())),
        "有正文的短主题提交不该被过滤掉"
    );
}

#[test]
fn mined_fragments_are_substrings_of_the_commit_content() {
    let _lock = git_lock();
    let repo = three_commit_repo("fragments");
    let commits = load_commits(repo.path(), 0).expect("读提交历史");

    let cases = mine_commits(&commits, MiningConfig::default());

    assert!(!cases.is_empty());
    for case in &cases {
        assert_eq!(
            case.expect_fragments.len(),
            case.expect_ids.len(),
            "片段与记录 id 一一对应"
        );
        for (id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let commit = commits
                .iter()
                .find(|commit| &commit.short_hash == id)
                .expect("标准答案指向的提交必须存在");
            let content = if commit.body.trim().is_empty() {
                commit.subject.clone()
            } else {
                format!("{}\n\n{}", commit.subject.trim(), commit.body.trim())
            };
            assert!(
                content.contains(fragment.as_str()),
                "片段必须真的在提交正文里：{fragment}"
            );
        }
    }
}

// ============================================================
// 四、确定性与清理
// ============================================================

#[test]
fn deterministic_across_two_runs() {
    let _lock = git_lock();
    let repo = three_commit_repo("determinism");
    let config = MiningConfig::default();

    let first = {
        let commits = load_commits(repo.path(), 0).expect("读提交历史");
        let cases = mine_commits(&commits, config);
        (commits, cases)
    };
    let second = {
        let commits = load_commits(repo.path(), 0).expect("读提交历史");
        let cases = mine_commits(&commits, config);
        (commits, cases)
    };

    assert_eq!(first.0, second.0, "提交记录逐条相等");
    assert_eq!(first.1, second.1, "题目逐条相等（逐字可比）");
}

#[test]
fn temp_repo_is_removed_on_drop() {
    let _lock = git_lock();
    let repo = TempRepo::new("cleanup");
    let path = repo.path().to_path_buf();

    assert!(path.is_dir(), "临时仓库应当建好");

    drop(repo);

    assert!(!path.exists(), "临时仓库必须在退出时清理掉");
}

// ============================================================
// 约定式前缀剥离（题面「像人话」的关键）
// ============================================================

#[test]
fn conventional_prefix_is_stripped_from_the_question_subject() {
    // 本仓提交主题 = 「约定式前缀 + 人写摘要」⇒ 前缀吃光 16 字预算 ⇒ 半截词不像人话。
    let subject = "feat(memory): Phase 2 v0.1 PoC — Episode/Note/Forge";
    let commits = vec![CommitDoc {
        short_hash: "abc1234".to_string(),
        subject: subject.to_string(),
        body: "主体：实现 Episode 与 Note 的核心结构。\n\n原因：先把地基打牢。".to_string(),
        timestamp: 1_700_000_000,
    }];
    let records = to_mine_records(&commits);
    assert_eq!(
        records[0].title.as_deref(),
        Some("Phase 2 v0.1 PoC — Episode/Note/Forge"),
        "前缀应被剥掉，标题留人写摘要"
    );
    let cases = mine_commits(&commits, MiningConfig::default());
    assert!(!cases.is_empty());
    for case in &cases {
        assert!(
            !case.query.contains("feat(memory):"),
            "题目里不该残留约定式前缀：{}",
            case.query
        );
        assert!(
            !case.query.contains("feat("),
            "题目里不该残留半截前缀：{}",
            case.query
        );
    }
}

#[test]
fn non_conventional_subjects_are_left_untouched() {
    // 不像 type(scope) 的行首 ⇒ 原样保留（不乱剥）。
    //
    // ⚠️ **type 必须全小写**（`feat`/`docs`/`fix` 是约定式提交的类型词），
    // 但 **scope 允许大写**（真实仓里可能有 `feat(Web):`）⇒ `feat(Memory):` **会**被剥。
    for subject in [
        "研究：记忆检索的召回延迟压到了十毫秒",
        "Memory: Phase 2 PoC",
        "docs(路径 空格): 保持原样吗",
    ] {
        let commits = vec![CommitDoc {
            short_hash: "def5678".to_string(),
            subject: subject.to_string(),
            body: "正文内容。".to_string(),
            timestamp: 1_700_000_001,
        }];
        let records = to_mine_records(&commits);
        assert_eq!(
            records[0].title.as_deref(),
            Some(subject),
            "不该被剥：{subject}"
        );
    }
}

#[test]
fn prefix_only_subject_falls_back_to_the_original() {
    // 剥完为空（主题只有前缀）⇒ 保留原主题（不产出空标题）。
    let commits = vec![CommitDoc {
        short_hash: "aaa1111".to_string(),
        subject: "docs:".to_string(),
        body: "只有前缀的提交。".to_string(),
        timestamp: 1_700_000_002,
    }];
    let records = to_mine_records(&commits);
    assert_eq!(records[0].title.as_deref(), Some("docs:"));
}
