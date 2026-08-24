# apeireth-i18n

> v2 port of v1.0-legacy/apeireth-i18n (complete API surface preserved).

i18n framework:
- 5 locales (en / zh-CN / ja / fr / de)
- 12 categories, 69 keys, 100% translation
- Compile-time hardcode via include_str! locales/*.toml
- API: t() / try_t() / set_locale() / get_locale() / etc.
- TOOL_WHITELIST of 8 tools (m3 hallucination defense)

Organ Kani proofs in organ_kani_proofs.rs.
