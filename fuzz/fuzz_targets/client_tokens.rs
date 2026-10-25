#![no_main]

//! fuzz: **客户端令牌面**（`client_tokens`）—— 认证 + 秘密不落日志。
//!
//! 三条判据全部取自该模块**自己文档化的契约**，不自造期望：
//!
//! 1. **结构校验必须真拒**（`ClientTokenSpec::validate` 的承诺）：
//!    返回 `Ok` ⇒ 名非空白且 ≤64 字节、令牌非空白、**令牌不等于逃生阀哨兵
//!    `UNPROTECTED_SENTINEL`("none")**、绑定 user_id（若有）非空白且 ≤64 字节。
//!    哨兵那条最要紧：允许把 "none" 当客户端令牌 = 把"无防护"当成一个可登录身份。
//!
//! 2. **表级唯一性**（`ClientTokenConfig::validate` 的承诺）：
//!    返回 `Ok` ⇒ 名两两不同、**令牌值两两不同**（文档写明：重复令牌值是审计歧义，
//!    同一枚 Bearer 可归两个客户端名 —— 拒开，不猜归谁）。
//!
//! 3. **秘密不得漏出**：`redacted_summary()` 与 `Debug` 都承诺「令牌只报 configured /
//!    打码」。断言它们的输出**不含令牌原文** —— 这是"令牌不进日志"的底线，
//!    破了就等于把凭据写进日志文件。
//!
//! 另加：任何输入都不得 panic。

use apeireth_gateway::canonical_entry::GatewayTokenGate;
use apeireth_gateway::client_tokens::{ClientTokenConfig, ClientTokenSpec};
use libfuzzer_sys::fuzz_target;

const SENTINEL: &str = GatewayTokenGate::UNPROTECTED_SENTINEL;

/// `ClientTokenSpec::redacted_summary` 的**精确期望文案**。
///
/// ⚠️ 为什么不用「输出里是否包含令牌原文」这种包含式断言（2026-10-25 我踩过）：
/// 摘要的固定文案本身含 `token=configured` / `token=absent`，
/// 若被 fuzz 的令牌恰好是这些字样的**前缀**（如 `token=configu`），
/// 包含式断言会误报「漏出原文」——**自造期望 = 自造假阳性**。
/// 改为精确匹配：只有真正的原文泄漏才可能让等式失败。
fn expected_summary(name: &str, token: &str) -> String {
    let state = if token.trim().is_empty() {
        "absent"
    } else {
        "configured"
    };
    format!("client \"{}\": token={state}", name.trim())
}

fuzz_target!(|data: &[u8]| {
    let Ok(raw) = std::str::from_utf8(data) else {
        return;
    };

    // ── ① 解析入口不得 panic（任意文本喂进去）────────────────────────────────
    let parsed = ClientTokenConfig::from_env_value(raw);

    // ── ② 若解析成功，表级 + 结构级契约必须成立 ──────────────────────────────
    if let Ok(config) = &parsed {
        // 先自行校验：`from_env_value` 不保证已校验，故以 `validate()` 为契约入口。
        if config.validate().is_ok() {
            let mut names = std::collections::BTreeSet::new();
            let mut tokens = std::collections::BTreeSet::new();
            for spec in &config.clients {
                let name = spec.name.trim();
                let token = spec.token.trim();

                assert!(!name.is_empty(), "校验放行了空客户端名: {name:?}");
                assert!(
                    name.len() <= 64,
                    "校验放行了超长客户端名({} 字节)",
                    name.len()
                );
                assert!(!token.is_empty(), "校验放行了空令牌");
                assert!(
                    token != SENTINEL,
                    "校验放行了逃生阀哨兵当客户端令牌 —— 「无防护」被当成可登录身份"
                );
                if let Some(uid) = &spec.user_id {
                    let uid = uid.trim();
                    assert!(!uid.is_empty(), "校验放行了空白 user_id");
                    assert!(
                        uid.len() <= 64,
                        "校验放行了超长 user_id({} 字节)",
                        uid.len()
                    );
                }

                assert!(names.insert(name.to_string()), "校验放行了重复名: {name:?}");
                assert!(
                    tokens.insert(token.to_string()),
                    "校验放行了重复令牌值 —— 审计歧义（同一枚 Bearer 可归两个客户端名）"
                );
            }
        }

        // ── ③ 秘密不得出现在脱敏输出里（精确文案判定, 不可能假阳性）──────────
        for spec in &config.clients {
            let expected = expected_summary(&spec.name, &spec.token);
            assert_eq!(
                spec.redacted_summary(),
                expected,
                "redacted_summary 文案不符（疑似漏出原文）"
            );
            // Debug 承诺: 令牌打码为 <redacted>
            let debug = format!("{spec:?}");
            assert!(
                debug.contains("token: <redacted>"),
                "Debug 未把令牌打码: {debug:?}"
            );
        }

        // 启动日志承诺 = 逐条 redacted_summary（0 明文）
        let expected_lines: Vec<String> = config
            .clients
            .iter()
            .map(|s| expected_summary(&s.name, &s.token))
            .collect();
        assert_eq!(
            config.redacted_startup_log(),
            expected_lines,
            "redacted_startup_log 与承诺文案不符（疑似漏出原文）"
        );
    }

    // ── ④ 单条 spec 的结构校验：哨兵必拒（独立于表的入口）──────────────────
    let spec_sentinel = ClientTokenSpec {
        name: "c".to_string(),
        token: SENTINEL.to_string(),
        user_id: None,
    };
    assert!(
        spec_sentinel.validate().is_err(),
        "单条 spec 校验必须拒绝逃生阀哨兵当令牌"
    );

    // 任意文本当令牌时，若校验通过则必须不是哨兵、且非空白
    let spec_arbitrary = ClientTokenSpec {
        name: "c".to_string(),
        token: raw.to_string(),
        user_id: None,
    };
    if spec_arbitrary.validate().is_ok() {
        let t = spec_arbitrary.token.trim();
        assert!(!t.is_empty(), "校验放行了空白令牌");
        assert!(t != SENTINEL, "校验放行了哨兵令牌");
        // 脱敏输出仍须精确等于承诺文案
        assert_eq!(
            spec_arbitrary.redacted_summary(),
            expected_summary(&spec_arbitrary.name, t),
            "redacted_summary 文案不符（疑似漏出原文）"
        );
        assert!(
            format!("{spec_arbitrary:?}").contains("token: <redacted>"),
            "Debug 未把令牌打码"
        );
    }
});
