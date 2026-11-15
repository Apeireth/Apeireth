//! The self-authored regression corpus and its oracle.
//!
//! ⭐ 口径（必须原样保留）：本卷是**回归集**，由本仓作者与实现同一批人编写，
//! **不是独立盲测**。它只证明"交付契约 + 评分契约"仍然成立，不证明模型回答质量。
//! 任何新增题一旦用于修实现，就同时成为开发证据，不得再称独立盲测。
//!
//! 题面与期望答案分开存放：oracle 不进入任何模型可见上下文。

use super::artifacts::hash;
use super::schema::{
    ExpectedFact, Fact, InputBundle, Operation, OracleBundle, OracleCase, Query, RequiredStep,
    SessionSpec, Status, TurnKind, TurnSpec, INPUT_PROTOCOL, ORACLE_PROTOCOL,
};

/// Corpus identity. Changing it changes every bound digest.
pub const CORPUS_ID: &str = "memory-longterm-regression-2026-11";

/// Opaque token that must never reach a model-facing request, a stored
/// record or a run artifact. It makes oracle leakage directly decidable.
pub const ORACLE_CANARY: &str = "apeireth-oracle-canary-7c41d2e9";

/// Primary session: one long history with several true statements.
pub const SESSION_PRIMARY: &str = "s-primary";
/// Secondary session: one record owned by another account.
pub const SESSION_SECONDARY: &str = "s-secondary";
/// Quoted session: one record that is a quote, not the owner's own fact.
pub const SESSION_QUOTED: &str = "s-quoted";

const TURN_TIME: &str = "2026-11-01T08:00:00Z";

fn turn(id: &str, session: &str, text: &str, kind: TurnKind) -> TurnSpec {
    TurnSpec {
        id: id.to_string(),
        session: session.to_string(),
        at_utc: TURN_TIME.to_string(),
        text: text.to_string(),
        kind,
    }
}

/// The model-facing corpus: sessions, turns, operations and queries.
pub fn input_bundle() -> InputBundle {
    let sessions = vec![
        SessionSpec {
            id: SESSION_PRIMARY.to_string(),
            owner: "owner-primary".to_string(),
        },
        SessionSpec {
            id: SESSION_SECONDARY.to_string(),
            owner: "owner-secondary".to_string(),
        },
        SessionSpec {
            id: SESSION_QUOTED.to_string(),
            owner: "owner-quoted".to_string(),
        },
    ];

    // Noise turns carry no fact and never contain a topic keyword of any query
    // (住 / 城市 / 项目 / 房子), so they can never answer a question.
    let mut turns = vec![
        turn("t01", SESSION_PRIMARY, "我住在北岚市。", TurnKind::Assert),
        turn(
            "t02",
            SESSION_PRIMARY,
            "今天的会议是下午三点开始。",
            TurnKind::Noise,
        ),
        turn(
            "t03",
            SESSION_PRIMARY,
            "我刚才在楼下买了咖啡。",
            TurnKind::Noise,
        ),
        turn(
            "t04",
            SESSION_PRIMARY,
            "晨星项目的联络城市是东隅市。",
            TurnKind::Assert,
        ),
        turn(
            "t05",
            SESSION_PRIMARY,
            "窗外的天已经黑了。",
            TurnKind::Noise,
        ),
        turn(
            "t06",
            SESSION_PRIMARY,
            "我把上周的纪要整理好了。",
            TurnKind::Noise,
        ),
        turn(
            "t07",
            SESSION_PRIMARY,
            "北辰项目的联络城市是南汀市。",
            TurnKind::Assert,
        ),
        turn("t08", SESSION_PRIMARY, "楼下的桂花开了。", TurnKind::Noise),
        turn("t09", SESSION_PRIMARY, "我带了两支笔。", TurnKind::Noise),
        turn(
            "t10",
            SESSION_PRIMARY,
            "明天的行程还没定。",
            TurnKind::Noise,
        ),
        turn(
            "t11",
            SESSION_PRIMARY,
            "刚才的雨已经停了。",
            TurnKind::Noise,
        ),
        turn(
            "t12",
            SESSION_PRIMARY,
            "我把旧笔记本收起来了。",
            TurnKind::Noise,
        ),
        turn(
            "t13",
            SESSION_PRIMARY,
            "那边的房子是租的。",
            TurnKind::Ambiguous,
        ),
        turn(
            "t14",
            SESSION_PRIMARY,
            "今天的地铁晚点了。",
            TurnKind::Noise,
        ),
        turn(
            "t15",
            SESSION_PRIMARY,
            "我把台灯换成了暖光。",
            TurnKind::Noise,
        ),
        turn(
            "t16",
            SESSION_PRIMARY,
            "上周的录像还没剪完。",
            TurnKind::Noise,
        ),
        turn(
            "t17",
            SESSION_PRIMARY,
            "这周的复盘约在周五。",
            TurnKind::Noise,
        ),
        turn(
            "t18",
            SESSION_PRIMARY,
            "我把钥匙放在玄关了。",
            TurnKind::Noise,
        ),
        turn(
            "t19",
            SESSION_PRIMARY,
            "冰箱里的牛奶快过期了。",
            TurnKind::Noise,
        ),
        turn(
            "t20",
            SESSION_PRIMARY,
            "我把旧衣服捐了出去。",
            TurnKind::Noise,
        ),
        // Another account: the record is real, but it is not in the scope of
        // the querying session.
        turn("t21", SESSION_SECONDARY, "我住在雾原市。", TurnKind::Assert),
        // A quote: it is not the owner's own assertion.
        turn(
            "t22",
            SESSION_QUOTED,
            "他说我住在西岑市。",
            TurnKind::Quoted,
        ),
    ];

    turns.shrink_to_fit();

    let operations = vec![Operation::Forget {
        turn: "t07".to_string(),
        reason: "regression: the source of this record was withdrawn".to_string(),
    }];

    let queries = vec![
        Query {
            id: "c01_long_range_direct".to_string(),
            session: SESSION_PRIMARY.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "我住在哪个城市？".to_string(),
        },
        Query {
            id: "c02_long_range_project".to_string(),
            session: SESSION_PRIMARY.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "晨星项目的联络城市是什么？".to_string(),
        },
        Query {
            id: "c03_quoted_not_citable".to_string(),
            session: SESSION_QUOTED.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "我住在哪个城市？".to_string(),
        },
        Query {
            id: "c04_ambiguous_subject".to_string(),
            session: SESSION_PRIMARY.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "我那边的房子是租的还是买的？".to_string(),
        },
        Query {
            id: "c05_forgotten_source".to_string(),
            session: SESSION_PRIMARY.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "北辰项目的联络城市是什么？".to_string(),
        },
        Query {
            id: "c06_foreign_scope".to_string(),
            session: SESSION_PRIMARY.to_string(),
            at_utc: TURN_TIME.to_string(),
            text: "我住在雾原市吗？".to_string(),
        },
    ];

    InputBundle {
        protocol: INPUT_PROTOCOL.to_string(),
        corpus_id: CORPUS_ID.to_string(),
        owner: "owner-primary".to_string(),
        sessions,
        turns,
        operations,
        queries,
        notes: vec![
            "回归集：由本仓作者编写，不是独立盲测。".to_string(),
            "oracle 与题面分开存放，oracle 不进入模型可见上下文。".to_string(),
            "同账号文件隔离：不同账号的记录放在不同会话作用域内。".to_string(),
        ],
    }
}

/// The oracle: one case per query, authored against the corpus text.
pub fn oracle_bundle(input: &InputBundle) -> OracleBundle {
    let cases = vec![
        OracleCase {
            id: "c01_long_range_direct".to_string(),
            dimension: "long_range_direct".to_string(),
            status: Status::Known,
            facts: vec![ExpectedFact {
                fact: Fact {
                    subject: "用户".to_string(),
                    predicate: "居住城市".to_string(),
                    value: "北岚市".to_string(),
                    valid_from: None,
                    valid_to: None,
                },
                support_anchors: vec!["我住在北岚市".to_string()],
            }],
            required_steps: Vec::new(),
            forbidden_steps: Vec::new(),
            forbidden_anchors: vec!["我住在西岑市".to_string()],
        },
        OracleCase {
            id: "c02_long_range_project".to_string(),
            dimension: "long_range_project".to_string(),
            status: Status::Known,
            facts: vec![ExpectedFact {
                fact: Fact {
                    subject: "晨星项目".to_string(),
                    predicate: "联络城市".to_string(),
                    value: "东隅市".to_string(),
                    valid_from: None,
                    valid_to: None,
                },
                support_anchors: vec!["晨星项目的联络城市是东隅市".to_string()],
            }],
            required_steps: Vec::new(),
            forbidden_steps: Vec::new(),
            forbidden_anchors: Vec::new(),
        },
        OracleCase {
            id: "c03_quoted_not_citable".to_string(),
            dimension: "quoted_not_citable".to_string(),
            status: Status::Unknown,
            facts: Vec::new(),
            required_steps: Vec::new(),
            forbidden_steps: Vec::new(),
            forbidden_anchors: vec!["我住在西岑市".to_string()],
        },
        OracleCase {
            id: "c04_ambiguous_subject".to_string(),
            dimension: "ambiguous_subject".to_string(),
            status: Status::Ambiguous,
            facts: Vec::new(),
            required_steps: vec![RequiredStep {
                id: "ambiguous_source".to_string(),
                after: None,
                support_anchors: vec!["那边的房子是租的".to_string()],
            }],
            forbidden_steps: Vec::new(),
            forbidden_anchors: Vec::new(),
        },
        OracleCase {
            id: "c05_forgotten_source".to_string(),
            dimension: "forgotten_source".to_string(),
            status: Status::Unknown,
            facts: Vec::new(),
            required_steps: Vec::new(),
            forbidden_steps: Vec::new(),
            forbidden_anchors: vec!["北辰项目的联络城市是南汀市".to_string()],
        },
        OracleCase {
            id: "c06_foreign_scope".to_string(),
            dimension: "foreign_scope".to_string(),
            status: Status::Unknown,
            facts: Vec::new(),
            required_steps: Vec::new(),
            forbidden_steps: Vec::new(),
            forbidden_anchors: vec!["我住在雾原市".to_string()],
        },
    ];

    OracleBundle {
        protocol: ORACLE_PROTOCOL.to_string(),
        input_sha256: bundle_digest(input),
        canary: ORACLE_CANARY.to_string(),
        cases,
    }
}

/// Canonical digest of the corpus: the bytes a third party can recompute.
pub fn bundle_digest(input: &InputBundle) -> String {
    hash(&serde_json::to_string(input).unwrap_or_default())
}
