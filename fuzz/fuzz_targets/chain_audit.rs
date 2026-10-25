#![no_main]
// 仓根 `clippy.toml` 禁 `std::fs::write` / `create_dir_all` 等直调 —— 那条规矩是给
// **产品代码**用的（写盘必须走原子写助手）。fuzz harness 的职责恰恰是**真的落盘**
// 去喂被测链面的 IO 路径（append / 重放 / 回滚 / 篡改检测），故在此显式豁免并
// 说明理由。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

//! fuzz: **治理审计链**（`apeireth_gateway::chain_audit`）—— 问责的证据基础。
//!
//! 审计链是哈希链式记录：每条记录含前一条的哈希，篡改可检出。失守意味着
//! ① 记录被改写/删除却查不出来；② 链分叉导致验证永久失败；③ 顺序/时间被伪造。
//!
//! ## 判据全部取自被测模块自己写明的契约（出处逐条标在断言旁的 `src:` 注释里）
//!
//! - **每行一个记录 + 重放重建 + 篡改拒开**：`chain_audit.rs` 模块头
//!   "append-only jsonl (每行一个 [`AuditRecord`]) + 打开时全量重放重建
//!   [`AuditHashChain`] 并逐条比对哈希 (账本被篡改 = 拒开, fail-closed)"
//!   （`src: crates/adapters/gateway/src/chain_audit.rs:5-8`）+ `open` 文档
//!   "逐行严格解析 [`AuditRecord`]、重放重建哈希链并逐条比对 `previous_hash`/
//!   `current_hash` (任何篡改/断链 = 拒开), 最后 `verify()`"（`src: chain_audit.rs:48-51`）
//!   ⇒ ① 追加的字段必须**逐字**读回（分隔符注入不许让字段串位）；② 任何改写/
//!   断链/删中间一条/交换顺序必须**拒开**；③ 坏行（非 UTF-8 / 截半条）必须拒开。
//! - **哈希覆盖面**：`audit.rs` 的 "Canonical serialization" 段（`src:
//!   crates/foundation/governance/src/audit.rs:8-22`）—— 哈希固定序覆盖
//!   `sequence` / `timestamp_epoch_millis` / `event_kind` / `subject` /
//!   `previous_hash`，且 "Length prefixes are `u64` big-endian byte lengths. This is
//!   unambiguous and avoids hash instability from `HashMap` ordering or separator
//!   characters in free-text fields."
//!   ⇒ ① 这五项任一被改写必须检出；② 自由文本里的分隔符（换行/冒号/井号/引号）
//!   **不许**影响哈希与字段归属。
//! - **链完整性四查**：`audit.rs` 的 `verify` 文档 "Checks sequence continuity,
//!   genesis linkage, previous-hash pointers, and each record's current hash. An
//!   empty chain verifies successfully."（`src: audit.rs:166-168`）
//!   ⇒ 删中间一条 / 交换两条 / 改内容必须 `verify()` 报错；空链必须通过。
//! - **时钟不参与链序**：`audit.rs` 模块头 "`append` takes a canonical [`Timestamp`];
//!   it never reads the wall clock."（`src: audit.rs:25-27`）
//!   ⇒ 同一时刻的记录 / 时钟回拨不许让链失效（**不**断言时间单调：契约没承诺）。
//! - **审批 subject 严格解析**：`chain_audit.rs` "解析严格, 遇未知形状即错
//!   (不猜、不冒充)"（`src: chain_audit.rs:221-225`）+ "从链上已有 approval.* 事件
//!   重建读面 (解析失败 = 拒开)"（`src: chain_audit.rs:346-347`）
//!   ⇒ 解析不出来必须**拒开**；解析出来时**必须**与写进去的逐字相同（不许冒充）。
//! - **回滚语义**：`chain_audit.rs` "savepoint 回滚: 文件截断回第 `committed_len`
//!   条末尾 + 链按保留记录重建 (哈希重算确定性, 重建链与原链逐字节同哈希)"
//!   （`src: chain_audit.rs:138-139`）
//!   ⇒ 回滚后链必须仍 `verify()` 通过、账本必须仍开得出、其后追加仍开得出。
//! - **多写者分叉是已披露口径**：`chain_audit.rs` "0 装口径: 单写者假设 —— 链
//!   sequence = 记录数, 多进程并发追加会分叉; 生产装配单进程单 sink"
//!   （`src: chain_audit.rs:15-16`）
//!   ⇒ 分叉**本身**是已披露行为（不判它违约）；但分叉后的账本属于"断链"，按
//!   "任何篡改/断链 = 拒开" 必须**拒开**（fail-closed），不许默默当好账本收下。
//!
//! ## 本批 fuzz 落地时**收紧**的两处契约（修复前各有库层面复现，见仓内回归测试）
//!
//! - **落盘 `sequence` 进信封**：`AuditRecord::sequence` 文档 "Position in the
//!   chain, counting from zero"（`src: audit.rs:67-68`）⇒ 落盘值必须等于重放
//!   下标；修复前改写落盘 `sequence` **查不出来**（顺序可被伪造）。
//! - **审批 subject 编码可唯一解码**：`pair_id`/`subject` 带 `#` ⇒ 链内 subject
//!   不可唯一解码 ⇒ **写面拒收 + 读面拒读**（不猜、不冒充）；修复前读面会**冒充**
//!   另一条记录（pair_id 被截断、round 串位、subject 吞掉后半段）。
//!
//! ## 刻意不造的期望（自造期望 = 假阳性，本项目历史已栽 8 次）
//!
//! 1. **尾部截断检测不了**：哈希链没有外部 head 承诺 ⇒ 砍掉尾部若干条**之后**
//!    剩余前缀仍是一条合法链。本 target **不**断言"截尾必须被发现"（契约没承诺，
//!    `audit.rs:165-168` 的四查里也没有长度/头承诺）。截**半条**记录必须被发现
//!    （那是非 UTF-8/畸形行 ⇒ 严格解析拒开），这一条**有**断言。
//! 2. **亚毫秒时间戳改写检测不了**：哈希口径是 `timestamp_epoch_millis`
//!    （`audit.rs:14`），而落盘字段是 RFC3339（含纳秒位）⇒ 改纳秒位不改哈希。
//!    影响面 < 1ms（属"哈希覆盖面"的口径边界，不是绕过），本 target **不**断言
//!    它被检出，只断言**毫秒及以上**的改写必被检出。
//! 3. **账本字节布局不在完整性信封内**：加/删空行不改变任何记录 ⇒ 链仍合法。
//!    本 target 不断言"空行必须拒开"，只断言"空行不许让回滚截坏账本"。
//! 4. **平台/时区差异**：时间戳按 RFC3339 落盘，契约未承诺时区形态。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Once};

use apeireth_core::kernel::Timestamp;
use apeireth_gateway::{ChainApprovalAudit, FileChainAuditSink, ImApprovalAuditCommit};
use apeireth_governance::{
    ApprovalAuditPair, ApprovalAuditRecord, ApprovalAuditSink, ApprovalOutcome, AuditHashChain,
    AuditRecord, AuditSink, FixedChainClock, GENESIS_PREVIOUS_HASH,
};
use libfuzzer_sys::fuzz_target;

// ===========================================================================
// 临时账本（本次运行独有：pid + 纳秒 + 原子计数）
// ===========================================================================

static SEQ: AtomicU64 = AtomicU64::new(0);

/// 造一个本次运行独有的账本路径; 目录建不出来（环境问题）⇒ 返回 None，
/// 本轮**不作断言**（环境故障不是库的缺陷）。
fn ledger_path(tag: &str) -> Option<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "apeireth-chain-fuzz-{}-{nanos}-{seq}-{tag}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).ok()?;
    Some(dir.join("governance-audit.jsonl"))
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

/// 落盘时间戳：契约外的 i64 毫秒（超 chrono 范围）会被 `from_epoch_millis`
/// 判为 None ⇒ 换成安全值。**这是 harness 的锅**（不能因为自造超范围时间戳
/// 而炸在 `expect` 上），不是库的缺陷。
fn safe_ms(ms: i64) -> Timestamp {
    Timestamp::from_epoch_millis(ms)
        .or_else(|| Timestamp::from_epoch_millis(0))
        .expect("epoch 0 must be representable")
}

/// 改哈希的首字符（保证与原值不同：篡改量不许是空操作）。
fn flip_first(hash: &str) -> String {
    let mut chars: Vec<char> = hash.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = if *first == '0' { '1' } else { '0' };
    }
    chars.into_iter().collect()
}

/// 账本的非空行数（"每行一个 AuditRecord" ⇒ 非空行数 == 记录数）。
fn non_empty_lines(path: &Path) -> usize {
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// 读出账本的每一行（`lines()` 口径：不含末尾空串；`write_lines` 与之互逆）。
fn read_lines(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines().map(str::to_string).collect()
}

/// 写回账本的行（末尾补一个换行：落盘形态 = 每行一个记录 + 行尾换行）。
fn write_lines(path: &Path, lines: &[String]) {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    fs::write(path, text.as_bytes()).expect("write ledger");
}

// ===========================================================================
// 载荷形状：记录内容面（空 / 超长 / Unicode / 零宽 / RTL / 分隔符 / 换行 /
// NUL / JSON 转义面）。全部来自「自由文本里的分隔符不许影响哈希与字段归属」
// 这一条契约（audit.rs:20-22）。
// ===========================================================================

const SHAPES: usize = 12;

/// 把 fuzz 载荷摊到内容面的形态上。`payload` 已截断到 1 KiB（防爆）。
fn shape(sel: u8, payload: &str) -> String {
    match (sel as usize) % SHAPES {
        0 => payload.to_string(),
        1 => String::new(),
        2 => payload.repeat(64),
        3 => format!("{payload}\n{payload}"),
        4 => format!("{payload}\r\n{payload}"),
        5 => format!("{payload}#7:{payload}"),
        6 => format!("{payload}:{payload}"),
        7 => format!("{payload}\u{200B}{payload}"),
        8 => format!("\u{202E}{payload}\u{202C}"),
        9 => format!("{payload}\u{0000}{payload}"),
        10 => format!("\"{payload}\"\\{payload}"),
        _ => "z".repeat(4096),
    }
}

/// 载荷截断（1 KiB）：超长内容面由 `shape` 自己造（repeat / 4096）。
fn payload_of(data: &[u8]) -> String {
    let cut = data.len().min(1024);
    String::from_utf8_lossy(&data[..cut]).into_owned()
}

// ===========================================================================
// 固定契约检查（每进程一次：这些断言靠夹具事实，不靠随机输入）
// ===========================================================================

/// F1 空账本：开得出、链为空、`verify()` 通过。
///
/// 契约：`audit.rs` "An empty chain verifies successfully."（`src: audit.rs:168`）
/// 以及 `chain_audit.rs` "打开时全量重放重建 [`AuditHashChain`]"（`src: chain_audit.rs:6-7`）
fn f01_empty_ledger_opens_and_verifies() {
    let Some(path) = ledger_path("f01") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open empty ledger");
    assert_eq!(sink.len(), 0, "新账本必须是空链");
    assert!(sink.records().is_empty(), "空账本读面必须为空");
    sink.verify().expect("空链必须 verify 通过");
    cleanup(&path);
}

/// F2 内容面逐字往返：追加的字段必须**逐字**读回（分隔符注入不许让字段串位）。
///
/// 契约：`chain_audit.rs` "append-only jsonl (每行一个 [`AuditRecord`]) + 打开时
/// 全量重放重建"（`src: chain_audit.rs:5-7`）+ `audit.rs` "avoids hash instability
/// from ... separator characters in free-text fields"（`src: audit.rs:20-22`）
fn f02_content_round_trip_is_exact() {
    let Some(path) = ledger_path("f02") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    let mut expected: Vec<(String, String, i64)> = Vec::new();
    for index in 0..SHAPES {
        let event_kind = shape(index as u8, &format!("kind-{index}"));
        let subject = shape((index as u8).wrapping_add(5), &format!("subject-{index}"));
        let millis = 1_700_000_000_000 + index as i64;
        sink.append(&event_kind, &subject, safe_ms(millis))
            .expect("append");
        expected.push((event_kind, subject, millis));
    }
    drop(sink);
    let reopened = FileChainAuditSink::open(path.clone()).expect("reopen");
    let records = reopened.records();
    assert_eq!(records.len(), expected.len(), "读回记录数必须等于追加数");
    for (index, record) in records.iter().enumerate() {
        let (kind, subject, millis) = &expected[index];
        assert_eq!(&record.event_kind, kind, "第 {index} 条 event_kind 必须逐字读回");
        assert_eq!(&record.subject, subject, "第 {index} 条 subject 必须逐字读回");
        assert_eq!(
            record.timestamp.epoch_millis(),
            *millis,
            "第 {index} 条时间戳必须读回"
        );
        assert_eq!(record.sequence, index as u64, "第 {index} 条序号必须连续");
    }
    assert_eq!(
        non_empty_lines(&path),
        expected.len(),
        "每行一个 AuditRecord ⇒ 非空行数必须等于记录数"
    );
    reopened.verify().expect("重放后链必须完整");
    cleanup(&path);
}

/// F3 篡改检测：哈希覆盖的五个字段任一被改写 ⇒ 拒开。
///
/// 契约：`chain_audit.rs` "任何篡改/断链 = 拒开"（`src: chain_audit.rs:49-50`）
/// 以及 `audit.rs` 的 canonical serialization（`src: audit.rs:8-22`：覆盖 sequence /
/// timestamp_epoch_millis / event_kind / subject / previous_hash）。
/// ⚠️ 时间戳只断言**毫秒及以上**的改写（哈希口径就是 `timestamp_epoch_millis`，
/// 纳秒位不在信封内 —— 那是口径边界，见文件头"刻意不造的期望"第 2 条）。
fn f03_tamper_is_refused() {
    let Some(path) = ledger_path("f03") else { return };
    let seed = |path: &Path| {
        let sink = FileChainAuditSink::open(path.to_path_buf()).expect("open");
        sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
            .expect("append 0");
        sink.append("governance.denied.gate", "tool.shell", safe_ms(1_700_000_000_001))
            .expect("append 1");
    };
    seed(&path);
    let original = read_lines(&path);
    assert_eq!(original.len(), 2, "两条记录各占一行");

    // 五个被哈希覆盖的字段各改一次（改第 1 条）。
    for field in ["event_kind", "subject", "sequence", "previous_hash", "current_hash", "timestamp"] {
        let mut lines = original.clone();
        let mut record: AuditRecord =
            serde_json::from_str(&lines[1]).expect("record parses");
        match field {
            "event_kind" => record.event_kind.push_str("-tampered"),
            "subject" => record.subject.push_str("-tampered"),
            "sequence" => record.sequence = record.sequence.wrapping_add(7),
            // 哈希：改**首字符**（保证与原值不同，不靠运气）。
            "previous_hash" => {
                record.previous_hash = flip_first(&record.previous_hash);
            }
            "current_hash" => {
                record.current_hash = flip_first(&record.current_hash);
            }
            "timestamp" => {
                record.timestamp = safe_ms(record.timestamp.epoch_millis() + 1);
            }
            _ => unreachable!(),
        }
        lines[1] = serde_json::to_string(&record).expect("serialize");
        assert_ne!(
            lines[1], original[1],
            "harness 自检: 篡改量必须真的改到文件里 (否则拒开断言是空断言): {field}"
        );
        write_lines(&path, &lines);
        assert!(
            FileChainAuditSink::open(path.clone()).is_err(),
            "改写被哈希覆盖的字段 ({field}) 必须拒开 (fail-closed)"
        );
    }
    // 未改动的原账本仍开得出（自检：上面的拒开不是因为别的坏）。
    write_lines(&path, &original);
    let sink = FileChainAuditSink::open(path.clone()).expect("reopen pristine ledger");
    assert_eq!(sink.len(), 2);
    sink.verify().expect("pristine chain verifies");
    cleanup(&path);
}

/// F4 删中间一条 ⇒ 拒开（sequence 连续性 + previous_hash 指针）。
///
/// 契约：`audit.rs` "Checks sequence continuity, genesis linkage, previous-hash
/// pointers, and each record's current hash."（`src: audit.rs:167-168`）
fn f04_middle_deletion_is_refused() {
    let Some(path) = ledger_path("f04") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    for index in 0..3 {
        sink.append("governance.allowed", &format!("tool.{index}"), safe_ms(1_700_000_000_000 + index))
            .expect("append");
    }
    drop(sink);
    let lines = read_lines(&path);
    let mut kept = lines.clone();
    kept.remove(1);
    write_lines(&path, &kept);
    assert!(
        FileChainAuditSink::open(path.clone()).is_err(),
        "删中间一条必须拒开 (断链)"
    );
    cleanup(&path);
}

/// F5 交换两条顺序 ⇒ 拒开。
///
/// 契约：`audit.rs` 同上四查（`src: audit.rs:167-168`）；链序是问责的语义
/// （`chain_audit.rs` "记录读面 (测试/自述; 顺序即链序)"，`src: chain_audit.rs:130`）
fn f05_reorder_is_refused() {
    let Some(path) = ledger_path("f05") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    for index in 0..3 {
        sink.append("governance.allowed", &format!("tool.{index}"), safe_ms(1_700_000_000_000 + index))
            .expect("append");
    }
    drop(sink);
    let lines = read_lines(&path);
    let mut swapped = lines.clone();
    swapped.swap(1, 2);
    write_lines(&path, &swapped);
    assert!(
        FileChainAuditSink::open(path.clone()).is_err(),
        "交换两条顺序必须拒开 (断链)"
    );
    cleanup(&path);
}

/// F6 截半条记录 ⇒ 拒开（严格解析）。
///
/// 契约：`chain_audit.rs` "逐行严格解析 [`AuditRecord`]"（`src: chain_audit.rs:49`）
fn f06_partial_record_is_refused() {
    let Some(path) = ledger_path("f06") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("append 0");
    sink.append("governance.denied.gate", "tool.shell", safe_ms(1_700_000_000_001))
        .expect("append 1");
    drop(sink);
    let text = fs::read_to_string(&path).expect("read");
    let cut = &text[..text.len() / 2];
    fs::write(&path, cut.as_bytes()).expect("write cut");
    assert!(
        FileChainAuditSink::open(path.clone()).is_err(),
        "截半条记录必须拒开 (畸形行)"
    );
    cleanup(&path);
}

/// F7 同一时刻的记录 / 重复载荷 / 时钟回拨 ⇒ 链仍完整。
///
/// 契约：`audit.rs` "`append` takes a canonical [`Timestamp`]; it never reads the
/// wall clock."（`src: audit.rs:26-27`）⇒ 链不依赖时钟，**不**要求时间单调。
fn f07_same_timestamp_and_clock_rollback_still_verify() {
    let Some(path) = ledger_path("f07") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("append 0");
    sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("append 1 (same payload, same instant)");
    sink.append("governance.allowed", "tool.repo", safe_ms(1_699_999_999_999))
        .expect("append 2 (clock rollback)");
    sink.verify().expect("同刻/重复/回拨都不许让链失效");
    drop(sink);
    let reopened = FileChainAuditSink::open(path.clone()).expect("reopen");
    assert_eq!(reopened.len(), 3);
    assert_eq!(reopened.records()[1].timestamp, reopened.records()[0].timestamp);
    cleanup(&path);
}

/// F8 审批对经链落账：写进去的必须逐字读回。
///
/// 契约：`chain_audit.rs` "审批对记录的链内 subject 编码: `{pair_id}#{round}:
/// {subject}` (asked) / `{pair_id}#{round}:{subject}:{outcome}` (decision)"
/// 以及 "解析严格, 遇未知形状即错 (不猜、不冒充)"（`src: chain_audit.rs:221-225`）
fn f08_approval_pair_round_trip() {
    let Some(path) = ledger_path("f08") else { return };
    let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
    let audit = ChainApprovalAudit::open(
        sink.clone(),
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    )
    .expect("open pair ledger");
    // 契约内的 id/capability（不含 '#' 与 ':'）。
    let pair = ApprovalAuditPair::new(
        "pair-1",
        7,
        "tool.shell",
        ApprovalOutcome::AllowedOnce,
    );
    audit.commit(&pair).expect("pair commit");
    assert_eq!(sink.len(), 2, "审批对两半都进链");
    sink.verify().expect("审批对落链后链完整");
    drop(audit);
    drop(sink);
    let reopened = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen"));
    let rebuilt = ChainApprovalAudit::open(
        reopened,
        Arc::new(FixedChainClock::new(1_700_000_000_001)),
    )
    .expect("rebuild pair ledger");
    assert_eq!(
        rebuilt.committed(),
        vec![pair.asked().clone(), pair.decision().clone()],
        "审批对必须逐字读回 (不猜、不冒充)"
    );
    cleanup(&path);
}

/// F9 审批 subject 形状非法 ⇒ 解析失败必须**拒开**（不许猜、不许冒充）。
///
/// 契约：`chain_audit.rs` "解析严格, 遇未知形状即错 (不猜、不冒充)"
/// （`src: chain_audit.rs:224-225`）+ "从链上已有 approval.* 事件重建读面
/// (解析失败 = 拒开)"（`src: chain_audit.rs:346-347`）
fn f09_approval_parse_is_strict() {
    let Some(path) = ledger_path("f09") else { return };
    // 直接往链里塞 approval.* 事件的畸形 subject（跳过编码面，测解析面）。
    // ⚠️ 每个形状必须配**对应**的事件种类：decision 的形状（多一段 outcome）
    // 塞给 asked 是合法的（asked 取第一个 ':' 之后的全部当 subject）⇒ 那是
    // harness 写错，不是库的错（本 target 第一版就栽在这：把
    // "pair#1:tool:unknown" 塞给 approval.asked，库正确地解析成
    // subject="tool:unknown" —— 是我的期望值配错了事件种类）。
    for (event_kind, subject) in [
        ("approval.asked", "no-hash-no-colon"),     // 缺 '#'
        ("approval.asked", "pair#notanumber:tool"), // round 非数字
        ("approval.asked", "pair#1"),                // 缺 ':'
        ("approval.decision", "pair#1:tool"),        // 缺 outcome 段
        ("approval.decision", "pair#1:tool:unknown"), // outcome 未知
        ("approval.decision", "pair#x:tool:allowed_once"), // round 非数字
        ("approval.asked", "pair#1:to#ol"),                 // subject 段带 '#' (串位形状)
        ("approval.decision", "pair#1:to#ol:allowed_once"), // subject 段带 '#' (串位形状)
    ] {
        let sink = FileChainAuditSink::open(path.clone()).expect("open");
        sink.append(event_kind, subject, safe_ms(1_700_000_000_000))
            .expect("append");
        drop(sink);
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen chain"));
        let parsed = ChainApprovalAudit::open(
            sink,
            Arc::new(FixedChainClock::new(1_700_000_000_000)),
        );
        assert!(
            parsed.is_err(),
            "畸形 approval subject 必须拒开 (不猜、不冒充): {event_kind} {subject:?}"
        );
        let _ = fs::remove_file(&path);
    }
    cleanup(&path);
}

/// F10 并发追加不许丢记录（`AuditSink: Send + Sync` ⇒ 并发用是预期用法）。
///
/// 契约：`chain.rs` "pub trait AuditSink: Send + Sync" + "Injected port: append
/// one tamper-evident audit record."（`src: crates/foundation/governance/src/chain.rs:227-231`）
/// 以及 `chain_audit.rs` "append-only"（`src: chain_audit.rs:5`）⇒ 追加不许丢。
fn f10_concurrent_appends_keep_every_record() {
    let Some(path) = ledger_path("f10") else { return };
    let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
    let threads = 4usize;
    let per_thread = 8usize;
    let barrier = Arc::new(Barrier::new(threads));
    let mut handles = Vec::new();
    for thread in 0..threads {
        let sink = sink.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            for index in 0..per_thread {
                sink.append(
                    "governance.allowed",
                    &format!("tool.{thread}-{index}"),
                    safe_ms(1_700_000_000_000 + index as i64),
                )
                .expect("concurrent append");
            }
        }));
    }
    for handle in handles {
        handle.join().expect("thread joined");
    }
    assert_eq!(sink.len(), threads * per_thread, "并发追加不许丢记录");
    sink.verify().expect("并发追加后链完整");
    drop(sink);
    let reopened = FileChainAuditSink::open(path.clone()).expect("reopen");
    assert_eq!(reopened.len(), threads * per_thread);
    assert_eq!(
        non_empty_lines(&path),
        threads * per_thread,
        "每行一个 AuditRecord ⇒ 并发追加不许写串行"
    );
    cleanup(&path);
}

/// F11 多写者分叉：分叉是**已披露**口径，但分叉后的账本属"断链" ⇒ 必须拒开。
///
/// 契约：`chain_audit.rs` "0 装口径: 单写者假设 —— 链 sequence = 记录数,
/// 多进程并发追加会分叉"（`src: chain_audit.rs:15-16`）⇒ 分叉本身不判违约；
/// "任何篡改/断链 = 拒开"（`src: chain_audit.rs:49-50`）⇒ 分叉账本必须拒开
/// （fail-closed），不许默默当好账本收下。
fn f11_multi_writer_fork_is_refused_at_open() {
    let Some(path) = ledger_path("f11") else { return };
    let first = FileChainAuditSink::open(path.clone()).expect("open first writer");
    let second = FileChainAuditSink::open(path.clone()).expect("open second writer");
    first
        .append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("first writer append");
    second
        .append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("second writer append");
    assert!(
        FileChainAuditSink::open(path.clone()).is_err(),
        "多写者分叉后的账本必须拒开 (断链, fail-closed)"
    );
    cleanup(&path);
}

/// F12 savepoint 回滚：链必须仍完整、账本必须仍开得出、其后追加仍开得出。
///
/// 契约：`chain_audit.rs` "savepoint 回滚: 文件截断回第 `committed_len` 条末尾
/// 以及链按保留记录重建 (哈希重算确定性, 重建链与原链逐字节同哈希)"
/// （`src: chain_audit.rs:138-139`）
fn f12_rollback_keeps_the_ledger_openable() {
    let Some(path) = ledger_path("f12") else { return };
    let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
    sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("r0");
    sink.append("governance.denied.gate", "tool.shell", safe_ms(1_700_000_000_001))
        .expect("r1");
    sink.append("governance.allowed", "tool.filesystem", safe_ms(1_700_000_000_002))
        .expect("r2");
    let mut audit = ChainApprovalAudit::open(
        sink.clone(),
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    )
    .expect("open pair ledger");
    audit.rollback(2);
    assert_eq!(sink.len(), 2, "回滚后链回到存档点");
    sink.verify().expect("回滚后链仍完整");
    drop(audit);
    drop(sink);
    let reopened = FileChainAuditSink::open(path.clone()).expect("回滚后账本必须仍开得出");
    assert_eq!(reopened.len(), 2);
    reopened
        .append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_003))
        .expect("回滚后追加");
    drop(reopened);
    let after = FileChainAuditSink::open(path.clone()).expect("回滚+追加后必须仍开得出");
    assert_eq!(after.len(), 3);
    after.verify().expect("chain verifies");
    cleanup(&path);
}

/// F13 撕裂写（末行缺换行）之后追加：账本必须仍是「每行一个 AuditRecord」。
///
/// 契约：`chain_audit.rs` "append-only jsonl (每行一个 [`AuditRecord`])"
/// （`src: chain_audit.rs:5-6`）⇒ 追加不许把两条记录挤成一行（那会让账本从此
/// 开不出 ⇒ 审计永久降级到内存）。
fn f13_append_after_a_torn_line_keeps_the_ledger_openable() {
    let Some(path) = ledger_path("f13") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    sink.append("governance.allowed", "tool.repo", safe_ms(1_700_000_000_000))
        .expect("append");
    drop(sink);
    let mut bytes = fs::read(&path).expect("read");
    bytes.pop();
    fs::write(&path, &bytes).expect("write torn");
    let reopened = FileChainAuditSink::open(path.clone()).expect("撕裂写末行仍开得出");
    assert_eq!(reopened.len(), 1);
    reopened
        .append("governance.denied.gate", "tool.shell", safe_ms(1_700_000_000_001))
        .expect("撕裂写后追加");
    drop(reopened);
    let after = FileChainAuditSink::open(path.clone()).expect("撕裂写+追加后必须仍开得出");
    assert_eq!(after.len(), 2, "撕裂写后追加必须仍是两条记录 (每行一个)");
    after.verify().expect("chain verifies");
    cleanup(&path);
}

/// F14 非 UTF-8 账本字节 ⇒ 拒开（不许 panic、不许默默当好账本）。
///
/// 契约：`chain_audit.rs` "逐行严格解析 [`AuditRecord`]"（`src: chain_audit.rs:49`）
fn f14_non_utf8_ledger_is_refused() {
    let Some(path) = ledger_path("f14") else { return };
    for bytes in [
        vec![0xFFu8, 0xFE, 0x00, 0x01],
        b"{\"sequence\":0,\"timestamp\":".to_vec(),
        vec![0x80],
    ] {
        fs::write(&path, &bytes).expect("write raw");
        let opened = FileChainAuditSink::open(path.clone());
        assert!(
            opened.is_err(),
            "非 UTF-8 / 畸形账本必须拒开: {bytes:?}"
        );
    }
    cleanup(&path);
}

/// F15 记录不许被静默丢弃：开得出 ⇒ 落盘非空行数必须等于链记录数。
///
/// 契约：`chain_audit.rs` "逐行严格解析 [`AuditRecord`]、重放重建哈希链"
/// （`src: chain_audit.rs:49-50`）—— 解析不了就拒开，不许"读坏用默认"地跳过。
fn f15_no_record_is_silently_dropped() {
    let Some(path) = ledger_path("f15") else { return };
    let sink = FileChainAuditSink::open(path.clone()).expect("open");
    for index in 0..3 {
        sink.append("governance.allowed", &format!("tool.{index}"), safe_ms(1_700_000_000_000 + index))
            .expect("append");
    }
    drop(sink);
    let reopened = FileChainAuditSink::open(path.clone()).expect("reopen");
    assert_eq!(
        reopened.len(),
        non_empty_lines(&path),
        "开得出 ⇒ 落盘记录数必须等于链记录数 (不许静默丢弃)"
    );
    cleanup(&path);
}

/// F16 内存链本体（`AuditHashChain`）：四查的**可公开观测**部分 + 确定性 + 链接关系。
///
/// 契约：`audit.rs` "Checks sequence continuity, genesis linkage, previous-hash
/// pointers, and each record's current hash. An empty chain verifies
/// successfully."（`src: audit.rs:166-168`）+ "same inputs produce deterministic
/// chains"（既有回归 `audit.rs:256-272`）+ `AuditRecord` 文档 "Previous record's
/// `current_hash`, or [`GENESIS_PREVIOUS_HASH`] for the first record."
/// （`src: audit.rs:76-78`）
///
/// ⚠️ 链的记录在模块外**不可变**（`AuditHashChain::records` 只给 `&[AuditRecord]`，
/// 没有 `records_mut`）⇒ "改记录 ⇒ verify 失败" 只能从**落盘面**观测（见 F03–F05），
/// 本条只断言公开可见的部分（链接关系 / 确定性 / 空链）。
fn f16_in_memory_chain_contract() {
    let mut chain = AuditHashChain::new();
    assert!(chain.is_empty());
    assert!(chain.verify().is_ok(), "空链必须 verify 通过");
    for index in 0..3 {
        chain.append(
            "governance.allowed",
            format!("tool.{index}"),
            safe_ms(1_700_000_000_000 + index),
        );
    }
    assert!(chain.verify().is_ok());
    let records = chain.records();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].sequence, 0, "序号必须从 0 起连续");
    assert_eq!(
        records[0].previous_hash, GENESIS_PREVIOUS_HASH,
        "首条记录的 previous_hash 必须是创世哈希"
    );
    for index in 1..records.len() {
        assert_eq!(
            records[index].previous_hash, records[index - 1].current_hash,
            "第 {index} 条必须指向前一条的 current_hash"
        );
        assert_eq!(records[index].sequence, index as u64, "序号必须连续");
        assert_eq!(
            records[index].current_hash.len(),
            64,
            "SHA-256 十六进制必须是 64 字符"
        );
        assert!(
            records[index].current_hash.chars().all(|c| c.is_ascii_hexdigit()),
            "哈希必须是十六进制"
        );
    }
    // 确定性：同样输入 ⇒ 同样哈希（既有回归 `audit.rs:256-272`）。
    let mut twin = AuditHashChain::new();
    for index in 0..3 {
        twin.append(
            "governance.allowed",
            format!("tool.{index}"),
            safe_ms(1_700_000_000_000 + index),
        );
    }
    assert_eq!(chain.records(), twin.records(), "同输入必须同链");
    assert_eq!(
        chain.last().map(|record| record.current_hash.clone()),
        twin.last().map(|record| record.current_hash.clone()),
        "同输入必须同头哈希"
    );
}

/// F17 审批读面不许"冒充"：链里写了什么，读面就得是什么（不许静默改写）。
///
/// 契约：`chain_audit.rs` "解析严格, 遇未知形状即错 (不猜、不冒充)"
/// （`src: chain_audit.rs:224-225`）
fn f17_approval_read_face_matches_the_chain() {
    let Some(path) = ledger_path("f17") else { return };
    let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
    let audit = ChainApprovalAudit::open(
        sink.clone(),
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    )
    .expect("open pair ledger");
    let mut pairs = Vec::new();
    for index in 0..3u64 {
        let pair = ApprovalAuditPair::new(
            format!("pair-{index}"),
            index,
            format!("tool.{index}"),
            ApprovalOutcome::AllowedOnce,
        );
        audit.commit(&pair).expect("commit");
        pairs.push(pair);
    }
    drop(audit);
    drop(sink);
    let reopened = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen"));
    let rebuilt = ChainApprovalAudit::open(
        reopened,
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    )
    .expect("rebuild");
    let expected: Vec<ApprovalAuditRecord> = pairs
        .iter()
        .flat_map(|pair| [pair.asked().clone(), pair.decision().clone()])
        .collect();
    assert_eq!(rebuilt.committed(), expected, "读面必须与链逐字一致");
    cleanup(&path);
}

fn fixed_contracts() {
    f01_empty_ledger_opens_and_verifies();
    f02_content_round_trip_is_exact();
    f03_tamper_is_refused();
    f04_middle_deletion_is_refused();
    f05_reorder_is_refused();
    f06_partial_record_is_refused();
    f07_same_timestamp_and_clock_rollback_still_verify();
    f08_approval_pair_round_trip();
    f09_approval_parse_is_strict();
    f10_concurrent_appends_keep_every_record();
    f11_multi_writer_fork_is_refused_at_open();
    f12_rollback_keeps_the_ledger_openable();
    f13_append_after_a_torn_line_keeps_the_ledger_openable();
    f14_non_utf8_ledger_is_refused();
    f15_no_record_is_silently_dropped();
    f16_in_memory_chain_contract();
    f17_approval_read_face_matches_the_chain();
}

// ===========================================================================
// 随机面：按模式分派（内容面 / 篡改面 / 结构面 / 审批面 / 原始字节面）
// ===========================================================================

const MODES: u8 = 5;

/// 模式 0：内容面往返（分隔符注入 / Unicode / 超长 / 空 ⇒ 字段不许串位）。
fn mode_content(path: &Path, payload: &str) {
    let sink = FileChainAuditSink::open(path.to_path_buf()).expect("open");
    let count = 1 + (payload.len() % 3);
    let mut expected: Vec<(String, String, i64)> = Vec::new();
    for index in 0..count {
        let event_kind = shape(index as u8, payload);
        let subject = shape((index as u8).wrapping_add(7), payload);
        let millis = 1_700_000_000_000 + index as i64;
        sink.append(&event_kind, &subject, safe_ms(millis))
            .expect("append");
        expected.push((event_kind, subject, millis));
    }
    drop(sink);
    let reopened = FileChainAuditSink::open(path.to_path_buf()).expect("reopen");
    let records = reopened.records();
    assert_eq!(records.len(), expected.len(), "读回记录数必须等于追加数");
    for (index, record) in records.iter().enumerate() {
        let (kind, subject, millis) = &expected[index];
        assert_eq!(&record.event_kind, kind, "event_kind 必须逐字读回");
        assert_eq!(&record.subject, subject, "subject 必须逐字读回");
        assert_eq!(record.timestamp.epoch_millis(), *millis, "时间戳必须读回");
    }
    assert_eq!(
        non_empty_lines(path),
        expected.len(),
        "每行一个 AuditRecord"
    );
    reopened.verify().expect("重放后链完整");
}

/// 模式 1：篡改面（改哈希覆盖的字段 ⇒ 必须拒开）。
fn mode_tamper(path: &Path, payload: &str) {
    let sink = FileChainAuditSink::open(path.to_path_buf()).expect("open");
    for index in 0..3 {
        sink.append(
            "governance.allowed",
            &shape(index as u8, payload),
            safe_ms(1_700_000_000_000 + index),
        )
        .expect("append");
    }
    drop(sink);
    let lines = read_lines(path);
    let fields = ["event_kind", "subject", "sequence", "previous_hash", "current_hash", "timestamp"];
    let field = fields[payload.len() % fields.len()];
    let target = 1 + (payload.len() % 2);
    let mut mutated = lines.clone();
    let Ok(mut record) = serde_json::from_str::<AuditRecord>(&mutated[target]) else {
        return;
    };
    // ⚠️ 篡改量必须**真的改到文件里**: 空载荷时 `shape()` 可能给出空串 ⇒
    // 篡改变成空操作 ⇒ 账本当然开得出（那是 harness 的锅，不是库的）。
    let extra = {
        let shaped = shape(payload.len() as u8, payload);
        if shaped.is_empty() {
            "-tamper".to_string()
        } else {
            shaped
        }
    };
    match field {
        "event_kind" => record.event_kind.push_str(&extra),
        "subject" => record.subject.push_str(&extra),
        "sequence" => record.sequence = record.sequence.wrapping_add(1 + payload.len() as u64),
        "previous_hash" => record.previous_hash = "a".repeat(64),
        "current_hash" => record.current_hash = "b".repeat(64),
        "timestamp" => {
            record.timestamp = safe_ms(record.timestamp.epoch_millis() + 1 + payload.len() as i64);
        }
        _ => unreachable!(),
    }
    mutated[target] = serde_json::to_string(&record).expect("serialize");
    assert_ne!(
        mutated[target], lines[target],
        "harness 自检: 篡改量必须真的改到文件里 (否则下面的拒开断言是空断言)"
    );
    write_lines(path, &mutated);
    assert!(
        FileChainAuditSink::open(path.to_path_buf()).is_err(),
        "改写被哈希覆盖的字段 ({field}) 必须拒开"
    );
}

/// 模式 2：结构面（删中间一条 / 交换两条 / 截半条 ⇒ 必须拒开）。
fn mode_structure(path: &Path, payload: &str) {
    let sink = FileChainAuditSink::open(path.to_path_buf()).expect("open");
    for index in 0..4 {
        sink.append(
            "governance.allowed",
            &format!("tool.{index}"),
            safe_ms(1_700_000_000_000 + index),
        )
        .expect("append");
    }
    drop(sink);
    let lines = read_lines(path);
    match payload.len() % 3 {
        0 => {
            let mut kept = lines.clone();
            kept.remove(1 + (payload.len() % 2).min(kept.len().saturating_sub(2)));
            write_lines(path, &kept);
            assert!(
                FileChainAuditSink::open(path.to_path_buf()).is_err(),
                "删中间一条必须拒开"
            );
        }
        1 => {
            let mut swapped = lines.clone();
            swapped.swap(1, 2);
            write_lines(path, &swapped);
            assert!(
                FileChainAuditSink::open(path.to_path_buf()).is_err(),
                "交换两条顺序必须拒开"
            );
        }
        _ => {
            let text = fs::read_to_string(path).expect("read");
            let cut = &text[..text.len() / 2];
            fs::write(path, cut.as_bytes()).expect("write cut");
            assert!(
                FileChainAuditSink::open(path.to_path_buf()).is_err(),
                "截半条记录必须拒开"
            );
        }
    }
}

/// 模式 3：审批面（合法的 id/capability ⇒ 逐字读回；非法的 ⇒ 拒开不冒充）。
fn mode_approval(path: &Path, payload: &str) {
    let sink = Arc::new(FileChainAuditSink::open(path.to_path_buf()).expect("open"));
    let audit = ChainApprovalAudit::open(
        sink.clone(),
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    )
    .expect("open pair ledger");
    let pair_id = shape(payload.len() as u8, payload);
    let subject = shape((payload.len() as u8).wrapping_add(3), payload);
    let pair = ApprovalAuditPair::new(
        pair_id.clone(),
        payload.len() as u64,
        subject.clone(),
        ApprovalOutcome::AllowedOnce,
    );
    // 写面: 契约 (pair_id/subject 不含 '#') 被破 ⇒ 必须拒写 (fail-closed)。
    let written = audit.commit(&pair).is_ok();
    if !written {
        assert!(
            pair_id.contains('#') || subject.contains('#'),
            "契约内的 pair_id/subject (不含 '#') 写失败 ⇒ 契约被破: \
             pair_id={pair_id:?} subject={subject:?}"
        );
        assert!(
            audit.committed().is_empty(),
            "拒写后读面必须为空 (不许半截冒充记录): pair_id={pair_id:?}"
        );
    }
    drop(audit);
    drop(sink);
    let reopened = Arc::new(FileChainAuditSink::open(path.to_path_buf()).expect("reopen"));
    let rebuilt = ChainApprovalAudit::open(
        reopened,
        Arc::new(FixedChainClock::new(1_700_000_000_000)),
    );
    match (written, rebuilt) {
        // 写进去了 + 解析出来了 ⇒ 必须与写进去的**逐字**相同（不猜、不冒充）。
        (true, Ok(ledger)) => {
            let expected = vec![pair.asked().clone(), pair.decision().clone()];
            assert_eq!(
                ledger.committed(),
                expected,
                "解析成功时读面必须与写进去的逐字一致 (pair_id={pair_id:?} subject={subject:?})"
            );
        }
        // 写进去了 + 解析不出来 ⇒ 契约内的 id/capability 不该走到这里。
        (true, Err(_)) => {
            assert!(
                pair_id.contains('#') || subject.contains('#'),
                "契约内的 pair_id/subject (不含 '#') 解析失败 ⇒ 契约被破: \
                 pair_id={pair_id:?} subject={subject:?}"
            );
        }
        // 拒写了 ⇒ 链里没有 approval 记录 ⇒ 读面必须为空且**开得出**。
        (false, Ok(ledger)) => {
            assert!(
                ledger.committed().is_empty(),
                "拒写后读面必须为空 (不许半截冒充记录): pair_id={pair_id:?}"
            );
        }
        (false, Err(_)) => {
            panic!(
                "拒写后链里没有 approval.* 记录 ⇒ 读面不该拒开 (pair_id={pair_id:?} subject={subject:?})"
            );
        }
    }
}

/// 模式 4：原始字节面（任意字节当账本 ⇒ 不许 panic；开得出就不许丢记录）。
fn mode_raw(path: &Path, payload: &str) {
    fs::write(path, payload.as_bytes()).expect("write raw");
    let opened = FileChainAuditSink::open(path.to_path_buf());
    if let Ok(sink) = opened {
        assert_eq!(
            sink.len(),
            non_empty_lines(path),
            "开得出 ⇒ 落盘记录数必须等于链记录数 (不许静默丢弃)"
        );
        sink.verify().expect("开得出的账本必须 verify 通过");
    }
}

// ===========================================================================
// fuzz 入口
// ===========================================================================

fuzz_target!(|data: &[u8]| {
    // 固定契约检查（一次性）：不依赖随机输入的事实断言。
    static FIXED: Once = Once::new();
    FIXED.call_once(fixed_contracts);

    let Some(&mode) = data.first() else {
        return;
    };
    let payload = payload_of(&data[1..]);
    let Some(path) = ledger_path("run") else {
        return;
    };
    match mode % MODES {
        0 => mode_content(&path, &payload),
        1 => mode_tamper(&path, &payload),
        2 => mode_structure(&path, &payload),
        3 => mode_approval(&path, &payload),
        _ => mode_raw(&path, &payload),
    }
    cleanup(&path);
});
