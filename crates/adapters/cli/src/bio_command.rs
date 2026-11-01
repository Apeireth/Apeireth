//! `apeireth bio` —— 生物机制族的用户开关出口。
//!
//! ADR-0015（owner 2026-10-24 裁定）：**个人线全带 14 条机制，默认关，由用户自行开启**。
//! 本命令是那个「用户能开」的入口 —— 在此之前只有 env 能开，普通用户不会设 env。
//!
//! # 为什么先做 CLI，不做前端
//!
//! 前端正在重构（另一条线负责）。本命令读写**同一个** `bio-mechanisms.json`；
//! 前端重构完成后读同一个文件即可接设置界面，语义不变、不用改这里。
//!
//! # 三个动词
//!
//! - `list`：14 条机制逐条状态（配置是否开 / 有无独立 env / 当前是否生效）；
//! - `enable <name>`：打开一条；
//! - `disable <name>`：关掉一条。
//!
//! **只改配置文件，不碰 env** —— env 是专业用户/CI 的层，优先级高于配置文件；
//! 所以「用户开了但 env 关了」时，实际仍是关的，`list` 会把这一点显示出来。

use std::path::Path;

use crate::bio_switches::{switches_path, BioMechanismSwitches, BIO_MECHANISM_NAMES};

/// 用法帧。
pub const USAGE: &str = "apeireth bio list\napeireth bio enable <mechanism>\napeireth bio disable <mechanism>\n\n  生物机制族（14 条）的用户开关。默认全部关闭。\n  打开一条：apeireth bio enable nociception\n\n  可选机制：neuromodulation / homeostasis / circadian / immune_memory /\n  nociception / recovery / attachment / social_repair / quorum / lifecycle /\n  play / ritual / ecosystem_health / capacity\n\n  配置文件：数据根下 bio-mechanisms.json（前端设置界面将来读同一个文件）。\n  env 优先级更高，可用 =0 显式覆盖配置。";

/// 命令入口（数据根 = 默认数据目录）。
pub fn run_bio(args: &[String]) -> Result<String, String> {
    run_bio_at(&crate::default_panel_data_dir(), args)
}

/// 显式数据根入口（测试与非 env 装配用；语义与 [`run_bio`] 逐字一致）。
pub fn run_bio_at(data_root: &Path, args: &[String]) -> Result<String, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Err(USAGE.to_string()),
    };
    match verb {
        "list" => list(data_root),
        "enable" => set(data_root, rest, true),
        "disable" => set(data_root, rest, false),
        other => Err(format!("unknown bio command: {other}\n{USAGE}")),
    }
}

/// 列出 14 条机制的状态。
fn list(data_root: &Path) -> Result<String, String> {
    let switches = BioMechanismSwitches::load(data_root)?;
    let mut out = format!(
        "生物机制族：14 条，默认全部关闭\n配置文件：{}\n",
        switches_path(data_root).display()
    );
    if switches.enabled.is_empty() {
        out.push_str("（当前没有开启任何一条）\n");
    }
    out.push_str("\n机制名                配置   独立env      生效\n");
    out.push_str("------------------------------------------------\n");
    for name in BIO_MECHANISM_NAMES {
        let in_file = switches.is_enabled_in_file(name);
        let has_env = BioMechanismSwitches::has_env_switch(name);
        let effective = switches.effective(name);
        out.push_str(&format!(
            "{name:<20}  {file:<5} {env:<12} {eff}\n",
            file = if in_file { "开" } else { "关" },
            env = if has_env { "有" } else { "无" },
            eff = if effective {
                "是"
            } else if in_file && has_env {
                // 配置文件开了但被 env 关掉 —— 这是最容易困惑的状态，必须说出来。
                "否（被 env 覆盖）"
            } else {
                "否"
            }
        ));
    }
    out.push_str(
        "\n说明：\n\
         · 「配置」= bio-mechanisms.json 里是否开启（本命令改这一层）；\n\
         · 「独立env」= 该机制是否有自己的 env 开关；**无**的机制目前只能靠配置文件，\n\
         且装配面若未接该开关则不会生效（自述面 capabilities.bio_mechanisms 的 wired 字段为准）；\n\
         · 「生效」= 配置面判定，不是「真的在跑」——后者看自述面的 wired。\n",
    );
    Ok(out)
}

/// 开关一条机制。
fn set(data_root: &Path, rest: &[String], on: bool) -> Result<String, String> {
    let name = match rest.first() {
        Some(name) => name.as_str(),
        None => return Err(format!("需要指定机制名\n{USAGE}")),
    };
    let mut switches = BioMechanismSwitches::load(data_root)?;
    let changed = if on {
        switches.enable(name)?
    } else {
        switches.disable(name)?
    };
    switches.save(data_root)?;
    let verb = if on { "已打开" } else { "已关闭" };
    let mut out = if changed {
        format!("{verb}：{name}\n")
    } else {
        format!(
            "{name} 本来就是{}状态（未改变）\n",
            if on { "打开" } else { "关闭" }
        )
    };
    out.push_str(&format!(
        "配置文件：{}\n",
        switches_path(data_root).display()
    ));
    // 如实提示：配置文件开了但 env 关着 = 实际不开。
    if on && !switches.effective(name) {
        out.push_str(
            "⚠️ 注意：这条机制有独立 env 开关且当前 env 把它关着，\
             所以实际仍未生效（env 优先级高于配置文件）。\n",
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-bio-cmd-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn list_shows_all_fourteen() {
        let root = temp_root("list");
        let out = run_bio_at(&root, &["list".into()]).expect("应成功");
        for name in BIO_MECHANISM_NAMES {
            assert!(out.contains(name), "list 应包含 {name}");
        }
        assert!(out.contains("默认全部关闭"));
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn enable_then_disable_round_trips() {
        let root = temp_root("toggle");
        let out = run_bio_at(&root, &["enable".into(), "play".into()]).expect("enable");
        assert!(out.contains("已打开"), "应报告打开: {out}");

        let listed = run_bio_at(&root, &["list".into()]).expect("list");
        // play 有独立 env；若 env 未设则生效。
        assert!(listed.contains("play"));

        let out = run_bio_at(&root, &["disable".into(), "play".into()]).expect("disable");
        assert!(out.contains("已关闭"), "应报告关闭: {out}");
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn unknown_mechanism_is_refused_with_hint() {
        let root = temp_root("unknown");
        let err = run_bio_at(&root, &["enable".into(), "happiness".into()]).expect_err("应拒绝");
        assert!(err.contains("未知生物机制"), "理由应可读: {err}");
        assert!(err.contains("nociception"), "应列出可选名: {err}");
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn unknown_verb_is_refused() {
        let root = temp_root("verb");
        let err = run_bio_at(&root, &["dance".into()]).expect_err("应拒绝");
        assert!(err.contains("unknown bio command"));
    }

    #[test]
    fn mechanisms_without_env_are_marked_honestly() {
        let root = temp_root("noenv");
        let out = run_bio_at(&root, &["list".into()]).expect("list");
        // capacity 没有独立 env —— list 必须如实显示「无」。
        let capacity_line = out
            .lines()
            .find(|line| line.starts_with("capacity"))
            .expect("应有 capacity 行");
        assert!(
            capacity_line.contains("无"),
            "capacity 应标「无」独立 env: {capacity_line}"
        );
    }
}
