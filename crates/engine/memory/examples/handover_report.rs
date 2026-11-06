//! **handover 演练报告**（真实语料）：把「换人接手」跑一遍给人看。
//!
//! # 这个例子回答什么
//!
//! 设计稿 §8 **B4**（脱机归档 + 迁移回滚 + handover 演练）与 §6.1 **L5 可继承**
//! （脱机可读 · schema 自说明覆盖 · **handover 演练通过**）的验收。
//!
//! ⇒ 这个例子用**真实语料**（《紅樓夢》120 回）+ **真库**（跑过真实迁移的 SQLite）
//! 造一份 handover 包，然后回答三件事：
//!
//! 1. **包长什么样**：`render_package_summary` 的**原样输出**（接手者**先看这个**）；
//! 2. **接手者能不能接**：`drill_handover` 的**原样输出**（四件事逐项判定）；
//! 3. **包能不能脱机读**：包内归档物的 JSONL（**原文可 grep** ⇒ 不依赖原库）。
//!
//! # 口径（钉死，别自创）
//!
//! * **语料在仓外**（`<数据根>/novels/redchamber.txt`）⇒ **语料不在 ⇒ 明确报「没跑」
//!   并非零退出**（不许把「没跑」当「没问题」）；
//! * **真库**：本例子把 120 回正文**逐字**写进一个 SQLite 库（跑过真实迁移）
//!   ⇒ 导出物与生产同构（**不是**生产库 ⇒ 如实标注）；
//! * **不写生产路径**：库文件落在**临时目录**，跑完就删；
//! * **已知问题照抄**：引用的是仓内**实测**数字（`golden_diag` 的 `recall@10 = 0.2983`），
//!   不编、不美化。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能在真实语料上跑出报告（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✘：已在《紅樓夢》**真实语料**上跑过（2026-10-27），
//!   但**未在生产记忆库上实测过**（本机无生产库）⇒ 不宣称已验证。

use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::handover::{
    build_package, drill_handover, render_package_summary, CurrentState, NextStep,
};
use apeireth_memory::novel_corpus::{load_chapters, mine_novel, to_mine_records};
use apeireth_memory::offline_archive::{export_archive, to_jsonl, verify_archive, ExportConfig};
use apeireth_memory::SqliteMemoryStore;

/// 语料文件名（**《紅樓夢》**：Project Gutenberg #24264，繁体 120 回）。
const CORPUS_FILE: &str = "redchamber.txt";

/// 语料文件的环境变量（可覆盖 ⇒ 便于在别处复跑）。
const CORPUS_ENV: &str = "APEIRETH_NOVEL_CORPUS";

/// 主体 id（这批记忆归**一个**主体 ⇒ 一条链 ⇒ 链头 1 条）。
const SUBJECT: &str = "novel-redchamber";

/// 演练里的**生成时刻**（注入 ⇒ 报告可复现；固定值 ⇒ 两次跑逐字相等）。
const CREATED_AT_MS: i64 = 1_760_000_000_000;

fn main() -> ExitCode {
    let corpus = match locate_corpus() {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("· 《紅樓夢》语料**不在**⇒ **没跑**（不是「跑过了」）：{reason}");
            eprintln!("  用法：handover_report [语料路径]（或设 {CORPUS_ENV}）");
            eprintln!("  期望位置：{}", novels_dir().join(CORPUS_FILE).display());
            return ExitCode::FAILURE;
        }
    };

    println!("════════ handover 演练报告（真实语料）════════");
    println!("语料：{}", corpus.display());

    let chapters = match load_chapters(&corpus) {
        Ok(chapters) => chapters,
        Err(error) => {
            eprintln!("· **语料读不了**：{error}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "解析：{} 回（正文 {} 字）",
        chapters.len(),
        chapters
            .iter()
            .map(|chapter| chapter.body.chars().count())
            .sum::<usize>()
    );

    // 挖题（真实语料 ⇒ 真实题量；这是状态里「已知问题」的数字来源）。
    let cases = mine_novel(&chapters, MiningConfig::default());
    println!("挖题：{} 道（真实语料挖出来的）", cases.len());

    // 造**真库**（跑过真实迁移 ⇒ 列集与生产一致）⇒ 导出物与生产同构。
    let db_path = match build_library(&chapters) {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("· **造库失败**：{reason}");
            return ExitCode::FAILURE;
        }
    };
    println!("库（临时）：{}", db_path.display());

    let store = match SqliteMemoryStore::open(&db_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("· **打开库失败**：{error}");
            return ExitCode::FAILURE;
        }
    };
    let archive = {
        let conn = store.conn().expect("拿连接");
        match export_archive(&conn, &ExportConfig::new(), CREATED_AT_MS) {
            Ok(archive) => archive,
            Err(reason) => {
                eprintln!("· **导出失败**：{reason}");
                return ExitCode::FAILURE;
            }
        }
    };
    println!(
        "归档：{} 条记录 · {} 条链头 · 未纳管 {} 条",
        archive.manifest.episode_count,
        archive.manifest.chain_head_count,
        archive.manifest.unmanaged_count
    );

    let state = CurrentState {
        db_path: db_path.display().to_string(),
        episode_count: archive.episodes.len(),
        subject_count: archive.chain_heads.len(),
        unmanaged_count: archive.manifest.unmanaged_count,
        current_algorithm: Some("hybrid（BM25 + 向量）".to_string()),
        known_issues: known_issues(cases.len()),
    };
    let next_steps = next_steps(cases.len());

    let package = build_package(archive, state, next_steps, CREATED_AT_MS);

    println!();
    println!("── render_package_summary（原样输出）──");
    println!("{}", render_package_summary(&package).trim_end());

    println!();
    println!("── drill_handover（原样输出）──");
    let (passed, notes) = drill_handover(&package);
    for note in &notes {
        println!("{note}");
    }

    println!();
    println!("── 包内归档物：JSONL（脱机可读 ⇒ 可 grep 原文）──");
    let text = to_jsonl(&package.archive);
    for line in text.lines().take(1) {
        println!("  │ {line}");
    }
    for line in text.lines().skip(1).take(1) {
        // ⚠️ 记录行**整回正文**（一回几千字）⇒ 打印时截断，否则刷屏；
        // **包里逐字保留**（`to_jsonl` 不截断）⇒ 截断只发生在**显示**这一层。
        println!(
            "  │ {}(…正文太长，此处截断显示；包里逐字保留…)",
            truncate(line, 200)
        );
    }
    println!(
        "  （共 {} 行：1 行 manifest + {} 条记录 + {} 条链头）",
        text.lines().count(),
        package.archive.episodes.len(),
        package.archive.chain_heads.len()
    );
    // **grep 演示**：同一份文本里直接搜原文 ⇒ 不需要任何专有工具。
    let hits = text.lines().filter(|line| line.contains("賈雨村")).count();
    println!("  grep 演示：在同一份文本里搜「賈雨村」⇒ 命中 {hits} 行");

    println!();
    match verify_archive(&package.archive) {
        Ok(()) => println!("独立校验：**通过**（条数 + 内容哈希 + 链账核对都相符）"),
        Err(error) => println!("独立校验：**不通过** —— {error}"),
    }

    println!();
    if passed {
        println!("演练结论：**通过**（四件事都过 ⇒ 一个只拿到包的接手者能接得下去）");
    } else {
        println!("演练结论：**不通过**（⇒ 上面标 ❌ 的那几项就是接手者会卡住的地方）");
    }

    // 临时库用完就删（**不留垃圾**，也不碰任何生产路径）。
    let _ = std::fs::remove_file(&db_path);

    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// **已知问题**（**如实** ⇒ 坏消息不藏）。
///
/// # 口径
///
/// * 数字全部来自**仓内实测**（`golden_diag` / `golden_set`）⇒ **不编**；
/// * 另两条是**本批的事实**（本模块只到库级；B4 的「迁移回滚」还没做）⇒ 同样如实写。
fn known_issues(mined_cases: usize) -> Vec<String> {
    vec![
        "内部卷（《紅樓夢》120 回 / 476 题）实测 recall@10 = 0.2983 ⇒ 73% 的题目标记录没进检索结果；\
         天花板 coverage 0.2712 ⇒ **瓶颈在召回，不在注入**（golden_diag 实测，2026-10-26）"
            .to_string(),
        format!(
            "本次挖题得到 {mined_cases} 道候选题（未精标）⇒ 题集本身**尚未定稿**，recall 数字只作参考"
        ),
        "handover 本体仍是**库级实现**（NOT IMPLEMENTED）：无 CLI、无定时任务、生产 0 调用点 \
         ⇒ 接手者目前只能靠本例子手工造包"
            .to_string(),
        "B4 的**「迁移回滚」**那一块**尚无实现** ⇒ 本包只覆盖 B4 的第一块砖（脱机归档）\
         与第三块砖（handover 演练）"
            .to_string(),
    ]
}

/// **下一步**（每条都带**可跑的验收判据** ⇒ 演练才判得过）。
fn next_steps(mined_cases: usize) -> Vec<NextStep> {
    vec![
        NextStep {
            what: "把召回缺口拆到检索层：先查候选池覆盖率（coverage），再谈排序".to_string(),
            why: "recall@10 = 0.2983 的天花板是 coverage 0.2712 ⇒ 目标记录压根没进候选池时，\
                  注入层与排序层再优化也没用"
                .to_string(),
            acceptance:
                "cargo run -p apeireth-memory --example recall_gap_report ⇒ 输出 GapClass 分布；\
                         候选池覆盖率 ≥ 0.45（基线 0.2712），且 NotInPool 占比逐题可归因"
                    .to_string(),
            cost: 2,
        },
        NextStep {
            what: "把 handover 包接到 CLI 子命令（`memory handover`）⇒ 接手者一条命令就能拿包"
                .to_string(),
            why: "本批只到库级（NOT IMPLEMENTED，生产 0 调用点）⇒ 交接还得靠人手工跑本例子"
                .to_string(),
            acceptance: "cargo run -p apeireth-cli -- memory handover --out pkg.json ⇒ \
                         产出的包与本例子的包**逐字相等**（同一 created_at_ms）"
                .to_string(),
            cost: 1,
        },
        NextStep {
            what: format!(
                "补 B4 的第三块砖：**迁移回滚**演练（当前只覆盖了脱机归档 + handover 演练）"
            ),
            why: "设计稿 §8 B4 的验收是「脱机可读；**回滚可演练**」⇒ 缺回滚 ⇒ B4 未完成"
                .to_string(),
            acceptance: "cargo nextest run -p apeireth-memory --profile ci ⇒ 迁移回滚用例全绿，\
                         且回滚后重导出的 content_digest 与回滚前**一致**（⇒ 数据没丢）"
                .to_string(),
            cost: 3,
        },
        NextStep {
            what: format!("把内部卷题集定稿（当前 {mined_cases} 道候选题未精标）").to_string(),
            why: "题集没定稿 ⇒ recall 数字会随挖题配置漂移 ⇒ 基线不可复核".to_string(),
            acceptance: "scripts/ 下的内部卷基线命令重跑两次 ⇒ 题数与 recall@10 **逐字相等**，\
                         且结果归档到基线文件"
                .to_string(),
            cost: 2,
        },
    ]
}

/// 把每一回正文**逐字**写进一个真库（临时文件 ⇒ 跑完即删）。
fn build_library(chapters: &[apeireth_memory::novel_corpus::Chapter]) -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join("apeireth-handover-drill.sqlite3");
    let _ = std::fs::remove_file(&path);
    let store = SqliteMemoryStore::open(&path).map_err(|error| error.to_string())?;

    let records = to_mine_records(chapters);
    for record in &records {
        let episode = Episode {
            id: record.id.clone(),
            // ⚠️ 库的 `timestamp` 是 **epoch 秒**；小说语料的 `timestamp_ms` 是毫秒
            // ⇒ 这里换算（**如实**换算，不塞错单位 ⇒ 接手者按 `timestamp` 排序才没错）。
            timestamp: record.timestamp_ms / 1000,
            role: "user".to_string(),
            content: record.content.clone(),
            session_id: "novel-redchamber".to_string(),
        };
        store
            .put_episode_for_subject(&episode, SUBJECT)
            .map_err(|error| error.to_string())?;
    }
    Ok(path)
}

/// 截断一长串文本（**只影响显示** ⇒ 不碰数据）。
///
/// **按字符截**（不是按字节）⇒ 不会把一个汉字劈成半个（劈了就不是原文了）。
fn truncate(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push('…');
    }
    out
}

/// 找语料文件（命令行参数 ⇒ 环境变量 ⇒ 数据根下的 `novels/`）。
fn locate_corpus() -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(raw) = std::env::args().nth(1) {
        candidates.push(PathBuf::from(raw));
    }
    if let Some(raw) = std::env::var_os(CORPUS_ENV) {
        candidates.push(PathBuf::from(raw));
    }
    candidates.push(novels_dir().join(CORPUS_FILE));

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "候选路径都不存在：{}",
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" · ")
    ))
}

/// 语料目录（`$APEIRETH_DATA_DIR/novels/`，默认 `~/.apeireth/novels/`）。
///
/// **为什么用 `apeireth_core::paths::default_data_root()`**：数据根在仓里有**唯一真源**
/// ⇒ 本例子不自己再拼一遍 `$HOME/.apeireth`（两份实现漂移 = 读不到刚下载的语料）。
fn novels_dir() -> PathBuf {
    apeireth_core::paths::default_data_root().join("novels")
}
