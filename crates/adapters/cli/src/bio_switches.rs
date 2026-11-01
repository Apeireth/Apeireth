//! 生物机制族的**用户开关面**（配置文件 + CLI）。
//!
//! # 为什么需要这个模块
//!
//! 14 条生物机制（SG-13…SG-26）本来就是「能力已实现、默认关、逐条可开」——
//! 但它们**只能通过 env 打开**（每条机制各有一个开关环境变量）。
//! 普通用户不会去设环境变量 ⇒ 实际上「用户能自己打开」这件事**不成立**。
//!
//! ADR-0015 裁定（owner 2026-10-24）：**个人线全带 14 条机制，默认关，
//! 由用户自行开启**。本模块就是那个「用户能开」的入口。
//!
//! # 为什么先做配置文件 + CLI，不做前端
//!
//! 前端正在重构（另一条线负责）。本模块只提供**后端数据面**：
//! 用户现在就能用 `apeireth bio enable <name>` 打开；
//! 前端重构完成后，读**同一个** `bio-mechanisms.json` 即可接设置界面，
//! 不需要再改动本模块的语义。
//!
//! # 优先级（三层，env 最高）
//!
//! ```text
//! 1. env 显式设置（见 `env_name_for` 的映射表）
//!    —— 专业用户 / CI / 临时调试，**最高优先**，且允许用 `=0` 显式关掉配置文件里的开启
//! 2. 用户配置文件 `bio-mechanisms.json`（本模块维护）
//! 3. 默认关
//! ```
//!
//! env 能用 `=0` 关掉配置文件的开启 —— 这条很重要：否则「用户开了但想临时关一下」
//! 就没有路径（只能改文件）。
//!
//! # 纪律
//!
//! - **默认关**：文件不存在 / 解析失败 / 名字不认识 ⇒ 该机制关（fail-closed）；
//! - **零行为变化**：没有配置文件时，行为与「只有 env」完全一致；
//! - **名字闭合**：只认 [`BIO_MECHANISM_NAMES`] 里的 14 个名字，拼错即拒绝（不猜）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 14 条机制的规范名（与 `BioMechanismKind::as_str` 逐一对应，闭合词表）。
pub const BIO_MECHANISM_NAMES: [&str; 14] = [
    "neuromodulation",
    "homeostasis",
    "circadian",
    "immune_memory",
    "nociception",
    "recovery",
    "attachment",
    "social_repair",
    "quorum",
    "lifecycle",
    "play",
    "ritual",
    "ecosystem_health",
    "capacity",
];

/// 每条机制对应的 env 名（`None` = 该机制没有独立 env，只有配置文件能开）。
///
/// **这是当前装配面的真实映射**，不是理想映射 —— 有几个机制共用或没有独立开关
/// （例如 `attachment` / `social_repair` 的候选由 care candidates 总开关控制）。
/// 映射缺的机制在配置文件里打开后，仍需等装配面补上独立开关才会生效；
/// 本模块**如实返回**「env 名未知」，不假装。
pub fn env_name_for(mechanism: &str) -> Option<&'static str> {
    match mechanism {
        "nociception" => Some("APEIRETH_ENABLE_BIO_PAIN"),
        "circadian" => Some("APEIRETH_ENABLE_BIO_CIRCADIAN"),
        "immune_memory" => Some("APEIRETH_ENABLE_IMMUNE_MEMORY"),
        "quorum" => Some("APEIRETH_ENABLE_QUORUM_GATE"),
        "neuromodulation" => Some("APEIRETH_ENABLE_BIO_TONE"),
        "recovery" => Some("APEIRETH_ENABLE_BIO_REPAIR_GUIDANCE"),
        "lifecycle" => Some("APEIRETH_ENABLE_BIO_LIFECYCLE"),
        "play" => Some("APEIRETH_ENABLE_BIO_PLAY"),
        "attachment" | "ritual" => Some("APEIRETH_ENABLE_BIO_CARE_CANDIDATES"),
        "social_repair" => Some("APEIRETH_ENABLE_BIO_CONTEXT_POLICY"),
        _ => None,
    }
}

/// 配置文件文件名（数据根下）。
pub const BIO_SWITCHES_FILE: &str = "bio-mechanisms.json";

/// 配置文件格式。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BioMechanismSwitches {
    /// 格式版本（当前恒 1）。
    #[serde(default = "default_version")]
    pub version: u32,
    /// 用户显式开启的机制（规范名）。
    #[serde(default)]
    pub enabled: BTreeSet<String>,
}

fn default_version() -> u32 {
    1
}

impl BioMechanismSwitches {
    /// 从数据根加载（文件不存在 / 解析失败 = 全关，fail-closed）。
    ///
    /// 解析失败时**不静默**：`Err` 里带原因，由调用方决定怎么报。
    pub fn load(data_root: &Path) -> Result<Self, String> {
        let path = data_root.join(BIO_SWITCHES_FILE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            // 文件不存在 = 用户还没开过任何机制（默认关）。
            return Ok(Self {
                version: 1,
                enabled: BTreeSet::new(),
            });
        };
        let raw: Self = serde_json::from_str(&text)
            .map_err(|error| format!("{} 解析失败: {error}", path.display()))?;
        // 过滤未知名字（不认识的忽略，但保留在文件里以便用户自己看）。
        let known: BTreeSet<String> = BIO_MECHANISM_NAMES.iter().map(|n| n.to_string()).collect();
        Ok(Self {
            version: raw.version,
            enabled: raw.enabled.intersection(&known).cloned().collect(),
        })
    }

    /// 写回数据根。
    pub fn save(&self, data_root: &Path) -> Result<(), String> {
        let path = data_root.join(BIO_SWITCHES_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("建目录 {} 失败: {error}", parent.display()))?;
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|error| format!("序列化失败: {error}"))?;
        std::fs::write(&path, text + "\n")
            .map_err(|error| format!("写 {} 失败: {error}", path.display()))
    }

    /// 某机制是否在配置文件里开启。
    pub fn is_enabled_in_file(&self, mechanism: &str) -> bool {
        self.enabled.contains(mechanism)
    }

    /// 打开一条机制（返回是否发生改变）。
    pub fn enable(&mut self, mechanism: &str) -> Result<bool, String> {
        Self::validate_name(mechanism)?;
        Ok(self.enabled.insert(mechanism.to_string()))
    }

    /// 关掉一条机制（返回是否发生改变）。
    pub fn disable(&mut self, mechanism: &str) -> Result<bool, String> {
        Self::validate_name(mechanism)?;
        Ok(self.enabled.remove(mechanism))
    }

    /// 名字必须闭集内（拼错即拒绝 —— 不猜用户想开哪条）。
    pub fn validate_name(mechanism: &str) -> Result<(), String> {
        if BIO_MECHANISM_NAMES.contains(&mechanism) {
            return Ok(());
        }
        Err(format!(
            "未知生物机制 `{mechanism}`。可选：{}",
            BIO_MECHANISM_NAMES.join(" / ")
        ))
    }

    /// **三层优先级解析**：给装配面用的 `raw` 值。
    ///
    /// 返回 `Option<String>`，语义与既有 `*_from_env_value(raw)` 完全兼容：
    /// - `Some(v)` → 交给既有纯函数判 `"1"`；
    /// - `None` → 既有纯函数按「未设」处理（= 关）。
    ///
    /// 优先级：env 有值就用 env（含 `=0` 显式关）→ 配置文件 → `None`。
    pub fn raw_for(&self, mechanism: &str) -> Option<String> {
        if let Some(env) = env_name_for(mechanism) {
            if let Ok(value) = std::env::var(env) {
                return Some(value);
            }
        }
        if self.is_enabled_in_file(mechanism) {
            return Some("1".to_string());
        }
        None
    }

    /// 解析某机制**当前是否生效**（配置面视角，不含装配面是否接线）。
    ///
    /// ⚠️ 这是**配置是否打开**，不是「该机制是否真的在跑」 ——
    /// 后者要看自述面 `capabilities.bio_mechanisms` 的 `wired` 字段。
    /// 两个问题分开答，不混。
    pub fn effective(&self, mechanism: &str) -> bool {
        self.raw_for(mechanism).as_deref().map(str::trim) == Some("1")
    }

    /// 该机制是否有独立 env 开关（`false` = 只有配置文件能开，装配面可能还没接）。
    pub fn has_env_switch(mechanism: &str) -> bool {
        env_name_for(mechanism).is_some()
    }
}

/// 配置文件的完整路径（报告面用）。
pub fn switches_path(data_root: &Path) -> PathBuf {
    data_root.join(BIO_SWITCHES_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-bio-switches-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn missing_file_means_all_off() {
        let root = temp_root("missing");
        let switches = BioMechanismSwitches::load(&root).expect("缺文件应成功（全关）");
        assert!(switches.enabled.is_empty());
        for name in BIO_MECHANISM_NAMES {
            assert!(!switches.is_enabled_in_file(name));
        }
    }

    #[test]
    fn enable_persists_and_reloads() {
        let root = temp_root("persist");
        let mut switches = BioMechanismSwitches::load(&root).expect("load");
        assert!(switches.enable("nociception").expect("enable"));
        switches.save(&root).expect("save");

        let reloaded = BioMechanismSwitches::load(&root).expect("reload");
        assert!(reloaded.is_enabled_in_file("nociception"));
        assert!(!reloaded.is_enabled_in_file("play"));
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn unknown_name_is_refused() {
        let mut switches = BioMechanismSwitches::default();
        let err = switches.enable("pain").expect_err("短名应被拒");
        assert!(err.contains("未知生物机制"), "理由应可读: {err}");
        assert!(err.contains("nociception"), "应提示正确名字: {err}");
    }

    #[test]
    fn malformed_file_is_an_error_not_silent_off() {
        let root = temp_root("malformed");
        std::fs::create_dir_all(&root).expect("dir");
        std::fs::write(switches_path(&root), "{ not json").expect("write");
        let err = BioMechanismSwitches::load(&root).expect_err("坏文件应报错");
        assert!(err.contains("解析失败"), "理由应可读: {err}");
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn unknown_names_in_file_are_ignored() {
        let root = temp_root("unknown");
        std::fs::create_dir_all(&root).expect("dir");
        std::fs::write(
            switches_path(&root),
            r#"{"version":1,"enabled":["nociception","not_a_mechanism"]}"#,
        )
        .expect("write");
        let switches = BioMechanismSwitches::load(&root).expect("load");
        assert!(switches.is_enabled_in_file("nociception"));
        assert!(!switches.is_enabled_in_file("not_a_mechanism"));
        let _ = std::fs::remove_file(switches_path(&root));
    }

    #[test]
    fn config_file_makes_raw_some_and_effective_true() {
        let root = temp_root("effective");
        let mut switches = BioMechanismSwitches::load(&root).expect("load");
        // 用一条没有 env 干扰的机制（ecosystem_health 无独立 env）。
        switches.enable("ecosystem_health").expect("enable");
        assert_eq!(
            switches.raw_for("ecosystem_health").as_deref(),
            Some("1"),
            "配置文件开启应给出 raw=1"
        );
        assert!(switches.effective("ecosystem_health"));
    }

    #[test]
    fn not_enabled_gives_none() {
        let switches = BioMechanismSwitches::default();
        // 注意：若 CI 环境恰好设了 env，这条断言会失败 —— 故只断配置文件视角。
        assert!(!switches.is_enabled_in_file("capacity"));
        assert_eq!(switches.raw_for("capacity"), None);
    }

    #[test]
    fn vocabulary_is_closed_and_complete() {
        assert_eq!(BIO_MECHANISM_NAMES.len(), 14);
        // 名字唯一。
        let unique: BTreeSet<&&str> = BIO_MECHANISM_NAMES.iter().collect();
        assert_eq!(unique.len(), 14);
        // 没有独立 env 的机制如实返回 None（不假装）。
        assert!(env_name_for("capacity").is_none());
        assert!(env_name_for("ecosystem_health").is_none());
        assert!(env_name_for("homeostasis").is_none());
    }
}
