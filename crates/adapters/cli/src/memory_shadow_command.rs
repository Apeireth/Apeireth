//! `apeireth memory shadow` / `apeireth memory swap` —— **B3 批**（影子双跑 + 换算法流程）
//! 的**面向人的出口**。
//!
//! # 这个命令面补的是哪个洞
//!
//! B3 批（影子双跑 + 裁决 + 三道闸 + 归档 + 回退）已经接进生产（**默认关**），
//! 但此前**没有面向人的出口** ⇒ ⇒ 「要用起来还差的那一步」。
//! 本模块把那一步补上：**五个动作**（两个只读、一个观测、一个切换、一个回退）。
//!
//! # 子命令（语义 + 只读性）
//!
//! | 子命令 | 做什么 | 只读？ |
//! |---|---|---|
//! | `memory shadow status` | 指针现状（当前/上一版旋钮、代次、切换时刻）+ 最近报告 + **三道闸当前判定** | 是 |
//! | `memory shadow run` | 用 `shadow.json` 的探针题跑一轮影子 ⇒ 出报告 + 逐题 **id 与指标** | 否（写归档报告） |
//! | `memory swap status` | **能不能切** + **卡在哪道闸**（带数字） | 是 |
//! | `memory swap apply` | 走**库层同一套流程**尝试切换 | 否 |
//! | `memory swap rollback` | 回退到上一版（**撤销**方向 ⇒ 不需要审批） | 否 |
//!
//! # 退出码（三档，含义不许混）
//!
//! * **0 = 确实做成了**：状态读到了 / 报告产出了 / 真的切了 / 真的回退了；
//! * **1 = 做了但被拒**（或「有发现」）：治理面没放行 / 三道闸没过 / 没得回退
//!   / 状态判定为「不许换」；
//! * **2 = 没跑成**：未配置 / 配置坏 / 库打不开 / 参数错。
//!
//! ⚠️ **「没跑」绝不写成「成功」**：默认态（没有 `shadow.json`）⇒ **退出码 2**，
//! 输出里写「未配置 / 未启用」⇒ **不许**出现「成功」「已通过」这类字样。
//!
//! # 三条纪律（与库级 / 接线层逐字一致）
//!
//! 1. **不自己造一套流程**：`apply` / `rollback` 直接调**接线层与库层同一套**
//!    （`shadow_wire::ShadowSwapService::swap` / `::rollback`
//!    ⇒ 治理面 → 裁决 → 三道闸 → 归档 → 写指针）；
//! 2. **不自己造审批**：切换必须过仓里**既有的治理链**；**当前装配根没有治理链句柄**
//!    ⇒ ⇒ **切换必被拒** ⇒ 本命令**如实显示被拒的真实原因**，不假装成功；
//! 3. **报告里没有答案字段 ⇒ 输出也不许带答案 / 正文**：逐题只列 **id 与指标**
//!    （条数 + token），不列标准答案、不列查询正文、不列召回命中的记录 id。
//!
//! # 只读命令怎么保证「一个字节都不动」
//!
//! * 只读命令在**临时快照**上算分（认知库先复制到临时目录再打开）
//!   ⇒ SQLite 的 WAL / 迁移副作用**不落在用户的数据根**；
//! * 归档目录、指针、旋钮在只读命令里**只读不写**；
//! * ⇒ 只读命令跑完，**数据根的文件树逐项相同**（测试逐项钉住这条）。
//!
//! # 中性表述
//!
//! 只描述**机制**（影子双跑 / 裁决 / 三道闸 / 归档 / 回退 / 治理链），
//! 不引用任何外部产品名。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::paths::default_data_root;
use apeireth_memory::shadow_run::{SwapDecision, SwapVerdict, DEFAULT_RUNS};
use apeireth_memory::swap_flow::{list_archives, SwapCost, MIN_SHADOW_CASES, MIN_SHADOW_RUNS};
use apeireth_memory::{MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime_assembly::shadow_wire::{
    config_from_data_root, decision_label, install_shadow, swap_cost, ChallengerKnobs,
    ShadowConfigBody, ShadowSwapService, SHADOW_ARCHIVE_DIR,
};

/// 退出码：**确实做成了**（状态读到了 / 报告产出了 / 真切了 / 真回退了）。
const EXIT_OK: u8 = 0;
/// 退出码：**做了但被拒**（或「有发现」：治理没放行 / 闸没过 / 没得回退 / 不许换）。
const EXIT_REFUSED: u8 = 1;
/// 退出码：**没跑成**（未配置 / 配置坏 / 库打不开 / 参数错）。
const EXIT_CANNOT: u8 = 2;

/// 命令用法（与其它命令的 `USAGE` 风格一致）。
pub const USAGE: &str = "apeireth memory shadow status|run [--data-root <DIR>] [--db <DB>]\n\
apeireth memory swap status|apply|rollback [--data-root <DIR>] [--db <DB>]\n\n\
  B3 批（影子双跑 + 换算法流程）的面向人的出口。\n\n\
  shadow status   只读：指针现状 + 最近报告 + 三道闸当前判定\n\
  shadow run      用探针题跑一轮影子 ⇒ 出报告 + 逐题指标（不带答案 / 正文）\n\
  swap status     只读：能不能切 + 卡在哪道闸（带数字）\n\
  swap apply      走库层同一套流程尝试切换（没有治理链 ⇒ 必被拒，如实报真实原因）\n\
  swap rollback   回退到上一版（撤销方向 ⇒ 不需要审批）\n\n\
  --data-root  数据根（缺省 = APEIRETH_DATA_DIR 或用户目录下的 .apeireth）\n\
  --db        认知库路径（缺省 = APEIRETH_COGNITIVE_DB 或数据根下 cognitive.sqlite3）\n\n\
  退出码：0 = 确实做成了 · 1 = 做了但被拒 / 有发现 · 2 = 没跑成（未配置 / 配置坏 / 库打不开 / 参数错）";

/// 命令行选项（数据根 + 认知库）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    /// 数据根（`shadow.json` / 归档 / 指针 / 旋钮都落这里）。
    data_root: PathBuf,
    /// 认知库路径。
    db: PathBuf,
}

/// 一道闸的判定（**只读展示**）。
///
/// ⚠️ **放行与否的权威是库级** [`SwapCost::allows_swap`]（库级那把尺子）；
/// 这里只把**每道闸的判据与实测数字**摊开给人看（哪道闸拦下的、差多少）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct GateLine {
    /// 闸名。
    name: &'static str,
    /// 是否通过。
    passed: bool,
    /// 人话说明（带数字）。
    detail: String,
}

/// 命令入口（`main.rs` 的 `memory` 组分派到这里）。
///
/// # 返回
///
/// 退出码语义见模块文档（三档：**0 确实做成了 / 1 被拒或有发现 / 2 没跑成**）。
pub fn run(args: &[String]) -> u8 {
    let Some(group) = args.first().map(String::as_str) else {
        eprintln!("{USAGE}");
        return EXIT_CANNOT;
    };
    match group {
        "shadow" => match args.get(1).map(String::as_str) {
            Some("status") => run_shadow_status(&args[2..]),
            Some("run") => run_shadow_run(&args[2..]),
            _ => {
                eprintln!("未知子命令：memory shadow（可用：status / run）");
                eprintln!("{USAGE}");
                EXIT_CANNOT
            }
        },
        "swap" => match args.get(1).map(String::as_str) {
            Some("status") => run_swap_status(&args[2..]),
            Some("apply") => run_swap_apply(&args[2..]),
            Some("rollback") => run_swap_rollback(&args[2..]),
            _ => {
                eprintln!("未知子命令：memory swap（可用：status / apply / rollback）");
                eprintln!("{USAGE}");
                EXIT_CANNOT
            }
        },
        other => {
            eprintln!("未知命令组：memory {other}（可用：shadow / swap）");
            eprintln!("{USAGE}");
            EXIT_CANNOT
        }
    }
}

// --------------------------------------------------------------------- 选项

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut data_root: Option<PathBuf> = None;
    let mut db: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let mut take = |name: &str| -> Result<PathBuf, String> {
            let Some(value) = args.get(index + 1) else {
                return Err(format!("{name} 缺参数"));
            };
            if value.trim().is_empty() {
                return Err(format!("{name} 的值不能为空"));
            }
            Ok(PathBuf::from(value))
        };
        match flag {
            "--data-root" => {
                data_root = Some(take("--data-root")?);
                index += 2;
            }
            "--db" => {
                db = Some(take("--db")?);
                index += 2;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }
    let data_root = data_root.unwrap_or_else(default_data_root);
    let db = db.unwrap_or_else(|| match std::env::var("APEIRETH_COGNITIVE_DB") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => data_root.join("cognitive.sqlite3"),
    });
    Ok(Options { data_root, db })
}

/// 当前时刻（毫秒；**注入**给库层流程 ⇒ 归档内容可复现）。
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

// ----------------------------------------------------------------- 快照

/// 认知库的**临时快照**。
///
/// # 为什么只读命令也要快照
///
/// 认知库是 SQLite（`SqliteMemoryStore::open` 会跑迁移并把日志模式设成 WAL）
/// ⇒ **直接打开就会在数据根里留下 `-wal` / `-shm` 边车并改库头**
/// ⇒ ⇒ 「只读命令不改任何文件」这条纪律就破了。
/// ⇒ ⇒ 折中：**先复制到临时目录再打开** ⇒ 用户的库一个字节都不动
/// （代价：多一次拷贝；收益：只读命令真的只读）。
struct Snapshot {
    /// 临时目录（析构时清掉）。
    dir: PathBuf,
    /// 快照里的库路径。
    db: PathBuf,
}

impl Snapshot {
    /// 抓一份快照（含 WAL 边车 ⇒ 快照与源库一致）。
    fn capture(source: &Path) -> Result<Self, String> {
        let stamp = now_ms();
        let dir = std::env::temp_dir().join(format!(
            "apeireth-shadow-snapshot-{}-{stamp}",
            std::process::id()
        ));
        fs_err::create_dir_all(&dir)
            .map_err(|error| format!("临时目录建不出来（{}）：{error}", dir.display()))?;
        let db = dir.join("cognitive.sqlite3");
        if let Err(error) = fs_err::copy(source, &db) {
            let _ = fs_err::remove_dir_all(&dir);
            return Err(format!("认知库快照失败（{}）：{error}", source.display()));
        }
        // WAL 边车一起复制（源库若有未 checkpoint 的事务 ⇒ 快照才与源库一致）。
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", source.display()));
            if sidecar.exists() {
                let _ = fs_err::copy(&sidecar, dir.join(format!("cognitive.sqlite3{suffix}")));
            }
        }
        Ok(Self { dir, db })
    }

    /// 快照里的库路径。
    fn db_path(&self) -> &Path {
        &self.db
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs_err::remove_dir_all(&self.dir);
    }
}

// ----------------------------------------------------------------- 准备

/// 准备结果：**未配置** 或 **已装配**。
enum Prepared {
    /// 没有 `shadow.json` ⇒ 未配置 / 未启用（默认态）。
    NotConfigured,
    /// 已装配（服务 + 配置）。
    Ready {
        /// 服务句柄（**接线层那个**，不是本命令自己造的一套）。
        service: Arc<ShadowSwapService>,
        /// 配置（展示用：候选算法 id / 轮数 / 题数）。
        config: ShadowConfigBody,
    },
}

/// 装配服务（**走接线层同一个入口** ⇒ 语义与生产一致）。
fn prepare(options: &Options) -> Result<Prepared, String> {
    if !options.db.exists() {
        return Err(format!(
            "认知库不存在：{} —— 先跑一次会写记忆的功能，或用 --db 指定路径",
            options.db.display()
        ));
    }
    let snapshot = Snapshot::capture(&options.db)?;
    let store = Arc::new(
        SqliteMemoryStore::open(snapshot.db_path()).map_err(|error| {
            format!("认知库打不开（{}）：{error}", snapshot.db_path().display())
        })?,
    );
    let backend: Arc<dyn MemoryBackend> = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    // ⚠️ **治理链 = None**：与装配根**同口径**（当前没有治理链句柄）
    // ⇒ ⇒ **切换必被拒**（失败闭合，不自造审批）。
    let service = install_shadow(backend, governance, &options.data_root, None)
        .map_err(|error| format!("影子服务装配失败：{error}"))?;
    let Some(service) = service else {
        return Ok(Prepared::NotConfigured);
    };
    let config = config_from_data_root(&options.data_root)
        .map_err(|error| format!("影子配置读不出来：{error}"))?
        .ok_or_else(|| "影子配置读不出来：文件在准备期间消失了".to_string())?;
    Ok(Prepared::Ready { service, config })
}

/// 准备失败 ⇒ 如实打印 + 退出码 2（**没跑成**）。
fn prepare_or_report(options: &Options) -> Result<Prepared, u8> {
    prepare(options).map_err(|error| {
        eprintln!("memory shadow/swap 跑不成：{error}");
        EXIT_CANNOT
    })
}

/// 配置里的影子轮数（缺省 = 库级默认）。
fn runs_of(config: &ShadowConfigBody) -> usize {
    config.runs.unwrap_or(DEFAULT_RUNS)
}

// ----------------------------------------------------------------- 渲染

/// 旋钮的一行文字（**没设**就是没设，不许显示成 0）。
fn knobs_line(knobs: &ChallengerKnobs) -> String {
    let alpha = knobs
        .first_pos_alpha
        .map(|value| format!("{value}"))
        .unwrap_or_else(|| "(未设)".to_string());
    let lambda = knobs
        .diversity_lambda
        .map(|value| format!("{value}"))
        .unwrap_or_else(|| "(未设)".to_string());
    format!("first_pos_alpha = {alpha} · diversity_lambda = {lambda}")
}

/// 分数的一行文字（**没度量到**就是没度量到 ⇒ 写成「未度量到」，不许显示成 0）。
fn score_text(value: Option<f64>) -> String {
    match value {
        Some(score) => format!("{score:.4}"),
        None => "未度量到".to_string(),
    }
}

/// token 的一行文字（**没度量到**就是没度量到 ⇒ 写成「未度量到」，不许显示成 0）。
fn token_text(value: Option<usize>) -> String {
    match value {
        Some(tokens) => format!("{tokens}"),
        None => "未度量到".to_string(),
    }
}

/// 三道闸的逐条判定（**判据与数字都来自库级口径**）。
fn gate_lines(cost: &SwapCost, verdict: &SwapVerdict) -> Vec<GateLine> {
    let decision_ok = matches!(
        verdict.decision,
        SwapDecision::Approved | SwapDecision::ApprovedWithGain
    );
    let samples_ok = cost.shadow_runs >= MIN_SHADOW_RUNS && cost.shadow_cases >= MIN_SHADOW_CASES;
    vec![
        GateLine {
            name: "① 裁决闸",
            passed: decision_ok,
            detail: format!(
                "{} —— {}（现役分数 {} · 候选分数 {} · 现役 token {} · 候选 token {}）",
                decision_label(verdict.decision),
                verdict.reason,
                score_text(verdict.champion_score),
                score_text(verdict.challenger_score),
                token_text(verdict.champion_tokens),
                token_text(verdict.challenger_tokens)
            ),
        },
        GateLine {
            name: "② 回退速度闸",
            passed: cost.rollback_ms < cost.switch_ms,
            detail: format!(
                "回退 {}ms vs 切换 {}ms（回退必须更快 ⇒ 否则「可回退」是假的）",
                cost.rollback_ms, cost.switch_ms
            ),
        },
        GateLine {
            name: "③ 样本充分闸",
            passed: samples_ok,
            detail: format!(
                "影子轮数 {}（需 >= {}）· 样本题数 {}（需 >= {}）",
                cost.shadow_runs, MIN_SHADOW_RUNS, cost.shadow_cases, MIN_SHADOW_CASES
            ),
        },
    ]
}

/// 打印三道闸（**通过 / 拦下**都写清楚）。
fn print_gates(lines: &[GateLine]) {
    for line in lines {
        println!(
            "    {} {} —— {}",
            if line.passed { "✓" } else { "✗" },
            line.name,
            line.detail
        );
    }
}

/// 第一道没过的闸（`None` = 全过）。
fn first_blocking_gate(lines: &[GateLine]) -> Option<&GateLine> {
    lines.iter().find(|line| !line.passed)
}

/// **两侧都没量到东西**时的**观测提示**（**只提示、不改裁决**）。
///
/// # 为什么要有这一行
///
/// 库级裁决判的是「**有没有变差**」⇒ 两边都是 0 分时它判「未劣化 ⇒ 允许换」。
/// 但「**没量到东西**」≠「量到的东西没变差」⇒ ⇒ 人看到的必须包含
/// 「**这一轮没有有效证据**」（库级自己的口径：**没证据 ≠ 没变差**）
/// ⇒ ⇒ 不许让读者把 `0 vs 0` 当成「实测通过」。
///
/// ⚠️ **不改裁决、不改退出码**（裁决的权威是库级）⇒ 只是把**证据强度**说清楚。
fn no_evidence_note(verdict: &SwapVerdict) -> Option<String> {
    let empty = |value: Option<f64>| value.map(|score| score <= 0.0).unwrap_or(true);
    if !(empty(verdict.champion_score) && empty(verdict.challenger_score)) {
        return None;
    }
    Some(
        "⚠ 观测：两侧**都没量到**召回（分数都是 0 / 未度量到）\n\
         ⇒ 这一轮**没有有效证据**\n\
         ⇒ 裁决闸此刻判「未劣化 ⇒ 允许换」只是**没有反证**，不是「实测通过」\n\
         （库级口径：没证据 ≠ 没变差 ⇒ 真要换之前请先确认库里真的有可召回的记录）"
            .to_string(),
    )
}

/// 打印「两侧都没量到」的观测提示（**有则打**）。
fn print_no_evidence_note(verdict: &SwapVerdict) {
    if let Some(note) = no_evidence_note(verdict) {
        for line in note.lines() {
            println!("  {line}");
        }
    }
}

/// 归档目录里的报告名（**目录不存在 = 还没有报告**，不是错）。
fn archived_reports(data_root: &Path) -> Vec<String> {
    let dir = data_root.join(SHADOW_ARCHIVE_DIR);
    list_archives(&dir).unwrap_or_default()
}

/// 指针的一行摘要（**没换过**就是没换过，不许显示成「当前 = 默认」以外的样子）。
fn pointer_lines(
    pointer: Option<&apeireth_runtime_assembly::shadow_wire::PointerFile>,
) -> Vec<String> {
    match pointer {
        Some(file) => vec![
            format!(
                "当前算法 = {} · 上一版 = {} · 代次 = {} · 切换时刻 = {}",
                file.current,
                file.previous.clone().unwrap_or_else(|| "(无)".to_string()),
                file.generation,
                file.switched_at_ms
            ),
            format!("当前旋钮：{}", knobs_line(&file.current_knobs)),
            format!(
                "上一版旋钮：{}",
                file.previous_knobs
                    .as_ref()
                    .map(knobs_line)
                    .unwrap_or_else(|| "(无 —— 从没换过)".to_string())
            ),
        ],
        None => vec!["指针：不存在 ⇒ **从没换过**（现役 = 初始算法，没得回退）".to_string()],
    }
}

// ----------------------------------------------------------------- 子命令

/// `memory shadow status` —— 只读：指针现状 + 最近报告 + 三道闸当前判定。
fn run_shadow_status(args: &[String]) -> u8 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错：{error}");
            eprintln!("{USAGE}");
            return EXIT_CANNOT;
        }
    };
    let prepared = match prepare_or_report(&options) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let Prepared::Ready { service, config } = prepared else {
        eprintln!(
            "未配置 / 未启用：数据根 {} 下没有 shadow.json ⇒ 影子双跑与换算法**没有开**\
             （默认态是刻意的：不构造、不读盘、不写盘）",
            options.data_root.display()
        );
        return EXIT_CANNOT;
    };

    let pointer = match service.read_pointer() {
        Ok(pointer) => pointer,
        Err(error) => {
            eprintln!("指针读不出来：{error}");
            return EXIT_CANNOT;
        }
    };
    let verdict = match service.judge() {
        Ok(verdict) => verdict,
        Err(error) => {
            eprintln!("裁决跑不起来：{error}");
            return EXIT_CANNOT;
        }
    };
    let cost = swap_cost(runs_of(&config), config.cases.len());
    let gates = gate_lines(&cost, &verdict);
    let allowed = cost.allows_swap(verdict.decision);

    println!("影子双跑 / 换算法：状态（只读）");
    println!("  数据根：{}", options.data_root.display());
    println!(
        "  配置：已启用（候选算法 = {} · 影子轮数 = {} · 探针题 = {}）",
        config.challenger_id,
        runs_of(&config),
        config.cases.len()
    );
    for line in pointer_lines(pointer.as_ref()) {
        println!("  {line}");
    }
    let reports = archived_reports(&options.data_root);
    if reports.is_empty() {
        println!("  归档报告：暂无（目录 {} 还没有报告）", SHADOW_ARCHIVE_DIR);
    } else {
        println!("  归档报告：{} 份（最近 5 份）", reports.len());
        for name in reports.iter().rev().take(5) {
            println!("    - {name}");
        }
    }
    println!("  三道闸当前判定：");
    print_gates(&gates);
    print_no_evidence_note(&verdict);
    match first_blocking_gate(&gates) {
        None => println!("  结论：三道闸全过 ⇒ 库级流程**允许换**（仍需治理面放行）"),
        Some(blocked) => println!("  结论：**不许换** —— 卡在 {}", blocked.name),
    }
    if allowed {
        EXIT_OK
    } else {
        EXIT_REFUSED
    }
}

/// `memory shadow run` —— 跑一轮影子 ⇒ 出报告 + 逐题指标。
fn run_shadow_run(args: &[String]) -> u8 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错：{error}");
            eprintln!("{USAGE}");
            return EXIT_CANNOT;
        }
    };
    let prepared = match prepare_or_report(&options) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let Prepared::Ready { service, config } = prepared else {
        eprintln!(
            "未配置 / 未启用：数据根 {} 下没有 shadow.json ⇒ 影子双跑**没有开**\
             （默认态是刻意的：不构造、不读盘、不写盘）",
            options.data_root.display()
        );
        return EXIT_CANNOT;
    };

    let (report, verdict) = match service.observe() {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("影子跑不起来：{error}");
            return EXIT_CANNOT;
        }
    };

    println!("影子双跑：跑一轮（只观测 · 不改生产结果）");
    println!("  数据根：{}", options.data_root.display());
    println!(
        "  现役算法 = {} · 候选算法 = {} · 轮数 = {} · 探针题 = {}",
        report.champion_id,
        report.challenger_id,
        runs_of(&config),
        config.cases.len()
    );
    println!(
        "  归档目录：{}",
        options.data_root.join(SHADOW_ARCHIVE_DIR).display()
    );
    println!("  逐题结果（**只有 id 与指标**：不带答案 / 正文 / 命中记录 id）：");
    for pair in &report.pairs {
        println!(
            "    {} · 现役 {} 条 / {} token · 候选 {} 条 / {} token",
            pair.case_id,
            pair.champion.ids.len(),
            pair.champion.injected_tokens,
            pair.challenger.ids.len(),
            pair.challenger.injected_tokens
        );
    }
    let cost = swap_cost(runs_of(&config), config.cases.len());
    let gates = gate_lines(&cost, &verdict);
    println!("  裁决：{}", decision_label(verdict.decision));
    println!("    {}", verdict.reason);
    println!("  三道闸当前判定：");
    print_gates(&gates);
    print_no_evidence_note(&verdict);
    match first_blocking_gate(&gates) {
        None => println!("  结论：三道闸全过 ⇒ 库级流程**允许换**（仍需治理面放行）"),
        Some(blocked) => println!("  结论：**不许换** —— 卡在 {}", blocked.name),
    }
    EXIT_OK
}

/// `memory swap status` —— 只读：能不能切 + 卡在哪道闸（带数字）。
fn run_swap_status(args: &[String]) -> u8 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错：{error}");
            eprintln!("{USAGE}");
            return EXIT_CANNOT;
        }
    };
    let prepared = match prepare_or_report(&options) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let Prepared::Ready { service, config } = prepared else {
        eprintln!(
            "未配置 / 未启用：数据根 {} 下没有 shadow.json ⇒ 换算法流程**没有开**\
             （默认态是刻意的：不构造、不读盘、不写盘）",
            options.data_root.display()
        );
        return EXIT_CANNOT;
    };

    let verdict = match service.judge() {
        Ok(verdict) => verdict,
        Err(error) => {
            eprintln!("裁决跑不起来：{error}");
            return EXIT_CANNOT;
        }
    };
    let cost = swap_cost(runs_of(&config), config.cases.len());
    let gates = gate_lines(&cost, &verdict);
    let allowed = cost.allows_swap(verdict.decision);

    println!("换算法：能不能切（只读判定）");
    println!("  数据根：{}", options.data_root.display());
    println!(
        "  目标算法 = {} · 影子轮数 = {} · 样本题数 = {}",
        config.challenger_id, cost.shadow_runs, cost.shadow_cases
    );
    println!(
        "  治理面：**没有治理链句柄** ⇒ 与装配根同口径 ⇒ 切换**必被拒**（失败闭合，不自造审批）"
    );
    println!("  三道闸：");
    print_gates(&gates);
    print_no_evidence_note(&verdict);
    let blocked = first_blocking_gate(&gates);
    match (allowed, blocked) {
        (true, None) => {
            println!("  结论：库级三道闸**全过** ⇒ 但治理面没放行 ⇒ **现在还切不了**");
            EXIT_REFUSED
        }
        (false, Some(line)) => {
            println!("  结论：**切不了** —— 卡在 {}", line.name);
            EXIT_REFUSED
        }
        _ => {
            println!("  结论：**切不了**");
            EXIT_REFUSED
        }
    }
}

/// `memory swap apply` —— 走**库层同一套流程**尝试切换。
fn run_swap_apply(args: &[String]) -> u8 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错：{error}");
            eprintln!("{USAGE}");
            return EXIT_CANNOT;
        }
    };
    let prepared = match prepare_or_report(&options) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let Prepared::Ready { service, .. } = prepared else {
        eprintln!(
            "未配置 / 未启用：数据根 {} 下没有 shadow.json ⇒ 换算法流程**没有开**\
             （默认态是刻意的：不构造、不读盘、不写盘）",
            options.data_root.display()
        );
        return EXIT_CANNOT;
    };

    // ⚠️ **不自己实现流程**：直接调接线层那个服务（治理面 → 裁决 → 三道闸 → 归档 → 写指针）。
    let outcome = match block_on_flat(service.swap(now_ms())) {
        Ok(outcome) => outcome,
        Err(error) => {
            // ⚠️ **被拒就是被拒**（治理面没放行 / 计划过期 / 落盘失败）⇒ 如实显示真实原因。
            eprintln!("换算法**被拒**：{error}");
            return EXIT_REFUSED;
        }
    };
    if !outcome.switched {
        eprintln!("换算法**没有发生**：{}", outcome.detail);
        return EXIT_REFUSED;
    }
    println!("换算法：已切换");
    println!("  {}", outcome.detail);
    println!("  归档报告：{}", outcome.archive_path);
    EXIT_OK
}

/// `memory swap rollback` —— 回退到上一版（撤销方向 ⇒ 不需要审批）。
fn run_swap_rollback(args: &[String]) -> u8 {
    let options = match parse_options(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错：{error}");
            eprintln!("{USAGE}");
            return EXIT_CANNOT;
        }
    };
    let prepared = match prepare_or_report(&options) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
    let Prepared::Ready { service, .. } = prepared else {
        eprintln!(
            "未配置 / 未启用：数据根 {} 下没有 shadow.json ⇒ 换算法流程**没有开**\
             （默认态是刻意的：不构造、不读盘、不写盘）",
            options.data_root.display()
        );
        return EXIT_CANNOT;
    };

    let outcome = match service.rollback(now_ms()) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("回退**没做成**：{error}");
            return EXIT_REFUSED;
        }
    };
    if !outcome.switched {
        eprintln!("回退**没有发生**：{}", outcome.detail);
        return EXIT_REFUSED;
    }
    println!("换算法：已回退");
    println!("  {}", outcome.detail);
    println!("  归档报告：{}", outcome.archive_path);
    EXIT_OK
}

/// 在**同步**入口里跑库层的异步流程（`swap` 要过治理链 ⇒ 是 async）。
fn block_on<F: std::future::Future>(future: F) -> Result<F::Output, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("异步运行时建不起来：{error}"))?;
    Ok(runtime.block_on(future))
}

/// `block_on` 的**扁平化**版本：把「运行时起不来」与「被调用方自己返回的 Err」**合成一层**。
///
/// # 为什么要有这个
/// `block_on` 自己的返回类型是 `Result<F::Output, String>` ⇒ 当 `F::Output` 本身
/// 也是 `Result<_, String>`（如 `swap`）⇒ ⇒ **嵌套两层** ⇒ ⇒ 调用处很容易
/// 把「被拒」当成「成功」⇒ ⇒ 这里**压成一层** ⇒ ⇒ 失败一律带**真实原因**。
fn block_on_flat<F, T, E>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let outcome = block_on(future)?;
    outcome.map_err(|error| error.to_string())
}
