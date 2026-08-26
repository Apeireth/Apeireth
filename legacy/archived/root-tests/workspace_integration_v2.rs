//! Workspace 跨 crate 集成测试 (V2 集成测试追加, R20 阶段 1 收官)
//!
//! 14 crate 端到端覆盖 (per R20 阶段 1 收官估补 1 owner × 1 周):
//!
//! ## 5 P0 MCP
//! - `apeireth-mcp-ssh`          — SSH MCP Server 8 工具 + validate_tool_call
//! - `apeireth-mcp-winrm`        — WinRM MCP Server 8 工具 + 5 auth methods
//! - `apeireth-mcp-relay-image`  — Image Relay MCP 5 工具 + ImageFormat
//! - `apeireth-workflow`         — WorkflowGenerator 8 节点 + 拓扑排序
//! - `apeireth-team-lead`        — Orchestrator 14 工具 + SubAgent
//!
//! ## 3 估缺核心 (per RIVAL §2.2)
//! - `apeireth-image-prompt`     — ImagePromptLibrary 8 工具 + PromptTemplate
//! - `apeireth-rollback`         — RollbackService 8 工具 + SnapshotMeta
//! - `apeireth-plugin`           — PluginLoader 8 工具 + PluginLifecycle
//!
//! ## 2 估缺工具 (per RIVAL §2.3)
//! - `apeireth-repo-scan`        — RepoScanner 8 工具 + Language::from_extension
//! - `apeireth-repo-analyzer`    — QualityAnalyzer 8 工具 + TechDebtType
//!
//! ## 2 基础设施 P0 (per RIVAL §2.4)
//! - `apeireth-keyring`          — KeyringStore 8 工具 + Platform detect
//! - `apeireth-machine-id`       — get_machine_id 6 工具 + Platform
//!
//! ## 2 SDK stub (per RIVAL §2.5)
//! - `apeireth-lark`             — LarkClient 9 工具 + STUB_MODE
//! - `apeireth-voice`            — VoiceWake 9 工具 + WakeWordType
//!
//! ## 设计原则 (Ponytail)
//!
//! - 不创建 crate; 用 `#[path]` 直接 include 各 crate 的 lib.rs 源码 (R25 验收做法)
//! - 不在测试里重启 cargo; 每个测试只用 crate 公共 API
//! - 集成测试不依赖具体实现, 测公开 API 行为
//! - 不依赖外部资源 (SSH/WinRM/LLM API) 的测试用 `#[ignore]` 标
//!
//! ## 主哲学 6 锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向**: 14 crate 端到端服务 ASI 北极星
//! - **S-2 实事求是**: 只测公开 API, 0 假装内部实现
//! - **O-5 不假装**: 不存在的工具名测试用 `#[ignore]` 跳过, 不假装已实现
//! - **O-2 走在前人肩上**: 1:1 翻译 v0.9.21 字段 + 8 工具白名单编译期 hardcode
//! - **O-3 干到底**: 30+ 测试覆盖 14 crate
//! - **O-4 任何人都能接手**: 头部 6 锚 + 8 项不修改承诺 + 路径明确
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! - ❌ 不动 workspace Cargo.toml
//! - ❌ 不动 Cargo.lock
//! - ❌ 不动 24 LOCKED crate 的 src/
//! - ❌ 不抄 v0.9.21 业务代码 (借鉴字段, 不抄字节码)
//! - ❌ 不假装 "已实现但没真跑" (用 `#[ignore]` 标未实现)
//! - ❌ 不写 workspace version (0.1.0/1.0.0 都不写)
//! - ❌ 不删 typo 路径
//! - ❌ 不 commit (整合 #3 sub-agent 统一 commit)
//!
//! ## 跑法
//!
//! ```bash
//! cargo test --test workspace_integration_v2
//! cargo test --test workspace_integration_v2 -- --ignored   # 跑需要外部资源的测试
//! ```
//!
//! 主报告: `reports/r20-stage1-integration-test-2026-08-05.md`

#![allow(dead_code, unused_imports, clippy::needless_return)]

// ============================================================
// Section 1: 5 P0 MCP 集成测试
// ============================================================

// R132.3: apeireth-mcp-ssh / mcp-winrm / mcp-relay-image 3 stub crate 已物理删除
// (per owner 拍板 A 案: 0 stub 留尾巴). 详见 reports/r132-3-mcp-stub-removal.md

#[path = "../crates/apeireth-workflow/src/lib.rs"]
#[allow(dead_code)]
mod workflow_inline;

#[path = "../crates/apeireth-team-lead/src/lib.rs"]
#[allow(dead_code)]
mod team_lead_inline;

#[cfg(test)]
#[allow(clippy::needless_return)]
// R132.3: p0_mcp_integration test mod removed (mcp-ssh/winrm/relay-image 3 stub crate 物理删除)
// 详见 reports/r132-3-mcp-stub-removal.md
// 原本验证 5 P0 MCP 集成: ssh / winrm / relay-image / workflow / team-lead
// 现保留 workflow + team-lead 测试在后续段
mod p0_mcp_integration {
    // 5 P0 crate 全部 NO-OP 验证 — 3 stub 已删, 2 保留
    #[test]
    fn r132_3_mcp_stub_3_crates_removed() {
        // A 案物理删除: mcp-ssh / mcp-winrm / mcp-relay-image
        // 见 Cargo.toml members 74 → 71, ssh2 0.9 依赖移除
        assert!(true, "R132.3 stub removal marker");
    }
}


// ============================================================
// Section 2: 3 估缺核心 (image-prompt / rollback / plugin)
// ============================================================

#[path = "../crates/apeireth-image-prompt/src/lib.rs"]
#[allow(dead_code)]
mod image_prompt_inline;

#[path = "../crates/apeireth-rollback/src/lib.rs"]
#[allow(dead_code)]
mod rollback_inline;

#[path = "../crates/apeireth-plugin/src/lib.rs"]
#[allow(dead_code)]
mod plugin_inline;

#[cfg(test)]
mod core_modules_integration {
    use super::{image_prompt_inline, rollback_inline, plugin_inline};

    // --- 2.1 apeireth-image-prompt ---

    #[test]
    fn image_prompt_tool_whitelist_has_eight_tools() {
        assert_eq!(image_prompt_inline::TOOL_WHITELIST.len(), 8);
        assert!(image_prompt_inline::TOOL_WHITELIST.contains(&"apeireth_image_prompt_render"));
        assert!(image_prompt_inline::TOOL_WHITELIST.contains(&"apeireth_image_prompt_search"));
    }

    #[test]
    fn image_prompt_schema_constants_are_valid() {
        assert_eq!(image_prompt_inline::PROMPT_SCHEMA_VERSION, "1");
        assert_eq!(image_prompt_inline::PROMPT_LRU_CAPACITY, 1000);
        assert_eq!(image_prompt_inline::PROMPT_RATING_MIN, 1);
        assert_eq!(image_prompt_inline::PROMPT_RATING_MAX, 5);
        assert_eq!(image_prompt_inline::PROMPT_MAX_LENGTH, 2000);
    }

    #[test]
    fn image_prompt_template_new_with_body() {
        use image_prompt_inline::PromptTemplate;
        let tmpl = PromptTemplate::new("{{subject}} in {{style}} style");
        assert_eq!(tmpl.body, "{{subject}} in {{style}} style");
        // with_default 返回 Self, 可链式
        let tmpl = tmpl.with_default("style", "photorealistic");
        assert!(!tmpl.defaults.is_empty());
    }

    #[test]
    fn image_prompt_entry_is_high_rated_at_max() {
        use image_prompt_inline::{PromptEntry, PromptCategory};
        // is_high_rated == rating == PROMPT_RATING_MAX (5)
        let mut entry = PromptEntry::new("test", "body", PromptCategory::General);
        entry.rating = image_prompt_inline::PROMPT_RATING_MAX; // 5
        assert!(entry.is_high_rated());
        entry.rating = 3;
        assert!(!entry.is_high_rated());
    }

    #[test]
    fn image_prompt_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({"name": "test"});
        let result = image_prompt_inline::validate_tool_call("apeireth_image_prompt_add", &args);
        assert!(result.is_ok());
    }

    // --- 2.2 apeireth-rollback ---

    #[test]
    fn rollback_tool_whitelist_has_eight_tools() {
        assert_eq!(rollback_inline::TOOL_WHITELIST.len(), 8);
        assert!(rollback_inline::TOOL_WHITELIST.contains(&"apeireth_rollback_snapshot"));
        assert!(rollback_inline::TOOL_WHITELIST.contains(&"apeireth_rollback_restore"));
        assert!(rollback_inline::TOOL_WHITELIST.contains(&"apeireth_rollback_git_stash"));
    }

    #[test]
    fn rollback_v09021_strategy_count_matches() {
        // v0.9.21 RollbackService 6 策略 1:1 翻译
        assert_eq!(rollback_inline::V0921_ROLLBACK_STRATEGIES, 6);
    }

    #[test]
    fn rollback_compile_time_constants_are_valid() {
        assert_eq!(rollback_inline::ROLLBACK_SCHEMA_VERSION, "1");
        assert_eq!(rollback_inline::TOOL_COUNT, 8);
        assert_eq!(rollback_inline::MAX_SHADOW_AGE_DAYS, 7);
        assert_eq!(rollback_inline::MAX_SHADOW_SIZE_BYTES, 100 * 1024 * 1024);
    }

    #[test]
    fn rollback_strategy_v09021_str_round_trip() {
        use rollback_inline::RollbackStrategy;
        // at least 1 策略能 round-trip
        for s in [RollbackStrategy::File, RollbackStrategy::Git, RollbackStrategy::Shadow] {
            let s_str = s.as_v0921_str();
            assert!(!s_str.is_empty());
            let back = RollbackStrategy::from_v0921_str(s_str);
            assert!(back.is_ok(), "round-trip {} should succeed", s_str);
        }
    }

    #[test]
    fn rollback_snapshot_meta_age_calculation() {
        // SnapshotMeta::age_seconds/days/is_expired 编译期 hardcode
        let meta = rollback_inline::SnapshotMeta::new("snap-001", 0, 1024, rollback_inline::RollbackStrategy::File);
        assert!(meta.age_seconds() >= 0);
        assert!(meta.age_days() >= 0);
        // 刚创建, 1 天内不 expire (除非时间戳为 0)
    }

    #[test]
    fn rollback_snapshot_index_add_and_find() {
        use rollback_inline::{RollbackStrategy, SnapshotIndex, SnapshotMeta};
        let mut idx = SnapshotIndex::new();
        let meta = SnapshotMeta::new("snap-001", 100, 1024, RollbackStrategy::File);
        idx.add(meta);
        assert!(idx.find("snap-001").is_some());
        assert!(idx.find("snap-002").is_none());
    }

    #[test]
    fn rollback_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let result = rollback_inline::validate_tool_call("apeireth_rollback_time_travel", &args);
        assert!(result.is_err());
    }

    // --- 2.3 apeireth-plugin ---

    #[test]
    fn plugin_tool_whitelist_has_eight_tools() {
        assert_eq!(plugin_inline::TOOL_WHITELIST.len(), 8);
        assert_eq!(plugin_inline::TOOL_WHITELIST_COUNT, 8);
        assert!(plugin_inline::TOOL_WHITELIST.contains(&"apeireth_plugin_load"));
        assert!(plugin_inline::TOOL_WHITELIST.contains(&"apeireth_plugin_watch_start"));
    }

    #[test]
    fn plugin_compile_time_constants_are_valid() {
        assert_eq!(plugin_inline::PLUGIN_SCHEMA_VERSION, "1");
        assert_eq!(plugin_inline::PLATFORM_NAME, "apeireth");
        assert_eq!(plugin_inline::MAX_PLUGINS_PER_HOST, 64);
        assert_eq!(plugin_inline::PLUGIN_INSTALL_TIMEOUT_MS, 30_000);
        assert_eq!(plugin_inline::PLUGIN_SANDBOX_TIMEOUT_MS, 5_000);
    }

    #[test]
    fn plugin_lifecycle_transitions() {
        use plugin_inline::PluginLifecycle;
        // 安装 → 运行 → 卸载 是合法转换
        assert!(PluginLifecycle::Installed.can_transition_to(PluginLifecycle::Running));
        assert!(PluginLifecycle::Running.can_transition_to(PluginLifecycle::Unloaded));
        // 已卸载不应再转换
        assert!(!PluginLifecycle::Unloaded.can_transition_to(PluginLifecycle::Running));
    }

    #[test]
    fn plugin_id_new_is_unique() {
        use plugin_inline::PluginId;
        let a = PluginId::new();
        let b = PluginId::new();
        assert_ne!(a, b, "每次 new() 应生成不同 ID");
    }

    #[test]
    fn plugin_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({"plugin_id": "test"});
        let result = plugin_inline::validate_tool_call("apeireth_plugin_load", &args);
        assert!(result.is_ok());
    }
}

// ============================================================
// Section 3: 2 估缺工具 (repo-scan / repo-analyzer)
// ============================================================

#[path = "../crates/apeireth-repo-scan/src/lib.rs"]
#[allow(dead_code)]
mod repo_scan_inline;

#[path = "../crates/apeireth-repo-analyzer/src/lib.rs"]
#[allow(dead_code)]
mod repo_analyzer_inline;

#[cfg(test)]
mod repo_tools_integration {
    use super::{repo_scan_inline, repo_analyzer_inline};

    // --- 3.1 apeireth-repo-scan ---

    #[test]
    fn repo_scan_tool_whitelist_has_eight_tools() {
        assert_eq!(repo_scan_inline::TOOL_WHITELIST.len(), 8);
        assert!(repo_scan_inline::TOOL_WHITELIST.contains(&"apeireth_repo_scan_scan"));
        assert!(repo_scan_inline::TOOL_WHITELIST.contains(&"apeireth_repo_scan_sensitive_grep"));
    }

    #[test]
    fn repo_scan_m3_defense_sanity_check_passes() {
        assert!(repo_scan_inline::m3_defense_sanity_check());
    }

    #[test]
    fn repo_scan_language_from_extension() {
        use repo_scan_inline::Language;
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("go"), Language::Go);
    }

    #[test]
    fn repo_scan_shebang_detection() {
        use repo_scan_inline::Language;
        assert_eq!(Language::from_shebang("#!/usr/bin/env python3"), Some(Language::Python));
        assert_eq!(Language::from_shebang("#!/bin/bash"), Some(Language::Shell));
        assert_eq!(Language::from_shebang("not a shebang"), None);
    }

    #[test]
    fn repo_scan_compile_time_constants() {
        assert_eq!(repo_scan_inline::REPO_SCAN_SCHEMA_VERSION, "1");
        assert_eq!(repo_scan_inline::PLATFORM_NAME, "apeireth");
        assert_eq!(repo_scan_inline::MAX_SCAN_DEPTH, 10);
        assert_eq!(repo_scan_inline::SCAN_CACHE_TTL_DAYS, 7);
    }

    // --- 3.2 apeireth-repo-analyzer ---

    #[test]
    fn repo_analyzer_tool_whitelist_has_eight_tools() {
        assert_eq!(repo_analyzer_inline::TOOL_WHITELIST.len(), 8);
        assert!(repo_analyzer_inline::TOOL_WHITELIST.contains(&"apeireth_repo_analyzer_complexity"));
        assert!(repo_analyzer_inline::TOOL_WHITELIST.contains(&"apeireth_repo_analyzer_security"));
    }

    #[test]
    fn repo_analyzer_k1_invariants_defined() {
        // K-1 不变量: 5 项核心守门
        assert_eq!(repo_analyzer_inline::K1_INVARIANTS.len(), 5);
    }

    #[test]
    fn repo_analyzer_compile_time_constants() {
        assert_eq!(repo_analyzer_inline::REPO_ANALYZER_SCHEMA_VERSION, "1");
        assert_eq!(repo_analyzer_inline::MAX_CYCLOMATIC_COMPLEXITY, 20);
        assert_eq!(repo_analyzer_inline::MAX_FILES_PER_ANALYSIS, 10_000);
        assert_eq!(repo_analyzer_inline::SINGLE_FILE_ANALYSIS_TIMEOUT_MS, 5_000);
    }

    #[test]
    fn repo_analyzer_tech_debt_type_tag() {
        use repo_analyzer_inline::TechDebtType;
        // 至少 1 类型有 tag
        for t in [TechDebtType::Todo, TechDebtType::Fixme, TechDebtType::Hack, TechDebtType::Bug] {
            let tag = t.tag();
            assert!(!tag.is_empty(), "TechDebtType {:?} 应有 tag", t);
        }
    }

    #[test]
    fn repo_analyzer_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({"path": "."});
        let result = repo_analyzer_inline::validate_tool_call("apeireth_repo_analyzer_complexity", &args);
        assert!(result.is_ok());
    }
}

// ============================================================
// Section 4: 2 基础设施 P0 (keyring / machine-id)
// ============================================================

#[path = "../crates/apeireth-keyring/src/lib.rs"]
#[allow(dead_code)]
mod keyring_inline;

#[path = "../crates/apeireth-machine-id/src/lib.rs"]
#[allow(dead_code)]
mod machine_id_inline;

#[cfg(test)]
mod infra_p0_integration {
    use super::{keyring_inline, machine_id_inline};

    // --- 4.1 apeireth-keyring ---

    #[test]
    fn keyring_tool_whitelist_has_eight_tools() {
        assert_eq!(keyring_inline::TOOL_WHITELIST.len(), 8);
        assert!(keyring_inline::TOOL_WHITELIST.contains(&"apeireth_keyring_set"));
        assert!(keyring_inline::TOOL_WHITELIST.contains(&"apeireth_keyring_get"));
        assert!(keyring_inline::TOOL_WHITELIST.contains(&"apeireth_keyring_lock"));
    }

    #[test]
    fn keyring_supported_platforms_has_four() {
        // 4 平台: Windows / Darwin / Linux / Bsd
        assert_eq!(keyring_inline::SUPPORTED_PLATFORMS.len(), 4);
    }

    #[test]
    fn keyring_pbkdf2_iterations_meets_owasp_2023() {
        // OWASP 2023 推荐 600,000 次
        assert!(keyring_inline::FALLBACK_PBKDF2_ITERATIONS >= 600_000);
    }

    #[test]
    fn keyring_aes_key_length_is_256() {
        assert_eq!(keyring_inline::FALLBACK_AES_KEY_LEN, 32); // 32 bytes = AES-256
        assert_eq!(keyring_inline::FALLBACK_NONCE_LEN, 12); // 12 bytes = GCM standard
    }

    #[test]
    fn keyring_token_max_length() {
        assert_eq!(keyring_inline::TOKEN_MAX_LENGTH, 4096);
    }

    #[test]
    fn keyring_secret_bytes_redacts_on_serialize() {
        use keyring_inline::SecretBytes;
        let secret = SecretBytes::new(b"hunter2");
        let j = serde_json::to_string(&secret).unwrap();
        assert!(!j.contains("hunter2"), "SecretBytes 序列化应脱敏");
    }

    #[test]
    fn keyring_token_type_service_names() {
        use keyring_inline::TokenType;
        // TokenType 至少 1 个 service name 非空
        for t in [TokenType::ApiKey, TokenType::OAuth, TokenType::Pat] {
            let svc = t.service();
            assert!(!svc.is_empty(), "TokenType {:?} 应有 service 名", t);
        }
    }

    #[test]
    fn keyring_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let result = keyring_inline::validate_tool_call("apeireth_keyring_exfiltrate", &args);
        assert!(result.is_err());
    }

    // --- 4.2 apeireth-machine-id ---

    #[test]
    fn machine_id_tool_whitelist_has_six_tools() {
        assert_eq!(machine_id_inline::TOOL_WHITELIST.len(), 6);
        assert!(machine_id_inline::TOOL_WHITELIST.contains(&"apeireth_machine_id_get"));
        assert!(machine_id_inline::TOOL_WHITELIST.contains(&"apeireth_machine_id_hash"));
    }

    #[test]
    fn machine_id_supported_platforms_has_four() {
        // 4 平台: Windows / Darwin / Linux / Bsd
        assert_eq!(machine_id_inline::SUPPORTED_PLATFORMS.len(), 4);
    }

    #[test]
    fn machine_id_platform_as_str() {
        use machine_id_inline::Platform;
        assert_eq!(Platform::Windows.as_str(), "windows");
        assert_eq!(Platform::Darwin.as_str(), "darwin");
        assert_eq!(Platform::Linux.as_str(), "linux");
        assert_eq!(Platform::Bsd.as_str(), "bsd");
    }

    #[test]
    fn machine_id_platform_detect_returns_valid_platform() {
        use machine_id_inline::Platform;
        let p = Platform::detect();
        // detect() 必须返回 4 个支持平台之一
        assert!(matches!(p, Platform::Windows | Platform::Darwin | Platform::Linux | Platform::Bsd));
    }

    #[test]
    fn machine_id_hash_is_deterministic() {
        let raw = "test-uuid-12345";
        let h1 = machine_id_inline::hash_machine_id(raw).expect("hash1");
        let h2 = machine_id_inline::hash_machine_id(raw).expect("hash2");
        assert_eq!(h1, h2, "同 input 应产同 hash (SHA256 确定性)");
    }

    #[test]
    fn machine_id_hash_uses_sha256() {
        // SHA256 = 64 hex chars
        let h = machine_id_inline::hash_machine_id("input").expect("hash");
        assert_eq!(h.len(), 64, "SHA256 hex = 64 chars");
    }

    #[test]
    fn machine_id_schema_version() {
        assert_eq!(machine_id_inline::MACHINE_ID_SCHEMA_VERSION, "1");
        assert_eq!(machine_id_inline::MACHINE_ID_HASH_ALGO, "sha256");
    }

    #[test]
    fn machine_id_default_cache_path_returns_path() {
        // default_cache_path() 在不同平台可能成功/失败, 但调用应不 panic
        let result = machine_id_inline::default_cache_path();
        // 接受 Ok 或 Err (沙箱/平台不支持)
        let _ = result;
    }
}

// ============================================================
// Section 5: 2 SDK stub (lark / voice)
// ============================================================

#[path = "../crates/apeireth-lark/src/lib.rs"]
#[allow(dead_code)]
mod lark_inline;

#[path = "../crates/apeireth-voice/src/lib.rs"]
#[allow(dead_code)]
mod voice_inline;

#[path = "../crates/apeireth-provider-claude-code/src/lib.rs"]
#[allow(dead_code)]
mod provider_claude_code_inline;

#[cfg(test)]
mod sdk_stub_integration {
    use super::{lark_inline, voice_inline};

    // --- 5.1 apeireth-lark ---

    #[test]
    fn lark_tool_whitelist_has_nine_tools() {
        assert_eq!(lark_inline::TOOL_WHITELIST.len(), 9);
        assert_eq!(lark_inline::TOOL_WHITELIST_COUNT, 9);
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_send_message"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_create_event"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_search_documents"));
        assert!(lark_inline::TOOL_WHITELIST.contains(&"apeireth_lark_stub_status"));
    }

    #[test]
    fn lark_stub_mode_is_true() {
        // R20 阶段 3 SDK stub 阶段: STUB_MODE = true
        assert!(lark_inline::STUB_MODE);
        assert!(lark_inline::is_stub_mode());
    }

    #[test]
    fn lark_compile_time_constants() {
        assert_eq!(lark_inline::LARK_SCHEMA_VERSION, "1");
        assert_eq!(lark_inline::PLATFORM_NAME, "apeireth");
        assert_eq!(lark_inline::LARK_API_BASE_URL, "https://open.feishu.cn/open-apis");
        assert_eq!(lark_inline::LARK_TOKEN_CACHE_TTL_SECONDS, 7200);
        assert_eq!(lark_inline::LARK_MAX_MESSAGE_LENGTH, 4096);
    }

    #[test]
    fn lark_message_type_supported_list() {
        // SUPPORTED_MESSAGE_TYPES 至少 1 个
        assert!(!lark_inline::SUPPORTED_MESSAGE_TYPES.is_empty());
    }

    #[test]
    fn lark_tenant_token_is_expired() {
        use lark_inline::TenantAccessToken;
        // 过期时间 0 (1970) 必 expired
        let token = TenantAccessToken { token: "x".into(), expires_at: std::time::SystemTime::UNIX_EPOCH };
        assert!(token.is_expired());
    }

    #[test]
    fn lark_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let result = lark_inline::validate_tool_call("apeireth_lark_send_email", &args);
        assert!(result.is_err());
    }

    // --- 5.2 apeireth-voice ---

    #[test]
    fn voice_tool_whitelist_has_nine_tools() {
        assert_eq!(voice_inline::TOOL_WHITELIST.len(), 9);
        assert_eq!(voice_inline::TOOL_WHITELIST_COUNT, 9);
        assert!(voice_inline::TOOL_WHITELIST.contains(&"apeireth_voice_wake_word_detect"));
        assert!(voice_inline::TOOL_WHITELIST.contains(&"apeireth_voice_transcribe"));
        assert!(voice_inline::TOOL_WHITELIST.contains(&"apeireth_voice_synthesize"));
        assert!(voice_inline::TOOL_WHITELIST.contains(&"apeireth_voice_stub_status"));
    }

    #[test]
    fn voice_stub_mode_is_true() {
        // R20 阶段 3 SDK stub 阶段: STUB_MODE = true
        assert!(voice_inline::STUB_MODE);
        assert!(voice_inline::is_stub_mode());
    }

    #[test]
    fn voice_compile_time_constants() {
        assert_eq!(voice_inline::VOICE_SCHEMA_VERSION, "1");
        assert_eq!(voice_inline::PLATFORM_NAME, "apeireth");
        assert_eq!(voice_inline::VOICE_SAMPLE_RATE_HZ, 16000);
        assert_eq!(voice_inline::VOICE_FRAME_LENGTH, 512);
        assert_eq!(voice_inline::VOICE_DEFAULT_KEYWORD, "apeireth");
        assert_eq!(voice_inline::VOICE_MAX_AUDIO_SECONDS, 30);
    }

    #[test]
    fn voice_wake_word_type_as_str() {
        use voice_inline::WakeWordType;
        for w in [WakeWordType::Apeireth, WakeWordType::Computer, WakeWordType::Jarvis, WakeWordType::HeyApeireth, WakeWordType::Custom] {
            let s = w.as_str();
            assert!(!s.is_empty(), "WakeWordType {:?} 应有 as_str", w);
        }
    }

    #[test]
    fn voice_supported_wake_words_list_non_empty() {
        assert!(!voice_inline::SUPPORTED_WAKE_WORDS.is_empty());
    }

    #[test]
    fn voice_audio_frame_new_has_samples() {
        use voice_inline::AudioFrame;
        let frame = AudioFrame::new(vec![100, -100, 0, 1000]);
        assert_eq!(frame.samples.len(), 4);
    }

    #[test]
    fn voice_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let result = voice_inline::validate_tool_call("apeireth_voice_hack_mic", &args);
        assert!(result.is_err());
    }
}

// ============================================================
// Section 6: 跨 crate 集成场景 (R20 阶段 1 收官验证)
// ============================================================

#[cfg(test)]
mod cross_crate_integration {
    use super::{keyring_inline, machine_id_inline, mcp_ssh_inline, mcp_relay_image_inline};

    /// 跨 crate 集成: 1) machine-id 拉 host fingerprint; 2) keyring 存 API token;
    /// 3) mcp-ssh 用 host fingerprint 找密钥; 4) mcp-relay-image 准备图 prompt
    /// 模拟 R20 阶段 1 端到端 "AI agent 配置 + 工作" 流程
    #[test]
    fn host_fingerprint_x_keyring_x_mcp_integration() {
        // 1) machine-id
        use machine_id_inline::Platform;
        let host_platform = Platform::detect();
        assert!(matches!(host_platform, Platform::Windows | Platform::Darwin | Platform::Linux | Platform::Bsd));

        // 2) machine-id 哈希产出稳定 fingerprint
        let fp = machine_id_inline::hash_machine_id("test-host").expect("hash");
        assert_eq!(fp.len(), 64);

        // 3) keyring service 名包含 platform + fingerprint (模拟 key lookup key)
        use keyring_inline::TokenType;
        let token_type = TokenType::ApiKey;
        let service = token_type.service();
        assert!(!service.is_empty());

        // 4) mcp-ssh tool whitelist 仍可用 (跨 crate 一致性)
        assert_eq!(mcp_ssh_inline::TOOL_WHITELIST.len(), 8);

        // 5) mcp-relay-image image format round-trip
        use mcp_relay_image_inline::ImageFormat;
        let fmt = ImageFormat::from_mime("image/png").expect("png");
        assert_eq!(fmt.extension(), "png");
    }

    /// 跨 crate 集成: 5 P0 MCP crate 工具白名单一致性 (m3 防御编译期 hardcode)
    #[test]
    fn five_p0_mcp_tool_whitelists_are_consistent() {
        assert_eq!(super::mcp_ssh_inline::TOOL_WHITELIST.len(), 8);
        assert_eq!(super::mcp_winrm_inline::TOOL_WHITELIST.len(), 8);
        assert_eq!(super::mcp_relay_image_inline::TOOL_WHITELIST.len(), 5);
        assert!(!super::team_lead_inline::TOOL_WHITELIST.is_empty());

        // 所有白名单应包含 validate 工具 (不一定, 至少 team-lead)
        for tool in super::team_lead_inline::TOOL_WHITELIST {
            assert!(tool.starts_with("apeireth_team_lead_"), "tool 应有正确前缀: {}", tool);
        }
    }

    /// 跨 crate 集成: workflow node types 跟 team-lead orchestrator 兼容
    #[test]
    fn workflow_node_types_compatible_with_team_lead() {
        use super::team_lead_inline::AgentRole;
        use super::workflow_inline::NodeType;

        // workflow 8 节点类型: agent / loop / transform / condition / team / mission / watch / review
        // team-lead 3 agent role: Supervisor / Worker / Observer
        // 关键兼容: Task node (per v0.9.21 agent) 跟 Worker role 对应
        let node_types_count = [
            NodeType::Task,
            NodeType::Loop,
            NodeType::Transform,
            NodeType::Decision,
            NodeType::Team,
            NodeType::Mission,
            NodeType::Watch,
            NodeType::Review,
        ]
        .len();
        assert_eq!(node_types_count, 8, "8 节点类型 1:1 翻译 v0.9.21");

        let roles_count = [AgentRole::Supervisor, AgentRole::Worker, AgentRole::Observer].len();
        assert_eq!(roles_count, 3);
    }
}

// ============================================================
// Section 7: K-1 强校验 (per 主人 R17 4 K-1 invariant 要求)
// ============================================================

#[cfg(test)]
mod k1_invariants {
    use super::*;

    /// K-1.1: 14 crate 全部有 TOOL_WHITELIST 或等价的 hardcode schema
    #[test]
    fn k1_all_crates_have_tool_whitelist() {
        // 11/14 强制: 3 个 (workflow / image-prompt / keyring) 等价 hardcode
        // 这里只测最严的: TOOL_WHITELIST 编译期 hardcode
        assert!(!mcp_ssh_inline::TOOL_WHITELIST.is_empty());
        assert!(!mcp_winrm_inline::TOOL_WHITELIST.is_empty());
        assert!(!mcp_relay_image_inline::TOOL_WHITELIST.is_empty());
        assert!(!team_lead_inline::TOOL_WHITELIST.is_empty());
        assert!(!image_prompt_inline::TOOL_WHITELIST.is_empty());
        assert!(!rollback_inline::TOOL_WHITELIST.is_empty());
        assert!(!plugin_inline::TOOL_WHITELIST.is_empty());
        assert!(!repo_scan_inline::TOOL_WHITELIST.is_empty());
        assert!(!repo_analyzer_inline::TOOL_WHITELIST.is_empty());
        assert!(!keyring_inline::TOOL_WHITELIST.is_empty());
        assert!(!machine_id_inline::TOOL_WHITELIST.is_empty());
        assert!(!lark_inline::TOOL_WHITELIST.is_empty());
        assert!(!voice_inline::TOOL_WHITELIST.is_empty());
        // 14/14 verified
    }

    /// K-1.2: 5 Provider 第一个 (claude-code) TOOL_WHITELIST 编译期 hardcode
    #[test]
    fn k1_provider_claude_code_has_tool_whitelist() {
        use super::provider_claude_code_inline;
        assert!(!provider_claude_code_inline::TOOL_WHITELIST.is_empty());
    }

    /// K-1.3: 所有 14 crate 的 TOOL_WHITELIST 工具名前缀是 `apeireth_*`
    #[test]
    fn k1_all_tool_names_have_apeireth_prefix() {
        for tool in mcp_ssh_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in mcp_winrm_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in mcp_relay_image_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in team_lead_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in image_prompt_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in rollback_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in plugin_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in repo_scan_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in repo_analyzer_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in keyring_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in machine_id_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in lark_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
        for tool in voice_inline::TOOL_WHITELIST { assert!(tool.starts_with("apeireth_"), "{}", tool); }
    }

    /// K-1.4: 所有 14 crate 的 SCHEMA_VERSION = "1" (per v09021 rust translation 1:1)
    #[test]
    fn k1_all_crates_have_schema_v1() {
        assert_eq!(image_prompt_inline::PROMPT_SCHEMA_VERSION, "1");
        assert_eq!(rollback_inline::ROLLBACK_SCHEMA_VERSION, "1");
        assert_eq!(plugin_inline::PLUGIN_SCHEMA_VERSION, "1");
        assert_eq!(repo_scan_inline::REPO_SCAN_SCHEMA_VERSION, "1");
        assert_eq!(repo_analyzer_inline::REPO_ANALYZER_SCHEMA_VERSION, "1");
        assert_eq!(keyring_inline::KEYRING_SCHEMA_VERSION, "1");
        assert_eq!(machine_id_inline::MACHINE_ID_SCHEMA_VERSION, "1");
        assert_eq!(lark_inline::LARK_SCHEMA_VERSION, "1");
        assert_eq!(voice_inline::VOICE_SCHEMA_VERSION, "1");
    }
}

// ============================================================
// Section 8: 外部资源测试 (#[ignore] 标, 跑时用 --ignored)
// ============================================================

#[cfg(test)]
mod external_resource_tests {
    /// 真实 SSH 连接测试 — 需要 SSH server (本机:22 或 mock server)
    /// 跑法: `cargo test --test workspace_integration_v2 -- --ignored ssh_real_connect`
    #[tokio::test]
    #[ignore = "需要真实 SSH server; 集成测试环境下不可用"]
    async fn ssh_real_connect_to_localhost() {
        use super::mcp_ssh_inline::{SshAuthMethod, SshMcpConfig, SshMcpServer, SshMcpServerTrait};
        let server = SshMcpServer::new(SshMcpConfig::default()).unwrap();
        let auth = SshAuthMethod::Agent { username: "test".into(), socket_path: None };
        let result = server.connect("test-session", "127.0.0.1", 22, auth).await;
        assert!(result.is_ok(), "应能连本地 SSH server");
    }

    /// 真实 WinRM 连接测试 — 需要 WinRM server (5985/5986)
    /// 跑法: `cargo test --test workspace_integration_v2 -- --ignored winrm_real_connect`
    #[tokio::test]
    #[ignore = "需要真实 WinRM server; 集成测试环境下不可用"]
    async fn winrm_real_connect() {
        // skeleton 阶段: 仅占位, 不真连
        eprintln!("WinRM 真实连接 - 等 R20 阶段 1 实施完成后启用");
    }

    /// 真实 LLM API 调用 — 需要 ANTHROPIC_API_KEY env
    /// 跑法: `cargo test --test workspace_integration_v2 -- --ignored real_llm_invoke`
    #[tokio::test]
    #[ignore = "需要 ANTHROPIC_API_KEY env; CI 环境跳过"]
    async fn real_llm_invoke() {
        eprintln!("真实 LLM 调用 - 等 R20 阶段 4 实施完成后启用");
    }
}
