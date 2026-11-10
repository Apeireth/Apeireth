//! council ballot 的 **append-only 落盘面**（JSONL，一行一条）。
//!
//! # 这个模块补的是什么缺口
//!
//! `apeireth nightwatch`（守夜人复盘）自报过一条 advisory gap：
//!
//! > council ballot 未持久化 —— 立场平衡复盘段空转
//!
//! 深查后确认比字面更严重：`council`（顾问团）已在生产装配，每次决策会产出若干条
//! 顾问评价（`orchestration::AdvisorEvaluation { advisor, kind, verdict }`），但
//! **这些票没有任何地方落盘** —— 它们只活在当次进程的内存里，重启即散。
//!
//! 守夜人复盘要吃的是 `rubric::Ballot`（立场票），两边形状对不上是一层问题；
//! **票本身不存在**是更底层的问题。本模块只补**落盘这一层**：把已经定形的立场票
//! 写成**可序列化的事实**追加进 JSONL 账，让复盘段有东西可读。
//!
//! # 为什么落的是「字符串化的事实」而不是 `rubric::Ballot`
//!
//! 账文件是**跨 crate 的长期格式**：它要在治理 crate 的类型演进之后仍然可回放。
//! 直接序列化 `rubric::Ballot` 会把治理侧的改型（枚举改名、加字段、换类型）
//! 直接变成账文件的破坏性变更。落一层**扁平化事实**（立场 / 域写成 snake_case 字符串、
//! 顾问自评质量分与落盘时刻写成标量）之后：
//!
//! * 账文件格式只由**本模块**决定，治理侧改型不会毁掉历史；
//! * 读回端不依赖治理 crate ⇒ 复盘可以只读账、不牵治理；
//! * 字段全是标量 ⇒ 单行 JSON 稳定、可 grep、可 `jq`。
//!
//! # 语义（与 `apeireth_guard::decision_log` 同纪律）
//!
//! * **append-only**：一行一条 JSON 对象，只追加，永不改写既有行；
//! * **读出错不 panic、不吞**：坏行**跳过**并计入 [`CouncilBallotRead::skipped`]，
//!   整份读失败返回 `Err`（不伪装成空账）；
//! * **缺失 = 合法空态**：账文件不存在 ⇒ 读回空 vec（不是错误：还没有票发生过）。
//!
//! # 读取上界策略：**有界尾部读**（bounded tail read）
//!
//! 选型：`read_recent` **只读文件尾部至多 4 MiB**
//! （[`MAX_BALLOT_LOG_READ_BYTES`]），而不是「读全文件再取尾部 N 条」。
//!
//! * **为什么**：复盘只看最近的若干条票；账会无限增长，「读全文件」在长跑进程里
//!   就是把内存吃穿（跑了几周的实例，账可以是几百 MiB）。
//! * **怎么做**：先取文件长度，从 `len - min(len, 4 MiB)` 处 seek 读出尾部窗口；
//!   窗口起点若不在文件首（`start > 0`），窗口第一行是**被截断的半行** ⇒ 丢弃、
//!   不参与解析（否则会把半行误判成坏行，白丢一个 `skipped`）。
//! * **代价（明说）**：单行超过 4 MiB 的票读不回来（窗口装不下整行 ⇒ 当作截断半行
//!   丢掉）；`limit` 大于尾部窗口能装下的条数时，返回的是**窗口内**能解析出的全部
//!   （可能少于 `limit`）。这是拿「读回条数」换「内存有界」，刻意如此。
//!
//! # 不做（诚实边界）
//!
//! - **不做映射层**：`orchestration::AdvisorEvaluation` → 本记录的转换（立场词表、
//!   置信度兜底、权重来源）由**映射层**负责（rubric 侧）。本模块只存已经定形的
//!   事实，`stance` / `domain` 在这里就是**不透明字符串**，不依赖治理 crate 的枚举。
//! - **不改写既有行**：没有去重、没有压缩、没有轮转、没有 fsync 事务。
//! - **不做跨进程文件锁**：同进程内一把 [`Mutex`] 串行化 append；多进程并发写不在本批范围。
//! - **不保证读回全量**：读端有 4 MiB 上界，超出部分**不读**（见上节）。
//! - **不新增 crate / 不新增依赖 / 不新增环境变量**（本仓 env 名预算已满）。

#![forbid(unsafe_code)]

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// council ballot 账的文件名（放在数据目录下）。
pub const COUNCIL_BALLOT_LOG_FILE: &str = "council-ballots.jsonl";

/// 单次读取的字节上界（4 MiB）—— 见模块文档「读取上界策略」。
pub const MAX_BALLOT_LOG_READ_BYTES: u64 = 4 * 1024 * 1024;

/// 一条 council ballot 的**落盘形状**（append-only 账的一行）。
///
/// 与 `rubric::Ballot` 的区别：这里存的是**可序列化的事实**（字符串化的 stance/domain、
/// 顾问自评质量分、落盘时刻），不依赖治理 crate 的类型 ⇒ 账文件格式稳定、可回放。
///
/// 数值字段的取值域由**映射层**保证（置信度 / 权重 / 自评分均应为 `0.0..=1.0`）；
/// 本层只负责如实落盘，不做二次夹取（`NaN` / `inf` 会被 `serde_json` 写成 `null`，
/// 读回时该行解析失败 ⇒ 按坏行跳过）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouncilBallotRecord {
    /// 顾问稳定名。
    pub advisor: String,
    /// 立场（`rubric::StanceKind` 的 snake_case 名：approve/disapprove/hold/abstain）。
    pub stance: String,
    /// 置信度 0..=1。
    pub confidence: f64,
    /// 置信度是否兜底（顾问没显式给，由映射层折算）。
    pub confidence_defaulted: bool,
    /// 票权重 0..=1（现有顾问注册权重）。
    pub weight: f64,
    /// 顾问域（Safety/Performance/… 的 snake_case 名；无则 None）。
    pub domain: Option<String>,
    /// 顾问自评质量分 0..=1（verdict 的原始分数）。
    pub advisor_score: f64,
    /// 落盘时刻（epoch 毫秒）。
    pub at_ms: i64,
}

/// 一次读取的结果：读回的票 + 被跳过的坏行数。
///
/// `skipped > 0` ⇒ 账里有行读不出来（损坏 / 半行 / 词表变更）。**不 panic、不吞**：
/// 坏行只影响它自己，其余行照常读回；但这个计数让复盘面**看得见**账已经不完整。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CouncilBallotRead {
    /// 读回的票（按落盘顺序，**最新在后**）。
    pub records: Vec<CouncilBallotRecord>,
    /// 被跳过的坏行数（解析失败 / 非法 UTF-8 的行）。
    pub skipped: usize,
}

/// council ballot 的 append-only 账（JSONL，一行一条）。
///
/// 语义与仓库里既有的守卫判定账（`apeireth_guard::decision_log`）一致：
/// * 只追加，不改写既有行；
/// * 每行一个 JSON 对象；
/// * 读出错不 panic、不吞：损坏行跳过并计入 `skipped`，读整体失败返回 `Err`。
///
/// 写面用「追加打开 + 写一行 + 换行」——**不是**临时文件 + rename（那是覆盖式语义，
/// 与 append-only 账相悖）。
pub struct CouncilBallotLog {
    path: PathBuf,
    /// 写盘互斥（同进程内多线程 append 不交错）。
    write_lock: Mutex<()>,
}

impl std::fmt::Debug for CouncilBallotLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CouncilBallotLog")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl CouncilBallotLog {
    /// 打开（或将要创建）账文件。
    ///
    /// **不做 I/O**：只记住路径。父目录在第一次 [`append`](Self::append) 时创建
    /// （「还没有票发生过」不该留下一个空账文件，那会让人以为复盘面在跑）。
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: Mutex::new(()),
        }
    }

    /// 账文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 追加一条（创建父目录；写失败如实返回 Err 字符串）。
    ///
    /// 写盘失败**不 panic**：调用方自己决定是吞掉并计数，还是让复盘段降级。
    pub fn append(&self, record: &CouncilBallotRecord) -> Result<(), String> {
        let line = serde_json::to_string(record)
            .map_err(|error| format!("council ballot 序列化失败: {error}"))?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "council ballot 账写锁中毒".to_string())?;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "创建 council ballot 账目录失败 ({}): {error}",
                        parent.display()
                    )
                })?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| {
                format!(
                    "打开 council ballot 账失败 ({}): {error}",
                    self.path.display()
                )
            })?;
        // 一行一次 write（拼好换行再写），避免分成两次 write 造成半行。
        let mut payload = line;
        payload.push('\n');
        file.write_all(payload.as_bytes())
            .map_err(|error| format!("写 council ballot 账失败: {error}"))
    }

    /// 读最近 `limit` 条（按落盘顺序，**最新在后**；`limit == 0` ⇒ 空 vec）。
    ///
    /// - 损坏行跳过（不 panic），整份读失败返回 `Err`；
    /// - 账文件不存在 ⇒ 空 vec（不是错误：还没有票发生过）；
    /// - 读取有 4 MiB 上界，见 [`CouncilBallotRead`] 与模块文档「读取上界策略」。
    pub fn read_recent(&self, limit: usize) -> Result<Vec<CouncilBallotRecord>, String> {
        Ok(self.read_recent_with_skipped(limit)?.records)
    }

    /// 同 [`read_recent`](Self::read_recent)，但**同时**给出被跳过的坏行数。
    ///
    /// 复盘面据此能说清「账读回了几条、丢了几条」，而不是把不完整的历史说成完整的。
    pub fn read_recent_with_skipped(&self, limit: usize) -> Result<CouncilBallotRead, String> {
        if limit == 0 {
            return Ok(CouncilBallotRead::default());
        }
        let (window, truncated_head) = self.read_tail_window()?;
        let mut records: Vec<CouncilBallotRecord> = Vec::new();
        let mut skipped = 0usize;
        let text = String::from_utf8_lossy(&window);
        for (index, raw) in text.lines().enumerate() {
            // 窗口起点不在文件首 ⇒ 第一行是被截断的半行，丢弃（不算坏行）。
            if truncated_head && index == 0 {
                continue;
            }
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<CouncilBallotRecord>(line) {
                Ok(record) => records.push(record),
                Err(_) => skipped += 1,
            }
        }
        if records.len() > limit {
            // 保留最近的 limit 条（尾部）。
            records.drain(..records.len() - limit);
        }
        Ok(CouncilBallotRead { records, skipped })
    }

    /// 读出尾部窗口（至多 [`MAX_BALLOT_LOG_READ_BYTES`] 字节）。
    ///
    /// 返回 `(窗口字节, 窗口起点是否被截断)`。
    fn read_tail_window(&self) -> Result<(Vec<u8>, bool), String> {
        let mut file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            // 账还不存在 ⇒ 合法空态（还没有票发生过），不是错误。
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Vec::new(), false));
            }
            Err(error) => {
                return Err(format!(
                    "读 council ballot 账失败 ({}): {error}",
                    self.path.display()
                ));
            }
        };
        let length = file
            .metadata()
            .map_err(|error| format!("读 council ballot 账元信息失败: {error}"))?
            .len();
        if length == 0 {
            return Ok((Vec::new(), false));
        }
        let window = length.min(MAX_BALLOT_LOG_READ_BYTES);
        let start = length - window;
        let mut buffer = Vec::with_capacity(window as usize);
        file.seek(SeekFrom::Start(start))
            .map_err(|error| format!("定位 council ballot 账尾部失败: {error}"))?;
        (&mut file)
            .take(window)
            .read_to_end(&mut buffer)
            .map_err(|error| format!("读 council ballot 账尾部失败: {error}"))?;
        Ok((buffer, start > 0))
    }
}

/// 默认账路径：`<data_dir>/council-ballots.jsonl`。
#[must_use]
pub fn default_council_ballot_log_path(data_dir: &Path) -> PathBuf {
    data_dir.join(COUNCIL_BALLOT_LOG_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn record(advisor: &str, stance: &str, at_ms: i64) -> CouncilBallotRecord {
        CouncilBallotRecord {
            advisor: advisor.to_string(),
            stance: stance.to_string(),
            confidence: 0.75,
            confidence_defaulted: false,
            weight: 0.5,
            domain: Some("safety".to_string()),
            advisor_score: 0.25,
            at_ms,
        }
    }

    /// 空账（文件不存在）⇒ 读回空 vec，**不是错误**。
    #[test]
    fn missing_file_reads_as_empty_not_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = CouncilBallotLog::open(dir.path().join("never-written.jsonl"));
        let records = log.read_recent(10).expect("读空账不应报错");
        assert!(records.is_empty(), "没写过 ⇒ 读回空 vec: {records:?}");
    }

    /// 追加 3 条 ⇒ 读回 3 条，顺序与追加顺序一致，内容逐字段相等。
    #[test]
    fn append_three_then_read_back_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = CouncilBallotLog::open(dir.path().join("ballots.jsonl"));
        let first = record("safety", "approve", 1_700_000_000_000);
        let second = record("performance", "hold", 1_700_000_000_001);
        let third = CouncilBallotRecord {
            advisor: "cost".to_string(),
            stance: "abstain".to_string(),
            confidence: 0.1,
            confidence_defaulted: true,
            weight: 1.0,
            domain: None,
            advisor_score: 0.0,
            at_ms: 1_700_000_000_002,
        };
        log.append(&first).expect("append 1");
        log.append(&second).expect("append 2");
        log.append(&third).expect("append 3");

        let records = log.read_recent(10).expect("读回");
        assert_eq!(records.len(), 3, "应读回 3 条: {records:?}");
        assert_eq!(records[0], first, "第 1 条应逐字段相等");
        assert_eq!(records[1], second, "第 2 条应逐字段相等");
        assert_eq!(records[2], third, "第 3 条应逐字段相等");
        // 顺序 = 追加顺序（最新在后）。
        assert_eq!(
            records
                .iter()
                .map(|item| item.advisor.as_str())
                .collect::<Vec<_>>(),
            vec!["safety", "performance", "cost"]
        );
        assert_eq!(records[2].at_ms, 1_700_000_000_002, "落盘时刻应原样保留");
        assert!(records[2].confidence_defaulted, "兜底标记应原样保留");
        assert_eq!(records[2].domain, None, "无域应为 None");
    }

    /// `limit` 小于总数 ⇒ 只返回最近 N 条（最新在后）。
    #[test]
    fn read_recent_keeps_only_the_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = CouncilBallotLog::open(dir.path().join("ballots.jsonl"));
        for index in 0..5 {
            log.append(&record("advisor", "approve", i64::from(index)))
                .expect("append");
        }
        let tail = log.read_recent(2).expect("读回");
        assert_eq!(tail.len(), 2, "只回最近 2 条");
        assert_eq!(tail[0].at_ms, 3, "较老的一条在前");
        assert_eq!(tail[1].at_ms, 4, "最新的一条在后");
    }

    /// `limit == 0` ⇒ 空 vec（不读盘、不报错）。
    #[test]
    fn zero_limit_reads_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = CouncilBallotLog::open(dir.path().join("ballots.jsonl"));
        log.append(&record("advisor", "approve", 1))
            .expect("append");
        assert!(
            log.read_recent(0).expect("读回").is_empty(),
            "limit=0 ⇒ 空 vec"
        );
    }

    /// 一行损坏 JSON ⇒ 其余行照常读回，坏行被跳过（不 panic），并计入 `skipped`。
    #[test]
    fn corrupt_line_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ballots.jsonl");
        let log = CouncilBallotLog::open(&path);
        log.append(&record("safety", "approve", 1))
            .expect("append 1");
        {
            // 手工插入一行坏数据（模拟半行写入 / 磁盘损坏）。
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open for corrupt");
            writeln!(file, "{{ not json at all").expect("write corrupt");
        }
        log.append(&record("performance", "disapprove", 2))
            .expect("append 2");

        let records = log.read_recent(10).expect("读回");
        assert_eq!(
            records.len(),
            2,
            "坏行应被跳过而不是毁掉整份历史: {records:?}"
        );
        assert_eq!(records[0].advisor, "safety");
        assert_eq!(records[1].advisor, "performance");

        let with_stats = log.read_recent_with_skipped(10).expect("读回");
        assert_eq!(with_stats.skipped, 1, "坏行应被计入 skipped");
        assert_eq!(with_stats.records.len(), 2);
    }

    /// 父目录不存在 ⇒ `append` 先创建父目录再写成功。
    #[test]
    fn append_creates_missing_parent_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("deep").join("nested").join("ballots.jsonl");
        assert!(
            !nested.parent().expect("parent").exists(),
            "前置条件：父目录不存在"
        );
        let log = CouncilBallotLog::open(&nested);
        log.append(&record("safety", "approve", 42))
            .expect("append 应成功");
        assert!(nested.exists(), "父目录应被创建、账文件应存在");
        let records = log.read_recent(10).expect("读回");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].at_ms, 42);
    }

    /// `append` 两次后文件里是 **2 行**（证明是追加不是覆盖）。
    #[test]
    fn append_is_append_only_never_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ballots.jsonl");
        let log = CouncilBallotLog::open(&path);
        log.append(&record("safety", "approve", 1))
            .expect("append 1");
        log.append(&record("performance", "disapprove", 2))
            .expect("append 2");

        let raw = std::fs::read_to_string(&path).expect("读原始字节");
        let lines: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2, "两行 ⇒ 不是覆盖式写: {raw:?}");
        assert!(raw.ends_with('\n'), "每行都应以换行收尾");
        assert!(
            lines[0].contains("safety"),
            "第 1 行必须还在（覆盖式写会把它抹掉）: {raw:?}"
        );
        assert!(lines[1].contains("performance"), "第 2 行应是新追加的");
    }

    /// 默认账路径 = `<data_dir>/council-ballots.jsonl`。
    #[test]
    fn default_path_lives_under_the_data_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = default_council_ballot_log_path(dir.path());
        assert_eq!(path, dir.path().join("council-ballots.jsonl"));
        assert_eq!(
            path.file_name().expect("file name"),
            COUNCIL_BALLOT_LOG_FILE
        );
    }

    /// 超过读取上界（4 MiB）的账：**只读尾部**，尾部窗口内的票照常读回。
    ///
    /// 这条钉死「有界尾部读」策略：文件头部（填充垃圾）不被读、不影响尾部结果。
    #[test]
    fn oversized_log_is_read_from_the_tail_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("ballots.jsonl");
        // 头部：4 MiB 的一行垃圾（不是合法 JSON），自成一行。
        let filler = "x".repeat(MAX_BALLOT_LOG_READ_BYTES as usize);
        std::fs::write(&path, format!("{filler}\n")).expect("写填充");
        // 尾部：3 条真票。
        let log = CouncilBallotLog::open(&path);
        for index in 0..3 {
            log.append(&record("advisor", "approve", i64::from(index)))
                .expect("append");
        }
        let read = log.read_recent_with_skipped(10).expect("读回");
        assert_eq!(
            read.records.len(),
            3,
            "尾部窗口内的 3 条必须全部读回: {:?}",
            read.records.len()
        );
        assert_eq!(read.records[2].at_ms, 2, "最新一条在最后");
        assert_eq!(
            read.skipped, 0,
            "被截断的半行应被丢弃（不算坏行）: {:?}",
            read.skipped
        );
    }
}
