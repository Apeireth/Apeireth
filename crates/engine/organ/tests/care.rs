//! V3-a 双引擎主动关怀: 引擎级测试证据 (四类触发器 / 双引擎兜底 / 打扰纪律四件 /
//! StoredDoc 拒开 / 自述真值 / 标准回合输入)。
//!
//! 时钟全程注入冻结时钟 ([`FrozenCareClock`]), 确定性可复跑。

use std::sync::Arc;

use apeireth_organ::care::{
    AlwaysAdmitBudget, CareBudgetGate, CareBudgetVerdict, CareClock, CareDiscipline, CareEngine,
    CareHold, CareJudgmentInput, CareModelPort, CarePoll, CareRecord, CareRule, CareStore,
    CareStoreError, CareTrigger, CareVerdict, FrozenCareClock, ModelContextJudge,
    RuleFallbackJudge, SystemCareClock, CARE_RULES_DOC_NAME, CARE_RULES_DOC_VERSION,
};

/// 2026-08-16 00:00:00 UTC (与 emergence 同一锚点口径)。
const DAY_START: i64 = 1_786_838_400_000;
const HOUR_MS: i64 = 3_600_000;
const MINUTE_MS: i64 = 60_000;

fn rule(id: &str, trigger: CareTrigger) -> CareRule {
    CareRule {
        id: id.to_string(),
        trigger,
        material: "问问他最近怎么样".to_string(),
        enabled: true,
        created_at_ms: DAY_START,
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    store: CareStore,
    clock: Arc<FrozenCareClock>,
}

fn fixture(rules: Vec<CareRule>, discipline: CareDiscipline, at_ms: i64) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    let mut body = store.list().expect("empty body");
    body.discipline = discipline;
    body.rules = rules;
    store.save(&body).expect("save");
    Fixture {
        _dir: dir,
        store,
        clock: Arc::new(FrozenCareClock::new(at_ms)),
    }
}

impl Fixture {
    fn engine(&self) -> CareEngine {
        CareEngine::new(
            self.store.clone(),
            self.clock.clone(),
            Arc::new(RuleFallbackJudge),
        )
    }
}

/// ① 久未联系 (inactive): N 小时无交互到点, 发起后以发起时刻续算 (不重复轰炸)。
#[test]
fn inactive_rule_fires_after_n_quiet_hours_and_rearms_after_the_next_window() {
    let fx = fixture(
        vec![rule("check-in", CareTrigger::Inactive { hours: 48 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine();
    engine.observe_interaction(DAY_START);

    fx.clock.set(DAY_START + 47 * HOUR_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));

    fx.clock.set(DAY_START + 48 * HOUR_MS);
    let initiative = match engine.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("48 小时沉默必须到点: {other:?}"),
    };
    assert_eq!(initiative.candidate.rule_id, "check-in");
    assert!(initiative.candidate.trigger.contains("久未联系 48 小时"));

    // 发起不是交互: 连续沉默下一次窗口才再发 (以发起时刻为锚)。
    fx.clock.set(DAY_START + 48 * HOUR_MS + MINUTE_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));
    fx.clock.set(DAY_START + 96 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));

    // 交互刷新锚点: 又要等满 N 小时。
    engine.observe_interaction(DAY_START + 97 * HOUR_MS);
    fx.clock.set(DAY_START + 120 * HOUR_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));
}

/// ② 作息时刻 (daily_moment): 每日 HH:mm 到点, 每日一次。
#[test]
fn daily_moment_rule_fires_at_the_daily_time_once_per_day() {
    let fx = fixture(
        vec![rule(
            "morning",
            CareTrigger::DailyMoment {
                minutes_of_day: 8 * 60 + 30,
            },
        )],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine();

    fx.clock.set(DAY_START + 8 * HOUR_MS + 29 * MINUTE_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));

    fx.clock.set(DAY_START + 8 * HOUR_MS + 30 * MINUTE_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));

    // 当日去重: 同日再 poll 不再发。
    fx.clock.set(DAY_START + 9 * HOUR_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));

    // 次日同一时刻再发。
    fx.clock
        .set(DAY_START + 86_400_000 + 8 * HOUR_MS + 30 * MINUTE_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// ②b 过点补发窗口: 作息时刻过点 catchup 窗口内仍补发, 超窗当日跳过。
#[test]
fn daily_moment_catch_up_window_bounds_the_deferral() {
    let mut discipline = CareDiscipline::default();
    discipline.catchup_minutes = 60;
    let fx = fixture(
        vec![rule(
            "morning",
            CareTrigger::DailyMoment {
                minutes_of_day: 8 * 60,
            },
        )],
        discipline,
        DAY_START,
    );
    let mut engine = fx.engine();
    fx.clock.set(DAY_START + 8 * HOUR_MS + 30 * MINUTE_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));

    // 第二条同型规则 (未发过) 在超窗后不补发。
    let fx2 = fixture(
        vec![rule(
            "morning2",
            CareTrigger::DailyMoment {
                minutes_of_day: 8 * 60,
            },
        )],
        CareDiscipline {
            catchup_minutes: 60,
            ..CareDiscipline::default()
        },
        DAY_START,
    );
    let mut engine2 = fx2.engine();
    fx2.clock.set(DAY_START + 10 * HOUR_MS);
    assert!(
        matches!(engine2.poll().expect("poll"), CarePoll::Idle),
        "过点超过补发窗口 = 当日跳过"
    );
}

/// ③ 纪念日 (anniversary): 每年月日当天到点, 每年一次。
#[test]
fn anniversary_rule_fires_on_the_month_day_once_per_year() {
    // DAY_START = 2026-08-16; 规则钉 08-16。
    let fx = fixture(
        vec![rule(
            "anniversary",
            CareTrigger::Anniversary { month: 8, day: 16 },
        )],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine();

    fx.clock.set(DAY_START + 12 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));

    // 次日不再发 (当日一次)。
    fx.clock.set(DAY_START + 86_400_000 + 12 * HOUR_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));

    // 明年同月日再发 (2026→2027 平年 365 天)。
    fx.clock.set(DAY_START + 365 * 86_400_000 + 12 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// ④ 间隔循环 (interval): 距上次发起每 N 小时到点。
#[test]
fn interval_rule_fires_every_n_hours_from_the_last_firing() {
    let fx = fixture(
        vec![rule("pulse", CareTrigger::Interval { hours: 6 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine();

    fx.clock.set(DAY_START + 6 * HOUR_MS - MINUTE_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));
    fx.clock.set(DAY_START + 6 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
    fx.clock.set(DAY_START + 6 * HOUR_MS + MINUTE_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));
    fx.clock.set(DAY_START + 12 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// 双引擎兜底: 无模型配置 = 纯规则引擎照发 (规则兜底保证不漏)。
#[test]
fn dual_engine_without_a_model_still_dispatches_by_rules() {
    let fx = fixture(
        vec![rule("check-in", CareTrigger::Interval { hours: 1 })],
        CareDiscipline::default(),
        DAY_START,
    );
    // 无模型判定端口: RuleFallbackJudge 是缺省档。
    let mut engine = fx.engine();
    fx.clock.set(DAY_START + HOUR_MS);
    let initiative = match engine.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("纯规则必须照发: {other:?}"),
    };
    assert_eq!(
        initiative.verdict.mode,
        apeireth_organ::care::CareJudgeMode::RuleFallback
    );
    assert!(initiative.verdict.speak);
    let turn_input = initiative.to_turn_input();
    assert!(turn_input.contains("[care:check-in]"), "{turn_input}");
    assert!(turn_input.contains("问问他最近怎么样"), "{turn_input}");
}

/// 模型情境引擎: 显式判定面 —— 该不该说 / 说什么, 不做隐式生成。
#[test]
fn model_context_judge_vetoes_and_redirects_explicitly() {
    struct ScriptedPort {
        raw: String,
    }
    impl CareModelPort for ScriptedPort {
        fn verdict(&self, _input: &CareJudgmentInput) -> Result<String, String> {
            Ok(self.raw.clone())
        }
    }

    // 判定说不 → Declined (台账留痕, 当次到点消费掉)。
    let fx = fixture(
        vec![rule("check-in", CareTrigger::Interval { hours: 1 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = CareEngine::new(
        fx.store.clone(),
        fx.clock.clone(),
        Arc::new(ModelContextJudge::new(Arc::new(ScriptedPort {
            raw: r#"{"speak": false, "focus": null, "reason": "他正在开会, 不打扰"}"#.to_string(),
        }))),
    );
    fx.clock.set(DAY_START + HOUR_MS);
    match engine.poll().expect("poll") {
        CarePoll::Declined { rule_id, verdict } => {
            assert_eq!(rule_id, "check-in");
            assert!(!verdict.speak);
            assert_eq!(
                verdict.mode,
                apeireth_organ::care::CareJudgeMode::ModelContext
            );
        }
        other => panic!("显式判定说不必须落 Declined: {other:?}"),
    }
    let body = fx.store.list().expect("list");
    let record = body.last_care().expect("台账留痕");
    assert!(!record.spoke);
    assert_eq!(record.judge, "model_context");

    // 判定该说 + 说什么方向 → Initiating (focus 进标准回合输入)。
    let fx2 = fixture(
        vec![rule("check-in", CareTrigger::Interval { hours: 1 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine2 = CareEngine::new(
        fx2.store.clone(),
        fx2.clock.clone(),
        Arc::new(ModelContextJudge::new(Arc::new(ScriptedPort {
            raw: r#"{"speak": true, "focus": "问问他周末的登山计划", "reason": "时机合适"}"#
                .to_string(),
        }))),
    );
    fx2.clock.set(DAY_START + HOUR_MS);
    let initiative = match engine2.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("判定该说必须发起: {other:?}"),
    };
    assert_eq!(
        initiative.verdict.focus.as_deref(),
        Some("问问他周末的登山计划")
    );
    assert!(initiative.to_turn_input().contains("问他周末的登山计划"));
}

/// 模型判定失败 = 回退规则判定 (兜底不漏发, 真因如实留痕)。
#[test]
fn model_context_failure_falls_back_to_the_rule_verdict_with_an_honest_note() {
    struct BrokenPort;
    impl CareModelPort for BrokenPort {
        fn verdict(&self, _input: &CareJudgmentInput) -> Result<String, String> {
            Err("模型不可用".to_string())
        }
    }
    let fx = fixture(
        vec![rule("check-in", CareTrigger::Interval { hours: 1 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = CareEngine::new(
        fx.store.clone(),
        fx.clock.clone(),
        Arc::new(ModelContextJudge::new(Arc::new(BrokenPort))),
    );
    fx.clock.set(DAY_START + HOUR_MS);
    let initiative = match engine.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("判定失败必须回退规则照发: {other:?}"),
    };
    assert_eq!(
        initiative.verdict.mode,
        apeireth_organ::care::CareJudgeMode::RuleFallback
    );
    assert!(initiative.verdict.reason.contains("回退规则判定"));
    assert!(initiative.verdict.reason.contains("判定调用失败"));
}

/// 打扰纪律 ① 勿扰窗口: 窗口内拦下不发, 窗口过后 (仍在补发窗口内) 仍发。
#[test]
fn quiet_window_holds_care_until_the_window_ends() {
    let fx = fixture(
        vec![rule(
            "late",
            CareTrigger::DailyMoment {
                minutes_of_day: 22 * 60 + 30,
            },
        )],
        CareDiscipline {
            quiet_start_minutes: Some(22 * 60),
            quiet_end_minutes: Some(23 * 60 + 30),
            catchup_minutes: 120,
            ..CareDiscipline::default()
        },
        DAY_START,
    );
    let mut engine = fx.engine();
    fx.clock.set(DAY_START + 22 * HOUR_MS + 30 * MINUTE_MS);
    match engine.poll().expect("poll") {
        CarePoll::Held { rule_id, hold } => {
            assert_eq!(rule_id, "late");
            assert_eq!(hold, CareHold::QuietWindow);
        }
        other => panic!("勿扰窗口内必须拦下: {other:?}"),
    }
    // 拦下不丢: 窗口结束仍在补发窗口内 → 补发。
    fx.clock.set(DAY_START + 23 * HOUR_MS + 35 * MINUTE_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// 打扰纪律 ② 专注抑制: 连续交互期不插话, 交互停歇后恢复。
#[test]
fn focus_suppression_holds_care_during_a_live_interaction() {
    let fx = fixture(
        vec![rule("pulse", CareTrigger::Interval { hours: 1 })],
        CareDiscipline {
            focus_gap_minutes: 15,
            ..CareDiscipline::default()
        },
        DAY_START,
    );
    let mut engine = fx.engine();
    engine.observe_interaction(DAY_START + HOUR_MS - 5 * MINUTE_MS);
    fx.clock.set(DAY_START + HOUR_MS);
    match engine.poll().expect("poll") {
        CarePoll::Held { hold, .. } => assert_eq!(hold, CareHold::Focus),
        other => panic!("连续交互期必须拦下: {other:?}"),
    }
    // 交互停歇 (超过专注窗口) → 恢复。
    fx.clock.set(DAY_START + HOUR_MS + 10 * MINUTE_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// 打扰纪律 ③ 频率上限: 每 N 小时最多 M 次, 满了拦下 (候选拦下不丢)。
#[test]
fn frequency_cap_holds_care_once_the_window_is_full() {
    let fx = fixture(
        vec![
            rule("a", CareTrigger::Interval { hours: 1 }),
            rule("b", CareTrigger::Interval { hours: 1 }),
        ],
        CareDiscipline {
            frequency_max: 1,
            frequency_window_hours: 12,
            ..CareDiscipline::default()
        },
        DAY_START,
    );
    let mut engine = fx.engine();
    fx.clock.set(DAY_START + HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
    // 第二条规则到点, 但频率上限已满 → 拦下。
    match engine.poll().expect("poll") {
        CarePoll::Held { rule_id, hold } => {
            assert_eq!(rule_id, "b");
            assert_eq!(hold, CareHold::FrequencyCap);
        }
        other => panic!("频率上限必须拦下: {other:?}"),
    }
    // 窗口滑过后恢复 (窗口 12h, 首发在 +1h, +14h 时已出窗)。
    fx.clock.set(DAY_START + 14 * HOUR_MS);
    assert!(matches!(
        engine.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// 打扰纪律 ④ 关怀预算 (同账本接缝): 烧不起就不发, 计费随标准回合落同账本。
#[test]
fn budget_gate_holds_care_when_the_shared_ledger_is_exhausted() {
    struct ExhaustedBudget;
    impl CareBudgetGate for ExhaustedBudget {
        fn admit(&self, _at_ms: i64) -> CareBudgetVerdict {
            CareBudgetVerdict::Exhausted {
                reason: "cost_micros 10000/10000 (会话配额账本, 与三维烧钱闸同一本账)".to_string(),
            }
        }
    }
    let fx = fixture(
        vec![rule("pulse", CareTrigger::Interval { hours: 1 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine().with_budget_gate(Arc::new(ExhaustedBudget));
    fx.clock.set(DAY_START + HOUR_MS);
    match engine.poll().expect("poll") {
        CarePoll::Held { hold, .. } => assert_eq!(hold, CareHold::Budget),
        other => panic!("预算耗尽必须拦下: {other:?}"),
    }

    // 对照: 恒放行闸 (默认档) 不拦。
    let mut open = fx.engine().with_budget_gate(Arc::new(AlwaysAdmitBudget));
    assert!(matches!(
        open.poll().expect("poll"),
        CarePoll::Initiating(_)
    ));
}

/// StoredDoc 拒开: 畸形 / 串档 / 未来版本 / 坏条目一律拒开, 引擎如实报错不带病跑。
#[test]
fn care_rules_stored_doc_refuses_to_open_on_every_broken_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    assert!(store.load().expect("missing is empty").is_none());

    let cases: Vec<serde_json::Value> = vec![
        // 畸形 JSON (文本面直接写坏文件)。
        serde_json::Value::Null,
        // 串档身份。
        serde_json::json!({
            "name": "apeireth.something_else",
            "version": CARE_RULES_DOC_VERSION,
            "compatible_versions": [CARE_RULES_DOC_VERSION],
            "body": { "rules": [] }
        }),
        // 未来版本。
        serde_json::json!({
            "name": CARE_RULES_DOC_NAME,
            "version": 99,
            "compatible_versions": [99],
            "body": { "rules": [] }
        }),
        // 坏条目 (非法 id)。
        serde_json::json!({
            "name": CARE_RULES_DOC_NAME,
            "version": CARE_RULES_DOC_VERSION,
            "compatible_versions": [CARE_RULES_DOC_VERSION],
            "body": { "rules": [ {
                "id": "bad id", "material": "m", "created_at_ms": 0,
                "trigger": { "kind": "interval", "hours": 1 }
            } ] }
        }),
        // 坏条目 (词表外触发器)。
        serde_json::json!({
            "name": CARE_RULES_DOC_NAME,
            "version": CARE_RULES_DOC_VERSION,
            "compatible_versions": [CARE_RULES_DOC_VERSION],
            "body": { "rules": [ {
                "id": "x", "material": "m", "created_at_ms": 0,
                "trigger": { "kind": "cron", "expr": "* * * * *" }
            } ] }
        }),
    ];
    for (index, case) in cases.iter().enumerate() {
        let raw = if case.is_null() {
            "{ not json".to_string()
        } else {
            case.to_string()
        };
        std::fs::write(store.path(), raw).expect("write");
        let err = store.load().expect_err(&format!("case {index} 必须拒开"));
        assert!(
            matches!(err, CareStoreError::Stored(_)) || matches!(err, CareStoreError::Refused(_)),
            "case {index}: {err}"
        );
        // 引擎侧同样不带病跑: poll 走正式通道报真因。
        let mut engine = CareEngine::new(
            store.clone(),
            Arc::new(FrozenCareClock::new(DAY_START)),
            Arc::new(RuleFallbackJudge),
        );
        assert!(engine.poll().is_err(), "case {index}: 坏规则表必须错误即帧");
    }
}

/// 自述关怀节真值: 规则数 / 下次触发 / 最近关怀, 缺位如实 (不编数)。
#[test]
fn care_status_reports_rule_count_next_trigger_and_last_care_truth() {
    let fx = fixture(
        vec![rule(
            "morning",
            CareTrigger::DailyMoment {
                minutes_of_day: 8 * 60 + 30,
            },
        )],
        CareDiscipline::default(),
        DAY_START,
    );
    let status = fx
        .store
        .status(DAY_START + 8 * HOUR_MS, None)
        .expect("status");
    assert_eq!(status.rules, 1);
    assert_eq!(
        status.next_trigger_at_ms,
        Some(DAY_START + 8 * HOUR_MS + 30 * MINUTE_MS),
        "下次触发 = 今日 08:30"
    );
    assert!(status.last_care.is_none(), "尚无关怀 = 显式 None, 不编造");

    fx.store
        .record_care(
            CareRecord {
                rule_id: "morning".to_string(),
                at_ms: DAY_START + 8 * HOUR_MS + 30 * MINUTE_MS,
                judge: "rule_fallback".to_string(),
                spoke: true,
                note: "规则兜底: 触发器到点即发, 不漏关怀".to_string(),
            },
            DAY_START + 8 * HOUR_MS + 30 * MINUTE_MS,
        )
        .expect("record");
    let status = fx
        .store
        .status(DAY_START + 9 * HOUR_MS, None)
        .expect("status");
    let last = status.last_care.expect("最近关怀真值");
    assert_eq!(last.rule_id, "morning");
    assert!(last.spoke);
    assert_eq!(
        status.next_trigger_at_ms,
        Some(DAY_START + 86_400_000 + 8 * HOUR_MS + 30 * MINUTE_MS),
        "已发过 → 下次触发是明天 08:30"
    );
}

/// 输出面: 关怀发起 = 标准回合输入 (零新消息通道; 消息由标准回合生成进会话流)。
#[test]
fn care_initiative_composes_a_standard_turn_input_and_never_a_side_channel() {
    let fx = fixture(
        vec![rule("check-in", CareTrigger::Inactive { hours: 24 })],
        CareDiscipline::default(),
        DAY_START,
    );
    let mut engine = fx.engine();
    engine.observe_interaction(DAY_START);
    fx.clock.set(DAY_START + 24 * HOUR_MS);
    let initiative = match engine.poll().expect("poll") {
        CarePoll::Initiating(initiative) => initiative,
        other => panic!("必须发起: {other:?}"),
    };
    let turn_input = initiative.to_turn_input();
    assert!(turn_input.starts_with("[care:check-in]"), "{turn_input}");
    assert!(
        turn_input.contains("久未联系 24 小时无交互"),
        "{turn_input}"
    );
    assert!(turn_input.contains("rule_fallback 该说"), "{turn_input}");
    assert!(turn_input.contains("以一句关怀消息回应"), "{turn_input}");
}

/// 判定合同是闭合的: 缺字段 / 空理由 / 非 JSON 一律拒绝 (不做隐式生成)。
#[test]
fn the_verdict_contract_is_closed_and_never_invents_content() {
    assert!(CareVerdict::from_structured_json(
        r#"{"speak": true, "focus": "方向", "reason": "合适"}"#
    )
    .is_ok());
    assert!(CareVerdict::from_structured_json(r#"{"speak": true}"#).is_err());
    assert!(
        CareVerdict::from_structured_json(r#"{"speak": "yes", "reason": "x"}"#).is_err(),
        "speak 必须是布尔"
    );
    assert!(
        CareVerdict::from_structured_json(r#"{"speak": true, "reason": " "}"#).is_err(),
        "空理由拒绝"
    );
}

/// 时钟可注入: 系统时钟与冻结时钟同合同 (测试用冻结, 生产用系统)。
#[test]
fn clocks_share_one_contract_and_the_frozen_clock_is_deterministic() {
    let frozen = FrozenCareClock::new(DAY_START);
    assert_eq!(frozen.now_ms(), DAY_START);
    frozen.advance(HOUR_MS);
    assert_eq!(frozen.now_ms(), DAY_START + HOUR_MS);
    assert!(SystemCareClock.now_ms() > DAY_START, "系统时钟是真时间");
}

/// 停用规则不参与触发 (留档不删)。
#[test]
fn disabled_rules_never_fire_but_stay_archived() {
    let mut disabled = rule("off", CareTrigger::Interval { hours: 1 });
    disabled.enabled = false;
    let fx = fixture(vec![disabled], CareDiscipline::default(), DAY_START);
    let mut engine = fx.engine();
    fx.clock.set(DAY_START + 10 * HOUR_MS);
    assert!(matches!(engine.poll().expect("poll"), CarePoll::Idle));
    assert_eq!(fx.store.list().expect("list").rules.len(), 1, "留档不删");
}
