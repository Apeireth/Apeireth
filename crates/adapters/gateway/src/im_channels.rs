//! IM 快捷接入: 渠道配置面 (件一)。
//!
//! 两个入口, 一份 fail-closed 契约:
//!
//! * `APEIRETH_IM_CHANNELS` (JSON 数组, 每项 `{id, kind, webhook_or_endpoint,
//!   secret?, enabled}`) 是主入口: 设了就必须解析并通过校验, 否则加载失败;
//! * 数据目录的 `im-channels.json` 是次入口, 走存储文档单文档语义
//!   (身份 + 信封自洽 + 版本契约): 坏文件**拒绝打开**, 0 静默回退默认。
//!
//! 装配面 ([`assemble_im_channels`]) 把启用渠道变成传输目标 + 重连预算 +
//! **脱敏启动日志**: 渠道共享秘密 0 明文进日志 (只报 `configured` / `none`)。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_sdk::im::{ImChannelKind, ImChannelTarget, ImReconnectPolicy, ImSecret};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 渠道列表的环境变量入口 (JSON 数组)。
pub const IM_CHANNELS_ENV: &str = "APEIRETH_IM_CHANNELS";

/// 数据目录里的渠道列表文件名。
pub const IM_CHANNELS_FILE: &str = "im-channels.json";

/// 渠道列表文件的存储文档身份。
pub const IM_CHANNELS_DOC_NAME: &str = "im-channels";

/// 渠道列表文件的格式版本。
pub const IM_CHANNELS_DOC_VERSION: u32 = 1;

/// 一个渠道配置 (wire 形状: `{id, kind, webhook_or_endpoint, secret?, enabled}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImChannelSpec {
    /// 渠道 id (稳定标识, 端点日志与会话映射的 key)。
    pub id: String,
    /// 渠道 kind (`im-feishu` / `im-wecom` / `im-qq`)。
    pub kind: ImChannelKind,
    /// 出站 webhook / endpoint (绝对 http(s) URL)。
    pub webhook_or_endpoint: String,
    /// 入站校验共享秘密 (可选; 0 明文进日志)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<ImSecret>,
    /// 是否接入运行 (显式字段, 0 默认)。
    pub enabled: bool,
}

impl ImChannelSpec {
    /// 结构校验: id 非空且不重复由列表校验, 端点绝对 URL, 秘密非空。
    pub fn validate(&self) -> Result<(), ImConfigError> {
        if self.id.trim().is_empty() {
            return Err(ImConfigError::InvalidSpec {
                id: "<unnamed>".to_string(),
                reason: "channel id must not be empty".to_string(),
            });
        }
        if self.id.len() > 64 {
            return Err(ImConfigError::InvalidSpec {
                id: self.id.clone(),
                reason: "channel id must be at most 64 bytes".to_string(),
            });
        }
        if !apeireth_sdk::im::is_absolute_http_url(&self.webhook_or_endpoint) {
            return Err(ImConfigError::InvalidSpec {
                id: self.id.clone(),
                reason: "webhook_or_endpoint must be an absolute http(s) URL".to_string(),
            });
        }
        Ok(())
    }

    /// 转成传输目标 (含共享秘密)。
    pub fn target(&self) -> Result<ImChannelTarget, ImConfigError> {
        ImChannelTarget::new(
            self.id.clone(),
            self.kind,
            self.webhook_or_endpoint.clone(),
            self.secret.clone(),
        )
        .map_err(|e| ImConfigError::InvalidSpec {
            id: self.id.clone(),
            reason: e.to_string(),
        })
    }

    /// 脱敏启动日志行 (秘密只报 `configured` / `none`, 0 明文)。
    pub fn redacted_summary(&self) -> String {
        format!(
            "{} enabled={}",
            self.target()
                .map(|target| target.redacted_summary())
                .unwrap_or_else(|_| format!(
                    "im channel \"{}\": <invalid spec>",
                    if self.id.is_empty() {
                        "<unnamed>"
                    } else {
                        &self.id
                    }
                )),
            self.enabled
        )
    }
}

/// 渠道列表 (配置主体)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImChannelConfig {
    /// 全部渠道 (配置顺序)。
    pub channels: Vec<ImChannelSpec>,
}

impl ImChannelConfig {
    /// 显式列表构造。
    pub fn new(channels: Vec<ImChannelSpec>) -> Self {
        Self { channels }
    }

    /// 解析 JSON (env 的数组形或文件的 `{"channels": [...]}` 形), 然后校验。
    /// fail-closed: 任何缺陷都是错误, 0 回退默认。
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, ImConfigError> {
        let config: Self = match value {
            serde_json::Value::Array(items) => Self {
                channels: serde_json::from_value(serde_json::Value::Array(items)).map_err(|e| {
                    ImConfigError::InvalidJson {
                        reason: e.to_string(),
                    }
                })?,
            },
            serde_json::Value::Object(_) => {
                serde_json::from_value(value).map_err(|e| ImConfigError::InvalidJson {
                    reason: e.to_string(),
                })?
            }
            other => {
                return Err(ImConfigError::InvalidJson {
                    reason: format!("expected a JSON array or object, found {other}"),
                })
            }
        };
        config.validate()?;
        Ok(config)
    }

    /// 解析 `APEIRETH_IM_CHANNELS` 原始值。fail-closed。
    pub fn from_env_value(raw: &str) -> Result<Self, ImConfigError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| ImConfigError::InvalidJson {
                reason: format!("{IM_CHANNELS_ENV}: {e}"),
            })?;
        Self::from_json_value(value)
    }

    /// 校验每个渠道 + 拒绝重复 id。
    pub fn validate(&self) -> Result<(), ImConfigError> {
        let mut seen = std::collections::BTreeSet::new();
        for spec in &self.channels {
            spec.validate()?;
            if !seen.insert(spec.id.clone()) {
                return Err(ImConfigError::DuplicateChannel {
                    id: spec.id.clone(),
                });
            }
        }
        Ok(())
    }

    /// 主入口: 未设/空值 = `Ok(None)`; 有值但不合法 = 错误 (0 静默丢弃)。
    pub fn load_from_env() -> Result<Option<Self>, ImConfigError> {
        match std::env::var(IM_CHANNELS_ENV) {
            Ok(raw) if raw.trim().is_empty() => Ok(None),
            Ok(raw) => Self::from_env_value(&raw).map(Some),
            Err(_) => Ok(None),
        }
    }

    /// 次入口: 数据目录文件缺失 = `Ok(None)`; 存在但拒开 = 错误。
    pub fn load_from_data_dir(data_dir: &Path) -> Result<Option<Self>, ImConfigError> {
        let path = im_channels_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc =
            stored_doc::open_single::<Self>(&path, &doc_compat()).map_err(ImConfigError::Stored)?;
        doc.body.validate()?;
        Ok(Some(doc.body))
    }

    /// 主入口优先, 次入口兜底, 都没有 = 空配置 (inert)。
    pub fn load(data_dir: Option<&Path>) -> Result<Self, ImConfigError> {
        if let Some(config) = Self::load_from_env()? {
            return Ok(config);
        }
        if let Some(dir) = data_dir {
            if let Some(config) = Self::load_from_data_dir(dir)? {
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    /// 启用的渠道 (配置顺序)。
    pub fn enabled_channels(&self) -> impl Iterator<Item = &ImChannelSpec> {
        self.channels.iter().filter(|spec| spec.enabled)
    }

    /// 脱敏启动日志: 每渠道一行。
    pub fn redacted_startup_log(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(ImChannelSpec::redacted_summary)
            .collect()
    }
}

/// 数据目录里的渠道列表路径。
pub fn im_channels_path(data_dir: &Path) -> PathBuf {
    data_dir.join(IM_CHANNELS_FILE)
}

/// 渠道列表文件的读契约 (仅当前版本可读)。
pub fn doc_compat() -> DocCompat {
    DocCompat::exact(IM_CHANNELS_DOC_NAME, IM_CHANNELS_DOC_VERSION)
}

/// 装配结果: 启用渠道的传输目标 + 重连预算 + 脱敏启动日志。
#[derive(Debug, Clone, Default)]
pub struct ImChannelAssembly {
    /// 启用渠道的传输目标 (按配置顺序)。
    pub targets: Vec<ImChannelTarget>,
    /// 断线重连预算 (出站传输共用)。
    pub reconnect: ImReconnectPolicy,
    /// 脱敏启动日志行。
    pub startup_log: Vec<String>,
}

impl ImChannelAssembly {
    /// 是否有渠道接入运行 (无 = 本地审批/对话零回归, IM 面 inert)。
    pub fn is_active(&self) -> bool {
        !self.targets.is_empty()
    }

    /// 按渠道 id 取传输目标。
    pub fn target(&self, channel_id: &str) -> Option<&ImChannelTarget> {
        self.targets.iter().find(|target| target.id == channel_id)
    }
}

/// 启动装配: 配置 → 启用渠道目标 + 重连预算 + 脱敏启动日志。
///
/// 失败条件: 任一启用渠道的目标不可构造 (fail-closed: 不跳过坏渠道)。
pub fn assemble_im_channels(
    config: &ImChannelConfig,
    reconnect: ImReconnectPolicy,
) -> Result<ImChannelAssembly, ImConfigError> {
    config.validate()?;
    let mut targets = Vec::new();
    for spec in config.enabled_channels() {
        targets.push(spec.target()?);
    }
    let startup_log = config.redacted_startup_log();
    Ok(ImChannelAssembly {
        targets,
        reconnect,
        startup_log,
    })
}

/// 渠道配置缺陷。每个变体对加载都是致命的: 坏配置拒绝打开。
#[derive(Debug, Error)]
pub enum ImConfigError {
    /// JSON 形状/解析不过。
    #[error("im channel configuration is not valid JSON: {reason}")]
    InvalidJson {
        /// 解析器拒绝的原因。
        reason: String,
    },
    /// 单个渠道结构非法。
    #[error("im channel {id:?} is invalid: {reason}")]
    InvalidSpec {
        /// 渠道 id (或 `<unnamed>`)。
        id: String,
        /// 拒绝原因。
        reason: String,
    },
    /// 渠道 id 重复。
    #[error("im channel {id:?} is configured twice")]
    DuplicateChannel {
        /// 重复的渠道 id。
        id: String,
    },
    /// 存储文档拒开 (坏 JSON / 身份不符 / 版本不可读)。
    #[error("im channel configuration file refused to open: {0}")]
    Stored(#[from] StoredDocError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str) -> ImChannelSpec {
        ImChannelSpec {
            id: id.to_string(),
            kind: ImChannelKind::ImFeishu,
            webhook_or_endpoint: "https://open.example.test/hook".to_string(),
            secret: Some(ImSecret::new("shared-super-secret").unwrap()),
            enabled: true,
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-im-config-{tag}-{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn env_shape_parses_and_validates() {
        let raw = r#"[{"id":"primary","kind":"im-feishu","webhook_or_endpoint":"https://open.example.test/hook","secret":"s","enabled":true}]"#;
        let config = ImChannelConfig::from_env_value(raw).unwrap();
        assert_eq!(config.channels.len(), 1);
        assert_eq!(config.channels[0].kind, ImChannelKind::ImFeishu);
        assert!(config.channels[0].enabled);
    }

    #[test]
    fn broken_configuration_fails_closed() {
        assert!(ImChannelConfig::from_env_value("not json").is_err());
        let unknown_kind = r#"[{"id":"a","kind":"im-other","webhook_or_endpoint":"https://x.test/h","enabled":true}]"#;
        assert!(ImChannelConfig::from_env_value(unknown_kind).is_err());
        let relative_endpoint =
            r#"[{"id":"a","kind":"im-qq","webhook_or_endpoint":"/hook","enabled":true}]"#;
        assert!(ImChannelConfig::from_env_value(relative_endpoint).is_err());
        let duplicate = r#"[
            {"id":"a","kind":"im-qq","webhook_or_endpoint":"https://x.test/h","enabled":true},
            {"id":"a","kind":"im-qq","webhook_or_endpoint":"https://x.test/h2","enabled":true}
        ]"#;
        assert!(matches!(
            ImChannelConfig::from_env_value(duplicate),
            Err(ImConfigError::DuplicateChannel { .. })
        ));
        let missing_endpoint = r#"[{"id":"a","kind":"im-wecom","enabled":true}]"#;
        assert!(ImChannelConfig::from_env_value(missing_endpoint).is_err());
    }

    #[test]
    fn stored_doc_file_is_loaded_and_broken_file_refuses_to_open() {
        let dir = temp_dir("stored");
        let path = im_channels_path(&dir);
        let body = ImChannelConfig::new(vec![spec("primary")]);
        stored_doc::save_single(&path, &doc_compat(), body, stored_doc::DEFAULT_DOC_MODE).unwrap();
        let loaded = ImChannelConfig::load_from_data_dir(&dir)
            .unwrap()
            .expect("file present");
        assert_eq!(loaded.channels[0].id, "primary");

        fs_err::write(&path, "{ not json").unwrap();
        assert!(ImChannelConfig::load_from_data_dir(&dir).is_err());
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_reads_as_absent_not_as_default_channels() {
        let dir = temp_dir("absent");
        assert!(ImChannelConfig::load_from_data_dir(&dir).unwrap().is_none());
        let _ = fs_err::remove_dir_all(&dir);
    }

    #[test]
    fn startup_log_redacts_the_shared_secret() {
        let config = ImChannelConfig::new(vec![spec("primary")]);
        for line in config.redacted_startup_log() {
            assert!(!line.contains("shared-super-secret"), "{line}");
            assert!(line.contains("secret=configured"), "{line}");
            assert!(line.contains("kind=im-feishu"), "{line}");
            assert!(line.contains("enabled=true"), "{line}");
        }
    }

    #[test]
    fn assembly_keeps_only_enabled_channels_and_is_inert_when_empty() {
        let mut disabled = spec("off");
        disabled.enabled = false;
        let config = ImChannelConfig::new(vec![spec("on"), disabled]);
        let assembly = assemble_im_channels(&config, ImReconnectPolicy::default()).unwrap();
        assert!(assembly.is_active());
        assert_eq!(assembly.targets.len(), 1);
        assert_eq!(assembly.targets[0].id, "on");
        assert!(assembly.target("on").is_some());
        assert!(assembly.target("off").is_none());

        let empty = assemble_im_channels(&ImChannelConfig::default(), ImReconnectPolicy::default())
            .unwrap();
        assert!(!empty.is_active());
    }
}
