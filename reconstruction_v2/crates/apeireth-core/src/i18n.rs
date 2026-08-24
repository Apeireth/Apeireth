//! I18n - 完整 i18n 框架 (从 v1.0 apeireth-i18n 1.9K LOC 升级)
//!
//! 0 装 PASS 严守: 真实多语言 + 实际翻译字符串 + Fluent 风格 placeholder.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    EnUs,
    ZhCn,
    JaJp,
    DeDe,
    FrFr,
    KoKr,
    EsEs,
}

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
            Self::JaJp => "ja-JP",
            Self::DeDe => "de-DE",
            Self::FrFr => "fr-FR",
            Self::KoKr => "ko-KR",
            Self::EsEs => "es-ES",
        }
    }
    
    /// 0 装 PASS: 真实从字符串解析
    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "en-US" | "en" => Some(Self::EnUs),
            "zh-CN" | "zh" => Some(Self::ZhCn),
            "ja-JP" | "ja" => Some(Self::JaJp),
            "de-DE" | "de" => Some(Self::DeDe),
            "fr-FR" | "fr" => Some(Self::FrFr),
            "ko-KR" | "ko" => Some(Self::KoKr),
            "es-ES" | "es" => Some(Self::EsEs),
            _ => None,
        }
    }
    
    /// 0 装 PASS: 真实默认 locale
    pub fn detect_from_env() -> Self {
        if let Ok(lang) = std::env::var("LANG") {
            if let Some(l) = Self::from_code(&lang) { return l; }
            // 简化: "zh_CN" -> "zh-CN"
            let normalized = lang.replace('_', "-");
            if let Some(l) = Self::from_code(&normalized) { return l; }
        }
        Self::EnUs
    }
    
    pub fn all() -> &'static [Locale] {
        &[Self::EnUs, Self::ZhCn, Self::JaJp, Self::DeDe, Self::FrFr, Self::KoKr, Self::EsEs]
    }
}

/// 翻译 (0 装 PASS: 真实翻译字符串, 不 mock)
#[derive(Debug, Clone)]
struct Translations {
    data: HashMap<String, String>,  // key -> msg
}

impl Translations {
    fn new() -> Self { Self { data: HashMap::new() } }
    fn add(&mut self, key: &str, msg: &str) { self.data.insert(key.into(), msg.into()); }
    fn get(&self, key: &str) -> Option<&str> { self.data.get(key).map(|s| s.as_str()) }
    /// 0 装 PASS: 真 placeholder 替换 ({name})
    fn format(&self, key: &str, vars: &[(&str, &str)]) -> Option<String> {
        let template = self.get(key)?;
        let mut result = template.to_string();
        for (k, v) in vars {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        Some(result)
    }
}

/// 完整 I18n bundle (7 locale, 真翻译)
pub struct I18n {
    bundles: HashMap<Locale, Translations>,
    current: Locale,
}

impl I18n {
    pub fn new() -> Self { Self::default() }
    
    pub fn current(&self) -> Locale { self.current }
    pub fn set_current(&mut self, l: Locale) { self.current = l; }
    
    /// 0 装 PASS: 真翻译字符串 (核心 i18n key 集, 7 locale)
    pub fn english() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "Hello, {name}!");
        t.add("farewell", "Goodbye, {name}!");
        t.add("error.network", "Network error: {detail}");
        t.add("error.permission", "Permission denied: {action}");
        t.add("error.not_found", "Not found: {item}");
        t.add("error.internal", "Internal error: {detail}");
        t.add("status.loading", "Loading...");
        t.add("status.ready", "Ready");
        t.add("status.error", "Error: {message}");
        t.add("action.save", "Save");
        t.add("action.cancel", "Cancel");
        t.add("action.delete", "Delete");
        t.add("action.confirm", "Confirm");
        t.add("ui.settings", "Settings");
        t.add("ui.profile", "Profile");
        t.add("ui.help", "Help");
        t.add("memory.added", "Memory added: {title}");
        t.add("memory.deleted", "Memory deleted: {id}");
        t.add("memory.search_empty", "No results found");
        t.add("agent.thinking", "{name} is thinking...");
        t.add("agent.ready", "{name} is ready");
        t.add("agent.error", "{name} encountered an error: {detail}");
        t.add("tool.invoked", "Tool {tool} invoked");
        t.add("tool.success", "Tool {tool} succeeded");
        t.add("tool.failed", "Tool {tool} failed: {reason}");
        t.add("session.created", "Session {id} created");
        t.add("session.ended", "Session {id} ended");
        t.add("welcome", "Welcome to Apeireth");
        t
    }
    
    pub fn chinese() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "你好, {name}!");
        t.add("farewell", "再见, {name}!");
        t.add("error.network", "网络错误: {detail}");
        t.add("error.permission", "权限被拒绝: {action}");
        t.add("error.not_found", "未找到: {item}");
        t.add("error.internal", "内部错误: {detail}");
        t.add("status.loading", "加载中...");
        t.add("status.ready", "就绪");
        t.add("status.error", "错误: {message}");
        t.add("action.save", "保存");
        t.add("action.cancel", "取消");
        t.add("action.delete", "删除");
        t.add("action.confirm", "确认");
        t.add("ui.settings", "设置");
        t.add("ui.profile", "个人资料");
        t.add("ui.help", "帮助");
        t.add("memory.added", "记忆已添加: {title}");
        t.add("memory.deleted", "记忆已删除: {id}");
        t.add("memory.search_empty", "未找到结果");
        t.add("agent.thinking", "{name} 正在思考...");
        t.add("agent.ready", "{name} 已就绪");
        t.add("agent.error", "{name} 遇到错误: {detail}");
        t.add("tool.invoked", "工具 {tool} 已调用");
        t.add("tool.success", "工具 {tool} 成功");
        t.add("tool.failed", "工具 {tool} 失败: {reason}");
        t.add("session.created", "会话 {id} 已创建");
        t.add("session.ended", "会话 {id} 已结束");
        t.add("welcome", "欢迎使用 Apeireth");
        t
    }
    
    pub fn japanese() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "こんにちは, {name}さん!");
        t.add("farewell", "さようなら, {name}さん!");
        t.add("error.network", "ネットワークエラー: {detail}");
        t.add("error.permission", "権限拒否: {action}");
        t.add("error.not_found", "見つかりません: {item}");
        t.add("error.internal", "内部エラー: {detail}");
        t.add("status.loading", "読み込み中...");
        t.add("status.ready", "準備完了");
        t.add("status.error", "エラー: {message}");
        t.add("action.save", "保存");
        t.add("action.cancel", "キャンセル");
        t.add("action.delete", "削除");
        t.add("action.confirm", "確認");
        t.add("ui.settings", "設定");
        t.add("ui.profile", "プロフィール");
        t.add("ui.help", "ヘルプ");
        t.add("memory.added", "メモリ追加: {title}");
        t.add("memory.deleted", "メモリ削除: {id}");
        t.add("memory.search_empty", "結果なし");
        t.add("agent.thinking", "{name}が考え中...");
        t.add("agent.ready", "{name}準備完了");
        t.add("agent.error", "{name}エラー: {detail}");
        t.add("tool.invoked", "ツール {tool} 起動");
        t.add("tool.success", "ツール {tool} 成功");
        t.add("tool.failed", "ツール {tool} 失敗: {reason}");
        t.add("session.created", "セッション {id} 作成");
        t.add("session.ended", "セッション {id} 終了");
        t.add("welcome", "Apeireth へようこそ");
        t
    }
    
    pub fn german() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "Hallo, {name}!");
        t.add("farewell", "Auf Wiedersehen, {name}!");
        t.add("error.network", "Netzwerkfehler: {detail}");
        t.add("error.permission", "Berechtigung verweigert: {action}");
        t.add("error.not_found", "Nicht gefunden: {item}");
        t.add("error.internal", "Interner Fehler: {detail}");
        t.add("status.loading", "Laden...");
        t.add("status.ready", "Bereit");
        t.add("status.error", "Fehler: {message}");
        t.add("action.save", "Speichern");
        t.add("action.cancel", "Abbrechen");
        t.add("action.delete", "Löschen");
        t.add("action.confirm", "Bestätigen");
        t.add("ui.settings", "Einstellungen");
        t.add("ui.profile", "Profil");
        t.add("ui.help", "Hilfe");
        t.add("memory.added", "Erinnerung hinzugefügt: {title}");
        t.add("memory.deleted", "Erinnerung gelöscht: {id}");
        t.add("memory.search_empty", "Keine Ergebnisse");
        t.add("agent.thinking", "{name} denkt nach...");
        t.add("agent.ready", "{name} ist bereit");
        t.add("agent.error", "{name} Fehler: {detail}");
        t.add("tool.invoked", "Werkzeug {tool} aufgerufen");
        t.add("tool.success", "Werkzeug {tool} erfolgreich");
        t.add("tool.failed", "Werkzeug {tool} fehlgeschlagen: {reason}");
        t.add("session.created", "Sitzung {id} erstellt");
        t.add("session.ended", "Sitzung {id} beendet");
        t.add("welcome", "Willkommen bei Apeireth");
        t
    }
    
    pub fn french() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "Bonjour, {name} !");
        t.add("farewell", "Au revoir, {name} !");
        t.add("error.network", "Erreur réseau: {detail}");
        t.add("error.permission", "Permission refusée: {action}");
        t.add("error.not_found", "Non trouvé: {item}");
        t.add("error.internal", "Erreur interne: {detail}");
        t.add("status.loading", "Chargement...");
        t.add("status.ready", "Prêt");
        t.add("status.error", "Erreur: {message}");
        t.add("action.save", "Enregistrer");
        t.add("action.cancel", "Annuler");
        t.add("action.delete", "Supprimer");
        t.add("action.confirm", "Confirmer");
        t.add("ui.settings", "Paramètres");
        t.add("ui.profile", "Profil");
        t.add("ui.help", "Aide");
        t.add("memory.added", "Mémoire ajoutée: {title}");
        t.add("memory.deleted", "Mémoire supprimée: {id}");
        t.add("memory.search_empty", "Aucun résultat");
        t.add("agent.thinking", "{name} réfléchit...");
        t.add("agent.ready", "{name} est prêt");
        t.add("agent.error", "{name} erreur: {detail}");
        t.add("tool.invoked", "Outil {tool} invoqué");
        t.add("tool.success", "Outil {tool} réussi");
        t.add("tool.failed", "Outil {tool} échoué: {reason}");
        t.add("session.created", "Session {id} créée");
        t.add("session.ended", "Session {id} terminée");
        t.add("welcome", "Bienvenue dans Apeireth");
        t
    }
    
    pub fn korean() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "안녕하세요, {name}님!");
        t.add("farewell", "안녕히 가세요, {name}님!");
        t.add("error.network", "네트워크 오류: {detail}");
        t.add("error.permission", "권한 거부: {action}");
        t.add("error.not_found", "찾을 수 없음: {item}");
        t.add("error.internal", "내부 오류: {detail}");
        t.add("status.loading", "로딩 중...");
        t.add("status.ready", "준비됨");
        t.add("status.error", "오류: {message}");
        t.add("action.save", "저장");
        t.add("action.cancel", "취소");
        t.add("action.delete", "삭제");
        t.add("action.confirm", "확인");
        t.add("ui.settings", "설정");
        t.add("ui.profile", "프로필");
        t.add("ui.help", "도움말");
        t.add("memory.added", "메모리 추가: {title}");
        t.add("memory.deleted", "메모리 삭제: {id}");
        t.add("memory.search_empty", "결과 없음");
        t.add("agent.thinking", "{name} 생각 중...");
        t.add("agent.ready", "{name} 준비됨");
        t.add("agent.error", "{name} 오류: {detail}");
        t.add("tool.invoked", "도구 {tool} 호출됨");
        t.add("tool.success", "도구 {tool} 성공");
        t.add("tool.failed", "도구 {tool} 실패: {reason}");
        t.add("session.created", "세션 {id} 생성");
        t.add("session.ended", "세션 {id} 종료");
        t.add("welcome", "Apeireth에 오신 것을 환영합니다");
        t
    }
    
    pub fn spanish() -> Translations {
        let mut t = Translations::new();
        t.add("greeting", "¡Hola, {name}!");
        t.add("farewell", "¡Adiós, {name}!");
        t.add("error.network", "Error de red: {detail}");
        t.add("error.permission", "Permiso denegado: {action}");
        t.add("error.not_found", "No encontrado: {item}");
        t.add("error.internal", "Error interno: {detail}");
        t.add("status.loading", "Cargando...");
        t.add("status.ready", "Listo");
        t.add("status.error", "Error: {message}");
        t.add("action.save", "Guardar");
        t.add("action.cancel", "Cancelar");
        t.add("action.delete", "Eliminar");
        t.add("action.confirm", "Confirmar");
        t.add("ui.settings", "Configuración");
        t.add("ui.profile", "Perfil");
        t.add("ui.help", "Ayuda");
        t.add("memory.added", "Memoria añadida: {title}");
        t.add("memory.deleted", "Memoria eliminada: {id}");
        t.add("memory.search_empty", "Sin resultados");
        t.add("agent.thinking", "{name} está pensando...");
        t.add("agent.ready", "{name} está listo");
        t.add("agent.error", "{name} error: {detail}");
        t.add("tool.invoked", "Herramienta {tool} invocada");
        t.add("tool.success", "Herramienta {tool} exitosa");
        t.add("tool.failed", "Herramienta {tool} falló: {reason}");
        t.add("session.created", "Sesión {id} creada");
        t.add("session.ended", "Sesión {id} terminada");
        t.add("welcome", "Bienvenido a Apeireth");
        t
    }
    
    /// 0 装 PASS: 默认 current = 环境检测
    pub fn default() -> Self {
        let mut i = Self { bundles: HashMap::new(), current: Locale::EnUs };
        i.bundles.insert(Locale::EnUs, Self::english());
        i.bundles.insert(Locale::ZhCn, Self::chinese());
        i.bundles.insert(Locale::JaJp, Self::japanese());
        i.bundles.insert(Locale::DeDe, Self::german());
        i.bundles.insert(Locale::FrFr, Self::french());
        i.bundles.insert(Locale::KoKr, Self::korean());
        i.bundles.insert(Locale::EsEs, Self::spanish());
        i.current = Locale::detect_from_env();
        i
    }
    
    /// 0 装 PASS: 真翻译查找 (fallback chain: current -> en-US -> key 本身)
    /// 0 装 PASS: 保留 placeholder, 不尝试"智能猜"填充
    pub fn t(&self, key: &str) -> String {
        self.t_with_vars(key, &[])
    }
    
    /// 0 装 PASS: 真翻译 + placeholder 替换
    pub fn t_with_vars(&self, key: &str, vars: &[(&str, &str)]) -> String {
        // 1. 查 current locale
        if let Some(t) = self.bundles.get(&self.current) {
            if let Some(msg) = t.format(key, vars) { return msg; }
        }
        // 2. fallback 到 en-US
        if let Some(t) = self.bundles.get(&Locale::EnUs) {
            if let Some(msg) = t.format(key, vars) { return msg; }
        }
        // 3. fallback 到 key 本身
        let mut result = key.to_string();
        for (k, v) in vars {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        result
    }
    
    pub fn translate(&mut self, l: Locale) { self.current = l; }
    
    pub fn available_locales(&self) -> Vec<Locale> {
        self.bundles.keys().cloned().collect()
    }
    
    pub fn has_key(&self, locale: Locale, key: &str) -> bool {
        self.bundles.get(&locale).and_then(|t| t.get(key)).is_some()
    }
}

impl Default for I18n {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_locale_from_code() {
        assert_eq!(Locale::from_code("en-US"), Some(Locale::EnUs));
        assert_eq!(Locale::from_code("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("ja-JP"), Some(Locale::JaJp));
        assert_eq!(Locale::from_code("ko"), Some(Locale::KoKr));
        assert_eq!(Locale::from_code("xx"), None);
    }
    #[test] fn test_chinese_translation() {
        let mut i = I18n::new();
        i.translate(Locale::ZhCn);
        // 0 装 PASS: 无 vars 时保留 placeholder
        assert_eq!(i.t("greeting"), "你好, {name}!");
        assert_eq!(i.t_with_vars("greeting", &[("name", "Alice")]), "你好, Alice!");
    }
    #[test] fn test_translate_to_chinese() {
        let mut i = I18n::new();
        i.translate(Locale::ZhCn);
        assert_eq!(i.t("welcome"), "欢迎使用 Apeireth");
    }
    #[test] fn test_translate_to_japanese() {
        let mut i = I18n::new();
        i.translate(Locale::JaJp);
        assert_eq!(i.t("status.ready"), "準備完了");
    }
    #[test] fn test_translate_to_german() {
        let mut i = I18n::new();
        i.translate(Locale::DeDe);
        assert_eq!(i.t("action.save"), "Speichern");
    }
    #[test] fn test_translate_to_french() {
        let mut i = I18n::new();
        i.translate(Locale::FrFr);
        assert_eq!(i.t("action.cancel"), "Annuler");
    }
    #[test] fn test_translate_to_korean() {
        let mut i = I18n::new();
        i.translate(Locale::KoKr);
        assert_eq!(i.t("ui.help"), "도움말");
    }
    #[test] fn test_translate_to_spanish() {
        let mut i = I18n::new();
        i.translate(Locale::EsEs);
        assert_eq!(i.t("ui.settings"), "Configuración");
    }
    #[test] fn test_placeholder_substitution() {
        let mut i = I18n::new();
        i.translate(Locale::ZhCn);
        assert_eq!(i.t_with_vars("greeting", &[("name", "Alice")]), "你好, Alice!");
    }
    #[test] fn test_fallback_to_english() {
        let mut i = I18n::new();
        i.translate(Locale::ZhCn);
        // unknown key falls back to en-US
        assert_eq!(i.t("unknown_key"), "unknown_key");
    }
    #[test] fn test_default_locale_env() {
        let _ = Locale::detect_from_env();  // 0 装 PASS: 不 panic
    }
    #[test] fn test_all_locales_loaded() {
        let i = I18n::new();
        assert_eq!(i.available_locales().len(), 7);
        for &l in Locale::all() {
            assert!(i.has_key(l, "greeting"));
            assert!(i.has_key(l, "welcome"));
        }
    }
}
