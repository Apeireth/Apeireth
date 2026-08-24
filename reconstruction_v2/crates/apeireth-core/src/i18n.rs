//! I18n - 国际化 wrapper (从 v1.0 apeireth-i18n 1.9K LOC 收敛)
//!
//! 0 装 PASS: 简化 MessageBundle (内存 HashMap per locale), 完整 v1.0 era 不做 (.po/.mo 文件加载).

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    EnUs,
    ZhCn,
    JaJp,
    DeDe,
    FrFr,
}

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
            Self::JaJp => "ja-JP",
            Self::DeDe => "de-DE",
            Self::FrFr => "fr-FR",
        }
    }
}

/// MessageBundle - 0 装 PASS 真实存储 (HashMap<String, String>), 不是 i18n 真框架
#[derive(Default)]
pub struct MessageBundle {
    messages: HashMap<String, HashMap<Locale, String>>,
}

impl MessageBundle {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, key: impl Into<String>, locale: Locale, msg: impl Into<String>) {
        self.messages.entry(key.into()).or_default().insert(locale, msg.into());
    }
    /// 0 装 PASS: 找不到返 key (不假装)
    pub fn get(&self, key: &str, locale: Locale) -> Option<&str> {
        self.messages.get(key).and_then(|m| m.get(&locale)).map(|s| s.as_str())
    }
}

/// I18n - 全局 locale + bundle
pub struct I18n {
    current: Arc<RwLock<Locale>>,
    bundle: Arc<RwLock<MessageBundle>>,
}

impl I18n {
    pub fn new(initial: Locale) -> Self {
        Self { current: Arc::new(RwLock::new(initial)), bundle: Arc::new(RwLock::new(MessageBundle::new())) }
    }
    pub async fn set_locale(&self, l: Locale) { *self.current.write().await = l; }
    pub async fn locale(&self) -> Locale { *self.current.read().await }
    pub async fn add(&self, key: impl Into<String>, locale: Locale, msg: impl Into<String>) {
        self.bundle.write().await.add(key, locale, msg);
    }
    /// 0 装 PASS: 找不到返 key 本身
    pub async fn t(&self, key: &str) -> String {
        let bundle = self.bundle.read().await;
        let locale = *self.current.read().await;
        bundle.get(key, locale).unwrap_or(key).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_bundle_get_set() {
        let mut b = MessageBundle::new();
        b.add("hello", Locale::EnUs, "Hello");
        b.add("hello", Locale::ZhCn, "你好");
        assert_eq!(b.get("hello", Locale::EnUs), Some("Hello"));
        assert_eq!(b.get("hello", Locale::ZhCn), Some("你好"));
        assert_eq!(b.get("missing", Locale::EnUs), None);
    }
    #[tokio::test]
    async fn test_i18n_t() {
        let i = I18n::new(Locale::EnUs);
        i.add("greet", Locale::EnUs, "Hi").await;
        i.add("greet", Locale::ZhCn, "你好").await;
        assert_eq!(i.t("greet").await, "Hi");
        i.set_locale(Locale::ZhCn).await;
        assert_eq!(i.t("greet").await, "你好");
        // missing key 返 key 本身
        assert_eq!(i.t("missing").await, "missing");
    }
}
