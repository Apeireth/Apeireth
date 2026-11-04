//! `apeireth trait` 命令面 (裁定 ④⑤): 性格周报 + 四杆锁定/解锁 (显式动作)。
//!
//! | 动作 | 语义 | 档 |
//! |---|---|---|
//! | `report` | 打印近 7 天性格周报 (四杆移动量 / 移动原因 top / 锁定态) | 只读 (成长计既有账, 不新造账) |
//! | `lock <slider>` | 锁定一根滑杆: 此后成长沉积**跳过**该杆并留审计痕 | 显式人动作 (写锁定档 `trait-locks.json`) |
//! | `unlock <slider>` | 解锁一根滑杆: 成长沉积恢复移动该杆 | 显式人动作 (无自动解锁路径) |
//!
//! 滑杆名 (稳定 id, 与 env 旋钮/工具帧同名): `memory_fade` /
//! `curiosity_strength` / `tone_saturation` / `consolidation_cadence`。
//!
//! 周报有两个出口形态: 默认**人读文本**; `report --json` 出 **JSON 帧**
//! (字段名与 `tool.trait_report` 工具帧同构 —— 同一本成长账、同一份锁定档、
//! 同一套字段名与取值口径), 与 `apeireth exposure report --json` /
//! `apeireth egress ledger --json` 同族 (布尔旗标, 不带值)。
//!
//! 与 `tool.trait_report` **同一内核两个出口**: 周报读数同一本成长账
//! (`tuning-log.jsonl`)、同一份锁定档 (`trait-locks.json`), 人类界面是
//! 同内核的包装出口 (删掉本命令, agent 侧仍完整可用)。

use apeireth_orchestration::self_tuning::TunableParam;
use apeireth_orchestration::trait_report::TraitReport;
use apeireth_runtime_assembly::{
    now_epoch_ms, set_trait_lock, tuning_log_path, TraitGrowthFacts, TraitGrowthPaths,
};
use serde_json::json;

/// 用法说明 (未知动作/坏参数一律报真因 + 用法)。
pub const USAGE: &str = "apeireth trait report\n\
     apeireth trait lock <slider>\n\
     apeireth trait unlock <slider>\n\
     \n\
     sliders: memory_fade | curiosity_strength | tone_saturation | consolidation_cadence\n\
     旗标: report 可带 --json (布尔旗标, 不带值; JSON 出口)\n\
     动作只有 report / lock / unlock 三个; 多余参数与未知旗标一律拒绝即帧";

/// 滑杆 id 解析 (稳定 id = 调参面板/env 旋钮/工具帧同名)。
pub fn parse_slider(raw: &str) -> Result<TunableParam, String> {
    let wanted = raw.trim();
    TunableParam::ALL
        .into_iter()
        .find(|param| param.id() == wanted)
        .ok_or_else(|| format!("trait: unknown slider {raw:?}\n{USAGE}"))
}

/// 默认落位执行 (成长账 = `tuning-log.jsonl`, 锁定档同目录)。
pub fn run_trait(args: &[String]) -> Result<String, String> {
    run_trait_at(&TraitGrowthPaths::new(tuning_log_path()), args)
}

/// 指定落位的执行内核 (测试可钉死路径; 与 [`run_trait`] 同一语义)。
///
/// 动作只有三个: `report` (只读周报) / `lock` / `unlock` (显式人动作)。
/// 动作名认出来之后, **多余参数与未知旗标各自指名拒** (不再一律报
/// "unknown action" —— 动作名已知时说"未知动作"是把真因说歪了)。
pub fn run_trait_at(paths: &TraitGrowthPaths, args: &[String]) -> Result<String, String> {
    let Some((action, rest)) = args.split_first() else {
        return Err(format!(
            "trait: 缺动作 (只有 report / lock / unlock)\n{USAGE}"
        ));
    };
    match action.as_str() {
        "report" => report_action(paths, rest),
        "lock" | "unlock" => slider_action(paths, action, rest),
        other => Err(format!(
            "trait: unknown action {other:?} (动作只有 report / lock / unlock)\n{USAGE}"
        )),
    }
}

/// `report` 动作: 只认 `--json` 一个布尔旗标; 位置参数一律是多余参数。
fn report_action(paths: &TraitGrowthPaths, rest: &[String]) -> Result<String, String> {
    let scanned = scan_flags("report", "只认 --json", true, rest)?;
    if let Some(extra) = scanned.positional.first() {
        return Err(format!(
            "trait report 多余参数: {extra} (report 只接受 --json)\n{USAGE}"
        ));
    }
    let facts = TraitGrowthFacts::read(paths)?;
    let report = TraitReport::build(&facts.records, &facts.locks, facts.values, now_epoch_ms());
    if scanned.json {
        render_report_json(&report, facts.skipped, paths)
    } else {
        Ok(render_report(&report, facts.skipped, paths))
    }
}

/// `lock` / `unlock` 动作: 收**一个**滑杆名; 旗标一律拒 (这两个动作没有旗标)。
fn slider_action(
    paths: &TraitGrowthPaths,
    action: &str,
    rest: &[String],
) -> Result<String, String> {
    let scanned = scan_flags(action, "只接受一个滑杆名 (不收旗标)", false, rest)?;
    match scanned.positional.as_slice() {
        [slider] => set_lock_at(paths, slider, action == "lock"),
        [] => Err(format!(
            "trait: unknown action usage: {action:?} 需要一个滑杆名 (<slider>)\n{USAGE}"
        )),
        extra => Err(format!(
            "trait {action} 多余参数: {} (只接受一个滑杆名)\n{USAGE}",
            extra[1..].join(" / ")
        )),
    }
}

/// 旗标扫描结果 (布尔旗标不带值; 位置参数含 `--` 之后的词)。
struct Scanned {
    /// `--json` 是否命中。
    json: bool,
    /// 位置参数 (`--` 之后的词也在内)。
    positional: Vec<String>,
}

/// 白名单式旗标扫描 —— **拒绝即帧**。
///
/// 两条纪律 (与 `apeireth egress` 同款):
/// 1. 认识的按类型取: 布尔旗标 (`--json`) **不吞**后面的词;
/// 2. 不认识的**指名拒** (不静默忽略) —— 悬空旗标 / 拼错的旗标都指名报。
///
/// `--` 之后不做旗标解析: 一律当位置参数。
fn scan_flags(
    action: &str,
    known: &str,
    json_allowed: bool,
    args: &[String],
) -> Result<Scanned, String> {
    let mut scanned = Scanned {
        json: false,
        positional: Vec::new(),
    };
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--" {
            scanned.positional.extend(args[index + 1..].iter().cloned());
            break;
        }
        if arg.len() > 1 && arg.starts_with('-') {
            if json_allowed && arg == "--json" {
                scanned.json = true;
                index += 1;
                continue;
            }
            return Err(format!(
                "trait {action} 未知旗标: {arg} ({action} {known})\n{USAGE}"
            ));
        }
        scanned.positional.push(arg.to_string());
        index += 1;
    }
    Ok(scanned)
}

/// 锁定/解锁 (显式动作): 写锁定档并回报当前四杆锁定态。
fn set_lock_at(paths: &TraitGrowthPaths, raw_slider: &str, locked: bool) -> Result<String, String> {
    let param = parse_slider(raw_slider)?;
    let locks = set_trait_lock(&paths.locks_path(), param, locked)?;
    let verb = if locked { "已锁定" } else { "已解锁" };
    let effect = if locked {
        "此后成长沉积跳过该杆并留审计痕 (显式人动作仍可改值)"
    } else {
        "此后成长沉积恢复移动该杆 (解锁 = 显式动作, 无自动解锁路径)"
    };
    let mut out = format!("{verb}{} ({}): {effect}\n", param.label(), param.id());
    out.push_str("当前锁定态:\n");
    for slider in TunableParam::ALL {
        let state = if locks.is_locked(slider) {
            "已锁定"
        } else {
            "未锁定"
        };
        out.push_str(&format!(
            "  [{state}] {} ({})\n",
            slider.label(),
            slider.id()
        ));
    }
    out.push_str(&format!("锁定档: {}\n", paths.locks_path().display()));
    Ok(out)
}

/// 周报文本渲染 (数字与原因全来自成长计既有账, 不编造)。
fn render_report(report: &TraitReport, skipped: usize, paths: &TraitGrowthPaths) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "性格周报 · 近 {} 天 ({} .. {})\n",
        report.window_days, report.window_from_ms, report.window_to_ms
    ));
    for row in &report.sliders {
        let lock = if row.locked { "已锁定" } else { "未锁定" };
        out.push_str(&format!(
            "  [{lock}] {} ({})  值 {:.3}  移动 {:+.3} ({} 次, 锁定跳过 {} 条)\n",
            row.label,
            row.param.id(),
            row.value,
            row.net_delta,
            row.move_count,
            row.locked_skipped
        ));
        match &row.last_move {
            Some(movement) => out.push_str(&format!(
                "      最近移动: {:+.3} @ {} — {}\n",
                movement.delta, movement.at_epoch_ms, movement.reason
            )),
            None => out.push_str("      最近移动: 无\n"),
        }
    }
    out.push_str("移动原因 top:\n");
    if report.top_reasons.is_empty() {
        out.push_str("  (窗内无实际移动)\n");
    }
    for (index, row) in report.top_reasons.iter().enumerate() {
        out.push_str(&format!(
            "  {}. {} ({} 次)\n",
            index + 1,
            row.reason,
            row.count
        ));
    }
    out.push_str(&format!(
        "账面: 窗内记录 {} 条 (锁定跳过 {} 条), 坏行跳过 {} 条\n",
        report.records_in_window, report.locked_skipped, skipped
    ));
    out.push_str(&format!(
        "成长账: {} (锁定档 {})\n",
        paths.log_path.display(),
        paths.locks_path().display()
    ));
    out
}

/// 周报 JSON 渲染 (与 `tool.trait_report` 工具帧**同构**: 同一套字段名与
/// 同一取值口径 —— 名字 / 锁定态 / 各项指标一项不少; 另加两个落位路径)。
///
/// 字段名沿用工具帧的冻结字段 (`write_face` / `window_days` / `sliders` /
/// `top_reasons` …), 两个出口因此可逐值对照。
fn render_report_json(
    report: &TraitReport,
    skipped: usize,
    paths: &TraitGrowthPaths,
) -> Result<String, String> {
    let sliders: Vec<serde_json::Value> = report
        .sliders
        .iter()
        .map(|row| {
            json!({
                "param": row.param.id(),
                "label": row.label,
                "value": row.value,
                "locked": row.locked,
                "net_delta": row.net_delta,
                "move_count": row.move_count,
                "locked_skipped": row.locked_skipped,
                "last_move": row.last_move.as_ref().map(|movement| json!({
                    "param": movement.param.id(),
                    "label": movement.label,
                    "at_epoch_ms": movement.at_epoch_ms,
                    "delta": movement.delta,
                    "reason": movement.reason,
                })),
            })
        })
        .collect();
    let payload = json!({
        "write_face": "read_only",
        "window_days": report.window_days,
        "window_from_ms": report.window_from_ms,
        "window_to_ms": report.window_to_ms,
        "generated_at_ms": report.generated_at_ms,
        "sliders": sliders,
        "records_in_window": report.records_in_window,
        "locked_skipped": report.locked_skipped,
        "skipped_records": skipped,
        "top_reasons": report.top_reasons.iter().map(|row| json!({
            "reason": row.reason,
            "count": row.count,
        })).collect::<Vec<serde_json::Value>>(),
        "ledger_path": paths.log_path.display().to_string(),
        "locks_path": paths.locks_path().display().to_string(),
    });
    serde_json::to_string_pretty(&payload).map_err(|error| format!("性格周报序列化失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_ids_parse_and_unknown_is_refused_with_usage() {
        for slider in TunableParam::ALL {
            assert_eq!(parse_slider(slider.id()).expect("id parses"), slider);
        }
        assert!(parse_slider("not_a_slider")
            .unwrap_err()
            .contains("unknown slider"));
        assert!(parse_slider("not_a_slider")
            .unwrap_err()
            .contains("sliders:"));
    }

    #[test]
    fn unknown_action_is_refused_with_usage() {
        let paths = TraitGrowthPaths::new(std::env::temp_dir().join("apeireth-trait-none.jsonl"));
        let args: Vec<String> = ["delete", "everything"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let error = run_trait_at(&paths, &args).unwrap_err();
        assert!(error.contains("unknown action"), "{error}");
        assert!(error.contains("apeireth trait report"), "{error}");
    }

    /// 多余参数 = 指名「多余参数」, 不再冒充「unknown action」。
    #[test]
    fn extra_arguments_are_named_as_extra_not_as_unknown_action() {
        let paths = TraitGrowthPaths::new(std::env::temp_dir().join("apeireth-trait-extra.jsonl"));
        let report_extra: Vec<String> = ["report", "surplus"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let error = run_trait_at(&paths, &report_extra).unwrap_err();
        assert!(error.contains("多余参数"), "{error}");
        assert!(error.contains("surplus"), "指名多余的那一个: {error}");
        assert!(!error.contains("unknown action"), "{error}");

        let lock_extra: Vec<String> = ["lock", "memory_fade", "tone_saturation"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let error = run_trait_at(&paths, &lock_extra).unwrap_err();
        assert!(error.contains("多余参数"), "{error}");
        assert!(!error.contains("unknown action"), "{error}");
    }

    /// 未知旗标 = 拒绝即帧 + 指名旗标 (不静默忽略)。
    #[test]
    fn unknown_flags_are_refused_by_name() {
        let paths = TraitGrowthPaths::new(std::env::temp_dir().join("apeireth-trait-flag.jsonl"));
        let args: Vec<String> = ["report", "--nope"].iter().map(|s| s.to_string()).collect();
        let error = run_trait_at(&paths, &args).unwrap_err();
        assert!(error.contains("未知旗标"), "{error}");
        assert!(error.contains("--nope"), "指名旗标: {error}");
    }

    /// `--json` 出口 = 与工具帧同构的 JSON (字段名沿用冻结字段)。
    #[test]
    fn report_json_flag_emits_the_tool_frame_shape() {
        let paths = TraitGrowthPaths::new(std::env::temp_dir().join("apeireth-trait-json.jsonl"));
        let args: Vec<String> = ["report", "--json"].iter().map(|s| s.to_string()).collect();
        let out = run_trait_at(&paths, &args).expect("report --json");
        let frame: serde_json::Value = serde_json::from_str(&out).expect("可解析 JSON");
        assert_eq!(frame["write_face"], "read_only");
        assert_eq!(frame["window_days"], 7);
        assert_eq!(frame["sliders"].as_array().expect("sliders").len(), 4);
        assert_eq!(frame["records_in_window"], 0);
    }
}
