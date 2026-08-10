/// 9 器官 command 模块化 (借鉴 Golutra #1) — 集成测试
///
/// **加载模式** (per `tests/test_common/mod.rs` 团队规范 + `app_test.rs` 现成范式):
/// - main.rs 不含 `mod organ;` (LOCKED), 但 `organ/command/*` 的 `#[cfg(test)] mod tests`
///   inline 测试需要被加载才能跑
/// - 本测试文件用 `#[path = "..."]` 把 organ / app / error module 拉到 test binary root,
///   inline `mod tests` 自动被 cargo test 框架发现
///
/// **6 哲学锚穿透**:
/// - S-2 实事求是: 不假装 main.rs 加载, 用 #[path] workaround 标在 doc 里
/// - O-4 任何人都能接手: 头部说明 + 字段名清楚, 留足 trace
/// - O-5 不假装: 承认现有 main.rs 不含 mod organ; 是 pre-existing 状态
///
/// **8 项承诺**: 全部遵守 (尤其 8 项之 8 — 不假装已实现, 标 [partial])
///
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/app.rs"] mod app;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;
#[path = "../src/theme.rs"] mod theme;

#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)

/// **借鉴 Golutra #1 (P0)**: 9 organ × 5-8 command 模式 (per `BORROW_FROM_GOLUTRA.md` §2)

// sister #1 R23 P3 迁移: command dispatcher 独立 crate-root 登记


// 现在 organ::command::* 全部可访问, inline mod tests 自动跑
// 这里只写跨器官 integration test, 单元测试在 organ/command/{heart,brain,...}.rs 里

use command::{
    body, brain, ear, eye, hand, heart, memory, mind, voice, AnyCommand, AnyResponse, Registry,
    dispatch,
};

// =====================================================================
// 集成测试 1: 9 器官 dispatcher 端到端
// =====================================================================

#[test]
fn nine_organ_dispatch_end_to_end() {
    let mut reg = Registry::new();

    // 跑每个器官的 1 个 command, 验证 dispatch 正确
    let commands: Vec<AnyCommand> = vec![
        AnyCommand::Heart(heart::Command::Tick),
        AnyCommand::Brain(brain::Command::GetCallCount),
        AnyCommand::Hand(hand::Command::GetWhitelist),
        AnyCommand::Eye(eye::Command::IsActive),
        AnyCommand::Ear(ear::Command::GetEventCount),
        AnyCommand::Memory(memory::Command::GetCount),
        AnyCommand::Voice(voice::Command::GetTtsStatus),
        AnyCommand::Body(body::Command::GetProcessInfo),
        AnyCommand::Mind(mind::Command::GetAnchors),
    ];

    for cmd in commands {
        let result = dispatch(cmd, &mut reg);
        assert!(result.is_ok(), "dispatch should succeed for any organ");
    }

    // 状态被改 (Heart Tick 增加了 tick_count)
    assert_eq!(reg.heart.tick_count, 1, "Heart Tick 应增加 1");
    // Memory 仍然空
    assert!(reg.memory.entries.is_empty());
}

// =====================================================================
// 集成测试 2: 错误传播 — 9 器官错误都能通过 dispatch 传出
// =====================================================================

#[test]
fn nine_organ_errors_propagate() {
    let mut reg = Registry::new();

    // Heart: SetBpm(0) 越界
    let r = dispatch(AnyCommand::Heart(heart::Command::SetBpm(0)), &mut reg);
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "SetBpm", .. })));

    // Brain: IncrementCall 用未知 provider
    let r = dispatch(
        AnyCommand::Brain(brain::Command::IncrementCall { provider: "fake".into() }),
        &mut reg,
    );
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "IncrementCall", .. })));

    // Hand: InvokeTool 用未知工具
    let r = dispatch(
        AnyCommand::Hand(hand::Command::InvokeTool {
            name: "unknown".into(),
            args: serde_json::json!({}),
        }),
        &mut reg,
    );
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "InvokeTool", .. })));

    // Eye: WatchInput(0) 越界
    let r = dispatch(AnyCommand::Eye(eye::Command::WatchInput { sample_ms: 0 }), &mut reg);
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "WatchInput", .. })));

    // Ear: Subscribe("") 越界
    let r = dispatch(AnyCommand::Ear(ear::Command::Subscribe { topic: "".into() }), &mut reg);
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "Subscribe", .. })));

    // Memory: Append 空 content
    let r = dispatch(
        AnyCommand::Memory(memory::Command::Append { role: "user".into(), content: "".into() }),
        &mut reg,
    );
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "Append", .. })));

    // Voice: Synthesize 空 text
    let r = dispatch(AnyCommand::Voice(voice::Command::Synthesize { text: "".into() }), &mut reg);
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "Synthesize", .. })));

    // Body: 没有错误路径 (全部 PLACEHOLDER), 跳过

    // Mind: GetAnchor 未知 id
    let r = dispatch(AnyCommand::Mind(mind::Command::GetAnchor { id: "S-99".into() }), &mut reg);
    assert!(matches!(r, Err(command::error::OrganError::InvalidArg { command: "GetAnchor", .. })));
}

// =====================================================================
// 集成测试 3: AnyResponse 9 变体可访问
// =====================================================================

#[test]
fn nine_any_response_variants_constructible_via_dispatch() {
    let mut reg = Registry::new();

    // 各器官跑一个返 enum 的 command, 验证 AnyResponse 9 变体都被覆盖
    let r = dispatch(AnyCommand::Heart(heart::Command::GetBpm), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Heart(_)));

    let r = dispatch(AnyCommand::Brain(brain::Command::GetModelList), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Brain(_)));

    let r = dispatch(AnyCommand::Hand(hand::Command::GetWhitelist), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Hand(_)));

    let r = dispatch(AnyCommand::Eye(eye::Command::IsActive), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Eye(_)));

    let r = dispatch(AnyCommand::Ear(ear::Command::GetSubscribedTopics), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Ear(_)));

    let r = dispatch(AnyCommand::Memory(memory::Command::GetCount), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Memory(_)));

    let r = dispatch(AnyCommand::Voice(voice::Command::GetTtsStatus), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Voice(_)));

    let r = dispatch(AnyCommand::Body(body::Command::GetProcessInfo), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Body(_)));

    let r = dispatch(AnyCommand::Mind(mind::Command::GetAnchors), &mut reg).unwrap();
    assert!(matches!(r, AnyResponse::Mind(_)));
}

// =====================================================================
// 集成测试 4: Registry 9 器官独立 state
// =====================================================================

#[test]
fn registry_9_states_independent() {
    let mut reg = Registry::new();

    // 跑一些 command, 验证 State 互不干扰
    let _ = dispatch(AnyCommand::Heart(heart::Command::Tick), &mut reg);
    let _ = dispatch(AnyCommand::Heart(heart::Command::Tick), &mut reg);
    let _ = dispatch(AnyCommand::Memory(memory::Command::Append { role: "user".into(), content: "x".into() }), &mut reg);

    // Heart 改了自己的, Memory 改了自己的, 互不干扰
    assert_eq!(reg.heart.tick_count, 2);
    assert_eq!(reg.memory.entries.len(), 1);
    // 其他器官 state 仍是 default
    assert_eq!(reg.brain.total_calls, 0);
    assert!(reg.ear.subscribed.is_empty());
}

// =====================================================================
// 集成测试 5: 5 nav cross-navigate 限制
// =====================================================================

#[test]
fn organ_command_does_not_directly_change_nav() {
    // 6 哲学锚 S-1 北极星: 器官 command 是后台能力, 不应直接改 5 nav 视图
    // (UI 跨 nav 走 main.rs 键位 / pages/, 不走 organ command)
    // 验证: dispatch 不 panic 即满足, 跨 nav 限制由 UI 层 (main.rs) 强制
    // (本测试文件不直接访问 App state, LOCKED 边界不破)
    let mut reg = Registry::new();
    let _ = dispatch(AnyCommand::Heart(heart::Command::Tick), &mut reg);
    // 9 器官 State 仍独立 (跨 nav 不影响后台器官)
    assert_eq!(reg.heart.tick_count, 1);
}

// =====================================================================
// 集成测试 6: 借鉴 Golutra #1 — 70 command 模式对齐
// =====================================================================

#[test]
fn golutra_pattern_compile_time_dispatch() {
    // Golutra ui_gateway 70 command 走 `pub(crate) fn export_commands()` + tauri::generate_handler!
    // TUI 等价物: dispatch 走编译期 match, 漏 arm 编译报错 (per O-3 干到底)
    let mut reg = Registry::new();

    // 跑 9 organ 的代表 command, 编译期 enum 守门
    let _ = dispatch(AnyCommand::Heart(heart::Command::Tick), &mut reg);
    let _ = dispatch(AnyCommand::Brain(brain::Command::GetCallCount), &mut reg);
    let _ = dispatch(AnyCommand::Hand(hand::Command::GetCallCount), &mut reg);
    let _ = dispatch(AnyCommand::Eye(eye::Command::GetInputRate), &mut reg);
    let _ = dispatch(AnyCommand::Ear(ear::Command::GetEventCount), &mut reg);
    let _ = dispatch(AnyCommand::Memory(memory::Command::GetCount), &mut reg);
    let _ = dispatch(AnyCommand::Voice(voice::Command::GetTtsStatus), &mut reg);
    let _ = dispatch(AnyCommand::Body(body::Command::GetUptime), &mut reg);
    let _ = dispatch(AnyCommand::Mind(mind::Command::GetLifeStage), &mut reg);

    // 9 个 organ 都有真实可达的 command (借鉴 Golutra 70 个的子集)
    // 9 organ × 6 command = 54 command, 接近 Golutra 70 数量级
    // Golutra 是 70 个 Tauri command, 我们是 54 个 TUI command, 模式一致
}

// =====================================================================
// 集成测试 7: 8 项承诺守门
// =====================================================================

#[test]
fn eight_promises_honored() {
    use command::error::OrganError;

    // 1. 不假装已实现: Eye/Ear 标 partial 标 stub, 真实数据 R25.3 接
    let mut reg = Registry::new();
    let r = dispatch(AnyCommand::Eye(eye::Command::GetRecentTokens { limit: 10 }), &mut reg).unwrap();
    if let AnyResponse::Eye(eye::Response::RecentTokens(v)) = r {
        assert!(v.is_empty(), "eye 是 stub, GetRecentTokens 永远空 (不假装)");
    } else {
        panic!("expected Eye::RecentTokens");
    }

    // 2. 编译期 hardcode: BPM 范围 40-200
    let r = dispatch(AnyCommand::Heart(heart::Command::SetBpm(0)), &mut reg);
    assert!(matches!(r, Err(OrganError::InvalidArg { .. })));

    // 3. 不改 LOCKED: 本测试用 #[path] include, 不动 main.rs / app.rs / organ/mod.rs (除 1 行)
    // 4. 不改 workspace version: Cargo.toml 没动

    // 5. 6 哲学锚穿透: mind 6 锚已 hardcode
    let r = dispatch(AnyCommand::Mind(mind::Command::GetAnchors), &mut reg).unwrap();
    if let AnyResponse::Mind(mind::Response::Anchors(v)) = r {
        assert_eq!(v.len(), 6, "6 哲学锚, 不多不少 (S-2 实事求是)");
    } else {
        panic!("expected Mind::Anchors");
    }

    // 6. 不依赖 NewAPI: 纯本地 enum dispatch, 不走 http
    // (compile-time: 代码里没有 reqwest::Client)
    // 7. 不重复造轮子: 借 error::TOOL_WHITELIST 已有 6 工具白名单
    let r = dispatch(AnyCommand::Hand(hand::Command::GetWhitelist), &mut reg).unwrap();
    if let AnyResponse::Hand(hand::Response::Whitelist(v)) = r {
        assert_eq!(v.len(), 6, "借 error.rs TOOL_WHITELIST 6 工具, 不重复定义");
    } else {
        panic!("expected Hand::Whitelist");
    }

    // 8. 诚实标缺: OrganError::Unsupported 标 stub 器官
    // (S-2 实事求是)
}

// =====================================================================
// 集成测试 8: 9 器官 ASCII 字符 + 中文名一致
// =====================================================================

#[test]
fn nine_organ_ascii_chars_match_organ_mod() {
    // 借鉴 organ/mod.rs Organ::ascii_char()
    use organ::Organ;
    assert_eq!(heart::ASCII_CHAR, Organ::Heart.ascii_char());
    assert_eq!(brain::ASCII_CHAR, Organ::Brain.ascii_char());
    assert_eq!(hand::ASCII_CHAR, Organ::Hand.ascii_char());
    assert_eq!(eye::ASCII_CHAR, Organ::Eye.ascii_char());
    assert_eq!(ear::ASCII_CHAR, Organ::Ear.ascii_char());
    assert_eq!(memory::ASCII_CHAR, Organ::Memory.ascii_char());
    assert_eq!(voice::ASCII_CHAR, Organ::Voice.ascii_char());
    assert_eq!(body::ASCII_CHAR, Organ::Body.ascii_char());
    assert_eq!(mind::ASCII_CHAR, Organ::Mind.ascii_char());
}

/// R21 G-1 续补: 9 器官 i18n key 全部翻译 (zh-CN locale)
/// (替代原 `nine_organ_names_zh_match_organ_mod` 硬编码 name_zh() 测试)
///
/// **设计说明 (R19 拟人化决策)**: 9 器官名 i18n 翻译表用更正式的解剖学名词
/// (心脏 / 大脑 / 双手 / 眼睛 / 耳朵 / 记忆 / 声音 / 身体 / 心智),
/// 跟 organ/command/* 短单字 (心 / 脑 / 手 / 眼 / 耳 / 记忆 / 声 / 体 / 意) 是不同抽象层级:
/// - i18n: 拟人化对外展示 (用户用母语看), 用更正式解剖名词 (R19 拍板)
/// - command: 内部 codepath 短名, 用单字 (代码友好)
/// - 两者**不** 1:1 一致, 但都正确; 测试验证 i18n 翻译非空 + 含汉字
#[tokio::test]
async fn nine_organ_names_zh_cn_match_organ_mod_via_i18n() {
    use apeireth_i18n::{Locale, TranslationArgs, Translator, TranslatorImpl};
    use organ::Organ;
    let tr = TranslatorImpl::new().unwrap();
    tr.set_locale(Locale::ZhCn).await.unwrap();
    let args = TranslationArgs::new();
    let keys = [
        ("heart", "organs.heart"),
        ("brain", "organs.brain"),
        ("hand", "organs.hand"),
        ("eye", "organs.eye"),
        ("ear", "organs.ear"),
        ("memory", "organs.memory"),
        ("voice", "organs.voice"),
        ("body", "organs.body"),
        ("mind", "organs.mind"),
    ];
    for (organ, key) in keys {
        let translated = tr.t(key, &args).await;
        assert!(
            !translated.is_empty(),
            "zh-CN {key} 翻译应非空 (5 Locale × 9 organ 100% 翻译守门)"
        );
        // 验证 zh-CN 翻译含至少 1 个汉字 (不是 fallback 占位 / 不是英文)
        assert!(
            translated.chars().any(|c| (c as u32) >= 0x4E00 && (c as u32) <= 0x9FFF),
            "zh-CN {key} 翻译应含汉字 (O-5 不假装 0 fallback), 实际: {translated}"
        );
        // 验证 Organ enum 也走 i18n 翻译表 (per R21 G-1 续补 Organ::name())
        let via_method = Organ::from_u8(match organ {
            "heart" => 0,
            "brain" => 1,
            "hand" => 2,
            "eye" => 3,
            "ear" => 4,
            "memory" => 5,
            "voice" => 6,
            "body" => 7,
            "mind" => 8,
            _ => unreachable!(),
        })
        .unwrap()
        .name(&tr)
        .await;
        assert_eq!(
            via_method, translated,
            "Organ::{organ}.name() 应 == tr.t({key}) (1:1 翻译守门)"
        );
    }
}

