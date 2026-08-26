//! R177 i18n organ Kani proofs (W6)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_i18_01_locale_count() {
    assert_eq!(SUPPORTED_LOCALES.len(), 5);
}

#[test]
fn r177_i18_02_default_locale() {
    assert_eq!(DEFAULT_LOCALE, Locale::En);
}

#[test]
fn r177_i18_03_schema_version() {
    assert_eq!(I18N_SCHEMA_VERSION, "1");
}

#[test]
fn r177_i18_04_platform_name() {
    assert_eq!(PLATFORM_NAME, "apeireth");
}

#[test]
fn r177_i18_05_expected_keys() {
    assert_eq!(EXPECTED_KEY_COUNT, 69);
}

#[cfg(kani)]
#[kani::proof]
fn r177_i18_kani_01_max_key_len() {
    assert!(MAX_KEY_LENGTH >= 1);
}

#[cfg(kani)]
#[kani::proof]
fn r177_i18_kani_02_max_nesting() {
    assert!(MAX_NESTING_DEPTH >= 1);
}
