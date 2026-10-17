//! V5-b 暴露面体检 CLI 面: `apeireth exposure report [--json] [--out <PATH>] [--bundle <BUNDLE>]`。
//!
//! 同一引擎 ([`apeireth_tools_canonical::exposure_audit`]) 的第二个出口: 模型面是
//! `tool.exposure_audit` (只读档, AI 先用), 本面是人类/运维的包装出口 (不另起
//! 炉灶、不两套逻辑)。体检**只读零写**: 被测面一个字节都不动; 报告默认出
//! stdout, `--out` 是用户显式指定的报告落档 (报告是唯一产出物)。
//!
//! 诚实口径: 只测本地面 (凭据文件分布/明文风险点/本机监听/备份排除项/绑定
//! 完整性); 外网暴露面 (公网入口可达性) 显式「未测」。
//!
//! W3-② 绑定一致性的判定真源在装载面 (`UserAccess::load` 的拒开语义), 本面只
//! 消费判定结果 —— 令牌值零采集, 报告只出计数与绑定名。

use std::path::{Path, PathBuf};

use apeireth_tools_canonical::exposure_audit::{
    run_exposure_audit, BindingProbe, BindingSurface, ExposureAuditConfig,
};

/// 本面的用法帧 (错误即帧: 参数错 = 带用法的正式通道)。
pub const USAGE: &str =
    "Usage:\n  apeireth exposure report [--json] [--out <PATH>] [--bundle <BUNDLE>]";

/// `apeireth exposure` 入口 (env 解析数据根)。
pub fn run_exposure(args: &[String]) -> Result<String, String> {
    run_exposure_at(&crate::default_panel_data_dir(), args)
}

/// 显式数据根入口 (测试与非 env 装配用; 语义与 [`run_exposure`] 逐字一致)。
pub fn run_exposure_at(data_root: &Path, args: &[String]) -> Result<String, String> {
    let config = ExposureAuditConfig::new(data_root).with_bindings(binding_probe(data_root));
    run_exposure_on(&config, args)
}

/// 全注入入口 (探测口可换 fixture; 语义与 [`run_exposure_at`] 逐字一致)。
pub fn run_exposure_on(config: &ExposureAuditConfig, args: &[String]) -> Result<String, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Err(USAGE.to_string()),
    };
    if verb != "report" {
        return Err(format!("unknown exposure command: {verb}\n{USAGE}"));
    }
    let mut json = false;
    let mut out: Option<PathBuf> = None;
    let mut bundle: Option<PathBuf> = None;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--json" => json = true,
            "--out" | "--bundle" => {
                let flag = rest[index].clone();
                index += 1;
                let Some(value) = rest.get(index) else {
                    return Err(format!("exposure report {flag} requires a value\n{USAGE}"));
                };
                if value.trim().is_empty() {
                    return Err(format!(
                        "exposure report {flag} requires a non-empty value\n{USAGE}"
                    ));
                }
                if flag == "--out" {
                    out = Some(PathBuf::from(value));
                } else {
                    bundle = Some(PathBuf::from(value));
                }
            }
            flag if flag.starts_with("--") => {
                return Err(format!("unknown exposure report argument: {flag}\n{USAGE}"));
            }
            value => {
                return Err(format!(
                    "unknown exposure report argument: {value}\n{USAGE}"
                ));
            }
        }
        index += 1;
    }

    let mut config = config.clone();
    if let Some(bundle) = bundle {
        config.bundle = Some(bundle);
    }
    let report = run_exposure_audit(&config);
    let body = if json {
        report.to_json()
    } else {
        report.to_text()
    };
    match out {
        Some(path) => {
            fs_err::write(&path, body.as_bytes())
                .map_err(|error| format!("exposure report write failed: {error}"))?;
            Ok(format!("{body}\nreport written to {}", path.display()))
        }
        None => Ok(body),
    }
}

/// W3-② 绑定一致性的探测口 (真源 = 装载面的拒开语义; 令牌值零采集)。
///
/// 判定链: 客户端令牌表装载 → 用户表装载 + 令牌↔用户绑定一致性校验
/// (`UserAccess::load`, 坏表/绑定不一致拒开) —— 拒开原因即体检发现。
pub fn binding_probe(data_root: impl Into<PathBuf>) -> BindingProbe {
    let data_root = data_root.into();
    std::sync::Arc::new(move || binding_surface_of(&data_root))
}

/// 跑一次绑定一致性判定 (只读; 令牌值不进任何输出)。
fn binding_surface_of(data_root: &Path) -> BindingSurface {
    match apeireth_gateway::ClientTokenTable::load(Some(data_root)) {
        Err(error) => BindingSurface::Checked {
            clients: 0,
            users: 0,
            problems: vec![format!("客户端令牌表拒开: {error}")],
        },
        Ok(tokens) => match apeireth_gateway::UserAccess::load(data_root, &tokens) {
            Err(error) => BindingSurface::Checked {
                clients: tokens.len(),
                users: 0,
                problems: vec![format!("令牌↔用户绑定核对拒开: {error}")],
            },
            Ok(access) => match access.table() {
                None => BindingSurface::Absent {
                    reason: "用户表未配置 (users.json 缺席), 绑定一致性无可核对面".to_string(),
                },
                Some(table) => BindingSurface::Checked {
                    clients: tokens.len(),
                    users: table.users.len(),
                    problems: Vec::new(),
                },
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    #[test]
    fn report_parses_json_out_and_bundle_flags() {
        let config = ExposureAuditConfig::new("data");
        for args in [
            s(&["report"]),
            s(&["report", "--json"]),
            s(&["report", "--json", "--bundle", "bundle-a"]),
            s(&["report", "--bundle", "bundle-a", "--json"]),
        ] {
            assert!(
                run_exposure_on(&config, &args).is_ok(),
                "must accept: {args:?}"
            );
        }
    }

    #[test]
    fn dangling_unknown_and_positional_shapes_refuse_with_usage() {
        let config = ExposureAuditConfig::new("data");
        for args in [
            s(&[]),
            s(&["verify"]),
            s(&["report", "--out"]),
            s(&["report", "--out", "   "]),
            s(&["report", "--nope"]),
            s(&["report", "positional"]),
            s(&["report", "--bundle"]),
        ] {
            let error = run_exposure_on(&config, &args).expect_err("must refuse");
            assert!(error.contains("Usage:"), "错误即帧带用法: {error}");
        }
    }
}
