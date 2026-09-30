//! IM 快捷接入: 渠道 kind 词表 + 传输目标。
//!
//! kind 是中性渠道标识 (`im-feishu` / `im-wecom` / `im-qq`), 闭合 3 variant,
//! 解析 fail-closed: 未知 kind 报 [`crate::im::error::ImError::InvalidConfig`],
//! 0 静默默认。[`ImChannelTarget`] 是传输层视角的渠道 (id + kind + 端点 +
//! 可选共享秘密), 由配置层装配后注入。

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::im::error::{ImError, ImResult};
use crate::im::secret::ImSecret;

/// 渠道 kind 数守门 (3 个中性渠道 id)。
pub const IM_CHANNEL_KIND_COUNT: usize = 3;

/// 渠道 kind (闭合 3 variant, wire 值即中性渠道 id)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImChannelKind {
    /// `im-feishu` 渠道 (消息/卡片 wire 面消费 [`crate::lark`] 适配族)。
    ImFeishu,
    /// `im-wecom` 渠道。
    ImWecom,
    /// `im-qq` 渠道。
    ImQq,
}

impl ImChannelKind {
    /// kind 数守门 (3)。
    pub const COUNT: usize = IM_CHANNEL_KIND_COUNT;

    /// 三个 kind 的全集 (wire 序)。
    pub const ALL: [Self; 3] = [Self::ImFeishu, Self::ImWecom, Self::ImQq];

    /// wire 字符串 (中性渠道 id)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ImFeishu => "im-feishu",
            Self::ImWecom => "im-wecom",
            Self::ImQq => "im-qq",
        }
    }

    /// 解析 wire 字符串; 未知 kind = `None` (0 静默默认)。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "im-feishu" => Some(Self::ImFeishu),
            "im-wecom" => Some(Self::ImWecom),
            "im-qq" => Some(Self::ImQq),
            _ => None,
        }
    }

    /// 该 kind 单条文本消息的字节上限 (分段策略的上界)。
    pub const fn max_text_bytes(self) -> usize {
        match self {
            Self::ImFeishu => crate::lark::MAX_MESSAGE_TEXT_BYTES,
            Self::ImWecom => 4096,
            Self::ImQq => 4096,
        }
    }
}

impl std::fmt::Display for ImChannelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ImChannelKind {
    type Err = ImError;

    fn from_str(raw: &str) -> ImResult<Self> {
        Self::parse(raw).ok_or_else(|| ImError::InvalidConfig {
            reason: format!("unknown channel kind {raw:?}"),
        })
    }
}

/// 传输层视角的渠道目标 (配置层装配后注入)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImChannelTarget {
    /// 配置里的渠道 id。
    pub id: String,
    /// 渠道 kind。
    pub kind: ImChannelKind,
    /// 出站 webhook / endpoint (绝对 http(s) URL)。
    pub endpoint: String,
    /// 入站校验共享秘密 (可选; 0 明文进日志)。
    pub secret: Option<ImSecret>,
}

impl ImChannelTarget {
    /// 构造传输目标 (结构校验: id 非空 + 端点是绝对 http(s) URL)。
    pub fn new(
        id: impl Into<String>,
        kind: ImChannelKind,
        endpoint: impl Into<String>,
        secret: Option<ImSecret>,
    ) -> ImResult<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ImError::InvalidConfig {
                reason: "channel id must not be empty".to_string(),
            });
        }
        let endpoint = endpoint.into();
        if !is_absolute_http_url(&endpoint) {
            return Err(ImError::InvalidConfig {
                reason: format!("channel {id:?} endpoint must be an absolute http(s) URL"),
            });
        }
        Ok(Self {
            id,
            kind,
            endpoint,
            secret,
        })
    }

    /// 脱敏目标行 (日志红线: 秘密 0 明文)。
    pub fn redacted_summary(&self) -> String {
        format!(
            "im channel \"{}\": kind={} endpoint={} secret={}",
            self.id,
            self.kind.as_str(),
            redact_endpoint(&self.endpoint),
            if self.secret.is_some() {
                "configured"
            } else {
                "none"
            }
        )
    }
}

/// 是否绝对 http(s) URL (无 scheme 相对路径一律拒绝)。
pub fn is_absolute_http_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    match trimmed.split_once("://") {
        Some((scheme, rest)) => {
            matches!(scheme, "http" | "https") && !rest.is_empty() && !rest.starts_with('/')
        }
        None => false,
    }
}

/// 端点脱敏: 保留 scheme/host/path, 秘密类 query 参数值换 `[redacted]`。
fn redact_endpoint(raw: &str) -> String {
    let (base, query) = match raw.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (raw, None),
    };
    match query {
        None => base.to_string(),
        Some(query) => {
            let pairs: Vec<String> = query
                .split('&')
                .map(|pair| match pair.split_once('=') {
                    Some((key, _)) if is_secret_key(key) => format!("{key}=[redacted]"),
                    _ => pair.to_string(),
                })
                .collect();
            format!("{base}?{}", pairs.join("&"))
        }
    }
}

/// 键名是否秘密类 (token / secret / key / sign / password / credential)。
fn is_secret_key(key: &str) -> bool {
    let lowered = key.trim_start_matches('-').to_ascii_lowercase();
    const FRAGMENTS: &[&str] = &[
        "token",
        "secret",
        "key",
        "sign",
        "password",
        "passwd",
        "credential",
        "auth",
    ];
    FRAGMENTS.iter().any(|fragment| lowered.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_wire_values_round_trip_and_unknown_is_rejected() {
        assert_eq!(ImChannelKind::COUNT, 3);
        for kind in ImChannelKind::ALL {
            assert_eq!(ImChannelKind::parse(kind.as_str()), Some(kind));
            assert_eq!(kind.to_string(), kind.as_str());
            let parsed: ImChannelKind = kind.as_str().parse().unwrap();
            assert_eq!(parsed, kind);
        }
        assert_eq!(ImChannelKind::parse("im-slack"), None);
        assert_eq!(ImChannelKind::parse(""), None);
        assert!(matches!(
            "im-slack".parse::<ImChannelKind>(),
            Err(ImError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn target_rejects_empty_id_and_relative_endpoint() {
        let ok = ImChannelTarget::new(
            "primary",
            ImChannelKind::ImFeishu,
            "https://open.example.test/hook",
            Some(ImSecret::new("s3cret").unwrap()),
        )
        .unwrap();
        assert!(ok.redacted_summary().contains("im-feishu"));

        assert!(matches!(
            ImChannelTarget::new("", ImChannelKind::ImQq, "https://x.test/h", None),
            Err(ImError::InvalidConfig { .. })
        ));
        assert!(matches!(
            ImChannelTarget::new("c", ImChannelKind::ImQq, "/hook/only", None),
            Err(ImError::InvalidConfig { .. })
        ));
        assert!(matches!(
            ImChannelTarget::new("c", ImChannelKind::ImQq, "ftp://x.test/h", None),
            Err(ImError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn redacted_summary_never_leaks_the_secret() {
        let target = ImChannelTarget::new(
            "primary",
            ImChannelKind::ImWecom,
            "https://open.example.test/hook?key=super-secret&region=eu",
            Some(ImSecret::new("shared-super-secret").unwrap()),
        )
        .unwrap();
        let line = target.redacted_summary();
        assert!(!line.contains("super-secret"), "{line}");
        assert!(line.contains("region=eu"), "{line}");
        assert!(line.contains("secret=configured"), "{line}");
    }

    #[test]
    fn kind_message_budgets_are_bounded() {
        for kind in ImChannelKind::ALL {
            assert!(kind.max_text_bytes() > 0);
            assert!(kind.max_text_bytes() <= crate::lark::MAX_MESSAGE_TEXT_BYTES);
        }
    }
}
