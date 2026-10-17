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
//! 与 `tool.trait_report` **同一内核两个出口**: 周报读数同一本成长账
//! (`tuning-log.jsonl`)、同一份锁定档 (`trait-locks.json`), 人类界面是
//! 同内核的包装出口 (删掉本命令, agent 侧仍完整可用)。

use apeireth_orchestration::self_tuning::TunableParam;
use apeireth_orchestration::trait_report::TraitReport;
use apeireth_runtime_assembly::{
    now_epoch_ms, set_trait_lock, tuning_log_path, TraitGrowthFacts, TraitGrowthPaths,
};

/// 用法说明 (未知动作/坏参数一律报真因 + 用法)。
pub const USAGE: &str = "apeireth trait report\n\
     apeireth trait lock <slider>\n\
     apeireth trait unlock <slider>\n\
     \n\
     sliders: memory_fade | curiosity_strength | tone_saturation | consolidation_cadence";

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
pub fn run_trait_at(paths: &TraitGrowthPaths, args: &[String]) -> Result<String, String> {
    match args.split_first() {
        Some((action, rest)) if action == "report" && rest.is_empty() => report_at(paths),
        Some((action, rest)) if (action == "lock" || action == "unlock") && rest.len() == 1 => {
            let locked = action == "lock";
            set_lock_at(paths, &rest[0], locked)
        }
        _ => Err(format!("trait: unknown action\n{USAGE}")),
    }
}

/// 周报 (只读): 近 7 天四杆移动量 / 移动原因 top / 锁定态 (真值打印)。
fn report_at(paths: &TraitGrowthPaths) -> Result<String, String> {
    let facts = TraitGrowthFacts::read(paths)?;
    let report = TraitReport::build(&facts.records, &facts.locks, facts.values, now_epoch_ms());
    Ok(render_report(&report, facts.skipped, paths))
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
}
