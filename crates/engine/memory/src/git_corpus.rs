//! `apeireth-memory::git_corpus` — git 提交历史 → 记忆挖题器的语料接线。
//!
//! # 为什么有这个模块
//!
//! 设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §7.1 的「内部卷」
//! 要**真实语料**（挖 2000 条候选 → owner 精标 300 条）。实测结论记在
//! `docs/status/d0-fidelity-audit.md` §4b：
//!
//! * `<数据根>/diary/*.json` ⇒ 本机**只有 1 条**（`apeireth dream` 写的认知梦境日志，
//!   不是用户内容）⇒ **量不够**，挖不出卷子；
//! * ⭐ **git 提交历史** ⇒ 本仓 **3531 条**提交；近 300 条里 **18 条**主题含
//!   「因为 / 原因 / 裁定 / 决定 / 理由」⇒ 样本读起来**就是**「当时为什么这么定」。
//!
//! ⇒ 本模块把 git 提交历史接成 [`crate::golden_mining::mine`] 能吃的记录序列，
//! **复用同一套挖题器**（不重写挖掘算法 ⇒ 两批语料出的题口径一致、跨批次可比）。
//!
//! # 语料口径（为什么提交历史算得上「决策记忆」）
//!
//! 一条提交天然带三样东西，正好对上挖题器的三种题型：
//!
//! | 提交里有什么 | 挖题器能出的题型 |
//! |---|---|
//! | 主题（人写的标题） | `keyword` —— 问题主体直接取标题 ⇒ **像人话** |
//! | 主题/正文里的「因为 / 裁定 / 决定 / 理由」 | `decision`（「这件事最后是怎么定的？」） |
//! | 提交时间 + 同主题的多次提交 | `temporal` / `conflict`（「什么时候定下来的」「后来怎么改的」） |
//!
//! # 边界（**不夸大**）
//!
//! * 提交历史是「**工程决策记忆**」，**不是**「用户对话记忆」⇒ 内部卷测的是
//!   「长期工程记忆的检索与注入」，**不测**个人生活记忆（报告里必须写清这条边界）。
//! * **只读当前分支**（不加 `--all`）⇒ 不把别的分支混进来；**只读**（绝不写仓库、
//!   绝不 `checkout`、绝不改任何东西）。
//! * 提交之间的 `thread_id` 留 `None`：提交**没有**「会话」概念，编一个假分组键
//!   会让不相关的两条被当成「同主题冲突」⇒ 题目就假了（与 `wiki_corpus_real` 同口径）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径（本模块的真实位置）：
//!
//! - **IMPLEMENTED** ✔：类型、读仓库、接线、导出齐备（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：**无 CLI、无定时任务、生产 0 调用点**；
//! - **DEFAULT ENABLED** ✘：没有开关位，也未默认启用；
//! - **HARDWARE VALIDATED** ✘：只在本仓真实历史上**手工探查**过一次（见交付说明），
//!   **未成规模**（设计稿要 2000 条候选 ⇒ 还差一个数量级）。
//!
//! 对外一律按 **NOT IMPLEMENTED** 表述（新增未接线，不宣称可用）。

use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::golden_mining::{MineRecord, MinedCase, MiningConfig};

/// 一条提交（作为记忆记录）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDoc {
    /// 短 hash（**记录 id 用它**，短且稳定）。
    pub short_hash: String,
    /// 提交主题（标题）。
    pub subject: String,
    /// 提交正文（body；可能为空）。
    pub body: String,
    /// 提交者时间（epoch 秒）。
    pub timestamp: i64,
}

/// 字段分隔符（ASCII Unit Separator）：主题/时间/正文之间的分隔。
///
/// 为什么不用 `\n` 或 `\t`：**正文里本来就有换行** ⇒ 用行分隔会切碎正文；
/// 用 `\t` 又可能撞上正文里的制表符。ASCII 单元分隔符在真实提交正文里几乎不会出现
/// ⇒ 切分是**安全**的。
const FIELD_SEP: char = '\x1f';

/// 记录分隔符（ASCII Record Separator）：一条提交与下一条提交之间的分隔。
///
/// 为什么不用空行：正文里可能有空行 ⇒ 空行不能当记录边界。
const RECORD_SEP: char = '\x1e';

/// `git log` 的格式串（字段分隔 + 记录分隔 ⇒ 正文换行不会破坏解析）。
///
/// `%h` = 短 hash、`%s` = 主题、`%at` = 作者时间（epoch 秒）、`%b` = 正文。
const LOG_FORMAT: &str = "%h%x1f%s%x1f%at%x1f%b%x1e";

/// 主题至少多少字才算「内容够长」（正文为空时的兜底下限）。
///
/// 为什么是 8：太短的主题（「改个 typo」）挖不出有意义的题；但也不能太严，
/// 否则「为什么改」这种短而有力的决策标题会被误杀。
const MIN_SUBJECT_CHARS: usize = 8;

/// 错误类型。
#[derive(Debug, thiserror::Error)]
pub enum GitCorpusError {
    #[error("git 命令不可用：{0}")]
    GitUnavailable(String),
    #[error("不是 git 仓库或仓库不可读：{path}")]
    NotARepository { path: String },
    #[error("git log 失败：{0}")]
    GitFailed(String),
}

/// 从 git 仓库读提交历史（**倒序 newest-first**，与 `git log` 一致）。
///
/// # 怎么读
///
/// 用 `git log --format=…`（**不引新依赖**；`std::process::Command` 即可）：
/// 格式串 = `%h%x1f%s%x1f%at%x1f%b%x1e`
/// （`\x1f` = 字段分隔、`\x1e` = 记录分隔 ⇒ **正文里的换行不会破坏解析**）。
///
/// # 纪律
///
/// * **只读**（绝不写仓库、绝不 `checkout`、绝不改任何东西）；
/// * `limit` = 最多读多少条（**0 ⇒ 不限制**；参数类型是 `usize` ⇒ 不存在负数）；
///   `limit` 生效时**取最新的 N 条**（`git log -n N`）；
/// * 仓库路径不存在 / 不是 git 仓库 ⇒ 返回 `Err`（**如实**，不静默返回空）；
/// * `git` 不存在 ⇒ 返回 `Err`（说明清楚）；
/// * **确定性**：同一仓库同一参数 ⇒ 同样输出；
/// * 只读**当前分支**（不加 `--all`，避免把别的分支混进来）。
///
/// # 空仓库
///
/// 仓库存在、但**一个提交都没有**（刚 `git init`）⇒ 返回 `Ok(vec![])`。
/// 这**不是错误**：仓库确实没有历史。判据用 `rev-parse --verify HEAD` 的**退出码**
/// （不匹配错误文案 ⇒ 不受 `LC_ALL` 影响）。
///
/// # 传入仓库**子目录**会怎样
///
/// 路径落在某个工作树内即可（仓库根或它的子目录都行）—— 与 `git log` 的标准行为一致：
/// 传子目录会读到**整个仓库**的历史（git 的历史本来就不按目录切分）。
/// 完全落在任何工作树之外 ⇒ `Err(NotARepository)`。
pub fn load_commits(repo: &Path, limit: usize) -> Result<Vec<CommitDoc>, GitCorpusError> {
    // ① 路径不存在 / 不是目录 ⇒ 直接报「不是仓库」（如实，不静默返回空）。
    if !repo.is_dir() {
        return Err(GitCorpusError::NotARepository {
            path: repo.display().to_string(),
        });
    }

    // ② 是不是 git 仓库？`rev-parse --git-dir` 成功 ⇒ 是（退出码判定，不猜文案）。
    let probe = run_git(repo, &["rev-parse", "--git-dir"])?;
    if !probe.status.success() {
        return Err(GitCorpusError::NotARepository {
            path: repo.display().to_string(),
        });
    }

    // ③ 仓库里有没有提交？`rev-parse --verify HEAD` 失败 ⇒ 空仓库 ⇒ 没有历史（不是错误）。
    let head = run_git(repo, &["rev-parse", "--verify", "HEAD"])?;
    if !head.status.success() {
        return Ok(Vec::new());
    }

    // ④ 读历史（`--no-pager`：输出被管道接走时 git 本来就不分页，写明更稳；
    //    `--no-optional-locks`：只读命令不该写锁文件 ⇒ 严守「只读」纪律）。
    let format_arg = format!("--format={LOG_FORMAT}");
    let output = if limit > 0 {
        let limit_arg = limit.to_string();
        run_git(
            repo,
            &[
                "--no-pager",
                "--no-optional-locks",
                "log",
                "-n",
                &limit_arg,
                &format_arg,
            ],
        )?
    } else {
        run_git(
            repo,
            &["--no-pager", "--no-optional-locks", "log", &format_arg],
        )?
    };

    if !output.status.success() {
        return Err(GitCorpusError::GitFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_commits(&raw))
}

/// 把提交变成挖题器能吃的记录。
///
/// # 内容拼装（决定挖出来的题像不像人话 ⇒ 照此）
///
/// * `content` = 主题 + `"\n\n"` + 正文（**正文为空则只要主题**）；
/// * `title` = 主题（**`Some`**）；
/// * `timestamp_ms` = 提交时间 × 1000；
/// * `thread_id` = **None**（提交之间没有「会话」概念；但**同一主题前缀**的提交会形成
///   「同主题多次提交」⇒ 天然可挖 temporal / conflict 型题）；
/// * `id` = 短 hash。
///
/// # 主题为空的提交
///
/// **整体跳过**（不产出记录）：标题空 ⇒ 挖题器的问题主体就没有来源，
/// 编一个假标题只会让题目变成假话（与 `wiki_corpus_real` 的「不编标题」同口径）。
pub fn to_mine_records(commits: &[CommitDoc]) -> Vec<MineRecord> {
    commits
        .iter()
        .filter_map(|commit| {
            let subject = commit.subject.trim();
            if subject.is_empty() {
                return None;
            }
            let body = commit.body.trim();
            let content = if body.is_empty() {
                subject.to_string()
            } else {
                format!("{subject}\n\n{body}")
            };
            Some(MineRecord {
                id: commit.short_hash.clone(),
                content,
                // 标题用**剥掉约定式前缀**的摘要（见 [`strip_conventional_prefix`]）。
                title: Some(strip_conventional_prefix(subject)),
                // epoch 秒 → 毫秒；用饱和乘法防溢出（脏数据不该让程序崩）。
                timestamp_ms: commit.timestamp.saturating_mul(1000),
                thread_id: None,
            })
        })
        .collect()
}

/// 剥掉提交主题里的**约定式前缀**（`feat(x): ` / `docs: ` / `fix: ` …）。
///
/// # 为什么剥
///
/// 本仓提交主题是「约定式前缀 + 人写摘要」：
/// `feat(memory): Phase 2 v0.1 PoC - Episode/Note/Forge/...`。
/// 而挖题器把标题裁到 **16 字**当问题主体 ⇒ 前缀把预算吃光 ⇒
/// 题目变成「feat(memory): Ph后来是怎么改的？」——**半截词，不像人话**。
///
/// 剥掉后：「Phase 2 v0.1 PoC 后来是怎么改的？」——**像人话**。
///
/// # 口径
///
/// * 只剥**行首**的 `type(scope): ` / `type: ` 形态（**约定式提交**的固定形状）；
/// * 剥完若**空** ⇒ 保留原主题（不产出空标题）；
/// * **不动**正文（正文里的 `feat(...)` 是内容，不是前缀）。
///
/// ⚠️ 这是**语料侧**的口径（不是挖题器侧）⇒ `golden_mining` 一行未改。
fn strip_conventional_prefix(subject: &str) -> String {
    let trimmed = subject.trim();
    // 形如 `feat(scope): ` 或 `docs: ` ⇒ 取第一个 `: ` 之前的部分，判它像不像 type(scope)。
    let Some(colon) = trimmed.find(':') else {
        return trimmed.to_string();
    };
    let head = &trimmed[..colon];
    let Some(open) = head.find('(') else {
        // 无括号形态：`docs` / `fix` ⇒ 全小写字母 ⇒ 当作 type。
        if !head.is_empty() && head.chars().all(|c| c.is_ascii_lowercase()) {
            let rest = trimmed[colon + 1..].trim_start();
            return if rest.is_empty() {
                trimmed.to_string()
            } else {
                rest.to_string()
            };
        }
        return trimmed.to_string();
    };
    // 有括号形态：`feat(memory)` ⇒ type 必须是纯小写字母，scope 可以有字母数字/连字符/斜杠。
    let Some(close) = head.find(')') else {
        return trimmed.to_string();
    };
    let type_part = &head[..open];
    let scope_part = &head[open + 1..close];
    let type_ok = !type_part.is_empty() && type_part.chars().all(|c| c.is_ascii_lowercase());
    let scope_ok = !scope_part.is_empty()
        && scope_part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/');
    if !(type_ok && scope_ok) {
        return trimmed.to_string();
    }
    let rest = trimmed[colon + 1..].trim_start();
    if rest.is_empty() {
        return trimmed.to_string();
    }
    rest.to_string()
}

/// 从提交历史挖题（= `to_mine_records` + `golden_mining::mine` + 可选过滤）。
///
/// # 过滤规则（让题目「像人话」）
///
/// 只保留**内容够长**的提交（正文非空，或主题长度 ≥ 8 个字符）⇒ 太短的主题挖不出
/// 有意义的题（「改个 typo」不该占掉一条记录的出题预算）。
///
/// # 确定性
///
/// `mine()` 本身确定；本函数只做**过滤**（不改顺序、不改题目文本）⇒ 同一输入逐字相等。
pub fn mine_commits(commits: &[CommitDoc], config: MiningConfig) -> Vec<MinedCase> {
    let kept: Vec<CommitDoc> = commits
        .iter()
        .filter(|commit| is_substantial(commit))
        .cloned()
        .collect();
    let records = to_mine_records(&kept);
    crate::golden_mining::mine(&records, config)
}

// ============================================================
// 内部
// ============================================================

/// 这条提交的内容**够长**吗（正文非空，或主题 ≥ [`MIN_SUBJECT_CHARS`] 字）。
fn is_substantial(commit: &CommitDoc) -> bool {
    !commit.body.trim().is_empty() || commit.subject.trim().chars().count() >= MIN_SUBJECT_CHARS
}

/// 在仓库目录里跑一条 git 命令（**只读**）。
///
/// 失败分两类：
/// * **进程起不来**（`git` 不在 PATH）⇒ [`GitCorpusError::GitUnavailable`]；
/// * **跑起来了但退出码非 0** ⇒ 由调用方按语义归类（仓库不存在 / log 失败）。
fn run_git(repo: &Path, args: &[&str]) -> Result<Output, GitCorpusError> {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        // 显式关掉 stdin：git 不得从父进程偷读输入（测试里父进程可能是终端）。
        .stdin(Stdio::null())
        .output()
        .map_err(|error| GitCorpusError::GitUnavailable(describe_spawn_error(&error)))
}

/// 把「进程起不来」的原因说清楚（owner 看错误文案就能判断是环境问题还是仓库问题）。
fn describe_spawn_error(error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        format!("PATH 里找不到 git 可执行文件（{error}）")
    } else {
        format!("启动 git 失败：{error}")
    }
}

/// 解析 `git log` 的原始输出 ⇒ 提交列表。
///
/// **为什么用分隔符而不是按行切**：正文（body）里**有换行** ⇒ 按行切会切碎正文。
/// 格式串已把字段与记录都用 ASCII 控制字符隔开 ⇒ 这里只要按分隔符切即可。
///
/// 容错：hash 为空的块直接丢（不该发生，但**不 panic**）；时间戳解析失败 ⇒ 记 0
/// （**不猜**时间戳，与 `wiki_corpus_real` 同口径）。
fn parse_commits(raw: &str) -> Vec<CommitDoc> {
    let mut out: Vec<CommitDoc> = Vec::new();
    for chunk in raw.split(RECORD_SEP) {
        // git 在每条记录后补一个换行 ⇒ 切出来的块首尾会有空白 ⇒ 去掉。
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let mut fields = chunk.splitn(4, FIELD_SEP);
        let hash = fields.next().unwrap_or_default().trim();
        if hash.is_empty() {
            continue;
        }
        let subject = fields.next().unwrap_or_default().trim();
        let timestamp_text = fields.next().unwrap_or_default().trim();
        let body = fields.next().unwrap_or_default().trim();
        out.push(CommitDoc {
            short_hash: hash.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            timestamp: timestamp_text.parse::<i64>().unwrap_or(0),
        });
    }
    out
}
